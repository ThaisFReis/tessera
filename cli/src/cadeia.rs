//! A ponte com a rede, por cima da `stellar` CLI.
//!
//! Chamar a `stellar contract invoke` em vez de falar XDR direto com o RPC é
//! uma escolha deliberada: a assinatura e o versionamento de protocolo ficam
//! com a ferramenta oficial, e o que este projeto tem de provar é a
//! criptografia, não a serialização de transação.
//!
//! **Nenhuma chave privada passa por aqui.** A `stellar` CLI guarda e assina;
//! o Tessera só monta argumentos.

use std::process::Command;

#[derive(Debug)]
pub enum Erro {
    Comando(String),
    Rede { saida: String },
}

impl std::fmt::Display for Erro {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Erro::Comando(s) => write!(f, "não consegui executar a stellar CLI: {}", s),
            Erro::Rede { saida, .. } => write!(f, "{}", resumir(saida)),
        }
    }
}

/// A `stellar` CLI despeja diagnóstico demais num erro. O que interessa para
/// quem está na frente da tela é o nome do erro do contrato.
fn resumir(saida: &str) -> String {
    for nome in NOMES_DE_ERRO {
        if saida.contains(nome) {
            return nome.to_string();
        }
    }
    saida
        .lines()
        .find(|l| l.contains("error") || l.contains("Error"))
        .unwrap_or(saida.lines().next().unwrap_or(""))
        .trim()
        .to_string()
}

/// Os erros do contrato, na ordem do `contracterror`. Casar pelo nome é frágil
/// de propósito: se o contrato ganhar um erro novo e a CLI não souber dele, a
/// mensagem crua aparece em vez de uma tradução errada.
const NOMES_DE_ERRO: &[&str] = &[
    "PropostaJaExiste", "PropostaNaoExiste", "OpcoesForaDaFaixa", "LimiarInvalido",
    "PrazoNoPassado", "VotacaoEncerrada", "VotacaoAindaAberta", "JaVotou",
    "NaoEstaNaListaDeAptos", "PesoNaoUnitario", "ProvaBinariaInvalida",
    "ProvaDeSomaInvalida", "PontoForaDoSubgrupo", "AberturaNaoFecha", "JaApurada",
    "MesaAbaixoDoLimiar", "NaoEMembroDaMesa", "MembroRepetido",
    "AnonimatoInsuficiente", "ArgumentoMalFormado", "EscolhaForaDoBinario",
    "SomaDiferenteDoPeso", "TotalDiferenteDoComparecimento",
];

pub struct Resposta {
    pub valor: String,
    pub tx: Option<String>,
}

#[derive(Clone)]
pub struct Cadeia {
    pub contrato: String,
    pub rede: String,
}

impl Cadeia {
    pub fn nova(contrato: &str, rede: &str) -> Cadeia {
        Cadeia { contrato: contrato.to_string(), rede: rede.to_string() }
    }

    /// Resolve o nome de uma identidade da `stellar` CLI no endereço `G…`.
    pub fn endereco(identidade: &str) -> Result<String, Erro> {
        let s = Command::new("stellar")
            .args(["keys", "address", identidade])
            .output()
            .map_err(|e| Erro::Comando(e.to_string()))?;
        if !s.status.success() {
            return Err(Erro::Rede {
                saida: String::from_utf8_lossy(&s.stderr).to_string(),
            });
        }
        Ok(String::from_utf8_lossy(&s.stdout).trim().to_string())
    }

    /// Chama um método. `enviar = false` simula (leitura), `true` assina e
    /// manda.
    pub fn invocar(
        &self,
        fonte: &str,
        enviar: bool,
        metodo: &str,
        args: &[(&str, String)],
    ) -> Result<Resposta, Erro> {
        let mut cmd = Command::new("stellar");
        cmd.args(["contract", "invoke", "--id", &self.contrato, "--source", fonte,
                  "--network", &self.rede]);
        if enviar {
            cmd.arg("--send=yes");
        }
        cmd.arg("--");
        cmd.arg(metodo);
        for (k, v) in args {
            cmd.arg(format!("--{}", k));
            cmd.arg(v);
        }

        let s = cmd.output().map_err(|e| Erro::Comando(e.to_string()))?;
        let err = String::from_utf8_lossy(&s.stderr).to_string();
        if !s.status.success() {
            return Err(Erro::Rede { saida: err });
        }
        let valor = String::from_utf8_lossy(&s.stdout).trim().to_string();
        Ok(Resposta { valor, tx: extrair_tx(&err) })
    }

