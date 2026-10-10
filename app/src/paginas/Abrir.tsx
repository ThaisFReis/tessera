import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { CarteiraLocal } from "../carteira";
import { guardarLista } from "../lista";
import { abrir, enderecoXdr, ledgerAtual, MAX_OPCOES, RAIZ_ABERTA } from "../rede";
import { carregar } from "../wasm";
import { explicar, secaoNasceuPequena } from "../secoes";
import { Aviso, Icone, LinkBastidores, Passos } from "../ui";
import { useDiario } from "./comum";

const PASSOS = ["Montando a raiz de aptos", "Assinando", "Confirmando na Stellar"];

/* ORGANIZADOR. Monta a cédula, o eleitorado e a janela, e abre na rede. */
export default function Abrir() {
  const [aptos, setAptos] = useState("");
  const [opcoes, setOpcoes] = useState("aprovar, rejeitar");
  const [anel, setAnel] = useState(true);
  const [fechadura, setFechadura] = useState(true);
  const [mesa, setMesa] = useState("");
  const [limiar, setLimiar] = useState(3);
  const [secoes, setSecoes] = useState(1);
  const [limite, setLimite] = useState(10);
  const [aberta, setAberta] = useState(true);
  const [minCaderno, setMinCaderno] = useState(10);
  const [minVoto, setMinVoto] = useState(30);
  const [falha, setFalha] = useState("");
  const [ocupado, setOcupado] = useState(false);
  const [progresso, setProgresso] = useState(0);
  const diario = useDiario("abrir");
  const ir = useNavigate();

  const linhas = (s: string) =>
    s.split(/[\n,]/).map((x) => x.trim()).filter(Boolean);

  async function enviar() {
    setFalha("");
    setOcupado(true);
    setProgresso(0);
    diario({ tipo: "ato", txt: "abriu uma votação" });
    try {
      const lista = aberta ? [] : linhas(aptos);
      if (!aberta && lista.length < 2) {
        throw new Error("o eleitorado precisa de ao menos duas pessoas");
      }
      const n = linhas(opcoes).length;
      if (n < 2) throw new Error("a pergunta precisa de ao menos duas opções");
      // O teto é do orçamento de CPU: cada opção sigilosa custa uma prova
      // disjuntiva, e o contrato recusa com `OpcoesForaDaFaixa`.
      if (n > MAX_OPCOES) {
        throw new Error(`a pergunta cabe em até ${MAX_OPCOES} opções, e esta tem ${n}`);
      }

      if (secoes < 1 || (!aberta && secoes > lista.length)) {
        throw new Error(
          aberta ? "as seções precisam ser ao menos uma" : `as seções têm de estar entre 1 e ${lista.length}`,
        );
      }

      // **O piso de anonimato, antecipado.** O contrato recusa apurar uma
      // seção com menos de τ cédulas, e essa recusa chega no fim da votação,
      // quando já não há o que fazer. Aqui ainda dá para escolher diferente.
      // Na votação aberta não dá para conferir: o eleitorado não existe na
      // hora de abrir, e é por isso que ali o aviso é outro.
      if (!aberta) {
        const pequena = secaoNasceuPequena(lista.length, secoes);
        if (pequena) throw new Error(explicar(lista.length, secoes, pequena));
      }

      const w = await carregar();
      const c = await CarteiraLocal.abrir();
      diario({ tipo: "val", txt: `organizador = ${c.endereco()}` });

      // O identificador nasce antes da raiz, e não é detalhe: a divisão em
      // seções é derivada dele, então a raiz depende dele.
      const id = crypto.getRandomValues(new Uint8Array(32));
      const proposta = Array.from(id, (b) => b.toString(16).padStart(2, "0")).join("");

      const xdrs = lista.map(enderecoXdr);
      const pesos = new Uint32Array(lista.length).fill(1);
      // 32 zeros é o sentinela de votação aberta: não há lista, e o contrato
      // pula a prova de Merkle.
      const raiz = aberta ? RAIZ_ABERTA : w.raiz_de_aptos(xdrs, pesos);
      diario({ tipo: "val", txt: aberta ? "sem lista: votação aberta" : `raiz de aptos = ${raiz}` });
      if (!aberta) {
        diario({ tipo: "nota", txt: "32 bytes vão para o contrato. A lista não vai." });
      }
      // **A seção não é decidida aqui, e isso é o ponto** (DEC-012). Ela sai da
      // assinatura da rodada da baliza que abre o comparecimento — uma
      // assinatura que ainda não existe neste instante. Enquanto a seção saía
      // do identificador da proposta, quem organizava moía o identificador até
      // pôr alguém numa seção cheia de atacantes, e o anonimato efetivo virava
      // 1. Agora não há o que moer, e nem esta tela sabe a divisão.
      if (!aberta && secoes > 1) {
        diario({
          tipo: "nota",
          txt: `${secoes} seções — a divisão sai da baliza depois, e nem quem abre a conhece agora`,
        });
      }

      // A rodada da baliza que abre o comparecimento. Tem de estar no futuro —
      // uma rodada já vencida tem assinatura publicada, e aí a divisão seria
      // moível. Rodadas saem a cada 3 segundos; meio minuto à frente é folga de
      // sobra para a transação entrar.
      const rodadaAbertura =
        !aberta && secoes > 1
          ? Number(w.rodada_em(BigInt(Math.floor(Date.now() / 1000) + 30)))
          : 0;

      // Mesa vazia exige limiar zero, e limiar zero exige mesa vazia.
      const membros = linhas(mesa);
      const k = membros.length === 0 ? 0 : limiar;
      if (membros.length > 0 && (k < 1 || k > membros.length)) {
        throw new Error(`o limiar tem de estar entre 1 e ${membros.length}`);
      }
      // O contrato recusa com `MembroRepetido`: um endereço duas vezes na mesa
      // conta duas parcelas para a mesma pessoa, e o limiar deixa de significar
      // o que diz.
      if (new Set(membros).size !== membros.length) {
        throw new Error("a mesa tem um endereço repetido, e o contrato recusa");
      }

      const agora = await ledgerAtual();
      // Ledger da testnet ≈ 6 s.
      const abre = agora + Math.round((minCaderno * 60) / 6);
      const fecha = abre + Math.round((minVoto * 60) / 6);

      // **A fechadura de tempo.** A rodada que destranca as cédulas é a que
      // vence pouco depois de a janela fechar — a janela é contada em ledgers,
      // e a rodada num instante, então a conversão é pelos mesmos minutos que a
      // pessoa digitou. O minuto de folga cobre a diferença entre o ledger
      // previsto e o ledger de verdade.
      //
      // Só faz sentido sem mesa: com mesa já há quem abra, e as duas coisas não
      // se somam. E só em anel, porque é a cédula em anel que não reparte o
      // fator com ninguém.
      const comFechadura = anel && fechadura && membros.length === 0;
      const rodada = comFechadura
        ? Number(w.rodada_em(BigInt(Math.floor(Date.now() / 1000) + (minCaderno + minVoto) * 60 + 60)))
        : 0;
      if (comFechadura) {
        diario({
          tipo: "nota",
          txt: `fechadura na rodada ${rodada} da baliza — a chave que abre as cédulas nasce sozinha quando a janela fechar`,
        });
      }
      // `abre >= fecha` é votação de duração zero, recusada com `PrazoNoPassado`.
      if (fecha <= abre) {
        throw new Error("a janela de votação precisa durar ao menos um minuto");
      }
      if (membros.length === 0) {
        diario({ tipo: "nota", txt: "sem mesa: ninguém poderá apurar, e é isso que se quis" });
      }
      setProgresso(1);
      await abrir(
        c, proposta, [{ opcoes: n, confidencial: true }], raiz, membros,
        k, abre, fecha, anel, aberta ? 1 : secoes, aberta ? limite : 0,
        rodada, rodadaAbertura, diario,
      );
      setProgresso(2);

      // A lista fica aqui porque não cabe na rede — e quem vota precisa dela.
      if (!aberta) guardarLista(proposta, lista);
      ir(`/votacao/${proposta}`);
    } catch (e) {
      const msg = String((e as Error).message ?? e);
      setFalha(msg);
      diario({ tipo: "x", txt: msg });
    } finally {
      setOcupado(false);
    }
  }

  return (
    <main className="pagina pagina-estreita">
      <header>
        <span className="eyebrow">ORGANIZADOR</span>
        <h1>Abrir uma votação.</h1>
      </header>

      <section>
        <h2>A PERGUNTA</h2>
        <label className="campo">
          <span className="eyebrow">OPÇÕES</span>
          <input value={opcoes} onChange={(e) => setOpcoes(e.target.value)} />
          <span className="campo-ajuda">Separadas por vírgula. A pergunta é sigilosa.</span>
        </label>
      </section>

      <section>
        <h2>QUEM PODE VOTAR</h2>
        <label className="escolha-modo">
          <input type="radio" name="eleitorado" checked={aberta} onChange={() => setAberta(true)} />
          <span>
            <strong>qualquer pessoa</strong>
            <span className="campo-ajuda">
              Quem chegar vota, sem lista nenhuma. É o modo da demonstração aberta.
            </span>
          </span>
        </label>
        <label className="escolha-modo" style={{ marginTop: 9 }}>
          <input type="radio" name="eleitorado" checked={!aberta} onChange={() => setAberta(false)} />
          <span>
            <strong>só quem está na lista</strong>
            <span className="campo-ajuda">
              A lista não vai para a rede — só a raiz de Merkle. Ela fica guardada neste navegador
              e você precisa entregá-la a quem vota.
            </span>
          </span>
        </label>

        {aberta ? (
          <Aviso tipo="nota">
            <strong>Numa votação aberta o total não significa nada.</strong> Qualquer pessoa vota
            quantas vezes quiser criando carteiras novas, e na testnet o friendbot as financia de
            graça. Também não existe lista de quem faltou, logo não existe voto obrigatório.
            <br />
            O que continua valendo é o que importa: <strong>ninguém descobre a escolha de
            ninguém, e nada liga uma pessoa à cédula dela</strong>. É sigilo que esta votação
            demonstra, não contagem.
          </Aviso>
        ) : (
          <label className="campo" style={{ marginTop: 20 }}>
            <span className="eyebrow">ENDEREÇOS APTOS</span>
            <textarea
              value={aptos}
              onChange={(e) => setAptos(e.target.value)}
              rows={6}
              placeholder="um endereço G… por linha"
            />
          </label>
        )}
      </section>

      <section>
        <h2>O MODO</h2>
        <label className="escolha-modo">
          <input type="checkbox" checked={anel} onChange={(e) => setAnel(e.target.checked)} />
          <span>
            <strong>caderno separado da urna</strong>
            <span className="campo-ajuda">
              {anel
                ? "quem comparece aparece com nome; a cédula sai de uma chave de uso único e não liga a ninguém"
                : "a cédula sai do endereço de quem vota: o ledger mostra que você votou, nunca em quê"}
            </span>
          </span>
        </label>
      </section>

      {anel && (
        <section>
          <h2>AS SEÇÕES</h2>
          {aberta ? (
            <>
              <label className="campo campo-estreito">
                <span className="eyebrow">PESSOAS POR SEÇÃO</span>
                <input
                  type="number"
                  value={limite}
                  min={5}
                  onChange={(e) => setLimite(Number(e.target.value))}
                />
              </label>
              <p>
                Você não adivinha quanta gente vem: diz o tamanho da seção e o contrato conta.{" "}
                <strong>A seção enche e a próxima abre sozinha.</strong>
              </p>
              <p>
                Verificar um anel custa <strong>10.822.850 instruções por membro</strong>. Com 30
                pessoas num anel só, cada cédula usa 91,4% do teto de CPU de uma transação e só
                uma entra por ledger — medido: 18 de 30 em 646 s, o resto expirou. Em seções de
                dez, a mesma cédula custa 37,2% e três entram por ledger.
              </p>
              {limite < 5 && (
                <Aviso tipo="erro">
                  Abaixo de cinco o contrato recusa a cédula: um anel pequeno demais não esconde
                  quem está nele. Use cinco ou mais.
                </Aviso>
              )}
            </>
          ) : (
            <>
              <label className="campo campo-estreito">
                <span className="eyebrow">QUANTAS</span>
                <input
                  type="number"
                  value={secoes}
                  min={1}
                  onChange={(e) => setSecoes(Number(e.target.value))}
                />
              </label>
              <p>
                <strong>Você não escolhe quem fica com quem</strong>, e nem pode: a divisão sai
                da assinatura de uma rodada da baliza que <em>ainda não venceu</em> neste
                instante. Não há o que moer. Quando ela for publicada, qualquer pessoa com a
                lista recalcula e confere. Se o organizador escolhesse, poria um dissidente numa
                seção sozinho e leria o voto dele.
              </p>
            </>
          )}
          <p>
            O preço é o conjunto de anonimato: ele passa a ser a sua seção, não a votação inteira.
            Com mesa, o resultado é único — o acumulador não sabe de que seção veio cada cédula.
            Com fechadura de tempo os totais são <strong>por seção</strong>, e a consequência tem
            de ser dita: numa seção unânime, qualquer pessoa sabe em que cada um dos seus membros
            votou. Até 32 pessoas, uma seção só é a configuração recomendada — acima disso o
            anel estoura o teto de CPU, e aí as seções deixam de ser escolha.
          </p>
        </section>
      )}

      <section>
        <h2>A MESA</h2>
        <label className="campo">
          <span className="eyebrow">MEMBROS</span>
          <textarea
            value={mesa}
            onChange={(e) => setMesa(e.target.value)}
            rows={3}
            placeholder="um endereço por linha — deixe vazio para não haver mesa"
          />
        </label>
        {mesa.trim() && (
          <label className="campo campo-estreito">
            <span className="eyebrow">LIMIAR</span>
            <input
              type="number"
              value={limiar}
              min={1}
              onChange={(e) => setLimiar(Number(e.target.value))}
            />
          </label>
        )}
        <p>
          {mesa.trim()
            ? "Quem está aqui pode reunir as parcelas e publicar o total — e o contrato recusa qualquer total que não abra o acumulado."
            : "Deixe vazio e ninguém poderá apurar."}
        </p>
        <p>
          <strong>Numa cédula em anel o fator de aleatoriedade não é repartido com ninguém</strong>,
          nem com a mesa. Então, em anel, mesa é só decoração: ninguém reconstrói a abertura e
          ninguém publica total. Sem mesa, o contrato diz isso em vez de a tela pedir desculpas —
          qualquer total afirmado cai em <code>AberturaNaoFecha</code>.
        </p>
        {anel && !mesa.trim() ? (
          <>
            <label className="escolha-modo">
              <input
                type="checkbox"
                checked={fechadura}
                onChange={(e) => setFechadura(e.target.checked)}
              />
              <span>
                <strong>fechadura de tempo</strong>
                <span className="campo-ajuda">
                  {fechadura
                    ? "o placar aparece sozinho quando a janela fechar, sem mesa e sem ninguém de confiança"
                    : "sem fechadura: o sigilo é absoluto, e o resultado é impossível"}
                </span>
              </span>
            </label>
            {fechadura ? (
              <>
                <p>
                  A cédula sai cifrada para uma rodada de uma baliza pública que vence quando a
                  janela fechar. <strong>Antes daquele instante ninguém abre</strong> — nem você,
                  que está abrindo esta votação. Depois dele, qualquer pessoa abre e apura, e é
                  isso que torna o placar impossível de travar: não depende de ninguém voltar.
                </p>
                <p>
                  O que isso custa: a baliza é uma suposição de confiança. Um conluio de um limiar
                  dos seus operadores abriria o conteúdo das cédulas antes da hora, e{" "}
                  <strong>se ela parar de publicar, o placar não sai e não há plano B</strong>. O
                  que não depende dela é o sigilo até o fim da janela, que é o relógio do ledger.
                </p>
              </>
            ) : (
              <p>O preço é este: o sigilo é absoluto, e o resultado é impossível.</p>
            )}
          </>
        ) : (
          <p>O preço é este: o sigilo é absoluto, e o resultado é impossível.</p>
        )}
      </section>

      <section>
        <h2>A JANELA</h2>
        <div className="campo-duplo">
          <label className="campo">
            <span className="eyebrow">{anel ? "COMPARECIMENTO" : "ESPERA"} (MIN)</span>
            <input
              type="number"
              value={minCaderno}
              min={1}
              onChange={(e) => setMinCaderno(Number(e.target.value))}
            />
          </label>
          <label className="campo">
            <span className="eyebrow">VOTAÇÃO (MIN)</span>
            <input
              type="number"
              value={minVoto}
              min={1}
              onChange={(e) => setMinVoto(Number(e.target.value))}
            />
          </label>
        </div>
      </section>

      <section>
        <button className="primary-button" onClick={enviar} disabled={ocupado}>
          {ocupado ? "Abrindo…" : "Abrir na testnet"} <Icone nome="arrow" />
        </button>
        {ocupado && <Passos passos={PASSOS} atual={progresso} />}
        {falha && <Aviso tipo="erro">{falha}</Aviso>}
        <LinkBastidores origem="abrir" />
      </section>
    </main>
  );
}
