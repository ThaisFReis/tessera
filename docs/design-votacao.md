# O desenho do dapp

A identidade visual vem do pitch deck em `index.html`: Instrument Serif,
Space Grotesk, Martian Mono e a paleta carvão, papel `#ecefe6`, verde-lima
`#c8f24f` e oliva. As fontes são servidas localmente pelo app.

O fundo profundo `#080a08` começou valendo só para a tela da cédula e passou a
valer para o dapp inteiro: a urna não é um lugar à parte do resto da votação.
Navegação e rodapé continuam em todas as rotas. O que distingue o fluxo de quem
vota — `/comparecer/:id` e `/votar/:id` — é a composição, não a cor: cabeçalho
mais baixo, navegação secundária retirada, e a trilha **Presença · Voto ·
Confirmação** como única orientação dentro do fluxo.

## Onde moram os primitivos

Botão, rótulo em mono, estado central, aviso, anel de carregamento, lista de
passos e ícone estão em `app/src/ui.css` e `app/src/ui.tsx`. Nasceram dentro de
`paginas/votar.css` e `paginas/Votar.tsx`, quando só a cédula tinha desenho, e
foram promovidos antes de serem replicados: cinco cópias das mesmas regras
divergiriam em silêncio.

`paginas/votar.css` ficou com o que é da cédula — opções, revisão, trilha.

## Conferir o design

Dentro de `app`, execute `npm run dev` e abra `/demo/votar`. Essa rota tem uma
proposta e sete participantes ilustrativos, permite escolher, revisar, alterar
e simular o envio. Não lê nem escreve na Stellar, não cria carteiras, e **não
escreve no diário** — `/bastidores` é o registro do que aconteceu de verdade, e
uma linha encenada ali valeria menos que nenhuma. A simulação é identificada na
interface, inclusive na confirmação.

A rota real continua em `/votar/:id`. A proposta na rede só contém quantidades
de opções; por isso, os nomes reais continuam numerados, com o identificador
original visível. O enunciado e os nomes da prévia não são usados em votações
reais.

## `/bastidores`

O diário — o `o que está acontecendo` — não fica mais no pé de cada rota. Ele
tem rota própria, para caber numa segunda janela ao lado daquela em que a
pessoa age: é o que o vídeo da demonstração precisa mostrar, e os dois lados não
cabem numa tela só.

Nas rotas ficou, no lugar dele, a lista de passos em linguagem de pessoa
(`Preparando sua cédula · Criando uma assinatura de uso único · Confirmando na
Stellar`) e um link que abre os bastidores em outra janela. `?origem=votar`
filtra por fluxo.

A loja fica em `app/src/diario.ts`. `BroadcastChannel` entrega ao vivo entre
janelas da mesma origem sem servidor — e o projeto inteiro é sem servidor. O
`localStorage` é lastro, não canal: é o que permite abrir os bastidores depois e
ainda ver a rodada.

A tela mostra o que a pessoa fez (`›`), o que a página chamou (`$`), os valores
(incluindo o custo em instruções de CPU que a simulação descobriu), as notas
(`//`), o aceite (`✓`) e a recusa (`✗`), com o erro do contrato traduzido do
número para o nome — `ImagemJaUsada (#30)`, não `Error(Contract, #30)`.

A datilografia foi portada de `console/index.html`, com o alvo derivado do
relógio e não de um contador: `requestAnimationFrame` já travou essa renderização
em 6 de 19 linhas quando a aba perdeu o foco, e uma gravação de tela é
exatamente a situação em que a aba perde o foco.

## O guarda do diário

Enquanto o diário era efêmero e local, um segredo vazado nele sumia com a aba.
Persistido e transmitido entre janelas, ele **fica** — e vira o lugar mais fácil
do sistema para achar o `r` de alguém. `anotar()` passa cada texto por um crivo
antes de gravar.

O crivo olha para a **conjunção** de um nome de segredo e um valor na mesma
linha, não para a palavra sozinha. A diferença importa: `Comparecer` escreve *"a
chave secreta do anel fica nesta aba, e só aqui"*, que não vaza nada e é
justamente a frase que explica a garantia, e escreve `chave pública = 09fd…`,
que é um valor público de propósito. Um crivo por palavra apagaria as duas.

`app/scripts/diario.test.mjs` congela as duas bordas.

## Interação

- Nenhuma opção vem selecionada; a revisão exige uma escolha.
- A revisão permite voltar antes da confirmação definitiva.
- O envio mostra preparação, assinatura e confirmação.
- O comprovante real só aparece após o retorno de sucesso da rede, com link
  para a transação e sem mostrar a escolha.
- Erros de leitura permitem tentar novamente. Propostas incompatíveis,
  presença ausente e janelas fechadas impedem o envio pela interface.
- O layout adapta as telas para celular e respeita movimento reduzido —
  inclusive a datilografia dos bastidores, que nesse caso imprime tudo de uma vez.

Validado com TypeScript e build de produção, e com uma rodada inteira na testnet
em duas janelas, com os bastidores abertos na segunda o tempo todo.
