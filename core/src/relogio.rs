//! A fechadura de tempo: o retentor da abertura que **não é gente**.
//!
//! O problema que este módulo existe para resolver está em `docs/RELOGIO.md`, e
//! é uma impossibilidade aparente: para o placar não existir durante a votação,
//! alguém tem de estar retendo a abertura; para ele surgir sozinho ao final,
//! ninguém pode estar retendo. São a mesma frase negada — a menos que o
//! retentor seja o tempo.
//!
//! A baliza de limiar `quicknet` da drand assina, a cada 3 segundos, o número
//! da rodada corrente. A assinatura de uma rodada **futura** ainda não existe, e
//! vai existir publicamente no instante daquela rodada. Então dá para cifrar o
//! fator `r` para uma rodada futura: a chave é a assinatura que ainda não saiu.
//! Quando ela sai, sai para todo mundo ao mesmo tempo.
//!
//! **Este módulo não fala com a rede** (SPEC §3). A assinatura chega como
//! argumento e é *conferida* aqui; quem a busca no relé é `app/src/rede.ts` ou a
//! `cli`. Um crate de matemática que faz HTTP não é testável sem rede, e o
//! portão tem de rodar offline.
//!
//! Pôr pareamento aqui não engorda o Wasm do contrato: `tessera.wasm` tem
//! 37.568 bytes com e sem este módulo, medido — e o motivo não é o ligador, é
//! que `tessera-core` é **dev-dependency** do contrato, usada pelos testes para
//! cruzar provador e verificador. O contrato aliás *não poderia* decifrar: o
//! host só expõe `pairing_check`, sem pareamento com saída de valor, e a
//! decifragem precisa do **valor**. Ele não precisa — quem recusa um total que
//! mente é o compromisso de Pedersen, não a fechadura.
//!
//! O que o contrato precisa é só da **aritmética da rodada**, que ele repete em
//! três linhas em vez de arrastar este crate para dentro do Wasm. Repetir é
//! arriscado, então `contrato` cruza as duas com `core` a cada `cargo test`:
//! `a_rodada_do_contrato_bate_com_a_do_core`.
//!
//! E a conferência não é opcional por construção: `decifrar` só aceita uma
//! [`Chave`], e o único jeito de obter uma `Chave` é passar por [`conferir`].
//! Um relé que minta é recusado **antes** de qualquer decifragem (INV-24).

use crate::acaso;
use ark_bls12_381::{Bls12_381, Fr, G1Affine, G2Affine, G2Projective};
use ark_ec::hashing::curve_maps::wb::WBMap;
use ark_ec::hashing::map_to_curve_hasher::MapToCurveBasedHasher;
use ark_ec::hashing::HashToCurve;
use ark_ec::pairing::Pairing;
use ark_ec::{AffineRepr, CurveGroup, PrimeGroup};
use ark_ff::field_hashers::DefaultFieldHasher;
use ark_ff::{BigInteger, PrimeField};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use sha2::{Digest, Sha256};

/// Constantes da cadeia `quicknet` da drand. Medidas em 2026-10-07; a tabela
/// com data está em `docs/SOURCES.md`, e são **formato congelado** (SPEC §5).
pub const GENESE: u64 = 1_692_803_367;
pub const PERIODO: u64 = 3;

/// A chave pública da cadeia: ponto de G2 comprimido, 96 bytes.
pub const CHAVE_CADEIA: [u8; 96] = [
    0x83, 0xcf, 0x0f, 0x28, 0x96, 0xad, 0xee, 0x7e, 0xb8, 0xb5, 0xf0, 0x1f, 0xca, 0xd3, 0x91, 0x22,
    0x12, 0xc4, 0x37, 0xe0, 0x07, 0x3e, 0x91, 0x1f, 0xb9, 0x00, 0x22, 0xd3, 0xe7, 0x60, 0x18, 0x3c,
    0x8c, 0x4b, 0x45, 0x0b, 0x6a, 0x0a, 0x6c, 0x3a, 0xc6, 0xa5, 0x77, 0x6a, 0x2d, 0x10, 0x64, 0x51,
    0x0d, 0x1f, 0xec, 0x75, 0x8c, 0x92, 0x1c, 0xc2, 0x2b, 0x0e, 0x17, 0xe6, 0x3a, 0xaf, 0x4b, 0xcb,
    0x5e, 0xd6, 0x63, 0x04, 0xde, 0x9c, 0xf8, 0x09, 0xbd, 0x27, 0x4c, 0xa7, 0x3b, 0xab, 0x4a, 0xf5,
    0xa6, 0xe9, 0xc7, 0x6a, 0x4b, 0xc0, 0x9e, 0x76, 0xea, 0xe8, 0x99, 0x1e, 0xf5, 0xec, 0xe4, 0x5a,
];

