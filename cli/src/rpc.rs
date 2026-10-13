//! **O ledger lido direto, sem a `stellar` CLI e sem indexador.**
//!
//! O verificador do caminho anônimo precisa de duas coisas que
//! `stellar contract invoke` não dá: os **eventos** de uma proposta, que é de
//! onde a lista ordenada de compromissos sai, e os **argumentos** da transação
//! que depositou cada cédula, que é onde a assinatura em anel está — o evento
//! `anonimo` não a carrega.
//!
//! Tudo por `curl` e JSON. O RPC do Soroban aceita `xdrFormat: "json"` e
//! devolve `topicJson`/`valueJson` já decodificados (medido contra a testnet do
//! protocolo 29, ver `docs/SOURCES.md`), o que dispensa um crate de XDR aqui —
//! e dispensar é o ponto: o verificador tem de ser pequeno o bastante para
//! alguém ler inteiro antes de confiar nele.
//!
//! **Retenção.** O RPC público guarda eventos por ~7 dias e transações por
//! menos. Quando a janela já passou, isto devolve o que achou e **diz** que
//! achou menos — nunca completa com silêncio.

use serde_json::Value;
use std::process::Command;

/// Uma cédula anônima como o evento a publica.
#[derive(Debug, Clone, PartialEq)]
pub struct CedulaDoLedger {
    pub secao: u32,
    pub imagem: String,
    pub compromissos: Vec<String>,
    pub escolhas: Vec<u32>,
    pub cripto: String,
    pub ledger: u64,
    pub tx: String,
}

pub struct Rpc {
    url: String,
}

impl Rpc {
    pub fn nova(rede: &str) -> Rpc {
        Rpc {
            url: match rede {
                "mainnet" | "public" => "https://mainnet.sorobanrpc.com".into(),
                outra => format!("https://soroban-{}.stellar.org", outra),
            },
        }
    }

    fn chamar(&self, metodo: &str, params: Value) -> Result<Value, String> {
        let corpo = serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": metodo, "params": params
        })
        .to_string();
        let s = Command::new("curl")
            .args([
                "-s",
                "--max-time",
                "30",
                "-X",
                "POST",
                &self.url,
                "-H",
                "Content-Type: application/json",
                "-d",
                &corpo,
            ])
            .output()
            .map_err(|e| format!("curl não rodou: {}", e))?;
        let v: Value = serde_json::from_slice(&s.stdout)
            .map_err(|e| format!("o RPC não devolveu JSON: {}", e))?;
        if let Some(e) = v.get("error") {
            return Err(format!("o RPC recusou {}: {}", metodo, e));
        }
        v.get("result")
            .cloned()
            .ok_or_else(|| format!("resposta de {} sem resultado", metodo))
    }

    /// A faixa de ledgers que o RPC ainda guarda. É o que separa "não achei" de
    /// "não existe".
    pub fn faixa(&self) -> Result<(u64, u64), String> {
        // `getEvents` devolve `oldestLedger` em qualquer resposta aceita, e é
        // de onde a faixa sai. Pedir do ledger 1 seria recusado; pedir do
        // último é aceito e vazio, e o campo vem igual.
        let fim = self.chamar("getLatestLedger", serde_json::json!({}))?["sequence"]
            .as_u64()
            .ok_or("getLatestLedger sem sequence")?;
        let v = self.chamar(
            "getEvents",
            serde_json::json!({ "startLedger": fim, "pagination": { "limit": 1 } }),
        )?;
        let velho = v["oldestLedger"]
            .as_u64()
            .ok_or("getEvents sem oldestLedger")?;
        Ok((velho, fim))
    }

    /// Os eventos `anonimo` de uma proposta, na ordem em que o ledger os
    /// fechou — que é a ordem que a cadeia de compromissos prende.
    pub fn cedulas(
        &self,
        contrato: &str,
        proposta: &str,
        desde: u64,
    ) -> Result<Vec<CedulaDoLedger>, String> {
        let mut todas = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let filtros = serde_json::json!([{ "type": "contract", "contractIds": [contrato] }]);
            let params = match &cursor {
                Some(c) => serde_json::json!({
                    "filters": filtros, "xdrFormat": "json",
                    "pagination": { "cursor": c, "limit": 200 }
                }),
                None => serde_json::json!({
                    "startLedger": desde.max(1), "filters": filtros, "xdrFormat": "json",
                    "pagination": { "limit": 200 }
                }),
            };
            let r = self.chamar("getEvents", params)?;
            let eventos = r["events"].as_array().cloned().unwrap_or_default();
            let quantos = eventos.len();
            for e in eventos {
                // **Filtrada pela proposta.** Um contrato guarda várias
                // votações, e sem este filtro as cédulas de uma entrariam na
                // cadeia de outra — que não fecharia, mas depois de o
                // verificador já ter dito quantas achou.
                if let Some(c) = cedula_do_evento(&e, proposta) {
                    todas.push(c);
                }
            }
            match r["cursor"].as_str() {
                Some(c) if quantos > 0 => cursor = Some(c.to_string()),
                _ => return Ok(todas),
            }
        }
    }

    /// Os argumentos da invocação daquela transação, em `ScVal` JSON. É onde a
    /// assinatura em anel está — o evento não a carrega.
    pub fn argumentos(&self, tx: &str) -> Result<Vec<Value>, String> {
        let r = self.chamar(
            "getTransaction",
            serde_json::json!({ "hash": tx, "xdrFormat": "json" }),
        )?;
        if r["status"].as_str() != Some("SUCCESS") {
            return Err(format!(
                "a transação {} não está mais na janela do RPC (status {})",
                &tx[..8.min(tx.len())],
                r["status"].as_str().unwrap_or("?")
            ));
        }
        argumentos_do_envelope(&r["envelopeJson"]).ok_or_else(|| {
            format!(
                "o envelope de {} não é uma invocação",
                &tx[..8.min(tx.len())]
            )
        })
    }
}

