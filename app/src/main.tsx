import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { HashRouter } from "react-router-dom";
import App from "./App";
import "./estilo.css";

/**
 * `HashRouter`, e não `BrowserRouter`.
 *
 * Medido no `dist` servido como arquivo estático: abrir `/demo/votar` direto
 * pedia `/demo/assets/index-*.js` e levava 404. A causa é `base: "./"` no
 * `vite.config.ts` — os caminhos dos assets são relativos, então uma rota com
 * segmento os resolve contra o próprio segmento. Navegando por dentro do app
 * funcionava; **recarregar ou abrir um link compartilhado, não**.
 *
 * E link compartilhado é o caso normal deste produto: é assim que uma votação
 * chega a quem vota. Um `/votacao/<id>` que quebra ao ser aberto é o pior lugar
 * possível para esse bug.
 *
 * O `#` resolve sem depender de onde o site é servido: o caminho real continua
 * sendo a raiz, e a rota vive no fragmento, que nenhum servidor estático
 * precisa entender. Serve em `/`, em `/tessera/`, ou onde for — sem fallback de
 * 404 e sem `base` absoluto chutado antes de saber o destino.
 */
createRoot(document.getElementById("raiz")!).render(
  <StrictMode>
    <HashRouter>
      <App />
    </HashRouter>
  </StrictMode>,
);
