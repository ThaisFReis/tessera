# Tessera — a demo

A demo é **uma tela**, e ela vota de verdade. Quem assiste vê uma pessoa criar
a proposta, outra votar, e a apuração acontecer — cada clique virando uma
transação na testnet, com hash conferível.

```
  navegador  →  ponte (localhost)  →  tessera  →  stellar  →  testnet
```

A página é estática, e página estática não assina transação. A **ponte**
(`console/ponte.py`) resolve isso do jeito mais curto que existe: ela chama a
CLI, que já sabe provar, assinar e enviar. As chaves ficam onde sempre
estiveram, na `stellar keys` da máquina — a ponte não as vê, não as pede e não
as transporta.

**Sem a ponte, a mesma página continua funcionando** como reencenação de uma
rodada real. É assim que ela roda no GitHub Pages. A ponte não muda o que a tela
afirma; muda o tempo verbal — e o selo no alto de cada painel diz qual dos dois
está valendo:

```
  AO VIVO · TESTNET          RODADA GRAVADA
```

---

## 0. O estado

Dez painéis, três atos. Todos construídos, e todos ligados à ponte.

| Ato | Painel | Ao vivo faz |
|---|---|---|
| | Cabeçalho | — |
| **I** | 1 · montar a cédula | **abre a proposta na testnet** |
| **I** | 2 · eleitorado, mesa e prazo | mostra a abertura recém-publicada |
| **II** | 3 · analisar a cédula | — |
| **II** | 4 · votar | **envia o voto**, um eleitor por vez |
| **II** | 5 · pronto | confirmação; não há recibo a queimar |
| **III** | 6 · a cédula indecifrável | — |
| **III** | 7 · a grade | — |
| **III** | 8 · apurar | **apura**, e **faz a mesa mentir de verdade** |
| **III** | 9 · sigilo abaixo do mínimo | **reproduz a recusa por τ** |
| **III** | 10 · o verificador | **roda o verificador** |

E a coluna da mecânica em todos os dez.

**O botão "a mesa mente" não é mais simulação visual.** Com a ponte, ele manda
um total falso para a rede, e o contrato recusa de verdade. A cena deixou de
ser uma animação e virou um fato gravável.

### Uma correção minha, e ela muda o que a demo pode afirmar

Eu disse antes que o console era "a mesma conferência, em JavaScript" e "duas
implementações independentes". **Está errado.** `verificador(d)` lê os booleanos
de `d.verificacao`, que o verificador em Rust gravou. Não há uma linha de
BLS12-381 no console — nenhuma.

O console **apresenta um veredito**; não recalcula. Com a ponte, o botão
`Conferir com o verificador` roda o verificador de verdade — mas quem verifica
continua sendo o processo em Rust, e a tela continua só mostrando o que ele
disse. O painel 10 precisa dizer isso.

---

## 1. Três atos, três atores

A demo segue o ciclo de vida de uma votação, e troca de ator duas vezes. É isso
que a torna legível para quem nunca viu o projeto.

| Ato | Ator | O que ele faz | Painéis |
|---|---|---|---|
| I | **Organizador** | abre a proposta: perguntas, natureza, prazo, mesa | 1–2 |
| II | **Votante** | analisa a cédula, vota, e vê o sigilo protegido | 3–5 |
| III | **Qualquer um** | apura, e confere | 6–9 |

E, atrás de tudo, a coluna da mecânica.

---

## 2. A tela, em dois lados

À esquerda, a pessoa. À direita, **um terminal com o código rodando**.

```
┌──────────────────────────────┬─────────────────────────────────┐
│  Destituir a diretoria?      │  core/src/cds.rs                │
│                              │                                 │
│   ( ) SIM    ( ) NAO         │  if v == 0 {                    │
│                              │      // ramo 0 é o real;        │
│  [ votar ]                   │      // o ramo 1 é fabricado    │
│                              │      // de trás para frente     │
│                              │      let a0 = (*h * t).into();  │
│                              │      ...                        │
│                              │                                 │
│                              │  v = 1 na escolhida, 0 nas outras│
│                              │  r = ▓▓▓▓▓  nunca enviado       │
│                              │  C = 024018d9e520aa4f…          │
└──────────────────────────────┴─────────────────────────────────┘
```

