//! A criptografia do contrato: geradores, validação de ponto, Merkle e as
//! duas verificações de prova.
//!
//! Todo número aqui foi medido na testnet antes de a função existir. As
//! referências entre parênteses são as sondas de `bls-smoke/RESULTADOS.md`.

use crate::tipos::{Erro, ProvaCds, ProvaSoma};
use soroban_sdk::{
    crypto::bls12_381::{Bls12381Fr, Bls12381G1Affine},
    Address, Bytes, BytesN, Env, Vec,
};
use soroban_sdk::xdr::ToXdr;

/// "Nothing up my sleeve": `H` sai de hash-to-curve de uma string fixa, então
/// ninguém conhece `log_G(H)`. Se alguém conhecesse, poderia abrir qualquer
/// compromisso para qualquer valor e o vínculo cairia.
///
/// Conferido na sonda 11: on-curve, in-subgroup, `≠ G`, `≠ −G`, `≠ identidade`
/// e `≠ ±k·G` para `k` em 1..512.
pub const DST_H: &[u8] = b"TESSERA-V1-GENERATOR-H";
pub const DST_CDS: &[u8] = b"TESSERA-V1-FIAT-SHAMIR";
pub const DST_SOMA: &[u8] = b"TESSERA-V1-SOMA";

const DOM_FOLHA: u8 = 0x00;
const DOM_NO: u8 = 0x01;

/// Gerador canônico de G1, não comprimido: `be_bytes(X) ‖ be_bytes(Y)`.
pub fn gerador_g(env: &Env) -> Bls12381G1Affine {
    Bls12381G1Affine::from_bytes(BytesN::from_array(env, &[
        0x17,0xf1,0xd3,0xa7,0x31,0x97,0xd7,0x94,0x26,0x95,0x63,0x8c,0x4f,0xa9,0xac,0x0f,
        0xc3,0x68,0x8c,0x4f,0x97,0x74,0xb9,0x05,0xa1,0x4e,0x3a,0x3f,0x17,0x1b,0xac,0x58,
        0x6c,0x55,0xe8,0x3f,0xf9,0x7a,0x1a,0xef,0xfb,0x3a,0xf0,0x0a,0xdb,0x22,0xc6,0xbb,
        0x08,0xb3,0xf4,0x81,0xe3,0xaa,0xa0,0xf1,0xa0,0x9e,0x30,0xed,0x74,0x1d,0x8a,0xe4,
        0xfc,0xf5,0xe0,0x95,0xd5,0xd0,0x0a,0xf6,0x00,0xdb,0x18,0xcb,0x2c,0x04,0xb3,0xed,
        0xd0,0x3c,0xc7,0x44,0xa2,0x88,0x8a,0xe4,0x0c,0xaa,0x23,0x29,0x46,0xc5,0xe7,0xe1,
    ]))
}

/// `H` por hash-to-curve. Custa 2.653.011 (sonda 2), então é calculado uma vez
/// e guardado na instância.
pub fn calcular_h(env: &Env) -> Bls12381G1Affine {
    let bls = env.crypto().bls12_381();
    let msg = Bytes::from_slice(env, DST_H);
    let dst = Bytes::from_slice(env, DST_H);
    bls.hash_to_g1(&msg, &dst)
}

/// **Validação obrigatória de todo ponto que entra.**
///
/// A sonda 10 mediu on-curve em 4.367 e in-subgroup em 734.877, e mostrou que
/// o host **não** valida dentro da aritmética: `g1_add` custa 110.748, que é
/// 15% de uma checagem de subgrupo, e uma operação não contém algo 7× mais
/// caro que ela. Logo a validação é aditiva e é nossa.
///
/// Sem ela, um ponto de ordem pequena passa pelo MSM e vaza informação sobre
/// o escalar — ataque de subgrupo pequeno.
pub fn validar(env: &Env, p: &Bls12381G1Affine) -> Result<(), Erro> {
    let bls = env.crypto().bls12_381();
    if bls.g1_is_on_curve(p) && bls.g1_is_in_subgroup(p) {
        Ok(())
    } else {
        Err(Erro::PontoForaDoSubgrupo)
    }
}

