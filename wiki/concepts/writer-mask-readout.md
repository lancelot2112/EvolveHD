# Writer-mask readout

A Prime edge predicts its target with bits, not weights. **Each source cell owns writer masks over the target's code space. A target cell's score is how much of its code those masks cover.** Graded strength comes from stacking masks that are written at different rates; edges are mixed log-linearly with learned gains.

## How it works

- **Score:** for source cell c and target cell j, s(j) = Σ over levels k of |M[k][c] AND code_j| / (32 L).
- **Write:** after each tick, each level k is written with probability 2^−k. Up to 8 bits of the actual next cell's code are set and as many bits outside it are cleared, so mask size stays fixed. The levels act as **stochastic binary counters**, and a successor's score grows with the log of how often it follows.
- **Mix:** p(j) ∝ exp(Σ_e β_e · s_e(j)). Each β is learned by gradient on log-loss with RMS-normalised steps. Negative gains are allowed and do occur; they correct for correlated edges.
- **Cost:** a count per (source cell, target cell) is updated incrementally through an owner table (bit → target cells), so scoring is a single gather.

## Why graded

One binary mask encodes a *set* of successors, not a distribution. That was enough in the toy worlds, where one successor dominates (85%), but not for text ([11](../experiments/11-real-text.md)). Order-1 byte model over 80k bytes:

| Readout | Bits per byte |
|---|---|
| 1 level × 160 bits | 4.11 |
| 1 level × 640 bits | 4.06 |
| 8 levels × 640 bits, 1-bit writes | 3.69 |
| 8 levels × 640 bits, 8-bit writes | 3.67 |
| Witten-Bell exact counts | 3.49 |

With exact order-4 contexts the readout still trails count-based backoff by about 0.22 bits per byte (2.18 against 1.96). That gap is an [open question](../open-questions.md).

## History

- **Toy worlds ([06](../experiments/06-growth-interfaces.md)–[10](../experiments/10-full-primitives.md)):** one level of 48 bits (self) or 128–160 bits (context), with integer vote weights of 1 + gain / 0.05, capped at 4.
- **Weights:** without them, the only way to up-weight a source was to stack redundant edges. Weights halved the number of edges ([08](../experiments/08-bootstrapping.md)).

## Related

[Sparse codes](sparse-codes.md) · [Drive and prime](drive-and-prime.md) · [Shadow trials](shadow-trials.md) · [Credit without backprop](credit-without-backprop.md)

## Code

`Edge` and `TargetIndex` in `text_sim.py`; `edge.rs` in the Rust crate.

## Evidence in

[01 · early engine](../experiments/01-early-engine.md) · [06 · growth interfaces](../experiments/06-growth-interfaces.md) · [08 · bootstrapping](../experiments/08-bootstrapping.md) · [11 · real text](../experiments/11-real-text.md)

*[All concepts](README.md) · [Wiki home](../README.md)*
