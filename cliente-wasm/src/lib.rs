//! A matemática do Tessera no navegador.
//!
//! Este crate existe para que uma frase do projeto deixe de depender de quem
//! está rodando a demo. Hoje o `r` que esconde um voto nasce na CLI, na máquina
//! de quem organiza; quem organiza poderia guardá-lo. Aqui ele nasce na aba de
//! quem vota, numa função que devolve compromissos e provas e **nunca** o
//! devolve — e não existe servidor do outro lado que pudesse vê-lo, porque a
//! página é um arquivo estático.
//!
//! É o mesmo `tessera-core` do contrato, do verificador e da CLI. Não é uma
//! reimplementação em JavaScript: uma segunda implementação divergiria, e a
//! divergência entre provador e verificador é o bug mais caro possível neste
//! projeto. Por isso o console não roda BLS12-381 — este módulo roda, e é o
//! mesmo código Rust compilado para wasm32.
//!
//! ## O que sai daqui, e o que não sai
//!
//! Sai: compromissos, disjuntivas, provas de soma, assinatura em anel, imagem
//! de chave, folhas e caminhos de Merkle. Tudo público, tudo conferível.
//!
//! Não sai: o `r`. Ele vive dentro da chamada e morre com ela. As parcelas de
//! Shamir saem **só** quando a proposta tem mesa, porque sem mesa não há
//! apuração — e sem parcela não há como ninguém abrir um voto, nunca.

use serde::{Deserialize, Serialize};
use tessera_core::ark::{Fr, G1Affine};
use tessera_core::{anel, cds, merkle, pedersen, ponto, shamir, soma};
use wasm_bindgen::prelude::*;

// ---------- travessia hex ----------

fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{:02x}", x));
    }
    s
}

fn de_hex(s: &str) -> Result<Vec<u8>, JsValue> {
    if s.len() % 2 != 0 {
        return Err(JsValue::from_str("hex de comprimento ímpar"));
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| JsValue::from_str("hex inválido"))
        })
        .collect()
}

fn ponto_de(s: &str) -> Result<G1Affine, JsValue> {
    ponto::de_hex(s).map_err(|e| JsValue::from_str(&format!("ponto inválido: {:?}", e)))
}

fn fr_hex(f: &Fr) -> String {
    hex(&pedersen::fr_para_bytes_be(f))
}

fn fr_de(s: &str) -> Result<Fr, JsValue> {
    let b = de_hex(s)?;
    if b.len() != 32 {
        return Err(JsValue::from_str("escalar não tem 32 bytes"));
    }
    Ok(pedersen::fr_de_bytes_be(&b))
}

// ---------- formas que o JavaScript enxerga ----------

#[derive(Serialize, Deserialize)]
pub struct PerguntaJs {
    pub opcoes: u32,
    pub confidencial: bool,
}

#[derive(Serialize)]
pub struct ProvaCdsJs {
    pub a0: String,
    pub a1: String,
    pub e0: String,
    pub z0: String,
    pub e1: String,
    pub z1: String,
}

#[derive(Serialize)]
pub struct ProvaSomaJs {
    pub a: String,
    pub z: String,
}

#[derive(Serialize)]
pub struct Cedula {
    pub compromissos: Vec<String>,
    pub provas: Vec<ProvaCdsJs>,
    pub provas_soma: Vec<ProvaSomaJs>,
    pub escolhas: Vec<u32>,
    /// Presente **só** quando a proposta tem mesa. `parcelas[membro][opção]`.
    /// Sem mesa o campo vem vazio, e é essa ausência que torna a abertura de um
    /// voto impossível em vez de improvável.
    pub parcelas: Vec<Vec<String>>,
}

#[derive(Serialize)]
pub struct CedulaAnonima {
    pub cedula: Cedula,
    pub imagem: String,
    pub c0: String,
    pub z: Vec<String>,
}

#[derive(Serialize)]
pub struct ChaveDeAnel {
    /// Fica na aba. Vai para o `comparecer` só a pública.
    pub secreta: String,
    pub publica: String,
}

#[derive(Serialize)]
pub struct CaminhoJs {
    pub irmaos: Vec<String>,
    pub indice: u32,
    /// A seção que a lista deu a esta pessoa. Vai junto para o `comparecer`.
    pub secao: u32,
}

// ---------- a árvore de aptos ----------

