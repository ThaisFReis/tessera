/**
 * O placar que aparece sozinho.
 *
 * Esta é a peça que a assembleia sem mesa não tinha. Com mesa, alguém reúne as
 * parcelas e afirma os totais; sem mesa não havia ninguém, e a votação ficava
 * secreta para sempre — sigilo absoluto, resultado impossível. A fechadura de
 * tempo troca o retentor humano por um relógio: a chave que abre as cédulas
 * nasce sozinha no instante da rodada da baliza, e a partir dali **qualquer
 * pessoa** faz a conta.
 *
 * "Qualquer pessoa" inclui esta aba. É por isso que o placar aparece sem
 * ninguém clicar — e é por isso que ninguém consegue travá-lo: não depende de
 * quem organizou voltar, nem de quem votou voltar, nem desta aba em
 * particular.
 *
 * ## As duas coisas que este módulo não faz
 *
 * Não mostra nada antes de `fecha_em`, nem parcial, nem "parcialmente
 * decifrado". Não é discrição de interface: antes do fim da janela a chave da
 * rodada não existe, então não há parcial para mostrar. O portão é o relógio,
 * não um `if` numa tela.
 *
 * E não confia no relé da baliza: a assinatura passa por um pareamento em
 * `buscarBaliza` antes de encostar em qualquer conta.
 */
import {
  abrirSecao,
  apurarSecao,
  cedulasDaSecao,
  lerCadeia,
  lerResultado,
  lerResultadoSecao,
  publicarResultado,
  type Abertura,
  type Diario,
  type PropostaRede,
} from "./rede";
import { CarteiraEfemera } from "./carteira";
import { portao } from "./portao";
import { somarSecoes, TAU, type ParcialDaSecao } from "./secoes";
import { carregar } from "./wasm";

export type FasePlacar =
  /** A janela não fechou. Nada sai daqui, nem parcial. */
  | { tipo: "janela-aberta"; faltam: number }
  /** Proposta sem fechadura: quem apura é a mesa, ou ninguém. */
  | { tipo: "sem-fechadura" }
  /** A janela fechou, a rodada da baliza ainda não venceu. */
  | { tipo: "esperando-relogio"; faltam: number }
  | { tipo: "lendo" }
  | { tipo: "decifrando"; cedulas: number }
  | { tipo: "publicando" }
  | { tipo: "vazia" }
  /** Alguma seção não deu para ler ou apurar: a soma parcial não é o placar. */
  | { tipo: "multissecao"; secoes: number; publicadas: number }
  | {
      tipo: "pronto";
      totais: number[];
      abriram: number;
      /** Quantas havia. `null` quando o número veio do ledger, que guarda
       *  quantas abriram e não quantas existiam — e inventar o denominador
       *  seria pior que omiti-lo. */
      cedulas: number | null;
      /** Se estes números já estão no ledger, e não só nesta aba. */
      publicado: boolean;
      tx?: string;
    }
  | { tipo: "falha"; porque: string };

/** Quantas opções confidenciais a cédula tem — é o passo da lista achatada. */
const opcoesConfidenciais = (p: PropostaRede) =>
  p.perguntas.filter((q) => q.confidencial).reduce((a, q) => a + q.opcoes, 0);

/**
 * Apura sozinho, contando cada passo.
 *
 * `avancar` é chamado a cada mudança de fase — a tela desenha o que chegar, e o
 * diário narra. Não lança: toda falha vira `{ tipo: "falha" }`, porque uma
 * exceção aqui apagaria o placar de todo mundo por causa de um relé fora do ar.
 */
