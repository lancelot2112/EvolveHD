# 32 · A learned, bitwise hippocampal store, and holographic memory (2026-10-21)

*[All experiments](README.md)* · [← 31](31-comprehension.md) · [33 →](33-learned-map.md)

**Concepts:** [Shift-bind](../concepts/shift-bind.md) · [Writer-mask readout](../concepts/writer-mask-readout.md)

Question: can the hippocampal store of [experiment 31](31-comprehension.md) learn what it was told? That store answered 98.4% of questions, but it was told which words are content words and where clauses end. Here nothing is hand-coded, and inside the store only bits and small integers are allowed: bitwise operations, add and subtract, shifts, counters and lookups. The word level's own float gains, which weigh what the store offers, stay as they are. The code calls these steps experiments 31b to 31e.

## The benchmark: held-out phrasings

`scripts/babi_gen2.py` writes the stories of experiment 31, but the second half also uses synonyms never seen in the first: *walked* and *travelled* for went and moved, *grabbed* and *took* for got and picked up, *put down* for dropped, *handed* for gave, and the question form "what place is X in ?" for "where is X ?". 817,666 words, 51 distinct words, 19,084 questions scored over the second half (6,889 asked in the new form; 9,996 whose fact used a new verb). Reference rows (`scripts/babi_score.py`):

| Model | Person | Object | Who has | All |
|---|---|---|---|---|
| Chance | 12.5% | 12.5% | 25.0% | 14.2% |
| Order-2 counts | 12.2% | 11.3% | 1.4% | 10.6% |
| Plain store of exp 31 (told content words and clause ends) | 99.6% | 95.9% | 98.8% | **98.9%** |
| Tracking rule (hand-written) | 100.0% | 98.9% | 100.0% | 99.8% |

The plain store handles the new phrasings at once: 99.7% on the new question form and 98.9% on facts with a new verb.

## Learned roles from meaning classes (53.7%)

The first fully learned store (`--epi-learned`) took away the content split and the clause ends. Each word is bound to the words of a fading 8-word window before it. A word's role is the online class of the meaning codes of the two words before it ([experiment 27](27-meaning-codes.md)), and the question form's class gates the recall channels. Over the whole benchmark it answers **53.7%** (8 role classes, 14,336-bit meaning codes), or 52.3% (32 classes, 2,048 bits). Few classes lump different slots together; many classes split the evidence over hundreds of channels.

## Why a window is not enough

A word-level simulation (exact word identities, so no code collisions; `runs/exp31d/sims/`) found the missing piece. Even cueing with the content words only, a fading window store answers about 59% of the questions, because it binds across clauses:

> daniel went to the bedroom . john went to the attic . where is john ?

Here the window binds john more strongly to *bedroom* (two words before him, across the full stop) than to *attic* (four words after, in his own clause). Clause structure is what the hand-coded store had for free.

**Learned segmentation** fixes it. For each word u, compare what comes before u with what comes after. A word after which the next word carries little information about the previous one is a boundary: I(previous; next | u) / min(H(previous), H(next)) is small. In the simulation, binding only back to the first boundary, and cueing with the content words only (an oracle), gives 99% on "where is X" and 100% on "who has". Replacing the oracle with learned attention over the cue positions, per context, gives 96–97% on "where is X".

## The bitwise store

`src/hippo.rs`, in the word level through `wordslots --hippo`. The store-only diagnostic is `evolvehd assoc --hippo`, scored by `scripts/hippo_chan.py`.

- **Codes:** each word has a fixed block code: 16 blocks of 256 bits, one bit set per block, from a hash.
- **Learned segmentation, as an integer test:**
  - For each word, three bit rows hold the code of the word seen just before it at one earlier occurrence, just after it, and of that pair.
  - Each row is rewritten with probability 1/16 per occurrence, a stochastic write, so it holds a random earlier draw.
  - A hit is an item all of whose bits are already there. Over n occurrences the hit rates are collision probabilities, 2^−H₂.
  - The word is a boundary when I / min(H) < 1/3, which works out to lo³ · hi² > n² · pair³ in hit counts (lo, hi: the smaller and larger of the before and after counts).
  - Comparing with the *last* occurrence instead of a random earlier one failed: repeated questions make consecutive occurrences look predictable.
  - Learned boundaries: "." and "is" on the small set; ".", "?" and "is" on the full one. "The" and "to" are correctly left out.
