# Tessera

**Voto secreto para qualquer governança, na Stellar.**
Hackathon Find Your Way (Meridian) · trilha General

Tessera é um módulo de votação para contratos Soroban. Não é um aplicativo de
governança nem uma DAO: é um contrato que qualquer governança já existente pode
chamar para realizar uma votação em que

- **sabe-se quem compareceu**, e isso é público e auditável — é o que permite
  voto obrigatório, porque `aptos − compareceram` é a lista de quem faltou;
- **não se sabe em que cada pessoa votou**, e o compromisso publicado é
  perfeitamente ocultante: matematicamente vazio de informação, contra qualquer
  poder computacional, para sempre;
- **não se sabe de quem é cada cédula**, porque ela sai de uma chave de uso
  único com assinatura em anel — e nada no ledger liga as duas coisas;
- **qualquer pessoa pode recalcular o resultado** a partir do ledger e detectar
  uma mesa apuradora que minta.

A ideia que organiza o desenho: num registro permanente, "criptografado hoje"
significa "legível quando a chave vazar". Então o ledger **nunca recebe um texto
cifrado do voto**. Ele recebe um compromisso de Pedersen, `C = v·G + r·H`. O
voto em si transita fora da cadeia e é destruído.

**O princípio que organiza tudo é um só: não é possível ligar uma cédula a quem
a depositou.** Não é uma propriedade entre outras — é de onde o desenho inteiro
decorre. O anel existe para que a cédula não carregue remetente; a imagem de
chave, para impedir a segunda cédula sem revelar de quem é a primeira; a chave
de uso único, para que nem o pagamento da taxa ligue as duas pontas.

E é por isso que abrir uma cédula diz o que *aquela cédula* votou e nada mais.
Quem abre não ganha o vínculo, porque o vínculo não está guardado em lugar
nenhum para ser ganho.

A segunda ideia é o princípio do voto secreto, que resolve sem criptografia
nenhuma o problema de o remetente da cédula ser a identidade de quem vota:
**o caderno diz quem compareceu, a urna diz o que foi votado, e nada liga os
dois.** Tessera faz isso em dois atos — `comparecer()` identificado, e
`votar_anonimo()` de uma chave efêmera, com uma imagem de chave que impede a
segunda cédula da mesma pessoa sem revelar quem ela é.

---

## Estado, sem maquiagem

| | |
|---|---|
| Sondas criptográficas medidas na testnet | ✅ 13 sondas |
| Contrato de urna (`abrir`/`votar`/`apurar`) | ✅ no ar, 30 testes |
| Caderno e urna separados (`comparecer`/`votar_anonimo`) | ✅ no ar, rodada de 30 na testnet |
| Seções (anel por seção, resultado único) | ✅ no ar, 30 em 3 seções |
| Cliente CLI | ✅ rodada completa na testnet |
| Verificador público | ✅ no `tessera verificar` |
| dapp (React, cripto no navegador) | ✅ roda local, **não publicado** |
| Apuração pelo dapp | ⬜ só pela CLI — as parcelas vivem no navegador de cada membro |
| Console de demonstração | ✅ visor sobre a saída da CLI |

Todo número abaixo veio de uma invocação real na testnet ou de um teste que roda
no host do Soroban, nunca de projeção. O que é projeção está marcado como
projeção.

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

## O anel, as seções, e o teto que some

Esconder a escolha não basta quando o remetente da cédula é o endereço de quem
vota. O anel resolve isso: a cédula sai de uma chave de uso único, com uma
assinatura que prova que quem assinou está entre os que compareceram **sem
dizer qual deles**. Uma imagem de chave `I = x·Hp` colide quando a mesma pessoa
tenta votar duas vezes, e o contrato recusa com `ImagemJaUsada` sem saber de
quem é.

O anel é a prova disjuntiva de `core/src/cds.rs` generalizada de 2 para `n`
ramos — mesmo sigma-protocolo, mesmo Fiat–Shamir. **Sem SNARK, sem cerimônia de
setup, sem circuito.**

Verificar custa **10.822.850 instruções por membro**, linear. Medido com
`votar_anonimo()` inteiro, não a soma das primitivas:

| no anel | instruções | do teto de 400M |
|---|---|---|
| 5 | 94.517.040 | 23,6% |
| 10 | 148.747.642 | 37,2% |
| 20 | 257.211.151 | 64,3% |
| 30 | 365.680.568 | 91,4% |

