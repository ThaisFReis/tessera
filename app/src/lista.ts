/**
 * A lista de aptos, que **não vai para a rede**.
 *
 * O contrato guarda 32 bytes de raiz e mais nada — é o desenho, e é o que
 * mantém o eleitorado fora do ledger. Mas quem vota precisa da lista inteira
 * para montar o próprio caminho de Merkle, então ela tem de viajar por fora.
 *
 * Aqui ela fica no `localStorage` de quem abriu, e é oferecida como um texto
 * para colar. Num condomínio, a lista já é conhecida de quem mora nele; numa
 * assembleia pública, quem abre publica onde quiser. Nenhum dos dois casos
 * precisa de servidor, e um servidor aqui não teria o que esconder: a raiz na
 * rede acusa qualquer lista adulterada.
 */
const chave = (proposta: string) => `tessera:aptos:${proposta}`;

export function guardarLista(proposta: string, enderecos: string[]) {
  localStorage.setItem(chave(proposta), JSON.stringify(enderecos));
}

export function lerLista(proposta: string): string[] | null {
  const v = localStorage.getItem(chave(proposta));
  return v ? (JSON.parse(v) as string[]) : null;
}

/** A chave de anel de quem compareceu. Perder é perder o voto. */
const chaveAnel = (proposta: string) => `tessera:anel:${proposta}`;

export type ChaveDeAnel = {
  secreta: string;
  publica: string;
  /** A seção que a lista deu a esta pessoa. O anel de votar é o dela. */
  secao: number;
};

/**
 * A pública vai junto porque é ela que acha o índice do ramo dentro do anel na
 * hora de assinar. Derivá-la de novo daria no mesmo; guardá-la evita precisar
 * do wasm só para isso.
 */
export function guardarChaveDeAnel(proposta: string, c: ChaveDeAnel) {
  localStorage.setItem(chaveAnel(proposta), JSON.stringify(c));
}

export function lerChaveDeAnel(proposta: string): ChaveDeAnel | null {
  const v = localStorage.getItem(chaveAnel(proposta));
  return v ? (JSON.parse(v) as ChaveDeAnel) : null;
}

/**
 * Apaga a chave de anel. **Chamado assim que a cédula entra.**
 *
 * Depois de votar ela não serve para mais nada — e serve contra quem a tem:
 * de `secreta` sai a imagem `I = x·Hp`, e a imagem está na cédula, no ledger,
 * em claro. Quem ler este `localStorage` acha **qual das cédulas é sua**.
 *
 * Esse é exatamente o vínculo que o anel existe para não haver. Guardá-lo
 * depois do voto seria reconstruí-lo num arquivo local, de graça.
 *
 * Sobrescreve antes de remover, como `recibo::queimar` faz na CLI: o
 * `localStorage` não garante que remover apague os bytes.
 */
export function esquecerChaveDeAnel(proposta: string) {
  const k = chaveAnel(proposta);
  try {
    const v = localStorage.getItem(k);
    if (v) localStorage.setItem(k, "0".repeat(v.length));
    localStorage.removeItem(k);
  } catch {
    /* aba anônima ou cota: não pode derrubar o voto que já entrou */
  }
}
