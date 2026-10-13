# Tessera — pitch video script

This is the **pitch video**, the one that goes with the submission and the
seven-slide deck. It is not `docs/VIDEO.md`: that one is the **nine-minute
demo**, from sign-in to tally, real time and uncut. The two live side by side —
the pitch gets you watched, the demo proves it.

The Portuguese version is `docs/VIDEO-PITCH.md`, with the same eight blocks.

> **What this video has to prove, in one sentence:** there is a way for a
> governance body to vote in secret and still have anyone re-check the result —
> and it is standing, measured, on testnet.

**Real length: 4min11** of speech at 160 words a minute, pauses not counted. I
tried to make it fit 3min00 and it does not fit without wrecking the demo, which
is the only block that convinces on its own. If the limit is a hard three
minutes, cut in this order — each cut is marked in the text:

1. the second example in §2 (−4s);
2. the middle paragraph of §6, the one about tally cost (−15s);
3. the first sentence of §5, through "the hard part" (−7s).

That lands at 3min45 — still over three minutes. Below that §4 starts losing
the two columns, and then the video stops proving anything.

---

## How this text was written

Written to be **heard once**, not read. A listener cannot go back, cannot see
the comma, and has not memorised the slide. The rules I followed:

- **Short sentences, one idea each.** Where there was an em dash, there is now
  a full stop.
- **Concrete subjects.** People act, not concepts. "Secrecy protects the freedom
  to dissent" became "they just don't vote".
- **Explain before naming.** First what the thing does, then what it is called —
  never the other way round. That is why "Pedersen commitment" is gone: the name
  did not fit without stealing the explanation's time.
- **No triads.** Three parallel items in a row read as generated text. One good
  example beats three summarised ones.
- **No "it's not X, it's Y" formula**, which appeared five times in the earlier
  draft.
- **Second person.** "How you voted" grips harder than "individual choice".

### The golden rule

**No number that was not measured, no word that `§2` of `SPEC.md` forbids.**
This script never says "secure", "audited", "production-ready" or "trustless",
and never promises a result if the beacon stops.

### Check the day before recording

- [ ] **The dapp responds.** On 13 Oct `thaisfreis.github.io/tessera/dapp/`
      returned **404** — PR #23 was not merged, and Pages serves `master`.
      Slide 04 of the deck already points at that link.
- [ ] The contract responds on the explorer.
- [ ] The round you will show is still inside the RPC retention window, if you
      are going to show the verifier.

---

## Structure, slide by slide

| Block | Slide | Time | Screen |
|---|---|---|---|
| 1 Hook | 01 cover | 0:00–0:09 | you, or the cover |
| 2 Problem | 02 | 0:09–0:35 | slide |
| 3 Design | 03 | 0:35–1:41 | slide |
| 4 Demo | 04 | 1:41–2:28 | **screen capture** |
| 5 Why Stellar | 05 | 2:28–2:49 | slide |
| 6 Evidence | 06 | 2:49–3:23 | slide |
| 7 Team | — | 3:23–3:42 | you, or Karn |
| 8 Close | 07 | 3:42–4:12 | slide |

§7 has no slide in the deck — see the note at the end of this page.

---

## 1. The hook — 0:00 to 0:09

**Screen:** you on camera, or the cover. Nothing else.

> Nobody can find out how you voted. Not even whoever ran the vote.
>
> This is Tessera, and it is running on Stellar testnet right now.

**Direction:** this is the only chance the video gets. Say it slowly, with no
"hi, my name is" — the name comes in §7.

---

## 2. The problem — 0:09 to 0:35

**Screen:** slide 02.

> Today, when you vote in a DAO or on a board, your vote is public. With your
> name next to it. Forever.
>
> Think of the board member who has to vote against the budget of the person
> they report to. Or the contributor voting against the whale who pays for
> their work.
>
> These people don't vote wrong. They just don't vote. And that never shows up
> anywhere.

**Direction:** "They just don't vote" is the line of the block. Pause before and
after it.

**Cut 1, if you need it:** drop "Or the contributor voting against the whale who
pays for their work." The board member alone does the job.

---

## 3. The design — 0:35 to 1:41

**Screen:** slide 03.

> The obvious fix would be to encrypt the vote. Except then you trade one
> problem for another. The encrypted vote stays recorded forever, and the key
> that opens it is somewhere. Five years from now somebody finds that key and
> reads how you voted today.
>
> So Tessera doesn't record your vote. It records a number that proves you
> voted, without saying what you voted. On its own that number is useless. But
> you can add all of them up and check the result.
>
> And opening a ballot, to see what it voted? That depends on how the vote was
> set up. In the simple mode, nobody opens anything. Ever. Not even me. The
> result comes out of the sum, and no ballot is ever opened one by one.
>
> With a time lock, anyone can open them. But only after the appointed hour,
> and the key isn't held by anyone: it is born on its own, out of a public
> draw, at the agreed moment.
>
> And opening a ballot shows what it voted. Never whose it is.

