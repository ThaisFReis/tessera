# Tessera

**Voto secreto para qualquer governança, na Stellar.**
Hackathon Find Your Way (Meridian) · trilha General

Tessera é um módulo de votação para contratos Soroban. Não é um aplicativo de
governança nem uma DAO: é um contrato com três funções que qualquer governança
já existente pode chamar para realizar uma votação em que

- **sabe-se que uma pessoa votou**, e isso é público e auditável;
- **não se sabe em que ela votou**, e isso é secreto *para sempre*, não apenas
  enquanto a criptografia de hoje resistir;
- **qualquer pessoa pode recalcular o resultado** a partir do ledger e detectar
  uma mesa apuradora que minta.

A ideia que organiza o desenho: num registro permanente, "criptografado hoje"
significa "legível quando a chave vazar". Então o ledger **nunca recebe um texto
cifrado do voto**. Ele recebe um compromisso de Pedersen, que é perfeitamente
ocultante — matematicamente vazio de informação, contra qualquer poder
computacional, para sempre. O voto em si transita fora da cadeia e é destruído.

---

## Estado, sem maquiagem

| | |
|---|---|
| Sonda criptográfica medida na testnet | ✅ no ar |
| Desenho Pedersen verificado on-chain | ✅ medido |
| Contrato de urna (`abrir`/`votar`/`apurar`) | ❌ em construção |
| Cliente CLI | ❌ em construção |
| Verificador público | ❌ em construção |

**O que está no ar hoje é a sonda que estabelece o modelo de custo, não a urna.**
Todo número abaixo veio de uma invocação real na testnet da Stellar, nunca de
simulação. O que é projeção está marcado como projeção.

---

## Os números medidos

Teto de CPU por transação, achado por bissecção na testnet: **400.000.000**.

| operação | instruções | taxa |
|---|---|---|
| `g1_add` | 110.748 | |
| `g1_mul` | 3.290.524 | |
| `g1_msm` | 2.454.719 + 1.471.917 por termo | |
| `hash_to_g1` | 2.653.011 | |
| `pairing_check(k)` | 10.571.128 + 6.746.479 por par | |
| verificação Schnorr | 6.737.614 | 9.558 stroops |
| **prova disjuntiva CDS** | **10.980.243** | **14.144 stroops** |
| **apuração Pedersen** | **5.408.931** | **8.622 stroops** |
| **caminho de Merkle (profundidade 8)** | **127.863** | **5.235 stroops** |
| apuração ElGamal (desenho anterior) | 6.712.183 | 11.852 stroops |
| verificação Groth16 (4 entradas públicas) | 47.371.348 | |

Três conclusões que mudaram o desenho:

1. **`g1_add` é ~30× mais barato que `g1_mul`.** A agregação homomórfica é
   praticamente de graça; o custo está todo na verificação de prova.
2. **Não faça a cadeia procurar o resultado, faça-a conferir.** Log discreto por
   força bruta custa ~124k por unidade de peso, o que dá um teto de ~3.218 de
   peso por transação. Conferir uma abertura afirmada custa **5.408.931,
   constante no comparecimento** — medido idêntico para 250 e para 1.000.000.
3. **O desenho com sigilo permanente é também o mais barato.** Pedersen é 19%
   mais barato em CPU e 27% em taxa que o ElGamal que ele substituiu. Não houve
   trade-off a pagar.

E o orçamento fechou: **um voto completo custa 33.480.865 instruções, 8,4% de
uma transação, com folga de 11,9×.** Nenhuma linha desse orçamento é projeção.

E uma que responde à pergunta mais comum: **ZK cabe.** `pairing_check`
compartilha a exponenciação final entre os pares, então uma verificação Groth16
usa 11,8% de uma transação e caberiam 8 numa só. O gargalo de ZK na Stellar não
é o orçamento da cadeia.

### As sondas

| sonda | pergunta | veredito |
|---|---|---|
| 1 | As host functions BLS12-381 existem e a serialização bate? | sim |
| 2 | Quanto custa cada primitiva? | tabela acima |
| 3 | Um sigma-protocolo verifica dentro do contrato? | sim |
| 4 | A apuração homomórfica fecha ponta a ponta? | sim |
| 5 | Buscar o total é viável? | **não** — conferir, nunca procurar |
| 6 | Groth16 cabe no orçamento? | sim, 11,8% do teto |
| 7 | O desenho Pedersen fecha, e recusa mesa mentindo? | sim, e é mais barato |
| 8 | Por quanto tempo o verificador consegue ler o passado? | **7 dias** no RPC |
| 9 | O estado arquiva? | **TTL padrão de 7 dias**, teto de 180 |
| 10 | Quanto custa validar um ponto que chega? | 738.901, ou 1,1% da transação |
| 11 | `H` é um gerador honesto? | sim, e não é `±k·G` para k em 1..512 |
| 12 | A prova de boa formação cabe? | **sim, 10,98M — a projeção errava 2,5×** |
| 13 | Quanto custa conferir a aptidão? | 127.863 — a projeção errava **8×** |

