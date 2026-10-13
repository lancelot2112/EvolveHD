# Engine model

The engine is a graph. **Bit vectors with history** are the nodes and **pattern matchers** are the edges. A matcher reads masked 64-bit windows from past ticks, fires when the set-bit count passes a threshold, competes with overlapping matchers, and writes a code into the current tick of a target vector. Learning is local: matchers swap mask bits, protect useful bits by making them sticky, and are spawned and recycled.

This page summarises the [reference engine spec (draft, 2026-10-02)](https://claude.ai/code/artifact/5d2ada31-74bc-4e4d-90e6-486c3efe362a). The [growth grammar](growth-grammar.md) sits on top of it.

## Objects

*Related concepts: [sparse codes](concepts/sparse-codes.md) · [matcher pools](concepts/matcher-pools.md) · [credit without backprop](concepts/credit-without-backprop.md) · [time: delays and holds](concepts/time-delays-and-holds.md)*

**Vector**

| Field | Meaning |
|---|---|
| Width | Bits, a multiple of 64. Default 2048 (32 words) |
| History | Ring buffer of past ticks; depth is a power of two, set by the deepest reader |
| Carry rule | `fresh`: each tick starts empty. `persist`: each tick starts as a copy of the previous one |
| Tick transform | Applied to the carried copy: none, rotate by r, or shift by r |
| Collision policy | How contested bits resolve when writes disagree |

**Matcher**

| Field | Meaning |
|---|---|
| Read terms | One or more of (vector, delay ≥ 1, word index, 64-bit mask) |
| Sticky mask | Per read term, the subset of the mask learning may not change |
| Threshold | Fires when the summed popcount across read terms reaches it |
| Write | Target vector, a fixed code, and set, clear or toggle |
| Competition group | The matchers it competes with |
| State | Age, strike counters for hysteresis, optional prediction code |

Reward, threshold level and feedback are ordinary vectors that some matchers read; nothing in the engine treats them specially.

## Tick

Every read is at least one tick back, so no matcher sees another's write from the same tick. All matchers can be evaluated in any order, or in parallel, with identical results.

1. **Advance:** each vector's head moves; the new tick is empty or a transformed copy.
2. **Input:** external inputs are written into the current tick.
3. **Match:** every matcher sums its popcounts over history and compares with its threshold.
4. **Compete:** winners are resolved inside each competition group (top k, default 1; ties to more sticky bits, then a seeded coin). Losers that fired are inhibited.
5. **Write:** all winners write at once into three order-free accumulators: S = OR of sets, C = OR of clears, X = XOR of toggles. Bits in both S and C are *contested* and resolved by the collision policy; toggles apply last.
6. **Learn:** swap, sticky, spawn and recycle rules run.

## Learning rules

| Rule | Trigger | Effect |
|---|---|---|
| Hebbian swap | Fired and won | Drop one mask bit not set on the input, add one input bit that was set; a bit is dropped only after two strikes |
| Anti-Hebbian swap | Fired and was inhibited | Drop one matched mask bit, add a random bit (optional) |
| Stick | Won, echo, positive reinforcement | Make one matched mask bit sticky |
| Unstick | Won, echo, negative; or won, no echo, positive | Remove two sticky bits; if none, re-roll the output code |
| Leak | Every tick | One random matcher loses one sticky bit |
| Spawn | Real novelty, or the winner predicted wrong | New matcher whose mask is copied from the current context |
| Recycle | Pool at cap and a spawn is due | Replace a matcher with no sticky bits past a grace age |
| Threshold homeostasis | Firing rate out of range | Raise or lower the threshold (optional) |

**Credit without backprop.** One global reinforcement signal. Each matcher decides locally whether it applies, using an *echo* that arrives through ordinary recurrent connections:
- An output-stage matcher's own code survived competition downstream.
- An upstream matcher has a small feedback mask, learned by the same Hebbian swap, that matched the downstream vector after its competition.
- A matcher may carry a predicted code for its next input, so prediction gives internal reinforcement with no external reward.

**Timing.** Every read term has its own integer delay, chosen at spawn and kept by selection: wrong delays are right only by chance and get recycled. History depth follows the deepest reader, plus one doubling of headroom.

## What the early sims fixed

See [early engine experiments](experiments/01-early-engine.md) for the numbers.

- Unsticking must be faster than sticking (two bits per negative event, against one).
- The leak is required; without it, over-specific matchers go sticky after one lucky hit and freeze the pool.
- Spawn only on real novelty; spawning whenever nothing fired runs away to the cap.
- Output codes should be colocated; spread codes blur responsibility.
- Exact delays are needed for fast-changing features. Log-spaced taps work only for slow ones.
- Matchers that hold one guess must be judged by a **track record** (hit/miss counters), not killed on a miss. That fixed transfer and cut spawning churn about fivefold.

## How later work changed this layer

- **Binding:** use shift-bind on cleaned-up codes, not XOR ([audit sims](experiments/04-audit-sims.md)).
- **Anti-Hebbian:** safe only with sticky bits, and only on bits that aren't the loser's own ([grow and sleep](experiments/03-grow-sleep.md)).
- **Graded readout:** a single writer mask can't represent wide distributions. Stochastic multi-level writer masks with learned gains are needed for text ([real text](experiments/11-real-text.md)).
- **Duplicate cells under noise must be merged** in a sleep-style pass ([real text](experiments/11-real-text.md)).
