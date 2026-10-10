/**
 * **O diagnóstico que faltava para a §11-D.**
 *
 * O split automático funciona sequencialmente: com `limite_secao = 2`, a
 * terceira pessoa abre a seção 1. Sob rajada, não — vinte de uma vez com
 * limite 10 deram exatamente 10 aceitas, e as recusadas foram justamente as que
 * precisariam abrir a seção 1.
 *
 * Já foi descartado: a janela do footprint (o caso sequencial usa a mesma e
 * passa), o orçamento de escrita e o de leitura. O que falta é separar duas
 * hipóteses, e para isso basta uma medição:
 *
 * - **se as seis entrarem** com limite 2 — o que força três seções a nascer ao
 *   mesmo tempo — o problema é de escala, e a rajada de 20 esbarra em outra
 *   coisa (contenção na entrada do caderno, provavelmente);
 * - **se repetir o padrão "só a primeira seção"**, o mecanismo tem um buraco
 *   que o caso sequencial não mostra.
 *
 * A votação é **aberta** (`raiz_aptos` de 32 zeros): é o único modo em que a
 * seção vem da ordem de chegada, que é o que o `limite_secao` divide.
 *
 *   node scripts/rajada-secao.mjs
 *   N=6 LIMITE=2 JANELA=6 node scripts/rajada-secao.mjs
 */

import { createRequire } from "node:module";
import {
  Address, Contract, Keypair, Networks, SorobanDataBuilder,
  TransactionBuilder, nativeToScVal, rpc, scValToNative, xdr,
} from "@stellar/stellar-sdk";

const require = createRequire(import.meta.url);
const wasm = require("../../cliente-wasm/pacote-node/tessera_cliente.js");

const RPC = "https://soroban-testnet.stellar.org";
const HORIZON = "https://horizon-testnet.stellar.org";
const FRIENDBOT = "https://friendbot.stellar.org";
const PASSPHRASE = Networks.TESTNET;
const CONTRATO = process.env.TESSERA_CONTRATO ??
  "CDWGDWDPCXAAA3FO24NRRYS77HEMFHWDIBK4O3GB6L6BQGYWC7JYW3YS";

const N = Number(process.env.N ?? 6);
const LIMITE = Number(process.env.LIMITE ?? 2);
/** Quantas seções declarar no footprint, além da prevista na simulação. */
const JANELA = Number(process.env.JANELA ?? 6);
const FOLGA = Number(process.env.FOLGA ?? 8192);
/** Multiplicador da taxa reembolsável reservada. 1 reproduz o de hoje. */
const RESERVA = Number(process.env.RESERVA ?? 4);
const TAXA_INCLUSAO = 1_000_000;

const servidor = new rpc.Server(RPC);
const contrato = new Contract(CONTRATO);

const hexBytes = (h) => Buffer.from(h, "hex");
const bytesHex = (b) => Buffer.from(b).toString("hex");
const bN = (h) => xdr.ScVal.scvBytes(hexBytes(h));
const u32 = (n) => xdr.ScVal.scvU32(n);
const u64 = (n) => xdr.ScVal.scvU64(new xdr.Uint64(BigInt(n)));
const vec = (v) => xdr.ScVal.scvVec(v);
const addr = (g) => new Address(g).toScVal();
const dorme = (ms) => new Promise((r) => setTimeout(r, ms));
const diz = (s) => console.log(`  ${s}`);

async function ledgerAtual() {
  const r = await fetch(`${HORIZON}/ledgers?order=desc&limit=1`);
  return (await r.json())._embedded.records[0].sequence;
}

async function nascer() {
  for (let t = 0; t < 6; t++) {
    const par = Keypair.random();
    const r = await fetch(`${FRIENDBOT}?addr=${par.publicKey()}`);
    if (r.ok || r.status === 400) return par;
    await dorme(1500 * (t + 1));
  }
  throw new Error("friendbot não financiou");
}

/** As entradas `Anel(proposta, s)` de uma faixa de seções. */
function chavesDeAnel(id, primeira, ultima) {
  const chaves = [];
  for (let s = primeira; s <= ultima; s++) {
    chaves.push(
      xdr.LedgerKey.contractData(
        new xdr.LedgerKeyContractData({
          contract: new Address(CONTRATO).toScAddress(),
          key: vec([xdr.ScVal.scvSymbol("Anel"), bN(id), u32(s)]),
          durability: xdr.ContractDataDurability.persistent(),
        }),
      ),
    );
  }
  return chaves;
}

