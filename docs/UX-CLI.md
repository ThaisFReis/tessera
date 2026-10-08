# Tessera — CLI, especificação de saída

**Autor:** Kaan (UX Designer) · **Data:** 2026-10-01
**Isto é o Nível 1 de [UX.md](UX.md) §7:** a interface obrigatória, a que sustenta a demo sozinha.
**Para transcrever hoje.** Cada bloco abaixo é a saída literal, já medida em colunas.

---

## 0. Regras que valem para toda a saída

**Largura fixa de 72 colunas.** Não é estética: é a demo. Num vídeo 1920×1080, 72 colunas ficam legíveis em tela cheia sem forçar fonte pequena. A 100 colunas o jurado não lê o hex, e o hex é o produto.

**Nada de caracteres de moldura** (`┌─┐│`). Quebram em fonte errada e envelhecem a tela. Use só `─` como régua, largura 72.

**Alinhamento por pontos.** O projeto já escreve assim no `RESULTADOS.md`; a CLI herda a mesma voz.

```
  Rótulo ............. valor
```

Rótulo + pontos ocupam 21 colunas, o valor começa na 22.

**Status nunca é só cor.** Sempre glyph + palavra: `✓ confere`, `✗ recusada`, `⚠ atenção`. O spec já exige isso, e um vídeo comprimido come saturação.

**Paleta, igual à do deck.** Truecolor, com degradação limpa.

| papel | hex | ANSI |
|---|---|---|
| acento, verificado | `#C8F24F` | `\033[38;2;200;242;79m` |
| texto | `#ECEFE6` | `\033[38;2;236;239;230m` |
| secundário | `#9AA093` | `\033[38;2;154;160;147m` |
| apagado | `#6B7065` | `\033[38;2;107;112;101m` |
| recusa | `#FF6B4A` | `\033[38;2;255;107;74m` |
| reset | — | `\033[0m` |

Respeite `NO_COLOR` e saída não-tty: sem cor, os glyphs e as palavras carregam tudo. Teste com `tessera votar … | cat` antes de gravar.

**O verde marca o que foi verificado, nunca o que foi escolhido.** Regra do UX.md §8, e ela importa mais no terminal, onde cor é o único recurso visual que existe.

**O bloco de hex é sempre 48 caracteres por linha, 4 linhas.** 96 bytes viram um retângulo sólido. Largura e altura fixas, sempre, em todo comando: é a assinatura visual do produto.

---

## 1. `tessera abrir` — o operador

Duas formas. A **curta** é a cédula de uma pergunta, sigilosa:

```
$ tessera abrir --proposta contas-2025 \
    --pergunta "Aprovar as contas da diretoria de 2025?" \
    --opcoes aprovar,rejeitar \
    --aptos aptos.csv --mesa mesa.csv -k 3 --prazo 2h
```

A **longa** repete `--pergunta "texto | opções | natureza"`, e é onde mora a
cédula mista (PROTOCOLO §6.5):

```
$ tessera abrir --proposta assembleia-2026 \
    --pergunta "Aprovar as contas de 2025? | aprovar, rejeitar | publica" \
    --pergunta "Destituir a diretoria? | sim, nao | sigilosa" \
    --pergunta "Eleger a cadeira 3 | ana, bruno, carla | sigilosa" \
    --aptos aptos.csv --mesa mesa.csv -k 3 --prazo 2h
```

A natureza é `sigilosa` ou `publica`, e **cai em `sigilosa` quando omitida**: o
sigilo é o padrão, e abrir uma pergunta tem de ser um ato deliberado de quem
escreve a cédula, nunca um esquecimento.

```
  TESSERA · abrindo votação
  ────────────────────────────────────────────────────────────────────

  1. Aprovar as contas de 2025?
    em aberto ........ aprovar · rejeitar
  2. Destituir a diretoria?
    em sigilo ........ sim · nao
  3. Eleger a cadeira 3
    em sigilo ........ ana · bruno · carla

  Aptos .............. 50  (raiz de Merkle 7c3a…b219)
  Peso ............... um voto por pessoa
  Mesa ............... 5 membros, 3 assinaturas para apurar
  Sigilo mínimo ...... 5 votos confidenciais
  Encerra ............ em 2h00  (ledger 1.291.400)

  ── publicado ───────────────────────────────────────────────────────

  Contrato ........... CA2LFMOO…TRTP
  Transação .......... 8d1f44a2       ledger 1.284.498
  Taxa ............... 13.204 stroops

  ✓ votação aberta

  Compartilhe com quem vota:
    tessera cedula --proposta contas-2025 --identidade SEU_NOME
```

