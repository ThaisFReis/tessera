import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { CarteiraLocal } from "../carteira";
import { guardarLista } from "../lista";
import { abrir, enderecoXdr, ledgerAtual } from "../rede";
import { carregar } from "../wasm";
import { Aviso, Icone, LinkBastidores, Passos } from "../ui";
import { useDiario } from "./comum";

const PASSOS = ["Montando a raiz de aptos", "Assinando", "Confirmando na Stellar"];

/* ORGANIZADOR. Monta a cédula, o eleitorado e a janela, e abre na rede. */
export default function Abrir() {
  const [aptos, setAptos] = useState("");
  const [opcoes, setOpcoes] = useState("aprovar, rejeitar");
  const [anel, setAnel] = useState(true);
  const [mesa, setMesa] = useState("");
  const [limiar, setLimiar] = useState(3);
  const [secoes, setSecoes] = useState(1);
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
    diario({ tipo: "ato", txt: "abriu uma votação" });
    try {
      const lista = linhas(aptos);
      if (lista.length < 2) throw new Error("o eleitorado precisa de ao menos duas pessoas");
      const n = linhas(opcoes).length;
      if (n < 2) throw new Error("a pergunta precisa de ao menos duas opções");

      if (secoes < 1 || secoes > lista.length) {
        throw new Error(`as seções têm de estar entre 1 e ${lista.length}`);
      }

      const w = await carregar();
      const c = await CarteiraLocal.abrir();
      diario({ tipo: "val", txt: `organizador = ${c.endereco()}` });

      // O identificador nasce antes da raiz, e não é detalhe: a divisão em
      // seções é derivada dele, então a raiz depende dele.
      const id = crypto.getRandomValues(new Uint8Array(32));
      const proposta = Array.from(id, (b) => b.toString(16).padStart(2, "0")).join("");

      const xdrs = lista.map(enderecoXdr);
      const pesos = new Uint32Array(lista.length).fill(1);
      const raiz = w.raiz_de_aptos(proposta, xdrs, pesos, secoes);
      diario({ tipo: "val", txt: `raiz de aptos = ${raiz}` });
      diario({ tipo: "nota", txt: "32 bytes vão para o contrato. A lista não vai." });
      if (secoes > 1) {
        const d = Array.from(w.secoes_de(proposta, xdrs, secoes) as Uint32Array);
        const tam = Array.from({ length: secoes }, (_, s) => d.filter((x) => x === s).length);
        diario({ tipo: "nota", txt: `seções de ${tam.join(", ")} — sorteadas pela lista, não escolhidas` });
      }

      const agora = await ledgerAtual();
      // Ledger da testnet ≈ 6 s.
      const abre = agora + Math.round((minCaderno * 60) / 6);
      const fecha = abre + Math.round((minVoto * 60) / 6);
      const membros = linhas(mesa);
      if (!membros.length) throw new Error("o contrato exige ao menos um membro de mesa");
      if (limiar < 1 || limiar > membros.length) {
        throw new Error(`o limiar tem de estar entre 1 e ${membros.length}`);
      }
      setProgresso(1);
      await abrir(
        c, proposta, [{ opcoes: n, confidencial: true }], raiz, membros,
        limiar, abre, fecha, anel, secoes, diario,
      );
      setProgresso(2);

      // A lista fica aqui porque não cabe na rede — e quem vota precisa dela.
      guardarLista(proposta, lista);
      ir(`/votacao/${proposta}`);
    } catch (e) {
      const msg = String((e as Error).message ?? e);
      setFalha(msg);
      diario({ tipo: "x", txt: msg });
    } finally {
      setOcupado(false);
    }
  }

  return (
    <main className="pagina pagina-estreita">
      <header>
        <span className="eyebrow">ORGANIZADOR</span>
        <h1>Abrir uma votação.</h1>
      </header>

      <section>
        <h2>A PERGUNTA</h2>
        <label className="campo">
          <span className="eyebrow">OPÇÕES</span>
          <input value={opcoes} onChange={(e) => setOpcoes(e.target.value)} />
          <span className="campo-ajuda">Separadas por vírgula. A pergunta é sigilosa.</span>
        </label>
      </section>

      <section>
        <h2>O ELEITORADO</h2>
        <label className="campo">
          <span className="eyebrow">ENDEREÇOS APTOS</span>
          <textarea
            value={aptos}
            onChange={(e) => setAptos(e.target.value)}
            rows={6}
            placeholder="um endereço G… por linha"
          />
          <span className="campo-ajuda">
            A lista não vai para a rede — só a raiz de Merkle. Ela fica guardada neste navegador e
            você precisa entregá-la a quem vota.
          </span>
        </label>
      </section>

      <section>
        <h2>O MODO</h2>
        <label className="escolha-modo">
          <input type="checkbox" checked={anel} onChange={(e) => setAnel(e.target.checked)} />
          <span>
            <strong>caderno separado da urna</strong>
            <span className="campo-ajuda">
              {anel
                ? "quem comparece aparece com nome; a cédula sai de uma chave de uso único e não liga a ninguém"
                : "a cédula sai do endereço de quem vota: o ledger mostra que você votou, nunca em quê"}
            </span>
          </span>
        </label>
      </section>

      {anel && (
        <section>
          <h2>AS SEÇÕES</h2>
          <label className="campo campo-estreito">
            <span className="eyebrow">QUANTAS</span>
            <input
              type="number"
              value={secoes}
              min={1}
              onChange={(e) => setSecoes(Number(e.target.value))}
            />
          </label>
          <p>
            Verificar um anel custa <strong>10.822.850 instruções por membro</strong>. Com 30
            pessoas num anel só, cada cédula usa 91,4% do teto de CPU de uma transação e só uma
            entra por ledger — medido: 18 de 30 cédulas em 646 s, o resto expirou. Em três seções
            de dez, a mesma cédula custa 37,2% e três entram por ledger.
          </p>
          <p>
            <strong>Você não escolhe quem fica com quem.</strong> A divisão é sorteada a partir da
            lista e do identificador da votação, e qualquer pessoa com a lista recalcula e confere.
            Se o organizador escolhesse, poria um dissidente numa seção sozinho e leria o voto dele.
          </p>
          <p>
            O preço é o conjunto de anonimato: ele passa a ser a sua seção, não a votação inteira.
            O resultado continua único — o acumulador não sabe de que seção veio cada cédula.
          </p>
        </section>
      )}

      <section>
        <h2>A MESA</h2>
        <label className="campo">
          <span className="eyebrow">MEMBROS</span>
          <textarea
            value={mesa}
            onChange={(e) => setMesa(e.target.value)}
            rows={3}
            placeholder="um endereço por linha"
          />
        </label>
        <label className="campo campo-estreito">
          <span className="eyebrow">LIMIAR</span>
          <input
            type="number"
            value={limiar}
            min={1}
            onChange={(e) => setLimiar(Number(e.target.value))}
          />
        </label>
        <p>
          O contrato exige mesa: <code>limiar == 0 || limiar &gt; mesa</code> é{" "}
          <code>LimiarInvalido</code>. Mas, <strong>numa cédula em anel, o dapp não reparte o fator
          de aleatoriedade com ninguém</strong> — a mesa existe no contrato e não recebe parcela.
          Logo ninguém reconstrói a abertura, ninguém apura, e ninguém abre um voto.
        </p>
        <p>
          Assembleia aberta sem mesa nenhuma seria o desenho limpo, e exige mudar o contrato. Está
          anotado.
        </p>
      </section>

      <section>
        <h2>A JANELA</h2>
        <div className="campo-duplo">
          <label className="campo">
            <span className="eyebrow">{anel ? "COMPARECIMENTO" : "ESPERA"} (MIN)</span>
            <input
              type="number"
              value={minCaderno}
              min={1}
              onChange={(e) => setMinCaderno(Number(e.target.value))}
            />
          </label>
          <label className="campo">
            <span className="eyebrow">VOTAÇÃO (MIN)</span>
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
          {ocupado ? "Abrindo…" : "Abrir na testnet"} <Icone nome="arrow" />
        </button>
        {ocupado && <Passos passos={PASSOS} atual={progresso} />}
        {falha && <Aviso tipo="erro">{falha}</Aviso>}
        <LinkBastidores origem="abrir" />
      </section>
    </main>
  );
}
