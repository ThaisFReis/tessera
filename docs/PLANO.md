# Tessera — Plano de execução até a submissão

**Escrito:** 2026-09-30 23:33 (-03) · **revisado** 2026-09-30 após retorno da SDF
**Congelamento de código:** 2026-10-04 (domingo) 12:00
**Submissão fecha:** 2026-10-05 (segunda) 19:00 — *confirmar fuso na página do hackathon*
**Meta de submissão:** 2026-10-05 12:00, deixando 7h de margem

Tempo real disponível de código: **3 dias cheios + 1 manhã**.
Quinta 01, sexta 02, sábado 03, e domingo 04 até meio-dia.

Isto não é tempo para construir tudo que o [PROTOCOLO.md](PROTOCOLO.md) descreve. O plano
abaixo escolhe o que entra, **decide os cortes agora** (§4) em vez de improvisar
sob pressão no sábado à noite, e define três portões de decisão com saída
pré-combinada.

---

## 0. O retorno da SDF, e o que ele muda

Duas sugestões, com custos muito diferentes. Medi as duas antes de decidir.

### 0.1 "Tentar usar ZK também" — **não entra na v1, e agora temos o número**

Medi o caminho Groth16 na sonda 6 (2026-09-30). `pairing_check` compartilha a
exponenciação final entre os pares:

```
pairing_check(k) = 10.571.128 + 6.746.479 · k        (medido)
```

Groth16 se reduz a *um* produto de 4 pares, então a verificação completa custa
**47.371.348 instruções com 4 entradas públicas: 11,8% do teto, 8 por
transação.**

**O orçamento on-chain não é o gargalo de ZK na Stellar.** Isso responde a
sugestão dele de forma mais forte do que implementar às pressas: em vez de um
circuito mal-acabado, entregamos a *medição* que diz que o caminho está aberto,
e o ecossistema já tem trilha para percorrê-lo — Nethermind cortou 64% das
instruções da verificação Noir/UltraHonk em Soroban movendo operações para host
functions dos protocolos 25/26 (taxa de 0,23 → 0,09 XLM, 2026-08-04), e existe
receita pública de Circom + snarkjs verificando na testnet.

O que falta não é a cadeia: é circuito, setup e provador no cliente. Isso é
semana, não dia, e escrever circuito ZK em 3 dias junto com o resto é o jeito
mais rápido de não entregar nada. **Fica na v2 (§12.3 do spec), agora com
número medido em vez de projeção.**

### 0.2 "Votos semi-confidenciais" — **entra na v1, e é quase de graça**

Esta é a boa, e encaixa sem primitiva nova: **um campo público é apenas um
compromisso cuja abertura é revelada.** O contrato confere `C == v·G + r·H` com
um MSM de 2 termos, a agregação homomórfica não distingue aberto de fechado, e
um campo público custa ~5,4M contra ~27M de uma prova CDS. **Publicidade é mais
barata que sigilo.**

O que destrava, em governança real: cédula com sigilo por pergunta; categoria
pública com direção secreta; e principalmente **publicidade opcional por
votante** — vários estatutos de cooperativa e regimentos de conselho asseguram
o direito de registrar voto divergente em separado, como proteção jurídica de
quem divergiu. Hoje esse direito é incompatível com voto secreto. Aqui os dois
coexistem na mesma urna.

**Mas vem com uma regra obrigatória**, e perceber isso é o que separa recurso de
armadilha: §6.6 do spec, o teorema da partição. Toda informação pública
particiona o conjunto de anonimato — 48 de 50 publicando determinam os 2
secretos por subtração. Pior, é adversarialmente explorável: uma coligação
publica de propósito para encolher o conjunto secreto. O contrato precisa
recusar apurar qualquer célula com menos de `τ` confidenciais.

É o **mesmo teorema** de §6.3 (pesos públicos), generalizado. Os dois problemas
colapsam numa regra só, o que é a razão de isto caber no cronograma.

**Custo real na v1:** sob `UmPorPessoa` (corte 3) e sem outros campos públicos,
a partição tem exatamente **duas células** — publicado e secreto — e a checagem
de `τ` é uma comparação. São ~2h, não 4, e só porque está sendo desenhado agora
em vez de remendado depois.

## 1. O que precisa existir para submeter

