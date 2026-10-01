extern crate std;

use super::{BlsSmoke, BlsSmokeClient};
use soroban_sdk::{
    bytes, bytesn,
    crypto::bls12_381::{Bls12381Fr as Fr, Bls12381G1Affine as G1},
    testutils::arbitrary::std::println,
    Bytes, Env, Vec, U256,
};

/// Teto de CPU por transacao, MEDIDO empiricamente na testnet em 2026-09-30
/// por bisseccao com `add_n`: n=3500 (~387M) passa, n=3600 (~398M) devolve
/// `HostError: Error(Budget, ExceededLimit)`. Logo o teto e 400M, nao os 100M
/// que eu havia assumido.
const TX_CPU_LIMIT: u64 = 400_000_000;

/// Gerador canonico de G1, uncompressed: be_bytes(X) || be_bytes(Y).
fn generator(env: &Env) -> G1 {
    G1::from_bytes(bytesn!(
        env,
        0x17f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb08b3f481e3aaa0f1a09e30ed741d8ae4fcf5e095d5d00af600db18cb2c04b3edd03cc744a2888ae40caa232946c5e7e1
    ))
}

fn fr(env: &Env, v: u32) -> Fr {
    Fr::from_u256(U256::from_u32(env, v))
}

fn setup() -> (Env, BlsSmokeClient<'static>) {
    let env = Env::default();
    let id = env.register(BlsSmoke, ());
    let client = BlsSmokeClient::new(&env, &id);
    (env, client)
}

/// Mede o custo de CPU de um fechamento, isolando-o do que veio antes.
fn cpu<T>(env: &Env, f: impl FnOnce() -> T) -> u64 {
    let mut b = env.cost_estimate().budget();
    b.reset_unlimited();
    let _ = f();
    env.cost_estimate().budget().cpu_instruction_cost()
}

// ---------------------------------------------------------------- sonda 1

#[test]
fn sonda1_serializacao_do_gerador() {
    let (env, client) = setup();
    let g = generator(&env);

    let (on_curve, in_subgroup) = client.probe(&g);

    println!("\n== SONDA 1: serializacao ==");
    println!("gerador canonico na curva ........ {}", on_curve);
    println!("gerador canonico no subgrupo ..... {}", in_subgroup);

    assert!(on_curve, "host rejeitou be_bytes(X)||be_bytes(Y) — encoding diverge");
    assert!(in_subgroup);
}

#[test]
#[should_panic]
fn sonda1_bytes_invalidos_trapam() {
    let (env, client) = setup();
    // 96 bytes que nao sao um ponto: from_bytes nao valida, o host trapa.
    let lixo = G1::from_array(&env, &[1u8; 96]);
    client.probe(&lixo);
}

// ---------------------------------------------------------------- sonda 3

#[test]
fn sonda3_schnorr_verifica_on_chain() {
    let (env, client) = setup();
    let bls = env.crypto().bls12_381();
    let g = generator(&env);

    // chaves da mesa
    let sk = fr(&env, 7);
    let pk = bls.g1_mul(&g, &sk);

    // provador: k -> A = k*G ; e = H(...) ; z = k + e*sk
    let k = fr(&env, 424242);
    let a = bls.g1_mul(&g, &k);
    let e = client.challenge(&g, &pk, &a);
    let z = bls.fr_add(&k, &bls.fr_mul(&e, &sk));

    assert!(client.verify_schnorr(&g, &pk, &a, &z), "prova valida recusada");

    // prova adulterada tem de cair
    let z_ruim = bls.fr_add(&z, &fr(&env, 1));
    assert!(!client.verify_schnorr(&g, &pk, &a, &z_ruim), "prova forjada aceita");

    println!("\n== SONDA 3: sigma-protocolo ==");
    println!("prova valida aceita, prova forjada recusada. OK");
}

// ---------------------------------------------------------------- sonda 4

