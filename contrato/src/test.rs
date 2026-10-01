//! Testes do contrato.
//!
//! **O que separa estes testes de um mock:** as provas são geradas pelo
//! `tessera-core`, em Rust nativo com arkworks, e verificadas aqui pelo host do
//! Soroban, em Wasm. O cruzamento provador↔verificador (smoke B3) acontece a
//! cada `cargo test`, não uma vez num vetor congelado.

extern crate std;

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    xdr::ToXdr,
    Bytes, BytesN, Env, Vec,
};
use std::vec::Vec as Vetor;
use tessera_core::{cds, merkle, pedersen, ponto, soma};

use tessera_core::ark::{Fr as ArkFr, G1Affine as ArkG1};

// ---------- pontes entre arkworks e o SDK ----------

fn g1(env: &Env, p: &ArkG1) -> Bls12381G1Affine {
    Bls12381G1Affine::from_bytes(BytesN::from_array(env, &ponto::serializar(p)))
}

fn escalar(env: &Env, f: &ArkFr) -> Bls12381Fr {
    Bls12381Fr::from_bytes(BytesN::from_array(env, &pedersen::fr_para_bytes_be(f)))
}

fn bytes_de(b: &Bytes) -> Vetor<u8> {
    b.iter().collect()
}

/// Os mesmos bytes que `cripto::contexto` monta dentro do contrato.
fn ctx(env: &Env, proposta: &BytesN<32>, votante: &Address, opcao: u32) -> Vetor<u8> {
    let mut v: Vetor<u8> = proposta.to_array().to_vec();
    v.extend(bytes_de(&votante.clone().to_xdr(env)));
    v.extend_from_slice(&opcao.to_be_bytes());
    v
}

// ---------- cenário ----------

struct Cenario {
    env: Env,
    cliente: TesseraClient<'static>,
    proposta: BytesN<32>,
    aptos: Vetor<Address>,
    arvore: merkle::Arvore,
    mesa: Vetor<Address>,
    g: ArkG1,
    h: ArkG1,
}

const OPCOES: u32 = 2;
const FECHA_EM: u32 = 1000;

fn montar(n_aptos: usize) -> Cenario {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(10);

    let id = env.register(Tessera, ());
    let cliente = TesseraClient::new(&env, &id);

    let aptos: Vetor<Address> = (0..n_aptos).map(|_| Address::generate(&env)).collect();
    let folhas: Vetor<merkle::Apto> = aptos
        .iter()
        .map(|a| merkle::Apto {
            endereco: bytes_de(&a.clone().to_xdr(&env)),
            peso: 1,
        })
        .collect();
    let arvore = merkle::Arvore::montar(&folhas).unwrap();

    let mesa: Vetor<Address> = (0..5).map(|_| Address::generate(&env)).collect();
    let mut mesa_sdk = Vec::new(&env);
    for m in &mesa {
        mesa_sdk.push_back(m.clone());
    }

    let proposta: BytesN<32> = BytesN::from_array(&env, &[7u8; 32]);
    cliente.abrir(
        &Address::generate(&env),
        &proposta,
        &OPCOES,
        &BytesN::from_array(&env, &arvore.raiz()),
        &mesa_sdk,
        &3u32,
        &FECHA_EM,
    );

    let h = ponto::de_hex(&{
        let hs = cliente.gerador_h();
        let mut s = std::string::String::new();
        for b in hs.to_array().iter() {
            s.push_str(&std::format!("{:02x}", b));
        }
        s
    })
    .expect("H do contrato nao desserializa no core");

    Cenario {
        env,
        cliente,
        proposta,
        aptos,
        arvore,
        mesa,
        g: pedersen::gerador(),
        h,
    }
}

impl Cenario {
    fn caminho(&self, i: usize) -> (Vec<BytesN<32>>, u32) {
        let c = self.arvore.caminho(i).unwrap();
        let mut v = Vec::new(&self.env);
        for s in &c.irmaos {
            v.push_back(BytesN::from_array(&self.env, s));
        }
        (v, c.indice)
    }

    fn mesa_sdk(&self, k: usize) -> Vec<Address> {
        let mut v = Vec::new(&self.env);
        for m in self.mesa.iter().take(k) {
            v.push_back(m.clone());
        }
        v
    }

