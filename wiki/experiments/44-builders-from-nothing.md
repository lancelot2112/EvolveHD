# 44 · Two builders from nothing, each replacing the learned boundary set in every job (2026-10-25)

*[All experiments](README.md)* · [← 43](43-multi-scale-cheap-test.md)

Forum round eight (project doc `claude/forum-round-eight.md`) asked for a unit built from nothing, with experiment 42's judge (or the level above) disposing. This page tests the two builders ranked first:
- **G1**, the recurrent seat's surprise chunks, with its write gate in place of the question drop;
- **G2**, the TEM seat's slot-collision ledger as the boundary proposer.

Experiment 43 changed only the bindings. Here each builder replaces the learned boundary set L in **every** job L did.

**A bug found while testing, now fixed.** A change made during experiment 42's later work (commit `464aaff`, for the surprise proposers) made commits close the clause judge's clauses for every proposer. With k90's proposals this cut clauses at a crossed "the" and kept "the" a stop: experiment 42's best stack fell from 100% to 33% on small qa2. The fix (binary `sit-44i`) restores it to 100%. Affected runs: experiment 42's clean-ledger section (`sit-42f`, `sit-42g`), the first G2 runs and the first write-gate run on experiment 42's stack (35%), all rerun below. Not affected: experiment 42's main judge results (`sit-41p`, `sit-41q`), experiment 43 and every G1 run (no judge).

**Results (one seed each):**
- **G1 fails on every test** (small qa2 39%, story set 40.4%, pilot 14 / 12%, full qa2 19 / 2%, long 13.3% / 2 of 8). Both its falsifiers are hit (small qa2 < 80, story < 93). The chunks themselves come out as the seat predicted. What breaks it:
  - **Level-1 bindings across 8 chunks flood the store.** Entities reach a median of 2,600–3,300 heads (about 113 with level 1 cut to 2 chunks). Even with oracle commits and an oracle question drop, small qa2 is 34% at full depth and 77% with level 1 cut to 2 chunks.
  - **Commits at every chunk close** (about every 2 words): with level 1 cut to 2 chunks, oracle commits at "." lift training accuracy from 25% to 45%.
  - **The write gate's skipping costs facts** (tested on experiment 42's stack below): from the start it takes small qa2 from 100% to 64%, and giving memory examples first does not rescue it.
- **G2 builds a sane set on bAbI text and fails on the qa1 book-text streams.**
  - Small qa2: **100%** with no hand ban. It proposes ". ?", never "the".
  - Story set: 95.8%. It proposes ". ? is", as the TEM seat predicted; the judge crosses "is".
  - Full qa2: **92 / 71%**, against experiment 42's 87 / 34% and the hand ban's 97 / 65%. L fills with book words that never cut anything, but "." and "?" stay stops.
  - Pilot and long BABILong (qa1): **0%.** "?" is never proposed there, so the reader never answers in answer-only mode, and dozens of book words are proposed.
- **The write gate, given examples first, still fails** (small qa2 58–76% with warm-ups of 25k and 51k words; 71% with the gate only in the test). Storing questions is not the harm: with no drop and no gate, small qa2 is 100% and the pilot 100 / 100%. Only the story set needs the drop (94.5% without it, where-object 64%).
- **k90 openness stays the proposer for now.** It is still the only rule that proposes "?" on the qa1 book-text streams. The ledger's 4k result on full qa2 is the best yet, on one seed.

## G1: surprise chunks (`--sit-chunk`, `--sit-chunk-gate`)

**The level rule.**
- A chunk starts at w unless w is the majority successor of the word before: succ_top[u] = w and 2·top > n(u), counted before this pair.
- A chunk that reaches 32 words closes anyway.
- The chunk's head is its first word (the surprising one).

**Every job L did, replaced:**