| Entregável | Gate | Estado |
|---|---|---|
| Repositório público | bloqueia `[repo url]` em 2 decks | ❌ |
| Contrato `abrir`/`votar`/`apurar` na testnet | critério "execução técnica" | ❌ |
| Cliente CLI que gera e submete um voto | critério "user experience" | ❌ |
| Verificador público independente | é o que torna P5 real, não decorativa | ❌ |
| Vídeo ponta a ponta | bloqueia `[video url]` em 2 decks | ❌ |
| README que permite reproduzir | credibilidade | ❌ |
| Deck PT + EN | — | ✅ feito |
| Spec | — | ✅ feito |
| Sonda medida na testnet | — | ✅ feito |
| Orçamento Groth16 medido (sonda 6) | responde a dica de ZK da SDF | ✅ feito |
| Publicidade opcional por votante + regra `τ` | responde a dica semi-confidencial | ❌ |

---

## 2. Cronograma

### Quinta 01/10 — medir e fechar a matemática

**08:00–08:30 · Repositório público** — ✅ **FEITO 2026-10-01**

Meia hora que elimina um risco de submissão quatro dias antes. Sem isso, dois
decks têm colchete vazio e o risco vai crescendo em silêncio.

```bash
# repo novo, público, com bls-smoke + tessera/
gh repo create tessera --public --description "Secret ballots for any governance, on Soroban"
```

- ✅ `git init`, estrutura `bls-smoke/` + `docs/` + decks na raiz.
- ✅ `LICENSE` MIT (o deck afirma MIT — a afirmação precisa ser verdadeira).
- ✅ README como porta de entrada, com os números medidos e as lacunas.
- ⬜ `gh repo create --public` e push.
- ⬜ Preencher `[url do repositório]` nos dois decks, slides `shipped` e `close`.

Os decks ficam em `index.html` e `index.en.html` **na raiz de propósito**: com
GitHub Pages ligado, eles passam a ter URL pública, o que resolve o problema de
os artifacts estarem privados.

**Aceite:** a URL abre em janela anônima, e os quatro colchetes de repo estão preenchidos.

---

**08:30–12:00 · As três medições do §9**

Ordem por risco. Cada uma é uma função nova na sonda, medida por diferença como
em §7.1 (custo com `n=11` menos custo com `n=1`, dividido por 10).

1. **`verify_cds`** (§9.1) — o maior número do orçamento e o único grande que é
   projeção. Implementar o verificador CDS de §6.1 e medir.
2. **`deserialize_n`** (§9.2) — receber `n` pontos G1 por argumento e só
   validar on-curve + in-subgroup. São ~6 pontos por `votar()`.
3. **`rent_n`** (§9.3) — escrever 1.000 entradas tipo `Votou` e ler a taxa.
4. **`verify_reveal`** (§9.4) — conferir uma abertura revelada, o caminho do
   campo público de §6.5. Projetado em ~5,4M; sai junto com a 1 e a 2.

> A quarta medição do §9, o orçamento Groth16, **já foi feita** na sonda 6 na
> noite de 30/09. Uma a menos.

**Aceite:** três números reais em `../bls-smoke/RESULTADOS.md`, e o orçamento de
§7.2 do spec reescrito com ✅ no lugar dos ❌.

> **PORTÃO 1 — quinta 12:00.** Soma de `votar()` com números reais.
> - **≤ 200M:** segue o plano.
> - **200–350M:** corta para 2 opções fixas (sim/não) e Merkle profundidade 8.
> - **> 350M:** a prova CDS vira o problema central. Troca para prova de faixa
>   mais barata, ou aceita 1 opção por transação (`m` transações por votante).
>   Decidir em 30 min, não ruminar.

---

**13:00–19:00 · `tessera-core`**

Um crate de matemática pura, `no_std`-compatível, **compartilhado entre contrato,
cliente e verificador**. Esta é a decisão de arquitetura que mais economiza tempo:
provador e verificador saem do mesmo código, então o formato de prova não pode
divergir, e o verificador público fica quase de graça depois que o cliente existe.

```
tessera/
├── core/          # Pedersen, CDS, Schnorr, Shamir, Merkle — sem Soroban
├── contract/      # abrir/votar/apurar, chama core
├── cli/           # provador + submissão, chama core
└── verifier/      # relê o ledger e reconfere, chama core
```