A reta é `40,3 M fixos + 10,85 M por pessoa`, então a parede aritmética fica em
32. Mas na testnet um anel de 30 mostrou outro teto antes desse: **só uma cédula
dessas entra por ledger**, e 18 de 30 confirmaram em 646 s enquanto o resto
expirou.

Daí as **seções**. O eleitorado é dividido, cada seção tem o seu anel, e o custo
por cédula para de depender do tamanho da votação. Trinta pessoas em três seções
de dez, na testnet:

```
caderno   30/30 aceitas · 30 transações · 25 no mesmo ledger
urna      30/30 aceitas · 3 por ledger · zero recusas
cédula    148.889.608 instruções · 37,2% do teto
```

**O resultado continua único.** O acumulador é por proposta e não sabe de que
seção veio cada cédula. Publicar totais por seção abriria a brecha da seção
unânime, que entrega todo mundo que caiu nela; aqui não é preciso.

O que se paga é o conjunto de anonimato, que passa a ser a seção.

### Quem divide as seções

Ninguém escolhe. Se o organizador escolhesse, poria um dissidente numa seção
sozinho — anel de um, voto ligado à pessoa, sem precisar de conluio. A ordem vem
de `H(0x03 ‖ proposta ‖ endereço)` e as seções saem em rodízio sobre ela, o que
as deixa do mesmo tamanho a menos de um e deixa qualquer pessoa com a lista
recalcular e conferir.

E a seção entra **na folha de Merkle** — `H(0x00 ‖ endereço ‖ peso ‖ seção)` —
senão seria argumento da chamada e quem vota escolheria a sua.

O limite que sobra está declarado: quem abre ainda pode moer o identificador da
proposta atrás de um sorteio que lhe agrade. Com blocos de tamanho igual isso
não produz uma seção de um, que é o ataque que importa.

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

No modo anel a CLI **abre** a votação e não vota nela — o voto em anel é do
dapp, que é onde a chave de anel pode viver no navegador de quem vota:

```bash
tessera abrir --proposta assembleia --anel --secoes 3 \
              --pergunta "Aprovar?" --opcoes aprovar,rejeitar \
              --aptos … --mesa mesa1 -k 1 --inicio 5m --prazo 30m
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
[`docs/PROTOCOLO.md`](docs/PROTOCOLO.md) §5.1 e §8.3 — resumidas:

- o verificador lê dos **arquivos de histórico** por padrão, não do RPC;
- `abrir()` estende o TTL de `Proposta` e dos acumuladores ao teto da rede
  (**1,73 XLM**, independente do comparecimento), e deixa as entradas `Votou`
  arquivarem;
- o aluguel do **próprio contrato** — 29 KB de Wasm por 180 dias — custa
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
cd bls-smoke   && cargo test --lib -- --nocapture --test-threads=1  # 13 sondas
cd ../core     && cargo test --lib                                  # 68 testes
cd ../contrato && cargo test                                        # 30 testes
cd ../cli      && cargo test                                        # 25 testes
cd ../app      && for t in scripts/*.test.mjs; do node "$t"; done    # 3 testes
```

Requer `rustc 1.97.1`, `stellar-cli 25.2.0`, alvo `wasm32v1-none`, Node 22+
(os testes do app usam a remoção de tipos nativa do Node).
**Cento e trinta e seis testes em Rust e três em JavaScript**, todos passando,
sem rede.

O dapp precisa do cliente wasm antes de rodar:

```bash
cd app && npm run wasm && npm run dev     # http://localhost:5273
```

`npm run wasm` reconstrói e **relinka** o pacote: o `pnpm` liga os arquivos por
hard-link e o `wasm-pack` reescreve o `.wasm` como arquivo novo, então sem o
relink a cola nova chama o binário velho e o sintoma é
`table index is out of bounds`.

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
| **Tessera (cédula mista)** | `CAZVUPKVXCV6CB2V2LC4OY5FHU3HG5OVIDMSQB4Z7VST36XEZMIDWILH` |
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
| [`docs/PROTOCOLO.md`](docs/PROTOCOLO.md) | a especificação do protocolo: modelo de ameaça, criptografia, interface, orçamento, desenhos rejeitados |
| [`docs/SMOKES.md`](docs/SMOKES.md) | o catálogo de smoke tests, com a árvore de decisão |
| [`docs/PLANO.md`](docs/PLANO.md) | cronograma até a submissão, com portões e cortes pré-decididos |
| [`docs/UX.md`](docs/UX.md) | a especificação de UX, e por que o valor deste produto é uma ausência |
| [`docs/UX-CLI.md`](docs/UX-CLI.md) | a saída exata da CLI, medida em colunas |
| [`docs/design-votacao.md`](docs/design-votacao.md) | o desenho do dapp, os bastidores e o guarda do diário |
| [`docs/SPEC.md`](docs/SPEC.md) | a especificação de trabalho: invariantes, portões, quadro de tarefas, decisões |
| [`docs/SOURCES.md`](docs/SOURCES.md) | todo fato externo conferido, com data |
| [`CLAUDE.md`](CLAUDE.md) | como se trabalha neste repositório |

