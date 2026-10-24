# 41 · Forum round six, first tests: a halving sketch, split boundary roles, boundaries judged by use (2026-10-24)

*[All experiments](README.md)* · [← 40](40-standard-set-and-qa2.md)

Forum round six reviewed the equations against the code. It proposed an ordered next iteration, E0 to E6 (project doc `claude/forum-round-six.md`). This experiment runs the first three steps.

**Results** (all single runs at seed 0):
- **E1, the halving sketch, partly holds.**
  - On the full qa2 stream with chains, long-term heads at word 520k fall from 147,847 to 25,698, and the run takes 1,449 s instead of 5,724 s. Accuracy is unchanged.
  - Candidates per word still grow, to 3,124 by the end (16,945 before). A few hub entities collect hundreds of heads each (the largest list reaches 970). Which words they are was not logged in this run.
  - It costs nothing on the story set or the BABILong pilot.
- **E0, splitting the boundary set's jobs, is falsified.** Small qa2 rises from 33% to 74% without the hand ban. But the story set falls to 53.5% and the pilot to 2 / 4%, because the veto also removes "?" and "." as binding stops.
- **E2, boundaries judged by use, fails with both judges tried.**
  - The forum's rule is biased toward cutting: "the" scored span 1, cut 543.
  - A symmetric rank judge is biased toward crossing: it drops every stop on small qa2 (13%).
  - The 1-in-8 exploration crossings themselves cost accuracy, because they are written into long-term memory.

## Stacks

- **qa2 and small qa2:** the BABILong stack (experiment 40), plus `--sit-chain`. Small qa2 (`bl_qa2s`: 1,000 plain training samples, 100 test) also has `--sit-lt-budget 131072`. The test is frozen from word 102,303 and read in full.
- **Story set and BABILong pilot:** as in experiment 40 (`runs/exp36/rq30.sh`, `runs/exp38/bl_split.sh bl_pilot`).

Binaries `sit-41a` to `sit-41f` (each adds a flag; earlier flags behave the same). Scripts: `runs/exp41/e1.sh`, `e0.sh`, `e2.sh`, `e2r.sh`. Both cores were shared between two runs at a time.

## E1: the halving sketch (`--sit-sketch-decay`)

Every 2¹⁶ increments, each counter of the path sketch halves. A path then has to recur recently to pass the recurrence gate. The run also logs heads per entity (`HEADS_PER_ENTITY`). The five largest lists were added to that line later (from `sit-41c`), so only the small-qa2 runs below show them.

Full qa2 stream (525,866 words; frozen from word 220,618), chains, no budget:

| | Before (`R_qa2_chain`, exp 40) | Halving sketch | Halving sketch + budget |
|---|---|---|---|
| Run time | 5,724 s | 1,449 s | 1,607 s |
| Heads at word 520k | 147,847 | 25,698 | 40,033 |
| Candidates per word, mean | 4,093 | 1,448 | 1,676 |
| Candidates per word at 520k | 16,945 | 3,124 | 3,763 |
| Time per 20k words at 520k | 732 s | 108 s | 121 s |
| Test 0k / 4k | 16 / 18% | 17 / 16% | 27 / 25% |

The forum's predictions:

| Prediction | Result |
|---|---|
| Heads at 520k ≤ 30k | **Held** (25,698) |
| Accuracy within ±2 points | **Held** |
| Candidates per word ≤ 150 | **Failed** (3,124) |
| Time ≤ 30 s per 20k words | **Failed** (108 s) |

The budgeted run's 27 / 25% (against 16 / 18% without the budget) is one seed.

Heads per entity, halving sketch, no budget:

| Word | Entities | Median | 90th percentile | Largest |
|---|---|---|---|---|
| 140k | 550 | 1 | 3 | 85 |
| 300k | 3,099 | 2 | 3 | 439 |
| 520k | 6,106 | 1 | 3 | 970 |

