# Tessera — Catálogo de smoke tests

**Data:** 2026-10-01 (dia 1 do [PLANO.md](PLANO.md))
**Pergunta que este doc responde:** o que ainda não sabemos, e qual decisão cada resposta destrava.

Um smoke test aqui não é teste de regressão. É **uma pergunta que, respondida errado, muda a arquitetura.** Se a resposta não muda nenhuma decisão, não é smoke: é teste, e vai para a suíte normal.

Cada entrada tem: a pergunta, o método, o critério, **a decisão que destrava**, o custo em tempo e o que acontece se falhar.

---

## 0. A lacuna que está escondida em plena vista

As seis sondas em `../bls-smoke/` parecem cobrir tudo. Elas não cobrem o produto.

> **A sonda mediu ElGamal exponencial. O spec especifica Pedersen.**
> A apuração por abertura do agregado — que é o desenho final, a razão do sigilo
> permanente e o conteúdo dos dois decks — **nunca foi executada uma vez.**

Isso não invalida nenhuma medição: as primitivas são as mesmas (`g1_add`, `g1_msm`), e o modelo de custo transfere. Mas "o modelo de custo transfere" é uma afirmação, e ninguém a testou.

**É o smoke B1, e é o mais importante da lista.** Quarenta minutos. Se passar, o projeto inteiro está de pé. Se falhar, existe um plano B *já medido* (ElGamal, §3.3 deste doc).

E há um segundo grupo que ninguém registrou: **três riscos de plataforma que quebram a verificabilidade em silêncio, depois da demo.** Grupo A. Falhar hoje é barato; descobrir em frente ao jurado, não.

---

## 1. O que já está respondido

Para não re-medir. Tudo abaixo veio de invocação real na testnet, 2026-09-30.

| # | pergunta | resposta |
|---|---|---|
| S1 | Host functions BLS12-381 existem no SDK 28? | sim |
| S1 | Host aceita `be_bytes(X) ‖ be_bytes(Y)`, 96 B? | sim |
| S2 | Custo das primitivas? | add 110.748 · mul 3.290.524 · msm 2.454.719 + 1.471.917/termo · hash_to_g1 2.653.011 |
| S3 | Sigma-protocolo verifica dentro do contrato? | sim, 6.737.614 |
| S4 | Apuração homomórfica fecha ponta a ponta? | sim (ElGamal, pesos 5+3+1 → 9) |
| S5 | Buscar o total é viável? | **não**, ~124k/unidade, teto ~3.218. Conferir: 6.712.183 constante |
| S5 | Contrato recusa total falso? | sim |
| S6 | Groth16 cabe? | sim, 47.371.348 = 11,8% do teto · `pairing_check(k) = 10.571.128 + 6.746.479k` |
| — | Teto de CPU real? | 400.000.000, por bissecção |
| — | Taxas? | 9.558 / 11.852 stroops |

---

## 2. Grupo A — Plataforma: os que quebram em silêncio

Estes são os perigosos. Não falham no `cargo test`. Falham semanas depois, ou na frente de quem está avaliando, e quebram justamente **P4 e P5** — a verificabilidade, que é a metade do pitch que não é sigilo.

### A1 · Arquivamento de estado apaga a urna?

**Pergunta.** Entradas persistentes do Soroban têm TTL e são **arquivadas** quando ele expira. Uma transação cuja footprint referencia uma entrada persistente arquivada **falha** até alguém restaurar com `RestoreFootprintOp`. Os acumuladores `Acum(id, j)` e as 50 entradas `Votou(id, addr)` são persistentes. Então: uma votação aberta hoje e apurada em duas semanas ainda funciona? E o verificador, seis meses depois?

**Método.** Abrir uma proposta na testnet, escrever as entradas, ler o TTL de cada uma (`extend_ttl` / consulta de TTL), e calcular quantos ledgers faltam para arquivar. Testar o caminho de restauração.

**Critério.** O TTL de toda entrada que a apuração e o verificador precisam é conhecido, e existe uma política explícita de extensão.

