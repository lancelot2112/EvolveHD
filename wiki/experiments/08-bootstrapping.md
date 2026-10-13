# 08 · Bootstrapping depth and per-edge weights (2026-10-12)

*[All experiments](README.md)* · [← 07](07-graph-growth.md) · [09 →](09-attention-temporal.md)

**Concepts:** [Stepping stones](../concepts/stepping-stones.md) · [Growth operators](../concepts/growth-operators.md) · [Writer-mask readout](../concepts/writer-mask-readout.md)

Question: does solving an easy puzzle build parts the system can reuse for a harder one, and what does the environment have to provide for that to keep happening?

## Setup

**Staged-depth world.** A, B, C and E are i.i.d. 8-symbol inputs, D1 is an irrelevant slow chain and D2 is noise. Three puzzle nodes, each with 85% rule-following and all inputs at t−1:

| Node | Rule | What solving it requires |
|---|---|---|
| T2 | LS1[A, B] | a pair |
| T6 | LS2[LS1[A, B], C] | T2's hidden intermediate plus C |
| T8 | LS3[LS2[LS1[A, B], C], E] | T6's hidden intermediate plus E |

**No-stepping-stone world.** The same, but T2 and T6 are replaced by noise, so T8 is a bare 4-input puzzle with no easier puzzle that rewards building any of its parts.

**Grammar.** The graph-sim controller: screened candidates, 4 parallel shadow trials, synergy and conjoin binds. Binds may now take any node as a part, including other binds and distilled nodes, up to 512 cells. A new **distill** operator (Compress): when a bind → T edge is kept with Δ ≥ 0.1, add an 8-cell node Z[T] whose state is that edge's own prediction of T. This compresses the bind down to the variable T cares about, which then becomes an ordinary source.

6 seeds, 120k ticks.

## Results

| config | T2 | T6 | T8 | levels solved (of 3) | T8 solved by |
|---|---|---|---|---|---|
| staged, binds only | 0.862 | 0.868 | 0.124 (chance) | 2 in 6/6 | never |
| **staged, binds + distill** | **0.867** | **0.869** | **0.865** | **3 in 6/6** | bind(E, Z[T6]) at ~32k |
| staged, binds + distill + weights | 0.870 | 0.869 | 0.868 | 3 in 6/6 | same |
| no stepping stones, binds + distill | 0.127 | 0.123 | 0.124 | 0 in 6/6 | never (no bind ever grown) |

Accuracy is over the last 10k ticks; the ceiling is 0.869.

- Each level was solved in turn, in every seed:
  - T2 at 12k, via bind(A, B).
  - T6 at 18–20k, via bind(C, A*B), a 512-cell bind of a bind. The synergy screen could see this pair only because A*B already existed for T2.
  - T8 at 32–34k, via bind(E, Z[T6]), a 64-cell bind over the distilled level-2 variable.
- Without distill, level 3 stalls. Its pair would be bind(E, C*(A*B)) = 4096 cells, over the size cap, and too large to estimate from a few thousand ticks anyway.
- With no stepping stones, nothing is ever grown. Every pair carries zero synergy about T8, so no screen fires. A 4-way screen would need tables of about 32k cells, far beyond what tens of thousands of ticks can estimate.

## Per-edge vote weights (graph world, 6 seeds)

Each kept edge votes with integer weight 1 + round(Δ / 0.05), capped at 4, updated at each prune check.

| config | mean acc, all nodes: phase 1 / phase 2 | T4 (phase 1) | kept edges at end |
|---|---|---|---|
| I4 (weight 1) | 0.567 / 0.552 | 0.626 | 7.2 |
| I4 + weights | **0.581** / 0.544 | **0.759** | **3.7** |
| hand wiring, weight 1 / 2 / 3 | 0.539 / 0.579 / 0.582 | 0.710 / 0.770 / 0.773 | 4 |

- Weights replaced redundant stacking: the same or better accuracy with half the edges. T4 gained 13 points because its single bind edge can now outvote the noisy self tap.
- Phase 2 was slightly lower with weights (0.544 vs 0.552). A heavy edge that has gone stale is a louder wrong voter until it is pruned.

## What this says about bootstrapping

1. **The system climbs only where the world leaves stepping stones.** Every level was found because the level below had already built the part it needed (A*B, then Z[T6]). Remove the easier puzzles and the same grammar finds nothing.
2. **The cycle that keeps climbing is expand → associate → compress.** Bind builds the conjunction, the Prime edge learns what it predicts, and distill squeezes it back to a small variable. Without the compress step, representations grow multiplicatively and stall after two levels.
3. **The curriculum doesn't need to be ordered in time.** All three puzzles were present from tick 0; the system solved them in depth order by itself, because only the shallowest unsolved one shows any signal to the screens.
4. **So the environment definition is a set of information sources with compositional structure at graded depth**, where intermediate variables are themselves useful to predict. Real data has this naturally (bytes → letters → words → phrases; edges → parts → objects). Structure with no partial signal (parity, cryptographic mixing) is invisible to any local learner and needs either a teacher that exposes the intermediates or brute-force search.
5. The next step is for the system to choose its own stepping stones: a Select (BG) loop that directs attention or action toward the sources where learning progress is fastest.

## Code

[`reference/python/boot_sim.py`](../../reference/python/boot_sim.py); see [reference code](../reference-code.md) for how to run it.
