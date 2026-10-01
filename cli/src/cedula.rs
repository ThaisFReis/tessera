//! A montagem de uma cédula, e as shares que vão para a mesa.
//!
//! Este é o arquivo onde o voto existe em claro, e é o único. Depois daqui
//! `v_j` não transita, não é escrito e não é guardado: o que sai são
//! compromissos e provas.

use crate::chave;
use serde_json::{json, Value};
use tessera_core::ark::{Fr, G1Affine};
use tessera_core::{cds, pedersen, ponto, shamir, soma};

/// Uma pergunta da cédula, como o cliente a enxerga.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pergunta {
    pub opcoes: usize,
    pub confidencial: bool,
}

pub struct Cedula {
    /// Achatados sobre as perguntas **sigilosas**, em ordem.
    pub compromissos: Vec<G1Affine>,
    pub provas: Vec<cds::Prova>,
    /// **Uma por pergunta sigilosa.** Uma prova só, sobre a cédula inteira,
    /// afirmaria `Σ(tudo) = peso` — e com peso 1 isso obrigaria quem responde
    /// a pergunta 1 a abster-se das outras.
    pub somas: Vec<soma::Prova>,
    /// Achatadas sobre as perguntas **públicas**, em ordem.
    pub publicas: Vec<u32>,
    /// Os `r_j`. Vão para o recibo e para as shares, e **para mais lugar
    /// nenhum**.
    pub acasos: Vec<Fr>,
}

/// `proposta ‖ addr_xdr ‖ pergunta ‖ opção` — os mesmos bytes que
/// `cripto::contexto` monta dentro do contrato.
///
/// Prende cada prova a esta proposta, a esta pessoa, a esta pergunta e a esta
/// opção. Sem o endereço, copiar o `C` e a prova de outra pessoa é um voto
/// válido. **Sem a pergunta**, a disjuntiva da pergunta 1 vale na pergunta 2,
/// e o eleitor marca a segunda sem provar nada sobre ela.
pub fn contexto(proposta: &[u8; 32], addr_xdr: &[u8], pergunta: u32, opcao: u32) -> Vec<u8> {
    let mut v = proposta.to_vec();
    v.extend_from_slice(addr_xdr);
    v.extend_from_slice(&pergunta.to_be_bytes());
    v.extend_from_slice(&opcao.to_be_bytes());
    v
}

/// A opção usada no contexto da prova de soma. `u32::MAX` porque ela não
/// pertence a opção nenhuma, e tem de ser um valor que nenhuma opção real
/// alcança.
pub const OPCAO_DA_SOMA: u32 = u32::MAX;

/// Monta a cédula inteira: compromissos e provas nas perguntas sigilosas,
/// resposta em claro nas públicas.
///
/// `escolhas[q]` é a opção marcada na pergunta `q`.
pub fn montar(
    h: &G1Affine,
    proposta: &[u8; 32],
    addr_xdr: &[u8],
    perguntas: &[Pergunta],
    escolhas: &[usize],
) -> Result<Cedula, String> {
    if escolhas.len() != perguntas.len() {
        return Err(format!(
            "a cédula tem {} perguntas e vieram {} escolhas",
            perguntas.len(),
            escolhas.len()
        ));
    }
    let g = pedersen::gerador();
    let mut compromissos = Vec::new();
    let mut provas = Vec::new();
    let mut somas = Vec::new();
    let mut publicas = Vec::new();
    let mut acasos = Vec::new();

    for (q, pg) in perguntas.iter().enumerate() {
        let escolha = escolhas[q];
        if escolha >= pg.opcoes {
            return Err(format!(
                "a pergunta {} tem {} opções e a escolha foi {}",
                q + 1,
                pg.opcoes,
                escolha
            ));
        }
        if !pg.confidencial {
            for j in 0..pg.opcoes {
                publicas.push(if j == escolha { 1u32 } else { 0 });
            }
            continue;
        }

        let mut rs = Vec::with_capacity(pg.opcoes);
        for _ in 0..pg.opcoes {
            rs.push(
                pedersen::acaso_fr().map_err(|_| "sem aleatoriedade do sistema".to_string())?,
            );
        }
        let cs: Vec<G1Affine> = (0..pg.opcoes)
            .map(|j| {
                let v = if j == escolha { 1u64 } else { 0 };
                pedersen::comprometer(&g, h, &pedersen::escalar(v), &rs[j])
            })
            .collect();

        for j in 0..pg.opcoes {
            let v = if j == escolha { 1u64 } else { 0 };
            provas.push(
                cds::provar(
                    &contexto(proposta, addr_xdr, q as u32, j as u32),
                    &g,
                    h,
                    &cs[j],
                    v,
                    &rs[j],
                )
                .map_err(|e| {
                    format!("não consegui provar a opção {} da pergunta {}: {:?}", j, q + 1, e)
                })?,
            );
        }

        // A prova de soma desta pergunta, sobre a fatia dela.
        let rho = rs.iter().fold(pedersen::escalar(0), |a, r| a + r);
        let d = soma::alvo(&g, &cs, 1);
        somas.push(
            soma::provar(
                &contexto(proposta, addr_xdr, q as u32, OPCAO_DA_SOMA),
                h,
                &d,
                &rho,
            )
            .map_err(|e| format!("não consegui provar a soma da pergunta {}: {:?}", q + 1, e))?,
        );

        compromissos.extend(cs);
        acasos.extend(rs);
    }

    Ok(Cedula { compromissos, provas, somas, publicas, acasos })
}

