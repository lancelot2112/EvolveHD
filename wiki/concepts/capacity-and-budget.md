# Capacity and budget

Everything is finite: cells per pool, derived nodes per area, trial slots, learning time. **The design choice is not to avoid limits, but to make growth respond to them.** Prune what doesn't pay, recycle what isn't used, and prefer structures that fit.

## Limits in play

| Limit | Value in the sims | What happens at the limit |
|---|---|---|
| Cells per pool | 40 (concept area), 192 (byte), 1024 (bind), 4096 (chunk) | LRU eviction; [sleep](sleep-and-consolidation.md) frees cells |
| Derived nodes per area | 16 to 24 | Growth pauses until [cleanup](growth-operators.md#cleanup) frees a slot |
| Trial slots | 2 per target, 6 per area | Candidates wait in the ranked queue |
| Learning time | Blocks shared between inputs | [Attention](attention.md) decides |

## Growth adapts to capacity

On text, a 1024-cell real bind pool can't hold 3-byte contexts. The data rule refused those binds, so growth built **factored skip-pair binds** instead. With the same real pools, the grown structure beat a hand-wired order-4 chain: 2.61 against 2.74 bits per byte. Capacity still cost 0.28 bits against exact, unlimited tables ([11](../experiments/11-real-text.md)).

## Capacity at the word level

A 4,096-cell chunk pool cannot hold English word prefixes: it was evicted 101,000 times in 400k bytes. A conjoin over it (1,024 cells) was evicted 37,000 times and gained nothing with real pools, while the same move with exact nodes cut 0.05–0.09 bits per byte. The real conjoin never had an extended trial, so capacity is the likely cause rather than a proven one ([13](../experiments/13-word-pairs.md)). Word-level structure needs pools that grow with demand, or a cheaper code for word identity.

## Capacity triggers growth elsewhere

When sleep ends near the cap (38 of 40 cells), that is the trigger for growing a parallel vector rather than splitting one. Splitting was rejected because it fragments the readers' windows ([03](../experiments/03-grow-sleep.md), [history](../history.md)).

## Waste is a budget leak

- **Re-trialling known-bad candidates** kept the controller too busy to grow a compress node ([06](../experiments/06-growth-interfaces.md)).
- **Junk Select trials counted as opportunity** and soaked up 43 of 200 blocks ([10](../experiments/10-full-primitives.md)).
- **Unused nodes keep costing compute** until cleanup removes them.

## Related

[Matcher pools](matcher-pools.md) · [Sleep and consolidation](sleep-and-consolidation.md) · [Growth operators](growth-operators.md) · [Attention](attention.md)

## Evidence in

[03 · grow sleep](../experiments/03-grow-sleep.md) · [11 · real text](../experiments/11-real-text.md) · [13 · word pairs](../experiments/13-word-pairs.md)

*[All concepts](README.md) · [Wiki home](../README.md)*
