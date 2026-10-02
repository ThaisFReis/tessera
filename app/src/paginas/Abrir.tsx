import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { CarteiraLocal } from "../carteira";
import { guardarLista } from "../lista";
import { abrir, enderecoXdr, ledgerAtual } from "../rede";
import { carregar } from "../wasm";
import { Diario, Erro, useDiario } from "./comum";

/* ORGANIZADOR. Monta a cédula, o eleitorado e a janela, e abre na rede. */
export default function Abrir() {
  const [aptos, setAptos] = useState("");
  const [opcoes, setOpcoes] = useState("aprovar, rejeitar");
  const [anel, setAnel] = useState(true);
  const [mesa, setMesa] = useState("");
  const [limiar, setLimiar] = useState(3);
  const [minCaderno, setMinCaderno] = useState(10);
  const [minVoto, setMinVoto] = useState(30);
  const [falha, setFalha] = useState("");
  const [ocupado, setOcupado] = useState(false);
  const { passos, diario } = useDiario();
  const ir = useNavigate();

  const linhas = (s: string) =>
    s.split(/[\n,]/).map((x) => x.trim()).filter(Boolean);

  async function enviar() {
    setFalha("");
    setOcupado(true);
    try {
      const lista = linhas(aptos);
      if (lista.length < 2) throw new Error("o eleitorado precisa de ao menos duas pessoas");
      const n = linhas(opcoes).length;
      if (n < 2) throw new Error("a pergunta precisa de ao menos duas opções");

      const w = await carregar();
      const c = await CarteiraLocal.abrir();
      diario({ tipo: "val", txt: `organizador = ${c.endereco()}` });

      const xdrs = lista.map(enderecoXdr);
      const raiz = w.raiz_de_aptos(xdrs, new Uint32Array(lista.length).fill(1));
      diario({ tipo: "val", txt: `raiz de aptos = ${raiz}` });
      diario({ tipo: "nota", txt: "32 bytes vão para o contrato. A lista não vai." });

      const agora = await ledgerAtual();
      // Ledger da testnet ≈ 6 s.
      const abre = agora + Math.round((minCaderno * 60) / 6);
      const fecha = abre + Math.round((minVoto * 60) / 6);
      const id = crypto.getRandomValues(new Uint8Array(32));
      const proposta = Array.from(id, (b) => b.toString(16).padStart(2, "0")).join("");

      const membros = linhas(mesa);
      if (!membros.length) throw new Error("o contrato exige ao menos um membro de mesa");
      if (limiar < 1 || limiar > membros.length) {
        throw new Error(`o limiar tem de estar entre 1 e ${membros.length}`);
      }
      await abrir(
        c, proposta, [{ opcoes: n, confidencial: true }], raiz, membros,
        limiar, abre, fecha, anel, diario,
      );

      // A lista fica aqui porque não cabe na rede — e quem vota precisa dela.
      guardarLista(proposta, lista);
      ir(`/votacao/${proposta}`);
    } catch (e) {
      setFalha(String((e as Error).message ?? e));
    } finally {
      setOcupado(false);
    }
  }

  return (
    <main>
      <h1>Abrir uma votação</h1>
      <p>Organizador</p>

      <h2>a pergunta</h2>
      <input value={opcoes} onChange={(e) => setOpcoes(e.target.value)} size={50} />
      <p>opções separadas por vírgula · a pergunta é sigilosa</p>

      <h2>o eleitorado</h2>
      <textarea
        value={aptos}
        onChange={(e) => setAptos(e.target.value)}
        rows={6}
        cols={60}
        placeholder="um endereço G… por linha"
      />
      <p>
        a lista não vai para a rede — só a raiz de Merkle. Ela fica guardada neste navegador e
        você precisa entregá-la a quem vota.
      </p>

      <h2>o modo</h2>
      <label>
        <input type="checkbox" checked={anel} onChange={(e) => setAnel(e.target.checked)} />{" "}
        caderno separado da urna
      </label>
      <p>
        {anel
          ? "quem comparece aparece com nome; a cédula sai de uma chave de uso único e não liga a ninguém"
          : "a cédula sai do endereço de quem vota: o ledger mostra que você votou, nunca em quê"}
      </p>

      <h2>a mesa</h2>
      <textarea
        value={mesa}
        onChange={(e) => setMesa(e.target.value)}
        rows={3}
        cols={60}
        placeholder="um endereço por linha"
      />
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
      <label>
        limiar{" "}
        <input
          type="number"
          value={limiar}
          min={1}
          onChange={(e) => setLimiar(Number(e.target.value))}
        />
      </label>

      <h2>a janela</h2>
      <label>
        {anel ? "comparecimento" : "espera"} (min){" "}
        <input
          type="number"
          value={minCaderno}
          min={1}
          onChange={(e) => setMinCaderno(Number(e.target.value))}
        />
      </label>{" "}
      <label>
        votação (min){" "}
        <input
          type="number"
          value={minVoto}
          min={1}
          onChange={(e) => setMinVoto(Number(e.target.value))}
        />
      </label>

      <p>
        <button onClick={enviar} disabled={ocupado}>
          {ocupado ? "abrindo…" : "abrir na testnet"}
        </button>
      </p>
      <Erro msg={falha} />
      <Diario passos={passos} />
    </main>
  );
}
