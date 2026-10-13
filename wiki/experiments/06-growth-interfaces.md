# 06 · Typed interfaces and growth operators (2026-10-12)

*[All experiments](README.md)* · [← 05](05-claustrum-consensus.md) · [07 →](07-graph-growth.md)

**Concepts:** [Shadow trials](../concepts/shadow-trials.md) · [Screens](../concepts/screens.md) · [Growth operators](../concepts/growth-operators.md) · [Writer-mask readout](../concepts/writer-mask-readout.md)

Question: if every edge is a typed channel and growth is triggered by measurable pressure, does the network settle or thrash, and do lower nodes have to harden before upper nodes grow?

## Setup

**World.** A hidden topic (6 topics, dwell ~33 ticks) picks which 12 of 48 words are in play and each word's two successors (0.75 / 0.25). Words shared by two topics have different successors in each, so the previous word alone is ambiguous.

Ceilings from stationary counting: previous word only 0.593, previous word + true topic 0.729. Online writer masks can beat 0.593 because they track recent statistics.

**Signals.**
- S1: the topic code in phase 1, then an irrelevant slow chain (6 states, dwell 100) after a swap at tick 60k.
- S2: the reverse of S1.
- N: one of 16 symbols, i.i.d. every tick.
- W2: the word node's own t−2 tap.

S1, S2 and N are noisy 32-of-2048 codes.

**Nodes.**
- Every source is a matcher pool: k-WTA k=1, recruit on novelty, Hebbian swaps. Its output is the winner's fixed 32-bit code, so every channel has fixed sparsity.
- The word node W predicts its next winner. Each Prime edge gives each source cell a writer mask over W's code space (48 bits for the self tap, 160 for context edges). The mask moves 1 bit per event toward the code of the word that actually followed. Score_j = Σ_e popcount(writer_e[src winner] & code_j).

**Controller.**

| Pressure | Operator |
|---|---|
| Surprise plateau | Trial one candidate edge: 4k burn-in, then 2k ticks of paired ablation. Keep if z ≥ 2.5 and Δacc ≥ 1%. |
| Kept edge no longer pays | Re-check kept edges in 4k blocks. Prune if z < 1 or Δ < 0.3% (hysteresis). |
| Rejected candidate | Retry with doubling backoff. |
| Candidates exhausted, plateau, and lagged co-activity in W (G/dof at lags 3–20 > 15) | Grow a Compress node P (16 cells, 96-bit masks over the OR of the last 12 W codes) and offer it as a candidate. |

**Variants.**
- v1: trial edges vote in the acted-on prediction; candidates are tried in random order.
- v2: trial edges are **shadowed** (scored, not acted on), kept edges are ablated against a baseline that excludes the trial edge, never-tried candidates go first, and one rejection is enough for a candidate to stop blocking structural growth.

6 seeds per config.

## A. Edge discovery and rewiring (120k ticks, signals swap at 60k)

| config | acc phase 1 (last 10k) | acc phase 2 (last 10k) | kept at 60k | kept at 120k | trials per run |
|---|---|---|---|---|---|
| fixed: self only | 0.606 | 0.610 | | | |
| fixed: self + true topic | 0.671 | 0.672 | | | |
| fixed: self + W2 | **0.691** | **0.692** | | | |
| fixed: self + W2 + true topic | 0.678 | 0.677 | | | |
| fixed: every candidate except W2 (S1, S2, N) | 0.570 | 0.566 | | | |
| fixed: every candidate incl. W2 | 0.626 | 0.625 | | | |
| ctrl v1, S1/S2/N | 0.638 ±0.033 | 0.659 ±0.011 | S1 4/6, none 2/6 | S2 6/6 | 10.2 |
| ctrl v1, S1/S2/N, backoff reset on prune | 0.638 ±0.033 | 0.652 ±0.021 | S1 4/6 | S2 5/6 | 10.5 |
| **ctrl v2, S1/S2/N** | **0.664 ±0.010** | **0.666 ±0.010** | **S1 6/6** | **S2 6/6** | 10.5 |
| ctrl v2, S1/S2/N, 2k burn-in | 0.672 ±0.007 | 0.668 ±0.012 | S1 6/6 | S2 6/6 | 11.7 |
| ctrl v1, S1/S2/N/W2, reset | 0.675 ±0.008 | 0.675 ±0.019 | W2 4/6, S1 2/6 | W2 4/6, S2 2/6 | 11.0 |
| ctrl v2, S1/S2/N/W2 | 0.681 ±0.008 | 0.687 ±0.017 | W2 3/6, S1 3/6 | W2 5/6, S2 1/6 | 10.8 |

