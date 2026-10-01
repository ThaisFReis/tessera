# Tessera — Especificação do Protocolo

**Versão:** 0.1 (rascunho de implementação)
**Data:** 2026-09-30
**Estado:** desenho fechado; sonda criptográfica medida na testnet; contrato de urna não implementado
**Escopo:** Hackathon Find Your Way (Meridian), trilha General. Prazo de submissão: 2026-10-05 19:00

---

## 0. O que é

Tessera é um **módulo de votação secreta para contratos Soroban**. Não é um aplicativo
de governança, não é uma DAO e não é um produto vertical. É um contrato com três
funções que qualquer governança já existente pode chamar para realizar uma votação
em que:

- **sabe-se que uma pessoa votou**, e isso é público e auditável;
- **não se sabe em que ela votou**, e isso é secreto *para sempre*, não apenas
  enquanto a criptografia de hoje resistir;
- **qualquer pessoa pode recalcular o resultado** a partir do ledger e detectar
  uma mesa apuradora que minta.

A governança integradora mantém suas próprias regras sobre quem pode votar, quanto
vale cada voto e o que fazer com o resultado. Tessera não opina sobre nada disso.

### 0.1 Por que "para sempre" é o requisito central

Um registro distribuído é permanente por construção. Um texto cifrado gravado nele
é legível-em-princípio para sempre: basta que a chave vaze, que o parâmetro de
segurança envelheça, ou que apareça uma máquina capaz de resolver o problema que
o protegia. "Criptografado hoje" é, num ledger, sinônimo de "legível quando a
chave aparecer".

Isso não é um detalhe teórico para uma votação. A ameaça que justifica voto secreto
— retaliação de quem tem poder sobre quem vota — não tem prazo de validade. Uma
pessoa que votou contra a diretoria em 2026 continua vulnerável em 2046.

Daí a decisão de desenho que organiza todo o resto: **o ledger nunca recebe um
texto cifrado do voto.** Ele recebe um compromisso perfeitamente ocultante, que é
uma objeto matematicamente vazio de informação, e a informação real transita fora
da cadeia e é destruída.

---

## 1. Modelo de ameaça

### 1.1 O adversário

O adversário relevante **está dentro da governança** e tem poder sobre quem vota:
a diretoria que controla a linha de crédito, o gestor que assina o orçamento, a
baleia que financia o grant. Não é um atacante externo anônimo.

Capacidades assumidas:

| Capacidade | Assumida? |
|---|---|
| Ler todo o ledger, agora e para sempre | **Sim** |
| Poder computacional ilimitado, no futuro | **Sim** (é o ponto do desenho) |
| Computador quântico | **Sim** |
| Coagir ou subornar votantes antes do voto | **Sim** |
| Retaliar depois do resultado | **Sim** |
| Controlar a governança integradora | **Sim** |
| Controlar até `k-1` membros da mesa apuradora | **Sim** |
| Controlar `k` ou mais membros da mesa | **Não** (fora do modelo) |
| Observar fisicamente a pessoa votando | **Não** (nenhuma criptografia resolve isso) |
| Quebrar o log discreto **antes** da apuração | **Não** (só afeta integridade, não sigilo) |

### 1.2 O que é garantido

**P1. Sigilo permanente (everlasting privacy).** Um adversário com poder
computacional ilimitado, de posse de todo o ledger para sempre, não obtém
*nenhuma* informação sobre o voto individual de ninguém. Não é uma afirmação
sobre dificuldade computacional; é uma afirmação sobre ausência de informação.
Ver §3.2 para a prova.

**P2. Sigilo contra a mesa apuradora.** Menos de `k` membros da mesa em conluio
não recuperam nenhum voto individual, ainda que somem todo o material que
receberam. Ver §4.3.

**P3. Integridade da apuração.** A mesa não consegue publicar um total falso sem
resolver um log discreto em BLS12-381. O contrato rejeita; qualquer pessoa
reproduz a rejeição.

**P4. Verificabilidade individual.** Cada votante confere que o seu compromisso
está no acumulador que gerou o total publicado.

**P5. Verificabilidade universal.** Qualquer pessoa, a partir do ledger e de
nada mais, recalcula o agregado e confere a abertura publicada.

**P6. Unicidade.** Uma pessoa apta vota no máximo uma vez por proposta, e isso é
verificável on-chain.

**P7. Fase única.** Não há commit-e-revela. Quem vota e desaparece não invalida
a votação nem trava o quórum. Ver §11.1.

### 1.3 O que NÃO é garantido

Declarar isto faz parte do desenho, não é ressalva de rodapé. Um sistema de
votação que promete sigilo além do que entrega é pior que um honesto.

**N1. Anonimato do ato de votar.** O ledger registra *que* aquela conta votou.
Isso é intencional — a governança precisa apurar comparecimento e quórum — e é o
que torna o desenho viável sem ZK. Quem precisa esconder até a participação
precisa da versão desvinculada (§12.3).

**N2. Resistência à coação (receipt-freeness).** **Esta é a lacuna mais séria da
v1 e precisa ser dita em voz alta.** Quem vota conhece o próprio fator de
aleatoriedade `r_i`. Com `r_i` e `v_i`, a pessoa *consegue provar a um terceiro*
em que votou, porque o compromisso `C_i` é público e a abertura confere. Logo
um coator que exija o recibo consegue verificá-lo.

O protocolo remove o registro público permanente do voto. Ele **não** remove a
capacidade de quem vota de se auto-incriminar voluntariamente. Isso exige
re-randomização por um relayer ou credenciais estilo JCJ, e está fora da v1.
Ver §12.2.

**N3. Mesa que não destrói suas cópias.** Se `k` membros conservarem o material
recebido e conspirarem, recuperam votos individuais. O sigilo permanente vale
contra quem lê o *ledger*; contra a mesa, vale o limiar `k` e o procedimento de
destruição. Isso é procedimental, não estrutural.

**N4. Resultado unânime.** Se todos votam igual, o total revela o voto de todos.
Isso é verdade em qualquer sistema de votação, inclusive papel, e é aceitável.

**N5. Voto ponderado com pesos públicos distintos.** Ver §6.3 — é um vetor de
ataque real, e a mitigação é obrigatória, não opcional.

---

## 2. Notação e parâmetros