// ---------- desempacotar `ScVal` em JSON ----------
//
// O formato é o do `stellar-xdr --output json`: um objeto com uma chave só,
// que é o nome do caso da união. Tudo aqui é função pura, e por isso tem teste.

pub fn bytes_de(v: &Value) -> Option<String> {
    v.get("bytes")?.as_str().map(|s| s.to_string())
}

pub fn u32_de(v: &Value) -> Option<u32> {
    v.get("u32")?.as_u64().map(|n| n as u32)
}

pub fn vetor_de(v: &Value) -> Option<&Vec<Value>> {
    v.get("vec")?.as_array()
}

/// Um escalar de `Fr`. **O host o serializa como `u256` decimal**, não como
/// bytes — medido contra a testnet, e é a diferença entre o verificador refazer
/// a assinatura em anel e recusar todas elas.
pub fn u256_de(v: &Value) -> Option<String> {
    match v.get("u256")? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

pub fn simbolo_de(v: &Value) -> Option<&str> {
    v.get("symbol")?.as_str()
}

/// `(anonimo, proposta, secao)` mais `(imagem, compromissos, escolhas, cripto)`.
pub fn cedula_do_evento(e: &Value, proposta: &str) -> Option<CedulaDoLedger> {
    let t = e.get("topicJson")?.as_array()?;
    if t.len() < 3 || simbolo_de(&t[0])? != "anonimo" || bytes_de(&t[1])? != proposta {
        return None;
    }
    let d = vetor_de(e.get("valueJson")?)?;
    if d.len() != 4 {
        return None;
    }
    Some(CedulaDoLedger {
        secao: u32_de(&t[2])?,
        imagem: bytes_de(&d[0])?,
        compromissos: vetor_de(&d[1])?.iter().filter_map(bytes_de).collect(),
        escolhas: vetor_de(&d[2])?.iter().filter_map(u32_de).collect(),
        cripto: bytes_de(&d[3])?,
        ledger: e.get("ledger").and_then(|l| l.as_u64()).unwrap_or(0),
        tx: e.get("txHash")?.as_str()?.to_string(),
    })
}

/// Os argumentos de `invoke_contract`, de dentro do envelope.
pub fn argumentos_do_envelope(env: &Value) -> Option<Vec<Value>> {
    env.get("tx")?
        .get("tx")?
        .get("operations")?
        .as_array()?
        .iter()
        .find_map(|op| {
            op.get("body")?
                .get("invoke_host_function")?
                .get("host_function")?
                .get("invoke_contract")?
                .get("args")?
                .as_array()
                .cloned()
        })
}

#[cfg(test)]
mod testes {
    use super::*;

    /// O JSON é o que o RPC da testnet devolveu de verdade, recortado — um
    /// evento `anonimo` da proposta `d06750b2…`. Inventar a forma aqui faria o
    /// verificador ler silêncio e dizer que conferiu.
    const PROPOSTA: &str = "d06750b29f715aecb3cd636ebc48751e3af098203c1d4789b4bcb157163960bc";

    /// **A cédula de outra votação do mesmo contrato não entra.** Um contrato
    /// guarda várias propostas, e sem o filtro as cédulas de uma entrariam na
    /// cadeia de outra.
    #[test]
    fn a_cedula_de_outra_proposta_nao_entra() {
        let v: Value = serde_json::from_str(EVENTO).unwrap();
        assert!(cedula_do_evento(&v, PROPOSTA).is_some());
        assert_eq!(cedula_do_evento(&v, &"ff".repeat(32)), None);
    }

    const EVENTO: &str = r#"{
      "ledger": 5117233,
      "txHash": "651e1aa76296ee29e844f428d25c66cd7ab02f8d55a55617b514edafddd4e5d3",
      "topicJson": [
        {"symbol": "anonimo"},
        {"bytes": "d06750b29f715aecb3cd636ebc48751e3af098203c1d4789b4bcb157163960bc"},
        {"u32": 1}
      ],
      "valueJson": {"vec": [
        {"bytes": "0ed60bfb"},
        {"vec": [{"bytes": "0bdea852"}, {"bytes": "145bd05e"}]},
        {"vec": []},
        {"bytes": "8f3bc0be"}
      ]}
    }"#;

    #[test]
    fn o_evento_anonimo_desempacota_com_a_secao() {
        let v: Value = serde_json::from_str(EVENTO).unwrap();
        let c = cedula_do_evento(&v, PROPOSTA).expect("o evento não desempacotou");
        assert_eq!(c.secao, 1, "a seção vem do terceiro tópico (§11-N)");
        assert_eq!(c.imagem, "0ed60bfb");
        assert_eq!(c.compromissos, std::vec!["0bdea852", "145bd05e"]);
        assert!(c.escolhas.is_empty());
        assert_eq!(c.cripto, "8f3bc0be");
        assert_eq!(c.ledger, 5_117_233);
    }

    /// Um evento de outro assunto não vira cédula em silêncio.
    #[test]
    fn outros_eventos_sao_ignorados_e_nao_viram_cedula_vazia() {
        let mut v: Value = serde_json::from_str(EVENTO).unwrap();
        v["topicJson"][0] = serde_json::json!({"symbol": "comparec"});
        assert_eq!(cedula_do_evento(&v, PROPOSTA), None);
    }

    /// **Um evento de duas posições é recusado, não lido como se tivesse
    /// três.** É o evento antes do §11-N: sem a seção, cada cédula seria
    /// atribuída à seção 0, e uma votação de três seções apuraria errado
    /// afirmando ter conferido.
    #[test]
    fn o_evento_sem_a_secao_no_topico_e_recusado() {
        let mut v: Value = serde_json::from_str(EVENTO).unwrap();
        v["topicJson"] = serde_json::json!([
            {"symbol": "anonimo"},
            {"bytes": PROPOSTA}
        ]);
        assert_eq!(cedula_do_evento(&v, PROPOSTA), None);
    }

    /// E um dado com outra aridade também: a ordem dos quatro campos é
    /// interface congelada, e ler três como se fossem quatro trocaria o
    /// criptograma pelas escolhas.
    #[test]
    fn o_dado_com_outra_aridade_e_recusado() {
        let mut v: Value = serde_json::from_str(EVENTO).unwrap();
        v["valueJson"] = serde_json::json!({"vec": [{"bytes": "00"}, {"vec": []}, {"vec": []}]});
        assert_eq!(cedula_do_evento(&v, PROPOSTA), None);
    }

    /// `c0` e os `z` da assinatura em anel chegam como `u256` decimal. Ler
    /// como bytes devolvia `None`, e o verificador recusava **todas** as
    /// assinaturas dizendo que o anel não fechava.
    #[test]
    fn o_escalar_vem_como_u256_decimal() {
        let v = serde_json::json!({"u256": "27999606060534953815546491389545140458083123015600"});
        assert_eq!(
            u256_de(&v).unwrap(),
            "27999606060534953815546491389545140458083123015600"
        );
        assert_eq!(u256_de(&serde_json::json!({"bytes": "00"})), None);
    }

    #[test]
    fn os_argumentos_saem_do_envelope() {
        let v: Value = serde_json::json!({
          "tx": { "tx": { "operations": [
            { "body": { "invoke_host_function": { "host_function": { "invoke_contract": {
                "function_name": "votar_anonimo",
                "args": [{"bytes": "ab"}, {"u32": 0}]
            }}}}}
          ]}}
        });
        let a = argumentos_do_envelope(&v).expect("o envelope não desempacotou");
        assert_eq!(a.len(), 2);
        assert_eq!(bytes_de(&a[0]).unwrap(), "ab");
        assert_eq!(u32_de(&a[1]).unwrap(), 0);
    }

    /// Uma transação que não é invocação de contrato — um pagamento, digamos —
    /// não devolve argumentos vazios.
    #[test]
    fn um_envelope_que_nao_e_invocacao_nao_da_argumentos() {
        let v: Value = serde_json::json!({
          "tx": { "tx": { "operations": [ { "body": { "payment": {} } } ] } }
        });
        assert_eq!(argumentos_do_envelope(&v), None);
    }
}