    /// Monta uma cédula confidencial honesta e devolve também os `r_j`, que no
    /// mundo real iriam para a mesa em shares de Shamir.
    fn cedula(&self, i: usize, escolha: u32) -> (Vec<Bls12381G1Affine>, Vec<ProvaCds>, ProvaSoma, Vetor<ArkFr>) {
        let env = &self.env;
        let votante = &self.aptos[i];

        let rs: Vetor<ArkFr> = (0..OPCOES).map(|_| pedersen::acaso_fr().unwrap()).collect();
        let cs: Vetor<ArkG1> = (0..OPCOES)
            .map(|j| {
                let v = if j == escolha { 1u64 } else { 0 };
                pedersen::comprometer(&self.g, &self.h, &pedersen::escalar(v), &rs[j as usize])
            })
            .collect();

        let mut compromissos = Vec::new(env);
        let mut provas = Vec::new(env);
        for j in 0..OPCOES {
            let v = if j == escolha { 1u64 } else { 0 };
            let p = cds::provar(
                &ctx(env, &self.proposta, votante, j),
                &self.g,
                &self.h,
                &cs[j as usize],
                v,
                &rs[j as usize],
            )
            .unwrap();
            compromissos.push_back(g1(env, &cs[j as usize]));
            provas.push_back(ProvaCds {
                a0: g1(env, &p.a0),
                a1: g1(env, &p.a1),
                e0: escalar(env, &p.e0),
                z0: escalar(env, &p.z0),
                e1: escalar(env, &p.e1),
                z1: escalar(env, &p.z1),
            });
        }

        let rho = rs.iter().fold(ArkFr::from(0u64), |a, r| a + r);
        let d = soma::alvo(&self.g, &cs, 1);
        let ps = soma::provar(
            &ctx(env, &self.proposta, votante, u32::MAX),
            &self.h,
            &d,
            &rho,
        )
        .unwrap();

        (
            compromissos,
            provas,
            ProvaSoma { a: g1(env, &ps.a), z: escalar(env, &ps.z) },
            rs,
        )
    }

    fn votar(&self, i: usize, escolha: u32) -> Vetor<ArkFr> {
        let (cs, provas, psoma, rs) = self.cedula(i, escolha);
        let (caminho, indice) = self.caminho(i);
        self.cliente
            .votar(&self.proposta, &self.aptos[i], &cs, &provas, &psoma, &caminho, &indice, &1u32);
        rs
    }
}

// ===================== a rodada completa =============================

/// **A rodada inteira, com criptografia de verdade do começo ao fim.**
///
/// Seis pessoas votam em sigilo: quatro na opção 0, duas na opção 1. A mesa
/// soma os `r` que recebeu, abre o agregado, e o contrato confere. O resultado
/// sai certo sem que nenhum voto individual tenha existido em lugar nenhum.
#[test]
fn rodada_completa_confidencial() {
    let c = montar(16);
    let escolhas = [0u32, 0, 1, 0, 1, 0];

    // cada pessoa vota; os r_j iriam para a mesa em shares de Shamir
    let mut soma_r = [ArkFr::from(0u64); OPCOES as usize];
    for (i, e) in escolhas.iter().enumerate() {
        let rs = c.votar(i, *e);
        for j in 0..OPCOES as usize {
            soma_r[j] += rs[j];
        }
        assert!(c.cliente.ja_votou(&c.proposta, &c.aptos[i]));
    }
    assert_eq!(c.cliente.comparecimento(&c.proposta), (6, 0));

    // a mesa reconstroi R_j por Shamir e publica (T_j, R_j)
    let totais_esperados = [4u32, 2];
    let mut totais = Vec::new(&c.env);
    let mut aberturas = Vec::new(&c.env);
    for j in 0..OPCOES as usize {
        totais.push_back(totais_esperados[j]);
        aberturas.push_back(escalar(&c.env, &soma_r[j]));
    }

    c.env.ledger().set_sequence_number(FECHA_EM + 1);
    let r = c.cliente.apurar(&c.proposta, &c.mesa_sdk(3), &totais, &aberturas);

    assert_eq!(r.get(0).unwrap(), 4);
    assert_eq!(r.get(1).unwrap(), 2);
    assert_eq!(c.cliente.resultado(&c.proposta).unwrap(), r);
}

/// **A mesa mentindo.** O compromisso é computacionalmente vinculante, então
/// abrir o mesmo `A` para outro total exigiria `log_G(H)`. Qualquer pessoa
/// detecta, e o contrato recusa antes de publicar.
#[test]
fn mesa_que_mente_no_total_e_recusada() {
    let c = montar(16);
    let mut soma_r = [ArkFr::from(0u64); OPCOES as usize];
    for i in 0..6 {
        let rs = c.votar(i, if i < 4 { 0 } else { 1 });
        for j in 0..OPCOES as usize {
            soma_r[j] += rs[j];
        }
    }
    c.env.ledger().set_sequence_number(FECHA_EM + 1);

    let ab = |env: &Env| {
        let mut v = Vec::new(env);
        for j in 0..OPCOES as usize {
            v.push_back(escalar(env, &soma_r[j]));
        }
        v
    };
    let tot = |env: &Env, a: u32, b: u32| {
        let mut v = Vec::new(env);
        v.push_back(a);
        v.push_back(b);
        v
    };

    // inverter o resultado mantendo a soma: 4,2 vira 2,4
    assert_eq!(
        c.cliente.try_apurar(&c.proposta, &c.mesa_sdk(3), &tot(&c.env, 2, 4), &ab(&c.env)),
        Err(Ok(Erro::AberturaNaoFecha))
    );
    // inflar um lado: a soma deixa de bater com o comparecimento
    assert_eq!(
        c.cliente.try_apurar(&c.proposta, &c.mesa_sdk(3), &tot(&c.env, 5, 2), &ab(&c.env)),
        Err(Ok(Erro::TotalDiferenteDoComparecimento))
    );
    // e o honesto passa
    let r = c.cliente.apurar(&c.proposta, &c.mesa_sdk(3), &tot(&c.env, 4, 2), &ab(&c.env));
    assert_eq!(r.get(0).unwrap(), 4);
}