**Direction:** "Not even me" is the turn of the whole pitch. Do not rush it.

---

## 4. The demo — 1:41 to 2:28

**Screen:** capture of the dapp. It is the longest block on purpose, and the one
block that cannot shrink.

> Enough talking.
>
> This is a real vote, on testnet. No organiser, no server, nobody holding a
> key.
>
> First the person signs in. They prove they are entitled to vote and sign the
> attendance roll, with their address, in public. Everyone sees who turned up.
>
> Then they vote. From a different address, created on the spot, with nothing
> to do with the first one. The ballot proves it came from somebody on that
> roll, without saying who.
>
> Look at the two columns. Thirty people voted. Not one address appears in
> both. Not one.
>
> The deadline passes and the result comes out. The wallet that tallied it
> never voted. One of the seven ballots was corrupted on purpose: it lost its
> own vote, and the other six counted.

**Direction:** if you can show **one** thing, show the two columns side by side.
It is the only image in the video that proves something on its own. Don't say
"trustless" — say "nobody holding a key", which is what is true.

---

## 5. Why Stellar — 2:28 to 2:49

**Screen:** slide 05.

> Why Stellar. The heavy maths Tessera needs already ships inside Soroban. I
> didn't have to write the hard part.
>
> So everything can run on direct proofs, with no setup ceremony and no need to
> trust whoever assembled the system.
>
> And it came out cheap. A full secret vote cost a cent and a half of XLM.

**Cut 3, if you need it:** start at "Everything can run on direct proofs".

---

## 6. The evidence — 2:49 to 3:23

**Screen:** slide 06, the lime one.

> None of this is an estimate. All of it was measured on the contract that is
> live.
>
> A full secret vote uses nine per cent of what fits in one transaction. And
> tallying doesn't get more expensive as more people join: from five voters to
> forty, the cost went up one and a half per cent.
>
> But here's the part I like most. You don't have to take my word for it. There
> is a separate program that pulls everything off the blockchain, redoes the
> maths from scratch and checks the result.

**Direction:** this is the deck's only light slide. Hard cut — the contrast does
the work by itself.

**Cut 2, if you need it:** drop the whole middle paragraph.

---

## 7. The team — 3:23 to 3:42

**Screen:** the Karn repository, or you. See the note at the end.

> I'm Thais Reis, a software engineer in Rio de Janeiro. I build applications
> end to end: client, API, database, smart contracts and deployment.
>
> Three first places at hackathons. And Tessera isn't my first governance
> protocol either. I maintain Karn, an open governance protocol. This is exactly
> where it plugs in.

**Where each claim comes from** (CV, 13 Oct 2026):

- **"end to end"** — Yalla, **sole** engineer on the product: React client,
  Node/TypeScript API, PostgreSQL schema, Rust contracts and deployment.
- **"three first places"** — Monad Blitz Rio de Janeiro (2026), Hackathon Nola
  "God Level, Code Your Future" (2025) and Morro Makers (2025).
- **"I maintain Karn"** — open governance protocol, founder and sole
  contributor: three Rust contracts, 53 tests, 5 documented security findings
  resolved, a typed TypeScript SDK published to npm.

**What was left out, and is worth having ready** if the judges ask:

- **Stellar Instaward via Stellar 37º (2026)**, an SDF regional programme, and
  the invitation to the **Builder Summit** as one of about a hundred builders.
  It is the strongest credential you have **for this jury specifically** — the
  SDF has already picked you once. It came out of the script and I won't put it
  back unasked, but if there is a Q&A, it is the first thing to say.
- The numbers from Yalla's first live event: 113 tickets across 8 tiers, 98
  attendees, 257 bar vouchers redeemed with none expired, peaking at 39
  check-ins per hour.
- Two upgradable Rust contracts with deliberately disjoint admin and upgrade
  authority, and one upgrade executed live in production.

> **Confirm before recording:** the block assumes **solo**. If anyone else is on
> Tessera — design, advice, review — tell me and I rewrite it.

---

## 8. The close — 3:42 to 4:12

**Screen:** slide 07.

