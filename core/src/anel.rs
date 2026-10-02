//! Assinatura em anel ligável: prova "sou um destes", sem dizer qual.
//!
//! É o que separa o caderno da urna. O caderno registra quem compareceu, com
//! nome e endereço, porque voto obrigatório precisa saber quem faltou. A cédula
//! sai de uma chave efêmera e carrega esta assinatura, que prova que quem a
//! mandou é um dos membros — e nada além disso. Nenhum dos dois registros liga
//! uma pessoa a uma cédula.
//!
//! **É a mesma construção do `cds.rs`, generalizada.** Lá são dois ramos, um
//! real e um fabricado de trás para frente, com os desafios parciais somando o
//! desafio total. Aqui são `n` ramos encadeados num ciclo: quem assina começa
//! no próprio índice, fabrica todos os outros em volta do anel, e fecha o ciclo
//! usando a chave que só ele tem. Sigma-protocolo com Fiat–Shamir, sem SNARK,
//! sem setup, sem circuito — a mesma postura do resto do projeto.
//!
//! ```text
//!   c_{i+1} = Hash(msg, z_i·G + c_i·P_i, z_i·Hp + c_i·I)
//! ```
//!
//! Duas bases, não uma. A primeira amarra o anel às chaves públicas; a segunda
//! amarra a **imagem de chave** `I = x·Hp`, e é ela que torna o anel *ligável*:
//! a mesma pessoa assinando duas vezes produz a mesma imagem, e a segunda
//! cédula é recusada. Sem a imagem, um membro votaria quantas vezes quisesse —
//! anonimato sem ligabilidade é fraude.
//!
//! ## `Hp` é por proposta, e isso não é detalhe
//!
//! Se `Hp` fosse um gerador fixo do sistema, a imagem `I = x·Hp` seria a mesma
//! em toda votação que a pessoa participasse — e daria para dizer "quem votou
//! em A também votou em B", montando um perfil ao longo do tempo. Com
//! `Hp = hash_to_g1(proposta)`, a imagem vive dentro de uma proposta e morre
//! com ela. O verificador calcula `Hp` **uma vez**, não `n` vezes, que é o que
//! torna isto pagável dentro de uma transação.
//!
//! ## A hipótese, dita na cara
//!
//! O anonimato depende de ninguém conseguir decidir, dado `{P_i = x_i·G}` e
//! `I = x_s·Hp`, qual `i` é o `s` — isto é, de DDH ser difícil em G1. Na
//! BLS12-381 isso é a hipótese XDH: o emparelhamento é do tipo 3, não existe
//! mapa eficiente de G1 para G2, e por isso não dá para testar a tupla. É
//! hipótese padrão, mas é hipótese, e não a mesma coisa que o sigilo
//! **perfeito** do compromisso de Pedersen. Quem lê a tela merece saber a
//! diferença.
//!
//! ## E o que o anel cobra
//!
//! O anel publica o conjunto. Anonimato de anel é anonimato *dentro de um grupo
//! conhecido*: o verificador precisa de todas as chaves para conferir todos os
//! ramos. Então a lista de membros deixa de ser 32 bytes de raiz e passa a ser
//! pública — como o eleitorado no Brasil, que é público, e como o caderno, que
//! também é. O que fica secreto é só o vínculo.

use crate::pedersen;
use crate::ponto;
use ark_bls12_381::{Fr, G1Affine, G1Projective};
use ark_ec::AffineRepr;
use ark_ff::{BigInteger, PrimeField};
use sha2::{Digest, Sha256};

/// Domain separation tag do desafio do anel. Diferente do DST do CDS de
/// propósito: um desafio de anel nunca pode ser reaproveitado como desafio de
/// disjuntiva, nem o contrário.
pub const DST_DESAFIO: &[u8] = b"TESSERA-V1-ANEL";

/// DST do `Hp` por proposta.
pub const DST_HP: &[u8] = b"TESSERA-V1-ANEL-HP";