async function tentar(par, metodo, args, { folga = 0, extras = [] } = {}) {
  try {
    const conta = await servidor.getAccount(par.publicKey());
    const bruta = new TransactionBuilder(conta, {
      fee: String(TAXA_INCLUSAO), networkPassphrase: PASSPHRASE,
    }).addOperation(contrato.call(metodo, ...args)).setTimeout(180).build();

    const sim = await servidor.simulateTransaction(bruta);
    if (rpc.Api.isSimulationError(sim)) {
      return { ok: false, fase: "simulação", erro: sim.error.split("\n")[0] };
    }
    let pronta = rpc.assembleTransaction(bruta, sim).build();

    if (folga > 0 || extras.length > 0) {
      const dados = new SorobanDataBuilder(
        pronta.toEnvelope().v1().tx().ext().sorobanData().toXDR("base64"),
      );
      const r = dados.build().resources();
      // **`diskReadBytes` junto, não só `writeBytes`.** Toda entrada declarada
      // é carregada, não só a usada — então declarar as seções vizinhas
      // aumenta a leitura também. Esquecer isto deu `txFailed` com **uma**
      // pessoa só, que é indistinguível do mecanismo não funcionar; foi a
      // primeira coisa que este diagnóstico mediu.
      dados.setResources(
        r.instructions(),
        r.diskReadBytes() + folga,
        r.writeBytes() + folga,
      );
      // **A taxa com sobra larga, de propósito.** Cada entrada a mais no
      // footprint custa taxa de recurso, e errar para menos devolve `txFailed`
      // na aplicação — o que, medido, é indistinguível de o mecanismo não
      // funcionar. Foi exatamente o que aconteceu na primeira tentativa deste
      // diagnóstico: uma pessoa só, e falhou.
      // Exatamente como `rede.ts::enviar` faz — é o caminho que o dapp usa, e
      // é esse que o diagnóstico tem de exercitar.
      if (extras.length) {
        const atuais = dados.getReadWrite();
        const vistas = new Set(atuais.map((k) => k.toXDR("base64")));
        const novas = extras.filter((k) => !vistas.has(k.toXDR("base64")));
        dados.setReadWrite([...atuais, ...novas]);
      }
      // **A taxa reembolsável, reservada com sobra.** É o que a medição de
      // 2026-10-10 achou: sob rajada, quem acaba escrevendo a seção 1 ou 2 paga
      // aluguel de uma entrada **nova** que a simulação — feita com o caderno
      // de antes — não previu, e a reserva não cobre. O erro é
      // `InsufficientRefundableFee`, e o padrão "só a primeira seção encheu"
      // vem daí, não de buraco no mecanismo.
      //
      // A diferença volta: taxa reembolsável reservada e não usada é devolvida.
      const antiga = BigInt(dados.build().resourceFee().toString());
      dados.setResourceFee(antiga * BigInt(RESERVA));
      const taxa = BigInt(pronta.fee) + antiga * BigInt(RESERVA) + BigInt(folga) * 200n;
      pronta = TransactionBuilder.cloneFrom(pronta, {
        fee: taxa.toString(), sorobanData: dados.build(),
      }).build();
    }

    pronta.sign(par);
    const envio = await servidor.sendTransaction(pronta);
    if (envio.status === "ERROR") {
      return { ok: false, fase: "envio",
               erro: envio.errorResult?.result().switch().name ?? "?" };
    }
    for (let i = 0; i < 180; i++) {
      const r = await servidor.getTransaction(envio.hash);
      if (r.status === rpc.Api.GetTransactionStatus.SUCCESS) {
        return { ok: true, ledger: r.ledger, hash: envio.hash,
                 valor: r.returnValue ? scValToNative(r.returnValue) : null };
      }
      if (r.status === rpc.Api.GetTransactionStatus.FAILED) {
        // **O motivo, não só "falhou".** Um `txFailed` sem o código é
        // indistinguível entre "o contrato recusou" e "a declaração de
        // recursos não cobriu o que a aplicação precisou".
        let porque = "txFailed";
        try {
          const res = r.resultXdr?.result();
          porque = res?.switch?.().name ?? porque;
          const ops = res?.results?.();
          if (ops?.length) {
            const o = ops[0].tr?.();
            porque += ` · ${o?.switch?.().name ?? ""}${o?.invokeHostFunctionResult?.()?.switch?.().name ?? ""}`;
          }
        } catch {
          /* o SDK muda de forma; o código é informação, não pode derrubar a medição */
        }
        return { ok: false, fase: "aplicação", ledger: r.ledger, erro: porque };
      }
      await dorme(1000);
    }
    return { ok: false, fase: "espera", erro: "expirou" };
  } catch (e) {
    return { ok: false, fase: "exceção", erro: String(e.message ?? e).split("\n")[0].slice(0, 140) };
  }
}

