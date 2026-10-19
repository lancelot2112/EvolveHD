# 21 · Word-like units for the level above (2026-10-18)

*[All experiments](README.md)* · [← 20](20-fair-screens.md)

**Concepts:** [Stepping stones](../concepts/stepping-stones.md) · [Growth operators](../concepts/growth-operators.md#chunk) · [Screens](../concepts/screens.md)

Question: in [20](20-fair-screens.md) the word-piece level kept order-2 context but never a chunk, while a brain over whole words grew holds and binds that beat order-2 counts. Is the block the units level 1 receives? First a short test: start the stack from whole words. Then: can level 0 hand up word-like units by itself, without being told what a word is?

All runs are single-seed, with experiment 20's fixes (mixture screens, fresh extensions) at every level, readout v3, and the upper-level readout settings (8-bit codes, aging every 1,024 writes, bias edge).

## 1. The short test: a stack that starts from whole words

`stack --syms` now lets level 0 read a symbol stream, here lower-cased words and punctuation from the three Austen novels (`--syms-vocab` gives each symbol's text).

| Level-0 vocabulary | "Other" | Level 0 bits per word (WB orders 1 / 2) | Level 0 keeps a chunk? | Level 1 (phrases) spawned | Level 1 bits (WB orders 0 / 1) |
|---|---|---|---|---|---|
| Top 512 words | 18% | 5.140 (5.207 / 5.102) | Yes, at 52k words (+0.0107) | Yes | 3.962 (3.837 / 3.894) |
| Top 64 words | 42% | 3.271 (3.392 / 3.251) | Yes, at 22k words | Yes | 6.049 (5.981 / 5.927) |
| Top 2,048 words | 6% | 6.981 (6.452 / 6.444) | Yes | Yes, at 123k words | 5.608 (4.768 / 4.938) |

- **With word units the stack climbs.** The word level keeps a chunk, and a phrase level spawns by itself. Level 1 of a byte stack, whose inputs are word-pieces, has never done that. Its symbols are runs of words cut where a word is surprising: "captain wentworth was ", "as soon as possible .", "dear miss woodhouse ,".
- **It doesn't depend on coverage.** With only 64 words and 42% "other", the chunk is still kept.
- **The phrase level learns nothing.** On 103k–121k phrases, even order-1 counts barely beat (or lose to) unigram counts, and the readout trails both. Section 4 has six times more phrases, with the same result.
- **On Austen the word level doesn't beat order-2 counts** in any row (5.140 against 5.102 at 512 words), and at 2,048 words it trails order-1 counts by 0.53 bits: the readout is the limit for big vocabularies. Only on the whole corpus does the word level beat order 2 (section 4).

## 2. Handing up word-like units from bytes

Level 0 hands up the run of bytes since its last cut. Two generic cut rules were tried, neither of which knows about spaces:
- **Surprise** (`--hand-up-z`): cut before a byte whose surprise is more than z sds above its running mean. This was the default before, with the chunk node's own z of 0.5.
- **Uncertainty peaks** (`--hand-up-entropy z`): cut where the entropy of the level's prediction, computed before the byte arrives, is at a local peak more than z sds above its running mean. Many continuations are possible there; this is branching entropy, a known unsupervised word-boundary cue.

How word-like the units are (`scripts/unit_shape.py`, weighted by occurrence: share ending at a word end, i.e. last byte not a letter; share shaped like one whole word, i.e. letters then non-letters). The units file for these runs held only the 20,000 commonest units, which cover 97–98% of occurrences for the first and third rows, 89% for entropy z 1, 68% for z 1.5 and 57% for surprise z 1.5. The units left out are rare and long. So the shares below describe the commonest units; the bounds over all units are in brackets where the file covers less than 95%. Mean lengths are exact (bytes after level 1 spawned ÷ units handed up).

| Cut | Mean length | End at a word end | One whole word | Several whole words |
|---|---|---|---|---|
| Surprise, z 0.5 (before) | 3.9 bytes | 52% | 31% | 8% |
| Surprise, z 1.5 | 11.1 | 51% (29–72%) | 21% (12–55%) | 8% |
| Entropy peaks, z 0 | 3.8 | 67% | 61% | 0% |
| **Entropy peaks, z 1** | 5.7 | **90%** (80–91%) | **83%** (74–85%) | 6% |
| Entropy peaks, z 1.5 | 9.5 | 97% (65–98%) | 73% (49–82%) | 21% |

The commonest units with entropy peaks at z 1 are words: "the ", "to ", "and ", "of ", "a ", "I ", "in ", "was ", "her ", with some prefixes ("re", "be", "de", "pro", "un").

**But level 1 keeps no chunk within the data** (three Austen novels, level-1 vocabulary 512 unless stated). Chunks were trialled and rejected in the surprise z 0.5, entropy z 0 (best +0.005) and slow-vocabulary runs. In the entropy z 1 runs with adaptive and fixed vocabularies, a chunk trial was still open when the data ended. In the others no chunk was trialled. The z 0.5 row is experiment 20's last configuration and reproduces it exactly.

| Cut, vocabulary | "Other" | Level 1 bits / WB0 / WB1 | Level 1 kept |
|---|---|---|---|
| Surprise z 0.5, adaptive (halflife 50k) | 35% | 5.326 / 5.994 / 5.204 | bias, order 2 |
| Surprise z 1.5, adaptive | 75% | 2.991 / 2.901 / 2.887 | bias |
| Entropy z 0, adaptive | 31% | 5.194 / 6.402 / 5.145 | order 2 |
| Entropy z 1, adaptive | 43% | 4.779 / 5.266 / 4.649 | bias, order 2 |
| Entropy z 1, slow (halflife 500k) | 44% | 4.660 / 5.143 / 4.513 | bias, order 2 |
| Entropy z 1, fixed (never reassigned) | 50% | 4.059 / 4.382 / 3.883 | bias |
| Entropy z 1, 2,048 slots (adaptive) | 31% | 6.739 / 6.812 / 6.057 | bias |
| Entropy z 1.5, fixed | 68% | 2.983 / 2.991 / 2.831 | bias |

"Order 2" is `(byte@2*byte)`, the previous unit bound with the current one. Bits are not comparable across rows: each row's stream is different.

## 3. Why words climb and these units don't

- **Probably not coverage:** whole words with 42% "other" keep a chunk; units with 31–50% don't.
- **Probably not vocabulary churn:** with a fixed vocabulary no chunk was kept either, though its trial was still open at the end, and that vocabulary (first come) also had more "other" (50% against 43%).
- **Not the stack:** the entropy-unit stream fed alone as level 0 of a new stack (`stack --syms` on level 1's saved stream) behaves identically: bias edge only, 4.048 bits per unit, a chunk trial still open at the end.
- **The streams differ in structure.** Order-2 context, beyond order-1 counts:

| Stream | Mixture gain | Kneser-Ney gain |
|---|---|---|
| Entropy units (z 1, fixed vocabulary) | +0.037 | +0.005 |
| Whole words, top 64 | +0.148 | +0.153 |

The whole-word stream has 4 times the order-2 structure by the mixture measure and 30 times by Kneser-Ney. A likely reason, not measured directly, is that the units are not consistent. About one in ten is a fragment (among the commonest units), and the same word may be cut differently in different contexts, which would scatter the very sequences a chunk captures. Another possibility, also untested: on unit streams the level-1 readout trails order-1 counts (4.059 against 3.883), while at 64 and 512 words it beats them. A higher threshold cuts inside words less (97% end at word ends at z 1.5), but then it merges words into many distinct multi-word units, and a 512-slot vocabulary covers only a third of the stream.

## 4. The stack from whole words on the whole corpus

Same settings, level 0 reading the whole corpus as words (top 512, 22% "other"), with growing real pools instead of exact nodes (with exact nodes, writer-mask rows grew at a rate that would have reached about 7 GB). `stack --syms runs/exp21/all_w512.syms --syms-vocab runs/exp21/all_w512.txt --grow-pools --pool-budget 524288 --levels 4 --vocab 512 …`, 62 minutes.

| Level | Symbols | Bits per symbol, second half | Witten-Bell orders 0 / 1 / 2 | Kept |
|---|---|---|---|---|
| 0: words | 2.66M | **4.710** | 6.091 / 5.045 / 4.921 | the previous word, holds on earlier words, a bind using the chunk |
| 1: phrases (spawned at word 66k) | 660k | 4.089 | 3.918 / 3.920 / 4.107 | bias |

- **The word level beats order-2 counts by 0.21 bits per word.** Its chunk was kept at 66k words (+0.0109) and pruned at 2.36M (+0.0006). A bind built on it, `(((byte@2*chunk(z=0.5))*byte)*(byte@2*byte))`, kept at 444k (+0.02), probably carries what it did.
- **The phrase level learns nothing.** Even order-1 counts gain nothing over unigram counts on its stream. "Other" is 64% of it, because phrases are so varied that 512 slots cover only a third of them. Its chunk screens peaked at +0.0008, so no level 2 appeared.

In these runs (one seed each) the stack climbed one level above the units it was given, then stalled. Each level up, the units get longer and more varied; even an ideal top-512 vocabulary would cover only 36% of the whole-corpus phrases. Coverage may not be the whole story, though: the phrase level over 64 words, with only 37% "other", learned nothing either.

## Findings

1. **Units were the block for the first climb.** Given whole words, the word level keeps a chunk and a phrase level spawns, on Austen and on the whole corpus, even with only 64 words and 42% "other".
2. **Uncertainty peaks find word-like units without being told what a word is:** among the commonest units, 90% end at a word end and 83% are one whole word (at least 80% and 74% over all units), against 52% and 31% for surprise cuts.
3. **But level 1 keeps no chunk over them.** The unit stream has a quarter (mixture) to a thirtieth (Kneser-Ney) of the whole-word stream's order-2 structure. Inconsistent cuts are the likely cause, but untested. No chunk was kept with adaptive, slow or fixed vocabularies, or with 512 or 2,048 slots, though in two runs a chunk trial was still open at the end.
4. **Above the first climb, the phrase level learns nothing,** even from 660k phrases. Phrases are too varied for one symbol each: with 512 slots, 64% of the whole-corpus phrase stream is "other". But a phrase level with only 37% "other" (over 64 words) learned nothing either, so coverage is likely only part of it.
5. **What it points to (untested):** units above the first level probably need codes that share structure, so that similar phrases overlap (for example, handing up a chunk's own code, a bundle of its parts, instead of a slot number), rather than one arbitrary symbol per unit.

## Code

- `brain.rs`: `Brain::from_syms`; `Brain::ent` (prediction entropy per tick, with running mean and variance); `completed_chunk` returns the cut position; `Cfg::hand_up_z`, `hand_up_entropy`, `vocab_fixed`.
- `stack.rs`: `Stack::from_syms`, `base_texts`; `VocabMap::with`, `fixed`.
- `main.rs`: `stack --syms FILE --syms-vocab TXT`; the stack writes `levelN.units.txt` (the units handed up, commonest first; capped at 20,000 entries in these runs, uncapped since).
- `scripts/unit_shape.py`.
- Flags: `--hand-up-z`, `--hand-up-entropy`, `--vocab-fixed`.