**A coluna direita não resume: ela mostra.** As linhas de código são recortes
literais de `core/src/*.rs`, com os valores desta rodada passando por elas. O
comentário que já estava no crate explica o sigilo melhor do que qualquer
parágrafo escrito em volta dele.

> `// ramo 0 é o real; o ramo 1 é fabricado de trás para frente`

**Os valores que a demo se recusa a mostrar aparecem tarjados**, com o motivo
ao lado — `r` não está no estado, e não existe parcela de Shamir de um voto
individual. A ausência é parte da explicação, não uma lacuna.

**Esses recortes são cópias, e cópia envelhece** — se o crate mudar e o recorte
não mudar junto, a demo passa a mentir com cara de código-fonte. Por isso existe
uma guarda:

```bash
python3 console/guarda.py
```

O bloco `const ANCORAS` no topo do script da página declara, para cada arquivo
citado, os trechos que têm de continuar existindo lá. A guarda confere os dois
sentidos: nenhum terminal cita arquivo não declarado, e nenhuma âncora sumiu.

**Constantes entram com o valor inteiro de propósito.** Trocar `τ` de 5 para 3
sem mexer na tela é exatamente o erro que isto pega — testei trocando, e a
guarda acusou.

Com a mecânica carregando os dados, a esquerda pôde emagrecer. A regra passou a
ser: **se o terminal já diz, a esquerda não repete.** A apuração perdeu dois
cartões de dez linhas e ficou só com o placar, legível do fundo da sala.

Os dez traços:

| Painel | Arquivo | O que mostra |
|---|---|---|
| 1 | `contrato/src/tipos.rs` | `Pergunta { opcoes, confidencial }` e os limites |
| 2 | `core/src/merkle.rs` | `folha`, `no`, e a lista que nunca é publicada |
| 3 | `core/src/merkle.rs` | `verificar` — provar aptidão sem consultar cadastro |
| 4 | `core/src/pedersen.rs` | `comprometer` — `v`, `r` tarjado, `C` |
| 5 | `cli/src/recibo.rs` | o teste que prova que nada é gravado |
| 6 | `core/src/cds.rs` | `provar` — o ramo real e o fabricado |
| 7 | `core/src/soma.rs` | a prova por pergunta, e o desafio com `pergunta` |
| 8 | `core/src/pedersen.rs` | `agregar` e `verifica_agregado`, linha a linha |
| 9 | `contrato/src/lib.rs` | a guarda de `TAU`, e os dois `✓` antes da recusa |
| 10 | `cli/src/comandos.rs` | o laço do verificador — e o que a página não faz |

---

# ATO I — O ORGANIZADOR

## Painel 1 — montar a cédula 

**Esquerda.** Um formulário que monta a proposta pergunta por pergunta. Para
cada uma: o texto, as opções, e **a natureza** — um par de botões.

```
  Pergunta 1   [ Aprovar as contas de 2025?            ]
               [ APROVAR, REJEITAR                     ]
               ( ) em sigilo        (•) em aberto

  Pergunta 2   [ Destituir a diretoria?                ]
               [ SIM, NAO                              ]
               (•) em sigilo        ( ) em aberto

  Pergunta 3   [ Eleger a cadeira 3                    ]
               [ ANA, BRUNO, CARLA                     ]
               (•) em sigilo        ( ) em aberto

                                   [ + pergunta ]
```

E, acima, o que essas escolhas acabaram de definir — **o rótulo é derivado, não
escolhido:**

```
  ESTA CÉDULA É SEMICONFIDENCIAL
  1 pergunta em aberto · 2 em sigilo · mesma transação
```

| Se o organizador marcou | A cédula é |
|---|---|
| todas `em sigilo` | **confidencial** |
| uma mistura | **semiconfidencial** |
| todas `em aberto` | uma votação comum, e Tessera não é necessária |

**Decisão de desenho:** o organizador **não escolhe "confidencial" ou
"semiconfidencial"** num seletor. Ele marca a natureza de cada pergunta, e o
modo *sai disso*. Um seletor de modo seria um segundo lugar de verdade, podendo
contradizer as perguntas. Aqui não há como ficar inconsistente.