- **Memory:** two bit matrices (forward and backward), each a ring of 8 bit planes. Each word is bound to the words before it, back to and including the first boundary. A read takes each content bit from its newest plane only, so a later binding overrides an older one, with recency weights that halve per plane (shifts).
- **Ranking:** a recalled word is ranked by how much more it is recalled here than it is bound anywhere: the ratio of its score to its baseline (how often it has been bound at all), compared by cross-multiplying, with no division. This is the default, used in the full-benchmark runs. Ranking by excess share instead, s·B − b·S by subtraction (`--hp-excess`), did about as well on the small set. Cancelling what all channels recall in common was worse: the holder appears in several channels and cancels against itself.
- **Channels:** for each of the last 4 words and each direction, hop 1 (its partners), and hop 2 (the partners of its best partners, weighted by rank).
- **Choosing a channel, per context:** 8-bit counters per bit of the context's address (the codes of the words 1, 3 and 4 back, bound together) and channel. Two rules were tried: errors only (move toward a channel that had the next word among its top words when the chosen one did not), or each channel's hit rate as a running average in 8 bits, c += (255·hit − c) >> 4.
- **A learned type filter** (`--hp-typed`, tried on the small set only): each context's bit rows OR in the codes of the words that followed it. Offered lists keep only words that fit; hop-2 starting words are taken before the filter.

The full-benchmark runs below use the parts above except the type filter, with ratio ranking; the phase roles described further down are not in them either.

### Results

Full benchmark, through the word level, one seed each:

| Store | Person | Object | Who has | All | New form | New verb |
|---|---|---|---|---|---|---|
| Learned meaning classes | 51.8% | 38.0% | 80.6% | 53.7% | 60.2% | 53.7% |
| **Bitwise, error-driven channel choice** | 85.1% | 34.0% | 71.8% | 75.7% | 83.9% | 74.9% |
| **Bitwise, hit-rate channel choice** | 83.4% | 36.3% | 84.3% | **76.6%** | 81.1% | 75.7% |
| Plain store (told content words, clause ends) | 99.6% | 95.9% | 98.8% | 98.9% | 99.7% | 98.9% |

The bitwise store is the best fully learned one so far on this benchmark, and handles new phrasings early: on the first 20 / 100 / 500 questions in the new form, the error-driven version scores 80% / 89% / 81% and the hit-rate version 60% / 73% / 74%. It still trails the hand-coded store by 22 points. The biggest gap per question type is "where is the object" (36% against 96%), but most of the points are lost on "where is X", which is 72% of the questions.

Store alone, small benchmark (300 stories, 24,659 words). The share of questions whose channel has the answer as its first candidate of the right kind: the best single channel, then the channel the store picks.

| Version | Where is X | Where is the object | Who has |
|---|---|---|---|
| First (only "." learned, raw rank) | 60% / 30% | 52% / 33% | 92% / 28% |
| Random-draw segmentation, ranking against the baseline, top-4 inclusion | 63% / 91% | 65% / 40% | 84% / 60% |
| + phase roles, hop 2 from the clause's first word, type filter, hit-rate choice | 57% / 72% | 73% / 65% | 91% / 91% |

Through the word level on the same small set: plain store 84.2%, bitwise store 67.4% (no store: 12.4%). The small set understates everything: the plain store scores 98.9% on the full benchmark.

What the variants showed:
- **Phase roles:** a count of words since the last learned boundary (stopping at 7) is each word's role, bound by shifting its code's positions (unbound by shifting back). Alone they did not help: they spread each word's bindings over 8 roles. But hop 2 from the clause's first word (phase 0) found the holder: object → clause subject → place reached 73% on "where is the object", against 65% before.
- **Choosing the channel is the bottleneck.** The best channel often holds the answer, but the store picks it only when its learning signal agrees with "the answer's kind". The learned type filter supplies the kind: "who has" picked correctly 91% of the time.