// ---------- serialização para a `stellar contract invoke` ----------

pub fn hex_ponto(p: &G1Affine) -> String {
    ponto::para_hex(p)
}

/// **`Bls12381Fr` vai em DECIMAL, nunca hex.**
///
/// A armadilha já mordeu uma vez neste projeto: hex falha com
/// `invalid digit found in string`, **mas um hex que por acaso só tenha
/// dígitos é aceito em silêncio como o decimal errado** — `...0065` entrou
/// como 65 em vez de 101, e dois compromissos "deram certo" com valores
/// errados antes de alguém notar.
pub fn dec_escalar(f: &Fr) -> String {
    pedersen::fr_para_decimal(f)
}

pub fn provas_json(provas: &[cds::Prova]) -> String {
    let v: Vec<Value> = provas
        .iter()
        .map(|p| {
            json!({
                "a0": hex_ponto(&p.a0), "a1": hex_ponto(&p.a1),
                "e0": dec_escalar(&p.e0), "e1": dec_escalar(&p.e1),
                "z0": dec_escalar(&p.z0), "z1": dec_escalar(&p.z1),
            })
        })
        .collect();
    Value::Array(v).to_string()
}

/// As provas de soma, uma por pergunta sigilosa.
pub fn somas_json(ps: &[soma::Prova]) -> String {
    let v: Vec<Value> = ps
        .iter()
        .map(|p| json!({ "a": hex_ponto(&p.a), "z": dec_escalar(&p.z) }))
        .collect();
    Value::Array(v).to_string()
}

/// As respostas em claro, achatadas sobre as perguntas públicas.
pub fn escolhas_json(es: &[u32]) -> String {
    Value::Array(es.iter().map(|e| json!(e)).collect()).to_string()
}

/// A cédula, no formato que o `abrir` do contrato espera.
pub fn perguntas_json(ps: &[Pergunta]) -> String {
    let v: Vec<Value> = ps
        .iter()
        .map(|p| json!({ "opcoes": p.opcoes, "confidencial": p.confidencial }))
        .collect();
    Value::Array(v).to_string()
}

pub fn pontos_json(ps: &[G1Affine]) -> String {
    Value::Array(ps.iter().map(|p| json!(hex_ponto(p))).collect()).to_string()
}

pub fn hashes_json(hs: &[[u8; 32]]) -> String {
    Value::Array(
        hs.iter()
            .map(|h| json!(h.iter().map(|b| format!("{:02x}", b)).collect::<String>()))
            .collect(),
    )
    .to_string()
}

pub fn enderecos_json(es: &[String]) -> String {
    Value::Array(es.iter().map(|e| json!(e)).collect()).to_string()
}

/// Divide cada `r_j` em shares `k`-de-`N` para a mesa.
///
/// No mundo real cada share viaja por canal autenticado e cifrado até o membro
/// dela. Aqui vão para `./shares/<membro>/`, que é a simulação honesta: o que
/// importa é que **nenhum lugar do sistema reúne os `r` de uma pessoa**, e a
/// mesa só consegue somar.
pub fn dividir_para_a_mesa(
    acasos: &[Fr],
    limiar: u32,
    membros: usize,
) -> Result<Vec<Vec<String>>, String> {
    let mut por_membro = vec![Vec::new(); membros];
    for r in acasos {
        let shares = shamir::dividir(r, limiar as usize, membros)
            .map_err(|e| format!("não consegui dividir para a mesa: {:?}", e))?;
        for (l, s) in shares.iter().enumerate() {
            por_membro[l].push(dec_escalar(&s.valor));
        }
    }
    Ok(por_membro)
}

