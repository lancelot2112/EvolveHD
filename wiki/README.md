# EvolveHD wiki

EvolveHD is a learning system built from high-dimensional bit vectors. Windowed matchers read them, fire on a popcount threshold, and adjust slowly. On top of that engine sits a **growth grammar**. The network starts as a few input pools and grows everything else from seven primitives, joined by typed channels. A structure is added only when a measured pressure calls for it, and kept only while a shadow trial shows it pays.

The goal is not to simulate a brain. It is a robust learner that uses the strengths of a computer and adapts to any kind of information. Brain anatomy is used as a source of working designs, not as a target.

## Where things stand (2026-10-13)

- **The grammar works on real text with no hand-listed candidates.** It grows its own byte-context model from raw bytes:
  - 2.61 bits per byte on the last third of *Persuasion*, with real matcher pools.
  - 2.33 with exact nodes, against 2.18 for hand wiring.
  - Over the whole book it beats gzip.
  - A new chunk move, a context reset at surprising bytes, takes real pools to 2.55 ([12](experiments/12-words.md)).
  - With exact nodes, a conjoin of the previous and current word-piece is kept, cutting a further 0.05–0.09 bits per byte: the first word-level structure ([13](experiments/13-word-pairs.md)).
  - Over the whole 11.7M-byte Gutenberg corpus, fixed real pools reached 2.56 against 2.11 for order-4 counts ([14](experiments/14-full-corpus.md)). Pools that grow on demand reach 2.125, beating order-4 counts on 10 of 18 books, and bzip2 (2.135) on the whole file ([15](experiments/15-growing-pools.md)).
- **All seven primitives exist as grammar moves** and pass their standalone tests: Expand, Associate, Compress, Gate, Select, Correct, Broadcast.
- **Open:** learning speed at size (every new cell needs its own writer-mask rows), the memory horizon of growing pools, the readout's gap to count models on long books, and scaling the screens to thousands of nodes.
- **The Rust crate** ports the text experiment and reproduces its table, about 10 times faster than Python ([reference code](reference-code.md#the-rust-crate)). The Python sims in [`reference/python`](../reference/python) remain the reference for everything else.

## Reading order

1. [Project history](history.md): how the design got here, phase by phase.
2. [Engine model](engine-model.md): vectors, matchers, tick semantics and learning rules (the layer underneath).
3. [Brain mapping](brain-mapping.md): what each brain area computes, and why that led to seven primitives.
4. [Primitives and channels](primitives.md): the seven primitives, the moves built from them, typed edges and invariants.
5. [Growth grammar](growth-grammar.md): screens, growth operators, shadow trials, attention, defaults.
6. [Concepts](concepts/README.md): one page per idea, from sparse codes to attention. Each has its mechanism, its evidence and its related concepts.
7. [Experiments](experiments/README.md): every simulation, with setup, numbers and what it changed.
8. [Lessons](lessons.md): the design rules the experiments forced.
9. [Open questions](open-questions.md).
10. [Glossary](glossary.md).
11. [Reference code](reference-code.md): the Python sims, how to run them, and how they map to the Rust crate.

## Start from a question

| If you want to know… | Read |
|---|---|
| How a structure gets added and kept | [Screens](concepts/screens.md) → [Shadow trials](concepts/shadow-trials.md) → [Growth operators](concepts/growth-operators.md) |
| How it learns without backprop | [Credit without backprop](concepts/credit-without-backprop.md), [Writer-mask readout](concepts/writer-mask-readout.md) |
| Why depth is reachable at all | [Stepping stones](concepts/stepping-stones.md), [08 · bootstrapping](experiments/08-bootstrapping.md) |
| How it handles time | [Time: delays and holds](concepts/time-delays-and-holds.md), [09](experiments/09-attention-temporal.md) |
| How it chooses what to learn | [Attention](concepts/attention.md), [Broadcast](concepts/broadcast.md) |
| How it does on real data | [11 · real text](experiments/11-real-text.md), [Capacity and budget](concepts/capacity-and-budget.md) |
| Which biology each piece came from | [Brain mapping](brain-mapping.md) |

## Living documents outside the repo

These are private to the project owner until shared.

- [Growth grammar spec](https://claude.ai/code/artifact/d37e3fa6-5a32-45a9-a6cc-88fcb7fc4d09): the current spec for this layer, with a diagram of the growth loop.
- [Reference engine spec (draft)](https://claude.ai/code/artifact/5d2ada31-74bc-4e4d-90e6-486c3efe362a): vectors, matchers, tick semantics (2026-10-02).
- [Grown Graph Explorer](https://claude.ai/artifact/24hUcx1C3WH2iZFNMFHSfJ): replays every growth run (primitives, binds, trials, keeps, prunes) on a timeline.
