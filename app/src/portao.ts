/**
 * Os dois portões do placar, e a ordem entre eles.
 *
 * Mora sozinho, sem importar nada, pelo mesmo motivo de `usoUnico.ts`: é a
 * regra de que a afirmação "nenhum placar antes do fim, nem parcial" depende, e
 * um módulo sem dependências pode ser testado por `node` direto, sem navegador
 * e sem rede.
 *
 * São dois porque medem coisas diferentes e nenhuma das duas basta. `fecha_em`
 * é **sequência de ledger**: é ela que diz que a urna não aceita mais cédula. A
 * rodada da baliza vence num **instante**: é ela que diz que a chave que abre
 * as cédulas passou a existir. Uma votação pode ter fechado sem que o relógio
 * tenha aberto, e a tela precisa dizer qual das duas coisas falta.
 *
 * O contrato impõe os mesmos dois, na mesma ordem, em `apurar_secao`
 * (`VotacaoAindaAberta` e depois `RelogioAindaNaoAbriu`). Aqui eles existem
 * para a tela explicar; lá para valer. Quem decide é o contrato.
 */

export type Portao =
  /** Sem fechadura: quem apura é a mesa, ou ninguém. */
  | { tipo: "sem-fechadura" }
  /** A urna ainda aceita cédula. Nada de placar, nem parcial. */
  | { tipo: "janela-aberta"; faltam: number }
  /** A urna fechou; a chave que abre as cédulas ainda não existe. */
  | { tipo: "esperando-relogio"; faltam: number }
  | { tipo: "pode-apurar" };

/**
 * `ledger` e `fecha_em` são sequência; `agora` e `vence` são segundos unix.
 *
 * A janela vem **antes** na resposta de propósito: enquanto a urna aceita
 * cédula, que o relógio já tenha aberto é irrelevante — e dizer "esperando o
 * relógio" ali seria falso, porque não é o relógio que falta.
 */
export function portao(
  p: { rodada: bigint; fecha_em: number },
  ledger: number,
  agora: number,
  vence: number,
): Portao {
  if (p.rodada === 0n) return { tipo: "sem-fechadura" };
  if (ledger < p.fecha_em) return { tipo: "janela-aberta", faltam: p.fecha_em - ledger };
  if (agora < vence) return { tipo: "esperando-relogio", faltam: vence - agora };
  return { tipo: "pode-apurar" };
}
