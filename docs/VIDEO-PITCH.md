# Tessera — roteiro do vídeo de pitch

Este é o **vídeo de pitch**, o que acompanha a submissão e o deck de sete
slides. Ele não é o `docs/VIDEO.md`: aquele é a **demonstração de nove
minutos**, do comparecimento ao placar, em tempo real e sem corte. Os dois
convivem — o pitch convence a assistir, a demonstração prova.

A versão em inglês é `docs/VIDEO-PITCH.en.md`, com os mesmos oito blocos.

> **O que este vídeo tem a provar, em uma frase:** existe um jeito de uma
> governança votar em segredo e ainda assim qualquer pessoa reconferir o
> resultado — e ele está de pé, medido, na testnet.

**Duração real: 4min04** de fala, a 160 palavras por minuto, sem contar as
pausas. Eu tentei fazer caber em 3min00 e não cabe sem estragar a demonstração,
que é o único bloco que convence sozinho. Se o limite for duro em 3 minutos,
corte nesta ordem — cada corte está marcado no texto:

1. o segundo exemplo do §2 (−5s);
2. o parágrafo do meio do §6, o do custo da apuração (−13s);
3. a primeira frase do §5, até "parte difícil" (−7s).

Dá 3min39 — ainda acima de três minutos. Abaixo disso o §4 começa a perder as
duas colunas, e aí o vídeo não prova mais nada.

---

## Como este texto foi escrito

Texto para ser **ouvido uma vez só**, não lido. Quem escuta não volta atrás,
não vê a vírgula e não tem o slide decorado. As regras que eu segui:

- **Frase curta, uma ideia por frase.** Onde havia travessão, agora tem ponto.
- **Sujeito concreto.** Quem age é gente, não conceito. "O sigilo protege a
  liberdade de discordar" virou "elas só não votam".
- **Explicar antes de nomear.** Primeiro o que a coisa faz, depois como ela se
  chama — nunca o contrário. Por isso "compromisso de Pedersen" saiu: o nome
  não cabia sem roubar o tempo da explicação.
- **Nada de tríade.** Três itens paralelos seguidos soam a texto gerado. Um
  exemplo bom vale mais que três resumidos.
- **Sem a fórmula "não é X, é Y"**, que aparecia cinco vezes na versão anterior.
- **Segunda pessoa.** "Como você votou" prende mais que "a escolha individual".

### Regra de ouro

**Nenhum número que não tenha sido medido, nenhuma palavra que a `§2` do
`SPEC.md` proíba.** Este roteiro nunca diz "seguro", "auditado", "pronto para
produção" nem "trustless", e nunca promete resultado se a baliza parar.

### O que conferir na véspera da gravação

- [ ] **O dapp responde.** No dia 13/10 `thaisfreis.github.io/tessera/dapp/`
      devolvia **404** — o PR #23 não foi mergeado, e o Pages serve o `master`.
      O slide 04 do deck já aponta para esse link.
- [ ] O contrato responde no explorer.
- [ ] A rodada que você vai mostrar está na janela de retenção do RPC, se for
      mostrar o verificador.

---

## A estrutura, slide a slide

| Bloco | Slide | Tempo | Tela |
|---|---|---|---|
| 1 Gancho | 01 capa | 0:00–0:08 | você, ou a capa |
| 2 Problema | 02 | 0:08–0:32 | slide |
| 3 Desenho | 03 | 0:32–1:41 | slide |
| 4 Demonstração | 04 | 1:41–2:31 | **captura de tela** |
| 5 Por que Stellar | 05 | 2:31–2:51 | slide |
| 6 Evidência | 06 | 2:51–3:22 | slide |
| 7 Time | — | 3:22–3:42 | você, ou o Karn |
| 8 Fecho | 07 | 3:42–4:04 | slide |

O §7 não tem slide no deck — veja a nota no fim desta página.

---

## 1. O gancho — 0:00 a 0:08

**Tela:** você em câmera, ou a capa. Nada mais.

> Ninguém consegue saber como você votou. Nem quem organizou a votação.
>
> Isso é Tessera, e está rodando na testnet da Stellar agora.

**Direção:** é a única chance que o vídeo tem. Fale devagar, sem "oi, meu nome
é" — o nome vem no §7.

---

## 2. O problema — 0:08 a 0:32

**Tela:** slide 02.

> Hoje, quando você vota numa DAO ou num conselho, o seu voto fica público. Com
> o seu nome do lado. Para sempre.
>
> Pensa no conselheiro que precisa votar contra o orçamento do chefe dele. Ou
> no contribuidor que vota contra a baleia que paga o trabalho dele.
>
> Essas pessoas não votam errado. Elas só não votam. E isso não aparece em
> lugar nenhum.

