# Tessera

**English** · [Português](README.pt.md)

**Secret ballot for any governance, on Stellar.**
Find Your Way hackathon (Meridian) · General track

Tessera is a voting module for Soroban contracts. It is not a governance app or
a DAO: it is a contract that any existing governance can call to run a vote in
which

- **who turned out is known**, and that is public and auditable — it is what
  makes compulsory voting possible, because `eligible − turned out` is the list
  of who stayed away;
- **how each person voted is not known**, and the published commitment is
  perfectly hiding: mathematically empty of information, against any amount of
  computing power, forever;
- **whose ballot is whose is not known**, because the ballot comes from a
  one-time key with a ring signature — and nothing on the ledger links the two;
- **anyone can recompute the result** from the ledger and catch a tallying
  committee that lies.

The idea that organizes the design: on a permanent record, "encrypted today"
means "readable once the key leaks". So the ledger **never receives a ciphertext
of the vote**. It receives a Pedersen commitment, `C = v·G + r·H`. The vote
itself travels off-chain and is destroyed.

With the **time lock** there is one exception, and it is deliberate: the event
starts carrying the factor `r` encrypted to a future round of a public beacon.
The objection above still stands and does not apply, because there **is no
durable secret** there: the key is published on purpose at a known instant. The
slide we attack says "encrypted forever, hoping the key never escapes"; this is
**encrypted until 20:59**. They are different things, and the second is the
answer to the first.

**One principle organizes everything: a ballot cannot be linked to the person
who cast it.** It is not one property among others — the whole design follows
from it. The ring exists so the ballot carries no sender; the key image, to
block a second ballot without revealing whose the first one was; the one-time
key, so that not even the fee payment links the two ends.

And that is why opening a ballot tells you what *that ballot* voted and nothing
more. Whoever opens it does not gain the link, because the link is not stored
anywhere to be gained.

The second idea is the principle of the secret ballot, which solves, with no
cryptography at all, the problem of the ballot's sender being the voter's
identity: **the roll says who turned out, the ballot box says what was voted,
and nothing links the two.** Tessera does this in two acts — an identified
`comparecer()` ("turn out"), and `votar_anonimo()` ("vote anonymously") from an
ephemeral key, with a key image that blocks a second ballot from the same person
without revealing who they are.

> Contract functions, CLI commands and error codes keep their Portuguese names;
> the first time one appears here, its meaning follows in parentheses.

---

## Status, without makeup

