# O retentor que não é humano

Estudo, não implementação. A pergunta que o originou: **por que não pode haver
placar automático com sigilo parcial?** A resposta foi que o sigilo durante a
votação exige que alguém esteja *retendo* a abertura, e o placar automático
exige que ninguém esteja. São a mesma frase negada — a menos que o retentor não
seja gente.

Este documento estuda o único retentor não-humano que existe pronto hoje:
**criptografia com fechadura de tempo** (*timelock*) sobre uma baliza de
aleatoriedade por limiar — a rede **drand**, esquema **tlock**.

Tudo o que está marcado *medido* foi conferido nesta máquina, com os valores
transcritos. O que não foi medido está marcado como tal.

---

## 1. O mecanismo, em uma frase

A drand é uma rede de operadores independentes que, a cada 3 segundos, assina
em conjunto o número da rodada corrente. A assinatura de uma rodada **futura**
ainda não existe — e vai existir, publicamente, no instante exato daquela
rodada. Dá para **cifrar para uma rodada futura**: a chave de decifragem é a
assinatura que ainda não saiu.

Quem retém, portanto, é o tempo. E quando ele solta, solta para todo mundo ao
mesmo tempo.

## 2. Os números da quicknet

| | |
|---|---|
| hash da cadeia | `52db9ba7…c84e971` |
| esquema | `bls-unchained-g1-rfc9380` |
| gênese | `1692803367` (23 ago 2023, 15:09:27 UTC) |
| período | 3 s |
| chave pública | 96 bytes — ponto **G2** comprimido (medido: byte alto `0x83`) |
| assinatura | 48 bytes — ponto **G1** comprimido |

A rodada de um instante `t` em segundos:

    rodada = (t − 1692803367) / 3 + 1

**Medido:** para `t = 1791421407` a fórmula dá `32872681`, e a rodada 32872681
vence exatamente em `t`. O relé publicava a 32872680 — a anterior. A fórmula é
exata, sem gambiarra de arredondamento.

E a curva é a **mesma do contrato**: BLS12-381. Não entra curva nova no projeto.

## 3. Duas medições que decidem o desenho

### 3.1 O `core` já alcança o pareamento — medido

`ark-bls12-381 0.5` com a feature `curve`, que já está no `core/Cargo.toml`,
entrega `Bls12_381::pairing` e `MapToCurveBasedHasher`/`WBMap` para G1.
Conferido com `e(xP, Q) == e(P, xQ)`. **Nenhuma dependência nova.**

### 3.2 A assinatura da drand confere com o que temos — medido

Vetor congelado, rodada 6.000.000 da quicknet:

    assinatura 848a0288a7102249bd6a274f65414ec8ca5b12c5e6f13a322e315ab734107784cd9b7ed4ebae73980cd72730ac6eb9f7

Com DST `BLS_SIG_BLS12381G1_XMD:SHA-256_SSWU_RO_NUL_`:

| mensagem assinada | `e(σ, g₂) == e(H₁(msg), P)` |
|---|---|
| `sha256(rodada_be8)` | **verdadeiro** |
| `rodada_be8` cru | falso |

Ou seja: a mensagem é o SHA-256 do número da rodada em 8 bytes big-endian. Isso
não estava claro em prosa nenhuma que li; saiu da medição.

### 3.3 O contrato **não pode** decifrar — e não precisa

A decifragem do tlock é IBE de Boneh–Franklin: precisa do **valor** de
`e(σ, U)` em G_T para derivar a máscara. O host do Soroban expõe
`pairing_check`, que devolve só um booleano — `soroban-sdk 28.0.0`, conferido
na fonte instalada: não existe função de pareamento com saída.

Isto podia ser o fim do estudo. Não é, por um motivo que já estava no projeto:
**o contrato nunca precisou entender a abertura, só recusá-la quando mente.**
O `apurar` confere `A_j = T_j·G + R_j·H` e recusa um total falso *na hora*
(INV-11). A fechadura de tempo não precisa de nenhuma cumplicidade do contrato:
ela só precisa tornar `R_j` indisponível antes de um instante e disponível
depois, **para qualquer pessoa**.