#[test]
fn sonda4_apuracao_homomorfica_fecha() {
    let (env, client) = setup();
    let bls = env.crypto().bls12_381();
    let g = generator(&env);

    let sk = fr(&env, 31337);
    let pk = bls.g1_mul(&g, &sk);

    // tres cedulas com pesos diferentes (acionista grande, medio, pequeno)
    let pesos: [u32; 3] = [5, 3, 1];
    let aleatorios: [u32; 3] = [11, 22, 33];

    let mut c1s = Vec::new(&env);
    let mut c2s = Vec::new(&env);
    for i in 0..3 {
        let r = fr(&env, aleatorios[i]);
        let m = fr(&env, pesos[i]);
        let c1 = bls.g1_mul(&g, &r);
        let c2 = bls.g1_add(&bls.g1_mul(&pk, &r), &bls.g1_mul(&g, &m));
        c1s.push_back(c1);
        c2s.push_back(c2);
    }

    let total = client.tally(&g, &c1s, &c2s, &sk, &32);

    println!("\n== SONDA 4: apuracao ==");
    println!("pesos cifrados ... {:?}", pesos);
    println!("total apurado .... {}", total);

    assert_eq!(total, 9, "agregacao homomorfica nao fechou");
}

// ---------------------------------------------------------------- sonda 2

#[test]
fn sonda2_custo_e_projecao_de_orcamento() {
    let (env, client) = setup();
    let bls = env.crypto().bls12_381();
    let g = generator(&env);
    let s = fr(&env, 12345);
    let p2 = bls.g1_mul(&g, &fr(&env, 99));

    // custo por operacao = (custo com 11 - custo com 1) / 10, cancelando o
    // overhead de invocacao do contrato.
    let add1 = cpu(&env, || client.add_n(&g, &p2, &1));
    let add11 = cpu(&env, || client.add_n(&g, &p2, &11));
    let c_add = (add11 - add1) / 10;

    let mul1 = cpu(&env, || client.mul_n(&g, &s, &1));
    let mul11 = cpu(&env, || client.mul_n(&g, &s, &11));
    let c_mul = (mul11 - mul1) / 10;

    // msm de tamanho 8 vs 2, para ver a economia de escala
    let mut ps2 = Vec::new(&env);
    let mut ss2 = Vec::new(&env);
    let mut ps8 = Vec::new(&env);
    let mut ss8 = Vec::new(&env);
    for i in 0..8u32 {
        let p = bls.g1_mul(&g, &fr(&env, i + 2));
        let sc = fr(&env, i + 100);
        if i < 2 {
            ps2.push_back(p.clone());
            ss2.push_back(sc.clone());
        }
        ps8.push_back(p);
        ss8.push_back(sc);
    }
    let c_msm2 = cpu(&env, || client.msm(&ps2, &ss2));
    let c_msm8 = cpu(&env, || client.msm(&ps8, &ss8));

    let msg = bytes!(&env, 0xdeadbeef);
    let c_h1 = cpu(&env, || client.hash_g1(&msg));

    // pairing check de um par (custo do caminho Groth16, nao da urna)
    let q = client.hash_g2(&msg);
    let mut pv = Vec::new(&env);
    let mut qv = Vec::new(&env);
    pv.push_back(g.clone());
    qv.push_back(q);
    let c_pair = cpu(&env, || client.pairing(&pv, &qv));

    // sigma-protocolo completo (2 mul + 1 add + sha256 + overhead)
    let sk = fr(&env, 7);
    let pk = bls.g1_mul(&g, &sk);
    let k = fr(&env, 424242);
    let a = bls.g1_mul(&g, &k);
    let e = client.challenge(&g, &pk, &a);
    let z = bls.fr_add(&k, &bls.fr_mul(&e, &sk));
    let c_schnorr = cpu(&env, || client.verify_schnorr(&g, &pk, &a, &z));

    // Prova disjuntiva de boa-formacao da cedula (m em {0,1}), 2 ramos,
    // estimada a partir das primitivas medidas: 8 mul + 6 add + 1 hash.
    let c_cedula = 8 * c_mul + 6 * c_add + (c_schnorr - 2 * c_mul - c_add);
    let cedulas_por_tx = TX_CPU_LIMIT / c_cedula.max(1);

    println!("\n== SONDA 2: custo de CPU (instrucoes) ==");
    println!("g1_add ........................... {}", c_add);
    println!("g1_mul ........................... {}", c_mul);
    println!("g1_msm (k=2) ..................... {}", c_msm2);
    println!("g1_msm (k=8) ..................... {}", c_msm8);
    println!("hash_to_g1 ....................... {}", c_h1);
    println!("pairing_check (1 par) ............ {}", c_pair);
    println!("verify_schnorr (invocacao cheia) . {}", c_schnorr);
    println!("\n-- projecao --");
    println!("teto por tx (medido na testnet) ... {}", TX_CPU_LIMIT);
    println!("prova de cedula estimada ......... {}", c_cedula);
    println!("cedulas verificaveis por tx ...... {}", cedulas_por_tx);
    println!(
        "veredito ......................... {}",
        if cedulas_por_tx >= 10 {
            "PLANO A folgado"
        } else if cedulas_por_tx >= 1 {
            "PLANO A com verificacao amortizada / em lote"
        } else {
            "PLANO B — verificacao fora da cadeia"
        }
    );

    assert!(c_mul > 0 && c_add > 0, "medicao nao isolou as operacoes");
    assert!(cedulas_por_tx >= 1, "nao cabe uma cedula por transacao");
}

