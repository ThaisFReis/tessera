/** Presentation-only translations for errors returned by the existing WASM ABI. */
const WASM_ERRORS: Record<string, string> = {
  "hex de comprimento ímpar": "Hex input must have an even number of characters.",
  "hex inválido": "Invalid hexadecimal input.",
  "escalar não tem 32 bytes": "A scalar must contain exactly 32 bytes.",
  "endereços e pesos de tamanhos diferentes": "Address and weight counts must match.",
  "nada a somar: a lista de aberturas veio vazia": "Cannot sum an empty list of openings.",
  "um compromisso por opção confidencial em cada cédula, na ordem da cadeia": "Each ballot must have one commitment per confidential option, in chain order.",
  "uma escolha por pergunta, na ordem da cédula": "Provide one choice per question, in ballot order.",
  "escolha fora das opções da pergunta": "The selected option is outside this question's range.",
};
const CONTEXTS: Record<string, string> = {
  "ponto inválido": "Invalid curve point", assinatura: "Signature", rodada: "Beacon round",
  caminho: "Merkle proof", "árvore": "Merkle tree", acaso: "Randomness",
  baliza: "Beacon verification", anel: "Ring signature", fechadura: "Time lock",
  disjuntiva: "Disjunctive proof", soma: "Sum proof",
};

export function mensagemErro(error: unknown): string {
  const raw = String(error instanceof Error ? error.message : error);
  if (WASM_ERRORS[raw]) return WASM_ERRORS[raw];
  const size = raw.match(/^uma abertura tem (\d+) bytes, e um escalar de Fr tem 32$/);
  if (size) return `An opening has ${size[1]} bytes; an Fr scalar requires 32.`;
  // Keep the original diagnostic code and parameters; only translate the label.
  return raw.replace(/^([^:]+):/, (match, label: string) =>
    CONTEXTS[label] ? `${CONTEXTS[label]}:` : match,
  );
}
