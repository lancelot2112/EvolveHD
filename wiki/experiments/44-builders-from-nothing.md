# 44 · Two builders from nothing, each replacing the learned boundary set in every job (2026-10-25)

*[All experiments](README.md)* · [← 43](43-multi-scale-cheap-test.md)

Forum round eight (project doc `claude/forum-round-eight.md`) asked for a unit built from nothing, with experiment 42's judge (or the level above) disposing. This page tests the two builders ranked first:
- **G1**, the recurrent seat's surprise chunks, with its write gate in place of the question drop;
- **G2**, the TEM seat's slot-collision ledger as the boundary proposer.

Experiment 43 changed only the bindings. Here each builder replaces the learned boundary set L in **every** job L did.

**Results (one seed each):**
- **G1 fails on every test** (small qa2 39%, story set 40.4%, pilot 14 / 12%, full qa2 19 / 2%, long 13.3% / 2 of 8). Both its falsifiers are hit (small qa2 < 80, story < 93). The chunks themselves come out as the seat predicted. Two parts break it:
  - **The write gate does not replace the question drop.** On experiment 42's best stack, swapping the drop for the gate takes small qa2 from 100% to 35%. With questions stored, the top routes at test answers start from "?" and "where" as entities (a trace is saved in the session scratchpad, `qdbg.txt`).
  - **Level-1 bindings across 8 chunks flood the store.** Entities reach a median of 2,600–3,300 heads (about 113 with level 1 cut to 2 chunks). Even with an oracle question drop, small qa2 is 34% at full depth and 77% with level 1 cut to 2 chunks.
- **G2 builds a sane set on bAbI text and fails on the qa1 book-text streams.**
  - Small qa2: **100%** with no hand ban. It proposes ". ?", never "the".
  - Story set: 95.6%. It proposes ". ? is", as the TEM seat predicted.
  - Pilot and long BABILong (qa1): **0%.** "?" is never proposed there, so the reader never answers in answer-only mode, and dozens of book words are proposed.
  - Full qa2: 92 / 33%, about experiment 42's 87 / 34%, but L fills with book words and the judge crosses ".".
- **k90 openness stays the proposer.** It is still the only rule that proposes a usable set on book text.

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
| **Recurrent E2: exp 42's best stack, write gate in place of the drop** | **35%** ("the" stays a stop; no question word found) | — |

- **The write gate fails as a replacement for the question drop.**
  - At an answer the gate stores the question's bindings whenever memory was wrong, which early on is almost always. Memory then fills with question bindings, it stays wrong, and the loop feeds itself.
  - At one test answer, "where is the football ? kitchen", the top routes ran football → milk → garden, ? → mary → bedroom and where → john → hallway.
  - The gate's refreshes are mostly function words: "." 17,506 of 18,324, "the" 17,100. But it also skips about a third of object mentions ("milk": 668 head and 236 slot refreshes out of 2,483), where the head refreshed may belong to another person. This is the neuroscientist's warning.
- **Level-1 bindings across 8 chunks crowd the ranking,** as the superoptimizer and TEM seats predicted.
  - Bindings into the next sentence are newer than the fact, so they take "newest place" (rank 0).
  - Paths with names in them still recur at least 4 times in bAbI's small vocabulary, so they pass the recurrence gate. The seat counted on that gate to pick the level.
  - Without the gate, lists reach the cap (4,096). Small qa2 runs 900 s against 61 s for experiment 42's best stack.
- **The variants at depth 2 swing from 14% to 44%,** so single diagnostic runs only show direction while questions are stored. The two large effects (gate against drop, depth 7 against 2) hold across runs.

## G2: the collision ledger proposes (`--sit-bnd-collide`)

**The rule** (the TEM seat):
- An open frame holds the content kinds (kinds of 3 or more members) seen since it opened.
- A content word whose kind is already in the frame closes it and opens a new one. This is a reset, charged to the word before.
- A word is proposed when resets follow it at least twice as often as they follow words in general: resets(u)·N ≥ 2·R·n(u), with n(u) ≥ 16 and resets(u) > 0. Counts halve at 2¹⁶ per word and 2²⁴ overall.
- Experiment 42's length-weighted judge disposes as it does k90's proposals (one significant flip per re-pooling). The proposals are L in every job: stops (through the judge), commits, drift, the cue, the question drop and the detector.

Binary `sit-44d` (`sit-44b` lacked the resets > 0 condition, so for words 2,000–4,000 every word was proposed; its runs are in `runs/exp44/old/`: small qa2 93%, story set 97.3%). Script `runs/exp44/g2.sh`.

| Test | G2 | Exp 42 best | TEM's prediction |
|---|---|---|---|
| Small qa2 | **100%** (L ". ?"; "the" never proposed) | 100% | ≥ 95, "the" never proposed |
| Story set | **95.6%** (where-object 82.7%; L ". ? is") | 98.5% | ≥ 97 (falsified < 95) |
| Pilot 0k / 4k | **0 / 0%** (no answers given) | 100 / 100% | ≥ 96 / 96 |
| Full qa2 0k / 4k | **92 / 33%** | 87 / 34% | ≥ 90 / 60 |
| Long 16k / 64k | **0 / 0%** (no answers given) | 93.3 / 100% (old L + cap) | ≥ 90 / 7 of 8 |

- **On bAbI text the ledger builds the set k90 builds, minus "the".**
  - Small qa2: "." loses 56 / 744 for crossing, and "?" loses 36 / 340.
  - Story set: ". ? is" all stay stops.
- **On book text it proposes dozens of words** (pilot: "is expect number soon corner afraid toward near …"; long: "from approached number of soon into …"; full qa2: "expression explanation sacrament minister …").
- **On the qa1 streams (pilot, long) it never proposes "?"** (on small qa2, the story set and full qa2, "?" is proposed and stays a stop). In "where is mary ? kitchen", the frame resets at "mary" (a second person), which is charged to "is", so the answer finds no collision. With no "?" in L, the detector finds nothing and answer-only runs give no answers. The TEM seat's offline check predicted this ("'.' yes, '?' no" on the pilot with approximate kinds).
- **On full qa2 the judge crosses "."** (at word 292k, in book text), and the 4k test stays at experiment 42's level.
- **The story set is 95.6% with the fix and 97.3% without it,** so one early difference moves a single run by about 2 points.

## What this means

- **Neither builder replaces k90 openness yet.**
  - G2's ledger is a sane builder on event text and keeps "the" out without a judge. On book text it has no restraint, and on qa1 it misses the question mark.
  - G1's chunks segment as predicted, but the design around them (gate, far level-1 bindings, a detector that needs working memory) fails.
- **Questions must not be stored as facts.** The write gate cannot replace the question drop. Answering needs a question signal that does not come from memory's own success, because early on memory is not succeeding.
- **Level-1 bindings must be selective.** Eight chunks by exact path admits every cross-sentence relation. The TEM seat's alternative (bind frames only through a shared entity) is the untested option.
- **Single seed throughout.**

## Next

1. **Combine the two that work:** k90 openness and G2's ledger together (a word proposed by both, or the ledger restricted to k90's proposals). This could keep "?" on book text while keeping "the" out on bAbI.
2. **Make a question signal that doesn't depend on memory working first,** for example the TEM seat's form features within L, before retrying any write gate.
3. **If chunks are retried, bind level 1 only through a shared entity** (TEM's G5) rather than by text distance.