/// Escalar a partir de um `u32`.
pub fn fr(env: &Env, v: u32) -> Bls12381Fr {
    Bls12381Fr::from_u256(soroban_sdk::U256::from_u32(env, v))
}

/// `−e`, para montar `z·H − e·X` como um MSM de 2 termos.
fn neg(env: &Env, e: &Bls12381Fr) -> Bls12381Fr {
    env.crypto().bls12_381().fr_sub(&fr(env, 0), e)
}

/// O desafio de Fiat–Shamir, 248 bits.
///
/// O byte alto é zerado porque `Bls12381Fr::from_bytes` **não reduz módulo r**:
/// sem isso, um hash acima do módulo seria um escalar não canônico. Zerar é
/// seguro num desafio, que só precisa ser imprevisível. Seria **errado** no
/// fator de aleatoriedade do compromisso, que precisa de uniformidade sobre
/// todo o corpo — `core/src/acaso.rs` usa rejeição, não redução.
fn para_desafio(env: &Env, buf: &Bytes) -> Bls12381Fr {
    let mut e = env.crypto().sha256(buf).to_array();
    e[0] = 0;
    Bls12381Fr::from_bytes(BytesN::from_array(env, &e))
}

/// O contexto que prende uma prova a esta proposta, a esta pessoa e a esta
/// opção.
///
/// **Sem isto, copiar o `C` e a prova de outra pessoa é um voto válido.** A
/// prova convence de que `v ∈ {0,1}` e nada nela diz de quem é; `Votou` impede
/// votar duas vezes, não impede votar com a cédula alheia.
/// **O contexto amarra a prova a uma pergunta, não só a uma opção.**
///
/// Sem o índice da pergunta, a opção 0 da pergunta 1 e a opção 0 da pergunta 2
/// produziriam o mesmo desafio de Fiat–Shamir — e uma disjuntiva feita para
/// uma valeria para a outra. O eleitor copiaria a própria prova da pergunta 1
/// para a 2 e marcaria a segunda sem provar nada sobre ela. Numa cédula de uma
/// pergunta só isso não existia; numa cédula mista é a primeira coisa que
/// quebra.
///
/// A prova de soma da pergunta `q` usa `(q, u32::MAX)`, então ela também não
/// migra entre perguntas.
pub fn contexto(
    env: &Env,
    proposta: &BytesN<32>,
    votante: &Address,
    pergunta: u32,
    opcao: u32,
) -> Bytes {
    let mut b = Bytes::from_array(env, &proposta.to_array());
    b.append(&votante.clone().to_xdr(env));
    b.extend_from_array(&pergunta.to_be_bytes());
    b.extend_from_array(&opcao.to_be_bytes());
    b
}

/// `e = H(DST ‖ len(ctx) ‖ ctx ‖ C ‖ a0 ‖ a1)`.
///
/// O comprimento entra antes do conteúdo para que `("ab","c")` e `("a","bc")`
/// não produzam o mesmo hash.
pub fn desafio_cds(
    env: &Env,
    ctx: &Bytes,
    c: &Bls12381G1Affine,
    a0: &Bls12381G1Affine,
    a1: &Bls12381G1Affine,
) -> Bls12381Fr {
    let mut buf = Bytes::from_slice(env, DST_CDS);
    buf.extend_from_array(&(ctx.len() as u32).to_be_bytes());
    buf.append(ctx);
    buf.extend_from_array(&c.to_array());
    buf.extend_from_array(&a0.to_array());
    buf.extend_from_array(&a1.to_array());
    para_desafio(env, &buf)
}

