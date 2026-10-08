/**
 * Uma rodada em anel inteira, na testnet, sem interface nenhuma.
 *
 * É o teste de aceitação que nenhum `cargo test` alcança: os testes do contrato
 * provam que o host aceita a assinatura, e o portão 2 prova que o cliente monta
 * a cédula certa — mas ninguém ainda provou que o **XDR** que o JavaScript
 * monta chega ao contrato do jeito que ele espera. É o único trecho do caminho
 * sem cobertura, e é onde um `votar_anonimo` quebraria na frente de todo mundo.
 *
 * O que ele demonstra, e que é a tese do projeto:
 *
 *   caderno   7 endereços identificados, públicos, com prova de Merkle
 *   urna      7 cédulas de chaves de uso único, sem relação entre si
 *             nem com nenhum dos 7 do caderno
 *
 *   node scripts/rodada-anel.mjs
 */

import { createRequire } from "node:module";
import {
  Address,
  BASE_FEE,
  Contract,
  Keypair,
  Networks,
  TransactionBuilder,
  nativeToScVal,
  rpc,
  scValToNative,
  xdr,
} from "@stellar/stellar-sdk";

const require = createRequire(import.meta.url);
const wasm = require("../../cliente-wasm/pacote-node/tessera_cliente.js");

const RPC = "https://soroban-testnet.stellar.org";
const HORIZON = "https://horizon-testnet.stellar.org";
const FRIENDBOT = "https://friendbot.stellar.org";
const PASSPHRASE = Networks.TESTNET;
const CONTRATO = process.env.TESSERA_CONTRATO ??
  "CCQHRQZP3R3QMMKEEOOOS7XXINR7WGSMTKGNR4GD6BDNLC6JSPUZIDHZ";

const ELEITORADO = 10;
const COMPARECEM = 7;
const OPCOES = ["aprovar", "rejeitar"];

const servidor = new rpc.Server(RPC);
const contrato = new Contract(CONTRATO);

// ---------- utilidades ----------

const hexBytes = (h) => Buffer.from(h, "hex");
const bytesHex = (b) => Buffer.from(b).toString("hex");
const bN = (h) => xdr.ScVal.scvBytes(hexBytes(h));
// `Bls12381Fr` é `U256` no contrato, não `BytesN<32>`. Mandar bytes faz o
// wrapper do #[contractimpl] falhar ao desserializar — e ele não devolve erro,
// ele **trapa**: `UnreachableCodeReached`, antes de qualquer código do contrato
// rodar. Foi o que custou duas rodadas de testnet para achar.
const fr = (h) => nativeToScVal(BigInt("0x" + h), { type: "u256" });
const u32 = (n) => xdr.ScVal.scvU32(n);
const u64 = (n) => xdr.ScVal.scvU64(new xdr.Uint64(BigInt(n)));
// Sem fechadura de tempo nestas cargas: o criptograma vai vazio.
const semCripto = () => xdr.ScVal.scvBytes(new Uint8Array(0));
const vec = (v) => xdr.ScVal.scvVec(v);
const addr = (g) => new Address(g).toScVal();
// 44 bytes: SCV_ADDRESS ‖ ACCOUNT ‖ ED25519 ‖ chave. `toScAddress()` dá 40 e
// omite o discriminante do ScVal — a raiz de Merkle não bate e o contrato
// recusa com NaoEstaNaListaDeAptos, que não diz por quê.
const xdrDe = (g) => bytesHex(new Address(g).toScVal().toXDR());
const dorme = (ms) => new Promise((r) => setTimeout(r, ms));

let passo = 0;
const diz = (s) => console.log(`  ${s}`);
const titulo = (s) => console.log(`\n${String(++passo).padStart(2, "0")} · ${s}\n${"─".repeat(70)}`);

async function ledgerAtual() {
  const r = await fetch(`${HORIZON}/ledgers?order=desc&limit=1`);
  return (await r.json())._embedded.records[0].sequence;
}

async function nascer() {
  const par = Keypair.random();
  const r = await fetch(`${FRIENDBOT}?addr=${par.publicKey()}`);
  if (!r.ok && r.status !== 400) throw new Error(`friendbot ${r.status}`);
  return par;
}

async function ler(metodo, ...args) {
  const conta = await servidor.getAccount(leitor.publicKey());
  const tx = new TransactionBuilder(conta, { fee: BASE_FEE, networkPassphrase: PASSPHRASE })
    .addOperation(contrato.call(metodo, ...args))
    .setTimeout(30)
    .build();
  const sim = await servidor.simulateTransaction(tx);
  if (rpc.Api.isSimulationError(sim)) throw new Error(`${metodo}: ${sim.error}`);
  return scValToNative(sim.result.retval);
}