/// O DST do esquema `bls-unchained-g1-rfc9380`. É **deles**, não nosso: mudar
/// uma letra faz toda assinatura falhar sem dizer por quê.
const DST_BALIZA: &[u8] = b"BLS_SIG_BLS12381G1_XMD:SHA-256_SSWU_RO_NUL_";

/// Os nossos, para os três hashes do criptograma (SPEC §5).
const DST_SIGMA: &[u8] = b"TESSERA-V1-RELOGIO-SIGMA";
const DST_MASCARA: &[u8] = b"TESSERA-V1-RELOGIO-MASCARA";
const DST_PAD: &[u8] = b"TESSERA-V1-RELOGIO-PAD";

/// Tamanho do criptograma, por opção confidencial. Formato congelado (SPEC §5):
/// `U` (96 B, G2 comprimido) ‖ `V` (32 B) ‖ `W` (32 B).
pub const TAMANHO: usize = 160;

#[derive(Debug, PartialEq)]
pub enum Erro {
    /// A assinatura não é a da rodada pedida, ou não é um ponto de G1. Nunca
    /// decifre depois disto: é um relé mentindo.
    AssinaturaNaoConfere,
    /// O criptograma não tem 160 bytes, ou o `U` não é um ponto de G2.
    CriptogramaMalFormado,
    /// A decifragem não fecha com o próprio criptograma. Ou a chave é de outra
    /// rodada, ou alguém mexeu nos bytes.
    NaoFecha,
    /// Instante anterior à gênese da baliza: não existe rodada.
    AntesDaGenese,
    SemAleatoriedade,
}

impl From<acaso::Erro> for Erro {
    fn from(_: acaso::Erro) -> Self {
        Erro::SemAleatoriedade
    }
}

/// A assinatura de uma rodada, **já conferida**. Não há construtor público
/// além de [`conferir`], e é isso que faz a INV-24 ser estrutural: não existe
/// caminho de tipos que decifre sem passar pela conferência.
///
/// `Debug` é inofensivo aqui: a assinatura da baliza é pública por construção —
/// ela é publicada para todo mundo no instante da rodada. Não é segredo, é
/// relógio.
#[derive(Debug)]
pub struct Chave(G1Affine);

/// A rodada da baliza que vence no instante `t` (segundos unix).
pub fn rodada(t: u64) -> Result<u64, Erro> {
    if t < GENESE {
        return Err(Erro::AntesDaGenese);
    }
    Ok((t - GENESE) / PERIODO + 1)
}

/// O instante em que a rodada vence. Inverso exato de [`rodada`].
pub fn instante(rodada: u64) -> u64 {
    GENESE + (rodada - 1) * PERIODO
}

/// A chave pública da cadeia, como ponto de G2.
pub fn chave_da_cadeia() -> G2Affine {
    G2Affine::deserialize_compressed(&CHAVE_CADEIA[..]).expect("a constante da cadeia é congelada")
}

/// A identidade da rodada: o ponto de G1 que a baliza assina.
///
/// A mensagem é `sha256(rodada em 8 bytes big-endian)` — **medido**, porque a
/// rodada crua não confere, e nenhuma prosa que lemos dizia qual das duas era.
pub fn identidade(rodada: u64) -> G1Affine {
    let msg = Sha256::digest(rodada.to_be_bytes());
    MapToCurveBasedHasher::<
        ark_ec::short_weierstrass::Projective<ark_bls12_381::g1::Config>,
        DefaultFieldHasher<Sha256, 128>,
        WBMap<ark_bls12_381::g1::Config>,
    >::new(DST_BALIZA)
    .expect("o DST da baliza é congelado")
    .hash(&msg)
    .expect("hash-to-curve não falha para entrada de 32 bytes")
}