**What this means:**
- **Most entities stay at 1–3 heads.** The total grows mostly because book text keeps introducing new entities.
- **The largest lists grow linearly.** Which entities own them on the full stream was not logged. On small qa2 they depend on the stops:
    - with stops "the ." (E2, forum judge), the largest are apple, football, milk, sandra and kitchen, at 129–148 heads;
    - with no stops at all (E2, rank judge), they are the four people (921–974 heads) and "went".
    - A head is one per (entity, exact path). Exact paths through up to 8 words are close to unique, so an entity that appears in many different contexts keeps collecting new heads.
- **The chain walks those lists, so its cost tracks the hubs.** The fix belongs in hop 2's fan-out, or in what makes a path count as the same relation, not in the sketch.

**The halving sketch costs nothing elsewhere:** story set 98.5% (as before), pilot 100 / 100%.

## E0: split the boundary set's jobs (`--sit-bind-indep`)

L, the learned boundary set, still decides commits, drift and the detector. Only L's independent words stop the binding and cue walks. The question drop uses `is_q`, and floor samples update the mood counter after the freeze. Run with `--sit-mood-floor`.

| Test | Base | E0 | Binding stops (E0) |
|---|---|---|---|
| Small qa2, chain + budget | 33% | **74%** | "." |
| + `--sit-bind-drift` (drift only at stops; run by hand, no script) | | 77% | "." |
| Story set | 98.5% | **53.5%** | "." ("?" vetoed) |
| BABILong pilot | 100 / 100% | **2 / 4%** | "of" (". ? , a and" vetoed) |

- **The forum set a falsification bar of story set < 90%, and E0 lands at 53.5%.**
- **Binding across "?" and "." does real damage.** So the veto is the wrong judge even for binding, not only in its wiring.
- **Some of the gap to the hand ban's 100% remains unexplained.** Drift at "the" is not the cause (77% with `--sit-bind-drift`).

## E2: boundaries judged by use (`--sit-bnd-use`)

L proposes. On a hash coin (one occurrence in 8), the walk does the opposite of the current stop decision. Every exact path that crosses a proposed word records the first one it crosses, u. At each prediction the bigram did not already make, the routes' path keys decide.

**The forum's judge:**
- If the top route crosses u, was right, and no route that does not cross u held the word: span(u) += 1.
- If the top route crosses u, was wrong, and a non-crossing route held the word: cut(u) += 1.
- u stops the walk while span ≤ cut. Both halve when span + cut reaches 1,024.

**The rank judge (`--sit-bnd-use-rank`):**
- If the best-ranked holder of the right word crosses u: span(u) += 1.
- If a route crossing some other proposed word u outranks it: cut(u) += 1.

| Test | Forum judge | Rank judge | Halving sketch only |
|---|---|---|---|
| Small qa2 | 33% ("the" span 1 / cut 543; stops "the .") | 13% (the 647 / 273, . 482 / 167; no stops) | — |
| Story set | 93.5% (stops ". ?") | 90.1% (stops ". ?") | 98.5% |
| BABILong pilot | 52 / 44% (all stops kept) | 100 / 72% ("?" stops no more: 498 / 372) | 100 / 100% |

**Why it fails:**
- **The forum's judge is biased toward cutting.** Span and cut test the same condition, "a route that does not cross u holds the word", once negated. With hundreds of routes proposing, that condition almost always holds, so span almost never counts.
- **The rank judge is biased the other way.** Crossing paths are more numerous and more varied, so the best holder crosses u often, whatever u is.
- **Exploration is not free.** The 1-in-8 crossing bindings enter long-term heads. That costs 5 points on the story set and up to 56 on the pilot even when every stop is kept. The forum meant them to stay in working memory, but working-memory routes carry classes, not exact paths, so they cannot be attributed.

**The judge needs a normalized comparison.** It should compare credit per binding written, crossing against within, as the superoptimizer and computational-neuroscientist seats wrote it: across · n_within ≤ within · n_across. Exploration bindings should stay out of long-term memory.

## Next

1. **The hub fan-out:** a learned hop-2 table (credit on first relation → second relation), or a cap derived from the budget, measured on the full qa2 stream.
2. **A normalized use judge,** with exploration kept out of long-term memory. Bar: "the" leaves L on small qa2 while the story set stays ≥ 97% and the pilot ≥ 95 / 95%.
3. **The detector (E3) and the budget (E5) are untested.**
