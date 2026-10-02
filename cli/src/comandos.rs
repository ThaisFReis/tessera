//! Os sete comandos.
//!
//! A ordem das telas segue `docs/UX-CLI.md`, e as frases que estão lá entre
//! aspas estão aqui entre aspas. Duas delas não podem ser suavizadas, e os
//! comentários dizem por quê.

use crate::cadeia::Cadeia;
use crate::cedula::{self, Cedula};
use crate::estado::{Apuracao, Estado, Mesa, PerguntaEstado, Verificacao, Voto};
use crate::recibo::{self, Recibo};
use crate::tela;
use sha2::{Digest, Sha256};
use tessera_core::ark::{Fr, G1Affine};
use tessera_core::{cds, merkle, pedersen, ponto};

type R = Result<(), String>;

/// O id de 32 bytes que o contrato usa, derivado do nome que o humano digita.
///
/// Assim quem opera escreve `contas-2025` e o ledger recebe um identificador
/// de tamanho fixo, sem uma tabela de tradução em lugar nenhum.
fn id32(nome: &str) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"TESSERA-V1-PROPOSTA");
    h.update(nome.as_bytes());
    h.finalize().into()
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

fn de_hex(s: &str) -> Result<Vec<u8>, String> {
    if s.len() % 2 != 0 {
        return Err("hex de tamanho ímpar".into());
    }
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

/// `--aptos a,b,c` ou `--aptos arquivo.txt` (um nome por linha).
fn lista(arg: &str) -> Result<Vec<String>, String> {
    if std::path::Path::new(arg).exists() {
        let s = std::fs::read_to_string(arg).map_err(|e| e.to_string())?;
        return Ok(s
            .lines()
            .map(|l| l.split('#').next().unwrap_or("").trim().to_string())
            .filter(|l| !l.is_empty())
            .collect());
    }
    Ok(arg.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
}

/// `2h`, `30m`, `90s` ou um número de ledgers. Um ledger fecha em ~5 s.
fn prazo_em_ledgers(s: &str) -> Result<u64, String> {
    let s = s.trim();
    let (n, mult) = match s.chars().last() {
        Some('h') => (&s[..s.len() - 1], 720),
        Some('m') => (&s[..s.len() - 1], 12),
        Some('s') => (&s[..s.len() - 1], 1),
        _ => (s, 1),
    };
    let v: u64 = n.parse().map_err(|_| format!("não entendi o prazo {:?}", s))?;
    Ok(v * mult)
}

/// "Ainda não abriu" e "encerra em tanto" são estados diferentes, e a tela de
/// quem vai votar precisa dizer qual dos dois é — senão a pessoa tenta votar e
/// leva uma recusa que parece defeito.
fn janela(inicio: u64, fim: u64, agora: u64) -> String {
    if agora < inicio {
        format!("Abre {}", relogio(inicio as i64 - agora as i64))
    } else {
        format!("Encerra {}", relogio(fim as i64 - agora as i64))
    }
}

fn relogio(ledgers: i64) -> String {
    if ledgers <= 0 {
        return "encerrada".into();
    }
    let s = ledgers * 5;
    // Abaixo de um minuto, "em 0min" soa como "já era" — e numa janela curta é
    // justamente o trecho em que alguém está olhando o relógio.
    if s < 60 {
        format!("em {}s", s)
    } else if s < 3600 {
        format!("em {}min", s / 60)
    } else {
        format!("em {}h{:02}", s / 3600, (s % 3600) / 60)
    }
}

fn h_do_contrato(e: &Estado) -> Result<G1Affine, String> {
    ponto::de_hex(&e.gerador_h).map_err(|x| format!("H do contrato ilegível: {:?}", x))
}

/// Os rótulos das opções **sigilosas**, achatados em ordem — a mesma ordem dos
/// acumuladores, dos totais e das aberturas.
///
/// Numa cédula de várias perguntas o rótulo precisa dizer de qual pergunta ele
/// é: duas perguntas podem ter uma opção `sim`, e "SIM: 4" sem contexto é um
/// número solto.
/// O resultado achatado sobre **todas** as perguntas, somando a parte sigilosa
/// (que a mesa abriu) com a parte em claro (que já estava no ledger).
///
/// As duas origens da parte em claro são diferentes e é por isso que a conta
/// não é um `zip`: quem votou em sigilo respondeu em claro só as perguntas
/// públicas, e quem abriu o voto respondeu em claro a cédula inteira.
fn resultado_local(e: &Estado, totais: &[u32]) -> Vec<u32> {
    let mut r = Vec::new();
    let (mut off_todas, mut off_publ, mut off_conf) = (0usize, 0usize, 0usize);

    for pg in e.perguntas.iter() {
        for j in 0..pg.opcoes.len() {
            let mut claro = 0u32;
            for v in e.votos.iter() {
                let Some(esc) = v.escolhas.as_ref() else { continue };
                let i = if v.publico { off_todas + j } else if pg.confidencial { continue } else { off_publ + j };
                claro += esc.get(i).copied().unwrap_or(0);
            }
            r.push(if pg.confidencial { totais[off_conf + j] + claro } else { claro });
        }
        off_todas += pg.opcoes.len();
        if pg.confidencial {
            off_conf += pg.opcoes.len();
        } else {
            off_publ += pg.opcoes.len();
        }
    }
    r
}

fn rotulos_confidenciais(e: &Estado) -> Vec<String> {
    let varias = e.perguntas.iter().filter(|p| p.confidencial).count() > 1;
    e.perguntas
        .iter()
        .enumerate()
        .filter(|(_, p)| p.confidencial)
        .flat_map(|(q, p)| {
            p.opcoes.iter().map(move |o| {
                if varias {
                    format!("{}·{}", q + 1, o)
                } else {
                    o.clone()
                }
            })
        })
        .collect()
}

// ===================== abrir =========================================

/// Lê uma pergunta no formato `"texto | opção, opção | sigilosa"`.
///
/// A natureza cai em `sigilosa` quando omitida — o sigilo é o padrão, e abrir
/// uma pergunta tem de ser um ato deliberado de quem escreve a cédula.
fn ler_pergunta(spec: &str) -> Result<PerguntaEstado, String> {
    let partes: Vec<&str> = spec.split('|').map(|p| p.trim()).collect();
    if partes.len() < 2 {
        return Err(format!(
            "não entendi a pergunta {:?}.\n  Use: \"texto | opção, opção | sigilosa\"",
            spec
        ));
    }
    let texto = partes[0].to_string();
    if texto.is_empty() {
        return Err(format!("a pergunta {:?} está sem texto", spec));
    }
    let opcoes: Vec<String> = lista(partes[1])?;
    if opcoes.len() < 2 || opcoes.len() > 16 {
        return Err(format!(
            "a pergunta {:?} tem {} opções, e são de 2 a 16",
            texto,
            opcoes.len()
        ));
    }
    let confidencial = match partes.get(2).map(|n| n.to_lowercase()) {
        None => true,
        Some(n) if n.is_empty() || n == "sigilosa" || n == "confidencial" => true,
        Some(n) if n == "publica" || n == "pública" || n == "aberta" => false,
        Some(n) => {
            return Err(format!(
                "não sei o que é uma pergunta {:?}. Use `sigilosa` ou `publica`.",
                n
            ))
        }
    };
    Ok(PerguntaEstado { texto, opcoes, confidencial })
}

#[allow(clippy::too_many_arguments)]
pub fn abrir(
    proposta: &str,
    pergunta: &[String],
    opcoes: Option<&str>,
    aptos: &str,
    mesa: &str,
    limiar: u32,
    inicio: &str,
    prazo: &str,
    contrato: &str,
    rede: &str,
    governanca: &str,
) -> R {
    // Duas formas. A curta — `--pergunta TEXTO --opcoes a,b` — é a cédula de
    // sempre: uma pergunta, sigilosa. A longa é `--pergunta` repetido, com a
    // natureza de cada uma.
    let perguntas: Vec<PerguntaEstado> = match opcoes {
        Some(o) => {
            if pergunta.len() != 1 {
                return Err(
                    "com `--opcoes` vai uma `--pergunta` só.\n  Para várias perguntas, repita `--pergunta \"texto | opções | sigilosa\"` e não use `--opcoes`."
                        .into(),
                );
            }
            let opcoes = lista(o)?;
            if opcoes.len() < 2 || opcoes.len() > 16 {
                return Err(format!("são 2 a 16 opções, e você deu {}", opcoes.len()));
            }
            vec![PerguntaEstado { texto: pergunta[0].clone(), opcoes, confidencial: true }]
        }
        None => {
            if pergunta.is_empty() {
                return Err("a cédula está vazia: passe ao menos uma `--pergunta`.".into());
            }
            pergunta.iter().map(|p| ler_pergunta(p)).collect::<Result<_, _>>()?
        }
    };
    if perguntas.len() > 8 {
        return Err(format!("são até 8 perguntas, e você deu {}", perguntas.len()));
    }
    // O orçamento de CPU é limitado pelas opções **sigilosas**: 13.501.500
    // instruções cada. As públicas são conferidas a olho e custam ~350 mil a
    // cédula inteira.
    let conf: usize = perguntas.iter().filter(|p| p.confidencial).map(|p| p.opcoes.len()).sum();
    if conf > 16 {
        return Err(format!(
            "a cédula tem {} opções sigilosas somadas, e o teto é 16.\n  \
             Cada uma custa 13,5M de instruções; 16 já são 56% do teto de uma transação.\n  \
             Abra alguma pergunta (`| publica`) ou divida a cédula.",
            conf
        ));
    }
    let nomes_aptos = lista(aptos)?;
    let nomes_mesa = lista(mesa)?;
    if nomes_aptos.is_empty() {
        return Err("a lista de aptos está vazia".into());
    }
    if limiar == 0 || limiar as usize > nomes_mesa.len() {
        return Err(format!(
            "o limiar é {} e a mesa tem {} membros",
            limiar,
            nomes_mesa.len()
        ));
    }

    let c = Cadeia::nova(contrato, rede);
    let enderecos_aptos: Vec<String> = nomes_aptos
        .iter()
        .map(|n| resolver(n))
        .collect::<Result<_, _>>()?;
    let enderecos_mesa: Vec<String> = nomes_mesa
        .iter()
        .map(|n| resolver(n))
        .collect::<Result<_, _>>()?;

    // A árvore: folhas H(0x00 ‖ addr_xdr ‖ peso), na ORDEM da lista. A ordem é
    // o documento — a raiz muda se ela mudar.
    let folhas: Vec<merkle::Apto> = enderecos_aptos
        .iter()
        .map(|a| cedula::xdr(a).map(|e| merkle::Apto { endereco: e, peso: 1 }))
        .collect::<Result<_, _>>()?;
    let arvore = merkle::Arvore::montar(&folhas).map_err(|e| format!("{:?}", e))?;
    let raiz = arvore.raiz();

    let agora = Cadeia::ledger_atual(rede).ok_or("não consegui ler o ledger atual")?;
    // A janela é contada a partir de agora: `--inicio` diz quando ela começa e
    // `--prazo` quanto ela dura depois disso. `--inicio 0` é abertura imediata,
    // que é o padrão.
    let espera = prazo_em_ledgers(inicio)?;
    let abre_em = agora + espera;
    let fecha_em = abre_em + prazo_em_ledgers(prazo)?;
    let pid = id32(proposta);

    tela::titulo("abrindo votação");
    tela::branco();
    for (q, pg) in perguntas.iter().enumerate() {
        tela::linha(&format!("{}. {}", q + 1, pg.texto));
        tela::campo(
            if pg.confidencial { "  em sigilo" } else { "  em aberto" },
            &pg.opcoes.join(" · "),
        );
    }
    tela::branco();
    tela::campo(
        "Aptos",
        &format!("{}  (raiz de Merkle {})", enderecos_aptos.len(), tela::abreviar(&hex(&raiz), 4, 4)),
    );
    tela::campo("Peso", "um voto por pessoa");
    tela::campo(
        "Mesa",
        &format!("{} membros, {} assinaturas para apurar", enderecos_mesa.len(), limiar),
    );
    tela::campo("Sigilo mínimo", "5 votos confidenciais");
    tela::campo(
        "Abre",
        &if espera == 0 {
            "agora".to_string()
        } else {
            format!("{}  (ledger {})", relogio(espera as i64), abre_em)
        },
    );
    tela::campo(
        "Encerra",
        &format!("{}  (ledger {})", relogio((fecha_em - agora) as i64), fecha_em),
    );

    let r = c
        .invocar(
            governanca,
            true,
            "abrir",
            &[
                ("governanca", resolver(governanca)?),
                ("proposta", hex(&pid)),
                (
                    "perguntas",
                    cedula::perguntas_json(
                        &perguntas
                            .iter()
                            .map(|p| cedula::Pergunta {
                                opcoes: p.opcoes.len(),
                                confidencial: p.confidencial,
                            })
                            .collect::<Vec<_>>(),
                    ),
                ),
                ("raiz_aptos", hex(&raiz)),
                ("mesa", cedula::enderecos_json(&enderecos_mesa)),
                ("limiar", limiar.to_string()),
                ("abre_em", abre_em.to_string()),
                ("fecha_em", fecha_em.to_string()),
            ],
        )
        .map_err(|e| e.to_string())?;

    let tx = r.tx.clone().unwrap_or_default();
    let (taxa, ledger) = c.detalhes(&tx).unwrap_or((0, 0));

    // H vem do contrato, não é recalculado aqui: é o ponto em que cliente e
    // cadeia precisam concordar, e pedir é mais seguro que deduzir.
    let gh = c
        .invocar(governanca, false, "gerador_h", &[])
        .map_err(|e| e.to_string())?
        .valor
        .trim_matches('"')
        .to_string();

    tela::secao("publicado");
    tela::campo("Contrato", &tela::abreviar(contrato, 8, 4));
    tela::campo("Transação", &format!("{}       ledger {}", &tx[..8.min(tx.len())], ledger));
    tela::campo("Taxa", &format!("{} stroops", taxa));
    tela::confere("votação aberta");

    Estado {
        proposta: proposta.into(),
        perguntas,
        contrato: contrato.into(),
        rede: rede.into(),
        gerador_h: gh,
        raiz_aptos: hex(&raiz),
        aptos: enderecos_aptos,
        identidades: nomes_aptos,
        sigilo_minimo: 5,
        mesa: Mesa { membros: enderecos_mesa.len(), limiar, enderecos: enderecos_mesa, identidades: nomes_mesa },
        inicio_ledger: abre_em,
        prazo_ledger: fecha_em,
        abertura_tx: tx,
        votos: vec![],
        aberturas: vec![],
        apuracao: None,
        verificacao: None,
    }
    .gravar()?;

    tela::proximo(
        "Compartilhe com quem vota:",
        &format!("tessera cedula --proposta {} --identidade SEU_NOME", proposta),
    );
    Ok(())
}

fn resolver(identidade: &str) -> Result<String, String> {
    if identidade.starts_with('G') && identidade.len() == 56 {
        return Ok(identidade.to_string());
    }
    Cadeia::endereco(identidade).map_err(|e| e.to_string())
}

// ===================== cedula ========================================

/// **O comando que ganha a demo.** Ele não vota: ele mostra.
pub fn mostrar_cedula(proposta: &str, identidade: &str) -> R {
    let e = Estado::ler(proposta)?;
    let endereco = resolver(identidade)?;
    let h = h_do_contrato(&e)?;
    let pid = id32(proposta);
    let addr = cedula::xdr(&endereco)?;

    let apta = e.indice_do_apto(&endereco).is_some();
    let votou = e.ja_votou(&endereco).is_some();
    let agora = Cadeia::ledger_atual(&e.rede).unwrap_or(0);

    tela::titulo("cédula");
    tela::branco();
    for (q, pg) in e.perguntas.iter().enumerate() {
        tela::linha(&format!("{}. {}", q + 1, pg.texto));
        tela::campo(
            if pg.confidencial { "  em sigilo" } else { "  em aberto" },
            &pg.opcoes.join(" · "),
        );
    }
    tela::branco();
    tela::linha(&format!(
        "{} · você {} · {}",
        janela(e.inicio_ledger, e.prazo_ledger, agora),
        if apta { "é apta" } else { "NÃO está na lista" },
        if votou { "já votou" } else { "ainda não votou" }
    ));

    if !apta {
        return nao_apta(&e);
    }

    // A demonstração do sigilo precisa de uma pergunta sigilosa. Numa cédula
    // inteiramente pública não há o que esconder, e dizer que há seria mentir.
    let Some(q_sig) = e.perguntas.iter().position(|p| p.confidencial) else {
        tela::secao("esta cédula é toda pública");
        tela::linha("Nenhuma pergunta é sigilosa: tudo que você marcar vai em claro");
        tela::linha("para o ledger, ao lado do seu endereço, para sempre.");
        tela::branco();
        tela::regua();
        tela::proximo(
            "Para votar:",
            &format!(
                "tessera votar --proposta {} --opcao {} --identidade {}",
                proposta, e.perguntas[0].opcoes[0], identidade
            ),
        );
        return Ok(());
    };

    // Dois exemplos, com acaso DESCARTÁVEL: o voto real sorteia um `r` novo.
    // As outras perguntas vão em zero; o que importa é o par da sigilosa.
    let esc_a: Vec<usize> = vec![0; e.perguntas.len()];
    let mut esc_b = esc_a.clone();
    esc_b[q_sig] = 1;
    let perg: Vec<cedula::Pergunta> = e
        .perguntas
        .iter()
        .map(|p| cedula::Pergunta { opcoes: p.opcoes.len(), confidencial: p.confidencial })
        .collect();
    let a = cedula::montar(&h, &pid, &addr, &perg, &esc_a)?;
    let b = cedula::montar(&h, &pid, &addr, &perg, &esc_b)?;

    // O primeiro compromisso da pergunta sigilosa escolhida.
    let desloc: usize = e
        .perguntas
        .iter()
        .take(q_sig)
        .filter(|p| p.confidencial)
        .map(|p| p.opcoes.len())
        .sum();

    let pg = &e.perguntas[q_sig];
    tela::secao("o que a rede guardaria, para sempre");
    tela::linha(&format!("Pergunta {}: {}", q_sig + 1, pg.texto));
    tela::branco();
    tela::blocos_lado_a_lado(
        &ponto::serializar(&a.compromissos[desloc]),
        &ponto::serializar(&b.compromissos[desloc]),
        "Se você votar  A",
        "Se você votar  B",
    );
    tela::branco();
    tela::linha(&format!(
        "96 bytes cada. Um é \"{}\" e o outro é \"{}\".",
        pg.opcoes[0], pg.opcoes[1]
    ));
    tela::branco();
    // Esta frase NÃO pode ser suavizada. É a única vez no produto em que um
    // superlativo é literalmente correto, porque a garantia é
    // information-theoretic e não computacional. "Praticamente impossível"
    // seria impreciso E mais fraco. UX-CLI §2.2.
    tela::linha("Não existe cálculo, computador ou tempo que diga qual é qual.");
    tela::linha("Nem hoje, nem em cinquenta anos. É o que você está publicando.");

    tela::secao("seu sigilo");
    // Numa cédula mista, o que vai em claro tem de ser dito antes do voto, não
    // descoberto depois no explorador de blocos.
    if e.e_mista() {
        let publicas: Vec<&str> = e
            .perguntas
            .iter()
            .filter(|p| !p.confidencial)
            .map(|p| p.texto.as_str())
            .collect();
        tela::linha(&format!(
            "Esta cédula é mista. {} vai em claro, ao lado do seu endereço:",
            if publicas.len() == 1 { "Uma pergunta" } else { "Algumas perguntas" }
        ));
        for p in &publicas {
            tela::linha(&format!("  · {}", p));
        }
        tela::linha("O resto fica em sigilo, na mesma transação.");
        tela::branco();
    }
    let conf = e.confidenciais();
    // A regra de τ aqui é previsão, não veredito: quem lê ainda não votou.
    // Dizer "⚠ abaixo do mínimo" para quem seria a primeira é assustar sem
    // informar — o número só vira veredito na apuração.
    if conf == 0 {
        tela::linha(&format!(
            "Ninguém votou em segredo ainda. Você seria a 1ª, e a apuração",
        ));
        tela::linha(&format!(
            "só acontece com {} ou mais — ou com ninguém em segredo.",
            e.sigilo_minimo
        ));
    } else if conf < e.sigilo_minimo {
        tela::linha(&format!(
            "{} de {} em segredo. Faltam {} para a apuração acontecer.",
            conf,
            e.aptos.len(),
            e.sigilo_minimo - conf
        ));
    } else {
        tela::linha(&format!(
            "{} de {} estão em segredo. Mínimo é {}.  ✓ folgado",
            conf,
            e.aptos.len(),
            e.sigilo_minimo
        ));
    }
    tela::branco();
    tela::regua();

    if votou {
        tela::proximo(
            "Você já votou. Para conferir que seu voto está na contagem:",
            &format!("tessera status --proposta {}", proposta),
        );
    } else {
        // Uma `--opcao` por pergunta, na ordem da cédula.
        let exemplo: String = e
            .perguntas
            .iter()
            .map(|p| format!("--opcao {} ", p.opcoes[0]))
            .collect();
        tela::proximo(
            "Para votar:",
            &format!(
                "tessera votar --proposta {} {}--identidade {}",
                proposta, exemplo, identidade
            ),
        );
    }
    Ok(())
}

fn nao_apta(e: &Estado) -> R {
    tela::recusa("esta conta não está na lista de aptos");
    tela::branco();
    tela::campo(
        "Raiz de aptos",
        &format!("{}  ({} aptos)", tela::abreviar(&e.raiz_aptos, 4, 4), e.aptos.len()),
    );
    tela::branco();
    // A última frase não é desculpa, é arquitetura: o módulo não opina sobre
    // aptidão (SPEC §0). Dizer isso no erro ensina o modelo a quem integra.
    tela::linha("Se você deveria estar, fale com quem abriu a votação. Tessera");
    tela::linha("não decide quem vota.");
    tela::branco();
    Ok(())
}

// ===================== votar =========================================

pub fn votar(proposta: &str, opcao: &[String], identidade: &str, publico: bool) -> R {
    let mut e = Estado::ler(proposta)?;
    let endereco = resolver(identidade)?;

    // Uma `--opcao` por pergunta, na ordem da cédula. Faltar uma é recusa, não
    // abstenção silenciosa: abster-se precisaria da sua própria prova.
    if opcao.len() != e.perguntas.len() {
        return Err(format!(
            "a cédula tem {} pergunta{} e vieram {} escolha{}.\n  Passe uma `--opcao` por pergunta, na ordem:\n{}",
            e.perguntas.len(),
            if e.perguntas.len() == 1 { "" } else { "s" },
            opcao.len(),
            if opcao.len() == 1 { "" } else { "s" },
            e.perguntas
                .iter()
                .enumerate()
                .map(|(q, p)| format!("    {}. {}  ({})", q + 1, p.texto, p.opcoes.join(" · ")))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    let escolhas_idx: Vec<usize> = opcao
        .iter()
        .zip(e.perguntas.iter())
        .enumerate()
        .map(|(q, (o, pg))| {
            pg.opcoes.iter().position(|x| x == o).ok_or_else(|| {
                format!(
                    "{:?} não é opção da pergunta {} ({}): {}",
                    o,
                    q + 1,
                    pg.texto,
                    pg.opcoes.join(", ")
                )
            })
        })
        .collect::<Result<_, _>>()?;

    let indice = match e.indice_do_apto(&endereco) {
        Some(i) => i,
        None => return nao_apta(&e),
    };
    if let Some(v) = e.ja_votou(&endereco) {
        return ja_votou(v);
    }

    let folhas: Vec<merkle::Apto> = e
        .aptos
        .iter()
        .map(|a| cedula::xdr(a).map(|x| merkle::Apto { endereco: x, peso: 1 }))
        .collect::<Result<_, _>>()?;
    let arvore = merkle::Arvore::montar(&folhas).map_err(|x| format!("{:?}", x))?;
    let caminho = arvore.caminho(indice).map_err(|x| format!("{:?}", x))?;

    tela::titulo("votando");
    tela::branco();
    // A escolha aparece em caixa alta uma vez, no topo, e nunca mais: repetir
    // a escolha na tela é ensaiar o hábito de deixá-la visível. UX-CLI §3.1.
    // Numa cédula mista, cada linha diz qual natureza ela tem — a pessoa não
    // pode descobrir depois o que foi em claro.
    for (q, pg) in e.perguntas.iter().enumerate() {
        tela::campo(
            &format!("{}. {}", q + 1, pg.texto),
            &format!(
                "{}   [{}]",
                opcao[q].to_uppercase(),
                if publico || !pg.confidencial { "em claro" } else { "em segredo" }
            ),
        );
    }
    tela::campo(
        "Sigilo",
        if publico {
            "PÚBLICO — a cédula inteira vai em claro para o ledger"
        } else if e.e_mista() {
            "misto — as perguntas sigilosas em segredo, as outras em claro"
        } else {
            "em segredo"
        },
    );

    let c = Cadeia::nova(&e.contrato, &e.rede);
    let tx;

    if publico {
        // Revelação voluntária: a cédula INTEIRA em claro, inclusive as
        // perguntas sigilosas.
        let mut escolhas: Vec<u32> = Vec::new();
        for (q, pg) in e.perguntas.iter().enumerate() {
            for j in 0..pg.opcoes.len() {
                escolhas.push(if j == escolhas_idx[q] { 1 } else { 0 });
            }
        }
        tela::secao("o que a rede vai guardar");
        tela::linha(&format!(
            "a escolha em claro: [{}]",
            escolhas.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", ")
        ));
        tela::branco();
        tela::linha("Uma cédula pública não tem o que ocultar — e por isso ela");
        tela::linha("encolhe o conjunto de anonimato de quem escolheu sigilo.");

        let r = c
            .invocar(
                identidade,
                true,
                "votar_publico",
                &[
                    ("proposta", hex(&id32(proposta))),
                    ("votante", endereco.clone()),
                    ("escolhas", serde_json::to_string(&escolhas).unwrap()),
                    ("caminho", cedula::hashes_json(&caminho.irmaos)),
                    ("indice", caminho.indice.to_string()),
                    ("peso", "1".into()),
                ],
            )
            .map_err(|x| x.to_string())?;
        tx = r.tx.unwrap_or_default();
        registrar(
            &mut e, proposta, identidade, &endereco, vec![], vec![], vec![],
            true, Some(escolhas), &tx, &c,
        )?;
    } else {
        let h = h_do_contrato(&e)?;
        let addr = cedula::xdr(&endereco)?;
        let perg: Vec<cedula::Pergunta> = e
            .perguntas
            .iter()
            .map(|p| cedula::Pergunta { opcoes: p.opcoes.len(), confidencial: p.confidencial })
            .collect();
        let ced: Cedula = cedula::montar(&h, &id32(proposta), &addr, &perg, &escolhas_idx)?;

        tela::secao("o que a rede vai guardar");
        if ced.compromissos.is_empty() {
            tela::linha("esta cédula não tem pergunta sigilosa: tudo vai em claro");
        } else {
            // O compromisso da primeira pergunta sigilosa — o que a pessoa vê
            // é o que o ledger recebe.
            tela::bloco_hex(&ponto::serializar(&ced.compromissos[0]));
        }

        let r = c
            .invocar(
                identidade,
                true,
                "votar",
                &[
                    ("proposta", hex(&id32(proposta))),
                    ("votante", endereco.clone()),
                    ("compromissos", cedula::pontos_json(&ced.compromissos)),
                    ("provas", cedula::provas_json(&ced.provas)),
                    ("provas_soma", cedula::somas_json(&ced.somas)),
                    ("escolhas", cedula::escolhas_json(&ced.publicas)),
                    ("caminho", cedula::hashes_json(&caminho.irmaos)),
                    ("indice", caminho.indice.to_string()),
                    ("peso", "1".into()),
                ],
            )
            .map_err(|x| x.to_string())?;
        tx = r.tx.unwrap_or_default();
        let compromissos_hex: Vec<String> = ced.compromissos.iter().map(ponto::para_hex).collect();

        // As shares vão para a mesa. Nenhum lugar reúne os `r` de uma pessoa.
        let por_membro = cedula::dividir_para_a_mesa(&ced.acasos, e.mesa.limiar, e.mesa.membros)?;
        entregar_shares(proposta, &por_membro)?;

        let publicas = if ced.publicas.is_empty() { None } else { Some(ced.publicas.clone()) };
        registrar(
            &mut e, proposta, identidade, &endereco, compromissos_hex,
            ced.provas.iter().map(|p| hex(&p.serializar())).collect(),
            ced.somas.iter().map(|p| hex(&p.serializar())).collect(),
            false, publicas, &tx, &c,
        )?;

        // No recibo vai **só o segredo**. As provas são públicas e ficam no
        // estado, para que queimar o recibo não custe auditabilidade.
        recibo::gravar(&Recibo {
            identidade: identidade.into(),
            proposta: proposta.into(),
            escolhas: escolhas_idx.clone(),
            acasos: ced.acasos.iter().map(|r| hex(&pedersen::fr_para_bytes_be(r))).collect(),
        })?;
    }

    let (taxa, ledger) = c.detalhes(&tx).unwrap_or((0, 0));
    let e = Estado::ler(proposta)?;
    let posicao = e.votos.len();
    let conf = e.confidenciais();

    tela::secao("enviado");
    tela::campo("Transação", &format!("{}       ledger {}", &tx[..8.min(tx.len())], ledger));
    tela::campo("Taxa", &format!("{} stroops", taxa));
    if !publico {
        // Verificabilidade individual numa frase que Dona Marta entende: não é
        // "seu commitment está no acumulador", é "você é a 27ª de 43".
        tela::campo("Posição", &format!("{} de {} em segredo", posicao, conf));
    }
    tela::confere("seu voto está na contagem");
    tela::branco();
    tela::regua();

    if publico {
        tela::proximo(
            "Sua escolha é pública e não há recibo a queimar. Para conferir:",
            &format!("tessera status --proposta {}", proposta),
        );
        return Ok(());
    }

    // O aviso de coação é o último bloco e o maior. A última coisa na tela é a
    // que fica na memória e a que sobra no scroll — é a decisão de design mais
    // importante deste comando. UX-CLI §3.1, UX §5.2.
    //
    // E não é vermelho: vermelho é da apuração recusada. Aqui não é erro, é a
    // verdade sobre o estado do mundo.
    tela::atencao(&format!(
        "A CHAVE EM ./recibos/{}.key PROVA O SEU VOTO",
        identidade
    ));
    tela::branco();
    tela::linha("   Enquanto ela existir, você consegue provar a qualquer pessoa");
    tela::linha("   em que votou — e quem te obrigar a mostrar consegue conferir.");
    tela::branco();
    tela::linha("   A rede nunca vai saber. Mas você pode ser forçada a contar.");
    tela::proximo(
        "   Apague agora:",
        &format!("  tessera queimar --identidade {}", identidade),
    );
    Ok(())
}

fn ja_votou(v: &Voto) -> R {
    // Sem tom de reprimenda: o erro mais comum é a pessoa não ter certeza se o
    // voto entrou, então a mensagem responde essa pergunta. UX-CLI §8.3.
    tela::recusa("você já votou nesta proposta");
    tela::branco();
    tela::linha(&format!(
        "Voto registrado no ledger {}, posição {}.",
        v.ledger, v.posicao
    ));
    tela::linha("Uma pessoa apta vota uma vez. Isso é conferível por qualquer um.");
    tela::branco();
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn registrar(
    e: &mut Estado,
    proposta: &str,
    identidade: &str,
    endereco: &str,
    compromissos: Vec<String>,
    provas: Vec<String>,
    provas_soma: Vec<String>,
    publico: bool,
    escolhas: Option<Vec<u32>>,
    tx: &str,
    c: &Cadeia,
) -> R {
    let (_, ledger) = c.detalhes(tx).unwrap_or((0, 0));
    e.votos.push(Voto {
        posicao: e.votos.len() + 1,
        identidade: identidade.into(),
        endereco: endereco.into(),
        compromissos,
        provas,
        provas_soma,
        publico,
        escolhas,
        tx: tx.into(),
        ledger,
    });
    let _ = proposta;
    e.gravar()
}

/// As shares viajariam por canal autenticado e cifrado até cada membro. Aqui
/// vão para `./shares/<membro>/`, que é a simulação honesta: o que importa é
/// que nenhum lugar reúne os `r` de uma pessoa.
fn entregar_shares(proposta: &str, por_membro: &[Vec<String>]) -> R {
    for (l, shares) in por_membro.iter().enumerate() {
        let dir = std::path::PathBuf::from("shares").join((l + 1).to_string());
        std::fs::create_dir_all(&dir).map_err(|x| x.to_string())?;
        let p = dir.join(format!("{}.jsonl", proposta));
        let linha = serde_json::to_string(shares).map_err(|x| x.to_string())?;
        let mut s = std::fs::read_to_string(&p).unwrap_or_default();
        s.push_str(&linha);
        s.push('\n');
        std::fs::write(&p, s).map_err(|x| x.to_string())?;
    }
    Ok(())
}

// ===================== queimar =======================================

/// A tela mais curta e a de maior carga emocional do produto.
pub fn queimar(identidade: &str) -> R {
    tela::titulo("queimando o recibo");
    tela::branco();
    let feitos = recibo::queimar(identidade)?;
    if feitos.is_empty() {
        tela::linha(&format!(
            "Não há recibo em ./recibos/{}.key — nada a queimar.",
            identidade
        ));
        tela::branco();
        return Ok(());
    }
    for f in &feitos {
        let vago = tela::LARGURA.saturating_sub(f.chars().count() + 11);
        tela::linha(&format!("{}{}{}", f, " ".repeat(vago), "sobrescrito"));
    }
    tela::confere("pronto");
    tela::branco();
    // A frase que o Tessera inteiro existe para poder dizer. Não acrescente
    // nada em volta dela.
    tela::linha("Agora nem você consegue provar em que votou.");
    tela::branco();
    // E esta é indispensável: sem ela a ação assusta e ninguém a executa.
    tela::linha("Seu voto continua na contagem, e você continua podendo conferir");
    tela::linha("que ele está lá.");
    tela::branco();
    Ok(())
}

// ===================== status ========================================

/// Dois números, grandes, centrados. A única tela que alguém olha de longe.
pub fn status(proposta: &str) -> R {
    let e = Estado::ler(proposta)?;
    let agora = Cadeia::ledger_atual(&e.rede).unwrap_or(0);
    let (conf, publ) = (e.confidenciais(), e.publicos());

    tela::titulo_com(proposta, &janela(e.inicio_ledger, e.prazo_ledger, agora).to_lowercase());
    tela::branco();
    let esq = format!("{} de {} votaram", e.votos.len(), e.aptos.len());
    tela::centrado(&format!("{}            {} em segredo", esq, conf));
    tela::centrado(&format!("{}{} públicos", " ".repeat(esq.chars().count() + 13), publ));
    tela::branco();

    let pode = conf == 0 || conf >= e.sigilo_minimo;
    tela::campo_veredito(
        "Sigilo mínimo",
        &format!("{}", e.sigilo_minimo),
        pode,
    );
    tela::campo(
        "Mesa",
        &format!("{} de {} assinaturas para apurar", e.mesa.limiar, e.mesa.membros),
    );
    tela::branco();
    tela::regua();

    if let Some(a) = &e.apuracao {
        tela::proximo(
            &format!("Já apurada: {:?}. Conferir por conta própria:", a.afirmado),
            &format!("tessera verificar --proposta {}", proposta),
        );
    } else if agora >= e.prazo_ledger {
        tela::proximo("Apurar:", &format!("tessera apurar --proposta {}", proposta));
    } else {
        tela::proximo(
            "Enquanto está aberta:",
            &format!("tessera cedula --proposta {} --identidade SEU_NOME", proposta),
        );
    }
    Ok(())
}

// ===================== apurar ========================================

pub fn apurar(proposta: &str, forcar: Option<&str>) -> R {
    let mut e = Estado::ler(proposta)?;
    let h = h_do_contrato(&e)?;
    let g = pedersen::gerador();
    let c = Cadeia::nova(&e.contrato, &e.rede);

    tela::titulo("apurando");
    tela::branco();

    // 1. a mesa soma localmente as shares que recebeu, e k membros reconstroem.
    //    Só as perguntas sigilosas têm abertura: as públicas já estão em claro.
    let n_conf = e.opcoes_confidenciais();
    if n_conf == 0 {
        return Err("esta cédula é toda pública: não há abertura a reconstruir.".into());
    }
    let somas = ler_shares(proposta, n_conf, e.mesa.membros)?;
    let aberturas = cedula::reconstruir_aberturas(&somas, e.mesa.limiar, n_conf)?;
    tela::campo(
        "Mesa",
        &format!("{} de {} reconstruíram a abertura agregada", e.mesa.limiar, e.mesa.membros),
    );
    let conf = e.confidenciais();
    tela::campo_veredito(
        "Sigilo",
        &format!("{} confidenciais, mínimo {}", conf, e.sigilo_minimo),
        conf == 0 || conf >= e.sigilo_minimo,
    );

    // 2. a mesa PROCURA o total. Fora da cadeia procurar é de graça; é a outra
    //    metade da descoberta da sonda 5.
    let acumuladores = acumuladores_da_cadeia(&c, proposta, n_conf)?;
    let rotulos_conf = rotulos_confidenciais(&e);
    let mut totais = Vec::new();
    for j in 0..n_conf {
        let t = pedersen::descobrir_total(&acumuladores[j], &g, &h, &aberturas[j], e.aptos.len() as u64)
            .ok_or_else(|| {
                format!(
                    "a mesa não achou total nenhum para {:?}: as shares não abrem o acumulador",
                    rotulos_conf[j]
                )
            })?;
        totais.push(t as u32);
    }

    // `--forcar-total` existe para gravar a recusa. É a mesa mentindo.
    if let Some(f) = forcar {
        totais = f
            .split(',')
            .map(|s| s.trim().parse::<u32>().map_err(|x| x.to_string()))
            .collect::<Result<_, _>>()?;
        if totais.len() != n_conf {
            return Err(format!(
                "--forcar-total precisa de um número por opção sigilosa, e são {}",
                n_conf
            ));
        }
    }

    tela::secao("a mesa afirmou");
    for (j, o) in rotulos_conf.iter().enumerate() {
        tela::campo(&o.to_uppercase(), &totais[j].to_string());
    }

    // 3. o contrato confere. Esta ordem — a mesa afirmou → o contrato conferiu
    //    → o resultado — é a arquitetura de confiança impressa de cima para
    //    baixo, e ensina sozinha por que não é preciso confiar na mesa.
    tela::secao("o contrato conferiu");
    let mut fecha_local = true;
    for j in 0..n_conf {
        let ok = pedersen::verifica_agregado(&acumuladores[j], &g, &h, totais[j] as u64, &aberturas[j]);
        fecha_local &= ok;
        tela::campo_veredito(
            &format!("Acumulado {}", rotulos_conf[j].to_uppercase()),
            &format!("{}·G + R·H", totais[j]),
            ok,
        );
    }

    // **Um endosso por membro, uma transação por pessoa.**
    //
    // `k` autorizações numa transação só não é expressável pelo ferramental da
    // Stellar: `stellar tx sign` assina o envelope, não as entradas de
    // autorização do Soroban, e a rede recusa com `TxBadAuthExtra`. Cada
    // membro manda a sua, e o contrato conta — que é também como `k` pessoas
    // em `k` máquinas realmente trabalham.
    let aberturas_dec: Vec<String> = aberturas.iter().map(cedula::dec_escalar).collect();
    let args = [
        ("proposta", hex(&id32(proposta))),
        ("totais", serde_json::to_string(&totais).unwrap()),
        ("aberturas", serde_json::to_string(&aberturas_dec).unwrap()),
    ];

    let mut envio = Ok(crate::cadeia::Resposta { valor: String::new(), tx: None });
    let mut fechou = false;
    let mut abriu_secao = false;
    for (i, membro) in e.mesa.identidades.iter().take(e.mesa.limiar as usize).enumerate() {
        let mut com_membro = args.to_vec();
        com_membro.push(("membro", e.mesa.enderecos[i].clone()));
        envio = c.invocar(membro, true, "apurar", &com_membro);
        match &envio {
            Err(x) => {
                return recusada(&x.to_string(), conf, e.sigilo_minimo, &totais, fecha_local)
            }
            Ok(r) => {
                if !abriu_secao {
                    tela::secao("a mesa endossa");
                    abriu_secao = true;
                }
                fechou = !r.valor.trim().is_empty() && r.valor.trim() != "null";
                tela::campo(
                    membro,
                    &format!(
                        "endossou  ({} de {})",
                        i + 1,
                        e.mesa.limiar
                    ),
                );
            }
        }
    }
    let _ = fechou;

    match envio {
        Err(x) => recusada(&x.to_string(), conf, e.sigilo_minimo, &totais, fecha_local),
        Ok(r) => {
            let tx = r.tx.unwrap_or_default();
            let (taxa, ledger) = c.detalhes(&tx).unwrap_or((0, 0));
            tela::branco();
            tela::campo("Transação", &format!("{}       ledger {}", &tx[..8.min(tx.len())], ledger));
            tela::campo("Taxa", &format!("{} stroops", taxa));
            tela::branco();
            tela::regua();

            // O resultado vem DEPOIS da conferência e é a única linha centrada:
            // a ordem comunica que o número só vale porque passou pelo bloco
            // acima.
            let resultado = resultado_local(&e, &totais);
            let mut off = 0usize;
            for (q, pg) in e.perguntas.iter().enumerate() {
                let fatia = &resultado[off..off + pg.opcoes.len()];
                let vencedora = fatia
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, t)| **t)
                    .map(|(j, _)| j)
                    .unwrap();
                // O placar sai da vencedora para baixo, não na ordem das
                // opções: "REJEITAR · 3 a 4" leria o número errado como o dela.
                let mut placar: Vec<u32> = fatia.to_vec();
                placar.sort_unstable_by(|a, b| b.cmp(a));
                if e.perguntas.len() > 1 {
                    tela::linha(&format!("{}. {}", q + 1, pg.texto));
                }
                tela::centrado_grande(&format!(
                    "{} · {}",
                    pg.opcoes[vencedora].to_uppercase(),
                    placar.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(" a ")
                ));
                off += pg.opcoes.len();
            }

            e.apuracao = Some(Apuracao { afirmado: totais, confere: true, tx, ledger });
            e.aberturas = aberturas.iter().map(cedula::dec_escalar).collect();
            e.gravar()?;

            tela::proximo(
                "Conferir por conta própria:",
                &format!("tessera verificar --proposta {}", proposta),
            );
            Ok(())
        }
    }
}

fn recusada(erro: &str, conf: u32, minimo: u32, totais: &[u32], fecha_local: bool) -> R {
    tela::recusa("apuração recusada · nada foi publicado");
    tela::branco();
    if erro.contains("AnonimatoInsuficiente") {
        // O estado que mais impressiona quem entende de votação: o produto
        // recusando um resultado que PODERIA publicar, para proteger quem
        // está em minoria. UX-CLI §8.2.
        tela::linha(&format!(
            "Com apenas {} votos em segredo, publicar o total revelaria",
            conf
        ));
        tela::linha("esses votos por subtração: quem publicou é conhecido, e o");
        tela::linha("resto sai por diferença.");
        tela::branco();
        // Obrigatória: sem ela, a recusa parece perda de dados.
        tela::linha("A apuração volta a ser possível se mais pessoas votarem em");
        tela::linha("segredo. Nenhum voto foi perdido.");
        let _ = minimo;
    } else if erro.contains("AberturaNaoFecha") || !fecha_local {
        tela::linha("O total afirmado não corresponde aos compromissos no ledger.");
        tela::linha(&format!(
            "Para publicar {} a mesa teria de resolver um log discreto",
            totais.first().copied().unwrap_or(0)
        ));
        tela::linha("em BLS12-381.");
        tela::branco();
        // A tese da verificabilidade em oito palavras.
        tela::linha("Não é denúncia depois. É recusa na hora.");
    } else {
        tela::linha(erro);
    }
    tela::branco();
    Ok(())
}

/// Cada membro soma **localmente** as shares que recebeu. Esta função é o que
/// cada um rodaria na própria máquina; aqui ela lê os diretórios porque a
/// entrega é simulada, mas a aritmética é a mesma.
fn ler_shares(proposta: &str, opcoes: usize, membros: usize) -> Result<Vec<(u32, Vec<Fr>)>, String> {
    let mut saida = Vec::new();
    for l in 1..=membros {
        let p = std::path::PathBuf::from("shares").join(l.to_string()).join(format!("{}.jsonl", proposta));
        let Ok(s) = std::fs::read_to_string(&p) else { continue };
        let mut soma = vec![pedersen::escalar(0); opcoes];
        for linha in s.lines().filter(|l| !l.trim().is_empty()) {
            let v: Vec<String> = serde_json::from_str(linha).map_err(|e| e.to_string())?;
            for (j, dec) in v.iter().enumerate().take(opcoes) {
                soma[j] += decimal_para_fr(dec)?;
            }
        }
        saida.push((l as u32, soma));
    }
    Ok(saida)
}

fn decimal_para_fr(s: &str) -> Result<Fr, String> {
    use std::str::FromStr;
    Fr::from_str(s).map_err(|_| format!("escalar decimal inválido: {}", s))
}

fn acumuladores_da_cadeia(c: &Cadeia, proposta: &str, opcoes: usize) -> Result<Vec<G1Affine>, String> {
    let r = c
        .invocar("urna-smoke", false, "acumulador", &[("proposta", hex(&id32(proposta)))])
        .map_err(|e| e.to_string())?;
    let v: Vec<String> = serde_json::from_str(&r.valor)
        .map_err(|e| format!("não entendi os acumuladores: {} ({})", e, r.valor))?;
    if v.len() != opcoes {
        return Err(format!("o contrato devolveu {} acumuladores e a proposta tem {} opções", v.len(), opcoes));
    }
    v.iter()
        .map(|s| ponto::de_hex(s).map_err(|e| format!("acumulador ilegível: {:?}", e)))
        .collect()
}

// ===================== verificar =====================================

/// A tela do cético, e do jurado. **Não fala com a CLI: relê o ledger.**
pub fn verificar(proposta: &str) -> R {
    let mut e = Estado::ler(proposta)?;
    let g = pedersen::gerador();
    let c = Cadeia::nova(&e.contrato, &e.rede);
    let h = h_do_contrato(&e)?;

    tela::titulo("verificador independente");
    tela::branco();
    tela::campo("Fonte", &format!("contrato {} na {}", tela::abreviar(&e.contrato, 8, 4), e.rede));
    // Honesto: o que o verificador confia é o conjunto de compromissos
    // individuais do arquivo local — e mesmo esse está preso pela soma, que
    // vem da cadeia. Tudo o mais é lido do contrato.
    tela::campo("Confia em", "nada que a CLI gravou e ela não possa provar");
    tela::branco();

    // 1. a raiz de aptos, recalculada da lista
    let folhas: Vec<merkle::Apto> = e
        .aptos
        .iter()
        .map(|a| cedula::xdr(a).map(|x| merkle::Apto { endereco: x, peso: 1 }))
        .collect::<Result<_, _>>()?;
    let arvore = merkle::Arvore::montar(&folhas).map_err(|x| format!("{:?}", x))?;
    let raiz_local = hex(&arvore.raiz());
    let prop = c
        .invocar("urna-smoke", false, "proposta", &[("proposta", hex(&id32(proposta)))])
        .map_err(|x| x.to_string())?;
    let raiz_na_cadeia = prop.valor.split("\"raiz_aptos\":\"").nth(1).and_then(|s| s.split('"').next()).unwrap_or("").to_string();
    let aptidao = raiz_local == raiz_na_cadeia && !raiz_na_cadeia.is_empty();
    tela::campo_veredito(
        "Aptidão",
        &format!("{} de {} na raiz {}", e.aptos.len(), e.aptos.len(), tela::abreviar(&raiz_local, 4, 4)),
        aptidao,
    );

    // 2. unicidade
    let mut vistos = std::collections::HashSet::new();
    let repetidos = e.votos.iter().filter(|v| !vistos.insert(&v.endereco)).count();
    tela::campo_veredito("Unicidade", &format!("{} repetidos", repetidos), repetidos == 0);

    // 3. boa formação: cada prova CDS e cada prova de soma, reverificadas do
    //    zero — **por pergunta**, com o contexto que amarra a pergunta.
    let n_conf = e.opcoes_confidenciais();
    let n_perg_conf = e.perguntas_confidenciais();
    let mut boa = true;
    let (mut checadas, mut somas_ok, mut somas_total, mut cedulas) = (0usize, 0usize, 0usize, 0usize);
    for v in e.votos.iter().filter(|v| !v.publico) {
        if v.provas.is_empty() {
            continue;
        }
        let addr = cedula::xdr(&v.endereco)?;
        cedulas += 1;
        if v.provas.len() != n_conf || v.provas_soma.len() != n_perg_conf {
            boa = false;
            continue;
        }

        let (mut off, mut i_soma) = (0usize, 0usize);
        for (q, pg) in e.perguntas.iter().enumerate() {
            if !pg.confidencial {
                continue;
            }
            // cada v_j ∈ {0,1}, nesta pergunta
            let mut cs = Vec::with_capacity(pg.opcoes.len());
            for j in 0..pg.opcoes.len() {
                let cj = ponto::de_hex(&v.compromissos[off + j]).map_err(|x| format!("{:?}", x))?;
                match cds::Prova::desserializar(&de_hex(&v.provas[off + j])?) {
                    Ok(p) => {
                        if cds::verificar(
                            &cedula::contexto(&id32(proposta), &addr, q as u32, j as u32),
                            &g, &h, &cj, &p,
                        ) {
                            checadas += 1;
                        } else {
                            boa = false;
                        }
                    }
                    Err(_) => boa = false,
                }
                cs.push(cj);
            }

            // e Σ v_j = 1 **nesta pergunta**. As duas provas são necessárias e
            // nenhuma é suficiente: sem esta, v = (3, −2) passaria pelas
            // disjuntivas de cima. E se a soma fosse uma só para a cédula
            // inteira, responder uma pergunta obrigaria a abster-se das outras.
            somas_total += 1;
            match tessera_core::soma::Prova::desserializar(&de_hex(&v.provas_soma[i_soma])?) {
                Ok(p) => {
                    let d = tessera_core::soma::alvo(&g, &cs, 1);
                    if tessera_core::soma::verificar(
                        &cedula::contexto(&id32(proposta), &addr, q as u32, cedula::OPCAO_DA_SOMA),
                        &h, &d, &p,
                    ) {
                        somas_ok += 1;
                    } else {
                        boa = false;
                    }
                }
                Err(_) => boa = false,
            }
            off += pg.opcoes.len();
            i_soma += 1;
        }
    }
    tela::campo_veredito("Boa formação", &format!("{} disjuntivas válidas", checadas), boa);
    tela::campo_veredito(
        "Soma por pergunta",
        &format!("{} de {} somam 1", somas_ok, somas_total),
        somas_ok == somas_total && cedulas > 0,
    );

    // 4. as respostas em claro: estão no ledger, basta conferir pergunta a
    //    pergunta. Vale para as duas origens — quem abriu a cédula inteira e
    //    quem respondeu em claro só as perguntas públicas.
    let claro_ok = e.votos.iter().all(|v| {
        let Some(esc) = v.escolhas.as_ref() else { return true };
        let mut off = 0usize;
        for pg in e.perguntas.iter() {
            if !v.publico && pg.confidencial {
                continue;
            }
            let fatia = &esc[off..(off + pg.opcoes.len()).min(esc.len())];
            if fatia.len() != pg.opcoes.len()
                || fatia.iter().any(|x| *x > 1)
                || fatia.iter().sum::<u32>() != 1
            {
                return false;
            }
            off += pg.opcoes.len();
        }
        true
    });
    let com_claro = e.votos.iter().filter(|v| v.escolhas.is_some()).count();
    tela::campo_veredito(
        "Respostas em claro",
        &format!("{} de {} conferem", com_claro, com_claro),
        claro_ok,
    );

    // 5. o limiar de anonimato
    let conf = e.confidenciais();
    let tau = conf == 0 || conf >= e.sigilo_minimo;
    tela::campo_veredito(
        "Sigilo mínimo",
        &format!("{} confidenciais ≥ {}", conf, e.sigilo_minimo),
        tau,
    );

    // 6. o agregado, refeito dos compromissos e conferido contra a cadeia
    let acumuladores = acumuladores_da_cadeia(&c, proposta, n_conf)?;
    let rotulos_conf = rotulos_confidenciais(&e);
    let apuracao = e.apuracao.clone();
    let mut agregado = true;
    tela::branco();
    for j in 0..n_conf {
        let meus: Vec<G1Affine> = e
            .votos
            .iter()
            .filter(|v| !v.publico)
            .map(|v| ponto::de_hex(&v.compromissos[j]).map_err(|x| format!("{:?}", x)))
            .collect::<Result<_, _>>()?;
        let soma_local = pedersen::agregar(&meus);
        let mesmo = soma_local == acumuladores[j];

        let mut abre = true;
        if let (Some(a), false) = (&apuracao, e.aberturas.is_empty()) {
            let r = decimal_para_fr(&e.aberturas[j])?;
            abre = pedersen::verifica_agregado(&acumuladores[j], &g, &h, a.afirmado[j] as u64, &r);
        }
        agregado &= mesmo && abre;
        let rotulo = if j == 0 { "Agregado refeito" } else { "" };
        tela::campo_veredito(
            rotulo,
            &format!(
                "{:<9} {}·G + R·H",
                rotulos_conf[j].to_uppercase(),
                apuracao.as_ref().map(|a| a.afirmado[j]).unwrap_or(0)
            ),
            mesmo && abre,
        );
    }

    tela::branco();
    tela::regua();
    let tudo = aptidao && repetidos == 0 && boa && claro_ok && tau && agregado;
    if tudo && apuracao.is_some() {
        tela::confere("o resultado publicado é o resultado correto");
    } else if tudo {
        tela::confere("tudo o que já existe confere · ainda não apurada");
    } else {
        tela::recusa("alguma coisa não fecha · veja as linhas acima");
    }

    tela::branco();
    // O bloco que resolve o problema de design: o valor é uma ausência, e aqui
    // a ausência vira saída afirmativa de uma ferramenta hostil. Não é o
    // produto prometendo sigilo — é um auditor que tentou tudo, provou o que
    // podia, e relata o que não conseguiu.
    tela::linha("E o que este verificador NÃO conseguiu descobrir:");
    tela::linha(&format!(
        "  em que cada uma das {} pessoas votou.",
        conf
    ));
    tela::branco();

    e.verificacao = Some(Verificacao {
        aptidao,
        unicidade: repetidos == 0,
        boa_formacao: boa,
        aberturas: claro_ok,
        sigilo_minimo: tau,
        agregado,
    });
    e.gravar()?;
    Ok(())
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn o_id_da_proposta_e_estavel_e_separado_por_dominio() {
        let a = id32("contas-2025");
        assert_eq!(a, id32("contas-2025"));
        assert_ne!(a, id32("contas-2026"));
        // e não é o sha256 cru do nome: o DST impede colisão com outro
        // sistema que hasheie o mesmo texto
        let cru: [u8; 32] = Sha256::digest(b"contas-2025").into();
        assert_ne!(a, cru);
    }

    #[test]
    fn prazo_aceita_relogio_e_ledgers() {
        assert_eq!(prazo_em_ledgers("2h").unwrap(), 1440);
        assert_eq!(prazo_em_ledgers("30m").unwrap(), 360);
        assert_eq!(prazo_em_ledgers("100").unwrap(), 100);
        assert!(prazo_em_ledgers("amanhã").is_err());
    }

    #[test]
    fn relogio_nunca_mente_sobre_votacao_encerrada() {
        assert_eq!(relogio(0), "encerrada");
        assert_eq!(relogio(-50), "encerrada");
        assert_eq!(relogio(1440), "em 2h00");
        assert_eq!(relogio(360), "em 30min");
    }

    #[test]
    fn lista_aceita_virgula_e_arquivo() {
        assert_eq!(lista("a, b ,c").unwrap(), vec!["a", "b", "c"]);
        let p = std::env::temp_dir().join(format!("tessera-lista-{}.txt", std::process::id()));
        std::fs::write(&p, "marta\n# comentário\njoao   # inline\n\n").unwrap();
        assert_eq!(
            lista(p.to_str().unwrap()).unwrap(),
            vec!["marta", "joao"]
        );
        let _ = std::fs::remove_file(&p);
    }
}
