/**
 * A regra da chave de uso único.
 *
 * Ela foi afrouxada de propósito — de "uma assinatura" para "uma cédula" —
 * para que uma recusa por taxa possa ser reenviada com lance maior sem trocar
 * de chave. O afrouxamento é seguro porque uma transação recusada não entrou
 * em ledger nenhum, mas mexe na invariante que sustenta a desvinculação
 * inteira. Então fica preso aqui.
 *
 *   node scripts/usoUnico.test.mjs
 */
import { strict as assert } from "node:assert";
import { UmaCedula } from "../src/usoUnico.ts";

const CEDULA = "AAAAGAAAAAA=";
const OUTRA = "AAAAGAAAAAE=";

/* A mesma cédula, reassinada: passa quantas vezes for. */
{
  const t = new UmaCedula();
  t.registrar(CEDULA);
  t.registrar(CEDULA);
  t.registrar(CEDULA);
}

/* Outra cédula na mesma chave: nunca. */
{
  const t = new UmaCedula();
  t.registrar(CEDULA);
  assert.throws(() => t.registrar(OUTRA), /já assinou outra cédula/,
    "a chave de uso único aceitou uma SEGUNDA cédula — a desvinculação cai");
}

/* Marca indeterminada: falha fechada, mesmo sendo a primeira a repetir. */
{
  const t = new UmaCedula();
  t.registrar(null);
  assert.throws(() => t.registrar(null), /já assinou outra cédula/,
    "sem conseguir identificar a operação, a trava TEM de recusar a segunda");
}

/* E uma indeterminada depois de uma conhecida também não passa. */
{
  const t = new UmaCedula();
  t.registrar(CEDULA);
  assert.throws(() => t.registrar(null), /já assinou outra cédula/);
}

console.log("✓ uma cédula por chave, e a reassinatura da mesma é permitida");
