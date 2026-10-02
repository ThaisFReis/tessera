/**
 * O vetor que importa, do lado do JavaScript.
 *
 * É o mesmo vetor de `cli/src/chave.rs::xdr_bate_com_o_que_o_sdk_produz`, e
 * existe pelo mesmo motivo: se o XDR do endereço divergir, a folha de Merkle e
 * o contexto das provas divergem do contrato, e o que volta é
 * `NaoEstaNaListaDeAptos` — um erro que não diz por quê, e que já custou uma
 * rodada inteira na testnet.
 *
 *   node scripts/xdr.test.mjs
 */
import { Address } from "@stellar/stellar-sdk";
import { strict as assert } from "node:assert";

const CONTA = "GCZC4HW5TA2FSFPSFX3SJTOCDFQ6KZBHDHEG5O32STRX4PQXUX4NLGWJ";
const ESPERADO =
  "000000120000000000000000" +
  "b22e1edd98345915f22df724cdc21961e5642719c86ebb7a94e37e3e17a5f8d5";

const x = new Address(CONTA).toScVal().toXDR().toString("hex");
assert.equal(x.length / 2, 44, "a folha tem 44 bytes");
assert.equal(x, ESPERADO, "o XDR do endereço DIVERGE do que o contrato lê");
console.log("✓ o XDR do endereço bate com o vetor da CLI");
