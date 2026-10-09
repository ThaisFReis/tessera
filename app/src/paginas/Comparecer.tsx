import { useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { CarteiraLocal } from "../carteira";
import { guardarChaveDeAnel, guardarLista, lerLista } from "../lista";
import { comparecer, ehAberta, enderecoXdr, lerAbertura, lerProposta, lerSecao, lerSecoes } from "../rede";
import { carregar } from "../wasm";
import { Aviso, Icone, LinkBastidores, Passos, Trilha } from "../ui";
import { useDiario } from "./comum";
import "./votar.css";

const PASSOS = ["Montando o seu caminho de Merkle", "Criando a chave de participação", "Confirmando na Stellar"];

/* VOTANTE, primeiro ato: o caderno.
   É o único momento em que o nome da pessoa aparece — e é ele que permite voto
   obrigatório, porque `aptos − compareceram` é a lista de quem faltou. */
export default function Comparecer() {
  const { id = "" } = useParams();
  const [lista, setLista] = useState<string[]>(() => lerLista(id) ?? []);
  const [aberta, setAberta] = useState<boolean | null>(null);
  const [colada, setColada] = useState("");
  const [falha, setFalha] = useState("");
  const [ocupado, setOcupado] = useState(false);
  const [progresso, setProgresso] = useState(0);
  const diario = useDiario("comparecer", id);
  const ir = useNavigate();

  useEffect(() => {
    let vivo = true;
    lerProposta(id)
      .then((p) => vivo && setAberta(p ? ehAberta(p) : null))
      .catch(() => vivo && setAberta(null));
    return () => {
      vivo = false;
    };
  }, [id]);

  function aceitarColada() {
    const l = colada.split(/[\n,]/).map((x) => x.trim()).filter(Boolean);
    guardarLista(id, l);
    setLista(l);
  }

  async function enviar() {
    setFalha("");
    setOcupado(true);
    setProgresso(0);
    diario({ tipo: "ato", txt: "foi confirmar a presença, com o próprio nome" });
    try {
      const w = await carregar();
      const c = await CarteiraLocal.abrir();
      const meu = c.endereco();
      const indice = lista.indexOf(meu);
      if (aberta === false && indice < 0) {
        throw new Error(
          `${meu} não está na lista de aptos desta votação. O contrato recusaria com NaoEstaNaListaDeAptos.`,
        );
      }

      const prop = await lerProposta(id);
      if (!prop) throw new Error("Esta votação não foi encontrada.");

      // Na votação aberta não há lista, logo não há caminho de Merkle — e a
      // seção vem da ordem de chegada; quem entrar no meio empurra, e é por
      // isso que há janela no footprint logo abaixo.
      const caminho = ehAberta(prop)
        ? { irmaos: [] as string[], indice: 0 }
        : (w.caminho_de(lista.map(enderecoXdr), new Uint32Array(lista.length).fill(1), indice) as {
            irmaos: string[];
            indice: number;
          });
      diario({ tipo: "val", txt: `caminho de Merkle com ${caminho.irmaos.length} irmão${caminho.irmaos.length === 1 ? "" : "s"}` });

      // **A seção vem da baliza** (DEC-012), não da lista e não do
      // identificador da proposta. Enquanto saía da proposta, quem organizava
      // moía o identificador até pôr alguém numa seção cheia de atacantes.
      // A assinatura da rodada de abertura não existe quando a votação é
      // aberta, então não há o que moer — e o contrato confere a seção que esta
      // tela calcula.
      let seccao: number;
      if (ehAberta(prop)) {
        seccao = Math.max(0, (await lerSecoes(id)) - 1);
      } else if (prop.secoes > 1) {
        const assinatura = await lerAbertura(id);
        if (!assinatura) {
          throw new Error(
            "A rodada da baliza que decide as seções ainda não foi registrada. " +
              "Qualquer pessoa pode registrá-la assim que a rodada vencer, e sem " +
              "ela o contrato recusa o comparecimento.",
          );
        }
        seccao = w.secao_de(assinatura, enderecoXdr(meu), prop.secoes) as number;
        diario({ tipo: "val", txt: `seção ${seccao} de ${prop.secoes} — derivada da baliza, não escolhida` });
      } else {
        seccao = 0;
      }

      setProgresso(1);
      const chave = w.nova_chave_de_anel() as { secreta: string; publica: string };
      diario({ tipo: "nota", txt: "a chave secreta do anel fica nesta aba, e só aqui" });
      diario({ tipo: "val", txt: `chave pública = ${chave.publica.slice(0, 24)}…` });

      setProgresso(2);
      await comparecer(
        c, id, chave.publica, caminho.irmaos, caminho.indice, seccao, diario,
        Math.max(lista.length, 40),
        // Três seções além da prevista: cobre uma rajada de 3× o limite
        // entrando entre a simulação e a aplicação.
        ehAberta(prop) ? 3 : 0,
      );
      // Na aberta quem decide a seção é o contrato: pergunta a ele, não ao
      // palpite do cliente.
      const secao = (await lerSecao(id, meu)) ?? seccao;
      if (secao !== seccao) {
        diario({ tipo: "val", txt: `o contrato pôs você na seção ${secao}, por ordem de chegada` });
      }
      // Sem ela não há voto depois. Perder esta aba é perder o voto.
      guardarChaveDeAnel(id, { ...chave, secao });
      ir(`/votacao/${id}`);
    } catch (e) {
      const msg = String((e as Error).message ?? e);
      setFalha(msg);
      diario({ tipo: "x", txt: msg });
    } finally {
      setOcupado(false);
    }
  }

  return (
    <main className="voting-page">
      <div className="voting-breadcrumb">
        <Link to={`/votacao/${id}`}><span aria-hidden="true">←</span> Voltar</Link>
      </div>
      <h1 className="sr-only">Sua presença</h1>
      <Trilha etapas={[
        { label: "Presença", completo: false, ativo: true },
        { label: "Voto", completo: false, ativo: false },
        { label: "Confirmação", completo: false, ativo: false },
      ]} />

      <section className="ballot-panel" aria-label="Confirmação de presença" aria-busy={ocupado}>
        <div className="ballot-topline">
          <span className="eyebrow">VOTAÇÃO · {id.slice(0, 8) || "—"}</span>
          <span className="ballot-status"><span className="status-dot" />Comparecimento</span>
        </div>

        <div className="ballot-body stage-enter">
          <h2>Este é o caderno.</h2>
          <p className="ballot-description">
            Ele é <strong>identificado e público</strong>: o ledger vai mostrar que você
            compareceu, e é disso que sai a lista de quem faltou. Ele não mostra em que você
            votou — a cédula vem depois, de outra chave.
          </p>

          {aberta === null ? (
            <p className="campo-ajuda" style={{ marginTop: 29 }}>Lendo a votação…</p>
          ) : aberta ? (
            <div className="stage-enter">
              <p className="review-note" style={{ marginTop: 29 }}>
                <Icone nome="users" /> Votação aberta: não há lista, e você entra como chegou.
              </p>
              <div className="ballot-action">
                <button className="primary-button" onClick={enviar} disabled={ocupado}>
                  {ocupado ? "Comparecendo…" : "Confirmar minha presença"} <Icone nome="arrow" />
                </button>
              </div>
              {ocupado && <Passos passos={PASSOS} atual={progresso} />}
            </div>
          ) : lista.length === 0 ? (
            <div className="stage-enter">
              <label className="campo" style={{ marginTop: 29 }}>
                <span className="eyebrow">A LISTA DE APTOS</span>
                <textarea
                  value={colada}
                  onChange={(e) => setColada(e.target.value)}
                  rows={6}
                  placeholder="um endereço G… por linha"
                />
                <span className="campo-ajuda">
                  Ela não está na rede, de propósito: o contrato guarda 32 bytes de raiz e nada
                  mais. Cole aqui a lista que o organizador entregou.
                </span>
              </label>
              <div className="ballot-action">
                <button className="primary-button" onClick={aceitarColada} disabled={!colada.trim()}>
                  Usar esta lista <Icone nome="arrow" />
                </button>
              </div>
            </div>
          ) : (
            <div className="stage-enter">
              <p className="review-note" style={{ marginTop: 29 }}>
                <Icone nome="users" /> {lista.length} aptos nesta lista.
              </p>
              <div className="ballot-action">
                <button className="primary-button" onClick={enviar} disabled={ocupado}>
                  {ocupado ? "Comparecendo…" : "Confirmar minha presença"} <Icone nome="arrow" />
                </button>
              </div>
              {ocupado && <Passos passos={PASSOS} atual={progresso} />}
            </div>
          )}
          {falha && <Aviso tipo="erro">{falha}</Aviso>}
        </div>
      </section>

      <details className="privacy-details">
        <summary><Icone nome="lock" /><span>Por que a presença é pública</span><span className="disclosure-plus" aria-hidden="true">+</span></summary>
        <p>
          Sem o caderno não dá para saber quem faltou, e sem isso não existe voto obrigatório.
          É a mesma separação da urna: o caderno diz quem veio, a urna diz o que foi votado, e
          nada liga os dois.
        </p>
        <p>
          A chave de participação criada aqui fica neste navegador. Ela é o que assina a cédula
          depois, sem o seu endereço junto — perder esta aba é perder o voto.
        </p>
      </details>

      <p className="bastidores-rodape"><LinkBastidores origem="comparecer" /></p>
    </main>
  );
}