**Direita — o ponto mais fino do desenho inteiro, e o lugar dele é aqui:**

> A natureza é **da pergunta**, fixada agora, na abertura, e vale para todo
> mundo.
>
> **Por que não deixar cada eleitor escolher?** Porque a lista de *quem pediu
> sigilo na pergunta 2* seria, ela mesma, pública. Numa assembleia isso é quase
> o próprio voto — e teria sido criado pelo recurso que existia para proteger. A
> escolha é do estatuto, não da pessoa.
>
> Em `Proposta`, cada pergunta é `Pergunta { opcoes: u32, confidencial: bool }`.
> Dentro do contrato, "semiconfidencial" não é um modo: é um vetor com os dois
> valores de `bool`.

Limites, que a tela deve impor em vez de deixar o contrato recusar: **8
perguntas**, e **16 opções sigilosas no total** da cédula.

## Painel 2 — eleitorado, mesa e prazo 

**Esquerda.**

```
  Quem vota ........ 7 pessoas          [ ver lista ]
  Peso ............. um voto por pessoa
  Mesa ............. 5 membros · 3 assinaturas para apurar
  Sigilo mínimo .... 5 votos confidenciais
  Encerra em ....... [ 15 min ]

                                      [ abrir votação ]
```

Depois do clique, o que foi publicado — e isso é dado real da rodada:

```
  ✓ votação aberta
  Transação ... 201be45a      ledger 4973643
  Encerra ..... ledger 4973822
```

**Direita, quatro objetos, e cada um responde a uma pergunta óbvia da plateia:**

> **"A lista de eleitores vai para a blockchain?"** Não. Vai uma **raiz de
> Merkle** de 32 bytes — folha `H(0x00 ‖ addr ‖ peso)`, nó `H(0x01 ‖ esq ‖
> dir)`. O contrato **nunca guarda a lista**. Cada eleitor prova a própria
> aptidão por caminho, e provar aptidão não é consultar um cadastro.
>
> **"Quem pode abrir os votos?"** Ninguém sozinho, e nunca um voto. A mesa
> recebe **agora** as parcelas de Shamir `k`-de-`n`. Elas só servem para
> reconstruir `Σr` — a soma dos acasos. Não existe parcela de um voto
> individual.
>
> **"O prazo é confiável?"** É `fecha_em`, um número de ledger. Não é relógio
> de servidor. Antes dele, `apurar` recusa com
> `Error(Contract, #7) = VotacaoAindaAberta`.
>
> **"E o sigilo mínimo?"** τ = 5, fixado aqui. É o painel 8, e o organizador
> está concordando com ele agora.

O estado que a transação escreveu, inteiro:

```rust
Proposta {
    perguntas,      // Vec<Pergunta { opcoes, confidencial }>
    raiz_aptos,     // 32 bytes
    mesa, limiar,   // 5 endereços, k = 3
    fecha_em,       // ledger
}
```

Custo medido de `abrir` com 3 perguntas e 7 aptos: **40.561.382 stroops**. Paga
uma vez, pela governança — não pelo eleitor.

---

# ATO II — O VOTANTE

## Painel 3 — analisar a cédula 

Troca de ator. **Deixe isso explícito na tela** — é o momento em que a plateia
precisa perceber que quem fala agora é outra pessoa.

**Esquerda.** A proposta como o eleitor a recebe: as três perguntas, a natureza
de cada uma à vista, e o seu estado.

```
  Encerra em 12min · você é apta · ainda não votou
```

E a frase que o produto diz antes do voto, não depois:

```
  Esta cédula é mista. Uma pergunta vai em claro, ao lado do seu endereço:
    · Aprovar as contas de 2025?
  O resto fica em sigilo, na mesma transação.
```

**Direita:**

> **Antes de votar você vê o que vai publicar.** A divulgação vem primeiro —
> depois do envio seria aviso, não escolha.
>
> `você é apta` saiu de um caminho de Merkle conferido contra a raiz, do lado do
> cliente. Nenhuma consulta a cadastro.

## Painel 4 — votar 

**Esquerda.** Ele escolhe uma opção em cada pergunta e envia.

**Direita, conforme clica:**

