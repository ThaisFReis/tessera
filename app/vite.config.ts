import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwind from "@tailwindcss/vite";

// `base` relativo porque o destino é GitHub Pages num subcaminho — e porque um
// arquivo estático que funciona de qualquer lugar é o ponto: não há servidor
// nem caminho absoluto de que a página dependa.
export default defineConfig({
  base: "./",
  plugins: [react(), tailwind()],
  // O .wasm vai como asset, não inline: 216 KB em base64 dentro do JS atrasaria
  // a primeira tela sem necessidade.
  assetsInclude: ["**/*.wasm"],
  optimizeDeps: { exclude: ["tessera-cliente"] },
});
