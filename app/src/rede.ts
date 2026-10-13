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
import { carregar } from "./wasm";

export const REDE = {
  rpc: "https://soroban-testnet.stellar.org",
  horizon: "https://horizon-testnet.stellar.org",
  friendbot: "https://friendbot.stellar.org",
  passphrase: Networks.TESTNET,
  contrato: "CDWGDWDPCXAAA3FO24NRRYS77HEMFHWDIBK4O3GB6L6BQGYWC7JYW3YS",
  explorer: "https://stellar.expert/explorer/testnet",
};

/**
 * A baliza: a cadeia `quicknet` da drand e o relé de onde a assinatura vem.
 *
 * **O relé é um servidor como qualquer outro, e esta camada não confia nele.**
 * Tudo o que ele devolve passa por `conferir_baliza` — um pareamento contra a
 * chave pública congelada da cadeia, em `core/src/relogio.rs` — antes de
 * encostar num cálculo. Se o relé mentir, a conferência recusa; se o relé
 * sumir, qualquer outro serve, porque a assinatura é a mesma para todo mundo.
 */
export const BALIZA = {
  cadeia: "52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971",
  rele: "https://api.drand.sh/v2/chains",
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
 * Quantas vezes a taxa reembolsável que a simulação calculou.
 *
 * Quatro, medido: a simulação precifica o aluguel das entradas que **existem**
 * naquele instante, e sob rajada quem aplica depois pode ter de criar uma
 * entrada nova — a seção seguinte. Com ×1, a rajada de seis com limite 2 deixa
 * 2 entrarem; com ×4, entram 6. Ver §11-D.
 */
const RESERVA_DE_ALUGUEL = 4n;

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
  1: "Proposal already exists",
  2: "Proposal not found",
  3: "Option count is out of range",
  4: "Invalid threshold",
  5: "Deadline is in the past",
  6: "Voting has closed",
  7: "Voting is still open",
  8: "Already voted",
  9: "Not on the eligibility list",
  10: "Only unit voting weights are supported",
  11: "Invalid binary proof",
  12: "Invalid sum proof",
  13: "Point is outside the required subgroup",
  14: "Opening does not match the commitments",
  15: "Already tallied",
  16: "Committee threshold not met",
  17: "Not a committee member",
  18: "Duplicate committee member",
  19: "Insufficient anonymity",
  20: "Malformed argument",
  21: "Choice must be binary",
  22: "Sum does not match voting weight",
  23: "Total does not match attendance",
  24: "Member already endorsed this tally",
  25: "Question count is out of range",
  26: "Voting has not started",
  27: "Check-in has closed",
  28: "Already checked in",
  29: "Invalid ring signature",
  30: "This participation key has already voted",
  31: "Incorrect voting mode",
  32: "Invalid section",
  33: "This vote has open participation",
  34: "Beacon round does not match",
  35: "Time lock has not opened",
  36: "Ballot chain does not match",
  37: "Tally does not improve the recorded result",
  38: "Malformed ciphertext",

};

/** Arredondar para inteiro faz uma chamada barata ler "0% do teto", que soa
 *  como ausência de medida em vez de medida pequena. */
function porcento(cpu: number): string {
  const p = (cpu / 400_000_000) * 100;
  return p < 1 ? `${p.toFixed(2)}%` : `${Math.round(p)}%`;
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
  return `${inteiro.slice(0, corte).trim()} · diagnostic log in the console`;
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
const u64 = (n: number) => xdr.ScVal.scvU64(new xdr.Uint64(BigInt(n)));
const bytes = (h: string) =>
  xdr.ScVal.scvBytes(h === "" ? new Uint8Array(0) : hexParaBytes(h));
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
    throw new Error(resumir(`${metodo}: ${sim.error}`));
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
  /** A rodada da baliza que destranca as cédulas. `0` = sem fechadura. */
  rodada: bigint;
  /** A rodada de onde sai a divisão em seções (DEC-012). */
  rodada_abertura: bigint;
};

export const lerProposta = (id: string) =>
  ler("proposta", bytesN(id)) as Promise<PropostaRede | null>;

