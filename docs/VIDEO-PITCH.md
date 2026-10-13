# Tessera — roteiro do vídeo de pitch (3 minutos)

Este é o **vídeo de pitch**, o que acompanha a submissão e o deck de sete
slides. Ele não é o `docs/VIDEO.md`: aquele é a **demonstração de nove
minutos**, do comparecimento ao placar, em tempo real e sem corte. Os dois
convivem — o pitch convence a assistir, a demonstração prova.

> **O que este vídeo tem a provar, em uma frase:** existe um jeito de uma
> governança votar em segredo e ainda assim qualquer pessoa reconferir o
> resultado — e ele está de pé, medido, na testnet.

Duração alvo: **3 minutos**. A narração tem **539 palavras** na versão que vai
ao sintetizador, o que dá **3min22** a 160 palavras por minuto, sem contar as
pausas. Se o limite for duro em 3min exatos, corte o §2 para **um** exemplo em
vez de três (−18s) e tire as pausas do §4 (−6s). Nunca corte o §4 em texto.

---

## Regra de ouro

**Nenhum número que não tenha sido medido, nenhuma palavra que a `§2` do
`SPEC.md` proíba.** Em particular, este roteiro nunca diz "seguro", "auditado",
"pronto para produção" nem "trustless", e nunca promete resultado se a baliza
parar.

### O que conferir na véspera da gravação

- [ ] **O dapp responde.** No dia 13/10 `thaisfreis.github.io/tessera/dapp/`
      devolvia **404** — o PR #23 não foi mergeado, e o Pages serve o `master`.
      Enquanto estiver 404, o §4 **não pode** dizer "está no ar": troque por
      "roda local, e o contrato está no explorer". O deck tem o mesmo problema
      no slide 04.
- [ ] O contrato responde no explorer.
- [ ] A rodada que você vai mostrar está na janela de retenção do RPC, se for
      mostrar o verificador.

---

## A estrutura, slide a slide

| Bloco | Slide | Tempo | Tela |
|---|---|---|---|
| 1 Gancho | 01 capa | 0:00–0:11 | você, ou a capa |
| 2 Problema | 02 | 0:11–0:35 | slide |
| 3 Desenho | 03 | 0:35–1:05 | slide |
| 4 Demonstração | 04 | 1:05–1:52 | **captura de tela** |
| 5 Por que Stellar | 05 | 1:52–2:10 | slide |
| 6 Evidência | 06 | 2:10–2:37 | slide |
| 7 Time | — | 2:37–2:55 | você |
| 8 Fecho | 07 | 2:55–3:16 | slide |

O §7 não tem slide no deck. Ou você fala em câmera, ou acrescenta uma linha de
assinatura no slide 07 — veja a nota no fim.

---

## 1. O gancho — 0:00 a 0:11

**Tela:** você em câmera, ou a capa. Nada mais.

> Uma urna em que ninguém — nem quem a abriu — consegue ligar uma cédula a
> quem a depositou.
>
> Isso é Tessera, e está rodando agora na testnet da Stellar.

**Direção:** é a primeira frase e é a única chance. Fale devagar, sem
preâmbulo, sem "oi, meu nome é". O nome vem no §7.

---

## 2. O problema — 0:11 a 0:35

**Tela:** slide 02.

> Em governança on-chain, o voto é público. E o risco é pessoal.
>
> O conselheiro que precisa votar contra o orçamento de quem ele depende. O
> cooperado que discorda da diretoria que decide o crédito dele. O
> contribuidor que vota contra a baleia que financia o trabalho dele.
>
> Nome, mais escolha, mais registro permanente, dá exposição permanente. O
> sigilo protege a liberdade de discordar.

**Direção:** os três exemplos são o coração do bloco. Se tiver que cortar
tempo, fique com **um** — o do conselheiro — e não com os três resumidos.

---

## 3. O desenho — 0:35 a 1:05

**Tela:** slide 03.

