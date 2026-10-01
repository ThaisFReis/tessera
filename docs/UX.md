# Tessera — Especificação de UX/UI

**Autor:** Kaan (UX Designer) · **Data:** 2026-09-30
**Para:** a demo de submissão do hackathon, congelamento 2026-10-04 12:00
**Leia antes:** [SPEC.md](SPEC.md) §1 (modelo de ameaça), §6.5–6.6 (semi-confidencial), [PLANO.md](PLANO.md) §4 (cortes)

---

## 1. O problema de design

**O valor deste produto é uma ausência.**

Você vota, e nada aparece. A promessa inteira é "ninguém descobre seu voto" — e isso é, por construção, invisível. Um aplicativo comum mostra a você o que você fez. O Tessera existe para não mostrar a ninguém o que você fez, inclusive a você mesma, depois.

Na linguagem de Norman: o sistema não tem **feedback** natural para sua propriedade mais importante, e não tem **significante** algum para segurança. E ninguém se sente protegido por uma ausência.

Isso tem uma consequência dura e uma oportunidade.

A consequência: se a interface não fizer nada, a pessoa vota e sai sentindo que usou um formulário qualquer. A confiança não se forma. E numa urna, confiança não é um atributo agradável — é o produto.

A oportunidade: **o sigilo pode ser mostrado sem ser quebrado.** O compromisso de Pedersen é um objeto que a pessoa pode olhar. Ele parece ruído. Esse ruído *é* a proteção. Dá para colocar na tela.

> **A diretriz que organiza tudo neste documento:** tornar o sigilo legível, nunca descrevê-lo. Não escreva "seu voto está seguro". Mostre os 96 bytes que a rede vai guardar para sempre, e deixe a pessoa trocar a escolha e ver que o objeto muda por inteiro sem nunca mudar de forma.

---

## 2. Para quem

### Persona primária — Dona Marta

54 anos, conselheira fiscal de uma cooperativa de crédito no interior. Vai votar sobre aprovar ou rejeitar as contas da diretoria. Ela quer rejeitar.

O presidente da diretoria aprovou o empréstimo do filho dela no ano passado.

Dona Marta usa WhatsApp todo dia, resolve Pix sem ajuda, e nunca ouviu as palavras "compromisso criptográfico". Ela não quer entender a matemática. Ela quer **saber se é seguro votar não**, e essa pergunta não é técnica — é sobre o presidente descobrir ou não.

Em assembleia de papel, ela se sentiria segura porque viu a urna fechada e viu as cédulas queimarem. Na tela, ela não viu nada.

> O que Dona Marta precisa, em ordem:
> 1. **Entender que o voto dela não vai aparecer em lugar nenhum.** Não por promessa, por evidência.
> 2. **Conferir que o voto entrou na conta.** Senão o sigilo é indistinguível de voto jogado no lixo.
> 3. **Não carregar um recibo que a incrimine.** Ela não sabe que isso é um risco. A interface tem que saber por ela.

O item 3 é o mais importante e o menos óbvio. Ver §5.2.

### Persona secundária — o operador da assembleia

Abre a votação, acompanha o comparecimento, chama a apuração. Está com pressa, com a sala cheia, e precisa saber duas coisas: quantos já votaram e se dá para apurar. Não precisa de dashboard. Precisa de dois números grandes.

### Persona secundária — o cético

O conselheiro que acha que votação eletrônica é maquiagem. **No vídeo da submissão, o jurado É o cético.** É para ele que a tela de verificação existe, e é o único que vai ler o resultado do verificador linha por linha. Desenhe para ele e você desenha para o jurado.

---

## 3. Os três momentos

Toda a experiência cabe em três momentos. Não são "telas de um app": são os três instantes em que a confiança se forma ou se perde.

```
   ┌─────────────┐      ┌─────────────┐      ┌─────────────┐
   │   CÉDULA    │ ───▶ │ CONFERÊNCIA │ ───▶ │  APURAÇÃO   │
   │             │      │             │      │             │
   │ escolher e  │      │ provar que  │      │ ver o total │
   │ ver o que a │      │ entrou, e   │      │ fechar, e   │
   │ rede vê     │      │ queimar o   │      │ a mentira   │
   │             │      │ recibo      │      │ ser recusada│
   └─────────────┘      └─────────────┘      └─────────────┘
     Dona Marta           Dona Marta           o cético
```

---

## 4. Momento 1 — Cédula