#[test]
fn sonda5_verificar_e_barato_procurar_nao() {
    let (env, client) = setup();
    let bls = env.crypto().bls12_381();
    let g = generator(&env);
    let sk = fr(&env, 31337);
    let pk = bls.g1_mul(&g, &sk);

    // uma cedula de peso alto: simula um acionista relevante
    let peso: u32 = 250;
    let r = fr(&env, 77);
    let mut c1s = Vec::new(&env);
    let mut c2s = Vec::new(&env);
    c1s.push_back(bls.g1_mul(&g, &r));
    c2s.push_back(bls.g1_add(&bls.g1_mul(&pk, &r), &bls.g1_mul(&g, &fr(&env, peso))));

    let c_busca = cpu(&env, || client.tally(&g, &c1s, &c2s, &sk, &(peso + 1)));
    let c_check = cpu(&env, || client.tally_checked(&g, &c1s, &c2s, &sk, &peso));

    assert!(client.tally_checked(&g, &c1s, &c2s, &sk, &peso));
    assert!(!client.tally_checked(&g, &c1s, &c2s, &sk, &(peso + 1)));

    let por_unidade = c_busca / peso as u64;
    println!("\n== SONDA 5: apurar por busca vs por verificacao ==");
    println!("tally por forca bruta (peso {}) ... {}", peso, c_busca);
    println!("  custo marginal por unidade ...... {}", por_unidade);
    println!("tally_checked (total asseverado) .. {}", c_check);
    println!("  razao ........................... {}x", c_busca / c_check.max(1));
    println!(
        "  peso total maximo por busca ..... ~{} (teto {} )",
        TX_CPU_LIMIT / por_unidade.max(1),
        TX_CPU_LIMIT
    );
    // prova de que o custo de VERIFICAR nao depende do peso, e o de PROCURAR sim:
    // mesma cedula, peso um milhao de vezes maior, custo praticamente igual.
    let peso_grande: u32 = 1_000_000;
    let mut d1 = Vec::new(&env);
    let mut d2 = Vec::new(&env);
    let r2 = fr(&env, 91);
    d1.push_back(bls.g1_mul(&g, &r2));
    d2.push_back(bls.g1_add(
        &bls.g1_mul(&pk, &r2),
        &bls.g1_mul(&g, &fr(&env, peso_grande)),
    ));
    let c_check_grande = cpu(&env, || client.tally_checked(&g, &d1, &d2, &sk, &peso_grande));
    assert!(client.tally_checked(&g, &d1, &d2, &sk, &peso_grande));

    println!("tally_checked (peso 1.000.000) .... {}", c_check_grande);
    let teto_busca = TX_CPU_LIMIT / por_unidade.max(1);

    assert!(c_check < c_busca, "verificacao deveria ser mais barata que a busca");
    assert!(
        c_check_grande < c_check * 2,
        "custo de verificar deveria ser ~constante no peso"
    );
    assert!(
        teto_busca < 10_000, // ainda muito abaixo de uma assembleia real
        "se a busca aguentasse assembleia real, a conclusao arquitetural muda"
    );
}