/// Confere a assinatura contra a chave pública da cadeia, e só então devolve a
/// [`Chave`] que decifra. **INV-24.**
pub fn conferir(rodada_pedida: u64, assinatura: &[u8]) -> Result<Chave, Erro> {
    let s = G1Affine::deserialize_compressed(assinatura).map_err(|_| Erro::AssinaturaNaoConfere)?;
    let q = identidade(rodada_pedida);
    // `e(σ, g₂) == e(H₁(rodada), P)`. Com σ = x·H₁(rodada) e P = x·g₂, os dois
    // lados são `e(H₁, g₂)^x`.
    if Bls12_381::pairing(s, G2Affine::generator()) != Bls12_381::pairing(q, chave_da_cadeia()) {
        return Err(Erro::AssinaturaNaoConfere);
    }
    Ok(Chave(s))
}

/// Cifra o fator para a rodada. O `sigma` vem do CSPRNG do sistema.
pub fn cifrar(r: &Fr, rodada: u64) -> Result<[u8; TAMANHO], Erro> {
    cifrar_com(r, rodada, &acaso::escalar_aleatorio()?)
}

/// O mesmo, com o `sigma` dado — para teste e para vetor congelado.
///
/// O esquema é o do `tlock`: cifra baseada em identidade de Boneh–Franklin, com
/// a rodada no lugar da identidade, derandomizada à Fujisaki–Okamoto. O `s` sai
/// de `H(sigma ‖ m)` e não do sorteio, e é isso que deixa a decifragem
/// **conferir o próprio criptograma** no fim — sem essa checagem, mexer num byte
/// devolveria um fator aleatório em silêncio, e o total não fecharia sem dizer
/// por quê.
///
/// `s` só precisa ser imprevisível, não uniforme sobre todo o corpo: é a mesma
/// distinção que `acaso.rs` faz entre um desafio de Fiat-Shamir e o fator `r`.
/// Quem precisa de uniformidade é o `r`, e ele vem de `pedersen::acaso_fr`.
pub fn cifrar_com(r: &Fr, rodada: u64, sigma: &[u8; 32]) -> Result<[u8; TAMANHO], Erro> {
    let m = em_bytes(r);
    let s = escalar_de(DST_SIGMA, &[sigma.as_slice(), &m]);

    let u = (G2Projective::generator() * s).into_affine();
    let gid = Bls12_381::pairing(identidade(rodada), (chave_da_cadeia() * s).into_affine());

    let mut saida = [0u8; TAMANHO];
    u.serialize_compressed(&mut saida[..96])
        .expect("G2 comprimido tem 96 bytes");
    let mascara = hash32(DST_MASCARA, &[&alvo_em_bytes(&gid)]);
    let pad = hash32(DST_PAD, &[sigma.as_slice()]);
    for i in 0..32 {
        saida[96 + i] = sigma[i] ^ mascara[i];
        saida[128 + i] = m[i] ^ pad[i];
    }
    Ok(saida)
}

/// Decifra. Só aceita uma [`Chave`], que só sai de [`conferir`].
pub fn decifrar(c: &[u8], chave: &Chave) -> Result<Fr, Erro> {
    if c.len() != TAMANHO {
        return Err(Erro::CriptogramaMalFormado);
    }
    let u = G2Affine::deserialize_compressed(&c[..96]).map_err(|_| Erro::CriptogramaMalFormado)?;

    // `e(σ, U) = e(x·H₁, s·g₂) = e(H₁, P)^s`, que é exatamente o que cifrou.
    let gid = Bls12_381::pairing(chave.0, u);
    let mascara = hash32(DST_MASCARA, &[&alvo_em_bytes(&gid)]);
    let mut sigma = [0u8; 32];
    for i in 0..32 {
        sigma[i] = c[96 + i] ^ mascara[i];
    }
    let pad = hash32(DST_PAD, &[sigma.as_slice()]);
    let mut m = [0u8; 32];
    for i in 0..32 {
        m[i] = c[128 + i] ^ pad[i];
    }

    // A checagem de Fujisaki–Okamoto: refaz o `s` e confere que ele produz o
    // mesmo `U`. Chave de outra rodada e byte mexido caem aqui.
    let s = escalar_de(DST_SIGMA, &[sigma.as_slice(), &m]);
    if (G2Projective::generator() * s).into_affine() != u {
        return Err(Erro::NaoFecha);
    }
    // E o `m` tem de ser um escalar canônico: senão `from_be_bytes_mod_order`
    // reduziria em silêncio e devolveria um fator que ninguém cifrou.
    if !acaso::menor_que_r(&m) {
        return Err(Erro::NaoFecha);
    }
    Ok(Fr::from_be_bytes_mod_order(&m))
}

