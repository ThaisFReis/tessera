import { mensagemErro } from "../mensagens";
import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { assembleias, ledgerAtual, type Assembleia, type Fase } from "../rede";
import { Carregando, Estado, Icone } from "../ui";

const minutos = (l: number) => Math.max(0, Math.round((l * 6) / 60));

const BALDES: { fase: Fase | "abertas"; titulo: string; nota: string }[] = [
  { fase: "votacao", titulo: "voting now", nota: "ballots are being accepted" },
  {
    fase: "comparecimento",
    titulo: "check-in open",
    nota: "participants can check in; voting opens afterward",
  },
  { fase: "agendada", titulo: "upcoming", nota: "the window has not started" },
  { fase: "encerrada", titulo: "closed", nota: "voting has ended" },
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
        {a.anel ? "attendance separate from ballots" : "identified voting"} ·{" "}
        {a.perguntas} question {a.perguntas === 1 ? "" : "s"}
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
        if (vivo) setFalha(mensagemErro(e));
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
        <h1>Votes.</h1>
        <p className="pagina-sub">
          Read directly from contract events on the network, without a separate index or backend. This list covers the recent event window available from the RPC provider.</p>
      </header>

      {falha && (
        <Estado titulo="The network did not respond." curto>
          <p className="error-message" role="alert">{falha}</p>
        </Estado>
      )}
      {tudo === null && !falha && <Carregando titulo="Reading the network." nota="Looking for votes in contract events." />}
      {vazio && (
        <Estado titulo="None in the last ~3 hours." curto>
          <p>Only recent events are queried. Older votes may still be accessible through their direct links.</p>
          <Link className="primary-button" to="/abrir">Create a vote <Icone nome="arrow" /></Link>
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
