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

With exact order-4 contexts this readout trails count-based backoff by about 0.22 bits per byte (2.18 against 1.96).

## Readout v2

About a third of that gap was mask capacity ([16](../experiments/16-readout-and-generation.md)). A 640-bit level holds about 20 target codes, and each write sets only 8 of the target's 32 bits, so frequent successors crowd out rare ones. Readout v2 uses:
- **1,024 bits per level and 16-bit writes.** Hand-wired order 4 went from 2.182 to 2.103 (with template rows), and grown exact nodes from 2.321 to 2.183 (full readout v2). Memory is unchanged.
- **Template rows:** new rows copy one random template per edge instead of drawing their own masks. Accuracy was unchanged on the hand-wired chain, and runs are 5–10% faster.
- **A learned confidence gain:** a second gain per edge on scores scaled by the row's Witten-Bell confidence n / (n + T). It gains 0.002–0.004; an earlier fixed confidence scale cost 0.011.

About 0.14 bits per byte remained between log-linear writer masks and Witten-Bell backoff over the same contexts. Readout v3 closes most of it.

## Readout v3

Most of the remaining error was noise in the counters, not capacity ([18](../experiments/18-sparser-codes.md)). A write used to reach level k as a whole, decided by one coin, so a successor seen a few times sat at a random height, give or take a level. Readout v3 (`--readout3`) uses:
- **Per-bit levels:** each bit of the written code reaches level k on its own coin, with probability 2^−k. The fraction of a code's bits at each level becomes a smooth count.
- **Rows that start empty**, so unseen successors score zero instead of a random background.
- **1,024-bit masks and writes of the whole code**, plus readout v2's learned confidence gain.

With the same order-4 contexts it is 0.016 bits per byte behind Witten-Bell counts (1.974 against 1.958), down from 0.14. The grown system on Austen, with growing real pools, beats order-4 counts on the last 500k bytes (1.843 against 1.878) and matches them over all three novels (1.948 against 1.944). At level 0 the two parts only help together: per-bit levels alone gained 0.004 on grown exact nodes, and empty rows alone cost 0.011 (one seed). On a word-piece stream, per-bit levels help on their own.

## Capacity limit

A target code is 32 bits out of 2,048, so codes crowd each other in a row that holds many successors. The readout's error grows with their number: learning a pure Zipf unigram, it trails counts by 0.06 bits per symbol at 16 symbols, 0.10 at 64 and 0.44 at 512. Byte contexts have 16–40 successors, so this costs little. A 512-symbol word-piece stream needs a 489-way distribution in its busiest row. There the readout's error (0.44) equals all the gain order-1 context offers, and a brain over that stream predicted it slightly worse than unigram counts (4.71 against 4.70; order-1 counts 4.25). That is why a stack of levels stopped one level above bytes ([17](../experiments/17-level-stack.md)).

Most of that error turned out to be counting noise, which readout v3 removes: on a 16-symbol unigram the error falls from 0.40 (default settings) to 0.03 bits. What is left does grow with outcomes, and there **sparser readout codes** help: the readout can use only k of each target's 32 bits (`--code-bits`). At 512 symbols, 8-bit codes cut the error from 0.43 to 0.24. What matters is how many bits a row must hold (outcomes × code bits) against its mask: with up to 128 Zipf symbols 32-bit codes are slightly better, so level 0 keeps them; upper levels of a stack use 8 ([18](../experiments/18-sparser-codes.md)).

Two smaller defects showed up on the way, both visible only in busy rows:
- **Saturation.** Levels written with probability 2^−k saturate at about 256 writes. Aging (all counts halve every 256 writes) or a coarser base fixes it.
- **Random background.** Rows that start full of random bits keep them when one successor dominates. Rows can start empty instead (`--sparse-rows`).

## History

- **Toy worlds ([06](../experiments/06-growth-interfaces.md)–[10](../experiments/10-full-primitives.md)):** one level of 48 bits (self) or 128–160 bits (context), with integer vote weights of 1 + gain / 0.05, capped at 4.
- **Weights:** without them, the only way to up-weight a source was to stack redundant edges. Weights halved the number of edges ([08](../experiments/08-bootstrapping.md)).

## Related

[Sparse codes](sparse-codes.md) · [Drive and prime](drive-and-prime.md) · [Shadow trials](shadow-trials.md) · [Credit without backprop](credit-without-backprop.md)

## Code

`Edge` and `TargetIndex` in `text_sim.py`; `edge.rs` in the Rust crate (`bitwise`, `TargetIndex::rk`).

## Evidence in

[01 · early engine](../experiments/01-early-engine.md) · [06 · growth interfaces](../experiments/06-growth-interfaces.md) · [08 · bootstrapping](../experiments/08-bootstrapping.md) · [11 · real text](../experiments/11-real-text.md) · [16 · readout and generation](../experiments/16-readout-and-generation.md) · [17 · level stack](../experiments/17-level-stack.md) · [18 · sparser codes](../experiments/18-sparser-codes.md)

*[All concepts](README.md) · [Wiki home](../README.md)*