## Holographic memory

The same store with the bit planes replaced by a quantized complex trace (`src/holo.rs`, `--hp-holo`):
- **Codes and binding:** each word has 4-phase codes (each element 1, i, −1 or −i: 2 bits). Binding key → value adds their phases mod 4.
- **Writing:** adds ±1 on the real or imaginary part of each element.
- **Levels:** each part is a small integer clipped to a number of levels. 4 levels is −2 … +1 (2 bits), 8 is −4 … +3, 16 is −8 … +7. Saturation is the forgetting: new writes wear old ones away.
- **Reading:** rotates the trace back by the key's phase (a swap and a sign change per element) and correlates with each value's code: add and subtract only.
- **Eight phases:** would need a factor of 1/√2. It can be done with shifts (181/256 = 0.70703, i.e. (x·181) >> 8), or with integer points (7,0), (5,5), which have radius 7 against 7.07.
- **A bug found on the way:** adding phases commutes, so one code per word made key → value and value → key the same entry, and forward and backward collapsed. Words now have separate key and value codes.

**How deep it remembers** (simulation): the age at which recall of the right value, among 50, falls below 80%:

| Elements | 8 levels (6 bits) | 16 levels (8 bits) | 32 levels (10 bits) |
|---|---|---|---|
| 512 | 8 | 31 | 0 |
| 1,024 | 22 | 45 | 3 |
| 2,048 | 29 | 79 | 81 |

About elements ÷ 17 to ÷ 26 at 16 levels, or 40–60 bindings per kilobyte. With too many levels nothing saturates, and recall collapses under the pile-up (the 0 and 3). Splitting the bits into a fast trace plus a slow trace that keeps a random quarter of the writes was worse than one trace with more levels.

**Sparse writes:** each key writes and reads only K of the D elements (its active set, from its hash):

| Elements | Levels | Written per binding | Depth at 80% | Read cost |
|---|---|---|---|---|
| 4,096 | 16 | all 4,096 | 126 | 4× |
| 4,096 | 16 | 1,024 | 207 | 1× |
| 16,384 | 16 | 1,024 | 711 | 1× |
| 65,536 | 16 | 1,024 | 3,048 | 1× |
| 65,536 | 8 | 256 | 3,539 | ¼× |
| 65,536 | 4 | 64 | 2,798 | 1/16× |
| 65,536 | 16 | 256 | 44 | ¼× |

Depth grows with the total size and cost with the active set, but only if the levels fall with K. Few levels make each element quiet, so fewer active elements are enough. Sparsifying alone, keeping 16 levels, collapses (44). (The first three rows come from `sparse.py`, the rest from `sparse2.py`, which draws active elements with replacement and smooths recall over 100 ages instead of 20.)

**Down to two and three levels** (65,536 elements, depth at 80%; `tern.py`):

| Levels per part | 16 active | 64 active | 256 active |
|---|---|---|---|
| 2: the sign of the last write (overwrite) | **2,433** | 1,761 | 785 |
| 3: ternary, −1, 0, +1 | 1,409 | **2,806** | 1,390 |
| 4: −2 … +1 | 0 | 2,798 | 1,844 |
| 5: −2 … +2 | 0 | **3,331** | 2,743 |
| 8: −4 … +3 | 0 | 38 | **3,539** |

- **Two levels is the bit memory again:** each part keeps the sign of its last write, so the newest binding wins outright, per element. It is the sparsest that works: 16 active elements give 2,433 bindings, about 150 per element read.
- **Three levels is ternary:** agreeing writes hold at ±1, conflicting ones cancel to 0 ("don't know") instead of leaving a wrong sign. It peaks at 64 active elements.
- **The more levels, the more active elements a binding needs;** symmetric ranges do a little better (5 levels against 4). A 0 means recall never reached 80%, even for the newest bindings.

**Orthogonalizing before storing** (simulation, 1,024 elements): random codes are nearly orthogonal already, so it does not help them. When value codes share a fraction of their elements, as meaning codes do, it restores capacity. Subtracting the mean code before storing, and from the codebook on the way out, does nearly all of it (77% → 93% at 120 pairs with 60% shared, whitening 95%).