**Direção:** "Elas só não votam" é a frase do bloco. Pause antes e depois dela.

**Corte 1, se precisar:** tire "Ou no contribuidor que vota contra a baleia que
paga o trabalho dele." O conselheiro sozinho já faz o trabalho.

---

## 3. O desenho — 0:32 a 1:41

**Tela:** slide 03.

> O jeito óbvio seria cifrar o voto. Só que aí você troca um problema por
> outro. O voto cifrado fica gravado para sempre, e a chave que abre ele está
> em algum lugar. Daqui a cinco anos alguém acha essa chave e lê como você
> votou hoje.
>
> Então o Tessera não grava o seu voto. Ele grava um número que prova que você
> votou, sem dizer o que você votou. Sozinho esse número não serve para nada.
> Mas dá para somar todos eles e conferir o resultado.
>
> E quem abre uma cédula, para ver o que ela votou? Você pode nomear uma mesa
> apuradora, repartir a chave entre essas pessoas, e confiar que elas apaguem
> as cópias depois. Dá para fazer, e o Tessera faz.
>
> Mas o modo que ele defende não tem mesa nenhuma. Ninguém tem chave. Nem eu.
> As cédulas viajam cifradas para uma hora marcada, e nessa hora um sorteio
> público publica a chave que abre todas. Aí qualquer pessoa reconta o
> resultado sozinha, cédula por cédula.
>
> E abrir uma cédula mostra o que ela votou. Nunca de quem ela é.

**Direção:** "Nem eu" é o ponto de virada do pitch inteiro. Não corra nele.

---

## 4. A demonstração — 1:41 a 2:31

**Tela:** captura do dapp. É o bloco mais longo de propósito, e é o único que
não pode encolher.

> Chega de falar.
>
> Isso aqui é uma votação de verdade, na testnet. Sem mesa nenhuma, sem
> servidor, ninguém segurando chave.
>
> Primeiro a pessoa se identifica. Prova que tem direito de votar e assina a
> lista de presença, em público. Todo mundo vê quem apareceu.
>
> Depois ela vota. De outro endereço, criado na hora, que não tem nada a ver
> com o primeiro. A cédula prova que saiu de alguém daquela lista, sem dizer
> de quem.
>
> Olha as duas colunas. Nenhum endereço aparece nas duas. E não é sorte desta
> rodada: numa outra, com trinta votantes, a interseção deu exatamente zero.
>
> O prazo vence e o resultado sai. Quem apurou foi uma carteira que nem votou.
> Uma das sete cédulas estava corrompida de propósito: perdeu o próprio voto,
> e as outras seis contaram.

**Direção:** se der para mostrar **uma** coisa só, mostre as duas colunas lado
a lado. É a única imagem do vídeo que prova algo sozinha. Não diga "sem
confiança" nem "trustless" — diga "ninguém segurando chave", que é o que é
verdade.

---

## 5. Por que Stellar — 2:31 a 2:51

**Tela:** slide 05.

> Por que Stellar. A matemática pesada já vem pronta dentro da Soroban. Eu não
> precisei programar a parte difícil.
>
> Então dá para fazer tudo com prova direta, sem cerimônia de instalação e sem
> precisar confiar em quem montou o sistema.
>
> E ficou barato. Um voto secreto completo custou um centavo e meio de XLM.

**Corte 3, se precisar:** comece em "Dá para fazer tudo com prova direta".

---

## 6. A evidência — 2:51 a 3:22

**Tela:** slide 06, o lima.

> Nada disso é estimativa. Tudo foi medido no contrato que está no ar.
>
> Um voto secreto completo usa nove por cento do que cabe numa transação. E
> apurar não fica mais caro conforme entra gente: de cinco votantes para
> quarenta, o custo subiu um e meio por cento.
>
> Mas a parte de que eu mais gosto é esta. Você não precisa acreditar em mim.
> Tem um programa separado que baixa tudo da blockchain, refaz as contas do
> zero e confere o resultado.

**Direção:** o slide é o único de fundo claro do deck. Corte seco — o contraste
faz o trabalho sozinho.

**Corte 2, se precisar:** tire o parágrafo do meio inteiro.

---

## 7. O time — 3:22 a 3:42

**Tela:** o repositório do Karn, ou você. Veja a nota no fim.

> Eu sou Thais Reis, engenheira de software no Rio de Janeiro. Eu construo
> aplicação de ponta a ponta: cliente, API, banco, contrato e deploy.
>
> Três primeiros lugares em hackathon. E Tessera também não é a minha primeira
> governança. Eu mantenho o Karn, um protocolo de governança aberto. É
> exatamente onde ele encaixa.