/// Menos de `k` membros não apuram, e quem não é da mesa não apura.
#[test]
fn mesa_abaixo_do_limiar_nao_apura() {
    let c = montar(16);
    for i in 0..5 {
        c.votar(i, 0);
    }
    c.env.ledger().set_sequence_number(FECHA_EM + 1);

    let mut t = Vec::new(&c.env);
    t.push_back(5u32);
    t.push_back(0u32);
    let mut a = Vec::new(&c.env);
    a.push_back(escalar(&c.env, &ArkFr::from(0u64)));
    a.push_back(escalar(&c.env, &ArkFr::from(0u64)));

    assert_eq!(
        c.cliente.try_apurar(&c.proposta, &c.mesa_sdk(2), &t, &a),
        Err(Ok(Erro::MesaAbaixoDoLimiar))
    );

    // três assinantes, mas um não é da mesa
    let mut intrusa = c.mesa_sdk(2);
    intrusa.push_back(c.aptos[0].clone());
    assert_eq!(
        c.cliente.try_apurar(&c.proposta, &intrusa, &t, &a),
        Err(Ok(Erro::NaoEMembroDaMesa))
    );

    // três assinantes, mas o mesmo repetido — reduziria o limiar em silêncio
    let mut repetida = c.mesa_sdk(1);
    repetida.push_back(c.mesa[0].clone());
    repetida.push_back(c.mesa[1].clone());
    assert_eq!(
        c.cliente.try_apurar(&c.proposta, &repetida, &t, &a),
        Err(Ok(Erro::MembroRepetido))
    );
}

// ===================== o que `votar` recusa ==========================

#[test]
fn ninguem_vota_duas_vezes() {
    let c = montar(16);
    c.votar(0, 0);
    let (cs, provas, psoma, _) = c.cedula(0, 1);
    let (caminho, indice) = c.caminho(0);
    assert_eq!(
        c.cliente.try_votar(&c.proposta, &c.aptos[0], &cs, &provas, &psoma, &caminho, &indice, &1u32),
        Err(Ok(Erro::JaVotou))
    );
}

#[test]
fn quem_nao_esta_na_lista_nao_vota() {
    let c = montar(16);
    let env = &c.env;
    let intrusa = Address::generate(env);

    // cédula montada para a intrusa, com o caminho de outra pessoa
    let rs: Vetor<ArkFr> = (0..OPCOES).map(|_| pedersen::acaso_fr().unwrap()).collect();
    let cs_ark: Vetor<ArkG1> = (0..OPCOES)
        .map(|j| {
            let v = if j == 0 { 1u64 } else { 0 };
            pedersen::comprometer(&c.g, &c.h, &pedersen::escalar(v), &rs[j as usize])
        })
        .collect();
    let mut compromissos = Vec::new(env);
    let mut provas = Vec::new(env);
    for j in 0..OPCOES {
        let v = if j == 0 { 1u64 } else { 0 };
        let p = cds::provar(
            &ctx(env, &c.proposta, &intrusa, j),
            &c.g, &c.h, &cs_ark[j as usize], v, &rs[j as usize],
        ).unwrap();
        compromissos.push_back(g1(env, &cs_ark[j as usize]));
        provas.push_back(ProvaCds {
            a0: g1(env, &p.a0), a1: g1(env, &p.a1),
            e0: escalar(env, &p.e0), z0: escalar(env, &p.z0),
            e1: escalar(env, &p.e1), z1: escalar(env, &p.z1),
        });
    }
    let rho = rs.iter().fold(ArkFr::from(0u64), |a, r| a + r);
    let d = soma::alvo(&c.g, &cs_ark, 1);
    let ps = soma::provar(&ctx(env, &c.proposta, &intrusa, u32::MAX), &c.h, &d, &rho).unwrap();
    let psoma = ProvaSoma { a: g1(env, &ps.a), z: escalar(env, &ps.z) };

    let (caminho, indice) = c.caminho(0);
    assert_eq!(
        c.cliente.try_votar(&c.proposta, &intrusa, &compromissos, &provas, &psoma, &caminho, &indice, &1u32),
        Err(Ok(Erro::NaoEstaNaListaDeAptos))
    );
}

