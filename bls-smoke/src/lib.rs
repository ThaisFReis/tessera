#![no_std]
//! Smoke test das host functions BLS12-381 (CAP-0059) em Soroban.
//!
//! Responde, em ordem, as tres perguntas que decidem o Plano A da urna
//! confidencial (ElGamal exponencial + apuracao homomorfica on-chain):
//!
//!   1. As host functions existem no pin do SDK e a serializacao de ponto bate?
//!   2. Quanto custa cada primitiva, e quantas cedulas cabem numa transacao?
//!   3. A matematica de apuracao fecha de ponta a ponta dentro do contrato?
//!
//! Nao e producao: nao ha autorizacao, armazenamento, nullifier nem mesa k-de-n.
//! E uma sonda de viabilidade.

use soroban_sdk::{
    contract, contractimpl,
    crypto::bls12_381::{Bls12381Fr, Bls12381G1Affine, Bls12381G2Affine},
    Bytes, BytesN, Env, Vec,
};

/// Domain separation tag do desafio de Fiat-Shamir.
/// TODO: renomear para TESSERA-V1-FIAT-SHAMIR de uma vez so, antes de
/// publicar qualquer vetor de teste (PROTOCOLO.md §2).
const DST: &[u8] = b"KARN-URNA-V0-CHALLENGE";

/// DST do segundo gerador. "Nothing up my sleeve": H sai de hash-to-curve de
/// uma string publica e fixa, nunca de sorteio, nunca configuravel pela
/// governanca. Se alguem souber h tal que H = h*G, abre qualquer compromisso.
const DST_H: &[u8] = b"TESSERA-V1-GENERATOR-H";

/// DST do desafio de Fiat-Shamir do CDS. Tem de ser byte a byte igual ao do
/// `core`: um DST divergente faz TODA prova falhar, sem dizer por que.
const DST_DESAFIO: &[u8] = b"TESSERA-V1-FIAT-SHAMIR";

#[contract]
pub struct BlsSmoke;

#[contractimpl]
impl BlsSmoke {
    // ---------- Sonda 1: existencia e serializacao ----------

    /// Devolve (esta_na_curva, esta_no_subgrupo) para 96 bytes crus.
    /// Se o gerador canonico falhar aqui, a serializacao que o host espera
    /// nao e `be_bytes(X) || be_bytes(Y)` e todo o resto muda.
    pub fn probe(env: Env, p: Bls12381G1Affine) -> (bool, bool) {
        let bls = env.crypto().bls12_381();
        (bls.g1_is_on_curve(&p), bls.g1_is_in_subgroup(&p))
    }

    // ---------- Sonda 2: custo das primitivas ----------

    /// `n` somas em G1 encadeadas.
    pub fn add_n(env: Env, a: Bls12381G1Affine, b: Bls12381G1Affine, n: u32) -> Bls12381G1Affine {
        let bls = env.crypto().bls12_381();
        let mut acc = a;
        for _ in 0..n {
            acc = bls.g1_add(&acc, &b);
        }
        acc
    }

    /// `n` multiplicacoes escalares em G1 encadeadas.
    pub fn mul_n(env: Env, p: Bls12381G1Affine, s: Bls12381Fr, n: u32) -> Bls12381G1Affine {
        let bls = env.crypto().bls12_381();
        let mut acc = p;
        for _ in 0..n {
            acc = bls.g1_mul(&acc, &s);
        }
        acc
    }

    /// Multi-scalar multiplication: o atalho que substitui k muls + k-1 adds.
    pub fn msm(env: Env, ps: Vec<Bls12381G1Affine>, ss: Vec<Bls12381Fr>) -> Bls12381G1Affine {
        env.crypto().bls12_381().g1_msm(ps, ss)
    }

    /// hash-to-curve: como um votante deriva um ponto sem confiar em vetor fixo.
    pub fn hash_g1(env: Env, msg: Bytes) -> Bls12381G1Affine {
        let dst = Bytes::from_slice(&env, DST);
        env.crypto().bls12_381().hash_to_g1(&msg, &dst)
    }

    /// hash-to-curve em G2 — usado so para montar o par do pairing check.
    pub fn hash_g2(env: Env, msg: Bytes) -> Bls12381G2Affine {
        let dst = Bytes::from_slice(&env, DST);
        env.crypto().bls12_381().hash_to_g2(&msg, &dst)
    }

