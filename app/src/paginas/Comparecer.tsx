import { useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { CarteiraLocal } from "../carteira";
import { guardarChaveDeAnel, guardarLista, lerLista } from "../lista";
import { comparecer, enderecoXdr } from "../rede";
import { carregar } from "../wasm";
import { Diario, Erro, useDiario } from "./comum";

/* VOTANTE, primeiro ato: o caderno.
   É o único momento em que o nome da pessoa aparece — e é ele que permite voto
   obrigatório, porque `aptos − compareceram` é a lista de quem faltou. */
export default function Comparecer() {
  const { id = "" } = useParams();
  const [lista, setLista] = useState<string[]>(() => lerLista(id) ?? []);
  const [colada, setColada] = useState("");
  const [falha, setFalha] = useState("");
  const [ocupado, setOcupado] = useState(false);
  const { passos, diario } = useDiario();
  const ir = useNavigate();

  function aceitarColada() {
    const l = colada.split(/[\n,]/).map((x) => x.trim()).filter(Boolean);
    guardarLista(id, l);
    setLista(l);
  }

  async function enviar() {
    setFalha("");
    setOcupado(true);
    try {
      const w = await carregar();
      const c = await CarteiraLocal.abrir();
      const meu = c.endereco();
      const indice = lista.indexOf(meu);
      if (indice < 0) {
        throw new Error(
          `${meu} não está na lista de aptos desta votação. O contrato recusaria com NaoEstaNaListaDeAptos.`,
        );
      }

      const xdrs = lista.map(enderecoXdr);
      const caminho = w.caminho_de(xdrs, new Uint32Array(lista.length).fill(1), indice) as {
        irmaos: string[];
        indice: number;
      };
      diario({ tipo: "val", txt: `caminho de Merkle com ${caminho.irmaos.length} irmãos` });

      const chave = w.nova_chave_de_anel() as { secreta: string; publica: string };
      diario({ tipo: "nota", txt: "a chave secreta do anel fica nesta aba, e só aqui" });
      diario({ tipo: "val", txt: `chave pública = ${chave.publica.slice(0, 24)}…` });

      await comparecer(c, id, chave.publica, caminho.irmaos, caminho.indice, diario);
      // Sem ela não há voto depois. Perder esta aba é perder o voto.
      guardarChaveDeAnel(id, chave);
      ir(`/votacao/${id}`);
    } catch (e) {
      setFalha(String((e as Error).message ?? e));
    } finally {
      setOcupado(false);
    }
  }

  return (
    <main>
      <h1>Comparecer</h1>
      <p>Votante · votação {id.slice(0, 16)}…</p>
      <p>
        Este é o caderno. Ele é <strong>identificado e público</strong>: o ledger vai mostrar que
        você compareceu, e é disso que sai a lista de quem faltou. Ele não mostra em que você
        votou — a cédula vem depois, de outra chave.
      </p>

      {lista.length === 0 ? (
        <>
          <h2>a lista de aptos</h2>
          <p>
            Ela não está na rede, de propósito: o contrato guarda 32 bytes de raiz e nada mais.
            Cole aqui a lista que o organizador entregou.
          </p>
          <textarea
            value={colada}
            onChange={(e) => setColada(e.target.value)}
            rows={6}
            cols={60}
            placeholder="um endereço G… por linha"
          />
          <p>
            <button onClick={aceitarColada}>usar esta lista</button>
          </p>
        </>
      ) : (
        <>
          <p>{lista.length} aptos nesta lista.</p>
          <p>
            <button onClick={enviar} disabled={ocupado}>
              {ocupado ? "comparecendo…" : "comparecer"}
            </button>
          </p>
        </>
      )}
      <Erro msg={falha} />
      <Diario passos={passos} />
    </main>
  );
}