/// A seção de cada apto, na ordem da lista.
///
/// Derivada, não escolhida: quem organiza não decide quem se esconde atrás de
/// quem. Qualquer pessoa com a lista recalcula isto e confere a raiz.
#[wasm_bindgen]
pub fn secoes_de(
    proposta_hex: String,
    enderecos_xdr: Vec<String>,
    secoes: u32,
) -> Result<Vec<u32>, JsValue> {
    let es = enderecos(&enderecos_xdr)?;
    Ok(merkle::dividir(&de_hex(&proposta_hex)?, &es, secoes))
}

#[wasm_bindgen]
pub fn raiz_de_aptos(
    proposta_hex: String,
    enderecos_xdr: Vec<String>,
    pesos: Vec<u32>,
    secoes: u32,
) -> Result<String, JsValue> {
    Ok(hex(
        &arvore(&proposta_hex, &enderecos_xdr, &pesos, secoes)?.raiz()
    ))
}

#[wasm_bindgen]
pub fn caminho_de(
    proposta_hex: String,
    enderecos_xdr: Vec<String>,
    pesos: Vec<u32>,
    secoes: u32,
    indice: usize,
) -> Result<JsValue, JsValue> {
    let a = arvore(&proposta_hex, &enderecos_xdr, &pesos, secoes)?;
    let c = a
        .caminho(indice)
        .map_err(|e| JsValue::from_str(&format!("caminho: {:?}", e)))?;
    let es = enderecos(&enderecos_xdr)?;
    let d = merkle::dividir(&de_hex(&proposta_hex)?, &es, secoes);
    let js = CaminhoJs {
        irmaos: c.irmaos.iter().map(|s| hex(s)).collect(),
        indice: c.indice,
        secao: d[indice],
    };
    serde_wasm_bindgen::to_value(&js).map_err(Into::into)
}

fn enderecos(xdr: &[String]) -> Result<Vec<Vec<u8>>, JsValue> {
    xdr.iter().map(|e| de_hex(e)).collect()
}

/// A raiz e o caminho saem **da mesma função**, de propósito: a divisão é
/// calculada aqui dentro, uma vez, a partir dos mesmos dados. Se o chamador
/// pudesse passar as seções por fora, uma raiz montada com uma divisão e um
/// caminho montado com outra dariam `NaoEstaNaListaDeAptos` sem dizer por quê.
fn arvore(
    proposta_hex: &str,
    enderecos_xdr: &[String],
    pesos: &[u32],
    secoes: u32,
) -> Result<merkle::Arvore, JsValue> {
    if enderecos_xdr.len() != pesos.len() {
        return Err(JsValue::from_str(
            "endereços e pesos de tamanhos diferentes",
        ));
    }
    let es = enderecos(enderecos_xdr)?;
    let d = merkle::dividir(&de_hex(proposta_hex)?, &es, secoes);
    let folhas: Vec<merkle::Apto> = es
        .into_iter()
        .zip(pesos)
        .zip(&d)
        .map(|((endereco, p), secao)| merkle::Apto {
            endereco,
            peso: *p,
            secao: *secao,
        })
        .collect();
    merkle::Arvore::montar(&folhas).map_err(|e| JsValue::from_str(&format!("árvore: {:?}", e)))
}

// ---------- a chave de anel ----------

/// Sorteia a chave com que a pessoa vai assinar o anel.
///
/// A secreta **não sai da aba**: ela vai para o `localStorage` de quem vota e é
/// o que permite votar depois de ter comparecido. Perder essa chave é perder o
/// voto, e é o preço de não ter servidor guardando nada.
#[wasm_bindgen]
pub fn nova_chave_de_anel() -> Result<JsValue, JsValue> {
    let x = pedersen::acaso_fr().map_err(|e| JsValue::from_str(&format!("acaso: {:?}", e)))?;
    let p = anel::chave_publica(&pedersen::gerador(), &x);
    let js = ChaveDeAnel {
        secreta: fr_hex(&x),
        publica: ponto::para_hex(&p),
    };
    serde_wasm_bindgen::to_value(&js).map_err(Into::into)
}

// ---------- a cédula ----------

/// A cédula identificada: o contexto das provas é o XDR do endereço.
#[wasm_bindgen]
pub fn cedula(
    proposta_hex: String,
    endereco_xdr: String,
    h_hex: String,
    perguntas_js: JsValue,
    escolhas: Vec<u32>,
    membros_mesa: usize,
    limiar: u32,
) -> Result<JsValue, JsValue> {
    let ident = de_hex(&endereco_xdr)?;
    let perguntas: Vec<PerguntaJs> = serde_wasm_bindgen::from_value(perguntas_js)?;
    let c = montar(
        &de_hex(&proposta_hex)?,
        &ident,
        &h_hex,
        &perguntas,
        &escolhas,
        membros_mesa,
        limiar,
    )?;
    serde_wasm_bindgen::to_value(&c).map_err(Into::into)
}

