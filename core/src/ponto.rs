//! Serialização de pontos G1, no formato exato que o host do Soroban espera.
//!
//! **96 bytes, não comprimido: `be_bytes(X) || be_bytes(Y)`.**
//!
//! Arkworks serializa em little-endian e com bits de flag no byte alto. O host
//! do Soroban não. Esta conversão é feita na mão de propósito: é o ponto exato
//! onde provador off-chain e verificador on-chain divergiriam em silêncio, e
//! o teste contra os vetores da testnet existe para travá-la.

use ark_bls12_381::{Fq, G1Affine};
use ark_ec::AffineRepr;
use ark_ff::{BigInteger, PrimeField};

pub const TAMANHO: usize = 96;

/// **O ponto no infinito, como o host do Soroban o codifica.**
///
/// Não é 96 bytes de zero — isso o host recusa com "point not on curve". É o
/// bit de flag de infinito do formato zcash (`0x40` no byte alto), com o resto
/// zerado. Descoberto empiricamente ao somar cada candidato ao gerador e ver
/// qual devolvia o gerador: só `0x40` passou; zeros e `0xc0` abortaram.
///
/// É exatamente a classe de divergência silenciosa que este arquivo existe
/// para fechar: o acumulador de uma proposta sem votos é o infinito, então
/// errar aqui quebraria a primeira cédula de toda votação.
pub const INFINITO: [u8; TAMANHO] = {
    let mut b = [0u8; TAMANHO];
    b[0] = 0x40;
    b
};

#[derive(Debug, PartialEq)]
pub enum Erro {
    TamanhoErrado(usize),
    ForaDaCurva,
    ForaDoSubgrupo,
    CoordenadaInvalida,
}

/// Ponto G1 → 96 bytes, como o host lê.
pub fn serializar(p: &G1Affine) -> [u8; TAMANHO] {
    let mut saida = [0u8; TAMANHO];
    if p.is_zero() {
        return INFINITO;
    }
    let (x, y) = (p.x().unwrap(), p.y().unwrap());
    saida[..48].copy_from_slice(&x.into_bigint().to_bytes_be());
    saida[48..].copy_from_slice(&y.into_bigint().to_bytes_be());
    saida
}

/// 96 bytes → ponto G1, **validado**.
///
/// On-curve e in-subgroup, sempre. O host não faz essa checagem dentro da
/// aritmética (medido na sonda 10: `g1_add` custa 15% de uma checagem de
/// subgrupo, logo não pode contê-la), e pular a validação abre ataque de
/// subgrupo pequeno.
pub fn desserializar(b: &[u8]) -> Result<G1Affine, Erro> {
    if b.len() != TAMANHO {
        return Err(Erro::TamanhoErrado(b.len()));
    }
    if b == INFINITO {
        return Ok(G1Affine::identity());
    }
    let x = Fq::from_be_bytes_mod_order(&b[..48]);
    let y = Fq::from_be_bytes_mod_order(&b[48..]);

    // from_be_bytes_mod_order reduz; se os bytes não eram canônicos, a
    // ida-e-volta não fecha e o ponto não é o que o remetente escreveu.
    let p = G1Affine::new_unchecked(x, y);
    if serializar(&p) != b[..TAMANHO] {
        return Err(Erro::CoordenadaInvalida);
    }
    if !p.is_on_curve() {
        return Err(Erro::ForaDaCurva);
    }
    if !p.is_in_correct_subgroup_assuming_on_curve() {
        return Err(Erro::ForaDoSubgrupo);
    }
    Ok(p)
}

/// Hex → ponto, para ler vetores de teste e argumentos de CLI.
pub fn de_hex(s: &str) -> Result<G1Affine, Erro> {
    let s = s.trim();
    if s.len() != TAMANHO * 2 {
        return Err(Erro::TamanhoErrado(s.len() / 2));
    }
    let mut b = [0u8; TAMANHO];
    for i in 0..TAMANHO {
        b[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).map_err(|_| Erro::CoordenadaInvalida)?;
    }
    desserializar(&b)
}

/// Ponto → hex minúsculo, o formato que a `stellar contract invoke` aceita.
pub fn para_hex(p: &G1Affine) -> String {
    serializar(p).iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod testes {
    use super::*;
    use ark_ec::PrimeGroup;
    use ark_bls12_381::G1Projective;

    /// O gerador canônico tem de serializar exatamente como `G_HEX` do
    /// `vetores.env`, que o host da testnet aceitou na sonda 1.
    #[test]
    fn gerador_bate_com_o_vetor_da_testnet() {
        const G_HEX: &str = "17f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb08b3f481e3aaa0f1a09e30ed741d8ae4fcf5e095d5d00af600db18cb2c04b3edd03cc744a2888ae40caa232946c5e7e1";
        let g: G1Affine = G1Projective::generator().into();
        assert_eq!(
            para_hex(&g),
            G_HEX,
            "o gerador nativo NAO serializa como o host espera"
        );
    }

    /// O infinito tem de ida-e-voltar na codificação do host, não em zeros.
    #[test]
    fn infinito_e_a_flag_do_host_nao_zeros() {
        let id = G1Affine::identity();
        assert_eq!(serializar(&id)[0], 0x40);
        assert!(serializar(&id)[1..].iter().all(|&b| b == 0));
        assert_eq!(desserializar(&INFINITO).unwrap(), id);
        assert!(
            desserializar(&[0u8; TAMANHO]).is_err(),
            "96 zeros foram aceitos como infinito; o host os recusa"
        );
    }

    #[test]
    fn ida_e_volta() {
        let g: G1Affine = G1Projective::generator().into();
        assert_eq!(desserializar(&serializar(&g)).unwrap(), g);
        let id = G1Affine::identity();
        assert_eq!(desserializar(&serializar(&id)).unwrap(), id);
    }

    #[test]
    fn recusa_lixo() {
        assert_eq!(desserializar(&[0u8; 50]), Err(Erro::TamanhoErrado(50)));
        let mut b = [0xAAu8; 96];
        b[0] = 0x01;
        assert!(desserializar(&b).is_err(), "aceitou bytes que nao sao ponto");
    }
}
