import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import {
  fase as calcularFase,
  lerAnel,
  lerComparecimento,
  lerProposta,
  ledgerAtual,
  REDE,
  type PropostaRede,
} from "../rede";
import { Erro } from "./comum";

export default function Votacao() {
  const { id = "" } = useParams();
  const [p, setP] = useState<PropostaRede | null>(null);
  const [anel, setAnel] = useState<string[]>([]);
  const [cedulas, setCedulas] = useState(0);
  const [ledger, setLedger] = useState(0);
  const [falha, setFalha] = useState("");

  useEffect(() => {
    let vivo = true;
    (async () => {
      try {
        const [prop, l] = await Promise.all([lerProposta(id), ledgerAtual()]);
        if (!vivo) return;
        setP(prop);
        setLedger(l);
        if (prop?.anel) setAnel(await lerAnel(id));
        const [conf] = await lerComparecimento(id);
        if (vivo) setCedulas(conf);
      } catch (e) {
        if (vivo) setFalha(String((e as Error).message ?? e));
      }
    })();
    return () => {
      vivo = false;
    };
  }, [id]);

  if (falha) return <Erro msg={falha} />;
  if (!p) return <p>lendo a proposta…</p>;

  const f = calcularFase(p, ledger);

  return (
    <main>
      <h1>Votação {id.slice(0, 16)}…</h1>
      <dl>
        <dt>modo</dt>
        <dd>
          {p.anel
            ? "caderno separado da urna — quem faltou é público, de quem é cada cédula não é"
            : "voto identificado — o ledger mostra que o seu endereço votou, nunca em quê"}
        </dd>
        <dt>fase</dt>
        <dd>{f}</dd>
        <dt>janela</dt>
        <dd>
          abre {p.abre_em} · fecha {p.fecha_em} · agora {ledger}
        </dd>
        <dt>perguntas</dt>
        <dd>
          {p.perguntas.map((q, i) => (
            <span key={i}>
              {i + 1}. {q.opcoes} opções {q.confidencial ? "em sigilo" : "em aberto"};{" "}
            </span>
          ))}
        </dd>
        <dt>mesa</dt>
        <dd>
          {p.mesa.length === 0
            ? "nenhuma — ninguém pode abrir um voto, e por isso ninguém pode apurar"
            : `${p.limiar} de ${p.mesa.length} para apurar`}
        </dd>
        <dt>quem pode abrir a sua cédula</dt>
        <dd>
          {p.mesa.length === 0
            ? "ninguém"
            : p.mesa.length === 1
              ? `uma pessoa: ${p.mesa[0]}`
              : `${p.limiar} de ${p.mesa.length}, em conluio`}
        </dd>
        {p.anel && (
          <>
            <dt>anel</dt>
            <dd>
              {anel.length} {anel.length === 1 ? "pessoa compareceu" : "pessoas compareceram"}
              {anel.length === 1 && " — um anel de um não esconde ninguém"}
            </dd>
          </>
        )}
        <dt>cédulas na urna</dt>
        <dd>{cedulas}</dd>
      </dl>

      <h2>o que dá para fazer agora</h2>
      <ul>
        {p.anel && f === "comparecimento" && (
          <li>
            <Link to={`/comparecer/${id}`}>comparecer</Link> — identificado, e é o que permite
            votar depois
          </li>
        )}
        {f === "votacao" && (
          <li>
            <Link to={`/votar/${id}`}>votar</Link>
          </li>
        )}
        {f === "encerrada" && (
          <li>
            <Link to={`/apurar/${id}`}>apurar</Link>
          </li>
        )}
      </ul>

      <p>
        <a href={`${REDE.explorer}/contract/${REDE.contrato}`} target="_blank" rel="noopener">
          ver o contrato no explorer ↗
        </a>
      </p>
    </main>
  );
}