/// Verifica a prova disjuntiva: `C` comete 0 **ou** 1, e nada diz qual.
///
/// Três igualdades, todas obrigatórias:
///
/// 1. `e0 + e1 == H(ctx ‖ C ‖ a0 ‖ a1)` — só um dos ramos pode ser simulado;
/// 2. `z0·H == a0 + e0·C` — ramo `v = 0`;
/// 3. `z1·H == a1 + e1·(C − G)` — ramo `v = 1`.
///
/// Cada ramo é **um MSM de 2 termos**, não dois muls somados. Essa escolha vale
/// 60% do custo: 10.980.243 medidos contra ~27M projetados (sonda 12).
pub fn verificar_cds(
    env: &Env,
    ctx: &Bytes,
    g: &Bls12381G1Affine,
    h: &Bls12381G1Affine,
    c: &Bls12381G1Affine,
    p: &ProvaCds,
) -> bool {
    let bls = env.crypto().bls12_381();

    if bls.fr_add(&p.e0, &p.e1) != desafio_cds(env, ctx, c, &p.a0, &p.a1) {
        return false;
    }

    // ramo 0: z0·H − e0·C == a0
    let mut ps = Vec::new(env);
    let mut ss = Vec::new(env);
    ps.push_back(h.clone());
    ps.push_back(c.clone());
    ss.push_back(p.z0.clone());
    ss.push_back(neg(env, &p.e0));
    if bls.g1_msm(ps, ss) != p.a0 {
        return false;
    }

    // ramo 1: z1·H − e1·(C − G) == a1
    let c_menos_g = bls.g1_add(c, &(-g.clone()));
    let mut ps = Vec::new(env);
    let mut ss = Vec::new(env);
    ps.push_back(h.clone());
    ps.push_back(c_menos_g);
    ss.push_back(p.z1.clone());
    ss.push_back(neg(env, &p.e1));
    bls.g1_msm(ps, ss) == p.a1
}

/// Verifica a prova de soma: `D = (Σ C_j) − w·G` é `ρ·H` para um `ρ` conhecido.
///
/// Se cada `v_j ∈ {0,1}` (garantido pelas CDS) e a soma é `w`, então a cédula
/// é bem formada. **Uma prova, não `m` provas** — por isso o custo de `votar()`
/// cresce com `m` só nas disjuntivas.
pub fn verificar_soma(
    env: &Env,
    ctx: &Bytes,
    h: &Bls12381G1Affine,
    d: &Bls12381G1Affine,
    p: &ProvaSoma,
) -> bool {
    let bls = env.crypto().bls12_381();

    let mut buf = Bytes::from_slice(env, DST_SOMA);
    buf.extend_from_array(&(ctx.len() as u32).to_be_bytes());
    buf.append(ctx);
    buf.extend_from_array(&d.to_array());
    buf.extend_from_array(&p.a.to_array());
    let e = para_desafio(env, &buf);

    // z·H − e·D == a
    let mut ps = Vec::new(env);
    let mut ss = Vec::new(env);
    ps.push_back(h.clone());
    ps.push_back(d.clone());
    ss.push_back(p.z.clone());
    ss.push_back(neg(env, &e));
    bls.g1_msm(ps, ss) == p.a
}

/// Confere o caminho de Merkle da lista de aptos.
///
/// A folha é **recalculada** a partir do endereço e do peso que chegaram na
/// chamada. Peso inflado produz outra folha e o caminho não fecha — essa é a
/// defesa inteira contra peso falso (SPEC §6.4).
///
/// Separação de domínio entre folha (`0x00`) e nó (`0x01`): sem ela, uma folha
/// de 64 bytes bem escolhida seria apresentada como nó interno.
///
/// Custa 127.863 em profundidade 8, 13.128 por nível (sonda 13).
pub fn verificar_aptidao(
    env: &Env,
    votante: &Address,
    peso: u32,
    indice: u32,
    irmaos: &Vec<BytesN<32>>,
    raiz: &BytesN<32>,
) -> bool {
    let mut buf = Bytes::from_slice(env, &[DOM_FOLHA]);
    buf.append(&votante.clone().to_xdr(env));
    buf.extend_from_array(&peso.to_be_bytes());
    let mut atual = env.crypto().sha256(&buf).to_bytes();

    let mut i = indice;
    for irmao in irmaos.iter() {
        let mut b = Bytes::from_slice(env, &[DOM_NO]);
        if i % 2 == 0 {
            b.extend_from_array(&atual.to_array());
            b.extend_from_array(&irmao.to_array());
        } else {
            b.extend_from_array(&irmao.to_array());
            b.extend_from_array(&atual.to_array());
        }
        atual = env.crypto().sha256(&b).to_bytes();
        i /= 2;
    }
    // O índice tem de ter se esgotado: um índice maior que a árvore seria
    // outro caminho para a mesma raiz.
    i == 0 && atual == *raiz
}
