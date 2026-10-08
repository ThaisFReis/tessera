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
    "PropostaJaExiste",
    "PropostaNaoExiste",
    "OpcoesForaDaFaixa",
    "LimiarInvalido",
    "PrazoNoPassado",
    "VotacaoEncerrada",
    "VotacaoAindaAberta",
    "JaVotou",
    "NaoEstaNaListaDeAptos",
    "PesoNaoUnitario",
    "ProvaBinariaInvalida",
    "ProvaDeSomaInvalida",
    "PontoForaDoSubgrupo",
    "AberturaNaoFecha",
    "JaApurada",
    "MesaAbaixoDoLimiar",
    "NaoEMembroDaMesa",
    "MembroRepetido",
    "AnonimatoInsuficiente",
    "ArgumentoMalFormado",
    "EscolhaForaDoBinario",
    "SomaDiferenteDoPeso",
    "TotalDiferenteDoComparecimento",
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
        Cadeia {
            contrato: contrato.to_string(),
            rede: rede.to_string(),
        }
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
        cmd.args([
            "contract",
            "invoke",
            "--id",
            &self.contrato,
            "--source",
            fonte,
            "--network",
            &self.rede,
        ]);
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
        Ok(Resposta {
            valor,
            tx: extrair_tx(&err),
        })
    }

    /// Taxa e ledger vêm do Horizon, não da CLI: a `stellar contract invoke`
    /// não os imprime, e a demo mostra os dois.
    ///
    /// Com limite de tempo, e de propósito: são números de enfeite. Sem eles a
    /// demo perde duas linhas; com a consulta travada ela perde a votação —
    /// medi uma parada de 140 s no meio de uma fila de votos, que levou o
    /// prazo junto.
    pub fn detalhes(&self, tx: &str) -> Option<(u64, u64)> {
        let url = format!(
            "https://horizon-{}.stellar.org/transactions/{}",
            self.rede, tx
        );
        let s = Command::new("curl")
            .args(["-s", "--max-time", "10", &url])
            .output()
            .ok()?;
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
        let s = Command::new("curl")
            .args(["-s", "--max-time", "10", &url])
            .output()
            .ok()?;
        let v: serde_json::Value = serde_json::from_slice(&s.stdout).ok()?;
        v["_embedded"]["records"][0]["sequence"].as_u64()
    }
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