Decks: [português](index.html) · [inglês](index.en.html)

Console: [`console/`](console/) — visor sobre uma rodada real, com a grade de
compromissos indistinguíveis que não cabe num terminal. É um **visor**, não um
aplicativo: sem carteira, sem servidor, e não assina nada.

dapp: [`app/`](app/) — React + TypeScript, estático, com `tessera-core`
compilado para WebAssembly. As provas nascem na aba de quem vota, então **não
existe servidor que pudesse ver o fator de aleatoriedade, porque não existe
servidor**. A rota `/bastidores` mostra, numa segunda janela, o que o contrato
responde a cada ato — inclusive o custo em instruções que a simulação descobriu.
Ainda não está publicado em lugar nenhum.

---

## O que este protocolo **não** garante

Declarar os limites faz parte do desenho. Um sistema de votação que promete
sigilo além do que entrega é pior que um honesto.

- **Duas garantias de naturezas diferentes, e elas não são a mesma coisa.** O
  compromisso de Pedersen é **perfeitamente** ocultante: não há suposição a
  quebrar, nem com computador quântico. O anonimato do anel **não** é — ele
  repousa sobre um problema que se acredita difícil. Achatar as duas numa só
  frase venderia o que não existe.
- **Anonimato do comparecimento.** O caderno registra *que* aquela conta
  compareceu. É intencional: é disso que sai a lista de quem faltou, e sem ela
  não existe voto obrigatório. O que o ledger não mostra é qual cédula é dela.
- **A lista de aptos fica pública no modo anel.** Um anel só é verificável por
  quem tem as chaves de todos os ramos: anonimato de anel é anonimato *dentro de
  um conjunto conhecido*. É coerente com o voto secreto, onde o eleitorado e a
  lista de presença são públicos e só o vínculo é secreto — mas é uma troca, e
  ela não existia no desenho identificado.
- **Um anel de um não esconde ninguém.** O sigilo é propriedade do grupo, não da
  matemática sozinha. Com seções o contrato recusa anel abaixo de `TAU`; sem
  seções ele avisa e deixa passar, porque ali ninguém escolheu o grupo.
- **Uma votação em anel não apura.** Ela não reparte com a mesa o fator que
  esconde o voto, então ninguém reconstrói a abertura — e ninguém publica total.
  Não é promessa, é o contrato: qualquer total afirmado cai em
  `AberturaNaoFecha`. O sigilo é absoluto e o resultado é impossível.
- **Resistência à coação, enquanto você vota.** Quem estiver olhando a sua tela
  vê a sua escolha, e nenhum protocolo conserta isso. O que o Tessera garante é
  o **depois**: o fator `r` que esconde o voto nasce na sua aba e morre com ela,
  nenhum cliente o grava, e nenhum comando lê um arquivo desses.

  A v1 gravava um recibo com os `r_j` e pedia que a pessoa o apagasse. Era pior
  que inútil: quem coage simplesmente manda não apagar, e nenhum comando lia o
  arquivo — ele não servia para conferir voto nem para apurar. Hoje `votar` não
  grava, e `queimar` existe só para limpar sobras de rodadas antigas,
  sobrescrevendo antes de remover. Dois testes travam isso, e um deles lê o
  próprio fonte de `comandos.rs`, porque ausência não se prova de outro jeito.

  **O que continua em aberto:** um cliente adulterado pode guardar o `r` sem
  você saber. Rodar o seu é a defesa, e é por isso que o dapp é estático e o
  `core` é o mesmo crate em todos os caminhos.
- **O conteúdo de uma cédula aberta.** O que o protocolo protege é o *vínculo*,
  não o conteúdo: quem reúne a abertura descobre o que aquela cédula votou, e
  nunca de quem ela é. Numa votação sem mesa ninguém reúne nada.
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
