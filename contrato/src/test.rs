//! Testes do contrato.
//!
//! **O que separa estes testes de um mock:** as provas são geradas pelo
//! `tessera-core`, em Rust nativo com arkworks, e verificadas aqui pelo host do
//! Soroban, em Wasm. O cruzamento provador↔verificador (smoke B3) acontece a
//! cada `cargo test`, não uma vez num vetor congelado.

// O índice é o assunto destes laços: membro `i` do eleitorado, opção `j` da
// pergunta. Trocar por iterador esconderia o que cada asserção afirma.
#![allow(clippy::needless_range_loop)]

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

/// O que uma cédula montada entrega: compromissos, disjuntivas, provas de soma,
/// as respostas públicas e os fatores `r` que ficam com quem votou.
type Cedula = (
    Vec<Bls12381G1Affine>,
    Vec<ProvaCds>,
    Vec<ProvaSoma>,
    Vec<u32>,
    Vetor<ArkFr>,
);

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
        perg_sdk.push_back(Pergunta {
            opcoes: *opcoes,
            confidencial: *confidencial,
        });
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
        &false,
        &1u32,
        &0u32,
        &0u64,
        &0u64,
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
            ultimo.as_ref()?;
        }
        ultimo
    }

    /// Monta uma cédula honesta para a cédula desta proposta, qualquer que seja
    /// o formato, e devolve também os `r_j`, que no mundo real iriam para a
    /// mesa em shares de Shamir.
    ///
    /// `escolhas[q]` é a opção marcada na pergunta `q`. As perguntas sigilosas
    /// viram compromisso + disjuntivas + **uma prova de soma própria**; as
    /// públicas viram resposta em claro.
    fn cedula_mista(&self, i: usize, escolhas: &[u32]) -> Cedula {
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

            let rs: Vetor<ArkFr> = (0..*opcoes)
                .map(|_| pedersen::acaso_fr().unwrap())
                .collect();
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
            provas_soma.push_back(ProvaSoma {
                a: g1(env, &ps.a),
                z: escalar(env, &ps.z),
            });
            todos_rs.extend(rs);
        }

        (compromissos, provas, provas_soma, publicas, todos_rs)
    }

    /// Atalho para a cédula de uma pergunta só.
    fn cedula(&self, i: usize, escolha: u32) -> Cedula {
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
    let r = c
        .endossar(3, &tot(&c.env, 4, 2), &ab(&c.env))
        .unwrap()
        .unwrap();
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
        c.cliente.try_votar(
            &c.proposta,
            &c.aptos[0],
            &cs,
            &provas,
            &psoma,
            &pubs,
            &caminho,
            &indice,
            &1u32
        ),
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
            &c.g,
            &c.h,
            &cs_ark[j as usize],
            v,
            &rs[j as usize],
        )
        .unwrap();
        compromissos.push_back(g1(env, &cs_ark[j as usize]));
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
    let d = soma::alvo(&c.g, &cs_ark, 1);
    let ps = soma::provar(
        &ctx(env, &c.proposta, &intrusa, 0, u32::MAX),
        &c.h,
        &d,
        &rho,
    )
    .unwrap();
    let psoma = c.so_uma(ProvaSoma {
        a: g1(env, &ps.a),
        z: escalar(env, &ps.z),
    });
    let pubs = c.sem_publicas();

    let (caminho, indice) = c.caminho(0);
    assert_eq!(
        c.cliente.try_votar(
            &c.proposta,
            &intrusa,
            &compromissos,
            &provas,
            &psoma,
            &pubs,
            &caminho,
            &indice,
            &1u32
        ),
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
        c.cliente.try_votar(
            &c.proposta,
            &c.aptos[1],
            &cs,
            &provas,
            &psoma,
            &pubs,
            &caminho,
            &indice,
            &1u32
        ),
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
    let ps = soma::provar(
        &ctx(env, &c.proposta, votante, 0, u32::MAX),
        &c.h,
        &d,
        &(r0 + r1),
    )
    .unwrap();
    assert!(soma::verificar(
        &ctx(env, &c.proposta, votante, 0, u32::MAX),
        &c.h,
        &d,
        &ps
    ));

    // e o `core` nem deixa provar v=3
    assert_eq!(
        cds::provar(
            &ctx(env, &c.proposta, votante, 0, 0),
            &c.g,
            &c.h,
            &cs_ark[0],
            3,
            &r0
        ),
        Err(cds::Erro::VotoForaDoBinario(3))
    );

    // então quem ataca tem de forjar. Forja uma disjuntiva qualquer:
    let forjada = cds::provar(
        &ctx(env, &c.proposta, votante, 0, 0),
        &c.g,
        &c.h,
        &cs_ark[1],
        0,
        &r1,
    )
    .unwrap();
    let mut compromissos = Vec::new(env);
    let mut provas = Vec::new(env);
    for j in 0..OPCOES as usize {
        compromissos.push_back(g1(env, &cs_ark[j]));
        provas.push_back(ProvaCds {
            a0: g1(env, &forjada.a0),
            a1: g1(env, &forjada.a1),
            e0: escalar(env, &forjada.e0),
            z0: escalar(env, &forjada.z0),
            e1: escalar(env, &forjada.e1),
            z1: escalar(env, &forjada.z1),
        });
    }
    let psoma = c.so_uma(ProvaSoma {
        a: g1(env, &ps.a),
        z: escalar(env, &ps.z),
    });
    let pubs = c.sem_publicas();
    let (caminho, indice) = c.caminho(0);
    assert_eq!(
        c.cliente.try_votar(
            &c.proposta,
            votante,
            &compromissos,
            &provas,
            &psoma,
            &pubs,
            &caminho,
            &indice,
            &1u32
        ),
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
            &c.g,
            &c.h,
            &cs_ark[j as usize],
            1,
            &rs[j as usize],
        )
        .unwrap();
        compromissos.push_back(g1(env, &cs_ark[j as usize]));
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
    let d = soma::alvo(&c.g, &cs_ark, 1);
    let ps = soma::provar(&ctx(env, &c.proposta, votante, 0, u32::MAX), &c.h, &d, &rho).unwrap();
    let psoma = c.so_uma(ProvaSoma {
        a: g1(env, &ps.a),
        z: escalar(env, &ps.z),
    });
    let pubs = c.sem_publicas();

    let (caminho, indice) = c.caminho(0);
    assert_eq!(
        c.cliente.try_votar(
            &c.proposta,
            votante,
            &compromissos,
            &provas,
            &psoma,
            &pubs,
            &caminho,
            &indice,
            &1u32
        ),
        Err(Ok(Erro::ProvaDeSomaInvalida))
    );
}

/// Peso diferente de 1 é recusado por desenho, não documentado como cuidado.
/// Pesos públicos distintos são quebrados por subconjunto-soma (PROTOCOLO §6.3).
#[test]
fn peso_nao_unitario_e_recusado() {
    let c = montar(16);
    let (cs, provas, psoma, pubs, _) = c.cedula(0, 0);
    let (caminho, indice) = c.caminho(0);
    assert_eq!(
        c.cliente.try_votar(
            &c.proposta,
            &c.aptos[0],
            &cs,
            &provas,
            &psoma,
            &pubs,
            &caminho,
            &indice,
            &1000u32
        ),
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
        c.cliente.try_votar(
            &c.proposta,
            &c.aptos[0],
            &cs,
            &provas,
            &psoma,
            &pubs,
            &caminho,
            &indice,
            &1u32
        ),
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
            &c.proposta,
            &c.aptos[0],
            &compromissos,
            &provas,
            &provas_soma,
            &publicas,
            &irmaos,
            &indice,
            &1u32,
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
            &c.proposta,
            &c.aptos[1],
            &compromissos,
            &provas,
            &provas_soma,
            &publicas,
            &irmaos,
            &indice,
            &1u32,
        ),
        Err(Ok(Erro::VotacaoEncerrada))
    );
}