| Símbolo | Significado |
|---|---|
| `G1`, `G2` | grupos da curva BLS12-381, ordem primária `r` |
| `Fr` | corpo escalar de ordem `r` |
| `G` | gerador canônico de `G1` |
| `H` | segundo gerador de `G1`, `H = hash_to_g1(DST_H)` |
| `m` | número de opções da proposta |
| `n` | número de votantes aptos |
| `w_i` | peso do votante `i` |
| `v_{i,j}` | voto de `i` na opção `j`, em `{0,1}` |
| `r_{i,j}` | aleatoriedade do compromisso, uniforme em `Fr` |
| `C_{i,j}` | compromisso de Pedersen, `v_{i,j}·G + r_{i,j}·H` |
| `k`, `N` | limiar e tamanho da mesa apuradora |

**Serialização.** `G1` afim, 96 bytes, não comprimido: `be_bytes(X) || be_bytes(Y)`.
`G2`, 192 bytes. Isto está **verificado** contra o host da testnet (§10.1).

**Domain separation tags.** Cada uso de hash tem DST próprio e distinto:

```
DST_H          = "TESSERA-V1-GENERATOR-H"
DST_CHALLENGE  = "TESSERA-V1-FIAT-SHAMIR"
DST_NULLIFIER  = "TESSERA-V1-NULLIFIER"
```

> **Pendência de implementação.** O contrato da sonda usa
> `DST = "KARN-URNA-V0-CHALLENGE"`, herdado de antes do nome Tessera e de antes
> do desenho Pedersen. Trocar o DST invalida toda prova gerada com o anterior,
> o que é exatamente o comportamento desejado, mas precisa ser feito **de uma
> vez só** e antes de qualquer vetor de teste ser publicado.

### 2.1 A escolha de `H`

`H` precisa ter log discreto desconhecido em relação a `G`, senão o compromisso
deixa de ser ocultante — quem souber `h` tal que `H = h·G` abre qualquer
compromisso. Por isso `H` é derivado por `hash_to_g1` de um DST público e fixo
("nothing up my sleeve"), nunca sorteado por alguém e nunca configurável pela
governança integradora.

`hash_to_g1` custa 2.653.011 instruções (medido). É calculado **uma vez** na
inicialização e guardado, nunca por transação.

---

## 3. O esquema criptográfico

### 3.1 Compromisso de Pedersen

Para o votante `i` e a opção `j`:

```
C_{i,j} = v_{i,j}·G + r_{i,j}·H        com v_{i,j} ∈ {0,1},  r_{i,j} ←$ Fr
```

Propriedades relevantes:

- **Aditivamente homomórfico:** `C_a + C_b` é o compromisso de `v_a + v_b` com
  aleatoriedade `r_a + r_b`. É isso que permite apurar somando pontos.
- **Perfeitamente ocultante:** ver §3.2.
- **Computacionalmente vinculante:** abrir o mesmo `C` para dois valores
  distintos revela `log_G(H)`. É o que impede a mesa de mentir.

Note a assimetria deliberada: o **sigilo** é incondicional e o **vínculo** é
computacional. Essa é a escolha certa aqui, e é o oposto da escolha de um
esquema de cifra. Se o log discreto cair amanhã, uma mesa maliciosa passa a
conseguir falsificar um total — e isso é detectável, contestável e reparável.
Se o sigilo fosse computacional, a queda revelaria todos os votos já dados,
retroativamente e de forma irreparável. Trocamos um risco irreversível por um
reversível.

### 3.2 Por que o sigilo é permanente

**Afirmação.** `C = v·G + r·H`, com `r` uniforme em `Fr` e `H` gerador, não
carrega informação alguma sobre `v`.

**Prova.** Seja `v'` qualquer outro valor. Como `H` é gerador de `G1`, existe
`h = log_G(H)` e existe

```
r' = r + (v − v')·h⁻¹   (mod r)
```

tal que `v'·G + r'·H = v·G + r·H = C`. Logo, para *todo* `v'` existe exatamente
um `r'` que produz o mesmo `C`, e como `r` é uniforme, `r'` também é. A
distribuição de `C` é portanto idêntica para qualquer `v`: `C` é uniforme em
`G1` e independente de `v`. ∎

Não há o que decifrar. Um adversário com tempo infinito, o ledger completo e um
computador quântico está exatamente na mesma posição de quem não tem nada. Isso
é o que "everlasting" significa aqui, e é a diferença categórica em relação a
gravar ElGamal ou qualquer cifra na cadeia.

**Onde mora a informação, então.** Em `(v_i, r_i)`, que existem apenas fora da
cadeia: no cliente de quem votou e, em pedaços, nos membros da mesa (§4.3).
Destruídos, o voto individual deixa de existir como fato recuperável no universo.
É o equivalente digital de queimar a cédula — e essa analogia é exata, não
retórica.

### 3.3 Apuração por abertura do agregado

Somando os compromissos de todos os votantes na opção `j`:

```
A_j = Σ_i C_{i,j} = (Σ_i v_{i,j})·G + (Σ_i r_{i,j})·H = T_j·G + R_j·H
```

A mesa publica `(T_j, R_j)`. O contrato confere

```
A_j == T_j·G + R_j·H
```

que é um MSM de 2 termos. `R_j` é a abertura do **agregado**, e revelar `R_j` não
revela nenhum `r_{i,j}` individual: é a soma de `n` valores uniformes, e conhecer
a soma de `n` incógnitas uniformes não determina nenhuma delas.

Pelo vínculo computacional, a mesa não consegue publicar `T'_j ≠ T_j` que passe.

---

## 4. Papéis e fluxo

### 4.1 Papéis

| Papel | Quem | Confiança exigida |
|---|---|---|
| **Governança integradora** | o contrato que chama Tessera | define aptidão e pesos; não vê votos |
| **Votante** | conta Stellar na lista de aptos | nenhuma |
| **Mesa apuradora** | `N` contas, limiar `k` | sigilo até `k-1` em conluio; nenhuma para integridade |
| **Verificador** | qualquer pessoa | nenhuma |

A mesa **não** é confiável para integridade. Ela é confiável para sigilo apenas
abaixo do limiar, e essa é a única suposição procedimental do desenho. A v2
(§12.3) a remove.

### 4.2 Fluxo

