# Tessera — especificação de trabalho

Formato definido em [`CLAUDE.md`](../CLAUDE.md). A especificação **do protocolo**
— modelo de ameaça, criptografia, orçamento, desenhos rejeitados — vive em
[`PROTOCOLO.md`](PROTOCOLO.md); as citações no código (`PROTOCOLO §6.4`) apontam
para lá.

---

## §0 Regras para agentes

**Approved:** _(pendente — o humano escreve `Approved: AAAA-MM-DD` aqui)_

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
| "quem faltou é público, de quem é cada cédula não é" | `contrato/src/test.rs::o_caderno_diz_quem_faltou_e_a_urna_nao_diz_de_quem`; interseção caderno ∩ urna = 0 em `app/scripts/rodada-anel.mjs` |
| custo em instruções de qualquer operação | o teste que o mede, citado junto do número |
| "roda na testnet" | hash da transação, verificável no explorer |
| "não há servidor que veja o fator de aleatoriedade" | o dapp é estático; as provas nascem no navegador |
| "ninguém pode abrir uma cédula em anel" | ela não reparte o fator com a mesa; `AberturaNaoFecha` recusa qualquer total afirmado |

**Proibido afirmar:** que o dapp está publicado (não está), que houve auditoria,
que há garantia contra coação, ou qualquer número que não venha de medição.

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

**Dados-ouro:** `bls-smoke/vetores.env` (vetores públicos),
`contrato/test_snapshots/` (orçamento), `app/scripts/xdr.test.mjs` (o vetor de
44 bytes, espelhado em `cli/src/chave.rs`).

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

Contrato na testnet: `CB6WIY45JYIR6EN6NC3WOAEOHYSKMXHKPDEXCHNY3O4C2O2RJ4RBQIJ6`

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
| `cargo fmt --check` | ❌ **falha nos 5 crates** — nunca foi rodado |
| `cargo clippy` | ⚠️ 65 avisos (core 11, contrato 43, cli 11) |

As duas últimas linhas são falhas de base: não bloqueiam tarefa, mas nenhuma
tarefa pode piorá-las. Arrumar virou T-002 e T-003 — não foram consertadas aqui
porque `CLAUDE.md` proíbe adicionar portão em silêncio.

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
| T-002 | `cargo fmt` passa | T-000 | §8 | todo | `cargo fmt --check` passa nos 5 crates; nenhum teste muda de resultado |
| T-003 | `cargo clippy` sem avisos | T-002 | §8 | todo | 0 avisos nos 3 crates; `-D warnings` no portão |
| T-004 | Vídeo da demonstração | T-001 | §2 | todo | roteiro + gravação com `/bastidores` na segunda janela; nada encenado |
| T-005 | Publicar o dapp | T-001 | §7 | todo | estático no ar; o README deixa de dizer "não publicado" |
| T-006 | Atualizar os decks | T-001 | §2 | todo | slide 06 deixa de listar desvinculação como futura; seções aparecem |
| T-007 | "O modo" em `/abrir` | T-001 | §4 | todo | a tela não oferece votação que `/votar` recusa |
| T-008 | `/apurar` junta as parcelas da mesa | T-005 | §4 | todo | apuração pelo dapp, sem CLI |
| T-009 | Assembleia sem mesa nenhuma | T-000 | §5 | blocked | §11-A |

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

---

## §11 Perguntas em aberto

### §11-A — Assembleia aberta sem mesa nenhuma (bloqueia T-009)

O contrato exige mesa: `limiar == 0 || limiar > mesa.len()` cai em
`LimiarInvalido`. Mas numa cédula em anel a mesa **não recebe parcela nenhuma** —
ela existe no estado e não serve para nada, e a tela precisa explicar isso.

O desenho limpo seria permitir `limiar == 0` se e somente se `mesa` for vazia.
É mudança de ABI, logo redeploy, logo invalida toda votação aberta e os 12
arquivos que pinam o endereço. **Vale o redeploy antes da submissão?**

### §11-B — O dapp aberto é "nada encenado" até onde? (afeta T-005)

A exigência foi: ambiente controlado para o vídeo, e um dapp aberto para a
comunidade testar, em que ninguém descubra o voto de outra pessoa. O anel
entrega isso. Mas o dapp aberto na testnet depende do friendbot, que **vê o IP**
de quem cria a chave efêmera — vínculo fora do ledger, já dito na tela.

Isso é aceitável para a submissão, ou o dapp aberto deve exigir que a pessoa
traga a própria conta financiada?

### §11-C — `TAU` por seção (afeta T-009 e o desenho de seções)

Hoje `votar_anonimo` recusa anel abaixo de `TAU` **só quando há mais de uma
seção**, com o argumento de que ali alguém escolheu o agrupamento. Sem seções o
contrato avisa e deixa passar.

É a regra certa, ou toda votação em anel deveria exigir `TAU`?
