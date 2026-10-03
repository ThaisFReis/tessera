/**
 * O guarda do diário.
 *
 * O diário deixou de ser efêmero: ele agora atravessa janelas e sobrevive a uma
 * recarga, porque é isso que o vídeo da demonstração exige. O preço é que um
 * segredo vazado nele passa a **ficar**, e num lugar fácil de achar.
 *
 * Este teste congela as duas bordas do crivo. A de baixo é óbvia — `r` e os
 * fatores não passam. A de cima é a que custa caro esquecer: as frases que
 * *falam* de segredo sem revelar nenhum são justamente as que explicam a
 * garantia ao usuário, e um crivo por palavra as apagaria da tela.
 *
 *   node scripts/diario.test.mjs
 */
import { strict as assert } from "node:assert";
import { suspeito } from "../src/diario.ts";

const HEX = "4f2a9c01bb7e35d6f08a1c4e92b7d5306a1f8c2e4b9d07a3c5e18f260b4d7a9c";

/* Não passam: nome de segredo junto de um valor. */
for (const txt of [
  `chave secreta = ${HEX}`,
  `r = ${HEX}`,
  `r: ${HEX}`,
  `fator de aleatoriedade ${HEX}`,
  `segredo da mesa ${HEX}`,
  `chave privada = ${HEX}`,
  `semente ${HEX}`,
]) {
  assert.equal(suspeito(txt), true, `o guarda DEIXOU PASSAR: ${txt}`);
}

/* Passam: ou é só a palavra, ou é um valor que é público de propósito. */
for (const txt of [
  "a chave secreta do anel fica nesta aba, e só aqui",
  "o r que esconde o seu voto nasceu e morreu nesta aba",
  `chave pública = ${HEX}`,
  `raiz de aptos = ${HEX}`,
  `imagem de chave = ${HEX}`,
  `tx ${HEX}`,
  "taxa = 4275544 stroops",
  "32 bytes vão para o contrato. A lista não vai.",
  "caminho de Merkle com 4 irmãos",
  "cédulas na urna: 7",
]) {
  assert.equal(suspeito(txt), false, `o guarda RECUSOU uma linha legítima: ${txt}`);
}

console.log("✓ o guarda do diário separa a palavra do valor");