| Clicou numa pergunta | A tela mostra |
|---|---|
| **em aberto** | `escolha = 0` — um `u32`, ao lado do seu endereço. Sem compromisso, sem prova. |
| **em sigilo** | os 96 bytes do compromisso que vai ao ledger |
| e enviou | 1 transação · 3 perguntas · **0,0242 XLM** · 8.360 B |

**Ao vivo, a urna manda o voto de verdade.** Ela escolhe sozinha a próxima
pessoa que ainda não votou — numa gravação são sete votos seguidos, e travar na
que acabou de votar renderia `VotouDuasVezes`. Quem já votou aparece marcado e
desabilitado.

**A contradição que o desenho precisa evitar.** Se clicar em SIM mostrar *o*
compromisso do SIM ao lado do outro, a tela revela o mapa que o painel 6 jura
ser indecifrável. **A saída é a ordem:** aqui aparece só o compromisso da
escolha, nunca o par. Os dois juntos só no painel 6, onde "qual era qual?" já
não tem resposta na tela.

**Não há botão de abrir o voto.** Houve, e foi removido junto com
`votar_publico` no contrato: um voto aberto no ledger é coação verificável em
escala, e em bloco derrubava a apuração por τ.

> Uma transação, três perguntas, uma taxa. `Votou(proposta, eleitor)` é **uma
> chave por cédula**, não por pergunta: **não existe votar metade**. E o
> contrato verifica tudo — provas, aptidão, formato — **antes de escrever
> qualquer coisa**.

## Painel 5 — pronto

**Esquerda.** O voto entrou, e a tela vai direto para cá — não há botão a
apertar.

```
  Pronto
  MARTA · LEDGER 4.986.200

  Seu voto está na contagem.
  Seu sigilo está protegido.

  Nenhum arquivo nesta máquina prova em que você votou.
```

**Direita:**

> `votar` **não grava recibo**. Houve uma versão que gravava `r` em
> `./recibos/<identidade>.key` e pedia que a pessoa apagasse depois. Dois
> defeitos: quem coage manda não apagar, e nenhum comando lia o arquivo — nem
> para conferir voto, nem para apurar. Era passivo puro.
>
> Gravar e apagar em seguida deixa uma janela. Não gravar não deixa janela
> nenhuma: o `r` vive no processo e morre com ele.
>
> A auditoria não perdeu nada, porque nunca dependeu do segredo — compromissos e
> provas continuam no estado público. Um teste lê `comandos.rs` e falha se
> `recibo::gravar` voltar; é a única forma de provar uma ausência.

**Narração:**

> Não existe prova do seu voto em lugar nenhum. É isso que torna a compra de
> voto sem objeto: não há o que entregar.

## Painel 6 — a cédula indecifrável  (`momentoCedula`)

Os dois blocos de 96 bytes, lado a lado, da **mesma pessoa** e da **mesma
transação**. Um compromete o voto, o outro a ausência dele.

**Direita:**

> **Perfeitamente oculto.** Não "difícil de quebrar" — *sem informação*. Para
> qualquer `C` e qualquer `v`, existe um `r` que os liga. Um adversário com
> tempo infinito tem exatamente a chance de quem chuta.
>
> O que sobrevive é a soma: `C₁ + C₂ + … = (Σv)·G + (Σr)·H`. Dá para somar
> votos que você não consegue ler. **Todo o resto é consequência disso.**

A frase dura já está no console e **não pode ser suavizada** — é o único
superlativo do produto que é literalmente correto, porque a garantia é
teórico-informacional, não computacional.

**Não anime uma troca de opção com o bloco mudando.** Lado a lado é honesto; a
troca sugere um antes e um depois, e induz a leitura errada.

## Painel 7 — a grade, e os três objetos  (`grade`)

Todos os compromissos da proposta, lado a lado. 25 blocos na rodada real. É a
imagem do pitch, e **a razão de o console existir**: uma grade de blocos
indistinguíveis não tem como existir num terminal.

| Objeto | Tamanho | O que garante |
|---|---|---|
| Compromisso | 96 B | esconde o voto, preserva a soma |
| Prova disjuntiva (CDS) | 320 B | `v ∈ {0,1}` — sem dizer qual |
| Prova de soma (Schnorr) | 128 B | **uma por pergunta**: as opções somam o peso |