| Job | Before (L) | G1 |
|---|---|---|
| Binding window | walk back up to 8 words, stopping at binding stops | level 0: every word of the open chunk and the previous one, exact path, no stops; level 1: when w starts a chunk, it binds to the heads of the 7 chunks before the previous one, by the exact path of every word between, marked as level 1 |
| Route features | path key | path key (now carrying the level) plus the level, conjoined with the value's kind (and both hops' levels on chains) |
| Commit | at a word of L | when a chunk closes |
| Drift | at a word of L | when a chunk closes |
| Cue | back to the clause start | the last 3 chunks, at most 12 words |
| Range texts | clauses | chunks |
| Question drop | clause dropped at a question word of L | none: the write gate |
| Question detector (answer-only runs, head credit, open queries) | mood counter on L | contrast detector with no L: memory predicts the word after u at least twice as well as after words in general |

- **The write gate:** when memory's top route already predicted w, the head (or slot) behind it is refreshed (its time becomes now) and w creates no bindings. When nothing behind it holds w now (a range, or a chain's value as of an earlier time), w binds as usual.
- **L is never computed:** the k90 re-pooling is skipped under `--sit-chunk`, and the old per-word rule (`seg_update`) does not write L because the stack carries `--sit-bnd2`. Every run's report shows "learned boundaries:" empty.
- **The stack:** the cap (`--sit-entity-fair`) is kept and there is no halving sketch, as the seat asked. The cap acts only with the budget, so on the story set and the pilot (no budget, as in experiment 42) it does nothing. Otherwise the stacks are experiment 42's: small and full qa2 with chains and `--sit-lt-budget 131072`, long BABILong with the budget.
- **Not built:** the seat's per-level clock, and its later credit step (the level above judging pair-merge flags). Neither was part of its first experiment.

Binaries `sit-44a`–`sit-44g` (the route-level feature from `sit-44e`; `sit-44f`/`sit-44g` add diagnostic switches only, off by default). Script `runs/exp44/g1.sh` (default binary `sit-44g`; small qa2 was run on `sit-44e`, the other tests on `sit-44g`, which behave the same without the switches). Level 1 also stops once the path between passes 32 words.

**The chunks are what the seat measured offline:**

| Stream | Words per chunk | "the" starts a chunk | "." starts a chunk |
|---|---|---|---|
| Small qa2 | 1.87 (seat offline: 1.87) | 10 of 18,324 (offline 10) | 1,932 of 18,324 |
| Story set | 1.82 (offline 1.82) | 180 of 3,002 (offline 180) | 233 of 4,178 |
| Pilot (book text) | 1.12 | 13,739 of 22,909 | 8,654 of 19,043 |

**Results:**

| Test | G1 | Best so far (exp 42: k90 L + length-weighted judge) | Seat's prediction |
|---|---|---|---|
| Small qa2 | **39%** | 100% | ≥ 90 (falsified < 80) |
| Story set | **40.4%** (where-object 27.2%) | 98.5% | ≥ 95 (falsified < 93) |
| Pilot 0k / 4k | **14 / 12%** | 100 / 100% | ≥ 95 / 90 |
| Full qa2 0k / 4k | **19 / 2%** | 87 / 34% (hand ban 97 / 65%) | ≥ 90 / 50 |
| Long 16k / 64k | **13.3% / 2 of 8** | 93.3 / 100% (old L + cap, exp 41) | ≥ 90 / 7 of 8 |

The contrast detector without L picks content words, not "?":
- small qa2: "milk went apple football";
- story set: the eight objects;
- pilot: "sandra went daniel mary ? john".

Answers are poor, so the hit rate after "?" never stands out. The detector depends on memory working first.

### What breaks G1 (diagnostics on small qa2)

The diagnostic switches are environment variables, off in the runs above.