    /// Pairing check — nao e usado na urna, medido so para saber o teto de custo
    /// do caminho Groth16 (o roadmap de ZK).
    pub fn pairing(env: Env, ps: Vec<Bls12381G1Affine>, qs: Vec<Bls12381G2Affine>) -> bool {
        env.crypto().bls12_381().pairing_check(ps, qs)
    }

    /// Desafio de Fiat-Shamir. Exposto para o provador off-chain usar
    /// exatamente a mesma funcao que o verificador on-chain.
    pub fn challenge(env: Env, g: Bls12381G1Affine, pk: Bls12381G1Affine, a: Bls12381G1Affine) -> Bls12381Fr {
        challenge_fr(&env, &g, &pk, &a)
    }

    // ---------- Sonda 3: sigma-protocolo real verificado on-chain ----------

    /// Schnorr: prova de conhecimento de `sk` em `pk = sk*G`.
    /// Verifica `z*G == A + e*PK`, com `e = H(DST || G || PK || A)`.
    ///
    /// Esta e a forma exata da Chaum-Pedersen que a mesa apuradora usa para
    /// provar decifracao correta. Custo: 2 g1_mul + 1 g1_add + 1 sha256.
    pub fn verify_schnorr(env: Env, g: Bls12381G1Affine, pk: Bls12381G1Affine, a: Bls12381G1Affine, z: Bls12381Fr) -> bool {
        let bls = env.crypto().bls12_381();
        let e = challenge_fr(&env, &g, &pk, &a);
        let lhs = bls.g1_mul(&g, &z);
        let rhs = bls.g1_add(&a, &bls.g1_mul(&pk, &e));
        lhs == rhs
    }

    // ---------- Sonda 4: apuracao homomorfica de ponta a ponta ----------

    /// Agrega cedulas ElGamal exponenciais, decifra o total e resolve o log
    /// discreto por forca bruta.
    ///
    /// Cedula i: `C1 = r_i*G`, `C2 = r_i*PK + m_i*G`.
    /// Agregado: `(sum C1, sum C2)` cifra `sum m_i` — a propriedade que faz a
    /// apuracao ser publica sem revelar voto individual.
    /// Decifra: `M = sum C2 - sk * sum C1 = (sum m_i)*G`.
    ///
    /// Devolve `sum m_i`. Panica se o total passar de `max`.
    pub fn tally(env: Env, g: Bls12381G1Affine, c1s: Vec<Bls12381G1Affine>, c2s: Vec<Bls12381G1Affine>, sk: Bls12381Fr, max: u32) -> u32 {
        let bls = env.crypto().bls12_381();
        let n = c1s.len();
        if n == 0 || n != c2s.len() {
            panic!("cedulas malformadas");
        }

        // agregacao homomorfica
        let mut acc1 = c1s.get(0).unwrap();
        let mut acc2 = c2s.get(0).unwrap();
        for i in 1..n {
            acc1 = bls.g1_add(&acc1, &c1s.get(i).unwrap());
            acc2 = bls.g1_add(&acc2, &c2s.get(i).unwrap());
        }

        // decifracao com a chave da mesa
        let shared = bls.g1_mul(&acc1, &sk);
        let m = bls.g1_add(&acc2, &(-shared));

        // log discreto por forca bruta, limitado pelo peso total
        let mut cand = g.clone();
        let mut t: u32 = 1;
        while t <= max {
            if cand == m {
                return t;
            }
            cand = bls.g1_add(&cand, &g);
            t += 1;
        }
        panic!("total fora da faixa");
    }

    /// Variante sem busca: a mesa apuradora ASSERTA o total e o contrato apenas
    /// confere `M == total*G`. Troca uma busca O(total) por 1 g1_mul.
    ///
    /// Descoberta da sonda 2: forca bruta custa ~1 g1_add por unidade de peso,
    /// o que estoura o teto de CPU em qualquer assembleia real. Verificar e
    /// barato; procurar, nao.
    pub fn tally_checked(
        env: Env,
        g: Bls12381G1Affine,
        c1s: Vec<Bls12381G1Affine>,
        c2s: Vec<Bls12381G1Affine>,
        sk: Bls12381Fr,
        total_alegado: u32,
    ) -> bool {
        let bls = env.crypto().bls12_381();
        let n = c1s.len();
        if n == 0 || n != c2s.len() {
            panic!("cedulas malformadas");
        }
        let mut acc1 = c1s.get(0).unwrap();
        let mut acc2 = c2s.get(0).unwrap();
        for i in 1..n {
            acc1 = bls.g1_add(&acc1, &c1s.get(i).unwrap());
            acc2 = bls.g1_add(&acc2, &c2s.get(i).unwrap());
        }
        let m = bls.g1_add(&acc2, &(-bls.g1_mul(&acc1, &sk)));
        let alegado = bls.g1_mul(&g, &Bls12381Fr::from_u256(soroban_sdk::U256::from_u32(&env, total_alegado)));
        m == alegado
    }