**Decisões.** As perguntas vêm antes de qualquer parâmetro, porque são as únicas linhas que um humano na sala precisa ler. Cada uma diz `em sigilo` ou `em aberto` na própria linha: numa cédula mista, a natureza de cada pergunta é informação de primeira classe, não nota de rodapé. O `Sigilo mínimo` aparece aqui, na abertura, para que a regra `τ` seja uma configuração declarada e não uma surpresa na apuração. E a última linha é o próximo comando, pronto para copiar — toda tela termina dizendo o que fazer em seguida.

**O limite que a CLI recusa na hora.** O orçamento de CPU é limitado pelas opções **sigilosas** somadas (13.501.500 instruções cada; 16 já são 56% do teto de uma transação). Passar disso é recusado no `abrir`, com a conta na tela e duas saídas — abrir alguma pergunta ou dividir a cédula. As opções públicas não entram nessa conta: a cédula pública inteira custa 350.372.

---

### 1.1 A janela: `--inicio` e `--prazo`

```
tessera abrir --proposta assembleia --pergunta "..." \
    --aptos aptos.csv --mesa mesa.csv -k 3 --inicio 1h --prazo 2h
```

`--inicio` diz **quando a votação abre**, contado a partir de agora, e
`--prazo` **quanto ela dura depois disso**. Os dois aceitam `2h`, `30m`, `45s`
ou um número de ledgers.

O padrão de `--inicio` é `0`: abre no instante em que a transação entra no
ledger. Era o único comportamento até a v1.0; agendar é a adição, não o caso
comum.

A tela de abertura mostra os dois lados:

```
  Abre ............... em 1min  (ledger 4984758)
  Encerra ............ em 6min  (ledger 4984818)
```

E `tessera cedula` troca a frase conforme o lado da janela em que se está —
`Abre em 15s · você é apta` antes, `Encerra em 4min · você é apta` depois.
Sem isso a pessoa tenta votar cedo e leva uma recusa que parece defeito.

**As duas recusas são distintas de propósito:**

| | |
|---|---|
| `Error(Contract, #26)` | `VotacaoAindaNaoComecou` — chegou cedo |
| `VotacaoEncerrada` | chegou tarde |

Um erro só para os dois lados economizaria um discriminante e custaria a única
informação que importa para quem está na frente da urna.

Abertura e fechamento são **ledgers, não relógio de parede**. Um ledger da
testnet leva ~5 s, e é o que o contrato consegue verificar sozinho.

## 2. `tessera cedula` — onde o sigilo fica visível

**Este é o comando que ganha a demo.** Ele não vota. Ele mostra.

```
$ tessera cedula --proposta contas-2025 --identidade marta
```

```
  TESSERA · cédula
  ────────────────────────────────────────────────────────────────────

  Aprovar as contas da diretoria de 2025?

  Encerra em 1h47 · você é apta · ainda não votou

  ── o que a rede guardaria, para sempre ─────────────────────────────

  Se você votar  A                 Se você votar  B

  0f1ff9fcda6e556ba2da8fb40898    05ca03df795a67362944df075354
  b251409d204c79b6bd364f55abba    5960a8a4f9c2f5a2874850448462
  e01df25324d04d96e14e796236af    074213df5f5a595ff0f226d9eece
  63e89f1d73990b4960881cd7f3ef    5dd88639f930179936253 15eee3

  96 bytes cada. Um é "aprovar" e o outro é "rejeitar".

  Não existe cálculo, computador ou tempo que diga qual é qual.
  Nem hoje, nem em cinquenta anos. É o que você está publicando.

  ── seu sigilo ──────────────────────────────────────────────────────

  43 de 50 estão em segredo. Mínimo é 5.  ✓ folgado

  ────────────────────────────────────────────────────────────────────

  Para votar:
    tessera votar --proposta contas-2025 --opcao rejeitar --identidade marta
```

