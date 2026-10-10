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
| 2026-10-10 | Assinatura da rodada 6.000.100 da `quicknet` | `b6018631cdb80412e0690267164e381fc744e6a62ba3397c9becae5370100e87dcabde17a7325d5e7fb8d0d135cb664d` | v2 | https://api.drand.sh/v2/chains/52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971/rounds/6000100 — congelada em `contrato/src/test.rs::ASSINATURA_FIM` e em `app/scripts/fechadura.test.mjs`, para que o portão não dependa de rede |
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
