import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { CarteiraEfemera } from "../carteira";
import { lerChaveDeAnel } from "../lista";
import {
  lerAnel,
  lerGeradorH,
  lerHp,
  lerProposta,
  votarAnonimo,
  type PropostaRede,
} from "../rede";
import { carregar } from "../wasm";
import { Diario, Erro, useDiario } from "./comum";

/* VOTANTE, segundo ato: a urna.
   A cédula sai de uma chave de uso único. Se ela saísse da carteira de
   identidade, o vínculo que o anel existe para quebrar voltaria inteiro. */
export default function Votar() {
  const { id = "" } = useParams();
  const [p, setP] = useState<PropostaRede | null>(null);
  const [anel, setAnel] = useState<string[]>([]);
  const [escolha, setEscolha] = useState(0);
  const [falha, setFalha] = useState("");
  const [ocupado, setOcupado] = useState(false);
  const { passos, diario } = useDiario();
  const ir = useNavigate();
  const guardada = lerChaveDeAnel(id);

  useEffect(() => {
    let vivo = true;
    (async () => {
      try {
        const prop = await lerProposta(id);
        if (!vivo) return;
        setP(prop);
        if (prop?.anel) setAnel(await lerAnel(id));
      } catch (e) {
        if (vivo) setFalha(String((e as Error).message ?? e));
      }
    })();
    return () => {
      vivo = false;
    };
  }, [id]);

  async function enviar() {
    setFalha("");
    setOcupado(true);
    try {
      if (!p) throw new Error("proposta não carregada");
      if (!p.anel) throw new Error("esta rota é do voto em anel; a identificada ainda é da CLI");
      if (!guardada) {
        throw new Error(
          "não achei a sua chave de anel neste navegador. Ela nasce no comparecimento e nunca sai daqui — sem ela não há como provar que você é um dos que compareceram.",
        );
      }
      const w = await carregar();
      const [hp, h] = await Promise.all([lerHp(id), lerGeradorH()]);

      // Qual ramo do anel é o meu. A pública foi guardada no comparecimento
      // justamente para responder isto sem adivinhação.
      const i = anel.indexOf(guardada.publica);
      if (i < 0) {
        throw new Error(
          "a sua chave não está no anel desta votação. Ou você compareceu em outra, ou o comparecimento não chegou a entrar.",
        );
      }

      const c = w.cedula_anonima(
        id, hp, h, anel, i, guardada.secreta,
        [{ opcoes: p.perguntas[0].opcoes, confidencial: true }], new Uint32Array([escolha]),
      ) as {
        imagem: string; c0: string; z: string[];
        cedula: { compromissos: string[]; provas: never[]; provas_soma: never[]; escolhas: number[] };
      };
      diario({ tipo: "val", txt: `imagem de chave = ${c.imagem.slice(0, 24)}…` });
      diario({ tipo: "nota", txt: "o r que esconde o seu voto nasceu e morreu nesta aba" });

      const efemera = await CarteiraEfemera.nascer();
      diario({ tipo: "val", txt: `cédula assinada por ${efemera.endereco()} — uso único` });

      await votarAnonimo(
        efemera, id, anel, c.imagem, c.c0, c.z,
        c.cedula.compromissos, c.cedula.provas, c.cedula.provas_soma, c.cedula.escolhas,
        diario,
      );
      ir(`/votacao/${id}`);
    } catch (e) {
      setFalha(String((e as Error).message ?? e));
    } finally {
      setOcupado(false);
    }
  }

  if (!p) return <p>lendo a proposta…</p>;

  return (
    <main>
      <h1>Votar</h1>
      <p>Votante · votação {id.slice(0, 16)}…</p>

      <h2>o que o ledger vai saber sobre você</h2>
      <p>
        {p.anel
          ? `nada: esta cédula sai de uma chave de uso único. O anel de ${anel.length} prova que quem a mandou é um dos que compareceram, e nada diz qual.`
          : "o seu endereço votou, nunca em quê."}
      </p>
      <p>
        quem pode abrir esta cédula:{" "}
        {p.mesa.length === 0
          ? "ninguém"
          : p.mesa.length === 1
            ? "uma pessoa"
            : `${p.limiar} de ${p.mesa.length}, em conluio`}
      </p>
      {anel.length === 1 && <p>⚠ o anel tem uma pessoa só. Um anel de um não esconde ninguém.</p>}

      <h2>a cédula</h2>
      {Array.from({ length: p.perguntas[0]?.opcoes ?? 0 }, (_, j) => (
        <label key={j}>
          <input
            type="radio"
            name="opcao"
            checked={escolha === j}
            onChange={() => setEscolha(j)}
          />{" "}
          opção {j}
        </label>
      ))}

      <p>
        <button onClick={enviar} disabled={ocupado}>
          {ocupado ? "votando…" : "votar"}
        </button>
      </p>
      <Erro msg={falha} />
      <Diario passos={passos} />
    </main>
  );
}