/// A chave pública da cadeia nos 192 bytes que o host do Soroban lê.
///
/// A baliza publica comprimido (96 bytes em G2, 48 em G1) e o host só lê não
/// comprimido. Descomprimir exige raiz quadrada em `Fp`, que o host não tem —
/// então quem converte é o cliente, e é por isso que estas duas funções existem
/// aqui e não no contrato.
pub fn chave_da_cadeia_para_host() -> [u8; crate::ponto::TAMANHO_G2] {
    crate::ponto::serializar_g2(&chave_da_cadeia())
}

/// O gerador de G2, idem. O contrato precisa dele para o lado esquerdo de
/// `e(σ, g₂) == e(H₁, P)`.
pub fn gerador_g2_para_host() -> [u8; crate::ponto::TAMANHO_G2] {
    crate::ponto::serializar_g2(&G2Affine::generator())
}

/// A assinatura comprimida da baliza nos 96 bytes que o host lê.
///
/// Valida de passagem: um ponto fora da curva ou fora do subgrupo é recusado
/// aqui, antes de chegar ao contrato.
pub fn assinatura_para_host(assinatura: &[u8]) -> Result<[u8; crate::ponto::TAMANHO], Erro> {
    let s = G1Affine::deserialize_compressed(assinatura).map_err(|_| Erro::AssinaturaNaoConfere)?;
    Ok(crate::ponto::serializar(&s))
}

fn em_bytes(r: &Fr) -> [u8; 32] {
    let v = r.into_bigint().to_bytes_be();
    let mut saida = [0u8; 32];
    saida[32 - v.len()..].copy_from_slice(&v);
    saida
}

fn alvo_em_bytes(gid: &ark_ec::pairing::PairingOutput<Bls12_381>) -> Vec<u8> {
    let mut v = Vec::new();
    gid.serialize_uncompressed(&mut v)
        .expect("serializar em memória não falha");
    v
}

fn hash32(dst: &[u8], partes: &[&[u8]]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(dst);
    for p in partes {
        h.update(p);
    }
    h.finalize().into()
}

