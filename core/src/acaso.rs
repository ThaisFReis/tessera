//! Sorteio do fator de aleatoriedade `r` do compromisso de Pedersen.
//!
//! **Este é o único ponto do sistema em que um bug de implementação anula uma
//! garantia information-theoretic.**
//!
//! O compromisso `C = v·G + r·H` é perfeitamente ocultante porque, para todo
//! `v'`, existe exatamente um `r'` que produz o mesmo `C`. A prova depende de
//! `r` ser **uniforme sobre todo o corpo `Fr`**. Se `r` for uniforme apenas
//! sobre um subconjunto `S ⊂ Fr`, então `C` é uniforme sobre `{v·G + s·H}` —
//! um conjunto de tamanho `|S|` que *se desloca* conforme `v`. Dois votos
//! diferentes passam a ter distribuições diferentes, e um adversário sem
//! limite de computação as distingue.
//!
//! Daí a regra que este módulo existe para impor:
//!
//! > Zerar o byte alto é aceitável para um desafio de Fiat-Shamir, e **não é
//! > aceitável para o fator de aleatoriedade**.
//!
//! O desafio só precisa ser imprevisível; `r` precisa ser uniforme. O contrato
//! da sonda zera o byte alto em `challenge_fr` de propósito, e isso está certo
//! lá e estaria errado aqui. São dois usos que parecem o mesmo e não são.

/// O módulo `r` do corpo escalar de BLS12-381, big-endian.
pub const FR_MODULUS: [u8; 32] = [
    0x73, 0xed, 0xa7, 0x53, 0x29, 0x9d, 0x7d, 0x48, 0x33, 0x39, 0xd8, 0x08, 0x09, 0xa1, 0xd8, 0x05,
    0x53, 0xbd, 0xa4, 0x02, 0xff, 0xfe, 0x5b, 0xfe, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x01,
];

#[derive(Debug)]
pub enum Erro {
    /// A fonte de aleatoriedade do sistema falhou. Nunca continue sem `r`:
    /// um `r` improvisado é um voto legível.
    SemAleatoriedade,
}

/// `true` se `a < FR_MODULUS`, comparando big-endian.
pub fn menor_que_r(a: &[u8; 32]) -> bool {
    for i in 0..32 {
        if a[i] != FR_MODULUS[i] {
            return a[i] < FR_MODULUS[i];
        }
    }
    false // igual a r não serve
}

/// Sorteia um escalar **uniforme em `[0, r)`** do CSPRNG do sistema.
///
/// Método: 32 bytes do SO, o bit mais alto zerado (uniforme em `[0, 2^255)`),
/// e rejeição se o valor cair em `[r, 2^255)`. Como `r ≈ 0,906 · 2^255`, a
/// aceitação é de ~90,6% por tentativa.
///
/// Rejeição, e não redução módulo `r`: reduzir 32 bytes módulo `r` enviesaria
/// os valores baixos, e é exatamente o tipo de viés que não aparece em teste
/// nenhum e some com a garantia.
pub fn escalar_aleatorio() -> Result<[u8; 32], Erro> {
    for _ in 0..64 {
        let mut b = [0u8; 32];
        getrandom::getrandom(&mut b).map_err(|_| Erro::SemAleatoriedade)?;
        b[0] &= 0x7f; // uniforme em [0, 2^255)
        if menor_que_r(&b) {
            return Ok(b);
        }
    }
    // 64 rejeições seguidas tem probabilidade ~1e-66. Se acontecer, a fonte
    // está quebrada, e parar é a única resposta segura.
    Err(Erro::SemAleatoriedade)
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::collections::HashSet;

    /// SMOKE B6: o escalar está sempre dentro do corpo.
    #[test]
    fn sempre_menor_que_r() {
        for _ in 0..10_000 {
            let r = escalar_aleatorio().expect("fonte do SO falhou");
            assert!(menor_que_r(&r), "escalar fora de [0, r): {:02x?}", &r[..8]);
            assert_eq!(r[0] & 0x80, 0, "bit alto não foi zerado");
        }
    }

    /// SMOKE B6: nenhuma repetição. Uma colisão em 10 mil sorteios de 255 bits
    /// não é azar, é fonte quebrada.
    #[test]
    fn sem_repeticao() {
        let mut vistos = HashSet::new();
        for _ in 0..10_000 {
            assert!(
                vistos.insert(escalar_aleatorio().unwrap()),
                "escalar repetido: a fonte não é um CSPRNG"
            );
        }
    }

    /// SMOKE B6: a distribuição não é degenerada.
    ///
    /// Não é um teste estatístico sério — é uma rede para pegar a falha
    /// grosseira: fonte constante, contador, timestamp, ou um PRNG semeado
    /// com valor fixo. Qualquer um deles falha aqui.
    #[test]
    fn distribuicao_nao_degenerada() {
        let n = 10_000;
        let mut byte_alto = HashSet::new();
        let mut soma_byte_final: u64 = 0;
        let mut bits_um: u64 = 0;

        for _ in 0..n {
            let r = escalar_aleatorio().unwrap();
            byte_alto.insert(r[0]);
            soma_byte_final += r[31] as u64;
            bits_um += r.iter().map(|b| b.count_ones() as u64).sum::<u64>();
        }

        // o byte alto vai de 0x00 a 0x73: esperamos ver quase todos os valores
        assert!(
            byte_alto.len() > 100,
            "byte alto só assumiu {} valores distintos em {} sorteios",
            byte_alto.len(),
            n
        );

        // média do último byte perto de 127,5
        let media = soma_byte_final as f64 / n as f64;
        assert!(
            (media - 127.5).abs() < 6.0,
            "média do byte final = {:.1}, esperado ~127,5",
            media
        );

        // ~255 bits por sorteio, metade em 1
        let esperado = (n * 255 / 2) as f64;
        let desvio = (bits_um as f64 - esperado).abs() / esperado;
        assert!(
            desvio < 0.02,
            "fração de bits 1 desviou {:.1}% do esperado",
            desvio * 100.0
        );
    }

    /// Documenta, em código executável, POR QUE zerar o byte alto não serve
    /// para `r` — ainda que sirva para um desafio de Fiat-Shamir.
    #[test]
    fn zerar_byte_alto_perderia_o_corpo() {
        // a sonda faz `e[0] = 0` no desafio: cobre [0, 2^248)
        let cobertura_desafio = 248.0_f64;
        // r tem ~255 bits
        let cobertura_r = 254.86_f64;
        let fracao = 2f64.powf(cobertura_desafio - cobertura_r);
        assert!(
            fracao < 0.01,
            "zerar o byte alto cobriria {:.4}% de Fr",
            fracao * 100.0
        );
        // Ou seja: menos de 1% do corpo. Imperceptível para imprevisibilidade,
        // fatal para ocultação perfeita.
    }
}