### 4.1 O que a pessoa faz

Escolhe. E, pela primeira vez neste protocolo, **escolhe também se quer sigilo** (§6.5 do spec).

### 4.2 Layout

```
┌──────────────────────────────────────────────────────────────┐
│  COOPERATIVA X · ASSEMBLEIA ORDINÁRIA              encerra   │
│                                                     em 2h14  │
│  Aprovar as contas da diretoria                              │
│  do exercício de 2025?                                       │
│                                                              │
│  ┌────────────────────┐  ┌────────────────────┐              │
│  │                    │  │       ●            │              │
│  │     APROVAR        │  │     REJEITAR       │              │
│  │                    │  │                    │              │
│  └────────────────────┘  └────────────────────┘              │
│                                                              │
│  ────────────────────────────────────────────────────────    │
│                                                              │
│  O QUE A REDE VAI GUARDAR, PARA SEMPRE                       │
│                                                              │
│  0x0f1ff9fcda6e556ba2da8fb40898b251409d204c79b6bd36          │
│  4f55abbae01df25324d04d96e14e796236af63e89f1d7399            │
│  0b4960881cd7f3ef2e2c36a0d374d56b4d1dcf6a6e68907             │
│                                                              │
│  Troque sua escolha e veja: muda tudo, e continua             │
│  do mesmo tamanho. Não há como saber qual é qual.             │
│                                                              │
│  ────────────────────────────────────────────────────────    │
│                                                              │
│  ◉ Manter meu voto em segredo                                │
│  ○ Registrar meu voto publicamente                           │
│                                                              │
│  43 de 50 estão em segredo. Acima do mínimo de 5.            │
│                                                              │
│                                        [ CONFIRMAR VOTO ]    │
└──────────────────────────────────────────────────────────────┘
```

### 4.3 As decisões, e por quê

**O painel de hex não é enfeite, é o feedback que faltava.** Quando a pessoa troca de APROVAR para REJEITAR, os 96 bytes mudam por completo, na frente dela, e continuam com exatamente a mesma cara. Isso é o sigilo perfeito, demonstrado em meio segundo, sem uma palavra de criptografia. É a única coisa nesta tela que não pode ser cortada.

Não escreva "criptografado com segurança". Mostre o objeto.

**A recomputação precisa ser instantânea.** Se demorar 800ms e piscar um spinner, o efeito morre. O compromisso é um `g1_msm` de 2 termos — faça no cliente, sem rede.

**"Manter em segredo" vem primeiro e vem marcado.** Ordem e padrão são decisões éticas aqui, não estéticas. O caminho seguro é o caminho de menor esforço.

**O contador de sigilo é o teorema da partição virando frase.** "43 de 50 estão em segredo. Acima do mínimo de 5." A pessoa não precisa saber que existe um `τ`. Ela precisa saber que o grupo em que ela se esconde é grande. E se encolher, ela precisa ver:

> ⚠ **Só 4 pessoas ainda estão em segredo.** Com tão poucas, o resultado publicado revelaria os votos secretos por subtração. A apuração vai ser recusada até que haja pelo menos 5.

Esse aviso é o momento em que o produto se mostra mais cuidadoso do que o usuário esperava. Guarde-o para o vídeo.

**Nada de "você tem certeza?" antes de confirmar.** Modal de confirmação é o reflexo preguiçoso. O voto é reversível até o prazo? Então diga isso embaixo do botão. Não é? Então o botão diz `CONFIRMAR VOTO`, e isso já é a confirmação.

---

## 5. Momento 2 — Conferência

### 5.1 O que a pessoa faz

Confere que entrou. E destrói o recibo.

```
┌──────────────────────────────────────────────────────────────┐
│  ✓  Seu voto entrou na contagem                              │
│                                                              │
│  Posição 27 de 43 votos em segredo                           │
│  Transação  a4f2…9e1c          ledger 1.284.551              │
│                                                              │
│  Seu compromisso                                             │
│  0x0f1ff9fcda6e556ba2da8fb4…                                 │
│                                                              │
│  Ele está no acumulador que vai gerar o total. Qualquer      │
│  pessoa pode conferir isso, sem descobrir seu voto.          │
│                                                              │
│  ────────────────────────────────────────────────────────    │
│                                                              │
│  ⚠  ESTE RECIBO PROVA O SEU VOTO                             │
│                                                              │
│  Enquanto você guarda a chave abaixo, você consegue provar   │
│  a qualquer pessoa em que votou. Isso significa que alguém   │
│  que te obrigue a mostrar também consegue conferir.          │
│                                                              │
│  A rede nunca vai saber. Mas você pode ser forçada a contar. │
│                                                              │
│         [ APAGAR A CHAVE AGORA ]    guardar por enquanto     │
└──────────────────────────────────────────────────────────────┘
```