**Decisão que destrava.** Se o TTL padrão for menor que um ciclo de governança plausível, `abrir()` precisa **estender o TTL de tudo na abertura**, e isso tem custo e entra no orçamento de §7 do spec. Se não, a cláusula "qualquer pessoa recalcula a apuração a partir da rede" tem prazo de validade — e aí o deck está prometendo mais do que o protocolo entrega, exatamente o pecado que o §1.3 proíbe.

**Tempo.** 45 min · **RESULTADO 2026-10-01: TTL padrão 7,00 dias, teto 180 dias, 0,207 XLM por entrada estendida.** O TTL padrão coincide com a janela do RPC: sem intervenção os dois níveis do verificador morrem no dia 7. `abrir()` estende só `Proposta` e `Acum` (~0,6 XLM fixo); `Votou` pode arquivar. Ver SPEC §5.1.

### A2 · O verificador consegue ler o passado?

**Pergunta.** O verificador público relê todos os `votar()` da proposta e reconstrói o agregado. Ele depende do histórico do RPC. **A janela de retenção de transações e eventos do RPC é uma configuração, expressa em número de ledgers** — não é infinita.

Se a janela for curta, o verificador funciona no dia da votação e para de funcionar depois. P5 vira decorativa, que é precisamente o que [UX-CLI.md](UX-CLI.md) §7 diz que não pode acontecer.

**Método.** `getEvents` / `getTransactions` no RPC público da testnet buscando o ledger mais antigo disponível. Medir a janela real em ledgers e em dias.

**Critério.** A janela é conhecida e escrita no README.

**Decisão que destrava.** Três caminhos, e a medição escolhe:
1. Janela confortável → verificador lê do RPC, como planejado.
2. Janela curta → o verificador precisa **indexar continuamente** ou tirar um snapshot assinado. É trabalho novo, e muda o escopo de sábado.
3. Alternativa → ler dos *arquivos de histórico* em vez do RPC, que é mais lento e mais correto.

**Tempo.** 30 min · **RESULTADO 2026-10-01: a janela é 120.960 ledgers = 7,00 dias, e é aplicada.** Não afeta leitura de estado nem arquivos de histórico. O verificador passa a ter dois níveis (SPEC §8.3) e lê dos arquivos por padrão. **Era mesmo a falha mais provável, e aconteceu.**

### A3 · A transação de voto cabe, em bytes e em footprint?

**Pergunta.** Todo o orçamento de §7 do spec é de **CPU**. Nunca olhamos tamanho. Uma chamada `votar()` leva, para `m=2`: 2 compromissos (192 B) + 2 provas CDS (640 B) + 1 prova de soma (128 B) + caminho de Merkle profundidade 8 (256 B) + assinatura e envelope. E a footprint declara `Proposta`, `Acum(j)` para cada `j`, `Votou`, `GeradorH` — e **cada entrada tocada conta contra os limites de leitura e escrita por transação.**

**Método.** Montar a transação com dados sintéticos do tamanho real, submeter na testnet, ler tamanho e contagem de footprint contra os limites vigentes.

**Critério.** Cabe com folga ≥ 2× nos limites de tamanho, leitura e escrita.

**Decisão que destrava.** Se não couber: menos opções por transação, ou pontos comprimidos (48 B em vez de 96 — mas só se o host desserializar comprimido, que é um smoke por si), ou caminho de Merkle mais curto.

**Tempo.** 45 min · **✅ MEDIDO NA TESTNET 2026-10-01**, na maior transação que o projeto já mandou — a cédula mista da proposta `assembleia-2026` (contrato `CBAHLZMT…OOTG`).

| cédula | envelope | tx |
|---|---:|---|
| mista: 1 pública + 2 sigilosas, **5 opções sigilosas** | **8.360 B** | `9c7372b4…` |
| aberta por inteiro (revelação voluntária) | **2.320 B** | `d82e27b3…` |

Ambas aceitas pela rede, `status=SUCCESS`.

**A aritmética da carga, que é exata e não estimada:** cada opção sigilosa custa
`96 B` de compromisso + `320 B` de disjuntiva = **416 B**, e cada pergunta
sigilosa custa mais `128 B` de prova de soma. A cédula medida levou
`5×416 + 2×128 = 2.336 B` de prova; o resto dos 8.360 é caminho de Merkle,
endereços, entrada de autorização, footprint e assinatura.

