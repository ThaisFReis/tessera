import { useEffect, useState } from "react";
import { Route, Routes } from "react-router-dom";
import { REDE, assembleiasAbertas, ledgerAtual } from "./rede";

type Aberta = Awaited<ReturnType<typeof assembleiasAbertas>>[number];

/** Um ledger da testnet leva ~6 s. É o relógio desta aplicação. */
const emMinutos = (ledgers: number) => Math.max(0, Math.round((ledgers * 6) / 60));

function Curto({ v, n = 8 }: { v: string; n?: number }) {
  return (
    <span title={v}>
      {v.slice(0, n)}…{v.slice(-4)}
    </span>
  );
}

function Assembleia({ a, ledger }: { a: Aberta; ledger: number }) {
  const votando = ledger >= a.abre_em;
  const resta = votando ? a.fecha_em - ledger : a.abre_em - ledger;
  return (
    <li className="border-t border-hairline py-5 flex flex-wrap gap-x-10 gap-y-2 items-baseline">
      <span className="text-texto">
        <Curto v={a.proposta} n={12} />
      </span>
      <span className={a.anel ? "text-acento" : "text-secundario"}>
        {a.anel ? "caderno separado da urna" : "voto identificado"}
      </span>
      <span className="text-secundario">
        {a.perguntas} pergunta{a.perguntas === 1 ? "" : "s"} · {a.opcoes} opções sigilosas
      </span>
      <span className="text-apagado">
        {votando
          ? `votação encerra em ~${emMinutos(resta)}min`
          : `comparecimento aberto por ~${emMinutos(resta)}min`}
      </span>
    </li>
  );
}

function Inicio() {
  const [abertas, setAbertas] = useState<Aberta[] | null>(null);
  const [ledger, setLedger] = useState(0);
  const [falha, setFalha] = useState("");

  useEffect(() => {
    let vivo = true;
    (async () => {
      try {
        const [l, a] = await Promise.all([ledgerAtual(), assembleiasAbertas()]);
        if (!vivo) return;
        setLedger(l);
        setAbertas(a);
      } catch (e) {
        if (vivo) setFalha(String((e as Error).message ?? e));
      }
    })();
    return () => {
      vivo = false;
    };
  }, []);

  return (
    <main className="mx-auto max-w-4xl px-6 py-16">
      <p className="text-[10px] tracking-[0.2em] text-apagado uppercase">
        Tessera · testnet
      </p>
      <h1 className="mt-6 text-3xl leading-tight">
        O caderno diz quem faltou.
        <br />
        A urna não diz de quem é cada cédula.
      </h1>
      <p className="mt-6 max-w-xl text-sm leading-relaxed text-secundario">
        As duas coisas ao mesmo tempo, num contrato só. O comparecimento é
        identificado e público — é dele que sai a lista de quem não votou. A
        cédula sai de uma chave de uso único, com uma prova de que quem a mandou
        é um dos que compareceram, e nada além disso.
      </p>

      <dl className="mt-10 grid grid-cols-[auto_1fr] gap-x-6 gap-y-2 text-xs">
        <dt className="text-apagado">contrato</dt>
        <dd>
          <a
            className="text-acento hover:underline"
            href={`${REDE.explorer}/contract/${REDE.contrato}`}
            target="_blank"
            rel="noopener noreferrer"
          >
            <Curto v={REDE.contrato} /> ↗
          </a>
        </dd>
        <dt className="text-apagado">ledger</dt>
        <dd className="text-secundario">{ledger ? ledger.toLocaleString("pt-BR") : "—"}</dd>
      </dl>

      <section className="mt-16">
        <h2 className="text-[10px] tracking-[0.2em] text-apagado uppercase">
          Assembleias abertas agora
        </h2>
        <p className="mt-3 text-xs text-apagado">
          Lidas dos eventos da própria rede, sem índice e sem servidor.
        </p>

        {falha && <p className="mt-6 text-sm text-recusa">✗ {falha}</p>}
        {!falha && abertas === null && (
          <p className="mt-6 text-sm text-apagado">lendo a rede…</p>
        )}
        {abertas?.length === 0 && (
          <p className="mt-6 text-sm text-apagado">
            // nenhuma aberta agora. O RPC guarda uma janela de ledgers, então
            esta lista é "o que está aberto", não "tudo o que já existiu".
          </p>
        )}
        {abertas && abertas.length > 0 && (
          <ul className="mt-4 text-xs">
            {abertas.map((a) => (
              <Assembleia key={a.proposta} a={a} ledger={ledger} />
            ))}
          </ul>
        )}
      </section>
    </main>
  );
}

export default function App() {
  return (
    <Routes>
      <Route path="*" element={<Inicio />} />
    </Routes>
  );
}