/// Emite os vetores de teste para invocacao real na testnet via CLI.
/// Nao e assercao: e um gerador. Rodar com --nocapture.
#[test]
fn emitir_vetores_para_testnet() {
    let (env, client) = setup();
    let bls = env.crypto().bls12_381();
    let g = generator(&env);

    fn hx(b: &[u8]) -> std::string::String {
        use std::fmt::Write;
        let mut s = std::string::String::new();
        for x in b {
            write!(s, "{:02x}", x).unwrap();
        }
        s
    }

    // --- Schnorr ---
    let sk = fr(&env, 7);
    let pk = bls.g1_mul(&g, &sk);
    let k = fr(&env, 424242);
    let a = bls.g1_mul(&g, &k);
    let e = client.challenge(&g, &pk, &a);
    let z = bls.fr_add(&k, &bls.fr_mul(&e, &sk));

    println!("\n=== VETORES TESTNET ===");
    println!("G_HEX={}", hx(&g.to_array()));
    println!("PK_HEX={}", hx(&pk.to_array()));
    println!("A_HEX={}", hx(&a.to_array()));
    println!("Z_HEX={}", hx(&z.to_bytes().to_array()));

    // --- ElGamal / apuracao ---
    let sk2 = fr(&env, 31337);
    let pk2 = bls.g1_mul(&g, &sk2);
    let pesos: [u32; 3] = [5, 3, 1];
    let rands: [u32; 3] = [11, 22, 33];
    for i in 0..3 {
        let r = fr(&env, rands[i]);
        let c1 = bls.g1_mul(&g, &r);
        let c2 = bls.g1_add(&bls.g1_mul(&pk2, &r), &bls.g1_mul(&g, &fr(&env, pesos[i])));
        println!("C1_{}={}", i, hx(&c1.to_array()));
        println!("C2_{}={}", i, hx(&c2.to_array()));
    }
    println!("SK2_DEC=31337");
    println!("TOTAL=9");
}

/// SONDA 6: pairing_check com varios pares — o numero que decide se Groth16
/// cabe on-chain, e portanto se a dica de ZK da SDF e viavel.
///
/// A verificacao Groth16 se reduz a UM produto de pareamentos com 4 pares
/// (negando um lado da equacao), mais um MSM sobre as entradas publicas.
/// A exponenciacao final do pareamento e compartilhada entre os pares, logo
/// o custo marginal do 2o ao 4o par deve ser MUITO menor que o do 1o. Se for,
/// ZK e questao de engenharia, nao de orcamento.
#[test]
fn sonda6_pairing_multiplo_e_orcamento_groth16() {
    let (env, client) = setup();
    let g = generator(&env);
    let bls = env.crypto().bls12_381();

    let mut ps = Vec::new(&env);
    let mut qs = Vec::new(&env);
    let mut custo = [0u64; 5];

    for k in 1..=4u32 {
        ps.push_back(bls.g1_mul(&g, &fr(&env, k + 1)));
        qs.push_back(client.hash_g2(&Bytes::from_array(&env, &[k as u8, 0xAB])));
        let (pc, qc) = (ps.clone(), qs.clone());
        custo[k as usize] = cpu(&env, || client.pairing(&pc, &qc));
    }

    println!("\n== SONDA 6: pairing_check por numero de pares ==");
    for k in 1..=4usize {
        let marginal = if k == 1 { 0 } else { custo[k] - custo[k - 1] };
        println!(
            "k={} .......... {:>12}   marginal {:>12}",
            k, custo[k], marginal
        );
    }

    // base e marginal do pareamento, por regressao de 2 pontos
    let marg_pair = (custo[4] - custo[1]) / 3;
    let base_pair = custo[1].saturating_sub(marg_pair);

    // MSM das entradas publicas: base e marginal medidos como na sonda 2
    let mut p2 = Vec::new(&env);
    let mut s2 = Vec::new(&env);
    let mut p8 = Vec::new(&env);
    let mut s8 = Vec::new(&env);
    for i in 0..8u32 {
        let pt = bls.g1_mul(&g, &fr(&env, i + 3));
        let sc = fr(&env, i + 100);
        if i < 2 {
            p2.push_back(pt.clone());
            s2.push_back(sc.clone());
        }
        p8.push_back(pt);
        s8.push_back(sc);
    }
    let c_msm2 = cpu(&env, || client.msm(&p2, &s2));
    let c_msm8 = cpu(&env, || client.msm(&p8, &s8));
    let marg_msm = (c_msm8 - c_msm2) / 6;
    let base_msm = c_msm2.saturating_sub(2 * marg_msm);

    // Groth16: pairing_check de 4 pares + MSM de (n_pub + 1) termos
    for n_pub in [2u64, 4, 8] {
        let msm = base_msm + (n_pub + 1) * marg_msm;
        let total = custo[4] + msm;
        println!(
            "groth16 verify, {} entradas publicas .. {:>12}  ({:.1}% do teto, {} por tx)",
            n_pub,
            total,
            100.0 * total as f64 / TX_CPU_LIMIT as f64,
            TX_CPU_LIMIT / total.max(1)
        );
    }
    println!("-- pairing: base {} + {} por par", base_pair, marg_pair);
    println!("-- msm:     base {} + {} por termo", base_msm, marg_msm);

    // O fato que importa: o marginal por par e muito menor que o primeiro par.
    assert!(
        marg_pair * 2 < custo[1],
        "esperava exponenciacao final compartilhada: marginal {} vs 1o par {}",
        marg_pair,
        custo[1]
    );
    // E Groth16 com 4 entradas publicas tem de caber com folga no teto.
    let g16 = custo[4] + base_msm + 5 * marg_msm;
    assert!(
        g16 * 4 < TX_CPU_LIMIT,
        "groth16 {} nao cabe com folga 4x em {}",
        g16,
        TX_CPU_LIMIT
    );
}