```
  ┌─ 1. ABRIR ───────────────────────────────────────────────────┐
  │  governança → Tessera.abrir(proposta, opções, raiz_aptos,    │
  │                             mesa, k, prazo)                  │
  │  on-chain: registra a proposta, zera os acumuladores A_j     │
  └──────────────────────────────────────────────────────────────┘
                              │
  ┌─ 2. VOTAR (1 tx por votante, taxa paga por ele) ─────────────┐
  │  cliente: escolhe v, sorteia r, monta C_j e as provas        │
  │  cliente → mesa (canal privado): shares Shamir de r_{i,j}    │
  │  cliente → Tessera.votar(proposta, C[], provas, prova_aptid) │
  │  on-chain: verifica provas, marca votou, A_j += C_j          │
  └──────────────────────────────────────────────────────────────┘
                              │
  ┌─ 3. APURAR (1 tx) ───────────────────────────────────────────┐
  │  mesa: cada membro soma suas shares; k membros reconstroem   │
  │        R_j = Σ r_{i,j} (nunca um r_i isolado)                │
  │  mesa → Tessera.apurar(proposta, T[], R[])                   │
  │  on-chain: confere A_j == T_j·G + R_j·H, publica T[]         │
  │  mesa: DESTRÓI shares e material intermediário               │
  └──────────────────────────────────────────────────────────────┘
```

### 4.3 Por que shares de Shamir, e não uma mesa única

Se a mesa for uma entidade só, ela recebe `r_{i,j}` e, com `C_{i,j}` que é
público, calcula `C_{i,j} − r_{i,j}·H = v_{i,j}·G` e lê o voto individual. Uma
mesa única **vê todos os votos**. Isso derrota o propósito, porque a mesa é
tipicamente indicada por quem já tem poder na governança.

Solução: quem vota manda a cada membro `l` da mesa uma share Shamir
`s_{i,j,l}` de `r_{i,j}`, com limiar `k` de `N`. Como o compartilhamento de
Shamir é **aditivamente homomórfico nas shares**, cada membro soma localmente

```
S_{j,l} = Σ_i s_{i,j,l}
```

e `k` membros reconstroem `R_j = Σ_i r_{i,j}` a partir dos `S_{j,l}` — **sem
que nenhum `r_{i,j}` individual jamais seja reconstruído por ninguém.** A
informação necessária para ler um voto individual nunca se junta em lugar algum.

Isso é o que transforma o sigilo contra a mesa de "confie que eles apagam" em
"menos de `k` não conseguem, mesmo guardando tudo".

O que ainda resta procedimental: `k` membros em conluio *que tenham guardado as
shares brutas* conseguem. Daí N3.

---

## 5. Interface on-chain

```rust
/// Abre uma votação. Chamada pela governança integradora.
fn abrir(
    env: Env,
    proposta: BytesN<32>,        // id da proposta, escolhido pela governança
    opcoes: u32,                 // m, número de opções (2..=16)
    raiz_aptos: BytesN<32>,      // raiz de Merkle da lista na data de corte
    mesa: Vec<Address>,          // N membros
    limiar: u32,                 // k
    fecha_em: u64,               // ledger sequence de fechamento
) -> Result<(), Erro>;

/// Registra um voto. Chamada pelo votante, que paga a taxa.
fn votar(
    env: Env,
    proposta: BytesN<32>,
    votante: Address,                        // require_auth
    compromissos: Vec<Bls12381G1Affine>,     // C_j, j = 0..m
    prova_binaria: Vec<ProvaCDS>,            // v_j ∈ {0,1}, uma por opção
    prova_soma: ProvaSchnorr,                // Σ v_j = w
    prova_aptidao: Vec<BytesN<32>>,          // caminho de Merkle
    peso: u32,                               // w, conferido contra a folha
) -> Result<(), Erro>;

/// Apura. Chamada por k membros da mesa (multi-auth).
fn apurar(
    env: Env,
    proposta: BytesN<32>,
    totais: Vec<u32>,                        // T_j
    aberturas: Vec<Bls12381Fr>,              // R_j
) -> Result<Vec<u32>, Erro>;

/// Leitura. Qualquer pessoa.
fn acumulador(env: Env, proposta: BytesN<32>) -> Vec<Bls12381G1Affine>;
fn resultado(env: Env, proposta: BytesN<32>) -> Option<Vec<u32>>;
fn ja_votou(env: Env, proposta: BytesN<32>, votante: Address) -> bool;
```

### 5.1 Armazenamento

| Chave | Tipo | Durabilidade | Notas |
|---|---|---|---|
| `Proposta(id)` | metadados | persistent | opções, raiz, mesa, k, prazo |
| `Acum(id, j)` | `Bls12381G1Affine` | persistent | 96 bytes por opção |
| `Votou(id, addr)` | `bool` | persistent | uma entrada por votante |
| `Resultado(id)` | `Vec<u32>` | persistent | só após apurar |
| `GeradorH` | `Bls12381G1Affine` | instance | calculado uma vez no deploy |

`Votou(id, addr)` cresce linearmente em `n`. **Medido no smoke A1
(2026-10-01):** o TTL padrão de uma entrada persistente é 120.959 ledgers =
**7,00 dias**, o teto da rede é 3.110.399 ledgers = **180 dias**, e estender
uma entrada de 96 B ao teto custa **2.069.531 stroops ≈ 0,207 XLM**.

O TTL padrão coincide com a janela de retenção do RPC (§8.3): sem intervenção,
**no sétimo dia os dois níveis do verificador morrem juntos.**

**Política de TTL, que decorre do custo:**

| entrada | sobrevida | ação em `abrir()` |
|---|---|---|
| `Proposta(id)` | permanente | estender ao máximo |
| `Acum(id, j)` | permanente | estender ao máximo |
| `Votou(id, addr)` | até encerrar | TTL padrão, pode arquivar |

Estender todos os `Votou` custaria ~2.070 XLM numa assembleia de 10.000
pessoas. Estender só o que o nível 1 do verificador precisa custa **~0,6 XLM,
independente do comparecimento**. Os `Votou` só impedem voto duplo *durante* a
votação; a unicidade histórica é reconferida pelo nível 2, que lê eventos.

Entrada arquivada não é perdida: volta com `RestoreFootprintOp`, ao custo de
quem quiser verificar. É degradação, não perda.

