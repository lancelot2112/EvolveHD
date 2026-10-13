# Matcher pools

A matcher pool is the basic recognising unit: **a set of cells that each hold a K-bit mask, compete with k-winners-take-all, recruit a new cell on novelty, and evict when full.** Input pools, bind nodes and chunk nodes are all matcher pools; only their inputs differ.

## How it works

1. **Match.** Each live cell's overlap = popcount(mask AND input). The best cell wins.
2. **Recruit.** If the best overlap is below the recruit bar, a new cell is made. Its mask is K bits sampled from the input, and it gets a fresh [code](sparse-codes.md) and a new generation. With no free cell, the least recently used cell is evicted.
3. **Adjust.** On a partial match the winner swaps one bit toward the input (Hebbian). Under noise this is rationed, see below.
4. **Losers** can get anti-Hebbian swaps, but only safely when bits are protected by consolidation ([credit without backprop](credit-without-backprop.md)).

## What the experiments showed

- **Track the right thing when judging cells.** Kill-on-miss churned 90,000 spawns over 154k bytes; hit/miss counters fixed it ([01](../experiments/01-early-engine.md)).
- **Eviction policy matters as much as sleep.** LRU kills old knowledge; evicting the least-consolidated cell starves new concepts until sleep frees slots ([03](../experiments/03-grow-sleep.md)).
- **Set the recruit bar relative to noise.** With 25% bit dropout plus 4 spurious bits, a bar of 16 of 24 recruited 55,000 cells and destroyed recognition (13 bits per byte). A bar of 10 restored one byte per cell. Even then, rare low overlaps and Hebbian drift split each byte over duplicates, until [merging](sleep-and-consolidation.md) fixed most of it (3.18 bits per byte) ([11](../experiments/11-real-text.md)).
- **On clean input a bind pool is effectively exact hashing with a capacity.** The cost of "real" pools there is [capacity](capacity-and-budget.md), not recognition noise ([11](../experiments/11-real-text.md)).

## Parameters

| Pool | K (mask bits) | Recruit below | Capacity |
|---|---|---|---|
| Byte input | 24 of a 32-bit code | 10 | 192 |
| Bind | 48 of a 64-bit shift-bind input | 40 | 1024 |
| Chunk | 48 of 64 | 40 | 4096 |
| Toy concept area | 32 of 64 | 18 | 40 |

## Related

[Sparse codes](sparse-codes.md) · [Shift-bind](shift-bind.md) · [Sleep and consolidation](sleep-and-consolidation.md) · [Capacity and budget](capacity-and-budget.md) · [Engine model](../engine-model.md)

## Code

`Matcher`, `ByteReal`, `BindReal` and `consolidate` in `text_sim.py`; `pool.rs` in the Rust crate.

## Evidence in

[01 · early engine](../experiments/01-early-engine.md) · [03 · grow sleep](../experiments/03-grow-sleep.md) · [11 · real text](../experiments/11-real-text.md)

*[All concepts](README.md) · [Wiki home](../README.md)*
