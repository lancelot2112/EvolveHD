# Sleep and consolidation

Wake grows; sleep compresses. **Recruit freely on novelty while awake. Then, offline, remove what never consolidated, replay what did, and merge duplicates.** This keeps a fixed-capacity area accurate on old knowledge while it keeps learning new.

## How it works

1. **Prune the never-consolidated:** cells that won fewer than 5 times in their first 150 trials. Never prune for idleness.
2. **Generative replay:** recent episodes plus one generated item per consolidated cell (its own mask), so old concepts are rehearsed without their data.
3. **Merge:** if one cell explains at least 90% of another's replay items, fold them together. In the text runs this became a simple overlap test: byte cells whose masks share at least 8 of 24 bits are merged every 1,000 ticks, keeping the older cell.
4. **Re-centre** each cell on the episodes it won.

At the structure level, [garbage collection](growth-operators.md#cleanup) is the same idea: unused, already-tried nodes are removed, so the structure budget recycles.

## Why

- **Wake + sleep at a cap of 40 cells:** 0.99 / 0.96 / 0.98 accuracy over three phases, with 38 cells. Unbounded wake-only learning needed 101 cells ([03](../experiments/03-grow-sleep.md)).
- **Naive sleep that prunes idle cells forgets everything old:** 0.00 / 0.00 / 0.98 ([03](../experiments/03-grow-sleep.md)).
- **Most compression came from pruning, not merging** (42 prunes, 1 merge). With winner-take-all, a duplicate usually starves ([03](../experiments/03-grow-sleep.md)).
- **Under noisy input, merging is essential.** Duplicates split each byte across cells: 4.9 bits per byte without merging, 3.18 with it ([11](../experiments/11-real-text.md)).
- **An over-eager merge fuses siblings** (accuracy about 0.7) ([03](../experiments/03-grow-sleep.md)).

## Open

Where merging should live (a scheduled sleep phase or a continuous background pass), and what overlap rule generalises beyond bytes. See [open questions](../open-questions.md).

## Related

[Matcher pools](matcher-pools.md) · [Credit without backprop](credit-without-backprop.md) · [Capacity and budget](capacity-and-budget.md) · [Growth operators](growth-operators.md)

## Evidence in

[03 · grow sleep](../experiments/03-grow-sleep.md) · [11 · real text](../experiments/11-real-text.md)

*[All concepts](README.md) · [Wiki home](../README.md)*
