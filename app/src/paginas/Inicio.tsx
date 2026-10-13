import { Link } from "react-router-dom";
import { REDE } from "../rede";
import { Icone } from "../ui";
import "./inicio.css";

const ETAPAS = [
  { titulo: "Compareça.", texto: "Registre sua presença. Em uma votação com lista de aptos, o caderno permite conferir quem compareceu e quem faltou." },
  { titulo: "Vote.", texto: "No modo anônimo, uma assinatura em anel prova sua participação no grupo sem identificar qual integrante enviou a cédula." },
  { titulo: "Confira.", texto: "Com fechadura de tempo, qualquer pessoa pode apurar após o encerramento e a publicação da rodada da baliza. O dapp faz essa apuração ao abrir o resultado." },
];

export default function Inicio() {
  return (
    <main className="landing">
      <section className="landing-hero" aria-labelledby="inicio-titulo">
        <span className="eyebrow">VOTAÇÃO EM SOROBAN · STELLAR TESTNET</span>
        <h1 id="inicio-titulo">Sua presença é pública.<br /><span>Sua cédula é anônima.</span></h1>
        <p className="landing-intro">
          Um caderno para quem participa. Uma urna para as escolhas.
          No modo anônimo do Tessera, a prova de participação não identifica
          quem enviou cada cédula.
        </p>
        <div className="landing-actions">
          <Link className="primary-button" to="/demo/votar">Experimentar a cédula <Icone nome="arrow" /></Link>
          <Link className="secondary-button" to="/votacoes">Ver votações</Link>
        </div>
        <p className="landing-note">Prévia sem enviar transações. Votações na rede de testes.</p>
      </section>

      <section className="landing-flow" aria-labelledby="fluxo-titulo">
        <div className="landing-section-heading">
          <span className="eyebrow">COMO FUNCIONA</span>
          <h2 id="fluxo-titulo">Três passos.<br />Uma decisão coletiva.</h2>
        </div>
        <ol className="landing-steps">
          {ETAPAS.map((etapa, i) => (
            <li key={etapa.titulo}>
              <span className="landing-step-number" aria-hidden="true">0{i + 1}</span>
              <div><h3>{etapa.titulo}</h3><p>{etapa.texto}</p></div>
            </li>
          ))}
        </ol>
      </section>

      <section className="landing-limits" aria-labelledby="limites-titulo">
        <div className="landing-section-heading">
          <span className="eyebrow">ANTES DE PARTICIPAR</span>
          <h2 id="limites-titulo">O que fica público.</h2>
          <p>A presença e o placar agregado são públicos. Um resultado unânime pode revelar as escolhas de todos.</p>
        </div>
        <div className="landing-disclosures">
          <details open>
            <summary>O sigilo depende do modo escolhido.</summary>
            <p>O compromisso de Pedersen oculta a escolha. Com fechadura de tempo, o conteúdo das cédulas pode ser aberto depois da rodada; o anonimato de quem as enviou depende da assinatura em anel e do tamanho do grupo. Um anel de uma pessoa não oferece anonimato.</p>
          </details>
          <details>
            <summary>Como a votação é apurada?</summary>
            <p>Com fechadura de tempo, a apuração depende da baliza drand: se ela parar, não há alternativa prevista; um conluio de um limiar dos operadores pode abrir o conteúdo antes da hora. O contrato só aceita o resultado após o fechamento. Sem fechadura, a apuração com mesa é feita pela CLI. Uma votação em anel sem mesa e sem fechadura não apura.</p>
          </details>
          <details>
            <summary>Quem pode participar?</summary>
            <p>Na votação fechada, quem organiza define a lista de aptos. Na aberta, uma pessoa pode criar várias carteiras: esse modo demonstra o sigilo, mas não garante um voto por pessoa. Os participantes do anel são públicos.</p>
          </details>
          <details>
            <summary>Quais são os outros limites?</summary>
            <p>O anonimato do anel é computacional, não perfeito. O protocolo não impede que alguém observe sua tela enquanto você vota, nem que um cliente adulterado guarde sua escolha. O projeto não foi auditado por terceiros e está na testnet.</p>
          </details>
        </div>
      </section>

      <section className="landing-end" aria-labelledby="organizar-titulo">
        <div><span className="eyebrow">PARA QUEM ORGANIZA</span><h2 id="organizar-titulo">Abra a próxima votação.</h2></div>
        <Link className="secondary-button" to="/abrir">Organizar votação <Icone nome="arrow" /></Link>
      </section>
      <a className="landing-contract" href={`${REDE.explorer}/contract/${REDE.contrato}`} target="_blank" rel="noopener noreferrer">
        Conferir contrato na testnet <span>{REDE.contrato.slice(0, 8)}…{REDE.contrato.slice(-4)} ↗</span>
      </a>
    </main>
  );
}