/// Recebe `[membro][opção] = share` e devolve `R_j` reconstruído por `k`
/// membros — **sem que nenhum `r` individual se junte em lugar nenhum.**
pub fn reconstruir_aberturas(
    somas: &[(u32, Vec<Fr>)],
    limiar: u32,
    opcoes: usize,
) -> Result<Vec<Fr>, String> {
    if somas.len() < limiar as usize {
        return Err(format!(
            "só {} membros entregaram, e o limiar é {}",
            somas.len(),
            limiar
        ));
    }
    let mut r = Vec::with_capacity(opcoes);
    for j in 0..opcoes {
        let shares: Vec<shamir::Share> = somas
            .iter()
            .take(limiar as usize)
            .map(|(membro, v)| shamir::Share { membro: *membro, valor: v[j] })
            .collect();
        r.push(
            shamir::reconstruir(&shares, limiar as usize)
                .map_err(|e| format!("a mesa não reconstruiu a opção {}: {:?}", j, e))?,
        );
    }
    Ok(r)
}

pub fn xdr(endereco: &str) -> Result<Vec<u8>, String> {
    chave::xdr_de_endereco(endereco).map_err(|e| format!("endereço inválido: {:?}", e))
}

#[cfg(test)]
mod testes {
    use super::*;

    const H_HEX: &str = "1462b4b57a7d01e598685e913608bab6990de8cce1c705642c19a6b9ed660fc589758bdebb20c2bdde65a25b35b12206198669dff273abe7b9e6a0b3e636f8a8a41f9c81f6a79d308ec0fe57ef8a073c90e444002ba59807f987cce67d72eccf";
    const CONTA: &str = "GCZC4HW5TA2FSFPSFX3SJTOCDFQ6KZBHDHEG5O32STRX4PQXUX4NLGWJ";

    fn h() -> G1Affine {
        ponto::de_hex(H_HEX).unwrap()
    }

    /// Uma cédula montada aqui verifica com as mesmas funções que o contrato
    /// usa. Não substitui o teste contra o host — substitui descobrir na demo.
    #[test]
    fn a_cedula_que_eu_monto_verifica() {
        let g = pedersen::gerador();
        let (h, p32) = (h(), [7u8; 32]);
        let addr = xdr(CONTA).unwrap();

        let uma = [Pergunta { opcoes: 3, confidencial: true }];
        for escolha in 0..3 {
            let c = montar(&h, &p32, &addr, &uma, &[escolha]).unwrap();
            for j in 0..3 {
                assert!(
                    cds::verificar(&contexto(&p32, &addr, 0, j as u32), &g, &h, &c.compromissos[j], &c.provas[j]),
                    "a disjuntiva da opcao {} nao fecha",
                    j
                );
            }
            let d = soma::alvo(&g, &c.compromissos, 1);
            assert!(soma::verificar(
                &contexto(&p32, &addr, 0, OPCAO_DA_SOMA),
                &h,
                &d,
                &c.somas[0]
            ));
        }
    }

    /// **A cédula mista: uma prova de soma por pergunta, e nenhuma migra.**
    ///
    /// Duas perguntas sigilosas e uma pública. A prova da pergunta 1 tem de
    /// falhar quando conferida no contexto da pergunta 2 — é o furo que o
    /// índice da pergunta no desafio de Fiat–Shamir fecha.
    #[test]
    fn a_cedula_mista_fecha_pergunta_a_pergunta() {
        let g = pedersen::gerador();
        let (h, p32) = (h(), [11u8; 32]);
        let addr = xdr(CONTA).unwrap();
        let perguntas = [
            Pergunta { opcoes: 2, confidencial: true },
            Pergunta { opcoes: 2, confidencial: false },
            Pergunta { opcoes: 3, confidencial: true },
        ];
        let c = montar(&h, &p32, &addr, &perguntas, &[1, 0, 2]).unwrap();

        assert_eq!(c.compromissos.len(), 5, "2 + 3 opcoes sigilosas");
        assert_eq!(c.somas.len(), 2, "uma prova de soma por pergunta sigilosa");
        assert_eq!(c.publicas, vec![1, 0], "a pergunta publica vai em claro");
        assert_eq!(c.acasos.len(), 5, "um r por opcao sigilosa");

        // Cada pergunta sigilosa fecha no seu proprio contexto.
        let fatias: [(u32, usize, usize); 2] = [(0, 0, 2), (2, 2, 5)];
        for (q, ini, fim) in fatias {
            for j in ini..fim {
                assert!(cds::verificar(
                    &contexto(&p32, &addr, q, (j - ini) as u32),
                    &g, &h, &c.compromissos[j], &c.provas[j]
                ));
            }
        }
        let d0 = soma::alvo(&g, &c.compromissos[0..2], 1);
        assert!(soma::verificar(&contexto(&p32, &addr, 0, OPCAO_DA_SOMA), &h, &d0, &c.somas[0]));
        let d2 = soma::alvo(&g, &c.compromissos[2..5], 1);
        assert!(soma::verificar(&contexto(&p32, &addr, 2, OPCAO_DA_SOMA), &h, &d2, &c.somas[1]));

        // **A prova nao migra.** A disjuntiva da opcao 0 da pergunta 0,
        // conferida como se fosse da pergunta 2, tem de falhar.
        assert!(
            !cds::verificar(
                &contexto(&p32, &addr, 2, 0),
                &g, &h, &c.compromissos[0], &c.provas[0]
            ),
            "a prova da pergunta 1 nao pode valer na pergunta 3"
        );
        // E a prova de soma tambem nao.
        assert!(
            !soma::verificar(&contexto(&p32, &addr, 2, OPCAO_DA_SOMA), &h, &d0, &c.somas[0]),
            "a soma da pergunta 1 nao pode valer na pergunta 3"
        );
    }