Na v1 **não há nullifier**: como a identidade de quem vota é pública por
desenho (N1), a chave `(proposta, endereço)` resolve unicidade de forma mais
simples e barata. A construção `nullifier = H(DST_NULLIFIER ‖ conta ‖ proposta)`
fica reservada para a v2, onde quem vota é oculto e a chave não pode ser o
endereço.

---

## 6. Sistemas de prova

### 6.1 Boa formação binária: prova disjuntiva CDS

Quem vota precisa provar `v_j ∈ {0,1}` sem revelar qual. Construção padrão de
Cramer–Damgård–Schoenmakers: duas ramificações Schnorr, uma real e uma simulada,
com o desafio dividido por Fiat–Shamir.

Provar `C = 0·G + r·H` **ou** `C = 1·G + r·H`, isto é, `C` ou `C − G` é um
múltiplo conhecido de `H`:

```
Prover (caso v=0, simula o ramo 1):
  a_0 = t·H,                  t ←$ Fr
  e_1 ←$ Fr,  z_1 ←$ Fr,      a_1 = z_1·H − e_1·(C − G)
  e   = H(DST_CHALLENGE ‖ C ‖ a_0 ‖ a_1)
  e_0 = e − e_1
  z_0 = t + e_0·r
  π = (a_0, a_1, e_0, z_0, e_1, z_1)

Verifier:
  e_0 + e_1 == H(DST_CHALLENGE ‖ C ‖ a_0 ‖ a_1)
  z_0·H == a_0 + e_0·C
  z_1·H == a_1 + e_1·(C − G)
```

Tamanho: 2 pontos G1 (192 B) + 4 escalares (128 B) = **320 bytes por opção**.

Custo: cada verificação é 1 MSM de 2 termos, duas vezes. **Medido em
10.980.243 instruções e 14.144 stroops** (sonda 12, 2026-10-01). A projeção
anterior de ~27M supunha muls somados; o MSM de 2 termos por ramo a derrubou
em 60%.

### 6.2 Soma correta: Schnorr em base H

Se cada `v_j ∈ {0,1}` e a soma dos votos é `w`, então

```
D = (Σ_j C_j) − w·G = (Σ_j r_j)·H
```

é um múltiplo conhecido de `H`. Basta uma prova Schnorr de conhecimento de
`ρ = Σ_j r_j` com `D = ρ·H`. Uma prova, não `m` provas.

Custo: da ordem do `verify_schnorr` medido, 6.737.614 instruções incluindo o
overhead de invocação.

Tamanho: 1 ponto + 1 escalar = 128 bytes.

### 6.3 O ataque dos pesos públicos, e a mitigação obrigatória

**O problema.** Suponha pesos públicos e distintos: `w = {1, 3, 7, 12, 40}`, e o
total apurado para a opção A é `T_A = 19`. Existe **um único** subconjunto que
soma 19: `{7, 12}`. O resultado publicado, combinado com a lista pública de
pesos, revela exatamente quem votou em A. O sigilo criptográfico é perfeito e
**irrelevante**: a aritmética do resultado o contorna.

Isso é um problema de subconjunto-soma, e com pesos reais de governança (que são
tipicamente muito distintos e de cauda longa) o subconjunto é frequentemente
único ou quase.

**Mitigações, em ordem de preferência:**

1. **Um voto por pessoa (`w_i = 1` para todos).** O total é a contagem e não
   distingue ninguém. É o caso mais seguro e deve ser o padrão do módulo.
2. **Quantização em faixas.** Os pesos são arredondados para uma escala grossa
   (por exemplo `{1, 2, 5, 10}`) de modo que cada valor seja compartilhado por
   um número mínimo `τ` de votantes. O contrato **rejeita a abertura** da
   votação se alguma faixa tiver menos de `τ` membros. Perde-se precisão de peso
   e ganha-se um conjunto de anonimato auditável.
3. **Apuração por faixa.** Publica-se um total por faixa, não um total global.
   Cada faixa é um conjunto de anonimato de tamanho conhecido e mínimo `τ`.

**Decisão para a v1:** o módulo aceita `peso`, mas `abrir()` **exige** ou o modo
`UmPorPessoa`, ou uma tabela de faixas com `τ ≥ 5` verificada contra a raiz de
aptos. Pesos arbitrários distintos são **recusados pelo contrato**, não
documentados como "cuidado". Um sistema de votação não deve aceitar uma
configuração que anula o próprio sigilo.

### 6.4 Aptidão

`raiz_aptos` é a raiz de Merkle das folhas `H(endereço ‖ peso_ou_faixa)` da
lista na data de corte, montada pela governança integradora. Quem vota apresenta
o caminho. O contrato confere o caminho e usa o peso da folha, nunca o peso
informado na chamada.

Revela a identidade de quem vota — intencional, ver N1. Não revela nada sobre a
escolha.

---

### 6.5 Voto semi-confidencial: campos públicos na mesma cédula

Sugestão de um membro da SDF, e ela encaixa no esquema sem nenhuma primitiva
nova — porque **um campo público é apenas um compromisso cuja abertura é
revelada.**

```
confidencial:  publica C = v·G + r·H  +  prova CDS de que v ∈ {0,1}
público:       publica C = v·G + r·H  +  o par (v, r) em claro
```

O contrato verifica o campo público conferindo `C == v·G + r·H`, que é um MSM
de 2 termos. A agregação é **idêntica** nos dois casos, porque a soma
homomórfica não distingue um compromisso aberto de um fechado. Consequências:

- Um mecanismo só atende os dois modos. Não há segundo caminho de código.
- **Campo público é mais barato que confidencial:** ~5,4M (um MSM de 2 termos)
  contra ~27M da prova CDS. Publicidade economiza orçamento.
- A apuração não muda em nada.

#### Casos de governança que isso destrava

1. **Cédula com várias perguntas, sigilo por pergunta.** "Aprovar a pauta"
   pública, "destituir o diretor" confidencial, na mesma transação.
2. **Publicidade opcional por votante.** Quem vota *escolhe* publicar. Vários
   estatutos de cooperativa e regimentos de conselho asseguram o direito de
   registrar voto divergente em separado, justamente como proteção jurídica de
   quem divergiu. Hoje esse direito é incompatível com voto secreto; aqui os
   dois coexistem na mesma urna.
3. **Categoria pública, direção secreta.** "Voto como delegado da região X" é
   público e auditável; em que votou, não.
4. **Participação pública, escolha secreta.** É o desenho base (N1), e agora se
   revela como o caso mais simples de um mecanismo geral.