export async function apurarSozinho(
  id: string,
  p: PropostaRede,
  ledger: number,
  avancar: (f: FasePlacar) => void,
  diario?: Diario,
): Promise<void> {
  try {
    // Os dois portões vivem em `portao.ts`, sozinhos e testáveis. Quem decide
    // de verdade é o contrato, que lê o timestamp do ledger; isto aqui é para
    // a tela dizer **qual** das duas coisas falta, em vez de tentar e mostrar
    // um código de erro.
    const w = await carregar();
    const qual = portao(
      p,
      ledger,
      Math.floor(Date.now() / 1000),
      p.rodada === 0n ? 0 : Number(w.instante_da_rodada(p.rodada)),
    );
    if (qual.tipo !== "pode-apurar") return avancar(qual);

    const nConf = opcoesConfidenciais(p);
    const secoes = Math.max(1, p.secoes);

    // **§11-N, resolvida.** O tópico do evento passou a dizer a seção, então
    // quem apura separa as cédulas lendo só o ledger. O laço é por seção, e
    // cada uma é independente: uma seção que não dê para ler não impede as
    // outras, e o que sair incompleto não é apresentado como placar.
    avancar({ tipo: "lendo" });
    const parciais: (ParcialDaSecao & { aberturas: string[] })[] = [];
    let lidas = 0;

    for (let s = 0; s < secoes; s++) {
      // Quantas a cadeia registrou. É o denominador verdadeiro: se a leitura
      // dos eventos trouxer menos, o problema é a janela do RPC, e dizer isso
      // é melhor que levar `CadeiaNaoFecha` sem explicação.
      const { quantas } = await lerCadeia(id, s);
      if (quantas === 0) continue;
      const cedulas = await cedulasDaSecao(id, p.abre_em, s);
      if (cedulas.length !== quantas) {
        diario?.({
          tipo: "nota",
          txt:
            `seção ${s}: a cadeia registrou ${quantas} cédula${quantas === 1 ? "" : "s"} e ` +
            `eu li ${cedulas.length} dos eventos. Não apuro com lista incompleta.`,
        });
        continue;
      }

      lidas += cedulas.length;
      avancar({ tipo: "decifrando", cedulas: lidas });
      const a: Abertura = await abrirSecao(cedulas, nConf, p.rodada);
      diario?.({
        tipo: "nota",
        txt:
          `seção ${s}: a assinatura da rodada ${p.rodada} conferiu no pareamento, e ` +
          `${a.abriram} de ${a.cedulas} cédulas abriram`,
      });
      if (a.abriram < a.cedulas) {
        diario?.({
          tipo: "nota",
          txt: `${a.cedulas - a.abriram} criptograma${a.cedulas - a.abriram === 1 ? "" : "s"} não abriu — cada um custa só o próprio voto, e o placar segue`,
        });
      }

      // A seção guarda uma contagem e um ponto — nada em claro (DEC-017). Só
      // vale gravar se esta leitura abrir mais cédulas que a guardada.
      const ja = await lerResultadoSecao(id, s);
      if (!ja || ja.abriram < a.abriram) {
        avancar({ tipo: "publicando" });
        try {
          const c = await CarteiraEfemera.nascer();
          await apurarSecao(c, id, s, cedulas.flatMap((x) => x.compromissos), a, diario);
        } catch (e) {
          diario?.({
            tipo: "nota",
            txt: `a seção ${s} não foi gravada no ledger: ${String((e as Error).message ?? e)}. A conta acima não depende disso.`,
          });
        }
      }
      parciais.push({
        secao: s,
        totais: Array.from(a.totais),
        abriram: a.abriram,
        cedulas: a.cedulas,
        aberturas: Array.from(a.aberturas),
      });
    }

    // O placar que já estiver no ledger, para o caso de não dar para refazer a
    // conta agora.
    const noLedger = await lerResultado(id);

    if (parciais.length === 0) {
      if (noLedger) {
        return avancar({
          tipo: "pronto",
          totais: noLedger,
          abriram: 0,
          cedulas: null,
          publicado: true,
        });
      }
      return avancar({ tipo: "vazia" });
    }

    const soma = somarSecoes(parciais, secoes);
    if (!soma.completo) {
      if (noLedger) {
        return avancar({
          tipo: "pronto",
          totais: noLedger,
          abriram: 0,
          cedulas: null,
          publicado: true,
        });
      }
      return avancar({ tipo: "multissecao", secoes, publicadas: parciais.length });
    }

    const pronto = {
      tipo: "pronto" as const,
      totais: soma.totais,
      abriram: soma.abriram,
      cedulas: soma.cedulas,
    };

    // **O piso agregado.** Abaixo de τ cédulas abertas no total, o contrato não
    // publica placar em claro — e não publicar é a garantia funcionando, não um
    // defeito. A conta acima continua sendo a conta certa.
    if (soma.abriram < TAU) {
      diario?.({
        tipo: "nota",
        txt:
          `${soma.abriram} cédula${soma.abriram === 1 ? "" : "s"} aberta${soma.abriram === 1 ? "" : "s"} no total, e o piso de anonimato é ${TAU}: ` +
          `o contrato não grava um placar em claro que denunciaria quem votou.`,
      });
      return avancar({ ...pronto, publicado: false });
    }

    // **O placar da votação, numa transação só.** É o único lugar em que um
    // número de voto entra no ledger. Publicar não é o que torna o placar
    // verdadeiro — a conta acima é conferível por quem quiser, e o contrato
    // recusaria um agregado que não fechasse. Publicar serve a quem abrir esta
    // página depois e não quiser refazer a conta.
    if (noLedger && soma.totais.every((t, j) => t === noLedger[j])) {
      return avancar({ ...pronto, publicado: true });
    }
    avancar({ tipo: "publicando" });
    try {
      // Uma carteira descartável: apurar não é um ato de identidade, e assinar
      // com a carteira de identidade só acrescentaria um vínculo inútil ao
      // ledger. O contrato aceita qualquer endereço aqui — é isso que faz a
      // apuração não travável.
      const c = await CarteiraEfemera.nascer();
      const w = await carregar();
      const aberturas = soma.totais.map((_, j) =>
        w.somar_aberturas(parciais.map((x) => x.aberturas[j])),
      );
      const tx = await publicarResultado(
        c,
        id,
        parciais.map((x) => x.secao),
        soma.totais,
        aberturas,
        diario,
      );
      avancar({ ...pronto, publicado: true, tx });
    } catch (e) {
      const porque = String((e as Error).message ?? e);
      diario?.({
        tipo: "nota",
        txt: `o placar não foi gravado no ledger: ${porque}. A conta acima não depende disso.`,
      });
      avancar({ ...pronto, publicado: false });
    }
  } catch (e) {
    avancar({ tipo: "falha", porque: String((e as Error).message ?? e) });
  }
}
