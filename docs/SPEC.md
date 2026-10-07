# Tessera — especificação de trabalho

Formato definido em [`CLAUDE.md`](../CLAUDE.md). A especificação **do protocolo**
— modelo de ameaça, criptografia, orçamento, desenhos rejeitados — vive em
[`PROTOCOLO.md`](PROTOCOLO.md); as citações no código (`PROTOCOLO §6.4`) apontam
para lá.

---

## §0 Regras para agentes

**Approved: 2026-10-07** — dado em conversa ("vamos ignorar a timeline e fazer
o que falta"), depois de §11-A, §11-B e §11-C respondidas. Registrado por mim a
pedido; a decisão é do humano.

Vale o `CLAUDE.md` da raiz, com estas adições deste projeto:

1. **Número medido ou nenhum número.** Custo de CPU, taxa e vazão só entram em
   documento, tela ou mensagem de commit depois de sair de um teste que roda ou
   de uma invocação na testnet. Somar primitivas de cabeça já errou 9,8% uma vez
   e 8× outra. Projeção é marcada como projeção.
2. **Identidades de testnet são descartáveis.** Nenhuma chave de mainnet entra
   neste repositório, em nenhuma circunstância.
3. **`console/` e `console/ponte.py` não são apagados.** São o ambiente
   controlado da gravação e a rede de proteção se o prazo apertar.
4. **O `git push` é do humano.** Nunca sai sem a palavra dele na sessão corrente.

---

## §1 Objetivos e não-objetivos

**Marco corrente: submissão do hackathon Find Your Way (Meridian), trilha
General, até 12 out 2026 20:59** ([fonte](SOURCES.md)).

Sucesso é: um módulo de votação para Soroban em que se sabe quem compareceu, não
se sabe em que cada pessoa votou, não se sabe de quem é cada cédula, e qualquer
pessoa recalcula o resultado do ledger. Mais o que a submissão exige — vídeo e
repositório público legível.

**Não-objetivos deste marco:**

- mainnet;
- ser um aplicativo de governança ou uma DAO (é um módulo que uma governança
  existente chama);
- resistência à coação (ver §4, INV-12 — está declarado como limite, não
  resolvido);
- voto ponderado com pesos públicos distintos (o contrato **recusa**).

---

## §2 Política de afirmações

O que o projeto pode dizer sobre si, e com que evidência. Palavras como "seguro",
"auditado" ou "pronto para produção" **não** aparecem em lugar nenhum: nada aqui
foi auditado por terceiros.

| Pode afirmar | Evidência |
|---|---|
| "o compromisso publicado é perfeitamente ocultante" | propriedade do compromisso de Pedersen; `PROTOCOLO §3` |
| "o anonimato do anel é computacional, **não** perfeito" | repousa em DDH/XDH; dito explicitamente no README e na landing |
| "quem faltou é público, de quem é cada cédula não é" | **só na votação fechada** — `contrato/src/test.rs::o_caderno_diz_quem_faltou_e_a_urna_nao_diz_de_quem`; interseção caderno ∩ urna = 0 em `app/scripts/rodada-anel.mjs` |
| "o sigilo da escolha e a desvinculação valem nos dois modos" | o anel e o compromisso não dependem da lista; DEC-006 |
| custo em instruções de qualquer operação | o teste que o mede, citado junto do número |
| "roda na testnet" | hash da transação, verificável no explorer |
| "não há servidor que veja o fator de aleatoriedade" | o dapp é estático; as provas nascem no navegador |
| "ninguém pode abrir uma cédula em anel" | ela não reparte o fator com a mesa; `AberturaNaoFecha` recusa qualquer total afirmado |

**Proibido afirmar na votação aberta:** que há voto obrigatório, que existe
lista de quem faltou, ou que o total significa alguma coisa — qualquer pessoa
vota quantas vezes quiser criando carteiras, e na testnet o friendbot as
financia de graça. O que a votação aberta demonstra é **sigilo**, não contagem.

**Proibido afirmar:** que o dapp está publicado (não está), que houve auditoria,
que há garantia contra coação, que **não existe vínculo nenhum fora do ledger**
(o friendbot vê IP e endereço efêmero — DEC-004), ou qualquer número que não
venha de medição.

---

## §3 Arquitetura

```
core/          a matemática, compartilhada. Usa arkworks — o MESMO crate do
               host do Soroban, o que elimina pela raiz a divergência entre
               provador nativo e verificador Wasm.
  ├ pedersen   compromissos
  ├ cds        prova disjuntiva (voto ∈ {0,1})
  ├ anel       LSAG — o CDS generalizado de 2 para n ramos
  ├ merkle     folha, árvore, caminho, divisão em seções
  ├ shamir     repartição k-de-n do fator
  └ ponto      serialização G1 (inclui o infinito como flag zcash)

contrato/      o Wasm Soroban. Verifica; nunca prova.
cliente-wasm/  core compilado para o navegador (wasm-bindgen)
cli/           cliente e verificador de linha de comando
app/           o dapp (React + TS, estático)
console/       o visor da demonstração — NÃO é aplicativo, não assina nada
bls-smoke/     as 13 sondas que estabeleceram o modelo de custo
```

**Fronteiras de dependência**, e a lista só encolhe:

- `core` não depende de nada do projeto. É o único que `contrato`, `cli` e
  `cliente-wasm` podem compartilhar.
- `contrato` depende de `core` e do SDK Soroban. **Não** depende de `cli`,
  `app` nem `cliente-wasm`.
- `app` fala com a cadeia só por `src/rede.ts`, e com a matemática só por
  `src/wasm.ts`. Nenhuma página importa o SDK Stellar direto.
- `console/` não depende de `app/`, e vice-versa.

**Exceção permitida:** `app/scripts/*.mjs` importam o SDK direto, porque são
scripts de carga fora do dapp.

---

## §4 Invariantes

Os invariantes são o produto. Uma mudança que possa quebrar um deles para e
pergunta.

| # | Enunciado | Imposto por |
|---|---|---|
| INV-01 | O ledger **nunca** recebe texto cifrado do voto, só o compromisso | desenho; não existe caminho que cifre — ver §3 |
| INV-02 | Nenhum arquivo gravado contém o fator `r` | `cli` `o_json_nao_tem_lugar_para_aleatoriedade`; `console/embutir.py` (lista `PROIBIDO`) |
| INV-03 | O diário nunca grava nome de segredo junto de um valor | `app/scripts/diario.test.mjs` |
| INV-04 | Uma chave efêmera assina **uma cédula**, nunca duas | `app/scripts/usoUnico.test.mjs` |
| INV-05 | O caderno e a urna não se ligam | `contrato` `o_caderno_diz_quem_faltou_e_a_urna_nao_diz_de_quem` |
| INV-06 | A mesma pessoa não vota duas vezes, e a recusa não revela quem é | `contrato` `ninguem_vota_duas_vezes`; `core` `a_mesma_pessoa_produz_a_mesma_imagem` |
| INV-07 | A imagem de chave não atravessa propostas | `core` `a_imagem_nao_atravessa_propostas` |
| INV-08 | A seção está presa na folha de Merkle: o votante não escolhe a sua | `core` `a_secao_esta_presa_na_folha`; `contrato` `trinta_votantes_em_tres_secoes_pagam_o_preco_de_dez` |
| INV-09 | A divisão em seções não depende da ordem da lista nem de quem organiza | `core` `a_divisao_nao_depende_da_ordem_em_que_a_lista_chega`, `a_divisao_e_equilibrada_e_ninguem_fica_sozinho` |
| INV-10 | Peso inflado não chega na raiz | `core` `peso_inflado_nao_chega_na_raiz`; `contrato` recusa com `PesoNaoUnitario` |
| INV-11 | Uma mesa que mente no total é recusada **na hora**, não denunciada depois | `contrato` `mesa_que_mente_no_total_e_recusada` |
| INV-12 | Abaixo de `TAU` cédulas confidenciais, a apuração trava em vez de vazar | `contrato` `abaixo_de_tau_a_apuracao_trava_em_vez_de_vazar` |
| INV-13 | Um anel de 20 cabe numa transação | `contrato` `ate_quantas_pessoas_cabe_um_anel` (assere o teto) |
| INV-14 | O XDR do endereço tem 44 bytes e bate entre CLI e navegador | `cli` `xdr_bate_com_o_que_o_sdk_produz`; `app/scripts/xdr.test.mjs` |
| INV-15 | O infinito em G1 é a flag zcash, não 96 zeros | `contrato` `o_infinito_do_host_e_a_flag_zcash_nao_zeros` |
| INV-16 | Tudo o que a tela da demonstração afirma, o código sustenta | `console/guarda.py` — 8 afirmações, 33 conferências contra a fonte |
| INV-17 | As provas são geradas pelo `core` nativo e verificadas no host Wasm a cada `cargo test` | `contrato/src/test.rs` inteiro — não é vetor congelado |

---

## §5 Interfaces e áreas congeladas

**Formatos congelados** (mudam só com bump de versão, dados-ouro regerados e DEC):

| O quê | Onde | Por quê |
|---|---|---|
| Folha de Merkle `H(0x00 ‖ endereço ‖ peso_be ‖ secao_be)` | `core/src/merkle.rs::folha` | muda a raiz; quebra toda prova de aptidão existente |
| Separação de domínio `0x00` folha, `0x01` nó, `0x02` vazio, `0x03` seção | idem | sem ela, folha de 64 bytes vira nó interno |
| Serialização G1, inclusive o infinito | `core/src/ponto.rs` | o host recusa qualquer outra coisa |
| ABI do contrato (`abrir`, `comparecer`, `votar`, `votar_anonimo`, `apurar`) | `contrato/src/lib.rs` | redeploy invalida toda votação aberta e 12 arquivos que pinam o endereço |
| Códigos de erro `Erro` | `contrato/src/tipos.rs` | espelhados em `app/src/rede.ts::ERROS` |
| DSTs do anel (`TESSERA-V1-ANEL`, `-HP`, `-CONJUNTO`) | `core/src/anel.rs`, `contrato/src/cripto.rs` | divergir faz toda assinatura falhar sem dizer por quê |

**Dados-ouro:** `bls-smoke/vetores.env` (vetores públicos) e
`app/scripts/xdr.test.mjs` (o vetor de 44 bytes, espelhado em
`cli/src/chave.rs`).

`contrato/test_snapshots/` **não é dado-ouro**, apesar do nome. Medido em
T-002: rodar o mesmo teste sem tocar no código altera o arquivo. Eles sujam
todo diff com milhares de linhas que ninguém lê — e um arquivo que muda sozinho
não prende nada. Ver T-011.

**Não congelado, mas frágil:** `console/` — qualquer mudança nas telas passa por
`guarda.py` (INV-16).

---

## §6 Dependências e versões

Ver [`SOURCES.md`](SOURCES.md) para a tabela com datas. Resumo:

| | |
|---|---|
| rustc | 1.97.1 |
| stellar-cli | 25.2.0 |
| alvo | `wasm32v1-none` |
| node | 24.14.1 (os testes do app exigem 22+, remoção de tipos nativa) |
| pnpm | 10.33.0 |
| @stellar/stellar-sdk | 14.6.1 |
| matemática | `arkworks`, o mesmo crate do host Soroban |

Contrato na testnet: `CBAFYF5BCTIVPUUO67Y76GF2T2GKYD5JDFLXLBUYRC4QLPN5GZXRWN56`

`[VERIFY]` em aberto: nenhum.

---

## §7 Segurança e segredos

**Modelo de confiança.** O ledger é público e permanente, e é tratado como
hostil ao sigilo: nada que precise ficar secreto é publicado nele, nem cifrado.
Quem vota confia no próprio navegador. A mesa é confiável para **não publicar um
total falso** (o contrato recusa) mas **não** é confiável para guardar segredo —
daí o limiar `k`-de-`n`, e daí a cédula em anel não repartir nada com ela.

**Segredos.** Não há segredo neste repositório. As chaves vivem em
`stellar keys` (CLI) ou no `localStorage` da aba (dapp) e nunca transitam pela
ponte da demonstração. `.gitignore` exclui `target/`, `*.key`, `recibos/`,
`estado/`, `shares/`, `demo/`, `cliente-wasm/pacote*/`, `app/node_modules/`,
`app/dist/`.

`bls-smoke/vetores.env` é rastreado e **não é segredo**: gerador, chave pública,
compromissos e uma resposta de Schnorr, todos públicos por construção.

**Ambientes.** Só testnet. O `REDE` do dapp e o padrão da CLI apontam para
testnet; não existe configuração de mainnet em lugar nenhum.

---

## §8 Portões

Rodam a cada tarefa:

```bash
for d in core contrato cli cliente-wasm bls-smoke; do
  (cd $d && cargo fmt --check && cargo clippy --all-targets -- -D warnings)
done
cd bls-smoke   && cargo test --lib
cd core        && cargo test --lib
cd contrato    && cargo test
cd cli         && cargo test
cd app         && for t in scripts/*.test.mjs; do node "$t"; done
cd app         && npm run build          # inclui tsc -b
python3 console/guarda.py
```

**Linha de base, medida em 2026-10-03 antes de qualquer mudança:**

| Portão | Estado |
|---|---|
| `bls-smoke` | ✅ 13 testes |
| `core` | ✅ 68 testes |
| `contrato` | ✅ 30 testes |
| `cli` | ✅ 25 testes |
| testes do app | ✅ 3 testes |
| `npm run build` | ✅ |
| `console/guarda.py` | ✅ |
| `cargo fmt --check` | ✅ nos 5 crates — era falha de base, resolvida em T-002 |
| `cargo clippy -D warnings` | ✅ nos 5 crates — eram 65 avisos, resolvidos em T-003 |

A linha de base não tem mais falha nenhuma.

Rodada ponta a ponta na testnet (lenta, sob demanda):

```bash
cd app && node scripts/rodada-anel.mjs              # anel único, 7 votantes
cd app && SECOES=3 node scripts/rodada-30.mjs       # 30 votantes, 3 seções
cd app && node scripts/folga.mjs                    # contenção de escrita
```

---

## §9 Marcos e tarefas

**Marco: submissão, 12 out 2026 20:59.** Portão de revisão: todo PR.

| ID | Título | Deps | Lê | Estado | Critérios de aceitação |
|---|---|---|---|---|---|
| T-000 | Spec de trabalho e mudança do protocolo | — | — | review | `docs/SPEC.md` no formato do `CLAUDE.md`; protocolo em `PROTOCOLO.md`; 19 citações no código repontadas; portões passam |
| T-001 | README volta a dizer a verdade | — | — | review | PR #1 |
| T-002 | `cargo fmt` passa | T-000 | §8 | review | `cargo fmt --check` passa nos 5 crates; nenhum teste muda de resultado |
| T-003 | `cargo clippy` sem avisos | T-002 | §8 | review | 0 avisos nos **5** crates; `-D warnings` no portão |
| T-004 | Vídeo da demonstração | T-001 | §2 | todo | roteiro + gravação com `/bastidores` na segunda janela; nada encenado |
| T-005 | Publicar o dapp | T-001 | §7 | todo | estático no ar; o README deixa de dizer "não publicado" |
| T-006 | Atualizar os decks | T-001 | §2 | todo | slide 06 deixa de listar desvinculação como futura; seções aparecem |
| T-007 | "O modo" em `/abrir` | T-001 | §4 | todo | a tela não oferece votação que `/votar` recusa |
| T-008 | `/apurar` junta as parcelas da mesa | T-005 | §4 | todo | apuração pelo dapp, sem CLI |
| T-009 | Assembleia sem mesa nenhuma | T-000 | §5, §10 | todo | `limiar == 0` aceito sse `mesa` vazia; redeploy; os 12 arquivos repontados; `/abrir` e `/apurar` param de exigir mesa |
| T-010 | `/abrir` avisa quando a seção nasce pequena | T-009 | §10 | todo | recusa abrir com `aptos / secoes < TAU`, dizendo o tamanho que daria; teste do cálculo |
| T-011 | Parar o churn de `test_snapshots/` | T-002 | §5 | todo | ou viram determinísticos, ou saem do git; nenhum diff futuro os carrega |
| T-013 | Votação aberta e votação fechada | T-003 | §2, §4, §5 | review | `raiz_aptos` de 32 zeros = aberta; `comparecer` pula Merkle; seção por ordem de chegada; a tela diz o que cada modo não garante; redeploy junto de T-009 |
| T-012 | Migrar os eventos para `#[contractevent]` | T-003 | §5 | todo | `env.events().publish()` sai; `app/src/rede.ts` lê o formato novo; a lista de votações continua funcionando |

---

## §10 Registro de decisões

### DEC-001: O protocolo sai de `SPEC.md` para `PROTOCOLO.md` (2026-10-03, T-000)

**Contexto.** `CLAUDE.md` reserva `docs/SPEC.md` para a especificação de trabalho
do agente, com §0–§11 de significados próprios. Lá já vivia a especificação do
protocolo, 1.300 linhas com §0–§14, citada por 19 comentários no código
(`SPEC §6.4`, `§6.6`, `§7.2`, `§6.3`).

**Decisão.** Mover para `docs/PROTOCOLO.md` e repontar as 19 citações para
`PROTOCOLO §x.y`.

**Alternativas.** (a) Pôr a spec do agente em outro caminho e editar o
`CLAUDE.md` — rejeitada: o arquivo diz que é agnóstico de projeto e não se edita.
(b) Fundir os dois documentos — rejeitada: públicos diferentes, numerações
incompatíveis. (c) Deixar o conflito registrado em §11 sem resolver — rejeitada:
o humano pediu a mudança explicitamente.

**Consequências.** 19 comentários no código e 8 documentos mudaram de texto;
nenhum comportamento mudou. `PROTOCOLO.md` ganhou nota de proveniência no topo.

### DEC-002: Medições internas não entram em `SOURCES.md` (2026-10-03, T-000)

**Contexto.** `CLAUDE.md` pede registrar "todo fato externo conferido".
Custo de CPU e vazão são fatos *do projeto*, não externos.

**Decisão.** `SOURCES.md` guarda só fato externo e versão de ferramenta.
Medição interna vive no teste que a produz, que é o que a mantém verdadeira.

**Consequências.** §8 e §4 apontam para testes, não para a tabela.

### DEC-003: Assembleia sem mesa nenhuma, com o redeploy que vem junto (2026-10-07, T-009)

**Contexto.** §11-A. O contrato exige mesa (`limiar == 0 || limiar > mesa.len()`
cai em `LimiarInvalido`), mas numa cédula em anel a mesa **não recebe parcela
nenhuma**: existe no estado e não serve para nada. A tela precisava explicar uma
exigência sem função.

**Decisão.** Permitir `limiar == 0` se e somente se `mesa` for vazia. O humano
aceitou o redeploy em 2026-10-07.

**Alternativas.** Manter e explicar na tela — rejeitada: a tela já explicava, e
explicar bem uma coisa errada continua sendo uma coisa errada.

**Consequências.** Mudança de ABI, logo redeploy, logo toda votação aberta morre
e os 12 arquivos que pinam o endereço mudam. Desbloqueia T-009.

### DEC-004: O vínculo do friendbot fica declarado, não resolvido (2026-10-07, T-005)

**Contexto.** §11-B. O dapp cria uma chave nova por cédula e o friendbot a
financia — e vê o IP de quem pediu junto do endereço que vai votar. Minutos
depois aquele endereço manda uma cédula.

**Decisão.** Aceitar e declarar. O vínculo é *pessoa ↔ cédula*, nunca
*pessoa ↔ escolha*: a escolha segue protegida pelo compromisso, que é
perfeitamente ocultante. E é fora do ledger — quem lê a cadeia não vê nada
disso.

**Alternativas.** (a) Exigir que a pessoa traga a própria conta — rejeitada:
identifica de forma permanente, que é pior. (b) Passar o pedido por um proxy —
rejeitada: exigiria o servidor que o projeto existe para não ter.

**Consequências.** É limite de testnet: em mainnet não há friendbot. Já dito na
tela de votar; §2 passa a listar como afirmação proibida dizer que não existe
vínculo nenhum fora do ledger.

### DEC-005: `TAU` no anel é regra do contrato, mas o aviso é do organizador (2026-10-07, T-010)

**Contexto.** §11-C. O contrato recusa anel abaixo de `TAU` quando há mais de
uma seção, e só avisa quando há uma só. O humano observou que **seções existem
para número grande de pessoas** — então seção pequena é sintoma de configuração
errada, não um caso de uso.

**Decisão.** A regra do contrato fica: ela é imposta sobre o **comparecimento
real**, que é a única verdade que ele tem. Mas o momento da checagem estava
errado — quem descobre hoje é o votante, depois de todo mundo já ter
comparecido. O organizador passa a ser avisado em `/abrir`, quando ainda dá para
consertar.

**Alternativas.** (a) Exigir `TAU` em toda votação em anel — rejeitada: uma
assembleia de três não votaria em sigilo. (b) Validar no contrato em `abrir()` —
rejeitada: ele só tem a raiz de Merkle, não o tamanho do eleitorado; um número
declarado daria falsa garantia contra organizador de má-fé, e contra erro o
aviso no cliente resolve igual.

**Consequências.** Nenhuma mudança de ABI. Abre T-010.

---

### DEC-006: Votação aberta e fechada, e a seção derivada do endereço (2026-10-07, T-013)

**Contexto.** O dapp é para ficar aberto, e quem chega não está em lista
nenhuma. `comparecer` exigia prova de Merkle contra a `raiz_aptos` fixada em
`abrir`, então qualquer visitante era recusado com `NaoEstaNaListaDeAptos`.

**Decisão.** `raiz_aptos` de 32 zeros passa a significar **votação aberta**:
`comparecer` pula a prova e o contrato atribui a seção por
`H(proposta ‖ endereço) mod secoes`.

**Alternativas.** Atribuir por **ordem de chegada** — tentada, implantada e
**medida como errada**. A seção nomeia a entrada `Anel(proposta, secao)` que a
transação escreve, e o footprint é declarado na *simulação*; se a seção só
existe na *aplicação*, cada transação declara uma entrada e escreve outra. Na
testnet: 1 comparecimento por ledger e 2 de 9 recusados com `txFailed`. Derivar
do endereço devolveu 18 de 18 em 3 ledgers, com 18 transações.

**Consequências.** Dá para moer endereços até cair numa seção escolhida; numa
votação aberta isso não tira nada de ninguém, porque escolher o próprio
esconderijo não encolhe o de outra pessoa, e o piso de `TAU` continua valendo.

E a divisão por hash **não equilibra**: 18 pessoas em 3 seções deram 9, 4, 5 —
a de 4 ficou abaixo de `TAU` e aquelas 4 pessoas não votaram (`#19`). Então
**votação aberta deve usar uma seção só**, a menos que se saiba que virá muita
gente: o anel passa a ser quem apareceu, que é o melhor anonimato possível, e o
custo só aperta acima de ~20. §2 ganha a proibição de afirmar contagem no modo
aberto.

---

## §11 Perguntas em aberto

§11-A, §11-B e §11-C foram respondidas em 2026-10-07 e viraram DEC-003, DEC-004
e DEC-005.

Nenhuma pergunta em aberto.
