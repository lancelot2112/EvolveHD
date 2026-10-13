# Drive and prime

Two kinds of input reach an area. **Drive creates winners; prime only adds evidence to candidates that drive already produced.** Context, plans, recall and higher-level predictions are prime while the system is sensing. A gate can promote them to drive only when input is absent: imagination, planning, replay.

## How it works

- **Drive:** the input code (bytes, a lower area's winner) goes to the matcher pool, which picks the winner.
- **Prime:** [writer-mask](writer-mask-readout.md) votes from context nodes add to the target's logits. In the predictive form used for growth, Prime edges predict the next winner and are learned only from driven ticks.
- **Gated free-run:** in silence, the strongest primed candidate is allowed to win, so the area generates sequences.

This is the cortical pattern: basal input drives, and apical context amplifies but can't fire a cell alone ([brain mapping](../brain-mapping.md)).

## Why

Association area with 25% input noise ([04](../experiments/04-audit-sims.md)):

| Plan input | Perception | Prediction (ceiling 0.75) | Silent free-run, valid transitions |
|---|---|---|---|
| Plan as an equal driver | 0.24 | 0.18 | 0.34 (locks onto one word) |
| Plan as priming | 0.98 | 0.71 | 0.86 |
| Priming while sensing, drive in silence | 0.98 | 0.71 | 0.84, no repeats |

## Related

[Primitives and channels](../primitives.md) · [Writer-mask readout](writer-mask-readout.md) · [Broadcast](broadcast.md) · [Time: delays and holds](time-delays-and-holds.md)

## Evidence in

[04 · audit sims](../experiments/04-audit-sims.md)

*[All concepts](README.md) · [Wiki home](../README.md)*
