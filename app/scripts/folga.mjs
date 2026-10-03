/**
 * A hipótese barata para o gargalo do caderno.
 *
 * `comparecer()` faz read-modify-write em `Chave::Anel`, uma entrada que cresce
 * 96 bytes por pessoa. A simulação declara `writeBytes` para o anel **de
 * agora**; quem aplica depois precisa gravar um anel maior do que declarou, e a
 * rede recusa. Medido: uma por ledger, 262 transações para 30 pessoas.
 *
 * Mas o footprint é uma *declaração*, não uma medição. Se o cliente declarar
 * espaço para o anel cheio desde a primeira, a declaração continua válida não
 * importa quantas entraram antes — e isso é mudança só de cliente, sem tocar no
 * contrato nem redeployar.
 *
 * Este script dispara N comparecimentos de uma vez, com e sem folga, e compara.
 *
 *   node scripts/folga.mjs
 *   N=15 FOLGA=4096 node scripts/folga.mjs
 */

import { createRequire } from "node:module";
import {
  Address, Contract, Keypair, Networks, SorobanDataBuilder,
  TransactionBuilder, nativeToScVal, rpc, xdr,
} from "@stellar/stellar-sdk";

const require = createRequire(import.meta.url);
const wasm = require("../../cliente-wasm/pacote-node/tessera_cliente.js");

const RPC = "https://soroban-testnet.stellar.org";
const HORIZON = "https://horizon-testnet.stellar.org";
const FRIENDBOT = "https://friendbot.stellar.org";
const PASSPHRASE = Networks.TESTNET;
const CONTRATO = process.env.TESSERA_CONTRATO ??
  "CB6WIY45JYIR6EN6NC3WOAEOHYSKMXHKPDEXCHNY3O4C2O2RJ4RBQIJ6";

const N = Number(process.env.N ?? 15);
/** Bytes a mais de escrita declarada. 96 por membro do anel, com sobra. */
const FOLGA = Number(process.env.FOLGA ?? 8192);
const TAXA_INCLUSAO = 1_000_000;

const servidor = new rpc.Server(RPC);
const contrato = new Contract(CONTRATO);

const hexBytes = (h) => Buffer.from(h, "hex");
const bytesHex = (b) => Buffer.from(b).toString("hex");
const bN = (h) => xdr.ScVal.scvBytes(hexBytes(h));
const u32 = (n) => xdr.ScVal.scvU32(n);
const vec = (v) => xdr.ScVal.scvVec(v);
const addr = (g) => new Address(g).toScVal();
const xdrDe = (g) => bytesHex(new Address(g).toScVal().toXDR());
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
  throw new Error("friendbot");
}
async function nascerVarios(n, lote = 5) {
  const todos = [];
  for (let i = 0; i < n; i += lote) {
    todos.push(...(await Promise.all(Array.from({ length: Math.min(lote, n - i) }, nascer))));
  }
  return todos;
}

/** `folga = 0` reproduz o comportamento de hoje. */
async function tentar(par, metodo, args, folga) {
  try {
    const conta = await servidor.getAccount(par.publicKey());
    const bruta = new TransactionBuilder(conta, {
      fee: String(TAXA_INCLUSAO), networkPassphrase: PASSPHRASE,
    }).addOperation(contrato.call(metodo, ...args)).setTimeout(180).build();

    const sim = await servidor.simulateTransaction(bruta);
    if (rpc.Api.isSimulationError(sim)) return { ok: false, fase: "simulação", erro: sim.error.split("\n")[0] };

    // `assembleTransaction` já prende a autorização e o footprint da simulação.
    let pronta = rpc.assembleTransaction(bruta, sim).build();
    let declarado = pronta.toEnvelope().v1().tx().ext().sorobanData().resources().writeBytes();

    if (folga > 0) {
      const dados = new SorobanDataBuilder(
        pronta.toEnvelope().v1().tx().ext().sorobanData().toXDR("base64"),
      );
      const r = dados.build().resources();
      dados.setResources(r.instructions(), r.diskReadBytes(), r.writeBytes() + folga);
      // Os bytes a mais custam taxa. Pagar com sobra é barato; errar para menos
      // devolve `txInsufficientFee` e perde a vez.
      const taxa = BigInt(pronta.fee) + BigInt(folga) * BigInt(process.env.POR_BYTE ?? 200);
      const antes = pronta.sequence;
      pronta = TransactionBuilder.cloneFrom(pronta, {
        fee: taxa.toString(), sorobanData: dados.build(),
      }).build();
      if (pronta.sequence !== antes) {
        return { ok: false, fase: "clone", erro: `sequência mudou: ${antes} → ${pronta.sequence}` };
      }
      declarado = r.writeBytes() + folga;
    }

    pronta.sign(par);
    const envio = await servidor.sendTransaction(pronta);
    if (envio.status === "ERROR") {
      return { ok: false, fase: "envio", declarado,
               erro: envio.errorResult?.result().switch().name ?? "?" };
    }
    for (let i = 0; i < 180; i++) {
      const r = await servidor.getTransaction(envio.hash);
      if (r.status === rpc.Api.GetTransactionStatus.SUCCESS) {
        return { ok: true, declarado, ledger: r.ledger, taxa: Number(pronta.fee),
                 cobrado: Number(r.resultXdr?.feeCharged?.()?.toString() ?? 0) };
      }
      if (r.status === rpc.Api.GetTransactionStatus.FAILED) {
        return { ok: false, fase: "aplicação", declarado, ledger: r.ledger, erro: "txFailed" };
      }
      await dorme(1000);
    }
    return { ok: false, fase: "espera", declarado, erro: "expirou" };
  } catch (e) {
    return { ok: false, fase: "exceção", erro: String(e.message ?? e).split("\n")[0].slice(0, 120) };
  }
}