### 5.2 Por que essa tela é a mais importante do produto

O spec chama isso de **N2**, a lacuna mais séria da v1: quem vota conhece o próprio `r`, e com ele consegue provar o voto a um coator.

Dona Marta não sabe que isso é um risco. Ela vai guardar o print "por garantia", como guarda comprovante de Pix. E o print é exatamente a arma que o presidente precisa.

**Design que ignora isso é design que entrega a usuária.** Então:

- O aviso é **o elemento mais destacado da tela**, acima do botão.
- A linguagem não é jurídica nem técnica: *"A rede nunca vai saber. Mas você pode ser forçada a contar."*
- `APAGAR A CHAVE AGORA` é a ação primária. `guardar por enquanto` é texto, não botão.
- Depois de apagar: *"Pronto. Agora nem você consegue provar em que votou."* — e essa frase é a sensação que o produto inteiro existe para entregar.

Esse momento é o "queimar a cédula" do spec, virado interação. É onde a metáfora para de ser retórica.

### 5.3 Microcópia é segurança

Em um produto de voto secreto, a frase errada é uma falha. Três regras:

1. **Nunca prometa o que o protocolo não garante.** Nada de "100% anônimo" — o ato de votar é público (N1). Diga "sabe-se que você votou; não se sabe em quê".
2. **Nomeie o adversário.** Não "terceiros não autorizados". Diga "a diretoria", "quem apura", "quem tem poder sobre você". O medo é concreto; a cópia também.
3. **Prefira "ninguém" a "é difícil".** Quando a garantia é information-theoretic, dizer "ninguém consegue, nunca, com computador nenhum" é *mais correto* que hedge, e é a única vez em que superlativo é honesto. Use.

---

## 6. Momento 3 — Apuração

Esta é a tela do cético, e portanto do jurado.

```
┌──────────────────────────────────────────────────────────────┐
│  APURAÇÃO · COOPERATIVA X                                    │
│                                                              │
│   43 em segredo      7 públicos      50 de 50 votaram        │
│                                                              │
│  ────────────────────────────────────────────────────────    │
│                                                              │
│  O que o ledger guarda                                       │
│                                                              │
│  ░░░░ ░░░░ ░░░░ ░░░░ ░░░░ ░░░░ ░░░░ ░░░░ ░░░░ ░░░░          │
│  ░░░░ ░░░░ ░░░░ ░░░░ ░░░░ ░░░░ ░░░░ ░░░░ ░░░░ ░░░░          │
│  ░░░░ ░░░░ ░░░░ ░░░░ ░░░░ ░░░░ ░░░░ ░░░░ ░░░░ ░░░░          │
│  ░░░░ ░░░░ ░░░░ ░░░░ ░░░░ ░░░░ ░░░░ ░░░░ ░░░░ ░░░░          │
│  ▓▓▓▓ ▓▓▓▓ ▓▓▓▓ ▓▓▓▓ ▓▓▓▓ ▓▓▓▓ ▓▓▓▓                         │
│                                                              │
│  43 compromissos idênticos em forma. 7 abertos por escolha.  │
│                                                              │
│  ────────────────────────────────────────────────────────    │
│                                                              │
│  A mesa afirmou        APROVAR 19    REJEITAR 31             │
│  O contrato conferiu   ✓ fecha com o acumulado               │
│  O verificador refez   ✓ 50 provas · 50 aptos · 0 duplicados │
│                                                              │
│                            REJEITADO · 31 a 19               │
└──────────────────────────────────────────────────────────────┘
```

### 6.1 As decisões

**A grade de blocos é a imagem do pitch.** Quarenta e três quadradinhos visualmente indistinguíveis. Você olha e vê que não há nada para ver. Isso faz num segundo o que o slide `solution` faz em um parágrafo.

Os 7 públicos ficam em tom diferente — não melhor nem pior, só distinto. O semi-confidencial aparece como *forma*, não como legenda.