    // ---------- Sonda 7 (B1): Pedersen e abertura do agregado ----------
    //
    // A sonda 4 mediu ElGamal exponencial. O desenho final do SPEC e Pedersen:
    // o ledger recebe um compromisso PERFEITAMENTE ocultante em vez de um texto
    // cifrado, e a mesa abre o AGREGADO em vez de decifrar. Esta secao executa
    // esse desenho pela primeira vez.

    /// Segundo gerador `H`, por hash-to-curve de um DST publico e fixo.
    /// Calculado uma vez na inicializacao do contrato real, nunca por transacao
    /// (hash_to_g1 custa ~2,65M).
    pub fn gerador_h(env: Env) -> Bls12381G1Affine {
        let bls = env.crypto().bls12_381();
        let msg = Bytes::from_slice(&env, DST_H);
        let dst = Bytes::from_slice(&env, DST_H);
        bls.hash_to_g1(&msg, &dst)
    }

    /// Compromisso de Pedersen `C = v*G + r*H`, como um MSM de 2 termos.
    ///
    /// Perfeitamente ocultante: para todo v' existe r' com v'*G + r'*H == C,
    /// logo C e uniforme em G1 e independente de v. Nao ha o que decifrar,
    /// com qualquer poder computacional, nunca.
    pub fn commit(
        env: Env,
        g: Bls12381G1Affine,
        h: Bls12381G1Affine,
        v: u32,
        r: Bls12381Fr,
    ) -> Bls12381G1Affine {
        let bls = env.crypto().bls12_381();
        let mut ps = Vec::new(&env);
        let mut ss = Vec::new(&env);
        ps.push_back(g);
        ps.push_back(h);
        ss.push_back(u32_fr(&env, v));
        ss.push_back(r);
        bls.g1_msm(ps, ss)
    }

    /// Soma compromissos, como o acumulador `Acum(id, j)` do contrato faria a
    /// cada voto. Aditivamente homomorfico: a soma dos compromissos e o
    /// compromisso da soma, com a soma das aleatoriedades.
    pub fn agregar(env: Env, cs: Vec<Bls12381G1Affine>) -> Bls12381G1Affine {
        let bls = env.crypto().bls12_381();
        if cs.len() == 0 {
            panic!("agregado vazio");
        }
        let mut acc = cs.get(0).unwrap();
        for i in 1..cs.len() {
            acc = bls.g1_add(&acc, &cs.get(i).unwrap());
        }
        acc
    }

    /// O CORACAO DO DESENHO: confere a abertura do agregado.
    ///
    /// A mesa publica `(total, soma_r)` e o contrato confere
    /// `A == total*G + soma_r*H`, um MSM de 2 termos.
    ///
    /// Revelar `soma_r` NAO revela nenhum `r_i`: e a soma de n valores
    /// uniformes, e conhecer a soma de n incognitas nao determina nenhuma.
    /// Pelo vinculo computacional, a mesa nao consegue afirmar um total falso
    /// sem resolver um log discreto em BLS12-381.
    pub fn verify_aggregate(
        env: Env,
        a: Bls12381G1Affine,
        g: Bls12381G1Affine,
        h: Bls12381G1Affine,
        total: u32,
        soma_r: Bls12381Fr,
    ) -> bool {
        let bls = env.crypto().bls12_381();
        let mut ps = Vec::new(&env);
        let mut ss = Vec::new(&env);
        ps.push_back(g);
        ps.push_back(h);
        ss.push_back(u32_fr(&env, total));
        ss.push_back(soma_r);
        a == bls.g1_msm(ps, ss)
    }