- **Distractors were almost always rejected.** Across 78 controller runs (sections A, B, D), one distractor edge was kept: v1, Δ 0.0115, just over the 1% bar. The prune check removed it 4k ticks later.
- Wiring every candidate costs 4 points against self-only. Rejection and pruning are necessary, not cosmetic.
- The topic edge was found at 13k–35k ticks in v2 (11k–32k with a 2k burn-in, median 19k).
- **Stationary phases did not thrash in v2:** zero prunes of a useful edge. The only prunes were the dead S1 edge after the swap, 5k–15k ticks later.
- v1 had one false prune: a kept S2 edge (Δ 0.057) was pruned 8k ticks later while an immature S1 trial was voting, because the trial's noise masked S2's contribution.
- Recovery after the swap (swap → new topic edge kept) had a median of 17.5k ticks in v2 (range 6k–26k). This is dominated by block-based prune detection plus the serial 6k-tick trial queue. Resetting backoff on prune did not help.
- Rejected candidates keep being re-trialled on doubling backoff (~10 trials per 120k ticks, about half the run). Shadowing makes this free in accuracy, but not in compute.
- Redundancy: the word node's own t−2 tap carries most of the topic information. Adding the true topic on top of it costs 1.3 points, so whichever informative source is kept first makes the other redundant, and the controller correctly rejects the second one. Acceptance is order-dependent but parsimonious.

## B. Growing a Compress node from pressure (80k ticks)

| config | acc last 10k | P grown (median tick) | P kept | P winner NMI with topic |
|---|---|---|---|---|
| fixed: self only | 0.605 | – | – | – |
| fixed: self + P from t=0 | 0.631 | – | – | 0.615 |
| fixed: self + true topic | 0.662 | – | – | – |
| fixed: self + W2 | 0.692 | – | – | – |
| fixed: self + W2 + P | 0.661 | – | – | 0.618 |
| ctrl v1, topic world, S2/N | 0.609 ±0.017 | 5/6 (69k) | 2/6 | 0.587 |
| **ctrl v2, topic world, S2/N** | **0.629 ±0.010** | **6/6 (28.5k)** | **5/6** | **0.612** |
| ctrl v2, topic world, S2/N/W2 | 0.693 ±0.011 | 6/6 (42k) | 0/6 | 0.602 |
| ctrl v1, topic world, S2/N/W2 | 0.678 ±0.014 | 0/6 | – | – |
| ctrl v2, no-topic world, S2/N | 0.752 ±0.004 | 0/6 | – | – |
| no-topic world, P forced at 20k | 0.751 ±0.005 | forced | 0/6 | – |

- **The co-activity trigger is specific.** G/dof was 58–77 in the topic world and 3.5–6.7 in a world with no latent topic, so with a threshold of 15 P was grown in 6/6 runs vs 0/6.
- A grown P learns the topic without being told (NMI 0.61). As an edge it recovers about 42% of the true topic edge's benefit (0.629 vs 0.605 self-only and 0.662 with the true topic).
- When W2 is already kept, P is redundant: self + W2 + P scores below self + W2. The controller grew P and rejected it in 6/6 runs. When P was forced in a world with no latent structure, it was rejected 6/6.
- **Operator priority matters.** In v1, re-trialling known-bad candidates kept the controller busy, so the structural operator fired late (69k) or never (0/6 with W2). Letting candidates count as exhausted after one rejection moved growth to ~28k.

## C. Hardening order (60k ticks, churny lower node)

