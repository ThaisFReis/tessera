/**
 * A fechadura de tempo, ponta a ponta, na testnet.
 *
 * `rodada-anel.mjs` prova que o XDR da cédula chega ao contrato. `rodada-30.mjs`
 * prova que trinta cabem. Nenhum dos dois toca no que T-017 a T-020 construíram:
 * uma assembleia **sem mesa** que ainda assim tem placar, porque a chave que
 * abre as cédulas não está com ninguém — ela nasce sozinha, no instante de uma
 * rodada da baliza drand.
 *
 * O que este script tem a demonstrar, em ordem:
 *
 *   1. a cédula entra **cifrada**: o criptograma vai no evento, e nem quem
 *      votou pode abri-lo antes da hora;
 *   2. antes do fim da janela o contrato recusa qualquer placar — nem parcial;
 *   3. depois do fim, mas antes da rodada vencer, ele recusa também, e por
 *      outro motivo;
 *   4. quando a rodada vence, **qualquer pessoa** decifra e apura. Aqui é uma
 *      carteira descartável que acabou de nascer, que não votou e que ninguém
 *      autorizou;
 *   5. uma cédula sabotada — um byte trocado no criptograma — **não trava o
 *      placar**: custa só o próprio voto;
 *   6. quem apura omitindo cédulas honestas é **sobreposto** por quem as
 *      inclui, e a tentativa de voltar atrás é recusada.
 *
 * O 5 e o 6 juntos são a resposta a "não pode ser possível travar o placar".
 *
 *   node scripts/rodada-relogio.mjs
 */

import { createRequire } from "node:module";
import {
  Address, Contract, Keypair, Networks,
  TransactionBuilder, nativeToScVal, rpc, scValToNative, xdr,
} from "@stellar/stellar-sdk";

import { comTeto, instrucoes } from "./medir.mjs";

const require = createRequire(import.meta.url);
const wasm = require("../../cliente-wasm/pacote-node/tessera_cliente.js");

const RPC = "https://soroban-testnet.stellar.org";
const HORIZON = "https://horizon-testnet.stellar.org";
const FRIENDBOT = "https://friendbot.stellar.org";
const PASSPHRASE = Networks.TESTNET;
const CONTRATO = process.env.TESSERA_CONTRATO ??
  "CAZVUPKVXCV6CB2V2LC4OY5FHU3HG5OVIDMSQB4Z7VST36XEZMIDWILH";
/** A cadeia `quicknet` da drand. Ver docs/SOURCES.md. */
const CADEIA_BALIZA = "52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971";

const N = Number(process.env.N ?? 5);
const OPCOES = 2;
/** Quem leva o byte trocado no criptograma. `-1` desliga a sabotagem. */
const SABOTADA = Number(process.env.SABOTADA ?? 2);
const TAXA_INCLUSAO = Number(process.env.TAXA_INCLUSAO ?? 1_000_000);
/** Segundos de sobra entre o fim previsto da janela e o vencimento da rodada. */
const FOLGA_RODADA = Number(process.env.FOLGA_RODADA ?? 60);
/** Segundos por ledger na testnet, para converter a janela em tempo. */
const SEGUNDOS_POR_LEDGER = 5;

const servidor = new rpc.Server(RPC);
const contrato = new Contract(CONTRATO);

const hexBytes = (h) => Buffer.from(h, "hex");
const bytesHex = (b) => Buffer.from(b).toString("hex");
const bN = (h) => xdr.ScVal.scvBytes(hexBytes(h));
const bytes = (h) => xdr.ScVal.scvBytes(hexBytes(h));
const fr = (h) => nativeToScVal(BigInt("0x" + h), { type: "u256" });
const u32 = (n) => xdr.ScVal.scvU32(n);
const u64 = (n) => xdr.ScVal.scvU64(new xdr.Uint64(BigInt(n)));
const bool = (b) => xdr.ScVal.scvBool(b);
const vec = (v) => xdr.ScVal.scvVec(v);
const addr = (g) => new Address(g).toScVal();
const xdrDe = (g) => bytesHex(new Address(g).toScVal().toXDR());
const dorme = (ms) => new Promise((r) => setTimeout(r, ms));
const agoraSeg = () => Math.floor(Date.now() / 1000);

let passo = 0;
const diz = (s) => console.log(`  ${s}`);
const titulo = (s) => console.log(`\n${String(++passo).padStart(2, "0")} · ${s}\n${"─".repeat(70)}`);

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
  throw new Error("o friendbot não financiou");
}

