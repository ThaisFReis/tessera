import { Link } from "react-router-dom";
import { REDE } from "../rede";
import { Icone } from "../ui";
import "./inicio.css";

/**
 * A porta de entrada.
 *
 * O texto daqui não pode prometer mais do que o contrato cumpre — é a mesma
 * regra que `console/guarda.py` impõe ao deck. Por isso os números vêm de
 * medição (`contrato/src/test.rs::orcamento_do_anel` e as sondas do README), e
 * os limites estão numa seção própria em vez de ficarem de fora.
 */

const RISCOS = [
  { n: "01", onde: "CONSELHO", titulo: "Quem aprova\nseu orçamento.", txt: "Você precisa votar contra quem assina o orçamento de que depende.", rotulo: "Retaliação" },
  { n: "02", onde: "COOPERATIVA", titulo: "Quem controla\nseu crédito.", txt: "Você discorda da diretoria que decide sua próxima linha de crédito.", rotulo: "Coação" },
  { n: "03", onde: "DAO", titulo: "Quem financia\nseu trabalho.", txt: "Você vota contra a baleia que financia o grant de que vive.", rotulo: "Conformidade" },
];

const TEMPO = [
  { quando: "HOJE", titulo: "A cédula é cifrada.", txt: "O voto parece protegido, mas o texto cifrado continua na rede." },
  { quando: "NA APURAÇÃO", titulo: "Alguém tem a chave.", txt: "Quem conta pode ser justamente de quem o votante precisa se proteger." },
  { quando: "ANOS DEPOIS", titulo: "O vazamento volta no tempo.", txt: "Uma chave comprometida abre votos que já pareciam história." },
];

const CUSTOS = [
  { o: "anel de 5", i: "54.114.518", p: "13,5%" },
  { o: "anel de 10", i: "108.228.508", p: "27,1%" },
  { o: "anel de 20", i: "216.456.488", p: "54,1%" },
];

