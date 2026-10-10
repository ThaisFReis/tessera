import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import {
  fase as calcularFase,
  lerAnel,
  lerComparecimento,
  lerProposta,
  ledgerAtual,
  REDE,
  lerResultado,
  lerResultadoSecao,
  type PropostaRede,
} from "../rede";
import { Carregando, Estado, Icone } from "../ui";

const FASES: Record<string, string> = {
  agendada: "ainda não começou",
  comparecimento: "comparecimento aberto",
  votacao: "votação aberta",
  encerrada: "encerrada",
};

function Fato({ rotulo, children }: { rotulo: string; children: React.ReactNode }) {
  return <div><dt>{rotulo.toUpperCase()}</dt><dd>{children}</dd></div>;
}

export default function Votacao() {
  const { id = "" } = useParams();
  const [p, setP] = useState<PropostaRede | null>(null);
  const [anel, setAnel] = useState<number[]>([]);
  const [cedulas, setCedulas] = useState(0);
  const [placar, setPlacar] = useState<number[] | null>(null);
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
        if (prop?.anel) {
          // Uma leitura por seção: o anel de cada uma é uma entrada própria, e
          // é isso que faz o custo da cédula parar de crescer com a votação.
          const por = await Promise.all(
            Array.from({ length: prop.secoes }, (_, s) => lerAnel(id, s)),
          );
          if (vivo) setAnel(por.map((a) => a.length));
        }
        const [conf] = await lerComparecimento(id);
        // Com fechadura de tempo o placar mora por seção, e não há total
        // agregado no contrato: `resultado()` é o caminho da mesa.
        const r = prop?.rodada
          ? (await lerResultadoSecao(id, 0))?.totais ?? null
          : await lerResultado(id);
        if (vivo) {
          setCedulas(conf);
          setPlacar(r);
        }
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

  const f = calcularFase(p, ledger);

  return (
    <main className="pagina">
      <header>
        <span className="eyebrow">VOTAÇÃO · {FASES[f].toUpperCase()}</span>
        <h1>{id.slice(0, 16)}…</h1>
        <p className="pagina-sub">
          {p.anel
            ? "Caderno separado da urna — quem faltou é público, de quem é cada cédula não é."
            : "Voto identificado — o ledger mostra que o seu endereço votou, nunca em quê."}
        </p>
      </header>

      {f !== "encerrada" && (
        <section>
          <h2>O QUE DÁ PARA FAZER AGORA</h2>
          {p.anel && f === "comparecimento" && (
            <>
              <Link className="primary-button" to={`/comparecer/${id}`}>
                Confirmar presença <Icone nome="arrow" />
              </Link>
              <p>Identificado, e é o que permite votar depois.</p>
            </>
          )}
          {f === "votacao" && (
            <Link className="primary-button" to={`/votar/${id}`}>
              Votar <Icone nome="arrow" />
            </Link>
          )}
          {f === "agendada" && <p>A janela ainda não começou. Volte quando o ledger passar de {p.abre_em}.</p>}
        </section>
      )}
      {f === "encerrada" && placar && (
        <section>
          <h2>O PLACAR</h2>
          <dl className="fatos">
            {placar.map((n, i) => (
              <div key={i}><dt>OPÇÃO {i + 1}</dt><dd className="valor-mono">{n}</dd></div>
            ))}
          </dl>
          <p>
            O contrato conferiu a abertura contra o acumulado antes de gravar. Ver{" "}
            <Link to={`/apurar/${id}`}>a apuração</Link>.
          </p>
        </section>
      )}
      {f === "encerrada" && !placar && (
        <section>
          <h2>A URNA FECHOU</h2>
          {p.rodada !== 0n && (
            <p>
              O placar não precisa de ninguém: a chave que abre as cédulas é a assinatura da
              rodada <span className="valor-mono">{String(p.rodada)}</span> da baliza, e ela nasce
              sozinha. A apuração acontece ao abrir a página.
            </p>
          )}
          <Link className="secondary-button" to={`/apurar/${id}`}>Ver a apuração <Icone nome="arrow" /></Link>
        </section>
      )}

      <section>
        <h2>O QUE ESTÁ NO CONTRATO</h2>
        <dl className="fatos">
          <Fato rotulo="janela">
            <span className="valor-mono">abre {p.abre_em} · fecha {p.fecha_em} · agora {ledger}</span>
          </Fato>
          <Fato rotulo="perguntas">
            {p.perguntas.map((q, i) => (
              <span key={i}>
                {i + 1}. {q.opcoes} opções {q.confidencial ? "em sigilo" : "em aberto"};{" "}
              </span>
            ))}
          </Fato>
          <Fato rotulo="mesa">
            {p.mesa.length === 0
              ? "nenhuma — ninguém pode abrir um voto, e por isso ninguém pode apurar"
              : `${p.limiar} de ${p.mesa.length} para apurar`}
          </Fato>
          <Fato rotulo="fechadura de tempo">
            {p.rodada === 0n
              ? "nenhuma — o placar depende da mesa, ou não existe"
              : `rodada ${p.rodada} da baliza drand — antes dela a chave que abre as cédulas não ` +
                "existe, e depois dela qualquer pessoa apura"}
          </Fato>
          <Fato rotulo="quem pode abrir a sua cédula">
            {p.rodada !== 0n
              ? "depois da rodada, qualquer pessoa — e é assim que o placar existe sem mesa. " +
                "O que protege você não é o sigilo do conteúdo da cédula: é o anel, que não diz " +
                "de quem ela é. Antes da rodada, ninguém."
              : p.anel
              ? "ninguém — a cédula em anel não reparte o fator com a mesa, então não existe parcela a reunir"
              : p.mesa.length === 1
                ? `uma pessoa: ${p.mesa[0]}`
                : `${p.limiar} de ${p.mesa.length}, em conluio`}
          </Fato>
          {p.anel && (
            <Fato rotulo={p.secoes > 1 ? "seções" : "anel"}>
              {p.secoes > 1
                ? `${anel.reduce((a, b) => a + b, 0)} em ${p.secoes} seções: ${anel.join(", ")} — ` +
                  "o anel de cada cédula é o da seção de quem assina, e é por isso que o custo não cresce com a votação"
                : `${anel[0] ?? 0} ${anel[0] === 1 ? "pessoa compareceu" : "pessoas compareceram"}` +
                  (anel[0] === 1 ? " — um anel de um não esconde ninguém" : "")}
            </Fato>
          )}
          <Fato rotulo="cédulas na urna">{cedulas}</Fato>
        </dl>
      </section>

      <p>
        <a href={`${REDE.explorer}/contract/${REDE.contrato}`} target="_blank" rel="noopener">
          ver o contrato no explorer ↗
        </a>
      </p>
    </main>
  );
}