let leitor;
async function ler(metodo, ...args) {
  const conta = await servidor.getAccount(leitor.publicKey());
  const tx = new TransactionBuilder(conta, { fee: "100", networkPassphrase: PASSPHRASE })
    .addOperation(contrato.call(metodo, ...args))
    .setTimeout(30).build();
  const sim = await servidor.simulateTransaction(tx);
  if (rpc.Api.isSimulationError(sim)) throw new Error(`${metodo}: ${sim.error}`);
  return scValToNative(sim.result.retval);
}

/** Não lança: este script precisa **mostrar** recusas, não parar nelas. */
async function tentar(par, metodo, args, lance = TAXA_INCLUSAO) {
  try {
    const conta = await servidor.getAccount(par.publicKey());
    const bruta = new TransactionBuilder(conta, { fee: String(lance), networkPassphrase: PASSPHRASE })
      .addOperation(contrato.call(metodo, ...args))
      .setTimeout(180).build();
    const sim = await servidor.simulateTransaction(bruta);
    if (rpc.Api.isSimulationError(sim)) {
      return { ok: false, fase: "simulação", erro: sim.error.split("\n")[0] };
    }
    // `sim.cost.cpuInsns` não existe no SDK 14.6.1 — ver `medir.mjs`, que
    // lança em vez de devolver um zero que ninguém mediu.
    const cpu = instrucoes(sim, metodo);
    const pronta = rpc.assembleTransaction(bruta, sim).build();
    pronta.sign(par);
    const envio = await servidor.sendTransaction(pronta);
    if (envio.status === "ERROR") {
      return { ok: false, fase: "envio", cpu,
               erro: envio.errorResult?.result().switch().name ?? "?" };
    }
    for (let i = 0; i < 120; i++) {
      const r = await servidor.getTransaction(envio.hash);
      if (r.status === rpc.Api.GetTransactionStatus.SUCCESS) {
        return { ok: true, cpu, hash: envio.hash, ledger: r.ledger,
                 valor: r.returnValue ? scValToNative(r.returnValue) : undefined };
      }
      if (r.status === rpc.Api.GetTransactionStatus.FAILED) {
        return { ok: false, fase: "aplicação", cpu, erro: `on-chain ${envio.hash}` };
      }
      await dorme(1000);
    }
    return { ok: false, fase: "espera", cpu, erro: "preterida até expirar" };
  } catch (e) {
    return { ok: false, fase: "exceção", erro: String(e.message ?? e).split("\n")[0].slice(0, 200) };
  }
}

const provaCds = (p) => nativeToScVal(
  { a0: hexBytes(p.a0), a1: hexBytes(p.a1), e0: BigInt("0x" + p.e0),
    z0: BigInt("0x" + p.z0), e1: BigInt("0x" + p.e1), z1: BigInt("0x" + p.z1) },
  { type: { a0: ["symbol", null], a1: ["symbol", null], e0: ["symbol", "u256"],
            z0: ["symbol", "u256"], e1: ["symbol", "u256"], z1: ["symbol", "u256"] } });

const provaSoma = (p) => nativeToScVal(
  { a: hexBytes(p.a), z: BigInt("0x" + p.z) },
  { type: { a: ["symbol", null], z: ["symbol", "u256"] } });

/** Troca um dígito hexadecimal, sem mudar o tamanho. */
const trocarUmByte = (h) =>
  h.slice(0, -1) + (h.at(-1) === "a" ? "b" : "a");

/**
 * As cédulas como quem apura as encontra: lidas dos **eventos**, na ordem em
 * que entraram — que é a ordem da cadeia de compromissos que o contrato
 * re-encadeia. É o mesmo caminho de `rede.ts::cedulasDaSecao`.
 */
async function cedulasDosEventos(id, desde) {
  const r = await servidor.getEvents({
    startLedger: desde,
    filters: [{ type: "contract", contractIds: [CONTRATO] }],
  });
  return r.events
    .filter((e) => scValToNative(e.topic[0]) === "anonimo")
    .filter((e) => bytesHex(scValToNative(e.topic[1])) === id)
    .map((e) => {
      const v = scValToNative(e.value);
      return { compromissos: v[1].map(bytesHex), cripto: bytesHex(v[3]) };
    });
}

const hashes = [];
const anotar = (rotulo, r) => {
  if (r.hash) hashes.push([rotulo, r.hash]);
  return r;
};

