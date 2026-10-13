import { mensagemErro } from "../mensagens";
import { useEffect, useRef, useState } from "react";
import { Link, useParams } from "react-router-dom";
import {
  ledgerAtual,
  lerComparecimento,
  lerProposta,
  lerResultado,
  REDE,
  type PropostaRede,
} from "../rede";
import { apurarSozinho, type FasePlacar } from "../placar";
import { useDiario } from "./comum";
import { Carregando, Estado, Icone, LinkBastidores } from "../ui";

/* ORGANIZADOR / QUALQUER PESSOA: a apuração.
   Com mesa, ela afirma os totais e o contrato confere contra o acumulado. Com
   fechadura de tempo não há quem afirme: a chave nasce sozinha no instante da
   rodada, e esta página faz a conta — sem ninguém clicar. Sem mesa e sem
   fechadura, não há apuração possível, e a página explica por quê. */
export default function Apurar() {
  const { id = "" } = useParams();
  const diario = useDiario("apurar", id);
  const [p, setP] = useState<PropostaRede | null>(null);
  const [cedulas, setCedulas] = useState(0);
  const [daMesa, setDaMesa] = useState<number[] | null>(null);
  const [fase, setFase] = useState<FasePlacar | null>(null);
  const [falha, setFalha] = useState("");
  // Uma aba só apura uma vez. Sem isto, o React em modo estrito dispararia
  // duas transações para o mesmo placar — e a segunda levaria `NaoMelhora`.
  const apurou = useRef(false);

  useEffect(() => {
    let vivo = true;
    (async () => {
      try {
        const [prop, ledger] = await Promise.all([lerProposta(id), ledgerAtual()]);
        if (!vivo || !prop) return setFalha("This proposal does not exist in the contract");
        const [conf] = await lerComparecimento(id);
        if (!vivo) return;
        setP(prop);
        setCedulas(conf);

        if (prop.rodada === 0n) {
          const r = await lerResultado(id);
          if (vivo) setDaMesa(r);
          return;
        }
        if (apurou.current) return;
        apurou.current = true;
        await apurarSozinho(id, prop, ledger, (f) => vivo && setFase(f), diario);
      } catch (e) {
        if (vivo) setFalha(mensagemErro(e));
      }
    })();
    return () => {
      vivo = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [id]);

  if (falha) {
    return <main className="pagina"><Estado titulo="Could not load the proposal." curto>
      <p className="error-message" role="alert">{falha}</p>
      <Link className="primary-button" to="/votacoes">View votes <Icone nome="arrow" /></Link>
    </Estado></main>;
  }
  if (!p) return <main className="pagina"><Carregando titulo="Loading the proposal." /></main>;

  return (
    <main className="pagina">
      <header>
        <span className="eyebrow">TALLY ·  {cedulas} {cedulas === 1 ? "BALLOT" : "BALLOTS"}</span>
        <h1>{id.slice(0, 16)}…</h1>
      </header>

      {p.rodada !== 0n ? <ComFechadura id={id} p={p} fase={fase} /> : daMesa ? (
        <section>
          <h2>RESULTS</h2>
          <Totais totais={daMesa} />
          <p>
            These numbers were read from the  <strong>contract </strong>. It verified the opening against the accumulated ballots before recording them, and would reject an inconsistent total with  <code>AberturaNaoFecha</code> .</p>
          <p>
            Ask the organizer for the option labels: the on-chain proposal stores only their counts.</p>
        </section>
      ) : p.mesa.length === 0 ? (
        <section>
          <h2>THIS CONFIGURATION CANNOT BE TALLIED</h2>
          <div className="painel">
            <p>
              This vote was created without a committee and  <strong>without a time lock </strong>. No opening shares were distributed, so no one can reconstruct the sum of the blinding factors needed to publish a total.</p>
            <p>
              The contract enforces this: a total without a valid opening is rejected with  <code>AberturaNaoFecha</code>. Even the organizer cannot bypass this check.</p>
            <p><strong>In this configuration, ballots remain hidden and no tally can be produced. </strong></p>
            <p>
              With a time lock, the beacon publishes the opening key after the voting window closes, allowing anyone to tally the ballots.</p>
          </div>
        </section>
      ) : (
        <section>
          <h2>THE COMMITTEE MUST COMBINE ITS SHARES</h2>
          <div className="painel">
            <p>
              {p.limiar} of  {p.mesa.length} members reconstruct the aggregate opening and submit the totals. The contract checks them against the accumulated commitments and  <strong>rejects inconsistent totals immediately </strong>.
</p>
            <p>
              This page does not combine committee shares. Use the CLI to tally:</p>
            <pre className="bloco-codigo">tessera apurar --proposta {id.slice(0, 16)}…</pre>
          </div>
        </section>
      )}

      <p>
        <a href={`${REDE.explorer}/contract/${REDE.contrato}`} target="_blank" rel="noopener">
          View in explorer ↗ </a>
        {p.rodada !== 0n && <> · <LinkBastidores /></>}
</p>
    </main>
  );
}

function Totais({ totais }: { totais: number[] }) {
  return (
    <div className="painel">
      <dl className="fatos">
        {totais.map((n, i) => (
          <div key={i}>
            <dt>OPTION  {i + 1}</dt>
            <dd className="valor-mono">{n}</dd>
          </div>
        ))}
      </dl>
    </div>
  );
}

/** A apuração pela fechadura de tempo, fase por fase. */
function ComFechadura({ id, p, fase }: { id: string; p: PropostaRede; fase: FasePlacar | null }) {
  if (!fase) return <Carregando titulo="Checking whether the time lock has opened." />;

  switch (fase.tipo) {
    case "janela-aberta":
      return (
        <section>
          <h2>VOTING IS STILL OPEN</h2>
          <div className="painel">
            <p>
              There are no results,  <em>not even partial results </em>. Remaining: {" "}
              <span className="valor-mono">{fase.faltam}</span> ledgers until voting closes.</p>
            <p>
              This page does not have a hidden tally. The opening key is the signature of beacon round  <span className="valor-mono">{String(p.rodada)}</span> , which is only published when that round arrives. The organizer, voters and this page do not have it beforehand.</p>
            <p>
              Producing it early would require collusion by a threshold of beacon operators. The contract's closing-time rule does  <em>not </em>  depend on that assumption: it rejects any tally before  <code>fecha_em</code>, using the ledger clock.</p>
          </div>
          <Link className="secondary-button" to={`/votacao/${id}`}>View vote <Icone nome="arrow" /></Link>
        </section>
      );

    case "esperando-relogio":
      return (
        <section>
          <h2>VOTING HAS CLOSED. THE TIME LOCK IS STILL LOCKED.</h2>
          <div className="painel">
            <p>
              The voting window has ended. Beacon round  <span className="valor-mono">{String(p.rodada)}</span>{" "}
              is due in  <span className="valor-mono">{fase.faltam}</span> seconds. Until then, the contract rejects tallying with  {" "}
              <code>RelogioAindaNaoAbriu</code> , including requests from this page.</p>
            <p>Reload after the round is published to start tallying automatically.</p>
          </div>
        </section>
      );

    case "lendo":
      return <Carregando titulo="Reading ballots from contract events." />;
    case "decifrando":
      return <Carregando titulo={`Decrypting ${fase.cedulas} ballots in this browser.`} />;
    case "publicando":
      return <Carregando titulo="Recording the tally on-chain." />;

    case "vazia":
      return (
        <section>
          <h2>THE BALLOT BOX IS EMPTY</h2>
          <div className="painel">
            <p>The window has closed and the time lock has opened, but no ballots were cast. There is nothing to tally.</p>
          </div>
        </section>
      );

    case "multissecao":
      return (
        <section>
          <h2>THIS VOTE HAS  {fase.secoes} SECTIONS</h2>
          <div className="painel">
            <p>
              Tallying requires the complete ordered ballot list for each section, but  <strong>this page could not reconstruct every section </strong>. No complete aggregate tally is available yet.</p>
            <p>
              {fase.publicadas} of  {fase.secoes} sections have been processed on-chain. Once all  {" "}
              {fase.secoes} are available, the aggregate tally can be displayed.</p>
            <p>
              Check that all required contract events are still available from the RPC provider.</p>
          </div>
        </section>
      );

    case "falha":
      return (
        <section>
          <h2>TALLYING DID NOT FINISH</h2>
          <div className="painel">
            <p className="error-message" role="alert">{fase.porque}</p>
            <p>
              The encrypted ballots remain on-chain and the round key is public.  <strong>Anyone </strong>  can retry from this browser or another one.</p>
          </div>
        </section>
      );

    case "pronto":
      return (
        <section>
          <h2>RESULTS</h2>
          <Totais totais={fase.totais} />
          <p>
            {fase.cedulas === null ? (
              <>
                <strong>{fase.abriram}</strong>{" "}
                {fase.abriram === 1 ? "ballot opened" : "ballots opened"}, according to the ledger. </>
            ) : (
              <>
                <strong>{fase.abriram} of  {fase.cedulas}</strong>{" "}
                {fase.cedulas === 1 ? "ballot opened" : "ballots opened"}
                {fase.abriram < fase.cedulas && (
                  <> — {fase.cedulas - fase.abriram} ciphertext {fase.cedulas - fase.abriram === 1 ? " could not be opened" : "s could not be opened"}; each lost only its own vote </>
                )}
                .
              </>
            )}
</p>
          <p>
            These numbers were computed by decrypting ballots in this browser using the signature of beacon round  <span className="valor-mono">{String(p.rodada)}</span> , verified with a pairing before any calculation. {" "}
            {fase.publicado
              ? "They are on-chain: the contract checked the commitment sum before recording them."
              : "They have not been recorded on-chain yet. Anyone can independently repeat the calculation."}
</p>
          {fase.tx && (
            <p>
              <a href={`${REDE.explorer}/tx/${fase.tx}`} target="_blank" rel="noopener">
                View tally transaction ↗ </a>
</p>
          )}
          <p>
            Ask the organizer for the option labels: the on-chain proposal stores only their counts.</p>
        </section>
      );

    case "sem-fechadura":
      return null;
  }
}