**O pior caso do contrato** — 16 opções sigilosas, o teto de `MAX_OPCOES`,
distribuídas em 8 perguntas — leva `16×416 + 8×128 = 7.680 B` de prova, ou seja
**+5.344 B** sobre a cédula medida. Projeta um envelope de ~14 KB. *É projeção,
não medição: não enviei a cédula máxima.*

**Ressalva honesta:** não consegui confirmar nas fontes o teto declarado de
tamanho de transação da rede, então **não afirmo uma folga em múltiplos**. O
que está medido é que 8.360 B passam, e que o pior caso do contrato é da ordem
de 14 KB — uma transação Soroban grande, mas nada perto de um limite de
centenas de KB. Fechar o número do teto fica como pendência menor.

### A4 · Aluguel de estado para `n` grande

Pendência §9.3 do spec. 50 entradas `Votou` na demo; 10.000 numa assembleia real.

**Método.** Escrever 1.000 entradas e ler a taxa total.

**Critério.** Um número, e a extrapolação para 10.000 escrita no spec.

**Decisão que destrava.** Se o aluguel for alto, o desenho de armazenamento muda (bitmap em vez de uma entrada por votante — mas bitmap colide com a v2 desvinculada, então é decisão de arquitetura, não de otimização).

**Tempo.** 30 min · **Se falhar:** só o spec muda. Não bloqueia a demo de 50 votantes.

---

## 3. Grupo B — Criptografia: o desenho final nunca rodou

### B1 · Pedersen e a abertura do agregado fecham on-chain? ★

**O smoke mais importante do projeto.**

**Pergunta.** Com `C_i = v_i·G + r_i·H`, a soma `A = Σ C_i` abre como `T·G + R·H` com `T = Σv_i` e `R = Σr_i`? E o contrato confere isso com um MSM de 2 termos, aceitando `(T, R)` da mesa?

**Método.** Função nova na sonda: `verify_aggregate(A, T, R, G, H) -> bool`. Montar 5 compromissos no teste, somar, abrir, conferir. Testar também com `T+1` e com `R+1`.

**Critério.**
- honesto → `true`
- `T` alterado em 1 → `false`
- `R` alterado em 1 → `false`
- custo ≤ 8M (projetado 5,4M)

**Decisão que destrava.** Tudo. É o desenho do spec, da §3.3, do slide `solution` dos dois decks, e a razão pela qual o projeto afirma sigilo permanente.

**Se falhar:** plano B já medido — voltar a ElGamal exponencial com `tally_checked` (6.712.183, sonda 5, funcionando na testnet hoje). **Custo da queda: perde-se o sigilo permanente como propriedade entregue**, e os dois decks precisam de edição real, não cosmética. Por isso este smoke é o primeiro de hoje.

**Tempo.** 40 min · **RESULTADO 2026-10-01: PASSA.** 5.408.931 e 8.622 stroops, 19% e 27% mais barato que ElGamal. Projeção errou 0,2%. Negativos recusam. Plano B arquivado sem uso.

### B2 · `H` é um gerador com log discreto desconhecido?

**Pergunta.** `H = hash_to_g1(DST_H)` está na curva, no subgrupo de ordem `r`, e não é `G` nem múltiplo trivial dele?

**Método.** `g1_is_on_curve(H)`, `g1_is_in_subgroup(H)`, `H != G`, `H != -G`, `H != identidade`. E determinismo: duas derivações dão o mesmo ponto.

**Critério.** Todas verdadeiras, e `H` idêntico entre duas execuções e entre cliente e contrato.

**Decisão que destrava.** Se `H` não for determinístico entre o provador off-chain e o contrato, **nenhuma prova verifica** e o bug é dos que custam meio dia para achar. Trinta minutos aqui economizam isso.

**Tempo.** 20 min · **RESULTADO 2026-10-01: PASSA.** On-curve, in-subgroup, `≠ ±k·G` para k em 1..512, DST participa, e o vetor da testnet está travado no teste.

### B3 · O provador fora da cadeia e o verificador dentro concordam?