- `commit(v, r) -> C`, soma, igualdade.
- `H` por `hash_to_g1(DST_H)`, calculado uma vez.
- CDS: `prove_bit(v, r)` e `verify_bit(C, π)`.
- Schnorr em base `H`: `prove_sum`, `verify_sum`.
- Shamir `k`-de-`N` sobre `Fr`, com soma de shares.
- Merkle sha256, profundidade fixa 8.
- `verify_reveal(C, v, r)` — o caminho do campo público (§6.5).
- `conta_confidenciais()` e a checagem de `τ` (§6.6), duas células na v1.

**Aceite:** os treze testes unitários de §13 do spec passando, inclusive os
negativos (CDS rejeita `v=2`, abertura falha com `T` alterado em 1).

---

### Sexta 02/10 — o contrato

**08:00–13:00 · `abrir()` e `votar()`**

- Armazenamento conforme §5.1.
- `abrir`: registra proposta, zera acumuladores, grava `H`.
- `votar`: `require_auth`, confere Merkle, confere peso contra a folha, verifica
  CDS por opção, verifica soma, rejeita voto duplo, acumula `A_j += C_j`.

**14:00–19:00 · `apurar()` e testes de custo**

- `apurar`: confere `A_j == T_j·G + R_j·H`, exige `k` assinaturas, só após prazo.
- Testes de custo que **asseram teto** — regressão quebra o build em vez de
  aparecer na demo:
  - `votar()` binário cabe em 400M com folga ≥ 2×;
  - `apurar()` constante em `n` (medir com 10 e com 1.000);
  - custo de apuração independe do peso.

> **PORTÃO 2 — sexta 19:00.** `votar()` e `apurar()` passando nos testes locais?
> - **Sim:** segue.
> - **Não:** sábado começa cortando Merkle para allowlist explícita em storage
>   (§4, corte 3). Documenta a lacuna no README e no spec.

---

### Sábado 03/10 — cliente, verificador, testnet

**08:00–13:00 · CLI**

```bash
tessera abrir    --proposta <id> --opcoes 2 --aptos aptos.csv --mesa mesa.csv -k 3
tessera votar    --proposta <id> --opcao 1 --identidade alice
tessera apurar   --proposta <id> --mesa-shares ./shares/
tessera verificar --proposta <id>
```

O "canal privado" para as shares da mesa é um diretório local nesta versão
(§4, corte 2). Não é um canal de verdade, e o README e o vídeo dizem isso.

**14:00–17:00 · Verificador público**

Relê os `votar()` da proposta no ledger, reconstrói `A_j`, confere a abertura,
reconfere todas as provas, confere aptidão e unicidade. Binário separado, sem
estado, sem confiança.

**17:00–20:00 · Deploy e rodada integrada**

O roteiro é exatamente o do vídeo (§13 do spec):

1. Abrir proposta, 5 aptos, um voto por pessoa.
2. Cinco votos de identidades distintas, um divergente da maioria.
3. Apurar com `k=3` de `N=5`.
4. Rodar o verificador e mostrar que fecha.
5. Mostrar o ledger inteiro sem distinguir quem votou o quê.
6. Tentar apurar total falso e mostrar o contrato recusando.

> **PORTÃO 3 — sábado 20:00.** A rodada integrada fecha na testnet?
> - **Sim:** domingo é polimento e materiais.
> - **Não:** aciona o fallback nuclear (§5). Sem heroísmo de madrugada —
>   código criptográfico escrito às 3h da manhã é o jeito mais rápido de
>   submeter algo que não funciona na frente do jurado.

---

### Domingo 04/10 — congelar e gravar

**08:00–12:00 · Últimas correções → CONGELAMENTO 12:00**

Depois do meio-dia: nenhuma linha de contrato. Só texto, vídeo e empacotamento.
Este limite existe porque o modo de falha clássico de hackathon não é código
faltando, é commit de última hora que quebra a demo.

**13:00–18:00 · Vídeo e README**

- Gravar os 6 passos. Alvo de 3 a 4 minutos, nada de introdução longa.
- O passo 5 é o pitch inteiro numa tela. O passo 6 é o que diferencia de um
  sistema que só *afirma* ser verificável.