> This isn't magic, and there is fine print. If the vote has a deadline, it
> depends on that public draw. If the draw stops, no result comes out. If
> everyone votes the same way, the result gives everyone away, same as
> anywhere. And none of this has been audited. I'm not going to say it has.
>
> But checking a decision shouldn't cost exposing everyone's choice.
>
> The decision belongs to all of us. The choice is yours.
>
> Tessera.

**Direction:** hold slide 07 for two seconds after the last word, with the
repository address visible. Do not read the URL out loud.

---

## After recording

1. Put the URL on slide 04 of the deck, in **both** languages — it is marked
   `URL to be inserted before submission`.
2. Put the URL in the README.
3. If §4 was recorded with the dapp still 404, fix the "dapp live" line on
   slide 04 before submitting, or merge PR #23.

---

## What this script deliberately does not say

- That the project is secure, audited or production-ready.
- That opening the content early is impossible — it is collusion by a threshold
  of the beacon's operators.
- That a person's section is secret. It is **unpredictable until opening**,
  which is a different thing.
- That there is a plan B if the beacon stops. There isn't.
- Any number that did not come from a measurement recorded in
  `docs/SOURCES.md`.

---

# Narration for the cloned voice (ElevenLabs v4)

The narration will be generated with a clone of my voice. That changes three
things, which is why this section exists instead of you pasting the blocks above
straight into the synthesiser.

**One:** the synthesiser reads what is written, not what is formatted. Bold and
markdown either get read out loud or wreck the prosody. The blocks below are
plain text.

**Two:** a number written in digits is a gamble. Below, **every number is spelled
out** — the measured value is the same, only the spelling changed.

**Three:** with no face on screen, the team block lost the visual support it had.
See the note at the end of this section.

## The tags

