# 40 · The standard set over three seeds, the store, the question detector, BABILong qa2–qa5 (2026-10-24)

*[All experiments](README.md)* · [← 39](39-one-rule-per-level.md) · [41 →](41-forum-six-first-tests.md)

The owner's go-ahead on the plan after experiment 39:
1. Run the new stack against today's on the standard set, over three seeds.
2. Fix the question detector, and replace the gating settings with one budgeted rule.
3. Run BABILong qa2–qa5, then build two-hop chaining.

**Results:**
- **Where the new stack wins:** read-then-ask (two and three levels).
- **Where it loses:** local questions, owner, long BABILong (it is unstable across seeds), and possibly new vocabulary.
- **The detector fix did not work;** its collapses come from answers that are really wrong.
- **The budget holds the store's size,** but not accuracy at 64k.
- **qa2–qa5 are blocked by the word-level boundary rule,** which learns "the" as a clause end. With that diagnosed, chained hops plus the budget answer 100% of a qa2 test.

## Stacks

- **Today's stack, books:** experiment 37's flags (`runs/exp37/F.flags`) + `--sit-nest --sit-scope --sit-epoch`.
- **Story set:** F + the gating stack (`--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096`), no nest, scope or epoch.
- **BABILong:** F + nest + scope + that stack + `--sit-path-forget 2` (`runs/exp38/bl_split.sh`), no epoch.
- **New stack:** today's + `--sit-levels --sit-area-kinds --sit-area-anti 2 --sit-area-sticky` (experiment 39).

Binary `sit-40a`; job list and worker in `runs/exp40/`. The seed-0 references come from earlier builds of the same code: `R_vv_base` (`sit-39g`), `R_bl_long` (`sit-38y`), `R_vv_aks` and `R_bl_long_ak2s` (`sit-39h`). Seeds change `--sit-seed`, the reader's random codes, not the data. Runs shared 2 cores with other experiments, so times are not comparable.

**The seeds hardly move today's stack on the two-level read-then-ask book** (found 79.0 / 87.6 / 92.3–92.4% on every seed). Elsewhere they matter: on the three-level book, home is 65.0 / 15.0 / 0.0%; on BABILong 16k, 93.3 / 93.3 / 60.0%. A stronger replication would also use data from other generator seeds.

## The standard set

**Three-level read then ask** (`ld2m_vv`; found about facts in the volume being read, and in earlier volumes):

| Found | Today's: seed 0 / 1 / 2 | New: seed 0 / 1 / 2 |
|---|---|---|
| this volume, 64k–512k (565) | 94.0 / 88.1 / 87.3% | 99.8 / 99.8 / 99.8% |
| earlier volume, 64k–512k (355) | 15.8 / 17.7 / 17.5% | 99.2 / 99.2 / 99.2% |
| earlier volume, 512k+ (1,045) | 13.9 / 26.7 / 22.5% | 96.3 / 93.1 / 80.2% |
| home, 8k–64k (20) | 65.0 / 15.0 / 0.0% | 100 / 100 / 100% |

