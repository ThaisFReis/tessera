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