### 6.6 O teorema da partição

**Toda informação pública particiona o conjunto de anonimato.** Esta é a mesma
afirmação que §6.3 fazia sobre pesos, generalizada — e perceber que são o mesmo
teorema é o que torna o semi-confidencial seguro em vez de perigoso.

Dois exemplos de como a aritmética contorna criptografia perfeita:

- **Subtração.** 50 votantes, 48 publicam, total conhecido. Os 2 confidenciais
  saem por subtração. O sigilo dos dois compromissos é perfeito e irrelevante.
- **Correlação de campo.** "Delegado da região X" é público; a região X tem 3
  votantes e vota em bloco. A escolha secreta dos três vaza do total da região.

**A regra.** Sejam `F` os campos públicos de uma cédula. Eles induzem uma
partição dos votantes em células, uma por combinação de valores observados.
Em `apurar()`, o contrato:

1. calcula a partição induzida por todos os campos públicos;
2. conta quantas cédulas **confidenciais** caem em cada célula;
3. **recusa publicar** o total de qualquer célula com menos de `τ`
   confidenciais (`τ ≥ 5` na v1, igual a §6.3).

A publicidade opcional precisa dessa regra com mais força que os pesos, porque
é **adversarialmente explorável**: uma coligação que controle muitos votantes
pode publicar todos os seus votos de propósito, encolhendo o conjunto secreto
até determiná-lo. Sem a regra, o atacante escolhe quando o sigilo dos outros
acaba.

**Trade-off a declarar:** impor `τ` converte uma quebra de privacidade numa
**falha de liveness** — o atacante não lê os votos, mas consegue travar a
apuração. É a troca certa (uma votação travada é contestável e repetível; um
voto vazado não volta atrás), e precisa estar no regimento de quem integra, não
só no contrato.

> **Escopo na v1.** Entra a **publicidade opcional por votante** (caso 2): é o
> caso mais simples, o de maior valor para governança real, e o mais barato de
> implementar. Cédula multi-pergunta com sigilo por campo (caso 1) fica
> especificada e vai para a v1.1. A regra de `τ` de §6.6 entra junto com o
> primeiro, não depois — sem ela o recurso é uma armadilha.

## 7. Orçamento de custo

Teto **medido** por transação: **400.000.000 instruções de CPU** (§10.2).

### 7.1 Primitivas medidas na testnet

| operação | instruções | medido |
|---|---|---|
| `g1_add` | 110.748 | ✅ |
| `g1_mul` | 3.290.524 | ✅ |
| `g1_msm` (k=2) | 5.398.553 | ✅ |
| `g1_msm` (k=8) | 14.230.058 | ✅ |
| `hash_to_g1` | 2.653.011 | ✅ |
| `pairing_check` (1 par) | 17.317.607 | ✅ |
| `pairing_check` (k pares) | 10.571.128 + 6.746.479·k | ✅ |
| Groth16 verify (4 entradas públicas) | 47.371.348 | ✅ |
| `verify_schnorr` (invocação cheia) | 6.737.614 | ✅ |
| apuração conferida, constante no peso | 6.712.183 | ✅ |
| `g1_is_on_curve` | 4.367 | ✅ |
| `g1_is_in_subgroup` | 734.877 | ✅ |
| validação completa de um ponto | 738.901 | ✅ |

Marginal do MSM por termo ≈ 1,47M contra 3,29M de um `g1_mul` solto (base
≈ 2,46M). Consequência de desenho: **toda verificação com múltiplos termos é um
MSM único, nunca muls somados.**

### 7.2 Orçamento por transação

**`votar()`, proposta binária (`m = 2`):**

| item | custo | fonte |
|---|---|---|
| 2 × prova CDS | 21.960.486 | medido |
| 1 × prova de soma | 6.737.614 | medido |
| validação de 6 pontos G1 | 4.433.406 | medido |
| caminho de Merkle (depth 8, sha256) | ~1M | estimado |
| 2 × `g1_add` no acumulador | 221.496 | medido |
| **total** | **33.353.002** | |

Folga contra os 400M: **12×**. O orçamento usa 8,3% de uma transação, e não
sobrou nenhuma projeção grande: tudo acima foi medido em invocação real.

**O host não valida subgrupo sozinho:** `g1_add` custa 110.748, que é 15% de
uma checagem isolada, e uma operação não contém algo 7× mais caro que ela.
Logo `votar()` tem de conferir explicitamente todo ponto que recebe, e esse
custo é aditivo.

**`apurar()`:**

| item | custo | fonte |
|---|---|---|
| 2 × MSM de 2 termos (`T_j·G + R_j·H`) | ~10,8M | medido |
| leitura dos acumuladores | desprezível | |
| **total** | **~11M** | |

Constante no comparecimento. Não há busca on-chain, e essa é a diferença entre
um protocolo que funciona e um que não — ver §11.2.

### 7.3 Taxas observadas

| invocação | taxa |
|---|---|
| `verify_schnorr` | 9.558 stroops |
| `tally_checked` (3 cédulas) | 11.852 stroops |

~0,00096 XLM por verificação de prova, da ordem de US$ 0,0002 ao preço corrente.
A taxa não é barreira de comparecimento, e quem vota paga a própria transação —
o que evita que a governança tenha de subsidiar e, com isso, ganhe um ponto de
controle sobre quem consegue votar.

### 7.4 Custo de aluguel de estado

**Não estimado.** `Votou(id, addr)` é uma entrada persistente por votante.
Para 10.000 votantes são 10.000 entradas. Precisa de número antes de qualquer
afirmação sobre assembleias grandes. Ver §9.3.

---

## 8. Componentes fora da cadeia

### 8.1 Cliente de votação

Responsabilidades:

1. Buscar a proposta e a lista de aptos; montar o caminho de Merkle.
2. Sortear `r_{i,j}` de um CSPRNG. **Aleatoriedade fraca aqui quebra o sigilo
   permanente inteiro** — é o único ponto do desenho em que uma falha de
   implementação anula uma garantia information-theoretic.
3. Montar os compromissos e as provas.
4. Dividir cada `r_{i,j}` em shares Shamir `k`-de-`N` e entregá-las aos membros
   da mesa por canal autenticado e cifrado.