#[derive(Debug, Clone, PartialEq)]
pub struct Assinatura {
    /// O desafio que fecha o ciclo. Verificar é dar a volta no anel e cair
    /// exatamente aqui de novo.
    pub c0: Fr,
    /// Uma resposta por ramo. `z[i]` é real só no ramo de quem assinou, e nem
    /// o verificador distingue qual.
    pub z: Vec<Fr>,
    /// `I = x·Hp`. É o que impede a segunda cédula.
    pub imagem: G1Affine,
}

#[derive(Debug, PartialEq)]
pub enum Erro {
    AnelVazio,
    IndiceForaDoAnel { indice: usize, tamanho: usize },
    /// A chave secreta não abre a chave pública daquele índice. É erro de quem
    /// chama, e vale falhar alto: assinar com o índice errado produziria uma
    /// assinatura inválida e um bug muito mais caro de achar.
    ChaveNaoCorresponde,
    SemAleatoriedade,
    BytesInvalidos,
}

/// `96 + 32·(n+1)`: a imagem, o `c0`, e um `z` por ramo.
pub fn tamanho(n: usize) -> usize {
    96 + 32 * (n + 1)
}

fn fr_bytes(f: &Fr) -> [u8; 32] {
    let v = f.into_bigint().to_bytes_be();
    let mut saida = [0u8; 32];
    saida[32 - v.len()..].copy_from_slice(&v);
    saida
}

/// `Hp` da proposta, por hash-to-curve ingênuo: tenta sucessivos contadores até
/// cair num ponto da curva, e multiplica pelo cofator para garantir o subgrupo.
///
/// Não é constant-time nem uniforme como o `hash_to_g1` do host, e **não precisa
/// ser**: `Hp` é público, calculado por todo mundo a partir do mesmo
/// identificador de proposta, e nada secreto depende do caminho tomado. O que
/// importa é que o log discreto de `Hp` na base `G` seja desconhecido — e é,
/// porque ninguém escolheu `Hp`, ele caiu de um hash.
pub fn hp(proposta: &[u8]) -> G1Affine {
    for contador in 0u32..256 {
        let mut h = Sha256::new();
        h.update(DST_HP);
        h.update((proposta.len() as u32).to_be_bytes());
        h.update(proposta);
        h.update(contador.to_be_bytes());
        let semente: [u8; 32] = h.finalize().into();
        // Um escalar do hash multiplicando o gerador também daria um ponto —
        // mas com log discreto conhecido por quem calculou. Aqui o ponto vem de
        // uma coordenada x, que é o que mantém o log discreto desconhecido.
        if let Some(p) = G1Affine::from_random_bytes(&semente) {
            let p = p.mul_by_cofactor_to_group();
            if !p.eq(&G1Projective::default()) {
                return p.into();
            }
        }
    }
    unreachable!("256 tentativas sem um ponto na curva é impossível na prática")
}

/// A chave pública do anel: `P = x·G`.
pub fn chave_publica(g: &G1Affine, x: &Fr) -> G1Affine {
    (*g * x).into()
}

/// A imagem de chave: `I = x·Hp`.
pub fn imagem(hp: &G1Affine, x: &Fr) -> G1Affine {
    (*hp * x).into()
}

/// O preâmbulo: a mensagem e o anel inteiro, hasheados **uma vez**.
///
/// O conjunto precisa entrar no desafio — sem isso, uma assinatura válida para
/// um anel serviria para outro menor que apenas contivesse o mesmo signatário,
/// e um anel de uma pessoa revela quem é. Mas ele não precisa entrar `n` vezes.
/// Hashear `n` pontos em cada um dos `n` elos é trabalho quadrático, e dentro de
/// um contrato isso se paga em instruções: num anel de 20 seriam 38 KB passando
/// pelo SHA-256 em vez de 2 KB.
///
/// O contrato guarda este preâmbulo pela parte do anel, e é por isso que ele
/// consegue conferir que a lista apresentada é a mesma que foi fixada na
/// abertura sem guardar a lista inteira.
pub fn preambulo(msg: &[u8], anel: &[G1Affine]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(DST_DESAFIO);
    h.update((msg.len() as u32).to_be_bytes());
    h.update(msg);
    h.update((anel.len() as u32).to_be_bytes());
    for p in anel {
        h.update(ponto::serializar(p));
    }
    h.finalize().into()
}

