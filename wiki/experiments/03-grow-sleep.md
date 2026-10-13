# 03 · Grow on novelty, compress in sleep (2026-10-11)

*[All experiments](README.md)* · [← 02](02-bg-selection-bandit.md) · [04 →](04-audit-sims.md)

**Concepts:** [Sleep and consolidation](../concepts/sleep-and-consolidation.md) · [Matcher pools](../concepts/matcher-pools.md) · [Credit without backprop](../concepts/credit-without-backprop.md) · [Capacity and budget](../concepts/capacity-and-budget.md)

**Question.** Can an area recruit cells for new patterns while awake and compress them in sleep, without forgetting old patterns or exceeding a fixed capacity?

## Setup

- **Area:** D = 2048 inputs (64 active). Masks of K = 32 bits; a cell fires at popcount ≥ 12; k-WTA with k = 1. A new cell is recruited when the best overlap is below 18, with its mask sampled from the input.
- **Data:** 30 concepts in sibling pairs sharing 40 of 64 bits. Each instance keeps 80% of its prototype's bits, plus noise.
- **Schedule:** three phases of 1,500 trials, each introducing 10 new concepts; old concepts are never shown again.
- **Scoring:** a frozen probe after each phase. 5 instances per concept label the winners, then 20 fresh instances are tested. 8 seeds.

**Wake rules.**
- The winner gets a Hebbian swap: 2 swaps while young, 1 after.
- Every fired-but-inhibited cell gets an anti-Hebbian swap.
- Sticky variant: a mask bit active in at least 8 of the cell's wins can't be swapped.

**Sleep** runs every 500 trials, or when the free pool drops below 20%.
- Prune cells that won fewer than 5 times in their first 150 trials (never consolidated).
- Replay recent episodes, plus one generative item per consolidated cell (its own mask).
- Merge by ablation: remove cell m, and if one other cell explains at least 90% of m's replay items, bundle m into it.
- Re-centre each cell on the episodes it won.

## Results

| Config | Final acc. P1 / P2 / P3 | Cells after each phase | Recruits | Evictions |
|---|---|---|---|---|
| Wake-only, unbounded, plain anti-Hebb | 0.97 / 0.99 / 0.98 | 138 / 282 / 396 | 404 | 8 |
| Wake-only, unbounded, no anti-Hebb | 0.95 / 0.96 / 0.96 | 20 / 43 / 64 | 64 | 0 |
| Wake-only, unbounded, anti + sticky | 1.00 / 0.99 / 0.97 | 34 / 67 / 101 | 101 | 0 |
| Wake-only, cap 40, stop recruiting when full | 1.00 / 0.54 / 0.00 | 34 / 40 / 40 | 40 | 0 |
| Wake-only, cap 40, LRU evict | 0.00 / 0.47 / 1.00 | 34 / 40 / 40 | 108 | 68 |
| Wake-only, cap 40, evict least-consolidated | 1.00 / 0.99 / 0.79 | 34 / 40 / 40 | 332 | 292 |
| **Wake + sleep, cap 40, anti + sticky** | **0.99 / 0.96 / 0.98** | **15 / 26 / 38** | 92 | 12 |
| Wake + sleep, cap 40, no anti-Hebb | 0.92 / 0.89 / 0.99 | 11 / 20 / 29 | 63 | 0 |
| Wake + sleep, cap 40, plain anti-Hebb | 0.96 / 0.77 / 0.10 | 33 / 40 / 40 | 1544 | 1452 |
| Naive sleep (recent-only replay, prune idle) | 0.00 / 0.00 / 0.98 | 10 / 10 / 10 | 100 | 0 |
| Over-eager merge (θ 14/9, 60%) | 0.77 / 0.76 / 0.68 | 11 / 22 / 30 | 98 | 4 |

Seed spread on the bold row: ±0.03 / ±0.05 / ±0.04.

## Findings

1. **Plain anti-Hebbian on every inhibited firer is destructive.** Siblings and duplicates get pushed off shared bits until their own instances fall below the novelty threshold. That recruits more duplicates: 13 cells per concept.
2. **Sticky bits fix it.** Reinforced bits are immune, so anti-Hebbian strips only flaky bits. This leaves 3.4 cells per concept.
3. **Wake + sleep at cap 40 matches unbounded accuracy with 38 cells instead of 101,** and keeps phase-1 concepts at 0.99.
4. **Compression comes almost entirely from pruning never-consolidated cells** (42 prunes against 1 merge). With winner-take-all, a duplicate usually starves rather than co-learns.
5. **Pruning by idleness is catastrophic forgetting.** The protection comes from generative replay plus pruning on "never consolidated" rather than "not used lately".
6. **Eviction policy matters as much as sleep.** LRU kills the oldest knowledge; least-consolidated eviction starves new concepts until sleep frees slots.
7. **An over-eager merge threshold fuses siblings.**
8. **Sleep ends at 38 of 40 cells.** That is the trigger for growing a parallel vector.

## What it changed

Anti-Hebbian is gated by consolidation: full repulsion for young cells, and "drop only what isn't mine" for established ones. Pruning on "never consolidated" and generative replay became the sleep rule. The garbage collection in the growth grammar is the structural analogue. Merging duplicates came back as the fix for noisy input in [11 · real text](11-real-text.md).