> **Uma por pergunta, não uma por cédula.** Com uma só para a cédula inteira, a
> mesa compensaria: dois votos na pergunta 1 e zero na 2 fecham a soma total. O
> contrato confere pergunta por pergunta.
>
> São **sigma-protocolos com Fiat–Shamir, não SNARKs**. Sem setup confiável, sem
> circuito, sem cerimônia — nada aqui depende de um ritual em que você precise
> acreditar. O host do Soroban expõe MSM de BLS12-381 nativamente, e é por isso
> que a verificação cabe no contrato.

## Painel 8 — apurar  (`momentoConferencia` + `momentoApuracao`)

**Esquerda.** A tabela de quem votou, em que ledger, em que transação — e
**nenhum voto**. Depois, o placar:

```
  1. Aprovar as contas de 2025?   em aberto    APROVAR ...... 6 a 1
  2. Destituir a diretoria?        em sigilo    NAO .......... 5 a 2
  3. Eleger a cadeira 3            em sigilo    CARLA ........ 3 a 2 a 2
```

**Direita:**

> **Shamir `k`-de-`n` sobre Fr é aditivamente homomórfico.** Cada membro soma
> suas parcelas **localmente**. Três de cinco reconstroem `Σr` — e **só** `Σr`.
> Nenhum voto individual é reconstruível por ninguém, nem pela mesa inteira
> reunida.
>
> A mesa **afirma** o total. O contrato não acredita: confere que
> `total·G + Σr·H` é exatamente o acumulado que já estava na rede, **pergunta
> por pergunta**. Se não fecha, nada é publicado.
>
> A mesa é um mecanismo de **disponibilidade**, não de confiança. Pode se
> recusar a apurar. **Não pode mentir.**

**Mostre a recusa** — um botão `mesa mentindo` que troca o total por um falso e
deixa a conta não fechar, na tela.

> Não é que alguém auditou depois e descobriu. É que a apuração errada **não é
> representável** — não existe o estado em que ela foi publicada.

## Painel 9 — quórum de sigilo

Um clique monta a rodada: proposta nova, **4 das 7 pessoas votam**, o prazo
fecha, e a mesa tenta apurar.

```
  MANTER ............. 4
  Acumulado MANTER ... 4·G + R·H                      ✓
  Acumulado RESCINDIR  0·G + R·H                      ✓

  ✗ apuração recusada · nada foi publicado
  Error(Contract, #19) = AnonimatoInsuficiente
```

Os dois `✓` vêm **antes** da recusa, e a ordem é o efeito inteiro: a mesa
reconstruiu a abertura, achou os totais, os acumuladores fecharam — e o contrato
não publicou.

**Direita — e esta cena mudou de significado:**

> Com poucas cédulas o total entrega quem votou. No limite, uma cédula só *é* o
> voto daquela pessoa.
>
> **Isto é quórum, não recusa.** Antes era um ataque: bastavam três pessoas
> abrindo o voto por `votar_publico` para encolher o conjunto sigiloso de fora e
> vetar a assembleia inteira — negação de serviço contra a própria eleição.
> Removida aquela função, o único caminho até aqui é comparecimento baixo.
>
> Nenhum sistema eleitoral sério recusa contar. O Brasil protege a célula
> pequena **antes**, agregando seções com menos de 50 eleitores (TSE,
> Res. 23.669/2021); quando há nulidade, o remédio é eleição nova (CE art. 224),
> não ausência de resultado. O remédio aqui é o mesmo: estender o prazo, ou
> refazer com um eleitorado que caiba no sigilo.

**Narração:**

> Quatro pessoas votaram num eleitorado de sete. O contrato tinha o resultado
> correto na mão e não publicou — porque publicar entregaria as quatro. Isso é
> quórum, declarado na abertura, e o remédio é repetir com mais gente.

## Painel 10 — o verificador  (`verificador`)

```
  25 de 25 disjuntivas válidas                        ✓
  Soma por pergunta · 10 de 10 somam 1                ✓
  Respostas em claro · 7 de 7 conferem                ✓
  Agregado refeito bate com o acumulador              ✓
```

**Direita, e é aqui que a correção da §0 tem que aparecer:**