/// **A cédula roubada.** Copiar o `C` e as provas de outra pessoa não é um voto
/// válido, porque o desafio de Fiat–Shamir está preso a `(proposta, votante,
/// opção)`. Sem esse laço, a prova convenceria de que `v ∈ {0,1}` e nada nela
/// diria de quem é — e `Votou` impede votar duas vezes, não votar com a cédula
/// alheia.
#[test]
fn cedula_de_outra_pessoa_nao_vale() {
    let c = montar(16);
    let (cs, provas, psoma, _) = c.cedula(0, 1);
    let (caminho, indice) = c.caminho(1);
    assert_eq!(
        c.cliente.try_votar(&c.proposta, &c.aptos[1], &cs, &provas, &psoma, &caminho, &indice, &1u32),
        Err(Ok(Erro::ProvaBinariaInvalida))
    );
}

/// **O voto de peso 3.** O ataque que a disjuntiva existe para recusar: encher
/// a urna com um compromisso a um valor fora de `{0,1}`.
#[test]
fn voto_fora_do_binario_e_recusado() {
    let c = montar(16);
    let env = &c.env;
    let votante = &c.aptos[0];

    let (r0, r1) = (pedersen::acaso_fr().unwrap(), pedersen::acaso_fr().unwrap());
    let menos_dois = -pedersen::escalar(2);
    let cs_ark = std::vec![
        pedersen::comprometer(&c.g, &c.h, &pedersen::escalar(3), &r0),
        pedersen::comprometer(&c.g, &c.h, &menos_dois, &r1),
    ];

    // a soma fecha — v = (3, −2) soma 1 — então a prova de soma PASSARIA
    let d = soma::alvo(&c.g, &cs_ark, 1);
    let ps = soma::provar(&ctx(env, &c.proposta, votante, u32::MAX), &c.h, &d, &(r0 + r1)).unwrap();
    assert!(soma::verificar(&ctx(env, &c.proposta, votante, u32::MAX), &c.h, &d, &ps));

    // e o `core` nem deixa provar v=3
    assert_eq!(
        cds::provar(&ctx(env, &c.proposta, votante, 0), &c.g, &c.h, &cs_ark[0], 3, &r0),
        Err(cds::Erro::VotoForaDoBinario(3))
    );

    // então quem ataca tem de forjar. Forja uma disjuntiva qualquer:
    let forjada = cds::provar(&ctx(env, &c.proposta, votante, 0), &c.g, &c.h, &cs_ark[1], 0, &r1).unwrap();
    let mut compromissos = Vec::new(env);
    let mut provas = Vec::new(env);
    for j in 0..OPCOES as usize {
        compromissos.push_back(g1(env, &cs_ark[j]));
        provas.push_back(ProvaCds {
            a0: g1(env, &forjada.a0), a1: g1(env, &forjada.a1),
            e0: escalar(env, &forjada.e0), z0: escalar(env, &forjada.z0),
            e1: escalar(env, &forjada.e1), z1: escalar(env, &forjada.z1),
        });
    }
    let psoma = ProvaSoma { a: g1(env, &ps.a), z: escalar(env, &ps.z) };
    let (caminho, indice) = c.caminho(0);
    assert_eq!(
        c.cliente.try_votar(&c.proposta, votante, &compromissos, &provas, &psoma, &caminho, &indice, &1u32),
        Err(Ok(Erro::ProvaBinariaInvalida))
    );
}

/// **A cédula com dois votos.** Cada `v_j` é binário, e mesmo assim a soma dá
/// 2. É a disjuntiva que passa e a prova de soma que recusa — as duas são
/// necessárias e nenhuma é suficiente.
#[test]
fn votar_em_duas_opcoes_e_recusado() {
    let c = montar(16);
    let env = &c.env;
    let votante = &c.aptos[0];

    let rs: Vetor<ArkFr> = (0..OPCOES).map(|_| pedersen::acaso_fr().unwrap()).collect();
    let cs_ark: Vetor<ArkG1> = (0..OPCOES)
        .map(|j| pedersen::comprometer(&c.g, &c.h, &pedersen::escalar(1), &rs[j as usize]))
        .collect();

    let mut compromissos = Vec::new(env);
    let mut provas = Vec::new(env);
    for j in 0..OPCOES {
        let p = cds::provar(
            &ctx(env, &c.proposta, votante, j),
            &c.g, &c.h, &cs_ark[j as usize], 1, &rs[j as usize],
        ).unwrap();
        compromissos.push_back(g1(env, &cs_ark[j as usize]));
        provas.push_back(ProvaCds {
            a0: g1(env, &p.a0), a1: g1(env, &p.a1),
            e0: escalar(env, &p.e0), z0: escalar(env, &p.z0),
            e1: escalar(env, &p.e1), z1: escalar(env, &p.z1),
        });
    }

    let rho = rs.iter().fold(ArkFr::from(0u64), |a, r| a + r);
    let d = soma::alvo(&c.g, &cs_ark, 1);
    let ps = soma::provar(&ctx(env, &c.proposta, votante, u32::MAX), &c.h, &d, &rho).unwrap();
    let psoma = ProvaSoma { a: g1(env, &ps.a), z: escalar(env, &ps.z) };

    let (caminho, indice) = c.caminho(0);
    assert_eq!(
        c.cliente.try_votar(&c.proposta, votante, &compromissos, &provas, &psoma, &caminho, &indice, &1u32),
        Err(Ok(Erro::ProvaDeSomaInvalida))
    );
}

