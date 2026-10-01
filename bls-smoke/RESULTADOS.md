# Smoke BLS12-381 — resultado (2026-09-30)

Ambiente: `rustc 1.97.1`, `soroban-sdk 28.0.0`, `soroban-env-host 28.0.2`,
`stellar-cli 25.2.0`, alvo `wasm32v1-none`. Wasm: 12.455 bytes. 9/9 testes passam.

## Veredito: Plano A está de pé

As host functions de BLS12-381 (CAP-0059) existem no pin do SDK 28, a
serialização do gerador canônico (`be_bytes(X) || be_bytes(Y)`, 96 bytes) é
aceita pelo host, e a matemática de apuração fecha de ponta a ponta dentro do
contrato: três cédulas ElGamal de pesos 5, 3 e 1 agregam e decifram para 9.

## Custo de CPU medido (instruções)

| operação | custo |
|---|---|
| `g1_add` | 110.748 |
| `g1_mul` | 3.290.524 |
| `g1_msm` (k=2) | 5.398.553 |
| `g1_msm` (k=8) | 14.230.058 |
| `hash_to_g1` | 2.653.011 |
| `pairing_check` (1 par) | 17.317.607 |
| `pairing_check` (2 pares) | 24.064.086 |
| `pairing_check` (3 pares) | 30.810.565 |
| `pairing_check` (4 pares) | 37.557.044 |
| `verify_schnorr` (invocação cheia) | 6.737.614 |

Isolamento: custo por operação = (custo com n=11 − custo com n=1) / 10, o que
cancela o overhead de invocação do contrato.

## Três conclusões que mudam o desenho

**1. `g1_add` é ~30x mais barato que `g1_mul`.** A agregação homomórfica
(2 adds por cédula = ~221k) é praticamente de graça. O custo está todo na
verificação de prova, não na apuração.

**2. MSM tem economia de escala real.** Marginal por termo ≈ 1,47M contra 3,29M
de um `g1_mul` solto (base ≈ 2,46M). Toda verificação com múltiplos termos deve
ser escrita como um MSM único, não como muls somados. Prova disjuntiva de cédula
estimada em ~27M avulsa (14 por transação); batelada por combinação linear
aleatória deve cair para ~12M/cédula (~33 por transação).

**3. Não faça a cadeia PROCURAR o resultado — faça-a CONFERIR.** O log discreto
por força bruta custa ~124k por unidade de peso, o que dá um teto de **~3.218 de
peso total por transação**. Qualquer assembleia real estoura. A variante
`tally_checked`, em que a mesa apuradora assevera o total e o contrato verifica
`M == total*G`, custa **6.712.183 independente do peso** — idêntico para peso 250
e para 1.000.000.

## Arquitetura que decorre das medições

- **Submissão (1 tx por votante, taxa paga por ele):** verificar a prova de
  boa-formação da cédula (~27M avulsa, ~12M em MSM) + 2 `g1_add` no acumulador.
  Cabe com folga nos 400M. Não há gargalo.
- **Apuração (1 tx):** Chaum-Pedersen de decifração correta + conferência do
  total asseverado ≈ 10M. Sem busca on-chain.
- **Importação em lote** (custodiante submetendo procurações em massa) é o único
  caminho que precisa de batelada: ~33 cédulas/tx com MSM.

## Testnet — confirmado na rede real

Contrato: `CA2LFMOOLAFHUNYBD2MLNEUINYN76IZ6AOGNDPH27UX4MHOVIHGPTRTP`
Identidade descartável: `urna-smoke` (`GCZC4HW5...UX4NLGWJ`), financiada pelo friendbot.

| invocação | resultado | taxa cobrada |
|---|---|---|
| `probe(G)` | `[true, true]` | — |
| `verify_schnorr` (prova válida) | `true` | 9.558 stroops |
| `tally_checked` (3 cédulas, total 9) | `true` | 11.852 stroops |
| `tally_checked` (total 10, mesa mentindo) | `false` | — |

O host da testnet aceita a mesma serialização, o sigma-protocolo verifica, e a
apuração rejeita um total falso. ~0,00096 XLM por verificação de prova — a
ordem de US$ 0,0002 ao preço corrente.

### Teto de CPU: 400M, não 100M

Medido por bissecção com `add_n` na testnet: **n=3500 (~387M) passa, n=3600
(~398M) devolve `HostError: Error(Budget, ExceededLimit)`**. O teto real é
**400.000.000 de instruções** — minha premissa inicial de 100M estava 4×
conservadora. Consequências:

- cédulas verificáveis por transação: **14** (era 3). Veredito passa de
  "amortizada" para **Plano A folgado**.
- teto da busca por força bruta: ~3.218 de peso total (era ~804) — segue
  pequeno demais para assembleia real, então a conclusão 3 não muda.

## Sonda 6 — Groth16 cabe no orçamento (2026-09-30)

Medido depois do retorno de um membro da SDF sugerindo ZK. A pergunta: a
verificação de uma prova de conhecimento zero cabe nos 400M?

`pairing_check` **compartilha a exponenciação final** entre os pares. O custo
se decompõe em:

```
pairing_check(k) = 10.571.128 + 6.746.479 · k
```

O marginal por par (6,75M) é **2,6× mais barato que o primeiro par** (17,3M).
Isso é o que torna ZK viável: a verificação Groth16 se reduz a *um* produto de
pareamentos com 4 pares, não a quatro pareamentos separados.

| verificação Groth16 | instruções | % do teto | por tx |
|---|---|---|---|
| 2 entradas públicas | 44.427.514 | 11,1% | 9 |
| 4 entradas públicas | 47.371.348 | 11,8% | 8 |
| 8 entradas públicas | 53.259.016 | 13,3% | 7 |

Composição: `pairing_check(4)` + MSM de `(n_pub + 1)` termos, com o MSM medido
em base 2.454.719 + 1.471.917 por termo.

**Veredito: o orçamento on-chain não é o gargalo de ZK na Stellar.** Uma
verificação Groth16 usa ~12% de uma transação, e caberiam 8 numa só. O que
falta para usar ZK não é a cadeia — é circuito, setup e provador.

## Sonda 7 — Pedersen fecha (smoke B1, 2026-10-01)

O desenho final do SPEC é Pedersen com abertura do agregado, e até hoje **nunca
havia sido executado**: as sondas 1–6 mediram ElGamal exponencial. Este é o
smoke B1 de `tessera/SMOKES.md`, o primeiro da fila.

Contrato: `CCC4KNBCXIMXLHVEH3LG6CL32ZZJTTRNBRBTTSHH5ZYL6MUGTSGIDGKY`

| operação | custo | medido |
|---|---|---|
| `gerador_h` (só na inicialização) | 2.654.931 | ✅ |
| `commit` (lado do cliente) | 5.408.243 | ✅ |
| **`verify_aggregate`** (a apuração) | **5.408.931** | ✅ |
| taxa de `verify_aggregate` na testnet | **8.622 stroops** | ✅ |

### O que fechou na rede real

Cinco compromissos `C_i = v_i·G + r_i·H` com votos `1,0,1,1,0` e acasos
`101..505`, agregados pelo contrato:

| invocação | resultado |
|---|---|
| `gerador_h` | `1462b4b5…72eccf`, on-curve ✅, no subgrupo ✅ |
| `verify_aggregate(A, 3, 1515)` — mesa honesta | `true` |
| `verify_aggregate(A, 4, 1515)` — mesa mentindo | `false` |
| `verify_aggregate(A, 2, 1515)` — mesa mentindo | `false` |
| `verify_aggregate(A, 3, 1516)` — abertura errada | `false` |

Também verificado no teste: homomorfismo explícito
(`C(1,r₁)+C(1,r₂) == C(2,r₁+r₂)`), o caso de borda `T=0` com todos os votos
zero, e **o `H` local idêntico byte a byte ao `H` da testnet** — travado como
vetor em `src/test.rs`, porque provador e verificador derivando `H` diferente
é o bug de meio dia que o smoke B2 existe para evitar.

### Veredito: Pedersen é melhor que ElGamal em todos os eixos medidos

| | ElGamal (sonda 5) | Pedersen (sonda 7) |
|---|---|---|
| apuração on-chain | 6.712.183 | **5.408.931** (−19%) |
| taxa | 11.852 stroops | **8.622 stroops** (−27%) |
| sigilo | computacional, com prazo | **permanente, information-theoretic** |

A projeção do SPEC §7.2 era ~5,4M e o medido foi 5.408.931 — **erro de 0,2%**.
Cabem 73 apurações numa transação.

Não há trade-off aqui: o desenho que dá sigilo permanente é também o mais
barato. O plano B (voltar a ElGamal) fica arquivado sem uso.

### Armadilha operacional encontrada

`Bls12381Fr` entra na `stellar contract invoke` como **decimal**, não hex.
Passar hex falha com `invalid digit found in string` — e, pior, um hex que por
acaso só tenha dígitos (`0065`) é aceito silenciosamente como o decimal errado.
É por isso que `vetores.env` guarda `Z_DEC` ao lado de `Z_HEX`. A CLI do Tessera
tem de emitir decimal.

## Sonda 8 — A janela do RPC (smoke A2, 2026-10-01)

**Pergunta.** O verificador público relê os `votar()` do ledger e refaz o
agregado. Ele depende do histórico do RPC, e a retenção do RPC é configurável.
Por quanto tempo P5 — "qualquer pessoa recalcula a apuração a partir da rede" —
continua verdadeira?

**Medido no RPC público da testnet** (`soroban-testnet.stellar.org`):

```
ledgerRetentionWindow ... 120.960 ledgers
janela real ............. 120.959 ledgers
em tempo ................ 7,00 dias   (604.795 s, ledger médio 5,00 s)
```

O limite é aplicado, não é só documentação. `getEvents` com `startLedger`
anterior à janela devolve:

```
startLedger must be within the ledger range: 4846106 - 4967065
```

### A distinção que salva P5

A janela de 7 dias vale para **eventos e transações via RPC**. Não vale para:

- **estado do contrato** — `getLedgerEntries` lê a entrada atual sem janela
  nenhuma (confirmado: a instância do contrato continua legível);