**De onde vem cada afirmação** (currículo, 13/10/2026):

- **"aplicação de ponta a ponta"** — Yalla, engenheira **única** do produto:
  cliente em React, API em Node/TypeScript, esquema em PostgreSQL, contratos em
  Rust e deploy.
- **"três primeiros lugares"** — Monad Blitz Rio de Janeiro (2026), Hackathon
  Nola "God Level, Code Your Future" (2025) e Morro Makers (2025).
- **"eu mantenho o Karn"** — protocolo de governança aberto, fundadora e única
  contribuidora: três contratos em Rust, 53 testes, 5 achados de segurança
  documentados e resolvidos, SDK em TypeScript publicado no npm.

**O que ficou de fora, e vale ter na ponta da língua** se o júri perguntar:

- **Stellar Instaward via Stellar 37º (2026)**, programa regional da SDF, e
  convite ao **Builder Summit** entre cerca de cem construtores. É a credencial
  mais forte que você tem **para este júri em específico** — a SDF já te
  escolheu uma vez. Ela saiu do texto e eu não a coloco de volta sem você pedir,
  mas se houver Q&A, é a primeira coisa a dizer.
- Os números do primeiro evento ao vivo da Yalla: 113 ingressos em 8 faixas, 98
  presentes, 257 vouchers de bar resgatados sem nenhum expirado, pico de 39
  check-ins por hora.
- Dois contratos atualizáveis em Rust, com autoridade de admin e de upgrade
  separadas de propósito, e um upgrade executado ao vivo em produção.

> **Confirme antes de gravar:** o bloco assume **solo**. Se houver mais alguém
> em Tessera — design, consultoria, revisão — me diga e eu reescrevo.

---

## 8. O fecho — 3:42 a 4:04

**Tela:** slide 07.

> Conferir uma decisão não deveria custar expor a escolha de cada um. Dá para
> construir isso. Está construído, está rodando, e o código é aberto.
>
> Se você governa alguma coisa, Tessera encaixa no que você já usa. E se você
> mexe com criptografia, o repositório está lá para você quebrar.
>
> A decisão é de todos. A escolha é sua. Tessera.

**Direção:** é um convite, não uma ressalva — a voz não fala de limites porque
eles estão **na tela**: o rodapé do slide 07 traz as condições do sigilo, e ele
fica visível durante o bloco inteiro. Suba o tom em "está construído, está
rodando". Segure o slide dois segundos depois da última palavra, com o endereço
do repositório visível, e não leia a URL em voz alta.

---

## Depois de gravar

1. Insira a URL no slide 04 do deck, nos **dois** idiomas — ela está marcada
   como `URL a inserir antes da submissão`.
2. Insira a URL no README.
3. Se o §4 foi gravado com o dapp ainda em 404, conserte a linha "dapp no ar"
   do slide 04 antes de submeter, ou mergeie o PR #23.

---

## O que este roteiro deliberadamente não diz

- Que o projeto é seguro, auditado ou pronto para produção.
- Que abrir o conteúdo antes da hora é impossível — é conluio de um limiar dos
  operadores da baliza.
- Que a seção de uma pessoa é secreta. Ela é **imprevisível até a abertura**,
  que é coisa diferente.
- Que existe plano B se a baliza parar. Não existe.
- Qualquer número que não tenha vindo de uma medição registrada em
  `docs/SOURCES.md`.

---

# Narração para voz clonada (ElevenLabs v4)

A narração vai ser gerada com um clone da minha voz. Isso muda três coisas, e é
por isso que esta seção existe em vez de você colar os blocos de cima direto no
sintetizador.

**Um:** o sintetizador lê o que está escrito, não o que está formatado. Negrito
e markdown ou são lidos em voz alta ou atrapalham a prosódia. Os blocos abaixo
são texto puro.

**Dois:** número escrito em algarismo é uma aposta. Abaixo, **todo número está
por extenso** — o valor medido é o mesmo, só a grafia mudou.

**Três:** sem rosto na tela, o bloco do time perdeu o apoio visual que ele
tinha. Veja a nota no fim desta seção.

## As etiquetas

