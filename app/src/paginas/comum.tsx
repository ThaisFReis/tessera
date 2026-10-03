import { useEffect, useRef, useState } from "react";
import { anotar, type Anotar, type Origem } from "../diario";

/**
 * O diário de uma página.
 *
 * Ele não mora mais aqui dentro: escreve na loja compartilhada de
 * `src/diario.ts`, que atravessa janelas. A página só diz de onde o passo veio
 * — quem lê é `/bastidores`, numa segunda janela, que é o que o vídeo precisa.
 */
export function useDiario(origem: Origem, proposta?: string): Anotar {
  const ref = useRef<Anotar>(() => {});
  ref.current = (p) => anotar({ ...p, t: Date.now(), origem, proposta });
  return useRef<Anotar>((p) => ref.current(p)).current;
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
