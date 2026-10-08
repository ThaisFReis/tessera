//! Prova de soma: Schnorr em base `H`.
//!
//! Se cada `v_j ∈ {0,1}` — garantido pelas disjuntivas de `cds.rs` — e a soma
//! dos votos é `w`, então
//!
//! ```text
//! D = (Σ_j C_j) − w·G = (Σ_j r_j)·H
//! ```
//!
//! é um múltiplo conhecido de `H`. Provar conhecimento de `ρ = Σ_j r_j` com
//! `D = ρ·H` fecha a boa formação da cédula inteira.
//!
//! **Uma prova, não `m` provas.** É por isso que o custo de `votar()` cresce
//! com o número de opções apenas nas disjuntivas.
//!
//! O que isto NÃO prova sozinho: que cada `v_j` é binário. Sem as disjuntivas,
//! `v = (3, −2)` soma 1 e passaria aqui. As duas provas são necessárias e
//! nenhuma das duas é suficiente.

use crate::pedersen;
use crate::ponto;
use ark_bls12_381::{Fr, G1Affine, G1Projective};
use ark_ff::{BigInteger, PrimeField};
use sha2::{Digest, Sha256};

pub const DST_SOMA: &[u8] = b"TESSERA-V1-SOMA";
pub const TAMANHO: usize = 96 + 32;

