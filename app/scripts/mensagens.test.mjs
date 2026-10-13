import { strict as assert } from "node:assert";
import { mensagemErro } from "../src/mensagens.ts";

assert.equal(mensagemErro("hex inválido"), "Invalid hexadecimal input.");
assert.equal(mensagemErro(new Error("escolha fora das opções da pergunta")), "The selected option is outside this question's range.");
assert.equal(mensagemErro("uma abertura tem 16 bytes, e um escalar de Fr tem 32"), "An opening has 16 bytes; an Fr scalar requires 32.");
assert.equal(mensagemErro("baliza: AssinaturaInvalida"), "Beacon verification: AssinaturaInvalida");
assert.equal(mensagemErro(new Error("Network unavailable")), "Network unavailable");
assert.equal(mensagemErro("Unknown diagnostic #123"), "Unknown diagnostic #123");
console.log("English errors: WASM messages translated; unknown diagnostics preserved");
