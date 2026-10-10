/**
 * O roteador tem de ser o de fragmento, e isto é um guarda de regressão.
 *
 * Medido em T-005, com o `dist` servido como arquivo estático: abrir
 * `/demo/votar` direto pedia `/demo/assets/index-*.js` e levava 404. A causa é
 * `base: "./"` no `vite.config.ts` — caminhos de asset relativos, resolvidos
 * contra o segmento da rota. Navegar por dentro do app funcionava, e por isso
 * o bug passava despercebido no `npm run dev`; **recarregar ou abrir um link
 * compartilhado, não**.
 *
 * E link compartilhado é o caso normal deste produto: é assim que uma votação
 * chega a quem vota.
 *
 * Trocar de volta para `BrowserRouter` reintroduz o bug em silêncio, num lugar
 * onde ninguém olha — a tela funciona inteira em desenvolvimento. Por isso o
 * guarda lê o próprio fonte, como o teste que prova que `votar` não grava
 * recibo: ausência não se prova de outro jeito.
 *
 *   node scripts/rotas.test.mjs
 */
import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";

const bruto = readFileSync(new URL("../src/main.tsx", import.meta.url), "utf8");

/**
 * **Sem os comentários.** A primeira versão deste guarda falhou contra o
 * arquivo certo: o comentário que explica a escolha cita o nome proibido, e o
 * guarda encontrou a si mesmo. É o mesmo tropeço do guarda de rede do
 * `core/src/relogio.rs`, e a lição é a mesma — um guarda que lê fonte tem de
 * ler o **código**.
 */
const fonte = bruto.replace(/\/\*[\s\S]*?\*\//g, "").replace(/\/\/.*$/gm, "");

assert.match(fonte, /HashRouter/, "o roteador deixou de ser o de fragmento");
assert.doesNotMatch(
  fonte,
  /BrowserRouter/,
  "BrowserRouter voltou: com `base` relativo, todo link profundo quebra ao ser aberto direto",
);

/* E o `base` relativo, que é a outra metade da causa, continua onde estava. */
const vite = readFileSync(new URL("../vite.config.ts", import.meta.url), "utf8");
assert.match(vite, /base:\s*"\.\/"/, "o `base` mudou — reavalie o roteador junto");

console.log("rotas: o link compartilhado abre onde o site estiver servido");
