# 46 · The act path, stages 1–2: a babbling act gate replaces the question detector (2026-10-25)

*[All experiments](README.md)* · [← 45](45-validated-records.md) · [47 →](47-hearing-itself.md)

**The owner's point:** the reader needs an act path, reached through developmental stages (babbling first), and an act path gives the separation we want because cues for acting get routed there. The plan (project doc `claude/act-path-plan.md`) has four stages: babbling, routing, hearing itself, goal-directed acts. This page builds and tests the first two.

**Results (one seed each):**
- **The act gate learns "?" and nothing else, on every stream** (small qa2, story set, pilot, long BABILong), from babbling alone. It needs neither L nor working memory: experiment 44's contrast detector found "milk went apple football", and the old mood counter only looks at words in L.
- **It replaces the question detector and the question drop at no cost** on experiment 42's stack: small qa2 99%, story set 97.0%, pilot 100 / 100%.
- **It unblocks the collision ledger,** which never puts "?" in L: pilot 0 / 0% → 72 / 38%.
- **The union of k90 and the ledger, with the act path, scores 100% / 8 of 8 on long BABILong** (16k / 64k), the best long result so far.

## The rules (`--sit-act`, binary `sit-46a`)

**Stage 1, babbling: learn where acting pays.**
- **The act.** At each word u, the actor's candidate is memory's top content proposal (a value of a kind with 3 or more members).
- **Babbling.** During training it acts at 1 position in 8 at random, as well as wherever its gate says Go.
- **Payment, against the level below.** An act is paid when its word is the next word and the bigram (the most frequent successor of u) would have missed it. It is charged when wrong, and counts as nothing when the bigram alone was right.
- **The gate.** Go after u when paid > charged, with at least 16 counted; counts halve at 4,096. The gate is skill and freezes with the route learner.
- **What it replaces.** The question detector everywhere: where to answer in answer-only runs, head credit, open queries and ranges.

**Stage 2, routing.** At a Go word, the clause before it is the act's cue and is not committed to fact memory. This replaces the question drop, and the Go word need not be in L.

Script `runs/exp46/a.sh`. Stacks are experiment 42's: small qa2 with chains and `--sit-lt-budget 131072`; long BABILong with the budget and `--sit-entity-fair`.

| Run | Small qa2 | Story set (where-object) | Pilot 0k / 4k | Long 16k / 64k | Act gate's words |
|---|---|---|---|---|---|
| Exp 42's best (k90 + judge, mood detector, drop) | 100% | 98.5% (92.6%) | 100 / 100% | — | — |
| **k90 + judge + act path** | 99% | 97.0% (91.4%) | 100 / 100% | — | "?" on every stream |
| G2 ledger + judge (exp 44, mood detector on L) | 100% | 95.8% | 0 / 0% | 0 / 0% | — |
| **G2 ledger + judge + act path** | 100% | 97.0% (91.4%) | **72 / 38%** | — | "?" |
| Union + judge (exp 45, mood detector) | 100% | 97.8% (91.4%) | 100 / 100% | 100% / 7 of 8 | — |
| **Union + judge + act path** | 99% | 96.1% (87.7%) | 97 / 100% | **100% / 8 of 8** | "?" |

- **The gate picks the right place because an act must produce a content word that the world then says, and the bigram couldn't have predicted.**
  - After "milk", memory's content proposal (a place) is followed by "." or "?", so acting there is charged.
  - After "?", the answer follows, so acting there is paid whenever memory is right and the answer isn't the most common one.
- **The story set falls 1–2 points with the act path** (97.0% against 98.5% on k90; 96.1% against 97.8% on the union). The plan's prediction was ≥ 98%, so this misses it, though not the falsifier (< 97) on k90. One seed; experiment 44 showed about 2 points of single-run spread on the story set.
- **The ledger still fills L with book words on qa1,** which caps the pilot (72 / 38%). The act path removes the "?" dependence, not the noise.
- **The pilot's 1k training stories drop** with the act path (train qa1 1k: 73% on k90, against 96–97.5% before). The test is unaffected (100 / 100%). The gate needs counts before it opens, so early padded stories answer less. Not investigated.

## Next

1. **Stage 3: hearing itself.** Feed the actor's output back into the stream, tagged as self, starting with echo (paid by match).
2. **Three seeds** of the union with the act path on full qa2 and long, against experiment 42's best. The full qa2 4k test needs them (experiment 45: 25–71% across variants on one seed).
3. **Find the story set's lost 1–2 points:** compare routing at Go words with the mood counter's drop at the same words.