async function main() {
  console.log("\nTESSERA · a fechadura de tempo, ponta a ponta, na testnet");
  console.log("─".repeat(70));
  console.log(`  contrato ... ${CONTRATO}`);

  titulo("o eleitorado nasce");
  leitor = await nascer();
  const membros = [];
  for (let i = 0; i < N; i++) membros.push(await nascer());
  const enderecos = membros.map((m) => xdrDe(m.publicKey()));
  const pesos = membros.map(() => 1);
  const id = bytesHex(Keypair.random().rawPublicKey().subarray(0, 32));
  const raiz = wasm.raiz_de_aptos(enderecos, pesos);
  diz(`${N} identidades financiadas · raiz de aptos ${raiz.slice(0, 16)}…`);
  diz("uma seção só: a divisão em seções é outro assunto (§11-N)");

  titulo("abrir, sem mesa e com fechadura de tempo");
  const agora = await ledgerAtual();
  const abreEm = agora + 12 + 2 * N;
  const fechaEm = abreEm + 12 + 2 * N;
  // A rodada que destranca as cédulas: a que vence pouco depois de a janela
  // fechar. Tem de estar no futuro agora, senão o contrato recusa com
  // `RodadaNaoFecha` — uma rodada vencida já tem assinatura publicada, e as
  // cédulas abririam na hora de serem depositadas.
  const quandoFecha = agoraSeg() + (fechaEm - agora) * SEGUNDOS_POR_LEDGER;
  const rodada = Number(wasm.rodada_em(BigInt(quandoFecha + FOLGA_RODADA)));
  const venceEm = Number(wasm.instante_da_rodada(BigInt(rodada)));

  const r1 = anotar("abrir", await tentar(membros[0], "abrir", [
    addr(membros[0].publicKey()), bN(id),
    vec([nativeToScVal({ opcoes: OPCOES, confidencial: true },
      { type: { opcoes: ["symbol", "u32"], confidencial: ["symbol", "bool"] } })]),
    bN(raiz),
    vec([]),      // mesa: nenhuma. É o caso que não tinha placar.
    u32(0),       // limiar 0, que só é aceito com mesa vazia
    u32(abreEm), u32(fechaEm),
    bool(true),   // anel
    u32(1),       // uma seção
    u32(0),       // sem limite por seção
    u64(rodada),  // a fechadura
    u64(0),       // sem rodada de abertura: uma seção não precisa dela
  ]));
  if (!r1.ok) throw new Error(`abrir falhou — ${r1.fase}: ${r1.erro}`);
  diz(`proposta ${id.slice(0, 12)}… · sem mesa nenhuma`);
  diz(`comparecimento até o ledger ${abreEm} · votação até ${fechaEm}`);
  diz(`fechadura: rodada ${rodada} da baliza, que vence em ${venceEm - agoraSeg()}s`);
  diz(`tx ${r1.hash}`);

  const h = bytesHex(await ler("gerador_h"));
  const hp = bytesHex(await ler("hp", bN(id)));

  titulo("o caderno · identificado e público");
  const chaves = [];
  for (let i = 0; i < N; i++) {
    const k = wasm.nova_chave_de_anel();
    chaves.push(k);
    const c = wasm.caminho_de(enderecos, pesos, i);
    const r = await tentar(membros[i], "comparecer", [
      bN(id), addr(membros[i].publicKey()), bN(k.publica),
      vec(c.irmaos.map(bN)), u32(c.indice), u32(0),
    ]);
    if (!r.ok) throw new Error(`comparecer ${i}: ${r.fase} ${r.erro}`);
    diz(`${membros[i].publicKey().slice(0, 8)}…  compareceu · anel com ${i + 1}`);
  }

  titulo("espera a janela de votação abrir");
  for (;;) {
    const l = await ledgerAtual();
    if (l >= abreEm) break;
    process.stdout.write(`\r  ledger ${l} / ${abreEm}…`);
    await dorme(10000);
  }
  process.stdout.write("\r" + " ".repeat(44) + "\r");
  const anel = (await ler("anel", bN(id), u32(0))).map(bytesHex);
  diz(`anel congelado com ${anel.length} chaves`);

  titulo("a urna · cada cédula cifrada para uma rodada que ainda não existe");
  const escolhas = Array.from({ length: anel.length }, (_, i) => (i === 0 || i === 4 ? 0 : 1));
  const ledgerDaUrna = await ledgerAtual();
  const enviadas = [];
  for (let i = 0; i < anel.length; i++) {
    const c = wasm.cedula_anonima(
      id, hp, h, anel, i, chaves[i].secreta,
      [{ opcoes: OPCOES, confidencial: true }], [escolhas[i]],
      // **BigInt, não Number.** O `u64` do wasm-bindgen recusa um `Number` com
      // "Cannot convert N to a BigInt", e recusa na primeira cédula — o que
      // custou uma rodada inteira de testnet.
      BigInt(rodada),
    );
    const sabota = i === SABOTADA;
    const cripto = sabota ? trocarUmByte(c.cedula.cripto) : c.cedula.cripto;
    const par = await nascer();
    const r = await tentar(par, "votar_anonimo", [
      bN(id), u32(0), vec(anel.map(bN)), bN(c.imagem), fr(c.c0), vec(c.z.map(fr)),
      vec(c.cedula.compromissos.map(bN)),
      vec(c.cedula.provas.map(provaCds)),
      vec(c.cedula.provas_soma.map(provaSoma)),
      vec(c.cedula.escolhas.map(u32)),
      bytes(cripto),
    ]);
    if (!r.ok) throw new Error(`votar_anonimo ${i}: ${r.fase} ${r.erro}`);
    enviadas.push({ compromissos: c.cedula.compromissos, escolha: escolhas[i], sabota });
    diz(`cédula ${i + 1} de ${anel.length} · ${c.cedula.cripto.length / 2} bytes de criptograma` +
        (sabota ? "  ← um byte trocado, de propósito" : ""));
    if (i === 0) diz(`   ${comTeto(r.cpu)}`);
  }
  diz("o contrato aceitou a sabotada: ele confere a **forma** do criptograma, e");
  diz("não pode conferir o conteúdo — o host não tem pareamento com valor.");

  titulo("antes do fim da janela, nenhum placar — nem parcial");
  const apurador = await nascer();
  const listaCompleta = enviadas.flatMap((c) => c.compromissos);
  const cedo = await tentar(apurador, "apurar_secao", [
    bN(id), addr(apurador.publicKey()), u32(0),
    vec(listaCompleta.map(bN)),
    vec(enviadas.map(() => bool(true))),
    vec([u32(0), u32(0)]),
    vec(["0".repeat(64), "0".repeat(64)].map(fr)),
  ]);
  if (cedo.ok) throw new Error("apurou com a votação aberta");
  diz(`recusado: ${cedo.fase} · ${cedo.erro}`);
  diz("é o #7 VotacaoAindaAberta, e é o relógio do ledger — sem suposição nenhuma");

  titulo("espera a janela fechar");
  for (;;) {
    const l = await ledgerAtual();
    if (l >= fechaEm) break;
    process.stdout.write(`\r  ledger ${l} / ${fechaEm}…`);
    await dorme(10000);
  }
  process.stdout.write("\r" + " ".repeat(44) + "\r");
  diz(`a urna fechou no ledger ${fechaEm}`);

  if (agoraSeg() < venceEm) {
    titulo("a urna fechou, o relógio ainda não abriu");
    const antes = await tentar(apurador, "apurar_secao", [
      bN(id), addr(apurador.publicKey()), u32(0),
      vec(listaCompleta.map(bN)),
      vec(enviadas.map(() => bool(true))),
      vec([u32(0), u32(0)]),
      vec(["0".repeat(64), "0".repeat(64)].map(fr)),
    ]);
    if (antes.ok) throw new Error("apurou antes de a rodada vencer");
    diz(`recusado: ${antes.fase} · ${antes.erro}`);
    diz("agora é o #35 RelogioAindaNaoAbriu: outro portão, outro motivo");
    while (agoraSeg() < venceEm) {
      process.stdout.write(`\r  a rodada ${rodada} vence em ${venceEm - agoraSeg()}s…`);
      await dorme(3000);
    }
    process.stdout.write("\r" + " ".repeat(44) + "\r");
  }

  titulo("a rodada venceu · a chave que abre as cédulas nasceu sozinha");
  const rb = await fetch(`https://api.drand.sh/v2/chains/${CADEIA_BALIZA}/rounds/${rodada}`);
  if (!rb.ok) throw new Error(`a baliza não publicou a rodada ${rodada}: HTTP ${rb.status}`);
  const { signature } = await rb.json();
  diz(`assinatura da rodada ${rodada}: ${signature.slice(0, 24)}…`);
  // **Conferida antes de usar.** O relé é um servidor como qualquer outro.
  wasm.conferir_baliza(BigInt(rodada), signature);
  diz("conferida no pareamento contra a chave pública congelada da cadeia");

  titulo("quem apura é quem quiser · leu os eventos, decifrou, somou");
  const doLedger = await cedulasDosEventos(id, ledgerDaUrna);
  diz(`${doLedger.length} cédulas lidas dos eventos do contrato`);
  const mesmaOrdem = doLedger.every((c, i) =>
    c.compromissos.join() === enviadas[i].compromissos.join());
  if (!mesmaOrdem) throw new Error("os eventos vieram em outra ordem que a cadeia");
  diz("na mesma ordem em que entraram — é a ordem que a cadeia prende");

  const compromissos = doLedger.flatMap((c) => c.compromissos);
  const a = wasm.abertura_da_secao(
    compromissos, doLedger.map((c) => c.cripto), OPCOES, h, BigInt(rodada), signature,
  );
  diz(`abriram ${a.abriram} de ${a.cedulas} · placar decifrado ${Array.from(a.totais).join(" · ")}`);
  const esperado = enviadas.reduce((acc, c) => {
    if (!c.sabota) acc[c.escolha]++;
    return acc;
  }, new Array(OPCOES).fill(0));
  if (Array.from(a.totais).join() !== esperado.join()) {
    throw new Error(`o placar decifrado ${Array.from(a.totais)} não bate com o esperado ${esperado}`);
  }
  diz(`bate com o que foi votado, descontada a sabotada: ${esperado.join(" · ")}`);

  // ---- 6. omitir não gruda ----
  titulo("quem omite cédulas honestas é sobreposto");
  const parcial = a.abertas.map((b, i) => b && i < 2);
  const quantasParciais = parcial.filter(Boolean).length;
  const totaisParciais = new Array(OPCOES).fill(0);
  const aberturasParciais = new Array(OPCOES).fill(0n);
  // Refaz a conta só sobre as duas primeiras: a abertura é a soma dos fatores,
  // e sem elas o MSM não fecharia.
  const so2 = wasm.abertura_da_secao(
    doLedger.slice(0, 2).flatMap((c) => c.compromissos),
    doLedger.slice(0, 2).map((c) => c.cripto),
    OPCOES, h, BigInt(rodada), signature,
  );
  for (let j = 0; j < OPCOES; j++) {
    totaisParciais[j] = so2.totais[j];
    aberturasParciais[j] = so2.aberturas[j];
  }
  const rp = anotar("apurar_secao (parcial)", await tentar(apurador, "apurar_secao", [
    bN(id), addr(apurador.publicKey()), u32(0),
    vec(compromissos.map(bN)),
    vec(parcial.map(bool)),
    vec(totaisParciais.map((n) => u32(Number(n)))),
    vec(aberturasParciais.map((x) => fr(x))),
  ]));
  if (!rp.ok) throw new Error(`a apuração parcial falhou: ${rp.fase} ${rp.erro}`);
  diz(`uma apuração de ${quantasParciais} cédulas entrou primeiro · tx ${rp.hash}`);
  diz(`  no ledger: ${JSON.stringify(await ler("resultado_secao", bN(id), u32(0)))}`);

  const outro = await nascer();
  const rc2 = anotar("apurar_secao (completa)", await tentar(outro, "apurar_secao", [
    bN(id), addr(outro.publicKey()), u32(0),
    vec(compromissos.map(bN)),
    vec(a.abertas.map(bool)),
    vec(Array.from(a.totais).map((n) => u32(Number(n)))),
    vec(a.aberturas.map((x) => fr(x))),
  ]));
  if (!rc2.ok) throw new Error(`a apuração completa falhou: ${rc2.fase} ${rc2.erro}`);
  diz(`outra carteira, que não votou, incluiu as ${a.abriram} e sobrepôs · tx ${rc2.hash}`);
  diz(`  ${comTeto(rc2.cpu)}`);

  const volta = await tentar(apurador, "apurar_secao", [
    bN(id), addr(apurador.publicKey()), u32(0),
    vec(compromissos.map(bN)),
    vec(parcial.map(bool)),
    vec(totaisParciais.map((n) => u32(Number(n)))),
    vec(aberturasParciais.map((x) => fr(x))),
  ]);
  if (volta.ok) throw new Error("voltar atrás no placar foi aceito");
  diz(`tentar voltar ao placar menor: recusado · ${volta.erro}`);
  diz("é o #37 NaoMelhora — só um conjunto estritamente maior substitui o guardado");

  titulo("o que ficou no ledger");
  const final = await ler("resultado_secao", bN(id), u32(0));
  diz(`placar ............ ${final[1].join(" · ")}`);
  diz(`cédulas abertas ... ${final[0]} de ${enviadas.length}`);
  diz(`a sabotada ........ custou um voto, e só o dela`);
  diz(`mesa .............. nenhuma, em nenhum momento`);
  console.log("");
  for (const [rotulo, hash] of hashes) console.log(`  ${rotulo.padEnd(26)} ${hash}`);
  console.log(`\n  proposta ${id}`);
  console.log(`  https://stellar.expert/explorer/testnet/contract/${CONTRATO}\n`);
}

main().catch((e) => {
  console.error("\n✗", e.message ?? e);
  process.exit(1);
});
