//! Prova disjuntiva de Cramer–Damgård–Schoenmakers: `v ∈ {0,1}`.
//!
//! Quem vota precisa provar que o compromisso abre para 0 **ou** para 1, sem
//! revelar qual. Sem isso não há urna: um voto com `v = 1000` encheria a
//! apuração, e um com `v = -1` a esvaziaria.
//!
//! Duas ramificações Schnorr sobre a base `H`, uma real e uma **simulada**,
//! com o desafio dividido por Fiat–Shamir de modo que `e₀ + e₁ = e`. Quem
//! prova sabe abrir exatamente um dos ramos; o outro ele fabrica de trás para
//! frente, escolhendo a resposta antes do desafio. O verificador não distingue
//! os dois, e é isso que esconde o voto.
//!
//! - ramo 0 afirma `C = r·H`        (ou seja `v = 0`)
//! - ramo 1 afirma `C − G = r·H`    (ou seja `v = 1`)

use crate::acaso;
use crate::pedersen;
use crate::ponto;
use ark_bls12_381::{Fr, G1Affine, G1Projective};
use ark_ff::{BigInteger, PrimeField};
use sha2::{Digest, Sha256};

/// Domain separation tag do desafio. Tem de ser idêntico ao do contrato:
/// um DST divergente faz toda prova falhar, e o erro não diz por quê.
pub const DST_DESAFIO: &[u8] = b"TESSERA-V1-FIAT-SHAMIR";

/// 2 pontos (192 B) + 4 escalares (128 B).
pub const TAMANHO: usize = 320;

#[derive(Debug, Clone, PartialEq)]
pub struct Prova {
    pub a0: G1Affine,
    pub a1: G1Affine,
    pub e0: Fr,
    pub z0: Fr,
    pub e1: Fr,
    pub z1: Fr,
}

#[derive(Debug, PartialEq)]
pub enum Erro {
    VotoForaDoBinario(u64),
    SemAleatoriedade,
    BytesInvalidos,
}

fn fr_bytes(f: &Fr) -> [u8; 32] {
    let v = f.into_bigint().to_bytes_be();
    let mut saida = [0u8; 32];
    saida[32 - v.len()..].copy_from_slice(&v);
    saida
}

/// Desafio de Fiat–Shamir, ligado ao compromisso e aos dois anúncios.
///
/// O byte alto é zerado para garantir `e < 2^248 < r`, porque
/// `Bls12381Fr::from_bytes` do contrato **não reduz módulo r**. Aqui isso é
/// seguro: um desafio só precisa ser imprevisível, e 248 bits é muito mais que
/// os 128 necessários. O mesmo truque seria **errado** no fator de
/// aleatoriedade do compromisso, que precisa de uniformidade sobre todo o
/// corpo — ver `acaso.rs`.
pub fn desafio(c: &G1Affine, a0: &G1Affine, a1: &G1Affine) -> Fr {
    let mut h = Sha256::new();
    h.update(DST_DESAFIO);
    h.update(ponto::serializar(c));
    h.update(ponto::serializar(a0));
    h.update(ponto::serializar(a1));
    let mut e: [u8; 32] = h.finalize().into();
    e[0] = 0;
    Fr::from_be_bytes_mod_order(&e)
}

/// Prova que `C = v·G + r·H` com `v ∈ {0,1}`, sem revelar `v`.
pub fn provar(
    g: &G1Affine,
    h: &G1Affine,
    c: &G1Affine,
    v: u64,
    r: &Fr,
) -> Result<Prova, Erro> {
    if v > 1 {
        return Err(Erro::VotoForaDoBinario(v));
    }
    let fr = || pedersen::acaso_fr().map_err(|_| Erro::SemAleatoriedade);

    let t = fr()?;
    let c_menos_g: G1Projective = G1Projective::from(*c) - *g;

    if v == 0 {
        // ramo 0 é o real; o ramo 1 é fabricado de trás para frente
        let a0: G1Affine = (*h * t).into();
        let e1 = fr()?;
        let z1 = fr()?;
        let a1: G1Affine = (*h * z1 - c_menos_g * e1).into();

        let e = desafio(c, &a0, &a1);
        let e0 = e - e1;
        let z0 = t + e0 * r;
        Ok(Prova { a0, a1, e0, z0, e1, z1 })
    } else {
        // ramo 1 é o real
        let a1: G1Affine = (*h * t).into();
        let e0 = fr()?;
        let z0 = fr()?;
        let a0: G1Affine = (*h * z0 - G1Projective::from(*c) * e0).into();

        let e = desafio(c, &a0, &a1);
        let e1 = e - e0;
        let z1 = t + e1 * r;
        Ok(Prova { a0, a1, e0, z0, e1, z1 })
    }
}

