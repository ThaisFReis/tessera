/**
 * A regra da chave de uso único.
 *
 * Ela foi afrouxada de propósito — de "uma assinatura" para "uma cédula" —
 * para que uma recusa por taxa possa ser reenviada com lance maior sem
 * trocar de chave. O afrouxamento é seguro porque uma transação recusada não
 * entrou em ledger nenhum, mas ele mexe na invariante que sustenta a
 * desvinculação. Então fica preso aqui.
 *
 *   node scripts/carteira.test.mjs
 */
import { strict as assert } from "node:assert";
import {
  Account, Keypair, Networks, Operation, TransactionBuilder, Asset,
} from "@stellar/stellar-sdk";
import { CarteiraEfemera } from "../src/carteira.ts";

const par = Keypair.random();
const conta = () => new Account(par.publicKey(), "7");

const tx = (destino, taxa) =>
  new TransactionBuilder(conta(), { fee: String(taxa), networkPassphrase: Networks.TESTNET })
    .addOperation(Operation.payment({ destination: destino, asset: Asset.native(), amount: "1" }))
    .setTimeout(60).build().toXDR();

// A carteira nasce financiada pelo friendbot; aqui só queremos a trava.
const carteira = Object.create(CarteiraEfemera.prototype);
carteira.par = par;
carteira.assinou = null;

const outro = Keypair.random().publicKey();

// A MESMA cédula, com lance maior: tem de passar.
await carteira.assinar(tx(outro, 100));
await carteira.assinar(tx(outro, 4_000_000));
console.log("✓ a mesma cédula pode ser reassinada com lance maior");

// OUTRA cédula: tem de ser recusada.
await assert.rejects(
  () => carteira.assinar(tx(Keypair.random().publicKey(), 100)),
  /já assinou outra cédula/,
  "a chave de uso único aceitou uma SEGUNDA cédula — a desvinculação cai",
);
console.log("✓ uma segunda cédula na mesma chave é recusada");