### 2.1 Por que lado a lado, e não "troque e veja mudar"

No navegador, a pessoa troca a escolha e vê o hex mudar — a percepção vem do movimento. No terminal não há movimento, mas há algo melhor: **os dois ao mesmo tempo, sem dizer qual é qual.**

O espectador tenta descobrir. Falha. E a falha é a demonstração.

Rotular `A` e `B` em vez de "aprovar" e "rejeitar" é deliberado: assim que você nomeia, a pessoa para de olhar os bytes e começa a ler o rótulo. Anônimo, ela olha os bytes.

**Os dois blocos são exemplos com acaso descartável** — o voto real sorteia um `r` novo. Não escreva isso na tela principal; ponha em `--verboso`. A tela tem um trabalho só, e nota de pé de página rouba dele.

### 2.2 A frase que não pode ser suavizada

> *Não existe cálculo, computador ou tempo que diga qual é qual.*

É a única vez em todo o produto em que superlativo é literalmente correto, porque a garantia é information-theoretic e não computacional. Em qualquer outro produto isso seria marketing. Aqui é a especificação. **Não escreva "praticamente impossível" nem "computacionalmente seguro"** — hedge aqui é impreciso *e* mais fraco. Ver UX.md §5.3.

---

## 3. `tessera votar`

Uma `--opcao` por pergunta, **na ordem da cédula**:

```
$ tessera votar --proposta assembleia-2026 \
    --opcao aprovar --opcao sim --opcao ana --identidade marta
```

Faltar uma é **recusa**, com a cédula inteira impressa — nunca abstenção
silenciosa. Abster-se de uma pergunta precisaria da própria prova (PROTOCOLO §6.5), e
isso é v1.1; até lá, deixar de responder não pode ser um acidente de digitação.

```
  TESSERA · votando
  ────────────────────────────────────────────────────────────────────

  1. Aprovar as contas de 2025?  APROVAR   [em claro]
  2. Destituir a diretoria?  SIM   [em segredo]
  3. Eleger a cadeira 3  ANA   [em segredo]

  Sigilo ............. misto — as perguntas sigilosas em segredo,
                       as outras em claro

  ── o que a rede vai guardar ────────────────────────────────────────

  0f1ff9fcda6e556ba2da8fb40898b251409d204c79b6bd36
  4f55abbae01df25324d04d96e14e796236af63e89f1d7399
  0b4960881cd7f3ef2e2c36a0d374d56b4d1dcf6a6e689075
  e9eba9c317707f328e53297f21181edda1d71001742a223a

  ── enviado ─────────────────────────────────────────────────────────

  Transação .......... a4f29e1c       ledger 1.284.551
  Taxa ............... 11.980 stroops
  Posição ............ 27 de 43 em segredo

  ✓ seu voto está na contagem

  ────────────────────────────────────────────────────────────────────

  ⚠  A CHAVE EM ./recibos/marta.key PROVA O SEU VOTO

     Enquanto ela existir, você consegue provar a qualquer pessoa
     em que votou — e quem te obrigar a mostrar consegue conferir.

     A rede nunca vai saber. Mas você pode ser forçada a contar.

     Apague agora:
       tessera queimar --identidade marta
```

### 3.1 Decisões

**A escolha aparece em caixa alta uma vez, no topo, e nunca mais.** Depois disso a tela fala de compromisso e posição. A escolha não se repete porque repetir a escolha na tela é ensaiar o hábito de deixá-la visível.

**Cada linha diz `[em claro]` ou `[em segredo]`, e isso não é decoração.** Numa cédula mista a pessoa precisa saber o que vai para o ledger ao lado do endereço dela **antes** de enviar, não descobrir depois num explorador de blocos. A `cedula` já tinha avisado; `votar` repete na hora de confirmar, porque é a última tela antes do irreversível.