/// A cédula anônima: o contexto das provas é a **imagem de chave**, e a cédula
/// inteira é assinada em anel.
///
/// A ordem importa e não é arbitrária. A imagem é calculada primeiro porque ela
/// é a identidade que prende as provas; depois vêm os compromissos; e só então
/// o anel assina a mensagem que já contém tudo. Assinar antes deixaria a cédula
/// trocável depois da assinatura.
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn cedula_anonima(
    proposta_hex: String,
    hp_hex: String,
    h_hex: String,
    anel_hex: Vec<String>,
    indice: usize,
    secreta_hex: String,
    perguntas_js: JsValue,
    escolhas: Vec<u32>,
) -> Result<JsValue, JsValue> {
    let perguntas: Vec<PerguntaJs> = serde_wasm_bindgen::from_value(perguntas_js)?;
    let js = anonima(
        &proposta_hex,
        &hp_hex,
        &h_hex,
        &anel_hex,
        indice,
        &secreta_hex,
        &perguntas,
        &escolhas,
    )?;
    serde_wasm_bindgen::to_value(&js).map_err(Into::into)
}

/// A mesma coisa, em tipos Rust puros — é esta que o teste de aceitação do
/// contrato chama.
///
/// Existir separada não é zelo: a primeira versão montava a mensagem do anel
/// aqui dentro e o teste a montava de fora, com os mesmos nomes e tipos. As
/// duas divergiram, a assinatura passou a cobrir outra coisa, e o contrato
/// recusou com `AnelInvalido` numa rodada de testnet. O teste não pegou porque
/// não passava por aqui.
#[allow(clippy::too_many_arguments)]
pub fn anonima(
    proposta_hex: &str,
    hp_hex: &str,
    h_hex: &str,
    anel_hex: &[String],
    indice: usize,
    secreta_hex: &str,
    perguntas: &[PerguntaJs],
    escolhas: &[u32],
) -> Result<CedulaAnonima, JsValue> {
    let proposta = de_hex(proposta_hex)?;
    let g = pedersen::gerador();
    let hp = ponto_de(hp_hex)?;
    let x = fr_de(secreta_hex)?;
    let anel_pts: Vec<G1Affine> = anel_hex
        .iter()
        .map(|s| ponto_de(s))
        .collect::<Result<_, JsValue>>()?;

    let imagem = anel::imagem(&hp, &x);
    let ident = ponto::serializar(&imagem).to_vec();

    // Sem mesa: a cédula anônima do dapp aberto não reparte `r` com ninguém.
    let c = montar(&proposta, &ident, h_hex, perguntas, escolhas, 0, 0)?;

    // `c.escolhas` — as respostas públicas expandidas —, NÃO o `escolhas` de
    // entrada, que é uma por pergunta. É o que o contrato lê em
    // `mensagem_cedula`, e assinar o outro faz a assinatura cobrir uma cédula
    // que ninguém mandou.
    let msg = mensagem(&proposta, &c.compromissos, &c.escolhas)?;
    let s = anel::assinar(&msg, &g, &hp, &anel_pts, indice, &x)
        .map_err(|e| JsValue::from_str(&format!("anel: {:?}", e)))?;

    Ok(CedulaAnonima {
        imagem: ponto::para_hex(&s.imagem),
        c0: fr_hex(&s.c0),
        z: s.z.iter().map(fr_hex).collect(),
        cedula: c,
    })
}

/// `proposta ‖ len(compromissos) ‖ compromissos ‖ len(escolhas) ‖ escolhas`.
///
/// **Byte a byte igual a `cripto::mensagem_cedula` do contrato.** Divergir aqui
/// faria toda assinatura falhar sem dizer por quê.
pub fn mensagem(
    proposta: &[u8],
    compromissos: &[String],
    escolhas: &[u32],
) -> Result<Vec<u8>, JsValue> {
    let mut b = proposta.to_vec();
    b.extend_from_slice(&(compromissos.len() as u32).to_be_bytes());
    for c in compromissos {
        b.extend_from_slice(&de_hex(c)?);
    }
    b.extend_from_slice(&(escolhas.len() as u32).to_be_bytes());
    for e in escolhas {
        b.extend_from_slice(&e.to_be_bytes());
    }
    Ok(b)
}

