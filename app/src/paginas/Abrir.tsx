import { mensagemErro } from "../mensagens";
import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { CarteiraLocal } from "../carteira";
import { guardarLista } from "../lista";
import { abrir, enderecoXdr, ledgerAtual, MAX_OPCOES, RAIZ_ABERTA } from "../rede";
import { carregar } from "../wasm";
import { explicar, secaoNasceuPequena } from "../secoes";
import { Aviso, Icone, LinkBastidores, Passos } from "../ui";
import { useDiario } from "./comum";

const PASSOS = ["Building the eligibility root", "Signing", "Confirming on Stellar"];

/* ORGANIZADOR. Monta a cédula, o eleitorado e a janela, e abre na rede. */
export default function Abrir() {
  const [aptos, setAptos] = useState("");
  const [opcoes, setOpcoes] = useState("approve, reject");
  const [anel, setAnel] = useState(true);
  const [fechadura, setFechadura] = useState(true);
  const [mesa, setMesa] = useState("");
  const [limiar, setLimiar] = useState(3);
  const [secoes, setSecoes] = useState(1);
  const [limite, setLimite] = useState(10);
  const [aberta, setAberta] = useState(true);
  const [minCaderno, setMinCaderno] = useState(10);
  const [minVoto, setMinVoto] = useState(30);
  const [falha, setFalha] = useState("");
  const [ocupado, setOcupado] = useState(false);
  const [progresso, setProgresso] = useState(0);
  const diario = useDiario("abrir");
  const ir = useNavigate();

  const linhas = (s: string) =>
    s.split(/[\n,]/).map((x) => x.trim()).filter(Boolean);

  async function enviar() {
    setFalha("");
    setOcupado(true);
    setProgresso(0);
    diario({ tipo: "ato", txt: "Started a new vote" });
    try {
      const lista = aberta ? [] : linhas(aptos);
      if (!aberta && lista.length < 2) {
        throw new Error("The electorate must include at least two people");
      }
      const n = linhas(opcoes).length;
      if (n < 2) throw new Error("The question must have at least two options");
      // O teto é do orçamento de CPU: cada opção sigilosa custa uma prova
      // disjuntiva, e o contrato recusa com `OpcoesForaDaFaixa`.
      if (n > MAX_OPCOES) {
        throw new Error(`A question supports up to ${MAX_OPCOES} options; this one has ${n}`);
      }

      if (secoes < 1 || (!aberta && secoes > lista.length)) {
        throw new Error(
          aberta ? "There must be at least one section" : `The number of sections must be between 1 and ${lista.length}`,
        );
      }

      // **O piso de anonimato, antecipado.** O contrato recusa apurar uma
      // seção com menos de τ cédulas, e essa recusa chega no fim da votação,
      // quando já não há o que fazer. Aqui ainda dá para escolher diferente.
      // Na votação aberta não dá para conferir: o eleitorado não existe na
      // hora de abrir, e é por isso que ali o aviso é outro.
      if (!aberta) {
        const pequena = secaoNasceuPequena(lista.length, secoes);
        if (pequena) throw new Error(explicar(lista.length, secoes, pequena));
      }

      const w = await carregar();
      const c = await CarteiraLocal.abrir();
      diario({ tipo: "val", txt: `organizer = ${c.endereco()}` });

      // O identificador nasce antes da raiz, e não é detalhe: a divisão em
      // seções é derivada dele, então a raiz depende dele.
      const id = crypto.getRandomValues(new Uint8Array(32));
      const proposta = Array.from(id, (b) => b.toString(16).padStart(2, "0")).join("");

      const xdrs = lista.map(enderecoXdr);
      const pesos = new Uint32Array(lista.length).fill(1);
      // 32 zeros é o sentinela de votação aberta: não há lista, e o contrato
      // pula a prova de Merkle.
      const raiz = aberta ? RAIZ_ABERTA : w.raiz_de_aptos(xdrs, pesos);
      diario({ tipo: "val", txt: aberta ? "No eligibility list: open participation" : `eligibility root = ${raiz}` });
      if (!aberta) {
        diario({ tipo: "nota", txt: "Only the 32-byte root goes on-chain, not the list." });
      }
      // **A seção não é decidida aqui, e isso é o ponto** (DEC-012). Ela sai da
      // assinatura da rodada da baliza que abre o comparecimento — uma
      // assinatura que ainda não existe neste instante. Enquanto a seção saía
      // do identificador da proposta, quem organizava moía o identificador até
      // pôr alguém numa seção cheia de atacantes, e o anonimato efetivo virava
      // 1. Agora não há o que moer, e nem esta tela sabe a divisão.
      if (!aberta && secoes > 1) {
        diario({
          tipo: "nota",
          txt: `${secoes} sections — the beacon determines the allocation later; even the organizer cannot know it now`,
        });
      }

      // A rodada da baliza que abre o comparecimento. Tem de estar no futuro —
      // uma rodada já vencida tem assinatura publicada, e aí a divisão seria
      // moível. Rodadas saem a cada 3 segundos; meio minuto à frente é folga de
      // sobra para a transação entrar.
      const rodadaAbertura =
        !aberta && secoes > 1
          ? Number(w.rodada_em(BigInt(Math.floor(Date.now() / 1000) + 30)))
          : 0;

      // Mesa vazia exige limiar zero, e limiar zero exige mesa vazia.
      const membros = linhas(mesa);
      const k = membros.length === 0 ? 0 : limiar;
      if (membros.length > 0 && (k < 1 || k > membros.length)) {
        throw new Error(`The threshold must be between 1 and ${membros.length}`);
      }
      // O contrato recusa com `MembroRepetido`: um endereço duas vezes na mesa
      // conta duas parcelas para a mesma pessoa, e o limiar deixa de significar
      // o que diz.
      if (new Set(membros).size !== membros.length) {
        throw new Error("The committee contains a duplicate address, which the contract rejects");
      }

      const agora = await ledgerAtual();
      // Ledger da testnet ≈ 6 s.
      const abre = agora + Math.round((minCaderno * 60) / 6);
      const fecha = abre + Math.round((minVoto * 60) / 6);

      // **A fechadura de tempo.** A rodada que destranca as cédulas é a que
      // vence pouco depois de a janela fechar — a janela é contada em ledgers,
      // e a rodada num instante, então a conversão é pelos mesmos minutos que a
      // pessoa digitou. O minuto de folga cobre a diferença entre o ledger
      // previsto e o ledger de verdade.
      //
      // Só faz sentido sem mesa: com mesa já há quem abra, e as duas coisas não
      // se somam. E só em anel, porque é a cédula em anel que não reparte o
      // fator com ninguém.
      const comFechadura = anel && fechadura && membros.length === 0;
      const rodada = comFechadura
        ? Number(w.rodada_em(BigInt(Math.floor(Date.now() / 1000) + (minCaderno + minVoto) * 60 + 60)))
        : 0;
      if (comFechadura) {
        diario({
          tipo: "nota",
          txt: `Time lock at beacon round ${rodada} — the key for opening ballots is published when the voting window closes`,
        });
      }
      // `abre >= fecha` é votação de duração zero, recusada com `PrazoNoPassado`.
      if (fecha <= abre) {
        throw new Error("The voting window must last at least one minute");
      }
      if (membros.length === 0) {
        diario({ tipo: "nota", txt: "No committee: this configuration cannot produce a tally" });
      }
      setProgresso(1);
      await abrir(
        c, proposta, [{ opcoes: n, confidencial: true }], raiz, membros,
        k, abre, fecha, anel, aberta ? 1 : secoes, aberta ? limite : 0,
        rodada, rodadaAbertura, diario,
      );
      setProgresso(2);

      // A lista fica aqui porque não cabe na rede — e quem vota precisa dela.
      if (!aberta) guardarLista(proposta, lista);
      ir(`/votacao/${proposta}`);
    } catch (e) {
      const msg = mensagemErro(e);
      setFalha(msg);
      diario({ tipo: "x", txt: msg });
    } finally {
      setOcupado(false);
    }
  }

  return (
    <main className="pagina pagina-estreita">
      <header>
        <span className="eyebrow">ORGANIZER </span>
        <h1>Create a vote.</h1>
      </header>

      <section>
        <h2>THE QUESTION</h2>
        <label className="campo">
          <span className="eyebrow">OPTIONS </span>
          <input value={opcoes} onChange={(e) => setOpcoes(e.target.value)} />
          <span className="campo-ajuda">Separate options with commas. This question uses confidential ballots. </span>
        </label>
      </section>

      <section>
        <h2>WHO CAN VOTE</h2>
        <label className="escolha-modo">
          <input type="radio" name="eleitorado" checked={aberta} onChange={() => setAberta(true)} />
          <span>
            <strong>Anyone </strong>
            <span className="campo-ajuda">
              No eligibility list is required. This is the open demo mode. </span>
          </span>
        </label>
        <label className="escolha-modo" style={{ marginTop: 9 }}>
          <input type="radio" name="eleitorado" checked={!aberta} onChange={() => setAberta(false)} />
          <span>
            <strong>Only listed participants </strong>
            <span className="campo-ajuda">
              Only the Merkle root goes on-chain. The list stays in this browser, and you must share it with voters. </span>
          </span>
        </label>

        {aberta ? (
          <Aviso tipo="nota">
            <strong>An open vote cannot guarantee a meaningful count. </strong> Anyone can vote repeatedly by creating new wallets, which Friendbot funds for free on testnet. Without an eligibility list, there is no list of absentees or mandatory participation. <br />
            The anonymous ballot mechanism still applies:  <strong>participation and ballot submission remain separate </strong>. This mode demonstrates ballot privacy, not one vote per person. </Aviso>
        ) : (
          <label className="campo" style={{ marginTop: 20 }}>
            <span className="eyebrow">ELIGIBLE ADDRESSES </span>
            <textarea
              value={aptos}
              onChange={(e) => setAptos(e.target.value)}
              rows={6}
              placeholder="One G… address per line"
            />
          </label>
        )}
      </section>

      <section>
        <h2>VOTING MODE</h2>
        <label className="escolha-modo">
          <input type="checkbox" checked={anel} onChange={(e) => setAnel(e.target.checked)} />
          <span>
            <strong>Separate attendance from ballots </strong>
            <span className="campo-ajuda">
              {anel
                ? "Attendance is public; ballots use a one-time key and a ring signature to hide the sender within the group"
                : "The ballot is sent from the voter's address: the ledger records who voted, while the choice is hidden by a commitment"}
            </span>
          </span>
        </label>
      </section>

      {anel && (
        <section>
          <h2>SECTIONS</h2>
          {aberta ? (
            <>
              <label className="campo campo-estreito">
                <span className="eyebrow">PEOPLE PER SECTION </span>
                <input
                  type="number"
                  value={limite}
                  min={5}
                  onChange={(e) => setLimite(Number(e.target.value))}
                />
              </label>
              <p>
                Choose a section size without guessing how many people will join. {" "}
                <strong>When a section fills up, the contract opens the next one. </strong>
</p>
              <p>
                Verifying a ring costs  <strong>10,822,850 instructions per member </strong>. With 30 people in one ring, each ballot uses 91.4% of a transaction's CPU limit and only one fits per ledger — measured: 18 of 30 in 646 s; the rest expired. In sections of ten, a ballot uses 37.2% and three fit per ledger.</p>
              {limite < 5 && (
                <Aviso tipo="erro">
                  Below five participants, the contract rejects the ballot: the ring is too small to provide the required anonymity. Use five or more. </Aviso>
              )}
            </>
          ) : (
            <>
              <label className="campo campo-estreito">
                <span className="eyebrow">NUMBER OF SECTIONS </span>
                <input
                  type="number"
                  value={secoes}
                  min={1}
                  onChange={(e) => setSecoes(Number(e.target.value))}
                />
              </label>
              <p>
                <strong>You cannot choose who shares a section </strong>. Allocation comes from the signature of a beacon round that  <em>has not happened yet </em> . Once published, anyone with the list can recompute and verify the allocation. This prevents an organizer from deliberately isolating a voter.</p>
            </>
          )}
          <p>
            The trade-off is the anonymity set: it becomes your section, not the entire electorate. With either a committee or a time lock, the contract publishes  <strong>one aggregate tally </strong>. Sections store commitment sums, not individual section totals. A unanimous overall result can still reveal everyone's choice. Ring size is also limited by the transaction CPU budget.</p>
        </section>
      )}

      <section>
        <h2>TALLYING COMMITTEE</h2>
        <label className="campo">
          <span className="eyebrow">MEMBERS </span>
          <textarea
            value={mesa}
            onChange={(e) => setMesa(e.target.value)}
            rows={3}
            placeholder="One address per line — leave empty for no committee"
          />
        </label>
        {mesa.trim() && (
          <label className="campo campo-estreito">
            <span className="eyebrow">THRESHOLD </span>
            <input
              type="number"
              value={limiar}
              min={1}
              onChange={(e) => setLimiar(Number(e.target.value))}
            />
          </label>
        )}
        <p>
          {mesa.trim()
            ? "Committee members combine their shares and publish the total. The contract rejects any total that does not match the accumulated commitments."
            : "Without a committee, this mode cannot produce a tally."}
</p>
        <p>
          <strong>A ring ballot does not share its blinding factor with anyone </strong>, including the committee. Without a time lock, adding a committee does not make ring ballots tallyable. Any total without a valid opening is rejected with  <code>AberturaNaoFecha</code>.
</p>
        {anel && !mesa.trim() ? (
          <>
            <label className="escolha-modo">
              <input
                type="checkbox"
                checked={fechadura}
                onChange={(e) => setFechadura(e.target.checked)}
              />
              <span>
                <strong>Time lock </strong>
                <span className="campo-ajuda">
                  {fechadura
                    ? "The results page tallies after the window closes and the beacon publishes the required round, without a committee"
                    : "Without a time lock, ring ballots remain hidden and cannot be tallied"}
                </span>
              </span>
            </label>
            {fechadura ? (
              <>
                <p>
                  Ballots are encrypted for a public beacon round scheduled at the end of the voting window.  <strong>The opening key is not published before that round </strong> . After publication, anyone can open and tally the ballots without waiting for voters to return.</p>
                <p>
                  The beacon is a trust assumption: collusion by a threshold of its operators could open ballot contents early, and  {" "}
                  <strong>if it stops publishing, the tally cannot proceed and there is no fallback </strong>. Independently of the beacon, the ledger clock prevents the contract from accepting a tally before the voting window closes.</p>
              </>
            ) : (
              <p>In this configuration, ballots remain hidden and no tally can be produced.</p>
            )}
          </>
        ) : (
          <p>In this configuration, ballots remain hidden and no tally can be produced.</p>
        )}
      </section>

      <section>
        <h2>TIMING</h2>
        <div className="campo-duplo">
          <label className="campo">
            <span className="eyebrow">{anel ? "CHECK-IN" : "WAIT"} (MIN)</span>
            <input
              type="number"
              value={minCaderno}
              min={1}
              onChange={(e) => setMinCaderno(Number(e.target.value))}
            />
          </label>
          <label className="campo">
            <span className="eyebrow">VOTING (MIN) </span>
            <input
              type="number"
              value={minVoto}
              min={1}
              onChange={(e) => setMinVoto(Number(e.target.value))}
            />
          </label>
        </div>
      </section>

      <section>
        <button className="primary-button" onClick={enviar} disabled={ocupado}>
          {ocupado ? "Creating…" : "Create on testnet"} <Icone nome="arrow" />
</button>
        {ocupado && <Passos passos={PASSOS} atual={progresso} />}
        {falha && <Aviso tipo="erro">{falha}</Aviso>}
        <LinkBastidores origem="abrir" />
      </section>
    </main>
  );
}
