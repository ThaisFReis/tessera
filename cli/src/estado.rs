//! `./estado/<proposta>.json` — o contrato entre o Nível 1 e o Nível 2.
//!
//! Todo comando escreve este arquivo. Com ele, o console HTML é uma **função
//! pura** deste JSON: zero acoplamento com a CLI, e cortável sem consequência
//! (UX-CLI §9).
//!
//! **Nada de `r` neste arquivo, nunca.** A aleatoriedade vive só em
//! `./recibos/<identidade>.key`, que é o arquivo que `queimar` sobrescreve.
//! Se um `r` vazasse para cá, `queimar` viraria teatro.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Mesa {
    pub membros: usize,
    pub limiar: u32,
    pub enderecos: Vec<String>,
    pub identidades: Vec<String>,
}

/// Uma pergunta da cédula, com o texto que a pessoa lê.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PerguntaEstado {
    pub texto: String,
    pub opcoes: Vec<String>,
    pub confidencial: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Voto {
    pub posicao: usize,
    pub identidade: String,
    pub endereco: String,
    /// Achatados sobre as perguntas sigilosas, em ordem.
    pub compromissos: Vec<String>,
    /// As provas, em hex. São **públicas** — já estão no ledger — e por isso
    /// moram aqui e não no recibo. Se morassem no recibo, `queimar` apagaria a
    /// capacidade de qualquer pessoa reverificar a boa formação da cédula, e
    /// proteger quem vota passaria a custar auditabilidade. Não custa.
    #[serde(default)]
    pub provas: Vec<String>,
    /// **Uma por pergunta sigilosa.** Era uma só quando a cédula tinha uma
    /// pergunta; com várias, uma prova única deixaria de provar a boa formação
    /// de cada pergunta separadamente.
    #[serde(default)]
    pub provas_soma: Vec<String>,
    pub publico: bool,
    /// As respostas em claro. Numa cédula sigilosa são as perguntas públicas;
    /// numa cédula aberta por revelação voluntária, a cédula inteira.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escolhas: Option<Vec<u32>>,
    pub tx: String,
    pub ledger: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Apuracao {
    pub afirmado: Vec<u32>,
    pub confere: bool,
    pub tx: String,
    pub ledger: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Verificacao {
    pub aptidao: bool,
    pub unicidade: bool,
    pub boa_formacao: bool,
    pub aberturas: bool,
    pub sigilo_minimo: bool,
    pub agregado: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Estado {
    pub proposta: String,
    /// A cédula, em ordem. Uma cédula confidencial é aquela em que todas as
    /// perguntas são sigilosas; uma semiconfidencial mistura.
    pub perguntas: Vec<PerguntaEstado>,
    pub contrato: String,
    pub rede: String,
    /// `H` como o contrato o calculou. Pedir é mais seguro que deduzir: é o
    /// ponto em que cliente e cadeia precisam concordar byte a byte.
    pub gerador_h: String,
    pub raiz_aptos: String,
    /// Endereços dos aptos, na ordem da árvore. A ordem **é** o documento: a
    /// raiz muda se ela mudar, e quem vota precisa do próprio índice.
    pub aptos: Vec<String>,
    pub identidades: Vec<String>,
    pub sigilo_minimo: u32,
    pub mesa: Mesa,
    pub prazo_ledger: u64,
    pub abertura_tx: String,
    pub votos: Vec<Voto>,
    /// `R_j` publicados pela mesa, em decimal. São públicos por desenho: a
    /// soma de `n` valores uniformes não determina nenhum deles.
    #[serde(default)]
    pub aberturas: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apuracao: Option<Apuracao>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verificacao: Option<Verificacao>,
}

impl Estado {
    pub fn caminho(proposta: &str) -> PathBuf {
        PathBuf::from("estado").join(format!("{}.json", proposta))
    }

    pub fn ler(proposta: &str) -> Result<Estado, String> {
        let p = Estado::caminho(proposta);
        let s = std::fs::read_to_string(&p).map_err(|_| {
            format!(
                "não encontrei {}. Esta proposta foi aberta a partir deste diretório?",
                p.display()
            )
        })?;
        serde_json::from_str(&s).map_err(|e| format!("{} está corrompido: {}", p.display(), e))
    }

    pub fn gravar(&self) -> Result<(), String> {
        std::fs::create_dir_all("estado").map_err(|e| e.to_string())?;
        let s = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(Estado::caminho(&self.proposta), s).map_err(|e| e.to_string())
    }

    pub fn confidenciais(&self) -> u32 {
        self.votos.iter().filter(|v| !v.publico).count() as u32
    }

    pub fn publicos(&self) -> u32 {
        self.votos.iter().filter(|v| v.publico).count() as u32
    }

    pub fn indice_do_apto(&self, endereco: &str) -> Option<usize> {
        self.aptos.iter().position(|a| a == endereco)
    }

    pub fn ja_votou(&self, endereco: &str) -> Option<&Voto> {
        self.votos.iter().find(|v| v.endereco == endereco)
    }

    /// Quantas opções sigilosas a cédula tem somadas. É o tamanho dos vetores
    /// de compromisso, de prova e de abertura.
    pub fn opcoes_confidenciais(&self) -> usize {
        self.perguntas
            .iter()
            .filter(|p| p.confidencial)
            .map(|p| p.opcoes.len())
            .sum()
    }

    /// Quantas perguntas sigilosas — o número de provas de soma por cédula.
    pub fn perguntas_confidenciais(&self) -> usize {
        self.perguntas.iter().filter(|p| p.confidencial).count()
    }

    /// Todas as opções, de todas as perguntas, em ordem. É o formato do
    /// resultado que o contrato devolve.
    pub fn todas_as_opcoes(&self) -> Vec<&str> {
        self.perguntas
            .iter()
            .flat_map(|p| p.opcoes.iter().map(|o| o.as_str()))
            .collect()
    }

    /// A cédula tem alguma pergunta pública? É o que distingue uma votação
    /// semiconfidencial de uma inteiramente sigilosa.
    pub fn e_mista(&self) -> bool {
        self.perguntas.iter().any(|p| !p.confidencial)
            && self.perguntas.iter().any(|p| p.confidencial)
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    /// **O `r` não pode existir neste arquivo.** Se vazasse para cá, `queimar`
    /// viraria teatro: a pessoa apagaria o recibo e a prova do voto continuaria
    /// no diretório ao lado.
    #[test]
    fn o_json_nao_tem_lugar_para_aleatoriedade() {
        let e = Estado {
            proposta: "contas-2025".into(),
            votos: vec![Voto {
                posicao: 1,
                identidade: "marta".into(),
                endereco: "GABC".into(),
                compromissos: vec!["0f1ff9".into()],
                provas: vec!["aa".into()],
                provas_soma: vec!["bb".into()],
                publico: false,
                escolhas: None,
                tx: "a4f2".into(),
                ledger: 1,
            }],
            ..Default::default()
        };
        let s = serde_json::to_string(&e).unwrap();
        for proibido in ["\"r\"", "acaso", "aleatori", "segredo", "chave"] {
            assert!(
                !s.contains(proibido),
                "o estado serializado contem {:?}",
                proibido
            );
        }
    }

    #[test]
    fn conta_confidenciais_e_publicos() {
        let voto = |publico| Voto {
            posicao: 0, identidade: "x".into(), endereco: "G".into(),
            compromissos: vec![], provas: vec![], provas_soma: vec![],
            publico, escolhas: None, tx: "t".into(), ledger: 0,
        };
        let e = Estado {
            votos: vec![voto(false), voto(false), voto(true)],
            ..Default::default()
        };
        assert_eq!(e.confidenciais(), 2);
        assert_eq!(e.publicos(), 1);
    }

    #[test]
    fn ida_e_volta_do_json() {
        let e = Estado {
            proposta: "p".into(),
            perguntas: vec![
                PerguntaEstado {
                    texto: "Aprovar as contas?".into(),
                    opcoes: vec!["aprovar".into(), "rejeitar".into()],
                    confidencial: false,
                },
                PerguntaEstado {
                    texto: "Destituir a diretoria?".into(),
                    opcoes: vec!["sim".into(), "nao".into()],
                    confidencial: true,
                },
            ],
            sigilo_minimo: 5,
            ..Default::default()
        };
        let s = serde_json::to_string(&e).unwrap();
        let v: Estado = serde_json::from_str(&s).unwrap();
        assert_eq!(v.perguntas.len(), 2);
        assert_eq!(v.todas_as_opcoes().len(), 4);
        assert_eq!(v.opcoes_confidenciais(), 2, "só a segunda pergunta é sigilosa");
        assert_eq!(v.perguntas_confidenciais(), 1);
        assert!(v.e_mista());
        assert_eq!(v.sigilo_minimo, 5);
        assert!(v.apuracao.is_none());
    }
}
