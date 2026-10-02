import { Keypair } from "@stellar/stellar-sdk";
import { REDE } from "./rede";

/** Uma carteira é só isto: um endereço e uma assinatura. */
export interface Carteira {
  readonly tipo: "efemera" | "externa" | "passkey";
  endereco(): string;
  assinar(xdr: string): Promise<string>;
}

/**
 * A carteira de uso único — e é ela que torna o voto desvinculado.
 *
 * A urna brasileira não esconde o voto de quem tem acesso: ela **quebra o
 * vínculo**. O caderno diz quem compareceu, a urna diz o que foi votado, e nada
 * liga os dois. Aqui o papel do caderno é da carteira de identidade da pessoa,
 * que assina `comparecer`; o papel da urna é desta, que assina a cédula e é
 * descartada.
 *
 * Se a cédula fosse assinada pela carteira de identidade, o vínculo voltaria
 * inteiro e nada mais no sistema conseguiria desfazê-lo — nem o anel, nem o
 * compromisso de Pedersen. É por isso que `assinar` aqui não aceita reuso: a
 * chave vive para uma transação.
 */
export class CarteiraEfemera implements Carteira {
  readonly tipo = "efemera" as const;
  private par: Keypair;
  private usada = false;

  private constructor(par: Keypair) {
    this.par = par;
  }

  /**
   * Sorteia e financia. O friendbot é a única parte disto que vê um IP — e vê
   * o IP, não o voto. É um vínculo fora do ledger, e a tela diz isso.
   */
  static async nascer(): Promise<CarteiraEfemera> {
    const par = Keypair.random();
    const r = await fetch(`${REDE.friendbot}?addr=${par.publicKey()}`);
    if (!r.ok && r.status !== 400) {
      throw new Error(`o friendbot recusou financiar a chave: ${r.status}`);
    }
    return new CarteiraEfemera(par);
  }

  endereco(): string {
    return this.par.publicKey();
  }

  async assinar(xdr: string): Promise<string> {
    if (this.usada) {
      throw new Error(
        "esta chave já assinou. Uma chave por cédula é o que mantém as suas " +
          "cédulas sem relação entre si no ledger.",
      );
    }
    this.usada = true;
    const { TransactionBuilder } = await import("@stellar/stellar-sdk");
    const tx = TransactionBuilder.fromXDR(xdr, REDE.passphrase);
    tx.sign(this.par);
    return tx.toXDR();
  }
}

/**
 * A carteira de identidade: a mesma pessoa, sempre o mesmo endereço.
 *
 * Serve ao caderno — `comparecer` precisa dizer quem é — e a abrir propostas.
 * **Nunca** assina uma cédula numa proposta de anel.
 *
 * Na v1 ela é uma chave guardada no `localStorage` desta aba. É o suficiente
 * para a testnet, e é honesto sobre o que é: não há custódia, não há servidor,
 * e limpar o navegador perde a identidade.
 */
export class CarteiraLocal implements Carteira {
  readonly tipo = "externa" as const;
  private par: Keypair;

  private constructor(par: Keypair) {
    this.par = par;
  }

  static async abrir(chave = "tessera:identidade"): Promise<CarteiraLocal> {
    let semente = localStorage.getItem(chave);
    if (!semente) {
      const par = Keypair.random();
      semente = par.secret();
      localStorage.setItem(chave, semente);
      const r = await fetch(`${REDE.friendbot}?addr=${par.publicKey()}`);
      if (!r.ok && r.status !== 400) {
        throw new Error(`o friendbot recusou financiar a identidade: ${r.status}`);
      }
    }
    return new CarteiraLocal(Keypair.fromSecret(semente));
  }

  endereco(): string {
    return this.par.publicKey();
  }

  async assinar(xdr: string): Promise<string> {
    const { TransactionBuilder } = await import("@stellar/stellar-sdk");
    const tx = TransactionBuilder.fromXDR(xdr, REDE.passphrase);
    tx.sign(this.par);
    return tx.toXDR();
  }
}
