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

/// Os mesmos bytes que `cripto::contexto` monta dentro do contrato — incluindo
/// o índice da pergunta, sem o qual uma disjuntiva migraria entre perguntas.
fn ctx(
    env: &Env,
    proposta: &BytesN<32>,
    votante: &Address,
    pergunta: u32,
    opcao: u32,
) -> Vetor<u8> {
    let mut v: Vetor<u8> = proposta.to_array().to_vec();
    v.extend(bytes_de(&votante.clone().to_xdr(env)));
    v.extend_from_slice(&pergunta.to_be_bytes());
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
    /// O formato da cédula: `(opções, confidencial)` por pergunta.
    perguntas: Vetor<(u32, bool)>,
    g: ArkG1,
    h: ArkG1,
}

const OPCOES: u32 = 2;
const FECHA_EM: u32 = 1000;

/// A cédula padrão: uma pergunta, sigilosa, duas opções. É o caso de toda a
/// bateria antiga, e continua valendo — uma cédula confidencial é só a cédula
/// mista em que nenhuma pergunta é pública.
fn montar(n_aptos: usize) -> Cenario {
    montar_cedula(n_aptos, &[(OPCOES, true)])
}

fn montar_cedula(n_aptos: usize, perguntas: &[(u32, bool)]) -> Cenario {
    montar_janela(n_aptos, perguntas, 0, FECHA_EM)
}

