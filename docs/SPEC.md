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

**O princípio que organiza tudo:** *não é possível ligar uma cédula a quem a
depositou.* Não é uma propriedade entre outras — é a decisão de onde todo o
desenho decorre, e ela veio antes do anel, antes das seções e antes de qualquer
escolha de curva. O caderno diz quem compareceu, a urna diz o que foi votado, e
**nada liga os dois**.

Disso vem o resto: o anel existe para que a cédula não carregue remetente; a
imagem de chave existe para impedir a segunda cédula sem revelar de quem é a
primeira; a chave de uso único existe para que nem o pagamento da taxa ligue as
duas pontas.

E disso vem também o que o projeto **aceita**: abrir uma cédula anônima diz o
que *aquela cédula* votou, nunca de quem ela é. Quem abre não ganha o vínculo,
porque o vínculo não existe em lugar nenhum para ser ganho.

Sucesso é: um módulo de votação para Soroban em que se sabe quem compareceu, não
se sabe em que cada pessoa votou, não se sabe de quem é cada cédula, e qualquer
pessoa recalcula o resultado do ledger. Mais o que a submissão exige — vídeo e
repositório público legível.

**E, desde 2026-10-07, também:** o resultado aparece **sozinho** quando a janela
fecha, sem mesa, sem servidor e sem ninguém designado; nenhum resultado, nem
parcial, existe antes disso; e **nenhuma pessoa consegue travar o placar** — quem
sabota perde o próprio voto e mais nada. Como isso é possível sem contradizer a
impossibilidade óbvia (quem retém a abertura impede o automático, e quem não
retém não dá sigilo) está em [`RELOGIO.md`](RELOGIO.md): o retentor deixa de ser
gente e passa a ser uma fechadura de tempo sobre uma baliza de limiar. Decisões
DEC-008, DEC-009 e DEC-010.

**Não-objetivos deste marco:**

- mainnet;
- ser um aplicativo de governança ou uma DAO (é um módulo que uma governança
  existente chama);
- resistência à coação **durante** o voto: quem olha a sua tela vê a sua
  escolha, e nenhum protocolo conserta isso. O *depois* está resolvido — ver §2;
- voto ponderado com pesos públicos distintos (o contrato **recusa**);
- **interoperar com as implementações publicadas do `tlock`.** O formato de
  criptograma é nosso, com DSTs nossos. Decifrar com ferramenta de terceiro
  seria bom e não é requisito — ver §11-G.

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
| "depois de votar, nem quem votou reabre a própria cédula" | `r` não é persistido em nenhum cliente; `cli` `nenhum_comando_grava_recibo` lê o próprio fonte, e `queimar_sobrescreve_antes_de_remover` |
| "abrir uma cédula anônima revela o que ela votou, nunca de quem é" | INV-04b; o vínculo não é guardado em lugar nenhum |
| "ninguém pode abrir uma cédula em anel **antes da rodada da fechadura**" | a chave de decifragem é a assinatura da baliza, que não existe antes do instante da rodada |
| "nenhum resultado, nem parcial, existe no contrato antes do fechamento" | INV-18 — o relógio do ledger, **sem suposição nenhuma** |
| "uma cédula que não abre não trava as outras" | INV-19, INV-21 |
| "omitir uma cédula honesta não gruda" | INV-21 — só um conjunto estritamente maior substitui o guardado, e qualquer pessoa consegue incluí-la |
| "a assinatura da baliza se autovalida: um relé que minta é recusado" | INV-24, vetor congelado da rodada 6.000.000 |
| "nem quem organiza escolhe quem se esconde atrás de quem" | INV-08 e DEC-012 — a seção vem da baliza, que não existe quando a votação é aberta |
| "o piso de `τ` protege o conjunto de anonimato, e abrir cédula por cédula não o afrouxa" | INV-12 e INV-25; `PROTOCOLO §6.6` — o piso é sobre o anel, não sobre quantas cédulas abriram |
| "não existe segredo durável que possa vazar depois" | INV-23 — a chave é publicada de propósito no instante da rodada; nenhum cliente guarda o `r` |

**Proibido afirmar na votação aberta:** que há voto obrigatório, que existe
lista de quem faltou, ou que o total significa alguma coisa — qualquer pessoa
vota quantas vezes quiser criando carteiras, e na testnet o friendbot as
financia de graça. O que a votação aberta demonstra é **sigilo**, não contagem.

**Proibido afirmar sobre as seções:** que a seção de uma pessoa é secreta, ou
difícil de descobrir. Não é: o anel é nomeado, as seções particionam, e quem lê
os anéis lê a seção de todo mundo (DEC-012). O que se pode afirmar é que ela é
**imprevisível até a abertura** — que é o que impede escolher quem se esconde
atrás de quem.