/** Os tetos do contrato, espelhados de `contrato/src/tipos.rs`. O teto de
 *  opções é do orçamento de CPU: cada opção sigilosa custa uma disjuntiva. */
export const MAX_PERGUNTAS = 8;
export const MAX_OPCOES = 16;

/** A raiz de 32 zeros é o sentinela de votação aberta. */
export const RAIZ_ABERTA = "0".repeat(64);
export const ehAberta = (p: PropostaRede) => bytesParaHex(p.raiz_aptos) === RAIZ_ABERTA;

/**
 * As chaves de ledger dos anéis `[primeira, ultima]`, para somar ao footprint.
 *
 * **É isto que faz o split automático funcionar sob rajada.** A seção só existe
 * na aplicação — ela vem de `caderno / limite` —, mas o footprint é declarado
 * na simulação. Quem comparece sozinho acerta a previsão; vinte comparecendo
 * juntos, não. Declarando uma janela, a transação pode escrever em qualquer
 * seção da faixa sem ter declarado uma e escrito outra.
 *
 * Sem isso, o medido foi uma por ledger e recusas com `txFailed`.
 */
function chavesDeAnel(id: string, primeira: number, ultima: number): xdr.LedgerKey[] {
  const chaves: xdr.LedgerKey[] = [];
  for (let s = primeira; s <= ultima; s++) {
    chaves.push(
      xdr.LedgerKey.contractData(
        new xdr.LedgerKeyContractData({
          contract: new Address(REDE.contrato).toScAddress(),
          key: xdr.ScVal.scvVec([
            xdr.ScVal.scvSymbol("Anel"),
            bytesN(id),
            u32(s),
          ]),
          durability: xdr.ContractDataDurability.persistent(),
        }),
      ),
    );
  }
  return chaves;
}

/**
 * O placar, quando existe.
 *
 * Só existe depois que alguém reuniu a abertura e o contrato conferiu contra o
 * acumulado. Numa votação sem mesa isso nunca acontece, e é isso que `/apurar`
 * explica.
 */
export const lerResultado = async (id: string): Promise<number[] | null> =>
  ((await ler("resultado", bytesN(id))) as number[] | undefined) ?? null;

/**
 * O placar de uma seção, como o contrato o guardou: quantas cédulas abriram e
 * os totais sobre elas.
 *
 * Os dois números andam juntos de propósito. Um placar sobre um subconjunto
 * sem dizer qual subconjunto é um placar que mente por omissão — e o contrato
 * guarda os dois justamente para que a tela não possa separá-los.
 */
/**
 * O que uma seção apurada guarda: **quantas cédulas abriram e um ponto por
 * opção** — a soma dos compromissos abertos.
 *
 * Nenhum total em claro, desde T-026: o ponto é perfeitamente ocultante, e o
 * número de voto só existe no placar agregado, que `lerResultado` devolve.
 */
export const lerResultadoSecao = async (
  id: string,
  secao: number,
): Promise<{ abriram: number; pontos: string[] } | null> => {
  const r = (await ler("resultado_secao", bytesN(id), u32(secao))) as
    | [Uint8Array, Uint8Array[]]
    | null
    | undefined;
  return r ? { abriram: Number(r[0]), pontos: (r[1] as Uint8Array[]).map(bytesParaHex) } : null;
};

/**
 * A cadeia de uma seção: o encadeamento e **quantas cédulas** ele prendeu.
 *
 * É como quem lê os eventos sabe se achou todas. Sem isto, uma leitura
 * incompleta — janela do RPC, página perdida — levaria `CadeiaNaoFecha`, que é
 * indistinguível de uma cadeia adulterada.
 */
export const lerCadeia = async (
  id: string,
  secao: number,
): Promise<{ elo: string; quantas: number }> => {
  const r = (await ler("cadeia", bytesN(id), u32(secao))) as [Uint8Array, number] | undefined;
  return r ? { elo: bytesParaHex(r[0]), quantas: Number(r[1]) } : { elo: "", quantas: 0 };
};

