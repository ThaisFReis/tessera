/**
 * O piso de anonimato, conferido antes de abrir.
 *
 * O contrato recusa apurar abaixo de `τ`. Essa recusa é a garantia
 * funcionando — e chega no fim da votação, quando não há mais o que fazer. O
 * teste trava a antecipação: a tela recusa **abrir** uma divisão que já nasce
 * condenada, e diz o tamanho que daria.
 *
 *   node scripts/secoes.test.mjs
 */
import { strict as assert } from "node:assert";
import { explicar, secaoNasceuPequena, TAU } from "../src/secoes.ts";

assert.equal(TAU, 5);

/* Cabe: 30 em 3 dá 10 por seção. */
assert.equal(secaoNasceuPequena(30, 3), null);
/* A fronteira exata: 25 em 5 dá 5, que é o piso. */
assert.equal(secaoNasceuPequena(25, 5), null);
/* Um a menos não cabe. */
assert.deepEqual(secaoNasceuPequena(24, 5), { media: 4, maximo: 4, eleitoradoPequeno: false });

/* Uma seção só também é conferida: o eleitorado pode ser o pequeno. */
assert.equal(secaoNasceuPequena(5, 1), null);
assert.deepEqual(secaoNasceuPequena(4, 1), { media: 4, maximo: 0, eleitoradoPequeno: true });
assert.equal(secaoNasceuPequena(0, 1).eleitoradoPequeno, true);

/* `secoes = 0` não derruba a conta: trata como uma. */
assert.equal(secaoNasceuPequena(30, 0), null);

/* A frase diz o número que a pessoa precisa para decidir. */
const p = secaoNasceuPequena(24, 5);
const txt = explicar(24, 5, p);
assert.match(txt, /24 pessoas em 5 seções/);
assert.match(txt, /cerca de 4 por seção/);
assert.match(txt, /No máximo 4 seções/);
assert.match(txt, /a menor fica abaixo da média/);

const q = secaoNasceuPequena(3, 1);
assert.match(explicar(3, 1, q), /Nem com uma seção só/);

console.log("seções: a divisão que nasce abaixo do piso é recusada na hora de abrir");

/* ---------- §11-N: somar as seções ---------- */

const { somarSecoes } = await import("../src/secoes.ts");

/* Duas seções, as duas lidas agora: soma opção a opção. */
assert.deepEqual(
  somarSecoes(
    [
      { secao: 0, totais: [3, 5], abriram: 8, cedulas: 8 },
      { secao: 1, totais: [3, 3], abriram: 6, cedulas: 6 },
    ],
    2,
  ),
  { totais: [6, 8], abriram: 14, cedulas: 14, completo: true },
);

/* Falta uma: `completo` é falso, e o que há não é apresentado como o placar. */
assert.deepEqual(
  somarSecoes([{ secao: 0, totais: [3, 5], abriram: 8, cedulas: 8 }], 2),
  { totais: [3, 5], abriram: 8, cedulas: 8, completo: false },
);

/* Uma parcela veio do ledger, que não guarda o denominador: o total de cédulas
   do conjunto some, em vez de somar 8 + 0 e mentir. */
assert.deepEqual(
  somarSecoes(
    [
      { secao: 0, totais: [3, 5], abriram: 8, cedulas: null },
      { secao: 1, totais: [3, 3], abriram: 6, cedulas: 6 },
    ],
    2,
  ),
  { totais: [6, 8], abriram: 14, cedulas: null, completo: true },
);

/* Nenhuma seção: nada a somar, e não é um placar de zeros. */
assert.deepEqual(somarSecoes([], 3), {
  totais: [],
  abriram: 0,
  cedulas: 0,
  completo: false,
});

/* Seções com número de opções diferente é bug de quem chamou, não um placar
   para apresentar. */
assert.throws(() =>
  somarSecoes(
    [
      { secao: 0, totais: [1, 2], abriram: 3, cedulas: 3 },
      { secao: 1, totais: [1, 2, 3], abriram: 6, cedulas: 6 },
    ],
    2,
  ),
);

console.log("§11-N: o placar de várias seções é a soma das parcelas, e falta uma é visível");
