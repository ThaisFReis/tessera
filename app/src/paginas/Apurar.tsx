import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { lerComparecimento, lerProposta, lerResultado, REDE, type PropostaRede } from "../rede";
import { Carregando, Estado, Icone } from "../ui";

/* ORGANIZADOR / QUALQUER PESSOA: a apuração.
   A mesa afirma os totais, o contrato confere contra o acumulado, e recusa se
   não fechar. Sem mesa não há quem afirme — e é isso que a página explica. */
export default function Apurar() {
  const { id = "" } = useParams();
  const [p, setP] = useState<PropostaRede | null>(null);
  const [cedulas, setCedulas] = useState(0);
  const [placar, setPlacar] = useState<number[] | null>(null);
  const [falha, setFalha] = useState("");

  useEffect(() => {
    let vivo = true;
    (async () => {
      try {
        const prop = await lerProposta(id);
        const [conf] = await lerComparecimento(id);
        const r = await lerResultado(id);
        if (!vivo) return;
        setP(prop);
        setCedulas(conf);
        setPlacar(r);
      } catch (e) {
        if (vivo) setFalha(String((e as Error).message ?? e));
      }
    })();
    return () => {
      vivo = false;
    };
  }, [id]);

  if (falha) {
    return <main className="pagina"><Estado titulo="Não foi possível ler a proposta." curto>
      <p className="error-message" role="alert">{falha}</p>
      <Link className="primary-button" to="/votacoes">Ver votações <Icone nome="arrow" /></Link>
    </Estado></main>;
  }
  if (!p) return <main className="pagina"><Carregando titulo="Lendo a proposta." /></main>;

  return (
    <main className="pagina">
      <header>
        <span className="eyebrow">APURAÇÃO · {cedulas} {cedulas === 1 ? "CÉDULA" : "CÉDULAS"}</span>
        <h1>{id.slice(0, 16)}…</h1>
      </header>

      {placar ? (
        <section>
          <h2>O PLACAR</h2>
          <div className="painel">
            <dl className="fatos">
              {placar.map((n, i) => (
                <div key={i}>
                  <dt>OPÇÃO {i + 1}</dt>
                  <dd className="valor-mono">{n}</dd>
                </div>
              ))}
            </dl>
          </div>
          <p>
            Estes números não vieram da mesa: vieram do <strong>contrato</strong>. Ele conferiu a
            abertura contra o acumulado de todas as cédulas antes de gravá-los, e teria recusado
            com <code>AberturaNaoFecha</code> qualquer total que não fechasse.
          </p>
          <p>
            A correspondência entre opção e nome está com quem organizou: a proposta na rede só
            guarda quantidades.
          </p>
        </section>
      ) : p.mesa.length === 0 ? (
        <section>
          <h2>ESTA VOTAÇÃO NÃO APURA, E ISSO É O DESENHO</h2>
          <div className="painel">
            <p>
              Ela foi aberta sem mesa nenhuma. Ninguém recebeu parcela de abertura, então ninguém
              reconstrói a soma dos fatores que escondem os votos — e sem ela nenhum total pode ser
              publicado.
            </p>
            <p>
              Não é uma promessa: é o contrato. Qualquer total afirmado sem a abertura correta cai
              em <code>AberturaNaoFecha</code>. Nem quem abriu a votação consegue.
            </p>
            <p><strong>O preço é este: o sigilo é absoluto, e o resultado é impossível.</strong></p>
          </div>
        </section>
      ) : (
        <section>
          <h2>A MESA PRECISA SE REUNIR</h2>
          <div className="painel">
            <p>
              {p.limiar} de {p.mesa.length} membros reconstroem a abertura agregada, afirmam os
              totais, e o contrato confere contra o acumulado no ledger. Se os números não
              fecharem, ele recusa — <strong>não é denúncia depois, é recusa na hora</strong>.
            </p>
            <p>
              As parcelas vivem no navegador de cada membro. Esta rota ainda não as junta; por
              enquanto a apuração é pela CLI:
            </p>
            <pre className="bloco-codigo">tessera apurar --proposta {id.slice(0, 16)}…</pre>
          </div>
        </section>
      )}

      <p>
        <a href={`${REDE.explorer}/contract/${REDE.contrato}`} target="_blank" rel="noopener">
          ver no explorer ↗
        </a>
      </p>
    </main>
  );
}