/** Quantas seções existem agora. Na aberta cresce com o comparecimento. */
export const lerSecoes = async (id: string): Promise<number> =>
  ((await ler("secoes", bytesN(id))) as number | undefined) ?? 1;

/** Em que seção a pessoa caiu, segundo a cadeia. */
export const lerSecao = async (id: string, g: string): Promise<number | null> =>
  ((await ler("secao_de", bytesN(id), endereco(g))) as number | null) ?? null;

/// A assinatura da baliza da rodada de abertura, já conferida pelo contrato,
/// nos 96 bytes não comprimidos. É dela que a seção é derivada (DEC-012) — e
/// são **estes** bytes, não os 48 comprimidos que a baliza publica.
export const lerAbertura = async (id: string): Promise<string | null> => {
  const b = (await ler("abertura", bytesN(id))) as Uint8Array | null;
  return b ? bytesParaHex(b) : null;
};

/// Registra a assinatura. Qualquer pessoa, uma vez, depois que a rodada venceu
/// — e o contrato confere com um pareamento antes de guardar, senão quem a
/// apresentasse escolheria as seções.
export const registrarAbertura = (c: Carteira, id: string, assinaturaHost: string, d?: Diario) =>
  enviar(
    c,
    "registrar_abertura",
    [bytesN(id), bytesN(assinaturaHost)],
    d,
    `proposal ${id.slice(0, 8)}… · beacon signature, 96 bytes`,
  );

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

// ---------- a fechadura de tempo ----------

/**
 * Busca a assinatura da rodada no relé **e a confere** antes de devolver.
 *
 * A conferência é um pareamento contra a chave pública da cadeia, e acontece
 * aqui, nesta função, não no uso — é o que INV-24 exige. Nenhum chamador recebe
 * bytes que não passaram por ela, e nenhuma página fala com o relé direto.
 *
 * Devolve os 48 bytes comprimidos que a baliza publica. Para
 * `registrar_abertura` o contrato quer os 96 não comprimidos, e quem converte é
 * `assinatura_da_baliza` no wasm — o host não sabe descomprimir.
 */
export async function buscarBaliza(rodada: number | bigint): Promise<string> {
  const r = await fetch(`${BALIZA.rele}/${BALIZA.cadeia}/rounds/${rodada}`);
  if (!r.ok) {
    throw new Error(
      `The beacon has not published round ${rodada} (or the relay is unavailable): HTTP ${r.status}`,
    );
  }
  const j = (await r.json()) as { round: number; signature: string };
  if (BigInt(j.round) !== BigInt(rodada)) {
    throw new Error(`The relay returned round ${j.round} instead of ${rodada}`);
  }
  const w = await carregar();
  // Lança se o pareamento não fechar. É o único portão, e é por aqui que passa
  // tudo o que vem de fora.
  w.conferir_baliza(BigInt(rodada), j.signature);
  return j.signature;
}

/** Os 96 bytes não comprimidos que o host lê, a partir dos 48 da baliza. */
export async function balizaParaHost(assinatura: string): Promise<string> {
  return (await carregar()).assinatura_da_baliza(assinatura);
}

/** Uma cédula como o ledger a guarda: os compromissos e o criptograma. */
export type CedulaNoLedger = {
  imagem: string;
  compromissos: string[];
  /** 160 bytes por opção confidencial, em hexadecimal. */
  cripto: string;
  ledger: number;
};

/**
 * As cédulas depositadas, na ordem em que chegaram.
 *
 * **Sem indexador.** O evento `anonimo` carrega os compromissos e o
 * criptograma, e o RPC público os guarda por 7 dias — mais que qualquer janela
 * de votação. A ordem dos eventos é a ordem da cadeia de compromissos, que é o
 * que `apurar_secao` re-encadeia: ler na ordem errada é recusado com
 * `CadeiaNaoFecha`, não aceito em silêncio.
 *
 * A seção vem do **tópico** do evento (§11-N). Sem ela, uma votação de várias
 * seções era ilegível para quem apura: o criptograma não diz a seção, a
 * assinatura em anel não diz qual anel assinou, e a cadeia é sequencial — então
 * separar as cédulas seria testar as partições possíveis.
 */
