import { Link } from "react-router-dom";
import { REDE } from "../rede";
import { Icone } from "../ui";
import "./inicio.css";

const ETAPAS = [
  { titulo: "Check in.", texto: "Register your attendance. With an eligibility list, the register shows who checked in and who is absent." },
  { titulo: "Vote.", texto: "In anonymous mode, a ring signature proves membership in the group without identifying which participant sent the ballot." },
  { titulo: "Verify.", texto: "With a time lock, anyone can tally after voting closes and the beacon round is published. The dapp starts tallying when you open the results page." },
];

export default function Inicio() {
  return (
    <main className="landing">
      <section className="landing-hero" aria-labelledby="inicio-titulo">
        <span className="eyebrow">VOTING ON SOROBAN · STELLAR TESTNET </span>
        <h1 id="inicio-titulo">Your attendance is public. <br /><span>Your ballot is anonymous. </span></h1>
        <p className="landing-intro">
          A register for participants. A ballot box for choices. In Tessera's anonymous mode, the proof of participation does not identify who sent each ballot.</p>
        <div className="landing-actions">
          <Link className="primary-button" to="/demo/votar">Try the ballot <Icone nome="arrow" /></Link>
          <Link className="secondary-button" to="/votacoes">View votes </Link>
        </div>
        <p className="landing-note">The preview sends no transactions. Live votes run on testnet.</p>
      </section>

      <section className="landing-flow" aria-labelledby="fluxo-titulo">
        <div className="landing-section-heading">
          <span className="eyebrow">HOW IT WORKS </span>
          <h2 id="fluxo-titulo">Three steps. <br />One collective decision.</h2>
        </div>
        <ol className="landing-steps">
          {ETAPAS.map((etapa, i) => (
            <li key={etapa.titulo}>
              <span className="landing-step-number" aria-hidden="true">0{i + 1}</span>
              <div><h3>{etapa.titulo}</h3><p>{etapa.texto}</p></div>
            </li>
          ))}
        </ol>
      </section>

      <section className="landing-limits" aria-labelledby="limites-titulo">
        <div className="landing-section-heading">
          <span className="eyebrow">BEFORE YOU PARTICIPATE </span>
          <h2 id="limites-titulo">What stays public.</h2>
          <p>Attendance and the aggregate tally are public. A unanimous result can reveal everyone's choice.</p>
        </div>
        <div className="landing-disclosures">
          <details open>
            <summary>Privacy depends on the voting mode.</summary>
            <p>The Pedersen commitment hides the choice. With a time lock, ballot contents can be opened after the round; sender anonymity depends on the ring signature and the size of the group. A one-person ring offers no anonymity.</p>
          </details>
          <details>
            <summary>How are votes tallied?</summary>
            <p>Time-locked tallying depends on the drand beacon. If it stops, there is no fallback; collusion by a threshold of operators could open contents early. The contract only accepts results after voting closes. Without a time lock, committee tallying uses the CLI. Ring ballots without a committee or time lock cannot be tallied.</p>
          </details>
          <details>
            <summary>Who can participate?</summary>
            <p>In a restricted vote, the organizer defines the eligibility list. In an open vote, one person can create multiple wallets: this demonstrates privacy but does not guarantee one vote per person. Ring membership is public.</p>
          </details>
          <details>
            <summary>What are the other limitations?</summary>
            <p>Ring anonymity is computational, not perfect. The protocol cannot prevent someone from watching your screen while you vote or a modified client from recording your choice. The project has not been independently audited and runs on testnet.</p>
          </details>
        </div>
      </section>

      <section className="landing-end" aria-labelledby="organizar-titulo">
        <div><span className="eyebrow">FOR ORGANIZERS </span><h2 id="organizar-titulo">Create the next vote.</h2></div>
        <Link className="secondary-button" to="/abrir">Organize a vote <Icone nome="arrow" /></Link>
      </section>
      <a className="landing-contract" href={`${REDE.explorer}/contract/${REDE.contrato}`} target="_blank" rel="noopener noreferrer">
        View the testnet contract  <span>{REDE.contrato.slice(0, 8)}…{REDE.contrato.slice(-4)} ↗</span>
      </a>
    </main>
  );
}
