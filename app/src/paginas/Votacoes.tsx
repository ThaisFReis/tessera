import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { assembleias, ledgerAtual, type Assembleia, type Fase } from "../rede";
import { Erro } from "./comum";

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
    <li>
      <Link to={`/votacao/${a.proposta}`}>{a.proposta.slice(0, 16)}…</Link>{" "}
      {a.anel ? "[caderno separado da urna]" : "[voto identificado]"}{" "}
      {a.perguntas} pergunta{a.perguntas === 1 ? "" : "s"}
      {a.fase !== "encerrada" && <> · ~{minutos(resta)}min</>}
    </li>
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

  return (
    <main>
      <h1>Votações</h1>
      <p>ledger {ledger || "—"} · lidas dos eventos da rede, sem índice e sem servidor</p>
      <Erro msg={falha} />
      {tudo === null && !falha && <p>lendo a rede…</p>}
      {tudo?.length === 0 && <p>nenhuma nas últimas ~3 horas.</p>}
      {tudo &&
        BALDES.map(({ fase, titulo, nota }) => {
          const n = tudo.filter((a) => a.fase === fase);
          if (!n.length) return null;
          return (
            <section key={fase}>
              <h2>
                {titulo} ({n.length})
              </h2>
              <p>{nota}</p>
              <ul>
                {n.map((a) => (
                  <Linha key={a.proposta} a={a} ledger={ledger} />
                ))}
              </ul>
            </section>
          );
        })}
    </main>
  );
}