/// O mesmo cenário, com a janela escolhida. `abre_em = 0` é abertura imediata,
/// que é o que todas as outras baterias usam.
fn montar_janela(
    n_aptos: usize,
    perguntas: &[(u32, bool)],
    abre_em: u32,
    fecha_em: u32,
) -> Cenario {
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

    let mut perg_sdk = Vec::new(&env);
    for (opcoes, confidencial) in perguntas {
        perg_sdk.push_back(Pergunta { opcoes: *opcoes, confidencial: *confidencial });
    }

    let proposta: BytesN<32> = BytesN::from_array(&env, &[7u8; 32]);
    cliente.abrir(
        &Address::generate(&env),
        &proposta,
        &perg_sdk,
        &BytesN::from_array(&env, &arvore.raiz()),
        &mesa_sdk,
        &3u32,
        &abre_em,
        &fecha_em,
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
        perguntas: perguntas.to_vec(),
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

    /// **A mesa endossando, um membro por chamada.**
    ///
    /// Cada membro manda a sua própria transação; o contrato conta os endossos
    /// e publica no `k`-ésimo. Devolve o resultado, ou o erro do primeiro
    /// membro que for recusado — porque todo endosso confere tudo, e números
    /// falsos são recusados no primeiro que tentar, não no último.
    fn endossar_um(
        &self,
        i: usize,
        totais: &Vec<u32>,
        aberturas: &Vec<Bls12381Fr>,
    ) -> Result<Option<Vec<u32>>, soroban_sdk::Error> {
        match self
            .cliente
            .try_apurar(&self.proposta, &self.mesa[i], totais, aberturas)
        {
            Ok(v) => Ok(v.unwrap()),
            Err(Ok(e)) => Err(e.into()),
            Err(Err(_)) => panic!("o contrato devolveu um erro que o cliente não entende"),
        }
    }

    fn endossar(
        &self,
        quantos: usize,
        totais: &Vec<u32>,
        aberturas: &Vec<Bls12381Fr>,
    ) -> Result<Option<Vec<u32>>, soroban_sdk::Error> {
        let mut ultimo = Ok(None);
        for i in 0..quantos {
            ultimo = self.endossar_um(i, totais, aberturas);
            if ultimo.is_err() {
                return ultimo;
            }
        }
        ultimo
    }

    fn mesa_sdk(&self, k: usize) -> Vec<Address> {
        let mut v = Vec::new(&self.env);
        for m in self.mesa.iter().take(k) {
            v.push_back(m.clone());
        }
        v
    }

    /// Monta uma cédula honesta para a cédula desta proposta, qualquer que seja
    /// o formato, e devolve também os `r_j`, que no mundo real iriam para a
    /// mesa em shares de Shamir.
    ///
    /// `escolhas[q]` é a opção marcada na pergunta `q`. As perguntas sigilosas
    /// viram compromisso + disjuntivas + **uma prova de soma própria**; as
    /// públicas viram resposta em claro.
    fn cedula_mista(
        &self,
        i: usize,
        escolhas: &[u32],
    ) -> (
        Vec<Bls12381G1Affine>,
        Vec<ProvaCds>,
        Vec<ProvaSoma>,
        Vec<u32>,
        Vetor<ArkFr>,
    ) {
        let env = &self.env;
        let votante = &self.aptos[i];

        let mut compromissos = Vec::new(env);
        let mut provas = Vec::new(env);
        let mut provas_soma = Vec::new(env);
        let mut publicas = Vec::new(env);
        let mut todos_rs: Vetor<ArkFr> = Vetor::new();

        for (q, (opcoes, confidencial)) in self.perguntas.iter().enumerate() {
            let escolha = escolhas[q];
            if !confidencial {
                for j in 0..*opcoes {
                    publicas.push_back(if j == escolha { 1u32 } else { 0 });
                }
                continue;
            }

            let rs: Vetor<ArkFr> = (0..*opcoes).map(|_| pedersen::acaso_fr().unwrap()).collect();
            let cs: Vetor<ArkG1> = (0..*opcoes)
                .map(|j| {
                    let v = if j == escolha { 1u64 } else { 0 };
                    pedersen::comprometer(&self.g, &self.h, &pedersen::escalar(v), &rs[j as usize])
                })
                .collect();

            for j in 0..*opcoes {
                let v = if j == escolha { 1u64 } else { 0 };
                let p = cds::provar(
                    &ctx(env, &self.proposta, votante, q as u32, j),
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

            // Uma prova de soma **por pergunta**, sobre a fatia dela.
            let rho = rs.iter().fold(ArkFr::from(0u64), |a, r| a + r);
            let d = soma::alvo(&self.g, &cs, 1);
            let ps = soma::provar(
                &ctx(env, &self.proposta, votante, q as u32, u32::MAX),
                &self.h,
                &d,
                &rho,
            )
            .unwrap();
            provas_soma.push_back(ProvaSoma { a: g1(env, &ps.a), z: escalar(env, &ps.z) });
            todos_rs.extend(rs);
        }

        (compromissos, provas, provas_soma, publicas, todos_rs)
    }

    /// Atalho para a cédula de uma pergunta só.
    fn cedula(
        &self,
        i: usize,
        escolha: u32,
    ) -> (
        Vec<Bls12381G1Affine>,
        Vec<ProvaCds>,
        Vec<ProvaSoma>,
        Vec<u32>,
        Vetor<ArkFr>,
    ) {
        self.cedula_mista(i, &[escolha])
    }

    fn votar(&self, i: usize, escolha: u32) -> Vetor<ArkFr> {
        self.votar_misto(i, &[escolha])
    }

    fn votar_misto(&self, i: usize, escolhas: &[u32]) -> Vetor<ArkFr> {
        let (cs, provas, psoma, pubs, rs) = self.cedula_mista(i, escolhas);
        let (caminho, indice) = self.caminho(i);
        self.cliente.votar(
            &self.proposta,
            &self.aptos[i],
            &cs,
            &provas,
            &psoma,
            &pubs,
            &caminho,
            &indice,
            &1u32,
        );
        rs
    }

    /// Uma `ProvaSoma` solta, embrulhada para o formato de uma pergunta só.
    fn so_uma(&self, p: ProvaSoma) -> Vec<ProvaSoma> {
        let mut v = Vec::new(&self.env);
        v.push_back(p);
        v
    }

    fn sem_publicas(&self) -> Vec<u32> {
        Vec::new(&self.env)
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

    // dois endossos nao publicam nada
    assert_eq!(c.endossar(2, &totais, &aberturas).unwrap(), None);
    assert_eq!(c.cliente.resultado(&c.proposta), None);
    assert_eq!(c.cliente.endossos(&c.proposta, &totais, &aberturas), 2);

    // o terceiro fecha
    let r = c.endossar_um(2, &totais, &aberturas).unwrap().unwrap();
    assert_eq!(r.get(0).unwrap(), 4);
    assert_eq!(r.get(1).unwrap(), 2);
    assert_eq!(c.cliente.resultado(&c.proposta).unwrap(), r);
}

/// **Um membro endossa estes números, não "a apuração".**
///
/// Dois membros endossam `(4,2)` e um terceiro endossa `(2,4)` — que tambem
/// fecharia a soma, mas nao abre o acumulador. Nenhum dos dois digests atinge
/// o limiar, e nada e publicado. Sem a ligacao ao digest, tres autorizacoes
/// genericas publicariam o que a mesa quisesse.
#[test]
fn endosso_esta_preso_aos_numeros() {
    let c = montar(16);
    let mut soma_r = [ArkFr::from(0u64); OPCOES as usize];
    for i in 0..6 {
        let rs = c.votar(i, if i < 4 { 0 } else { 1 });
        for j in 0..OPCOES as usize {
            soma_r[j] += rs[j];
        }
    }
    c.env.ledger().set_sequence_number(FECHA_EM + 1);

    let mut ab = Vec::new(&c.env);
    for j in 0..OPCOES as usize {
        ab.push_back(escalar(&c.env, &soma_r[j]));
    }
    let mut certo = Vec::new(&c.env);
    certo.push_back(4u32);
    certo.push_back(2u32);

    assert_eq!(c.endossar(2, &certo, &ab).unwrap(), None);
    assert_eq!(c.cliente.endossos(&c.proposta, &certo, &ab), 2);
    // e o digest dos outros numeros nao tem endosso nenhum

    // o terceiro membro endossa OUTROS numeros: e recusado, e o digest certo
    // continua com dois endossos
    let mut outro = Vec::new(&c.env);
    outro.push_back(2u32);
    outro.push_back(4u32);
    assert_eq!(
        c.cliente.try_apurar(&c.proposta, &c.mesa[2], &outro, &ab),
        Err(Ok(Erro::AberturaNaoFecha))
    );
    assert_eq!(c.cliente.resultado(&c.proposta), None);
    assert_eq!(c.cliente.endossos(&c.proposta, &certo, &ab), 2);

    // e um membro nao endossa duas vezes para fechar o quorum sozinho
    assert_eq!(
        c.cliente.try_apurar(&c.proposta, &c.mesa[0], &certo, &ab),
        Err(Ok(Erro::MembroJaEndossou))
    );
    assert_eq!(c.cliente.resultado(&c.proposta), None);
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
        c.endossar(3, &tot(&c.env, 2, 4), &ab(&c.env)).unwrap_err(),
        Erro::AberturaNaoFecha.into()
    );
    // inflar um lado: a soma deixa de bater com o comparecimento
    assert_eq!(
        c.endossar(3, &tot(&c.env, 5, 2), &ab(&c.env)).unwrap_err(),
        Erro::TotalDiferenteDoComparecimento.into()
    );
    // e o honesto passa
    let r = c.endossar(3, &tot(&c.env, 4, 2), &ab(&c.env)).unwrap().unwrap();
    assert_eq!(r.get(0).unwrap(), 4);
}

/// Menos de `k` membros não apuram, e quem não é da mesa não apura.
#[test]
fn mesa_abaixo_do_limiar_nao_apura() {
    let c = montar(16);
    let mut soma_r = [ArkFr::from(0u64); OPCOES as usize];
    for i in 0..5 {
        let rs = c.votar(i, 0);
        for j in 0..OPCOES as usize {
            soma_r[j] += rs[j];
        }
    }
    c.env.ledger().set_sequence_number(FECHA_EM + 1);

    let mut t = Vec::new(&c.env);
    t.push_back(5u32);
    t.push_back(0u32);
    let mut a = Vec::new(&c.env);
    for j in 0..OPCOES as usize {
        a.push_back(escalar(&c.env, &soma_r[j]));
    }

    // dois endossos nao publicam, mesmo com os numeros certos
    assert_eq!(c.endossar(2, &t, &a).unwrap(), None);
    assert_eq!(c.cliente.resultado(&c.proposta), None);

    // quem nao e da mesa nao endossa
    assert_eq!(
        c.cliente.try_apurar(&c.proposta, &c.aptos[0], &t, &a),
        Err(Ok(Erro::NaoEMembroDaMesa))
    );
    assert_eq!(c.cliente.resultado(&c.proposta), None);

    // e o mesmo membro nao endossa duas vezes para fechar o quorum sozinho
    assert_eq!(
        c.cliente.try_apurar(&c.proposta, &c.mesa[1], &t, &a),
        Err(Ok(Erro::MembroJaEndossou))
    );
    assert_eq!(c.cliente.resultado(&c.proposta), None);

    // o terceiro membro de verdade fecha
    assert!(c.endossar_um(2, &t, &a).unwrap().is_some());
}

// ===================== o que `votar` recusa ==========================

#[test]
fn ninguem_vota_duas_vezes() {
    let c = montar(16);
    c.votar(0, 0);
    let (cs, provas, psoma, pubs, _) = c.cedula(0, 1);
    let (caminho, indice) = c.caminho(0);
    assert_eq!(
        c.cliente.try_votar(&c.proposta, &c.aptos[0], &cs, &provas, &psoma, &pubs, &caminho, &indice, &1u32),
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
            &ctx(env, &c.proposta, &intrusa, 0, j),
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
    let ps = soma::provar(&ctx(env, &c.proposta, &intrusa, 0, u32::MAX), &c.h, &d, &rho).unwrap();
    let psoma = c.so_uma(ProvaSoma { a: g1(env, &ps.a), z: escalar(env, &ps.z) });
    let pubs = c.sem_publicas();

    let (caminho, indice) = c.caminho(0);
    assert_eq!(
        c.cliente.try_votar(&c.proposta, &intrusa, &compromissos, &provas, &psoma, &pubs, &caminho, &indice, &1u32),
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
    let (cs, provas, psoma, pubs, _) = c.cedula(0, 1);
    let (caminho, indice) = c.caminho(1);
    assert_eq!(
        c.cliente.try_votar(&c.proposta, &c.aptos[1], &cs, &provas, &psoma, &pubs, &caminho, &indice, &1u32),
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
    let ps = soma::provar(&ctx(env, &c.proposta, votante, 0, u32::MAX), &c.h, &d, &(r0 + r1)).unwrap();
    assert!(soma::verificar(&ctx(env, &c.proposta, votante, 0, u32::MAX), &c.h, &d, &ps));

    // e o `core` nem deixa provar v=3
    assert_eq!(
        cds::provar(&ctx(env, &c.proposta, votante, 0, 0), &c.g, &c.h, &cs_ark[0], 3, &r0),
        Err(cds::Erro::VotoForaDoBinario(3))
    );

    // então quem ataca tem de forjar. Forja uma disjuntiva qualquer:
    let forjada = cds::provar(&ctx(env, &c.proposta, votante, 0, 0), &c.g, &c.h, &cs_ark[1], 0, &r1).unwrap();
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
    let psoma = c.so_uma(ProvaSoma { a: g1(env, &ps.a), z: escalar(env, &ps.z) });
    let pubs = c.sem_publicas();
    let (caminho, indice) = c.caminho(0);
    assert_eq!(
        c.cliente.try_votar(&c.proposta, votante, &compromissos, &provas, &psoma, &pubs, &caminho, &indice, &1u32),
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
            &ctx(env, &c.proposta, votante, 0, j),
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
    let ps = soma::provar(&ctx(env, &c.proposta, votante, 0, u32::MAX), &c.h, &d, &rho).unwrap();
    let psoma = c.so_uma(ProvaSoma { a: g1(env, &ps.a), z: escalar(env, &ps.z) });
    let pubs = c.sem_publicas();

    let (caminho, indice) = c.caminho(0);
    assert_eq!(
        c.cliente.try_votar(&c.proposta, votante, &compromissos, &provas, &psoma, &pubs, &caminho, &indice, &1u32),
        Err(Ok(Erro::ProvaDeSomaInvalida))
    );
}

/// Peso diferente de 1 é recusado por desenho, não documentado como cuidado.
/// Pesos públicos distintos são quebrados por subconjunto-soma (SPEC §6.3).
#[test]
fn peso_nao_unitario_e_recusado() {
    let c = montar(16);
    let (cs, provas, psoma, pubs, _) = c.cedula(0, 0);
    let (caminho, indice) = c.caminho(0);
    assert_eq!(
        c.cliente.try_votar(&c.proposta, &c.aptos[0], &cs, &provas, &psoma, &pubs, &caminho, &indice, &1000u32),
        Err(Ok(Erro::PesoNaoUnitario))
    );
}

#[test]
fn nao_se_vota_depois_do_prazo() {
    let c = montar(16);
    c.env.ledger().set_sequence_number(FECHA_EM);
    let (cs, provas, psoma, pubs, _) = c.cedula(0, 0);
    let (caminho, indice) = c.caminho(0);
    assert_eq!(
        c.cliente.try_votar(&c.proposta, &c.aptos[0], &cs, &provas, &psoma, &pubs, &caminho, &indice, &1u32),
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
        c.cliente.try_apurar(&c.proposta, &c.mesa[0], &t, &a),
        Err(Ok(Erro::VotacaoAindaAberta))
    );
}

/// A janela tem dois lados. Antes de `abre_em` o voto é recusado; dentro dela
/// passa. "Ainda não" e "não mais" têm erros diferentes, porque são situações
/// diferentes para quem está na frente da urna.
#[test]
fn a_votacao_so_aceita_voto_dentro_da_janela() {
    // Abre no ledger 100, fecha no 200; o cenário começa no 10.
    let c = montar_janela(16, &[(OPCOES, true)], 100, 200);

    let tentar = || {
        let (compromissos, provas, provas_soma, publicas, _) = c.cedula_mista(0, &[0]);
        let (irmaos, indice) = c.caminho(0);
        c.cliente.try_votar(
            &c.proposta, &c.aptos[0], &compromissos, &provas, &provas_soma,
            &publicas, &irmaos, &indice, &1u32,
        )
    };

    assert_eq!(tentar(), Err(Ok(Erro::VotacaoAindaNaoComecou)));

    c.env.ledger().set_sequence_number(150);
    assert!(tentar().is_ok(), "dentro da janela o voto entra");

    c.env.ledger().set_sequence_number(250);
    let (compromissos, provas, provas_soma, publicas, _) = c.cedula_mista(1, &[0]);
    let (irmaos, indice) = c.caminho(1);
    assert_eq!(
        c.cliente.try_votar(
            &c.proposta, &c.aptos[1], &compromissos, &provas, &provas_soma,
            &publicas, &irmaos, &indice, &1u32,
        ),
        Err(Ok(Erro::VotacaoEncerrada))
    );
}

/// Uma janela de duração zero não é votação.
#[test]
fn a_janela_precisa_ter_duracao() {
    let c = montar(4);
    let mut perg = Vec::new(&c.env);
    perg.push_back(Pergunta { opcoes: OPCOES, confidencial: true });
    let mut mesa_sdk = Vec::new(&c.env);
    for m in &c.mesa {
        mesa_sdk.push_back(m.clone());
    }
    let outra: BytesN<32> = BytesN::from_array(&c.env, &[43u8; 32]);
    assert_eq!(
        c.cliente.try_abrir(
            &Address::generate(&c.env), &outra, &perg,
            &BytesN::from_array(&c.env, &c.arvore.raiz()), &mesa_sdk,
            &3u32, &300u32, &300u32,
        ),
        Err(Ok(Erro::PrazoNoPassado))
    );
}

// ===================== semi-confidencial e o teorema da partição =====

/// **O quórum de sigilo, executável.**
///
/// Quatro cédulas, abaixo de `τ = 5`. O contrato recusa publicar, porque com
/// poucas cédulas o total determina os votos por subtração.
///
/// Antes isto era um ataque: bastavam três pessoas abrindo o voto por
/// `votar_publico` para encolher o conjunto sigiloso e vetar a assembleia.
/// Sem aquela função, o único caminho até aqui é comparecimento baixo — e aí a
/// recusa deixa de ser negação de serviço e vira quórum, com o mesmo remédio
/// que o Brasil usa: estender o prazo, ou refazer com um eleitorado que caiba
/// no sigilo.
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
        c.cliente.try_apurar(&c.proposta, &c.mesa[0], &t, &a),
        Err(Ok(Erro::AnonimatoInsuficiente))
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

    // Uma cédula com o formato dado, para encurtar as chamadas.
    let cedula = |perguntas: &[(u32, bool)]| {
        let mut v = Vec::new(&env);
        for (opcoes, confidencial) in perguntas {
            v.push_back(Pergunta { opcoes: *opcoes, confidencial: *confidencial });
        }
        v
    };
    let ok = cedula(&[(2, true)]);

    assert_eq!(
        cliente.try_abrir(&gov, &id, &cedula(&[(1, true)]), &raiz, &mesa, &2u32, &0u32, &1000u32),
        Err(Ok(Erro::OpcoesForaDaFaixa))
    );
    assert_eq!(
        cliente.try_abrir(&gov, &id, &cedula(&[(17, true)]), &raiz, &mesa, &2u32, &0u32, &1000u32),
        Err(Ok(Erro::OpcoesForaDaFaixa))
    );
    assert_eq!(
        cliente.try_abrir(&gov, &id, &cedula(&[]), &raiz, &mesa, &2u32, &0u32, &1000u32),
        Err(Ok(Erro::PerguntasForaDaFaixa))
    );
    // Nove perguntas passam de MAX_PERGUNTAS.
    let nove: Vetor<(u32, bool)> = (0..9).map(|_| (2u32, false)).collect();
    assert_eq!(
        cliente.try_abrir(&gov, &id, &cedula(&nove), &raiz, &mesa, &2u32, &0u32, &1000u32),
        Err(Ok(Erro::PerguntasForaDaFaixa))
    );
    // Cada pergunta cabe, mas o total confidencial passa de MAX_OPCOES — e é
    // o total que o orçamento de CPU limita.
    let gordas: Vetor<(u32, bool)> = (0..3).map(|_| (16u32, true)).collect();
    assert_eq!(
        cliente.try_abrir(&gov, &id, &cedula(&gordas), &raiz, &mesa, &2u32, &0u32, &1000u32),
        Err(Ok(Erro::PerguntasForaDaFaixa))
    );
    // As mesmas 48 opções, mas públicas, não custam disjuntiva nenhuma.
    let publicas: Vetor<(u32, bool)> = (0..3).map(|_| (16u32, false)).collect();
    assert!(cliente
        .try_abrir(
            &gov,
            &BytesN::from_array(&env, &[9u8; 32]),
            &cedula(&publicas),
            &raiz,
            &mesa,
            &2u32,
            &0u32,
            &1000u32
        )
        .is_ok());

    assert_eq!(
        cliente.try_abrir(&gov, &id, &ok, &raiz, &mesa, &4u32, &0u32, &1000u32),
        Err(Ok(Erro::LimiarInvalido))
    );
    assert_eq!(
        cliente.try_abrir(&gov, &id, &ok, &raiz, &mesa, &0u32, &0u32, &1000u32),
        Err(Ok(Erro::LimiarInvalido))
    );
    assert_eq!(
        cliente.try_abrir(&gov, &id, &ok, &raiz, &mesa, &2u32, &0u32, &5u32),
        Err(Ok(Erro::PrazoNoPassado))
    );

    let mut repetida = Vec::new(&env);
    let m = Address::generate(&env);
    repetida.push_back(m.clone());
    repetida.push_back(m);
    assert_eq!(
        cliente.try_abrir(&gov, &id, &ok, &raiz, &repetida, &2u32, &0u32, &1000u32),
        Err(Ok(Erro::MembroRepetido))
    );

    cliente.abrir(&gov, &id, &ok, &raiz, &mesa, &2u32, &0u32, &1000u32);
    assert_eq!(
        cliente.try_abrir(&gov, &id, &ok, &raiz, &mesa, &2u32, &0u32, &1000u32),
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

    let (cs, provas, psoma, pubs, _) = c.cedula(0, 0);
    let (caminho, indice) = c.caminho(0);
    let custo_votar = cpu(&c.env, || {
        c.cliente
            .votar(&c.proposta, &c.aptos[0], &cs, &provas, &psoma, &pubs, &caminho, &indice, &1u32)
    });

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

    // Portão 1 do PLANO: ≤200M segue sem cortes.
    assert!(
        custo_votar <= 200_000_000,
        "votar() custa {}, acima dos 200M do Portao 1",
        custo_votar
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
        // leitura pura, sem nenhuma operação de curva
        let leitura = cpu(&c.env, || c.cliente.comparecimento(&c.proposta));
        // o primeiro endosso, que confere tudo e ainda não publica
        let apuracao = cpu(&c.env, || c.cliente.apurar(&c.proposta, &c.mesa[0], &t, &a));
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

// ===================== a cédula mista ================================

/// **A cédula semiconfidencial, de ponta a ponta.**
///
/// Uma assembleia com três perguntas: aprovar as contas em aberto, destituir a
/// diretoria em sigilo, e eleger uma cadeira em sigilo. Uma transação por
/// pessoa, uma prova de aptidão por pessoa, e um resultado em que as duas
/// naturezas convivem.
#[test]
fn cedula_mista_apura_as_duas_partes() {
    let c = montar_cedula(16, &[(2, false), (2, true), (3, true)]);

    // 6 eleitores. Nas contas: 5 aprovam, 1 rejeita — e isso é público.
    // Na destituição: 2 a favor, 4 contra — e isso ninguém vê.
    let votos = [
        [0u32, 0, 0],
        [0, 0, 1],
        [0, 1, 2],
        [0, 1, 0],
        [0, 0, 1],
        [1, 0, 2],
    ];

    let mut soma_r = [ArkFr::from(0u64); 5]; // 2 + 3 opções sigilosas
    for (i, v) in votos.iter().enumerate() {
        let rs = c.votar_misto(i, v);
        assert_eq!(rs.len(), 5, "só as perguntas sigilosas geram r");
        for j in 0..5 {
            soma_r[j] += rs[j];
        }
    }

    // A parte pública já está somada em claro, antes de qualquer apuração.
    let pub_agora = c.cliente.total_publico(&c.proposta);
    assert_eq!(pub_agora.get(0).unwrap(), 5, "5 aprovaram as contas");
    assert_eq!(pub_agora.get(1).unwrap(), 1, "1 rejeitou");

    c.env.ledger().set_sequence_number(FECHA_EM + 1);

    // A mesa abre só as perguntas sigilosas, achatadas em ordem.
    let mut t = Vec::new(&c.env);
    for n in [4u32, 2, 2, 2, 2] {
        t.push_back(n);
    }
    let mut a = Vec::new(&c.env);
    for j in 0..5 {
        a.push_back(escalar(&c.env, &soma_r[j]));
    }

    let r = c.endossar(3, &t, &a).unwrap().unwrap();
    assert_eq!(r.len(), 7, "2 públicas + 2 + 3 sigilosas");
    // pergunta 1, pública
    assert_eq!(r.get(0).unwrap(), 5);
    assert_eq!(r.get(1).unwrap(), 1);
    // pergunta 2, sigilosa: 4 contra, 2 a favor
    assert_eq!(r.get(2).unwrap(), 4);
    assert_eq!(r.get(3).unwrap(), 2);
    // pergunta 3, sigilosa: 2 na cadeira 0, 2 na 1, 2 na 2
    assert_eq!(r.get(4).unwrap(), 2);
    assert_eq!(r.get(5).unwrap(), 2);
    assert_eq!(r.get(6).unwrap(), 2);
}

/// **A prova não migra de pergunta.**
///
/// O furo que a cédula mista cria e que a de uma pergunta só não tinha: sem o
/// índice da pergunta no desafio de Fiat–Shamir, a disjuntiva da pergunta 1
/// valeria para a pergunta 2, e o eleitor marcaria a segunda sem provar nada
/// sobre ela. Aqui ele tenta exatamente isso.
#[test]
fn prova_nao_migra_entre_perguntas() {
    let c = montar_cedula(16, &[(2, true), (2, true)]);
    let (cs, provas, psomas, pubs, _) = c.cedula_mista(0, &[0, 0]);

    // Copia a pergunta 1 por cima da pergunta 2 — compromissos, disjuntivas
    // e prova de soma. Tudo honesto; só está no lugar errado.
    let mut cs2 = Vec::new(&c.env);
    let mut pr2 = Vec::new(&c.env);
    for j in 0..2u32 {
        cs2.push_back(cs.get(j).unwrap());
        pr2.push_back(provas.get(j).unwrap());
    }
    for j in 0..2u32 {
        cs2.push_back(cs.get(j).unwrap());
        pr2.push_back(provas.get(j).unwrap());
    }
    let mut ps2 = Vec::new(&c.env);
    ps2.push_back(psomas.get(0).unwrap());
    ps2.push_back(psomas.get(0).unwrap());

    let (caminho, indice) = c.caminho(0);
    assert_eq!(
        c.cliente.try_votar(
            &c.proposta, &c.aptos[0], &cs2, &pr2, &ps2, &pubs, &caminho, &indice, &1u32
        ),
        Err(Ok(Erro::ProvaBinariaInvalida)),
        "a prova da pergunta 1 não pode valer na 2"
    );
}

/// **Uma prova de soma por pergunta — é correção, não otimização.**
///
/// Com uma prova só sobre a cédula inteira, `Σ(tudo) = peso` obrigaria quem
/// responde a pergunta 1 a abster-se da 2. O contrato exige uma por pergunta e
/// recusa a cédula que traga menos.
#[test]
fn falta_uma_prova_de_soma() {
    let c = montar_cedula(16, &[(2, true), (2, true)]);
    let (cs, provas, psomas, pubs, _) = c.cedula_mista(0, &[0, 1]);

    let mut so_uma = Vec::new(&c.env);
    so_uma.push_back(psomas.get(0).unwrap());

    let (caminho, indice) = c.caminho(0);
    assert_eq!(
        c.cliente.try_votar(
            &c.proposta, &c.aptos[0], &cs, &provas, &so_uma, &pubs, &caminho, &indice, &1u32
        ),
        Err(Ok(Erro::ArgumentoMalFormado))
    );
}

/// A parte pública de uma cédula mista obedece às mesmas regras, **por
/// pergunta** — não dá para compensar uma pergunta com outra.
#[test]
fn a_parte_publica_fecha_por_pergunta() {
    let c = montar_cedula(16, &[(2, false), (2, false), (2, true)]);
    let (cs, provas, psomas, _, _) = c.cedula_mista(0, &[0, 0, 0]);
    let (caminho, indice) = c.caminho(0);

    // Duas marcas na pergunta 1 e nenhuma na 2. A soma da cédula inteira é 2,
    // e seria 2 também se fosse uma marca em cada — mas por pergunta não fecha.
    let mut torta = Vec::new(&c.env);
    for v in [1u32, 1, 0, 0] {
        torta.push_back(v);
    }
    assert_eq!(
        c.cliente.try_votar(
            &c.proposta, &c.aptos[0], &cs, &provas, &psomas, &torta, &caminho, &indice, &1u32
        ),
        Err(Ok(Erro::SomaDiferenteDoPeso))
    );
}

/// **O custo da cédula mista, medido.**
///
/// A conta que decide o desenho: as perguntas públicas são praticamente de
/// graça, e o custo fixo — a prova de aptidão por Merkle — é pago **uma vez**,
/// o que torna uma cédula mista mais barata que as mesmas perguntas abertas
/// como propostas separadas.
#[test]
fn orcamento_da_cedula_mista() {
    let so_conf = montar_cedula(16, &[(2, true)]);
    let (cs, pr, ps, pb, _) = so_conf.cedula_mista(0, &[0]);
    let (cam, ind) = so_conf.caminho(0);
    let uma = cpu(&so_conf.env, || {
        so_conf
            .cliente
            .votar(&so_conf.proposta, &so_conf.aptos[0], &cs, &pr, &ps, &pb, &cam, &ind, &1u32)
    });

    let mista = montar_cedula(16, &[(2, false), (2, true), (2, true)]);
    let (cs, pr, ps, pb, _) = mista.cedula_mista(0, &[0, 0, 0]);
    let (cam, ind) = mista.caminho(0);
    let tres = cpu(&mista.env, || {
        mista
            .cliente
            .votar(&mista.proposta, &mista.aptos[0], &cs, &pr, &ps, &pb, &cam, &ind, &1u32)
    });

    std::println!("uma pergunta sigilosa ............ {:>12}", uma);
    std::println!("mista: 1 pública + 2 sigilosas ... {:>12}", tres);
    std::println!("duas propostas separadas ......... {:>12}", uma * 2);

    // Duas perguntas sigilosas numa cédula custam menos que duas propostas
    // separadas, porque a aptidão é provada uma vez só.
    assert!(
        tres < uma * 2,
        "a cédula mista tem de ser mais barata: {} vs {}",
        tres,
        uma * 2
    );
    assert!(tres < 400_000_000 / 2, "e tem de caber com folga");
}