**Proibido afirmar sobre a fechadura de tempo:** que abrir o **conteúdo** antes
da hora é *impossível*. Não é: é conluio de um limiar dos operadores da baliza —
gente que não foi escolhida por quem abriu a urna e não tem interesse na
votação, mas gente. Incondicional é só o que diz a INV-18: o **resultado** não
sai antes do fechamento. Também proibido chamar a baliza de "sem confiança" ou
"trustless", e prometer resultado se a baliza parar: se ela parar, não sai
resultado, e isso não tem plano B (§7).

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
  ├ relogio    fechadura de tempo: cifra o fator para uma rodada futura da
  │            baliza, decifra com a assinatura dela, e **verifica** essa
  │            assinatura antes de usá-la
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
- **`core` não fala com a rede.** `core/relogio` recebe a assinatura da rodada
  como argumento e a valida; quem busca no relé é `app/src/rede.ts` ou a `cli`.
  Um crate de matemática que faz HTTP não é testável sem rede, e o portão tem de
  rodar offline.

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
| INV-02b | Nenhum cliente persiste o fator `r`: depois de votar, nem quem votou reabre a própria cédula | `cli` `nenhum_comando_grava_recibo`, `queimar_sobrescreve_antes_de_remover` |
| INV-03 | O diário nunca grava nome de segredo junto de um valor | `app/scripts/diario.test.mjs` |
| INV-04 | Uma chave efêmera assina **uma cédula**, nunca duas | `app/scripts/usoUnico.test.mjs` |
| INV-04b | **Nada liga uma cédula a quem a depositou** — nem para quem abre a cédula | o anel, a imagem de chave e a carteira de uso único; `contrato` `o_caderno_diz_quem_faltou_e_a_urna_nao_diz_de_quem`; `app/scripts/rodada-anel.mjs` afere interseção caderno ∩ urna = 0 |
| INV-05 | O caderno e a urna não se ligam | `contrato` `o_caderno_diz_quem_faltou_e_a_urna_nao_diz_de_quem` |
| INV-06 | A mesma pessoa não vota duas vezes, e a recusa não revela quem é | `contrato` `ninguem_vota_duas_vezes`; `core` `a_mesma_pessoa_produz_a_mesma_imagem` |
| INV-07 | A imagem de chave não atravessa propostas | `core` `a_imagem_nao_atravessa_propostas` |
| INV-08 | **Na votação fechada**, a seção é imprevisível até `abre_em`: nem o votante nem quem organiza escolhe a sua | `core` `a_secao_vem_da_baliza_e_nao_do_identificador`, `moer_o_identificador_nao_isola_ninguem`; `contrato` `trinta_votantes_em_tres_secoes_pagam_o_preco_de_dez` — DEC-012 |
| INV-25 | O piso de `τ` é conferido contra o **anel**, nunca contra o subconjunto que abriu — sabotar não trava | `contrato` `sabotar_o_proprio_criptograma_nao_derruba_o_piso` |
| INV-18 | Nenhum resultado, **nem parcial**, existe no contrato antes de `fecha_em` | `contrato` `antes_do_fechamento_nao_existe_placar` — relógio do ledger, sem suposição |
| INV-19 | Uma cédula que não abre perde o próprio voto e **não** impede a apuração das outras | `contrato` `a_cedula_que_nao_abre_perde_so_o_proprio_voto` |
| INV-20 | A lista de compromissos apresentada na apuração é exatamente o conjunto de cédulas da seção — omitir e inventar são recusados | `contrato` `a_cadeia_recusa_omissao_e_invencao` |
| INV-21 | Um conjunto incluído só é substituído por um **estritamente maior** | `contrato` `omitir_nao_gruda_e_quem_inclui_sobrepoe` |
| INV-22 | A rodada da fechadura é determinada por `fecha_em`, e o contrato recusa divergência | `contrato` `a_rodada_vem_do_fechamento_e_nao_da_vontade_de_quem_abre` |
| INV-23 | Não existe segredo durável: a chave que abre as cédulas é publicada pela baliza no instante da rodada, e nenhum cliente guarda o `r` | INV-02b mais o desenho; `core` `o_fator_nao_sai_do_processo` |
| INV-24 | A assinatura da baliza se autovalida: um relé que minta é recusado antes de qualquer decifragem | `core` `a_assinatura_da_baliza_confere` — vetor congelado da rodada 6.000.000 |
| INV-09 | A divisão em seções não depende da ordem da lista nem de quem organiza | `core` `a_divisao_nao_depende_da_ordem_em_que_a_lista_chega`, `a_divisao_e_equilibrada_e_ninguem_fica_sozinho` |
| INV-10 | Peso inflado não chega na raiz | `core` `peso_inflado_nao_chega_na_raiz`; `contrato` recusa com `PesoNaoUnitario` |
| INV-10b | Mesa vazia exige limiar zero, e limiar zero exige mesa vazia — meio-termo não existe | `contrato` `assembleia_sem_mesa_abre_e_nao_apura` |
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
| Folha de Merkle `H(0x00 ‖ endereço ‖ peso_be)` — **o `secao_be` sai em T-023** (DEC-012) | `core/src/merkle.rs::folha` | muda a raiz; quebra toda prova de aptidão existente |
| Separação de domínio `0x00` folha, `0x01` nó, `0x02` vazio, `0x03` seção | idem | sem ela, folha de 64 bytes vira nó interno |
| Serialização G1, inclusive o infinito | `core/src/ponto.rs` | o host recusa qualquer outra coisa |
| ABI do contrato (`abrir`, `comparecer`, `votar`, `votar_anonimo`, `apurar`) | `contrato/src/lib.rs` | redeploy invalida toda votação aberta e 12 arquivos que pinam o endereço |
| Códigos de erro `Erro` | `contrato/src/tipos.rs` | espelhados em `app/src/rede.ts::ERROS` |
| DSTs do anel (`TESSERA-V1-ANEL`, `-HP`, `-CONJUNTO`) | `core/src/anel.rs`, `contrato/src/cripto.rs` | divergir faz toda assinatura falhar sem dizer por quê |
| Criptograma da fechadura: `U` (96 B, G2 comprimido) ‖ `V` (32 B) ‖ `W` (32 B) = **160 B por opção confidencial**, concatenados em ordem de pergunta | `core/src/relogio.rs` | é o que o evento carrega; mudar o layout torna ilegível toda cédula já depositada |
| DSTs da fechadura (`TESSERA-V1-RELOGIO-SIGMA`, `-MASCARA`, `-PAD`) | idem | divergir faz a decifragem devolver lixo em silêncio |
| Constantes da baliza: cadeia, gênese, período, chave pública, e o DST `BLS_SIG_BLS12381G1_XMD:SHA-256_SSWU_RO_NUL_` | `core/src/relogio.rs` | a rodada e a verificação dependem delas; ver §6 |
| Cadeia de compromissos `Cadeia ← H(anterior ‖ compromissos)` | `contrato/src/lib.rs` | é o que prova que a lista da apuração é o conjunto real de cédulas |

