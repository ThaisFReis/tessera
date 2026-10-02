import init, * as tessera from "tessera-cliente";

let pronto: Promise<void> | null = null;

/**
 * Carrega o `tessera-core` compilado para wasm.
 *
 * É o mesmo código Rust do contrato, do verificador e da CLI. Não existe uma
 * segunda implementação em JavaScript, e é de propósito: duas implementações
 * divergem, e a divergência entre provador e verificador é o bug mais caro
 * possível neste projeto.
 */
export async function carregar() {
  if (!pronto) pronto = init().then(() => undefined);
  await pronto;
  return tessera;
}
