import { useEffect, useMemo, useRef, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { assinar, historico, limpar as zerar, type Origem, type Passo } from "../diario";
import { ledgerAtual, REDE } from "../rede";
import { useLedger } from "./comum";
import "./bastidores.css";

/**
 * O outro lado da demonstração.
 *
 * A pessoa age numa janela; aqui aparece o que o contrato fez com aquilo. Os
 * dois não cabem na mesma tela, e tentar encaixá-los no pé de cada rota foi o
 * que motivou separar: um vídeo precisa das duas janelas lado a lado.
 */

const MS_POR_LINHA = 22;

const MARCA: Record<Passo["tipo"], string> = {
  cmd: "$", val: " ", ok: "✓", x: "✗", nota: "//", ato: "›",
};

const ORIGENS: Record<Origem, string> = {
  abrir: "organizador", comparecer: "caderno", votar: "urna", apurar: "apuração",
};

const relogio = (t: number) =>
  new Date(t).toLocaleTimeString("pt-BR", { hour12: false });

export default function Bastidores() {
  const [params] = useSearchParams();
  const filtro = params.get("origem") as Origem | null;

  const [tudo, setTudo] = useState<Passo[]>(() => historico());
  const [visiveis, setVisiveis] = useState(0);
  const [preso, setPreso] = useState(true);
  const ledger = useLedger(ledgerAtual);
  const quantos = useRef(0);
  const fim = useRef<HTMLDivElement>(null);

  const linhas = useMemo(
    () => (filtro ? tudo.filter((p) => p.origem === filtro) : tudo),
    [tudo, filtro],
  );

  useEffect(() => assinar(
    (p) => setTudo((v) => [...v, p]),
    () => { setTudo([]); quantos.current = 0; setVisiveis(0); },
  ), []);

  const reduzido = useMemo(
    () => typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches,
    [],
  );

  /**
   * A datilografia, portada de `console/index.html`.
   *
   * O alvo vem do relógio, não de um contador: `requestAnimationFrame` já
   * travou essa renderização em 6 de 19 linhas quando a aba perdeu o foco, e
   * uma gravação de tela é exatamente a situação em que a aba perde o foco.
   * Com o alvo derivado do tempo, um atraso é recuperado no próximo disparo em
   * vez de virar uma linha perdida.
   */
  useEffect(() => {
    if (quantos.current > linhas.length) { quantos.current = 0; setVisiveis(0); }
    if (reduzido) { quantos.current = linhas.length; setVisiveis(linhas.length); return; }
    if (quantos.current >= linhas.length) return;
    let id: ReturnType<typeof setTimeout>;
    const t0 = performance.now();
    const base = quantos.current;
    const bater = () => {
      const alvo = Math.min(
        linhas.length,
        base + Math.max(1, Math.ceil((performance.now() - t0) / MS_POR_LINHA)),
      );
      quantos.current = alvo;
      setVisiveis(alvo);
      if (alvo < linhas.length) id = setTimeout(bater, MS_POR_LINHA);
    };
    bater();
    return () => clearTimeout(id);
  }, [linhas.length, reduzido]);

  useEffect(() => {
    if (preso) fim.current?.scrollIntoView({ block: "end" });
  }, [visiveis, preso]);

  const mostradas = linhas.slice(0, visiveis);

  return <main className="bastidores">
    <header className="bast-topo">
      <div>
        <span className="eyebrow">BASTIDORES{filtro ? ` · ${ORIGENS[filtro].toUpperCase()}` : ""}</span>
        <h1>O que acontece do outro lado.</h1>
      </div>
      <dl className="bast-meta">
        <div><dt>contrato</dt><dd><a href={`${REDE.explorer}/contract/${REDE.contrato}`} target="_blank" rel="noopener noreferrer">{REDE.contrato.slice(0, 8)}…{REDE.contrato.slice(-4)} ↗</a></dd></div>
        <div><dt>ledger</dt><dd>{ledger || "—"}</dd></div>
        <div><dt>canal</dt><dd><span className="status-dot" /> ao vivo</dd></div>
      </dl>
    </header>

    <div className="bast-barra">
      <span>{linhas.length} {linhas.length === 1 ? "passo" : "passos"}</span>
      <div>
        <button className="text-button" onClick={() => setPreso(!preso)} aria-pressed={!preso}>
          {preso ? "soltar a rolagem" : "prender no fim"}
        </button>
        <button className="text-button" onClick={zerar}>limpar</button>
      </div>
    </div>

    <div className="bast-term" role="log" aria-live="polite" aria-label="Diário da rodada">
      {mostradas.length === 0
        ? <p className="bast-vazio">
            Nada ainda. Abra o dapp em outra janela e comece uma rodada —
            cada passo aparece aqui, nesta, enquanto acontece lá.
          </p>
        : mostradas.map((p, i) => <div className={`bast-linha t-${p.tipo}`} key={`${p.t}-${i}`}>
            <span className="bast-hora">{relogio(p.t)}</span>
            <span className="bast-marca" aria-hidden="true">{MARCA[p.tipo]}</span>
            <span className="bast-txt">{p.txt}</span>
            {!filtro && <span className="bast-origem">{ORIGENS[p.origem]}</span>}
          </div>)}
      <div ref={fim} />
    </div>
  </main>;
}