    /// **A rodada da mesa, inteira, sem nenhum `r` individual se juntando.**
    ///
    /// Sete pessoas votam. Cada uma divide os seus `r_j` em 5 shares com
    /// limiar 3. Cada membro soma o que recebeu. Três membros reconstroem
    /// `R_j` — e a mesa então **procura** `T_j`, que é o que a cadeia só
    /// confere.
    #[test]
    fn a_mesa_fecha_a_apuracao_por_shamir() {
        let g = pedersen::gerador();
        let (h, p32) = (h(), [9u8; 32]);
        let addr = xdr(CONTA).unwrap();
        let (opcoes, limiar, membros) = (2usize, 3u32, 5usize);
        let escolhas = [0usize, 0, 1, 0, 1, 0, 0];

        // cada membro acumula as shares que recebe, por opção
        let mut caixa: Vec<Vec<Fr>> = vec![vec![pedersen::escalar(0); opcoes]; membros];
        let mut acumuladores = vec![Vec::new(); opcoes];

        let uma = [Pergunta { opcoes: 2, confidencial: true }];
        for escolha in escolhas {
            let c = montar(&h, &p32, &addr, &uma, &[escolha]).unwrap();
            for j in 0..opcoes {
                acumuladores[j].push(c.compromissos[j]);
            }
            // o que viaja para a mesa sao shares, nunca o r
            for (l, shares) in tessera_core::shamir::dividir(&c.acasos[0], limiar as usize, membros)
                .unwrap()
                .iter()
                .enumerate()
            {
                caixa[l][0] += shares.valor;
            }
            for (l, shares) in tessera_core::shamir::dividir(&c.acasos[1], limiar as usize, membros)
                .unwrap()
                .iter()
                .enumerate()
            {
                caixa[l][1] += shares.valor;
            }
        }

        // tres membros quaisquer, e so eles
        let somas: Vec<(u32, Vec<Fr>)> = vec![
            (1, caixa[0].clone()),
            (3, caixa[2].clone()),
            (5, caixa[4].clone()),
        ];
        let aberturas = reconstruir_aberturas(&somas, limiar, opcoes).unwrap();

        // a mesa PROCURA o total; a cadeia so confere
        for j in 0..opcoes {
            let a = pedersen::agregar(&acumuladores[j]);
            let esperado = escolhas.iter().filter(|&&e| e == j).count() as u64;
            assert_eq!(
                pedersen::descobrir_total(&a, &g, &h, &aberturas[j], 50),
                Some(esperado),
                "a mesa nao achou o total da opcao {}",
                j
            );
            assert!(pedersen::verifica_agregado(&a, &g, &h, esperado, &aberturas[j]));
        }

        // e dois membros nao fecham
        assert!(reconstruir_aberturas(&somas[..2], limiar, opcoes).is_err());
    }

    /// O escalar vai em decimal. Travado em teste porque a armadilha já
    /// mordeu: hex com só dígitos entra em silêncio como o decimal errado.
    #[test]
    fn escalar_serializa_em_decimal_no_json() {
        let s = somas_json(&[soma::Prova {
            a: pedersen::gerador(),
            z: pedersen::escalar(101),
        }]);
        assert!(s.contains("\"z\":\"101\""), "{}", s);
        assert!(!s.contains("0065"));
    }

    #[test]
    fn o_contexto_e_o_mesmo_que_o_contrato_monta() {
        let addr = xdr(CONTA).unwrap();
        let c = contexto(&[7u8; 32], &addr, 2, 3);
        assert_eq!(c.len(), 32 + 44 + 4 + 4);
        assert_eq!(&c[..32], &[7u8; 32]);
        assert_eq!(&c[32..76], &addr[..]);
        assert_eq!(&c[76..80], &2u32.to_be_bytes(), "o indice da pergunta");
        assert_eq!(&c[80..], &3u32.to_be_bytes(), "o indice da opcao");

        // Duas perguntas diferentes nao podem dar o mesmo contexto — e esse
        // era exatamente o furo antes de a pergunta entrar no desafio.
        assert_ne!(contexto(&[7u8; 32], &addr, 0, 0), contexto(&[7u8; 32], &addr, 1, 0));
    }
}
