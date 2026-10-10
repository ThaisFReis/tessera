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
      `${aptos} ${aptos === 1 ? "pessoa" : "pessoas"} no eleitorado, e o contrato ` +
      `exige ${TAU} cédulas numa seção para apurar. Nem com uma seção só esta ` +
      `votação teria placar — não é a divisão que está errada, é o eleitorado ` +
      `que é pequeno demais para esconder alguém.`
    );
  }
  return (
    `${aptos} pessoas em ${secoes} seções dá cerca de ${p.media} por seção, e o ` +
    `contrato exige ${TAU} para apurar — a votação aconteceria e o placar não ` +
    `sairia. No máximo ${p.maximo} ${p.maximo === 1 ? "seção" : "seções"}, e ` +
    `como a divisão é por hash as seções saem desiguais: a menor fica abaixo da ` +
    `média, então é prudente ficar bem acima do piso.`
  );
}