    /// **A mesa `k`-de-`n` assinando junto.**
    ///
    /// `require_auth` em `k` endereços distintos precisa de `k` entradas de
    /// autorização assinadas — e uma `contract invoke` assina por uma conta só.
    /// O caminho é montar sem assinar (`--build-only`), passar o envelope de
    /// mão em mão (`tx sign`, uma vez por membro) e só então enviar.
    ///
    /// Na vida real esse envelope viaja entre as `k` pessoas; aqui ele passa
    /// por `k` invocações da mesma ferramenta. A diferença é operacional, não
    /// criptográfica: as assinaturas são as mesmas.
    pub fn invocar_em_conjunto(
        &self,
        assinantes: &[String],
        metodo: &str,
        args: &[(&str, String)],
    ) -> Result<Resposta, Erro> {
        let fonte = assinantes.first().ok_or_else(|| Erro::Rede {
            saida: "a mesa está vazia".into(),
        })?;

        let mut cmd = Command::new("stellar");
        cmd.args(["contract", "invoke", "--id", &self.contrato, "--source", fonte,
                  "--network", &self.rede, "--build-only", "--"]);
        cmd.arg(metodo);
        for (k, v) in args {
            cmd.arg(format!("--{}", k));
            cmd.arg(v);
        }
        let s = cmd.output().map_err(|e| Erro::Comando(e.to_string()))?;
        if !s.status.success() {
            return Err(Erro::Rede { saida: String::from_utf8_lossy(&s.stderr).to_string() });
        }
        let bruto = String::from_utf8_lossy(&s.stdout).trim().to_string();

        // `--build-only` entrega um envelope SEM footprint nem taxa de
        // recurso, e a rede o recusa com `TxMalformed`. A simulação é o passo
        // que os preenche — e é também o que monta as entradas de autorização
        // que cada membro vai assinar.
        let s = Command::new("stellar")
            .args(["tx", "simulate", &bruto, "--source", fonte, "--network", &self.rede])
            .output()
            .map_err(|e| Erro::Comando(e.to_string()))?;
        if !s.status.success() {
            return Err(Erro::Rede { saida: String::from_utf8_lossy(&s.stderr).to_string() });
        }
        let mut envelope = String::from_utf8_lossy(&s.stdout).trim().to_string();

        for membro in assinantes {
            envelope = Cadeia::assinar(&envelope, membro, &self.rede)?;
        }

        let s = Command::new("stellar")
            .args(["tx", "send", &envelope, "--network", &self.rede])
            .output()
            .map_err(|e| Erro::Comando(e.to_string()))?;
        let err = String::from_utf8_lossy(&s.stderr).to_string();
        if !s.status.success() {
            return Err(Erro::Rede { saida: format!("{}{}", String::from_utf8_lossy(&s.stdout), err) });
        }
        let saida = String::from_utf8_lossy(&s.stdout).to_string();
        Ok(Resposta {
            valor: saida.clone(),
            tx: extrair_tx(&err).or_else(|| extrair_hash_json(&saida)),
        })
    }

    fn assinar(envelope: &str, identidade: &str, rede: &str) -> Result<String, Erro> {
        let s = Command::new("stellar")
            .args(["tx", "sign", envelope, "--sign-with-key", identidade, "--network", rede])
            .output()
            .map_err(|e| Erro::Comando(e.to_string()))?;
        if !s.status.success() {
            return Err(Erro::Rede {
                saida: format!(
                    "{} não conseguiu assinar: {}",
                    identidade,
                    String::from_utf8_lossy(&s.stderr)
                ),
            });
        }
        Ok(String::from_utf8_lossy(&s.stdout).trim().to_string())
    }

    /// Taxa e ledger vêm do Horizon, não da CLI: a `stellar contract invoke`
    /// não os imprime, e a demo mostra os dois.
    pub fn detalhes(&self, tx: &str) -> Option<(u64, u64)> {
        let url = format!("https://horizon-{}.stellar.org/transactions/{}", self.rede, tx);
        let s = Command::new("curl").args(["-s", &url]).output().ok()?;
        let v: serde_json::Value = serde_json::from_slice(&s.stdout).ok()?;
        let taxa = v.get("fee_charged")?.as_str()?.parse().ok()?;
        let ledger = v.get("ledger")?.as_u64()?;
        Some((taxa, ledger))
    }

    /// Sequência de ledger atual, para converter prazo em relógio de parede.
    pub fn ledger_atual(rede: &str) -> Option<u64> {
        let url = format!(
            "https://horizon-{}.stellar.org/ledgers?order=desc&limit=1",
            rede
        );
        let s = Command::new("curl").args(["-s", &url]).output().ok()?;
        let v: serde_json::Value = serde_json::from_slice(&s.stdout).ok()?;
        v["_embedded"]["records"][0]["sequence"].as_u64()
    }
}

fn extrair_hash_json(saida: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(saida.trim()).ok()?;
    v.get("hash")
        .or_else(|| v.get("txHash"))
        .and_then(|h| h.as_str())
        .map(|s| s.to_string())
}

fn extrair_tx(saida: &str) -> Option<String> {
    saida
        .lines()
        .find_map(|l| l.split("/tx/").nth(1))
        .map(|s| s.trim().to_string())
        .filter(|s| s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit()))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn extrai_o_hash_da_transacao() {
        let s = "✅ Transaction submitted successfully!\n\
                 🔗 https://stellar.expert/explorer/testnet/tx/\
                 4546dba0a782c2323abbe9af24a8cf65e91296cdd8b83d95e5a2d9c021e73b91\n";
        assert_eq!(
            extrair_tx(s).unwrap(),
            "4546dba0a782c2323abbe9af24a8cf65e91296cdd8b83d95e5a2d9c021e73b91"
        );
        assert_eq!(extrair_tx("nada aqui"), None);
        assert_eq!(extrair_tx("https://x/tx/curto"), None);
    }

    /// O erro do contrato tem de atravessar o despejo de diagnóstico da
    /// `stellar` CLI inteiro e chegar legível na tela.
    #[test]
    fn acha_o_erro_do_contrato_no_meio_do_despejo() {
        let despejo = "2026-10-01 error: transaction simulation failed: \
            HostError: Error(Contract, #19)\n\
            Event log: [Diagnostic Event] ... Error(Contract, #19) \
            AnonimatoInsuficiente ... more noise";
        assert_eq!(resumir(despejo), "AnonimatoInsuficiente");
    }

    #[test]
    fn erro_desconhecido_aparece_cru_em_vez_de_traduzido_errado() {
        let s = "error: something nobody predicted";
        assert_eq!(resumir(s), "error: something nobody predicted");
    }
}
