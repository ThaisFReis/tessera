import { Link, NavLink, Route, Routes, useLocation } from "react-router-dom";
import Inicio from "./paginas/Inicio";
import Votacoes from "./paginas/Votacoes";
import Votacao from "./paginas/Votacao";
import Abrir from "./paginas/Abrir";
import Comparecer from "./paginas/Comparecer";
import Votar from "./paginas/Votar";
import Apurar from "./paginas/Apurar";
import Bastidores from "./paginas/Bastidores";
import { REDE } from "./rede";

export default function App() {
  const { pathname } = useLocation();
  // O fluxo de quem vota — presença e cédula — perde a navegação secundária.
  // A trilha de etapas é a única navegação que faz sentido ali dentro.
  const participante =
    pathname.startsWith("/votar/") || pathname === "/demo/votar" || pathname.startsWith("/comparecer/");
  return (
    <div className={participante ? "app participante" : "app"}>
      <a className="skip-link" href="#conteudo">Skip to content </a>
      <header className="app-header">
        <Link className="wordmark" to="/" aria-label="Tessera — home">
          <span className="brand-mark" aria-hidden="true">{Array.from({ length: 9 }, (_, i) => <i key={i} />)}</span>
          Tessera<span className="brand-period">.</span>
        </Link>
        {!participante && <nav aria-label="Main navigation">
          <NavLink to="/votacoes">Votes </NavLink>
          <NavLink to="/abrir">Organize </NavLink>
        </nav>}
        <a className="network-tag" href={`${REDE.explorer}/contract/${REDE.contrato}`} target="_blank" rel="noopener noreferrer">
          <span className="status-dot" /> Stellar <span className="network-name">Testnet</span><span aria-hidden="true">↗</span>
        </a>
      </header>
      <div id="conteudo" className={participante ? "participant-content" : "standard-content"}>
      <Routes>
        <Route path="/" element={<Inicio />} />
        <Route path="/votacoes" element={<Votacoes />} />
        <Route path="/votacao/:id" element={<Votacao />} />
        {/* organizador */}
        <Route path="/abrir" element={<Abrir />} />
        <Route path="/apurar/:id" element={<Apurar />} />
        {/* votante */}
        <Route path="/comparecer/:id" element={<Comparecer />} />
        <Route path="/votar/:id" element={<Votar />} />
        <Route path="/demo/votar" element={<Votar demo />} />
        {/* o outro lado, para a segunda janela do vídeo */}
        <Route path="/bastidores" element={<Bastidores />} />
        <Route path="*" element={<p className="pagina">Page not found</p>} />
      </Routes>
      </div>
      {!participante && <footer className="app-footer"><span>Tessera · The picture only emerges together. </span><span>Built on Stellar  <span aria-hidden="true">↗</span></span></footer>}
    </div>
  );
}
