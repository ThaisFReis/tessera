/**
 * Trinta votantes ao mesmo tempo, num anel só, na testnet.
 *
 * `rodada-anel.mjs` manda uma transação por vez e prova que o caminho funciona.
 * Este manda **todas de uma vez**, e é outra pergunta: o custo de CPU por
 * cédula já foi medido em `ate_quantas_pessoas_cabe_um_anel` (365.680.568 para
 * trinta, 91,4% do teto), mas nenhum teste tocou no que acontece quando trinta
 * transações disputam as mesmas entradas do ledger.
 *
 * E elas disputam. `comparecer()` faz read-modify-write em `Chave::Anel`, uma
 * entrada só para o anel inteiro. `votar_anonimo()` escreve nos mesmos dois
 * acumuladores e no mesmo contador de comparecimento. A simulação calcula o
 * footprint e os `writeBytes` contra o estado daquele instante — se o anel
 * cresceu entre simular e aplicar, a transação pode não ter declarado espaço
 * para o que vai escrever.
 *
 * O que este script responde:
 *   1. trinta comparecimentos simultâneos se perdem, ou a rede serializa?
 *   2. uma cédula de anel 30 cabe numa transação real, não só no Env local?
 *   3. em quantos ledgers trinta cédulas se espalham? (o teto de CPU é por
 *      transação, mas existe outro por ledger)
 *
 *   node scripts/rodada-30.mjs
 *   RETENTATIVAS=12 node scripts/rodada-30.mjs
 *
 * O que as rodadas de 2026-10-02 acharam, na testnet:
 *
 *   caderno   ANTES da folga de escrita: uma por ledger, sempre. Com 12
 *             retentativas, 23 de 30 entraram ao custo de 262 transações —
 *             8,7 queimadas por pessoa. A causa é `Chave::Anel` crescer: todas
 *             simulam contra o anel de agora e declaram `writeBytes` para ele,
 *             e quem aplica depois não cabe.
 *             DEPOIS: 30 de 30, zero recusadas, 3 ledgers, 45 transações, com
 *             15 entrando no mesmo ledger. O footprint é uma declaração, não
 *             uma medição — declarar espaço para o anel cheio custou 0,1% a
 *             mais no que de fato é cobrado (3.679.157 contra 3.682.659).
 *
 *   urna      a vazão cai com o tamanho do anel, porque o teto de CPU **por
 *             ledger** (≈500 M, inferido) divide por cédula:
 *
 *               anel  4 · ~21% do teto por cédula · 4 por ledger
 *               anel 12 · ~43%                    · 3 por ledger · 12/12 em 19,4 s
 *               anel 23 · ~73%                    · 1 por ledger · 6/23
 *
 *             Com lance de 100 stroops, 9 de 23 morriam em `txInsufficientFee`.
 *             Com lance de 1.000.000 e reenvio, essa classe de erro zerou.
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
  "CBAFYF5BCTIVPUUO67Y76GF2T2GKYD5JDFLXLBUYRC4QLPN5GZXRWN56";

const N = Number(process.env.N ?? 30);
const OPCOES = 2;
const RETENTATIVAS = Number(process.env.RETENTATIVAS ?? 0);
// Medido: a rede cobra o mínimo necessário, não o lance. A mesma transação com
// lance de 100 e de 1.000.000 foi cobrada idênticos 19.690.096 stroops. Os 100
// do BASE_FEE só servem para perder o leilão quando o ledger está disputado.
const TAXA_INCLUSAO = Number(process.env.TAXA_INCLUSAO ?? 1_000_000);
// Espaço de escrita declarado além do que a simulação pediu, para o anel que
// ainda vai crescer. Medido: 1 de 15 sem isto, 15 de 15 num ledger só com.
const FOLGA = Number(process.env.FOLGA ?? 96 * 40 + 1024);
/** Em quantas seções dividir. 1 reproduz o anel único. */
const SECOES = Number(process.env.SECOES ?? 1);
/** ABERTA=1 abre sem lista: raiz de 32 zeros, qualquer carteira comparece. */
const ABERTA = process.env.ABERTA === "1";