**Bump de ABI em T-018** (DEC-010): `votar_anonimo` ganha o argumento do
criptograma e `apurar` ganha o caminho por seção com subconjunto. O evento
`anonimo` passa a carregar o criptograma. Isso **invalida toda votação aberta** e
exige repontar os arquivos que pinam o endereço do contrato — como em T-009 e
T-013, no mesmo redeploy.

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

**Baliza de limiar** (quicknet da drand; todos os valores medidos em 2026-10-07,
tabela em [`SOURCES.md`](SOURCES.md)):

| | |
|---|---|
| cadeia | `52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971` |
| esquema | `bls-unchained-g1-rfc9380` — assinatura em G1 (48 B), chave em G2 (96 B) |
| gênese · período | 1692803367 · 3 s |
| rodada de um instante `t` | `(t − 1692803367)/3 + 1`, vencendo exatamente em `t` |
| mensagem assinada | `sha256(rodada em 8 bytes big-endian)` — a rodada crua **não** confere |

Nenhuma dependência nova: `ark-bls12-381 0.5` com a feature `curve`, que já está
no `core`, entrega pareamento e *hash-to-curve* em G1 (medido).

Contrato na testnet: `CDJ3VMFKEZP3TN6KF3REUXTVW2R7AT5FMMLX3OKADDJOAV2V5D4F7OLC`
— **substituído no redeploy de T-018.**

`[VERIFY]` em aberto: nenhum.

---

## §7 Segurança e segredos

**Modelo de confiança.** O ledger é público e permanente, e é tratado como
hostil ao sigilo: nada que precise ficar secreto é publicado nele, nem cifrado.
Quem vota confia no próprio navegador. A mesa é confiável para **não publicar um
total falso** (o contrato recusa) mas **não** é confiável para guardar segredo —
daí o limiar `k`-de-`n`, e daí a cédula em anel não repartir nada com ela.

**A fechadura de tempo, e o que ela troca.** O retentor da abertura deixa de ser
a mesa e passa a ser o tempo: o fator vai cifrado para uma rodada futura da
baliza, e a chave é a assinatura daquela rodada, que a baliza publica no instante
dela. Três consequências, ditas inteiras:

- **O que fica incondicional.** O contrato recusa apuração antes de `fecha_em`
  (INV-18). Nenhum conluio publica placar parcial no contrato.
- **O que passa a ser suposição.** Um limiar dos operadores da baliza, em
  conluio, decifraria o **conteúdo** das cédulas antes da hora e contaria por
  fora. É suposição mais fraca que "confie na sua mesa" — operadores
  independentes, que quem abriu a urna não escolheu —, mas é suposição, e a §2
  proíbe chamá-la de impossibilidade. O **vínculo** continua intacto em qualquer
  cenário: não está guardado em lugar nenhum (INV-04b).
- **Vivacidade sem plano B.** Baliza parada, resultado nunca. E todo plano B que
  abre sem a baliza abre **antes da hora** — ou seja, destrói a fechadura. A
  escolha foi fechadura só; as composições 2-de-2 e 1-de-2 estão descartadas em
  DEC-008.