/// Desafio do elo `i → i+1`.
///
/// O byte alto é zerado pelo mesmo motivo do `cds.rs`: o
/// `Bls12381Fr::from_bytes` do contrato **não reduz módulo r**, e um desafio só
/// precisa ser imprevisível. 248 bits são muito mais que os 128 necessários.
fn elo(pre: &[u8; 32], imagem: &G1Affine, a: &G1Affine, b: &G1Affine) -> Fr {
    let mut h = Sha256::new();
    h.update(pre);
    h.update(ponto::serializar(imagem));
    h.update(ponto::serializar(a));
    h.update(ponto::serializar(b));
    let mut e: [u8; 32] = h.finalize().into();
    e[0] = 0;
    Fr::from_be_bytes_mod_order(&e)
}

/// Assina `msg` como um dos membros de `anel`, sem revelar qual.
pub fn assinar(
    msg: &[u8],
    g: &G1Affine,
    hp: &G1Affine,
    anel: &[G1Affine],
    indice: usize,
    x: &Fr,
) -> Result<Assinatura, Erro> {
    let n = anel.len();
    if n == 0 {
        return Err(Erro::AnelVazio);
    }
    if indice >= n {
        return Err(Erro::IndiceForaDoAnel { indice, tamanho: n });
    }
    if chave_publica(g, x) != anel[indice] {
        return Err(Erro::ChaveNaoCorresponde);
    }

    let fr = || pedersen::acaso_fr().map_err(|_| Erro::SemAleatoriedade);
    let img = imagem(hp, x);
    let pre = preambulo(msg, anel);

    // O ramo real começa com um compromisso honesto de Schnorr nas duas bases.
    let u = fr()?;
    let mut c = vec![Fr::from(0u64); n];
    let mut z = vec![Fr::from(0u64); n];

    let prox = (indice + 1) % n;
    c[prox] = elo(&pre, &img, &(*g * u).into(), &(*hp * u).into());

    // E daqui em diante tudo é fabricado de trás para frente, dando a volta no
    // anel até voltar ao ponto de partida — exatamente o ramo simulado do CDS,
    // repetido n−1 vezes.
    let mut i = prox;
    while i != indice {
        z[i] = fr()?;
        let a: G1Affine = (*g * z[i] + G1Projective::from(anel[i]) * c[i]).into();
        let b: G1Affine = (*hp * z[i] + G1Projective::from(img) * c[i]).into();
        let seguinte = (i + 1) % n;
        c[seguinte] = elo(&pre, &img, &a, &b);
        i = seguinte;
    }

    // O ciclo só fecha para quem sabe `x`. É este passo, e só ele, que a chave
    // secreta torna possível.
    z[indice] = u - c[indice] * x;

    Ok(Assinatura { c0: c[0], z, imagem: img })
}

/// Dá a volta no anel e confere se cai de volta em `c0`.
pub fn verificar(
    msg: &[u8],
    g: &G1Affine,
    hp: &G1Affine,
    anel: &[G1Affine],
    s: &Assinatura,
) -> bool {
    let n = anel.len();
    if n == 0 || s.z.len() != n {
        return false;
    }
    let pre = preambulo(msg, anel);
    let mut c = s.c0;
    for i in 0..n {
        let a: G1Affine = (*g * s.z[i] + G1Projective::from(anel[i]) * c).into();
        let b: G1Affine = (*hp * s.z[i] + G1Projective::from(s.imagem) * c).into();
        c = elo(&pre, &s.imagem, &a, &b);
    }
    c == s.c0
}

impl Assinatura {
    /// `imagem ‖ c0 ‖ z[0] ‖ … ‖ z[n-1]`, na ordem que o contrato vai ler.
    pub fn serializar(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(tamanho(self.z.len()));
        b.extend_from_slice(&ponto::serializar(&self.imagem));
        b.extend_from_slice(&fr_bytes(&self.c0));
        for zi in &self.z {
            b.extend_from_slice(&fr_bytes(zi));
        }
        b
    }

