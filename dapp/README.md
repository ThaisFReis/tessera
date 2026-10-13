# `dapp/` — o dapp construído, servido pelo GitHub Pages

**Isto é artefato de build, e está no git de propósito.** O Pages deste
repositório é do tipo *legacy*: serve a raiz do `master` como arquivo estático,
sem GitHub Actions. O dapp depende de `cliente-wasm/pacote`, gerado por
`wasm-pack`, que **não** está no git e não deve estar — então publicar exige ou
um workflow novo (infraestrutura que o §0 diz para não acrescentar sozinho) ou
commitar o `dist` pronto. É a segunda.

Regenerar:

```bash
cd app && npm run wasm && npx vite build --outDir ../dapp --emptyOutDir
```

**Não edite nada aqui.** A fonte é `app/src/`.

Conferido antes de publicar, que é o passo que a §11-P manda não pular: servido
como arquivo estático num subcaminho e aberto num **link profundo**, não
navegando por dentro. Foi assim que o bug de rota da DEC-016 apareceu, e é assim
que ele não volta.
