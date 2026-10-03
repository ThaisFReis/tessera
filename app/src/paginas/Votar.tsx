import { useEffect, useRef, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { CarteiraEfemera } from "../carteira";
import { lerChaveDeAnel } from "../lista";
import { fase, ledgerAtual, lerAnel, lerGeradorH, lerHp, lerProposta, REDE, votarAnonimo, type PropostaRede } from "../rede";
import { carregar } from "../wasm";
import { useDiario } from "./comum";
import { Icone, LinkBastidores, Passos, Trilha } from "../ui";
import "./votar.css";

type Etapa = "escolha" | "revisao" | "enviando" | "concluido";

const propostaDemo: PropostaRede = {
  perguntas: [{ opcoes: 3, confidencial: true }], raiz_aptos: new Uint8Array(),
  mesa: [], limiar: 0, abre_em: 0, fecha_em: Number.MAX_SAFE_INTEGER, anel: true,
};
const opcoesDemo = [
  { titulo: "Aprovar", descricao: "Sou a favor da proposta." },
  { titulo: "Rejeitar", descricao: "Sou contra a proposta." },
  { titulo: "Abster-se", descricao: "Prefiro não me posicionar." },
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
        if (!/^[a-fA-F0-9]{64}$/.test(id)) throw new Error("O endereço desta votação é inválido. Abra uma votação pela lista.");
        const [prop, l] = await Promise.all([lerProposta(id), ledgerAtual()]);
        if (!prop) throw new Error("Esta votação não foi encontrada.");
        const membros = prop.anel ? await lerAnel(id) : [];
        if (vivo) { setP(prop); setAnel(membros); setLedger(l); }
      } catch (e) {
        if (vivo) setFalha(String((e as Error).message ?? e));
      } finally { if (vivo) setCarregando(false); }
    })();
    return () => { vivo = false; };
  }, [id, demo, tentativa]);

  const apto = demo || !!guardada && anel.includes(guardada.publica);
  const faseAtual = p && (demo ? "votacao" : fase(p, ledger));
  const suportada = p?.anel && p.perguntas.length === 1 && p.perguntas[0].confidencial && p.perguntas[0].opcoes > 0;
  const bloqueio = !p ? "" : !suportada ? "Esta cédula ainda não é compatível com a votação pelo app. Consulte o organizador."
    : faseAtual !== "votacao" ? (faseAtual === "encerrada" ? "Esta votação já foi encerrada." : "A votação ainda não começou. Aguarde o fim do comparecimento.")
    : !apto ? "Seu comparecimento não foi encontrado neste navegador. Use o navegador em que você confirmou sua presença." : "";
  const opcoes = demo ? opcoesDemo : Array.from({ length: p?.perguntas[0]?.opcoes ?? 0 }, (_, i) => ({ titulo: `Opção ${i + 1}`, descricao: `Identificador da cédula: ${i}` }));
  const opcao = escolha === null ? null : opcoes[escolha];
  const concluido = etapa === "concluido";
  const enviando = etapa === "enviando";
  const quantidade = demo ? 7 : anel.length;
  const passosEnvio = ["Preparando sua cédula", "Criando uma assinatura de uso único", demo ? "Simulando a confirmação" : "Confirmando na Stellar"];

  async function enviar() {
    if (trava.current || escolha === null || bloqueio || !p || etapa !== "revisao") return;
    trava.current = true;
    setFalha(""); setProgresso(0); setEtapa("enviando");
    diario({ tipo: "ato", txt: demo ? "confirmou a cédula na prévia" : "confirmou a cédula" });
    try {
      if (demo) {
        for (let i = 0; i < 3; i++) {
          if (!montada.current) return;
          setProgresso(i);
          await new Promise((resolve) => setTimeout(resolve, 950));
        }
      } else {
        if (!guardada) throw new Error("Sua chave de participação não está neste navegador.");
        const atual = await ledgerAtual();
        setLedger(atual);
        if (fase(p, atual) !== "votacao") throw new Error("A janela de votação não está aberta. Sua cédula não foi enviada.");
        const w = await carregar();
        const [hp, h] = await Promise.all([lerHp(id), lerGeradorH()]);
        const i = anel.indexOf(guardada.publica);
        if (i < 0) throw new Error("Sua chave não está no grupo de participantes desta votação.");
        const c = w.cedula_anonima(id, hp, h, anel, i, guardada.secreta,
          [{ opcoes: p.perguntas[0].opcoes, confidencial: true }], new Uint32Array([escolha]),
        ) as { imagem: string; c0: string; z: string[]; cedula: { compromissos: string[]; provas: never[]; provas_soma: never[]; escolhas: number[] } };
        diario({ tipo: "nota", txt: "Cédula e prova de participação preparadas neste navegador." });
        setProgresso(1);
        const efemera = await CarteiraEfemera.nascer();
        setProgresso(2);
        const hash = await votarAnonimo(efemera, id, anel, c.imagem, c.c0, c.z, c.cedula.compromissos, c.cedula.provas, c.cedula.provas_soma, c.cedula.escolhas, diario);
        if (!montada.current) return;
        setTx(hash);
      }
      if (montada.current) { setEscolha(null); setEtapa("concluido"); }
    } catch (e) {
      if (montada.current) { setFalha(String((e as Error).message ?? e)); setEtapa("revisao"); }
    } finally { trava.current = false; }
  }

  return <main className="voting-page">
    <div className="voting-breadcrumb"><Link to={demo ? "/" : `/votacao/${id}`}><span aria-hidden="true">←</span> Voltar</Link></div>
    <h1 className="sr-only">Seu voto</h1>
    <Trilha etapas={[
      { label: "Presença", completo: apto, ativo: !apto },
      { label: "Voto", completo: concluido, ativo: apto && !concluido },
      { label: "Confirmação", completo: concluido, ativo: concluido },
    ]} />

    <div className="voting-layout">
      <section className="ballot-panel" aria-label="Cédula de votação" aria-busy={carregando || enviando}>
        <div className="ballot-topline"><span className="eyebrow">{demo ? "PROPOSTA 001" : `VOTAÇÃO · ${id.slice(0, 8) || "—"}`}</span><span className="ballot-status"><span className="status-dot" />{demo ? "Demonstração" : carregando ? "Consultando" : faseAtual === "votacao" ? "Votação aberta" : faseAtual === "encerrada" ? "Encerrada" : "Aguardando"}</span></div>
        {carregando ? <div className="ballot-state" role="status"><span className="loading-ring" /><h2>Preparando sua votação.</h2><p>Consultando a proposta e os participantes na Stellar.</p></div> : !p ? <div className="ballot-state"><h2>Não foi possível abrir a votação.</h2><p className="error-message" role="alert">{falha}</p><button className="primary-button" onClick={() => setTentativa(tentativa + 1)}>Tentar novamente <Icone nome="arrow" /></button><Link className="text-link" to="/votacoes">Ver votações</Link></div> : <>
          {etapa === "escolha" && <div className="ballot-body stage-enter">
            <h2>{demo ? "Devemos destinar recursos ao próximo ciclo de projetos?" : "Qual é a sua escolha?"}</h2>
            {!demo && <p className="ballot-description">Consulte o enunciado e a correspondência das opções fornecidos pelo organizador.</p>}
            <fieldset className="ballot-options" disabled={!!bloqueio}><legend>Selecione uma opção</legend>{opcoes.map((o, i) => <label className={`ballot-option ${escolha === i ? "selected" : ""}`} key={i}>
              <input type="radio" name="voto" value={i} checked={escolha === i} onChange={() => { setEscolha(i); diario({ tipo: "ato", txt: "escolheu uma opção — qual, nem o diário sabe" }); }} /><span className="option-radio" aria-hidden="true" /><span className="option-copy"><strong>{o.titulo}</strong>{!demo && <small>{o.descricao}</small>}</span>
            </label>)}</fieldset>
            {bloqueio && <p className="notice-message" role="status">{bloqueio} {!apto && faseAtual === "comparecimento" && <Link to={`/comparecer/${id}`}>Confirmar presença →</Link>}</p>}
            <div className="ballot-action"><button className="primary-button" disabled={escolha === null || !!bloqueio} onClick={() => { setFalha(""); setEtapa("revisao"); diario({ tipo: "ato", txt: "foi revisar a cédula" }); }}>Revisar meu voto <Icone nome="arrow" /></button></div>

          </div>}
          {etapa === "revisao" && <div className="ballot-body review-body stage-enter"><span className="eyebrow">ANTES DE CONFIRMAR</span><h2 ref={titulo} tabIndex={-1}>Tudo certo com<br />a sua escolha?</h2><p className="ballot-description">Confira seu voto. Após o envio, ele não poderá ser alterado.</p><div className="review-selection"><span className="eyebrow">SUA ESCOLHA</span><strong>{opcao?.titulo}</strong><span>{opcao?.descricao}</span><Icone nome="lock" /></div><p className="review-note"><Icone nome="shield" /> A cédula será assinada por uma chave de uso único, separada da sua identidade.</p>{falha && <p className="error-message" role="alert">{falha}</p>}{bloqueio && <p className="notice-message" role="status">{bloqueio}</p>}<div className="ballot-action"><button className="text-button" onClick={() => setEtapa("escolha")}>← Alterar escolha</button><button className="primary-button" disabled={!!bloqueio} onClick={enviar}>{demo ? "Confirmar na prévia" : "Confirmar meu voto"}<Icone nome="arrow" /></button></div></div>}
          {enviando && <div className="ballot-state sending-state stage-enter"><span className="loading-ring" /><span className="eyebrow">{demo ? "SIMULAÇÃO EM ANDAMENTO" : "ENVIO EM ANDAMENTO"}</span><h2 ref={titulo} tabIndex={-1}>Enviando seu voto.</h2><p>Mantenha esta página aberta até a confirmação.</p><Passos passos={passosEnvio} atual={progresso} /></div>}
          {concluido && <div className="ballot-state success-state stage-enter"><span className="success-symbol"><Icone nome="check" /></span><span className="eyebrow">{demo ? "SIMULAÇÃO CONCLUÍDA" : "REGISTRO CONFIRMADO"}</span><h2 ref={titulo} tabIndex={-1}>{demo ? "Simulação concluída." : "Voto confirmado."}</h2><p>{demo ? "Você percorreu a experiência de votação. Nenhuma transação foi enviada à rede." : "Sua cédula foi registrada na Stellar. Sua escolha não aparece neste comprovante."}</p>{tx && <a className="receipt-link" href={`${REDE.explorer}/tx/${tx}`} target="_blank" rel="noopener noreferrer">Ver comprovante na Stellar ↗<small>{tx.slice(0, 16)}…{tx.slice(-8)}</small></a>}{demo ? <button className="primary-button" onClick={() => { setEscolha(null); setEtapa("escolha"); }}>Explorar novamente <Icone nome="arrow" /></button> : <Link className="primary-button" to={`/votacao/${id}`}>Acompanhar votação <Icone nome="arrow" /></Link>}</div>}
        </>}
      </section>
    </div>
    {quantidade === 1 && <p className="notice-message">Com apenas uma pessoa, o grupo não oferece anonimato.</p>}
    <details className="privacy-details">
      <summary><Icone nome="lock" /><span>Sobre a privacidade do voto</span><span className="disclosure-plus" aria-hidden="true">+</span></summary>
      <p>A presença é pública. A cédula usa outra chave para separar sua escolha da sua identidade.</p>
      {quantidade > 0 && <p>{quantidade} participantes no conjunto{demo ? " · dados ilustrativos" : ""}.</p>}
      <p>A prova de participação usa o grupo de quem compareceu. Esta cédula não compartilha com a mesa o segredo que permitiria abri-la individualmente; a apuração deste modo ainda não está disponível.</p>
      <p>Na testnet, o serviço que financia a chave de uso único pode ver seu IP. Preserve a chave de participação neste navegador.</p>
    </details>
    {demo && <p className="demo-note">Prévia interativa · nenhuma transação real</p>}
    {!demo && <p className="bastidores-rodape"><LinkBastidores origem="votar" /></p>}
  </main>;
}