De passagem, o que o contrato *consegue* fazer, se quisermos: verificar a
assinatura da drand, com `hash_to_g1` e `pairing_check`, que ele já tem. Seria
um relógio criptográfico independente do `timestamp` do ledger. Custo não
medido.

---

## 4. O desenho

Hoje, em modo confidencial, cada votante reparte `r_{i,j}` em parcelas de
Shamir para a mesa, e a mesa reconstrói `R_j = Σᵢ r_{i,j}`. O transporte das
parcelas é o que não existe no dapp — é a §11-E.

Com fechadura de tempo:

1. No `abrir`, a janela de fechamento `fim` determina a rodada
   `N = (fim − gênese)/período + 1`. O contrato **confere a aritmética** e
   recusa divergência. Isso é inteiro puro, barato, e é o que impede o abridor
   de cifrar para uma rodada no passado (abre na hora) ou longe no futuro
   (nunca abre).
2. Ao votar, o cliente cifra cada `r_{i,j}` para a rodada `N` e publica o
   criptograma.
3. Passado `fim`, a assinatura da rodada `N` está pública. **Qualquer pessoa**
   busca a assinatura, decifra todos os criptogramas, soma, e chama `apurar`.
   O contrato confere contra o acumulado e aceita ou recusa.

Sem mesa. Sem Shamir — o limiar já está dentro da drand, que é uma rede de
limiar. Sem servidor. Sem ninguém designado: o primeiro que chamar publica o
placar, e quem não chamou lê.

### 4.1 Onde o criptograma mora

Duas opções, e a segunda é melhor:

- **Armazenamento do contrato.** Cresce por cédula, e uma entrada que cresce é
  exatamente o que produziu a §11-D. Não.
- **Evento.** O criptograma vai no evento que `votar_anonimo` já emite. Não
  toca entrada compartilhada nenhuma, então **não tem o problema da rajada**, e
  o custo de escrita não cresce com o eleitorado. O dapp já lê eventos para
  montar a lista de votações. A retenção do RPC público é de 120.960 ledgers —
  7 dias, medido —, maior que qualquer janela de demonstração.

Tamanho do criptograma, por opção confidencial: `U` ∈ G2 (96 B) + máscara
(32 B) + pad (32 B) = **160 bytes**. Com `U` em G1 em vez de G2 cairia para
112 B, ao custo de inverter os grupos em relação ao tlock publicado — não
medimos se vale.

### 4.2 Por que isto **não** é o que o slide 03 ataca

A objeção 1 da §11-E rejeitou "parcelas cifradas no ledger" porque registro
permanente mais cifra é *legível quando a chave vazar*. A objeção está certa —
e não se aplica aqui.

Na versão com mesa, a chave é um segredo humano com vida indefinida: pode
vazar, pode ser intimada, pode ser arrancada, para sempre. Na versão com
fechadura de tempo **não existe segredo durável**: a chave é publicada de
propósito num instante conhecido, e a vida útil do sigilo é, por construção,
até o fim da votação. Depois disso nada fica escondido que não fosse ser
revelado de qualquer forma — a saber, o **conteúdo** das cédulas, que a DEC-007
já declarou não ser o que se protege. O vínculo continua não existindo em lugar
nenhum para vazar.

O slide 03 ataca "cifrado para sempre, na esperança de que a chave nunca
escape". Isto é "cifrado até as 20:59". São coisas diferentes, e a segunda é a
resposta à primeira.

---

## 5. O que isto custa, dito inteiro

### 5.1 Um saboteador trava o placar

O contrato não tem como saber se o criptograma de alguém decifra para o `r` que
está no compromisso daquela pessoa. Quem postar lixo faz `Σr` sair errado, o
`apurar` recusa, e **não sai placar nenhum**.

A versão com mesa tem o mesmo buraco — parcelas de Shamir podem ser lixo do
mesmo jeito. Não é regressão. Mas "automático" aumenta a aposta, e eu não vou
chamar de automático uma coisa que uma pessoa sozinha desliga sem ser
identificada.