export default function Inicio() {
  return (
    <main className="landing">
      <section className="hero">
        <span className="eyebrow">TESSERA · STELLAR TESTNET</span>
        <h1>Voto secreto.<br />Decisão coletiva.</h1>
        <p className="hero-sub">
          Um módulo de votação para contratos Soroban. Sabe-se que uma pessoa votou, e isso é
          público e auditável. Não se sabe em que ela votou — e isso não depende de ninguém
          cumprir uma promessa.
        </p>
        <div className="hero-acoes">
          <Link className="primary-button" to="/votacoes">Ver votações abertas <Icone nome="arrow" /></Link>
          <Link className="secondary-button" to="/demo/votar">Experimentar a cédula</Link>
        </div>
        <p className="hero-nota">
          A prévia não escreve na rede. As votações da lista são reais, na testnet.
        </p>
      </section>

      <section className="faixa">
        <span className="eyebrow">01 / O PRINCÍPIO</span>
        <h2>Uma peça guarda o segredo.<br />O conjunto revela a imagem.</h2>
        <div className="duas">
          <div className="painel">
            <span className="eyebrow">COMPROMISSO INDIVIDUAL</span>
            <p>Isolada, nenhuma escolha é legível. Um compromisso de Pedersen oculta o voto.</p>
          </div>
          <div className="painel">
            <span className="eyebrow">Σ / RESULTADO COLETIVO</span>
            <p>Somadas, uma decisão verificável. O agregado permite conferir o total declarado.</p>
          </div>
        </div>
        <p className="nota-faixa">
          <em>Tessera</em>, a pastilha de um mosaico — e, no latim, também a ficha de acesso.
          A elegibilidade faz parte do nome.
        </p>
      </section>

      <section className="faixa">
        <span className="eyebrow">02 / O PROBLEMA</span>
        <h2>O voto é público.<br />O risco é pessoal.</h2>
        <p className="faixa-sub">
          Quando nome e escolha ficam juntos na blockchain, a exposição não termina na apuração.
        </p>
        <div className="tres">
          {RISCOS.map((r) => (
            <div className="painel" key={r.n}>
              <span className="eyebrow">{r.n} / {r.onde}</span>
              <h3>{r.titulo}</h3>
              <p>{r.txt}</p>
              <span className="selo">{r.rotulo}</span>
            </div>
          ))}
        </div>
        <p className="equacao">NOME + ESCOLHA + REGISTRO PERMANENTE → EXPOSIÇÃO PERMANENTE</p>
      </section>

      <section className="faixa">
        <span className="eyebrow">03 / O TEMPO É PARTE DA AMEAÇA</span>
        <h2>O registro é permanente.<br />A chave pode vazar.</h2>
        <div className="tres">
          {TEMPO.map((t) => (
            <div className="painel" key={t.quando}>
              <span className="eyebrow">{t.quando}</span>
              <h3>{t.titulo}</h3>
              <p>{t.txt}</p>
            </div>
          ))}
        </div>
        <p className="destaque">Cédula de papel queima. Blockchain, não.</p>
        <p className="faixa-sub">
          Por isso o ledger <strong>nunca recebe um texto cifrado do voto</strong>. Ele recebe um
          compromisso de Pedersen, <code>C = v·G + r·H</code>, que é perfeitamente ocultante:
          matematicamente vazio de informação, contra qualquer poder computacional. Não há chave
          que possa vazar depois, porque não existe chave.
        </p>
      </section>

      <section className="faixa">
        <span className="eyebrow">04 / O CADERNO E A URNA</span>
        <h2>Quem faltou é público.<br />De quem é cada cédula, não.</h2>
        <p className="faixa-sub">
          Esconder a escolha não basta quando o remetente da cédula é o seu endereço. A urna
          brasileira resolve isso há décadas sem criptografia nenhuma: o caderno diz quem
          compareceu, a urna diz o que foi votado, e <strong>nada liga os dois</strong>. Tessera
          faz o mesmo em dois atos.
        </p>
        <div className="duas">
          <div className="painel">
            <span className="eyebrow">01 · COMPARECER</span>
            <h3>Identificado, e de propósito.</h3>
            <p>
              Você prova que está na lista de aptos e entra no caderno com o seu endereço. É
              público — e é exatamente isso que permite voto obrigatório, porque
              <em> aptos − compareceram</em> é a lista de quem faltou.
            </p>
          </div>
          <div className="painel">
            <span className="eyebrow">02 · VOTAR</span>
            <h3>De uma chave que não é a sua.</h3>
            <p>
              A cédula sai de uma chave de uso único, com uma assinatura em anel que prova que
              quem assinou está entre os que compareceram — sem dizer qual deles. Uma imagem de
              chave impede a segunda cédula da mesma pessoa.
            </p>
          </div>
        </div>
        <p className="nota-faixa">
          O anel é a prova disjuntiva que o projeto já tinha, generalizada de 2 para <em>n</em>
          ramos. Sem SNARK, sem cerimônia de setup, sem circuito.
        </p>
      </section>

      <section className="faixa">
        <span className="eyebrow">05 / MEDIDO, NÃO ESTIMADO</span>
        <h2>Os números vêm da rede.</h2>
        <p className="faixa-sub">
          O teto de CPU por transação na Stellar, achado por bissecção na testnet, é de
          400.000.000 de instruções. Um voto confidencial completo custa 36.781.170 — 9,2% de
          uma transação. Uma cédula pública custa 350.372: <strong>o sigilo custa 105×</strong>,
          e esse é o preço, medido.
        </p>
        <table className="tabela">
          <thead><tr><th>verificação do anel</th><th>instruções</th><th>do teto</th></tr></thead>
          <tbody>
            {CUSTOS.map((c) => (
              <tr key={c.o}><td>{c.o}</td><td className="num">{c.i}</td><td className="num">{c.p}</td></tr>
            ))}
          </tbody>
        </table>
        <p className="nota-faixa">
          Linear: 10.822.850 por membro. Com a cédula junto, <strong>vinte pessoas é o limite
          prático</strong> de um anel — o mesmo limite que o Brasil encontrou, e que lá se
          responde agregando seções pequenas.
        </p>
      </section>

      <section className="faixa">
        <span className="eyebrow">06 / O QUE ISTO NÃO FAZ</span>
        <h2>Confiança começa com<br />limites explícitos.</h2>
        <div className="duas">
          <div className="painel">
            <span className="eyebrow">DUAS GARANTIAS DIFERENTES</span>
            <p>
              O compromisso é <strong>perfeitamente</strong> ocultante — não há suposição a
              quebrar. O anonimato do anel não: ele repousa sobre um problema que se acredita
              difícil. São garantias de naturezas distintas, e achatar as duas numa só seria
              vender o que não existe.
            </p>
          </div>
          <div className="painel">
            <span className="eyebrow">A LISTA FICA PÚBLICA</span>
            <p>
              Um anel só é verificável por quem tem as chaves de todos os ramos: anonimato de
              anel é anonimato <em>dentro de um conjunto conhecido</em>. É coerente com a urna —
              no Brasil o eleitorado e o caderno são ambos públicos — mas é uma troca.
            </p>
          </div>
          <div className="painel">
            <span className="eyebrow">UM ANEL DE UM NÃO ESCONDE NINGUÉM</span>
            <p>
              O sigilo é uma propriedade do grupo, não da matemática sozinha. Com uma pessoa só,
              não há de quem se esconder — e a tela diz isso antes de você votar.
            </p>
          </div>
          <div className="painel">
            <span className="eyebrow">SEM MESA, NÃO HÁ APURAÇÃO</span>
            <p>
              Uma votação em anel não reparte com ninguém o fator que esconde o voto. Ninguém
              pode abrir uma cédula — e, pelo mesmo motivo, ninguém pode publicar um total. O
              sigilo é absoluto e o resultado é impossível: é o desenho, não um defeito.
            </p>
          </div>
        </div>
      </section>

      <section className="fecho">
        <h2>O sigilo não é<br />uma promessa.</h2>
        <p className="fecho-sub">É o desenho. Quem organiza a votação não tem como abrir a sua cédula, e nós também não.</p>
        <div className="hero-acoes">
          <Link className="primary-button" to="/votacoes">Ver votações <Icone nome="arrow" /></Link>
          <Link className="secondary-button" to="/abrir">Abrir a sua</Link>
        </div>
        <p className="hero-nota">
          <a href={`${REDE.explorer}/contract/${REDE.contrato}`} target="_blank" rel="noopener noreferrer">
            contrato {REDE.contrato.slice(0, 8)}…{REDE.contrato.slice(-4)} no explorer ↗
          </a>
        </p>
      </section>
    </main>
  );
}
