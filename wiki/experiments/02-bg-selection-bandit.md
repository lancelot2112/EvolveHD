# 02 · Basal-ganglia selection bandit (2026-10-10)

*[All experiments](README.md)* · [← 01](01-early-engine.md) · [03 →](03-grow-sleep.md)

**Concepts:** [Go/NoGo selection](../concepts/go-nogo-selection.md) · [Broadcast](../concepts/broadcast.md) · [Attention](../concepts/attention.md)

**Question.** Can basal-ganglia action selection (Go/NoGo pathways plus a dopamine critic) be built from popcounts, masks and bit swaps? And what does exploration and reversal learning need?

## Setup

- **Task:** contextual bandit. D = 2048 contexts (256 active bits, 20% resampled per trial); 32 latent states, 8 actions, one rewarded action per state. The reward map reverses at trial 3,000 of 6,000. 8 seeds per config.
- **Action value:** q[a] = popcount(C & R & Mgo[a]) − popcount(C & R & Mno[a]), where R is a random keep-mask (dropout τ acts as exploration temperature).
- **Critic:** V = popcount(C & Mv) / |C|, and δ = r − V.
- **Learning:** density-preserving swaps of k = round(8·|δ|) bits. δ > 0 moves Mgo[winner] and Mv toward C. δ < 0 moves Mno[winner] toward C and Mv away.
- **Options:**
  - **Conflict hold:** resample R and accumulate q while the top-2 margin is below m (at most 5 evaluations).
  - **Go LTD:** on δ < 0, also swap Mgo[winner] away from C.

## Results

| Config | Acc. last 500 before reversal | First 500 after | Final 500 | Trials to 90%, before / after | Evals per decision |
|---|---|---|---|---|---|
| Greedy τ = 0 | 0.99 | 0.02 | 0.86 | 1591 / never | 1.0 |
| τ = 0.15 | 0.98 | 0.02 | 0.89 | 1581 / 2891 | 1.0 |
| τ = 0.3 | 0.97 | 0.03 | 0.85 | 1846 / never | 1.0 |
| τ = 0.6 | 0.90 | 0.04 | 0.78 | 2920 / never | 1.0 |
| Anneal τ 0.6 → 0 | 0.99 | 0.02 | 0.90 | 1528 / 2817 | 1.0 |
| Surprise-driven τ | 0.90 | 0.04 | 0.80 | 2997 / never | 1.0 |
| τ = 0.3 + hold m = 4 | 0.98 | 0.03 | 0.93 | 1591 / 2667 | 1.57 |
| τ = 0.3 + hold m = 10 | 0.98 | 0.03 | 0.89 | 1425 / 2815 | 2.48 |
| **Go LTD, τ = 0** | 0.99 | 0.14 | **0.99** | **1262 / 1354** | 1.0 |
| Go LTD, τ = 0.15 | 0.98 | 0.12 | 0.99 | 1218 / 1538 | 1.0 |
| Go LTD, τ = 0.3 + hold m = 4 | 0.99 | 0.11 | 0.98 | 1231 / 1587 | 1.49 |

## Findings

1. **Exploration comes free from NoGo learning.** A failure pushes that action down in that context (lose-shift), which is systematic exploration. Added dropout noise mostly hurts. Random initial masks already give each context arbitrary but distinct preferences, which act as a prior.
2. **Without Go depression, reversal is pathological.** Accuracy drops below chance (perseveration) and recovers slower than first learning, or never reaches 90%.
3. **Go LTD on δ < 0 fixes it.** This is a dopamine-dip depression on the D1 side. Reversal recovers about as fast as initial learning, and final accuracy reaches 0.99.
4. **Conflict hold helps only under noise.** It acts as evidence accumulation, adding 8 points of final accuracy at about 1.5 evaluations per decision. Without noise it is unnecessary.
5. **Surprise-driven τ was worst.** Binary reward keeps |δ| high, so τ never settles. It needs a baseline-relative surprise signal.

## What it changed

Go LTD became part of Select. The [audit sims](04-audit-sims.md) later replaced the critic with a single-mask form. Select in [10 · all primitives](10-full-primitives.md) uses Go/NoGo with Go LTD.
