//! A saída, em 72 colunas.
//!
//! Não é estética: é a demo. Num vídeo 1920×1080, 72 colunas ficam legíveis em
//! tela cheia sem forçar fonte pequena. A 100 colunas o jurado não lê o hex, e
//! **o hex é o produto** (UX-CLI §0).
//!
//! Três regras que o resto do arquivo só implementa:
//!
//! - **Status nunca é só cor.** Sempre glyph + palavra: `✓ confere`,
//!   `✗ recusada`. Um vídeo comprimido come saturação.
//! - **O verde marca o que foi verificado, nunca o que foi escolhido.**
//! - **O bloco de hex é sempre 48 × 4.** 96 bytes viram um retângulo sólido,
//!   de largura e altura fixas, em todo comando. É a assinatura visual.

use std::io::IsTerminal;

/// A tela inteira ocupa 72 colunas: 2 de margem e 70 de conteúdo.
pub const TOTAL: usize = 72;
const MARGEM: &str = "  ";
pub const LARGURA: usize = TOTAL - 2;
/// Rótulo + pontos ocupam 20 colunas; o valor começa na 22 da tela.
const COLUNA_VALOR: usize = 20;

pub const ACENTO: &str = "\x1b[38;2;200;242;79m";
pub const TEXTO: &str = "\x1b[38;2;236;239;230m";
pub const SECUNDARIO: &str = "\x1b[38;2;154;160;147m";
pub const APAGADO: &str = "\x1b[38;2;107;112;101m";
pub const RECUSA: &str = "\x1b[38;2;255;107;74m";
pub const RESET: &str = "\x1b[0m";

/// Cor só quando faz sentido: respeita `NO_COLOR` e saída não-tty, porque a
/// demo é gravada e o `| cat` tem de continuar legível.
pub fn tem_cor() -> bool {
    std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal()
}

fn c(cor: &str, s: &str) -> String {
    if tem_cor() {
        format!("{}{}{}", cor, s, RESET)
    } else {
        s.to_string()
    }
}

pub fn titulo(s: &str) {
    println!();
    println!("{}{}", MARGEM, c(TEXTO, &format!("TESSERA · {}", s)));
    regua();
}

/// Título com algo alinhado à direita — só o `status` usa.
pub fn titulo_com(s: &str, direita: &str) {
    println!();
    let esq = format!("TESSERA · {}", s);
    let vago = LARGURA.saturating_sub(esq.chars().count() + direita.chars().count());
    println!("{}{}{}{}", MARGEM, c(TEXTO, &esq), " ".repeat(vago), c(SECUNDARIO, direita));
    regua();
}

pub fn regua() {
    println!("{}{}", MARGEM, c(APAGADO, &"─".repeat(LARGURA)));
}

/// `── rótulo ──────…`, a divisão interna de uma tela.
pub fn secao(s: &str) {
    println!();
    let texto = format!("── {} ", s);
    let resto = LARGURA.saturating_sub(texto.chars().count());
    println!("{}{}", MARGEM, c(APAGADO, &format!("{}{}", texto, "─".repeat(resto))));
    println!();
}

pub fn linha(s: &str) {
    println!("{}{}", MARGEM, s);
}

pub fn branco() {
    println!();
}

/// `  Rótulo ............. valor`
pub fn campo(rotulo: &str, valor: &str) {
    campo_cor(rotulo, valor, TEXTO);
}

/// Valor em acento: **só para o que foi verificado**, nunca para o que foi
/// escolhido. É a regra de cor do UX §8, e ela importa mais no terminal, onde
/// cor é o único recurso visual que existe.
#[allow(dead_code)]
pub fn campo_acento(rotulo: &str, valor: &str) {
    campo_cor(rotulo, valor, ACENTO);
}

fn campo_cor(rotulo: &str, valor: &str, cor: &str) {
    let n = rotulo.chars().count();
    let pontos = COLUNA_VALOR.saturating_sub(n + 1);
    println!(
        "{}{} {} {}",
        MARGEM,
        c(TEXTO, rotulo),
        c(APAGADO, &".".repeat(pontos)),
        c(cor, valor)
    );
}

/// Campo com veredito alinhado à direita da coluna 72.
pub fn campo_veredito(rotulo: &str, valor: &str, ok: bool) {
    let n = rotulo.chars().count();
    // Rótulo vazio é uma continuação da linha de cima: espaço, não pontos.
    let pontos = COLUNA_VALOR.saturating_sub(n + 1);
    if rotulo.is_empty() {
        let vago = LARGURA.saturating_sub(COLUNA_VALOR + 1 + valor.chars().count() + 1);
        println!(
            "{}{}{}{}{}",
            MARGEM,
            " ".repeat(COLUNA_VALOR + 1),
            c(TEXTO, valor),
            " ".repeat(vago),
            c(if ok { ACENTO } else { RECUSA }, if ok { "✓" } else { "✗" })
        );
        return;
    }
    let v = if ok { "✓" } else { "✗" };
    let usado = n + 1 + pontos + 1 + valor.chars().count();
    let vago = LARGURA.saturating_sub(usado + 1);
    println!(
        "{}{} {} {}{}{}",
        MARGEM,
        c(TEXTO, rotulo),
        c(APAGADO, &".".repeat(pontos)),
        c(TEXTO, valor),
        " ".repeat(vago),
        c(if ok { ACENTO } else { RECUSA }, v)
    );
}

pub fn confere(s: &str) {
    println!();
    println!("{}{}", MARGEM, c(ACENTO, &format!("✓ {}", s)));
}

pub fn recusa(s: &str) {
    println!();
    println!("{}{}", MARGEM, c(RECUSA, &format!("✗ {}", s)));
}