async function rodada(rotulo, folga) {
  const membros = await nascerVarios(N);
  const enderecos = membros.map((m) => xdrDe(m.publicKey()));
  const pesos = membros.map(() => 1);
  const raiz = wasm.raiz_de_aptos(enderecos, pesos);

  const agora = await ledgerAtual();
  const id = bytesHex(Keypair.random().rawPublicKey().subarray(0, 32));
  const r1 = await tentar(membros[0], "abrir", [
    addr(membros[0].publicKey()), bN(id),
    vec([nativeToScVal({ opcoes: 2, confidencial: true },
      { type: { opcoes: ["symbol", "u32"], confidencial: ["symbol", "bool"] } })]),
    bN(raiz), vec([addr(membros[0].publicKey())]),
    u32(1), u32(agora + 120), u32(agora + 400), xdr.ScVal.scvBool(true),
  ], 0);
  if (!r1.ok) throw new Error(`abrir: ${r1.fase} ${r1.erro}`);

  console.log(`\n${rotulo}`);
  console.log("─".repeat(70));
  const t0 = Date.now();
  const rs = await Promise.all(membros.map((m, i) => {
    const c = wasm.caminho_de(enderecos, pesos, i);
    const k = wasm.nova_chave_de_anel();
    return tentar(m, "comparecer", [
      bN(id), addr(m.publicKey()), bN(k.publica), vec(c.irmaos.map(bN)), u32(c.indice),
    ], folga);
  }));

  const ok = rs.filter((r) => r.ok);
  const ledgers = [...new Set(ok.map((r) => r.ledger))];
  diz(`${ok.length} de ${N} entraram, numa tentativa só`);
  diz(`writeBytes declarado: ${[...new Set(rs.map((r) => r.declarado))].join(", ")}`);
  if (ok.length) {
    const porLedger = {};
    for (const r of ok) porLedger[r.ledger] = (porLedger[r.ledger] ?? 0) + 1;
    diz(`${ledgers.length} ledgers · por ledger: ${Object.values(porLedger).join(", ")}`);
    diz(`taxa declarada: ${Math.max(...ok.map((r) => r.taxa)).toLocaleString("pt-BR")} stroops`);
    diz(`taxa COBRADA:   ${Math.max(...ok.map((r) => r.cobrado)).toLocaleString("pt-BR")} stroops  ← o que sai da conta`);
  }
  const erros = {};
  for (const r of rs.filter((x) => !x.ok)) erros[`${r.fase}: ${r.erro}`] = (erros[`${r.fase}: ${r.erro}`] ?? 0) + 1;
  for (const [k, n] of Object.entries(erros)) diz(`✗ ${n}× ${k}`);
  diz(`${((Date.now() - t0) / 1000).toFixed(1)} s`);
  return ok.length;
}

console.log(`\nTESSERA · ${N} comparecimentos simultâneos, com e sem folga de escrita`);
const sem = await rodada(`SEM FOLGA — o que o dapp faz hoje`, 0);
const com = await rodada(`COM FOLGA de ${FOLGA} bytes declarados a mais`, FOLGA);
console.log(`\n  ${sem}/${N} sem folga  ·  ${com}/${N} com folga\n`);
