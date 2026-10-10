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
use tessera_core::{anel, cds, merkle, pedersen, ponto, relogio, shamir, soma};
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
    bytes_de_hex(s).map_err(|e| JsValue::from_str(&e))
}

/// O mesmo, com erro em `String`.
///
/// Existe porque **`JsValue` não pode ser construído fora do wasm32** — ele
/// entra em pânico, e um pânico dentro de um teste do contrato aborta o
/// processo em vez de falhar (medido em T-019, `SIGABRT`). O caminho da
/// decifragem erra em `String` porque é o único cujos **erros** o teste nativo
/// exercita — recusar a assinatura errada é o que ele tem a provar.
fn bytes_de_hex(s: &str) -> Result<Vec<u8>, String> {
    if !s.len().is_multiple_of(2) {
        return Err("hex de comprimento ímpar".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| String::from("hex inválido")))
        .collect()
}

fn ponto_de(s: &str) -> Result<G1Affine, JsValue> {
    ponto_em(s).map_err(|e| JsValue::from_str(&e))
}

fn ponto_em(s: &str) -> Result<G1Affine, String> {
    ponto::de_hex(s).map_err(|e| std::format!("ponto inválido: {:?}", e))
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
    /// Os criptogramas da fechadura de tempo, concatenados em hexadecimal: 160
    /// bytes por opção confidencial, na mesma ordem dos compromissos. Vazio
    /// quando a proposta não tem fechadura (`rodada = 0`).
    ///
    /// É o **único** lugar do sistema em que o `r` sai da função que o criou —
    /// e sai cifrado para uma chave que ainda não existe. Ninguém, nem quem
    /// vota, pode abri-lo antes da rodada vencer.
    pub cripto: String,
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
}

// ---------- a árvore de aptos ----------

/// A seção de cada apto, na ordem da lista.
///
/// **Derivada da assinatura da baliza, não do identificador da proposta**
/// (DEC-012). Enquanto saía da proposta, quem organizava moía o identificador
/// até pôr o dissidente numa seção cheia de atacantes; a assinatura da rodada de
/// abertura não existe na hora de abrir, então não há o que moer. Qualquer
/// pessoa com a lista e a assinatura recalcula isto.
#[wasm_bindgen]
pub fn secoes_de(
    assinatura_hex: String,
    enderecos_xdr: Vec<String>,
    secoes: u32,
) -> Result<Vec<u32>, JsValue> {
    let es = enderecos(&enderecos_xdr)?;
    Ok(merkle::dividir(&de_hex(&assinatura_hex)?, &es, secoes))
}

/// A seção de **uma** pessoa. É o que a tela precisa para declarar o footprint
/// e para dizer à pessoa onde ela vota.
#[wasm_bindgen]
pub fn secao_de(assinatura_hex: String, endereco_xdr: String, secoes: u32) -> Result<u32, JsValue> {
    Ok(merkle::secao_de(
        &de_hex(&assinatura_hex)?,
        &de_hex(&endereco_xdr)?,
        secoes,
    ))
}

/// Quantas seções para um eleitorado deste tamanho: média de 20 por seção, teto
/// de 32 pela CPU do anel. Ver `core::merkle::ALVO_SECAO`.
#[wasm_bindgen]
pub fn secoes_para(aptos: usize) -> u32 {
    merkle::secoes_para(aptos)
}

/// A assinatura comprimida da baliza nos 96 bytes que o host lê, para
/// `registrar_abertura`. Valida o ponto de passagem.
#[wasm_bindgen]
pub fn assinatura_da_baliza(comprimida_hex: String) -> Result<String, JsValue> {
    let b = relogio::assinatura_para_host(&de_hex(&comprimida_hex)?)
        .map_err(|e| JsValue::from_str(&format!("assinatura: {:?}", e)))?;
    Ok(hex(&b))
}

/// A rodada da baliza que vence num instante, e o inverso. A tela escolhe o
/// prazo em tempo e o contrato confere a rodada — e a conta é a mesma dos dois
/// lados porque vem do mesmo módulo.
#[wasm_bindgen]
pub fn rodada_em(instante: u64) -> Result<u64, JsValue> {
    relogio::rodada(instante).map_err(|e| JsValue::from_str(&format!("rodada: {:?}", e)))
}

#[wasm_bindgen]
pub fn instante_da_rodada(rodada: u64) -> u64 {
    relogio::instante(rodada)
}

