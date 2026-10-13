# Shadow trials

A candidate edge has to **earn its vote.** It learns from the moment it is built, but is only scored, never acted on, until a paired comparison shows that it improves the target. Kept edges are re-checked the same way, and pruned when they stop paying.

## Protocol

1. **Shadow:** the trial edge's writer masks and gain learn every tick; the prediction the system acts on ignores it.
2. **Burn-in:** 4,000 ticks before anything is counted.
3. **Paired evaluation:** each tick records the target's loss without the edge minus its loss with it. Toy worlds count top-1 flips; text uses log-loss in bits.
4. **Keep** if the mean gain clears the bar (1% in toy worlds, 0.01 bits in text) and z ≥ 2.5. Otherwise reject and double the candidate's backoff.
5. **Re-check kept edges by ablation** every block. Prune if the gain falls below a lower bar (0.3% or 0.003 bits) or z < 1. The gap between the bars is the hysteresis.
6. **Concurrency:** at most 2 trials per target and 6 per area, run in parallel.
7. **Priority:** a newly built node gets one priority trial. Priority is spent, never held.

## Variants

- **Long trials for brute force:** a granule-layer readout climbs slowly, so it needs two passing windows in a row and is rejected only at 60k ticks ([10](../experiments/10-full-primitives.md)).
- **Interleaved trials for actions:** alternate ticks with and without the new context and compare reward, since counterfactual reward is unseen ([Go/NoGo selection](go-nogo-selection.md)).

## Why each rule exists

| Rule | Without it | Experiment |
|---|---|---|
| Trials don't vote | An immature trial's noise masked a kept edge, which was then falsely pruned | [06](../experiments/06-growth-interfaces.md) |
| Separate keep and prune bars | Thrash | [06](../experiments/06-growth-interfaces.md) |
| Parallel trials | Rewiring after a change took a median of 17.5k ticks against 8k; bind candidates monopolised a serial slot | [06](../experiments/06-growth-interfaces.md), [07](../experiments/07-graph-growth.md) |
| Priority spent once | Delay taps hogged a target's slot until the right bind was garbage-collected; that target was never solved | [10](../experiments/10-full-primitives.md) |
| Ablation re-checks | Dead edges kept voting after the world changed | [06](../experiments/06-growth-interfaces.md) |

Wiring every candidate without trials cost 4 points against wiring nothing at all ([06](../experiments/06-growth-interfaces.md)).

## Related

[Screens](screens.md) · [Growth operators](growth-operators.md) · [Credit without backprop](credit-without-backprop.md) · [Writer-mask readout](writer-mask-readout.md)

## Code

`check` and `fill` in `text_sim.py`; trial handling in `prims_sim.py` for granules and Select.

## Evidence in

[06 · growth interfaces](../experiments/06-growth-interfaces.md) · [07 · graph growth](../experiments/07-graph-growth.md) · [10 · full primitives](../experiments/10-full-primitives.md)

*[All concepts](README.md) · [Wiki home](../README.md)*
