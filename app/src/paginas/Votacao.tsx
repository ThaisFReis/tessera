import { mensagemErro } from "../mensagens";
import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import {
  fase as calcularFase,
  lerAnel,
  lerComparecimento,
  lerProposta,
  ledgerAtual,
  REDE,
  lerResultado,
  type PropostaRede,
} from "../rede";
import { Carregando, Estado, Icone } from "../ui";

const FASES: Record<string, string> = {
  agendada: "not started",
  comparecimento: "check-in open",
  votacao: "voting open",
  encerrada: "closed",
};

function Fato({ rotulo, children }: { rotulo: string; children: React.ReactNode }) {
  return <div><dt>{rotulo.toUpperCase()}</dt><dd>{children}</dd></div>;
}

export default function Votacao() {
  const { id = "" } = useParams();
  const [p, setP] = useState<PropostaRede | null>(null);
  const [anel, setAnel] = useState<number[]>([]);
  const [cedulas, setCedulas] = useState(0);
  const [placar, setPlacar] = useState<number[] | null>(null);
  const [ledger, setLedger] = useState(0);
  const [falha, setFalha] = useState("");

  useEffect(() => {
    let vivo = true;
    (async () => {
      try {
        const [prop, l] = await Promise.all([lerProposta(id), ledgerAtual()]);
        if (!vivo) return;
        setP(prop);
        setLedger(l);
        if (prop?.anel) {
          // Uma leitura por seção: o anel de cada uma é uma entrada própria, e
          // é isso que faz o custo da cédula parar de crescer com a votação.
          const por = await Promise.all(
            Array.from({ length: prop.secoes }, (_, s) => lerAnel(id, s)),
          );
          if (vivo) setAnel(por.map((a) => a.length));
        }
        const [conf] = await lerComparecimento(id);
        // Desde T-026 o placar é um só nos dois caminhos: a mesa grava
        // `resultado()` quando o limiar fecha, e a apuração sem mesa grava a
        // mesma chave em `publicar_resultado`. O que a seção guarda é uma
        // contagem e um ponto — nada que se possa mostrar como placar.
        const r = await lerResultado(id);
        if (vivo) {
          setCedulas(conf);
          setPlacar(r);
        }
      } catch (e) {
        if (vivo) setFalha(mensagemErro(e));
      }
    })();
    return () => {
      vivo = false;
    };
  }, [id]);

  if (falha) {
    return <main className="pagina"><Estado titulo="Could not load the proposal." curto>
      <p className="error-message" role="alert">{falha}</p>
      <Link className="primary-button" to="/votacoes">View votes <Icone nome="arrow" /></Link>
    </Estado></main>;
  }
  if (!p) return <main className="pagina"><Carregando titulo="Loading the proposal." /></main>;

  const f = calcularFase(p, ledger);

  return (
    <main className="pagina">
      <header>
        <span className="eyebrow">VOTE ·  {FASES[f].toUpperCase()}</span>
        <h1>{id.slice(0, 16)}…</h1>
        <p className="pagina-sub">
          {p.anel
            ? "Attendance and ballots are separate. Attendance is public; ring signatures hide who sent each ballot within the group."
            : "Identified voting: the ledger records the sender's address, while the choice is hidden by a commitment."}
</p>
      </header>

      {f !== "encerrada" && (
        <section>
          <h2>AVAILABLE NOW</h2>
          {p.anel && f === "comparecimento" && (
            <>
              <Link className="primary-button" to={`/comparecer/${id}`}>
                Confirm attendance <Icone nome="arrow" />
              </Link>
              <p>Check in with your identity to be able to vote later.</p>
            </>
          )}
          {f === "votacao" && (
            <Link className="primary-button" to={`/votar/${id}`}>
              Vote <Icone nome="arrow" />
            </Link>
          )}
          {f === "agendada" && <p>The window has not started. Return after ledger  {p.abre_em}.</p>}
        </section>
      )}
      {f === "encerrada" && placar && (
        <section>
          <h2>RESULTS</h2>
          <dl className="fatos">
            {placar.map((n, i) => (
              <div key={i}><dt>OPTION  {i + 1}</dt><dd className="valor-mono">{n}</dd></div>
            ))}
          </dl>
          <p>
            The contract verified the opening against the accumulated commitments before recording it. View {" "}
            <Link to={`/apurar/${id}`}>the tally </Link>.
</p>
        </section>
      )}
      {f === "encerrada" && !placar && (
        <section>
          <h2>VOTING HAS CLOSED</h2>
          {p.rodada !== 0n && (
            <p>
              No committee is needed: the opening key is the signature of beacon round  <span className="valor-mono">{String(p.rodada)}</span> . Once the beacon publishes it, tallying starts when you open the results page.</p>
          )}
          <Link className="secondary-button" to={`/apurar/${id}`}>View tally <Icone nome="arrow" /></Link>
        </section>
      )}

      <section>
        <h2>ON-CHAIN DETAILS</h2>
        <dl className="fatos">
          <Fato rotulo="window">
            <span className="valor-mono">opens  {p.abre_em} · closes  {p.fecha_em} · now  {ledger}</span>
          </Fato>
          <Fato rotulo="questions">
            {p.perguntas.map((q, i) => (
              <span key={i}>
                {i + 1}. {q.opcoes} options  {q.confidencial ? "confidential" : "public"};{" "}
              </span>
            ))}
          </Fato>
          <Fato rotulo="committee">
            {p.mesa.length === 0
              ? "none — tallying requires a time lock"
              : `${p.limiar} of ${p.mesa.length} to tally`}
          </Fato>
          <Fato rotulo="time lock">
            {p.rodada === 0n
              ? "none — tallying requires a compatible committee setup"
              : `round ${p.rodada} of the drand beacon — the opening key is not published before this round; ` +
                "after publication, anyone can tally"}
          </Fato>
          <Fato rotulo="who can open your ballot">
            {p.rodada !== 0n
              ? "after the round, anyone — allowing tallying without a committee. " +
                "The ring signature hides which group member sent the ballot. " +
                "Early opening would require collusion by a threshold of beacon operators."
              : p.anel
              ? "no one — a ring ballot does not share its blinding factor with the committee"
              : p.mesa.length === 1
                ? `one person: ${p.mesa[0]}`
                : `${p.limiar} of ${p.mesa.length}, acting together`}
          </Fato>
          {p.anel && (
            <Fato rotulo={p.secoes > 1 ? "sections" : "ring"}>
              {p.secoes > 1
                ? `${anel.reduce((a, b) => a + b, 0)} across ${p.secoes} sections: ${anel.join(", ")} — ` +
                  "each ballot uses the sender's section ring, so its cost depends on the section size"
                : `${anel[0] ?? 0} ${anel[0] === 1 ? "person checked in" : "people checked in"}` +
                  (anel[0] === 1 ? " — a one-person ring offers no anonymity" : "")}
            </Fato>
          )}
          <Fato rotulo="ballots cast">{cedulas}</Fato>
        </dl>
      </section>

      <p>
        <a href={`${REDE.explorer}/contract/${REDE.contrato}`} target="_blank" rel="noopener">
          View contract in explorer ↗ </a>
</p>
    </main>
  );
}
