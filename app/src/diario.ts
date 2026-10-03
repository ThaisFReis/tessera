/**
 * O diário: cada passo que o dapp dá, em ordem, e visível de outra janela.
 *
 * Ele existe para o vídeo da demonstração, onde a pessoa age numa janela e o
 * contrato responde na outra. Isso obriga três coisas que um `useState` dentro
 * de um componente não dá: atravessar janelas, sobreviver a uma recarga, e
 * dizer de onde cada linha veio.
 *
 * `BroadcastChannel` faz a entrega ao vivo entre janelas da mesma origem sem
 * servidor — e o projeto inteiro é sem servidor. O `localStorage` é lastro,
 * não canal: ele é o que permite abrir `/bastidores` depois e ainda ver a
 * rodada que já aconteceu.
 */

export type Origem = "abrir" | "comparecer" | "votar" | "apurar";
/** `ato` é o que a pessoa fez; os outros são o que a máquina fez. */
export type Tipo = "cmd" | "val" | "ok" | "x" | "nota" | "ato";

export type Passo = {
  tipo: Tipo;
  txt: string;
  t: number;
  origem: Origem;
  proposta?: string;
};

/** O que as páginas recebem: já sabem a própria origem, só passam o resto. */
export type Anotar = (p: { tipo: Tipo; txt: string }) => void;

type Recado = { passo: Passo } | { limpar: true };

const CANAL = "tessera:diario";
const LOG = "tessera:diario:log";
const TETO = 300;

/* ---------- o guarda ---------- */

/**
 * Enquanto o diário era efêmero e local, um segredo vazado nele sumia com a
 * aba. Persistido e transmitido entre janelas, ele **fica** — e vira o lugar
 * mais fácil do sistema para achar o `r` de alguém. É o mesmo risco que o teste
 * `o_json_nao_tem_lugar_para_aleatoriedade` e a lista `PROIBIDO` de
 * `console/embutir.py` já cercam nos outros dois lugares onde ele aparece.
 *
 * Mas um crivo por palavra não serve aqui. `Comparecer` escreve *"a chave
 * secreta do anel fica nesta aba, e só aqui"* — uma frase que contém `chave
 * secreta`, não vaza nada, e é justamente a que explica a garantia. E escreve
 * `chave pública = 8a10…`, que é um valor e é público de propósito.
 *
 * Então o crivo olha para a **conjunção**: um nome de segredo e um valor na
 * mesma linha. Nome sozinho passa; valor sozinho passa; os dois juntos, não.
 */
const NOMES = /\bsecret[ao]|\bsegredo|\baleatori|\bacaso|\bprivad[ao]|\bnonce\b|\bsemente\b/i;
/** Um valor é uma corrida longa de hexadecimal ou base64 — o formato de todo
 *  escalar e todo ponto que este sistema manipula. */
const VALOR = /[0-9a-f]{16,}|[A-Za-z0-9+/_-]{24,}={0,2}/i;
/** E o `r` nunca tem nome comprido: ele apareceria como `r = …`, e só. */
const FATOR = /\br\s*[=:]\s*\S/i;

export function suspeito(txt: string): boolean {
  return FATOR.test(txt) || (NOMES.test(txt) && VALOR.test(txt));
}

export const RECUSA = "passo recusado pelo guarda do diário";

/* ---------- a loja ---------- */

/* Preguiçoso de propósito: assim `suspeito()` pode ser importado por um teste
   de linha de comando sem abrir canal nenhum. */
let aberto: BroadcastChannel | null | undefined;
function canal(): BroadcastChannel | null {
  if (aberto === undefined) {
    aberto = typeof BroadcastChannel !== "undefined" ? new BroadcastChannel(CANAL) : null;
  }
  return aberto;
}
const ouvintes = new Set<(r: Recado) => void>();

export function historico(): Passo[] {
  try {
    const v = localStorage.getItem(LOG);
    return v ? (JSON.parse(v) as Passo[]) : [];
  } catch {
    return [];
  }
}

function gravar(passos: Passo[]) {
  try {
    localStorage.setItem(LOG, JSON.stringify(passos.slice(-TETO)));
  } catch {
    /* aba anônima, cota cheia: o diário é conveniência e não pode derrubar o voto */
  }
}

export function anotar(p: Passo) {
  const limpo: Passo = suspeito(p.txt) ? { ...p, tipo: "x", txt: RECUSA } : p;
  gravar([...historico(), limpo]);
  const recado: Recado = { passo: limpo };
  canal()?.postMessage(recado);
  ouvintes.forEach((f) => f(recado));
}

export function limpar() {
  try {
    localStorage.removeItem(LOG);
  } catch {
    /* idem */
  }
  const recado: Recado = { limpar: true };
  canal()?.postMessage(recado);
  ouvintes.forEach((f) => f(recado));
}

/** Quem acompanha: recebe cada passo novo, e o aviso de que a tela zerou. */
export function assinar(aoPasso: (p: Passo) => void, aoLimpar: () => void): () => void {
  const entregar = (r: Recado) => ("limpar" in r ? aoLimpar() : aoPasso(r.passo));
  ouvintes.add(entregar);
  const daOutraJanela = (e: MessageEvent<Recado>) => entregar(e.data);
  canal()?.addEventListener("message", daOutraJanela);
  return () => {
    ouvintes.delete(entregar);
    canal()?.removeEventListener("message", daOutraJanela);
  };
}