    // ---------- Sonda 12 (B4): prova disjuntiva CDS ----------
    //
    // Prova que o compromisso abre para 0 OU para 1, sem revelar qual. Sem
    // isso nao ha urna: um voto com v=1000 encheria a apuracao.
    //
    // Dois ramos Schnorr sobre a base H, com o desafio dividido de modo que
    // e0 + e1 = e. Cada ramo e um MSM de 2 termos, nao dois muls somados:
    // a sonda 2 mediu msm(k=2) em 5,4M contra 6,58M de dois muls.

    /// Desafio de Fiat-Shamir do CDS, ligado ao compromisso e aos dois anuncios.
    pub fn desafio_cds(
        env: Env,
        c: Bls12381G1Affine,
        a0: Bls12381G1Affine,
        a1: Bls12381G1Affine,
    ) -> Bls12381Fr {
        desafio_fr(&env, &c, &a0, &a1)
    }

    /// Verifica a prova disjuntiva. Tres igualdades, todas tem de fechar.
    pub fn verify_cds(
        env: Env,
        g: Bls12381G1Affine,
        h: Bls12381G1Affine,
        c: Bls12381G1Affine,
        a0: Bls12381G1Affine,
        a1: Bls12381G1Affine,
        e0: Bls12381Fr,
        z0: Bls12381Fr,
        e1: Bls12381Fr,
        z1: Bls12381Fr,
    ) -> bool {
        let bls = env.crypto().bls12_381();

        // 1. os desafios parciais somam o desafio ligado a (C, a0, a1)
        if bls.fr_add(&e0, &e1) != desafio_fr(&env, &c, &a0, &a1) {
            return false;
        }
        let zero = u32_fr(&env, 0);

        // 2. ramo 0: z0*H - e0*C == a0
        let mut ps = Vec::new(&env);
        let mut ss = Vec::new(&env);
        ps.push_back(h.clone());
        ps.push_back(c.clone());
        ss.push_back(z0);
        ss.push_back(bls.fr_sub(&zero, &e0));
        if bls.g1_msm(ps, ss) != a0 {
            return false;
        }

        // 3. ramo 1: z1*H - e1*(C - G) == a1
        let c_menos_g = bls.g1_add(&c, &(-g));
        let mut ps = Vec::new(&env);
        let mut ss = Vec::new(&env);
        ps.push_back(h);
        ps.push_back(c_menos_g);
        ss.push_back(z1);
        ss.push_back(bls.fr_sub(&zero, &e1));
        bls.g1_msm(ps, ss) == a1
    }

    // ---------- Sonda 10 (C1): desserializacao e checagem de subgrupo ----------
    //
    // Todo ponto G1 que entra por argumento precisa ser validado on-curve e
    // in-subgroup, senao ha ataque de subgrupo pequeno. Sao ~6 pontos por
    // `votar()` e o custo nunca foi isolado. A checagem de subgrupo e a cara:
    // ingenuamente e uma multiplicacao escalar pelo cofator.

    /// So on-curve, em `n` pontos.
    pub fn on_curve_n(env: Env, ps: Vec<Bls12381G1Affine>) -> u32 {
        let bls = env.crypto().bls12_381();
        let mut ok = 0u32;
        for p in ps.iter() {
            if bls.g1_is_on_curve(&p) {
                ok += 1;
            }
        }
        ok
    }

    /// So in-subgroup, em `n` pontos.
    pub fn in_subgroup_n(env: Env, ps: Vec<Bls12381G1Affine>) -> u32 {
        let bls = env.crypto().bls12_381();
        let mut ok = 0u32;
        for p in ps.iter() {
            if bls.g1_is_in_subgroup(&p) {
                ok += 1;
            }
        }
        ok
    }

    /// As duas, que e o que uma desserializacao segura faz de verdade.
    pub fn validar_n(env: Env, ps: Vec<Bls12381G1Affine>) -> u32 {
        let bls = env.crypto().bls12_381();
        let mut ok = 0u32;
        for p in ps.iter() {
            if bls.g1_is_on_curve(&p) && bls.g1_is_in_subgroup(&p) {
                ok += 1;
            }
        }
        ok
    }

    // ---------- Sonda 9 (A1): TTL e arquivamento de estado ----------
    //
    // Entradas persistentes do Soroban sao ARQUIVADAS quando o TTL expira, e
    // uma transacao cuja footprint referencia uma entrada arquivada FALHA ate
    // alguem restaurar. O acumulador `Acum(id, j)` e as n entradas `Votou`
    // sao persistentes. Se elas arquivarem, a apuracao e o verificador quebram.