/// Verifica a prova. Três igualdades, e todas têm de fechar.
pub fn verificar(g: &G1Affine, h: &G1Affine, c: &G1Affine, p: &Prova) -> bool {
    // 1. os desafios parciais somam o desafio ligado a (C, a0, a1)
    if p.e0 + p.e1 != desafio(c, &p.a0, &p.a1) {
        return false;
    }
    // 2. ramo 0: z0·H == a0 + e0·C
    if G1Projective::from(*h) * p.z0 != G1Projective::from(p.a0) + *c * p.e0 {
        return false;
    }
    // 3. ramo 1: z1·H == a1 + e1·(C − G)
    let c_menos_g = G1Projective::from(*c) - *g;
    if G1Projective::from(*h) * p.z1 != G1Projective::from(p.a1) + c_menos_g * p.e1 {
        return false;
    }
    true
}

impl Prova {
    /// 320 bytes, na ordem que o contrato vai ler.
    pub fn serializar(&self) -> [u8; TAMANHO] {
        let mut b = [0u8; TAMANHO];
        b[0..96].copy_from_slice(&ponto::serializar(&self.a0));
        b[96..192].copy_from_slice(&ponto::serializar(&self.a1));
        b[192..224].copy_from_slice(&fr_bytes(&self.e0));
        b[224..256].copy_from_slice(&fr_bytes(&self.z0));
        b[256..288].copy_from_slice(&fr_bytes(&self.e1));
        b[288..320].copy_from_slice(&fr_bytes(&self.z1));
        b
    }