/// SONDA 7 (smoke B1) — Pedersen e a abertura do agregado.
///
/// A sonda 4 mediu ElGamal. O SPEC especifica Pedersen, e ate agora o desenho
/// final NUNCA tinha rodado. Este teste o executa pela primeira vez:
///
///   C_i = v_i*G + r_i*H          compromisso perfeitamente ocultante
///   A   = sum C_i                 o acumulador do contrato
///   A  == T*G + R*H               a mesa abre o AGREGADO, nao decifra
///
/// Casos negativos sao metade do teste: um total falso tem de ser recusado,
/// senao nao ha integridade.
#[test]
fn sonda7_pedersen_abertura_do_agregado_fecha() {
    let (env, client) = setup();
    let g = generator(&env);
    let bls = env.crypto().bls12_381();

    // --- H: nothing-up-my-sleeve, e tem de ser um gerador de verdade
    let h = client.gerador_h();
    assert!(bls.g1_is_on_curve(&h), "H fora da curva");
    assert!(bls.g1_is_in_subgroup(&h), "H fora do subgrupo de ordem r");
    assert!(h != g, "H == G: compromisso deixaria de ser ocultante");
    assert_eq!(h, client.gerador_h(), "H nao e deterministico");

    // H MEDIDO NA TESTNET em 2026-10-01 (contrato CCC4KNBC...DGKY).
    // Se este assert quebrar, o provador off-chain e o verificador on-chain
    // derivam H diferente e NENHUMA prova verifica. E o bug de meio dia que o
    // smoke B2 existe para evitar.
    assert_eq!(
        h,
        G1::from_bytes(bytesn!(
            &env,
            0x1462b4b57a7d01e598685e913608bab6990de8cce1c705642c19a6b9ed660fc589758bdebb20c2bdde65a25b35b12206198669dff273abe7b9e6a0b3e636f8a8a41f9c81f6a79d308ec0fe57ef8a073c90e444002ba59807f987cce67d72eccf
        )),
        "H divergiu do valor da testnet"
    );

    // --- 5 votantes. Dois votam 0: exercita o escalar zero no MSM, que e
    //     metade das cedulas numa votacao binaria real.
    let votos: [u32; 5] = [1, 0, 1, 1, 0];
    let acasos: [u32; 5] = [101, 202, 303, 404, 505];

    let mut cs = Vec::new(&env);
    let mut r_total = fr(&env, 0);
    let mut t_total: u32 = 0;
    for i in 0..5 {
        let r = fr(&env, acasos[i]);
        cs.push_back(client.commit(&g, &h, &votos[i], &r));
        r_total = bls.fr_add(&r_total, &r);
        t_total += votos[i];
    }

    // --- o acumulador do contrato
    let a = client.agregar(&cs);

    // --- a mesa afirma (3, R) e o contrato confere
    assert!(
        client.verify_aggregate(&a, &g, &h, &t_total, &r_total),
        "abertura honesta do agregado NAO fecha"
    );

    // --- negativos: a mesa mentindo
    assert!(
        !client.verify_aggregate(&a, &g, &h, &(t_total + 1), &r_total),
        "total+1 passou: nao ha integridade"
    );
    assert!(
        !client.verify_aggregate(&a, &g, &h, &(t_total - 1), &r_total),
        "total-1 passou: nao ha integridade"
    );
    let r_errado = bls.fr_add(&r_total, &fr(&env, 1));
    assert!(
        !client.verify_aggregate(&a, &g, &h, &t_total, &r_errado),
        "soma_r+1 passou: nao ha integridade"
    );

    // --- caso de borda: ninguem votou nesta opcao, T = 0
    let mut cs0 = Vec::new(&env);
    let mut r0 = fr(&env, 0);
    for i in 0..3u32 {
        let r = fr(&env, 900 + i);
        cs0.push_back(client.commit(&g, &h, &0u32, &r));
        r0 = bls.fr_add(&r0, &r);
    }
    let a0 = client.agregar(&cs0);
    assert!(
        client.verify_aggregate(&a0, &g, &h, &0u32, &r0),
        "opcao com zero votos NAO fecha (escalar zero no MSM)"
    );

    // --- homomorfismo explicito: C(1,r1) + C(1,r2) == C(2, r1+r2)
    let r1 = fr(&env, 7777);
    let r2 = fr(&env, 8888);
    let soma = bls.g1_add(
        &client.commit(&g, &h, &1u32, &r1),
        &client.commit(&g, &h, &1u32, &r2),
    );
    assert_eq!(
        soma,
        client.commit(&g, &h, &2u32, &bls.fr_add(&r1, &r2)),
        "compromisso nao e aditivamente homomorfico"
    );

    // --- custo
    let c_commit = cpu(&env, || client.commit(&g, &h, &1u32, &r1));
    let c_verify = cpu(&env, || {
        client.verify_aggregate(&a, &g, &h, &t_total, &r_total)
    });
    let c_h = cpu(&env, || client.gerador_h());

    println!("\n== SONDA 7 (B1): Pedersen + abertura do agregado ==");
    println!("gerador_h (so na inicializacao) .. {}", c_h);
    println!("commit (cliente, fora da cadeia) . {}", c_commit);
    println!("verify_aggregate (a apuracao) .... {}", c_verify);
    println!("   ElGamal tally_checked, sonda 5 . 6712183");
    println!(
        "   diferenca ..................... {} ({}%)",
        6712183i64 - c_verify as i64,
        (100 * c_verify) / 6712183
    );
    println!("teto por tx ...................... {}", TX_CPU_LIMIT);
    println!(
        "apuracoes por tx ................. {}",
        TX_CPU_LIMIT / c_verify.max(1)
    );

    // Criterio do smoke B1: <= 8M (projetado 5,4M no SPEC §7.2).
    assert!(
        c_verify <= 8_000_000,
        "verify_aggregate custou {}, acima do teto de 8M do smoke",
        c_verify
    );
}