**Pergunta.** Até agora o provador viveu dentro do teste. No produto, a CLI gera a prova em Rust nativo e o contrato a verifica em Wasm. Mesma aritmética? Mesmo Fiat–Shamir? Mesma serialização?

**Método.** CLI gera `(C, π)` e grava em arquivo. `stellar contract invoke` passa o arquivo. O contrato devolve `true`.

**Critério.** `true` numa invocação real na testnet, com a prova vinda de um processo separado.

**Decisão que destrava.** É o ponto exato em que a maioria dos projetos de cripto quebra, e a causa é quase sempre uma destas três: ordem de bytes, DST divergente, ou redução módulo `r`. A armadilha do `Fr::from_bytes` que não reduz (§10.3 do spec) já mordeu uma vez.

**Tempo.** 1h · **✅ RESOLVIDO POR CONSTRUÇÃO 2026-10-01.** O caminho Pedersen está provado byte a byte (`core/src/pedersen.rs`, teste `agregado_bate_byte_a_byte_com_a_testnet`), e a raiz do risco foi removida usando **o mesmo crate do host** — o Soroban usa arkworks, e o `core` também.

O cruzamento que faltava para a CDS deixou de ser um smoke e virou estrutura: `tessera-core` é `dev-dependency` do contrato, então **toda** a bateria de testes gera as provas em Rust nativo e as verifica no host Soroban em Wasm. O cruzamento provador↔verificador acontece a cada `cargo test`, não uma vez num vetor congelado.

### B4 · CDS disjunctiva: custo e corretude

Pendência §9.1, o maior número projetado do orçamento (~27M).

**Método.** Implementar `verify_cds` e medir. Testar que aceita `v=0` e `v=1`, e **recusa** `v=2`, `v=-1`, ramos trocados e desafio adulterado.

**Critério.** Os quatro negativos recusam. Custo medido entra no Portão 1.

**Decisão que destrava.** O Portão 1 do plano, direto: ≤200M segue, 200–350M corta opções, >350M troca o sistema de prova.

**Tempo.** 2h · **RESULTADO 2026-10-01: PASSA, e a projeção errou por 2,5×.** 10.980.243 instruções, 14.144 stroops. `votar()` fecha em 33.353.002, 8,3% do teto, folga de 12×. Corretude: aceita `v∈{0,1}`, recusa prova forjada para `v=2`, ramos trocados, `e0`/`z0` adulterados e prova migrada de compromisso.

### B5 · Shamir sobre `Fr` soma como precisa somar?

**Pergunta.** A mesa `k`-de-`N` depende de o compartilhamento de Shamir ser **aditivamente homomórfico nas shares**: cada membro soma localmente, e `k` membros reconstroem `Σr_i` sem nunca reconstruir um `r_i`. Isso funciona sobre `Fr` com a aritmética disponível? Interpolação de Lagrange sobre `Fr` com `fr_inv`?

**Método.** Em Rust puro: dividir 5 valores `r_i` em 5 shares com limiar 3; somar as shares por membro; reconstruir com 3 membros; conferir que dá `Σr_i`. E conferir que 2 membros **não** reconstroem nada útil.

**Critério.** Reconstrução com 3 confere. Com 2, não.

**Decisão que destrava.** É o smoke de **P2** — sigilo contra a própria mesa. Sem ele, o corte 2 do plano escala: a mesa vira endereço único, que *vê todos os votos*, e isso precisa ir para o §1.3 do spec como lacuna declarada e para o slide `properties`.

**Tempo.** 1h · **RESULTADO 2026-10-01: PASSA.** `core/src/shamir.rs`, 7 testes. Cinco votantes dividem `r_i` em 5 shares com limiar 3; cada membro soma localmente; quatro trios distintos reconstroem `Σr_i`, e nenhum `r_i` individual jamais se junta. Dois membros não reconstroem — e o teste prova a forma forte: dois pontos são consistentes com **todo** segredo, então não excluem nenhum. **P2 está de pé e o corte 2 do plano não precisa escalar.**

### B6 · Aleatoriedade do cliente

**Pergunta.** `r_i` vem de um CSPRNG de verdade?

**Método.** Ler o código. Confirmar `getrandom` / `OsRng`, nunca `rand::thread_rng` semeado de forma determinística, nunca timestamp. Gerar 10.000 `r` e checar que não repetem e passam um teste de uniformidade grosseiro.

