import {
  Address,
  Contract,
  Networks,
  TransactionBuilder,
  BASE_FEE,
  rpc,
  xdr,
  SorobanDataBuilder,
  scValToNative,
  nativeToScVal,
} from "@stellar/stellar-sdk";
import type { Carteira } from "./carteira";
import type { Anotar } from "./diario";

export const REDE = {
  rpc: "https://soroban-testnet.stellar.org",
  horizon: "https://horizon-testnet.stellar.org",
  friendbot: "https://friendbot.stellar.org",
  passphrase: Networks.TESTNET,
  contrato: "CBAFYF5BCTIVPUUO67Y76GF2T2GKYD5JDFLXLBUYRC4QLPN5GZXRWN56",
  explorer: "https://stellar.expert/explorer/testnet",
};

/**
 * O lance de inclusão, em stroops.
 *
 * `BASE_FEE` são 100, e 100 perde o leilão assim que várias cédulas disputam o
 * mesmo ledger — foi o que derrubou 9 de 23 numa rodada de carga
 * (`scripts/rodada-30.mjs`), com `txInsufficientFee`.
 *
 * Subir o lance é de graça: a rede cobra o mínimo necessário, não o que você
 * ofereceu. Medido na testnet, a mesma transação com lance de 100 e de
 * 1.000.000 foi cobrada idênticos 19.690.096 stroops. Ao lado de uma taxa de
 * recurso de ~5 milhões por cédula, ser mesquinho aqui nunca economizou nada.
 */
const TAXA_INCLUSAO = 1_000_000;
/** Quantas vezes insistir, multiplicando o lance por 4 a cada vez. */
const LANCES = 3;

/**
 * Stroops a declarar por byte de escrita a mais. O excedente volta: medido, a
 * mesma chamada custou 3.679.157 sem folga e 3.682.659 com — 0,1% de
 * diferença no que sai da conta, contra 1 de 15 virando 15 de 15.
 */
const STROOPS_POR_BYTE = 400n;

/**
 * Espaço de escrita a declarar além do que a simulação pediu.
 *
 * `comparecer()` faz read-modify-write em `Chave::Anel`, uma entrada que cresce
 * 96 bytes por pessoa. A simulação declara `writeBytes` para o anel **de
 * agora** — e quem aplicar depois precisa gravar um anel maior do que declarou.
 * Com duas pessoas comparecendo no mesmo instante, uma passa e a outra é
 * recusada com `txFailed`, sem dizer por quê.
 *
 * O footprint é uma declaração, não uma medição: declarar espaço para o anel
 * cheio desde a primeira faz a declaração continuar válida. Medido na testnet,
 * 15 comparecimentos simultâneos: 1 de 15 sem isto, 15 de 15 num ledger só com.
 */
const folgaDoCaderno = (aptos: number) => 96 * Math.max(aptos, 1) + 1024;

const servidor = new rpc.Server(REDE.rpc);
const contrato = new Contract(REDE.contrato);

/** Quem escreve no diário. A loja fica em `src/diario.ts`, porque o diário
 *  precisa atravessar janelas e esta camada não sabe nada de janelas. */
export type Diario = Anotar;

/**
 * Os erros do contrato, por número.
 *
 * Espelha `contrato/src/tipos.rs`. Sem isto, a recusa mais importante da demo
 * inteira — a segunda cédula da mesma pessoa — aparece na tela como
 * `Error(Contract, #30)`, que não diz nada a ninguém.
 */
const ERROS: Record<number, string> = {
  1: "PropostaJaExiste", 2: "PropostaNaoExiste", 3: "OpcoesForaDaFaixa",
  4: "LimiarInvalido", 5: "PrazoNoPassado", 6: "VotacaoEncerrada",
  7: "VotacaoAindaAberta", 8: "JaVotou", 9: "NaoEstaNaListaDeAptos",
  10: "PesoNaoUnitario", 11: "ProvaBinariaInvalida", 12: "ProvaDeSomaInvalida",
  13: "PontoForaDoSubgrupo", 14: "AberturaNaoFecha", 15: "JaApurada",
  16: "MesaAbaixoDoLimiar", 17: "NaoEMembroDaMesa", 18: "MembroRepetido",
  19: "AnonimatoInsuficiente", 20: "ArgumentoMalFormado", 21: "EscolhaForaDoBinario",
  22: "SomaDiferenteDoPeso", 23: "TotalDiferenteDoComparecimento", 24: "MembroJaEndossou",
  25: "PerguntasForaDaFaixa", 26: "VotacaoAindaNaoComecou", 27: "ComparecimentoEncerrado",
  28: "JaCompareceu", 29: "AnelInvalido", 30: "ImagemJaUsada", 31: "ModoErrado",
};

