/**
 * A trava de **uma cédula por chave**.
 *
 * Mora sozinha, sem importar nada, por dois motivos: é a invariante de que a
 * desvinculação depende, e um módulo sem dependências pode ser testado por
 * `node` direto, sem navegador e sem rede.
 *
 * A regra não é "uma assinatura por chave". É uma *cédula*. A diferença
 * aparece quando a rede recusa por taxa: a transação não entrou em ledger
 * nenhum, e reassinar a mesma cédula com um lance maior não publica nada de
 * novo. Trocar de chave aí custaria mais e não esconderia mais nada, porque o
 * friendbot e o RPC já viram a primeira.
 */

/** Não é base64, então nunca colide com uma marca de verdade. */
const INDETERMINADA = "<indeterminada>";

export class UmaCedula {
  private assinou: string | null = null;

  /**
   * `marca` identifica a operação. `null` quer dizer que não deu para
   * identificar — e aí a trava falha fechada, voltando a ser "uma assinatura e
   * pronto". O sigilo não pode depender de um `catch` dar certo.
   */
  registrar(marca: string | null): void {
    const m = marca ?? INDETERMINADA;
    if (this.assinou !== null && (marca === null || this.assinou !== m)) {
      throw new Error(
        "esta chave já assinou outra cédula. Uma chave por cédula é o que " +
          "mantém as suas cédulas sem relação entre si no ledger.",
      );
    }
    this.assinou = m;
  }
}
