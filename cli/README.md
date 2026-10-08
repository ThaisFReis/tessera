# `tessera` — o cliente

```bash
cargo build --release
export PATH="$PWD/target/release:$PATH"
export TESSERA_CONTRATO=CCQHRQZP3R3QMMKEEOOOS7XXINR7WGSMTKGNR4GD6BDNLC6JSPUZIDHZ
```

Requer a `stellar` CLI com identidades criadas e financiadas. Quem vota paga a
própria taxa, então cada identidade precisa de saldo.

```bash
stellar keys generate --network testnet marta   # idem para cada votante e membro da mesa
```

## A rodada

```bash
tessera abrir   --proposta contas \
                --pergunta "Aprovar as contas da diretoria de 2025?" \
                --opcoes aprovar,rejeitar \
                --aptos marta,joao,ana,carlos,lucia,pedro,sofia \
                --mesa mesa1,mesa2,mesa3,mesa4,mesa5 -k 3 --prazo 8m

tessera cedula  --proposta contas --identidade marta          # mostra, não vota
tessera votar   --proposta contas --opcao rejeitar --identidade marta
tessera votar   --proposta contas --opcao aprovar --identidade pedro
tessera status  --proposta contas
tessera apurar  --proposta contas
tessera verificar --proposta contas
```

`--aptos` e `--mesa` aceitam lista por vírgula ou um arquivo com um nome por
linha. `--inicio` diz quando a votação abre (padrão `0`, imediato) e `--prazo`
quanto ela dura depois disso; os dois aceitam `8m`, `2h`, `45s` ou um número de
ledgers.

**Não existe votar em público, e não existe recibo.** Um voto aberto no ledger
é a forma mais forte de coação que há — quem coage confere sozinho, sem a
pessoa na frente — e um recibo guardado é o que a compra de voto precisa para
saber se foi entregue. `votar` não grava nada em disco: o acaso que esconde o
seu voto vive no processo e morre com ele.

Perguntas públicas são outra coisa, e continuam existindo: valem para todos e
são decididas pelo estatuto na abertura.

## A cédula mista

Uma pergunta por `--pergunta`, no formato `"texto | opções | natureza"`. A
natureza é `sigilosa` ou `publica`, e **cai em `sigilosa` se você omitir** — o
sigilo é o padrão, e abrir uma pergunta tem de ser deliberado.

```bash
tessera abrir   --proposta assembleia                 --pergunta "Aprovar as contas de 2025? | aprovar, rejeitar | publica"                 --pergunta "Destituir a diretoria? | sim, nao | sigilosa"                 --pergunta "Eleger a cadeira 3 | ana, bruno, carla | sigilosa"                 --aptos marta,joao,ana,carlos,lucia,pedro,sofia                 --mesa mesa1,mesa2,mesa3,mesa4,mesa5 -k 3 --prazo 30m

# uma --opcao por pergunta, na ordem da cédula
tessera votar   --proposta assembleia --opcao aprovar --opcao sim --opcao ana                 --identidade marta
```

Uma cédula **confidencial** é aquela em que todas as perguntas são sigilosas;
uma **semiconfidencial** mistura. É o mesmo mecanismo, em dois ajustes.

O teto é de 8 perguntas e **16 opções sigilosas somadas** — cada uma custa
13.501.500 instruções, e 16 já são 56% do teto de uma transação. As opções
públicas não entram nessa conta: a cédula pública inteira custa 350.372.

**Dê ao menos 8 minutos de prazo para 7 votantes:** cada voto confidencial leva
~30 s de relógio pela `stellar` CLI, e a janela fecha por ledger, não por
quantos já votaram.

## Os três diretórios, e o que cada um pode ter

| | |
|---|---|
| `estado/<proposta>.json` | o que **já está no ledger**: compromissos, provas, apuração. Público. É o contrato com o Nível 2 da UX. |
| `recibos/<identidade>.key` | **só o segredo**: os `r_j` de quem votou. É o que `queimar` sobrescreve. |
| `shares/<membro>/` | as shares de Shamir que viajariam por canal privado até cada membro da mesa. |

A separação é a regra: `recibos/` tem o que ninguém mais pode ter, `estado/`
tem o que todo mundo já tem. Por isso **queimar o recibo não custa
auditabilidade** — as provas são públicas e ficam no estado.

Os três estão no `.gitignore`.

## Como gravar as recusas

```bash
tessera apurar --proposta contas --forcar-total 4,1   # a mesa mentindo
```

A recusa por `τ` acontece sozinha numa votação com menos de 5 cédulas
confidenciais: o contrato recusa publicar um total que revelaria os votos por
subtração.

## Cor

Truecolor, e o verde marca **o que foi verificado, nunca o que foi escolhido**.
Respeita `NO_COLOR` e saída não-tty — teste com `| cat` antes de gravar.

Largura fixa de 72 colunas, régua `─`, hex em blocos de 48 × 4. Não é estética:
num vídeo 1920×1080 em tela cheia, a 100 colunas o jurado não lê o hex, e o hex
é o produto.