- **arquivos de histórico** — `history.stellar.org/prd/core-testnet/…`
  responde e está no ledger 4.967.039, praticamente ao vivo. É o registro
  canônico e permanente.

Logo o verificador tem **dois níveis, com prazos diferentes**:

| nível | o que prova | fonte | validade |
|---|---|---|---|
| 1 | o total publicado corresponde ao acumulador | estado do contrato | permanente¹ |
| 2 | o acumulador é a soma exata das cédulas aptas, não repetidas e bem formadas | eventos via RPC | **7 dias** |
| 2′ | idem | arquivos de histórico | permanente, mais lento |

¹ sujeito ao TTL de arquivamento — smoke A1, ainda não medido. Se `Acum`
arquivar, o nível 1 também cai, e aí A1 passa a ser o smoke mais grave da lista.

### Veredito

**P5 sobrevive, com uma qualificação que precisa estar escrita.** O verificador
não pode depender só do RPC: ou lê dos arquivos de histórico, ou indexa
continuamente. Lendo só do RPC, ele funciona no dia da votação e **para de
funcionar na segunda semana** — que é exatamente o modo de falha silenciosa que
este smoke existia para pegar.

Consequência para a demo: 7 dias é folgado para o hackathon, e as invocações da
sonda 7 (ledger 4.960.517) saem da janela em **6,6 dias**. Se o vídeo for
gravado hoje e um jurado tentar reproduzir dia 10, o `getEvents` recusa. **O
README tem de dizer isso, e o verificador tem de ter o modo arquivo.**

## Sonda 9 — TTL e arquivamento (smoke A1, 2026-10-01)

Contrato: `CCCZ4HPW3ESHO6BMNFGXX6DJRSWPWA7FBQH2MS6QGHC3U67DIZ434KFC`

| medida | valor |
|---|---|
| TTL padrão de entrada persistente nova | **120.959 ledgers = 7,00 dias** |
| TTL máximo da rede (`max_ttl`) | **3.110.399 ledgers = 180,0 dias** |
| taxa para estender uma entrada de 96 B ao máximo | **2.069.531 stroops ≈ 0,207 XLM** |
| o contrato consegue ler o próprio TTL? | **não** — só estender |

Medido escrevendo uma entrada de 96 bytes (do tamanho de um `Acum`) e lendo
`liveUntilLedgerSeq` fora da cadeia: `4.967.134 → 5.088.093` antes da extensão,
`→ 8.077.544` depois.

### A coincidência que é o verdadeiro achado

O TTL padrão é **120.959 ledgers**. A janela de retenção do RPC (sonda 8) é
**120.960 ledgers**. São o mesmo número.

Ou seja, sem intervenção, **no sétimo dia os dois níveis do verificador morrem
no mesmo instante**: o RPC para de servir os eventos *e* o acumulador arquiva.
Uma votação feita hoje fica integralmente inverificável a partir do dia 8,
sem que nada emita erro no dia 1.

Era exatamente este o modo de falha silenciosa que os smokes A1 e A2 existiam
para pegar, e ele era pior do que qualquer um dos dois isolado.

### Arquivado não é perdido

Uma entrada arquivada volta com `RestoreFootprintOp`. A verificabilidade não
desaparece: ela passa a exigir que **alguém pague uma restauração** antes de
tocar na entrada. É degradação, não perda — e tem de estar escrito assim, nem
mais dramático nem menos.

### Consequência de desenho: nem toda entrada merece sobreviver

A 0,207 XLM por entrada, estender tudo não escala:

| entradas | o que são | sobrevida necessária | custo a 180 dias |
|---|---|---|---|
| `Acum(id, j)` | m acumuladores | **permanente** | m × 0,207 XLM |
| `Proposta(id)` | metadados | **permanente** | 0,207 XLM |
| `Votou(id, addr)` | n anti-duplo-voto | só até encerrar | n × 0,207 XLM |

Para 10.000 votantes, estender os `Votou` custaria **~2.070 XLM**. Para os
`Acum` de uma votação binária, **0,41 XLM**.

**Decisão:** `abrir()` estende ao máximo apenas `Proposta` e os `Acum` — o que
o nível 1 do verificador precisa para sempre. Os `Votou` ficam no TTL padrão e
podem arquivar depois do encerramento: eles só impedem voto duplo *durante* a
votação, e a unicidade histórica é reconferida pelo nível 2, que lê eventos.

Isso mantém o custo de abertura em **~0,6 XLM independente do número de
votantes**, e é a diferença entre um módulo usável e um que cobra 2.000 XLM
por assembleia.

## O que este smoke NÃO cobre

Sem autorização, sem armazenamento, sem nullifier, sem mesa k-de-n (Shamir) e
sem a prova disjuntiva CDS implementada (smoke B4) — o custo dela (~27M) é estimado a
partir das primitivas medidas, não medido diretamente. A sonda 6 mede o custo
de *verificar* um Groth16, não de gerar a prova nem de escrever o circuito.

## Rodar

```
cargo test --lib -- --nocapture --test-threads=1
stellar contract build
```