In the store, small set (best single channel / picked), one seed:

| Memory | Where is X | Where is the object | Who has | Size |
|---|---|---|---|---|
| Bit planes | 58% / 79% | 67% / 53% | 88% / 88% | 32 MB |
| Dense trace, 4,096 elements, 64 levels, halved every 16 words | 52% / 72% | 47% / 40% | 76% / 74% | 12 KB |
| Sparse contextified trace, slow drift | 48% / 64% | 54% / 47% | 85% / 84% | 128 KB |
| Sparse contextified trace, faster drift | 50% / 69% | 44% / 44% | 74% / 72% | 128 KB |

- **Contextified keys** (`src/ctxholo.rs`, `--hp-ctxholo`): a word is written as a key in the current context. The context is 1,024 slots, each an element offset and a phase, and a few slots are redrawn per word and more per learned boundary. Keys from another story land on other elements, so recency comes from the drift, not from fading. It helps the facts asked about long after ("who has": 76% → 85%), not a person's latest room, whose previous room sits in nearly the same context.
- **Superseding** (`--hp-holo-sup`): before a new binding, subtract the value the key clearly held in that role. On a 6,000-word slice it fired on about every second word even at a 90% threshold (and 2.5 times per word at the 30% threshold used in the small-set runs): frequent words always "clearly hold" something. It did not help.
- The traces use a 256th to a 2,700th of the bit planes' memory (sizes counted at the bits per part their levels need) but trails them on every question type. The bit planes' "newest plane wins" already does what superseding was meant to.

## Findings

1. **Clause structure can be learned, with integer counters:** a word is a boundary when what follows it carries little of what came before, measured by collisions against a stochastically kept earlier draw. It finds "." and "is" (and "?" on the full benchmark) and leaves "the" and "to" alone.
2. **A fully learned, bitwise store answers 76.6%** on stories with held-out phrasings (the hand-coded store 98.9%; the earlier learned store 53.7%), and handles new phrasings at once.
3. **Choosing which channel to trust is now the bottleneck,** not memory: the right channel often has the answer. A learned filter of which words follow each context helps it most.
4. **Holographic traces store far more per bit,** most when writes are sparse and levels few (65,536 elements, 64 active, 4 levels: about 2,800 bindings deep at a 16th of the read cost). But in this store they lose to bit planes on recency.

Caveats: one seed throughout. Store-only numbers are on the small set. The simulations use random keys, while bAbI repeats a few keys ("john", "the") over and over.

## What next

- Choosing the channel with the word level's own expectation of the next word's kind.
- Remapping the context at learned episode changes (place-cell style) instead of steady drift.
- Storing [experiment 33](33-learned-map.md)'s place codes in a sparse trace.

## Code

- **Store:** `src/hippo.rs` (`--hippo`; `--hp-*` flags in `main.rs`: segmentation `--hp-seg-r`, `--hp-ctx-bind`, `--hp-excess`, `--hp-att-rate`, `--hp-khit`, `--hp-typed`, `--hp-phase`, `--hp-hop2-ph`, `--hp-k2`, `--hp-front-rank`).
- **Traces:** `src/holo.rs` (`--hp-holo D`, `--hp-holo-levels`, `--hp-holo-decay`, `--hp-holo-mag`, `--hp-holo-sup`) and `src/ctxholo.rs` (`--hp-ctxholo D`, `--hp-ch-k`, `--hp-ch-drift`, `--hp-ch-bdrift`).
- **Earlier attempts:** `src/assoc.rs` (dense block-code associative store, 31b) and `src/episodic.rs` (`--epi-learned`).
- **Scripts:** `scripts/babi_gen2.py`, `scripts/babi_score.py`, `scripts/assoc_score.py`, `scripts/hippo_chan.py`.
- **Runs:** `runs/exp30b/` (benchmark, learned meaning classes), `runs/exp31c/` (full-benchmark runs; `small/` has the small-set runs), `runs/exp31d/sims/` (the simulations).
