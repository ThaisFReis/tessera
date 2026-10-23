# Fontes

Todo fato externo conferido e todo código copiado, com data. Formato definido em
[`CLAUDE.md`](../CLAUDE.md).

Medições **internas** — custo de CPU, taxas, vazão — não entram aqui: elas vivem
no teste que as produz, que é o que as mantém verdadeiras. Ver
`contrato/src/test.rs::ate_quantas_pessoas_cabe_um_anel`,
`::trinta_votantes_em_tres_secoes_pagam_o_preco_de_dez` e
`app/scripts/rodada-30.mjs`.

## Fatos

| Data | Fato | Valor | Versão | Fonte |
|------|------|-------|--------|-------|
| 2026-10-07 | Hash da cadeia quicknet da drand | `52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971` | — | https://api.drand.sh/v2/chains |
| 2026-10-07 | Esquema, gênese e período da quicknet | `bls-unchained-g1-rfc9380`, 1692803367, 3 s | — | https://api.drand.sh/v2/chains/52db9ba7…/info |
| 2026-10-07 | Chave pública da quicknet | 96 bytes, G2 comprimido (byte alto `0x83`) | — | idem, comprimento medido |
| 2026-10-10 | `sim.cost` no `@stellar/stellar-sdk` | **não existe** — a propriedade inteira vem `undefined`, e `cpuInsns` não aparece em nenhum arquivo do pacote instalado | 14.6.1 | medido: simulação de `gerador_h` na testnet imprimiu `sim.cost = undefined`; os recursos da mesma simulação deram 657.855 instruções |
| 2026-10-10 | `votar_anonimo` com fechadura, anel de 5, 320 B de criptograma | 100.684.411 instruções | testnet | `app/scripts/rodada-relogio.mjs`, tx das cédulas da proposta `d06750b2…` |
| 2026-10-10 | `apurar_secao` de 5 cédulas e 2 opções, 4 abertas | 13.709.833 instruções | testnet | idem, tx `dd030deb74c9e9b8…` — **11,5% acima** da fórmula medida no `Env` local (11.033.699 + 252.722/cédula = 12.297.309): a diferença é autorização, evento e estado, que o teste local não cobra |
| 2026-10-10 | Rodada ponta a ponta da fechadura, sem mesa | proposta `d06750b29f715aecb3cd636ebc48751e3af098203c1d4789b4bcb157163960bc`, rodada 32.935.517 | testnet | `abrir` `5d479a706c7be721…` · `apurar_secao` parcial `9149b9359e84e0c2…` · completa `dd030deb74c9e9b8…` |
| 2026-10-10 | `apurar_secao` **sem MSM** (T-026): 6 e 12 cédulas, 2 opções | 1.749.929 e 3.266.264 instruções — 1.233.597 fixos mais **252.722 por cédula** | `Env` local | `contrato/src/test.rs::orcamento_da_apuracao_por_secao`, impresso a cada `cargo test`. A inclinação por cédula não mudou (hashes e somas em G1); o que saiu foi o MSM, e a seção ficou **7,2× mais barata** que os 12.550.031 de antes |
| 2026-10-10 | `publicar_resultado` de 1 seção, 2 opções (T-026) | 11.359.274 instruções | `Env` local | idem. Não cresce com as cédulas: lê um ponto por seção nomeada, soma-os e faz um MSM de dois termos por opção. Uma seção de 6: 1.749.929 + 11.359.274 = **13.109.203** no total, contra 12.550.031 antes — mais caro numa seção, e a diferença some a partir de duas |
| 2026-10-10 | Formato JSON do RPC do Soroban para `getEvents` | `xdrFormat: "json"` devolve `topicJson`/`valueJson` já decodificados | RPC do protocolo 29 | medido contra `https://soroban-testnet.stellar.org`, proposta `d06750b2…` no ledger 5.117.233 — é o que permite ao verificador em Rust ler eventos sem crate de XDR |
| 2026-10-11 | Semeadura do vídeo (T-004): janela restante depois de o script parar | `LEDGERS_URNA=90` deixa **76 ledgers (~6 min)** de urna e a rodada vence em **439 s** — a semeadura consome 14 dos 90 | testnet | `SEMEAR=1 N=6 node app/scripts/rodada-relogio.mjs`, proposta `fb3e7f59…`, contrato `CDWGDWDP…`, `abrir` `6fddeacc000a8538…` |
| 2026-10-11 | A lista colada no dapp reproduz a raiz do ledger | `b8684a9e26580bfdc74237f388aedc75ad859c01d9282121693e423918f95ba6` nos dois lados | testnet | mesmo parser de `Comparecer.tsx` (`split(/[\n,]/)`) sobre a lista que o modo semeadura imprime, contra `proposta().raiz_aptos` |
| 2026-10-10 | Inclinação do anel, remedida no `Env` local | **10.846.474 por pessoa** — 94.532.799 com 5, 148.762.187 com 10, 365.694.651 com 30, 387.388.610 com 32 | `Env` local | `contrato/src/test.rs::ate_quantas_pessoas_cabe_um_anel`. Confirma os 10,85 M do deck. **Não é o mesmo número que os 148.911.943 do README e dos decks**, que são da rodada de 30 em 3 seções na testnet: ambiente diferente, e comparar os dois foi um erro meu neste mesmo dia |
| 2026-10-10 | Rajada de comparecimentos com `limite_secao`, antes e depois | 6 com limite 2: **2 → 6 de 6** (3 seções no mesmo ledger) · 20 com limite 10: **10 → 17 de 20** (2 seções) | testnet | `app/scripts/rajada-secao.mjs`. Os dois gargalos eram de cliente: `InsufficientRefundableFee` (reserva calculada para o estado de antes da rajada) e `ResourceLimitExceeded` (a 5ª entrada `Anel` declarada na janela estoura o teto de recursos da transação, com **uma** pessoa só) |
| 2026-10-10 | Teto de transações Soroban aceitas no mesmo ledger, nesta carga | 17 — **estável** com 2.048 e com 8.192 bytes de folga declarada | testnet | idem. As 3 preteridas levam `ResourceLimitExceeded` e não foram recusadas pelo contrato |
| 2026-10-10 | Escalares `Bls12381Fr` nos argumentos de uma transação | chegam como **`u256` decimal**, não como bytes | RPC do protocolo 29, `xdrFormat: "json"` | medido: `c0` e `z` de `votar_anonimo` na tx `07acf7e0…`. Lidos como bytes, o verificador recusava **todas** as assinaturas em anel dizendo que o anel não fechava |
| 2026-10-10 | O `hash_to_g1` do host e o RFC 9380 do arkworks | o **mesmo** ponto, para a mensagem `DST ‖ proposta` com DST `TESSERA-V1-ANEL` | soroban-sdk 28.0.0 · ark-ec | `contrato/src/test.rs::o_hp_do_host_bate_com_o_do_core`, três identificadores. É o que permite ao verificador **recalcular** `Hp` em vez de lê-lo do contrato |
| 2026-10-10 | Rodada ponta a ponta com a ABI de T-012/§11-N/T-026 | contrato `CDWGDWDPCXAAA3FO24NRRYS77HEMFHWDIBK4O3GB6L6BQGYWC7JYW3YS`, proposta `7021e473559e3ed9cc301c5abb72cc44a3b6a9c36e3d2246d0105e499b7793e8`, rodada 32.951.126 | testnet | `abrir` `f8ac416fd2fcadaa…` · `apurar_secao` parcial `71de2a3246171bb7…`, completa `a7092547ee9759c6…` · `publicar_resultado` `2b124b56c5da06b5…`. 7 cédulas, uma sabotada, placar 2 · 4, sem mesa |
| 2026-10-10 | `votar_anonimo` com fechadura, anel de **7**, 320 B de criptograma | 123.605.080 instruções — 30,9% do teto | testnet | idem, primeira cédula |
| 2026-10-10 | `apurar_secao` **sem MSM** na testnet: 6 de 7 cédulas abertas | 3.048.531 instruções — 0,76% do teto | testnet | idem, tx `a7092547ee9759c6…`. Contra 13.709.833 da mesma chamada antes de T-026: **4,5× mais barata**, porque o MSM saiu |
| 2026-10-10 | `publicar_resultado` de 1 seção na testnet | 12.702.177 instruções — 3,2% do teto | testnet | idem, tx `2b124b56c5da06b5…`. É a transação a mais que T-026 acrescentou, e é onde o número de voto entra no ledger |
| 2026-10-10 | Assinatura da rodada 6.000.100 da `quicknet` | `b6018631cdb80412e0690267164e381fc744e6a62ba3397c9becae5370100e87dcabde17a7325d5e7fb8d0d135cb664d` | v2 | https://api.drand.sh/v2/chains/52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971/rounds/6000100 — congelada em `contrato/src/test.rs::ASSINATURA_FIM` e em `app/scripts/fechadura.test.mjs`, para que o portão não dependa de rede |
| 2026-10-13 | `cast_vote` do Governor da OpenZeppelin para Stellar | `fn cast_vote(e: &Env, proposal_id: BytesN<32>, vote_type: u32, reason: String, voter: Address) -> u128` — o `vote_type` é **inteiro em claro** (contra 0, a favor 1, abstenção 2) e vai para o ledger | docs sem número de versão | https://docs.openzeppelin.com/stellar-contracts/governance/governor — é a razão de o eleitor falar com Tessera e **nunca** com `cast_vote`. Afirmado no slide de integração desde T-006 e só agora conferido na fonte |
| 2026-10-13 | Contagem plugável do mesmo Governor | `count_vote`, `tally_succeeded` e `quorum_reached` são sobrescrevíveis; `counting_mode()` identifica a estratégia ativa | idem | idem — é o encaixe previsto para Tessera |
| 2026-10-13 | Poder de voto do Governor | `cast_vote` devolve `u128` | idem | idem — e o peso de Tessera é `u32`: é o atrito nomeado no slide de entrega |
| 2026-10-13 | Verificação ao vivo da rodada do slide de evidência | contrato `CDWGDWDP…W3YS`, proposta `7021e473…`, rodada 32.951.126: cadeia 7 de 7, 7 imagens de chave distintas, 7 anéis refeitos, 6 de 7 abriram, placar do ledger 2 · 4 **igual** ao recontado | testnet, janela de RPC 5.058.376–5.179.335 | `cargo run -p tessera-cli -- verificar --proposta 7021e473… --contrato CDWGDWDP…`, rodado em 2026-10-13. Tudo o que o slide de evidência afirma foi reconferido do ledger neste dia |
| 2026-10-13 | Etiquetas de áudio do Eleven v4 | existem por categoria — emoção (`thoughtful`, `curious`, `proud`, `let down`…), entrega (`whispers`, `softly`…), ritmo (`pause`, `long pause`, `slowly`, `snappy`…), reação e efeito. **Recomendação oficial: uma etiqueta por oração.** A lista não é fechada: aceita descrição livre | Eleven v4 / v4 Turbo / v3 | https://elevenlabs.io/blog/elevenlabs-audio-tags-list — usadas em `docs/VIDEO-PITCH.md`. **A documentação não diz nada sobre línguas que não o inglês**, então o comportamento em português é não verificado |
| 2026-10-13 | Modo de geração e resposta às etiquetas | o modo **Robust** reduz a resposta às etiquetas; Creative e Natural respondem mais | Eleven v3 (alpha) | https://elevenlabs.io/docs/best-practices/prompting — documentado para o v3; **não confirmado para o v4** |
| 2026-10-07 | Mensagem assinada pela drand em modo *unchained* | `sha256(rodada em 8 bytes big-endian)` — a rodada crua **não** confere | quicknet | medido: pareamento contra a rodada 6.000.000, DST `BLS_SIG_BLS12381G1_XMD:SHA-256_SSWU_RO_NUL_` |
| 2026-10-07 | Rodada de um instante `t` | `(t − 1692803367)/3 + 1`, vencendo exatamente em `t` | quicknet | medido: `t=1791421407` → 32872681, relé publicando a 32872680 |
| 2026-10-07 | `soroban-sdk` expõe pareamento com saída de valor | não — só `pairing_check`, que devolve `bool` | 28.0.0 | fonte instalada, `src/crypto/bls12_381.rs` |
| 2026-10-07 | `ark-bls12-381` com a feature `curve` alcança pareamento e *hash-to-curve* em G1 | sim, sem dependência nova | 0.5 | medido no `core`, `e(xP,Q)==e(P,xQ)` |
| 2026-10-03 | Prazo de submissão do hackathon | 12 out 2026, 20:59 | — | https://demo.stellarpassport.xyz/hackathons/find-your-way-meridian-hackathon |
| 2026-10-03 | Trilhas e prêmios (General: 2.000/1.000/500 USDC) | 5.000 USDC no total | — | idem |
| 2026-10-03 | Critérios de avaliação da trilha General | execução técnica, uso significativo da Stellar, originalidade, impacto, experiência de uso, apresentação | — | idem |
| 2026-10-08 | Ordem de `Fp2` nos pontos G2 que o host do Soroban lê | **`c1` antes de `c0`** (convenção zcash): `be(x.c1)‖be(x.c0)‖be(y.c1)‖be(y.c0)`, 192 bytes não comprimidos | sdk 28.0.0 | medido: `contrato` `o_host_confere_a_baliza_e_a_ordem_de_g2_e_medida` — com a ordem trocada o pareamento falha |
| 2026-10-08 | `cost.cpuInsns` da simulação no SDK | **não existe mais** — nenhuma ocorrência em `@stellar/stellar-sdk` 14.6.1; a CPU vem de `transactionData.resources().instructions()` | 14.6.1 | busca na árvore instalada; ver T-024 |
| 2026-10-03 | `SorobanResources` renomeou `readBytes` | `diskReadBytes` | protocolo 23 | `@stellar/stellar-sdk` 14.6.1, introspecção de `SorobanDataBuilder().build().resources()` |
| 2026-10-03 | A rede cobra o mínimo necessário, não o lance oferecido | lance 100 e 1.000.000 → mesma cobrança de 19.690.096 stroops | — | medido na testnet, duas invocações de `abrir` |
| 2026-10-03 | Folga de escrita declarada é devolvida | 3.679.157 sem folga · 3.682.659 com | — | medido na testnet, `app/scripts/folga.mjs` |
| 2026-10-02 | Janela útil de eventos do RPC público para a lista de votações | 2.000 ledgers (17.000 devolve zero em silêncio) | — | medido: 100→0, 500→1, 2000→1, 17000→0 |
| 2026-10-02 | XDR de um `Address` como folha de Merkle | 44 bytes (`toScVal().toXDR()`); `toScAddress()` dá 40 e omite o discriminante | SDK 14 | congelado em `app/scripts/xdr.test.mjs` e `cli/src/chave.rs` |
| 2026-10-02 | `Bls12381Fr` no ABI do contrato | `U256`, não `BytesN<32>` | — | `stellar contract info interface` |
| — | Teto de CPU por transação na testnet | 400.000.000 | — | achado por bissecção, `bls-smoke/RESULTADOS.md` |
| — | Codificação do ponto no infinito em G1 | flag zcash `0x40` no byte alto, zeros no resto — **não** 96 bytes de zero | — | achado somando candidatos ao gerador; travado em `core/src/ponto.rs` |
| — | Retenção do RPC público e TTL padrão de entrada persistente | 120.960 e 120.959 ledgers — ambos 7 dias | — | sondas 8 e 9, `bls-smoke/RESULTADOS.md` |
| 2026-10-23 | dapp publicado no GitHub Pages | `https://thaisfreis.github.io/tessera/dapp/` responde HTTP 200, `<title>Tessera</title>` | — | https://thaisfreis.github.io/tessera/dapp/ |

## Versões fixadas

| Data | Ferramenta | Versão |
|------|------------|--------|
| 2026-10-03 | rustc | 1.97.1 (8bab26f4f 2026-07-14) |
| 2026-10-03 | stellar-cli | 25.2.0 |
| 2026-10-03 | node | 24.14.1 (os testes do app usam remoção de tipos nativa, requer 22+) |
| 2026-10-03 | pnpm | 10.33.0 |
| 2026-10-03 | @stellar/stellar-sdk | 14.6.1 |

## Código copiado

| Data | Nosso caminho | Origem (URL + commit) | Licença | Tarefa |
|------|---------------|-----------------------|---------|--------|
| — | — | nenhum até aqui | — | — |

A matemática vem de `arkworks` como **dependência**, não como cópia — de
propósito: é o mesmo crate que o host do Soroban usa, o que elimina pela raiz a
divergência entre o provador nativo e o verificador Wasm.