/// Peso diferente de 1 é recusado por desenho, não documentado como cuidado.
/// Pesos públicos distintos são quebrados por subconjunto-soma (SPEC §6.3).
#[test]
fn peso_nao_unitario_e_recusado() {
    let c = montar(16);
    let (cs, provas, psoma, _) = c.cedula(0, 0);
    let (caminho, indice) = c.caminho(0);
    assert_eq!(
        c.cliente.try_votar(&c.proposta, &c.aptos[0], &cs, &provas, &psoma, &caminho, &indice, &1000u32),
        Err(Ok(Erro::PesoNaoUnitario))
    );
}

#[test]
fn nao_se_vota_depois_do_prazo() {
    let c = montar(16);
    c.env.ledger().set_sequence_number(FECHA_EM);
    let (cs, provas, psoma, _) = c.cedula(0, 0);
    let (caminho, indice) = c.caminho(0);
    assert_eq!(
        c.cliente.try_votar(&c.proposta, &c.aptos[0], &cs, &provas, &psoma, &caminho, &indice, &1u32),
        Err(Ok(Erro::VotacaoEncerrada))
    );
}

#[test]
fn nao_se_apura_antes_do_prazo() {
    let c = montar(16);
    for i in 0..5 {
        c.votar(i, 0);
    }
    let mut t = Vec::new(&c.env);
    t.push_back(5u32);
    t.push_back(0u32);
    let mut a = Vec::new(&c.env);
    a.push_back(escalar(&c.env, &ArkFr::from(0u64)));
    a.push_back(escalar(&c.env, &ArkFr::from(0u64)));
    assert_eq!(
        c.cliente.try_apurar(&c.proposta, &c.mesa_sdk(3), &t, &a),
        Err(Ok(Erro::VotacaoAindaAberta))
    );
}

// ===================== semi-confidencial e o teorema da partição =====

/// A cédula pública entra, soma no resultado, e o sigilo de quem escolheu
/// sigilo continua intacto.
#[test]
fn voto_publico_soma_no_resultado() {
    let c = montar(16);
    let mut soma_r = [ArkFr::from(0u64); OPCOES as usize];
    for i in 0..5 {
        let rs = c.votar(i, 0);
        for j in 0..OPCOES as usize {
            soma_r[j] += rs[j];
        }
    }
    // duas pessoas votam em público, na opção 1
    for i in 5..7 {
        let (caminho, indice) = c.caminho(i);
        let mut e = Vec::new(&c.env);
        e.push_back(0u32);
        e.push_back(1u32);
        c.cliente.votar_publico(&c.proposta, &c.aptos[i], &e, &caminho, &indice, &1u32);
    }
    assert_eq!(c.cliente.comparecimento(&c.proposta), (5, 2));

    c.env.ledger().set_sequence_number(FECHA_EM + 1);
    let mut t = Vec::new(&c.env);
    t.push_back(5u32);
    t.push_back(0u32);
    let mut a = Vec::new(&c.env);
    for j in 0..OPCOES as usize {
        a.push_back(escalar(&c.env, &soma_r[j]));
    }
    let r = c.cliente.apurar(&c.proposta, &c.mesa_sdk(3), &t, &a);
    assert_eq!(r.get(0).unwrap(), 5, "5 confidenciais na opcao 0");
    assert_eq!(r.get(1).unwrap(), 2, "2 publicas na opcao 1");
}

/// **O teorema da partição, executável.**
///
/// Quatro pessoas votam em sigilo — abaixo de `τ = 5`. O contrato recusa
/// publicar, porque com poucos confidenciais o total determina os votos por
/// subtração. O sigilo dos compromissos é perfeito **e irrelevante**: quem
/// ataca usa aritmética, não criptanálise.
///
/// A troca é deliberada e está declarada: impor `τ` converte uma quebra de
/// privacidade numa falha de liveness. Uma votação travada é contestável e
/// repetível; um voto vazado não volta atrás.
#[test]
fn abaixo_de_tau_a_apuracao_trava_em_vez_de_vazar() {
    let c = montar(16);
    let mut soma_r = [ArkFr::from(0u64); OPCOES as usize];
    for i in 0..4 {
        let rs = c.votar(i, 0);
        for j in 0..OPCOES as usize {
            soma_r[j] += rs[j];
        }
    }
    c.env.ledger().set_sequence_number(FECHA_EM + 1);

    let mut t = Vec::new(&c.env);
    t.push_back(4u32);
    t.push_back(0u32);
    let mut a = Vec::new(&c.env);
    for j in 0..OPCOES as usize {
        a.push_back(escalar(&c.env, &soma_r[j]));
    }
    assert_eq!(
        c.cliente.try_apurar(&c.proposta, &c.mesa_sdk(3), &t, &a),
        Err(Ok(Erro::AnonimatoInsuficiente))
    );
}

