# Broadcast

Broadcast compresses many inputs into **one number or one code that every area reads.** It sets regimes and gates learning; it never carries detailed content into an area's own computation. It has two independent axes, what it carries and when it acts:

| | Scalar | Semantic (a code) |
|---|---|---|
| **Tonic** | Regime: exploration temperature, thresholds, plasticity | Context: a task set or goal, read as [prime](drive-and-prime.md) |
| **Phasic** | Learning gate: reward-prediction error, surprise alarms | Ignition: one interpretation wins and is broadcast everywhere |

## Phasic scalar: the surprise alarm

Each target keeps a fast (300-tick) and a slow (5,000-tick) average of its accuracy. When the slow one is above 0.3 and the fast one falls 0.12 below it, an alarm fires (8,000-tick refractory). It:

- re-checks the target's kept edges early;
- clears backoffs on its candidates;
- drops trials chosen before the change;
- **restarts the target's [screens](screens.md) on post-alarm data;**
- halves burn-in and triples writer-mask learning for 4,000 ticks.

Only the screen restart mattered. In the first runs, the alarm without it moved median rewiring time from 18k to 16k ticks. With it (and the later grammar fixes), rewiring after a world change took 5k ticks against 12k without an alarm ([10](../experiments/10-full-primitives.md)).

## Phasic semantic: consensus

A claustrum-like loop sums the areas' outputs as a bipolar vote, broadcasts the consensus, mutes dissenting areas and boosts agreeing ones. It produces real unity (99% of areas agree) but **locks**. Adding fatigue to the winning interpretation gives rivalry-like alternation that tracks the evidence ([05](../experiments/05-claustrum-consensus.md)).

## Tonic scalar

The reward critic's baseline and the exploration level belong here. A surprise-driven temperature failed in the bandit: binary reward keeps |δ| high, so the temperature never settles ([02](../experiments/02-bg-selection-bandit.md)).

## Surprise as an event

The same fast/slow machinery gives a per-tick signal: the target's bits for the current input against its running mean and sd. Thresholded at μ + zσ, it becomes an event that Gate moves can use. Resetting a recurrent bind on that event gave the [chunk](growth-operators.md#chunk), the most valuable edge on text ([12](../experiments/12-words.md)).

## Related

[Attention](attention.md) · [Go/NoGo selection](go-nogo-selection.md) · [Screens](screens.md) · [Brain mapping](../brain-mapping.md)

## Code

`FullStation._broadcast` and `win` in `prims_sim.py`; the consensus loop is in `claustrum.py` (not archived in this repo).

## Evidence in

[02 · bg selection bandit](../experiments/02-bg-selection-bandit.md) · [05 · claustrum consensus](../experiments/05-claustrum-consensus.md) · [10 · full primitives](../experiments/10-full-primitives.md) · [12 · words](../experiments/12-words.md)

*[All concepts](README.md) · [Wiki home](../README.md)*