| | |
|---|---|
| Cryptographic probes measured on testnet | ✅ 13 probes |
| Ballot-box contract (`abrir`/`votar`/`apurar` — open/vote/tally) | ✅ live, 60 tests |
| Roll and ballot box split (`comparecer`/`votar_anonimo`) | ✅ live, 30-voter round on testnet |
| Sections (one ring per section, single result) | ✅ live, 30 voters in 3 sections |
| CLI client | ✅ full round on testnet |
| Public verifier, committee path | ✅ `tessera verificar` (verify) |
| Public verifier, **anonymous** path | ✅ `tessera verificar --contrato` · ring, chain, key image and decryption, no local file |
| dapp (React, crypto in the browser) | ✅ live at [thaisfreis.github.io/tessera/dapp](https://thaisfreis.github.io/tessera/dapp/) |
| Time lock (tally **without a committee**) | ✅ end-to-end round on testnet |
| Tally from the dapp | ✅ with time lock, unaided, in any number of sections · ⬜ with committee, CLI only |
| Demo console | ✅ viewer over the CLI output |

Every number below came from a real invocation on testnet or from a test that
runs on the Soroban host, never from a projection. What is a projection is
marked as one.

---

## The measured numbers

CPU ceiling per transaction, found by bisection on testnet: **400,000,000**.

| operation | instructions | fee |
|---|---|---|
| `g1_add` | 110,748 | |
| `g1_mul` | 3,290,524 | |
| `g1_msm` | 2,454,719 + 1,471,917 per term | |
| `hash_to_g1` | 2,653,011 | |
| `pairing_check(k)` | 10,571,128 + 6,746,479 per pair | |
| Schnorr verification | 6,737,614 | 9,558 stroops |
| **CDS disjunctive proof** | **10,980,243** | **14,144 stroops** |
| **Pedersen tally** | **5,408,931** | **8,622 stroops** |
| **Merkle path (depth 8)** | **127,863** | **5,235 stroops** |
| **full `votar()`, m=2** | **36,812,994** | |
| **`apurar()`, m=2** | **11,197,414** | |
| **`apurar_secao()`** (no committee, per section) | **1,233,597 + 252,722 per ballot** | |
| **`publicar_resultado()`** (the scoreboard, once) | **11,359,274** | |
| ElGamal tally (previous design) | 6,712,183 | 11,852 stroops |
| Groth16 verification (4 public inputs) | 47,371,348 | |

Three conclusions that changed the design:

1. **`g1_add` is ~30× cheaper than `g1_mul`.** Homomorphic aggregation is
   practically free; the whole cost is in proof verification.
2. **Don't make the chain search for the result, make it check one.** Brute-force
   discrete log costs ~124k per unit of weight, which gives a ceiling of ~3,218
   weight per transaction. Checking a claimed opening costs **5,408,931,
   constant in turnout** — measured identical for 250 and for 1,000,000. And the
   whole `apurar()` costs 11,197,414 with five voters and 11,371,398 with forty:
   1.5% for eight times as many people, which is what "does not grow with
   turnout" means in practice.
3. **The design with permanent secrecy is also the cheapest.** Pedersen is 19%
   cheaper in CPU and 27% cheaper in fees than the ElGamal it replaced. There was
   no trade-off to pay.

And the budget closed in the real contract, not in the sum of the probes: **a
confidential vote costs 36,812,994 instructions, 9.2% of a transaction, with
10.9× headroom.** The sum of the primitives was 33,480,865; the nearly 10% extra
is authorization, state and event. (It was 36,781,170 before the time lock; the
0.09% difference is the ciphertext shape check, and nothing else.)

A **public** ballot costs 350,372 — **105× less**. That is the price of secrecy,
measured.

And one that answers the most common question: **ZK fits.** `pairing_check`
shares the final exponentiation across pairs, so a Groth16 verification uses
11.8% of a transaction and 8 would fit in one. The ZK bottleneck on Stellar is
not the chain's budget.

### The probes

| probe | question | verdict |
|---|---|---|
| 1 | Do the BLS12-381 host functions exist and does serialization match? | yes |
| 2 | How much does each primitive cost? | table above |
| 3 | Does a sigma protocol verify inside the contract? | yes |
| 4 | Does the homomorphic tally close end to end? | yes |
| 5 | Is searching for the total viable? | **no** — check, never search |
| 6 | Does Groth16 fit in the budget? | yes, 11.8% of the ceiling |
| 7 | Does the Pedersen design close, and does it reject a lying committee? | yes, and it is cheaper |
| 8 | How far back can the verifier read? | **7 days** on RPC — and after that, the ring check and the chain check vanish together |
| 9 | Does state get archived? | **default TTL of 7 days**, ceiling of 180 |
| 10 | How much does validating an incoming point cost? | 738,901, or 1.1% of the transaction |
| 11 | Is `H` an honest generator? | yes, and it is not `±k·G` for k in 1..512 |
| 12 | Does the well-formedness proof fit? | **yes, 10.98M — the projection was off by 2.5×** |
| 13 | How much does checking eligibility cost? | 127,863 — the projection was off by **8×** |

Details and method in [`bls-smoke/RESULTADOS.md`](bls-smoke/RESULTADOS.md)
(in Portuguese).

---

## The ring, the sections, and the ceiling that disappears

Hiding the choice is not enough when the ballot's sender is the voter's address.
The ring solves that: the ballot comes from a one-time key, with a signature
proving that the signer is among those who turned out **without saying which
one**. A key image `I = x·Hp` collides when the same person tries to vote twice,
and the contract rejects it with `ImagemJaUsada` (image already used) without
knowing whose it is.

The ring is the disjunctive proof from `core/src/cds.rs` generalized from 2 to
`n` branches — same sigma protocol, same Fiat–Shamir. **No SNARK, no setup
ceremony, no circuit.**

Verification costs **10,822,850 instructions per member**, linear. Measured with
the whole `votar_anonimo()`, not the sum of the primitives:

| in the ring | instructions | of the 400M ceiling |
|---|---|---|
| 5 | 94,517,040 | 23.6% |
| 10 | 148,747,642 | 37.2% |
| 20 | 257,211,151 | 64.3% |
| 30 | 365,680,568 | 91.4% |

The line is `40.3 M fixed + 10.85 M per person`, so the arithmetic wall sits at
32. But on testnet a ring of 30 showed a different ceiling first: **only one such
ballot fits per ledger**, and 18 of 30 confirmed in 646 s while the rest
expired.

Hence **sections**. The electorate is split, each section has its own ring, and
the cost per ballot stops depending on the size of the vote. Thirty people in
three sections of ten, on testnet:

```
roll         30/30 accepted · 30 transactions · all in the same ledger
ballot box   30/30 accepted · 3 per ledger · 10 ledgers · 240 s
ballot       148,911,943 instructions · 37.2% of the ceiling
```

**With client-side retries, and that is part of the result.** The same round
without them accepted 16 of 30: ten ballots passed over until they expired and
four rejected over fees. It is not a contract limit — it is the network's
inclusion auction, and anyone writing a voting client needs to know that
resubmitting is mandatory, not an optimization. With `RETENTATIVAS=12`
(retries) nobody needed more than one attempt at the roll.

**With a committee, the result is single.** The accumulator is per proposal and
does not know which section each ballot came from.

**With the time lock, the section publishes no total at all.** It stores how
many ballots opened and the sum of the opened commitments — a Pedersen point,
perfectly hiding. The vote count exists in one place only: one more transaction,
over the whole electorate.

The dilemma this solves was real, and for a while the project declared it as a
limit instead of fixing it. The commitment chain, the ring and the rule that
prevents locking the scoreboard are all per section; an aggregate total seemed to
require every section to be tallied first, and then a section nobody tallied
would lock the whole scoreboard. The way out was to add up **points**, not
totals: each section contributes a point, the aggregate checks the same
two-term MSM as always, and the anonymity floor now applies to the **aggregate**
count — which cannot be locked, because a saboteur only withholds their own
ballot and the aggregate draws on the whole electorate.

What you pay, in any case, is the anonymity set, which becomes the section:
sections buy throughput by paying with the size of the group you hide in.

### Who splits the sections

Nobody chooses, and **not even whoever opens the vote can choose**. If the
organizer chose, they would put a dissenter alone in a section — a ring of one,
the vote tied to the person, no collusion needed.

The section is `H(0x03 ‖ beacon signature ‖ address) mod sections`, and the
signature is from a round that **has not yet come due when the vote is opened**.
There is nothing to grind: at opening time, the number that decides the sections
does not exist in the world. Once it exists, anyone records it in the contract —
which checks it with a pairing before accepting — and from then on anyone with
the list recomputes the whole split.

The first version derived it from `H(0x03 ‖ proposal ‖ address)` and put the
section **inside the Merkle leaf**. It was checkable, and still broken: whoever
opened the vote could grind the proposal identifier until it landed on a draw
that isolated someone. Swapping the proposal for the signature closed that, and
the leaf went back to `H(0x00 ‖ address ‖ weight)`.

The price is imbalance — a hash and a division do not round-robin, so sections
come out in different sizes. What pays for it is **sizing**: a target of 20 per
section, a ceiling of 32 from the ring's CPU. Measured by simulation, the chance
of a section falling below `τ` is 0.002% in the worst small range and 0.071% at a
thousand eligible voters — and when it does, the contract **refuses to tally
rather than leak**.

---

## The time lock

A ring vote does not share the factor that hides the vote with anyone. That
left the assembly **without a committee** in an uncomfortable place: absolute
secrecy and an impossible result. To get a scoreboard you had to pick trusted
people — and trusted people are exactly what a secret ballot should not require.

The time lock swaps the human holder for a **clock**. The ballot leaves the tab
with the factor `r` encrypted to a future round of [drand](https://drand.love) —
the `quicknet` chain, a public beacon that signs a round every 3 seconds. The key
that opens the ballot **is** that signature. Before the round's instant it has
not been published; after it, it is published for everyone.

The scheme is `tlock`'s: Boneh–Franklin identity-based encryption with the round
in place of the identity, derandomized Fujisaki–Okamoto style — which makes
decryption **check the ciphertext itself** instead of silently returning garbage
when a byte changes.

**The contract does not decrypt, and does not need to.** The Soroban host only
exposes `pairing_check`, with no pairing that outputs a value, and decryption
needs the value. It checks the ciphertext's shape and carries it in the event.
Whoever decrypts is anyone, after the round, in the browser — and what rejects a
lying total is still the Pedersen commitment, as it always was.

### The two gates

Tallying requires both, and both are clocks, not assumptions:

| gate | what it measures | error |
|---|---|---|
| the window closed | ledger sequence | `#7 VotacaoAindaAberta` (vote still open) |
| the round came due | unix instant | `#35 RelogioAindaNaoAbriu` (clock not open yet) |

Neither is enough alone, and the order matters: while the box accepts ballots,
whether the round has already come due is irrelevant. **Before both, no
scoreboard comes out — not even a partial one.**

### Nobody can lock the scoreboard

Whoever tallies presents the ordered list of every ballot in the section and
says which ones the ciphertext opened. The contract re-chains the list against a
32-byte hash chain that the ballots wrote on the way in, adds up the marked
commitments, and checks the same two-term MSM as always.

And it stores the result **only if it opens more ballots** than the tally
already stored. That is what prevents locking: anyone who leaves out an honest
ballot is overridden by anyone who includes it — and anyone can, because the
round's key is public. One honest observer is enough; voters do not have to come
back.

A sabotaged ballot — one byte swapped in the ciphertext, which the contract
cannot detect — **costs only its own vote**. There is no `τ` floor on how many
opened, and the absence is deliberate: requiring one would give anyone the power
to lock the tally by spoiling their own ciphertext.

### Measured on testnet

End-to-end round on 2026-10-10, proposal `7021e473…`, **no committee at any
point**, 7 ballots and one sabotaged on purpose:

```
ballot          320 B of ciphertext · 123,605,080 instructions (ring of 7)
box open        apurar rejected · #7  VotacaoAindaAberta
box closed      apurar rejected · #35 RelogioAindaNaoAbriu
round due       6 of 7 opened · scoreboard 2 · 4
partial of 2 landed first and was overridden by whoever included the 6
going back      rejected · #37 NaoMelhora
apurar_secao        3,048,531 instructions · 0.76% of the ceiling
publicar_resultado 12,702,177 instructions · 3.2% of the ceiling
lying aggregate rejected · AberturaNaoFecha
```

(`NaoMelhora`: does not improve. `AberturaNaoFecha`: opening does not close.)

Checking the beacon's signature inside the contract costs **24,053,490
instructions**, 6% of the ceiling. The per-section tally became **4.5× cheaper**
once the MSM left it: 3.0 M against the 13.7 M the same call cost when it
published totals. What appeared was the scoreboard transaction, 12.7 M, which
does not grow with the ballots — so with a single section the total is about the
same, and from two on it is smaller.

### What this costs in trust, said in full

**The beacon is an assumption, and it is the only one in the project that
depends on people.** A collusion of a threshold of the drand operators would
produce the signature early and open the **content** of the ballots in
progress. They are people nobody in this vote chose and who have no stake in it —
but they are people. That is why the project does **not** say that opening early
is impossible, and does **not** call the beacon "trustless".

What does not depend on it is the scoreboard: no result comes out before the
window closes, and that is the ledger's clock.

**And there is no plan B.** If the beacon stops publishing, the key is never
born and the scoreboard does not exist — the ballots stay on the ledger,
unreadable, until the round is published some day. The link still exists
nowhere, so nothing leaks; but the result does not come out. Promising a result
with the beacon stopped would be a lie, so we don't.

---

## The full round, on testnet

```bash
tessera abrir   --proposta contas --pergunta "Approve the 2025 accounts?" \
                --opcoes approve,reject --aptos marta,joao,ana,… \
                --mesa mesa1,…,mesa5 -k 3 --prazo 8m
tessera cedula  --proposta contas --identidade marta     # shows, does not vote
tessera votar   --proposta contas --opcao reject --identidade marta
tessera queimar --identidade marta
tessera apurar  --proposta contas
tessera verificar --proposta contas
```

The commands are `abrir` (open), `cedula` (ballot), `votar` (vote), `queimar`
(burn), `apurar` (tally) and `verificar` (verify); the flags are `--proposta`
(proposal), `--pergunta` (question), `--opcoes`/`--opcao` (options/option),
`--aptos` (eligible voters), `--mesa` (committee), `--prazo` (deadline) and
`--identidade` (identity).

And the round **without a committee**, which has no local file at all for the
verifier to read — the ballot left the browser, and the proposal identifier is
the one in the dapp's URL:

```bash
tessera verificar --proposta 7021e473…b7793e8 \
                  --contrato CDWGDWDPCXAAA3FO24NRRYS77HEMFHWDIBK4O3GB6L6BQGYWC7JYW3YS
```

It re-reads the commitments from the **events**, re-chains the chain against the
getter that says how many ballots should be there, checks that the key images
are all distinct, **redoes every ring signature** from the arguments of the
transaction that cast the ballot, decrypts every ciphertext against the beacon
round and checks that each commitment opens to `v ∈ {0,1}` — and only then
compares the scoreboard it recounted with the published one. The `Hp` the key
images are formed against is **recomputed**, not read from the contract: reading
and using it would mean trusting the thing being verified to say how double
voting is detected.

The CLI prints in Portuguese; this is its output verbatim:

```
Hp da proposta ..... 191e…c727 recalculado do identificador          ✓
Fechadura .......... rodada 32951126 conferida no pareamento         ✓
Seção 0 · cadeia ... 7 de 7 cédulas re-encadeiam                     ✓
Imagem de chave .... 7 distintas em 7 cédulas                        ✓
Anel ............... 7 assinaturas refeitas do zero                  ✓
Decifragem ......... 6 de 7 cédulas abriram em v ∈ {0,1}             ✓
Placar ............. ledger diz 2 · 4 · eu recontei 2 · 4            ✓
```

Line by line: the proposal's `Hp`, recomputed from the identifier; the time
lock, round 32951126 checked with a pairing; section 0's chain, 7 of 7 ballots
re-chain; key images, 7 distinct in 7 ballots; ring, 7 signatures redone from
scratch; decryption, 6 of 7 ballots opened to `v ∈ {0,1}`; scoreboard, the
ledger says 2 · 4 and the verifier recounted 2 · 4.

In ring mode the CLI **opens** the vote and does not vote in it — the ring vote
belongs to the dapp, which is where the ring key can live in the voter's
browser:

```bash
tessera abrir --proposta assembleia --anel --secoes 3 \
              --pergunta "Approve?" --opcoes approve,reject \
              --aptos … --mesa mesa1 -k 1 --inicio 5m --prazo 30m
```

(`--anel`: ring; `--secoes`: sections; `--inicio`: start.)

Real cost per vote, measured:

| | |
|---|---|
| `abrir` | 1.83 XLM |
| confidential `votar` | **157,267 stroops ≈ 0.0157 XLM** |
| keeping the contract alive for 180 days | 181.70 XLM, once |

`cedula` is the command that does not vote: it shows the two possible
commitments side by side, labeled A and B, without saying which is which.
Whoever is watching tries to tell, fails, and the failure is the demonstration.

---

## Three findings worth a warning

**Probes 8 and 9 found the same number by different routes.** The public RPC's
retention window is 120,960 ledgers; the default TTL of a persistent entry is
120,959. Both are **7 days**.

Left alone, on the seventh day both levels of verification die together: the RPC
stops serving the events *and* the accumulator is archived. Nothing raises an
error on day 1. The design consequences are in
[`docs/PROTOCOLO.md`](docs/PROTOCOLO.md) §5.1 and §8.3 (in Portuguese) — in
short:

- the verifier for the **committee** path reads from the **history archives**,
  not from RPC; the one for the **anonymous** path reads from the **events**,
  and so faces the 7-day window head-on: it prints the ledger range it can still
  reach **before** any verdict, and says how many ring signatures fell outside
  it instead of omitting the line. After the 7 days, the ring check and the
  chain check vanish together — the scoreboard stays checkable through the
  getters, the path to it does not;
- `abrir()` extends the TTL of `Proposta` and of the accumulators to the
  network ceiling (**1.73 XLM**, independent of turnout), and lets the `Votou`
  ("voted") entries be archived;
- the rent for **the contract itself** — 29 KB of Wasm for 180 days — costs
  **181.70 XLM** and lives in a separate `manter()` ("maintain") function that
  anyone can call. In the first version it was inside `abrir()`, and measuring
  on testnet showed the first call costing **182.39 XLM** against **1.72** for
  the second: the first governance to use the module would pay the bill for all
  the others.

**The point at infinity is not 96 zero bytes.** The host rejects zeros with
"point not on curve". The encoding is the zcash format's flag bit — `0x40` in
the high byte, zeros in the rest — and it is not written down anywhere we could
find; it was found by adding each candidate to the generator and seeing which one
returned the generator.

It matters because **the accumulator of a proposal with no votes is the point at
infinity**: getting it wrong breaks the first ballot of every vote, and only the
first. It is pinned from both sides, in `core/src/ponto.rs` and in the test
`o_infinito_do_host_e_a_flag_zcash_nao_zeros`.

**The `k`-of-`n` committee does not co-sign: it endorses.** The first version
asked for `k` authorizations in a single transaction, and that **cannot be
expressed** with Stellar's tooling — `stellar tx sign` signs the envelope, not
Soroban's authorization entries, and the network rejects it with
`TxBadAuthExtra`.

The fix turned out better than the original design. Each member sends **their
own** transaction and the contract counts the endorsements, which is how `k`
people on `k` machines actually work. And each endorsement is bound to
`sha256(totals ‖ openings)`: nobody endorses "the tally" in the abstract, they
endorse **these numbers**. Disagreeing is not endorsing.

Since every endorsement checks everything, a false tally is rejected at the
**first** member who tries, not the last.

**For anyone reproducing this:** the invocations recorded here leave the RPC
window seven days after they were made. After that, read from the history
archives.

---

## Reproduce

```bash
cd bls-smoke   && cargo test --lib -- --nocapture --test-threads=1  # 13 probes
cd ../core     && cargo test --lib                                  # 81 tests
cd ../contrato && cargo test                                        # 60 tests
cd ../cli      && cargo test                                        # 33 tests
cd ../app      && for t in scripts/*.test.mjs; do node "$t"; done    # 8 tests
```

Requires `rustc 1.97.1`, `stellar-cli 25.2.0`, the `wasm32v1-none` target, and
Node 22+ (the app tests use Node's native type stripping).
**One hundred eighty-seven tests in Rust and eight in JavaScript**, all passing,
no network.

The dapp needs the wasm client before it runs:

```bash
cd app && npm run wasm && npm run dev     # http://localhost:5273
```

`npm run wasm` rebuilds and **relinks** the package: `pnpm` hard-links the files
and `wasm-pack` rewrites the `.wasm` as a new file, so without the relink the new
glue calls the old binary and the symptom is
`table index is out of bounds`.

The `core/` crate is the math shared by the contract, the client and the
verifier. It uses **arkworks, the same crate as the Soroban host** — which
removes at the root any divergence between the native prover and the Wasm
verifier. And the prover↔verifier cross-check is not a frozen vector: **the
contract tests generate the proofs with `core`, in native Rust, and verify them
on the Soroban host, in Wasm, on every `cargo test`.** The full round — six
people voting in secret, the committee opening the aggregate, the right result
coming out — runs with real cryptography from start to finish.

The cost tests assert a ceiling: a cost regression breaks the build instead of
showing up in the demo.

Contracts on testnet:

| | |
|---|---|
| **Tessera (mixed ballot)** | `CDWGDWDPCXAAA3FO24NRRYS77HEMFHWDIBK4O3GB6L6BQGYWC7JYW3YS` |
| Tessera v1, single-question ballot | `CBLUSE2LCPPELS7GIQ5MRYKSW7KTILMWY7VBRFRSWAVG3L7TSRQYHP7H` |
| probes 1–13 (crypto, CDS and Merkle) | `CCL4CPAJ4ZVP25FP2PO7T3AMYGIVQZLYJZYZS53UAM5IFA5NFYGLUISR` |
| probe 9 (TTL) | `CCCZ4HPW3ESHO6BMNFGXX6DJRSWPWA7FBQH2MS6QGHC3U67DIZ434KFC` |

Test vectors in [`bls-smoke/vetores.env`](bls-smoke/vetores.env).
Watch out: `Bls12381Fr` goes into `stellar contract invoke` as **decimal**, never
hex — and a hex string that happens to contain only digits is silently accepted
as the wrong decimal.

---

## Documents

The documents below are in Portuguese, except where noted.

| | |
|---|---|
| [`docs/PROTOCOLO.md`](docs/PROTOCOLO.md) | the protocol specification: threat model, cryptography, interface, budget, rejected designs |
| [`docs/SMOKES.md`](docs/SMOKES.md) | the smoke-test catalog, with the decision tree |
| [`docs/PLANO.md`](docs/PLANO.md) | schedule up to submission, with gates and pre-decided cuts |
| [`docs/UX.md`](docs/UX.md) | the UX specification, and why this product's value is an absence |
| [`docs/UX-CLI.md`](docs/UX-CLI.md) | the exact CLI output, measured in columns |
| [`docs/VIDEO.md`](docs/VIDEO.md) | the dapp video script, with the measured timings and what §2 forbids saying |
| [`docs/design-votacao.md`](docs/design-votacao.md) | the dapp design, the backstage view and the journal guard |
| [`docs/SPEC.md`](docs/SPEC.md) | the working specification: invariants, gates, task board, decisions |
| [`docs/RELOGIO.md`](docs/RELOGIO.md) | the time lock: the beacon, the three measurements that unlocked it, and what it costs in trust |
| [`docs/SOURCES.md`](docs/SOURCES.md) | every external fact checked, with dates |
| [`CLAUDE.md`](CLAUDE.md) | how work is done in this repository (English) |

Decks: [Portuguese](index.html) · [English](index.en.html)

Console: [`console/`](console/) — a viewer over a real round, with the grid of
indistinguishable commitments that does not fit in a terminal. It is a
**viewer**, not an application: no wallet, no server, and it signs nothing.

dapp: [`app/`](app/) — React + TypeScript, static, with `tessera-core` compiled
to WebAssembly. The proofs are born in the voter's tab, so **there is no server
that could see the randomness factor, because there is no server**. The
`/bastidores` (backstage) route shows, in a second window, what the contract
answers to each act — including the instruction cost the simulation found.
It is live at [thaisfreis.github.io/tessera/dapp](https://thaisfreis.github.io/tessera/dapp/).

---

## What this protocol does **not** guarantee

Stating the limits is part of the design. A voting system that promises more
secrecy than it delivers is worse than an honest one.

- **Two guarantees of different kinds, and they are not the same thing.** The
  Pedersen commitment is **perfectly** hiding: there is no assumption to break,
  not even with a quantum computer. The ring's anonymity is **not** — it rests on
  a problem believed to be hard. Flattening the two into one sentence would sell
  something that does not exist.
- **Anonymity of turnout.** The roll records *that* the account turned out. That
  is intentional: it is where the list of who stayed away comes from, and without
  it there is no compulsory voting. What the ledger does not show is which ballot
  is theirs.
- **The eligible list is public in ring mode.** A ring can only be verified by
  someone who has the keys of every branch: ring anonymity is anonymity *within a
  known set*. That is consistent with the secret ballot, where the electorate and
  the attendance list are public and only the link is secret — but it is a
  trade-off, and it did not exist in the identified design.
- **A ring of one hides nobody.** Secrecy is a property of the group, not of the
  math alone. With sections the contract rejects a ring below `TAU`; without
  sections it warns and lets it through, because there nobody chose the group.
- **A ring vote with no committee and no time lock does not tally.** It does not
  share the factor that hides the vote with anyone, so nobody reconstructs the
  opening — and nobody publishes a total. It is not a promise, it is the
  contract: any claimed total falls into `AberturaNaoFecha`. Secrecy is absolute
  and the result is impossible. **With the time lock this no longer holds**, and
  on purpose: the holder becomes the clock, and anyone tallies after the round.
- **Coercion resistance, while you vote.** Whoever is watching your screen sees
  your choice, and no protocol fixes that. What Tessera guarantees is the
  **after**: the factor `r` that hides the vote is born in your tab and dies with
  it, no client stores it, and no command reads such a file.

  v1 stored a receipt with the `r_j` values and asked the person to delete it. It
  was worse than useless: a coercer simply orders you not to delete it, and no
  command read the file — it served neither to check a vote nor to tally. Today
  `votar` stores nothing, and `queimar` exists only to clean up leftovers from
  old rounds, overwriting before removing. Two tests pin this, and one of them
  reads the source of `comandos.rs` itself, because absence cannot be proven any
  other way.

  **What remains open:** a tampered client can keep `r` without you knowing.
  Running your own is the defense, and that is why the dapp is static and `core`
  is the same crate on every path.
- **The content of an opened ballot.** What the protocol protects is the *link*,
  not the content: whoever gathers the opening learns what that ballot voted, and
  never whose it is. With no committee and no time lock, nobody gathers anything.
  **With the time lock, after the round, anyone gathers it** — and what stays
  protected is the link, which is not stored anywhere to be gained.
- **The beacon, and the lack of a plan B.** The time lock depends on drand
  publishing the round's signature. A collusion of a threshold of its operators
  opens the content of the ballots early; if it stops, the scoreboard does not
  exist and there is no fallback. It is the only assumption in the project that
  depends on people, and that is why it is written here and not left implicit.
- **The aggregate scoreboard is public, as in any vote.** A unanimous result
  reveals everyone, and that cannot be fixed in any system. What was fixed is the
  smaller case: a section no longer publishes its own total, only a count and a
  point, so a unanimous section no longer gives away its members.
- **A committee that does not destroy its copies.** The `k`-of-`n` threshold is
  implemented in `core/src/shamir.rs` and protects against collusions below `k`:
  the committee adds the shares locally and no individual `r` is ever assembled
  anywhere. Above `k`, with the raw shares kept, the protection is
  procedural.
- **A unanimous result**, which reveals everyone in any voting system.
- **Weighted voting with distinct public weights**, which is broken by
  subset-sum. The contract **rejects** that configuration.

---

## License

MIT. See [LICENSE](LICENSE).