/** Arredondar para inteiro faz uma chamada barata ler "0% do teto", que soa
 *  como ausência de medida em vez de medida pequena. */
function porcento(cpu: number): string {
  const p = (cpu / 400_000_000) * 100;
  return p < 1 ? `${p.toFixed(2).replace(".", ",")}%` : `${Math.round(p)}%`;
}

/** Dá nome ao número, e deixa o resto intacto. */
export function traduzir(bruto: string): string {
  return bruto.replace(/Error\(Contract, #(\d+)\)/g, (todo, n) =>
    ERROS[Number(n)] ? `${ERROS[Number(n)]} (#${n})` : todo,
  );
}

/**
 * O mesmo erro, do tamanho de uma linha.
 *
 * O `HostError` do SDK traz o log de diagnóstico inteiro grudado na mensagem —
 * cada argumento da chamada em hexadecimal, o que para `votar_anonimo` passa de
 * mil caracteres. Numa tela de erro isso esconde a frase que importa, e nos
 * bastidores enterra a rodada inteira. O despejo continua inteiro no console.
 */
export function resumir(bruto: string): string {
  const inteiro = traduzir(bruto);
  const corte = inteiro.search(/\n\s*Event log/);
  if (corte < 0) return inteiro;
  return `${inteiro.slice(0, corte).trim()} · log de diagnóstico no console`;
}

// ---------- travessia de tipos ----------

export const hexParaBytes = (h: string): Uint8Array =>
  Uint8Array.from(h.match(/.{2}/g)!.map((b) => parseInt(b, 16)));

export const bytesParaHex = (b: Uint8Array): string =>
  Array.from(b, (x) => x.toString(16).padStart(2, "0")).join("");

const bytesN = (h: string) => xdr.ScVal.scvBytes(hexParaBytes(h));
const ponto = (h: string) => bytesN(h);

/**
 * `Bls12381Fr` é `U256` no contrato, **não** `BytesN<32>`.
 *
 * Mandar bytes faz o wrapper do `#[contractimpl]` falhar ao desserializar — e
 * ele não devolve erro, ele **trapa**, com `UnreachableCodeReached`, antes de
 * uma linha do contrato rodar. O erro não diz qual argumento está errado nem
 * que o problema é de tipo.
 */
const escalar = (h: string) => nativeToScVal(BigInt("0x" + h), { type: "u256" });
const u32 = (n: number) => xdr.ScVal.scvU32(n);
const vetor = (v: xdr.ScVal[]) => xdr.ScVal.scvVec(v);
const endereco = (g: string) => new Address(g).toScVal();

/**
 * O XDR de um endereço, que é o que entra na folha de Merkle.
 *
 * Tem de ser byte a byte igual ao `Address::to_xdr` do contrato, senão a raiz
 * não bate e ninguém prova aptidão.
 */
export const enderecoXdr = (g: string): string =>
  bytesParaHex(new Address(g).toScVal().toXDR());

// ---------- leitura ----------

/**
 * Leitura sem transação: simula a chamada e lê o retorno.
 *
 * Não custa taxa e não precisa de assinatura — é por isso que o dapp pode
 * mostrar o estado de uma proposta para quem ainda não tem carteira nenhuma.
 */
async function ler(metodo: string, ...args: xdr.ScVal[]): Promise<unknown> {
  const conta = await servidor.getAccount(
    // Qualquer conta serve numa simulação; esta é a que financia o friendbot.
    "GAIH3ULLFQ4DGSECF2AR555KZ4KNDGEKN4AFI4SU2M7B43MGK3QJZNSR",
  );
  const tx = new TransactionBuilder(conta, { fee: BASE_FEE, networkPassphrase: REDE.passphrase })
    .addOperation(contrato.call(metodo, ...args))
    .setTimeout(30)
    .build();
  const sim = await servidor.simulateTransaction(tx);
  if (rpc.Api.isSimulationError(sim)) {
    throw new Error(`${metodo}: ${sim.error}`);
  }
  return scValToNative(sim.result!.retval);
}

export type PerguntaRede = { opcoes: number; confidencial: boolean };
export type PropostaRede = {
  perguntas: PerguntaRede[];
  raiz_aptos: Uint8Array;
  mesa: string[];
  limiar: number;
  abre_em: number;
  fecha_em: number;
  anel: boolean;
  secoes: number;
};

export const lerProposta = (id: string) =>
  ler("proposta", bytesN(id)) as Promise<PropostaRede | null>;

/** A raiz de 32 zeros é o sentinela de votação aberta. */
export const RAIZ_ABERTA = "0".repeat(64);
export const ehAberta = (p: PropostaRede) => bytesParaHex(p.raiz_aptos) === RAIZ_ABERTA;

/**
 * A seção numa votação **aberta**: `H(proposta ‖ endereço) mod secoes`.
 *
 * O cliente precisa calcular o mesmo que o contrato **antes de enviar**, porque
 * a seção nomeia a entrada `Anel(proposta, secao)` que a transação vai escrever
 * — e o footprint é declarado na simulação. Derivar isso de ordem de chegada
 * custou uma rodada: uma por ledger, e recusas com `txFailed`.
 */
export async function secaoAberta(id: string, g: string, secoes: number): Promise<number> {
  const bytes = new Uint8Array([...hexParaBytes(id), ...hexParaBytes(enderecoXdr(g))]);
  const h = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
  return new DataView(h.buffer).getUint32(0, false) % secoes;
}

/** Em que seção a pessoa caiu, segundo a cadeia. */
export const lerSecao = async (id: string, g: string): Promise<number | null> =>
  ((await ler("secao_de", bytesN(id), endereco(g))) as number | null) ?? null;

export const lerAnel = async (id: string, secao: number): Promise<string[]> =>
  ((await ler("anel", bytesN(id), u32(secao))) as Uint8Array[]).map(bytesParaHex);

export const lerHp = async (id: string): Promise<string> =>
  bytesParaHex((await ler("hp", bytesN(id))) as Uint8Array);

export const lerGeradorH = async (): Promise<string> =>
  bytesParaHex((await ler("gerador_h")) as Uint8Array);

export const lerComparecimento = (id: string) =>
  ler("comparecimento", bytesN(id)) as Promise<[number, number]>;

export const compareceu = (id: string, g: string) =>
  ler("compareceu", bytesN(id), endereco(g)) as Promise<boolean>;

export async function ledgerAtual(): Promise<number> {
  const r = await fetch(`${REDE.horizon}/ledgers?order=desc&limit=1`);
  const j = await r.json();
  return j._embedded.records[0].sequence as number;
}

/**
 * Quantos ledgers para trás a busca de eventos olha.
 *
 * **Não é um número escolhido por conforto.** Pedir uma janela maior que o
 * limite do RPC não dá erro: devolve zero eventos, em silêncio. Medido na
 * testnet com o contrato tendo quatro aberturas recentes:
 *
 *     últimos    100 ledgers →  0
 *     últimos    500 ledgers →  1
 *     últimos  2.000 ledgers →  1
 *     últimos 17.000 ledgers →  0   ← pediu demais, veio vazio
 *
 * Uma lista vazia e uma consulta larga demais são indistinguíveis daqui, e a
 * segunda faria o dapp parecer quebrado para quem chega. 2.000 ledgers são umas
 * 3 horas, que cobre com folga a vida inteira de uma rodada.
 */
const JANELA = 2_000;

/**
 * As assembleias abertas agora, lidas dos eventos de `abrir`.
 *
 * **Sem índice e sem backend.** O contrato já publica o evento desde a v1, e
 * isto responde "o que está aberto agora" — não "tudo o que já existiu", que
 * exigiria um indexador. Para o dapp é exatamente a pergunta certa.
 */
export type Fase = "agendada" | "comparecimento" | "votacao" | "encerrada";

export type Assembleia = {
  proposta: string;
  perguntas: number;
  opcoes: number;
  abre_em: number;
  fecha_em: number;
  limiar: number;
  anel: boolean;
  ledger: number;
  fase: Fase;
};

/**
 * Em que ponto da vida a assembleia está.
 *
 * A mesma janela significa coisas diferentes nos dois modos. Com o caderno
 * separado da urna, o tempo antes de `abre_em` **é** o comparecimento — não é
 * espera. Sem anel, é só agendamento.
 */
export function fase(a: { abre_em: number; fecha_em: number; anel: boolean }, ledger: number): Fase {
  if (ledger >= a.fecha_em) return "encerrada";
  if (ledger < a.abre_em) return a.anel ? "comparecimento" : "agendada";
  return "votacao";
}

/**
 * Todas as assembleias da janela, em qualquer fase.
 *
 * **Sem índice e sem backend.** O contrato já publica o evento de `abrir`, e
 * isto responde "o que existe agora" — não "tudo o que já existiu", que
 * exigiria um indexador.
 */
export async function assembleias(desde?: number): Promise<Assembleia[]> {
  const atual = await ledgerAtual();
  const inicio = desde ?? Math.max(1, atual - JANELA);
  const r = await servidor.getEvents({
    startLedger: inicio,
    filters: [{ type: "contract", contractIds: [REDE.contrato] }],
  });
  return r.events
    .filter((e) => scValToNative(e.topic[0]) === "abrir")
    .map((e) => {
      const v = scValToNative(e.value) as [number, number, number, number, number, boolean];
      const base = {
        proposta: bytesParaHex(scValToNative(e.topic[1]) as Uint8Array),
        perguntas: v[0],
        opcoes: v[1],
        abre_em: v[2],
        fecha_em: v[3],
        limiar: v[4],
        anel: v[5],
        ledger: e.ledger,
      };
      return { ...base, fase: fase(base, atual) };
    })
    .reverse();
}

// ---------- escrita ----------

/**
 * Monta, simula, assina e envia — e conta cada passo ao diário.
 *
 * A assinatura acontece **depois** da simulação, de propósito: a simulação é o
 * que descobre o footprint e o custo, e assinar antes dela assinaria uma
 * transação que ainda vai mudar.
 */
async function enviar(
  carteira: Carteira,
  metodo: string,
  args: xdr.ScVal[],
  diario?: Diario,
  resumo?: string,
  folga = 0,
): Promise<string> {
  diario?.({ tipo: "cmd", txt: `${metodo}()` });
  if (resumo) diario?.({ tipo: "val", txt: resumo });

  for (let lance = 0; ; lance++) {
    try {
      return await uma(carteira, metodo, args, TAXA_INCLUSAO * 4 ** lance, diario, folga);
    } catch (e) {
      // Taxa curta não é erro do voto: a transação não entrou em ledger nenhum.
      // Re-simular junto com o lance maior também renova o footprint.
      if (lance >= LANCES - 1 || !(e instanceof NaoEntrou)) throw e;
      diario?.({ tipo: "nota", txt: "o ledger está disputado; subindo o lance e tentando de novo" });
    }
  }
}

/**
 * A transação que **não entrou em ledger nenhum** — por lance baixo, ou por ter
 * sido preterida até a validade expirar. As duas querem a mesma resposta:
 * oferecer mais e tentar de novo.
 *
 * Reenviar é seguro porque o contrato recusa a repetição por conta própria —
 * `ImagemJaUsada`, `JaCompareceu`, `PropostaJaExiste`. Se a primeira tiver
 * entrado enquanto esperávamos, a segunda cai num desses e nada se duplica.
 */
class NaoEntrou extends Error {}

async function uma(
  carteira: Carteira,
  metodo: string,
  args: xdr.ScVal[],
  lance: number,
  diario?: Diario,
  folga = 0,
): Promise<string> {
  const conta = await servidor.getAccount(carteira.endereco());
  const bruta = new TransactionBuilder(conta, {
    fee: String(lance),
    networkPassphrase: REDE.passphrase,
  })
    .addOperation(contrato.call(metodo, ...args))
    // 60 s é curto quando várias cédulas disputam o mesmo ledger: a transação
    // expira antes de chegar a vez dela, e quem votou vê um erro sem motivo.
    .setTimeout(180)
    .build();

  const sim = await servidor.simulateTransaction(bruta);
  if (rpc.Api.isSimulationError(sim)) {
    console.error(sim.error);
    diario?.({ tipo: "x", txt: resumir(sim.error) });
    throw new Error(resumir(sim.error));
  }

  // O custo que a simulação descobriu. É o número que sustenta a tese inteira
  // do projeto — 10.822.850 instruções por membro do anel, e um teto de 400 M
  // por transação — e até aqui ele nunca tinha aparecido numa tela.
  try {
    const r = sim.transactionData.build().resources();
    const cpu = r.instructions();
    diario?.({
      tipo: "val",
      txt: `${cpu.toLocaleString("pt-BR")} instruções de CPU · ${porcento(cpu)} do teto de 400 M`,
    });
  } catch {
    /* o SDK mudou de forma: o custo é informação, não pode derrubar o voto */
  }

  let pronta = rpc.assembleTransaction(bruta, sim).build();

  if (folga > 0) {
    const dados = new SorobanDataBuilder(
      pronta.toEnvelope().v1().tx().ext().sorobanData().toXDR("base64"),
    );
    const r = dados.build().resources();
    // `diskReadBytes`, não `readBytes`: o protocolo 23 renomeou.
    dados.setResources(r.instructions(), r.diskReadBytes(), r.writeBytes() + folga);
    pronta = TransactionBuilder.cloneFrom(pronta, {
      fee: (BigInt(pronta.fee) + BigInt(folga) * STROOPS_POR_BYTE).toString(),
      sorobanData: dados.build(),
    }).build();
  }

  // O declarado, não o cobrado: com folga de escrita os dois divergem bastante,
  // e o cobrado só existe depois que a transação entra.
  diario?.({ tipo: "val", txt: `taxa reservada = ${Number(pronta.fee).toLocaleString("pt-BR")} stroops` });

  const assinada = TransactionBuilder.fromXDR(
    await carteira.assinar(pronta.toXDR()),
    REDE.passphrase,
  );
  const envio = await servidor.sendTransaction(assinada);
  if (envio.status === "ERROR") {
    const motivo = envio.errorResult?.result().switch().name;
    if (motivo === "txInsufficientFee") throw new NaoEntrou(motivo);
    diario?.({ tipo: "x", txt: resumir(`a rede recusou: ${motivo ?? JSON.stringify(envio.errorResult)}`) });
    throw new Error(`a rede recusou a transação: ${motivo ?? "sem motivo declarado"}`);
  }

  for (let i = 0; i < 180; i++) {
    const r = await servidor.getTransaction(envio.hash);
    if (r.status === rpc.Api.GetTransactionStatus.SUCCESS) {
      // O que de fato saiu da conta. A diferença para o reservado volta, e é
      // por isso que declarar folga de escrita sai quase de graça.
      const cobrado = r.resultXdr?.feeCharged?.()?.toString();
      if (cobrado) {
        diario?.({ tipo: "val", txt: `taxa cobrada = ${Number(cobrado).toLocaleString("pt-BR")} stroops · o resto volta` });
      }
      diario?.({ tipo: "ok", txt: `tx ${envio.hash}` });
      return envio.hash;
    }
    if (r.status === rpc.Api.GetTransactionStatus.FAILED) {
      diario?.({ tipo: "x", txt: `a rede recusou · tx ${envio.hash}` });
      throw new Error(`transação falhou: ${envio.hash}`);
    }
    await new Promise((s) => setTimeout(s, 1000));
  }
  // Nunca foi incluída: preterida por quem ofereceu mais. Vale subir o lance.
  throw new NaoEntrou("a transação não entrou em nenhum ledger a tempo");
}

export const abrir = (
  c: Carteira,
  id: string,
  perguntas: PerguntaRede[],
  raiz: string,
  mesa: string[],
  limiar: number,
  abre_em: number,
  fecha_em: number,
  anel: boolean,
  secoes: number,
  d?: Diario,
) =>
  enviar(
    c,
    "abrir",
    [
      endereco(c.endereco()),
      bytesN(id),
      vetor(
        perguntas.map((p) =>
          nativeToScVal(
            { opcoes: p.opcoes, confidencial: p.confidencial },
            { type: { opcoes: ["symbol", "u32"], confidencial: ["symbol", "bool"] } },
          ),
        ),
      ),
      bytesN(raiz),
      vetor(mesa.map(endereco)),
      u32(limiar),
      u32(abre_em),
      u32(fecha_em),
      xdr.ScVal.scvBool(anel),
      u32(secoes),
    ],
    d,
    `proposta ${id.slice(0, 8)}… · ${perguntas.length} pergunta${perguntas.length === 1 ? "" : "s"} · ` +
      `mesa ${limiar} de ${mesa.length} · ${anel ? "caderno separado da urna" : "voto identificado"}` +
      (secoes > 1 ? ` · ${secoes} seções` : ""),
  );

/** O caderno: identificado, e é o único ato em que o nome da pessoa aparece. */
export const comparecer = (
  c: Carteira,
  id: string,
  chaveAnel: string,
  caminho: string[],
  indice: number,
  secao: number,
  d?: Diario,
  /** O tamanho do eleitorado: é o teto do anel, e dimensiona a folga. */
  aptos = 40,
) =>
  enviar(
    c,
    "comparecer",
    [bytesN(id), endereco(c.endereco()), ponto(chaveAnel), vetor(caminho.map(bytesN)), u32(indice), u32(secao)],
    d,
    `proposta ${id.slice(0, 8)}… · seção ${secao} · folha ${indice} · caminho com ${caminho.length} irmão${caminho.length === 1 ? "" : "s"}`,
    folgaDoCaderno(aptos),
  );

/**
 * A urna: assinada por uma chave de uso único, que não é a de quem compareceu.
 *
 * O `carteira` aqui **tem de ser** efêmera. Passar a carteira de identidade
 * funcionaria, custaria o mesmo, e destruiria a desvinculação sem nenhum aviso
 * da rede — por isso a checagem é aqui, antes de qualquer byte sair.
 */
export function votarAnonimo(
  c: Carteira,
  id: string,
  secao: number,
  anel: string[],
  imagem: string,
  c0: string,
  z: string[],
  compromissos: string[],
  provas: { a0: string; a1: string; e0: string; z0: string; e1: string; z1: string }[],
  provasSoma: { a: string; z: string }[],
  escolhas: number[],
  d?: Diario,
) {
  if (c.tipo !== "efemera") {
    throw new Error(
      "a cédula só pode ser assinada por uma chave de uso único. Assinar com a " +
        "carteira de identidade recria no ledger exatamente o vínculo que o anel " +
        "existe para quebrar.",
    );
  }
  return enviar(
    c,
    "votar_anonimo",
    [
      bytesN(id),
      u32(secao),
      vetor(anel.map(ponto)),
      ponto(imagem),
      escalar(c0),
      vetor(z.map(escalar)),
      vetor(compromissos.map(ponto)),
      vetor(
        provas.map((p) =>
          nativeToScVal(
            {
              a0: hexParaBytes(p.a0),
              a1: hexParaBytes(p.a1),
              e0: BigInt("0x" + p.e0),
              z0: BigInt("0x" + p.z0),
              e1: BigInt("0x" + p.e1),
              z1: BigInt("0x" + p.z1),
            },
            {
              type: {
                a0: ["symbol", null],
                a1: ["symbol", null],
                e0: ["symbol", "u256"],
                z0: ["symbol", "u256"],
                e1: ["symbol", "u256"],
                z1: ["symbol", "u256"],
              },
            },
          ),
        ),
      ),
      vetor(
        provasSoma.map((p) =>
          nativeToScVal(
            { a: hexParaBytes(p.a), z: BigInt("0x" + p.z) },
            { type: { a: ["symbol", null], z: ["symbol", "u256"] } },
          ),
        ),
      ),
      vetor(escolhas.map(u32)),
    ],
    d,
    `proposta ${id.slice(0, 8)}… · seção ${secao} · anel com ${anel.length} · ${compromissos.length} compromisso${compromissos.length === 1 ? "" : "s"}`,
  );
}
