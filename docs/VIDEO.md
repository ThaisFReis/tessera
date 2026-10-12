# Tessera — roteiro do vídeo do dapp

Roteiro de T-004: o dapp numa janela, `/#/bastidores` na outra, tudo na testnet.
**Nada encenado** — cada número que aparecer veio de uma transação que aconteceu
enquanto a câmera rodava, ou de uma que está no ledger e pode ser conferida no
explorer agora.

`docs/DEMO.md` é outro roteiro, do **console**, que mostra o fluxo da mesa. Os
dois não se misturam: o console é o ambiente controlado e o plano B; este é o
produto.

> **O que este vídeo tem a provar, em uma frase:** uma assembleia sem mesa
> nenhuma — sem ninguém de confiança — fecha com placar, ninguém consegue
> travá-lo, e qualquer pessoa reconfere tudo do zero.

Duração alvo: **9 minutos** com `LEDGERS_URNA=90`, ou **7** com `65`. O §6 é a
única parte dispensável, e é a mais forte — corte antes o tempo de fala do §1.

---

## 0. Antes de ligar a câmera

### 0.1 Por que existe um passo de preparo

Desde T-026 o placar em claro só sai com **cinco cédulas abertas** no agregado —
é o piso de anonimato, e ele é do contrato, não da tela. **Um vídeo com um
votante só nunca produz placar.** A recusa estaria certa e o vídeo, inútil.

Então o eleitorado é real e já está no ledger antes de a câmera ligar. Seis
pessoas comparecendo e votando não cabem em oito minutos de vídeo, e fingir que
cabem seria encenação. O que acontece diante da câmera é a **sua** cédula, a
sétima, e a apuração de todas.

Isso não é um atalho escondido: o §2 do vídeo diz isso em voz alta, e as
transações das outras seis estão no explorer.

### 0.2 O preparo

```bash
cd app && npm run wasm && npm run build && npm run preview
```

Pegue o seu endereço do Freighter na testnet — o que vai aparecer no vídeo — e
semeie a votação:

```bash
cd app
SEMEAR=1 N=6 SABOTADA=2 \
CONVIDADO=G...SEU_ENDERECO \
LEDGERS_CADERNO=30 LEDGERS_URNA=90 \
node scripts/rodada-relogio.mjs
```

Ele abre a votação sem mesa, com fechadura, financia seis eleitores, faz os seis
comparecerem e depositarem cédula — **uma delas sabotada de propósito** — e
**para**, com a urna aberta. No fim ele imprime:

- o identificador da proposta e o contrato;
- em que ledger a votação fecha, e quantos minutos faltam;
- a rodada da baliza e quando ela vence;
- **a lista de aptos**, para você colar no dapp. Você é o último da lista.

### 0.2.1 O relógio, medido

Rodado em 2026-10-11 com `LEDGERS_URNA=90`, o script terminou e imprimiu:

```
votação até   ledger 5148979 (faltam 76, ~6 min)
rodada        32988393, vence em 439s
cédulas       6 depositadas · 5 vão abrir
```

Ou seja, **a partir do instante em que o script termina**: a urna fecha em
~6 min e o placar fica possível em **~7 min 20 s**. A semeadura consome 14 dos
90 ledgers, e é por isso que o número impresso é menor que o configurado — use
sempre o que ele imprime, não a conta.

Isso põe o placar por volta do **minuto 7** do vídeo, se você ligar a câmera
assim que o script parar. Os tempos do roteiro abaixo assumem isso.

Se quiser um vídeo de 7 minutos em vez de 9, use `LEDGERS_URNA=65`: o placar
cai para ~5 min 20 s. É mais apertado — se você demorar no §1 ou no §2, a urna
fecha antes de a sua cédula entrar.

**Ensaie uma vez antes.** Não pelo texto — pelo tempo. Medidos em 2026-10-10:
comparecer confirma em ~6 s, a cédula em ~8 s, a apuração em um ledger.

### 0.2.2 O que foi conferido, e o que não foi

A lista impressa pelo script foi colada no mesmo parser que o dapp usa, e a raiz
de Merkle recalculada dela bate **byte a byte** com a que está no ledger:

```
raiz local ..... b8684a9e26580bfdc74237f388aedc75ad859c01d9282121693e423918f95ba6
raiz no ledger . b8684a9e26580bfdc74237f388aedc75ad859c01d9282121693e423918f95ba6
convidado ...... índice 6 · caminho com 3 irmãos
```

**O que não foi testado:** a assinatura no Freighter, porque exige a sua
carteira. É o único passo do §2 que pode surpreender ao vivo. Faça um ensaio
com uma semeadura descartável antes de gravar — e se o Freighter recusar, o
diário da segunda janela diz por quê.