async function enviar(par, metodo, args) {
  const conta = await servidor.getAccount(par.publicKey());
  const bruta = new TransactionBuilder(conta, { fee: BASE_FEE, networkPassphrase: PASSPHRASE })
    .addOperation(contrato.call(metodo, ...args))
    .setTimeout(90)
    .build();
  const sim = await servidor.simulateTransaction(bruta);
  if (rpc.Api.isSimulationError(sim)) throw new Error(`${metodo}: ${sim.error}`);
  const pronta = rpc.assembleTransaction(bruta, sim).build();
  pronta.sign(par);
  const envio = await servidor.sendTransaction(pronta);
  if (envio.status === "ERROR") {
    throw new Error(`${metodo} recusado: ${JSON.stringify(envio.errorResult)}`);
  }
  for (let i = 0; i < 60; i++) {
    const r = await servidor.getTransaction(envio.hash);
    if (r.status === rpc.Api.GetTransactionStatus.SUCCESS) {
      return { hash: envio.hash, cpu: sim.cost?.cpuInsns, taxa: pronta.fee };
    }
    if (r.status === rpc.Api.GetTransactionStatus.FAILED) {
      throw new Error(`${metodo} falhou on-chain: ${envio.hash}`);
    }
    await dorme(1000);
  }
  throw new Error(`${metodo} não confirmou`);
}

const provaCds = (p) =>
  nativeToScVal(
    {
      a0: hexBytes(p.a0), a1: hexBytes(p.a1),
      e0: BigInt("0x" + p.e0), z0: BigInt("0x" + p.z0),
      e1: BigInt("0x" + p.e1), z1: BigInt("0x" + p.z1),
    },
    { type: { a0: ["symbol", null], a1: ["symbol", null],
              e0: ["symbol", "u256"], z0: ["symbol", "u256"],
              e1: ["symbol", "u256"], z1: ["symbol", "u256"] } },
  );

const provaSoma = (p) =>
  nativeToScVal({ a: hexBytes(p.a), z: BigInt("0x" + p.z) },
    { type: { a: ["symbol", null], z: ["symbol", "u256"] } });

// ---------- a rodada ----------

let leitor;