export async function cedulasDaSecao(
  id: string,
  desdeLedger: number,
  secao: number,
): Promise<CedulaNoLedger[]> {
  const todas: CedulaNoLedger[] = [];
  let cursor: string | undefined;
  for (;;) {
    const r = await servidor.getEvents(
      cursor
        ? { cursor, filters: [{ type: "contract", contractIds: [REDE.contrato] }] }
        : {
            startLedger: Math.max(1, desdeLedger),
            filters: [{ type: "contract", contractIds: [REDE.contrato] }],
          },
    );
    for (const e of r.events) {
      if (scValToNative(e.topic[0]) !== "anonimo") continue;
      if (bytesParaHex(scValToNative(e.topic[1]) as Uint8Array) !== id) continue;
      if (Number(scValToNative(e.topic[2])) !== secao) continue;
      const v = scValToNative(e.value) as [Uint8Array, Uint8Array[], number[], Uint8Array];
      todas.push({
        imagem: bytesParaHex(v[0]),
        compromissos: v[1].map(bytesParaHex),
        cripto: bytesParaHex(v[3]),
        ledger: e.ledger,
      });
    }
    if (!r.cursor || r.events.length === 0) return todas;
    cursor = r.cursor;
  }
}

/** O que a decifragem reconstruiu, pronto para o contrato. */
export type Abertura = {
  abertas: boolean[];
  totais: number[];
  aberturas: string[];
  abriram: number;
  cedulas: number;
};

/**
 * Decifra a seção inteira no navegador e devolve o placar a apresentar.
 *
 * Nada disso precisa de carteira: a chave da rodada é pública, os
 * criptogramas estão no ledger, e a conta é a mesma para quem quer que a faça.
 * É isso que torna a apuração **não travável** — não depende de ninguém em
 * particular voltar.
 */
export async function abrirSecao(
  cedulas: CedulaNoLedger[],
  nConf: number,
  rodada: number | bigint,
): Promise<Abertura> {
  const assinatura = await buscarBaliza(rodada);
  const [w, h] = await Promise.all([carregar(), lerGeradorH()]);
  return w.abertura_da_secao(
    cedulas.flatMap((c) => c.compromissos),
    cedulas.map((c) => c.cripto),
    nConf,
    h,
    BigInt(rodada),
    assinatura,
  ) as Abertura;
}

/**
 * Entrega o placar da seção. Qualquer pessoa, depois do fim da janela e do
 * vencimento da rodada — e só substitui o guardado se abrir **mais** cédulas.
 */
export const apurarSecao = (
  c: Carteira,
  id: string,
  secao: number,
  compromissos: string[],
  a: Abertura,
  d?: Diario,
) =>
  enviar(
    c,
    "apurar_secao",
    [
      bytesN(id),
      endereco(c.endereco()),
      u32(secao),
      vetor(compromissos.map(ponto)),
      vetor(a.abertas.map((b) => xdr.ScVal.scvBool(b))),
    ],
    d,
    `proposal ${id.slice(0, 8)}… · section ${secao} · ${a.abriram} of ${a.cedulas} ballots opened`,
  );

/**
 * **O placar da votação.** A única transação em que um número de voto aparece.
 *
 * `apurar_secao` guarda uma contagem e um ponto por seção, e nada em claro —
 * é o conserto da DEC-017, em que o total de uma seção unânime era o voto dos
 * seus membros. Aqui as seções nomeadas são somadas e um MSM confere o
 * agregado, sobre o eleitorado inteiro.
 */
