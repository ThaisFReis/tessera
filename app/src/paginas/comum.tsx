import { useEffect, useRef, useState } from "react";
import type { Passo } from "../rede";

/** O diário: cada passo que a página dá, em ordem. */
export function useDiario() {
  const [passos, setPassos] = useState<Passo[]>([]);
  const diario = useRef((p: Passo) => setPassos((v) => [...v, p])).current;
  return { passos, diario, limpar: () => setPassos([]) };
}

export function Diario({ passos }: { passos: Passo[] }) {
  if (!passos.length) return null;
  return (
    <section>
      <h3>o que está acontecendo</h3>
      <pre>
        {passos
          .map((p) => `${{ cmd: "$", val: " ", ok: "✓", x: "✗", nota: "//" }[p.tipo]} ${p.txt}`)
          .join("\n")}
      </pre>
    </section>
  );
}

/** Pede o ledger atual uma vez, e de novo a cada 15 s. */
export function useLedger(ler: () => Promise<number>) {
  const [ledger, setLedger] = useState(0);
  useEffect(() => {
    let vivo = true;
    const bater = async () => {
      try {
        const l = await ler();
        if (vivo) setLedger(l);
      } catch {
        /* a rede volta no próximo */
      }
    };
    bater();
    const t = setInterval(bater, 15000);
    return () => {
      vivo = false;
      clearInterval(t);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  return ledger;
}

export function Erro({ msg }: { msg: string }) {
  return msg ? <p>✗ {msg}</p> : null;
}
