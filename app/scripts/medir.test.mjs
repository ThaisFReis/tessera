/**
 * A regra do número medido.
 *
 * O §0 do SPEC diz "número medido ou nenhum número". Este teste trava a
 * segunda metade: quando a medição não vem, o script **para**. A versão
 * anterior tinha `?? 0` e publicava zero instruções num relatório de carga —
 * que parece uma otimização e não um bug, e por isso ninguém olhava duas vezes.
 *
 *   node scripts/medir.test.mjs
 */
import { strict as assert } from "node:assert";
import { comTeto, instrucoes, TETO } from "./medir.mjs";

const simDe = (n) => ({
  transactionData: { build: () => ({ resources: () => ({ instructions: () => n }) }) },
});

/* O caminho bom devolve o número, sem transformar nada. */
assert.equal(instrucoes(simDe(148_911_943)), 148_911_943);

/* Zero, negativo, NaN e ausente são ausência de medição, não custo. */
for (const ruim of [0, -1, NaN, undefined, null, "12345"]) {
  assert.throws(() => instrucoes(simDe(ruim), "votar_anonimo"), /medir as instruções/,
    `aceitou ${JSON.stringify(ruim)} como medição`);
}

/* E a forma antiga do SDK — `sim.cost.cpuInsns` — não é uma saída silenciosa. */
assert.throws(() => instrucoes({ cost: { cpuInsns: "9999" } }), /o SDK mudou de forma/);
assert.throws(() => instrucoes({}), /o SDK mudou de forma/);

/* A mensagem diz qual chamada não mediu. */
assert.throws(() => instrucoes({}, "apurar_secao"), /apurar_secao/);

assert.equal(TETO, 400_000_000);
assert.equal(comTeto(40_000_000), "40.000.000 · 10,0% do teto");
/* E uma chamada barata não lê "0% do teto", que soa como ausência de medida. */
assert.equal(comTeto(350_372), "350.372 · 0,09% do teto");

console.log("medir: número medido, ou exceção — não existe terceira saída");