const servidor = new rpc.Server(RPC);
const contrato = new Contract(CONTRATO);

const hexBytes = (h) => Buffer.from(h, "hex");
const bytesHex = (b) => Buffer.from(b).toString("hex");
const bN = (h) => xdr.ScVal.scvBytes(hexBytes(h));
const fr = (h) => nativeToScVal(BigInt("0x" + h), { type: "u256" });
const u32 = (n) => xdr.ScVal.scvU32(n);
const vec = (v) => xdr.ScVal.scvVec(v);
const addr = (g) => new Address(g).toScVal();
const xdrDe = (g) => bytesHex(new Address(g).toScVal().toXDR());
const dorme = (ms) => new Promise((r) => setTimeout(r, ms));
/** O mesmo que `secao_aberta` faz no contrato: o cliente precisa saber antes de
 *  enviar, porque a seção nomeia a entrada que a transação vai escrever. */
async function secaoAberta(id, g, secoes) {
  const b = Buffer.concat([hexBytes(id), hexBytes(xdrDe(g))]);
  const h = await crypto.subtle.digest("SHA-256", b);
  return new DataView(h).getUint32(0, false) % secoes;
}

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

/** Em lotes, porque o friendbot não gosta de trinta de uma vez. */
async function nascerVarios(n, lote = 5) {
  const todos = [];
  for (let i = 0; i < n; i += lote) {
    const k = Math.min(lote, n - i);
    todos.push(...(await Promise.all(Array.from({ length: k }, nascer))));
    process.stdout.write(`\r  financiando ${todos.length}/${n}…`);
  }
  process.stdout.write("\r" + " ".repeat(40) + "\r");
  return todos;
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

/**
 * Como `enviar` do outro script, mas **não lança**: devolve o que aconteceu.
 * O ponto deste teste é ver as falhas, não parar na primeira.
 */
async function tentar(par, metodo, args, lance = TAXA_INCLUSAO, folga = 0) {
  const t0 = Date.now();
  try {
    const conta = await servidor.getAccount(par.publicKey());
    const bruta = new TransactionBuilder(conta, { fee: String(lance), networkPassphrase: PASSPHRASE })
      .addOperation(contrato.call(metodo, ...args))
      .setTimeout(180).build();
    const sim = await servidor.simulateTransaction(bruta);
    if (rpc.Api.isSimulationError(sim)) {
      return { ok: false, fase: "simulação", erro: sim.error.split("\n")[0] };
    }
    const cpu = Number(sim.cost?.cpuInsns ?? 0);
    let pronta = rpc.assembleTransaction(bruta, sim).build();
    if (folga > 0) {
      const dados = new SorobanDataBuilder(
        pronta.toEnvelope().v1().tx().ext().sorobanData().toXDR("base64"),
      );
      const r = dados.build().resources();
      dados.setResources(r.instructions(), r.diskReadBytes(), r.writeBytes() + folga);
      pronta = TransactionBuilder.cloneFrom(pronta, {
        fee: (BigInt(pronta.fee) + BigInt(folga) * 400n).toString(),
        sorobanData: dados.build(),
      }).build();
    }
    pronta.sign(par);
    const envio = await servidor.sendTransaction(pronta);
    if (envio.status === "ERROR") {
      const motivo = envio.errorResult?.result().switch().name ?? "?";
      return { ok: false, fase: "envio", cpu, naoEntrou: motivo === "txInsufficientFee", erro: motivo };
    }
    for (let i = 0; i < 180; i++) {
      const r = await servidor.getTransaction(envio.hash);
      if (r.status === rpc.Api.GetTransactionStatus.SUCCESS) {
        return { ok: true, cpu, taxa: Number(pronta.fee), ledger: r.ledger, ms: Date.now() - t0, hash: envio.hash };
      }
      if (r.status === rpc.Api.GetTransactionStatus.FAILED) {
        return { ok: false, fase: "aplicação", cpu, ledger: r.ledger,
                 erro: JSON.stringify(r.resultXdr?.result?.()?.switch?.()?.name ?? "FAILED").slice(0, 160) };
      }
      await dorme(1000);
    }
    return { ok: false, fase: "espera", cpu, naoEntrou: true, erro: "preterida até expirar" };
  } catch (e) {
    return { ok: false, fase: "exceção", erro: String(e.message ?? e).split("\n")[0].slice(0, 160) };
  }
}

function resumo(rotulo, rs) {
  const ok = rs.filter((r) => r.ok);
  const mal = rs.filter((r) => !r.ok);
  diz(`${rotulo}: ${ok.length} aceitas, ${mal.length} recusadas`);
  if (ok.length) {
    const cpus = ok.map((r) => r.cpu).filter(Boolean);
    const ledgers = [...new Set(ok.map((r) => r.ledger))].sort((a, b) => a - b);
    if (cpus.length) {
      diz(`  cpu ${Math.min(...cpus).toLocaleString("pt-BR")} – ${Math.max(...cpus).toLocaleString("pt-BR")}` +
          ` · ${(100 * Math.max(...cpus) / 400_000_000).toFixed(1)}% do teto no pior caso`);
    }
    diz(`  espalharam-se por ${ledgers.length} ledger${ledgers.length === 1 ? "" : "s"}` +
        ` (${ledgers[0]} … ${ledgers[ledgers.length - 1]})`);
    const porLedger = {};
    for (const r of ok) porLedger[r.ledger] = (porLedger[r.ledger] ?? 0) + 1;
    diz(`  por ledger: ${Object.values(porLedger).join(", ")}`);
    diz(`  mais lenta: ${(Math.max(...ok.map((r) => r.ms)) / 1000).toFixed(1)} s`);
  }
  const porErro = {};
  for (const r of mal) {
    const k = `${r.fase}: ${r.erro}`;
    porErro[k] = (porErro[k] ?? 0) + 1;
  }
  for (const [k, n] of Object.entries(porErro)) diz(`  ✗ ${n}× ${k}`);
  return { ok, mal };
}

async function main() {
  console.log(`\nTESSERA · ${N} votantes ao mesmo tempo, em ${SECOES} ${SECOES === 1 ? "anel" : "seções"}`);
  console.log("─".repeat(70));
  console.log(`  contrato ... ${CONTRATO}`);

  titulo("o eleitorado nasce");
  leitor = await nascer();
  const membros = await nascerVarios(N);
  diz(`${N} identidades financiadas`);

  const enderecos = membros.map((m) => xdrDe(m.publicKey()));
  const pesos = membros.map(() => 1);
  // O id nasce antes da raiz: a divisão em seções é derivada dele.
  const id = bytesHex(Keypair.random().rawPublicKey().subarray(0, 32));
  const raiz = ABERTA ? "0".repeat(64) : wasm.raiz_de_aptos(id, enderecos, pesos, SECOES);
  // Na aberta a seção vem do contrato; aqui guardamos o que ele devolver.
  const divisao = ABERTA
    ? new Array(N).fill(0)
    : Array.from(wasm.secoes_de(id, enderecos, SECOES));
  diz(`raiz de aptos = ${raiz.slice(0, 16)}…`);
  if (SECOES > 1) {
    const tam = Array.from({ length: SECOES }, (_, s) => divisao.filter((x) => x === s).length);
    diz(`${SECOES} seções de ${tam.join(", ")} — sorteadas pela lista, não escolhidas`);
  }

  titulo("abrir");
  const agora = await ledgerAtual();
  // Com retentativa cada rodada custa um ledger, então a janela cresce com N.
  const abreEm = agora + 40 + (RETENTATIVAS ? 3 * N : 10);
  const fechaEm = abreEm + 300;
  const mesa = [(await nascer()).publicKey()];

  const r1 = await tentar(membros[0], "abrir", [
    addr(membros[0].publicKey()), bN(id),
    vec([nativeToScVal({ opcoes: OPCOES, confidencial: true },
      { type: { opcoes: ["symbol", "u32"], confidencial: ["symbol", "bool"] } })]),
    bN(raiz), vec(mesa.map(addr)), u32(1), u32(abreEm), u32(fechaEm),
    xdr.ScVal.scvBool(true), u32(SECOES),
  ]);
  if (ABERTA) diz("aberta: raiz de 32 zeros, sem lista");
  if (!r1.ok) throw new Error(`abrir falhou — ${r1.fase}: ${r1.erro}`);
  diz(`proposta ${id.slice(0, 12)}… · comparecimento até ${abreEm} · votação até ${fechaEm}`);

  const h = bytesHex(await ler("gerador_h"));
  const hp = bytesHex(await ler("hp", bN(id)));

  titulo(`o caderno · ${N} comparecimentos disparados de uma vez`);
  diz("todos fazem read-modify-write na MESMA entrada Chave::Anel(proposta)");
  const chaves = membros.map(() => wasm.nova_chave_de_anel());
  if (RETENTATIVAS) {
    diz(`com até ${RETENTATIVAS} retentativas por pessoa — re-simula contra o estado novo`);
  }
  const tentativas = new Array(N).fill(1);
  const rc = await Promise.all(membros.map(async (m, i) => {
    const c = ABERTA
      ? { irmaos: [], indice: 0, secao: await secaoAberta(id, m.publicKey(), SECOES) }
      : wasm.caminho_de(id, enderecos, pesos, SECOES, i);
    const args = [bN(id), addr(m.publicKey()), bN(chaves[i].publica),
                  vec(c.irmaos.map(bN)), u32(c.indice), u32(c.secao)];
    let r = await tentar(m, "comparecer", args, TAXA_INCLUSAO, FOLGA);
    for (let t = 0; !r.ok && t < RETENTATIVAS; t++) {
      // `JaCompareceu` significa que uma tentativa anterior entrou: não insiste.
      if (String(r.erro).includes("#28")) break;
      await dorme(1000 + Math.floor(Math.random() * 4000));
      tentativas[i]++;
      r = await tentar(m, "comparecer", args, TAXA_INCLUSAO, FOLGA);
    }
    return r;
  }));
  if (RETENTATIVAS) {
    diz(`tentativas por pessoa: mínimo ${Math.min(...tentativas)}, máximo ${Math.max(...tentativas)}, ` +
        `total ${tentativas.reduce((a, b) => a + b, 0)} transações para ${N} comparecimentos`);
  }
  const { ok: comparec } = resumo("comparecimentos", rc);

  titulo("espera a janela virar");
  for (;;) {
    const l = await ledgerAtual();
    if (l >= abreEm) break;
    process.stdout.write(`\r  ledger ${l} / ${abreEm}…`);
    await dorme(15000);
  }
  process.stdout.write("\r" + " ".repeat(40) + "\r");

  if (ABERTA) {
    // Quem decide a seção é o contrato: pergunta a ele, um por um.
    for (let i = 0; i < N; i++) {
      const v = await ler("secao_de", bN(id), addr(membros[i].publicKey()));
      divisao[i] = v ?? -1; // -1 = não compareceu; não entra na contagem
    }
    const tam = Array.from({ length: SECOES }, (_, s) => divisao.filter((x) => x === s).length);
    diz(`o contrato distribuiu por ordem de chegada: ${tam.join(", ")}`);
  }

  const aneis = [];
  for (let s = 0; s < SECOES; s++) {
    aneis.push((await ler("anel", bN(id), u32(s))).map(bytesHex));
  }
  diz(`anéis congelados: ${aneis.map((a) => a.length).join(", ")}`);
  const noCaderno = aneis.reduce((a, b) => a + b.length, 0);
  if (noCaderno !== comparec.length) {
    diz(`  ⚠ ${comparec.length} comparecimentos aceitos mas ${noCaderno} nos anéis — houve escrita perdida`);
  }

  titulo(`a urna · ${noCaderno} cédulas disparadas de uma vez`);
  diz("todas escrevem nos MESMOS dois acumuladores e no mesmo contador");
  const indices = chaves
    .map((_, i) => i)
    .filter((i) => aneis[divisao[i]].includes(chaves[i].publica));
  const efemeras = await nascerVarios(indices.length);
  const t0 = Date.now();
  const rv = await Promise.all(indices.map(async (idx, k) => {
    const meu = aneis[divisao[idx]];
    const i = meu.indexOf(chaves[idx].publica);
    const c = wasm.cedula_anonima(
      id, hp, h, meu, i, chaves[idx].secreta,
      [{ opcoes: OPCOES, confidencial: true }], [k % OPCOES],
    );
    const argsVoto = [
      bN(id), u32(divisao[idx]), vec(meu.map(bN)), bN(c.imagem), fr(c.c0), vec(c.z.map(fr)),
      vec(c.cedula.compromissos.map(bN)),
      vec(c.cedula.provas.map((p) => nativeToScVal(
        { a0: hexBytes(p.a0), a1: hexBytes(p.a1), e0: BigInt("0x" + p.e0),
          z0: BigInt("0x" + p.z0), e1: BigInt("0x" + p.e1), z1: BigInt("0x" + p.z1) },
        { type: { a0: ["symbol", null], a1: ["symbol", null], e0: ["symbol", "u256"],
                  z0: ["symbol", "u256"], e1: ["symbol", "u256"], z1: ["symbol", "u256"] } }))),
      vec(c.cedula.provas_soma.map((p) => nativeToScVal(
        { a: hexBytes(p.a), z: BigInt("0x" + p.z) },
        { type: { a: ["symbol", null], z: ["symbol", "u256"] } }))),
      vec(c.cedula.escolhas.map(u32)),
    ];
    let r = await tentar(efemeras[k], "votar_anonimo", argsVoto);
    for (let t = 0; !r.ok && r.naoEntrou && t < RETENTATIVAS; t++) {
      await dorme(1000 + Math.floor(Math.random() * 4000));
      r = await tentar(efemeras[k], "votar_anonimo", argsVoto, TAXA_INCLUSAO * 4 ** (t + 1));
    }
    return r;
  }));
  const { ok: votos } = resumo("cédulas", rv);
  diz(`tudo levou ${((Date.now() - t0) / 1000).toFixed(1)} s do disparo à última confirmação`);

  titulo("o que o ledger sabe");
  const [conf] = await ler("comparecimento", bN(id));
  diz(`cédulas na urna ... ${conf}`);
  diz(`no caderno ........ ${noCaderno} de ${N}, em anéis de ${aneis.map((a) => a.length).join(", ")}`);
  const cruz = efemeras.map((e) => e.publicKey())
    .filter((e) => membros.some((m) => m.publicKey() === e));
  diz(`interseção caderno ∩ urna = ${cruz.length}`);
  if (cruz.length !== 0) throw new Error("o vínculo não foi quebrado");
  if (conf !== votos.length) {
    diz(`  ⚠ ${votos.length} cédulas aceitas mas o contador diz ${conf} — escrita perdida no acumulador`);
  }

  console.log(`\n  proposta ${id}`);
  console.log(`  https://stellar.expert/explorer/testnet/contract/${CONTRATO}\n`);
}

main().catch((e) => {
  console.error("\n✗", e.message ?? e);
  process.exit(1);
});