#[wasm_bindgen]
pub fn raiz_de_aptos(enderecos_xdr: Vec<String>, pesos: Vec<u32>) -> Result<String, JsValue> {
    Ok(hex(&arvore(&enderecos_xdr, &pesos)?.raiz()))
}

#[wasm_bindgen]
pub fn caminho_de(
    enderecos_xdr: Vec<String>,
    pesos: Vec<u32>,
    indice: usize,
) -> Result<JsValue, JsValue> {
    let a = arvore(&enderecos_xdr, &pesos)?;
    let c = a
        .caminho(indice)
        .map_err(|e| JsValue::from_str(&format!("caminho: {:?}", e)))?;
    let js = CaminhoJs {
        irmaos: c.irmaos.iter().map(|s| hex(s)).collect(),
        indice: c.indice,
    };
    serde_wasm_bindgen::to_value(&js).map_err(Into::into)
}

fn enderecos(xdr: &[String]) -> Result<Vec<Vec<u8>>, JsValue> {
    xdr.iter().map(|e| de_hex(e)).collect()
}

/// A árvore não sabe mais de seções: a folha é `H(0x00 ‖ endereço ‖ peso_be)`,
/// e a seção passou a sair da baliza (DEC-012). Era a divisão calculada aqui
/// dentro que garantia raiz e caminho coerentes; agora não há nada a coordenar,
/// porque não há divisão na folha.
fn arvore(enderecos_xdr: &[String], pesos: &[u32]) -> Result<merkle::Arvore, JsValue> {
    if enderecos_xdr.len() != pesos.len() {
        return Err(JsValue::from_str(
            "endereços e pesos de tamanhos diferentes",
        ));
    }
    let folhas: Vec<merkle::Apto> = enderecos(enderecos_xdr)?
        .into_iter()
        .zip(pesos)
        .map(|(endereco, p)| merkle::Apto { endereco, peso: *p })
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

// ---------- a fechadura, do outro lado ----------

/// O que quem apura apresenta ao contrato, já reconstruído a partir dos
/// criptogramas.
#[derive(Serialize)]
pub struct AberturaJs {
    /// Uma por cédula, na ordem da cadeia: se o criptograma dela abriu.
    pub abertas: Vec<bool>,
    /// `totais[opção]` — a soma dos votos das cédulas que abriram.
    pub totais: Vec<u32>,
    /// `aberturas[opção]` — a soma dos fatores das mesmas cédulas.
    pub aberturas: Vec<String>,
    /// Quantas cédulas abriram, de quantas havia. É o que a tela diz em voz
    /// alta, porque um placar sobre um subconjunto tem de dizer qual.
    pub abriram: u32,
    pub cedulas: u32,
}

/// Confere a assinatura da baliza contra a chave pública congelada da cadeia.
///
/// Existe separada de [`abertura_da_secao`] porque **a tela confere antes de
/// usar** (INV-24): o relé de onde a assinatura vem é um servidor como qualquer
/// outro, e nada do que ele diz entra no cálculo sem passar por um pareamento.
/// Devolve o erro em vez de um booleano para que não haja como ignorar.
#[wasm_bindgen]
pub fn conferir_baliza(rodada: u64, assinatura_hex: String) -> Result<(), JsValue> {
    relogio::conferir(rodada, &de_hex(&assinatura_hex)?)
        .map(|_| ())
        .map_err(|e| JsValue::from_str(&format!("baliza: {:?}", e)))
}

/// Abre a seção inteira: decifra cada criptograma, descobre o voto de cada
/// compromisso, e soma.
///
/// `compromissos` vem achatado em `cédulas × opções confidenciais`, na ordem da
/// cadeia; `criptos[i]` são os 160 bytes por opção daquela cédula, em
/// hexadecimal. A chave sai de [`relogio::conferir`] — não existe caminho de
/// tipos que decifre sem ela.
///
/// **Uma cédula que não abre perde só o próprio voto.** Criptograma mexido,
/// cifrado para outra rodada, ou que não corresponde ao compromisso: a cédula
/// entra como `abertas[i] = false` e a apuração segue. É isso que impede que
/// sabotar o próprio criptograma trave o placar de todos — e é por isso que
/// esta função não devolve erro no primeiro tropeço.
#[wasm_bindgen]
pub fn abertura_da_secao(
    compromissos_hex: Vec<String>,
    criptos_hex: Vec<String>,
    n_conf: usize,
    h_hex: String,
    rodada: u64,
    assinatura_hex: String,
) -> Result<JsValue, JsValue> {
    let js = abertura(
        &compromissos_hex,
        &criptos_hex,
        n_conf,
        &h_hex,
        rodada,
        &assinatura_hex,
    )
    .map_err(|e| JsValue::from_str(&e))?;
    serde_wasm_bindgen::to_value(&js).map_err(Into::into)
}

/// A mesma coisa em tipos Rust puros — é esta que o teste de aceitação do
/// contrato chama, pelo mesmo motivo de [`anonima`]: uma segunda implementação
/// divergiria, e aqui divergir significa um placar que o contrato recusa.
pub fn abertura(
    compromissos_hex: &[String],
    criptos_hex: &[String],
    n_conf: usize,
    h_hex: &str,
    rodada: u64,
    assinatura_hex: &str,
) -> Result<AberturaJs, String> {
    let chave = relogio::conferir(rodada, &bytes_de_hex(assinatura_hex)?)
        .map_err(|e| std::format!("baliza: {:?}", e))?;
    let g = pedersen::gerador();
    let h = ponto_em(h_hex)?;
    let quantas = criptos_hex.len();
    if n_conf == 0 || compromissos_hex.len() != quantas * n_conf {
        return Err(
            "um compromisso por opção confidencial em cada cédula, na ordem da cadeia".into(),
        );
    }

    let mut js = AberturaJs {
        abertas: Vec::with_capacity(quantas),
        totais: vec![0u32; n_conf],
        aberturas: Vec::new(),
        abriram: 0,
        cedulas: quantas as u32,
    };
    let mut somas = vec![Fr::from(0u64); n_conf];

    for (i, cripto) in criptos_hex.iter().enumerate() {
        let bruto = bytes_de_hex(cripto)?;
        // Decifra a cédula inteira **antes** de somar qualquer coisa: uma
        // cédula que abre pela metade não entra pela metade.
        let mut abertos: Vec<(usize, Fr)> = Vec::with_capacity(n_conf);
        for j in 0..n_conf {
            let fatia = match bruto.get(j * relogio::TAMANHO..(j + 1) * relogio::TAMANHO) {
                Some(f) => f,
                None => break,
            };
            let r = match relogio::decifrar(fatia, &chave) {
                Ok(r) => r,
                Err(_) => break,
            };
            // O `r` sozinho não diz o voto: diz qual dos dois compromissos
            // possíveis é o que está no ledger. A disjuntiva já garantiu, na
            // hora do voto, que não há terceira opção.
            let c = ponto_em(&compromissos_hex[i * n_conf + j])?;
            let v = if pedersen::comprometer(&g, &h, &pedersen::escalar(0), &r) == c {
                0
            } else if pedersen::comprometer(&g, &h, &pedersen::escalar(1), &r) == c {
                1
            } else {
                break;
            };
            abertos.push((v, r));
        }

        let abriu = abertos.len() == n_conf;
        js.abertas.push(abriu);
        if abriu {
            js.abriram += 1;
            for (j, (v, r)) in abertos.iter().enumerate() {
                js.totais[j] += *v as u32;
                somas[j] += r;
            }
        }
    }

    js.aberturas = somas.iter().map(fr_hex).collect();
    Ok(js)
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
        // A cédula identificada é o caminho da mesa, e ali quem abre são as
        // parcelas de Shamir. Fechadura de tempo e mesa não se somam.
        0,
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
    rodada: u64,
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
        rodada,
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
    rodada: u64,
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
    let c = montar(&proposta, &ident, h_hex, perguntas, escolhas, 0, 0, rodada)?;

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
#[allow(clippy::too_many_arguments)]
pub fn montar(
    proposta: &[u8],
    ident: &[u8],
    h_hex: &str,
    perguntas: &[PerguntaJs],
    escolhas: &[u32],
    membros_mesa: usize,
    limiar: u32,
    // A rodada da baliza para a qual cifrar o `r`. `0` não cifra nada.
    rodada: u64,
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
        cripto: String::new(),
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

        // A fechadura. Cifrar aqui, dentro do laço que tem o `r`, é o que
        // permite apurar sem mesa: o fator viaja no evento, ilegível, e a
        // chave que o abre nasce sozinha no instante da rodada.
        if rodada != 0 {
            for r in &rs {
                let cg = relogio::cifrar(r, rodada)
                    .map_err(|e| JsValue::from_str(&format!("fechadura: {:?}", e)))?;
                c.cripto.push_str(&hex(&cg));
            }
        }

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