> A saída não é cifrar o voto e guardar a chave — o texto cifrado fica
> registrado para sempre, e uma chave que vaze no futuro expõe uma escolha do
> passado.
>
> Então na rede vai um compromisso de Pedersen: perfeitamente ocultante,
> isolado não revela nada, e mesmo assim os compromissos somam.
>
> Quem abre uma cédula? Sem mesa e sem fechadura, ninguém, nunca. Com
> fechadura de tempo, qualquer pessoa, depois que a baliza drand publica a
> rodada. Cifrado até as 20:59, não cifrado para sempre.

**Direção:** "cifrado até as 20:59, não cifrado para sempre" é a frase que
costuma ser citada de volta. Pause depois dela.

---

## 4. A demonstração — 1:05 a 1:52

**Tela:** captura do dapp. É o bloco mais longo de propósito, e é o único que
não pode encolher.

> Chega de falar.
>
> Votação real na testnet, **sem mesa nenhuma** — ninguém tem chave.
>
> Primeiro o comparecimento: a pessoa prova que está na lista e entra no
> caderno com o endereço dela, identificada, em público.
>
> Depois a cédula sai de uma chave de uso único, com uma assinatura em anel —
> ela prova que quem assinou compareceu, sem dizer qual.
>
> Nada no ledger liga as duas colunas. Numa rodada de trinta votantes, a
> interseção entre o caderno e a urna foi **zero**.
>
> A janela fecha, a rodada da baliza vence, e o placar aparece. Quem apurou
> foi uma carteira que nem votou, e não pediu permissão a ninguém. Uma das
> sete cédulas tinha um byte trocado de propósito: custou um voto, e só o
> dela.

**Direção:** se houver tempo para mostrar **uma** coisa, mostre o placar
aparecendo. Mostrar o verificador rodando é melhor ainda, mas só se couber.

**Cuidado:** não diga "sem confiança" nem "trustless". Diga "sem mesa" e
"ninguém tem chave", que é o que é verdade.

---

## 5. Por que Stellar — 1:52 a 2:10

**Tela:** slide 05.

> Por que Stellar: a BLS12-381 já é nativa no host da Soroban.
>
> Não precisei reimplementar a curva em WebAssembly. São provas sigma com
> Fiat–Shamir: sem setup confiável, sem circuito, sem cerimônia.
>
> E o custo cabe no bolso de quem vota — um voto confidencial custou **0,0157
> XLM** na testnet.

---

## 6. A evidência — 2:10 a 2:37

**Tela:** slide 06, o lima.

> Nenhum número aqui é estimativa.
>
> Um voto confidencial, com duas provas de conhecimento zero verificadas
> dentro do contrato, usa **9,2% do teto** de uma transação. Dez vezes de
> folga.
>
> A apuração não cresce com o comparecimento: um e meio por cento a mais para
> oito vezes mais gente.
>
> E qualquer pessoa reconfere do zero — um verificador independente refaz as
> assinaturas em anel direto do ledger, sem ler arquivo nenhum da minha
> máquina.

**Direção:** o slide é o único de fundo claro do deck. Deixe-o entrar com um
corte seco — o contraste faz o trabalho.

---

## 7. O time — 2:37 a 2:55

**Tela:** você em câmera.

> Quem construiu isso: eu, Thais Reis, sozinha. Doze dias, cento e vinte e
> dois commits, do protocolo ao dapp. Cento e noventa e cinco testes passando.
> Licença MIT.
>
> Eu não tenho uma equipe para mostrar. Tenho um repositório inteiro, público
> desde o primeiro commit.

**Direção:** a última frase transforma a fraqueza aparente em evidência de
execução. Diga-a olhando para a câmera, sem se desculpar.

> **Confirme antes de gravar:** o roteiro assume **solo**, porque o histórico
> do repositório tem um autor só. Se houver mais alguém — design, consultoria,
> revisão — me diga e eu reescrevo este bloco. Nomear quem ajudou é melhor que
> parecer solo por omissão.

---

## 8. O fecho — 2:55 a 3:16

**Tela:** slide 07.

> O sigilo tem condições, e elas estão escritas: a baliza, quando há
> fechadura; e um grupo grande o bastante para esconder alguém. Nada disso foi
> auditado, e eu não vou dizer que foi.
>
> Mas conferir uma decisão coletiva não deveria exigir expor a escolha
> individual.
>
> A decisão é de todos. A escolha é sua.
>
> Tessera.