async function ler(metodo, ...args) {
  const par = Keypair.random();
  await fetch(`${FRIENDBOT}?addr=${par.publicKey()}`);
  const conta = await servidor.getAccount(par.publicKey());
  const t = new TransactionBuilder(conta, { fee: "100", networkPassphrase: PASSPHRASE })
    .addOperation(contrato.call(metodo, ...args)).setTimeout(30).build();
  const sim = await servidor.simulateTransaction(t);
  if (rpc.Api.isSimulationError(sim)) throw new Error(sim.error.split("\n")[0]);
  return sim.result?.retval ? scValToNative(sim.result.retval) : null;
}

console.log(`\nTESSERA · §11-D · ${N} comparecimentos simultâneos, limite ${LIMITE} por seção`);
console.log(`  contrato ${CONTRATO}`);
console.log("─".repeat(70));

const dono = await nascer();
const agora = await ledgerAtual();
const id = bytesHex(Keypair.random().rawPublicKey().subarray(0, 32));

// Votação **aberta**: raiz de 32 zeros. É o modo em que a seção vem da ordem de
// chegada, e `limite_secao` divide essa ordem.
const r1 = await tentar(dono, "abrir", [
  addr(dono.publicKey()), bN(id),
  vec([nativeToScVal({ opcoes: 2, confidencial: true },
    { type: { opcoes: ["symbol", "u32"], confidencial: ["symbol", "bool"] } })]),
  bN("00".repeat(32)), vec([]),
  u32(0), u32(agora + 200), u32(agora + 600), xdr.ScVal.scvBool(true),
  u32(1), u32(LIMITE), u64(0), u64(0),
]);
if (!r1.ok) throw new Error(`abrir: ${r1.fase} ${r1.erro}`);
diz(`proposta ${id.slice(0, 12)}… aberta, sem lista e com limite ${LIMITE}`);
diz(`esperado: ${N} pessoas em ${Math.ceil(N / LIMITE)} seções`);

const gente = [];
for (let i = 0; i < N; i += 3) {
  gente.push(...(await Promise.all(
    Array.from({ length: Math.min(3, N - i) }, nascer),
  )));
}
diz(`${gente.length} identidades financiadas`);

const t0 = Date.now();
const rs = await Promise.all(gente.map((m) => {
  const k = wasm.nova_chave_de_anel();
  return tentar(m, "comparecer", [
    bN(id), addr(m.publicKey()), bN(k.publica), vec([]), u32(0), u32(0),
  ], { folga: FOLGA, extras: chavesDeAnel(id, 0, JANELA) });
}));

const ok = rs.filter((r) => r.ok);
console.log("");
diz(`${ok.length} de ${N} entraram, numa tentativa só · ${((Date.now() - t0) / 1000).toFixed(1)} s`);
const porLedger = {};
for (const r of ok) porLedger[r.ledger] = (porLedger[r.ledger] ?? 0) + 1;
diz(`ledgers: ${Object.entries(porLedger).map(([l, n]) => `${l}×${n}`).join(" · ")}`);
const erros = {};
for (const r of rs.filter((x) => !x.ok)) {
  const k = `${r.fase}: ${r.erro}`;
  erros[k] = (erros[k] ?? 0) + 1;
}
for (const [k, n] of Object.entries(erros)) diz(`✗ ${n}× ${k}`);

const secoes = await ler("secoes", bN(id));
const caderno = await ler("comparecimento", bN(id));
diz(`o contrato diz ${secoes} seção(ões) e caderno ${JSON.stringify(caderno)}`);
for (let s = 0; s < Math.max(1, secoes); s++) {
  const anel = await ler("anel", bN(id), u32(s));
  diz(`  seção ${s}: anel de ${anel.length}`);
}

console.log("");
if (ok.length === N) {
  diz("VEREDITO: as seis entraram. O mecanismo não tem buraco — o que a rajada");
  diz("de 20 esbarra é escala, e o próximo passo é medir onde.");
} else if (ok.length <= LIMITE) {
  diz("VEREDITO: só a primeira seção encheu. O mecanismo tem um buraco que o");
  diz("caso sequencial não mostra, e o §11-D continua aberto com um alvo.");
} else {
  diz(`VEREDITO: ${ok.length} entraram, mais que uma seção e menos que todas.`);
  diz("Não é nenhuma das duas hipóteses inteiras — ver os erros acima.");
}
console.log("");