#[derive(Debug, PartialEq)]
pub enum Erro {
    SemAleatoriedade,
    TamanhoErrado(usize),
    PontoInvalido,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Prova {
    pub a: G1Affine,
    pub z: Fr,
}

/// `D = (Σ C_j) − w·G`. O verificador calcula o mesmo, a partir do que está
/// no ledger — nada aqui é confiado a quem prova.
pub fn alvo(g: &G1Affine, compromissos: &[G1Affine], peso: u64) -> G1Affine {
    let soma = compromissos
        .iter()
        .fold(G1Projective::from(G1Affine::identity()), |acc, c| acc + c);
    (soma - *g * pedersen::escalar(peso)).into()
}

/// `e = H(DST ‖ len(ctx) ‖ ctx ‖ D ‖ a)`, 248 bits.
///
/// Mesma ressalva de `cds.rs`: zerar o byte alto serve para um desafio e não
/// serviria para um fator de aleatoriedade.
pub fn desafio(contexto: &[u8], d: &G1Affine, a: &G1Affine) -> Fr {
    let mut h = Sha256::new();
    h.update(DST_SOMA);
    h.update((contexto.len() as u32).to_be_bytes());
    h.update(contexto);
    h.update(ponto::serializar(d));
    h.update(ponto::serializar(a));
    let mut e: [u8; 32] = h.finalize().into();
    e[0] = 0;
    Fr::from_be_bytes_mod_order(&e)
}

/// Prova conhecimento de `rho` com `D = rho·H`.
pub fn provar(contexto: &[u8], h: &G1Affine, d: &G1Affine, rho: &Fr) -> Result<Prova, Erro> {
    let t = pedersen::acaso_fr().map_err(|_| Erro::SemAleatoriedade)?;
    let a: G1Affine = (*h * t).into();
    let e = desafio(contexto, d, &a);
    Ok(Prova { a, z: t + e * rho })
}

/// `z·H == a + e·D`.
pub fn verificar(contexto: &[u8], h: &G1Affine, d: &G1Affine, p: &Prova) -> bool {
    let e = desafio(contexto, d, &p.a);
    G1Projective::from(*h) * p.z == G1Projective::from(p.a) + G1Projective::from(*d) * e
}

impl Prova {
    pub fn serializar(&self) -> [u8; TAMANHO] {
        let mut b = [0u8; TAMANHO];
        b[..96].copy_from_slice(&ponto::serializar(&self.a));
        b[96..].copy_from_slice(&self.z.into_bigint().to_bytes_be());
        b
    }
}

impl Prova {
    /// 128 bytes, para o verificador reler a prova do ledger.
    pub fn desserializar(b: &[u8]) -> Result<Prova, Erro> {
        if b.len() != TAMANHO {
            return Err(Erro::TamanhoErrado(b.len()));
        }
        let a = ponto::desserializar(&b[..96]).map_err(|_| Erro::PontoInvalido)?;
        Ok(Prova {
            a,
            z: Fr::from_be_bytes_mod_order(&b[96..]),
        })
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::cds;

    const H_HEX: &str = "1462b4b57a7d01e598685e913608bab6990de8cce1c705642c19a6b9ed660fc589758bdebb20c2bdde65a25b35b12206198669dff273abe7b9e6a0b3e636f8a8a41f9c81f6a79d308ec0fe57ef8a073c90e444002ba59807f987cce67d72eccf";
    const CTX: &[u8] = b"proposta-7|alice|soma";

    /// Uma cédula honesta de `m` opções, com um único voto.
    fn cedula(m: usize, escolha: usize) -> (G1Affine, G1Affine, Vec<G1Affine>, Fr) {
        let g = pedersen::gerador();
        let h = ponto::de_hex(H_HEX).unwrap();
        let rs: Vec<Fr> = (0..m).map(|_| pedersen::acaso_fr().unwrap()).collect();
        let cs: Vec<G1Affine> = (0..m)
            .map(|j| {
                let v = if j == escolha { 1u64 } else { 0 };
                pedersen::comprometer(&g, &h, &pedersen::escalar(v), &rs[j])
            })
            .collect();
        let rho = rs.iter().fold(Fr::from(0u64), |a, r| a + r);
        (g, h, cs, rho)
    }

    #[test]
    fn cedula_honesta_passa() {
        for m in [2usize, 3, 8, 16] {
            for escolha in 0..m {
                let (g, h, cs, rho) = cedula(m, escolha);
                let d = alvo(&g, &cs, 1);
                let p = provar(CTX, &h, &d, &rho).unwrap();
                assert!(verificar(CTX, &h, &d, &p), "m={} escolha={}", m, escolha);
            }
        }
    }

    /// **A cédula de duas escolhas.** Vota em duas opções e a soma dá 2, não 1.
    /// `D` sai diferente e o `ρ` que quem vota conhece não o abre.
    #[test]
    fn votar_duas_vezes_na_mesma_cedula_nao_passa() {
        let g = pedersen::gerador();
        let h = ponto::de_hex(H_HEX).unwrap();
        let (r0, r1) = (pedersen::acaso_fr().unwrap(), pedersen::acaso_fr().unwrap());
        let cs = vec![
            pedersen::comprometer(&g, &h, &pedersen::escalar(1), &r0),
            pedersen::comprometer(&g, &h, &pedersen::escalar(1), &r1),
        ];
        let rho = r0 + r1;
        let d = alvo(&g, &cs, 1); // afirma peso 1, mas votou 2
        let p = provar(CTX, &h, &d, &rho).unwrap();
        assert!(!verificar(CTX, &h, &d, &p), "cedula com dois votos passou");
    }

    /// **A cédula em branco.** Não vota em nada e afirma peso 1.
    #[test]
    fn cedula_em_branco_com_peso_um_nao_passa() {
        let g = pedersen::gerador();
        let h = ponto::de_hex(H_HEX).unwrap();
        let (r0, r1) = (pedersen::acaso_fr().unwrap(), pedersen::acaso_fr().unwrap());
        let cs = vec![
            pedersen::comprometer(&g, &h, &pedersen::escalar(0), &r0),
            pedersen::comprometer(&g, &h, &pedersen::escalar(0), &r1),
        ];
        let d = alvo(&g, &cs, 1);
        let p = provar(CTX, &h, &d, &(r0 + r1)).unwrap();
        assert!(!verificar(CTX, &h, &d, &p));
    }

    /// **Esta prova sozinha não basta, e o teste existe para lembrar.**
    ///
    /// `v = (3, −2)` soma 1 e passa aqui. Quem a recusa é a disjuntiva. As
    /// duas provas são necessárias e nenhuma é suficiente.
    #[test]
    fn soma_sozinha_nao_pega_voto_de_peso_tres() {
        let g = pedersen::gerador();
        let h = ponto::de_hex(H_HEX).unwrap();
        let (r0, r1) = (pedersen::acaso_fr().unwrap(), pedersen::acaso_fr().unwrap());
        let menos_dois = -pedersen::escalar(2);
        let cs = vec![
            pedersen::comprometer(&g, &h, &pedersen::escalar(3), &r0),
            pedersen::comprometer(&g, &h, &menos_dois, &r1),
        ];
        let d = alvo(&g, &cs, 1);
        let p = provar(CTX, &h, &d, &(r0 + r1)).unwrap();
        assert!(
            verificar(CTX, &h, &d, &p),
            "o cenario do teste nao se montou"
        );

        // e e a disjuntiva que recusa
        assert!(
            !cds::verificar(
                CTX,
                &g,
                &h,
                &cs[0],
                &cds::provar(CTX, &g, &h, &cs[1], 0, &r1).unwrap()
            ),
            "a disjuntiva de outro compromisso valeu"
        );
        assert_eq!(
            cds::provar(CTX, &g, &h, &cs[0], 3, &r0),
            Err(cds::Erro::VotoForaDoBinario(3))
        );
    }

    #[test]
    fn prova_nao_migra_de_contexto_nem_de_alvo() {
        let (g, h, cs, rho) = cedula(2, 0);
        let d = alvo(&g, &cs, 1);
        let p = provar(CTX, &h, &d, &rho).unwrap();

        assert!(!verificar(b"proposta-7|bob|soma", &h, &d, &p));
        let (_, _, outras, _) = cedula(2, 1);
        assert!(!verificar(CTX, &h, &alvo(&g, &outras, 1), &p));
    }

    #[test]
    fn adulteracao_e_recusada() {
        let (g, h, cs, rho) = cedula(2, 1);
        let d = alvo(&g, &cs, 1);
        let p = provar(CTX, &h, &d, &rho).unwrap();
        assert!(verificar(CTX, &h, &d, &p));

        let mut q = p.clone();
        q.z += pedersen::escalar(1);
        assert!(!verificar(CTX, &h, &d, &q), "z adulterado passou");

        let mut q = p.clone();
        q.a = cs[0];
        assert!(!verificar(CTX, &h, &d, &q), "a adulterado passou");
    }

    #[test]
    fn serializa_em_128_bytes() {
        let (g, h, cs, rho) = cedula(2, 0);
        let p = provar(CTX, &h, &alvo(&g, &cs, 1), &rho).unwrap();
        assert_eq!(p.serializar().len(), 128);
    }
}
