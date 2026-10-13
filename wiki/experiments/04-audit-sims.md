# 04 · Audit sims on the "current system" spec (2026-10-11)

*[All experiments](README.md)* · [← 03](03-grow-sleep.md) · [05 →](05-claustrum-consensus.md)

**Concepts:** [Go/NoGo selection](../concepts/go-nogo-selection.md) · [Shift-bind](../concepts/shift-bind.md) · [Drive and prime](../concepts/drive-and-prime.md) · [Sparse codes](../concepts/sparse-codes.md)

**Question.** An audit of the project's whole-system spec flagged twelve issues. Four of the main claims were tested in the spec's own notation:
- codes of 32 of 2048 bits;
- a striatum context C of 64 bits;
- masks of 256 bits;
- reward scaled to R = 16 bits;
- k = randround(|δ| / 2), with δ clamped to ±16.

8 seeds unless stated.

## 1. Striatum critic and Go LTD

**Setup:** 32 states, 8 actions, C = 64 bits with 20% resampled, reversal at trial 3,000 of 6,000.

**The biggest bug was the critic.** The spec's critic is V = |C & M_v+| − |C & M_v−|, and both masks only ever move toward C. V stays near 0 even at 66% accuracy, and averages −3 to −5 on wrong trials. So an unrewarded choice gets a **positive** δ, and its Go mask is strengthened.

| Critic | Variant | Acc. last 500 before | First 500 after | Final 500 | Trials to 90%, before / after (seeds reaching) |
|---|---|---|---|---|---|
| Two-mask (as written) | No Go LTD | 0.66 | 0.05 | 0.38 | never / never |
| Two-mask (as written) | + Go LTD | 0.73 | 0.12 | 0.51 | never / never |
| Two-mask, V ≥ 0 | No Go LTD | 0.89 | 0.06 | 0.55 | 1815 (6/8) / never |
| Two-mask, V ≥ 0 | + Go LTD | 0.91 | 0.14 | 0.82 | 2583 (7/8) / 2690 (2/8) |
| Single mask, toward on δ > 0, away on δ < 0 | No Go LTD | 1.00 | 0.04 | 0.99 | 837 / 1815 |
| Single mask | **+ Go LTD** | 1.00 | 0.32 | **1.00** | 624 / **860** |
| Single mask | + Go LTD + habits | 1.00 | 0.22 | 1.00 | 596 / 973 |
| Fixed baseline R/2 | No Go LTD | 1.00 | 0.08 | 0.99 | 699 / 1304 |
| Fixed baseline R/2 | + Go LTD | 1.00 | 0.36 | 1.00 | 541 / 736 |

With a working critic, Go LTD roughly halves reversal recovery (1815 → 860 trials). Habits cost about 110 trials of reversal speed and nothing at steady state.

## 2. Pair-bit binding

**Setup:** 4 × 4 contexts and 4 actions, with a Latin-square target so neither feature alone carries information. 4,000 trials.

The overlap of pair(a, b) with pair(a, c) shows how much of "a" each binding leaks; unrelated pairs overlap by about 1 bit. XOR ρ(a ^ ρ(b)) shares 15.3 bits, CDT 10.9, and shift-bind 1.1.

| Pair bits | Final acc., noise after binding | Final acc., 20% noise before binding |
|---|---|---|
| None | 0.25 (chance) | 0.26 |
| XOR (spec) | 0.30 | 0.26 |
| CDT | 0.45 | 0.29 |
| Shift-bind | 0.91 (90% at ~530) | 0.69 |
| Oracle (fresh code per pair) | 0.92 | 0.95 |

Shift-bind matches the oracle when its inputs are cleaned up first, so **bind after a k-WTA cleanup**.

## 3. Hippocampal recall: reinstatement against drift

**Setup:** 150 words with Zipf frequencies, 40 episodes of 12, no forward link.
- **Task A:** cue = an episode's start context; recall 11 rows.
- **Task B:** cue = a word that occurs only once, mid-episode; recall the next 4 rows.

| Update after each recall | A same-episode | A forward-correct | A correct prefix | B same-episode | B forward-correct |
|---|---|---|---|---|---|
| Full reinstatement | **0.85** | **0.84** | **8.9** | **0.97** | **0.66** |
| Drift only | 0.74 | 0.58 | 5.4 | 0.18 | 0.03 |
| Blend 16 of 32 slots, then drift | 0.81 | 0.76 | 7.6 | 0.81 | 0.53 |

Drift-only collapses when recall starts from an item cue: a word's bits are the same in every episode, so they can't carry the system back into its episode. Keep reinstatement.

## 4. Association area: plan as driver against priming

**Setup:** 40 words with 2 successors each (0.75 / 0.25), 25% input noise, 160 cells, k = 4. The grammar changes at step 6,000 of 12,000. 4 seeds.

| Plan input | Perception, expected words | Perception, surprise words | Prediction (ceiling 0.75) | After change: first / last 1000 | Silent free-run: distinct / valid / longest run |
|---|---|---|---|---|---|
| Driver, equal weight | 0.24 | 0.05 | 0.18 | 0.18 / 0.20 | 3.0 / 0.34 / 16.4 |
| Priming bonus | 0.98 | 0.70 | 0.71 | 0.51 / 0.72 | 14.2 / 0.86 / 5.6 |
| Gated: priming while sensing, driver in silence | 0.98 | 0.70 | 0.71 | 0.51 / 0.72 | 13.4 / 0.84 / 1.1 |

## What it changed

1. Single-mask critic (toward on δ > 0, away on δ < 0), plus Go LTD.
2. Shift-bind on cleaned codes. This is the Expand primitive's binding.
3. Keep retrieved-context reinstatement.
4. Plans and recall feed an area as **priming** while there is sensory input, and drive it only in silence. This became the Drive/Prime channel contract in [primitives](../primitives.md).
