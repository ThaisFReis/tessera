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
| Contrato de urna (`abrir`/`votar`/`apurar`) | ✅ no ar, 20 testes |
| Cliente CLI | ✅ rodada completa na testnet |
| Verificador público | ✅ no `tessera verificar` |
| Console de demonstração | ✅ visor sobre a saída da CLI |

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
| **`votar()` completo, m=2** | **36.781.170** | |
| **`apurar()`, m=2** | **11.150.582** | |
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

E o orçamento fechou no contrato de verdade, não na soma das sondas: **um voto
confidencial custa 36.781.170 instruções, 9,2% de uma transação, com folga de
10,9×.** A soma das primitivas dava 33.480.865; os 9,8% a mais são autorização,
estado e evento.

Uma cédula **pública** custa 350.372 — **105× menos**. É o preço do sigilo,
medido.

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

## A rodada completa, na testnet

```bash
tessera abrir   --proposta contas --pergunta "Aprovar as contas de 2025?" \
                --opcoes aprovar,rejeitar --aptos marta,joao,ana,… \
                --mesa mesa1,…,mesa5 -k 3 --prazo 8m
tessera cedula  --proposta contas --identidade marta     # mostra, não vota
tessera votar   --proposta contas --opcao rejeitar --identidade marta
tessera queimar --identidade marta
tessera apurar  --proposta contas
tessera verificar --proposta contas
```

Custo real por voto, medido:

| | |
|---|---|
| `abrir` | 1,83 XLM |
| `votar` confidencial | **157.267 stroops ≈ 0,0157 XLM** |
| manter o contrato vivo 180 dias | 181,70 XLM, uma vez |

O `cedula` é o comando que não vota: mostra os dois compromissos possíveis
lado a lado, rotulados A e B, sem dizer qual é qual. Quem assiste tenta
descobrir, falha, e a falha é a demonstração.

---

## Três descobertas que valem aviso

**As sondas 8 e 9 encontraram o mesmo número por caminhos diferentes.** A janela
de retenção do RPC público é de 120.960 ledgers; o TTL padrão de uma entrada
persistente é de 120.959. Ambos são **7 dias**.

Sem intervenção, no sétimo dia os dois níveis de verificação morrem juntos: o
RPC para de servir os eventos *e* o acumulador arquiva. Nada emite erro no
dia 1. As consequências de desenho estão em
[`docs/SPEC.md`](docs/SPEC.md) §5.1 e §8.3 — resumidas:

- o verificador lê dos **arquivos de histórico** por padrão, não do RPC;
- `abrir()` estende o TTL de `Proposta` e dos acumuladores ao teto da rede
  (**1,73 XLM**, independente do comparecimento), e deixa as entradas `Votou`
  arquivarem;
- o aluguel do **próprio contrato** — 21 KB de Wasm por 180 dias — custa
  **181,70 XLM** e fica numa função `manter()` separada, que qualquer pessoa
  chama. Na primeira versão isso estava dentro de `abrir()`, e a medição na
  testnet mostrou a primeira chamada custando **182,39 XLM** contra **1,72** da
  segunda: a primeira governança a usar o módulo pagaria a conta de todas as
  outras.

**O ponto no infinito não é 96 bytes de zero.** O host recusa zeros com
"point not on curve". A codificação é o bit de flag do formato zcash — `0x40`
no byte alto, zeros no resto — e não está escrita em lugar nenhum que
tivéssemos encontrado; foi achada somando cada candidato ao gerador e vendo
qual devolvia o gerador.

Importa porque **o acumulador de uma proposta sem votos é o infinito**: errar
isso quebra a primeira cédula de toda votação, e só a primeira. Está travado
dos dois lados, em `core/src/ponto.rs` e no teste
`o_infinito_do_host_e_a_flag_zcash_nao_zeros`.

**A mesa `k`-de-`n` não co-assina: endossa.** A primeira versão pedia `k`
autorizações numa transação só, e isso **não é expressável** pelo ferramental
da Stellar — `stellar tx sign` assina o envelope, não as entradas de
autorização do Soroban, e a rede recusa com `TxBadAuthExtra`.

A correção ficou melhor que o desenho original. Cada membro manda **a sua**
transação e o contrato conta os endossos, que é como `k` pessoas em `k`
máquinas realmente trabalham. E cada endosso está preso ao
`sha256(totais ‖ aberturas)`: ninguém endossa "a apuração" em abstrato, endossa
**estes números**. Discordar é não endossar.

Como todo endosso confere tudo, uma apuração falsa é recusada no **primeiro**
membro que tentar, não no último.

**Para quem for reproduzir:** as invocações registradas aqui saem da janela do
RPC sete dias depois de feitas. Depois disso, leia dos arquivos de histórico.

---

## Reproduzir

```bash
cd bls-smoke && cargo test --lib -- --nocapture --test-threads=1   # 13 sondas
cd ../core   && cargo test                                          # 48 testes
cd ../contrato && cargo test                                        # 21 testes
cd ../cli    && cargo test                                          # 23 testes
```

Requer `rustc 1.97+`, `stellar-cli 25.2+`, alvo `wasm32v1-none`.
Cento e seis testes, todos passando, sem rede.

O crate `core/` é a matemática compartilhada entre contrato, cliente e
verificador. Ele usa **arkworks, o mesmo crate do host do Soroban** — o que
elimina pela raiz a divergência entre o provador nativo e o verificador Wasm.
E o cruzamento provador↔verificador não é um vetor congelado: **os testes do
contrato geram as provas com o `core`, em Rust nativo, e as verificam no host
do Soroban, em Wasm, a cada `cargo test`.** A rodada completa — seis pessoas
votando em sigilo, a mesa abrindo o agregado, o resultado saindo certo — roda
com criptografia de verdade do começo ao fim.

Os testes de custo asseram teto: uma regressão de custo quebra o build em vez
de aparecer na demo.

Contratos na testnet:

| | |
|---|---|
| **Tessera (cédula mista)** | `CBAHLZMTSP52CVPJGN6ATDVP4XACCX2PVMGBVIVYMR363JQRSOL7OOTG` |
| Tessera v1, cédula de uma pergunta | `CBLUSE2LCPPELS7GIQ5MRYKSW7KTILMWY7VBRFRSWAVG3L7TSRQYHP7H` |
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

Console: [`console/`](console/) — visor sobre uma rodada real, com a grade de
compromissos indistinguíveis que não cabe num terminal. É um **visor**, não um
aplicativo: sem carteira, sem servidor, e não assina nada.

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
