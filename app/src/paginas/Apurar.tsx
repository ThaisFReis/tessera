import { useEffect, useRef, useState } from "react";
import { Link, useParams } from "react-router-dom";
import {
  ledgerAtual,
  lerComparecimento,
  lerProposta,
  lerResultado,
  REDE,
  type PropostaRede,
} from "../rede";
import { apurarSozinho, type FasePlacar } from "../placar";
import { useDiario } from "./comum";
import { Carregando, Estado, Icone, LinkBastidores } from "../ui";

/* ORGANIZADOR / QUALQUER PESSOA: a apuração.
   Com mesa, ela afirma os totais e o contrato confere contra o acumulado. Com
   fechadura de tempo não há quem afirme: a chave nasce sozinha no instante da
   rodada, e esta página faz a conta — sem ninguém clicar. Sem mesa e sem
   fechadura, não há apuração possível, e a página explica por quê. */
export default function Apurar() {
  const { id = "" } = useParams();
  const diario = useDiario("apurar", id);
  const [p, setP] = useState<PropostaRede | null>(null);
  const [cedulas, setCedulas] = useState(0);
  const [daMesa, setDaMesa] = useState<number[] | null>(null);
  const [fase, setFase] = useState<FasePlacar | null>(null);
  const [falha, setFalha] = useState("");
  // Uma aba só apura uma vez. Sem isto, o React em modo estrito dispararia
  // duas transações para o mesmo placar — e a segunda levaria `NaoMelhora`.
  const apurou = useRef(false);

  useEffect(() => {
    let vivo = true;
    (async () => {
      try {
        const [prop, ledger] = await Promise.all([lerProposta(id), ledgerAtual()]);
        if (!vivo || !prop) return setFalha("esta proposta não existe no contrato");
        const [conf] = await lerComparecimento(id);
        if (!vivo) return;
        setP(prop);
        setCedulas(conf);

        if (prop.rodada === 0n) {
          const r = await lerResultado(id);
          if (vivo) setDaMesa(r);
          return;
        }
        if (apurou.current) return;
        apurou.current = true;
        await apurarSozinho(id, prop, ledger, (f) => vivo && setFase(f), diario);
      } catch (e) {
        if (vivo) setFalha(String((e as Error).message ?? e));
      }
    })();
    return () => {
      vivo = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
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

      {p.rodada !== 0n ? <ComFechadura id={id} p={p} fase={fase} /> : daMesa ? (
        <section>
          <h2>O PLACAR</h2>
          <Totais totais={daMesa} />
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
              Ela foi aberta sem mesa e <strong>sem fechadura de tempo</strong>. Ninguém recebeu
              parcela de abertura, então ninguém reconstrói a soma dos fatores que escondem os
              votos — e sem ela nenhum total pode ser publicado.
            </p>
            <p>
              Não é uma promessa: é o contrato. Qualquer total afirmado sem a abertura correta cai
              em <code>AberturaNaoFecha</code>. Nem quem abriu a votação consegue.
            </p>
            <p><strong>O preço é este: o sigilo é absoluto, e o resultado é impossível.</strong></p>
            <p>
              Uma votação aberta com fechadura não tem esse preço: a chave que abre as cédulas
              nasce sozinha quando a janela fecha, e aí qualquer pessoa apura.
            </p>
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
        {p.rodada !== 0n && <> · <LinkBastidores /></>}
      </p>
    </main>
  );
}

function Totais({ totais }: { totais: number[] }) {
  return (
    <div className="painel">
      <dl className="fatos">
        {totais.map((n, i) => (
          <div key={i}>
            <dt>OPÇÃO {i + 1}</dt>
            <dd className="valor-mono">{n}</dd>
          </div>
        ))}
      </dl>
    </div>
  );
}

/** A apuração pela fechadura de tempo, fase por fase. */
function ComFechadura({ id, p, fase }: { id: string; p: PropostaRede; fase: FasePlacar | null }) {
  if (!fase) return <Carregando titulo="Vendo se o relógio já abriu." />;

  switch (fase.tipo) {
    case "janela-aberta":
      return (
        <section>
          <h2>A VOTAÇÃO AINDA ESTÁ ABERTA</h2>
          <div className="painel">
            <p>
              Não há placar — e não há <em>nem parcial</em>. Faltam{" "}
              <span className="valor-mono">{fase.faltam}</span> ledgers para a urna fechar.
            </p>
            <p>
              Isto não é a tela escondendo um número que ela tem. A chave que abre as cédulas é a
              assinatura da rodada <span className="valor-mono">{String(p.rodada)}</span> da
              baliza, e a baliza só a publica quando aquele instante chega — nem quem organizou,
              nem quem votou, nem esta página a tem.
            </p>
            <p>
              Produzi-la mais cedo não é impossível: exigiria conluio de um limiar dos operadores
              da baliza — gente que ninguém desta votação escolheu e que não tem interesse nela,
              mas gente. O que <em>não</em> depende de suposição nenhuma é o placar: o contrato
              recusa qualquer apuração antes de <code>fecha_em</code>, e isso é o relógio do
              ledger.
            </p>
          </div>
          <Link className="secondary-button" to={`/votacao/${id}`}>Ver a votação <Icone nome="arrow" /></Link>
        </section>
      );

    case "esperando-relogio":
      return (
        <section>
          <h2>A URNA FECHOU. O RELÓGIO AINDA NÃO ABRIU.</h2>
          <div className="painel">
            <p>
              A janela terminou, e a rodada <span className="valor-mono">{String(p.rodada)}</span>{" "}
              da baliza vence em <span className="valor-mono">{fase.faltam}</span> segundos. Até
              lá, o contrato recusa qualquer apuração com{" "}
              <code>RelogioAindaNaoAbriu</code> — inclusive a desta aba.
            </p>
            <p>Recarregue quando o tempo passar: o placar aparece sem você fazer mais nada.</p>
          </div>
        </section>
      );

    case "lendo":
      return <Carregando titulo="Lendo as cédulas dos eventos do contrato." />;
    case "decifrando":
      return <Carregando titulo={`Decifrando ${fase.cedulas} cédulas neste navegador.`} />;
    case "publicando":
      return <Carregando titulo="Gravando o placar no ledger." />;

    case "vazia":
      return (
        <section>
          <h2>A URNA ESTÁ VAZIA</h2>
          <div className="painel">
            <p>A janela fechou, o relógio abriu, e nenhuma cédula foi depositada. Não há o que apurar.</p>
          </div>
        </section>
      );

    case "multissecao":
      return (
        <section>
          <h2>ESTA VOTAÇÃO TEM {fase.secoes} SEÇÕES</h2>
          <div className="painel">
            <p>
              O placar de cada seção é apurável por qualquer pessoa que tenha a lista ordenada das
              cédulas daquela seção — mas <strong>esta página não consegue montá-la</strong>: o
              evento que o contrato publica diz a proposta e não a seção, e a assinatura em anel
              não revela qual anel assinou, que é o ponto dela.
            </p>
            <p>
              {fase.publicadas} de {fase.secoes} seções já têm placar no ledger. Quando as{" "}
              {fase.secoes} tiverem, esta página soma e mostra.
            </p>
            <p>
              Não é limitação do protocolo, é um campo faltando num tópico de evento. Está
              registrado como §11-N no caderno do projeto, junto do que custa consertá-lo.
            </p>
          </div>
        </section>
      );

    case "falha":
      return (
        <section>
          <h2>A APURAÇÃO NÃO FOI ATÉ O FIM</h2>
          <div className="painel">
            <p className="error-message" role="alert">{fase.porque}</p>
            <p>
              O placar não se perdeu: os criptogramas estão no ledger e a chave da rodada é
              pública. <strong>Qualquer pessoa</strong> refaz esta conta, desta aba ou de outra —
              é isso que torna a apuração não travável.
            </p>
          </div>
        </section>
      );

    case "pronto":
      return (
        <section>
          <h2>O PLACAR</h2>
          <Totais totais={fase.totais} />
          <p>
            {fase.cedulas === null ? (
              <>
                <strong>{fase.abriram}</strong>{" "}
                {fase.abriram === 1 ? "cédula abriu" : "cédulas abriram"}, segundo o que está
                gravado no ledger.
              </>
            ) : (
              <>
                <strong>{fase.abriram} de {fase.cedulas}</strong>{" "}
                {fase.cedulas === 1 ? "cédula abriu" : "cédulas abriram"}
                {fase.abriram < fase.cedulas && (
                  <> — {fase.cedulas - fase.abriram} criptograma
                    {fase.cedulas - fase.abriram === 1 ? " não abriu" : "s não abriram"}, e cada um
                    custou só o próprio voto</>
                )}
                .
              </>
            )}
          </p>
          <p>
            Ninguém afirmou estes números: eles saíram da decifragem das cédulas, neste navegador,
            com a assinatura da rodada <span className="valor-mono">{String(p.rodada)}</span> da
            baliza — conferida num pareamento antes de qualquer conta.{" "}
            {fase.publicado
              ? "E estão no ledger, onde o contrato conferiu a soma dos compromissos antes de gravá-los."
              : "Ainda não estão no ledger, e a conta não depende disso: qualquer pessoa a refaz."}
          </p>
          {fase.tx && (
            <p>
              <a href={`${REDE.explorer}/tx/${fase.tx}`} target="_blank" rel="noopener">
                ver a transação do placar ↗
              </a>
            </p>
          )}
          <p>
            A correspondência entre opção e nome está com quem organizou: a proposta na rede só
            guarda quantidades.
          </p>
        </section>
      );

    case "sem-fechadura":
      return null;
  }
}
