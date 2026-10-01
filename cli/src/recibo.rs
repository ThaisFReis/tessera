//! `./recibos/<identidade>.key` — a verificabilidade individual, e a lacuna N2.
//!
//! O arquivo guarda os `r_j` de uma cédula. Com eles, quem votou **consegue
//! provar o próprio voto a qualquer pessoa** — que é a verificabilidade
//! individual, e é também a lacuna mais séria da v1: quem coage também confere.
//!
//! O protocolo remove o registro público permanente. Não remove a capacidade
//! de alguém se auto-incriminar. O que dá para fazer é tornar o apagamento
//! trivial e dizê-lo em voz alta — daí `queimar`.

use std::io::Write;
use std::path::PathBuf;

pub struct Recibo {
    pub identidade: String,
    pub proposta: String,
    /// Uma por pergunta, na ordem da cédula.
    pub escolhas: Vec<usize>,
    /// Os `r_j`, em hexadecimal, achatados sobre as perguntas sigilosas.
    pub acasos: Vec<String>,
}

pub fn caminho(identidade: &str) -> PathBuf {
    PathBuf::from("recibos").join(format!("{}.key", identidade))
}

pub fn gravar(r: &Recibo) -> Result<(), String> {
    std::fs::create_dir_all("recibos").map_err(|e| e.to_string())?;
    let mut s = String::new();
    s.push_str("# TESSERA — ESTE ARQUIVO PROVA O SEU VOTO.\n");
    s.push_str("# Enquanto ele existir, você consegue provar em que votou —\n");
    s.push_str("# e quem te obrigar a mostrar consegue conferir.\n");
    s.push_str(&format!("proposta={}\n", r.proposta));
    for (q, e) in r.escolhas.iter().enumerate() {
        s.push_str(&format!("escolha{}={}\n", q, e));
    }
    for (j, a) in r.acasos.iter().enumerate() {
        s.push_str(&format!("r{}={}\n", j, a));
    }

    std::fs::write(caminho(&r.identidade), s).map_err(|e| e.to_string())
}

/// **Sobrescreve antes de remover.** `sobrescrito`, não `apagado`, e o código
/// tem de fazer o que a palavra diz: um `unlink` deixa os bytes no disco.
pub fn queimar(identidade: &str) -> Result<Vec<String>, String> {
    let mut feitos = Vec::new();
    for p in [caminho(identidade), PathBuf::from("recibos").join(format!("{}.json", identidade))] {
        if !p.exists() {
            continue;
        }
        let n = std::fs::metadata(&p).map_err(|e| e.to_string())?.len() as usize;
        {
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .open(&p)
                .map_err(|e| e.to_string())?;
            f.write_all(&vec![0u8; n]).map_err(|e| e.to_string())?;
            f.sync_all().map_err(|e| e.to_string())?;
        }
        std::fs::remove_file(&p).map_err(|e| e.to_string())?;
        feitos.push(p.display().to_string());
    }
    Ok(feitos)
}

#[cfg(test)]
mod testes {
    use super::*;

    /// O aviso tem de estar **dentro** do arquivo. Quem abrir o recibo meses
    /// depois, sem a tela da CLI à vista, precisa descobrir ali mesmo o que
    /// está segurando.
    #[test]
    fn o_arquivo_avisa_o_que_ele_e() {
        let dir = std::env::temp_dir().join(format!("tessera-recibo-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_current_dir(&dir).unwrap();

        gravar(&Recibo {
            identidade: "marta".into(),
            proposta: "contas-2025".into(),
            escolhas: vec![1, 0],
            acasos: vec!["aa".into(), "bb".into()],
        })
        .unwrap();

        let s = std::fs::read_to_string(caminho("marta")).unwrap();
        assert!(s.contains("PROVA O SEU VOTO"));
        assert!(s.contains("r0=aa") && s.contains("r1=bb"));
        assert!(s.contains("escolha0=1") && s.contains("escolha1=0"), "uma escolha por pergunta");
        // Só o segredo mora aqui: as provas são públicas e ficam no estado,
        // para que queimar o recibo não custe auditabilidade.
        assert!(
            !s.lines().any(|l| l.starts_with("prova")),
            "o recibo guardou prova, que e publica"
        );

        // e queimar sobrescreve de verdade antes de remover
        let feitos = queimar("marta").unwrap();
        assert_eq!(feitos.len(), 1);
        assert!(!caminho("marta").exists());
        assert!(queimar("marta").unwrap().is_empty(), "queimar duas vezes quebrou");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