The lower W node starts with immature masks (half random, Hebbian on 10% of wins) and sleep-prunes unconsolidated cells. Churn drops from ~190 recruit/prune events in the first 2k ticks to 10–20 per 2k by tick 10k.

| config | acc 0–10k | acc 10–20k | acc last 10k | P NMI | P stale mask bits | P recruits / evictions |
|---|---|---|---|---|---|---|
| no upper | 0.560 | 0.600 | 0.605 | – | – | – |
| oracle: self + true topic | 0.587 | 0.665 | 0.666 | – | – | – |
| upper at t=0, plastic | 0.548 | 0.628 | 0.633 | 0.604 | 3.7% | 16.5 / 1.3 |
| upper at t=0, hardens after 30 wins | 0.549 | 0.631 | 0.630 | 0.608 | 6.6% | 19.3 / 3.3 |
| upper gated on churn ≤ 15/2k (≈ tick 7.5k), plastic | 0.564 | 0.625 | 0.630 | 0.608 | 0.7% | 11.7 / 0 |
| upper gated, hardens | 0.563 | 0.629 | 0.633 | 0.616 | 0.8% | 13.2 / 0 |

Stress test: 30% of the lower node's live cells get new output codes at tick 30k, simulating a sleep re-centre or merge.

| config | acc 25–30k | acc 30–32k | acc 32–40k | acc last 10k | P stale bits |
|---|---|---|---|---|---|
| no upper | 0.609 | 0.559 | 0.606 | 0.608 | – |
| upper plastic | 0.632 | 0.516 | 0.622 | 0.633 | 5.4% |
| upper hardened | 0.634 | 0.539 | 0.627 | 0.636 | 9.5% |

- **The lower node does not have to harden first** at this scale. Growing the upper at t=0 costs about 1–1.5 points during the first 10k ticks (briefly worse than having no upper), plus wasted cells and stale bits. Final accuracy is the same in all four orderings.
- A lower-node reorganisation hits harder when an upper exists: −10 to −12 points vs −5 without one. Both plastic and hardened uppers are within about 1 point of their pre-recode level during ticks 32–40k, and back to it by the end.
- Robustness comes from redundancy. Each upper cell reads about a dozen lower codes, so losing 30% leaves it above threshold, and the upper's writer edge keeps relearning its targets.

## D. Correction: slow irrelevant sources

A single-seed calibration (different mask sizes, topic dwell 100) suggested that a slow but irrelevant signal could pass the edge test, acting as a "current context" register for the online writer masks. With 6 seeds at dwell 100 (paired Δ vs self-only):
- slow irrelevant chain: +0.008 ±0.020
- i.i.d. N: +0.004 ±0.019
- true topic: +0.059 ±0.010

With matched dwell (33), the controller accepted the chain 0/6 times. **Not supported.**

## Changes to the spec this supports

1. Run edge trials in **shadow**: the trial edge is scored but does not vote. Run one trial at a time, and ablate kept edges against a baseline that excludes the trial edge.
2. Use accept/prune hysteresis (accept z ≥ 2.5 and Δ ≥ 1%; prune z < 1 or Δ < 0.3% over 4k blocks). This gave no stationary thrash.
3. Order candidates untried-first, then by backoff. A candidate rejected once no longer blocks structural growth.
4. Grow a Compress node when candidates are exhausted, surprise has plateaued, and lagged co-activity G/dof > 15. Offer it as an ordinary candidate, so redundancy is rejected by the same test.
5. Size writer masks to the source's fan-out: a few targets for the self tap (48 bits here), many for context (160 bits). Saturated masks lose frequency weighting. Adding a redundant context edge still costs 1–1.5 points under additive voting.
6. Gate upper growth on lower churn to avoid the early dip. Hardening order is otherwise optional.
7. Open: rewiring after a world change is slow (median 17.5k ticks). A surprise jump (an LC-style phasic broadcast) should trigger immediate re-evaluation of kept edges instead of waiting for the next 4k block.

## Code

[`reference/python/growth_sim.py`](../../reference/python/growth_sim.py); see [reference code](../reference-code.md) for how to run it.