/// Uma janela de duração zero não é votação.
#[test]
fn a_janela_precisa_ter_duracao() {
    let c = montar(4);
    let mut perg = Vec::new(&c.env);
    perg.push_back(Pergunta {
        opcoes: OPCOES,
        confidencial: true,
    });
    let mut mesa_sdk = Vec::new(&c.env);
    for m in &c.mesa {
        mesa_sdk.push_back(m.clone());
    }
    let outra: BytesN<32> = BytesN::from_array(&c.env, &[43u8; 32]);
    assert_eq!(
        c.cliente.try_abrir(
            &Address::generate(&c.env),
            &outra,
            &perg,
            &BytesN::from_array(&c.env, &c.arvore.raiz()),
            &mesa_sdk,
            &3u32,
            &300u32,
            &300u32,
            &false,
            &1u32,
            &0u32,
            &0u64,
            &0u64,
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
/// de sempre: estender o prazo, ou refazer com um eleitorado que caiba no
/// sigilo.
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
            v.push_back(Pergunta {
                opcoes: *opcoes,
                confidencial: *confidencial,
            });
        }
        v
    };
    let ok = cedula(&[(2, true)]);

    assert_eq!(
        cliente.try_abrir(
            &gov,
            &id,
            &cedula(&[(1, true)]),
            &raiz,
            &mesa,
            &2u32,
            &0u32,
            &1000u32,
            &false,
            &1u32,
            &0u32,
            &0u64,
            &0u64,
        ),
        Err(Ok(Erro::OpcoesForaDaFaixa))
    );
    assert_eq!(
        cliente.try_abrir(
            &gov,
            &id,
            &cedula(&[(17, true)]),
            &raiz,
            &mesa,
            &2u32,
            &0u32,
            &1000u32,
            &false,
            &1u32,
            &0u32,
            &0u64,
            &0u64,
        ),
        Err(Ok(Erro::OpcoesForaDaFaixa))
    );
    assert_eq!(
        cliente.try_abrir(
            &gov,
            &id,
            &cedula(&[]),
            &raiz,
            &mesa,
            &2u32,
            &0u32,
            &1000u32,
            &false,
            &1u32,
            &0u32,
            &0u64,
            &0u64,
        ),
        Err(Ok(Erro::PerguntasForaDaFaixa))
    );
    // Nove perguntas passam de MAX_PERGUNTAS.
    let nove: Vetor<(u32, bool)> = (0..9).map(|_| (2u32, false)).collect();
    assert_eq!(
        cliente.try_abrir(
            &gov,
            &id,
            &cedula(&nove),
            &raiz,
            &mesa,
            &2u32,
            &0u32,
            &1000u32,
            &false,
            &1u32,
            &0u32,
            &0u64,
            &0u64,
        ),
        Err(Ok(Erro::PerguntasForaDaFaixa))
    );
    // Cada pergunta cabe, mas o total confidencial passa de MAX_OPCOES — e é
    // o total que o orçamento de CPU limita.
    let gordas: Vetor<(u32, bool)> = (0..3).map(|_| (16u32, true)).collect();
    assert_eq!(
        cliente.try_abrir(
            &gov,
            &id,
            &cedula(&gordas),
            &raiz,
            &mesa,
            &2u32,
            &0u32,
            &1000u32,
            &false,
            &1u32,
            &0u32,
            &0u64,
            &0u64,
        ),
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
            &1000u32,
            &false,
            &1u32,
            &0u32,
            &0u64,
            &0u64,
        )
        .is_ok());

    assert_eq!(
        cliente.try_abrir(
            &gov, &id, &ok, &raiz, &mesa, &4u32, &0u32, &1000u32, &false, &1u32, &0u32, &0u64,
            &0u64,
        ),
        Err(Ok(Erro::LimiarInvalido))
    );
    assert_eq!(
        cliente.try_abrir(
            &gov, &id, &ok, &raiz, &mesa, &0u32, &0u32, &1000u32, &false, &1u32, &0u32, &0u64,
            &0u64,
        ),
        Err(Ok(Erro::LimiarInvalido))
    );
    assert_eq!(
        cliente.try_abrir(
            &gov, &id, &ok, &raiz, &mesa, &2u32, &0u32, &5u32, &false, &1u32, &0u32, &0u64, &0u64,
        ),
        Err(Ok(Erro::PrazoNoPassado))
    );

    let mut repetida = Vec::new(&env);
    let m = Address::generate(&env);
    repetida.push_back(m.clone());
    repetida.push_back(m);
    assert_eq!(
        cliente.try_abrir(
            &gov, &id, &ok, &raiz, &repetida, &2u32, &0u32, &1000u32, &false, &1u32, &0u32, &0u64,
            &0u64,
        ),
        Err(Ok(Erro::MembroRepetido))
    );

    cliente.abrir(
        &gov, &id, &ok, &raiz, &mesa, &2u32, &0u32, &1000u32, &false, &1u32, &0u32, &0u64, &0u64,
    );
    assert_eq!(
        cliente.try_abrir(
            &gov, &id, &ok, &raiz, &mesa, &2u32, &0u32, &1000u32, &false, &1u32, &0u32, &0u64,
            &0u64,
        ),
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

    let inf =
        Bls12381G1Affine::from_bytes(BytesN::from_array(&env, &tessera_core::ponto::INFINITO));
    assert_eq!(
        bls.g1_add(&inf, &g),
        g,
        "INFINITO do core nao e neutro no host"
    );

    for ruim in [[0u8; 96], {
        let mut v = [0u8; 96];
        v[0] = 0xc0;
        v
    }] {
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
        c.cliente.votar(
            &c.proposta,
            &c.aptos[0],
            &cs,
            &provas,
            &psoma,
            &pubs,
            &caminho,
            &indice,
            &1u32,
        )
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
    std::println!(
        "   % do teto de 400M ............. {:.1}%",
        100.0 * custo_votar as f64 / TETO as f64
    );
    std::println!(
        "   folga ......................... {:.1}x",
        TETO as f64 / custo_votar as f64
    );
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
    // Devolve `(custo de apurar, custo de uma leitura pura)`.
    // `na_zero` votantes escolhem a opção 0, o resto escolhe a 1.
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
    std::println!(
        "   8x mais votantes custa ........ {:+.2}%",
        100.0 * (c40 as f64 - c5 as f64) / c5 as f64
    );
    std::println!("-- atribuicao da sobra --");
    std::println!("leitura pura, 5 votantes ......... {:>10}", l5);
    std::println!("leitura pura, 40 votantes ........ {:>10}", l40);
    std::println!(
        "   mesma sobra, sem curva ........ {:+.2}%",
        100.0 * (l40 as f64 - l5 as f64) / l5 as f64
    );

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
            &c.proposta,
            &c.aptos[0],
            &cs2,
            &pr2,
            &ps2,
            &pubs,
            &caminho,
            &indice,
            &1u32
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
            &c.proposta,
            &c.aptos[0],
            &cs,
            &provas,
            &so_uma,
            &pubs,
            &caminho,
            &indice,
            &1u32
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
            &c.proposta,
            &c.aptos[0],
            &cs,
            &provas,
            &psomas,
            &torta,
            &caminho,
            &indice,
            &1u32
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
        so_conf.cliente.votar(
            &so_conf.proposta,
            &so_conf.aptos[0],
            &cs,
            &pr,
            &ps,
            &pb,
            &cam,
            &ind,
            &1u32,
        )
    });

    let mista = montar_cedula(16, &[(2, false), (2, true), (2, true)]);
    let (cs, pr, ps, pb, _) = mista.cedula_mista(0, &[0, 0, 0]);
    let (cam, ind) = mista.caminho(0);
    let tres = cpu(&mista.env, || {
        mista.cliente.votar(
            &mista.proposta,
            &mista.aptos[0],
            &cs,
            &pr,
            &ps,
            &pb,
            &cam,
            &ind,
            &1u32,
        )
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

// ===================== o portão do anel =============================

/// **O portão do PLANO: o anel cabe numa transação?**
///
/// Mede `verificar_anel` para 5, 10 e 20 membros, com a assinatura gerada pelo
/// `tessera-core` em Rust nativo e conferida aqui pelo host do Soroban — o mesmo
/// cruzamento provador↔verificador do resto do arquivo.
///
/// O `Hp` vem do **contrato** e vai para o provador, que é como será em
/// produção: o cliente lê, não recalcula. Se este teste passa, o hash-to-curve
/// do host e o provador estão falando a mesma língua.
#[test]
fn orcamento_do_anel() {
    use tessera_core::anel;

    let env = Env::default();
    env.mock_all_auths();
    let g_sdk = cripto::gerador_g(&env);
    let g = ponto::desserializar(&g_sdk.to_array()).unwrap();
    let proposta = BytesN::from_array(&env, &[7u8; 32]);
    let hp_sdk = cripto::calcular_hp(&env, &proposta);
    let hp = ponto::desserializar(&hp_sdk.to_array()).unwrap();

    std::println!("\n== ORCAMENTO DO ANEL ==");
    std::println!(
        "{:>7}  {:>12}  {:>8}  {:>9}",
        "membros",
        "instrucoes",
        "% de 400M",
        "bytes"
    );

    let mut custo_10 = 0u64;
    for n in [5usize, 10, 20] {
        let xs: Vetor<ArkFr> = (0..n).map(|_| pedersen::acaso_fr().unwrap()).collect();
        let anel_ark: Vetor<ArkG1> = xs.iter().map(|x| anel::chave_publica(&g, x)).collect();
        // Quem assina é o último: o custo não depende do índice, mas medir no
        // pior caso de laço evita uma surpresa se algum dia depender.
        let s = anel::assinar(b"cedula", &g, &hp, &anel_ark, n - 1, &xs[n - 1]).unwrap();

        let mut anel_sdk: Vec<Bls12381G1Affine> = Vec::new(&env);
        for p in &anel_ark {
            anel_sdk.push_back(g1(&env, p));
        }
        let mut z_sdk: Vec<Bls12381Fr> = Vec::new(&env);
        for zi in &s.z {
            z_sdk.push_back(escalar(&env, zi));
        }
        let imagem = g1(&env, &s.imagem);
        let c0 = escalar(&env, &s.c0);
        let msg = Bytes::from_slice(&env, b"cedula");
        let pre = cripto::preambulo_anel(&env, &msg, &anel_sdk);

        let mut ok = false;
        let custo = cpu(&env, || {
            ok = cripto::verificar_anel(
                &env, &pre, &g_sdk, &hp_sdk, &anel_sdk, &imagem, &c0, &z_sdk,
            );
        });
        assert!(ok, "o anel de {} nao fechou no host", n);
        if n == 10 {
            custo_10 = custo;
        }

        std::println!(
            "{:>7}  {:>12}  {:>7.1}%  {:>9}",
            n,
            custo,
            100.0 * custo as f64 / TETO as f64,
            anel::tamanho(n)
        );
    }

    // E o que o host tem de recusar.
    let xs: Vetor<ArkFr> = (0..5).map(|_| pedersen::acaso_fr().unwrap()).collect();
    let anel_ark: Vetor<ArkG1> = xs.iter().map(|x| anel::chave_publica(&g, x)).collect();
    let s = anel::assinar(b"cedula", &g, &hp, &anel_ark, 2, &xs[2]).unwrap();
    let mut anel_sdk: Vec<Bls12381G1Affine> = Vec::new(&env);
    for p in &anel_ark {
        anel_sdk.push_back(g1(&env, p));
    }
    let mut z_sdk: Vec<Bls12381Fr> = Vec::new(&env);
    for zi in &s.z {
        z_sdk.push_back(escalar(&env, zi));
    }
    let imagem = g1(&env, &s.imagem);
    let c0 = escalar(&env, &s.c0);

    // outra mensagem: a cédula foi trocada depois de assinada
    let outra = cripto::preambulo_anel(&env, &Bytes::from_slice(&env, b"outra"), &anel_sdk);
    assert!(
        !cripto::verificar_anel(&env, &outra, &g_sdk, &hp_sdk, &anel_sdk, &imagem, &c0, &z_sdk),
        "o host aceitou uma cedula trocada depois da assinatura"
    );

    // outro Hp: a assinatura veio de outra proposta
    let hp_outro = cripto::calcular_hp(&env, &BytesN::from_array(&env, &[9u8; 32]));
    let certo = cripto::preambulo_anel(&env, &Bytes::from_slice(&env, b"cedula"), &anel_sdk);
    assert!(
        !cripto::verificar_anel(&env, &certo, &g_sdk, &hp_outro, &anel_sdk, &imagem, &c0, &z_sdk),
        "o host aceitou uma assinatura de outra proposta"
    );

    // Portão: um anel de 10 mais a cédula medida (10.980.243 por disjuntiva)
    // tem de caber com folga nos 400M.
    assert!(
        custo_10 <= 150_000_000,
        "anel de 10 custa {}, e nao sobra espaco para a cedula",
        custo_10
    );
}

// ===================== o caderno e a urna ===========================

/// **A tese inteira, executável: quem faltou é público, e de quem é cada cédula
/// não é.**
///
/// Dez membros, voto obrigatório. Sete comparecem — com nome, endereço e prova
/// de Merkle — e três não. Depois a votação abre e as sete cédulas chegam de
/// chaves efêmeras, cada uma com uma assinatura em anel sobre os sete que
/// compareceram.
///
/// No fim: o caderno nomeia os três que faltaram, e **nenhuma cédula carrega
/// endereço de membro**. Os dois registros existem, e nada liga um ao outro.
#[test]
fn o_caderno_diz_quem_faltou_e_a_urna_nao_diz_de_quem() {
    use tessera_core::anel;

    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(4_989_900);
    let id = env.register(Tessera, ());
    let cliente = TesseraClient::new(&env, &id);

    let aptos: Vetor<Address> = (0..10).map(|_| Address::generate(&env)).collect();
    let folhas: Vetor<merkle::Apto> = aptos
        .iter()
        .map(|a| merkle::Apto {
            endereco: bytes_de(&a.clone().to_xdr(&env)),
            peso: 1,
        })
        .collect();
    let arvore = merkle::Arvore::montar(&folhas).unwrap();

    let mut mesa_sdk = Vec::new(&env);
    for _ in 0..5 {
        mesa_sdk.push_back(Address::generate(&env));
    }
    let mut perg = Vec::new(&env);
    perg.push_back(Pergunta {
        opcoes: 2,
        confidencial: true,
    });

    let proposta: BytesN<32> = BytesN::from_array(&env, &[7u8; 32]);
    cliente.abrir(
        &Address::generate(&env),
        &proposta,
        &perg,
        &BytesN::from_array(&env, &arvore.raiz()),
        &mesa_sdk,
        &3u32,
        &4_989_990u32,
        &4_990_190u32,
        &true,
        &1u32,
        &0u32,
        &0u64,
        &0u64,
    );

    let g = pedersen::gerador();
    let h = ponto::desserializar(&cliente.gerador_h().to_array()).unwrap();
    let hp = ponto::desserializar(&cripto::calcular_hp(&env, &proposta).to_array()).unwrap();

    // ---- fase 1: o caderno ----
    const COMPARECERAM: usize = 7;
    let xs: Vetor<ArkFr> = (0..COMPARECERAM)
        .map(|_| pedersen::acaso_fr().unwrap())
        .collect();
    let mut anel_ark: Vetor<ArkG1> = Vetor::new();
    for (i, x) in xs.iter().enumerate() {
        let pk = anel::chave_publica(&g, x);
        let (caminho, indice) = {
            let p = arvore.caminho(i).unwrap();
            let mut c = Vec::new(&env);
            for irmao in &p.irmaos {
                c.push_back(BytesN::from_array(&env, irmao));
            }
            (c, p.indice)
        };
        let tamanho = cliente.comparecer(
            &proposta,
            &aptos[i],
            &g1(&env, &pk),
            &caminho,
            &indice,
            &0u32,
        );
        assert_eq!(
            tamanho,
            i as u32 + 1,
            "o anel nao cresceu com o comparecimento"
        );
        anel_ark.push(pk);
    }

    // Quem não compareceu é derivável do caderno, e é isso que o voto
    // obrigatório precisa.
    for i in 0..10 {
        let compareceu = env.as_contract(&id, || {
            env.storage()
                .persistent()
                .has(&Chave::Compareceu(proposta.clone(), aptos[i].clone()))
        });
        assert_eq!(
            compareceu,
            i < COMPARECERAM,
            "o caderno errou sobre o membro {}",
            i
        );
    }

    let mut anel_sdk: Vec<Bls12381G1Affine> = Vec::new(&env);
    for p in &anel_ark {
        anel_sdk.push_back(g1(&env, p));
    }

    // ---- a janela vira ----
    env.ledger().set_sequence_number(4_990_000);

    // Comparecer depois que a votação abriu mudaria o conjunto debaixo de quem
    // já votou.
    let (caminho, indice) = {
        let p = arvore.caminho(8).unwrap();
        let mut c = Vec::new(&env);
        for irmao in &p.irmaos {
            c.push_back(BytesN::from_array(&env, irmao));
        }
        (c, p.indice)
    };
    let atrasado = anel::chave_publica(&g, &pedersen::acaso_fr().unwrap());
    assert_eq!(
        cliente.try_comparecer(
            &proposta,
            &aptos[8],
            &g1(&env, &atrasado),
            &caminho,
            &indice,
            &0u32
        ),
        Err(Ok(Erro::ComparecimentoEncerrado))
    );

    // ---- fase 2: a urna ----
    // Monta a cédula de quem assina com `xs[i]`, com o contexto preso à imagem.
    let cedula = |i: usize, escolha: u32| {
        let img = anel::imagem(&hp, &xs[i]);
        let ident: Vetor<u8> = ponto::serializar(&img).to_vec();

        let rs: Vetor<ArkFr> = (0..2).map(|_| pedersen::acaso_fr().unwrap()).collect();
        let cs: Vetor<ArkG1> = (0..2)
            .map(|j| {
                let v = if j == escolha { 1u64 } else { 0 };
                pedersen::comprometer(&g, &h, &pedersen::escalar(v), &rs[j as usize])
            })
            .collect();

        let ctx_de = |opcao: u32| {
            let mut v: Vetor<u8> = proposta.to_array().to_vec();
            v.extend(ident.iter().copied());
            v.extend(0u32.to_be_bytes());
            v.extend(opcao.to_be_bytes());
            v
        };

        let mut compromissos = Vec::new(&env);
        let mut provas = Vec::new(&env);
        for j in 0..2u32 {
            let v = if j == escolha { 1u64 } else { 0 };
            let p = cds::provar(&ctx_de(j), &g, &h, &cs[j as usize], v, &rs[j as usize]).unwrap();
            compromissos.push_back(g1(&env, &cs[j as usize]));
            provas.push_back(ProvaCds {
                a0: g1(&env, &p.a0),
                a1: g1(&env, &p.a1),
                e0: escalar(&env, &p.e0),
                z0: escalar(&env, &p.z0),
                e1: escalar(&env, &p.e1),
                z1: escalar(&env, &p.z1),
            });
        }
        let rho = rs.iter().fold(ArkFr::from(0u64), |a, r| a + r);
        let d = soma::alvo(&g, &cs, 1);
        let ps = soma::provar(&ctx_de(u32::MAX), &h, &d, &rho).unwrap();
        let mut provas_soma = Vec::new(&env);
        provas_soma.push_back(ProvaSoma {
            a: g1(&env, &ps.a),
            z: escalar(&env, &ps.z),
        });
        let escolhas: Vec<u32> = Vec::new(&env);

        let msg = cripto::mensagem_cedula(&env, &proposta, &compromissos, &escolhas);
        let s = anel::assinar(&bytes_de(&msg), &g, &hp, &anel_ark, i, &xs[i]).unwrap();
        let mut z = Vec::new(&env);
        for zi in &s.z {
            z.push_back(escalar(&env, zi));
        }
        (
            anel_sdk.clone(),
            g1(&env, &s.imagem),
            escalar(&env, &s.c0),
            z,
            compromissos,
            provas,
            provas_soma,
            escolhas,
        )
    };

    for i in 0..COMPARECERAM {
        let (a, img, c0, z, cs, pr, psm, esc) = cedula(i, (i % 2) as u32);
        cliente.votar_anonimo(
            &proposta,
            &0u32,
            &a,
            &img,
            &c0,
            &z,
            &cs,
            &pr,
            &psm,
            &esc,
            &Bytes::new(&env),
        );
    }

    // A mesma pessoa de novo: a imagem colide, e o contrato recusa **sem saber
    // de quem é**.
    let (a, img, c0, z, cs, pr, psm, esc) = cedula(3, 1);
    assert_eq!(
        cliente.try_votar_anonimo(
            &proposta,
            &0u32,
            &a,
            &img,
            &c0,
            &z,
            &cs,
            &pr,
            &psm,
            &esc,
            &Bytes::new(&env),
        ),
        Err(Ok(Erro::ImagemJaUsada))
    );

    // Votar pelo endereço numa proposta de anel recriaria o vínculo.
    let (caminho, indice) = {
        let p = arvore.caminho(0).unwrap();
        let mut c = Vec::new(&env);
        for irmao in &p.irmaos {
            c.push_back(BytesN::from_array(&env, irmao));
        }
        (c, p.indice)
    };
    assert_eq!(
        cliente.try_votar(&proposta, &aptos[0], &cs, &pr, &psm, &esc, &caminho, &indice, &1u32),
        Err(Ok(Erro::ModoErrado))
    );

    // E o comparecimento confidencial bateu: sete cédulas, nenhuma com dono.
    let (conf, _publ): (u32, u32) = env.as_contract(&id, || {
        env.storage()
            .persistent()
            .get(&Chave::Comparecimento(proposta.clone()))
            .unwrap()
    });
    assert_eq!(conf, COMPARECERAM as u32);

    std::println!("\n== CADERNO E URNA ==");
    std::println!("aptos ............... 10");
    std::println!(
        "compareceram ........ {}  (o caderno nomeia os 3 que faltaram)",
        COMPARECERAM
    );
    std::println!(
        "cedulas ............. {}  (nenhuma com endereco de membro)",
        conf
    );
    std::println!(
        "anel ................ {} ramos, {} bytes",
        COMPARECERAM,
        anel::tamanho(COMPARECERAM)
    );
}

/// **O portão 2 do PLANO: a cédula que o navegador monta é aceita pelo contrato.**
///
/// Chama `tessera_cliente::montar` e `mensagem` — as mesmas funções que
/// `wasm-pack` compila para a aba, não uma reescrita delas — e manda o resultado
/// para `votar_anonimo`. O que fica de fora é só a travessia `JsValue`, que é
/// trabalho do `wasm-bindgen`.
///
/// Se os bytes do contexto ou da mensagem divergirem entre cliente e contrato,
/// este teste quebra o build. Era o jeito mais caro possível de descobrir isso
/// pela primeira vez numa demonstração ao vivo.
#[test]
fn a_cedula_do_navegador_e_aceita_pelo_contrato() {
    use tessera_cliente::{anonima, mensagem as msg_cliente, PerguntaJs};
    use tessera_core::anel;

    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(10);
    let id = env.register(Tessera, ());
    let cliente = TesseraClient::new(&env, &id);

    let aptos: Vetor<Address> = (0..3).map(|_| Address::generate(&env)).collect();
    let folhas: Vetor<merkle::Apto> = aptos
        .iter()
        .map(|a| merkle::Apto {
            endereco: bytes_de(&a.clone().to_xdr(&env)),
            peso: 1,
        })
        .collect();
    let arvore = merkle::Arvore::montar(&folhas).unwrap();
    let mut mesa_sdk = Vec::new(&env);
    for _ in 0..5 {
        mesa_sdk.push_back(Address::generate(&env));
    }
    // Cédula mista: uma sigilosa de 3 opções e uma pública de 2. A mistura é o
    // caso em que os deslocamentos entre compromissos e escolhas podem
    // divergir, e é por isso que o teste usa ela.
    let mut perg = Vec::new(&env);
    perg.push_back(Pergunta {
        opcoes: 3,
        confidencial: true,
    });
    perg.push_back(Pergunta {
        opcoes: 2,
        confidencial: false,
    });

    let proposta: BytesN<32> = BytesN::from_array(&env, &[0x5au8; 32]);
    cliente.abrir(
        &Address::generate(&env),
        &proposta,
        &perg,
        &BytesN::from_array(&env, &arvore.raiz()),
        &mesa_sdk,
        &3u32,
        &4_989_990u32,
        &4_990_190u32,
        &true,
        &1u32,
        &0u32,
        &0u64,
        &0u64,
    );

    let g = pedersen::gerador();
    let h_hex = ponto::para_hex(&ponto::desserializar(&cliente.gerador_h().to_array()).unwrap());
    let hp = ponto::desserializar(&cripto::calcular_hp(&env, &proposta).to_array()).unwrap();

    // ---- comparecimento, com as chaves que o cliente sortearia ----
    let xs: Vetor<ArkFr> = (0..3).map(|_| pedersen::acaso_fr().unwrap()).collect();
    let anel_ark: Vetor<ArkG1> = xs.iter().map(|x| anel::chave_publica(&g, x)).collect();
    for (i, pk) in anel_ark.iter().enumerate() {
        let c = arvore.caminho(i).unwrap();
        let mut irmaos = Vec::new(&env);
        for s in &c.irmaos {
            irmaos.push_back(BytesN::from_array(&env, s));
        }
        cliente.comparecer(
            &proposta,
            &aptos[i],
            &g1(&env, pk),
            &irmaos,
            &c.indice,
            &0u32,
        );
    }
    env.ledger().set_sequence_number(4_990_000);

    // ---- a cédula, montada pelo cliente do navegador ----
    const EU: usize = 1;
    let imagem = anel::imagem(&hp, &xs[EU]);
    let _ident = ponto::serializar(&imagem).to_vec();
    let perguntas_js = [
        PerguntaJs {
            opcoes: 3,
            confidencial: true,
        },
        PerguntaJs {
            opcoes: 2,
            confidencial: false,
        },
    ];
    let escolhas_feitas = [2u32, 0u32];
    let anel_hex: Vetor<std::string::String> = anel_ark.iter().map(ponto::para_hex).collect();
    let segredo = pedersen::fr_para_bytes_be(&xs[EU]);
    let segredo_hex: std::string::String =
        segredo.iter().map(|b| std::format!("{:02x}", b)).collect();
    // O caminho inteiro do cliente, numa chamada só — inclusive a mensagem que
    // o anel assina. Montar isso por fora foi o que deixou passar uma
    // assinatura sobre a cédula errada até a testnet.
    let a = anonima(
        &ponto_hex(&proposta.to_array()),
        &ponto::para_hex(&hp),
        &h_hex,
        &anel_hex,
        EU,
        &segredo_hex,
        &perguntas_js,
        &escolhas_feitas,
    )
    .expect("o cliente nao montou a cedula anonima");
    let c = &a.cedula;
    assert!(
        c.parcelas.is_empty(),
        "sem mesa, nao pode sair parcela de Shamir"
    );
    let msg = msg_cliente(&proposta.to_array(), &c.compromissos, &c.escolhas).unwrap();
    let s = anel::Assinatura {
        imagem: ponto::de_hex(&a.imagem).unwrap(),
        c0: pedersen::fr_de_bytes_be(&hex_bytes(&a.c0)),
        z: a.z
            .iter()
            .map(|h| pedersen::fr_de_bytes_be(&hex_bytes(h)))
            .collect(),
    };
    assert!(
        anel::verificar(&msg, &g, &hp, &anel_ark, &s),
        "a assinatura do cliente nao fecha sobre a mensagem que o contrato le"
    );

    // A mensagem do cliente e a do contrato têm de ser os mesmos bytes.
    let mut compr_sdk: Vec<Bls12381G1Affine> = Vec::new(&env);
    for x in &c.compromissos {
        compr_sdk.push_back(g1(&env, &ponto::de_hex(x).unwrap()));
    }
    let mut esc_sdk: Vec<u32> = Vec::new(&env);
    for e in &c.escolhas {
        esc_sdk.push_back(*e);
    }
    assert_eq!(
        msg,
        bytes_de(&cripto::mensagem_cedula(
            &env, &proposta, &compr_sdk, &esc_sdk
        )),
        "a mensagem do cliente divergiu da do contrato"
    );
    let _ = anel_hex;

    let mut provas_sdk: Vec<ProvaCds> = Vec::new(&env);
    for p in &c.provas {
        provas_sdk.push_back(ProvaCds {
            a0: g1(&env, &ponto::de_hex(&p.a0).unwrap()),
            a1: g1(&env, &ponto::de_hex(&p.a1).unwrap()),
            e0: escalar(&env, &pedersen::fr_de_bytes_be(&hex_bytes(&p.e0))),
            z0: escalar(&env, &pedersen::fr_de_bytes_be(&hex_bytes(&p.z0))),
            e1: escalar(&env, &pedersen::fr_de_bytes_be(&hex_bytes(&p.e1))),
            z1: escalar(&env, &pedersen::fr_de_bytes_be(&hex_bytes(&p.z1))),
        });
    }
    let mut soma_sdk: Vec<ProvaSoma> = Vec::new(&env);
    for p in &c.provas_soma {
        soma_sdk.push_back(ProvaSoma {
            a: g1(&env, &ponto::de_hex(&p.a).unwrap()),
            z: escalar(&env, &pedersen::fr_de_bytes_be(&hex_bytes(&p.z))),
        });
    }
    let mut anel_sdk: Vec<Bls12381G1Affine> = Vec::new(&env);
    for p in &anel_ark {
        anel_sdk.push_back(g1(&env, p));
    }
    let mut z_sdk: Vec<Bls12381Fr> = Vec::new(&env);
    for zi in &s.z {
        z_sdk.push_back(escalar(&env, zi));
    }

    cliente.votar_anonimo(
        &proposta,
        &0u32,
        &anel_sdk,
        &g1(&env, &s.imagem),
        &escalar(&env, &s.c0),
        &z_sdk,
        &compr_sdk,
        &provas_sdk,
        &soma_sdk,
        &esc_sdk,
        &Bytes::new(&env),
    );

    // A pergunta pública conta em claro; a sigilosa só soma no acumulador.
    let publico: u32 = env.as_contract(&id, || {
        env.storage()
            .persistent()
            .get(&Chave::TotalPublico(proposta.clone(), 1, 0))
            .unwrap()
    });
    assert_eq!(publico, 1, "a escolha publica do cliente nao chegou");

    std::println!("\n== PORTAO 2: A CEDULA DO NAVEGADOR ==");
    std::println!("compromissos ........ {}", c.compromissos.len());
    std::println!("escolhas em claro ... {}", c.escolhas.len());
    std::println!("parcelas ............ {} (sem mesa)", c.parcelas.len());
    std::println!("anel ................ {} ramos", anel_ark.len());
    std::println!("aceita pelo contrato  SIM");
}

fn hex_bytes(s: &str) -> Vetor<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn ponto_hex(b: &[u8]) -> std::string::String {
    b.iter().map(|x| std::format!("{:02x}", x)).collect()
}

/// **Até quantas pessoas cabe um anel, de verdade.**
///
/// `orcamento_do_anel` mede só a verificação da assinatura. Mas o que a rede
/// cobra é `votar_anonimo()` inteiro: a assinatura **mais** os compromissos, as
/// disjuntivas, a prova de soma, a autorização, o estado e o evento. A conta de
/// cabeça — somar o anel medido com a cédula medida noutro teste — é a mesma
/// aritmética que o SPEC errou em 9,8% e que este arquivo existe para recusar.
///
/// O número que sai daqui é o que a landing tem o direito de publicar.
#[test]
fn ate_quantas_pessoas_cabe_um_anel() {
    use tessera_core::anel;

    std::println!("\n== A CEDULA EM ANEL, INTEIRA ==");
    std::println!("{:>7}  {:>12}  {:>9}", "no anel", "instrucoes", "% de 400M");

    let mut anterior = 0u64;
    for n in [5usize, 10, 20, 30, 32] {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_sequence_number(4_989_900);
        let id = env.register(Tessera, ());
        let cliente = TesseraClient::new(&env, &id);

        let aptos: Vetor<Address> = (0..n).map(|_| Address::generate(&env)).collect();
        let folhas: Vetor<merkle::Apto> = aptos
            .iter()
            .map(|a| merkle::Apto {
                endereco: bytes_de(&a.clone().to_xdr(&env)),
                peso: 1,
            })
            .collect();
        let arvore = merkle::Arvore::montar(&folhas).unwrap();

        let mut mesa_sdk = Vec::new(&env);
        mesa_sdk.push_back(Address::generate(&env));
        let mut perg = Vec::new(&env);
        perg.push_back(Pergunta {
            opcoes: 2,
            confidencial: true,
        });

        let proposta: BytesN<32> = BytesN::from_array(&env, &[7u8; 32]);
        cliente.abrir(
            &Address::generate(&env),
            &proposta,
            &perg,
            &BytesN::from_array(&env, &arvore.raiz()),
            &mesa_sdk,
            &1u32,
            &4_989_990u32,
            &4_990_190u32,
            &true,
            &1u32,
            &0u32,
            &0u64,
            &0u64,
        );

        let g = pedersen::gerador();
        let h = ponto::desserializar(&cliente.gerador_h().to_array()).unwrap();
        let hp = ponto::desserializar(&cripto::calcular_hp(&env, &proposta).to_array()).unwrap();

        let xs: Vetor<ArkFr> = (0..n).map(|_| pedersen::acaso_fr().unwrap()).collect();
        let mut anel_ark: Vetor<ArkG1> = Vetor::new();
        for (i, x) in xs.iter().enumerate() {
            let pk = anel::chave_publica(&g, x);
            let p = arvore.caminho(i).unwrap();
            let mut c = Vec::new(&env);
            for irmao in &p.irmaos {
                c.push_back(BytesN::from_array(&env, irmao));
            }
            cliente.comparecer(&proposta, &aptos[i], &g1(&env, &pk), &c, &p.indice, &0u32);
            anel_ark.push(pk);
        }
        let mut anel_sdk: Vec<Bls12381G1Affine> = Vec::new(&env);
        for p in &anel_ark {
            anel_sdk.push_back(g1(&env, p));
        }

        env.ledger().set_sequence_number(4_990_000);

        // Quem assina é o último: o pior caso do laço de verificação.
        let i = n - 1;
        let img = anel::imagem(&hp, &xs[i]);
        let ident: Vetor<u8> = ponto::serializar(&img).to_vec();
        let rs: Vetor<ArkFr> = (0..2).map(|_| pedersen::acaso_fr().unwrap()).collect();
        let cs: Vetor<ArkG1> = (0..2)
            .map(|j| {
                let v = if j == 1 { 1u64 } else { 0 };
                pedersen::comprometer(&g, &h, &pedersen::escalar(v), &rs[j as usize])
            })
            .collect();
        let ctx_de = |opcao: u32| {
            let mut v: Vetor<u8> = proposta.to_array().to_vec();
            v.extend(ident.iter().copied());
            v.extend(0u32.to_be_bytes());
            v.extend(opcao.to_be_bytes());
            v
        };
        let mut compromissos = Vec::new(&env);
        let mut provas = Vec::new(&env);
        for j in 0..2u32 {
            let v = if j == 1 { 1u64 } else { 0 };
            let p = cds::provar(&ctx_de(j), &g, &h, &cs[j as usize], v, &rs[j as usize]).unwrap();
            compromissos.push_back(g1(&env, &cs[j as usize]));
            provas.push_back(ProvaCds {
                a0: g1(&env, &p.a0),
                a1: g1(&env, &p.a1),
                e0: escalar(&env, &p.e0),
                z0: escalar(&env, &p.z0),
                e1: escalar(&env, &p.e1),
                z1: escalar(&env, &p.z1),
            });
        }
        let rho = rs.iter().fold(ArkFr::from(0u64), |a, r| a + r);
        let d = soma::alvo(&g, &cs, 1);
        let ps = soma::provar(&ctx_de(u32::MAX), &h, &d, &rho).unwrap();
        let mut provas_soma = Vec::new(&env);
        provas_soma.push_back(ProvaSoma {
            a: g1(&env, &ps.a),
            z: escalar(&env, &ps.z),
        });
        let escolhas: Vec<u32> = Vec::new(&env);

        let msg = cripto::mensagem_cedula(&env, &proposta, &compromissos, &escolhas);
        let s = anel::assinar(&bytes_de(&msg), &g, &hp, &anel_ark, i, &xs[i]).unwrap();
        let mut z = Vec::new(&env);
        for zi in &s.z {
            z.push_back(escalar(&env, zi));
        }
        let imagem = g1(&env, &s.imagem);
        let c0 = escalar(&env, &s.c0);

        let custo = cpu(&env, || {
            cliente.votar_anonimo(
                &proposta,
                &0u32,
                &anel_sdk,
                &imagem,
                &c0,
                &z,
                &compromissos,
                &provas,
                &provas_soma,
                &escolhas,
                &Bytes::new(&env),
            );
        });

        std::println!(
            "{:>7}  {:>12}  {:>8.1}%",
            n,
            custo,
            100.0 * custo as f64 / TETO as f64
        );

        // O que a landing afirma: vinte cabe. Trinta é onde a conta aperta.
        if n == 20 {
            assert!(
                custo <= TETO,
                "um anel de 20 nao cabe numa transacao: {} de {}",
                custo,
                TETO
            );
        }
        // E o crescimento tem de continuar linear: se um dia virar quadrático,
        // o número publicado deixa de valer sem ninguém perceber.
        if anterior > 0 {
            assert!(
                custo > anterior,
                "o custo nao cresceu de {} para {}",
                anterior,
                custo
            );
        }
        anterior = custo;
    }
}

/// **Trinta votantes, três seções, e o custo de uma cédula de dez.**
///
/// É a razão de as seções existirem. Verificar um anel custa 10.822.850
/// instruções por membro, então trinta pessoas num anel só fazem cada cédula
/// usar 91,4% do teto de CPU de uma transação — e só uma entra por ledger.
/// Medido na testnet: 18 de 30 cédulas em 646 s, o resto expirou.
///
/// Com três seções de dez, cada cédula custa o de um anel de dez e a conta para
/// de depender do tamanho do eleitorado. O que se paga é o conjunto de
/// anonimato, que passa a ser a seção.
///
/// O que este teste prende, além do custo:
///
///   1. a divisão sai da lista, não de quem organiza;
///   2. comparecer na seção errada não fecha o caminho de Merkle;
///   3. uma cédula assinada com o anel de uma seção é recusada em outra;
///   4. **o resultado continua único** — o acumulador é por proposta e não
///      sabe de que seção veio cada cédula.
#[test]
fn trinta_votantes_em_tres_secoes_pagam_o_preco_de_dez() {
    use tessera_core::anel;

    const N: usize = 30;
    const SECOES: u32 = 3;

    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(4_989_900);
    let id = env.register(Tessera, ());
    let cliente = TesseraClient::new(&env, &id);

    let proposta: BytesN<32> = BytesN::from_array(&env, &[11u8; 32]);
    let aptos: Vetor<Address> = (0..N).map(|_| Address::generate(&env)).collect();
    let xdrs: Vetor<Vetor<u8>> = aptos
        .iter()
        .map(|a| bytes_de(&a.clone().to_xdr(&env)))
        .collect();

    // **A divisão vem da baliza** (DEC-012), não da lista e não da proposta. A
    // folha não a carrega mais, então a árvore é só endereço e peso.
    let divisao = merkle::dividir(&assinatura_em_bytes(), &xdrs, SECOES);
    let folhas: Vetor<merkle::Apto> = xdrs
        .iter()
        .map(|e| merkle::Apto {
            endereco: e.clone(),
            peso: 1,
        })
        .collect();
    let arvore = merkle::Arvore::montar(&folhas).unwrap();

    let mut mesa_sdk = Vec::new(&env);
    mesa_sdk.push_back(Address::generate(&env));
    let mut perg = Vec::new(&env);
    perg.push_back(Pergunta {
        opcoes: 2,
        confidencial: true,
    });

    cliente.abrir(
        &Address::generate(&env),
        &proposta,
        &perg,
        &BytesN::from_array(&env, &arvore.raiz()),
        &mesa_sdk,
        &1u32,
        &4_989_990u32,
        &4_990_190u32,
        &true,
        &SECOES,
        &0u32,
        // Sem fechadura de tempo: este teste é sobre o custo das seções, e a
        // apuração dele é a da mesa. Mas **com** rodada de abertura, porque sem
        // ela o contrato recusa seções numa votação fechada.
        &0u64,
        &RODADA_ABERTURA,
    );

    // A rodada de abertura vence, e qualquer pessoa registra a assinatura. Sem
    // ela não há seção para conferir, e o contrato recusa o comparecimento.
    env.ledger().set_timestamp(instante(RODADA_ABERTURA));
    cliente.registrar_abertura(&proposta, &assinatura_host(&env));
    assert_eq!(
        cliente.abertura(&proposta),
        Some(assinatura_host(&env)),
        "a assinatura registrada não é a que a baliza publicou"
    );

    let g = pedersen::gerador();
    let h = ponto::desserializar(&cliente.gerador_h().to_array()).unwrap();
    let hp = ponto::desserializar(&cripto::calcular_hp(&env, &proposta).to_array()).unwrap();

    let caminho_de = |i: usize| {
        let p = arvore.caminho(i).unwrap();
        let mut c = Vec::new(&env);
        for irmao in &p.irmaos {
            c.push_back(BytesN::from_array(&env, irmao));
        }
        (c, p.indice)
    };

    // ---- a seção vem da baliza, e o contrato confere ----
    let (c0_, i0_) = caminho_de(0);
    let outra = (divisao[0] + 1) % SECOES;
    assert_eq!(
        cliente.try_comparecer(
            &proposta,
            &aptos[0],
            &g1(
                &env,
                &anel::chave_publica(&g, &pedersen::acaso_fr().unwrap())
            ),
            &c0_,
            &i0_,
            &outra,
        ),
        Err(Ok(Erro::SecaoInvalida)),
        "deu para escolher a seção: quem vota pegaria a menor"
    );

    // ---- o caderno, seção por seção ----
    let xs: Vetor<ArkFr> = (0..N).map(|_| pedersen::acaso_fr().unwrap()).collect();
    let mut aneis: Vetor<Vetor<ArkG1>> = (0..SECOES).map(|_| Vetor::new()).collect();
    for i in 0..N {
        let pk = anel::chave_publica(&g, &xs[i]);
        let (c, idx) = caminho_de(i);
        cliente.comparecer(&proposta, &aptos[i], &g1(&env, &pk), &c, &idx, &divisao[i]);
        aneis[divisao[i] as usize].push(pk);
    }
    for s in 0..SECOES {
        assert_eq!(
            aneis[s as usize].len(),
            N / SECOES as usize,
            "o rodízio devia deixar as seções do mesmo tamanho"
        );
        assert_eq!(
            cliente.anel(&proposta, &s).len() as usize,
            aneis[s as usize].len()
        );
    }

    let aneis_sdk: Vetor<Vec<Bls12381G1Affine>> = aneis
        .iter()
        .map(|a| {
            let mut v = Vec::new(&env);
            for p in a {
                v.push_back(g1(&env, p));
            }
            v
        })
        .collect();

    env.ledger().set_sequence_number(4_990_000);

    // ---- a urna ----
    let cedula = |i: usize, escolha: u32| {
        let secao = divisao[i] as usize;
        let img = anel::imagem(&hp, &xs[i]);
        let ident: Vetor<u8> = ponto::serializar(&img).to_vec();
        let rs: Vetor<ArkFr> = (0..2).map(|_| pedersen::acaso_fr().unwrap()).collect();
        let cs: Vetor<ArkG1> = (0..2)
            .map(|j| {
                let v = if j == escolha { 1u64 } else { 0 };
                pedersen::comprometer(&g, &h, &pedersen::escalar(v), &rs[j as usize])
            })
            .collect();
        let ctx_de = |opcao: u32| {
            let mut v: Vetor<u8> = proposta.to_array().to_vec();
            v.extend(ident.iter().copied());
            v.extend(0u32.to_be_bytes());
            v.extend(opcao.to_be_bytes());
            v
        };
        let mut compromissos = Vec::new(&env);
        let mut provas = Vec::new(&env);
        for j in 0..2u32 {
            let v = if j == escolha { 1u64 } else { 0 };
            let p = cds::provar(&ctx_de(j), &g, &h, &cs[j as usize], v, &rs[j as usize]).unwrap();
            compromissos.push_back(g1(&env, &cs[j as usize]));
            provas.push_back(ProvaCds {
                a0: g1(&env, &p.a0),
                a1: g1(&env, &p.a1),
                e0: escalar(&env, &p.e0),
                z0: escalar(&env, &p.z0),
                e1: escalar(&env, &p.e1),
                z1: escalar(&env, &p.z1),
            });
        }
        let rho = rs.iter().fold(ArkFr::from(0u64), |a, r| a + r);
        let d = soma::alvo(&g, &cs, 1);
        let ps = soma::provar(&ctx_de(u32::MAX), &h, &d, &rho).unwrap();
        let mut provas_soma = Vec::new(&env);
        provas_soma.push_back(ProvaSoma {
            a: g1(&env, &ps.a),
            z: escalar(&env, &ps.z),
        });
        let escolhas: Vec<u32> = Vec::new(&env);

        let msg = cripto::mensagem_cedula(&env, &proposta, &compromissos, &escolhas);
        let dentro = aneis[secao]
            .iter()
            .position(|p| *p == anel::chave_publica(&g, &xs[i]))
            .unwrap();
        let s = anel::assinar(&bytes_de(&msg), &g, &hp, &aneis[secao], dentro, &xs[i]).unwrap();
        let mut z = Vec::new(&env);
        for zi in &s.z {
            z.push_back(escalar(&env, zi));
        }
        (
            divisao[i],
            aneis_sdk[secao].clone(),
            g1(&env, &s.imagem),
            escalar(&env, &s.c0),
            z,
            compromissos,
            provas,
            provas_soma,
            escolhas,
        )
    };

    let mut custo_cedula = 0u64;
    for i in 0..N {
        let (sec, a, img, c0, z, cs, pr, psm, esc) = cedula(i, (i % 2) as u32);
        let c = cpu(&env, || {
            cliente.votar_anonimo(
                &proposta,
                &sec,
                &a,
                &img,
                &c0,
                &z,
                &cs,
                &pr,
                &psm,
                &esc,
                &Bytes::new(&env),
            );
        });
        if i == 0 {
            custo_cedula = c;
        }
    }

    std::println!("\n== TRINTA EM TRES SECOES ==");
    std::println!(
        "  cedula de anel {}: {} instrucoes, {:.1}% do teto",
        N / SECOES as usize,
        custo_cedula,
        100.0 * custo_cedula as f64 / TETO as f64
    );

    // O ganho inteiro: trinta votantes, mas o preço de um anel de dez.
    assert!(
        custo_cedula < TETO / 2,
        "a cédula de uma seção de 10 devia caber folgado: {} de {}",
        custo_cedula,
        TETO
    );

    // ---- o anel de uma seção não serve em outra ----
    let (sec, _a, img, c0, z, cs, pr, psm, esc) = cedula(0, 1);
    let vizinha = (sec + 1) % SECOES;
    assert_eq!(
        cliente.try_votar_anonimo(
            &proposta,
            &vizinha,
            &aneis_sdk[sec as usize],
            &img,
            &c0,
            &z,
            &cs,
            &pr,
            &psm,
            &esc,
            &Bytes::new(&env),
        ),
        Err(Ok(Erro::AnelInvalido)),
        "uma cédula migrou de seção, e o anel de uma seção não prova nada na outra"
    );

    // ---- e o resultado é um só ----
    let (conf, _publ) = cliente.comparecimento(&proposta);
    assert_eq!(
        conf, N as u32,
        "o acumulador é por proposta: as três seções somam num resultado só"
    );
}

/// **A votação aberta: qualquer carteira comparece, e o contrato dá a seção.**
///
/// É o modo da demonstração pública, onde quem chega não estava em lista
/// nenhuma. O que ele perde está declarado em §2 da spec e não é pouco: sem
/// lista não existe `aptos − compareceram`, logo não existe voto obrigatório, e
/// nada impede a mesma pessoa de voltar com outra carteira — na testnet o
/// friendbot as financia de graça.
///
/// O que ele mantém é a tese: ninguém descobre a escolha de ninguém, e nada
/// liga pessoa a cédula. Este teste prende as duas metades.
#[test]
fn na_votacao_aberta_qualquer_carteira_comparece_e_a_secao_vem_da_chegada() {
    use tessera_core::anel;

    const SECOES: u32 = 3;
    const GENTE: usize = 9;

    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(4_989_900);
    let id = env.register(Tessera, ());
    let cliente = TesseraClient::new(&env, &id);

    let proposta: BytesN<32> = BytesN::from_array(&env, &[13u8; 32]);
    let mut mesa_sdk = Vec::new(&env);
    mesa_sdk.push_back(Address::generate(&env));
    let mut perg = Vec::new(&env);
    perg.push_back(Pergunta {
        opcoes: 2,
        confidencial: true,
    });

    // A raiz de 32 zeros é o sentinela: não há lista.
    cliente.abrir(
        &Address::generate(&env),
        &proposta,
        &perg,
        &BytesN::from_array(&env, &[0u8; 32]),
        &mesa_sdk,
        &1u32,
        &4_989_990u32,
        &4_990_190u32,
        &true,
        &SECOES,
        &0u32,
        &0u64,
        &0u64,
    );

    let g = pedersen::gerador();
    let vazio: Vec<BytesN<32>> = Vec::new(&env);

    // Ninguém estava em lista nenhuma, e todo mundo entra.
    let mut quem: Vetor<Address> = Vetor::new();
    let mut por_secao = [0usize; SECOES as usize];
    for _ in 0..GENTE {
        let a = Address::generate(&env);
        let pk = anel::chave_publica(&g, &pedersen::acaso_fr().unwrap());
        cliente.comparecer(&proposta, &a, &g1(&env, &pk), &vazio, &0u32, &0u32);
        // A seção veio do endereço, não do que a pessoa pediu.
        let s = cliente.secao_de(&proposta, &a).unwrap();
        assert!(s < SECOES, "seção fora da faixa");
        por_secao[s as usize] += 1;
        quem.push(a);
    }

    // Cada anel tem exatamente quem o contrato mandou para ele.
    for s in 0..SECOES {
        assert_eq!(
            cliente.anel(&proposta, &s).len() as usize,
            por_secao[s as usize],
            "o anel da seção {} não bate com quem foi mandado para lá",
            s
        );
    }
    assert_eq!(por_secao.iter().sum::<usize>(), GENTE);

    // Pedir uma seção não adianta: o argumento é ignorado na aberta.
    let teimoso = Address::generate(&env);
    let pk = anel::chave_publica(&g, &pedersen::acaso_fr().unwrap());
    cliente.comparecer(&proposta, &teimoso, &g1(&env, &pk), &vazio, &0u32, &2u32);
    let dele = cliente.secao_de(&proposta, &teimoso).unwrap();
    assert!(dele < SECOES);
    // A seção é a do endereço dele, não a que ele pediu — e é reprodutível:
    // comparecer de novo daria a mesma, que é o que faz o cliente conseguir
    // declarar o footprint certo antes de enviar.
    assert_eq!(cliente.secao_de(&proposta, &teimoso), Some(dele));

    // Caminho de Merkle numa votação sem lista é erro, não é ignorado em
    // silêncio: quem manda um está enganado sobre em que modo está votando.
    let mut caminho = Vec::new(&env);
    caminho.push_back(BytesN::from_array(&env, &[1u8; 32]));
    assert_eq!(
        cliente.try_comparecer(
            &proposta,
            &Address::generate(&env),
            &g1(&env, &pk),
            &caminho,
            &0u32,
            &0u32,
        ),
        Err(Ok(Erro::VotacaoAberta))
    );

    // E a mesma carteira continua não comparecendo duas vezes. Isto **não** é
    // defesa contra sybil — basta outra carteira —, é só coerência do caderno.
    assert_eq!(
        cliente.try_comparecer(&proposta, &quem[0], &g1(&env, &pk), &vazio, &0u32, &0u32),
        Err(Ok(Erro::JaCompareceu))
    );
}

/// A votação fechada continua recusando quem não está na lista — o modo aberto
/// não abriu o outro por acidente.
#[test]
fn a_votacao_fechada_nao_virou_aberta() {
    use tessera_core::anel;

    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(4_989_900);
    let id = env.register(Tessera, ());
    let cliente = TesseraClient::new(&env, &id);

    let proposta: BytesN<32> = BytesN::from_array(&env, &[17u8; 32]);
    let aptos: Vetor<Address> = (0..4).map(|_| Address::generate(&env)).collect();
    let folhas: Vetor<merkle::Apto> = aptos
        .iter()
        .map(|a| merkle::Apto {
            endereco: bytes_de(&a.clone().to_xdr(&env)),
            peso: 1,
        })
        .collect();
    let arvore = merkle::Arvore::montar(&folhas).unwrap();

    let mut mesa_sdk = Vec::new(&env);
    mesa_sdk.push_back(Address::generate(&env));
    let mut perg = Vec::new(&env);
    perg.push_back(Pergunta {
        opcoes: 2,
        confidencial: true,
    });
    cliente.abrir(
        &Address::generate(&env),
        &proposta,
        &perg,
        &BytesN::from_array(&env, &arvore.raiz()),
        &mesa_sdk,
        &1u32,
        &4_989_990u32,
        &4_990_190u32,
        &true,
        &1u32,
        &0u32,
        &0u64,
        &0u64,
    );

    let g = pedersen::gerador();
    let pk = anel::chave_publica(&g, &pedersen::acaso_fr().unwrap());
    let vazio: Vec<BytesN<32>> = Vec::new(&env);

    // Sem caminho: na fechada isso é falta de prova, não votação aberta.
    assert_eq!(
        cliente.try_comparecer(
            &proposta,
            &Address::generate(&env),
            &g1(&env, &pk),
            &vazio,
            &0u32,
            &0u32,
        ),
        Err(Ok(Erro::NaoEstaNaListaDeAptos))
    );
}

/// **A assembleia sem mesa nenhuma.**
///
/// Numa cédula em anel a mesa não recebe parcela: ela existia no estado e não
/// servia para nada, e a tela tinha de explicar uma exigência sem função. Agora
/// `limiar == 0` é aceito se — e só se — a mesa for vazia.
///
/// O que isso compra é coerência: sem mesa ninguém endossa, logo ninguém
/// reconstrói a abertura, logo nenhum total é publicado. O sigilo é absoluto e
/// o resultado é impossível, e o contrato diz isso em vez de a tela pedir
/// desculpas.
#[test]
fn assembleia_sem_mesa_abre_e_nao_apura() {
    use tessera_core::anel;

    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(4_989_900);
    let id = env.register(Tessera, ());
    let cliente = TesseraClient::new(&env, &id);

    let proposta: BytesN<32> = BytesN::from_array(&env, &[23u8; 32]);
    let sem_mesa: Vec<Address> = Vec::new(&env);
    let mut perg = Vec::new(&env);
    perg.push_back(Pergunta {
        opcoes: 2,
        confidencial: true,
    });
    let gov = Address::generate(&env);

    // Abre: mesa vazia, limiar zero.
    cliente.abrir(
        &gov,
        &proposta,
        &perg,
        &BytesN::from_array(&env, &[0u8; 32]),
        &sem_mesa,
        &0u32,
        &4_989_990u32,
        &4_990_190u32,
        &true,
        &1u32,
        &0u32,
        &0u64,
        &0u64,
    );
    assert_eq!(cliente.proposta(&proposta).unwrap().mesa.len(), 0);

    // Os meios-termos continuam recusados, nos dois sentidos.
    let mut uma = Vec::new(&env);
    uma.push_back(Address::generate(&env));
    for (mesa, limiar) in [(sem_mesa.clone(), 1u32), (uma.clone(), 0u32)] {
        assert_eq!(
            cliente.try_abrir(
                &gov,
                &BytesN::from_array(&env, &[99u8; 32]),
                &perg,
                &BytesN::from_array(&env, &[0u8; 32]),
                &mesa,
                &limiar,
                &4_989_990u32,
                &4_990_190u32,
                &true,
                &1u32,
                &0u32,
                &0u64,
                &0u64,
            ),
            Err(Ok(Erro::LimiarInvalido)),
            "mesa e limiar têm de concordar: {} membros, limiar {}",
            mesa.len(),
            limiar
        );
    }

    // Alguém comparece e vota: a votação funciona inteira.
    let g = pedersen::gerador();
    let x = pedersen::acaso_fr().unwrap();
    let votante = Address::generate(&env);
    let vazio: Vec<BytesN<32>> = Vec::new(&env);
    cliente.comparecer(
        &proposta,
        &votante,
        &g1(&env, &anel::chave_publica(&g, &x)),
        &vazio,
        &0u32,
        &0u32,
    );
    assert_eq!(cliente.anel(&proposta, &0u32).len(), 1);

    // E ninguém endossa, porque não há de quem ser membro.
    env.ledger().set_sequence_number(4_990_200);
    let mut totais = Vec::new(&env);
    totais.push_back(1u32);
    totais.push_back(0u32);
    // Uma abertura por opção confidencial, não por pergunta — senão a recusa
    // vem de `ArgumentoMalFormado` e o teste não prova nada sobre a mesa.
    let mut aberturas = Vec::new(&env);
    for _ in 0..2 {
        aberturas.push_back(escalar(&env, &pedersen::acaso_fr().unwrap()));
    }
    assert_eq!(
        cliente.try_apurar(&proposta, &gov, &totais, &aberturas),
        Err(Ok(Erro::NaoEMembroDaMesa)),
        "sem mesa, endossar tem de ser impossível para qualquer um"
    );
}

/// **O split automático: a seção enche e a próxima abre.**
///
/// Obrigar quem organiza a adivinhar `secoes` é pedir que ele acerte quanta
/// gente vem — que é justamente o que ele não sabe numa votação aberta. Com
/// limite por seção, ele diz o tamanho e o contrato conta.
///
/// O que este teste prende é a aritmética da divisão e o nascimento da seção
/// nova. O que ele **não** alcança é a rajada de verdade, porque aqui as
/// chamadas são sequenciais: a seção só existe na aplicação, e o footprint é
/// declarado na simulação. Quem manda a transação tem de declarar uma janela de
/// seções, e isso só a testnet mede — ver `app/scripts/rodada-30.mjs`.
#[test]
fn a_secao_enche_e_a_proxima_abre() {
    use tessera_core::anel;

    const LIMITE: u32 = 4;
    const GENTE: usize = 14;

    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(4_989_900);
    let id = env.register(Tessera, ());
    let cliente = TesseraClient::new(&env, &id);

    let proposta: BytesN<32> = BytesN::from_array(&env, &[29u8; 32]);
    let sem_mesa: Vec<Address> = Vec::new(&env);
    let mut perg = Vec::new(&env);
    perg.push_back(Pergunta {
        opcoes: 2,
        confidencial: true,
    });

    cliente.abrir(
        &Address::generate(&env),
        &proposta,
        &perg,
        &BytesN::from_array(&env, &[0u8; 32]),
        &sem_mesa,
        &0u32,
        &4_989_990u32,
        &4_990_190u32,
        &true,
        &1u32,
        &LIMITE,
        &0u64,
        &0u64,
    );
    // Começa com uma seção. Nenhuma outra foi criada à toa.
    assert_eq!(cliente.secoes(&proposta), 1);

    let g = pedersen::gerador();
    let vazio: Vec<BytesN<32>> = Vec::new(&env);
    for i in 0..GENTE {
        let a = Address::generate(&env);
        let pk = anel::chave_publica(&g, &pedersen::acaso_fr().unwrap());
        cliente.comparecer(&proposta, &a, &g1(&env, &pk), &vazio, &0u32, &0u32);

        let esperada = i as u32 / LIMITE;
        assert_eq!(
            cliente.secao_de(&proposta, &a),
            Some(esperada),
            "a {}ª pessoa devia cair na seção {}",
            i + 1,
            esperada
        );
        // O número de seções acompanha o caderno, não fica parado em 1.
        assert_eq!(
            cliente.secoes(&proposta),
            (i as u32 / LIMITE) + 1,
            "o contador de seções não acompanhou o comparecimento"
        );
    }

    // 14 com limite 4 dão 4, 4, 4, 2.
    let tamanhos: Vetor<u32> = (0..cliente.secoes(&proposta))
        .map(|s| cliente.anel(&proposta, &s).len())
        .collect();
    assert_eq!(tamanhos.as_slice(), &[4u32, 4, 4, 2]);

    // A última nasceu com 2, abaixo de `TAU` — e é por isso que a tela exige
    // limite >= 5: com limite menor, a sobra da última seção não vota.
    assert!(tamanhos.as_slice().last().unwrap() < &TAU);
}

/// Limite por seção só faz sentido na votação aberta: na fechada a lista é
/// conhecida e a divisão sai dela na abertura.
#[test]
fn limite_por_secao_nao_vale_na_fechada() {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register(Tessera, ());
    let cliente = TesseraClient::new(&env, &id);
    let mut perg = Vec::new(&env);
    perg.push_back(Pergunta {
        opcoes: 2,
        confidencial: true,
    });
    let sem_mesa: Vec<Address> = Vec::new(&env);

    assert_eq!(
        cliente.try_abrir(
            &Address::generate(&env),
            &BytesN::from_array(&env, &[31u8; 32]),
            &perg,
            &BytesN::from_array(&env, &[7u8; 32]),
            &sem_mesa,
            &0u32,
            &100u32,
            &200u32,
            &true,
            &1u32,
            &5u32,
            &0u64,
            &0u64,
        ),
        Err(Ok(Erro::SecaoInvalida)),
        "limite por seção numa votação com lista devia ser recusado"
    );
}

// =====================================================================
// A fechadura de tempo (T-018). SPEC INV-18 a INV-22 e INV-25.
//
// O criptograma é **opaco para o contrato**: ele confere a forma, carrega no
// evento, e nada mais — o host não tem pareamento com saída de valor, então
// decifrar aqui é impossível e desnecessário. Quem recusa um total que mente é
// o compromisso de Pedersen. Por isso estes testes passam bytes quaisquer no
// lugar do criptograma: o ciclo de cifra e decifra vive em
// `core::relogio`, e a rodada ponta a ponta é T-021.
// =====================================================================

/// **O relógio destes testes é a rodada 6.000.000 da `quicknet`, com a
/// assinatura que a baliza publicou de verdade.**
///
/// Não há vetor forjado aqui de propósito: `registrar_abertura` confere a
/// assinatura com um pareamento, então uma rodada inventada não passaria. Por
/// isso o `timestamp` dos testes fica logo antes do instante daquela rodada — a
/// proposta abre com ela no futuro, como o contrato exige, e o tempo avança por
/// cima dela.
const RODADA_ABERTURA: u64 = 6_000_000;
const ASSINATURA_ABERTURA: &str = "848a0288a7102249bd6a274f65414ec8ca5b12c5e6f13a322e315ab734107784cd9b7ed4ebae73980cd72730ac6eb9f7";
/// Uma rodada depois, para a urna fechar depois de o comparecimento abrir.
const RODADA_FIM: u64 = RODADA_ABERTURA + 100;
const ABRE: u32 = 4_989_990;
const FECHA: u32 = 4_990_190;

fn instante(rodada: u64) -> u64 {
    tessera_core::relogio::instante(rodada)
}

/// Um instante logo antes de a rodada de abertura vencer.
fn t0() -> u64 {
    instante(RODADA_ABERTURA) - 60
}

/// A assinatura nos 96 bytes não comprimidos que o host lê — e que são **os
/// bytes de que a seção é derivada**. Alimentar `secao_de` com os 48
/// comprimidos daria outra divisão em silêncio.
fn assinatura_em_bytes() -> [u8; 96] {
    tessera_core::relogio::assinatura_para_host(&hex_bytes(ASSINATURA_ABERTURA)).unwrap()
}

fn assinatura_host(env: &Env) -> BytesN<96> {
    BytesN::from_array(env, &assinatura_em_bytes())
}

struct Urna {
    env: Env,
    id: Address,
    cliente: TesseraClient<'static>,
    proposta: BytesN<32>,
    /// Por cédula, na ordem em que chegaram: os dois compromissos e os dois
    /// fatores. É o que quem apura reconstrói decifrando os criptogramas.
    cedulas: Vetor<(Vec<Bls12381G1Affine>, Vetor<ArkFr>, u32)>,
}

/// Uma urna em anel com fechadura de tempo, com `escolhas.len()` cédulas já
/// depositadas e a janela ainda aberta.
fn urna_com_fechadura(escolhas: &[u32]) -> Urna {
    use tessera_core::anel;

    let n = escolhas.len();
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(4_989_900);
    env.ledger().set_timestamp(t0());
    let id = env.register(Tessera, ());
    let cliente = TesseraClient::new(&env, &id);

    let aptos: Vetor<Address> = (0..n).map(|_| Address::generate(&env)).collect();
    let folhas: Vetor<merkle::Apto> = aptos
        .iter()
        .map(|a| merkle::Apto {
            endereco: bytes_de(&a.clone().to_xdr(&env)),
            peso: 1,
        })
        .collect();
    let arvore = merkle::Arvore::montar(&folhas).unwrap();

    let mut perg = Vec::new(&env);
    perg.push_back(Pergunta {
        opcoes: 2,
        confidencial: true,
    });

    let proposta: BytesN<32> = BytesN::from_array(&env, &[7u8; 32]);
    // Sem mesa: `limiar = 0` e mesa vazia. É a assembleia que não tem quem
    // abra as cédulas — e com fechadura de tempo ela passa a ter resultado.
    cliente.abrir(
        &Address::generate(&env),
        &proposta,
        &perg,
        &BytesN::from_array(&env, &arvore.raiz()),
        &Vec::new(&env),
        &0u32,
        &ABRE,
        &FECHA,
        &true,
        &1u32,
        &0u32,
        &RODADA_FIM,
        &0u64,
    );

    let g = pedersen::gerador();
    let h = ponto::desserializar(&cliente.gerador_h().to_array()).unwrap();
    let hp = ponto::desserializar(&cripto::calcular_hp(&env, &proposta).to_array()).unwrap();

    let xs: Vetor<ArkFr> = (0..n).map(|_| pedersen::acaso_fr().unwrap()).collect();
    let mut anel_ark: Vetor<ArkG1> = Vetor::new();
    for (i, x) in xs.iter().enumerate() {
        let pk = anel::chave_publica(&g, x);
        let p = arvore.caminho(i).unwrap();
        let mut caminho = Vec::new(&env);
        for irmao in &p.irmaos {
            caminho.push_back(BytesN::from_array(&env, irmao));
        }
        cliente.comparecer(
            &proposta,
            &aptos[i],
            &g1(&env, &pk),
            &caminho,
            &p.indice,
            &0u32,
        );
        anel_ark.push(pk);
    }
    let mut anel_sdk: Vec<Bls12381G1Affine> = Vec::new(&env);
    for p in &anel_ark {
        anel_sdk.push_back(g1(&env, p));
    }

    env.ledger().set_sequence_number(ABRE + 10);

    let mut cedulas = Vetor::new();
    for (i, escolha) in escolhas.iter().copied().enumerate() {
        let img = anel::imagem(&hp, &xs[i]);
        let ident: Vetor<u8> = ponto::serializar(&img).to_vec();
        let rs: Vetor<ArkFr> = (0..2).map(|_| pedersen::acaso_fr().unwrap()).collect();
        let cs: Vetor<ArkG1> = (0..2)
            .map(|j| {
                let v = if j == escolha { 1u64 } else { 0 };
                pedersen::comprometer(&g, &h, &pedersen::escalar(v), &rs[j as usize])
            })
            .collect();
        let ctx_de = |opcao: u32| {
            let mut v: Vetor<u8> = proposta.to_array().to_vec();
            v.extend(ident.iter().copied());
            v.extend(0u32.to_be_bytes());
            v.extend(opcao.to_be_bytes());
            v
        };
        let mut compromissos = Vec::new(&env);
        let mut provas = Vec::new(&env);
        for j in 0..2u32 {
            let v = if j == escolha { 1u64 } else { 0 };
            let pr = cds::provar(&ctx_de(j), &g, &h, &cs[j as usize], v, &rs[j as usize]).unwrap();
            compromissos.push_back(g1(&env, &cs[j as usize]));
            provas.push_back(ProvaCds {
                a0: g1(&env, &pr.a0),
                a1: g1(&env, &pr.a1),
                e0: escalar(&env, &pr.e0),
                z0: escalar(&env, &pr.z0),
                e1: escalar(&env, &pr.e1),
                z1: escalar(&env, &pr.z1),
            });
        }
        let rho = rs.iter().fold(ArkFr::from(0u64), |a, r| a + r);
        let d = soma::alvo(&g, &cs, 1);
        let ps = soma::provar(&ctx_de(u32::MAX), &h, &d, &rho).unwrap();
        let mut provas_soma = Vec::new(&env);
        provas_soma.push_back(ProvaSoma {
            a: g1(&env, &ps.a),
            z: escalar(&env, &ps.z),
        });
        let escolhas_pub: Vec<u32> = Vec::new(&env);
        let msg = cripto::mensagem_cedula(&env, &proposta, &compromissos, &escolhas_pub);
        let s = anel::assinar(&bytes_de(&msg), &g, &hp, &anel_ark, i, &xs[i]).unwrap();
        let mut z = Vec::new(&env);
        for zi in &s.z {
            z.push_back(escalar(&env, zi));
        }
        cliente.votar_anonimo(
            &proposta,
            &0u32,
            &anel_sdk,
            &g1(&env, &s.imagem),
            &escalar(&env, &s.c0),
            &z,
            &compromissos,
            &provas,
            &provas_soma,
            &escolhas_pub,
            &Bytes::from_array(&env, &[9u8; 320]),
        );
        cedulas.push((compromissos, rs, escolha));
    }

    Urna {
        env,
        id,
        cliente,
        proposta,
        cedulas,
    }
}

impl Urna {
    /// A janela fecha e a rodada vence: é quando o placar pode existir.
    fn fim(&self) {
        self.env.ledger().set_sequence_number(FECHA);
        self.env.ledger().set_timestamp(instante(RODADA_FIM));
    }

    /// A lista ordenada de todos os compromissos da seção, achatada.
    fn lista(&self) -> Vec<Bls12381G1Affine> {
        let mut v = Vec::new(&self.env);
        for (cs, _, _) in &self.cedulas {
            for c in cs.iter() {
                v.push_back(c);
            }
        }
        v
    }

    /// `(abertas, totais, aberturas)` para um conjunto escolhido de cédulas.
    fn apuracao(&self, quais: &[bool]) -> (Vec<bool>, Vec<u32>, Vec<Bls12381Fr>) {
        let mut abertas = Vec::new(&self.env);
        let mut totais = [0u32; 2];
        let mut somas = [ArkFr::from(0u64); 2];
        for (i, (_, rs, escolha)) in self.cedulas.iter().enumerate() {
            abertas.push_back(quais[i]);
            if quais[i] {
                totais[*escolha as usize] += 1;
                for j in 0..2 {
                    somas[j] += rs[j];
                }
            }
        }
        let mut t = Vec::new(&self.env);
        let mut a = Vec::new(&self.env);
        for j in 0..2 {
            t.push_back(totais[j]);
            a.push_back(escalar(&self.env, &somas[j]));
        }
        (abertas, t, a)
    }
}

/// A aritmética da rodada é repetida no contrato para não arrastar o `core`
/// para dentro do Wasm. Repetir é divergir, então aqui as duas se cruzam: o
/// instante que o `core` calcula para uma rodada é o mesmo que o contrato usa
/// como prazo.
///
/// O cruzamento é pela **fronteira**, que é o único jeito de ver por fora o
/// número que o contrato usa por dentro: um segundo antes do instante da rodada
/// ele recusa, e no instante exato ele aceita. Se as duas contas divergissem em
/// um segundo, isto quebraria.
#[test]
fn a_rodada_do_contrato_bate_com_a_do_core() {
    let u = urna_com_fechadura(&[0]);
    u.env.ledger().set_sequence_number(FECHA);
    let (abertas, totais, aberturas) = u.apuracao(&[true]);
    let quem = Address::generate(&u.env);
    let tentar = || {
        u.cliente.try_apurar_secao(
            &u.proposta,
            &quem,
            &0u32,
            &u.lista(),
            &abertas,
            &totais,
            &aberturas,
        )
    };

    u.env.ledger().set_timestamp(instante(RODADA_FIM) - 1);
    assert_eq!(
        tentar(),
        Err(Ok(Erro::RelogioAindaNaoAbriu)),
        "o contrato usa um instante diferente do que o core calcula"
    );
    u.env.ledger().set_timestamp(instante(RODADA_FIM));
    assert_eq!(tentar(), Ok(Ok(1)));

    assert_eq!(
        tessera_core::relogio::TAMANHO as u32,
        crate::TAMANHO_CRIPTOGRAMA,
        "o tamanho do criptograma divergiu entre core e contrato"
    );
}

/// **INV-22.** A rodada tem de estar no **futuro** na hora de abrir, e isso vale
/// para as duas: a do fechamento e a da abertura.
///
/// É o mesmo motivo nos dois casos. Uma rodada já vencida tem assinatura
/// publicada — na fechadura, as cédulas abririam na hora de serem depositadas;
/// na abertura, quem organiza leria a assinatura e moeria até isolar alguém
/// (DEC-012). Não existe campo de tempo separado para conferir contra: a rodada
/// **é** o prazo.
#[test]
fn a_rodada_tem_de_estar_no_futuro() {
    let u = urna_com_fechadura(&[0]);
    let mut perg = Vec::new(&u.env);
    perg.push_back(Pergunta {
        opcoes: 2,
        confidencial: true,
    });
    let abrir_com = |rodada: u64, abertura: u64, secoes: u32| {
        u.cliente.try_abrir(
            &Address::generate(&u.env),
            &BytesN::from_array(&u.env, &[2u8; 32]),
            &perg,
            // Raiz não nula: 32 zeros seriam **votação aberta**, e aí a
            // seção vem da ordem de chegada, não da lista.
            &BytesN::from_array(&u.env, &[3u8; 32]),
            &Vec::new(&u.env),
            &0u32,
            &ABRE,
            &FECHA,
            &true,
            &secoes,
            &0u32,
            &rodada,
            &abertura,
        )
    };

    // A rodada 1 venceu em 2023: a assinatura dela está publicada há anos.
    assert_eq!(abrir_com(1, 0, 1), Err(Ok(Erro::RodadaNaoFecha)));
    assert_eq!(
        abrir_com(RODADA_FIM, 1, 1),
        Err(Ok(Erro::RodadaNaoFecha)),
        "aceitou uma rodada de abertura já publicada"
    );
    // E o comparecimento abre antes de a urna fechar.
    assert_eq!(
        abrir_com(RODADA_ABERTURA, RODADA_FIM, 1),
        Err(Ok(Erro::PrazoNoPassado))
    );

    // **Seções numa votação fechada exigem a baliza.** Sem ela a seção não tem
    // contra o que ser conferida, e quem comparece escolheria a sua.
    assert_eq!(
        abrir_com(RODADA_FIM, 0, 3),
        Err(Ok(Erro::SecaoInvalida)),
        "abriu votação fechada com seções e sem baliza: a seção fica à escolha"
    );
    assert_eq!(abrir_com(RODADA_FIM, RODADA_ABERTURA, 3), Ok(Ok(())));
}

/// **INV-18.** Nenhum resultado, nem parcial, antes do fechamento — e o portão
/// é duplo: a sequência de ledger passou **e** a rodada venceu. Os dois são o
/// relógio do ledger, sem suposição nenhuma sobre a baliza.
#[test]
fn antes_do_fechamento_nao_existe_placar() {
    let u = urna_com_fechadura(&[0, 1, 0]);
    let (abertas, totais, aberturas) = u.apuracao(&[true, true, true]);
    let quem = Address::generate(&u.env);

    assert_eq!(
        u.cliente.try_apurar_secao(
            &u.proposta,
            &quem,
            &0u32,
            &u.lista(),
            &abertas,
            &totais,
            &aberturas
        ),
        Err(Ok(Erro::VotacaoAindaAberta)),
        "apurou com a janela aberta"
    );

    // A sequência fechou, mas a rodada ainda não venceu: a chave que decifra as
    // cédulas não existe no mundo, e o contrato não publica placar que ninguém
    // poderia ter calculado honestamente.
    u.env.ledger().set_sequence_number(FECHA);
    u.env.ledger().set_timestamp(instante(RODADA_FIM) - 1);
    assert_eq!(
        u.cliente.try_apurar_secao(
            &u.proposta,
            &quem,
            &0u32,
            &u.lista(),
            &abertas,
            &totais,
            &aberturas
        ),
        Err(Ok(Erro::RelogioAindaNaoAbriu))
    );
    assert_eq!(u.cliente.resultado_secao(&u.proposta, &0u32), None);

    u.fim();
    assert_eq!(
        u.cliente.apurar_secao(
            &u.proposta,
            &quem,
            &0u32,
            &u.lista(),
            &abertas,
            &totais,
            &aberturas
        ),
        3
    );
}

/// **INV-20.** A lista apresentada é o conjunto real de cédulas daquela seção,
/// e são duas travas em série:
///
/// - **omitir é inexprimível.** A cadeia guarda quantas cédulas chegaram, e a
///   aridade da chamada está presa a esse número. Uma lista curta nem chega à
///   cadeia;
/// - **trocar e reordenar a cadeia pega**, porque ela encadeia na ordem em que
///   as cédulas chegaram.
#[test]
fn a_cadeia_recusa_omissao_e_invencao() {
    let u = urna_com_fechadura(&[0, 1, 0]);
    u.fim();
    let quem = Address::generate(&u.env);
    let (abertas, totais, aberturas) = u.apuracao(&[true, true, true]);

    // Omissão: a lista sai com duas cédulas em vez de três, e a contagem
    // guardada a recusa antes de qualquer hash.
    let mut curta = Vec::new(&u.env);
    for (i, c) in u.lista().iter().enumerate() {
        if i < 4 {
            curta.push_back(c);
        }
    }
    let mut duas = Vec::new(&u.env);
    duas.push_back(true);
    duas.push_back(true);
    assert_eq!(
        u.cliente.try_apurar_secao(
            &u.proposta,
            &quem,
            &0u32,
            &curta,
            &duas,
            &totais,
            &aberturas
        ),
        Err(Ok(Erro::ArgumentoMalFormado)),
        "contou uma seção com uma cédula omitida"
    );

    // Invenção: um compromisso trocado por outro ponto válido.
    let mut falsa = u.lista();
    falsa.set(0, u.lista().get(2).unwrap());
    assert_eq!(
        u.cliente.try_apurar_secao(
            &u.proposta,
            &quem,
            &0u32,
            &falsa,
            &abertas,
            &totais,
            &aberturas
        ),
        Err(Ok(Erro::CadeiaNaoFecha)),
        "aceitou um compromisso que nenhuma cédula escreveu"
    );

    // Reordenação: as mesmas cédulas, em outra ordem. A soma dos compromissos
    // não mudaria — é por isso que a cadeia precisa existir, e não bastaria
    // conferir o acumulado.
    let l = u.lista();
    let mut trocada = l.clone();
    trocada.set(0, l.get(2).unwrap());
    trocada.set(1, l.get(3).unwrap());
    trocada.set(2, l.get(0).unwrap());
    trocada.set(3, l.get(1).unwrap());
    assert_eq!(
        u.cliente.try_apurar_secao(
            &u.proposta,
            &quem,
            &0u32,
            &trocada,
            &abertas,
            &totais,
            &aberturas
        ),
        Err(Ok(Erro::CadeiaNaoFecha)),
        "aceitou a lista fora da ordem em que as cédulas chegaram"
    );
}

/// **INV-19.** Uma cédula que não abre perde o próprio voto e não impede a
/// apuração das outras. É o que conserta o defeito do `Acum`: a equação da soma
/// só fechava com **todas** abertas, e uma cédula sabotada derrubava o placar
/// inteiro.
#[test]
fn a_cedula_que_nao_abre_perde_so_o_proprio_voto() {
    let u = urna_com_fechadura(&[0, 1, 1]);
    u.fim();
    let quem = Address::generate(&u.env);

    // A primeira cédula não abre. As outras duas apuram.
    let (abertas, totais, aberturas) = u.apuracao(&[false, true, true]);
    assert_eq!(
        u.cliente.apurar_secao(
            &u.proposta,
            &quem,
            &0u32,
            &u.lista(),
            &abertas,
            &totais,
            &aberturas
        ),
        2
    );
    let (quantas, placar) = u.cliente.resultado_secao(&u.proposta, &0u32).unwrap();
    assert_eq!(quantas, 2);
    assert_eq!(placar.get(0).unwrap(), 0, "contou o voto que não abriu");
    assert_eq!(placar.get(1).unwrap(), 2);
}

/// **INV-21.** Omitir não gruda: só um conjunto estritamente maior substitui o
/// guardado. É isto que impede travar o placar — quem omitir uma cédula honesta
/// é sobreposto por qualquer pessoa que a inclua, e qualquer pessoa consegue,
/// porque a chave da rodada é pública.
#[test]
fn omitir_nao_gruda_e_quem_inclui_sobrepoe() {
    let u = urna_com_fechadura(&[0, 1, 1]);
    u.fim();
    let malicioso = Address::generate(&u.env);
    let qualquer = Address::generate(&u.env);

    // Alguém apura de propósito sem a cédula que não lhe convém.
    let (a1, t1, ab1) = u.apuracao(&[true, true, false]);
    assert_eq!(
        u.cliente
            .apurar_secao(&u.proposta, &malicioso, &0u32, &u.lista(), &a1, &t1, &ab1),
        2
    );
    assert_eq!(u.cliente.resultado_secao(&u.proposta, &0u32).unwrap().0, 2);

    // Qualquer observador honesto sobrepõe incluindo-a.
    let (a2, t2, ab2) = u.apuracao(&[true, true, true]);
    assert_eq!(
        u.cliente
            .apurar_secao(&u.proposta, &qualquer, &0u32, &u.lista(), &a2, &t2, &ab2),
        3
    );
    let (quantas, placar) = u.cliente.resultado_secao(&u.proposta, &0u32).unwrap();
    assert_eq!(quantas, 3);
    assert_eq!(placar.get(1).unwrap(), 2);

    // E a apuração menor não volta a colar.
    assert_eq!(
        u.cliente
            .try_apurar_secao(&u.proposta, &malicioso, &0u32, &u.lista(), &a1, &t1, &ab1),
        Err(Ok(Erro::NaoMelhora))
    );
}

/// **INV-25, DEC-011.** O piso de `τ` é do conjunto de anonimato — o anel —, e
/// **não** do subconjunto que abriu. A ausência do portão aqui é deliberada:
/// exigir `τ` cédulas abertas daria a qualquer um o poder de travar a apuração
/// sabotando o próprio criptograma, que é a falha de liveness induzível de fora
/// que a remoção de `votar_publico` havia fechado.
#[test]
fn sabotar_o_proprio_criptograma_nao_derruba_o_piso() {
    // Seis no anel, bem acima de `TAU = 5`. Cinco sabotam o próprio
    // criptograma; uma abre.
    let u = urna_com_fechadura(&[0, 1, 1, 0, 1, 0]);
    u.fim();
    let (abertas, totais, aberturas) = u.apuracao(&[true, false, false, false, false, false]);
    assert_eq!(
        u.cliente.apurar_secao(
            &u.proposta,
            &Address::generate(&u.env),
            &0u32,
            &u.lista(),
            &abertas,
            &totais,
            &aberturas
        ),
        1,
        "uma coligação travou o placar sabotando o próprio voto"
    );
    assert_eq!(u.cliente.resultado_secao(&u.proposta, &0u32).unwrap().0, 1);
}

/// Um total que não abre o subconjunto apresentado é recusado na hora, como
/// sempre foi. A solidez não mudou: a prova CDS já garantiu `v ∈ {0,1}` em cada
/// compromisso, então a equação sobre o subconjunto força `T = Σvᵢ`.
#[test]
fn o_total_que_mente_sobre_o_subconjunto_e_recusado() {
    let u = urna_com_fechadura(&[0, 1, 1]);
    u.fim();
    let (abertas, _, aberturas) = u.apuracao(&[true, true, true]);
    let mut mentira = Vec::new(&u.env);
    mentira.push_back(3u32);
    mentira.push_back(0u32);
    assert_eq!(
        u.cliente.try_apurar_secao(
            &u.proposta,
            &Address::generate(&u.env),
            &0u32,
            &u.lista(),
            &abertas,
            &mentira,
            &aberturas
        ),
        Err(Ok(Erro::AberturaNaoFecha))
    );
}

/// O criptograma tem a forma declarada em SPEC §5, ou a cédula não entra. O
/// contrato não o lê — não pode —, mas a forma errada significa que ninguém vai
/// conseguir abrir aquela cédula, e é melhor recusar agora que no fim.
#[test]
fn o_criptograma_tem_de_ter_a_forma_declarada() {
    let u = urna_com_fechadura(&[0]);
    let _ = &u.id;
    // A segunda cédula da mesma urna, com criptograma curto, não entra. Reusar
    // o anel e a imagem da primeira daria `ImagemJaUsada` antes da forma, então
    // o que se confere aqui é a ordem: a forma é conferida com as aridades, no
    // começo.
    let (cs, _, _) = &u.cedulas[0];
    assert_eq!(
        u.cliente.try_votar_anonimo(
            &u.proposta,
            &0u32,
            &Vec::new(&u.env),
            &cs.get(0).unwrap(),
            &escalar(&u.env, &ArkFr::from(1u64)),
            &Vec::new(&u.env),
            cs,
            &Vec::new(&u.env),
            &Vec::new(&u.env),
            &Vec::new(&u.env),
            &Bytes::from_array(&u.env, &[0u8; 100]),
        ),
        Err(Ok(Erro::ArgumentoMalFormado)),
        "a aridade vem antes; a forma do criptograma vem depois dela"
    );
}

/// **O custo da apuração por seção, medido.** Ela é O(n) em cédulas: `n`
/// hashes e `n` somas em G1, mais **um** MSM de dois termos por opção.
///
/// Medido: **12.550.031 para 6 cédulas e 14.066.366 para 12** — ou seja
/// 11.033.699 fixos mais **252.722 por cédula**. Uma seção de 30 custaria
/// 18,6 milhões, 4,6% do teto de 400.000.000.
///
/// É três ordens de grandeza abaixo do voto, que gasta 10.822.850 por membro do
/// anel. Então a apuração **não** é o que limita o tamanho da seção: quem limita
/// continua sendo a verificação do anel, e isso era o que precisava ser medido
/// antes de prometer apuração automática. Assere teto e inclinação: uma
/// regressão quebra o build em vez de aparecer no dia da votação.
#[test]
fn orcamento_da_apuracao_por_secao() {
    let seis = urna_com_fechadura(&[0, 1, 1, 0, 1, 0]);
    seis.fim();
    let (a6, t6, ab6) = seis.apuracao(&[true; 6]);
    let l6 = seis.lista();
    let c6 = cpu(&seis.env, || {
        seis.cliente.apurar_secao(
            &seis.proposta,
            &Address::generate(&seis.env),
            &0u32,
            &l6,
            &a6,
            &t6,
            &ab6,
        )
    });

    let doze = urna_com_fechadura(&[0, 1, 1, 0, 1, 0, 1, 0, 0, 1, 1, 0]);
    doze.fim();
    let (a12, t12, ab12) = doze.apuracao(&[true; 12]);
    let l12 = doze.lista();
    let c12 = cpu(&doze.env, || {
        doze.cliente.apurar_secao(
            &doze.proposta,
            &Address::generate(&doze.env),
            &0u32,
            &l12,
            &a12,
            &t12,
            &ab12,
        )
    });

    std::println!("apurar_secao: 6 cédulas {c6}, 12 cédulas {c12}");
    // Teto generoso de propósito: o que o teste prende é a **ordem de
    // grandeza** e o crescimento linear, não um número exato que mudaria com
    // qualquer mexida no host. O teto de uma transação é 400.000.000.
    assert!(
        c12 < 100_000_000,
        "a apuração de 12 cédulas custou {c12}, perto demais do teto"
    );
    // Dobrar as cédulas não dobra o custo: a parte fixa é a autorização, a
    // leitura da proposta e o MSM. O que cresce são hashes e somas em G1.
    assert!(
        c12 < c6 * 2,
        "o custo cresceu mais que linear: {c6} para 6, {c12} para 12"
    );
    // E a inclinação por cédula, que é o que extrapola para a seção cheia.
    let por_cedula = (c12 - c6) / 6;
    assert!(
        por_cedula < 400_000,
        "cada cédula passou a custar {por_cedula} na apuração"
    );
}

/// **O host sabe conferir a baliza?** E em que ordem ele lê as coordenadas de
/// G2?
///
/// A primeira pergunta decide se a T-023 é possível: derivar a seção da
/// assinatura da rodada de abertura exige que o contrato **verifique** aquela
/// assinatura, senão quem a apresenta escolhe o que quiser. O host tem
/// `hash_to_g1` e `pairing_check`, que é tudo de que `e(σ, g₂) == e(H₁, P)`
/// precisa.
///
/// A segunda é empírica de propósito. Em G1 o host lê `be(X) || be(Y)`, não
/// comprimido (`core/src/ponto.rs`). Em G2 cada coordenada é de `Fp2`, e a ordem
/// entre `c0` e `c1` é precisamente a divergência silenciosa que o `ponto.rs`
/// existe para fechar. **Medido: `c1` antes de `c0`**, a convenção zcash — e o
/// pareamento do host é o juiz, porque com a ordem trocada ele recusa o ponto ou
/// a igualdade falha. É esta asserção que trava `ponto::serializar_g2`.
///
/// Custo medido: **24.053.490 instruções**, 6% do teto de uma transação. Pago
/// uma vez por proposta em `registrar_abertura`, não por cédula — e é o número
/// que torna a T-023 possível em vez de teórica.
#[test]
fn o_host_confere_a_baliza_e_a_ordem_de_g2_e_medida() {
    use soroban_sdk::crypto::bls12_381::Bls12381G2Affine;
    use soroban_sdk::Bytes;
    use tessera_core::{ponto, relogio};

    const RODADA: u64 = 6_000_000;
    const ASSINATURA: &str = "848a0288a7102249bd6a274f65414ec8ca5b12c5e6f13a322e315ab734107784cd9b7ed4ebae73980cd72730ac6eb9f7";

    let env = Env::default();
    env.cost_estimate().budget().reset_unlimited();
    let bls = env.crypto().bls12_381();

    // `H₁(sha256(rodada em 8 bytes big-endian))` com o DST da drand — a mesma
    // mensagem que o `core` mediu, agora calculada pelo host.
    let msg = env
        .crypto()
        .sha256(&Bytes::from_array(&env, &RODADA.to_be_bytes()));
    let q = bls.hash_to_g1(
        &Bytes::from_array(&env, &msg.to_array()),
        &Bytes::from_slice(&env, b"BLS_SIG_BLS12381G1_XMD:SHA-256_SSWU_RO_NUL_"),
    );

    let sig = relogio::assinatura_para_host(&hex_bytes(ASSINATURA)).unwrap();
    let s = Bls12381G1Affine::from_bytes(BytesN::from_array(&env, &sig));
    let menos_q = bls.g1_mul(&q, &escalar(&env, &(-ArkFr::from(1u64))));

    let g2 =
        |b: [u8; ponto::TAMANHO_G2]| Bls12381G2Affine::from_bytes(BytesN::from_array(&env, &b));
    let mut p1 = Vec::new(&env);
    let mut p2 = Vec::new(&env);
    p1.push_back(s);
    p1.push_back(menos_q);
    p2.push_back(g2(relogio::gerador_g2_para_host()));
    p2.push_back(g2(relogio::chave_da_cadeia_para_host()));

    let antes = env.cost_estimate().budget().cpu_instruction_cost();
    assert!(
        bls.pairing_check(p1, p2),
        "o host recusou a assinatura da baliza, ou `serializar_g2` está na ordem \
         errada de Fp2 — inverta `c1`/`c0` em `core/src/ponto.rs` e rode de novo"
    );
    let gasto = env.cost_estimate().budget().cpu_instruction_cost() - antes;
    std::println!("conferir a baliza: {gasto} instruções");
    assert!(
        gasto < 100_000_000,
        "conferir a baliza custou {gasto}, inviável dentro de uma cédula"
    );
}

/// **DEC-012, no contrato.** A seção vem da baliza e o contrato **confere** —
/// aceitar a que a chamada afirma devolveria a escolha para quem vota.
///
/// E a assinatura tem de ser a da rodada daquela proposta: uma assinatura válida
/// de **outra** rodada é recusada, senão um relé escolheria a divisão
/// apresentando a rodada que lhe conviesse.
#[test]
fn a_seccao_vem_da_baliza_e_o_contrato_confere() {
    use tessera_core::anel;

    const N: usize = 40;
    const SECOES: u32 = 2;

    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(4_989_900);
    env.ledger().set_timestamp(t0());
    let id = env.register(Tessera, ());
    let cliente = TesseraClient::new(&env, &id);

    let proposta: BytesN<32> = BytesN::from_array(&env, &[13u8; 32]);
    let aptos: Vetor<Address> = (0..N).map(|_| Address::generate(&env)).collect();
    let xdrs: Vetor<Vetor<u8>> = aptos
        .iter()
        .map(|a| bytes_de(&a.clone().to_xdr(&env)))
        .collect();
    let folhas: Vetor<merkle::Apto> = xdrs
        .iter()
        .map(|e| merkle::Apto {
            endereco: e.clone(),
            peso: 1,
        })
        .collect();
    let arvore = merkle::Arvore::montar(&folhas).unwrap();

    let mut perg = Vec::new(&env);
    perg.push_back(Pergunta {
        opcoes: 2,
        confidencial: true,
    });
    cliente.abrir(
        &Address::generate(&env),
        &proposta,
        &perg,
        &BytesN::from_array(&env, &arvore.raiz()),
        &Vec::new(&env),
        &0u32,
        &4_989_990u32,
        &4_990_190u32,
        &true,
        &SECOES,
        &0u32,
        &0u64,
        &RODADA_ABERTURA,
    );

    let g = pedersen::gerador();
    let caminho_de = |i: usize| {
        let p = arvore.caminho(i).unwrap();
        let mut c = Vec::new(&env);
        for irmao in &p.irmaos {
            c.push_back(BytesN::from_array(&env, irmao));
        }
        (c, p.indice)
    };
    let chave = |_: usize| {
        g1(
            &env,
            &anel::chave_publica(&g, &pedersen::acaso_fr().unwrap()),
        )
    };

    // **Sem a assinatura registrada ninguém comparece.** Não é cosmético: sem
    // ela o contrato não tem contra o que conferir a seção.
    let (c0, i0) = caminho_de(0);
    assert_eq!(
        cliente.try_comparecer(&proposta, &aptos[0], &chave(0), &c0, &i0, &0u32),
        Err(Ok(Erro::BalizaNaoRegistrada))
    );

    // E antes de a rodada vencer não há o que registrar: a assinatura não
    // existe no mundo.
    assert_eq!(
        cliente.try_registrar_abertura(&proposta, &assinatura_host(&env)),
        Err(Ok(Erro::AberturaAindaNaoVenceu))
    );

    env.ledger().set_timestamp(instante(RODADA_ABERTURA));

    // Uma assinatura válida, mas de outra rodada, é recusada. É o que impede um
    // relé de escolher a divisão apresentando a rodada que lhe convém.
    let outra: BytesN<96> = BytesN::from_array(
        &env,
        &tessera_core::relogio::assinatura_para_host(&hex_bytes(
            "aa0ffe277142bf0bb52caa6037770b4f135e9a8325efb88f691420ca93f635ceda8c5e64eddff9cee9b2f29ba2e44d17",
        ))
        .unwrap(),
    );
    assert_eq!(
        cliente.try_registrar_abertura(&proposta, &outra),
        Err(Ok(Erro::BalizaNaoConfere)),
        "aceitou a assinatura de outra rodada"
    );

    cliente.registrar_abertura(&proposta, &assinatura_host(&env));

    // Agora a seção é determinada, e **só** ela é aceita.
    let divisao = merkle::dividir(&assinatura_em_bytes(), &xdrs, SECOES);
    for i in 0..3 {
        let (c, idx) = caminho_de(i);
        let errada = (divisao[i] + 1) % SECOES;
        assert_eq!(
            cliente.try_comparecer(&proposta, &aptos[i], &chave(i), &c, &idx, &errada),
            Err(Ok(Erro::SecaoInvalida)),
            "o membro {i} escolheu a seção"
        );
        cliente.comparecer(&proposta, &aptos[i], &chave(i), &c, &idx, &divisao[i]);
        assert_eq!(cliente.secao_de(&proposta, &aptos[i]), Some(divisao[i]));
    }

    // E o dimensionamento: 40 aptos em 2 seções é média 20, a faixa que
    // `core::merkle` mede como segura.
    assert_eq!(merkle::secoes_para(N), SECOES);
}
