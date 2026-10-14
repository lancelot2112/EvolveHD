# 17 · A stack of levels (2026-10-13)

*[All experiments](README.md)* · [← 16](16-readout-and-generation.md)

**Concepts:** [Stepping stones](../concepts/stepping-stones.md) · [Writer-mask readout](../concepts/writer-mask-readout.md#capacity-limit) · [Growth operators](../concepts/growth-operators.md#chunk) · [Drive and prime](../concepts/drive-and-prime.md)

Question: growth stopped at word pairs because every level was judged by the next byte, and its moves were hand-written for bytes. If each level gets its own target, and the same moves run at every level, does a third level appear by itself?

## The design

- **Every level is the same brain, over its own symbol stream.** Level 0 reads bytes. When a level has kept a chunk, each completed chunk becomes a symbol of the next level's stream. The first 512 chunks seen twice get their own symbol; the rest share one "other" symbol. A new level is spawned to predict that stream, with its own target, surprise, screens, trials and growth. A chunk kept at level 1 (a phrase) would spawn level 2.
- **Top-down input is an ordinary candidate.** Each level's predicted next symbol is offered to the level below as an "upper" node. It is screened, trialled and conjoined with the chunk like anything else.
- **No level-specific code.** The chunk, previous-word and conjoin rules run at every level against that level's input. The brain became generic over a vocabulary of symbols; level 0 still reproduces every earlier number exactly.

Command: `evolvehd stack --data ... --levels 3 --vocab 512 [text flags]`; `evolvehd syms` runs one level alone on a saved symbol stream.

## What happened

Exact nodes, `--readout2 --words --run-window 200000 --conjoin-words --long-trials 5`:

| Run | Level 1 spawned | Level 1 symbols | Level 1 bits per symbol (second half) | Witten-Bell orders 0 / 1 / 2 on the same symbols | Level 2 |
|---|---|---|---|---|---|
| Two books (400k bytes) | byte 22,001 | 96,140 | 4.966 | – / 4.437 / 4.620 | No |
| Three Austen novels (2M bytes) | byte 22,003 | 483,299 | 4.710 | 4.696 / 4.253 / 4.290 | No |

- **Level 1 spawned by itself** in every run, as soon as level 0 kept its chunk.
- **Level 1 learned almost nothing.** On 483k word-pieces it predicts its stream slightly worse than unigram counts (4.710 against 4.696), while order-1 counts get 4.253. It kept one delay tap and nothing else. Only 488 of its 512 symbol slots were ever assigned; the shared "other" symbol is 48% of the stream.
- **So level 1 grew no chunk, and there was no level 2.** Its chunk screens were negative at every check (−0.29 to −0.01 bits on Austen).
- **The top-down node never helped.** Measured beyond the chunk at level 0, it screened between −0.10 and −0.02 bits, and all 13 of its trials were rejected. Level 0 was about the same with or without the stack: 2.051 against 2.038 over all of Austen, 1.899 against 1.903 on the last 500k.

## Why level 1 can't learn: the readout's error grows with the number of outcomes

The test bed: one level-1 brain alone (`evolvehd syms`) on a saved level-1 stream, with growth off, so only the order-1 edge (current symbol → next) learns, plus, where stated, an order-0 "bias" edge from a constant source (`--bias-edge`), whose one row learns the overall symbol frequencies.

**A 64-symbol stream** (one book; the shared "other" symbol is 71.5% of it). Unigram counts 2.303, order-1 counts 2.237:

| Readout | Bits per symbol |
|---|---|
| Default (640-bit masks, 8-bit writes) | 2.848 |
| 1,024-bit masks, 16-bit writes | 3.108 |
| 300-bit masks | 2.387 |
| Rows start empty ("sparse rows") | 3.018 |
| Sparse rows, counters at base 3 instead of 2 | 2.355 |
| Sparse rows, rows age (all counts halve) every 256 writes | 2.349 |
| + a bias edge | 2.340 |

Two defects showed up in busy rows:
- **Counter saturation.** A writer mask's levels are stochastic counters that saturate at about 2^8 = 256 writes. The six busiest of the 63 rows take over 1,000 writes each, and with "other" at 71.5% they carry most of the stream. A coarser base or aging (together with sparse rows) takes 2.848 to about 2.35; smaller masks alone (300 bits) reach 2.387.
- **Background noise.** New rows start full of random bits. In a row dominated by one successor, nothing pushes them out, so unseen symbols keep random scores. Sparse rows start empty.

Neither fix gets below unigram counts.

**The 512-symbol stream** (Austen). Unigram counts 4.696, order-1 counts 4.253. Every setting gives 4.72–4.89: default 4.782, readout2 4.830, sparse rows with aging every 256 writes 4.890, the same with 1,024-bit masks 4.821, a bias edge 4.720.

**A clean test of capacity.** Synthetic streams of independent symbols drawn from a Zipf distribution (200k symbols each); the only structure is the unigram, which a bias edge has to learn:

| Distinct symbols | 16 | 32 | 64 | 128 | 256 | 512 |
|---|---|---|---|---|---|---|
| Unigram counts (bits per symbol) | 3.406 | 4.156 | 4.866 | 5.561 | 6.210 | 6.880 |
| Bias edge, best setting | 3.466 | 4.234 | 4.963 | 5.694 | 6.454 | 7.321 |
| Gap | 0.06 | 0.08 | 0.10 | 0.13 | 0.24 | 0.44 |

The readout's error grows steadily with the number of outcomes a row must hold. A target code is 32 bits out of 2,048, so codes crowd each other in a row once it holds many successors. In the 512-symbol word-piece stream, the "other" row has 469 distinct successors; the other rows have 95 on average (weighted by use), with about 30 covering 90% of their mass. Byte contexts have 16–40, with 6–12 covering 90%.

On that stream, order-1 counts gain 0.44 bits over unigram counts, and the readout loses about 0.44 just representing a 512-way unigram. The information level 1 could use is eaten by the readout's own error, so its surprise is uninformative, its chunks don't pay, and its predictions are no use to the level below.

Smaller vocabularies don't escape this either, at least with readout2: with 64 and 128 symbols the "other" symbol dominates and level 1 still trails unigram counts (2.865 against 1.960; 2.826 against 2.662).

**On level 0** these options are mixed and stay off by default. Sparse rows improve the hand-wired chain's book A last 100k (readout2: 2.099 → 2.069), but cost a little there on grown runs (exact 2.183 → 2.194, real 2.518 → 2.532). Aging hurts (exact 2.319).

## Findings

1. **The stack works as machinery.** Levels spawn themselves from a kept chunk and run unchanged code; top-down input is just another candidate.
2. **No third level appeared, because level 1 cannot predict its own stream.** The writer-mask readout's error grows with the number of outcomes per row: 0.06 bits at 16, 0.44 at 512 on a pure unigram. On the word-piece stream that error is as large as all the structure there is to find.
3. **This is the next design problem, and it is in the engine, not the grammar.** Higher levels need a bigger code space (wider vectors or sparser codes), or a factored readout: predict a word-piece through a few small steps instead of one choice among hundreds.
4. **Two readout defects were found on the way:** counters saturate on busy rows, and random background never clears in rows with one dominant successor. Sparse rows and aging reduce them, but neither gets level 1 below unigram counts, and they help level 0 only on the hand-wired chain.

## Code

`stack.rs` (`Stack`, `VocabMap`); `brain.rs`: `new_symbols`, `push`, `predict_cell`, `completed_chunk`, `add_upper`, `Kind::Upper`; `edge.rs`: `sparse`, `age_every`, `EVOLVEHD_LEVEL_BASE`; `brain.rs`: `Kind::Const` (`--bias-edge`); `baseline.rs`: `witten_bell_bits_syms`. Flags: `--sparse-rows`, `--age-every`; commands `stack` and `syms`.