/// Uma votação inteiramente pública apura normalmente: não há sigilo a
/// proteger, então `τ` não se aplica.
#[test]
fn votacao_toda_publica_apura() {
    let c = montar(16);
    for i in 0..3 {
        let (caminho, indice) = c.caminho(i);
        let mut e = Vec::new(&c.env);
        e.push_back(1u32);
        e.push_back(0u32);
        c.cliente.votar_publico(&c.proposta, &c.aptos[i], &e, &caminho, &indice, &1u32);
    }
    c.env.ledger().set_sequence_number(FECHA_EM + 1);

    let mut t = Vec::new(&c.env);
    t.push_back(0u32);
    t.push_back(0u32);
    let mut a = Vec::new(&c.env);
    a.push_back(escalar(&c.env, &ArkFr::from(0u64)));
    a.push_back(escalar(&c.env, &ArkFr::from(0u64)));
    let r = c.cliente.apurar(&c.proposta, &c.mesa_sdk(3), &t, &a);
    assert_eq!(r.get(0).unwrap(), 3);
}

#[test]
fn voto_publico_tambem_obedece_as_regras_da_cedula() {
    let c = montar(16);
    let (caminho, indice) = c.caminho(0);

    let mut duas = Vec::new(&c.env);
    duas.push_back(1u32);
    duas.push_back(1u32);
    assert_eq!(
        c.cliente.try_votar_publico(&c.proposta, &c.aptos[0], &duas, &caminho, &indice, &1u32),
        Err(Ok(Erro::SomaDiferenteDoPeso))
    );

    let mut tres = Vec::new(&c.env);
    tres.push_back(3u32);
    tres.push_back(0u32);
    assert_eq!(
        c.cliente.try_votar_publico(&c.proposta, &c.aptos[0], &tres, &caminho, &indice, &1u32),
        Err(Ok(Erro::EscolhaForaDoBinario))
    );
}

// ===================== abrir =========================================

#[test]
fn abrir_recusa_configuracao_invalida() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(10);
    let cliente = TesseraClient::new(&env, &env.register(Tessera, ()));
    let gov = Address::generate(&env);
    let raiz = BytesN::from_array(&env, &[0u8; 32]);
    let id: BytesN<32> = BytesN::from_array(&env, &[1u8; 32]);

    let mut mesa = Vec::new(&env);
    for _ in 0..3 {
        mesa.push_back(Address::generate(&env));
    }

    assert_eq!(
        cliente.try_abrir(&gov, &id, &1u32, &raiz, &mesa, &2u32, &1000u32),
        Err(Ok(Erro::OpcoesForaDaFaixa))
    );
    assert_eq!(
        cliente.try_abrir(&gov, &id, &17u32, &raiz, &mesa, &2u32, &1000u32),
        Err(Ok(Erro::OpcoesForaDaFaixa))
    );
    assert_eq!(
        cliente.try_abrir(&gov, &id, &2u32, &raiz, &mesa, &4u32, &1000u32),
        Err(Ok(Erro::LimiarInvalido))
    );
    assert_eq!(
        cliente.try_abrir(&gov, &id, &2u32, &raiz, &mesa, &0u32, &1000u32),
        Err(Ok(Erro::LimiarInvalido))
    );
    assert_eq!(
        cliente.try_abrir(&gov, &id, &2u32, &raiz, &mesa, &2u32, &5u32),
        Err(Ok(Erro::PrazoNoPassado))
    );

    let mut repetida = Vec::new(&env);
    let m = Address::generate(&env);
    repetida.push_back(m.clone());
    repetida.push_back(m);
    assert_eq!(
        cliente.try_abrir(&gov, &id, &2u32, &raiz, &repetida, &2u32, &1000u32),
        Err(Ok(Erro::MembroRepetido))
    );

    cliente.abrir(&gov, &id, &2u32, &raiz, &mesa, &2u32, &1000u32);
    assert_eq!(
        cliente.try_abrir(&gov, &id, &2u32, &raiz, &mesa, &2u32, &1000u32),
        Err(Ok(Erro::PropostaJaExiste))
    );
}