pub fn atencao(s: &str) {
    println!();
    println!("{}{}", MARGEM, c(TEXTO, &format!("⚠  {}", s)));
}

pub fn centrado(s: &str) {
    let n = s.chars().count();
    let pad = (LARGURA.saturating_sub(n)) / 2;
    println!("{}{}{}", MARGEM, " ".repeat(pad), c(TEXTO, s));
}

pub fn centrado_grande(s: &str) {
    println!();
    centrado(s);
    println!();
}

/// O próximo comando, pronto para copiar. **Toda tela termina assim.**
pub fn proximo(explicacao: &str, comando: &str) {
    println!();
    println!("{}{}", MARGEM, c(SECUNDARIO, explicacao));
    println!("{}  {}", MARGEM, c(ACENTO, comando));
    println!();
}

/// 96 bytes → 4 linhas de 48 caracteres. Sempre.
pub fn hex(bytes: &[u8]) -> Vec<String> {
    let s: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    s.as_bytes()
        .chunks(48)
        .map(|c| String::from_utf8_lossy(c).to_string())
        .collect()
}

pub fn bloco_hex(bytes: &[u8]) {
    for l in hex(bytes) {
        println!("{}{}", MARGEM, c(SECUNDARIO, &l));
    }
}

/// **O bloco que ganha a demo:** dois compromissos lado a lado, rotulados A e
/// B, sem dizer qual é qual.
///
/// Rotular `A`/`B` em vez de "aprovar"/"rejeitar" é deliberado: assim que você
/// nomeia, a pessoa para de olhar os bytes e começa a ler o rótulo. Anônimo,
/// ela olha os bytes — tenta descobrir, falha, e a falha é a demonstração.
pub fn blocos_lado_a_lado(a: &[u8], b: &[u8], rot_a: &str, rot_b: &str) {
    let (ha, hb) = (hex(a), hex(b));
    // 28 caracteres de hex por coluna cabem em 72 com o espaço entre elas
    let corta = |v: &[String]| -> Vec<String> {
        v.iter().map(|l| l.chars().take(28).collect()).collect()
    };
    let (ca, cb) = (corta(&ha), corta(&hb));
    println!(
        "{}{}{}{}",
        MARGEM,
        c(TEXTO, rot_a),
        " ".repeat(34usize.saturating_sub(rot_a.chars().count())),
        c(TEXTO, rot_b)
    );
    println!();
    for i in 0..4 {
        println!(
            "{}{}    {}",
            MARGEM,
            c(SECUNDARIO, &ca[i]),
            c(SECUNDARIO, &cb[i])
        );
    }
}

/// `CBD5QTEJ…X43W`, para caber na linha sem perder a identificação.
pub fn abreviar(s: &str, inicio: usize, fim: usize) -> String {
    let n = s.chars().count();
    if n <= inicio + fim + 1 {
        return s.to_string();
    }
    let a: String = s.chars().take(inicio).collect();
    let b: String = s.chars().skip(n - fim).collect();
    format!("{}…{}", a, b)
}

#[cfg(test)]
mod testes {
    use super::*;

    /// O retângulo de hex é 48 × 4, em todo comando. Largura e altura fixas
    /// são a assinatura visual do produto.
    #[test]
    fn hex_e_sempre_48_por_4() {
        let b = [0xABu8; 96];
        let l = hex(&b);
        assert_eq!(l.len(), 4);
        for linha in &l {
            assert_eq!(linha.len(), 48);
        }
    }

    /// A tela inteira cabe em 72 colunas, margem incluída. Não é estética: a
    /// 100 colunas o jurado não lê o hex num vídeo em tela cheia, e o hex é o
    /// produto.
    #[test]
    fn nada_passa_de_72_colunas() {
        assert_eq!(MARGEM.len() + LARGURA, TOTAL);
        // a régua
        assert_eq!(MARGEM.len() + "─".repeat(LARGURA).chars().count(), TOTAL);
        // um campo com veredito, que é a linha mais larga que existe
        for rotulo in ["Aberturas públicas", "Agregado refeito"] {
            let valor = "43 confidenciais ≥ 5";
            let n = rotulo.chars().count();
            let pontos = COLUNA_VALOR.saturating_sub(n + 1);
            let usado = n + 1 + pontos + 1 + valor.chars().count();
            assert!(usado + 1 <= LARGURA, "{:?} estourou a largura", rotulo);
        }
    }

    /// Rótulo + pontos ocupam 20 colunas; o valor começa sempre na mesma.
    /// Sem isso o alinhamento por pontos quebra e a tela deixa de ser lida em
    /// diagonal.
    #[test]
    fn o_valor_comeca_sempre_na_mesma_coluna() {
        for rotulo in ["Opções", "Aptos", "Sigilo mínimo", "Mesa", "Transação"] {
            let n = rotulo.chars().count();
            let pontos = COLUNA_VALOR - (n + 1);
            assert_eq!(n + 1 + pontos, COLUNA_VALOR, "rotulo {:?}", rotulo);
        }
    }

    #[test]
    fn abreviacao_mantem_as_pontas() {
        assert_eq!(abreviar("CBD5QTEJPKQGNLFBX43W", 8, 4), "CBD5QTEJ…X43W");
        assert_eq!(abreviar("curto", 8, 4), "curto");
    }

    /// Sem tty e com `NO_COLOR`, nenhum escape ANSI sai — a demo é gravada e
    /// o `| cat` tem de continuar legível.
    #[test]
    fn sem_cor_nao_emite_escape() {
        // Em `cargo test` a saída não é tty, então `tem_cor()` é falso.
        assert!(!tem_cor());
        assert_eq!(c(ACENTO, "oi"), "oi");
    }
}
