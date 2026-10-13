# 01 · Early engine sims (2026-10-02 to 10-03)

*[All experiments](README.md)* · [02 →](02-bg-selection-bandit.md)

**Concepts:** [Credit without backprop](../concepts/credit-without-backprop.md) · [Matcher pools](../concepts/matcher-pools.md) · [Time: delays and holds](../concepts/time-delays-and-holds.md) · [Writer-mask readout](../concepts/writer-mask-readout.md)

**Question.** Can popcount-threshold matchers with bit-swap learning, sticky bits and recurrent echo learn features, credit, sequences, timing and text?

**Where.** These ran in chat, and the scripts were not archived here. The numbers below come from the evidence table of the [reference engine spec](https://claude.ai/code/artifact/5d2ada31-74bc-4e4d-90e6-486c3efe362a) and from the chat summaries.

## Toy tasks

Sequence results read as simple steps / context three steps back / order-dependent case.

| Experiment | Result | Seeds |
|---|---|---|
| Feature learning, one window, 6 noisy patterns | Swap rules learned 100%. Anti-Hebbian cut duplicate pairs from 0.75 to 0. Spawn plus hysteresis used 6.8 matchers. After 4 new patterns: 100% with spawning, 68% with a fixed pool of 12 | 8 |
| Credit, two areas, 6-matcher budget | Credit and blame vectors 99%; global reward only 80%; no credit 73%; spread codes 74% | 12 |
| Sticky bits with echo, same task | 98% with two-bit unstick; 88% without the feedback loop | 12 |
| Sequences, delay line with credit | 97% / 99%; one-step context 99% / 46% | 10 |
| Sequences, recurrent loop with sticky bits | 93% / 96% / 86%; without the leak 82% / 76% / 32% | 6 |
| Per-matcher delays inside one loop | 92% / 91% / 81%, against a matched control at 64% / 96% / 48% | 6 |
| Longer gaps (shared run of 2, 4, 6) | Plain recursion 96%, 95%, 91%; random delays 94%, 84%, 72% | 4 |
| Unequal path lengths, three areas | Unaligned 50%; delay found by selection 100% for mismatches of 1 to 3 ticks (about 200 spawns with 4 delay options); holding the input instead 91%, 76%, 69% | 6 |
| Delay spacing | Linear 100%; exact log spacing 50% off powers of two for fast features, 93–100% within reach for slow ones; smeared bins worse than exact log | 4 |

Credit and blame vectors are a directed backward pass, which the design avoids. They are listed for comparison only; the spec uses sticky bits with echo.

## Text

- **Encodings tried:**
  - character level;
  - one code per word, made by XOR-and-shift of letter codes;
  - a prediction loop guessing all next words;
  - a two-level, surprise-driven setup where higher predictions loop back to the lower level.
- **Transfer test.** Do rules learned on some words carry to words used the same way but never seen in that context? The "punish until deleted" rule made transfer unreliable. **Hit/miss track-record counters** fixed it in every seed and cut spawning churn about fivefold.
- **Input-reliability gating.** A malformed input (well below its expected bit count) skips learning, rather than blaming the matcher.
- **Real text, *Alice in Wonderland* (153,860 bytes),** single pass, top-1 next-byte accuracy, first and last 25k bytes:

| Predictor | First block | Last block |
|---|---|---|
| Lookup table, 1 byte of context | 30% | 34% |
| Lookup table, 2 bytes | 43% | 46% |
| Lookup table, 3 bytes | 50% | 58% |
| Lookup table, 4 bytes | 51% | 62% |
| Engine, pool 5,000 | 33% | 43% |
| Engine, pool 20,000 | 32% | 44% |
| Engine, pool 5,000, milder miss penalty | 34% | 46% |

  Real prose is mostly non-deterministic. A matcher that holds one guess and is kept only when right more than two thirds of the time rarely survives. That makes about 90,000 spawns in 154,000 bytes, with 5–12% of the pool sticky. A bigger pool barely helped.
- **With counters, four Gutenberg books (2.34 MB):** the engine matched or slightly beat a 3-byte table (0.561 against 0.537 at 700k bytes) but trailed a 5-byte table (0.624).

## Rust prototype

- Three wake-up strategies for matcher evaluation were benchmarked.
- A lock-free, share-nothing sharding scheme gave bit-identical results to sequential execution: each thread owns a set of current-byte values and every matcher anchored on them.
- A Claude Code handoff with milestones M1 to M4 was written.

## What it changed

Sticky bits plus echo replaced backward credit. The leak and two-bit unstick became required. Track-record counters replaced kill-on-miss. Delays are chosen by selection. The real-text gap to count tables showed that a matcher holding a single guess is too weak a readout; the graded readout in [11 · real text](11-real-text.md) is the later fix.
