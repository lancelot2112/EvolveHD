# Go/NoGo selection

Select is the action primitive: **opponent Go and NoGo masks per context, a critic that predicts reward, and three-factor learning from the prediction error δ.** It is how the system acts, and, applied to its own inputs, how it decides where to spend learning time ([attention](attention.md)).

## How it works

- **Action value:** q[a] = popcount(C & Go[a]) − popcount(C & NoGo[a]) over the context C. Choose the best action.
- **Critic:** V per context, δ = r − V.
  - δ > 0: Go[winner] moves toward C.
  - δ < 0: NoGo[winner] moves toward C, and **Go[winner] moves away** (Go LTD).
- **Critic form:** a single mask that moves toward C on δ > 0 and away on δ < 0. A fixed baseline also works.
- **Growing context:** a candidate context input is proposed when it carries reward information jointly with the action, I(R; S, A). It is then tested by **interleaving**: alternate ticks with and without it, compare reward. Shadow trials are impossible here, because the reward of the action not taken is never seen ([shadow trials](shadow-trials.md)).

## Why

- **Exploration comes free from NoGo learning:** a failure pushes that action down in that context (lose-shift). Added noise mostly hurt ([02](../experiments/02-bg-selection-bandit.md)).
- **Without Go LTD, reversal perseverates:** below chance after the switch, slow or no recovery. With it, recovery is as fast as first learning, with 0.99 final accuracy ([02](../experiments/02-bg-selection-bandit.md)). With a working critic it halves recovery, 1815 → 860 trials ([04](../experiments/04-audit-sims.md)).
- **The two-mask critic in the old spec rewarded wrong choices:** V stayed near 0, so an unrewarded action got positive δ, which capped accuracy at 0.66 ([04](../experiments/04-audit-sims.md)).
- **Grown context:** reward rose from 0.23 to 0.83 against a ceiling of about 0.86. The right context was kept in 6/6 runs and junk contexts were rejected ([10](../experiments/10-full-primitives.md)).

## Variants

- **Amygdala-style valence** uses the same machinery but unlearns by overlay (extinction is new inhibition, gated by context) rather than erasure ([brain mapping](../brain-mapping.md)).
- **A conflict hold** (accumulate evidence while the top-2 margin is small) helps only under noise ([02](../experiments/02-bg-selection-bandit.md)).

## Related

[Credit without backprop](credit-without-backprop.md) · [Attention](attention.md) · [Broadcast](broadcast.md) · [Brain mapping](../brain-mapping.md)

## Code

`FullStation.choose`, `learn_select` and `_select_check` in `prims_sim.py`.

## Evidence in

[02 · bg selection bandit](../experiments/02-bg-selection-bandit.md) · [04 · audit sims](../experiments/04-audit-sims.md) · [10 · full primitives](../experiments/10-full-primitives.md)

*[All concepts](README.md) · [Wiki home](../README.md)*
