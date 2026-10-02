import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { lerComparecimento, lerProposta, REDE, type PropostaRede } from "../rede";
import { Erro } from "./comum";

/* ORGANIZADOR / QUALQUER PESSOA: a apuração.
   A mesa afirma os totais, o contrato confere contra o acumulado, e recusa se
   não fechar. Sem mesa não há quem afirme — e é isso que a página explica. */
export default function Apurar() {
  const { id = "" } = useParams();
  const [p, setP] = useState<PropostaRede | null>(null);
  const [cedulas, setCedulas] = useState(0);
  const [falha, setFalha] = useState("");

  useEffect(() => {
    let vivo = true;
    (async () => {
      try {
        const prop = await lerProposta(id);
        const [conf] = await lerComparecimento(id);
        if (!vivo) return;
        setP(prop);
        setCedulas(conf);
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

  return (
    <main>
      <h1>Apurar</h1>
      <p>votação {id.slice(0, 16)}… · {cedulas} cédulas</p>

      {p.mesa.length === 0 ? (
        <>
          <h2>esta votação não apura, e isso é o desenho</h2>
          <p>
            Ela foi aberta sem mesa. Ninguém recebeu parcela de abertura, então ninguém reconstrói
            a soma dos fatores que escondem os votos — e sem ela nenhum total pode ser publicado.
          </p>
          <p>
            Não é uma promessa: é o contrato. Qualquer total afirmado sem a abertura correta cai em
            <code> AberturaNaoFecha</code>. Nem quem abriu a votação consegue.
          </p>
          <p>O preço é este: o sigilo é absoluto, e o resultado é impossível.</p>
        </>
      ) : (
        <>
          <h2>a mesa precisa se reunir</h2>
          <p>
            {p.limiar} de {p.mesa.length} membros reconstroem a abertura agregada, afirmam os
            totais, e o contrato confere contra o acumulado no ledger. Se os números não fecharem,
            ele recusa — não é denúncia depois, é recusa na hora.
          </p>
          <p>
            As parcelas vivem no navegador de cada membro. Esta rota ainda não as junta; por
            enquanto a apuração é pela CLI:
          </p>
          <pre>tessera apurar --proposta {id.slice(0, 16)}…</pre>
        </>
      )}

      <p>
        <a href={`${REDE.explorer}/contract/${REDE.contrato}`} target="_blank" rel="noopener">
          ver no explorer ↗
        </a>
      </p>
    </main>
  );
}
