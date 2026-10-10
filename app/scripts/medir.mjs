/**
 * O número de instruções de uma simulação — ou uma exceção.
 *
 * Não existe terceira saída, e é o ponto deste módulo.
 *
 * Os scripts de carga liam `sim.cost.cpuInsns`. Esse campo **não existe** no
 * SDK 14.6.1: `grep -r cpuInsns node_modules/@stellar/stellar-sdk` não devolve
 * nada, então a leitura é `undefined` sempre, não às vezes. O `rodada-30.mjs`
 * a convertia com `?? 0` e imprimia **0 instruções** num relatório que existe
 * justamente para publicar custos medidos.
 *
 * Zero não é um custo baixo: é um número inventado, e o §0 do SPEC proíbe
 * número inventado. Pior, era inventado de um jeito que parecia bom — um
 * relatório de carga dizendo "0 instruções" passa por otimização, não por bug,
 * e ninguém olha duas vezes.
 *
 * O número de verdade está nos recursos que a simulação devolve, que é o mesmo
 * lugar de onde `app/src/rede.ts` o tira para mostrar na tela. Se o SDK mudar
 * de forma outra vez, isto **quebra o script** em vez de imprimir um zero — e
 * quebrar é a resposta certa, porque um portão que mente é pior que um portão
 * que falha.
 */

/** O teto de CPU por transação, achado por bissecção na testnet. */
export const TETO = 400_000_000;

/**
 * Lê as instruções de uma simulação bem-sucedida. Lança se não conseguir.
 *
 * `onde` entra na mensagem para que a falha diga qual chamada não mediu, em
 * vez de só dizer que alguma não mediu.
 */
export function instrucoes(sim, onde = "a simulação") {
  let n;
  try {
    n = sim.transactionData.build().resources().instructions();
  } catch (e) {
    throw new Error(
      `não deu para medir as instruções de ${onde}: o SDK mudou de forma ` +
        `(${String(e.message ?? e).slice(0, 120)}). Conserte a leitura — não ` +
        `publique um número que não foi medido.`,
    );
  }
  if (typeof n !== "number" || !Number.isFinite(n) || n <= 0) {
    throw new Error(
      `não deu para medir as instruções de ${onde}: veio ${JSON.stringify(n)}. ` +
        `Zero não é um custo baixo, é a ausência de medição.`,
    );
  }
  return n;
}

/**
 * `12.345.678 · 3,1% do teto` — o número e o que ele significa, juntos.
 *
 * Arredondar para inteiro faz uma chamada barata ler "0% do teto", que soa como
 * ausência de medida em vez de medida pequena. É a mesma regra de
 * `rede.ts::porcento`, e pelo mesmo motivo.
 */
export function comTeto(cpu) {
  const p = (100 * cpu) / TETO;
  const txt = p < 1 ? `${p.toFixed(2)}%` : `${p.toFixed(1)}%`;
  return `${cpu.toLocaleString("pt-BR")} · ${txt.replace(".", ",")} do teto`;
}