Detalhes e método em [`bls-smoke/RESULTADOS.md`](bls-smoke/RESULTADOS.md).

---

## Duas descobertas que valem aviso

**As sondas 8 e 9 encontraram o mesmo número por caminhos diferentes.** A janela
de retenção do RPC público é de 120.960 ledgers; o TTL padrão de uma entrada
persistente é de 120.959. Ambos são **7 dias**.

Sem intervenção, no sétimo dia os dois níveis de verificação morrem juntos: o
RPC para de servir os eventos *e* o acumulador arquiva. Nada emite erro no
dia 1. As consequências de desenho estão em
[`docs/SPEC.md`](docs/SPEC.md) §5.1 e §8.3 — resumidas:

- o verificador lê dos **arquivos de histórico** por padrão, não do RPC;
- `abrir()` estende o TTL de `Proposta` e dos acumuladores ao teto da rede
  (~0,6 XLM fixo), e deixa as entradas `Votou` arquivarem.

**Para quem for reproduzir:** as invocações registradas aqui saem da janela do
RPC sete dias depois de feitas. Depois disso, leia dos arquivos de histórico.

---

## Reproduzir

```bash
cd bls-smoke && cargo test --lib -- --nocapture --test-threads=1   # 13 sondas
cd ../core   && cargo test                                          # 39 testes
```

Requer `rustc 1.97+`, `stellar-cli 25.2+`, alvo `wasm32v1-none`.
Cinquenta e dois testes, todos passando, sem rede.

O crate `core/` é a matemática compartilhada entre contrato, cliente e
verificador. Ele usa **arkworks, o mesmo crate do host do Soroban** — o que
elimina pela raiz a divergência entre o provador nativo e o verificador Wasm.
Três testes travam isso: o agregado Pedersen, a prova CDS e o caminho de
Merkle gerados no `core` reproduzem byte a byte o que o contrato aceitou na
testnet.

Os testes de custo asseram teto: uma regressão de custo quebra o build em vez
de aparecer na demo.

Contratos na testnet:

| | |
|---|---|
| sondas 1–13 (cripto, CDS e Merkle) | `CCL4CPAJ4ZVP25FP2PO7T3AMYGIVQZLYJZYZS53UAM5IFA5NFYGLUISR` |
| sonda 9 (TTL) | `CCCZ4HPW3ESHO6BMNFGXX6DJRSWPWA7FBQH2MS6QGHC3U67DIZ434KFC` |

Vetores de teste em [`bls-smoke/vetores.env`](bls-smoke/vetores.env).
Atenção: `Bls12381Fr` entra na `stellar contract invoke` como **decimal**, nunca
hex — e um hex que por acaso só tenha dígitos é aceito silenciosamente como o
decimal errado.

---

## Documentos

| | |
|---|---|
| [`docs/SPEC.md`](docs/SPEC.md) | a especificação do protocolo: modelo de ameaça, criptografia, interface, orçamento, desenhos rejeitados |
| [`docs/SMOKES.md`](docs/SMOKES.md) | o catálogo de smoke tests, com a árvore de decisão |
| [`docs/PLANO.md`](docs/PLANO.md) | cronograma até a submissão, com portões e cortes pré-decididos |
| [`docs/UX.md`](docs/UX.md) | a especificação de UX, e por que o valor deste produto é uma ausência |
| [`docs/UX-CLI.md`](docs/UX-CLI.md) | a saída exata da CLI, medida em colunas |

Decks: [português](index.html) · [inglês](index.en.html)

---

## O que este protocolo **não** garante

Declarar os limites faz parte do desenho. Um sistema de votação que promete
sigilo além do que entrega é pior que um honesto.

- **Anonimato do ato de votar.** O ledger registra *que* aquela conta votou.
  É intencional: a governança precisa de quórum.
- **Resistência à coação.** Quem vota conhece o próprio fator de aleatoriedade e
  *consegue* provar o voto a um terceiro. O protocolo remove o registro público
  permanente; não remove a capacidade de alguém se auto-incriminar. É a lacuna
  mais séria da v1.
- **Mesa que não destrói suas cópias.** O limiar `k`-de-`n` está implementado
  em `core/src/shamir.rs` e protege abaixo de `k` conluios: a mesa soma as
  shares localmente e nenhum `r` individual se junta em lugar algum. Acima de
  `k`, com as shares brutas guardadas, a proteção é procedimental.
- **Resultado unânime**, que revela todo mundo em qualquer sistema de votação.
- **Voto ponderado com pesos públicos distintos**, que é quebrado por
  subconjunto-soma. O contrato **recusa** essa configuração.

---

## Licença

MIT. Ver [LICENSE](LICENSE).