5. Submeter a transação.
6. Guardar o recibo de verificabilidade individual (`C_j` e a posição no
   acumulador) — e avisar quem vota, de forma explícita, que **esse recibo é
   também um instrumento de coação** (N2), e que apagá-lo é a escolha segura
   depois de conferir.

### 8.2 Mesa apuradora

1. Receber e validar shares.
2. Somar localmente, por opção.
3. Após o prazo, `k` membros reconstroem `R_j`.
4. Chamar `apurar()`.
5. **Destruir** shares e material intermediário.

O passo 5 não é auditável por meio criptográfico. É um compromisso
procedimental, e o desenho reduz sua importância (§4.3) sem eliminá-la.

### 8.3 Verificador público

Programa independente que, a partir apenas do ledger:

1. Relê todos os `votar()` da proposta e reconstrói `A_j` somando os `C_j`.
2. Confere `A_j == T_j·G + R_j·H` com os valores publicados.
3. Reconfere todas as provas de boa formação.
4. Confere que todo votante estava na raiz de aptos e votou no máximo uma vez.

Se isso não existir e não for fácil de rodar, P5 é decorativa. É entregável de
primeira classe, não um extra.

**Dois níveis, com prazos diferentes** (medido no smoke A2, 2026-10-01):

A janela de retenção do RPC público da testnet é de **120.960 ledgers = 7,00
dias**, e é aplicada: `getEvents` fora dela recusa. Isso não afeta leitura de
**estado** nem os **arquivos de histórico**, que são permanentes.

| nível | o que prova | fonte | validade |
|---|---|---|---|
| 1 | o total publicado corresponde ao acumulador | estado do contrato | permanente, sujeito ao TTL (§9.3) |
| 2 | o acumulador é a soma exata das cédulas aptas, não repetidas e bem formadas | eventos via RPC | **7 dias** |
| 2′ | idem | arquivos de histórico | permanente, mais lento |

**Decisão de desenho:** o verificador lê dos arquivos de histórico por padrão,
e usa o RPC como atalho quando a proposta está dentro da janela. Um verificador
que só saiba falar com o RPC funciona no dia da votação e falha na segunda
semana, em silêncio.

---

## 9. Pendências de medição

Ordenadas por risco. As três primeiras são de dia 1.

### 9.1 Prova disjuntiva CDS — ✅ RESOLVIDO

**Medido em 2026-10-01 (sonda 12): 10.980.243 instruções, 14.144 stroops.**
A projeção de ~27M errou por 2,5×, para o lado seguro, porque supunha muls
somados em vez de um MSM de 2 termos por ramo.

E o vetor de prova veio do `core` em Rust nativo e verificou no host da
testnet — provador e verificador concordam (smoke B3).

### 9.2 Desserialização de ponto com checagem de subgrupo — ✅ RESOLVIDO

**Medido em 2026-10-01 (sonda 10):** on-curve 4.367, in-subgroup 734.877,
as duas 738.901 por ponto. Seis pontos por `votar()` custam 4.433.406, ou 1,1%
da transação. A checagem de subgrupo custa 0,22× um `g1_mul`, o que indica
checagem por endomorfismo e não multiplicação pelo cofator.

E o host **não** a faz sozinho: validação é obrigatória e aditiva.

### 9.3 Aluguel de estado para `n` grande

**Ação:** medir o custo de escrever e manter 1.000 entradas `Votou`.

### 9.4 Verificação de abertura revelada (campo público)

Introduzida por §6.5. Projetada em ~5,4M (um MSM de 2 termos, medido), mas o
caminho completo com desserialização ainda não foi isolado.

### 9.5 Menores

- Custo do caminho de Merkle em função da profundidade.
- Custo do `require_auth` multi-assinatura de `k` membros em `apurar()`.
- Tamanho do wasm do contrato completo (a sonda tem 8.981 bytes).

---

## 10. Base empírica

Tudo nesta seção veio de invocação em rede real, não de simulação.

**Ambiente:** `rustc 1.97.1`, `soroban-sdk 28.0.0`, `soroban-env-host 28.0.2`,
`stellar-cli 25.2.0`, alvo `wasm32v1-none`. Sonda: 8.981 bytes de wasm, 7/7
testes passando.

**Contrato da sonda:** `CA2LFMOOLAFHUNYBD2MLNEUINYN76IZ6AOGNDPH27UX4MHOVIHGPTRTP`
**Identidade descartável:** `urna-smoke` (`GCZC4HW5...UX4NLGWJ`), financiada pelo friendbot.

### 10.1 O que foi confirmado

| pergunta | resposta |
|---|---|
| As host functions BLS12-381 existem no pin do SDK 28? | Sim |
| O host aceita `be_bytes(X) ‖ be_bytes(Y)`, 96 bytes? | Sim |
| Um sigma-protocolo verifica dentro do contrato? | Sim, `verify_schnorr` → `true` |
| A apuração homomórfica fecha ponta a ponta? | Sim, três cédulas de pesos 5, 3, 1 → total 9 |
| O contrato rejeita um total falso? | Sim, mesa alegando 10 → `false` |

### 10.2 O teto de CPU é 400M, não 100M

Medido por bissecção com `add_n` na testnet: **n=3500 (~387M) passa, n=3600
(~398M) devolve `HostError: Error(Budget, ExceededLimit)`**. A premissa inicial
de 100M estava 4× conservadora, e a correção mudou o veredito do projeto de
"precisa amortizar" para "cabe com folga".

### 10.3 Armadilha de implementação já encontrada

`Bls12381Fr::from_bytes` **não reduz módulo `r`**. Um hash de 32 bytes usado
direto como escalar pode exceder `r`. A sonda zera o byte mais significativo,
garantindo `e < 2^248 < r`. Custa ~8 bits de entropia do desafio, o que é
irrelevante para 128 bits de segurança, e evita um bug silencioso.

O contrato completo deve usar a mesma construção, ou reduzir corretamente.

---

## 11. Desenhos rejeitados

Registrados porque são as primeiras ideias de qualquer pessoa, e porque saber
por que não funcionam é parte da especificação.

### 11.1 Commit-e-revela

Quem vota publica `H(voto ‖ nonce)` e depois revela.

- **Não é resistente a coação.** A revelação é literalmente um recibo.
- **Duas fases quebram o quórum.** Quem não revela vira abstenção retroativa, e
  o resultado passa a depender de quem apareceu na segunda fase.
