import { mensagemErro } from "../mensagens";
import { useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { CarteiraLocal } from "../carteira";
import { guardarChaveDeAnel, guardarLista, lerLista } from "../lista";
import { comparecer, ehAberta, enderecoXdr, lerAbertura, lerProposta, lerSecao, lerSecoes } from "../rede";
import { carregar } from "../wasm";
import { Aviso, Icone, LinkBastidores, Passos, Trilha } from "../ui";
import { useDiario } from "./comum";
import "./votar.css";

const PASSOS = ["Building your Merkle proof", "Creating your participation key", "Confirming on Stellar"];

/* VOTANTE, primeiro ato: o caderno.
   É o único momento em que o nome da pessoa aparece — e é ele que permite voto
   obrigatório, porque `aptos − compareceram` é a lista de quem faltou. */
export default function Comparecer() {
  const { id = "" } = useParams();
  const [lista, setLista] = useState<string[]>(() => lerLista(id) ?? []);
  const [aberta, setAberta] = useState<boolean | null>(null);
  const [colada, setColada] = useState("");
  const [falha, setFalha] = useState("");
  const [ocupado, setOcupado] = useState(false);
  const [progresso, setProgresso] = useState(0);
  const diario = useDiario("comparecer", id);
  const ir = useNavigate();

  useEffect(() => {
    let vivo = true;
    lerProposta(id)
      .then((p) => vivo && setAberta(p ? ehAberta(p) : null))
      .catch(() => vivo && setAberta(null));
    return () => {
      vivo = false;
    };
  }, [id]);

  function aceitarColada() {
    const l = colada.split(/[\n,]/).map((x) => x.trim()).filter(Boolean);
    guardarLista(id, l);
    setLista(l);
  }

  async function enviar() {
    setFalha("");
    setOcupado(true);
    setProgresso(0);
    diario({ tipo: "ato", txt: "Started checking in with the participant's identity" });
    try {
      const w = await carregar();
      const c = await CarteiraLocal.abrir();
      const meu = c.endereco();
      const indice = lista.indexOf(meu);
      if (aberta === false && indice < 0) {
        throw new Error(
          `${meu} is not on this vote's eligibility list. The contract would reject it with NaoEstaNaListaDeAptos.`,
        );
      }

      const prop = await lerProposta(id);
      if (!prop) throw new Error("This vote was not found.");

      // Na votação aberta não há lista, logo não há caminho de Merkle — e a
      // seção vem da ordem de chegada; quem entrar no meio empurra, e é por
      // isso que há janela no footprint logo abaixo.
      const caminho = ehAberta(prop)
        ? { irmaos: [] as string[], indice: 0 }
        : (w.caminho_de(lista.map(enderecoXdr), new Uint32Array(lista.length).fill(1), indice) as {
            irmaos: string[];
            indice: number;
          });
      diario({ tipo: "val", txt: `Merkle path with ${caminho.irmaos.length} sibling${caminho.irmaos.length === 1 ? "" : "s"}` });

      // **A seção vem da baliza** (DEC-012), não da lista e não do
      // identificador da proposta. Enquanto saía da proposta, quem organizava
      // moía o identificador até pôr alguém numa seção cheia de atacantes.
      // A assinatura da rodada de abertura não existe quando a votação é
      // aberta, então não há o que moer — e o contrato confere a seção que esta
      // tela calcula.
      let seccao: number;
      if (ehAberta(prop)) {
        seccao = Math.max(0, (await lerSecoes(id)) - 1);
      } else if (prop.secoes > 1) {
        const assinatura = await lerAbertura(id);
        if (!assinatura) {
          throw new Error(
            "The beacon round that determines section allocation has not been registered yet. " +
              "Anyone can register it once it is published. Without it, " +
              "the contract rejects check-in.",
          );
        }
        seccao = w.secao_de(assinatura, enderecoXdr(meu), prop.secoes) as number;
        diario({ tipo: "val", txt: `section ${seccao} of ${prop.secoes} — derived from the beacon, not chosen` });
      } else {
        seccao = 0;
      }

      setProgresso(1);
      const chave = w.nova_chave_de_anel() as { secreta: string; publica: string };
      diario({ tipo: "nota", txt: "The secret ring key stays in this browser tab only" });
      diario({ tipo: "val", txt: `public key = ${chave.publica.slice(0, 24)}…` });

      setProgresso(2);
      await comparecer(
        c, id, chave.publica, caminho.irmaos, caminho.indice, seccao, diario,
        Math.max(lista.length, 40),
        // **Duas seções além da prevista, não três.** Medido em 2026-10-10
        // (§11-D): cada entrada `Anel` declarada entra no orçamento de
        // recursos da transação, e a quinta estoura o teto —
        // `ResourceLimitExceeded`, com uma pessoa só. Com três declaradas
        // além da prevista, a rajada de seis com limite 2 deixou 5 de 6; com
        // duas, entraram 6 de 6.
        ehAberta(prop) ? 2 : 0,
      );
      // Na aberta quem decide a seção é o contrato: pergunta a ele, não ao
      // palpite do cliente.
      const secao = (await lerSecao(id, meu)) ?? seccao;
      if (secao !== seccao) {
        diario({ tipo: "val", txt: `The contract assigned you to section ${secao}, in arrival order` });
      }
      // Sem ela não há voto depois. Perder esta aba é perder o voto.
      guardarChaveDeAnel(id, { ...chave, secao });
      ir(`/votacao/${id}`);
    } catch (e) {
      const msg = mensagemErro(e);
      setFalha(msg);
      diario({ tipo: "x", txt: msg });
    } finally {
      setOcupado(false);
    }
  }

  return (
    <main className="voting-page">
      <div className="voting-breadcrumb">
        <Link to={`/votacao/${id}`}><span aria-hidden="true">←</span> Back </Link>
      </div>
      <h1 className="sr-only">Your attendance</h1>
      <Trilha etapas={[
        { label: "Check-in", completo: false, ativo: true },
        { label: "Vote", completo: false, ativo: false },
        { label: "Confirmation", completo: false, ativo: false },
      ]} />

      <section className="ballot-panel" aria-label="Attendance confirmation" aria-busy={ocupado}>
        <div className="ballot-topline">
          <span className="eyebrow">VOTE ·  {id.slice(0, 8) || "—"}</span>
          <span className="ballot-status"><span className="status-dot" />Check-in </span>
        </div>

        <div className="ballot-body stage-enter">
          <h2>This is the attendance register.</h2>
          <p className="ballot-description">
            It is  <strong>identified and public </strong>: the ledger records your attendance. With an eligibility list, this reveals who is absent. It does not record your choice — your ballot comes later, from a different key.</p>

          {aberta === null ? (
            <p className="campo-ajuda" style={{ marginTop: 29 }}>Loading the vote…</p>
          ) : aberta ? (
            <div className="stage-enter">
              <p className="review-note" style={{ marginTop: 29 }}>
                <Icone nome="users" /> Open participation: no eligibility list is required.</p>
              <div className="ballot-action">
                <button className="primary-button" onClick={enviar} disabled={ocupado}>
                  {ocupado ? "Checking in…" : "Confirm my attendance"} <Icone nome="arrow" />
</button>
              </div>
              {ocupado && <Passos passos={PASSOS} atual={progresso} />}
            </div>
          ) : lista.length === 0 ? (
            <div className="stage-enter">
              <label className="campo" style={{ marginTop: 29 }}>
                <span className="eyebrow">ELIGIBILITY LIST </span>
                <textarea
                  value={colada}
                  onChange={(e) => setColada(e.target.value)}
                  rows={6}
                  placeholder="One G… address per line"
                />
                <span className="campo-ajuda">
                  The list is not stored on-chain: the contract stores only its 32-byte root. Paste the list provided by the organizer. </span>
              </label>
              <div className="ballot-action">
                <button className="primary-button" onClick={aceitarColada} disabled={!colada.trim()}>
                  Use this list <Icone nome="arrow" />
</button>
              </div>
            </div>
          ) : (
            <div className="stage-enter">
              <p className="review-note" style={{ marginTop: 29 }}>
                <Icone nome="users" /> {lista.length} eligible participants on this list.</p>
              <div className="ballot-action">
                <button className="primary-button" onClick={enviar} disabled={ocupado}>
                  {ocupado ? "Checking in…" : "Confirm my attendance"} <Icone nome="arrow" />
</button>
              </div>
              {ocupado && <Passos passos={PASSOS} atual={progresso} />}
            </div>
          )}
          {falha && <Aviso tipo="erro">{falha}</Aviso>}
        </div>
      </section>

      <details className="privacy-details">
        <summary><Icone nome="lock" /><span>Why attendance is public </span><span className="disclosure-plus" aria-hidden="true">+</span></summary>
        <p>
          With an eligibility list, the register shows who is absent, making mandatory participation possible. Attendance and ballots are separate: the register records who checked in, while the ballot box records the choices.</p>
        <p>
          The participation key created here stays in this browser tab. It signs your ballot later without attaching your address. Losing this tab means losing the key needed to vote.</p>
      </details>

      <p className="bastidores-rodape"><LinkBastidores origem="comparecer" /></p>
    </main>
  );
}
