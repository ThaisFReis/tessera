/**
 * Os dois portões do placar.
 *
 * A afirmação que este teste guarda é a mais fácil de quebrar sem perceber:
 * **nenhum placar antes do fim da votação, nem parcial**. Ela depende de dois
 * portões que medem coisas diferentes — a sequência do ledger fecha a urna, o
 * instante da rodada faz nascer a chave — e de nenhum deles poder ser
 * satisfeito pelo outro.
 *
 * O caso que importa é o terceiro: a rodada já venceu e a urna **ainda está
 * aberta**. Uma ordem errada entre os dois `if` responderia "pode apurar" ali,
 * e o placar sairia no meio da votação. O contrato recusaria — mas a tela já
 * teria mostrado, porque ela decifra sozinha e não precisa de ninguém.
 *
 *   node scripts/portao.test.mjs
 */
import { strict as assert } from "node:assert";
import { portao } from "../src/portao.ts";

const P = { rodada: 1000n, fecha_em: 500 };
const VENCE = 1_700_000_000;

/* Sem fechadura: quem apura é a mesa, ou ninguém. */
assert.equal(portao({ rodada: 0n, fecha_em: 0 }, 9e9, 9e9, 0).tipo, "sem-fechadura");

/* A urna aberta recusa, mesmo com o relógio já aberto. */
assert.deepEqual(portao(P, 499, VENCE + 1, VENCE), { tipo: "janela-aberta", faltam: 1 });
assert.deepEqual(portao(P, 1, VENCE + 99999, VENCE), { tipo: "janela-aberta", faltam: 499 });

/* A urna fechada, o relógio não: diz qual das duas coisas falta. */
assert.deepEqual(portao(P, 500, VENCE - 30, VENCE), { tipo: "esperando-relogio", faltam: 30 });

/* Os dois, e só os dois. */
assert.deepEqual(portao(P, 500, VENCE, VENCE), { tipo: "pode-apurar" });
assert.deepEqual(portao(P, 501, VENCE + 1, VENCE), { tipo: "pode-apurar" });

/* A fronteira dos dois, um de cada lado. */
assert.equal(portao(P, 499, VENCE, VENCE).tipo, "janela-aberta");
assert.equal(portao(P, 500, VENCE - 1, VENCE).tipo, "esperando-relogio");

console.log("portão: nenhum placar antes do fim da urna, e nenhum antes de a chave existir");