**Critério.** Fonte do SO, zero colisões.

**Decisão que destrava.** Nenhuma — mas é o **único ponto do sistema em que um bug de implementação anula uma garantia information-theoretic.** `r` previsível torna o compromisso abrível por qualquer um. O sigilo permanente inteiro, provado em §3.2 do spec, repousa sobre esta linha de código.

**Tempo.** 20 min · **RESULTADO 2026-10-01: PASSA**, e achou uma armadilha. Zerar o byte alto serve para um desafio de Fiat-Shamir e **não serve** para `r`: cobriria <1% de Fr e quebraria a ocultação perfeita. `core/src/acaso.rs` usa rejeição, não redução. Quatro testes escritos antes do cliente que eles guardam.

---

## 4. Grupo C — Orçamento restante

### C1 · Desserialização de ponto com checagem de subgrupo

Pendência §9.2. São ~6 pontos por `votar()` e o custo nunca foi isolado.

**Método.** `deserialize_n(pontos: Vec<G1>, n: u32)` que só valida; isolar por diferença.

**Critério.** Um número, e o orçamento de §7.2 do spec fechado com ✅.

**Decisão que destrava.** Entra no Portão 1. E resolve o aviso do Tyler: *não venda Groth16 no pitch sem medir desserialização com checagem de subgrupo.*

**Tempo.** 30 min · **RESULTADO 2026-10-01: PASSA.** 738.901 por ponto (on-curve 4.367 + subgrupo 734.877). Seis pontos = 1,1% da transação. Groth16 com validação vai a 12,4% do teto. O host não valida sozinho: `g1_add` é 15% de uma checagem, então validação é aditiva.

### C2 · `verify_reveal` — o campo público — ✅ DISPENSADO 2026-10-01

**O smoke deixou de existir porque o desenho que o exigia foi descartado.**

A pendência §9.4 supunha que uma pergunta pública publicaria o mesmo compromisso `C` mais o par `(v, r)` em claro, conferido com um MSM de 2 termos (~5,4M projetados). A implementação é mais simples: **uma pergunta pública não tem compromisso nenhum.** A resposta vai como `u32` e o contrato confere duas regras a olho — cada opção em `{0,1}` e a soma igual ao peso.

Não há abertura a verificar, porque não há compromisso. O `verify_reveal` nunca precisou ser escrito.

**A afirmação do deck fica — e com folga maior do que a projetada.** Medido no contrato: a cédula pública inteira custa **350.372**, contra **13.501.500 por opção sigilosa**. Não é 5× mais barato: é duas ordens de grandeza.

### C3 · Caminho de Merkle e `require_auth` de `k` membros

**Método.** Medir o caminho de profundidade 8 com sha256, e o `require_auth` multiassinatura de 3 de 5.

**Critério.** Dois números no orçamento.

**Tempo.** 40 min · **RESULTADO 2026-10-01 (metade do Merkle): PASSA.** Sonda 13. Caminho de profundidade 8 custa **127.863** instruções, **5.235 stroops** — a projeção do SPEC era ~1M e errava por 8×, para cima. Por nível: 13.128. Profundidade 20 (um milhão de aptos) custaria 285.399, 0,07% da transação. O vetor foi montado no `core` e fechou na testnet, o que estende o B3 ao caminho de Merkle. Negativos recusados na testnet: peso inflado de 1 para 1000, índice trocado, irmãos reordenados, caminho curto, intruso. **Falta medir o `require_auth` 3-de-5.**

---

## 5. Grupo D — Ponta a ponta

### D1 · A rodada completa na testnet

O roteiro do vídeo é o smoke. Cinco votantes, um divergente, apuração 3-de-5, verificador fechando.

**Critério.** Os seis passos de [UX-CLI.md](UX-CLI.md), em sequência, sem intervenção manual entre eles.

**Decisão que destrava.** O **Portão 3** do plano, sábado 20:00. Falha aqui aciona o fallback nuclear.

