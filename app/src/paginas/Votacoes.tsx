import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { assembleias, ledgerAtual, type Assembleia, type Fase } from "../rede";
import { Carregando, Estado, Icone } from "../ui";

const minutos = (l: number) => Math.max(0, Math.round((l * 6) / 60));

const BALDES: { fase: Fase | "abertas"; titulo: string; nota: string }[] = [
  { fase: "votacao", titulo: "votando agora", nota: "as cédulas estão entrando" },
  {
    fase: "comparecimento",
    titulo: "comparecimento aberto",
    nota: "o caderno ainda aceita gente; a votação abre depois",
  },
  { fase: "agendada", titulo: "vão abrir", nota: "a janela ainda não começou" },
  { fase: "encerrada", titulo: "encerradas", nota: "a urna fechou" },
];

function Linha({ a, ledger }: { a: Assembleia; ledger: number }) {
  const resta =
    a.fase === "encerrada" ? 0 : a.fase === "votacao" ? a.fecha_em - ledger : a.abre_em - ledger;
  return (
    <Link className="cartao" to={`/votacao/${a.proposta}`}>
      <span className="cartao-topo">
        <span className="cartao-id">{a.proposta.slice(0, 16)}…</span>
        {a.fase !== "encerrada" && (
          <span className="eyebrow">
            <Icone nome="clock" /> ~{minutos(resta)} MIN
          </span>
        )}
      </span>
      <span className="cartao-nota">
        {a.anel ? "caderno separado da urna" : "voto identificado"} ·{" "}
        {a.perguntas} pergunta{a.perguntas === 1 ? "" : "s"}
      </span>
    </Link>
  );
}

export default function Votacoes() {
  const [tudo, setTudo] = useState<Assembleia[] | null>(null);
  const [ledger, setLedger] = useState(0);
  const [falha, setFalha] = useState("");

  useEffect(() => {
    let vivo = true;
    (async () => {
      try {
        const [l, a] = await Promise.all([ledgerAtual(), assembleias()]);
        if (!vivo) return;
        setLedger(l);
        setTudo(a);
      } catch (e) {
        if (vivo) setFalha(String((e as Error).message ?? e));
      }
    })();
    return () => {
      vivo = false;
    };
  }, []);

  const vazio = tudo?.length === 0;

  return (
    <main className="pagina">
      <header>
        <span className="eyebrow">LEDGER {ledger || "—"}</span>
        <h1>Votações.</h1>
        <p className="pagina-sub">
          Lidas dos eventos da rede, sem índice e sem servidor. O que aparece aqui é o que o
          contrato registrou — nada passa por um intermediário que pudesse omitir uma votação.
        </p>
      </header>

      {falha && (
        <Estado titulo="A rede não respondeu." curto>
          <p className="error-message" role="alert">{falha}</p>
        </Estado>
      )}
      {tudo === null && !falha && <Carregando titulo="Lendo a rede." nota="Procurando votações nos eventos do contrato." />}
      {vazio && (
        <Estado titulo="Nenhuma nas últimas ~3 horas." curto>
          <p>A janela de eventos é curta de propósito: uma janela larga devolve zero em silêncio.</p>
          <Link className="primary-button" to="/abrir">Abrir uma votação <Icone nome="arrow" /></Link>
        </Estado>
      )}

      {tudo &&
        BALDES.map(({ fase, titulo, nota }) => {
          const n = tudo.filter((a) => a.fase === fase);
          if (!n.length) return null;
          return (
            <section key={fase} className="stage-enter">
              <h2>
                {titulo.toUpperCase()} · {n.length}
              </h2>
              <p>{nota}</p>
              <div>
                {n.map((a) => (
                  <Linha key={a.proposta} a={a} ledger={ledger} />
                ))}
              </div>
            </section>
          );
        })}
    </main>
  );
}