| Run | Small qa2 | Heads per entity at 100k (median over all 26, "went" included) |
|---|---|---|
| G1 before the level feature (log overwritten; numbers from the session) | 28% | 2,040 at 80k |
| G1 before the level feature, no write gate (questions stored, nothing dropped) | 31% | 3,347 (max 4,096) |
| **G1** | **39%** | 2,643 (max 3,670) |
| G1, level 1 limited to 2 chunks before the previous one | 44% | 113 |
| … + oracle commits at "." and oracle drop at "?" (no gate) | **77%** | — |
| G1 at full depth + oracle commits and drop (about half its commits are buffer overflows, at 64 bindings, not at ".") | 34% | 3,296 (max 4,096) |
| G1 at depth 2 + oracle detector ("?" only) | 14% | — |
| G1 at depth 2, no drift at chunk close | 27% | — |
| G1 at depth 2 + oracle commits at "." only (questions stored) | 52% (train 45%) | — |

- **The gate skips facts.** Its refreshes are mostly function words ("." 17,506 of 18,324, "the" 17,100), but it also skips about a third of object mentions ("milk": 668 head and 236 slot refreshes out of 2,483), where the head refreshed may belong to another person. This is the neuroscientist's warning. Turning the gate off (31% against 28%, before the level feature) shows it is not G1's main fault.
- **Commit timing matters.** With level 1 cut to 2 chunks, oracle commits at "." lift training accuracy from 25% to 45% (test 44% → 52%); adding the oracle drop at "?" gives 77% (training 48%), within single-run noise of the commit-only run on training.
- **Level-1 bindings across 8 chunks crowd the ranking,** as the superoptimizer and TEM seats predicted.
  - Bindings into the next sentence are newer than the fact, so they take "newest place" (rank 0).
  - Paths with names in them still recur at least 4 times in bAbI's small vocabulary, so they pass the recurrence gate. The seat counted on that gate to pick the level.
  - Without the gate, lists reach the cap (4,096). Small qa2 runs 900 s against 61 s for experiment 42's best stack.
- **The variants at depth 2 swing from 14% to 52% on the test,** so single diagnostic runs only show direction. The large effect (depth 7 against 2 with oracle commits: 34% against 77%) is clear.

## The write gate, with examples first (experiment 42's stack, small qa2)

The owner asked whether the gate should be learned and given more examples first. `--sit-gate-from T` keeps the question drop until word T, then the gate replaces it (training ends at 102,303). Binary `sit-44i`; diagnostics `SIT_GATE_KEEPDROP` (the gate skips, the drop stays) and `SIT_GATE_NOSKIP` (no drop, the gate never skips) in `sit-44j`.

| Run | Training / test |
|---|---|
| Question drop (exp 42's best) | 85.2 / **100%** |
| Gate from the start | 55.4 / 64% |
| Drop, then gate from word 25,000 | 63.6 / 58% |
| Drop, then gate from word 51,000 | 69.3 / 76% |
| Drop through training, gate only in the test | 85.2 / 71% |
| Gate skipping only, drop kept, from the start | 60.5 / 67% |
| Gate skipping only, drop kept, test only | 85.2 / 71% |
| **No drop, no gate (questions stored) from the start** | **87.9 / 100%** |
| No drop, no gate, test only | 85.2 / 100% |

- **More examples do not rescue the gate.** Even on a fully trained memory, the gate in the test costs 29 points.
- **The harm is the skipping, not storing questions.** "Memory's top route was right" does not mean memory holds that fact for this entity, so a refresh of another head plus a skipped write loses the fact.
- **The drop is needed on the story set only.** With no drop and no gate: small qa2 100%, pilot 100 / 100%, story set 94.5% (where-object 64.2%, against about 90% with the drop). There, stored "where is the milk ?" clauses get in the way.
- **The drop in experiment 42's stack is already learned from examples:** the mood counter treats a word as closing a question only once memory keeps predicting what follows it. A learned write gate would have to be judged by what a write does to later predictions (forum seven's shadow), not by whether memory guessed right this time.

## G2: the collision ledger proposes (`--sit-bnd-collide`)