**Três linhas, três agentes, nessa ordem:** a mesa *afirma*, o contrato *confere*, o verificador *refaz*. Essa sequência é a arquitetura de confiança do produto, e lida de cima para baixo ela ensina sozinha por que não é preciso confiar na mesa.

**O resultado vem por último e é a coisa maior da tela.** Depois da verificação, não antes. A ordem comunica: o número só vale porque passou pelas três linhas acima.

### 6.2 O estado que ganha a demo

Inverta o total para 20 e mostre:

```
  A mesa afirmou        APROVAR 20    REJEITAR 30
  O contrato conferiu   ✗  não fecha com o acumulado
                           apuração recusada, nada publicado

  Uma mesa que mente não consegue publicar. Não é
  denúncia depois: é recusa na hora.
```

**Esse é o passo 6 do roteiro e é o que separa um sistema que *afirma* ser verificável de um que é.** Nove de dez demos de hackathon mostram só o caminho feliz. Mostrar a recusa é o que faz um jurado técnico acreditar no resto.

---

## 7. O que construir em 3,5 dias

O PLANO.md cortou a interface gráfica (corte 1). Eu **não** vou pedir para desfazer esse corte — vou propor três níveis, e o nível 1 já ganha o critério.

### Nível 1 — obrigatório: a CLI é interface

Saída de terminal desenhada é UX de verdade, e cabe no tempo que já está no plano. A CLI tem que imprimir os três momentos:

```
$ tessera votar --proposta contas-2025 --opcao rejeitar --identidade marta

  COOPERATIVA X · Aprovar as contas da diretoria de 2025?
  Sua escolha ........ REJEITAR
  Sigilo ............. em segredo (43 de 50 em segredo, mínimo 5) ✓

  O que a rede vai guardar, para sempre:
    0f1ff9fcda6e556ba2da8fb40898b251409d204c79b6bd364f55abba…

  ── enviado ──────────────────────────────────────────────
  Transação .......... a4f29e1c      ledger 1.284.551
  Posição ............ 27 de 43 em segredo

  ⚠  A chave em ./recibos/marta.key prova o seu voto.
     A rede nunca vai saber. Mas você pode ser forçada a contar.
     Apague com:  tessera queimar --identidade marta
```

Isso entrega: o objeto na tela, o contador de sigilo, a confirmação de entrada e o aviso de coação. **Os quatro elementos essenciais, sem uma linha de HTML.**

### Nível 2 — desejável: o console de demonstração

Uma página HTML **estática, sem backend**, que lê o JSON que a CLI já escreve e desenha os três momentos. Não é um aplicativo: é um visor. Sem carteira, sem servidor, sem estado.

Por que vale: para o vídeo, um visor é indistinguível de um app. E a grade de 43 blocos idênticos não existe em terminal.

Por que é seguro de prometer: ~4h, zero risco para o contrato, e **completamente cortável** — se cair, o nível 1 sustenta a demo sozinho.

Onde entra no cronograma: **domingo 08:00–12:00, em paralelo às correções.** Se o Portão 3 falhou no sábado, não acontece.

> **Feito em 2026-10-01, antes do previsto**, porque o Portão 3 passou na quarta. Está em `console/index.html`: HTML estático, sem backend, com uma rodada real da testnet embutida para abrir mostrando algo em vez de um formulário vazio. `?p=<proposta>` lê o JSON ao lado; o seletor de arquivo abre qualquer outro.
>
> A grade de blocos indistinguíveis existe e é o que o terminal não conseguia dar: **dez objetos de 96 bytes, cinco cometendo um voto e cinco a ausência dele, idênticos em forma.** Os dados são reais e estão no ledger.

E o README diz o que ele é: *"console de demonstração sobre a saída da CLI"*. Escrever isso custa nada e é a diferença entre escopo e maquiagem.

### Nível 3 — não vai acontecer: o aplicativo

Carteira, autenticação, responsivo, estados de erro de rede. Semanas. Fica no roadmap.

---

## 8. Linguagem visual

**Reaproveite o deck, inteiro.** Os tokens já existem, estão testados, e assim o produto e a apresentação parecem uma coisa só. Custo de design: zero.

| papel | valor |
|---|---|
| fundo | `#141613` |
| fundo profundo | `#0E100D` |
| hairline | `#2A2D27` |
| texto | `#ECEFE6` |
| texto secundário | `#9AA093` |
| texto apagado | `#6B7065` |
| acento | `#C8F24F` |
| display | Instrument Serif 400 |
| texto | Space Grotesk 400/500 |
| técnico, hex, números | Martian Mono 400 |