**Não existe `--publico`, e a ausência é o recurso.** Houve uma versão em que o eleitor podia abrir a própria cédula inteira no ledger. Parecia um direito e era uma alavanca: quem coage confere sozinho, lendo o ledger, sem a pessoa na frente — coação verificável em escala. Uma escolha que o coator pode exigir não é liberdade.

Perguntas públicas continuam existindo e são outra coisa: valem para todos e são decididas pelo estatuto em `abrir` (PROTOCOLO §6.5).

**`Posição 27 de 43 em segredo` é a verificabilidade individual virando uma frase que Dona Marta entende.** Não é "seu commitment está no acumulador". É "você é a 27ª de 43".

**O último bloco é a garantia, não uma tarefa.** A última coisa na tela é a que fica na memória, e por isso ela era o aviso de coação com o comando para apagar o recibo. Agora `votar` não grava recibo nenhum, então o lugar é da garantia:

```
  ✓ seu sigilo está protegido

     Nenhum arquivo nesta máquina prova em que você votou.
     O acaso que escondeu o seu voto morreu com este comando —
     nem você consegue mais demonstrar a sua escolha.
```

Pedir uma faxina depois do voto era transferir para a pessoa um risco que o
desenho podia simplesmente não criar.

**O aviso não é vermelho.** Vermelho é reservado para a apuração recusada (UX.md §8). Aqui é `⚠` em texto normal com indentação, porque não é erro: é a verdade sobre o estado do mundo.

---

## 4. `tessera queimar` — vestigial, de propósito

```
$ tessera queimar --identidade marta
```

**Este comando quase não tem mais o que fazer.** Ele existia porque `votar`
gravava `./recibos/<identidade>.key` com o acaso `r`, e queimar era a faxina que
a pessoa precisava lembrar de fazer.

Isso tinha dois defeitos. Quem coage simplesmente manda não queimar — e o
arquivo não servia para mais nada: nenhum comando o lia, nem para conferir voto,
nem para apurar. Era passivo puro.

Agora `votar` não grava. `queimar` fica para os arquivos que rodadas antigas
deixaram, e a tela diz isso:

```
  TESSERA · queimando o recibo
  ──────────────────────────────────────────────────────────────────

  Não há recibo em ./recibos/marta.key — nada a queimar.
```

Quando há arquivo, ele é **sobrescrito** antes de removido: apagar o inode não
basta se o conteúdo continua no disco.

## 5. `tessera status` — dois números grandes

Para o operador com a sala cheia.

```
$ tessera status --proposta contas-2025
```

```
  TESSERA · contas-2025                        encerra em 1h12
  ────────────────────────────────────────────────────────────────────

              50 de 50 votaram            43 em segredo
                                           7 públicos

  Sigilo mínimo ...... 5          ✓ pode apurar
  Mesa ............... 3 de 5 assinaturas prontas

  ────────────────────────────────────────────────────────────────────

  Apurar:
    tessera apurar --proposta contas-2025 --shares ./shares/
```

Dois números, grandes, centrados, e o resto pequeno. É a única tela do produto que alguém olha de longe.

---

## 6. `tessera apurar`

> **Mudou na implementação.** A mesa não co-assina uma transação: **endossa, uma transação por membro.** `k` autorizações do Soroban numa transação só não é expressável pela `stellar` CLI — `tx sign` assina o envelope, não as entradas de autorização, e a rede recusa com `TxBadAuthExtra`. A tela ganhou um bloco:
>
> ```
>   ── a mesa endossa ───────────────────────────────────────────────────
>
>   mesa1 .............. endossou  (1 de 3)
>   mesa2 .............. endossou  (2 de 3)
>   mesa3 .............. endossou  (3 de 3)
> ```
>
> E ficou melhor de mostrar do que o original: o quórum se formando na tela, um nome por linha, é a arquitetura da mesa acontecendo à vista. Cada endosso está preso ao `sha256(totais ‖ aberturas)`, então ninguém endossa "a apuração" em abstrato.

```
$ tessera apurar --proposta contas-2025 --shares ./shares/
```