> Este é o veredito do verificador, que é **outro processo**, em Rust, que relê
> o estado e refaz toda a aritmética. A tela o **apresenta**; não o recalcula.
>
> Limitação, dita em voz alta (smoke D4): o verificador lê os compromissos do
> JSON local, não da rede. O que o prende à realidade é que a **soma** deles é
> conferida contra o acumulado on-chain — um JSON adulterado não passa. Ainda
> assim é independência parcial, e está registrada como tal.

---

## 11. Preparar os dados — o terminal, fora de quadro

### 11.1 Subir a ponte

Não há workspace na raiz: cada crate é um projeto.

```bash
cd cli && cargo build --release && cd ..
```

```bash
export TESSERA_CONTRATO=CBAHLZMTSP52CVPJGN6ATDVP4XACCX2PVMGBVIVYMR363JQRSOL7OOTG
python3 console/ponte.py
```

E abra **http://127.0.0.1:8780**. A ponte:

- escuta **só em 127.0.0.1**;
- serve `console/` **com charset** — o que resolve a mojibake que estragava a
  gravação com `python3 -m http.server`;
- chama a CLI com os argumentos em lista, **sem shell**, e valida nome de
  proposta, de identidade, prazo e total antes de chamar;
- roda a CLI de dentro de `demo/`, que é onde ela guarda `estado/`, `recibos/`
  e `shares/`.

Se a porta estiver ocupada: `TESSERA_PORTA=8781 python3 console/ponte.py`.

**Recusa não é erro.** Quando o contrato recusa, a ponte devolve a saída da CLI
na íntegra e a tela mostra. É assim que os painéis 8 e 9 funcionam.

### 11.1b Pela CLI, se preferir

```bash
export PATH="$PWD/cli/target/release:$PATH"
cd demo
```

Os testes rodam por crate:

```bash
for c in core contrato cli bls-smoke; do (cd $c && cargo test); done
python3 console/guarda.py
```

112 testes e 27 âncoras.

### 11.2 O problema do prazo — leia antes de começar

`apurar` só funciona depois que `fecha_em` passou; antes disso o contrato recusa
com `Error(Contract, #7) = VotacaoAindaAberta`. Isso **já derrubou duas
tentativas**. Não existe atalho: o ledger precisa avançar.

Abra as **duas** propostas no começo, para os prazos correrem juntos:

```bash
tessera abrir --proposta assembleia-demo \
  --pergunta "Aprovar as contas de 2025? | APROVAR, REJEITAR | publica" \
  --pergunta "Destituir a diretoria? | SIM, NAO | sigilosa" \
  --pergunta "Eleger a cadeira 3 | ANA, BRUNO, CARLA | sigilosa" \
  --aptos aptos.txt --mesa mesa.txt -k 3 --prazo 15m

tessera abrir --proposta coligacao-demo \
  --pergunta "Manter a coligação? | MANTER, RESCINDIR | sigilosa" \
  --aptos aptos.txt --mesa mesa.txt -k 3 --prazo 15m
```

Vote nas duas. Na segunda, **só 4 das 7 pessoas** — é o que deixa o
comparecimento abaixo de τ e faz a apuração parar por falta de quórum de
sigilo.

```bash
tessera votar --proposta assembleia-demo \
  --opcao APROVAR --opcao SIM --opcao ANA --identidade ana
```

Depois do prazo:

```bash
tessera apurar --proposta assembleia-demo
tessera verificar --proposta assembleia-demo
tessera apurar --proposta coligacao-demo     # recusa #19
```

### 11.3 Levar para a tela

```bash
cd .. && python3 console/embutir.py demo/estado/assembleia-demo.json
```

O script **recusa** embutir um estado com qualquer token proibido. Rode-o e leia
a saída antes de gravar.

Com o snapshot embutido a página abre direto do arquivo. Para ler outro estado,
`console/index.html?p=<proposta>` — e isso precisa de servidor, por causa do
`fetch`.

⚠ `python3 -m http.server` **não manda charset** e os acentos quebram. O GitHub
Pages renderiza certo. **Se for gravar o console, grave do Pages.**

---

## 12. Gravar