/// SONDA 10 (smoke C1) — quanto custa validar um ponto que chega por argumento.
///
/// Todo G1 recebido precisa ser conferido on-curve e in-subgroup, senao ha
/// ataque de subgrupo pequeno. Sao ~6 pontos por `votar()`. A checagem de
/// subgrupo e a suspeita: ingenuamente ela e uma multiplicacao escalar.
///
/// Isolamento por diferenca: (custo com n=11 menos custo com n=1) / 10,
/// que cancela o overhead de invocacao.
#[test]
fn sonda10_desserializacao_e_checagem_de_subgrupo() {
    let (env, client) = setup();
    let g = generator(&env);
    let bls = env.crypto().bls12_381();

    let mut p1 = Vec::new(&env);
    let mut p11 = Vec::new(&env);
    for i in 0..11u32 {
        let p = bls.g1_mul(&g, &fr(&env, i + 2));
        if i < 1 {
            p1.push_back(p.clone());
        }
        p11.push_back(p);
    }

    let marg = |a: u64, b: u64| (b - a) / 10;

    let c_curve = marg(
        cpu(&env, || client.on_curve_n(&p1)),
        cpu(&env, || client.on_curve_n(&p11)),
    );
    let c_sub = marg(
        cpu(&env, || client.in_subgroup_n(&p1)),
        cpu(&env, || client.in_subgroup_n(&p11)),
    );
    let c_ambos = marg(
        cpu(&env, || client.validar_n(&p1)),
        cpu(&env, || client.validar_n(&p11)),
    );

    // referencia: um g1_mul, para saber se a checagem de subgrupo e uma delas
    let c_mul = marg(
        cpu(&env, || client.mul_n(&g, &fr(&env, 3), &1)),
        cpu(&env, || client.mul_n(&g, &fr(&env, 3), &11)),
    );

    println!("\n== SONDA 10 (C1): validacao de ponto, por ponto ==");
    println!("g1_is_on_curve ............... {:>10}", c_curve);
    println!("g1_is_in_subgroup ............ {:>10}", c_sub);
    println!("as duas juntas ............... {:>10}", c_ambos);
    println!("   referencia: g1_mul ........ {:>10}", c_mul);
    println!(
        "   subgrupo / mul ............ {:.2}x",
        c_sub as f64 / c_mul as f64
    );
    println!("\n-- impacto em votar() (m=2) --");
    let pontos_por_voto = 6u64;
    let custo_val = pontos_por_voto * c_ambos;
    println!("6 pontos validados ........... {:>10}", custo_val);
    println!(
        "   % de uma transacao ........ {:.1}%",
        100.0 * custo_val as f64 / TX_CPU_LIMIT as f64
    );
    println!("\n-- impacto em Groth16 (o aviso do Tyler) --");
    // Groth16: 3 pontos G1 na prova (A, C e o agregado de entradas) + 2 G2.
    // So os G1 aqui; G2 e mais caro e fica para quando houver circuito.
    println!("3 pontos G1 da prova ......... {:>10}", 3 * c_ambos);
    println!(
        "   verify medido 47.371.348 -> {:>10} com validacao",
        47_371_348 + 3 * c_ambos
    );

    // O criterio do smoke: validar 6 pontos nao pode comer o orcamento.
    assert!(
        custo_val * 10 < TX_CPU_LIMIT,
        "validar 6 pontos custa {}, mais de 10% do teto",
        custo_val
    );
}