```
  TESSERA · apurando
  ────────────────────────────────────────────────────────────────────

  Mesa ............... 3 de 5 reconstruíram a abertura agregada
  Sigilo ............. 43 confidenciais, mínimo 5   ✓

  ── a mesa afirmou ──────────────────────────────────────────────────

  APROVAR ............ 19
  REJEITAR ........... 31

  ── o contrato conferiu ─────────────────────────────────────────────

  Acumulado APROVAR ..  19·G + R·H      ✓ fecha
  Acumulado REJEITAR .  31·G + R·H      ✓ fecha

  Transação .......... f71b0c83       ledger 1.291.402
  Taxa ............... 12.117 stroops

  ────────────────────────────────────────────────────────────────────

                   REJEITADO · 31 a 19

  Conferir por conta própria:
    tessera verificar --proposta contas-2025
```

**`a mesa afirmou` → `o contrato conferiu` → o resultado.** Essa ordem é a arquitetura de confiança do protocolo impressa de cima para baixo. Lida em sequência, ela ensina sozinha por que não é preciso confiar na mesa, sem uma linha de explicação.

**O resultado vem depois da conferência e é a única linha centrada.** A ordem comunica que o número só vale porque passou pelo bloco acima.

### 6.1 Numa cédula mista

Três mudanças, todas pela mesma razão: **as duas naturezas não podem se
confundir num número solto.**

**Os rótulos ganham o número da pergunta** quando há mais de uma pergunta
sigilosa — `2·SIM`, `3·CARLA`. Duas perguntas podem ter uma opção `sim`, e
"SIM: 4" sem contexto não é informação.

**`a mesa afirmou` distingue a origem**, linha a linha:

```
  APROVAR ............ 6 em público — esta pergunta é aberta
  2·SIM .............. 2 em segredo + 0 em público
  2·NAO .............. 3 em segredo + 2 em público
```

Uma pergunta aberta não tem "em segredo" para mostrar, e fingir que tem seria
inventar um sigilo que não existe.

**`o contrato conferiu` só lista as perguntas sigilosas.** Não há acumulador
para conferir numa pergunta pública — ela já estava em claro no ledger desde o
voto. Listar uma linha `✓` ali sugeriria uma verificação criptográfica que não
aconteceu.

**E há um placar por pergunta**, cada um precedido pelo texto da pergunta e
pela natureza dela:

```
  1. Aprovar as contas de 2025?  ·  em aberto
                   APROVAR · 6 a 1

  2. Destituir a diretoria?  ·  em sigilo
                      NAO · 5 a 2
```

---

## 7. `tessera verificar` — a tela do cético, e do jurado

Processo separado. Não fala com o contrato: **relê o ledger e refaz tudo.**

```
$ tessera verificar --proposta contas-2025
```

```
  TESSERA · verificador independente
  ────────────────────────────────────────────────────────────────────

  Fonte .............. ledger da testnet, RPC público
  Confia em .......... nada

  Lendo 1.284.498 → 1.291.402 ........................... 50 votos

  Aptidão ............ 50 de 50 na raiz 7c3a…b219        ✓
  Unicidade .......... 0 repetidos                        ✓
  Boa formação ....... 43 provas CDS válidas              ✓
  Aberturas públicas . 7 de 7 conferem                    ✓
  Sigilo mínimo ...... 43 confidenciais ≥ 5               ✓

  Agregado refeito ... APROVAR   19·G + R·H               ✓
                       REJEITAR  31·G + R·H               ✓

  ────────────────────────────────────────────────────────────────────

  ✓ o resultado publicado é o resultado correto

  E o que este verificador NÃO conseguiu descobrir:
    em que cada uma das 43 pessoas votou.
```

### 7.1 As duas linhas que fazem esta tela funcionar

**`Confia em ....... nada`** no cabeçalho. Três palavras que dizem o que o verificador é. Um jurado lê isso e entende o modelo de confiança inteiro antes da primeira checagem.

**O bloco final é a jogada.** Depois de sete `✓`, o verificador declara o próprio limite:

> *E o que este verificador NÃO conseguiu descobrir: em que cada uma das 43 pessoas votou.*

