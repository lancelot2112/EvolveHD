# 07 · Generative wiring: inline growth against population search (2026-10-12)

*[All experiments](README.md)* · [← 06](06-growth-interfaces.md) · [08 →](08-bootstrapping.md)

**Concepts:** [Screens](../concepts/screens.md) · [Shadow trials](../concepts/shadow-trials.md) · [Growth operators](../concepts/growth-operators.md) · [Shift-bind](../concepts/shift-bind.md)

Question: can the system *generate* its own wiring candidates (not pick from a hand-written menu), find structure that only exists in pairs, and how does that compare with a population search over wirings?

## Setup

**World.** Nine nodes, each emitting one of 8 symbols per tick. Every node is a target that predicts its own next symbol.

| Node | Rule | What it needs |
|---|---|---|
| SL | 85%-reliable view of a hidden slow latent L (dwell ~30) | – |
| A, B, D2 | i.i.d. | nothing helps |
| D1 | irrelevant slow chain (dwell ~30) | – |
| T1 | T1 = map[L], 70% | a single edge from SL |
| T3 | T3 = map'[T1], 85% | an internal edge from T1 |
| T2 | T2 = LatinSquare[A, B], 85% | **A and B together**; each alone carries zero information |
| T4 | T4 = perm_L(T4), 85% | **its own state and SL together** (a context × self conjunction) |

At tick 50k, SL and D1 swap channels, so T1 and T4 must rewire to D1. 100k ticks per lifetime.

**Nodes and edges.** Each node's cells are its symbols, each with a fixed 32-of-2048 code; the matcher layer was validated in earlier sims. A bind node over (X, Y) has 64 cells, one per symbol pair. It is the symbolic stand-in for shift-bind + k-WTA cleanup (audit sim 2). Edges are the same writer masks as before (48 bits for the self tap, 128 for context), using additive popcount voting.

**Candidate generation.** Any node or bind node can be wired into any target whose type matches; there is no hand-written list. Ignoring bind nodes, that is 72 possible edges; counting every possible pair-bind, about 400. Three operators propose candidates:

| Operator | Proposes | Rule |
|---|---|---|
| Screen | Single edges src → dst | Rank by debiased I(dst_t ; src_{t−1} \| dst_{t−1}) over the last 4k ticks (circular-shift null). Trial if > 0.02 bits. |
| Synergy | A new bind node over (X, Y) for a target | Every 5k ticks: I(dst; X,Y) − I(dst; X) − I(dst; Y) > 0.3 bits. Best pair only; up to 8 binds. |
| Conjoin | bind(src, dst), with trial priority | After a src → dst trial is rejected or pruned even though the screen saw > 0.2 bits — information that an additive edge couldn't use. |

**Trials.** Shadow trials as before: 4k burn-in, then 2k ticks of paired ablation; keep if z ≥ 2.5 and Δ ≥ 1%; prune if z < 1 or Δ < 0.3% per 4k block; backoff on rejection. At most one trial per target at a time. Runs are either serial (1 trial at a time globally) or parallel (4).

**Population search.** A (μ+λ) GA with μ = 8, λ = 8 and 15 generations, giving 128 lifetimes of 20k ticks each with fixed wiring. Mutations add or remove an edge from a base node or any of the 36 pair-binds. Fitness = mean accuracy over all 9 nodes for ticks 10–20k, minus 0.002 per edge. 2 GA seeds; 6 seeds for inline.

## Inline results

| config | acc, all nodes, ph1 / ph2 | T1 | T2 | T3 | T4 | structures found by 50k (of 6) | after swap, by 100k | junk keeps | trials per run |
|---|---|---|---|---|---|---|---|---|---|
| I1 random order, serial, no binds | 0.398 / 0.397 | 0.553 | 0.125 | 0.459 | 0.357 | T1 2, T3 1, T2 0, T4 0 | T1 2, T4 0 | 0 | 17 |
| I2 screened, 4 parallel, no binds | 0.428 / 0.428 | 0.631 | 0.124 | 0.648 | 0.358 | T1 6, T3 6, T2 0, T4 0 | T1 6, T4 0 | 1 | 49 |
| I3 screened, serial, synergy + conjoin | 0.537 / 0.554 | 0.536 | 0.867 | 0.870 | 0.479 | T1 0, T3 6, T2 6, T4 5 | T1 5, T4 6 | 1 | 17 |
| **I4 screened, 4 parallel, synergy + conjoin** | **0.567 / 0.552** | **0.661** | **0.870** | **0.867** | **0.626** | **T1 6, T3 6, T2 6, T4 6** | T1 6, T4 4 | 0 | 68 |

The T1–T4 columns are phase-1 accuracy over the last 10k ticks. A "structure" counts as found if the target keeps an edge carrying the right source: T1 ← any source containing SL; T3 ← any source containing T1; T2 ← A*B; T4 ← SL*T4 (D1 and D1*T4 after the swap).

Time to first keep (median, seeds that found it):

| config | T2 ← A*B | T4 ← SL*T4 | T1 ← SL | T3 ← T1 | after swap: T1 ← D1 | after swap: T4 ← D1*T4 |
|---|---|---|---|---|---|---|
| I1 | never | never | 16k (2/6) | 16k (1/6) | 20k (2/6) | never |
| I2 | never | never | 10k | 10k | 10k | never |
| I3 | 16k | 46k (5/6) | never in ph1 | 10k | 38k (5/6) | 20k |
| **I4** | **16k** | **22k** | **10k** | **10k** | **8k** | 20k (4/6) |