**Tempo.** 3h incluindo correções · **RESULTADO 2026-10-01: PASSA.** Sete pessoas numa votação na testnet: cinco em segredo, duas em público, uma divergente. Contrato `CBLUSE2L…HP7H`. Custo real por voto confidencial: **157.267 stroops**, 0,0157 XLM. Público: 118.448.

**O que o smoke encontrou, e que nenhum teste de unidade encontraria:**

1. **Multi-auth do Soroban numa transação só não é expressável pela `stellar` CLI.** `stellar tx sign` assina o envelope, não as entradas de autorização, e a rede recusa com `TxBadAuthExtra`. `apurar()` foi redesenhado para **um endosso por membro, uma transação por pessoa**, com cada endosso preso ao `sha256(totais ‖ aberturas)`. O desenho novo é melhor: é como `k` pessoas em `k` máquinas trabalham, e ninguém endossa "a apuração" em abstrato.
2. **Cada voto confidencial leva ~30 s de relógio** pela `stellar` CLI. Uma votação de 7 pessoas precisa de janela de 8 min, não de 3. Importa para o roteiro do vídeo.

### D2 · Mesa mentindo recusa de verdade

**Método.** `--forcar-total` com total inválido.

**Critério.** Contrato devolve falso, nada é publicado, e o estado fica inalterado — não parcialmente escrito.

**Decisão que destrava.** É o passo 6 do vídeo e o que separa verificabilidade real de afirmada. **Não é cortável.**

**Tempo.** 20 min · **✅ RESOLVIDO NA TESTNET 2026-10-01.** `--forcar-total 4,1` numa rodada em que os totais reais eram outros: a CLI recusou antes de enviar, com a frase "Não é denúncia depois. É recusa na hora.", e o contrato recusa pelo `AberturaNaoFecha` se alguém pular a CLI. Coberto também em teste de contrato (`mesa_que_mente_no_total_e_recusada`).

### D3 · A recusa por `τ`

**Método.** Votação com 4 confidenciais e 46 públicos. Tentar apurar.

**Critério.** Recusa, nada publicado, nenhum voto perdido, e a votação volta a ser apurável se mais gente votar em segredo.

**Decisão que destrava.** É o teorema da partição (§6.6) executando. Quinze segundos de vídeo, e o estado que mais impressiona quem entende de votação.

**Tempo.** 30 min · **✅ RESOLVIDO NA TESTNET 2026-10-01.** Proposta `coligacao-2026` no contrato `CBAHLZMT…OOTG`: 7 aptos, **4 votaram em segredo e 3 abriram o voto** — a coligação de §6.6 encolhendo o conjunto secreto de propósito. Depois do prazo, `apurar` foi recusada com `HostError: Error(Contract, #19)` = `AnonimatoInsuficiente`.

O que faz a cena funcionar é que **os números conferiam**: a mesa reconstruiu a abertura, achou `MANTER 4 · RESCINDIR 0`, e os dois acumuladores fecharam com `✓`. O contrato tinha o resultado correto em mãos e **recusou publicá-lo**. Não é falha de cálculo; é a troca de §6.6 — uma quebra de privacidade virando falha de liveness, ao vivo.

### D4 · O verificador é realmente independente?

**Pergunta.** Ele roda sem o contrato, sem a CLI, sem nenhum arquivo local de estado?

**Método.** Em diretório limpo, com só o id da proposta e o endereço do contrato, rodar o verificador. Sem `./estado/`, sem `./recibos/`.

**Critério.** Reconstrói tudo e fecha.

**Decisão que destrava.** Se ele precisar de arquivo local, **não é verificador, é visualizador** — e o `Confia em ....... nada` do cabeçalho é falso. Melhor descobrir antes de escrever a linha.

**Tempo.** 30 min.

---

## 6. Grupo E — Confiança (os meus)

Smokes de UX não medem usabilidade. Medem se a confiança se formou.

### E1 · O teste das duas perguntas

Uma pessoa de fora vota na demo. Depois:

1. *"Quem consegue descobrir em que você votou?"*
   **Passa:** "ninguém". **Falha:** "não sei", "a plataforma", "depende".
2. *"Seu voto entrou na conta?"*
   **Passa:** "sim, vi a posição 27". **Falha:** "acho que sim".