- **Quem revela por último vê o parcial** e decide com informação que os outros
  não tiveram.
- Num esquema com pesos, os pesos revelados fingerprintam os votantes.

### 11.2 Fazer a cadeia procurar o total

Na variante ElGamal, recuperar `T` do agregado exige log discreto por força
bruta: `M = T·G`, testar `T = 0, 1, 2, ...`. Medido: **~124k instruções por
unidade de peso**, o que dá teto de **~3.218 de peso total por transação**.
Qualquer assembleia real estoura.

Substituído por **conferir em vez de procurar**: a mesa afirma o total e o
contrato verifica. Custo **6.712.183 constante no peso** — medido idêntico para
peso 250 e para 1.000.000. Esta foi a medição que mais mudou o desenho.

O desenho Pedersen final herda a mesma lógica e fica ainda mais barato
(~5,4M, um MSM de 2 termos, contra 6,7M), porque a abertura do agregado
substitui a decifração.

### 11.3 ElGamal exponencial na cadeia

Foi o que a sonda implementou e mediu, e funciona. Rejeitado como desenho final
por um motivo só: grava texto cifrado no ledger, e portanto tem prazo de
validade (§0.1). Continua sendo a base empírica de todo o modelo de custo, e é
o que está publicado na testnet hoje. Vale notar que o voto anônimo do Tansu
é este desenho, implementado e em produção por outra equipe — ver §14.

### 11.4 Pesos públicos arbitrários

Ver §6.3. Não é rejeição de uma técnica, é rejeição de uma configuração: o
contrato recusa.

---

## 12. Roteiro

### 12.1 v1 — o que entra no hackathon

| item | estado |
|---|---|
| Sonda criptográfica medida e publicada | ✅ feito |
| Trocar DST para Tessera | pendente |
| Medir CDS, desserialização, aluguel (§9) | pendente, dia 1 |
| `abrir` / `votar` / `apurar` | pendente |
| Cliente de votação (CLI é suficiente) | pendente |
| Verificador público independente | pendente |
| Deploy na testnet + vetores de teste | pendente |
| Vídeo de demonstração ponta a ponta | pendente |

**Congelamento de código: 2026-10-04, 12:00.** Submissão fecha 2026-10-05 19:00.
Uma interface gráfica não entra na v1; o critério de "user experience" fica
servido pelo CLI e pelo verificador, e isso é uma perda consciente de pontos,
não um esquecimento.

### 12.2 v1.1 — fechar a lacuna de coação

N2 é a lacuna mais séria. Caminho: um relayer que re-randomiza o compromisso
(`C' = C + r'·H`) sem conhecer o voto, de modo que quem vota não consiga mais
produzir uma abertura válida do que está no ledger. Custa uma suposição sobre o
relayer e uma transação extra. Projetado, não medido.

### 12.3 v2 — desvincular estruturalmente

Substituir a prova de aptidão por uma credencial cega, de modo que o ledger
registre um compromisso válido sem registrar **quem** o emitiu. Remove N1 e a
última suposição procedimental sobre a mesa.

**O orçamento on-chain já está medido e não é o gargalo.** `pairing_check`
compartilha a exponenciação final entre os pares:

```
pairing_check(k) = 10.571.128 + 6.746.479 · k      (medido, sonda 6)
```

Como Groth16 se reduz a *um* produto de pareamentos com 4 pares, a verificação
completa custa **47.371.348 instruções com 4 entradas públicas: 11,8% do teto,
e caberiam 8 numa transação.** O marginal por par é 2,6× mais barato que o
primeiro par, e é exatamente isso que torna ZK viável aqui.

O que falta não é a cadeia: é circuito, setup confiável e provador no cliente.
O ecossistema já tem trilha para os dois caminhos — Nethermind otimizou a
verificação Noir/UltraHonk para Soroban movendo operações para host functions
dos protocolos 25/26, com redução de 64% de instruções e taxa caindo de 0,23
para 0,09 XLM (2026-08-04), e existe receita pública de Circom + snarkjs
verificando na testnet. A escolha entre Groth16/Circom e UltraHonk/Noir é de
engenharia, e as duas caberiam.

O nullifier de §5.1 passa a ser necessário nesta versão.

---

## 13. Plano de testes

**Unitários (`cargo test`)**

- Compromisso é homomórfico: `C(v_a,r_a) + C(v_b,r_b) == C(v_a+v_b, r_a+r_b)`.
- Abertura do agregado confere com valores honestos.
- Abertura do agregado **falha** com `T` alterado em 1.
- Abertura do agregado **falha** com `R` alterado em 1.
- CDS aceita `v=0` e `v=1`; **rejeita** `v=2` e `v=-1`.
- CDS rejeita prova com ramos trocados e com desafio adulterado.
- Prova de soma rejeita `Σv ≠ w`.
- Voto duplo é rejeitado.
- Caminho de Merkle inválido é rejeitado.
- Peso informado divergente da folha é rejeitado.
- `abrir()` recusa tabela de pesos com faixa abaixo de `τ`.
- Apuração antes do prazo é rejeitada.
- Apuração com menos de `k` assinaturas é rejeitada.

**De custo** — cada um assere um teto, para que uma regressão de custo quebre o
build em vez de aparecer na demo:

- `votar()` binário cabe em 400M com folga ≥ 2×.
- `apurar()` é constante no número de votantes (medir com 10 e com 1.000).
- Custo de apuração independe do peso (medir com 250 e com 1.000.000, como a
  sonda já faz).

**De integração, na testnet** — o roteiro do vídeo é exatamente este:

1. Abrir uma proposta com 5 aptos, um voto por pessoa.
2. Cinco votos de clientes distintos, um deles divergente da maioria.
3. Apurar com `k=3` de `N=5`.
4. Rodar o verificador público e mostrar que fecha.
5. Mostrar que o ledger, lido inteiro, não distingue quem votou o quê.
6. Tentar apurar um total falso e mostrar o contrato recusando.

O passo 5 é o pitch inteiro em uma tela. O passo 6 é o que diferencia de um
sistema que apenas *afirma* ser verificável.

---

## 14. Trabalho relacionado: o voto anônimo do Tansu