**Direção:** deixe o slide 07 na tela por dois segundos depois da última
palavra, com o endereço do repositório visível. Não leia a URL em voz alta.

---

## Depois de gravar

1. Insira a URL no slide 04 do deck, nos **dois** idiomas — ela está marcada
   como `URL a inserir antes da submissão`.
2. Insira a URL no README.
3. Se o §4 teve que ser gravado com o dapp ainda em 404, conserte a linha "dapp
   no ar" do slide 04 antes de submeter, ou mergeie o PR #23.

---

## O que este roteiro deliberadamente não diz

- Que o projeto é seguro, auditado ou pronto para produção.
- Que abrir o conteúdo antes da hora é impossível — é conluio de um limiar dos
  operadores da baliza.
- Que a seção de uma pessoa é secreta — ela é **imprevisível até a abertura**,
  que é coisa diferente.
- Que existe plano B se a baliza parar. Não existe.
- Qualquer número que não tenha vindo de uma medição registrada em
  `docs/SOURCES.md`.

---

# Narração para voz clonada (ElevenLabs v4)

A narração vai ser gerada com um clone da minha voz. Isso muda três coisas, e é
por isso que esta seção existe em vez de você colar os blocos de cima direto no
sintetizador.

**Um:** o sintetizador lê o que está escrito, não o que está formatado. Negrito,
travessão duplo e markdown ou são lidos em voz alta ou atrapalham a prosódia.
Os blocos abaixo são texto puro.

**Dois:** número escrito em algarismo é uma aposta. `0,0157` pode sair como
"zero vírgula zero cento e cinquenta e sete". Abaixo, **todo número está por
extenso** — o valor medido é o mesmo, só a grafia mudou.

**Três:** sem rosto na tela, o bloco do time perdeu o apoio visual que ele
tinha. Veja a nota no fim desta seção.

> As etiquetas entre colchetes são do v4 e o seu print as confirma, mas eu não
> testei nenhuma delas com a sua voz. Gere um bloco curto primeiro — sugiro o
> §1 — e só depois os sete restantes. Se alguma etiqueta vazar como palavra
> falada, tire todas: o texto funciona sem elas.

## Como cortar

Gere **um arquivo por bloco**, não um único áudio de três minutos. Três razões:
o corte entre blocos fica no silêncio e não no meio da respiração; você refaz um
bloco ruim sem regerar o resto; e o §4 precisa esticar ou encolher para casar
com a captura de tela, o que é impossível num arquivo só.

## Palavras para conferir no primeiro teste

Estas são as que o sintetizador pode errar em português. Se errar, a grafia
alternativa está ao lado — troque só se precisar.

| No texto | Se sair errado, escreva |
|---|---|
| Stellar | Stélar |
| Soroban | Sorobã |
| Pedersen | Pedérsen |
| drand | dê-rand |
| dapp | déppi |
| Fiat Shamir | Fíat Shamír |
| XLM | xis ele eme |
| WebAssembly | Uébi Assêmbli |

## Os blocos

### 1 · gancho

```
Uma urna em que ninguém, nem quem a abriu, consegue ligar uma cédula a quem a
depositou. [pause] Isso é Tessera, e está rodando agora na testnet da Stellar.
```

### 2 · problema

```
Em governança on-chain, o voto é público. E o risco é pessoal. [pause]

O conselheiro que precisa votar contra o orçamento de quem ele depende. O
cooperado que discorda da diretoria que decide o crédito dele. O contribuidor
que vota contra a baleia que financia o trabalho dele. [pause]

Nome, mais escolha, mais registro permanente, dá exposição permanente. O sigilo
protege a liberdade de discordar.
```

### 3 · desenho

```
A saída não é cifrar o voto e guardar a chave. O texto cifrado fica registrado
para sempre, e uma chave que vaze no futuro expõe uma escolha do passado.
[pause]

Então na rede vai um compromisso de Pedersen: perfeitamente ocultante, isolado
não revela nada, e mesmo assim os compromissos somam. [pause]

Quem abre uma cédula? Sem mesa e sem fechadura, ninguém, nunca. Com fechadura
de tempo, qualquer pessoa, depois que a baliza drand publica a rodada. [pause]
Cifrado até as vinte e cinquenta e nove. Não cifrado para sempre.
```