    /// Escreve uma entrada persistente do tamanho de um acumulador (96 bytes).
    pub fn acum_escrever(env: Env, id: u32, c: BytesN<96>) {
        env.storage().persistent().set(&id, &c);
    }

    // NOTA: o contrato NAO consegue ler o proprio TTL — nao ha `get_ttl` em
    // `storage().persistent()` no SDK 28. O TTL e lido FORA da cadeia, por
    // `getLedgerEntries`, no campo `liveUntilLedgerSeq`. O contrato so estende.

    /// Teto de TTL da rede, em ledgers.
    pub fn max_ttl(env: Env) -> u32 {
        env.storage().max_ttl()
    }

    /// Ledger atual.
    pub fn ledger_atual(env: Env) -> u32 {
        env.ledger().sequence()
    }

    /// Estende o TTL ao maximo da rede — o que `abrir()` teria de fazer para
    /// toda entrada que a apuracao e o verificador precisarao depois.
    pub fn acum_estender(env: Env, id: u32) -> u32 {
        let m = env.storage().max_ttl();
        env.storage().persistent().extend_ttl(&id, m - 1, m);
        m
    }

    // ====================== SONDA 13 (C3): caminho de Merkle ===============
    //
    // O ultimo item do orcamento de votar() que ainda era ESTIMATIVA (~1M no
    // PROTOCOLO §7.2). A folha e H(0x00 || endereco || peso_be): o contrato
    // RECALCULA a folha, entao um peso inflado produz outra folha e o caminho
    // nao fecha. Essa e a defesa inteira contra peso falso (PROTOCOLO §6.4).
    //
    // Separacao de dominio entre folha (0x00) e no (0x01): sem ela, uma folha
    // de 64 bytes bem escolhida seria apresentada como no interno.

    /// Confere o caminho de Merkle de um apto ate a raiz da proposta.
    pub fn verify_merkle(
        env: Env,
        endereco: BytesN<32>,
        peso: u32,
        indice: u32,
        irmaos: Vec<BytesN<32>>,
        raiz: BytesN<32>,
    ) -> bool {
        // folha = sha256(0x00 || endereco || peso_be)
        let mut buf = Bytes::from_slice(&env, &[0x00u8]);
        buf.extend_from_array(&endereco.to_array());
        buf.extend_from_array(&peso.to_be_bytes());
        let mut atual = env.crypto().sha256(&buf).to_bytes();

        let mut i = indice;
        for irmao in irmaos.iter() {
            let mut b = Bytes::from_slice(&env, &[0x01u8]);
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
        // o indice tem de ter se esgotado: um indice maior que a arvore seria
        // outro caminho para a mesma raiz.
        i == 0 && atual == raiz
    }
}

fn desafio_fr(
    env: &Env,
    c: &Bls12381G1Affine,
    a0: &Bls12381G1Affine,
    a1: &Bls12381G1Affine,
) -> Bls12381Fr {
    let mut buf = Bytes::from_slice(env, DST_DESAFIO);
    buf.extend_from_array(&c.to_array());
    buf.extend_from_array(&a0.to_array());
    buf.extend_from_array(&a1.to_array());
    let mut e = env.crypto().sha256(&buf).to_array();
    // from_bytes nao reduz modulo r; zerar o byte alto garante e < 2^248 < r.
    // Seguro para um DESAFIO. Seria errado para o acaso do compromisso.
    e[0] = 0;
    Bls12381Fr::from_bytes(BytesN::from_array(env, &e))
}

fn u32_fr(env: &Env, v: u32) -> Bls12381Fr {
    Bls12381Fr::from_u256(soroban_sdk::U256::from_u32(env, v))
}

fn challenge_fr(env: &Env, g: &Bls12381G1Affine, pk: &Bls12381G1Affine, a: &Bls12381G1Affine) -> Bls12381Fr {
    let mut buf = Bytes::from_slice(env, DST);
    buf.extend_from_array(&g.to_array());
    buf.extend_from_array(&pk.to_array());
    buf.extend_from_array(&a.to_array());

    let mut e = env.crypto().sha256(&buf).to_array();
    // Bls12381Fr::from_bytes nao reduz modulo r; zerar o byte alto garante e < 2^248 < r.
    e[0] = 0;
    Bls12381Fr::from_bytes(BytesN::from_array(env, &e))
}

#[cfg(test)]
mod test;
