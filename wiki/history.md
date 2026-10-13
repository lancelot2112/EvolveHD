# Project history

The design moved in five phases. It went from a hand-built bitwise pattern matcher, through biology as a source of mechanisms, to a grammar that grows its own network and is tested on real text.

| Phase | Dates (2026) | Question | Outcome |
|---|---|---|---|
| 1. Matcher engine | Oct 2–3 | Can popcount-threshold matchers with bit-swap learning learn features, sequences and text? | Yes on toys. On real text it matches a 3-byte lookup table, not a 5-byte one. Engine spec and a Rust prototype written |
| 2. Brain circuits | Oct 10–11 | Can the basal ganglia, sleep, hippocampus and claustrum be built from the same bit operations? | Yes, with specific fixes: Go LTD, sticky bits plus sleep, reinstatement, consensus plus fatigue |
| 3. Primitives | Oct 12 | What do cortex, hippocampus, basal ganglia and cerebellum compute, and is there a small shared set? | Seven primitives joined by typed channels |
| 4. Growth grammar | Oct 12–13 | Can a network grow itself from those primitives, inline, with no hand-listed candidates? | Yes: screens plus shadow trials find pairs, chains, delays, memory, actions and brute-force fallbacks; attention picks its own stepping stones |
| 5. Real data and build | Oct 13 | Does it work on raw text with real pools? | It grows its own byte-context model and beats gzip. A surprise-reset chunk pays, but its units are not words. The Rust port reproduces the text table |

## Phase 1: the matcher engine (Oct 2–3)

The starting sketch: large bit vectors (about 2,048 bits) connected in a graph by **pattern matchers/writers** with windowed masks.
- A matcher fires when the popcount of its masked window passes a threshold, and inhibits matchers that overlap it.
- A fired-and-inhibited matcher gets an anti-Hebbian bit swap; a fired, uninhibited one gets a Hebbian swap.
- Outputs are written with OR, XOR, NOR, AND, NAND or ROTATE.

Explored in a long series of Python sims (results in chat, summarised here):

- **Swap rules and feature learning:** competitive sparse coding converges.
- **Credit assignment without backprop.** First explicit credit and blame vectors. Then the person's proposal: **sticky bits**. A bit becomes sticky on positive reinforcement; otherwise it is unstickied, or one random mask bit flips. A recurrent echo carries credit back through the loop.
- **Sequences and phase:** recurrent loops with per-matcher delays. Log-spaced delay taps let one system model both short and long interactions. Linear taps give more predictable throughput.
- **Generalised history:** each vector keeps a ring buffer as deep as its deepest reader. Reads can come from the past; writes only to the present.
- **Growth without fragmenting windows:** splitting vectors was rejected. Parallel sibling vectors that connect upstream the same way were preferred.
- **Text encodings:** character level, then a word code by XOR-and-shift of letter codes, then a prediction loop that guesses all next words. Then a two-level surprise-driven setup where higher-level predictions feed back down.
- **Transfer:** do rules learned for some words carry to words used the same way but never seen in that context? Replacing "punish until deleted" with **hit/miss track-record counters** fixed unreliable transfer and cut spawning churn about fivefold.
- **Input-reliability gating:** a malformed input is treated as a sensor fault, not as blame for the matcher. This is a form of credit assignment.
- **Real text:**
  - On *Alice in Wonderland*, the engine reached 43–46% top-1 byte accuracy against 58% for an online 3-byte lookup table. Matchers that hold a single guess were judged too harshly on probabilistic contexts.
  - With counters, on four Gutenberg books (2.34 MB), the engine matched or slightly beat the 3-byte table (0.561 vs 0.537 at 700k bytes) but trailed a 5-byte table (0.624).
- **Rust prototype:** three wake-up strategies for matcher evaluation were benchmarked. A lock-free, share-nothing sharding scheme gave bit-identical results to sequential execution. A Claude Code handoff (milestones M1 to M4) was written.

The [reference engine spec](https://claude.ai/code/artifact/5d2ada31-74bc-4e4d-90e6-486c3efe362a) (Oct 2) fixed vectors, matchers, tick semantics, collision policy and learning rules. See [engine model](engine-model.md).

## Phase 2: brain circuits from bit operations (Oct 10–11)

Four sims, each building one circuit from popcounts, masks and swaps:

- [Basal-ganglia bandit](experiments/02-bg-selection-bandit.md): Go/NoGo masks with a critic. Exploration comes free from NoGo learning; reversal needs Go depression on negative δ.
- [Grow and sleep](experiments/03-grow-sleep.md): recruit on novelty, compress in sleep. Sticky bits make anti-Hebbian safe; pruning never-consolidated cells does the compression.
- [Audit sims](experiments/04-audit-sims.md): four claims from an audit of the "current system" spec. Single-mask critic, shift-bind instead of XOR, keep context reinstatement, plans prime rather than drive.
- [Claustrum consensus](experiments/05-claustrum-consensus.md): a global consensus loop needs fatigue, or it locks.

## Phase 3: primitives (Oct 12)

A discussion of what cortex, hippocampus, basal ganglia, amygdala and cerebellum compute led to seven primitives:
- **Expand**, **Associate** and **Compress**, sorted by expansion versus convergence.
- **Select**, **Correct** and **Gate**, sorted by learning signal.
- **Broadcast**, sorted by scope.

Each area is the same few primitives with different settings and wiring. See [brain mapping](brain-mapping.md) and [primitives](primitives.md).

## Phase 4: the growth grammar (Oct 12–13)

Seven experiments built and tested a grammar that grows networks inline:

1. [Typed growth](experiments/06-growth-interfaces.md): shadow trials stop thrash; a compress node is grown from co-activity.
2. [Generative wiring](experiments/07-graph-growth.md): screens, parallel trials and bind operators find pair structure that single edges can't see; blind population search fails.
3. [Bootstrapping](experiments/08-bootstrapping.md): solving easy puzzles builds parts for hard ones, provided distill keeps representations small.
4. [Time and attention](experiments/09-attention-temporal.md): delay taps and event-gated holds; learning-progress attention driven by the grammar's own screens.
5. [All primitives](experiments/10-full-primitives.md): Correct, Select and Broadcast as moves; a give-up rule; fixes for priority hogging and wasted budget.

## Phase 5: real text and the build (Oct 13)

- [Real text](experiments/11-real-text.md): the grammar grows a byte-context model on two books, with real pools. Capacity shapes what it grows, structure transfers between authors, and noise needs merging.
- [Words](experiments/12-words.md): cutting the stream at surprising bytes gives a chunk context that pays (2.55 bits per byte with real pools). The chunks are morph-like runs, not words, and a previous-word context is invisible at this data size.
- [Word pairs](experiments/13-word-pairs.md): with running count screens, the previous word becomes visible. A conjoin of the two word-pieces is kept with exact nodes, after trials of many-state sources were allowed to run longer.
- [The whole corpus](experiments/14-full-corpus.md): 11.7M bytes in 44 minutes. Real pools stay 0.4–0.6 bits per byte behind order-4 counts, so capacity is now the limit.
- The [growth grammar spec](https://claude.ai/code/artifact/d37e3fa6-5a32-45a9-a6cc-88fcb7fc4d09) was written, and the text experiment was ported to Rust in this repo; it reproduces the Python table about 10 times faster.

The design rules these phases forced are collected in [lessons](lessons.md); what remains is in [open questions](open-questions.md).
