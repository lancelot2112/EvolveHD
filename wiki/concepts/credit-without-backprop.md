# Credit without backprop

EvolveHD never passes error backward through the graph. **One global reinforcement signal is broadcast, and each matcher decides locally whether it applies to it.** Four mechanisms make that work.

## Sticky bits

A mask bit becomes sticky after positive reinforcement, and swap rules can't move it. Negative reinforcement unsticks bits, faster than they stick (two bits per negative event, against one). A slow **leak** unsticks a random bit somewhere in the pool each tick, so over-specific matchers can't freeze after one lucky hit.

- Two-area credit task: sticky bits with echo 98%, global reward without them 80%, no credit 73% ([01](../experiments/01-early-engine.md)).
- Without the leak: 82% / 76% / 32% against 93% / 96% / 86% on the sequence tasks ([01](../experiments/01-early-engine.md)).
- Sticky bits are also what make anti-Hebbian learning safe: they strip only flaky bits, which left 3.4 cells per concept instead of 13 ([03](../experiments/03-grow-sleep.md)).

## Echo

A matcher applies the global signal only if it took part in a loop that was actually used:
- an output-stage matcher's code survived competition downstream;
- or an upstream matcher's small feedback mask matched the downstream vector after its competition.

Prediction gives internal reinforcement too: a matcher carrying a predicted code is rewarded when the prediction arrives, so no external reward is needed for sequences. See the [engine model](../engine-model.md).

## Track records

Judge a matcher by hit/miss counters, not by its last result. Kill-on-miss made transfer to unseen contexts unreliable and churned the pool. Counters fixed transfer in every seed and cut spawning about fivefold ([01](../experiments/01-early-engine.md)).

## Input-reliability gating

If an input arrives malformed (well below its expected bit count), skip learning for that tick. A broken sensor is not the matcher's fault ([01](../experiments/01-early-engine.md)).

## How the growth layer extends it

At the growth level, credit is assigned to **structures** by [shadow trials](shadow-trials.md). Paired ablation measures each edge's contribution to the loss, again with no backward pass. For actions, the critic's δ plays the same role ([Go/NoGo selection](go-nogo-selection.md)).

## Related

[Engine model](../engine-model.md) · [Sleep and consolidation](sleep-and-consolidation.md) · [Shadow trials](shadow-trials.md) · [Go/NoGo selection](go-nogo-selection.md)

## Evidence in

[01 · early engine](../experiments/01-early-engine.md) · [03 · grow sleep](../experiments/03-grow-sleep.md)

*[All concepts](README.md) · [Wiki home](../README.md)*