async function main() {
  console.log("\nTESSERA · uma rodada em anel, do zero, na testnet");
  console.log("─".repeat(70));
  console.log(`  contrato ... ${CONTRATO}`);

  titulo("o eleitorado nasce");
  leitor = await nascer();
  const membros = [];
  for (let i = 0; i < ELEITORADO; i++) membros.push(await nascer());
  diz(`${ELEITORADO} identidades financiadas pelo friendbot`);

  const enderecos = membros.map((m) => xdrDe(m.publicKey()));
  const pesos = membros.map(() => 1);
  const id = bytesHex(Keypair.random().rawPublicKey().subarray(0, 32));
  const raiz = wasm.raiz_de_aptos(id, enderecos, pesos, 1);
  diz(`raiz de aptos = ${raiz.slice(0, 16)}…${raiz.slice(-8)}`);

  titulo("abrir, com o caderno separado da urna");
  const agora = await ledgerAtual();
  // O comparecimento precisa de folga: são 7 transações, ~6 s de ledger cada.
  const abreEm = agora + 90;
  const fechaEm = abreEm + 200;
  const mesa = [];
  for (let i = 0; i < 5; i++) mesa.push((await nascer()).publicKey());

  const r1 = await enviar(membros[0], "abrir", [
    addr(membros[0].publicKey()),
    bN(id),
    vec([nativeToScVal({ opcoes: OPCOES.length, confidencial: true },
      { type: { opcoes: ["symbol", "u32"], confidencial: ["symbol", "bool"] } })]),
    bN(raiz),
    vec(mesa.map(addr)),
    u32(3),
    u32(abreEm),
    u32(fechaEm),
    xdr.ScVal.scvBool(true),
    u32(1),      // secoes
    u32(0),      // limite_secao
    u64(0),      // fim_tempo
    u64(0),      // rodada
  ]);
  diz(`proposta ${id.slice(0, 12)}…`);
  diz(`comparecimento até o ledger ${abreEm} · votação até ${fechaEm}`);
  diz(`tx ${r1.hash}`);

  const h = bytesHex(await ler("gerador_h"));
  const hp = bytesHex(await ler("hp", bN(id)));
  diz(`H  = ${h.slice(0, 16)}…`);
  diz(`Hp = ${hp.slice(0, 16)}…   (por proposta: a imagem não atravessa votações)`);

  titulo("o caderno · identificado, público, e é ele que diz quem faltou");
  const chaves = [];
  for (let i = 0; i < COMPARECEM; i++) {
    const k = wasm.nova_chave_de_anel();
    chaves.push(k);
    const c = wasm.caminho_de(id, enderecos, pesos, 1, i);
    const r = await enviar(membros[i], "comparecer", [
      bN(id), addr(membros[i].publicKey()), bN(k.publica),
      vec(c.irmaos.map(bN)), u32(c.indice), u32(c.secao),
    ]);
    diz(`${membros[i].publicKey().slice(0, 8)}…  compareceu · anel com ${i + 1}`);
    if (i === 0) diz(`   cpu ${r.cpu} · taxa ${r.taxa} stroops`);
  }

  const faltaram = membros.slice(COMPARECEM).map((m) => m.publicKey().slice(0, 8));
  diz(`faltaram: ${faltaram.join(", ")}  ← a lista da penalidade`);

  titulo("espera a janela virar");
  for (;;) {
    const l = await ledgerAtual();
    if (l >= abreEm) break;
    diz(`ledger ${l} / ${abreEm}`);
    await dorme(20000);
  }

  const anel = (await ler("anel", bN(id), u32(0))).map(bytesHex);
  diz(`anel congelado com ${anel.length} chaves`);

  titulo("a urna · cada cédula de uma chave de uso único");
  const efemeras = [];
  for (let i = 0; i < COMPARECEM; i++) {
    const escolha = i < 5 ? 0 : 1;
    const c = wasm.cedula_anonima(
      id, hp, h, anel, i, chaves[i].secreta,
      [{ opcoes: OPCOES.length, confidencial: true }], [escolha],
    );
    const par = await nascer();
    efemeras.push(par.publicKey());
    const r = await enviar(par, "votar_anonimo", [
      bN(id), u32(0), vec(anel.map(bN)), bN(c.imagem), fr(c.c0), vec(c.z.map(fr)),
      vec(c.cedula.compromissos.map(bN)),
      vec(c.cedula.provas.map(provaCds)),
      vec(c.cedula.provas_soma.map(provaSoma)),
      vec(c.cedula.escolhas.map(u32)),
      semCripto(),
    ]);
    diz(`cédula ${i + 1} de ${COMPARECEM} · de ${par.publicKey().slice(0, 8)}… · imagem ${c.imagem.slice(0, 12)}…`);
    if (i === 0) diz(`   cpu ${r.cpu} · taxa ${r.taxa} stroops · anel de ${anel.length}`);
  }

  titulo("o que o ledger sabe, e o que não sabe");
  const [conf] = await ler("comparecimento", bN(id));
  diz(`cédulas na urna ....... ${conf}`);
  diz(`no caderno ............ ${COMPARECEM} de ${ELEITORADO}`);
  diz("");
  diz("nenhum dos endereços do caderno assinou uma cédula:");
  const cruzamento = efemeras.filter((e) => membros.some((m) => m.publicKey() === e));
  diz(`  interseção caderno ∩ urna = ${cruzamento.length}`);
  if (cruzamento.length !== 0) throw new Error("o vínculo não foi quebrado");

  // Uma segunda cédula da mesma pessoa tem de colidir na imagem.
  const repetida = wasm.cedula_anonima(
    id, hp, h, anel, 2, chaves[2].secreta,
    [{ opcoes: OPCOES.length, confidencial: true }], [1],
  );
  const par = await nascer();
  try {
    await enviar(par, "votar_anonimo", [
      bN(id), u32(0), vec(anel.map(bN)), bN(repetida.imagem), fr(repetida.c0), vec(repetida.z.map(fr)),
      vec(repetida.cedula.compromissos.map(bN)),
      vec(repetida.cedula.provas.map(provaCds)),
      vec(repetida.cedula.provas_soma.map(provaSoma)),
      vec(repetida.cedula.escolhas.map(u32)),
      semCripto(),
    ]);
    throw new Error("a segunda cédula da mesma pessoa passou");
  } catch (e) {
    if (String(e).includes("passou")) throw e;
    diz(`segunda cédula do mesmo membro, de outro endereço: recusada`);
    diz(`  ${String(e).slice(0, 120)}`);
  }

  console.log(`\n  proposta ${id}`);
  console.log(`  https://stellar.expert/explorer/testnet/contract/${CONTRATO}\n`);
}

main().catch((e) => {
  console.error("\n✗", e.message ?? e);
  process.exit(1);
});
