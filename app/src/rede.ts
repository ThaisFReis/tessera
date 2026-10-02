import {
  Address,
  Contract,
  Networks,
  TransactionBuilder,
  BASE_FEE,
  rpc,
  xdr,
  scValToNative,
  nativeToScVal,
} from "@stellar/stellar-sdk";
import type { Carteira } from "./carteira";

export const REDE = {
  rpc: "https://soroban-testnet.stellar.org",
  horizon: "https://horizon-testnet.stellar.org",
  friendbot: "https://friendbot.stellar.org",
  passphrase: Networks.TESTNET,
  contrato: "CBL7Z4AFMDPPJEP7YWFXLCUGRLO5VF7XAONPIW26CURO3ETCGSRV565X",
  explorer: "https://stellar.expert/explorer/testnet",
};

const servidor = new rpc.Server(REDE.rpc);
const contrato = new Contract(REDE.contrato);

/** Cada passo que a página dá, para a coluna "o que está acontecendo". */
export type Passo = { tipo: "cmd" | "val" | "ok" | "x" | "nota"; txt: string };
export type Diario = (p: Passo) => void;

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
};

export const lerProposta = (id: string) =>
  ler("proposta", bytesN(id)) as Promise<PropostaRede | null>;

export const lerAnel = async (id: string): Promise<string[]> =>
  ((await ler("anel", bytesN(id))) as Uint8Array[]).map(bytesParaHex);

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
export async function assembleiasAbertas(desde?: number) {
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
      return {
        proposta: bytesParaHex(scValToNative(e.topic[1]) as Uint8Array),
        perguntas: v[0],
        opcoes: v[1],
        abre_em: v[2],
        fecha_em: v[3],
        limiar: v[4],
        anel: v[5],
        ledger: e.ledger,
      };
    })
    .filter((a) => a.fecha_em > atual);
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
): Promise<string> {
  diario?.({ tipo: "cmd", txt: `${metodo}()` });
  const conta = await servidor.getAccount(carteira.endereco());
  const bruta = new TransactionBuilder(conta, {
    fee: BASE_FEE,
    networkPassphrase: REDE.passphrase,
  })
    .addOperation(contrato.call(metodo, ...args))
    .setTimeout(60)
    .build();

  const sim = await servidor.simulateTransaction(bruta);
  if (rpc.Api.isSimulationError(sim)) {
    diario?.({ tipo: "x", txt: sim.error });
    throw new Error(sim.error);
  }
  const pronta = rpc.assembleTransaction(bruta, sim).build();
  diario?.({ tipo: "val", txt: `taxa = ${pronta.fee} stroops` });

  const assinada = TransactionBuilder.fromXDR(
    await carteira.assinar(pronta.toXDR()),
    REDE.passphrase,
  );
  const envio = await servidor.sendTransaction(assinada);
  if (envio.status === "ERROR") {
    diario?.({ tipo: "x", txt: `a rede recusou: ${JSON.stringify(envio.errorResult)}` });
    throw new Error("a rede recusou a transação");
  }

  for (let i = 0; i < 40; i++) {
    const r = await servidor.getTransaction(envio.hash);
    if (r.status === rpc.Api.GetTransactionStatus.SUCCESS) {
      diario?.({ tipo: "ok", txt: `tx ${envio.hash}` });
      return envio.hash;
    }
    if (r.status === rpc.Api.GetTransactionStatus.FAILED) {
      diario?.({ tipo: "x", txt: `a rede recusou · tx ${envio.hash}` });
      throw new Error(`transação falhou: ${envio.hash}`);
    }
    await new Promise((s) => setTimeout(s, 1000));
  }
  throw new Error("a transação não confirmou em 40 segundos");
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
    ],
    d,
  );

/** O caderno: identificado, e é o único ato em que o nome da pessoa aparece. */
export const comparecer = (
  c: Carteira,
  id: string,
  chaveAnel: string,
  caminho: string[],
  indice: number,
  d?: Diario,
) =>
  enviar(
    c,
    "comparecer",
    [bytesN(id), endereco(c.endereco()), ponto(chaveAnel), vetor(caminho.map(bytesN)), u32(indice)],
    d,
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
  );
}