    pub fn desserializar(b: &[u8]) -> Result<Prova, Erro> {
        if b.len() != TAMANHO {
            return Err(Erro::BytesInvalidos);
        }
        Ok(Prova {
            a0: ponto::desserializar(&b[0..96]).map_err(|_| Erro::BytesInvalidos)?,
            a1: ponto::desserializar(&b[96..192]).map_err(|_| Erro::BytesInvalidos)?,
            e0: Fr::from_be_bytes_mod_order(&b[192..224]),
            z0: Fr::from_be_bytes_mod_order(&b[224..256]),
            e1: Fr::from_be_bytes_mod_order(&b[256..288]),
            z1: Fr::from_be_bytes_mod_order(&b[288..320]),
        })
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::ponto;

    const H_HEX: &str = "1462b4b57a7d01e598685e913608bab6990de8cce1c705642c19a6b9ed660fc589758bdebb20c2bdde65a25b35b12206198669dff273abe7b9e6a0b3e636f8a8a41f9c81f6a79d308ec0fe57ef8a073c90e444002ba59807f987cce67d72eccf";

    fn cenario(v: u64) -> (G1Affine, G1Affine, G1Affine, Fr, Prova) {
        let g = pedersen::gerador();
        let h = ponto::de_hex(H_HEX).unwrap();
        let r = pedersen::acaso_fr().unwrap();
        let c = pedersen::comprometer(&g, &h, &pedersen::escalar(v), &r);
        let p = provar(&g, &h, &c, v, &r).unwrap();
        (g, h, c, r, p)
    }

    #[test]
    fn aceita_voto_zero_e_voto_um() {
        for v in [0u64, 1] {
            let (g, h, c, _, p) = cenario(v);
            assert!(verificar(&g, &h, &c, &p), "prova honesta de v={} falhou", v);
        }
    }

    /// Um voto fora de {0,1} nem chega a produzir prova.
    #[test]
    fn recusa_voto_fora_do_binario() {
        let g = pedersen::gerador();
        let h = ponto::de_hex(H_HEX).unwrap();
        let r = pedersen::acaso_fr().unwrap();
        let c = pedersen::comprometer(&g, &h, &pedersen::escalar(2), &r);
        assert_eq!(provar(&g, &h, &c, 2, &r), Err(Erro::VotoForaDoBinario(2)));
    }

    /// E se alguém TENTAR forjar uma prova para v=2 usando o maquinário
    /// honesto de um dos ramos, ela não verifica. Este é o ataque real:
    /// encher a urna com um voto de peso 1000.
    #[test]
    fn prova_forjada_para_v_dois_nao_verifica() {
        let g = pedersen::gerador();
        let h = ponto::de_hex(H_HEX).unwrap();
        let r = pedersen::acaso_fr().unwrap();
        let c2 = pedersen::comprometer(&g, &h, &pedersen::escalar(2), &r);

        // o atacante finge que v=0 e tenta provar o ramo 0 com o r que ele tem
        let t = pedersen::acaso_fr().unwrap();
        let a0: G1Affine = (h * t).into();
        let e1 = pedersen::acaso_fr().unwrap();
        let z1 = pedersen::acaso_fr().unwrap();
        let a1: G1Affine = (h * z1 - (G1Projective::from(c2) - g) * e1).into();
        let e = desafio(&c2, &a0, &a1);
        let forjada = Prova { a0, a1, e0: e - e1, z0: t + (e - e1) * r, e1, z1 };

        assert!(
            !verificar(&g, &h, &c2, &forjada),
            "prova forjada para v=2 passou: a urna pode ser rechada"
        );
    }

    #[test]
    fn recusa_adulteracao() {
        let (g, h, c, _, p) = cenario(1);
        let um = pedersen::escalar(1);

        let mut q = p.clone();
        q.e0 = q.e0 + um;
        assert!(!verificar(&g, &h, &c, &q), "e0 adulterado passou");

        let mut q = p.clone();
        q.z0 = q.z0 + um;
        assert!(!verificar(&g, &h, &c, &q), "z0 adulterado passou");

        let mut q = p.clone();
        q.z1 = q.z1 + um;
        assert!(!verificar(&g, &h, &c, &q), "z1 adulterado passou");

        let mut q = p.clone();
        core::mem::swap(&mut q.a0, &mut q.a1);
        assert!(!verificar(&g, &h, &c, &q), "ramos trocados passaram");
    }

    /// A prova está presa ao SEU compromisso: não dá para reusar a de outra
    /// pessoa, que é o que permitiria votar sem saber abrir nada.
    #[test]
    fn prova_nao_migra_de_compromisso() {
        let (g, h, _, _, p) = cenario(1);
        let outro_r = pedersen::acaso_fr().unwrap();
        let outro_c = pedersen::comprometer(&g, &h, &pedersen::escalar(1), &outro_r);
        assert!(!verificar(&g, &h, &outro_c, &p), "prova migrou de compromisso");
    }

    #[test]
    fn ida_e_volta_320_bytes() {
        let (_, _, _, _, p) = cenario(0);
        let b = p.serializar();
        assert_eq!(b.len(), 320);
        assert_eq!(Prova::desserializar(&b).unwrap(), p);
    }

    /// Duas provas do mesmo voto têm de ser diferentes: se fossem iguais, o
    /// voto vazaria por comparação.
    #[test]
    fn provas_do_mesmo_voto_diferem() {
        let (_, _, _, _, p1) = cenario(1);
        let (_, _, _, _, p2) = cenario(1);
        assert_ne!(p1.serializar(), p2.serializar());
    }
}

/// Emite um vetor de prova para o contrato verificar. É o cruzamento
/// provador-nativo ↔ verificador-Wasm (smoke B3) para o caminho CDS.
#[cfg(test)]
mod vetor {
    use super::*;
    use crate::ponto;

    const H_HEX: &str = "1462b4b57a7d01e598685e913608bab6990de8cce1c705642c19a6b9ed660fc589758bdebb20c2bdde65a25b35b12206198669dff273abe7b9e6a0b3e636f8a8a41f9c81f6a79d308ec0fe57ef8a073c90e444002ba59807f987cce67d72eccf";

    #[test]
    fn emitir_vetor_cds_para_o_contrato() {
        let g = pedersen::gerador();
        let h = ponto::de_hex(H_HEX).unwrap();
        // r fixo, para o vetor ser reproduzível
        let r = pedersen::escalar(31337);
        let v = 1u64;
        let c = pedersen::comprometer(&g, &h, &pedersen::escalar(v), &r);
        let p = provar(&g, &h, &c, v, &r).unwrap();
        assert!(verificar(&g, &h, &c, &p));

        let dec = |f: &Fr| pedersen::fr_para_decimal(f);
        println!("\n# vetor CDS (v=1, r=31337) — gerado pelo core, para o contrato");
        println!("CDS_C={}", ponto::para_hex(&c));
        println!("CDS_A0={}", ponto::para_hex(&p.a0));
        println!("CDS_A1={}", ponto::para_hex(&p.a1));
        println!("CDS_E0={}", dec(&p.e0));
        println!("CDS_Z0={}", dec(&p.z0));
        println!("CDS_E1={}", dec(&p.e1));
        println!("CDS_Z1={}", dec(&p.z1));
    }
}
