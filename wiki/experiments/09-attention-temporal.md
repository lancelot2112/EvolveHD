# 09 · Temporal moves and learning-progress attention (2026-10-12)

*[All experiments](README.md)* · [← 08](08-bootstrapping.md) · [10 →](10-full-primitives.md)

**Concepts:** [Time: delays and holds](../concepts/time-delays-and-holds.md) · [Attention](../concepts/attention.md) · [Screens](../concepts/screens.md)

Two questions:
1. Can the grammar grow structure for puzzles that need looking back over time?
2. Can a Select loop decide where to spend a limited learning budget, so the system picks its own stepping stones?

## 1. Temporal moves

**World.** X and D2 are i.i.d. 8-symbol streams. F is a rare flag (5% of ticks). K shows a new value V when F fires, noise otherwise.
- **Td** = map[X eight ticks ago], 85% of the time.
- **Q** = map[V] 30% of the time, noise otherwise. It needs the value K showed at the last flag, which is on average 20 ticks back.

Ceilings: Td 0.869, Q 0.388; chance 0.125.

**New moves**, screened virtually: the candidate's series is computed from stored history, and a node is built only if it passes.

| Move | What it is | Candidates |
|---|---|---|
| delay(X, d) | X as it was d ticks ago | taps d = 2, 4, 8, 16 |
| hold(X \| Y = v) | X latched at every rare event Y = v and held until the next one | only symbol values seen on 0.5–10% of ticks count as events |

hold is an event-gated working-memory slot: the Gate primitive plus persistence.

Results over 60k ticks, 6 seeds:

| config | Td | Q | grown |
|---|---|---|---|
| no temporal moves | 0.126 (chance) | 0.180 | nothing |
| delay taps only | 0.870 | 0.179 | X@8 (5k), kept 11k |
| delay taps + event hold | 0.869 | **0.387** | X@8 (5k), then hold(K \| F=1) (10k), kept 16k |

Both reach ceiling in 6/6 seeds. The screen picked the right tap out of four (2.05 bits for X@8) and the right latch out of every rare-event candidate (0.29 bits). Q's small lift without a hold (0.18) comes from the self tap's fast recency, not from memory.

## 2. Select: where to spend the budget

Four stations share one budget of 1000-tick blocks. A station's clock only runs while it is attended. A policy chooses which station gets each block.

| Station | What's there | Its own ticks needed to solve everything |
|---|---|---|
| easy | two single-edge puzzles | ~10k |
| staged | T2 → T6 → T8 bootstrapping chain | ~33k |
| temporal | Td and Q above | ~16k |
| noise TV | looks structured, nothing learnable | never solvable |

Policies:

| Policy | Rule |
|---|---|
| round-robin | equal time to every station |
| surprise | go where error is highest |
| progress | go where accuracy changed most recently |
| progress+ | progress only when bigger than sampling noise, plus a bonus for structure grown recently, plus a small exploration bonus |
| opportunity | progress+ plus the bits the station's own screens see that no kept edge has captured yet, on still-unsolved targets |

Score is the mean normalised accuracy on the last block over the 7 learnable targets (0 = chance, 1 = ceiling). 6 seeds each.

| budget | policy | score | targets solved (of 7) | T8 | blocks: easy / staged / temporal / noise TV |
|---|---|---|---|---|---|
| 60 | surprise | 0.38 ±0.13 | 2.5 | 0.0 | 11 / 12 / 4 / **34** |
| 60 | progress | 0.54 ±0.14 | 3.7 | 0.16 | 13 / 18 / 16 / 13 |
| 60 | progress+ | 0.68 ±0.10 | 4.7 | 0.18 | 16 / 20 / 14 / 9 |
| 60 | round-robin | 0.79 ±0.07 | 5.5 | 0.0 | 15 / 15 / 15 / 15 |
| 60 | **opportunity** | **0.85 ±0.05** | **5.8** | 0.34 | 16 / **24** / 15 / **6** |
| 100 | surprise | 0.50 ±0.07 | 3.3 | 0.01 | 13 / 15 / 6 / **67** |
| 100 | progress | 0.87 ±0.16 | 6.2 | 0.48 | 23 / 26 / 32 / 19 |
| 100 | round-robin | 0.90 ±0.07 | 6.3 | 0.33 | 25 / 25 / 25 / 25 |
| 100 | progress+ | 1.00 ±0.01 | 7.0 | 1.00 | 22 / 34 / 23 / 21 |
| 100 | **opportunity** | **1.01 ±0.01** | **7.0** | **1.01** | 20 / **38** / 23 / 19 |
| 140 | round-robin | 1.00 | 7.0 | 1.00 | 35 each |
| 140 | progress+ | 1.00 | 7.0 | 1.00 | 35 / 38 / 34 / 34 |

## Findings

1. **Looking back over time needs its own moves, and they fit the same grammar.** A delay tap and an event-gated hold are just two more node types. They are proposed by a virtual screen over stored history, then go through the same shadow trial as everything else. Without them the temporal targets stay at chance, whatever else the grammar can do.
2. **Chasing surprise is the classic failure.** It spends two-thirds of the budget on the noise TV and is the worst policy at every budget.
3. **Naive learning progress is worse than doing nothing clever.** Once stations plateau, the change in accuracy is pure sampling noise, so the policy wanders. It also starves the staged station during incubation: before T8's bind appears, nothing visible improves.
4. **Two fixes make progress-driven attention work.**
   - Count progress only when it exceeds sampling noise.
   - Credit the grammar's own activity: structure grown, and information the screens can see but haven't captured yet.

   With both (the opportunity policy), the system put 38% of its budget on the staged chain and cut the noise TV to 6 of 60 blocks. It solved all 7 targets in 6/6 seeds with 100 blocks, against round-robin's 2/6 for T8. At the tight 60-block budget it beat round-robin, 0.85 vs 0.79.
5. **Most noise-TV time came late.** At budget 100 the opportunity policy gave the noise TV 16–21 blocks per run: 3–4 in the warm-up sweep, and 9–16 in the last 30 blocks, once the learnable stations had little left to capture. A stronger give-up rule (zero opportunity for a long stretch) would cut that further.
6. **The useful attention signal is internal.** The screens already estimate how much information is still uncaptured, and that is the signal Select should use as reward, not external error. This closes the loop: the grammar's screens tell Select where to look, and Select feeds the grammar the data it needs.

## Changes to the spec this supports

1. Add the temporal moves delay(X, d) and hold(X | rare event) to the grammar, and screen them virtually from history before building them.
2. Select's reward = significant learning progress + structure grown + screen opportunity. Never raw surprise.
3. Stations or inputs that are unattended keep their state, so attention can move without losing what was learned.

## Code

[`reference/python/curio_sim.py`](../../reference/python/curio_sim.py); see [reference code](../reference-code.md) for how to run it.