**Não existe segredo durável** (INV-23): a chave é publicada de propósito num
instante conhecido, então a vida do sigilo é, por construção, até o fim da
votação. É a diferença entre "cifrado para sempre, esperando que a chave nunca
escape" e "cifrado até as 20:59".

**Buscar a assinatura é transporte, não confiança:** ela se autovalida contra a
chave pública da cadeia (INV-24). Um relé que minta é recusado pelo cliente — ao
contrário de um servidor do Tessera, que teria de ser *confiado* porque veria o
fator.

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
cd app && node scripts/rodada-relogio.mjs           # fechadura: cédulas, fim
                                                    # da janela, apuração
                                                    # automática, e uma cédula
                                                    # sabotada que não trava
```

A `rodada-relogio.mjs` é o portão de aceitação de T-021 e **precisa de rede**
(relé da baliza e testnet). Os portões por tarefa continuam rodando offline.

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
| T-016 | O placar aparece no dapp quando existe | T-013 | §2 | review | `/apurar` e `/votacao` leem `resultado()`; sem mesa, explicam por que nunca haverá |
| T-017 | `core/relogio`: a fechadura de tempo | T-016 | §5, §6 | todo | cifra e decifra o fator para uma rodada; INV-24 com o vetor congelado da rodada 6.000.000; recusa assinatura que não confere **antes** de decifrar; roda offline; `core` não faz rede |
| T-018 | Contrato: subconjunto, cadeia e regra monotônica | T-017 | §4, §5, §7 | todo | criptograma no evento `anonimo`; `Cadeia(proposta, secao)` de 32 bytes que não cresce; `apurar` por seção com lista ordenada + bitmap, conferindo a cadeia e o MSM de dois termos; INV-18 a INV-22 com teste cada; rodada presa a `fecha_em` (INV-22); bump de ABI e redeploy com os arquivos repontados; custo medido e citado junto do número; `τ` conferido contra o anel da seção e **não** contra o subconjunto aberto (INV-25, DEC-011) |
| T-019 | Decifrar no navegador e buscar a rodada | T-017, T-018 | §3, §5 | todo | `cliente-wasm` exporta decifrar; `app/src/rede.ts` busca a assinatura e a **valida** antes de usar; nenhuma página importa o SDK nem o relé direto |
| T-020 | O placar aparece sozinho quando a janela fecha | T-019 | §2, §4 | todo | antes de `fecha_em` a tela não mostra nada, nem parcial; depois, apura e publica sem ninguém clicar; diz quantas de quantas cédulas abriram; o diário conta o que aconteceu |
| T-021 | Rodada ponta a ponta na testnet | T-020 | §8 | todo | `app/scripts/rodada-relogio.mjs`: cédulas, fim da janela, apuração automática, e **uma cédula sabotada que não trava o placar**; hash das transações no PR |
| T-023 | A seção vem da baliza, não do identificador | T-017 | §4, §5, §10 | todo | `dividir` troca `proposta` por assinatura da rodada de `abre_em`; a folha perde `secao_be`; teste de que moer o identificador não isola ninguém; dados-ouro regerados; a tela da votação aberta diz por que ela usa uma seção só (DEC-012) |
| T-022 | Alinhar README, UX e decks à fechadura | T-021 | §2 | todo | o que §2 passa a permitir e o que passa a proibir aparece nos três; a suposição da baliza e a falta de plano B ficam escritas, não implícitas |
| T-008 | ~~`/apurar` junta as parcelas da mesa~~ | — | §11 | substituída | §11-E respondida por DEC-008: o retentor deixa de ser a mesa. Ver T-017 a T-022 |
| T-009 | Assembleia sem mesa nenhuma | T-000 | §5, §10 | review | `limiar == 0` aceito sse `mesa` vazia; redeploy; os 12 arquivos repontados; `/abrir` e `/apurar` param de exigir mesa |
| T-014 | Limite por seção, com split automático na aberta | T-013 | §4, §5 | blocked | §11-D — contrato e testes prontos; a rajada ainda não entra | `abrir` troca `secoes` por `limite_secao`; a aberta enche e abre a próxima; o cliente declara uma janela de anéis no footprint; rajada de 20 com limite 10 entra |
| T-015 | O texto da coação descreve o código, não a v1 | T-001 | §2 | review | README e UX param de afirmar que quem vota consegue provar o voto depois; o que sobra de ameaça fica escrito |
| T-010 | `/abrir` avisa quando a seção nasce pequena | T-013 | §10 | todo | recusa abrir com `aptos / secoes < TAU`, dizendo o tamanho que daria; teste do cálculo |
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

### DEC-007: O vínculo é o que se protege, não o conteúdo da cédula (2026-10-07, T-016)

**Contexto.** Ao discutir se a mesa pode existir numa votação em anel, a
pergunta voltou ao princípio: o que exatamente o projeto promete não revelar?

**Decisão.** Registrar explicitamente o que já era a decisão fundadora: o que
não pode existir é o **vínculo** entre uma cédula e quem a depositou. Abrir uma
cédula anônima revela o que *aquela cédula* votou e nada mais — quem abre não
ganha o vínculo, porque ele não está guardado em lugar nenhum.

Isso resolve a aparente tensão com a exigência original ("ninguém pode descobrir
o voto de outra pessoa"): ela continua de pé, porque "o voto *de outra pessoa*"
pressupõe saber de quem é a cédula.

**Alternativas.** Tratar o conteúdo de cada cédula como igualmente secreto —
rejeitada: levaria a recusar qualquer apuração, já que um total é agregação de
conteúdos, e a §11-E mostra que isso inviabiliza o placar.

**Consequências.** §1 ganhou o princípio no topo, §2 ganhou a afirmação com a
evidência, §4 ganhou INV-04b. E as referências a um sistema eleitoral específico
saíram de 16 lugares: o princípio vale por si, não por analogia.

---

### DEC-008: O retentor da abertura é uma fechadura de tempo, não gente (2026-10-07, T-017)

**Contexto.** A §11-E travou o placar do dapp: publicar um total exige
apresentar `R = Σrᵢ`, quem sabe `R` é a mesa, e o transporte das parcelas até ela
não existe sem servidor. E havia uma impossibilidade por cima: para o placar não
existir durante a votação alguém tem de estar retendo a abertura, e para ele
surgir sozinho ninguém pode estar retendo.

**Decisão.** O retentor passa a ser o tempo. Cada fator vai cifrado para uma
rodada futura da baliza de limiar `quicknet` da drand; a chave é a assinatura
daquela rodada, que a baliza publica no instante dela, para todo mundo ao mesmo
tempo. Sem mesa, sem Shamir — o limiar já está dentro da baliza —, sem servidor
e sem ninguém designado: passado o fechamento, qualquer pessoa apura.

O contrato **não decifra** e não precisa: o host do Soroban só expõe
`pairing_check`, sem pareamento com saída de valor (sdk 28.0.0, conferido na
fonte), e o `apurar` já recusa um total que mente. A fechadura só tem de tornar a
abertura indisponível antes de um instante e disponível depois.

**Alternativas.** Mesa, com as parcelas cifradas no ledger — a objeção da §11-E
contra isso vale para a mesa e **não** vale aqui: com mesa a chave é segredo
humano de vida indefinida, que vaza, é intimada ou é arrancada, para sempre; com
fechadura não existe segredo durável (INV-23). Composição 2-de-2 (metade na
fechadura, metade na mesa) e 1-de-2: descartadas — a primeira piora a vivacidade
sem melhorar o que importa, a segunda deixa a mesa abrir quando quiser, e não
existe "fechadura com plano B". VDF, que não precisaria de operador nenhum:
verificar exige grupo RSA ou de classe, que o host não tem.

**Consequências.** §1 ganhou o objetivo; §2 ganhou cinco afirmações e a proibição
de chamar o sigilo do **conteúdo** de impossível; §3 ganhou `core/relogio` e a
fronteira "o `core` não fala com a rede"; §6 ganhou as constantes da baliza; §7
ganhou a troca inteira, inclusive a falta de plano B; §4 ganhou INV-18, INV-22,
INV-23 e INV-24. A §11-E está respondida.

### DEC-009: A apuração deixa de ser tudo-ou-nada (2026-10-07, T-018)

**Contexto.** A exigência "não pode ser possível travar o placar" expôs um
defeito que não é da fechadura: o contrato guarda, por opção confidencial, só o
acumulado `A_j = Σᵢ C_{i,j}`, e `A_j == T_j·G + R_j·H` só fecha com **todas** as
cédulas abertas. Uma que não abra derruba o resultado inteiro — hoje, com mesa,
sem fechadura nenhuma. Quem trava o placar é o `Acum`.

**Decisão.** A apuração passa a aceitar subconjunto. Cada cédula atualiza
`Cadeia(proposta, secao) ← H(anterior ‖ compromissos)` — 32 bytes que **não**
crescem, ao contrário do anel, que cresce 96 B por pessoa e produziu a §11-D.
Quem apura apresenta a lista ordenada de todos os compromissos da seção, um
bitmap de quais abriram, `T_j` e `R_j` sobre as abertas; o contrato re-encadeia a
lista (o que recusa omissão e invenção, INV-20), soma os marcados e confere o
mesmo MSM de dois termos de hoje. E guarda o resultado com **mais** cédulas
incluídas, aceitando substituição só por um conjunto estritamente maior
(INV-21) — quem omitir é sobreposto por qualquer pessoa que inclua, e qualquer
pessoa consegue, porque a chave da rodada é pública. Basta **um** observador
honesto, não que os votantes voltem.

Solidez: a prova CDS já garante `v ∈ {0,1}` em cada compromisso no momento do
voto, então o compromisso é vinculante e a equação sobre o subconjunto força
`T_j = Σvᵢ` e `R_j = Σrᵢ`. Ninguém fabrica um total. Sabotar passa a custar o
próprio voto e nada mais (INV-19).

**Alternativas.** Provar, no voto, que o criptograma cifra o mesmo `r` do
compromisso: tornaria a exclusão impossível em vez de inócua, e é circuito sobre
XOR e hash — fora de alcance. Verificar cada cédula com uma multiplicação em G1:
correto e caro; a soma dos compromissos marcados mais um MSM de dois termos dá o
mesmo com somas em G1, muito mais baratas.

**Consequências.** §4 ganhou INV-19, INV-20 e INV-21; §5 ganhou a cadeia como
formato congelado e o bump de ABI; o material já era público — o evento `anonimo`
publica `(imagem, compromissos, escolhas)`, os compromissos de cada cédula um por
um. E há uma consequência que **não** decidi: o `τ` perde a razão original, ver
§11-H.

### DEC-010: Bump de ABI e redeploy, com o que isso invalida (2026-10-07, T-018)

**Contexto.** `votar_anonimo` precisa carregar o criptograma e `apurar` precisa
do caminho por seção. A ABI é interface congelada (§5).

**Decisão.** Bump e redeploy, no modelo de T-009 e T-013: os arquivos que pinam o
endereço do contrato são repontados no mesmo commit, e as votações abertas no
endereço antigo morrem. O evento `anonimo` ganha um campo — o leitor em
`app/src/rede.ts` muda junto, no mesmo commit, porque não mudar apaga a lista de
votações do dapp (a lição de T-012).

**Alternativas.** Contrato novo em paralelo, mantendo o antigo: dois endereços
para explicar num vídeo de hackathon, sem ganho.

**Consequências.** §5 e §6 atualizados; o endereço da testnet em §6 é substituído
no PR de T-018.

### DEC-011: O `τ` fica, e o piso é sobre o anel (2026-10-07, T-018)

**Contexto.** Eu levantei na §11-H que a apuração por cédula de DEC-009 tiraria a
razão original do `τ`: ele existia para que um total **agregado** não
determinasse o voto de ninguém, e agora cada cédula é aberta de propósito.
Resposta do humano: o `τ` protege.

Protege, e eu tinha superestimado o problema. Lendo o `PROTOCOLO §6.6` inteiro:
o teorema da partição já cobre o caso, porque abrir cédula por cédula é só mais
informação pública — e ela **não encolhe o conjunto de anonimato**. Com 10 no
anel e 2 cédulas abertas, cada uma das duas continua podendo ser de qualquer um
dos 10. O que protege uma pessoa é o tamanho do anel, que é onde o piso já é
imposto quando há seções.

**Decisão.** O `τ` fica, enunciado como piso do **conjunto de anonimato**. No
caminho novo de apuração ele é conferido contra o tamanho do anel da seção, e
**nunca** contra o subconjunto que abriu. O portão `conf > 0 && conf < TAU`
continua valendo para quem apura com mesa. A INV-12 fica como está — "abaixo de
`τ` a apuração trava em vez de vazar" continua verdadeiro — e entra a INV-25 para
prender o ponto de conferência.

**Alternativas.** Conferir `τ` contra as cédulas **abertas**: parece a leitura
natural e é um buraco. Uma coligação sabotaria o próprio criptograma, as abertas
cairiam abaixo de `τ` e a apuração travaria — a falha de liveness induzível de
fora que a remoção de `votar_publico` havia fechado, de volta por outro caminho,
contra a exigência explícita de que ninguém consiga travar o placar. Tirar o `τ`:
recusada pelo humano, e sem motivo.

**Consequências.** `PROTOCOLO §6.6` ganhou a subseção que diz por que o piso não
muda de lugar; §2 ganhou a afirmação; §4 ganhou INV-25; T-018 ganhou o critério
de aceitação. O caso de **uma seção só** com anel abaixo de `τ` continua como
DEC-005 decidiu — aviso do organizador, não recusa —, e o aviso é T-010.

### DEC-012: A seção de uma pessoa não dá para esconder — dá para tornar imprevisível (2026-10-07, T-023)

**Contexto.** Exigência nova: *"o votante não pode saber sua seção; as seções não
são públicas ou pelo menos difíceis de saber."*

**O que é impossível, e por quê.** Esconder em que seção uma pessoa está, neste
desenho, não dá. A prova é curta: uma assinatura de anel só verifica contra um
anel **nomeado**, então o anel de cada cédula é necessariamente público; as
seções particionam o eleitorado, cada pessoa aparece em exatamente um anel; logo
quem lê os anéis lê a seção de todo mundo. Hoje é ainda mais direto: o contrato
guarda a seção em `Compareceu(proposta, endereço)`, `secao_de()` devolve, e o
evento `comparec` publica.

Anéis **sobrepostos**, com iscas, não resgatam: o comparecimento é público por
pessoa — e é afirmação de §2 que seja —, então os candidatos a uma cédula são
`anel ∩ quem compareceu`, e as iscas caem fora. Esconder a seção exigiria
esconder o comparecimento, que é o oposto do que o projeto promete.

**O ataque que importa não é saber, é escolher — e aqui eu errei.** DEC-006
disse que moer endereços até cair numa seção escolhida *"não tira nada de
ninguém, porque escolher o próprio esconderijo não encolhe o de outra pessoa."*
Está errado. Moer **para dentro** da seção da vítima não encolhe a seção, e
enche de atacantes: cada um conhece a própria imagem de chave, elimina a própria
cédula, e o que sobra é a da vítima. O conjunto de anonimato efetivo vira 1. O
mesmo vale para quem organiza numa votação fechada, que escolhe o identificador
da proposta — e é dele que `dividir` deriva a seção
(`H(0x03 ‖ proposta ‖ endereço)`), então moer o identificador isola quem quiser.

**Decisão.** A seção passa a ser **imprevisível até `abre_em`**: derivada da
assinatura da baliza da rodada de abertura, não do identificador da proposta.
Quem organiza não consegue moer, porque na hora de `abrir` a assinatura não
existe. Depois de `abre_em` ela é determinística e pública — e tem de ser, pelo
parágrafo da impossibilidade acima.

Isso sobrevive ao muro que matou a atribuição por ordem de chegada (DEC-006):
passado `abre_em` a seção é função de dado público, então o cliente a calcula na
**simulação** e o footprint nomeia a entrada certa. O que falhou antes falhava
por depender de estado só conhecido na aplicação.

**Consequência congelada:** a folha de Merkle perde o `secao_be`
(`H(0x00 ‖ endereço ‖ peso_be)`), porque a seção deixa de existir no momento em
que a folha é construída. É mudança de formato congelado (§5): bump, dados-ouro
regerados, e a INV-08 reescrita — a seção deixa de estar presa na folha e passa
a estar presa na baliza, que é uma amarra mais forte, não mais fraca.

**Residual declarado.** Numa votação **aberta**, quem quiser ainda cria
endereços *depois* de `abre_em` e mói para dentro de uma seção. Contra isso não
há derivação que ajude: a defesa é a de DEC-006 — votação aberta usa **uma seção
só**, e aí não há seção para onde moer — ou a divisão por ordem de chegada de
T-014, que não deriva do endereço (§11-D).

**O que eu não faço.** Remover `secao_de()` e o campo do evento como se fosse
conserto: a seção é derivável da assinatura e do endereço, então esconder a
consulta é segurança cosmética, e §2 proíbe afirmar o que o código não sustenta.
Se o campo sair, sai por economia, não por sigilo.

**Alternativas.** Anéis sobrepostos com iscas: não funciona com comparecimento
público (acima). Seção sorteada pelo PRNG do host no `comparecer`: é estado da
aplicação, e cai no mesmo muro de footprint medido em DEC-006.

---

## §11 Perguntas em aberto

§11-A, §11-B e §11-C foram respondidas em 2026-10-07 e viraram DEC-003, DEC-004
e DEC-005.

### §11-D — a rajada não abre seção nova (bloqueia T-014)

O split automático funciona **sequencialmente**: com limite 2, a terceira pessoa
abre a seção 1, declarando no footprint a janela `Anel(proposta, 0..3)`.
Verificado na testnet.

Sob rajada, não. Vinte pessoas de uma vez com limite 10 dão **exatamente 10
aceitas** — as que cabem na seção 0 — e 10 recusadas com `txFailed`. As que
precisariam abrir a seção 1 são justamente as que falham.

O que já foi descartado:

- **não é a janela do footprint**: o caso sequencial usa a mesma janela e passa;
- **não é orçamento de escrita**: a folga cobre 50 pontos e a seção tem 10;
- **não é orçamento de leitura**: subir `diskReadBytes` junto não mudou o número
  (10 de 20 nas duas vezes).

O diagnóstico que falta é pequeno e eu não consegui rodar — o DNS do friendbot
caiu (`EAI_AGAIN`) no meio: **seis simultâneos com limite 2**, que força três
seções a nascer ao mesmo tempo. Se as seis entrarem, o problema é de escala e
não de mecanismo; se repetir o padrão "só a primeira seção", o mecanismo tem um
buraco que o caso sequencial não mostra.

### §11-E — o dapp pode *mostrar* o placar, mas não *produzir* — **respondida** (2026-10-07)

As três saídas listadas abaixo eram as três que eu via. Existia uma quarta, e é
DEC-008: um retentor que não é gente. T-008 fica **substituída** por T-017 a
T-022. O registro original segue.

#### registro original

Publicar um total exige apresentar `R = Σrᵢ`, e o contrato confere contra o
acumulado. Quem sabe `R` é quem recebeu parcelas de Shamir — a mesa.

No dapp, `cedula_anonima` chama `montar(..., 0, 0)`: **zero membros, nenhuma
parcela**. Não por escolha de desenho, por falta de **transporte**: as parcelas
precisam chegar aos membros da mesa, e o dapp não tem servidor.

As três saídas, e por que nenhuma serve:

1. **Parcelas cifradas no ledger.** Contradiz a tese do projeto frontalmente:
   registro permanente mais cifra é "legível quando a chave vazar", que é
   exatamente o que o slide 03 ataca e o que motivou escolher Pedersen.
2. **Um servidor.** Contradiz "não existe servidor que pudesse ver o `r`,
   porque não existe servidor", que é afirmação de §2.
3. **Entrega fora de banda.** Quem vota baixa a parcela e a leva à mesa. É
   honesto e não é fluxo de dapp.

Então T-016 fez o que é possível: o dapp **lê** `resultado()` e mostra o placar
assim que o contrato o aceitar. Quem apura é a CLI.

**O que falta decidir:** para o vídeo mostrar um placar saindo, a votação
precisa ter mesa e as cédulas precisam repartir. Hoje nenhum caminho do dapp faz
isso. Vale mudar `cedula_anonima` para repartir — aceitando que `k` membros em
conluio abrem **uma cédula anônima**, sem saber de quem é — ou o placar do vídeo
sai da CLI?

**Resposta parcial, 2026-10-07:** existe uma quarta saída que as três não
cobriam — um retentor que não é gente. Ver §11-F e `docs/RELOGIO.md`.

Enquanto isso, a votação aberta funciona com **uma seção só**
(`limite_secao = 0`), que é o padrão recomendado em DEC-006 e o que a tela
sugere.

### §11-F — o retentor não-humano — **respondida** (2026-10-07)

Virou DEC-008, DEC-009 e DEC-010, com escopo completo: T-017 a T-022. O estudo
que a originou, com o que foi medido e o que não foi, fica em
[`RELOGIO.md`](RELOGIO.md). O registro original da pergunta segue abaixo.

#### registro original

O estudo está em [`docs/RELOGIO.md`](RELOGIO.md), com o que foi medido e o que
não foi. Resumo: a fechadura de tempo sobre a drand resolve a §11-E — abertura
indisponível antes de um instante, disponível depois **para qualquer pessoa**,
sem mesa, sem Shamir e sem servidor. O contrato não decifra (o host só tem
`pairing_check`) e **não precisa**: o `apurar` já recusa um total que mente.

**Decidido pelo humano em 2026-10-07:** abrir cedo não pode, o resultado só
aparece depois de `fecha_em`, e travar o placar não pode. Isso responde os itens
2 e parte do 1 abaixo, e força a mudança de protocolo da §8 do estudo — a
apuração deixa de ser tudo-ou-nada, porque quem trava hoje é o `Acum`, não a
fechadura. Falta decidir escopo e afirmação.

Quatro coisas eu não decido:

1. **Confiança.** Abrir cedo exige conluio de um limiar dos operadores da
   drand. É mais fraco que confiar na mesa, e não é "ninguém". Aceita?
2. **Vivacidade.** Drand parada = sem resultado, para sempre. E todo plano B que
   abre sem a drand abre *antes da hora*. Fechadura só, 2 de 2, ou 1 de 2?
3. **Afirmação.** O que o deck pode dizer, e a partir de qual fatia.
4. **Escopo.** Fatia A (mecanismo no `core` e na CLI, ~1 dia) só, ou A e B
   (dapp apura sozinho, 2–3 dias, com o buraco de §5.1 do estudo: um saboteador
   trava o placar até o protocolo guardar o compromisso de cada cédula)?

Minha recomendação está em §6 do estudo: A agora, B só se A sair rápido, e o
deck afirmando a peça medida em vez de prometer o placar automático.

### §11-H — o `τ` perde a razão original — **respondida** (2026-10-07)

Resposta do humano: *"o τ protege"*. Virou DEC-011, e ela mudou um critério de
aceitação de T-018 — não era só redação.

### §11-G — interoperar com o `tlock` publicado? (não bloqueia)

O formato de criptograma de §5 é nosso, com DSTs nossos, e isso basta para o
desenho: quem decifra é o nosso cliente, e a assinatura da baliza se autovalida
(INV-24). Casar byte a byte com o formato das implementações publicadas do
`tlock` deixaria qualquer ferramenta de terceiro abrir as cédulas depois do
fechamento, o que fortalece "qualquer pessoa apura" — e exige conferir os hashes
delas contra a fonte, trabalho que não medi. Vale depois da submissão?

### §11-H — o `τ` perde a razão original (não bloqueia o código)

O `τ` existia para que um total agregado não determinasse o voto de uma pessoa
quando pouca gente votou em sigilo (INV-12, `PROTOCOLO §6.6`). Com a apuração por
cédula de DEC-009, **cada cédula é aberta individualmente de propósito** — o
agregado deixa de ser o que esconde, e o que protege passa a ser o tamanho do
anel, não o piso de cédulas confidenciais.

O que fiz: **mantive o portão** do `τ` como está, porque mantê-lo não custa nada
e só recusa votações minúsculas. O que não fiz, por ser enfraquecer uma proteção
declarada: reescrever o enunciado da INV-12 e o texto do `PROTOCOLO §6.6`. A
pergunta é se o `τ` passa a ser enunciado como piso do **conjunto de anonimato**
— que é o que ele de fato protege agora — ou se sai.
