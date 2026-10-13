import { mensagemErro } from "../mensagens";
import { useEffect, useRef, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { CarteiraEfemera } from "../carteira";
import { esquecerChaveDeAnel, lerChaveDeAnel } from "../lista";
import { fase, ledgerAtual, lerAnel, lerGeradorH, lerHp, lerProposta, REDE, votarAnonimo, type PropostaRede } from "../rede";
import { carregar } from "../wasm";
import { useDiario } from "./comum";
import { Icone, LinkBastidores, Passos, Trilha } from "../ui";
import "./votar.css";

type Etapa = "escolha" | "revisao" | "enviando" | "concluido";

const propostaDemo: PropostaRede = {
  perguntas: [{ opcoes: 3, confidencial: true }], raiz_aptos: new Uint8Array(),
  mesa: [], limiar: 0, abre_em: 0, fecha_em: Number.MAX_SAFE_INTEGER, anel: true, secoes: 1,
  rodada: 0n, rodada_abertura: 0n,
};
const opcoesDemo = [
  { titulo: "Approve", descricao: "I support the proposal." },
  { titulo: "Reject", descricao: "I oppose the proposal." },
  { titulo: "Abstain", descricao: "I prefer not to take a position." },
];

export default function Votar({ demo = false }: { demo?: boolean }) {
  const { id = "" } = useParams();
  // Remonta o fluxo quando a proposta muda para nunca reaproveitar uma escolha.
  return <Urna key={`${demo}:${id}`} id={id} demo={demo} />;
}

function Urna({ id, demo }: { id: string; demo: boolean }) {
  const [p, setP] = useState<PropostaRede | null>(demo ? propostaDemo : null);
  const [anel, setAnel] = useState<string[]>([]);
  const [ledger, setLedger] = useState(0);
  const [escolha, setEscolha] = useState<number | null>(null);
  const [falha, setFalha] = useState("");
  const [carregando, setCarregando] = useState(!demo);
  const [tentativa, setTentativa] = useState(0);
  const [etapa, setEtapa] = useState<Etapa>("escolha");
  const [progresso, setProgresso] = useState(0);
  const [tx, setTx] = useState("");
  const [guardada] = useState(() => {
    if (demo) return null;
    try { return lerChaveDeAnel(id); } catch { return null; }
  });
  const anotar = useDiario("votar", demo ? undefined : id);
  // A prévia não escreve no diário: `/bastidores` é o registro do que
  // aconteceu de verdade, e uma linha encenada ali valeria menos que nenhuma.
  const diario: typeof anotar = demo ? () => {} : anotar;
  const titulo = useRef<HTMLHeadingElement>(null);
  const trava = useRef(false);
  const montada = useRef(true);
  useEffect(() => { montada.current = true; return () => { montada.current = false; }; }, []);
  useEffect(() => { if (etapa !== "escolha") titulo.current?.focus(); }, [etapa]);

  useEffect(() => {
    if (demo) return;
    let vivo = true;
    setCarregando(true);
    setFalha("");
    (async () => {
      try {
        if (!/^[a-fA-F0-9]{64}$/.test(id)) throw new Error("This vote's address is invalid. Open a vote from the list.");
        const [prop, l] = await Promise.all([lerProposta(id), ledgerAtual()]);
        if (!prop) throw new Error("This vote was not found.");
        // O anel que importa é o da MINHA seção — não o da votação inteira.
        const membros = prop.anel ? await lerAnel(id, guardada?.secao ?? 0) : [];
        if (vivo) { setP(prop); setAnel(membros); setLedger(l); }
      } catch (e) {
        if (vivo) setFalha(mensagemErro(e));
      } finally { if (vivo) setCarregando(false); }
    })();
    return () => { vivo = false; };
  }, [id, demo, tentativa]);

  const apto = demo || !!guardada && anel.includes(guardada.publica);
  const faseAtual = p && (demo ? "votacao" : fase(p, ledger));
  const suportada = p?.anel && p.perguntas.length === 1 && p.perguntas[0].confidencial && p.perguntas[0].opcoes > 0;
  const bloqueio = !p ? "" : !suportada ? "This ballot is not yet supported by the dapp. Contact the organizer."
    : faseAtual !== "votacao" ? (faseAtual === "encerrada" ? "This vote has already closed." : "Voting has not started. Wait until check-in ends.")
    : !apto ? "Your check-in was not found in this browser. Use the browser where you confirmed your attendance." : "";
  const opcoes = demo ? opcoesDemo : Array.from({ length: p?.perguntas[0]?.opcoes ?? 0 }, (_, i) => ({ titulo: `Option ${i + 1}`, descricao: `Ballot option ID: ${i}` }));
  const opcao = escolha === null ? null : opcoes[escolha];
  const concluido = etapa === "concluido";
  const enviando = etapa === "enviando";
  const quantidade = demo ? 7 : anel.length;
  const passosEnvio = ["Preparing your ballot", "Creating a one-time signature", demo ? "Simulating confirmation" : "Confirming on Stellar"];

  async function enviar() {
    if (trava.current || escolha === null || bloqueio || !p || etapa !== "revisao") return;
    trava.current = true;
    setFalha(""); setProgresso(0); setEtapa("enviando");
    diario({ tipo: "ato", txt: demo ? "Confirmed the preview ballot" : "Confirmed the ballot" });
    try {
      if (demo) {
        for (let i = 0; i < 3; i++) {
          if (!montada.current) return;
          setProgresso(i);
          await new Promise((resolve) => setTimeout(resolve, 950));
        }
      } else {
        if (!guardada) throw new Error("Your participation key is not in this browser.");
        const atual = await ledgerAtual();
        setLedger(atual);
        if (fase(p, atual) !== "votacao") throw new Error("The voting window is not open. Your ballot was not sent.");
        const w = await carregar();
        const [hp, h] = await Promise.all([lerHp(id), lerGeradorH()]);
        const i = anel.indexOf(guardada.publica);
        if (i < 0) throw new Error("Your key is not in this vote's participant group.");
        // A fechadura: `p.rodada` é a rodada da baliza em que a cédula
        // destranca, e `0` é a proposta sem fechadura. O `r` é cifrado aqui
        // dentro, na mesma chamada em que nasce — sai da aba já ilegível, para
        // uma chave que ainda não existe no mundo.
        const c = w.cedula_anonima(id, hp, h, anel, i, guardada.secreta,
          [{ opcoes: p.perguntas[0].opcoes, confidencial: true }], new Uint32Array([escolha]),
          p.rodada,
        ) as { imagem: string; c0: string; z: string[]; cedula: { compromissos: string[]; provas: never[]; provas_soma: never[]; escolhas: number[]; cripto: string } };
        diario({ tipo: "nota", txt: "Ballot and participation proof prepared in this browser." });
        setProgresso(1);
        const efemera = await CarteiraEfemera.nascer();
        setProgresso(2);
        const hash = await votarAnonimo(efemera, id, guardada.secao, anel, c.imagem, c.c0, c.z, c.cedula.compromissos, c.cedula.provas, c.cedula.provas_soma, c.cedula.escolhas, c.cedula.cripto, diario);
        if (!montada.current) return;
        // A cédula entrou: a chave de anel deixa de ser útil e passa a ser só
        // o vínculo entre você e ela. Morre aqui.
        esquecerChaveDeAnel(id);
        diario({ tipo: "nota", txt: "Participation key deleted after the ballot was accepted" });
        setTx(hash);
      }
      if (montada.current) { setEscolha(null); setEtapa("concluido"); }
    } catch (e) {
      if (montada.current) { setFalha(mensagemErro(e)); setEtapa("revisao"); }
    } finally { trava.current = false; }
  }

  return <main className="voting-page">
    <div className="voting-breadcrumb"><Link to={demo ? "/" : `/votacao/${id}`}><span aria-hidden="true">←</span> Back </Link></div>
    <h1 className="sr-only">Your vote</h1>
    <Trilha etapas={[
      { label: "Check-in", completo: apto, ativo: !apto },
      { label: "Vote", completo: concluido, ativo: apto && !concluido },
      { label: "Confirmation", completo: concluido, ativo: concluido },
    ]} />

    <div className="voting-layout">
      <section className="ballot-panel" aria-label="Voting ballot" aria-busy={carregando || enviando}>
        <div className="ballot-topline"><span className="eyebrow">{demo ? "PROPOSAL 001" : `VOTE · ${id.slice(0, 8) || "—"}`}</span><span className="ballot-status"><span className="status-dot" />{demo ? "Demo" : carregando ? "Loading" : faseAtual === "votacao" ? "Voting open" : faseAtual === "encerrada" ? "Closed" : "Waiting"}</span></div>
        {carregando ? <div className="ballot-state" role="status"><span className="loading-ring" /><h2>Preparing your vote.</h2><p>Loading the proposal and participants from Stellar.</p></div> : !p ? <div className="ballot-state"><h2>Could not open this vote.</h2><p className="error-message" role="alert">{falha}</p><button className="primary-button" onClick={() => setTentativa(tentativa + 1)}>Try again <Icone nome="arrow" /></button><Link className="text-link" to="/votacoes">View votes </Link></div> : <>
          {etapa === "escolha" && <div className="ballot-body stage-enter">
            <h2>{demo ? "Should we fund the next round of projects?" : "What is your choice?"}</h2>
            {!demo && <p className="ballot-description">Refer to the question and option labels provided by the organizer.</p>}
            <fieldset className="ballot-options" disabled={!!bloqueio}><legend>Select an option</legend>{opcoes.map((o, i) => <label className={`ballot-option ${escolha === i ? "selected" : ""}`} key={i}>
              <input type="radio" name="voto" value={i} checked={escolha === i} onChange={() => { setEscolha(i); diario({ tipo: "ato", txt: "Selected an option — the choice is not recorded in the activity log" }); }} /><span className="option-radio" aria-hidden="true" /><span className="option-copy"><strong>{o.titulo}</strong>{!demo && <small>{o.descricao}</small>}</span>
            </label>)}</fieldset>
            {bloqueio && <p className="notice-message" role="status">{bloqueio} {!apto && faseAtual === "comparecimento" && <Link to={`/comparecer/${id}`}>Confirm attendance → </Link>}</p>}
            <div className="ballot-action"><button className="primary-button" disabled={escolha === null || !!bloqueio} onClick={() => { setFalha(""); setEtapa("revisao"); diario({ tipo: "ato", txt: "Opened ballot review" }); }}>Review my vote <Icone nome="arrow" /></button></div>

          </div>}
          {etapa === "revisao" && <div className="ballot-body review-body stage-enter"><span className="eyebrow">BEFORE YOU CONFIRM </span><h2 ref={titulo} tabIndex={-1}>Ready to confirm <br />your choice?</h2><p className="ballot-description">Review your vote. Once submitted, it cannot be changed.</p><div className="review-selection"><span className="eyebrow">YOUR CHOICE </span><strong>{opcao?.titulo}</strong><span>{opcao?.descricao}</span><Icone nome="lock" /></div><p className="review-note"><Icone nome="shield" /> Your ballot will be signed with a one-time key, separate from your identity.</p>{falha && <p className="error-message" role="alert">{falha}</p>}{bloqueio && <p className="notice-message" role="status">{bloqueio}</p>}<div className="ballot-action"><button className="text-button" onClick={() => setEtapa("escolha")}>← Change choice</button><button className="primary-button" disabled={!!bloqueio} onClick={enviar}>{demo ? "Confirm preview" : "Confirm my vote"}<Icone nome="arrow" /></button></div></div>}
          {enviando && <div className="ballot-state sending-state stage-enter"><span className="loading-ring" /><span className="eyebrow">{demo ? "SIMULATION IN PROGRESS" : "SUBMISSION IN PROGRESS"}</span><h2 ref={titulo} tabIndex={-1}>Submitting your vote.</h2><p>Keep this page open until confirmation.</p><Passos passos={passosEnvio} atual={progresso} /></div>}
          {concluido && <div className="ballot-state success-state stage-enter"><span className="success-symbol"><Icone nome="check" /></span><span className="eyebrow">{demo ? "SIMULATION COMPLETE" : "SUBMISSION CONFIRMED"}</span><h2 ref={titulo} tabIndex={-1}>{demo ? "Simulation complete." : "Vote confirmed."}</h2><p>{demo ? "You have completed the voting preview. No transaction was sent to the network." : "Your ballot was recorded on Stellar. This confirmation does not reveal your choice."}</p>{tx && <a className="receipt-link" href={`${REDE.explorer}/tx/${tx}`} target="_blank" rel="noopener noreferrer">View confirmation on Stellar ↗ <small>{tx.slice(0, 16)}…{tx.slice(-8)}</small></a>}{demo ? <button className="primary-button" onClick={() => { setEscolha(null); setEtapa("escolha"); }}>Try again <Icone nome="arrow" /></button> : <Link className="primary-button" to={`/votacao/${id}`}>Follow this vote <Icone nome="arrow" /></Link>}</div>}
        </>}
      </section>
    </div>
    {quantidade === 1 && <p className="notice-message">A group of only one person offers no anonymity.</p>}
    <details className="privacy-details">
      <summary><Icone nome="lock" /><span>About ballot privacy </span><span className="disclosure-plus" aria-hidden="true">+</span></summary>
      <p>Attendance is public. Your ballot uses a different key to separate your choice from your identity.</p>
      {quantidade > 0 && <p>{quantidade} participants in the group {demo ? " · illustrative data" : ""}.</p>}
      <p>The participation proof uses the group of checked-in voters. This ballot does not share its individual opening secret with a committee.</p>
      {p?.rodada ? <p>Instead of a committee, this vote uses a  <strong>time lock </strong>: the ballot is encrypted for round  {String(p.rodada)}  of the drand beacon. Before that round, the opening key is not published; early opening would require collusion by a threshold of beacon operators. Afterward,  <strong>anyone </strong>  can open the ballot and tally it without a committee. The ring signature still hides which group member sent it.</p> : <p>Tallying is not available for this mode.</p>}
      <p>After submission,  <strong>the app cannot retrieve your individual vote </strong>. It does not retain the number that hides your choice, which  {p?.rodada ? "leaves this tab encrypted for a future round, never in plain text" : "is never saved"}. The participation key is deleted once the ballot is accepted; retaining it could identify your ballot. This does not protect against someone watching your screen or a modified client recording your secrets.</p>
      <p>On testnet, the service funding the one-time key may see your IP address. Keep this browser tab until your ballot is confirmed.</p>
    </details>
    {demo && <p className="demo-note">Interactive preview · no real transactions</p>}
    {!demo && <p className="bastidores-rodape"><LinkBastidores origem="votar" /></p>}
  </main>;
}