O conserto de verdade: guardar o compromisso **de cada cédula**, não só a soma.
Aí, depois de decifrar, qualquer um confere cédula por cédula, descarta as que
não fecham e apura o resto, nomeando as excluídas. É mudança de protocolo —
`Acum` deixa de ser a única coisa guardada —, não de cinco dias.

### 5.2 Uma suposição de confiança nova

Abrir antes da hora exige conluio de um limiar dos operadores da drand. É
gente, mas é gente que não tem interesse na votação e não foi escolhida por
quem abriu a urna. É mais fraco que "confie na sua mesa". **Não é "ninguém".**
Isso entra na §7 e limita o que a §2 pode afirmar.

### 5.3 Se a drand parar, não sai resultado

Nunca. E qualquer plano B que consiga abrir sem a drand consegue abrir **antes
da hora** — ou seja, destrói a fechadura. Então as composições honestas são só
duas:

- **2 de 2** (metade do `r` na fechadura, metade na mesa): ninguém abre cedo,
  mas qualquer uma das duas morrendo mata o resultado. Mais seguro, menos vivo.
- **1 de 2**: sai resultado se qualquer uma funcionar, e a mesa abre quando
  quiser. Mais vivo, e sem fechadura nenhuma na prática.

Não existe "fechadura com plano B". Ou se aceita que a drand é ponto único de
vivacidade, ou se aceita que a mesa pode abrir cedo.

### 5.4 Buscar a assinatura é transporte, não confiança

A assinatura vem de um relé HTTP. Isso parece reintroduzir servidor, e não
reintroduz: a assinatura **se autovalida** contra a chave pública da cadeia —
`e(σ, g₂) == e(H₁(sha256(rodada)), P)`, que é exatamente a medição de §3.2.
Um relé que minta é pego pelo cliente. É diferente de um servidor do Tessera,
que teria de ser *confiado* porque veria o `r`.

---

## 6. Dá para fazer até o prazo?

Prazo: 12 out 2026, 20:59. Faltam 5 dias, com T-004 (vídeo), T-005 (publicar) e
T-006 (decks) ainda abertos.

**Fatia A — o mecanismo, com medição.** `core/src/tlock.rs` com cifrar/decifrar,
o vetor congelado de §3.2 virando teste, e dois comandos na CLI: um que cifra
os `r` para a rodada do fechamento, outro que decifra depois dela. Não toca
contrato nem dapp. Estimativa: **~1 dia.** Entrega uma peça medida que o deck
pode afirmar com evidência.

**Fatia B — o dapp apura sozinho.** Um argumento `Bytes` em `votar_anonimo`, o
criptograma no evento, e no `apurar` a troca de "tem de ser da mesa" por "da
mesa, ou mesa vazia em modo relógio" — o resto da honestidade já é o
compromisso de Pedersen. Mais a decifragem exportada no `cliente-wasm` e o
fluxo de apuração na tela. Redeploy e rodada medida na testnet, como todas as
outras. Estimativa: **2 a 3 dias**, e carrega o buraco de §5.1.

A soma não cabe junto com vídeo, publicação e decks. A ordem que eu
recomendaria: **A agora**; o dapp da submissão segue com o que você já
escolheu; **B só se A sair rápido**. E o deck diz o que é verdade — o retentor
não-humano está construído e medido, e o dapp o liga depois do hackathon —, em
vez de prometer um placar automático que depende de três dias dando certo.

---

## 7. O que não dá para eu decidir

São todas da coluna "pergunte ao humano" do `CLAUDE.md`:

1. **Suposição de confiança nova** (§5.2): aceitar a drand como retentor.
2. **Vivacidade** (§5.3): fechadura só, 2 de 2, ou 1 de 2.
3. **Afirmação pública** (§2): o que o deck pode dizer, e quando.
4. **Escopo** (§6): fatia A só, ou A e B.

Registradas como §11-F na `docs/SPEC.md`.