**Two-level read then ask** (`ld2m_pr`, experiment 38's book C):

| Book C | Today's: seed 0 / 1 / 2 | New: seed 0 / 1 / 2 |
|---|---|---|
| found 8k–64k (186) | 79.0 / 79.0 / 79.0% | 100 / 100 / 100% |
| found 64k–512k (1,442) | 87.6 / 87.6 / 87.6% | 100 / 100 / 100% |
| found 512k+ (1,320) | 92.4 / 92.4 / 92.3% | 100 / 100 / 100% |
| home 8k–64k (17) | 41.2% each | 100% each |

**New vocabulary** (`ld2m_ps`: book C with every name, place, object and heading word new):

| Book C′ | Today's: seed 0 / 1 / 2 | New: seed 0 / 1 / 2 |
|---|---|---|
| found 8k–64k (186) | 88.2 / 86.0 / 81.7% | 100 / 100 / 100% |
| found 64k–512k (1,442) | 94.8 / 92.7 / 89.0% | 70.5 / 85.7 / 100% |
| found 512k+ (1,320) | 91.2 / 87.7 / 87.3% | 79.6 / 83.9 / 94.1% |
| home 8k–64k (17) | 64.7 / 64.7 / 70.6% | 100 / 100 / 100% |

**Two books, learning on throughout** (`ld2m_pq`, seed 0 only):

| Book B | Today's | New |
|---|---|---|
| found, all distances | 99.4–100% | 100% |
| home | 99.9–100% | 99.9–100% |
| owner | 99.5–100% | 95.3–100% |
| who has (local) | 98.9% | 73.5% |
| where is the object (local) | 91.3% | 71.0% |

**200k book, from scratch** (`ld200k`):

| | Today's: seed 0 / 1 / 2 | New: seed 0 / 1 / 2 |
|---|---|---|
| who has | 92.4 / 93.3 / 78.8% | 81.0 / 82.9 / 84.4% |
| where is the object | 76.3 / 81.7 / 75.4% | 77.6 / 77.6 / 78.9% |
| owner 8k–64k (763) | 97.9 / 94.8 / 96.2% | 90.8 / 89.6 / 88.3% |
| found 64k+ (159) | 93.7 / 94.3 / 93.1% | 96.9 / 95.6 / 95.0% |

**Story set:**

| Seed | Today's | New |
|---|---|---|
| 0 | 98.5% | 95.3% |
| 1 | 97.3% | 95.8% |
| 2 | 97.5% | 96.1% |

**BABILong long test** (qa1 at 16k, 30 documents; at 64k, 8 documents; frozen, answer-only):

| | Today's: seed 0 / 1 / 2 | New: seed 0 / 1 / 2 |
|---|---|---|
| 16k | 93.3 / 93.3 / 60.0% | 93.3 / 0.0 / 83.3% |
| 64k | 87.5 / 87.5 / 62.5% | 75.0 / 0.0 / 87.5% |

The new stack's seed 0 is `R_bl_long_ak2s` (no `--sit-levels`, which no BABILong question uses). Its seed-1 0% is no predictions at question boundaries. Both stacks swing by seed.

**What it shows:**
- **The new stack wins on reading a book by the index pass and answering at the end,** two and three levels deep:
    - two levels: 100% where today's gets 79.0–92.4%;
    - earlier volumes: 80–99% where today's gets 14–27%;
    - home goes to 100%.
- **On the new-vocabulary book it is mixed:** better at 8k–64k and on home, worse at 64k+ on seeds 0 and 1.
- **The area kinds cost on local questions, owner and the story set.** On two books the 24 people split into kinds of 20 and 4 (who has 73.5%).
- **On long BABILong the new stack is unstable across seeds.**

## Merging matchers

Two matchers that hold one pattern should merge (anti-Hebbian swaps are for patterns that merely overlap). `--sit-area-merge N` merges the matcher with fewer wins into the other once a loser has near-tied the same winner N times.

On the 200k book (seed 0), merge 8:
- **Fixed:** the people back in one kind; who has 94.1% (81.0% without merging, 92.4% for leader clustering); owner 93–100%.
- **Broken:** "live" merged with the objects; found at 64k+ 87.4%. The number words stay spread over several kinds with and without merging. Not adopted.

## The question detector

`--sit-mood-floor` treats a boundary below the question threshold as a question one time in 16, so a collapsed detector keeps receiving evidence.

| BABILong pilot | Without the floor | With it |
|---|---|---|
| area kinds, no anti-Hebbian (test 0k / 4k) | 0 / 0% | 10 / 10% |
| leader kinds | 100 / 100% | 100 / 100% |

- **The floor did not revive the collapsed configurations.** The 10% is the floor's own sampling. The two 128-bit and 64-bit near-tie configurations scored 5 / 6% and 1 / 2%.
- **The floor-sampled answers:**
    - area kinds without anti-Hebbian: all right (10 of 10 at 0k, 5 of 5 at 4k), yet the detector did not climb back;
    - 128 bits: 5 of 10 and 3 of 5;
    - 64 bits: mostly wrong (1 of 10, 1 of 5).
- **Training at 1k:** 84%, 73% and 65%, against 96.5% with leader kinds. So real answer failure explains part of the collapses, not all of them.
- **A variant made things worse.** Counting only answers given in question mode, and not training routes at floor-sampled boundaries, took leader kinds to 5–8 / 0–4% (two versions). It was reverted.

## One budgeted rule for the long-term store

`--sit-lt-budget N`:
- every content binding enters, and the store holds at most N heads;
- a clock hand halves each head's tag as it passes (takes one off for a path credited by a question) and evicts heads at 0, with their histories;
- a rewrite adds one to the tag, and a new head's tag starts at the bit length of its path's count.

It replaces `--sit-lt-credit`, and with `--sit-lt-min` set it keeps the recurrence gate.

| BABILong long (today's stack) | 16k | 64k | Heads at word 940k |
|---|---|---|---|
| gates, no budget (`R_bl_long`) | 93.3% | 87.5% | 1,089,751 |
| budget 131,072, every new head at tag 1 | 83.3% | 62.5% | 131,072 |
| budget, tag from path count | 80.0% | 62.5% | 131,072 |
| budget, tag from path count, recurrence gate kept | 96.7% | 37.5% | 131,072 |

- **The size holds.** The history entries still grow: 706k–772k at word 940k across the three budget runs.
- **With the new stack's kinds plus the budget** (`R_bl_long_aks_bud17`): 86.7% and 50.0%.
- **The 64k documents (about 51k words each) lose facts.** That is 3–7 documents of 8, so the 64k row is noisy.

## BABILong qa2–qa5

The data (`scripts/exp40/fetch_babilong.py`):
- **Training:** RMT-team/babilong-train-5k-samples, 1,000 samples at 0k and 200 at 1k.
- **Test:** RMT-team/babilong, 100 at 0k and 100 at 4k.
- qa3 and qa5 have no 1k training split; their second training part is the 2k split.

Each task is its own stream.

**Today's stack** (training while learning; frozen test read in full, since the answer-only test never scored):

| Task | Training 0k | Training 1k/2k | Test 0k | Test 4k |
|---|---|---|---|---|
| qa2 where is X (2 facts) | 26.9% | 18.0% | 17% | 18% |
| qa3 where was X before Y (3 facts) | 23.6% | 20.0% | 27% | 20% |
| qa4 two-place relations | 33.2% | 21.0% | 28% | 10% |
| qa5 three-place relations | 57.6% | 23.0% | 32% | 21% |

- **In answer-only mode every test scored 0%.**
    - qa2–qa4: the question detector never reached threshold.
    - qa5: it learned "to" as a question boundary and answered there (9,488 predictions), never after "?".
- **The transformer seat's prediction** (qa2 and qa3 at most 40% without chaining) holds.

**Why:**
- **The boundary rule learned "the" as a clause end on these streams** (qa2: "the . , and of"; qa4: "the is of ? , to and in a"; qa5: "the . to ? , and of a"). Clauses end before their place or object, so no fact binds a person to it.
- **qa2's answers follow the newest place.**
- **The old credit gate stopped writing heads for each relation path after that path's first 64 bindings** (counted over all entities). Uncredited paths were never written again, so people's long-term heads were stale (on the small stream their newest head times were before word 16k).

**Chained hops** (`--sit-chain`):
- Hop 1 is one of the cue's newest heads, or (by a reverse index) an entity whose head wrote the cue.
- Hop 2 is one of that entity's heads, either now or as of hop 1's time (from its history), ranked by recency across its paths.
- Route features conjoin the two paths, now-or-then, and both recencies.

**A diagnostic** (`SIT_BND_DENY=2`, "the" may never be a boundary; not a learner), on a small qa2 stream (1,000 plain training samples, 100 test, frozen test read in full):

| qa2 small (`bl_qa2s`) | Training | Test |
|---|---|---|
| today's stack | 25.5% | 33% |
| + `--sit-chain` | 26.8% | 33% |
| + `--sit-chain --sit-lt-budget 131072` | 26.1% | 33% |
| "the" never a boundary | 65.6% | 51% |
| + budget | 62.4% | 75% |
| + chain | 59.9% | 84% |
| **+ chain + budget** | **90.5%** | **100%** |

## A learned replacement for the diagnostic: failed

`--sit-bnd-indep` vetoes a boundary when the kind of the word before predicts the kind of the word after. It uses integer collision counts: two occurrences with the same kind before should not agree on the kind after more often than any two do.

| Test | Result | Learned boundaries |
|---|---|---|
| small qa2 | 14% plain, **99% with chain and budget** | "." only |
| story set | 56.9% (98.5% without) | "." only ("?" vetoed) |
| BABILong pilot | 0 / 0% | "of" only |

- **On the story set "?" is vetoed** because a question really does predict its answer's kind ("where is X ?" → a place).
- **On BABILong "." is vetoed** because the book text and the bAbI sentences make the side before tell the side after.
- **The full qa runs with the veto, chains and the budget were stopped at the 2-hour limit.**
    - They had slowed to 1,486 s (qa2) and 2,144 s (qa4) per 20k words.
    - The cause is mostly the chains, not the veto. Candidates per word grew with the store: 1,153 to 11,080 on qa2, 584 to 16,331 on qa4.
    - The full qa2 run with chains and no veto (`R_qa2_chain`) grew the same way (55 to 17,998 candidates, 13 s to 800 s per 20k words). It gained nothing, because "the" was still a boundary: training 25.5 / 15.5%, test 16 / 18%.
    - Chains need a cap before they scale.
- **The word-level boundary rule is now the bottleneck for multi-fact questions.**

## Open

- **A learned word-level boundary rule** that keeps "." and "?" in book text and rejects "the" in bAbI text.
- **The area kinds' costs:** local questions, owner, the story set, BABILong seed instability, and possibly new vocabulary.
- **The budget's loss on long documents.**
- **qa2–qa5 on the full streams** once boundaries are right.

## Caveats

- **The seeds change only the reader's random codes.**
- **Small samples:** home and owner on read-then-ask (17–20 and 6–7 questions); BABILong 64k (8 documents).
- **The new stack's BABILong seed 0 has no `--sit-levels`;** seeds 1 and 2 do. No BABILong question nests, but that is not tested.
- **The small qa2 runs have no command script.** The flags in the table are as run.
- **Runs shared 2 cores with others,** so times are not comparable.