export const publicarResultado = (
  c: Carteira,
  id: string,
  secoes: number[],
  totais: number[],
  aberturas: string[],
  d?: Diario,
) =>
  enviar(
    c,
    "publicar_resultado",
    [
      bytesN(id),
      endereco(c.endereco()),
      vetor(secoes.map(u32)),
      vetor(totais.map(u32)),
      vetor(aberturas.map(escalar)),
    ],
    d,
    `proposal ${id.slice(0, 8)}… · ${secoes.length} section${secoes.length === 1 ? "" : "s"} · tally ${totais.join(" · ")}`,
  );

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
  extras: xdr.LedgerKey[] = [],
): Promise<string> {
  diario?.({ tipo: "cmd", txt: `${metodo}()` });
  if (resumo) diario?.({ tipo: "val", txt: resumo });

  for (let lance = 0; ; lance++) {
    try {
      return await uma(carteira, metodo, args, TAXA_INCLUSAO * 4 ** lance, diario, folga, extras);
    } catch (e) {
      // Taxa curta não é erro do voto: a transação não entrou em ledger nenhum.
      // Re-simular junto com o lance maior também renova o footprint.
      if (lance >= LANCES - 1 || !(e instanceof NaoEntrou)) throw e;
      diario?.({ tipo: "nota", txt: "The ledger is busy; increasing the fee bid and retrying" });
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
  extras: xdr.LedgerKey[] = [],
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
      txt: `${cpu.toLocaleString("en-US")} CPU instructions · ${porcento(cpu)} of the 400 M limit`,
    });
  } catch {
    /* o SDK mudou de forma: o custo é informação, não pode derrubar o voto */
  }

  let pronta = rpc.assembleTransaction(bruta, sim).build();

  if (folga > 0 || extras.length > 0) {
    const dados = new SorobanDataBuilder(
      pronta.toEnvelope().v1().tx().ext().sorobanData().toXDR("base64"),
    );
    const r = dados.build().resources();
    // `diskReadBytes`, não `readBytes`: o protocolo 23 renomeou.
    // Toda entrada declarada é **carregada**, não só a usada: declarar as
    // seções vizinhas aumenta a leitura junto da escrita. Só subir
    // `writeBytes` deixou 10 de 20 em `txFailed`.
    dados.setResources(
      r.instructions(),
      r.diskReadBytes() + folga,
      r.writeBytes() + folga,
    );
    if (extras.length) {
      // As que a simulação já previu continuam; estas são as seções vizinhas,
      // que a transação pode acabar escrevendo se gente entrar no meio.
      const atuais = dados.getReadWrite();
      const vistas = new Set(atuais.map((k) => k.toXDR("base64")));
      const novas = extras.filter((k) => !vistas.has(k.toXDR("base64")));
      dados.setReadWrite([...atuais, ...novas]);
    }
    // **A taxa reembolsável, reservada com sobra** (§11-D, T-014).
    //
    // Medido em 2026-10-10: sob rajada, quem acaba escrevendo uma seção que a
    // simulação não previu paga **aluguel de uma entrada nova**, e a reserva
    // calculada para o estado de antes não cobre. O erro é
    // `InsufficientRefundableFee`, e é de onde vinha o padrão "só a primeira
    // seção encheu" — não de buraco no mecanismo de split. Seis comparecendo de
    // uma vez com limite 2 passaram de 2 de 6 para **6 de 6**, três seções
    // nascendo no mesmo ledger.
    //
    // A diferença volta: taxa reembolsável reservada e não usada é devolvida,
    // e é por isso que reservar com folga sai quase de graça.
    const reembolsavel = BigInt(dados.build().resourceFee().toString());
    dados.setResourceFee(reembolsavel * RESERVA_DE_ALUGUEL);
    pronta = TransactionBuilder.cloneFrom(pronta, {
      fee: (
        BigInt(pronta.fee) +
        reembolsavel * RESERVA_DE_ALUGUEL +
        BigInt(folga) * STROOPS_POR_BYTE
      ).toString(),
      sorobanData: dados.build(),
    }).build();
  }

  // O declarado, não o cobrado: com folga de escrita os dois divergem bastante,
  // e o cobrado só existe depois que a transação entra.
  diario?.({ tipo: "val", txt: `reserved fee = ${Number(pronta.fee).toLocaleString("en-US")} stroops` });

  const assinada = TransactionBuilder.fromXDR(
    await carteira.assinar(pronta.toXDR()),
    REDE.passphrase,
  );
  const envio = await servidor.sendTransaction(assinada);
  if (envio.status === "ERROR") {
    const motivo = envio.errorResult?.result().switch().name;
    if (motivo === "txInsufficientFee") throw new NaoEntrou(motivo);
    diario?.({ tipo: "x", txt: resumir(`Network rejected the transaction: ${motivo ?? JSON.stringify(envio.errorResult)}`) });
    throw new Error(`Network rejected the transaction: ${motivo ?? "no reason provided"}`);
  }

  for (let i = 0; i < 180; i++) {
    const r = await servidor.getTransaction(envio.hash);
    if (r.status === rpc.Api.GetTransactionStatus.SUCCESS) {
      // O que de fato saiu da conta. A diferença para o reservado volta, e é
      // por isso que declarar folga de escrita sai quase de graça.
      const cobrado = r.resultXdr?.feeCharged?.()?.toString();
      if (cobrado) {
        diario?.({ tipo: "val", txt: `charged fee = ${Number(cobrado).toLocaleString("en-US")} stroops · the remainder is refunded` });
      }
      diario?.({ tipo: "ok", txt: `tx ${envio.hash}` });
      return envio.hash;
    }
    if (r.status === rpc.Api.GetTransactionStatus.FAILED) {
      diario?.({ tipo: "x", txt: `Network rejected · tx ${envio.hash}` });
      throw new Error(`Transaction failed: ${envio.hash}`);
    }
    await new Promise((s) => setTimeout(s, 1000));
  }
  // Nunca foi incluída: preterida por quem ofereceu mais. Vale subir o lance.
  throw new NaoEntrou("The transaction was not included in a ledger before the timeout");
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
  limiteSecao: number,
  /** A rodada da baliza que destranca as cédulas, e a que **abre o
   *  comparecimento** e decide as seções.
   *
   *  Não há campo de tempo: o prazo **é** `instante(rodada)`, e guardar os dois
   *  só criaria divergência. As duas têm de estar no futuro na hora de abrir —
   *  uma rodada já vencida tem assinatura publicada, e aí as cédulas abririam
   *  na hora e as seções seriam moíveis. Quem calcula é `rodada_em` no wasm, o
   *  mesmo código que o contrato confere. */
  rodada = 0,
  rodadaAbertura = 0,
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
      u32(limiteSecao),
      u64(rodada),
      u64(rodadaAbertura),
    ],
    d,
    `proposal ${id.slice(0, 8)}… · ${perguntas.length} question${perguntas.length === 1 ? "" : "s"} · ` +
      `committee ${limiar} of ${mesa.length} · ${anel ? "attendance separate from ballots" : "identified voting"}` +
      (limiteSecao > 0
        ? ` · sections of ${limiteSecao}, created automatically`
        : secoes > 1
          ? ` · ${secoes} sections`
          : ""),
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
  /** Quantas seções além da prevista declarar. Cobre quem entrar no meio. */
  janela = 0,
) =>
  enviar(
    c,
    "comparecer",
    [bytesN(id), endereco(c.endereco()), ponto(chaveAnel), vetor(caminho.map(bytesN)), u32(indice), u32(secao)],
    d,
    `proposal ${id.slice(0, 8)}… · section ${secao} · leaf ${indice} · path with ${caminho.length} sibling${caminho.length === 1 ? "" : "s"}`,
    folgaDoCaderno(aptos),
    janela > 0 ? chavesDeAnel(id, secao, secao + janela) : [],
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
  /** Os criptogramas da fechadura, concatenados em hexadecimal: 160 bytes por
   *  opção confidencial. Vazio quando a proposta não tem fechadura.
   *
   *  O contrato não os lê — não pode, o host não tem pareamento com saída de
   *  valor. Ele confere a forma e os carrega no evento, e quem decifra é
   *  qualquer pessoa, depois da rodada. */
  cripto = "",
  d?: Diario,
) {
  if (c.tipo !== "efemera") {
    throw new Error(
      "A ballot must be signed with a one-time key. Signing with the " +
        "identity wallet would recreate the on-chain link that the ring signature " +
        "is intended to prevent.",
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
      bytes(cripto),
    ],
    d,
    `proposal ${id.slice(0, 8)}… · section ${secao} · ring size ${anel.length} · ${compromissos.length} commitment${compromissos.length === 1 ? "" : "s"}`,
  );
}