Checked against the [official v4 list](https://elevenlabs.io/blog/elevenlabs-audio-tags-list)
on 13 Oct 2026. **45 marks of eight kinds**, and 41 of them are pacing only:

| Tag | Where, and why |
|---|---|
| `[pause]` `[long pause]` | breath between ideas; the long one only at the four silences the text asks for |
| `[slowly]` | on the seven lines the script says not to rush — the opening, "Forever", "Five years from now", "Not even me", "Not one address appears in both", "I don't have a team" and "I'm not going to say it has" |
| `[snappy]` | once only, on the "Enough talking" that opens the demo |
| `[thoughtful]` | going into the §2 examples |
| `[let down]` | on "They just don't vote" — the line of the block |
| `[curious]` | on "And who can open it?", which is a real question |
| `[proud]` | on "the part I like most", the one time the narration lets itself enjoy something |

**Four emotion tags in the whole script.** The documentation recommends **one tag
per clause**, and I stayed well under that on purpose: a stack of emotion tags is
exactly how narration starts sounding performed. A pitch convinces by holding
back, not by acting.

### Three things I don't know

**One:** the v4 documentation **says nothing about non-English languages**. The
whole list is in English and no example is in another language. This matters for
the Portuguese script, not this one.

**Two:** I have not tested a single tag with your voice.

**Three:** `[let down]`, `[snappy]` and `[long pause]` are the least common on
the list, and the first candidates to leak.

### How to test before spending credits

1. Generate **§2** only — it is short and carries three different tags.
2. Listen for a **tag spoken as a word**.
3. If one leaks: **drop them all**. The text was written to work without them —
   punctuation already carries the rhythm, and the documentation says punctuation
   controls performance too.
4. If they work but sound theatrical, drop the four emotion tags first and keep
   the pacing ones. That is the likeliest adjustment.

> **A note on the model.** The v4 panel offers generation modes, and the v3
> documentation says **Robust** reduces responsiveness to tags. If you pick
> Robust, expect them to do little or nothing.

## How to cut

Generate **one file per block**, not a single four-minute audio. The cut between
blocks lands in silence instead of mid-breath; you redo a bad block without
regenerating the rest; and §4 has to stretch or shrink to match the screen
capture, which is impossible in one file.

## Words to check on the first test

| In the text | If it comes out wrong, write |
|---|---|
| Soroban | Sor-oh-bahn |
| Tessera | Tess-eh-ra |
| XLM | X L M |
| dapp | dapp |

## The blocks

### 1 · hook

```
[slowly] Nobody can find out how you voted. [pause] Not even whoever ran the
vote. [pause]

This is Tessera, and it is running on Stellar testnet right now.
```

### 2 · problem

```
Today, when you vote in a DAO or on a board, your vote is public. With your name
next to it. [slowly] Forever. [pause]

[thoughtful] Think of the board member who has to vote against the budget of the
person they report to. Or the contributor voting against the whale who pays for
their work. [pause]

These people don't vote wrong. [pause] [let down] They just don't vote. 
[long pause] And that never shows up anywhere.
```

### 3 · design

```
The obvious fix would be to encrypt the vote. [pause] Except then you trade one
problem for another. The encrypted vote stays recorded forever, and the key that
opens it is somewhere. [slowly] Five years from now somebody finds that key and
reads how you voted today. [long pause]

So Tessera doesn't record your vote. It records a number that proves you voted,
without saying what you voted. On its own that number is useless. But you can
add all of them up and check the result. [pause]

[curious] And opening a ballot, to see what it voted? [pause] That depends on
how the vote was set up. In the simple mode, nobody opens anything. Ever.
[pause] [slowly] Not even me. [pause] The result comes out of the sum, and no
ballot is ever opened one by one. [long pause]

With a time lock, anyone can open them. But only after the appointed hour, and
the key isn't held by anyone: it is born on its own, out of a public draw, at
the agreed moment. [pause]

And opening a ballot shows what it voted. [slowly] Never whose it is.
```

### 4 · demo

```
[snappy] Enough talking. [pause]

This is a real vote, on testnet. No organiser, no server, nobody holding a key.
[pause]

First the person signs in. They prove they are entitled to vote and sign the
attendance roll, with their address, in public. Everyone sees who turned up.
[pause]

Then they vote. From a different address, created on the spot, with nothing to
do with the first one. The ballot proves it came from somebody on that roll,
without saying who. [pause]

Look at the two columns. Thirty people voted. [slowly] Not one address appears
in both. [pause] Not one. [long pause]

The deadline passes and the result comes out. The wallet that tallied it never
voted. [pause] One of the seven ballots was corrupted on purpose: it lost its
own vote, and the other six counted.
```

### 5 · why Stellar

```
Why Stellar. [pause] The heavy maths Tessera needs already ships inside Soroban.
I didn't have to write the hard part. [pause]

So everything can run on direct proofs, with no setup ceremony and no need to
trust whoever assembled the system. [pause]

And it came out cheap. A full secret vote cost a cent and a half of XLM.
```

### 6 · evidence

```
None of this is an estimate. All of it was measured on the contract that is
live. [pause]

A full secret vote uses nine per cent of what fits in one transaction. And
tallying doesn't get more expensive as more people join: from five voters to
forty, the cost went up one and a half per cent. [pause]

[proud] But here's the part I like most. [pause] You don't have to take my word
for it. There is a separate program that pulls everything off the blockchain,
redoes the maths from scratch and checks the result.
```

### 7 · team

```
I'm Thais Reis, a software engineer in Rio de Janeiro. [pause] I build
applications end to end: client, API, database, smart contracts and deployment.
[long pause]

[proud] Three first places at hackathons. [pause] And Tessera isn't my first
governance protocol either. I maintain Karn, an open governance protocol.
[slowly] This is exactly where it plugs in.
```

### 8 · close

```
This isn't magic, and there is fine print. [pause] If the vote has a deadline,
it depends on that public draw. If the draw stops, no result comes out. If
everyone votes the same way, the result gives everyone away, same as anywhere.
And none of this has been audited. [slowly] I'm not going to say it has. 
[long pause]

But checking a decision shouldn't cost exposing everyone's choice. [pause]

The decision belongs to all of us. [pause] The choice is yours. [long pause]
Tessera.
```

## What to show during the team block

§7 is now about who you are, and every line in it has an obvious image behind it.
The screen should **show what the voice claims**, not illustrate generically.

| While the voice says | On screen |
|---|---|
| "a software engineer in Rio de Janeiro" | you — photo or camera |
| "ticketing platform... people at the door" | the platform running, or a photo of the event door |
| "three first places" | the three trophies, event logos or photos |
| "Stellar invited me to the Builder Summit" | the Stellar logo, or a photo from the Summit |
| "I maintain Karn" | the Karn repository |

If you can prepare **one**, prepare the last. Karn is the one that closes the
pitch: the deck already says Tessera's intended integrations are Karn and
OpenZeppelin's Governor, so the screen proves you are not proposing to plug into
somebody else's thing — you maintain both ends.

If you can prepare none, your photo with your name is honest and sufficient.
What I'd avoid is slide 07 with a signature line: the close loses force splitting
attention with an authorship credit.

> **Honesty note.** Nothing obliges you to say in the video that the narration is
> synthetic — it is your own voice, cloned by you, and that is production, not
> misrepresentation. But `§2` of `SPEC.md` exists because this project decided
> not to carry a single claim without backing. One line in the end credits,
> "narration generated with a clone of my voice", costs two seconds and matches
> the rest.
