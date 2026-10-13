/**
 * O piso de `τ`, visto da tela que abre a votação.
 *
 * O contrato recusa **apurar** uma seção com menos de `τ` cédulas: um anel
 * pequeno demais não esconde ninguém, e publicar o total ali entregaria quem
 * caiu nele. Essa recusa é a garantia funcionando — mas ela chega tarde, no fim
 * da votação, quando já não há o que fazer.
 *
 * Esta função a antecipa para o momento de abrir, que é o único em que ainda dá
 * para escolher diferente.
 *
 * Mora sozinha, sem importar nada, pelo mesmo motivo de `portao.ts` e
 * `usoUnico.ts`: a conta é curta, o erro é caro, e um módulo sem dependências
 * roda sob `node` direto.
 */

/** O piso de anonimato. Espelha `contrato/src/tipos.rs::TAU`. */
export const TAU = 5;

export type SecaoPequena = {
  /** Quantas pessoas, em média, cada seção receberia. */
  media: number;
  /** O maior número de seções que ainda mantém a média no piso. */
  maximo: number;
  /** `true` quando nem uma seção só resolve: o eleitorado é que é pequeno. */
  eleitoradoPequeno: boolean;
};

/**
 * `null` quando a divisão cabe; os números quando não cabe.
 *
 * **A média não é o mínimo**, e a diferença importa. A seção de cada pessoa sai
 * de `H(assinatura ‖ endereço) mod seções`, que não faz rodízio — então as
 * seções saem de tamanhos diferentes e a menor costuma ficar abaixo da média.
 * Por isso a recusa aqui é pela média: quem já começa com a média no limite
 * termina com alguma seção abaixo dele.
 */
export function secaoNasceuPequena(aptos: number, secoes: number): SecaoPequena | null {
  const media = Math.floor(aptos / Math.max(1, secoes));
  if (media >= TAU) return null;
  return {
    media,
    maximo: Math.floor(aptos / TAU),
    eleitoradoPequeno: aptos < TAU,
  };
}

/** A frase que a tela mostra. Separada da conta para poder ser lida em voz alta. */
export function explicar(aptos: number, secoes: number, p: SecaoPequena): string {
  if (p.eleitoradoPequeno) {
    return (
      `${aptos} ${aptos === 1 ? "person" : "people"} in the electorate; the contract ` +
      `requires ${TAU} ballots in a section to tally. Even a single section would not ` +
      `produce a result — the electorate is too small ` +
      `to meet the anonymity threshold.`
    );
  }
  return (
    `${aptos} people in ${secoes} sections gives about ${p.media} per section, while the ` +
    `contract requires ${TAU} to tally. Voting could proceed but no tally would ` +
    `be published. Use at most ${p.maximo} ${p.maximo === 1 ? "section" : "sections"}. Since ` +
    `hash-based allocation creates uneven sections, the smallest can fall below ` +
    `the average. Leave a comfortable margin above the threshold.`
  );
}

/** O que a apuração de **uma** seção produziu. */
export type ParcialDaSecao = {
  secao: number;
  totais: number[];
  abriram: number;
  /** Quantas cédulas havia. `null` quando o número veio do ledger, que guarda
   *  quantas abriram e não quantas existiam. */
  cedulas: number | null;
};

/**
 * O placar da votação é a soma das seções — e só é o placar quando todas estão
 * lá.
 *
 * Isto é o §11-N do lado da conta: separar as cédulas por seção passou a ser
 * possível porque o tópico do evento diz a seção, e o que falta é somar. Duas
 * decisões ficam aqui, longe de rede e de tela, para terem teste:
 *
 * - `completo` é falso enquanto faltar seção, e a tela não pode apresentar uma
 *   soma parcial como resultado;
 * - `cedulas` é `null` se **qualquer** parcela veio do ledger sem denominador.
 *   Somar 8 + 0 daria "14 de 8", que é pior que não dizer.
 */
export function somarSecoes(
  parciais: ParcialDaSecao[],
  secoes: number,
): { totais: number[]; abriram: number; cedulas: number | null; completo: boolean } {
  const n = parciais[0]?.totais.length ?? 0;
  if (parciais.some((p) => p.totais.length !== n)) {
    throw new Error("Sections with different option counts cannot be combined");
  }
  return {
    totais: Array.from({ length: n }, (_, j) => parciais.reduce((a, p) => a + p.totais[j], 0)),
    abriram: parciais.reduce((a, p) => a + p.abriram, 0),
    cedulas: parciais.some((p) => p.cedulas === null)
      ? null
      : parciais.reduce((a, p) => a + (p.cedulas ?? 0), 0),
    completo: parciais.length === Math.max(1, secoes),
  };
}