Isso resolve o problema de design do UX.md §1 — o valor é uma ausência — transformando a ausência em **saída afirmativa de uma ferramenta hostil**. Não é o produto prometendo sigilo. É um auditor que tentou tudo, provou tudo o que podia provar, e relata o que não conseguiu.

É a frase mais persuasiva do produto inteiro, e ela só funciona vinda daqui, do processo que não confia em ninguém.

---

## 8. Os estados de falha, que são metade da demo

### 8.1 Mesa mentindo — o passo 6 do roteiro

```
$ tessera apurar --proposta contas-2025 --forcar-total 20,30
```

```
  ── a mesa afirmou ──────────────────────────────────────────────────

  APROVAR ............ 20
  REJEITAR ........... 30

  ── o contrato conferiu ─────────────────────────────────────────────

  Acumulado APROVAR ..  20·G + R·H      ✗ NÃO FECHA
  Acumulado REJEITAR .  30·G + R·H      ✗ NÃO FECHA

  ✗ apuração recusada · nada foi publicado

  O total afirmado não corresponde aos compromissos no ledger.
  Para publicar 20 a mesa teria de resolver um log discreto
  em BLS12-381.

  Não é denúncia depois. É recusa na hora.
```

**Único lugar do produto onde vermelho aparece.** Por isso ele significa algo.

A última linha é a tese da verificabilidade em oito palavras, e é a frase que eu levaria para o vídeo.

### 8.2 Sigilo abaixo do mínimo — o teorema da partição, ao vivo

```
  Sigilo ............. 4 confidenciais, mínimo 5          ✗

  ✗ apuração recusada · nada foi publicado

  Com apenas 4 votos em segredo, publicar o total revelaria
  esses votos por subtração: quem publicou é conhecido, e o
  resto sai por diferença.

  A apuração volta a ser possível se mais pessoas votarem em
  segredo. Nenhum voto foi perdido.
```

**Este é o estado que mais impressiona quem entende de votação**, porque é o produto recusando um resultado que *poderia* publicar, para proteger quatro pessoas. Vale 15 segundos de vídeo.

A última frase é obrigatória: sem "nenhum voto foi perdido", a recusa parece perda de dados.

**Aconteceu na testnet em 2026-10-01** (smoke D3), na proposta `coligacao-2026`: 4 em segredo, 3 abriram o voto. O que faz a cena funcionar é que a tela mostra, logo acima da recusa, **os dois acumuladores fechando com `✓`** — a mesa reconstruiu a abertura e achou `MANTER 4 · RESCINDIR 0`. O resultado estava correto e na mão. O contrato recusou com `Error(Contract, #19)`.

Essa ordem na tela não é acidente: conferir **antes** de recusar é o que transforma "deu erro" em "escolheu não publicar".

### 8.3 Voto duplo

```
  ✗ você já votou nesta proposta

  Voto registrado no ledger 1.284.551, posição 27.
  Uma pessoa apta vota uma vez. Isso é conferível por qualquer um.
```

Sem tom de reprimenda. O erro mais comum é a pessoa não ter certeza se o voto entrou — então a mensagem de erro responde essa pergunta em vez de repreender.

### 8.4 Não apta

```
  ✗ esta conta não está na lista de aptos

  Raiz de aptos ...... 7c3a…b219  (data de corte: ledger 1.280.000)

  Se você deveria estar, fale com quem abriu a votação. Tessera
  não decide quem vota.
```

A última frase não é desculpa, é arquitetura: o módulo não opina sobre aptidão (PROTOCOLO §0). Dizer isso no erro ensina o modelo a quem está integrando.

---

## 9. O JSON — o contrato entre o Nível 1 e o Nível 2

**Escreva isto desde o primeiro commit, hoje.** Se a CLI já emitir este arquivo, o console HTML de domingo (UX.md §7, Nível 2) é uma função pura dele: 4 horas de HTML, zero acoplamento, e cortável sem consequência.

Todo comando escreve `./estado/<proposta>.json`:

```json
{
  "proposta": "assembleia-2026",
  "perguntas": [
    { "texto": "Aprovar as contas de 2025?",
      "opcoes": ["aprovar", "rejeitar"], "confidencial": false },
    { "texto": "Destituir a diretoria?",
      "opcoes": ["sim", "nao"], "confidencial": true },
    { "texto": "Eleger a cadeira 3",
      "opcoes": ["ana", "bruno", "carla"], "confidencial": true }
  ],
  "contrato": "CCQHRQZP3R3QMMKEEOOOS7XXINR7WGSMTKGNR4GD6BDNLC6JSPUZIDHZ",
  "raiz_aptos": "7c3a...b219",
  "aptos": ["GB4V...", "GAO2..."],
  "sigilo_minimo": 5,
  "mesa": { "membros": 5, "limiar": 3 },
  "prazo_ledger": 4971432,
  "votos": [
    {
      "posicao": 1,
      "compromissos": ["0f1ff9fc...", "05ca03df...", "...", "...", "..."],
      "provas": ["...", "...", "...", "...", "..."],
      "provas_soma": ["...", "..."],
      "publico": false,
      "escolhas": [1, 0],
      "tx": "a4f29e1c",
      "ledger": 4971074
    }
  ],
  "aberturas": ["3301999...", "2419847..."],
  "apuracao": {
    "afirmado": [2, 3, 2, 2, 1],
    "confere": true,
    "tx": "f71b0c83",
    "ledger": 4971440
  },
  "verificacao": {
    "aptidao": true, "unicidade": true, "boa_formacao": true,
    "aberturas": true, "sigilo_minimo": true, "agregado": true
  }
}
```

**Três regras de achatamento, e errar qualquer uma quebra o console em
silêncio:**

| campo | cobre | ordem |
|---|---|---|
| `compromissos`, `provas` | só as perguntas **sigilosas** | pergunta, depois opção |
| `provas_soma` | só as perguntas **sigilosas** | uma **por pergunta** |
| `aberturas`, `apuracao.afirmado` | só as opções **sigilosas** | pergunta, depois opção |
| `votos[].escolhas` | depende de `publico` | ver abaixo |

`escolhas` cobre as perguntas **públicas** da cédula, na ordem. Ele teve duas
leituras enquanto `--publico` existia — numa cédula aberta cobria a cédula
inteira — e quem lia precisava olhar `publico` antes de indexar. Removida
aquela função, a leitura é uma só. O campo `publico` continua no JSON para que
estados de rodadas antigas ainda abram.

**Nada de `r` neste arquivo, e em nenhum outro.** Não existe mais o recibo onde
ele morava: `votar` não escreve em disco.

**E o contrário também vale, e só ficou claro na implementação: as provas são públicas e por isso moram no estado, não no recibo.** Na primeira versão elas estavam no `.key`, e `queimar` apagava junto a capacidade de qualquer pessoa reverificar a boa formação daquela cédula — proteger quem vota passava a custar auditabilidade. Não custa. O recibo guarda **só o segredo**; o que já está no ledger fica no estado.

A separação é a regra: `recibos/` tem o que ninguém mais pode ter, `estado/` tem o que todo mundo já tem.

---

## 10. Checklist para hoje

Transcreva nesta ordem. Os três primeiros já sustentam o vídeo inteiro.

- [ ] Largura 72, régua `─`, pontos em 21 colunas, hex em 48×4
- [ ] Paleta truecolor + `NO_COLOR` + teste com `| cat`
- [ ] `cedula` com os dois blocos lado a lado, rotulados A e B
- [ ] `votar` com o aviso de coação como último e maior bloco
- [ ] `queimar` com "Agora nem você consegue provar em que votou"
- [ ] `verificar` com `Confia em ... nada` e o bloco final do que não descobriu
- [ ] `--forcar-total` para gravar a recusa
- [ ] A recusa por `τ` com "Nenhum voto foi perdido"
- [ ] Toda tela termina com o próximo comando, pronto para copiar
- [ ] `./estado/<proposta>.json` escrito por todos os comandos

Se o tempo apertar, corte nesta ordem: `status` primeiro (o operador vive sem), depois os erros 8.3 e 8.4. **`cedula`, `votar`, `verificar`, a recusa da mesa e a recusa por `τ` não são cortáveis** — são os cinco momentos do vídeo.