**The rule** (the TEM seat):
- An open frame holds the content kinds (kinds of 3 or more members) seen since it opened.
- A content word whose kind is already in the frame closes it and opens a new one. This is a reset, charged to the word before.
- A word is proposed when resets follow it at least twice as often as they follow words in general: resets(u)·N ≥ 2·R·n(u), with n(u) ≥ 16 and resets(u) > 0. Counts halve at 2¹⁶ per word and 2²⁴ overall.
- Experiment 42's length-weighted judge disposes as it does k90's proposals (one significant flip per re-pooling). The proposals are L in every job: stops (through the judge), commits, drift, the cue, the question drop and the detector.

Binary `sit-44j` (with the judge fix; runs tagged `g2f`). Earlier runs: `sit-44b` lacked the resets > 0 condition, so for words 2,000–4,000 every word was proposed (runs in `runs/exp44/old/`: small qa2 93%, story set 97.3%); `sit-44d` had the condition but not the judge fix (small qa2 100%, story 95.6%, full qa2 92 / 33%, pilot and long 0%). Script `runs/exp44/g2.sh`.

| Test | G2 | Exp 42 best | TEM's prediction |
|---|---|---|---|
| Small qa2 | **100%** (L ". ?"; "the" never proposed) | 100% | ≥ 95, "the" never proposed |
| Story set | **95.8%** (where-object 88.9%; L ". ? is", "is" crossed) | 98.5% | ≥ 97 (falsified < 95) |
| Pilot 0k / 4k | **0 / 0%** (no answers given) | 100 / 100% | ≥ 96 / 96 |
| Full qa2 0k / 4k | **92 / 71%** | 87 / 34% (hand ban 97 / 65%) | ≥ 90 / 60 |
| Long 16k / 64k | **0 / 0%** (no answers given) | 93.3 / 100% (old L + cap) | ≥ 90 / 7 of 8 |

- **On bAbI text the ledger builds the set k90 builds, minus "the".**
  - Small qa2: "." loses 476 / 539 for crossing, and "?" loses 35 / 338.
  - Story set: "." and "?" stay stops; "is" is crossed (flipping it back loses 1 / 298).
- **On book text it proposes dozens of words** (pilot: "is expect number soon corner afraid toward near …"; long: "from approached number of soon into …"; full qa2: "expression explanation sacrament minister …").
- **On the qa1 streams (pilot, long) it never proposes "?"** (on small qa2, the story set and full qa2, "?" is proposed and stays a stop). In "where is mary ? kitchen", the frame resets at "mary" (a second person), which is charged to "is", so the answer finds no collision. With no "?" in L, the detector finds nothing and answer-only runs give no answers. The TEM seat's offline check predicted this ("'.' yes, '?' no" on the pilot with approximate kinds).
- **On full qa2, "." and "?" stay stops** ("." loses 144 / 632 for crossing) and the book words in L never stop anything long enough to be judged (0 / 0). The 4k test reaches 71%, the best so far; one seed.
- **The story set moved between 95.6% and 97.3% across the three binaries,** so one early difference moves a single run by about 2 points.

## What this means

- **Neither builder replaces k90 openness yet.**
  - G2's ledger is a sane builder on event text and keeps "the" out without a judge. On book text it has no restraint, and on qa1 it misses the question mark.
  - G1's chunks segment as predicted, but the design around them (gate, far level-1 bindings, a detector that needs working memory) fails.
- **The write gate is the wrong tool.** Its skip loses facts, examples first or not. Stored questions only hurt the story set's where-object questions, and there the learned drop already works.
- **Level-1 bindings must be selective.** Eight chunks by exact path admits every cross-sentence relation. The TEM seat's alternative (bind frames only through a shared entity) is the untested option.
- **Single seed throughout.**

## Next

1. **Combine the two proposers:** the ledger's proposals plus k90's (a union, with the judge disposing), so "?" survives on qa1 book text and the full qa2 4k gain holds. Three seeds.
2. **If a write gate is retried, judge each write by its shadow** (what storing it does to later predictions on the same words), not by memory's guess.
3. **If chunks are retried, bind level 1 only through a shared entity** (TEM's G5) rather than by text distance.