/// O `H` do contrato é o mesmo que a sonda 11 fixou na testnet. Se mudar, todo
/// compromisso já feito deixa de abrir.
#[test]
fn gerador_h_bate_com_o_vetor_da_testnet() {
    const H_HEX: &str = "1462b4b57a7d01e598685e913608bab6990de8cce1c705642c19a6b9ed660fc589758bdebb20c2bdde65a25b35b12206198669dff273abe7b9e6a0b3e636f8a8a41f9c81f6a79d308ec0fe57ef8a073c90e444002ba59807f987cce67d72eccf";
    let env = Env::default();
    let cliente = TesseraClient::new(&env, &env.register(Tessera, ()));
    let h = cliente.gerador_h();
    let mut s = std::string::String::new();
    for b in h.to_array().iter() {
        s.push_str(&std::format!("{:02x}", b));
    }
    assert_eq!(s, H_HEX, "H do contrato DIVERGE do fixado na testnet");
}
/// **A codificação do infinito, que não é óbvia e quebraria a primeira cédula
/// de toda votação.**
///
/// O host recusa 96 bytes de zero com "point not on curve". O infinito é o bit
/// de flag do formato zcash: `0x40` no byte alto, zeros no resto. Descoberto
/// somando cada candidato ao gerador e vendo qual devolvia o gerador.
///
/// O `core` codifica igual, e o teste `ponto::infinito_e_a_flag_do_host` o
/// trava do outro lado.
#[test]
fn o_infinito_do_host_e_a_flag_zcash_nao_zeros() {
    let env = Env::default();
    let bls = env.crypto().bls12_381();
    let g = cripto::gerador_g(&env);

    let inf = Bls12381G1Affine::from_bytes(BytesN::from_array(
        &env,
        &tessera_core::ponto::INFINITO,
    ));
    assert_eq!(bls.g1_add(&inf, &g), g, "INFINITO do core nao e neutro no host");

    for ruim in [[0u8; 96], { let mut v = [0u8; 96]; v[0] = 0xc0; v }] {
        let p = Bls12381G1Affine::from_bytes(BytesN::from_array(&env, &ruim));
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| bls.g1_add(&p, &g))).is_err(),
            "o host aceitou uma codificacao de infinito que nao deveria"
        );
    }
}

// ===================== orçamento =====================================

/// Teto de CPU por transação, medido por bissecção na testnet (sonda 2).
const TETO: u64 = 400_000_000;

fn cpu<T>(env: &Env, f: impl FnOnce() -> T) -> u64 {
    let mut b = env.cost_estimate().budget();
    b.reset_unlimited();
    let _ = f();
    env.cost_estimate().budget().cpu_instruction_cost()
}

/// **O orçamento do contrato de verdade, não a soma das sondas.**
///
/// As sondas mediram primitivas isoladas e o SPEC somava 33.480.865. Este
/// teste mede `votar()` inteiro — autorização, leitura de estado, aptidão,
/// validação de ponto, duas disjuntivas, a prova de soma, a agregação e a
/// escrita — numa invocação só.
///
/// E **assere teto**: uma regressão de custo quebra o build em vez de aparecer
/// na demo.
#[test]
fn orcamento_de_votar_e_de_apurar() {
    let c = montar(16);

    let (cs, provas, psoma, _) = c.cedula(0, 0);
    let (caminho, indice) = c.caminho(0);
    let custo_votar = cpu(&c.env, || {
        c.cliente
            .votar(&c.proposta, &c.aptos[0], &cs, &provas, &psoma, &caminho, &indice, &1u32)
    });

    let (cse, _, _, _) = c.cedula(1, 1);
    let (cam1, ind1) = c.caminho(1);
    let mut e = Vec::new(&c.env);
    e.push_back(0u32);
    e.push_back(1u32);
    let custo_publico = cpu(&c.env, || {
        c.cliente
            .votar_publico(&c.proposta, &c.aptos[1], &e, &cam1, &ind1, &1u32)
    });
    let _ = cse;

    // mais quatro confidenciais, para passar de τ
    let mut soma_r = [ArkFr::from(0u64); OPCOES as usize];
    for i in 2..6 {
        let rs = c.votar(i, 0);
        for j in 0..OPCOES as usize {
            soma_r[j] += rs[j];
        }
    }
    // o voto 0, que foi medido acima, também conta
    let (cs0, _, _, _) = (0, 0, 0, 0);
    let _ = cs0;

    std::println!("\n== ORCAMENTO DO CONTRATO (m=2) ==");
    std::println!("votar() .......................... {:>10}", custo_votar);
    std::println!("   % do teto de 400M ............. {:.1}%", 100.0 * custo_votar as f64 / TETO as f64);
    std::println!("   folga ......................... {:.1}x", TETO as f64 / custo_votar as f64);
    std::println!("   projecao somada das sondas .... {:>10}", 33_480_865u64);
    std::println!("votar_publico() .................. {:>10}", custo_publico);
    std::println!("   razao confidencial/publico .... {:.0}x", custo_votar as f64 / custo_publico as f64);

    // Portão 1 do PLANO: ≤200M segue sem cortes.
    assert!(
        custo_votar <= 200_000_000,
        "votar() custa {}, acima dos 200M do Portao 1",
        custo_votar
    );
    assert!(
        custo_publico < custo_votar / 10,
        "a cedula publica deveria ser ao menos 10x mais barata"
    );
}