## Reference wirings and population search

| | T1 | T2 | T3 | T4 | mean all nodes | cost |
|---|---|---|---|---|---|---|
| self taps only | 0.543 | 0.125 | 0.426 | 0.355 | 0.393 | – |
| hand wiring (one true edge per target) | 0.583 | 0.870 | 0.610 | 0.710 | 0.539 | – |
| GA seed 0, best after 128 lifetimes | 0.541 | 0.121 | 0.866 | 0.349 | 0.441 | 2.56M ticks |
| GA seed 1, best after 128 lifetimes | 0.543 | 0.122 | 0.778 | 0.366 | 0.433 | 2.56M ticks |
| **I4 inline, single lifetime** | 0.661 | 0.870 | 0.867 | 0.626 | **0.567** | 50k ticks to find everything |

The GA fitness curves were 0.412 → 0.437 (seed 0) and 0.394 → 0.429 (seed 1). Both found only T3's structure (via T1 or the D1*T1 bind), never a pair.

## Findings

1. **The grammar now generates its own candidates and finds all four kinds of structure.** With screening, parallel shadow trials and the two bind operators (I4), every structure was found in 6/6 seeds by tick 22k. That includes the two that are invisible to single edges: an A×B pair, and a context × self conjunction. Across all 24 inline runs there were 2 junk keeps (none in I4), both pruned within 4k ticks.
2. **Single-edge growth cannot see pair structure.** With no bind operators (I1, I2), T2 stayed at chance (0.125) and T4 at self-only level in every seed, no matter how good the screen was. Wiring A and B as two separate additive edges also left T2 at 0.141 (one calibration run).
3. **Two different kinds of pair need two different detectors.**
   - **External pair (A × B):** synergy measures it directly (about 2.1 bits); it was grown at 5k and kept at 16k.
   - **Context × self:** the conditional screen sees about 1 bit from SL alone, but the additive SL → T4 edge gains only ~2 points and is rejected or pruned. The gap between "information the screen sees" and "accuracy the edge delivers" is exactly the signal that it needs binding with the target's own state. This is the HTM-style distal-context cell, and it falls out of the conjoin rule.
4. **Screening is what makes inline search scale.** Random order (I1) found T1 in 2/6 seeds and T3 in 1/6 in 50k ticks. The screen found both 6/6 by 10k.
5. **Parallel shadow trials fix the slow rewiring.**
   - Serial with binds (I3): the trial slot was monopolised by bind candidates, so T1 ← SL was never found in phase 1, and T1's rewiring after the swap took a median of 38k ticks.
   - Parallel (I4): T1 rewired in a median of 8k ticks, against 17.5k in the earlier serial test.
   - Still open: T4's rebinding to D1*T4 succeeded in only 4/6 seeds by tick 100k.
6. **Redundant edges act as weights.** I4 beat the hand wiring (0.567 vs 0.539 mean; T3 0.867 vs 0.610) because it kept several edges carrying the same source (T1 and binds containing T1). With equal-weight additive votes, stacking an informative source is the only way to up-weight it against the noisy self tap. A proper per-edge weight (or vote scaling by edge Δ) would make this explicit instead of accidental.
7. **Blind population search is hopeless at this size; informed inline search is not.** 128 GA lifetimes (2.56M ticks) found only the internal edge T3 ← T1, reaching 0.43–0.44 mean accuracy. One inline lifetime found everything in 50k ticks, reaching 0.567. The pair edges are about 1 in 400 per random mutation, and their fitness signal is invisible until both halves are present.

## Caveats

- The GA is deliberately naive: blind mutations, fixed wiring, 20k-tick lifetimes. A GA whose mutations were proposed by the screens would do far better, but it would mostly be borrowing the inline machinery.
- Nodes are symbolic, with clean recognition. The bind node is a stand-in for shift-bind + cleanup, which was validated separately in audit sim 2.
- The thresholds (0.02-bit screen, 0.3-bit synergy, 0.2-bit conjoin, accept/prune rules) were set by hand. Synergy estimates are biased upward by roughly 0.1–0.25 bits at 4k samples (an irrelevant D1*T4 pair scored 0.24 in calibration), which is why the threshold is 0.3.
- Only pairs were tested. Triples would need either a recursive bind (a bind of a bind) or a synergy screen over more variables, whose cost grows quickly.

## Where population search still fits

Not for wiring. Inline search with information screens found structure orders of magnitude faster. What a population can still do is tune the *grammar* that inline search runs on — the screen thresholds, burn-in lengths, number of parallel trials, mask sizes, and bind budgets — because no single lifetime can judge its own thresholds. The fitness for that would be learning speed across a family of worlds like this one.

## Changes to the spec this supports

1. Candidates are generated by type-matching any node to any target, then ranked by a debiased conditional-information screen. Nothing is hand-listed.
2. Run up to N shadow trials in parallel, one per target at a time. Parallelism gave the largest single improvement in rewiring speed.
3. Two bind operators:
   - **synergy**: grow bind(X, Y) when the pair carries more than the sum of the parts;
   - **conjoin**: grow bind(src, target) when the screen sees information that an additive edge can't use.
4. Add an explicit per-edge vote weight, so that stacking redundant edges isn't the only way to emphasise a source.
5. Use population search, if at all, for the grammar's parameters, not for wiring.

## Code

[`reference/python/graph_sim.py`](../../reference/python/graph_sim.py); see [reference code](../reference-code.md) for how to run it.