Conferidas na [lista oficial do v4](https://elevenlabs.io/blog/elevenlabs-audio-tags-list)
em 13/10/2026. São **46 marcações de oito tipos**, e 42 delas são só de ritmo:

| Etiqueta | Onde, e por quê |
|---|---|
| `[pause]` `[long pause]` | respiração entre ideias; a longa só nos quatro silêncios que o texto pede |
| `[slowly]` | nas sete frases que o roteiro manda não correr — a abertura, "Para sempre", "Daqui a cinco anos", "Nem eu", "Nenhum endereço aparece nas duas", "Eu não tenho um time" e "Eu não vou dizer que passou" |
| `[snappy]` | uma vez só, no "Chega de falar" que abre a demonstração |
| `[thoughtful]` | ao entrar nos exemplos do §2 |
| `[let down]` | em "Elas só não votam" — é a frase do bloco |
| `[curious]` | em "E quem abre?", que é uma pergunta de verdade |
| `[proud]` | em "a parte de que eu mais gosto", a única vez que a narração se permite gostar de algo |

**Quatro etiquetas de emoção no roteiro inteiro.** A documentação recomenda
**uma por oração**, e eu fiquei bem abaixo disso de propósito: o seu problema
com a versão anterior era soar artificial, e deck de etiqueta emotiva é
exatamente como se volta a soar. Narração de pitch convence por conter, não por
atuar.

### Três coisas que eu não sei

**Um:** a documentação do v4 **não diz nada sobre português**. Toda a lista está
em inglês e nenhum exemplo é em outra língua. As etiquetas podem funcionar
igual, podem funcionar pela metade, ou podem sair faladas.

**Dois:** não testei nenhuma delas com a sua voz.

**Três:** `[let down]`, `[snappy]` e `[long pause]` são as mais incomuns da
lista, e são as primeiras candidatas a vazar.

### Como testar antes de gastar crédito

1. Gere só o **§2**, que é curto e tem três etiquetas diferentes.
2. Ouça procurando **palavra inglesa falada**. Se "pause" ou "let down" aparecer
   como som, as etiquetas não estão pegando em português.
3. Se vazar: **tire todas**. O texto foi escrito para funcionar sem elas — a
   pontuação já carrega o ritmo, e a documentação diz que pontuação também
   controla performance.
4. Se funcionar mas soar teatral, tire primeiro as quatro de emoção e mantenha
   as de ritmo. É o ajuste mais provável.

> **Observação sobre o modelo.** O painel do v4 oferece modos de geração, e a
> documentação do v3 diz que o modo **Robust** reduz a resposta às etiquetas.
> Se você escolher Robust, espere que elas façam pouco ou nada.

## Como cortar

Gere **um arquivo por bloco**, não um único áudio de quatro minutos. O corte
entre blocos cai no silêncio em vez de cair no meio de uma respiração; você
refaz um bloco ruim sem regerar o resto; e o §4 precisa esticar ou encolher para
casar com a captura de tela, o que é impossível num arquivo só.

## Palavras para conferir no primeiro teste

| No texto | Se sair errado, escreva |
|---|---|
| Stellar | Stélar |
| Soroban | Sorobã |
| dapp | déppi |
| XLM | xis ele eme |
| DAO | dáu |

## Os blocos

### 1 · gancho

```
[slowly] Ninguém consegue saber como você votou. [pause] Nem quem organizou a
votação. [pause]

Isso é Tessera, e está rodando na testnet da Stellar agora.
```

### 2 · problema

```
Hoje, quando você vota numa DAO ou num conselho, o seu voto fica público. Com o
seu nome do lado. [slowly] Para sempre. [pause]

[thoughtful] Pensa no conselheiro que precisa votar contra o orçamento do chefe
dele. Ou no contribuidor que vota contra a baleia que paga o trabalho dele.
[pause]

Essas pessoas não votam errado. [pause] [let down] Elas só não votam. 
[long pause] E isso não aparece em lugar nenhum.
```

### 3 · desenho

```
O jeito óbvio seria cifrar o voto. [pause] Só que aí você troca um problema por
outro. O voto cifrado fica gravado para sempre, e a chave que abre ele está em
algum lugar. [slowly] Daqui a cinco anos alguém acha essa chave e lê como você
votou hoje. [long pause]

Então o Tessera não grava o seu voto. Ele grava um número que prova que você
votou, sem dizer o que você votou. Sozinho esse número não serve para nada. Mas
dá para somar todos eles e conferir o resultado. [pause]

[curious] E quem abre uma cédula, para ver o que ela votou? [pause] Você pode
nomear uma mesa apuradora, repartir a chave entre essas pessoas, e confiar que
elas apaguem as cópias depois. Dá para fazer, e o Tessera faz. [long pause]

Mas o modo que ele defende não tem mesa nenhuma. Ninguém tem chave. [slowly] Nem
eu. [pause] As cédulas viajam cifradas para uma hora marcada, e nessa hora um
sorteio público publica a chave que abre todas. Aí qualquer pessoa reconta o
resultado sozinha, cédula por cédula. [long pause]

E abrir uma cédula mostra o que ela votou. [slowly] Nunca de quem ela é.
```

### 4 · demonstração

```
[snappy] Chega de falar. [pause]

Isso aqui é uma votação de verdade, na testnet. Sem mesa nenhuma, sem servidor,
ninguém segurando chave. [pause]

Primeiro a pessoa se identifica. Prova que tem direito de votar e assina a lista
de presença, em público. Todo mundo vê quem apareceu. [pause]

Depois ela vota. De outro endereço, criado na hora, que não tem nada a ver com o
primeiro. A cédula prova que saiu de alguém daquela lista, sem dizer de quem.
[pause]

Olha as duas colunas. [slowly] Nenhum endereço aparece nas duas. [pause] E não é
sorte desta rodada: numa outra, com trinta votantes, a interseção deu exatamente
zero. [long pause]

O prazo vence e o resultado sai. Quem apurou foi uma carteira que nem votou.
[pause] Uma das sete cédulas estava corrompida de propósito: perdeu o próprio
voto, e as outras seis contaram.
```

### 5 · por que Stellar

```
Por que Stellar. [pause] A matemática pesada já vem pronta dentro da Soroban. Eu
não precisei programar a parte difícil. [pause]

Então dá para fazer tudo com prova direta, sem cerimônia de instalação e sem
precisar confiar em quem montou o sistema. [pause]

E ficou barato. Um voto secreto completo custou um centavo e meio de XLM.
```

### 6 · evidência

```
Nada disso é estimativa. Tudo foi medido no contrato que está no ar. [pause]

Um voto secreto completo usa nove por cento do que cabe numa transação. E apurar
não fica mais caro conforme entra gente: de cinco votantes para quarenta, o
custo subiu um e meio por cento. [pause]

[proud] Mas a parte de que eu mais gosto é esta. [pause] Você não precisa
acreditar em mim. Tem um programa separado que baixa tudo da blockchain, refaz
as contas do zero e confere o resultado.
```

### 7 · time

```
Eu sou Thais Reis, engenheira de software no Rio de Janeiro. [pause] Eu construo
aplicação de ponta a ponta: cliente, API, banco, contrato e deploy. [long pause]

[proud] Três primeiros lugares em hackathon. [pause] E Tessera também não é a
minha primeira governança. Eu mantenho o Karn, um protocolo de governança
aberto. [slowly] É exatamente onde ele encaixa.
```

### 8 · fecho

```
Conferir uma decisão não deveria custar expor a escolha de cada um. [pause] Dá
para construir isso. [slowly] Está construído, está rodando, e o código é
aberto. [long pause]

[proud] Se você governa alguma coisa, Tessera encaixa no que você já usa. E se
você mexe com criptografia, o repositório está lá para você quebrar.
[long pause]

A decisão é de todos. [pause] A escolha é sua. [long pause] Tessera.
```

## O que mostrar no bloco do time

O §7 agora é sobre quem você é, e cada frase dele tem uma imagem óbvia por trás.
A tela deve **mostrar o que a voz afirma**, não ilustrar genericamente.

| Enquanto a voz diz | Na tela |
|---|---|
| "engenheira de software no Rio de Janeiro" | você — foto ou câmera |
| "plataforma de ingressos... com gente na porta" | a plataforma rodando, ou uma foto do evento na portaria |
| "três primeiros lugares" | os três troféus, logos ou fotos dos eventos |
| "a Stellar me convidou para o Builder Summit" | o logo da Stellar, ou a foto do Summit |
| "eu mantenho o Karn" | o repositório do Karn |

Se der para preparar **uma** só, prepare a última. O Karn é a que faz o pitch
fechar: o deck já diz que a integração prevista de Tessera é o Karn e o Governor
da OpenZeppelin, então a tela prova que você não está propondo encaixar numa
coisa de terceiros — você mantém as duas pontas.

Se não der para preparar nenhuma, a sua foto com o nome é honesta e suficiente.
O que eu evitaria é o slide 07 com uma linha de assinatura: o fecho perde força
dividindo atenção com crédito de autoria.

> **Nota de honestidade.** Nada obriga você a dizer no vídeo que a narração é
> sintética — é a sua própria voz, clonada por você, e isso é produção, não
> representação falsa. Mas a `§2` do `SPEC.md` existe porque este projeto
> resolveu não ter nenhuma afirmação sem lastro. Uma linha nos créditos finais,
> "narração gerada com um clone da minha voz", custa dois segundos e combina com
> o resto.