### 4 · demonstração

```
Chega de falar. [pause]

Votação real na testnet, sem mesa nenhuma. Ninguém tem chave. [pause]

Primeiro o comparecimento: a pessoa prova que está na lista e entra no caderno
com o endereço dela, identificada, em público. [pause]

Depois a cédula sai de uma chave de uso único, com uma assinatura em anel. Ela
prova que quem assinou compareceu, sem dizer qual. [pause]

Nada no ledger liga as duas colunas. Numa rodada de trinta votantes, a
interseção entre o caderno e a urna foi zero. [pause]

A janela fecha, a rodada da baliza vence, e o placar aparece. Quem apurou foi
uma carteira que nem votou, e não pediu permissão a ninguém. [pause] Uma das
sete cédulas tinha um byte trocado de propósito. Custou um voto, e só o dela.
```

### 5 · por que Stellar

```
Por que Stellar: a B L S doze traço trezentos e oitenta e um já é nativa no
host da Soroban. [pause]

Não precisei reimplementar a curva em WebAssembly. São provas sigma com Fiat
Shamir: sem setup confiável, sem circuito, sem cerimônia. [pause]

E o custo cabe no bolso de quem vota. Um voto confidencial custou zero vírgula
zero um cinco sete XLM na testnet.
```

### 6 · evidência

```
Nenhum número aqui é estimativa. [pause]

Um voto confidencial, com duas provas de conhecimento zero verificadas dentro
do contrato, usa nove vírgula dois por cento do teto de uma transação. Dez
vezes de folga. [pause]

A apuração não cresce com o comparecimento: um e meio por cento a mais para
oito vezes mais gente. [pause]

E qualquer pessoa reconfere do zero. Um verificador independente refaz as
assinaturas em anel direto do ledger, sem ler arquivo nenhum da minha máquina.
```

### 7 · time

```
Quem construiu isso: eu, Thais Reis, sozinha. [pause] Doze dias, cento e vinte
e dois commits, do protocolo ao dapp. Cento e noventa e cinco testes passando.
Licença MIT. [pause]

Eu não tenho uma equipe para mostrar. [pause] Tenho um repositório inteiro,
público desde o primeiro commit.
```

### 8 · fecho

```
O sigilo tem condições, e elas estão escritas: a baliza, quando há fechadura, e
um grupo grande o bastante para esconder alguém. [pause] Nada disso foi
auditado, e eu não vou dizer que foi. [pause]

Mas conferir uma decisão coletiva não deveria exigir expor a escolha
individual. [pause]

A decisão é de todos. A escolha é sua. [pause] Tessera.
```

## O bloco do time, sem rosto

O §7 foi escrito para ser dito olhando para a câmera: "eu não tenho uma equipe
para mostrar" é uma frase que se sustenta no olho, não no texto. Com voz
clonada e sem rosto, ela fica no vazio.

Três saídas, da que eu faria para a que eu não faria:

1. **O repositório na tela, rolando.** `git log` correndo enquanto a frase é
   dita. Os cento e vinte e dois commits deixam de ser um número e viram uma
   imagem — é exatamente o que a frase promete.
2. **Uma foto sua, estática, com o nome.** Honesto e suficiente. É o que um
   deck faria.
3. **O slide 07 do deck com uma linha de assinatura.** Funciona, mas o fecho
   perde força por dividir a atenção com crédito de autoria.

Eu recomendo a primeira: ela é a única em que a tela prova o que a voz afirma.

> **Nota de honestidade.** Nada obriga você a dizer no vídeo que a narração é
> sintética — é a sua própria voz, clonada por você, e isso é produção, não
> representação falsa. Mas a `§2` do `SPEC.md` existe porque este projeto
> resolveu não ter nenhuma afirmação sem lastro. Uma linha nos créditos finais,
> "narração gerada com um clone da minha voz", custa dois segundos e combina
> com o resto.
