# Experiments

Every simulation so far, oldest first. Each page gives the question, setup, the numbers, the findings, and what it changed in the design. Results are from small Python sims (usually 4 to 8 seeds). Treat them as evidence for design choices, not as benchmarks.

| # | Experiment | Date | Headline result |
|---|---|---|---|
| 01 | [Early engine sims](01-early-engine.md) | Oct 2–3 | Sticky bits with echo give credit without backprop (98%). On real text the matcher engine matches a 3-byte lookup table, not a 5-byte one |
| 02 | [Basal-ganglia selection bandit](02-bg-selection-bandit.md) | Oct 10 | Exploration comes free from NoGo learning; Go LTD makes reversal as fast as first learning (final 0.99) |
| 03 | [Grow on novelty, compress in sleep](03-grow-sleep.md) | Oct 11 | Wake + sleep at a cap of 40 cells matches unbounded accuracy with 38 cells instead of 101; sticky bits make anti-Hebbian safe |
| 04 | [Audit sims](04-audit-sims.md) | Oct 11 | The two-mask critic rewards wrong choices; shift-bind 0.91 against XOR 0.30; keep reinstatement; plans must prime, not drive |
| 05 | [Claustrum consensus loop](05-claustrum-consensus.md) | Oct 11 | Consensus gives unity but locks; adding fatigue gives rivalry-like alternation that tracks evidence |
| 06 | [Typed interfaces and growth operators](06-growth-interfaces.md) | Oct 12 | Shadow trials end thrash; a compress node is grown from co-activity in 6/6 runs with a hidden topic, 0/6 without |
| 07 | [Generative wiring: inline against population](07-graph-growth.md) | Oct 12 | Screens + parallel trials + binds find all four structure types in 6/6 runs; a blind genetic search over 128 lifetimes finds one |
| 08 | [Bootstrapping depth](08-bootstrapping.md) | Oct 12 | Three levels solved in 6/6 runs with distill; nothing grows without stepping stones |
| 09 | [Temporal moves and attention](09-attention-temporal.md) | Oct 12 | Delay taps and holds reach ceiling; attention driven by the grammar's own screens beats round-robin (0.85 vs 0.79) |
| 10 | [All seven primitives](10-full-primitives.md) | Oct 13 | Correct 0.125 → 0.72; Select reward 0.23 → 0.83; broadcast rewires in 5k ticks against 12k; budget 0.966 against 0.932 |
| 11 | [Real text, real pools](11-real-text.md) | Oct 13 | Grows its own byte-context model: 2.61 bits per byte (real pools), 2.33 (exact nodes); beats gzip over a whole book |
| 12 | [Words: chunking by surprise](12-words.md) | Oct 13 | A context reset at surprising bytes is the most valuable edge on text (−0.08 bits per byte, real pools); its cuts are morph-like, 2.4× chance at word starts |
| 13 | [Word pairs: more data, then conjoin](13-word-pairs.md) | Oct 13 | With more data the previous word carries 0.2+ bits beyond the current chunk; a conjoin of the two is kept and cuts 0.05–0.09 bits per byte (exact nodes); successor classes fail; real pools lack the capacity |
| 14 | [The whole corpus](14-full-corpus.md) | Oct 13 | 11.7M bytes, 18 books: 2.56 bits per byte against 2.11 for order-4 counts; the gap doesn't close with data because real pools forget; distill pays for the first time (KJV Bible) |
| 15 | [Pools that grow on demand](15-growing-pools.md) | Oct 13 | Real pools that double under eviction pressure reach exact-node quality. Over the whole corpus: 2.125 bits per byte against 2.107 for order-4 counts, beating them on 10 of 18 books |
| 16 | [A better readout, and generation](16-readout-and-generation.md) | Oct 13 | Wider writer masks recover most of the readout gap (grown exact 2.321 → 2.183); on Austen the best system is 0.035 bits per byte behind order-4 counts; its continuations have words from the training text at the same rate as theirs |
| 17 | [A stack of levels](17-level-stack.md) | Oct 13 | A word-piece level spawns by itself, but predicts its stream no better than unigram counts: the readout's error grows with outcomes (0.44 bits at 512), so no third level appears |
| 18 | [Sparser codes, and counting with every bit](18-sparser-codes.md) | Oct 13 | Sparser readout codes help only where rows hold many codes; the main error was one coin per level per write. Per-bit levels close the readout gap at level 0 (0.016 bits per byte behind order-4 counts with the same contexts), and the grown system matches order-4 counts over three Austen novels (1.948 against 1.944). Level 1 learns its stream, but still no level 2 |
| 19 | [The stack on the whole corpus](19-whole-corpus-stack.md) | Oct 14 | Level 0 reaches 1.976 bits per byte against 2.107 for order-4 counts, beating them on 15 of 18 books. Level 1 still keeps no chunk: the screens' estimator hides order-2 information, and the level-1 vocabulary discards the content words that carry non-adjacent context (a recency cache is worth 0.13 bits per word on whole words) |
| 20 | [Fair screens, an adaptive vocabulary and a recency move](20-fair-screens.md) | Oct 17 | Screens that score a learned mixture help at every level: the whole corpus reaches 1.939 bits per byte (order-4 counts 2.107, beaten on 17 of 18 books), and a word-level brain beats order-2 counts by 0.225 bits per word with holds and binds over earlier words. The word-piece level now keeps order-2 context but still no chunk |

The rules these results forced are collected in [lessons](../lessons.md); the ideas behind them are in [concepts](../concepts/README.md); terms are in the [glossary](../glossary.md).

## Common setup

- **Codes:** 32 of 2048 bits per node output, unless stated.
- **Toy worlds:** 8-symbol nodes, 85% rule-following, top-1 accuracy as the measure. Chance is 0.125.
- **Text:** NLTK Gutenberg *Persuasion* (book A, 300k bytes) then *The Man Who Was Thursday* (book B, 100k bytes), scored prequentially in bits per byte.
- **Replay:** the [Grown Graph Explorer](https://claude.ai/artifact/24hUcx1C3WH2iZFNMFHSfJ) replays the growth runs from experiments 07 to 10 on a timeline.