| Ato | Painel | Duração | Corte de 3 min |
|---|---|---|---|
| | A pergunta — por que isso importa | 0:20 | ✓ |
| I | 1 · montar a cédula, e a natureza | 0:35 | ✓ |
| I | 2 · mesa, prazo, raiz de Merkle | 0:25 | — |
| II | 3 · analisar a cédula | 0:20 | — |
| II | 4 · votar | 0:30 | ✓ |
| II | 5 · pronto · o sigilo protegido | 0:25 | ✓ |
| III | 6 · a cédula indecifrável | 0:40 | ✓ **o coração** |
| III | 7 · a grade | 0:20 | ✓ |
| III | 8 · apurar + mesa mentindo | 0:40 | ✓ |
| III | 9 · τ | 0:35 | ✓ |
| III | 10 · verificador | 0:25 | ✓ |
| | | **5:20** | **3:00** |

**A troca de ator é um corte, e precisa ser visível.** Entre os painéis 2 e 3, e
entre 5 e 6, a tela deve anunciar de quem é a vez. Sem isso a plateia acha que é
sempre a mesma pessoa, e o desenho inteiro fica incompreensível.

### A abertura

> Uma assembleia precisa decidir três coisas. Aprovar as contas — isso pode ser
> na mão levantada. Destituir a diretoria — essa ninguém levanta a mão. E
> eleger uma cadeira.
>
> Hoje você escolhe: ou a votação é pública e rastreável, ou é secreta e você
> confia em quem conta. Tessera remove essa escolha.

### O fechamento

> Contrato de 22,1 KB, 112 testes, rodando na testnet. Compromisso de Pedersen,
> prova disjuntiva, Shamir para a mesa. Sem setup confiável, sem cerimônia.
>
> E a cédula mista: a mesma assembleia decide o que é público e o que é secreto,
> pergunta por pergunta, numa transação.

### O que nunca entra em quadro

- Chave privada, seed, saída de `stellar keys show`.
- O conteúdo de `recibos/` ou `shares/` por mais de um instante.
- Identidade que não seja descartável de testnet.

---

## 13. Os números, para consulta

Nada fora desta tabela deve ser afirmado em quadro.

| | |
|---|---|
| Contrato (testnet) | `CBAHLZMTSP52CVPJGN6ATDVP4XACCX2PVMGBVIVYMR363JQRSOL7OOTG` |
| Wasm otimizado | 22.584 B = **22,1 KB** |
| Testes | **112** · bls-smoke 13, core 49, contrato 25, cli 25 |
| `votar`, custo | **9.805.000 + 13.501.500 por opção sigilosa** |
| 1 pergunta sigilosa, 2 opções | 36.804.670 — **9,2%** do teto |
| mista: 1 pública + 2 sigilosas | 73.547.797 — **18,4%** |
| 16 opções sigilosas (o máximo) | 225.920.355 — **56,5%** |
| cédula pública inteira | 350.372 |
| Taxa, cédula mista | **0,0242 XLM** |
| Envelope de tx, cédula mista | **8.360 B** — a maior do projeto |
| Apuração, taxa | 2.702.641 stroops |
| τ | **5** |
| Rodada real | tx `6a774f8c`, ledger 4973370 |

**Dois números que o roteiro não afirma:**

- A folga sobre o teto de tamanho de transação. Medi 8.360 B e foi aceito; não
  confirmei o teto declarado nas fontes. A narração diz "aceito", não "x% do
  limite".
- Aluguel de estado para `n` grande — smoke A4, aberto.

**Uma correção que a medição impôs**, e que vale dizer: eu esperava que a cédula
mista economizasse CPU de forma relevante. Medi **0,08%**. O custo de `votar` é
linear nas opções sigilosas e a pergunta pública custa quase nada. O ganho é
**uma transação em vez de duas, uma taxa, e atomicidade** — não é desempenho.

---

## 14. Pendências

| Smoke | Estado |
|---|---|
| A3 · tamanho de transação | medido · teto da rede não confirmado |
| A4 · aluguel de estado para `n` grande | **aberto** |
| D4 · independência do verificador | parcial — e está dito no painel 6 |
| E1 · UX de duas perguntas | **aberto** |

Abstenção explícita fica para a v1.1: hoje a cédula sigilosa não tem como dizer
"nenhuma das opções" sem quebrar a prova de soma. SPEC §6.5 e roadmap §12.1.