- README: o que é, como rodar, o que está medido, **o que não está feito**
  (coação N2, mesa N3, pesos §6.3). As lacunas declaradas somam credibilidade;
  encontradas pelo jurado, subtraem tudo.
- Subir o vídeo, preencher `[url do vídeo]` nos dois decks.

---

### Segunda 05/10 — submeter

**08:00–11:00** — revisão final: links abrem em janela anônima, deck compartilhado
pelo menu Share (hoje está **privado**, jurado não abre), repo público, contrato
respondendo na testnet.

**12:00 — submeter.** Sete horas de margem antes das 19:00.

---

## 3. O que entra no código, em ordem de dependência

```
medições (§9)
   └── core: Pedersen + CDS + Schnorr + Shamir + Merkle
          └── contract: abrir / votar / apurar
                 ├── cli: provar e submeter
                 └── verifier: reconferir do ledger
                        └── rodada integrada na testnet
                               └── vídeo
```

Nada aqui pode ser paralelizado por uma pessoa só. O caminho crítico é inteiro,
e por isso os cortes de §4 são o único grau de liberdade real.

---

## 4. Cortes decididos agora

Pré-combinados hoje, com a cabeça fria, para não serem improvisados sábado à noite.

| # | Corte | Justificativa |
|---|---|---|
| 1 | **Sem interface gráfica.** CLI e verificador. | 3,5 dias não constroem contrato criptográfico *e* UI. O deck já não promete telas. Perde ponto em "user experience" conscientemente. |
| 2 | **Shamir com canal simulado.** Shares em diretório local. | A matemática `k`-de-`N` entra (é ela que sustenta P2); o canal autenticado e cifrado não. Declarado no README e no vídeo. |
| 3 | **Só `UmPorPessoa`.** Sem faixas de peso. | É o modo seguro por padrão (§6.3) e o único que não exige verificar `τ` contra a raiz. Faixas ficam especificadas, não implementadas. |
| 4 | **Merkle profundidade fixa 8** (256 aptos). | Suficiente para a demo. Profundidade variável é complexidade sem ganho de pitch. |
| 5 | **Duas opções na demo.** Contrato aceita `m ≤ 16`, testado com 2. | O orçamento de §7.2 é calculado para `m=2`. Mais opções não foram medidas. |
| 6 | **Sem resistência a coação** (N2). | Exige relayer de re-randomização. Fora de alcance, e declarado como lacuna conhecida. |
| 7 | **Sem circuito ZK.** Só a medição de orçamento. | §0.1. O número medido responde a dica melhor que um circuito mal-acabado. |
| 8 | **Semi-confidencial só no modo "publicidade opcional".** | §0.2. Cédula multi-pergunta fica especificada, não implementada. A regra `τ` entra junto, não depois. |

Se o cronograma atrasar, os próximos cortes saem **nesta ordem**: corte 3 já está
feito → Merkle vira allowlist explícita → mesa vira endereço único com a lacuna
documentada em voz alta. A prova CDS **não é cortável**: ela é o núcleo de
segurança, e sem ela o que sobra não é um protocolo de voto secreto.

---

## 5. Fallback nuclear

Se o Portão 3 falhar sábado 20:00, a submissão passa a ser:

- a sonda criptográfica medida e publicada (existe hoje, funciona hoje);
- o verificador rodando contra a sonda, não contra a urna;
- o PROTOCOLO.md completo como a contribuição intelectual;
- **uma linha do deck trocada:** no slide `measured`, "The ballot contract itself
  lands this week" vira a verdade do dia.

Isto já é uma submissão coerente — e é coerente **porque** o deck foi escrito
honesto desde o começo, separando o que está medido do que está projetado. Um
deck que afirmasse a urna construída exigiria reescrita sob pressão, no pior
momento possível. A honestidade do slide não foi escrúpulo: foi seguro.

---

## 6. Primeiro movimento

Amanhã, 01/10, 08:00, antes de abrir editor de código: **criar o repositório
público e preencher os quatro colchetes de URL nos dois decks.** Trinta minutos,
e o risco de submissão incompleta sai da mesa quatro dias antes do prazo.

Depois disso, `verify_cds` — e as outras três medições do §9, agora que a do
Groth16 já saiu.
