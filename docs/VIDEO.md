# Tessera — roteiro do vídeo do dapp

Este é o roteiro de T-004: o dapp numa janela, `/bastidores` na outra, tudo na
testnet. **Nada encenado** — cada número que aparecer na tela veio de uma
transação que aconteceu enquanto a câmera rodava, ou de uma que está no ledger
e pode ser conferida no explorer.

`docs/DEMO.md` é outro roteiro, do **console**, que mostra o fluxo da mesa. Os
dois não se misturam: o console é o ambiente controlado e o plano B; este é o
produto.

> **O que este vídeo tem a provar, em uma frase:** uma assembleia sem mesa
> nenhuma — sem ninguém de confiança — fecha com placar, e ninguém consegue
> travá-lo.

---

## 0. Antes de ligar a câmera

Uma coisa só precisa estar pronta, e ela leva tempo de relógio:

```bash
cd app && npm run wasm && npm run build && pnpm preview   # ou npm run dev
```

E **abra uma votação de ensaio** com a mesma configuração que você vai usar no
vídeo, para saber quanto tempo cada passo leva na testnet de hoje. Os números
que medimos em 2026-10-10: comparecer confirma em ~6 s, a cédula em ~8 s, e a
apuração em um ledger.

**O prazo é o inimigo do vídeo.** A janela de votação precisa fechar dentro da
gravação, e a rodada da baliza vence um minuto depois dela. Com 2 minutos de
comparecimento e 3 de votação, o placar aparece por volta do minuto 7. Se você
vai falar menos que isso, abra a votação **antes** de gravar e comece o vídeo já
no comparecimento — continua não sendo encenação, porque a transação de abertura
está no ledger e aparece no explorer.

Deixe duas janelas lado a lado: o dapp à esquerda, `/#/bastidores` à direita. O
diário atravessa as janelas, então a segunda mostra o que a primeira fez.

---

## 1. Abrir — 0:00 a 1:10

**Tela:** `/#/abrir`.

Preencha: duas opções, lista com os endereços de ensaio, **uma seção**, mesa
**vazia**, fechadura **ligada**, 2 e 3 minutos.

**O que dizer, enquanto preenche:**

> Uma votação secreta tem um problema que não é de criptografia: quem conta os
> votos. Se existe alguém com a chave, existe alguém de quem o votante precisa
> se proteger. Aqui a mesa está vazia — ninguém vai receber parcela nenhuma.

Pare no bloco da fechadura e **leia a tela em voz alta**, inclusive o custo:

> A cédula sai cifrada para uma rodada de uma baliza pública que vence quando a
> janela fechar. Antes disso ninguém abre, nem eu, que estou abrindo a votação.
> Depois, qualquer pessoa abre. E o que isso custa está escrito aqui: a baliza é
> uma suposição de confiança, e se ela parar, o placar não sai. Não há plano B.

**Não diga** que abrir cedo é impossível. É conluio de um limiar dos operadores
da baliza — gente sem interesse nesta votação, mas gente. A tela já diz isso
certo; basta ler.

**Na segunda janela:** o diário mostra a raiz de aptos, a rodada da fechadura e
o custo em instruções que a simulação descobriu.

---

## 2. Comparecer — 1:10 a 2:30

**Tela:** `/#/votacao/<id>` → "Confirmar presença".

> Este ato é identificado e público, e é de propósito: é daqui que sai a lista
> de quem faltou. Sem isso não existe voto obrigatório.

Mostre o endereço aparecendo no caderno. **Na segunda janela**, o diário mostra
a chave de participação nascendo *nesta aba* — e é a única cópia dela.

---

## 3. Votar — 2:30 a 4:00

**Tela:** `/#/votar/<id>`.

Abra o "Sobre a privacidade do voto" antes de confirmar, e leia o parágrafo da
fechadura. Depois confirme.

> A cédula vai ser assinada por uma chave que nasceu agora e que não tem relação
> com o meu endereço. O número que esconde o meu voto nasce nesta aba e sai
> daqui cifrado — nunca em claro, e nunca para um servidor, porque não existe
> servidor: esta página é um arquivo estático.

**Na segunda janela:** o custo da cédula em instruções e a porcentagem do teto.
É o momento de dizer o número medido:

> Uma cédula com anel de dez custa 148.911.943 instruções, 37,2% do teto de uma
> transação. Medido, não estimado.

---

## 4. A espera — 4:00 a 5:30

**Tela:** `/#/apurar/<id>`, com a votação ainda aberta.

Este é o painel mais importante do vídeo, e é um painel que **não mostra nada**.

> Não há placar. E não há nem parcial. A tela não está escondendo um número que
> ela tem: a chave que abre as cédulas ainda não foi publicada pela baliza. Nem
> eu tenho.

Mostre o contador de ledgers correndo. Se sobrar tempo, é aqui que cabe explicar
o anel e as seções.

---

## 5. O placar que aparece sozinho — 5:30 a 7:00

Quando a janela fechar, recarregue. Se a rodada ainda não venceu, a tela diz
isso — **mostre esse estado também**, porque são dois portões diferentes e é o
que prova que o sigilo não depende da baliza:

> Agora a urna fechou e o contrato continua recusando, por outro motivo: a
> rodada ainda não venceu. São dois relógios, e o primeiro deles é o do ledger,
> que não depende de suposição nenhuma.

Recarregue quando o contador zerar. **Não clique em nada**: o placar aparece.

> Ninguém afirmou estes números. Eles saíram da decifragem das cédulas, neste
> navegador, com uma assinatura que a baliza publicou para todo mundo ao mesmo
> tempo. E o contrato conferiu a soma antes de gravar.

---

## 6. O fecho — 7:00 a 7:30

Mostre a transação no explorer, e termine na frase que o projeto tem direito de
dizer:

> Não teve mesa. Não teve ninguém de confiança. E se alguém tivesse tentado
> travar o placar omitindo cédulas, qualquer pessoa — inclusive quem não votou —
> o sobreporia incluindo as que faltam. Isso rodou na testnet, e os hashes estão
> no repositório.

---

## O que **não** dizer

De `docs/SPEC.md §2`, e vale literalmente:

- não diga "seguro", "auditado", "pronto para produção" — nada foi auditado;
- não diga que abrir o conteúdo antes da hora é impossível;
- não chame a baliza de "sem confiança" nem de "trustless";
- não prometa resultado se a baliza parar;
- não diga que o dapp está publicado, enquanto não estiver;
- não cite número que não esteja em `docs/SOURCES.md`.

## Se algo falhar ao vivo

**Deixe falhar e diga o que é.** Uma cédula recusada por taxa é o leilão de
inclusão da rede, não o contrato — e o cliente reenvia. Mostrar isso é melhor
que cortar: nove de dez demos de hackathon só mostram o caminho feliz.

O plano B inteiro é `console/`, com `docs/DEMO.md`.