Três regras de uso:

1. **O hex é sempre Martian Mono, sempre.** É o material do produto. Tratar como dado, não como texto.
2. **O acento `#C8F24F` marca o que foi verificado**, nunca o que foi escolhido. Verde é "confere", não é "sua opção". Escolha se marca por peso e borda.
3. **Vermelho aparece uma vez só:** a apuração recusada. Se vermelho estiver em mais de um lugar, ele deixa de significar algo.

Acessibilidade: a recusa não pode ser só vermelha — leva o `✗` e a palavra "recusada" (o spec já exige isso para status).

---

## 9. Roteiro do vídeo, como experiência

O PLANO.md tem os 6 passos. Aqui está como filmá-los para que *sintam* algo. Alvo: **3min30**.

| t | o que aparece | o que o jurado sente |
|---|---|---|
| 0:00–0:20 | A pergunta da assembleia. Uma frase: "Dona Marta quer rejeitar as contas. O presidente aprovou o empréstimo do filho dela." | o problema é de gente, não de cripto |
| 0:20–0:50 | A cédula. Trocar APROVAR ↔ REJEITAR **três vezes** e deixar o hex mudar inteiro. Sem narração nesse trecho. | *entendi o sigilo sem ninguém me explicar* |
| 0:50–1:10 | Marcar "registrar publicamente", o contador cair, o aviso de mínimo aparecer | este sistema pensou no que eu não pensei |
| 1:10–1:35 | Conferência. O aviso de coação. Apagar a chave. "Agora nem você consegue provar em que votou." | é honesto sobre o próprio limite |
| 1:35–2:10 | Cinco votos entrando. A grade de blocos indistinguíveis. | a imagem do pitch |
| 2:10–2:40 | Apuração: mesa afirma, contrato confere, verificador refaz. REJEITADO 31 a 19. | a cadeia de confiança sem confiar na mesa |
| 2:40–3:10 | **Mesa mentindo. Recusa na hora.** | isto é real |
| 3:10–3:30 | O ledger aberto inteiro, sem distinguir ninguém. O endereço do contrato. | pode conferir você mesmo |

Duas regras de montagem:

- **O trecho 0:20–0:50 é sem voz.** Só o hex mudando. Deixe o jurado ter a percepção sozinho. Narração nesse ponto rouba o único momento de descoberta que o vídeo tem.
- **Nada de tela de abertura com logo.** O vídeo começa na pergunta da assembleia. Em hackathon, logo é segundo desperdiçado.

---

## 10. O que não fazer

- **Não explique criptografia na interface.** Mostre o objeto. Quem quiser saber abre o spec.
- **Não escreva "100% anônimo".** É falso (N1) e um jurado pega em dez segundos.
- **Não ponha barra de progresso em operação de 5 segundos.** A finalidade da Stellar é um recurso; esconda-a atrás de um spinner e você joga o recurso fora.
- **Não anime o compromisso "sendo criptografado".** Cadeado fechando, partícula voando: é a estética que faz parecer brinquedo. O hex mudando já é a melhor animação possível, porque é verdadeira.
- **Não esconda o aviso de coação atrás de "saiba mais".** É a lacuna mais séria do protocolo. Enterrá-la na interface é mentir por omissão de layout.
- **Não faça modal de confirmação.** Ver §4.3.

---

## 11. Como saber se funcionou

Teste com uma pessoa só, e não é usabilidade: é confiança.

Peça para alguém de fora votar na demo. Depois, duas perguntas:

1. *"Quem consegue descobrir em que você votou?"*
   **Sucesso:** "ninguém". **Falha:** "não sei", "a plataforma", "depende".
2. *"Seu voto entrou na conta?"*
   **Sucesso:** "sim, vi a posição 27". **Falha:** "acho que sim".

Se as duas respostas vierem certas de uma pessoa que nunca ouviu "compromisso de Pedersen", a interface fez o trabalho. Se a primeira vier errada, o painel de hex não está convencendo e nada mais na tela importa.

---

## 12. O que fica para depois

- Estados de erro de rede e assinatura recusada (o nível 2 assume caminho feliz)
- Responsivo de celular, que é onde Dona Marta realmente votaria
- Acessibilidade de leitor de tela nas grades de compromisso
- Fluxo da mesa apuradora k-de-n, que na v1 é diretório local (corte 2 do plano)
- Internacionalização