/// SONDA 11 (smoke B2) — `H` e um gerador honesto?
///
/// O compromisso so e perfeitamente ocultante se ninguem souber `h` tal que
/// `H = h*G`. Nao da para PROVAR que ninguem sabe — esse e o ponto de um
/// nothing-up-my-sleeve. Da para provar que H nao e um multiplo TRIVIAL de G,
/// que e como o desenho quebraria na pratica se o DST fosse mal escolhido.
#[test]
fn sonda11_h_e_gerador_honesto() {
    let (env, client) = setup();
    let g = generator(&env);
    let bls = env.crypto().bls12_381();
    let h = client.gerador_h();

    // na curva, no subgrupo de ordem r
    assert!(bls.g1_is_on_curve(&h));
    assert!(bls.g1_is_in_subgroup(&h));

    // nao e G, nem -G, nem o ponto no infinito
    let identidade = bls.g1_add(&g, &(-g.clone()));
    assert!(h != g, "H == G");
    assert!(h != -g.clone(), "H == -G");
    assert!(h != identidade, "H e a identidade");

    // nao e k*G para k pequeno: se o DST tivesse dado azar e produzido um
    // multiplo conhecido, o compromisso seria abrivel por qualquer um.
    let mut kg = g.clone();
    for k in 1..=512u32 {
        assert!(h != kg, "H == {}*G — DST produziu multiplo trivial", k);
        assert!(h != -kg.clone(), "H == -{}*G", k);
        kg = bls.g1_add(&kg, &g);
    }

    // o DST participa de verdade: trocar a string troca o ponto
    let outro = client.hash_g1(&Bytes::from_array(&env, b"TESSERA-V1-GENERATOR-H"));
    assert!(
        outro != h,
        "DST nao participa — hash_to_g1 ignora a tag e H nao e separado por dominio"
    );

    println!("\n== SONDA 11 (B2): H e gerador honesto ==");
    println!("on-curve ............ sim");
    println!("in-subgroup ......... sim");
    println!("!= k*G, k em 1..512 . sim");
    println!("DST participa ....... sim");
    println!("vetor da testnet .... travado em sonda7");
}