    pub fn desserializar(b: &[u8], n: usize) -> Result<Assinatura, Erro> {
        if b.len() != tamanho(n) {
            return Err(Erro::BytesInvalidos);
        }
        let imagem = ponto::desserializar(&b[0..96]).map_err(|_| Erro::BytesInvalidos)?;
        let c0 = Fr::from_be_bytes_mod_order(&b[96..128]);
        let mut z = Vec::with_capacity(n);
        for i in 0..n {
            let ini = 128 + 32 * i;
            z.push(Fr::from_be_bytes_mod_order(&b[ini..ini + 32]));
        }
        Ok(Assinatura { c0, z, imagem })
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn membros(n: usize) -> (G1Affine, Vec<Fr>, Vec<G1Affine>) {
        let g = pedersen::gerador();
        let xs: Vec<Fr> = (0..n).map(|_| pedersen::acaso_fr().unwrap()).collect();
        let ps = xs.iter().map(|x| chave_publica(&g, x)).collect();
        (g, xs, ps)
    }

    #[test]
    fn o_anel_fecha_para_quem_tem_a_chave() {
        let (g, xs, anel) = membros(7);
        let hp = hp(b"assembleia-2026");
        for i in 0..7 {
            let s = assinar(b"cedula", &g, &hp, &anel, i, &xs[i]).unwrap();
            assert!(verificar(b"cedula", &g, &hp, &anel, &s), "ramo {} não fechou", i);
        }
    }

    #[test]
    fn nao_fecha_para_quem_nao_esta_no_anel() {
        let (g, _, anel) = membros(5);
        let hp = hp(b"p");
        let intruso = pedersen::acaso_fr().unwrap();
        // Assinar com chave de fora é erro de quem chama, e falha alto.
        assert_eq!(
            assinar(b"m", &g, &hp, &anel, 0, &intruso),
            Err(Erro::ChaveNaoCorresponde)
        );
    }

    #[test]
    fn a_mensagem_esta_presa_a_assinatura() {
        let (g, xs, anel) = membros(4);
        let hp = hp(b"p");
        let s = assinar(b"voto em A", &g, &hp, &anel, 2, &xs[2]).unwrap();
        // Trocar a cédula depois de assinada é o ataque óbvio: o anel prova
        // pertencimento, e sem amarrar a mensagem provaria pertencimento a
        // qualquer cédula.
        assert!(!verificar(b"voto em B", &g, &hp, &anel, &s));
    }

    #[test]
    fn o_anel_esta_preso_a_assinatura() {
        let (g, xs, anel) = membros(6);
        let hp = hp(b"p");
        let s = assinar(b"m", &g, &hp, &anel, 1, &xs[1]).unwrap();
        // Um anel menor que ainda contenha o signatário revelaria quem é. Se o
        // conjunto não entrasse no desafio, a assinatura sobreviveria ao corte.
        let menor: Vec<G1Affine> = anel[0..2].to_vec();
        assert!(!verificar(b"m", &g, &hp, &menor, &s));
    }

    #[test]
    fn a_mesma_pessoa_produz_a_mesma_imagem() {
        let (g, xs, anel) = membros(5);
        let hp = hp(b"proposta-unica");
        let a = assinar(b"cedula 1", &g, &hp, &anel, 3, &xs[3]).unwrap();
        let b = assinar(b"cedula 2", &g, &hp, &anel, 3, &xs[3]).unwrap();
        // É assim que o contrato recusa a segunda cédula sem saber de quem é.
        assert_eq!(a.imagem, b.imagem);
    }

    #[test]
    fn pessoas_diferentes_produzem_imagens_diferentes() {
        let (g, xs, anel) = membros(5);
        let hp = hp(b"p");
        let a = assinar(b"m", &g, &hp, &anel, 0, &xs[0]).unwrap();
        let b = assinar(b"m", &g, &hp, &anel, 1, &xs[1]).unwrap();
        assert_ne!(a.imagem, b.imagem);
    }

    #[test]
    fn a_imagem_nao_atravessa_propostas() {
        let (g, xs, anel) = membros(3);
        let a = assinar(b"m", &g, &hp(b"proposta-A"), &anel, 0, &xs[0]).unwrap();
        let b = assinar(b"m", &g, &hp(b"proposta-B"), &anel, 0, &xs[0]).unwrap();
        // Se `Hp` fosse fixo do sistema, estas duas seriam iguais e daria para
        // dizer "quem votou em A também votou em B".
        assert_ne!(a.imagem, b.imagem);
    }

    #[test]
    fn hp_e_deterministico_e_esta_no_subgrupo() {
        let a = hp(b"assembleia-2026");
        assert_eq!(a, hp(b"assembleia-2026"));
        assert_ne!(a, hp(b"assembleia-2027"));
        assert!(a.is_on_curve());
        assert!(a.is_in_correct_subgroup_assuming_on_curve());
    }

    #[test]
    fn anel_de_um_funciona_e_nao_esconde_nada() {
        let (g, xs, anel) = membros(1);
        let hp = hp(b"p");
        let s = assinar(b"m", &g, &hp, &anel, 0, &xs[0]).unwrap();
        // Válido, e inútil como anonimato: o conjunto tem uma pessoa. A tela
        // tem de dizer o tamanho do anel, que é o tamanho do esconderijo.
        assert!(verificar(b"m", &g, &hp, &anel, &s));
        assert_eq!(anel.len(), 1);
    }

    #[test]
    fn anel_vazio_e_indice_fora_falham() {
        let g = pedersen::gerador();
        let hp = hp(b"p");
        let x = pedersen::acaso_fr().unwrap();
        assert_eq!(assinar(b"m", &g, &hp, &[], 0, &x), Err(Erro::AnelVazio));
        let anel = vec![chave_publica(&g, &x)];
        assert_eq!(
            assinar(b"m", &g, &hp, &anel, 5, &x),
            Err(Erro::IndiceForaDoAnel { indice: 5, tamanho: 1 })
        );
    }

    #[test]
    fn um_z_adulterado_derruba_o_anel() {
        let (g, xs, anel) = membros(5);
        let hp = hp(b"p");
        let mut s = assinar(b"m", &g, &hp, &anel, 2, &xs[2]).unwrap();
        s.z[4] += Fr::from(1u64);
        assert!(!verificar(b"m", &g, &hp, &anel, &s));
    }

    #[test]
    fn uma_imagem_forjada_derruba_o_anel() {
        let (g, xs, anel) = membros(5);
        let hp = hp(b"p");
        let mut s = assinar(b"m", &g, &hp, &anel, 2, &xs[2]).unwrap();
        // Trocar a imagem para escapar da detecção de voto duplo quebra o elo
        // da segunda base, que é exatamente o que a segunda base existe para
        // impedir.
        s.imagem = imagem(&hp, &xs[0]);
        assert!(!verificar(b"m", &g, &hp, &anel, &s));
    }

    #[test]
    fn ida_e_volta_pelos_bytes() {
        let (g, xs, anel) = membros(10);
        let hp = hp(b"p");
        let s = assinar(b"m", &g, &hp, &anel, 6, &xs[6]).unwrap();
        let b = s.serializar();
        assert_eq!(b.len(), tamanho(10));
        assert_eq!(b.len(), 448);
        let v = Assinatura::desserializar(&b, 10).unwrap();
        assert_eq!(v, s);
        assert!(verificar(b"m", &g, &hp, &anel, &v));
    }

    #[test]
    fn anel_de_vinte_ainda_fecha() {
        let (g, xs, anel) = membros(20);
        let hp = hp(b"comunidade");
        let s = assinar(b"m", &g, &hp, &anel, 19, &xs[19]).unwrap();
        assert!(verificar(b"m", &g, &hp, &anel, &s));
        assert_eq!(tamanho(20), 768);
    }
}
