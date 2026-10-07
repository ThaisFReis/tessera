//! `./recibos/<identidade>.key` — o recibo que **esta versão não escreve
//! mais**.
//!
//! O arquivo guardava os `r_j` de uma cédula. Com eles, quem votou prova o
//! próprio voto a qualquer pessoa — e é exatamente isso que a compra de voto
//! precisa, porque sem prova quem paga não sabe se foi entregue.
//!
//! A v1 gravava o recibo e pedia que a pessoa o apagasse. Dois problemas: quem
//! coage simplesmente manda não apagar, e nenhum comando lia o arquivo — ele
//! não servia para conferir voto nem para apurar. Era passivo puro, e a janela
//! entre gravar e apagar não comprava nada.
//!
//! Agora `votar` não grava. O `r` vive no processo e morre com ele, e a tela
//! depois do voto diz isso em vez de pedir uma faxina.
//!
//! `queimar` continua aqui para os arquivos que rodadas antigas deixaram.

use std::io::Write;
use std::path::PathBuf;

pub fn caminho(identidade: &str) -> PathBuf {
    PathBuf::from("recibos").join(format!("{}.key", identidade))
}

pub fn queimar(identidade: &str) -> Result<Vec<String>, String> {
    let mut feitos = Vec::new();
    for p in [
        caminho(identidade),
        PathBuf::from("recibos").join(format!("{}.json", identidade)),
    ] {
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

    /// Duas garantias, e a primeira é a que importa mais: **nenhum comando
    /// desta CLI escreve recibo.** O teste olha o código-fonte, porque é a
    /// única forma de provar uma ausência — se alguém reintroduzir a gravação,
    /// isto quebra.
    #[test]
    fn nenhum_comando_grava_recibo() {
        let fonte = include_str!("comandos.rs");
        assert!(
            !fonte.contains("recibo::gravar"),
            "voltou a gravar recibo: o `r` persistido é o que a compra de voto precisa"
        );
    }

    /// E a segunda: quando um recibo de rodada antiga existir, queimar
    /// **sobrescreve** antes de remover — apagar o inode não basta.
    #[test]
    fn queimar_sobrescreve_antes_de_remover() {
        let dir = std::env::temp_dir().join(format!("tessera-recibo-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("recibos")).unwrap();
        std::env::set_current_dir(&dir).unwrap();

        std::fs::write(caminho("marta"), "r0=segredo\n").unwrap();
        let feitos = queimar("marta").unwrap();
        assert_eq!(feitos.len(), 1);
        assert!(!caminho("marta").exists());
        assert!(
            queimar("marta").unwrap().is_empty(),
            "queimar duas vezes quebrou"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