/// SONDA 12 (smoke B4) — a prova disjuntiva CDS, verificada no contrato.
///
/// O vetor abaixo foi gerado pelo `core` em Rust NATIVO (arkworks) e e
/// verificado aqui em Wasm pelo host do Soroban. Se fechar, provador e
/// verificador concordam no caminho CDS — que e o smoke B3, e e onde a
/// maioria dos projetos de cripto quebra.
///
/// Voto v=1, acaso r=31337.
#[test]
fn sonda12_cds_verifica_e_custa() {
    let (env, client) = setup();
    let g = generator(&env);
    let h = client.gerador_h();

    let c = G1::from_bytes(bytesn!(&env, 0x191fab693fe01641555a088dd540a16d9ee62312feb3f752ffa55ebba270cfc49c96b4ad1ebd7098f21995333839ea8b1742141021a3d2c3834c415e63cbeeff2236cb95127faa339c7a05e21181eaa28f3b80e64bd506262cd1d45f7ce6df7a));
    let a0 = G1::from_bytes(bytesn!(&env, 0x153730e4e4a5d57c774119f37fa07f4bec68741934ce1f4c5dad6584d8e63aa5ff78aa6543b2426e429230194cd40ce20be2e4d403ddfb1c5b195bd883a89a437ae7e77a033e04f4e0b418970764976cac0e48a81b8ed9705b54df2ad07bc72c));
    let a1 = G1::from_bytes(bytesn!(&env, 0x03a1e4a301a681cc0024c375fb86364da96ae773fb2d3f4131af1c3490651497f292926f822a491ac54f64edb3920ec10c813d8135b948f37c3716416bf115eed635fa362b6ba12ad28fe1a33a51e398aed4e711973dddb976e4de1898b08529));
    let e0 = Fr::from_bytes(bytesn!(&env, 0x709dce372550d44aa0b33083323149d14be674ea27181d945750c6619e7ac050));
    let z0 = Fr::from_bytes(bytesn!(&env, 0x540de293552c98c9363bcb6c028870b2021a5181c76d433fb7ceb8ac6d973580));
    let e1 = Fr::from_bytes(bytesn!(&env, 0x03a64ddf3ed244944e2b922db48781d277a780636c3c74f457fe4eec891d4770));
    let z1 = Fr::from_bytes(bytesn!(&env, 0x5225db38c6cf1177518ef0b1b16ddea0ead83e461a465d97d8de160394553cd8));

    // --- a prova do core verifica no contrato
    assert!(
        client.verify_cds(&g, &h, &c, &a0, &a1, &e0, &z0, &e1, &z1),
        "prova gerada pelo core NAO verifica no contrato: provador e \
         verificador divergem"
    );

    // --- adulteracoes tem de ser recusadas
    let um = fr(&env, 1);
    let bls = env.crypto().bls12_381();
    assert!(
        !client.verify_cds(&g, &h, &c, &a0, &a1, &bls.fr_add(&e0, &um), &z0, &e1, &z1),
        "e0 adulterado passou"
    );
    assert!(
        !client.verify_cds(&g, &h, &c, &a0, &a1, &e0, &bls.fr_add(&z0, &um), &e1, &z1),
        "z0 adulterado passou"
    );
    assert!(
        !client.verify_cds(&g, &h, &c, &a1, &a0, &e0, &z0, &e1, &z1),
        "ramos trocados passaram"
    );
    // a prova esta presa ao SEU compromisso
    let outro_c = client.commit(&g, &h, &1u32, &fr(&env, 99));
    assert!(
        !client.verify_cds(&g, &h, &outro_c, &a0, &a1, &e0, &z0, &e1, &z1),
        "prova migrou de compromisso"
    );

    // --- custo
    let c_desafio = cpu(&env, || client.desafio_cds(&c, &a0, &a1));
    let c_cds = cpu(&env, || {
        client.verify_cds(&g, &h, &c, &a0, &a1, &e0, &z0, &e1, &z1)
    });

    println!("\n== SONDA 12 (B4): prova disjuntiva CDS ==");
    println!("desafio (sha256 + from_bytes) .... {:>10}", c_desafio);
    println!("verify_cds (invocacao cheia) ..... {:>10}", c_cds);
    println!("   projecao do SPEC era ~27M");
    println!("   erro da projecao .............. {:.0}%",
        100.0 * (27_000_000.0 - c_cds as f64) / 27_000_000.0);
    println!("\n-- orcamento de votar(), m=2 --");
    let votar = 2 * c_cds + 6_737_614 + 4_433_406 + 221_496;
    println!("2 CDS + soma + validacao + adds .. {:>10}", votar);
    println!("   % do teto de 400M ............. {:.1}%",
        100.0 * votar as f64 / TX_CPU_LIMIT as f64);
    println!("   folga ......................... {:.1}x",
        TX_CPU_LIMIT as f64 / votar as f64);

    // PORTAO 1 do PLANO: <=200M segue o plano sem corte.
    assert!(
        votar <= 200_000_000,
        "votar() custa {}, acima dos 200M do Portao 1",
        votar
    );
}