/// `proposta ‖ identidade ‖ pergunta ‖ opção` — igual a `cripto::contexto_de`.
pub fn contexto(proposta: &[u8], ident: &[u8], pergunta: u32, opcao: u32) -> Vec<u8> {
    let mut v = proposta.to_vec();
    v.extend_from_slice(ident);
    v.extend_from_slice(&pergunta.to_be_bytes());
    v.extend_from_slice(&opcao.to_be_bytes());
    v
}

/// Monta a cédula a partir de tipos Rust puros.
///
/// É público de propósito: o teste de aceitação do contrato chama **esta**
/// função, e não uma reescrita dela. O que o navegador roda e o que o teste
/// exercita são o mesmo código — a única coisa que fica de fora é a travessia
/// `JsValue`, que é trabalho do `wasm-bindgen`, não meu.
pub fn montar(
    proposta: &[u8],
    ident: &[u8],
    h_hex: &str,
    perguntas: &[PerguntaJs],
    escolhas: &[u32],
    membros_mesa: usize,
    limiar: u32,
) -> Result<Cedula, JsValue> {
    if perguntas.len() != escolhas.len() {
        return Err(JsValue::from_str(
            "uma escolha por pergunta, na ordem da cédula",
        ));
    }
    let g = pedersen::gerador();
    let h = ponto_de(h_hex)?;

    let mut c = Cedula {
        compromissos: Vec::new(),
        provas: Vec::new(),
        provas_soma: Vec::new(),
        escolhas: Vec::new(),
        parcelas: vec![Vec::new(); membros_mesa],
    };

    for (q, pg) in perguntas.iter().enumerate() {
        let escolha = escolhas[q];
        if escolha >= pg.opcoes {
            return Err(JsValue::from_str("escolha fora das opções da pergunta"));
        }
        if !pg.confidencial {
            for j in 0..pg.opcoes {
                c.escolhas.push(u32::from(j == escolha));
            }
            continue;
        }

        // Aqui nasce o `r`, e é a única função do sistema inteiro em que ele
        // existe. Nada abaixo o devolve.
        let rs: Vec<Fr> = (0..pg.opcoes)
            .map(|_| {
                pedersen::acaso_fr().map_err(|e| JsValue::from_str(&format!("acaso: {:?}", e)))
            })
            .collect::<Result<_, JsValue>>()?;
        let cs: Vec<G1Affine> = (0..pg.opcoes)
            .map(|j| {
                let v = u64::from(j == escolha);
                pedersen::comprometer(&g, &h, &pedersen::escalar(v), &rs[j as usize])
            })
            .collect();

        for j in 0..pg.opcoes {
            let v = u64::from(j == escolha);
            let p = cds::provar(
                &contexto(proposta, ident, q as u32, j),
                &g,
                &h,
                &cs[j as usize],
                v,
                &rs[j as usize],
            )
            .map_err(|e| JsValue::from_str(&format!("disjuntiva: {:?}", e)))?;
            c.compromissos.push(ponto::para_hex(&cs[j as usize]));
            c.provas.push(ProvaCdsJs {
                a0: ponto::para_hex(&p.a0),
                a1: ponto::para_hex(&p.a1),
                e0: fr_hex(&p.e0),
                z0: fr_hex(&p.z0),
                e1: fr_hex(&p.e1),
                z1: fr_hex(&p.z1),
            });
        }

        // Uma prova de soma por pergunta. Uma só para a cédula inteira
        // permitiria compensar entre perguntas.
        let rho = rs.iter().fold(Fr::from(0u64), |a, r| a + r);
        let d = soma::alvo(&g, &cs, 1);
        let ps = soma::provar(&contexto(proposta, ident, q as u32, u32::MAX), &h, &d, &rho)
            .map_err(|e| JsValue::from_str(&format!("soma: {:?}", e)))?;
        c.provas_soma.push(ProvaSomaJs {
            a: ponto::para_hex(&ps.a),
            z: fr_hex(&ps.z),
        });

        if membros_mesa > 0 {
            for r in &rs {
                let partes = shamir::dividir(r, limiar as usize, membros_mesa)
                    .map_err(|e| JsValue::from_str(&format!("shamir: {:?}", e)))?;
                for (m, p) in partes.iter().enumerate() {
                    c.parcelas[m].push(fr_hex(&p.valor));
                }
            }
        }
    }

    Ok(c)
}