### 0.3 As duas janelas

Dapp à esquerda, `/#/bastidores` à direita. O diário atravessa as janelas, então
a segunda mostra o que a primeira fez — e é onde os custos em instruções
aparecem.

---

## 1. O problema, e a tela que o declara — 0:00 a 1:15

**Tela:** `/#/abrir`, preenchida e **não enviada**. É a tela que abriu a votação
que vamos usar, com a mesma configuração.

> Uma votação secreta tem um problema que não é de criptografia: quem conta os
> votos. Se existe alguém com a chave, existe alguém de quem o votante precisa
> se proteger. Aqui a mesa está vazia — ninguém vai receber parcela nenhuma, em
> nenhum momento.

Pare no bloco da fechadura e **leia a tela em voz alta, inclusive o custo**:

> A cédula sai cifrada para uma rodada de uma baliza pública que vence quando a
> janela fechar. Antes disso ninguém abre, nem eu, que abri a votação. Depois,
> qualquer pessoa abre. E o que isso custa está escrito aqui: a baliza é uma
> suposição de confiança, e se ela parar, o placar não sai. Não há plano B.

**Não diga** que abrir cedo é impossível. É conluio de um limiar dos operadores
da baliza — gente sem interesse nesta votação, mas gente. A tela já diz isso
certo; basta ler.

Então, sem cortar, a frase que torna o resto honesto:

> Esta votação já está aberta há alguns minutos, e seis pessoas já votaram
> nela. São seis carteiras reais, e as transações estão no explorer. Seis
> pessoas votando não cabem em oito minutos — então elas já votaram, e eu sou a
> sétima.

---

## 2. A lista, e o caderno — 1:15 a 2:30

**Tela:** `/#/votacao/<id>` → "Confirmar presença".

A tela pede a lista de aptos. **Cole a que o script imprimiu** — e diga por que
ela existe:

> Esta lista é pública, e é o organizador que a entrega. O contrato não guarda
> os nomes: guarda a raiz de uma árvore de Merkle, e o que eu mando é a prova de
> que estou nela. Conferir isso custa 127.863 instruções.

Confirme a presença.

> Este ato é identificado e público, e é de propósito: é daqui que sai a lista
> de quem faltou. Sem isso não existe voto obrigatório. O que ele **não** diz é
> em que eu vou votar.

**Na segunda janela:** o diário mostra a chave de participação nascendo *nesta
aba*. Diga que é a única cópia:

> Esta chave não foi para servidor nenhum, porque não existe servidor — esta
> página é um arquivo estático. Se eu fechar esta aba, perco o voto.

---

## 3. A cédula — 2:30 a 3:45

**Tela:** `/#/votar/<id>`.

Abra "Sobre a privacidade do voto" antes de confirmar e leia o parágrafo da
fechadura. Depois confirme.

> A cédula vai ser assinada por uma chave que nasceu agora e que não tem relação
> com o meu endereço — é uma assinatura em anel, e ela prova que eu sou um dos
> que compareceram sem dizer qual. O número que esconde o meu voto nasce nesta
> aba e sai daqui cifrado, nunca em claro.

**Na segunda janela:** o custo da cédula e a porcentagem do teto.

> Uma cédula com anel de dez custa 148.911.943 instruções, 37,2% do teto de uma
> transação. Medido, não estimado — e é por isso que existem seções.

---

## 4. A espera — 3:45 a 6:45

**Tela:** `/#/apurar/<id>`, com a votação ainda aberta.

O painel mais importante do vídeo, e é um painel que **não mostra nada**.

> Não há placar. E não há nem parcial. A tela não está escondendo um número que
> ela tem: a chave que abre as cédulas ainda não foi publicada pela baliza. Nem
> eu tenho.

**Esta é a parte elástica do vídeo**, e é onde sobra tempo de propósito: são
uns três minutos até a urna fechar e mais um até a rodada vencer. É onde cabe
explicar o anel, as seções, e por que o compromisso é perfeitamente ocultante.
Mostre o contador de ledgers correndo enquanto fala.

Quando a janela fechar, recarregue. Se a rodada ainda não venceu, a tela muda —
**mostre esse estado também**, porque são dois portões diferentes:

> Agora a urna fechou e o contrato continua recusando, por outro motivo: a
> rodada ainda não venceu. São dois relógios, e o primeiro é o do ledger, que
> não depende de suposição nenhuma sobre a baliza.

---

## 5. O placar que aparece sozinho — 6:45 a 7:45

Recarregue quando o contador zerar. **Não clique em nada.**

