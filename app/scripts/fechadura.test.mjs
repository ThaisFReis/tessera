/**
 * A fechadura de tempo, do lado do JavaScript.
 *
 * O teste do contrato já prova o ciclo em Rust. O que **este** prova é o que o
 * Rust não alcança: que o pacote wasm que o navegador carrega exporta a
 * conferência e a decifragem, que elas atravessam a fronteira `JsValue`
 * inteiras, e que a assinatura errada é **recusada** em vez de devolver lixo.
 *
 * Sem rede. A rodada 6.000.100 e sua assinatura estão congeladas aqui e em
 * `docs/SOURCES.md` — buscá-la no relé a cada portão tornaria o portão
 * dependente de um servidor, e um portão que depende da rede não é portão.
 *
 *   node scripts/fechadura.test.mjs
 */
import { strict as assert } from "node:assert";
import * as w from "../../cliente-wasm/pacote-node/tessera_cliente.js";

const RODADA = 6_000_100n;
const ASSINATURA =
  "b6018631cdb80412e0690267164e381fc744e6a62ba3397c9becae5370100e87dcabde17a7325d5e7fb8d0d135cb664d";

/* A assinatura medida confere contra a chave pública congelada da cadeia. */
w.conferir_baliza(RODADA, ASSINATURA);

/* Um byte mexido não confere. É o pareamento recusando, não um `if`. */
const mexida = ASSINATURA.slice(0, 60) + (ASSINATURA[60] === "a" ? "b" : "a") + ASSINATURA.slice(61);
assert.throws(() => w.conferir_baliza(RODADA, mexida), /baliza/, "a assinatura mexida PASSOU");

/* A rodada vizinha também não: a identidade assinada é a da rodada. */
assert.throws(() => w.conferir_baliza(RODADA + 1n, ASSINATURA), /baliza/, "a rodada errada PASSOU");

/* ---- o ciclo inteiro: a cédula nasce cifrada e a rodada a abre ---- */

/** O `H` do contrato na testnet, congelado em `gerador_h_bate_com_o_vetor_da_testnet`. */
const H =
  "1462b4b57a7d01e598685e913608bab6990de8cce1c705642c19a6b9ed660fc589758bdebb20c2bdde65a25b35b12206198669dff273abe7b9e6a0b3e636f8a8a41f9c81f6a79d308ec0fe57ef8a073c90e444002ba59807f987cce67d72eccf";
const PROPOSTA = "19".repeat(32);

const chaves = [0, 1, 2].map(() => w.nova_chave_de_anel());
const anel = chaves.map((c) => c.publica);
// Um `Hp` qualquer serve aqui: o que este teste exercita é a fechadura, não o
// anel. Quem confere o anel é o contrato, e isso já tem teste em Rust.
const hp = anel[0];

const escolhas = [0, 1, 1];
const cedulas = escolhas.map((escolha, i) =>
  w.cedula_anonima(PROPOSTA, hp, H, anel, i, chaves[i].secreta,
    [{ opcoes: 2, confidencial: true }], new Uint32Array([escolha]), RODADA),
);

for (const c of cedulas) {
  assert.equal(c.cedula.cripto.length, 2 * 160 * 2, "160 bytes por opção confidencial");
}

const compromissos = cedulas.flatMap((c) => c.cedula.compromissos);
const criptos = cedulas.map((c) => c.cedula.cripto);

const a = w.abertura_da_secao(compromissos, criptos, 2, H, RODADA, ASSINATURA);
assert.deepEqual(a.abertas, [true, true, true], "alguma cédula não abriu");
assert.equal(a.abriram, 3);
assert.deepEqual(Array.from(a.totais), [1, 2], "o placar decifrado está errado");

/* A cédula sabotada custa um voto, não o placar. */
const sabotados = [...criptos];
sabotados[1] = sabotados[1].slice(0, -1) + (sabotados[1].at(-1) === "a" ? "b" : "a");
const b = w.abertura_da_secao(compromissos, sabotados, 2, H, RODADA, ASSINATURA);
assert.deepEqual(b.abertas, [true, false, true]);
assert.deepEqual(Array.from(b.totais), [1, 1], "o placar não sobreviveu à sabotagem");

/* E sem a chave da rodada não há placar nenhum. */
assert.throws(
  () => w.abertura_da_secao(compromissos, criptos, 2, H, RODADA + 1n, ASSINATURA),
  /baliza/,
  "decifrou com a assinatura de outra rodada",
);

console.log("fechadura: a rodada confere, a cédula abre, e a sabotada custa só o próprio voto");
