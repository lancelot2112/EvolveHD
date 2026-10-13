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

The rules these results forced are collected in [lessons](../lessons.md); the ideas behind them are in [concepts](../concepts/README.md); terms are in the [glossary](../glossary.md).

## Common setup

- **Codes:** 32 of 2048 bits per node output, unless stated.
- **Toy worlds:** 8-symbol nodes, 85% rule-following, top-1 accuracy as the measure. Chance is 0.125.
- **Text:** NLTK Gutenberg *Persuasion* (book A, 300k bytes) then *The Man Who Was Thursday* (book B, 100k bytes), scored prequentially in bits per byte.
- **Replay:** the [Grown Graph Explorer](https://claude.ai/artifact/24hUcx1C3WH2iZFNMFHSfJ) replays the growth runs from experiments 07 to 10 on a timeline.