**Decisão que destrava.** Se a 1 falhar, o bloco de hex não está convencendo e nada mais na tela importa — e a correção é de cópia e de hierarquia, não de código. Trinta minutos de conserto, se descoberto na sexta. Zero, se descoberto no domingo.

**Tempo.** 20 min, uma pessoa.

### E2 · A cédula sobrevive a `| cat`

**Método.** `tessera cedula … | cat` e `NO_COLOR=1 tessera cedula …`.

**Critério.** Os glyphs e as palavras carregam tudo; nada de essencial depende de cor.

**Decisão que destrava.** Vídeo comprimido come saturação, e o terminal de quem avalia não é o seu.

**Tempo.** 5 min · **✅ RESOLVIDO 2026-10-01.** Zero escapes ANSI através de `| cat`, e `tela::tem_cor()` respeita `NO_COLOR`. Travado em teste (`cli/src/tela.rs`, `sem_cor_nao_emite_escape`).

### E3 · Setenta e duas colunas em tela cheia

**Método.** Gravar 10 segundos a 1920×1080 e assistir no celular.

**Critério.** O hex é legível.

**Decisão que destrava.** A largura é escolha de demo, não de estilo. Se não der, a régua muda antes de os sete comandos existirem — depois são sete reescritas.

**Tempo.** 10 min.

---

## 7. Árvore de decisão

```
B1 Pedersen fecha?
├── não → volta para ElGamal (medido). Sigilo permanente sai
│         como propriedade entregue. Editar os 2 decks e o spec.
└── sim → segue
    │
    A2 janela do RPC comporta o verificador?
    ├── não → verificador ganha modo indexador. +3h no sábado.
    └── sim → segue
        │
        B4 + C1: soma de votar() com números reais
        ├── > 350M → troca o sistema de prova, ou 1 opção por tx
        ├── 200–350M → m=2 fixo, Merkle 8
        └── ≤ 200M → segue o plano
            │
            A1 TTL arquiva antes de um ciclo de governança?
            ├── sim → abrir() estende TTL. Custo no orçamento.
            └── não → segue
                │
                B5 Shamir soma?
                ├── não → mesa única, lacuna declarada no deck
                └── sim → P2 entregue
                    │
                    D1 rodada completa fecha? → PORTÃO 3
                    ├── não → fallback nuclear (PLANO §5)
                    └── sim → gravar vídeo
```

---

## 8. O que rodar hoje

O plano reservou 08:30–12:00. Não cabem vinte smokes. Cabem **quatro**, e são os quatro que podem mudar a arquitetura:

| ordem | smoke | tempo | por que hoje |
|---|---|---|---|
| 1 | **B1** Pedersen fecha | 40 min | o desenho final nunca rodou |
| 2 | **A2** janela do RPC | 30 min | falha mais provável da lista |
| 3 | **B2** `H` determinístico | 20 min | evita meio dia de bug na sexta |
| 4 | **C1** desserialização | 30 min | fecha o Portão 1 |

Sobra ~1h de folga, que some sozinha. Se sobrar de verdade: **B6** (aleatoriedade, 20 min) — é o único smoke cuja falha é invisível.

**Sexta de manhã, antes do contrato:** B4 (CDS, 2h, é implementação) · B3 (provador ↔ verificador, 1h).
**Sexta à tarde:** A3 (tamanho de transação) · C2 · C3.
**Sábado:** A1, A4, B5, D1, D2, D3, D4.
**Domingo de manhã:** E1, E2, E3 — antes de gravar, não depois.

Se o tempo apertar, corte nesta ordem: **A4** (só muda o spec) → **C3** (dá para estimar) → **B5** (vira lacuna declarada). **B1, A2, B4, D1, D2 e D3 não são cortáveis.** Os três últimos são o vídeo; os três primeiros são a arquitetura.

---

## 9. Regra de registro

Todo smoke escreve uma linha em `../bls-smoke/RESULTADOS.md`: a pergunta, o número, o veredito e a data. Um smoke sem resultado registrado é um smoke que vai ser refeito.

E dois números não podem viver em dois lugares (o deck já teve esse problema): **`RESULTADOS.md` é a fonte.** Spec, plano e decks citam, nunca republicam.