fn escalar_de(dst: &[u8], partes: &[&[u8]]) -> Fr {
    Fr::from_be_bytes_mod_order(&hash32(dst, partes))
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Rodada 6.000.000 da `quicknet`. Vetor congelado: se a baliza trocar de
    /// esquema, de DST ou de chave, isto quebra antes de qualquer cédula.
    const RODADA: u64 = 6_000_000;
    const ASSINATURA: [u8; 48] = hex48(
        "848a0288a7102249bd6a274f65414ec8ca5b12c5e6f13a322e315ab734107784cd9b7ed4ebae73980cd72730ac6eb9f7",
    );

    const fn hex48(s: &str) -> [u8; 48] {
        let b = s.as_bytes();
        let mut saida = [0u8; 48];
        let mut i = 0;
        while i < 48 {
            saida[i] = nibble(b[2 * i]) * 16 + nibble(b[2 * i + 1]);
            i += 1;
        }
        saida
    }

    const fn nibble(c: u8) -> u8 {
        match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            _ => panic!("hex inválido"),
        }
    }

    /// **INV-24.** A assinatura se autovalida: não precisamos confiar no relé.
    #[test]
    fn a_assinatura_da_baliza_confere() {
        assert!(conferir(RODADA, &ASSINATURA).is_ok());
    }

    #[test]
    fn a_assinatura_mexida_e_recusada() {
        let mut m = ASSINATURA;
        m[47] ^= 1;
        assert_eq!(
            conferir(RODADA, &m).unwrap_err(),
            Erro::AssinaturaNaoConfere
        );
    }

    /// Uma assinatura boa, apresentada como sendo de outra rodada, é recusada —
    /// senão um relé adiantaria o relógio da urna.
    #[test]
    fn a_assinatura_de_outra_rodada_e_recusada() {
        assert_eq!(
            conferir(RODADA + 1, &ASSINATURA).unwrap_err(),
            Erro::AssinaturaNaoConfere
        );
    }

    #[test]
    fn a_rodada_vem_do_instante_e_volta() {
        // Medido em 2026-10-07: para este instante a fórmula dá esta rodada, e
        // o relé publicava a anterior — a rodada vence exatamente no instante.
        assert_eq!(rodada(1_791_421_407).unwrap(), 32_872_681);
        assert_eq!(instante(32_872_681), 1_791_421_407);
        assert_eq!(instante(RODADA), 1_710_803_364);
        assert_eq!(rodada(instante(RODADA)).unwrap(), RODADA);
    }

    #[test]
    fn antes_da_genese_nao_existe_rodada() {
        assert_eq!(rodada(0).unwrap_err(), Erro::AntesDaGenese);
    }

    #[test]
    fn cifrar_e_decifrar_devolve_o_fator() {
        let r = Fr::from(123_456_789u64);
        let c = cifrar_com(&r, RODADA, &[7u8; 32]).unwrap();
        assert_eq!(c.len(), TAMANHO);
        let chave = conferir(RODADA, &ASSINATURA).unwrap();
        assert_eq!(decifrar(&c, &chave).unwrap(), r);
    }

    /// O fator cifrado para outra rodada não abre com esta chave. É o que faz a
    /// fechadura ser de tempo e não de senha.
    #[test]
    fn o_criptograma_de_outra_rodada_nao_abre() {
        let r = Fr::from(1u64);
        let c = cifrar_com(&r, RODADA + 1, &[7u8; 32]).unwrap();
        let chave = conferir(RODADA, &ASSINATURA).unwrap();
        assert_eq!(decifrar(&c, &chave).unwrap_err(), Erro::NaoFecha);
    }

    /// Mexer num byte do criptograma não devolve lixo em silêncio: a checagem
    /// de consistência o pega. Sem ela, uma cédula sabotada viraria um fator
    /// aleatório, e o total simplesmente não fecharia sem dizer por quê.
    #[test]
    fn criptograma_mexido_nao_devolve_lixo_em_silencio() {
        let c = cifrar_com(&Fr::from(42u64), RODADA, &[7u8; 32]).unwrap();
        let chave = conferir(RODADA, &ASSINATURA).unwrap();
        for i in [0, 95, 96, 128, 159] {
            let mut m = c;
            m[i] ^= 1;
            // Mexer no `U` pode nem desserializar — o que também é recusa. O
            // que o teste não aceita é um `Ok` com fator trocado.
            let e = decifrar(&m, &chave).unwrap_err();
            assert!(
                e == Erro::NaoFecha || e == Erro::CriptogramaMalFormado,
                "byte {i} devolveu {e:?}"
            );
        }
    }

    #[test]
    fn criptograma_de_tamanho_errado_e_recusado() {
        let chave = conferir(RODADA, &ASSINATURA).unwrap();
        assert_eq!(
            decifrar(&[0u8; 159], &chave).unwrap_err(),
            Erro::CriptogramaMalFormado
        );
    }

    /// O sorteio não é determinístico: dois criptogramas do mesmo fator diferem.
    #[test]
    fn cifrar_duas_vezes_nao_repete() {
        let r = Fr::from(9u64);
        assert_ne!(cifrar(&r, RODADA).unwrap(), cifrar(&r, RODADA).unwrap());
    }

    /// **SPEC §3.** Este módulo não fala com a rede. Como toda ausência, só dá
    /// para provar olhando a fonte — se alguém puser um cliente HTTP aqui, isto
    /// quebra.
    #[test]
    fn o_relogio_nao_fala_com_a_rede() {
        let fonte = include_str!("relogio.rs");
        // Montados em pedaços de propósito: `include_str!` inclui **este**
        // bloco, e um literal inteiro aqui faria o teste se encontrar a si
        // mesmo e falhar sempre.
        let proibidos = [
            ["ht", "tp"].concat(),
            ["req", "west"].concat(),
            ["ur", "eq"].concat(),
            ["Tcp", "Stream"].concat(),
            ["fet", "ch("].concat(),
        ];
        for proibido in proibidos {
            assert!(
                !fonte.contains(proibido.as_str()),
                "`{proibido}` apareceu: a assinatura chega como argumento, não da rede"
            );
        }
    }
}
