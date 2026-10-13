# Stepping stones

Depth is reachable only through easier puzzles. **A system that grows by screening can find a structure only if some part of it already carries signal.** Easier puzzles that reward building those parts are the stepping stones. The environment, not the controller, decides what is learnable this way.

## The cycle that climbs

expand → associate → compress, repeated:
1. **Bind** builds a conjunction ([shift-bind](shift-bind.md)).
2. **A Prime edge** learns what it predicts ([writer-mask readout](writer-mask-readout.md)).
3. **Distill** squeezes it back into a small variable that the next level can bind cheaply ([growth operators](growth-operators.md#distill)).

## Evidence

**Staged-depth world:** T2 = LS1[A, B], T6 = LS2[LS1[A, B], C], T8 = LS3[…, E] ([08](../experiments/08-bootstrapping.md)).

| Config | Levels solved |
|---|---|
| Binds only | 2 of 3: the third bind would need 4,096 cells |
| Binds + distill | 3 of 3 in 6/6 runs (T2 at 12k, T6 at 18k, T8 at 32k) |
| No stepping stones (T2 and T6 replaced by noise) | 0: no screen ever fires |

- **Order in time doesn't matter.** All puzzles were present from tick 0; the shallowest unsolved one is the only one with signal, so the system solves them in depth order by itself.
- **Real data has stepping stones built in.** On text, the grammar grew delays, then byte pairs, then 3-byte and skip contexts ([11](../experiments/11-real-text.md)). The next level up appeared too: a chunk reset at surprising bytes paid by itself, but its cuts are morph-like, and word-to-word context was still invisible ([12](../experiments/12-words.md)).
- **Without stepping stones, only brute force works:** a granule layer whose cost explodes with the number of inputs ([10](../experiments/10-full-primitives.md)).

## Design consequence

The environment definition is a set of information sources with compositional structure at graded depth, where intermediate variables are themselves worth predicting. Parity-like structure with no partial signal needs a teacher that exposes the intermediates, or brute force.

## Related

[Attention](attention.md) (choosing which stepping stone to work on) · [Growth operators](growth-operators.md) · [Screens](screens.md) · [Capacity and budget](capacity-and-budget.md)

## Evidence in

[08 · bootstrapping](../experiments/08-bootstrapping.md) · [12 · words](../experiments/12-words.md)

*[All concepts](README.md) · [Wiki home](../README.md)*