/// **`apurar()` não cresce com o comparecimento.**
///
/// É a descoberta que decidiu o desenho (sonda 5): procurar o total por força
/// bruta custa 124.277 por unidade de peso e tem teto de ~3.218 de peso por
/// transação, enquanto conferir uma abertura afirmada é um MSM de 2 termos por
/// opção — e um MSM não sabe quantas pessoas votaram.
///
/// O teste separa o que é criptografia do que é busca no estado:
///
/// 1. com o mesmo comparecimento e resultados completamente diferentes, o
///    custo é **idêntico byte a byte** — a aritmética não depende do resultado;
/// 2. de 5 para 40 votantes sobra ~1,9%, e a sobra **não é criptográfica**:
///    uma leitura pura, que não toca em curva nenhuma, cresce na mesma
///    proporção. É o mapa de estado do host local, que guarda uma entrada
///    `Votou` por votante. Na rede, a footprint de `apurar()` carrega um
///    número fixo de entradas — `Proposta`, `Acum × m`, `TotalPublico × m`,
///    `Comparecimento`, `Resultado` — e nenhuma `Votou`.
#[test]
fn apurar_nao_cresce_com_o_comparecimento() {
    /// Devolve `(custo de apurar, custo de uma leitura pura)`.
    ///
    /// `na_zero` votantes escolhem a opção 0, o resto escolhe a 1.
    let medir = |n: usize, na_zero: usize| -> (u64, u64) {
        let c = montar(64);
        let mut soma_r = [ArkFr::from(0u64); OPCOES as usize];
        for i in 0..n {
            let rs = c.votar(i, if i < na_zero { 0 } else { 1 });
            for j in 0..OPCOES as usize {
                soma_r[j] += rs[j];
            }
        }
        c.env.ledger().set_sequence_number(FECHA_EM + 1);
        let mut t = Vec::new(&c.env);
        t.push_back(na_zero as u32);
        t.push_back((n - na_zero) as u32);
        let mut a = Vec::new(&c.env);
        for j in 0..OPCOES as usize {
            a.push_back(escalar(&c.env, &soma_r[j]));
        }
        let mesa = c.mesa_sdk(3);

        // leitura pura, sem nenhuma operação de curva
        let leitura = cpu(&c.env, || c.cliente.comparecimento(&c.proposta));
        let apuracao = cpu(&c.env, || c.cliente.apurar(&c.proposta, &mesa, &t, &a));
        (apuracao, leitura)
    };

    let (c5, l5) = medir(5, 5);
    let (c40, l40) = medir(40, 40);
    let (c40_dividido, _) = medir(40, 5);

    std::println!("\n== apurar(), m=2 ==");
    std::println!("5 votantes ....................... {:>10}", c5);
    std::println!("40 votantes ...................... {:>10}", c40);
    std::println!("40 votantes, resultado (5,35) .... {:>10}", c40_dividido);
    std::println!("   8x mais votantes custa ........ {:+.2}%",
        100.0 * (c40 as f64 - c5 as f64) / c5 as f64);
    std::println!("-- atribuicao da sobra --");
    std::println!("leitura pura, 5 votantes ......... {:>10}", l5);
    std::println!("leitura pura, 40 votantes ........ {:>10}", l40);
    std::println!("   mesma sobra, sem curva ........ {:+.2}%",
        100.0 * (l40 as f64 - l5 as f64) / l5 as f64);

    // 1. a aritmetica nao depende do resultado
    assert_eq!(
        c40, c40_dividido,
        "o custo de apurar depende de QUAL foi o resultado"
    );

    // 2. a sobra nao e criptografica: uma leitura pura cresce tambem.
    assert!(
        l40 > l5,
        "a leitura pura nao cresceu; a atribuicao da sobra esta errada"
    );

    // 3. e e pequena: menos de 3% num intervalo de 8x.
    let sobra = (c40 - c5) as f64 / c5 as f64;
    assert!(
        sobra < 0.03,
        "apurar cresceu {:.1}% de 5 para 40 votantes",
        sobra * 100.0
    );

    // 4. e nada por votante tem forma de curva: a sobra por votante fica bem
    //    abaixo de um unico g1_add (110.748, sonda 2).
    let por_votante = (c40 - c5) / 35;
    assert!(
        por_votante < 110_748 / 10,
        "sobra de {} por votante: ha algo criptografico crescendo com n",
        por_votante
    );
}