A tela passa por "Lendo as cédulas dos eventos do contrato", "Decifrando N
cédulas neste navegador", "Gravando o placar no ledger" — e o placar aparece.

Esta é a parte que mudou, e vale narrar os dois passos:

> Primeiro o navegador leu as cédulas dos eventos, decifrou cada uma com a
> assinatura que a baliza publicou para todo mundo ao mesmo tempo, e mandou para
> o contrato a lista e quais abriram. O contrato guardou uma contagem e **um
> ponto** — a soma dos compromissos. Nenhum número de voto.
>
> O placar veio de uma segunda transação, sobre o eleitorado inteiro. É o único
> lugar do ledger em que um número de voto existe. E o contrato só grava depois
> de conferir que aquela soma abre: um placar inventado é recusado aqui.

Se uma cédula não abriu — e uma não vai abrir, porque o preparo sabotou uma de
propósito — a tela diz "N de M cédulas abriram". Use:

> Uma das cédulas tem um byte trocado. Ela custou um voto, e só o dela. O
> contrato aceitou aquela cédula quando ela entrou, porque ele confere a
> **forma** do criptograma e não pode conferir o conteúdo — ele não decifra, e
> não precisa.

---

## 6. O cético — 7:45 a 8:45  *(corte aqui se precisar encurtar)*

Numa terceira janela, um terminal. **Este é o beat mais forte do vídeo**, porque
é o único em que nada do que está na tela é produto nosso rodando:

```bash
tessera verificar --proposta <id> --contrato CDWGDWDPCXAAA3FO24NRRYS77HEMFHWDIBK4O3GB6L6BQGYWC7JYW3YS
```

> Isto não é o dapp. É um programa separado que não lê arquivo nenhum do meu
> computador. Ele pega os compromissos dos eventos do contrato, re-encadeia a
> lista contra o que a cadeia registrou, confere que as imagens de chave são
> todas distintas — que é o voto duplo, detectado sem saber de quem é nenhuma
> cédula —, **refaz cada assinatura em anel** a partir dos argumentos da
> transação que depositou a cédula, decifra cada criptograma contra a rodada da
> baliza e confere que cada compromisso abre em zero ou um. Só então compara o
> placar que ele recontou com o que está publicado.

E a última linha da saída, que é o ponto:

> E o que este verificador NÃO conseguiu descobrir: de quem é cada uma das sete
> cédulas, nem em que alguém votou. Ele tentou tudo, provou o que podia, e
> relata o que não conseguiu.

Se alguma assinatura ficar fora da janela de retenção do RPC, ele diz quantas —
**não corte isso**. É a diferença entre um verificador e um carimbo.

---

## 7. O fecho — 8:45 a 9:00

Mostre a transação no explorer e termine:

> Não teve mesa. Não teve ninguém de confiança. Se alguém tivesse tentado travar
> o placar omitindo cédulas, qualquer pessoa — inclusive quem não votou — o
> sobreporia incluindo as que faltam. Isso rodou na testnet, os hashes estão no
> repositório, e o verificador que acabou de rodar está lá também.

---

## O que **não** dizer

De `docs/SPEC.md §2`, e vale literalmente:

- não diga "seguro", "auditado", "pronto para produção" — nada foi auditado;
- não diga que abrir o conteúdo antes da hora é impossível;
- não chame a baliza de "sem confiança" nem de "trustless";
- não prometa resultado se a baliza parar;
- não diga que o dapp está publicado, enquanto não estiver;
- não diga que a seção de alguém é secreta — ela é pública, e o que se pode
  dizer é que é **imprevisível até a abertura**;
- não cite número que não esteja em `docs/SOURCES.md`.

E o que **passou a ser dizível**, que não era antes de T-026 e T-025:

- a seção **não publica** o seu total, só uma contagem e um ponto — então uma
  seção unânime não denuncia os seus membros. O que continua público é o placar
  agregado, como em qualquer votação;
- o verificador independente cobre o **caminho anônimo**, com o limite da janela
  de retenção do RPC dito junto.

## Se algo falhar ao vivo

**Deixe falhar e diga o que é.** Uma cédula recusada por taxa é o leilão de
inclusão da rede, não o contrato — e o cliente reenvia. Mostrar isso é melhor
que cortar: nove de dez demos de hackathon só mostram o caminho feliz.

Se a janela fechar antes de você votar, você ainda tem vídeo: as seis cédulas
semeadas abrem, o placar sai, e o verificador roda. O que se perde é a sua
cédula aparecendo, não a demonstração.

O plano B inteiro é `console/`, com `docs/DEMO.md`.
