# Sparse codes

Every node in EvolveHD speaks the same format: **one winning cell, shown as a fixed code of 32 set bits out of 2048.** Fixed sparsity is the impedance-matching rule. Any node's output can plug into any other node's input of the right type, because every input sees the same density.

## How it works

- Each cell of a node owns a random code: 32 distinct positions in 0..2048. Two random codes overlap by about 0.5 bits on average, so codes are nearly orthogonal.
- A node's output on a tick is the code of its winner (k-WTA with k = 1, see [matcher pools](matcher-pools.md)). Raw inputs (bytes, symbols) also get fixed random codes, so the front end knows nothing about the data.
- Because codes are fixed per cell, a downstream reader can learn *which cell* won from the bits alone. Edges then key on the winner's cell id, and the code is only how the identity travels.
- When a cell is re-recruited for something else, its **generation** counter bumps and it gets a new code. Readers reset what they learned about it.

## Why

- Binding and cleanup both need clean, fixed-density codes. Shift-bind on raw noisy input fell from 0.91 to 0.69 ([04 · audit sims](../experiments/04-audit-sims.md)).
- Fixed density is what lets the [growth controller](../growth-grammar.md) offer any node as a candidate for any target without type checks beyond the [channel](drive-and-prime.md).

## Parameters

| Parameter | Value |
|---|---|
| Width D | 2048 bits (32 u64 words) |
| Active bits per code | 32 |
| Expected overlap of two random codes | 32 × 32 / 2048 = 0.5 bits |

## Readout codes

The readout may use fewer bits of a target's code than the rest of the engine (`--code-bits k`, the first k of the 32 positions). Matching still sees all 32, so noise tolerance is unchanged. Sparser readout codes let a writer-mask row hold more successors. That pays when rows must hold many codes, as at a word-piece level: there 8-bit codes beat 32-bit codes by 0.1–0.2 bits per symbol, while at bytes they make no difference ([18](../experiments/18-sparser-codes.md)). See [writer-mask readout](writer-mask-readout.md#capacity-limit).

## Related

[Matcher pools](matcher-pools.md) · [Shift-bind](shift-bind.md) · [Writer-mask readout](writer-mask-readout.md) · [Primitives and channels](../primitives.md)

## Code

`rand_code` and `Node.new_code` in `text_sim.py`; `bits.rs` in the Rust crate.

## Evidence in

[04 · audit sims](../experiments/04-audit-sims.md) · [18 · sparser codes](../experiments/18-sparser-codes.md)

*[All concepts](README.md) · [Wiki home](../README.md)*
