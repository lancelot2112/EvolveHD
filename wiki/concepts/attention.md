# Attention

With many inputs and a fixed budget, the system has to choose **where the next block of learning time goes.** That choice is made by [Select](go-nogo-selection.md), and its reward comes from the grammar's own bookkeeping: progress, structure grown, and information the [screens](screens.md) can see but no edge has captured.

## Score per input, each block

- **Significant progress:** the change in accuracy over the last k visits against the k before, minus two standard errors, floored at zero.
- **Structure grown:** 0.02 per keep, bind, distill, tap, hold or granule layer in the last 4 visits.
- **Opportunity:** 0.05 × the largest uncaptured screen value on an unsolved target, minus 0.05. Add 0.2 while a granule readout is still climbing, or while a Select trial with real information is running.
- **Exploration:** a small bonus that shrinks with visits.

## Why

**Four stations** (easy, staged chain, temporal, noise TV) ([09](../experiments/09-attention-temporal.md)). Score from 0 (chance) to 1 (ceiling):

| Policy | 60 blocks | 100 blocks | Noise-TV share |
|---|---|---|---|
| Chase surprise | 0.38 | 0.50 | about two thirds |
| Naive learning progress | 0.54 | 0.87 | about a fifth |
| Round-robin | 0.79 | 0.90 | a quarter |
| Opportunity | **0.85** | **1.01** | 10% at 60 blocks |

- **Chasing surprise** is the noise-TV trap.
- **Naive progress** is mostly sampling noise once stations plateau. It also abandons a station during incubation, before its next structure appears.

**Seven stations, 200 blocks** ([10](../experiments/10-full-primitives.md)): opportunity 0.966 against round-robin 0.932. That held only once wasted Select trials stopped counting as opportunity. The slow cerebellar station then got the most time of any station and beat round-robin there (0.70 vs 0.57).

## Give-up

An input with no sign of life for 6 visits goes dormant for 10 blocks, then 20, 40, up to 80. A pending fallback (a dry streak building toward a granule layer, or a young granule trial) counts as alive.
- Without that clause, give-up abandoned the cerebellar station before it could start (0.52 → 0.15).
- With it, give-up never fired in 200 blocks. Telling hopeless from slow costs about 35k ticks per input ([10](../experiments/10-full-primitives.md)).

## Related

[Go/NoGo selection](go-nogo-selection.md) · [Broadcast](broadcast.md) · [Screens](screens.md) · [Stepping stones](stepping-stones.md) · [Capacity and budget](capacity-and-budget.md)

## Code

`run_select` in `curio_sim.py`; `run_budget`, `opportunity` and `pending_fallback` in `prims_sim.py`.

## Evidence in

[02 · bg selection bandit](../experiments/02-bg-selection-bandit.md) · [09 · attention temporal](../experiments/09-attention-temporal.md) · [10 · full primitives](../experiments/10-full-primitives.md)

*[All concepts](README.md) · [Wiki home](../README.md)*
