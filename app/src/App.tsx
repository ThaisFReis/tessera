import { Link, Route, Routes } from "react-router-dom";
import Votacoes from "./paginas/Votacoes";
import Votacao from "./paginas/Votacao";
import Abrir from "./paginas/Abrir";
import Comparecer from "./paginas/Comparecer";
import Votar from "./paginas/Votar";
import Apurar from "./paginas/Apurar";
import { REDE } from "./rede";

/* Sem desenho nenhum, de propósito: enquanto as rotas estão nascendo, o que
   importa é o fluxo estar certo. A tela vem depois, e vem por cima. */
export default function App() {
  return (
    <div>
      <header>
        <strong>Tessera</strong> · testnet ·{" "}
        <a href={`${REDE.explorer}/contract/${REDE.contrato}`} target="_blank" rel="noopener">
          {REDE.contrato.slice(0, 8)}…
        </a>
        <nav>
          <Link to="/">votações</Link> | <Link to="/abrir">abrir uma votação</Link>
        </nav>
      </header>
      <hr />
      <Routes>
        <Route path="/" element={<Votacoes />} />
        <Route path="/votacao/:id" element={<Votacao />} />
        {/* organizador */}
        <Route path="/abrir" element={<Abrir />} />
        <Route path="/apurar/:id" element={<Apurar />} />
        {/* votante */}
        <Route path="/comparecer/:id" element={<Comparecer />} />
        <Route path="/votar/:id" element={<Votar />} />
        <Route path="*" element={<p>rota inexistente</p>} />
      </Routes>
    </div>
  );
}