O Tansu ([tansu.dev](https://tansu.dev), Áustria, SCF #28, #30 e Public Goods
Q3'26, US$ 271.720) implementou voto anônimo na Soroban com **o mesmo esquema
criptográfico deste documento**: compromisso de Pedersen sobre BLS12-381,
`C = g·v + h·r`, apuração por soma homomórfica, geradores derivados por
`hash_to_g1` com DST. O anúncio é de 2025-06-17; o código auditado abaixo é o
branch `main` de `tupui/soroban-versioning`, último push 2026-03-09, lido em
2026-09-30.

Registrar isso aqui tem duas funções. A primeira é honestidade: **Pedersen
homomórfico na Soroban não é inédito, e nenhuma alegação de primazia deve
aparecer em material de divulgação do Tessera.** A segunda é que o Tansu é o
contraexemplo que justifica §0.1 melhor do que qualquer argumento abstrato —
ele é o desenho de §11.3, implementado e em uso.

### 14.1 Onde os dois esquemas divergem

| | Tansu (`main`, 2026-03-09) | Tessera v1 |
|---|---|---|
| Texto cifrado do voto no ledger | **sim** — `encrypted_votes` e `encrypted_seeds` por votante | **não**, por construção (§0.1) |
| Sigilo sobrevive ao vazamento da chave | **não** | sim (P1) |
| Mesa apuradora | **única**, `k = 1`, detém a chave que decifra todos os votos | limiar `k`-de-`N` (P2, §4.3) |
| Prova de que `v ∈ {0,1}` | **nenhuma** | CDS disjuntiva (§6.1) |
| Prova de soma correta | nenhuma | Schnorr em base `H` (§6.2) |
| Pesos públicos distintos | aceitos sem mitigação | **recusados** pelo contrato (§6.3, §11.4) |
| Opções por cédula | 3, fixas | `m`, genérico |
| Elegibilidade | colateral em XLM | raiz de Merkle de aptos (§6.4) |
| Custo da apuração | `O(n·m)` multiplicações de ponto numa transação | constante em `n` e em peso (§7, medido) |

### 14.2 Os três achados que importam

**A. O ledger guarda o voto cifrado, ao lado do endereço.** A struct persistida
é:

```rust
pub struct AnonymousVote {
    pub address: Address,
    pub weight: u32,
    pub encrypted_seeds: Vec<String>,   // cifrado com a chave pública da DAO
    pub encrypted_votes: Vec<String>,   // cifrado com a chave pública da DAO
    pub commitments: Vec<BytesN<96>>,
}
```

A chave pública vem de `AnonymousVoteConfig`, fixada em
`anonymous_voting_setup`. O compromisso é perfeitamente ocultante e **está
acompanhado, no mesmo registro permanente, do criptograma do voto e da
semente.** Quem obtiver a chave privada — por vazamento, por intimação, ou por
envelhecimento do parâmetro em 2046 — lê o voto individual de cada pessoa,
nominalmente, porque `address` está no mesmo registro. É exatamente a ameaça de
§0.1, e é a razão pela qual o sigilo do Tessera é incondicional em vez de
cifrado.

**B. Não há prova de boa formação.** Em voto anônimo, `vote()` verifica duas
coisas: que `commitments.len() == 3` e que cada entrada desserializa como ponto
de `G1`. Nada vincula `v` a `{0,1}`. Como `build_commitments_from_votes` roda
em simulação no cliente, o `v` é escolhido livremente por quem vota: um
compromisso com `v = 10^9` entra, a soma homomórfica fecha, `proof()` retorna
`true` e a apuração conta um bilhão de aprovações. A única barreira é o
maintainer notar ao decifrar — isto é, a integridade repousa na mesa confiável,
não no contrato. §6.1 e §6.2 existem para remover isso do contrato, e são a
contribuição técnica própria do Tessera.

**C. Pesos públicos sem mitigação.** `weight: u32` é público por votante e
`proof()` escala cada compromisso por ele. Não há faixas, não há `τ`, não há
modo `UmPorPessoa`. **O ataque de subconjunto-soma de §6.3 se aplica ao Tansu
sem adaptação alguma.** A diferença de postura é a tese de §11.4: aqui o
contrato recusa a configuração em vez de documentá-la como cuidado.

### 14.3 Observações menores, registradas para não se perderem

- **Sementes são `u128`**, não escalares em `Fr` (≈255 bits). Com `r` restrito a
  2^128 de ≈2^255, o compromisso é estatisticamente — não perfeitamente —
  ocultante. Não é praticamente explorável, e é discutível apenas porque o
  achado A já torna o ponto irrelevante. Aqui `r` é uniforme em `Fr` (§3.1).
- **`MAX_VOTES_PER_PROPOSAL = 1000`** e `proof()` percorre todos os votos
  fazendo 3 `g1_mul` + 3 `g1_add` por voto, numa única invocação. É `O(n·m)`
  contra um teto de 400M. **Não medimos onde quebra** e o repositório deles
  também não: todos os casos de `test_cost_estimates.rs` usam `PublicVote`, e
  não existe teste de orçamento para a apuração anônima. A afirmação aqui é
  sobre a forma do custo, não sobre um limite numérico.

### 14.4 Consequência prática

O Tansu tem dapp em funcionamento, três rodadas de SCF e tração real — e lhe
falta exatamente a camada especificada neste documento. Isso faz dele o
primeiro candidato a integrador do Tessera, não um concorrente a ser contornado.
A história de adoção "o Tessera pluga aqui" é concreta em vez de hipotética, e o
contato natural é Pamphile Roy ([github.com/tupui](https://github.com/tupui)).

---

## Apêndice A — vetores de teste

Em `../bls-smoke/vetores.env`: gerador `G`, chave pública `PK`, compromisso
Schnorr `A`, resposta `Z` (hex e decimal), três pares ElGamal `C1/C2` de pesos
5, 3 e 1, chave secreta `31337`, total esperado `9`.

Gerados por `emitir_vetores_para_testnet` e usados para invocação real via
`stellar contract invoke`. Servem de referência cruzada para qualquer
reimplementação.

## Apêndice B — reproduzir

```bash
cd bls-smoke
cargo test --lib -- --nocapture --test-threads=1
stellar contract build
```

## Apêndice C — origem deste documento

Este spec nasceu em `experiments/`, um repositório de sondas de viabilidade,
e foi movido para o repositório do Tessera em 2026-10-01 sem alteração de
conteúdo. A sonda `bls-smoke` veio junto, como o registro de onde os números
vieram: nenhuma cifra deste documento é estimativa de gabinete.
