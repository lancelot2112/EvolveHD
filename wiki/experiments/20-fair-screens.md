# 20 · Fair screens, an adaptive vocabulary and a recency move (2026-10-17)

*[All experiments](README.md)* · [← 19](19-whole-corpus-stack.md) · [21 →](21-word-units.md)

**Concepts:** [Screens](../concepts/screens.md#estimator-bias) · [Shadow trials](../concepts/shadow-trials.md) · [Stepping stones](../concepts/stepping-stones.md) · [Writer-mask readout](../concepts/writer-mask-readout.md)

Question: [19](19-whole-corpus-stack.md) named three likely reasons a word-piece level keeps no chunk. Its screens use an estimator that hides sparse contexts; its first-come vocabulary is half "other"; and no move builds non-adjacent context. Fix each one: what changes?

All runs are single-seed, except one check: seed 1 repeats the main byte-level effect (grown exact nodes, old 2.081 / 2.459 / 2.342 / 2.408, mixture 1.990 / 2.443 / 2.300 / 2.294). Differences of 0.02 or less below are within what one seed can show.

Naming: `byte@2` is the symbol one back from the current one, `byte@3` two back (the "byte" node is a level's own input, whatever its symbols are).

## The fixes

1. **Fair screens (`--screen-mix`).** Every screen scores a learned mixture of the baseline prediction and the candidate's. That covers the running count screens, the held-out screens, and the delay, hold, bind and node screens: (1 − λ)·p(y′ | baseline) + λ·p(y′ | candidate, baseline).
   - The running screens learn λ online by an EM step.
   - The windowed screens pick λ from a grid on a fifth of the held-out half, then score the rest.
   - A weight of 0 is always available, so a candidate with no information scores about zero instead of being charged for sparse contexts. The threshold is therefore lower (`--mix-thr`, 0.005 bits).
   - Before, the delay, hold and bind screens used plug-in mutual information with a shift null. That works for bytes, but is noise once a context has hundreds of thousands of states.
2. **An adaptive vocabulary (`--vocab-halflife N`).** The symbols one level hands up are keyed by chunk text, not by chunk cell. Counts halve every N chunks. A text seen twice takes a free slot, or the slot of the weakest symbol if its count is more than twice that symbol's. A reassigned symbol gets a fresh code, so the level above treats it as a new input.
3. **A recency move (`--recency W`).** A candidate edge from a constant source whose single writer-mask row holds about the last W codes (W × code bits, starting empty), so it scores what was seen recently: a cache, in bits.

Two smaller changes came out of testing:
- **Fresh extensions (`--fresh-extensions`):** an extended trial is judged on its gain since the last extension, not since it began.
- **No constant parts in binds:** a bind never takes a constant node as a part.

All are off by default; every earlier result still reproduces.

## 1. Fair screens

**At the byte level** (two books, book A last 100k / book B first 50k after A / book B 50–100k / book A whole):

| System | Old screens | Mixture screens |
|---|---|---|
| Grown, exact nodes | 2.079 / 2.471 / 2.353 / 2.405 | **1.991 / 2.451 / 2.323 / 2.294** |
| + chunks, conjoin, extensions (exact) | 2.007 / 2.423 / 2.286 / 2.322 | 1.960 / 2.403 / 2.257 / 2.266 |
| + fresh extensions | 2.005 / 2.479 / 2.320 / 2.321 | **1.943 / 2.385 / 2.237 / 2.255** |
| Growing real pools + chunks, conjoin; right: + fresh extensions | 2.027 / 2.509 / 2.376 / 2.333 | 1.955 / 2.402 / 2.267 / 2.261 |
| Witten-Bell order 4 | 1.958 / 2.586 / 2.385 / 2.153 | |

All with readout v3. In the "+ fresh extensions" row, the left column has fresh extensions with the old screens, and the right column has both. Mixture screens are the main effect. Fresh extensions add a little on top of them, but with the old screens they hurt book B (2.423 → 2.479). On book A's last 100k the grown system now beats order-4 counts (1.943 against 1.958), a margin within seed noise.

**Three Austen novels, exact nodes, chunks and conjoin** (level 0 of the stack runs below):

| | *Emma* | *Persuasion* | *Sense and Sensibility* | Last 500k | Whole |
|---|---|---|---|---|---|
| Witten-Bell order 4 | 2.008 | 1.904 | 1.887 | 1.878 | 1.944 |
| Old screens ([18](18-sparser-codes.md)) | 2.073 | 1.876 | 1.852 | 1.840 | 1.955 |
| **Mixture screens + fresh extensions** (and the stack's adaptive vocabulary above) | 2.021 | **1.822** | **1.804** | **1.795** | **1.903** |

**On whole words** (the whole corpus as lower-cased words and punctuation, the 512 commonest, 22% "other"; growth on, upper-level readout settings):

| | Bits per word, second half |
|---|---|
| Witten-Bell orders 1 / 2 | 5.045 / 4.921 |
| Bias edge only, no growth | 4.877 |
| **Mixture screens, grown** | **4.696** |

The grown brain beats order-2 counts by 0.225 bits per word. At the end it held:
- `(byte@2*byte)`: the current word with the one before (order 2);
- `((hold(byte@2|#s19)*byte)*(byte@2*byte@3))`: a hold that latches the previous word whenever symbol 19 appears, combined with the two words before the current one;
- `hold(byte@3|#s31)`, `(hold(byte@3|#s31)*(byte@2*byte))` and `(byte*hold(byte@3|#s25))`: holds that latch the word two back whenever a particular word appears, alone and bound with recent words.

(The run's gains per node were not saved. A companion run with recency candidates, 4.703 bits per word, kept and later pruned similar structures, e.g. `(hold(byte@3|#s14)*byte)` at +0.028.)

That is non-adjacent context, grown by the grammar's own moves. With the old screens, the Austen word stream (512 words) offered only delays of 6–16 (and about 15 holds on them, all rejected) and kept nothing beyond the bias edge: 5.326 bits per word, against 5.150 with mixture screens (order-1 counts 5.207, order 2 5.102).

**At the word-piece level** (the saved level-1 stream from [18](18-sparser-codes.md), Austen, 513 symbols):
- With mixture screens and a 100k window, the delay screen offers the previous symbol and the bind screen offers order-2 context, `(byte@2*byte)`.
- Each bind trial gained 0.0035, 0.0038, 0.0051, 0.0053 bits: rising, but under the 0.01 bar, and every rejection rebuilt the bind from nothing.
- Under the old rule, which judges the mean since the trial began, a 20× extension rejected the bind at 0.0061, and a 40× extension never reached a verdict, holding a trial slot all run. The early ticks of a slow learner pulling the mean down is the likely reason, not tested directly.
- With fresh extensions (up to 20× the trial length), the order-2 bind is kept, and level 1 improves from 4.50 to 4.465 bits per symbol (order-1 counts 4.329).

## 2. The adaptive vocabulary

On Austen, with a halflife of 50,000 chunks (run `stack_fix_b` of section 4):
- 437 slots were given a new text over the run.
- "Other" fell from 46% to 36% of the level-1 stream.
- The vocabulary at the end holds names from the book being read: "Willoughby", "Mrs. Jennings", "Edward", "ucy " (Lucy).

The stream carries more information: unigram counts 5.96 bits per symbol against 4.81, order-1 counts 5.20 against 4.33. Kneser-Ney order 2 gains 0.017 bits over order 1, and a mixture 0.062.

## 3. The recency move

| Whole-corpus words (512), growth off | Bits per word |
|---|---|
| No bias edge | 4.967 |
| + recent(32) | 4.888 |
| + recent(16) and recent(64) | 4.875 |
| Bias edge as configured for upper levels (1,024-bit row, halving every 1,024 writes) | 4.877 |
| + recent(32) | 4.877 |
| Bias edge with a slow memory (2,048 bits, halving every 65,536 writes) | 4.932 |
| + recent(32) | **4.876** |

- **Recency is worth 0.056 bits per word over a slow-memory bias edge,** 0.079 over no bias edge, and nothing over the fast one. For comparison, [19](19-whole-corpus-stack.md)'s mixture test on this 512-word stream gives a count-based cache +0.036 beyond order-1 counts (a different baseline).
- **The engine already had it.** At upper levels the bias edge forgets fast enough to act as a cache: with it, a recency edge adds nothing. Only with a slow-memory bias edge does the recency edge pay.
- In growth runs the recency candidates were never kept: on Austen words their trials gained at most 0.004 bits, on whole-corpus words at most 0.0088. In those runs the bias edge itself was pruned early (at tick 91k), so most recency trials ran without it. Why they still fell short of the 0.01 bar is not settled.
- A row of 2,048 bits holds at most 256 eight-bit codes, and far fewer before unseen codes overlap it heavily. recent(1000) was capped at the full row; its first trials gained 0.004 and later ones about nothing.

## 4. The stack on Austen

`stack --data runs/austen3.bin --symbolic --words --run-window 200000 --conjoin-words --long-trials 5 --readout3`, levels above 0 with `--code-bits 8 --write-bits 8 --age-every 1024 --bias-edge`:

| Run | Level 0 whole | Level 1 bits / WB0 / WB1 | Level 1 kept | Level 2 |
|---|---|---|---|---|
| Old screens ([18](18-sparser-codes.md)) | 1.955 | 4.501 / 4.816 / 4.329 | bias | No |
| Upper levels: mixture screens, 100k window, fresh 20× extensions, recency 100, 1000 | 1.954 | 4.464 / 4.812 / 4.329 | bias, `(byte@2*byte)` | No |
| The same + adaptive vocabulary | 1.952 | 5.310 / 5.958 / 5.201 | bias, `(byte@2*byte)` | No |
| Mixture screens and fresh extensions at every level + adaptive vocabulary (no recency) | **1.903** | 5.326 / 5.994 / 5.204 | bias, `(byte@2*byte)` | No |

Level-1 bits per symbol can't be compared across vocabularies: the adaptive one hands up a stream with more information in it.

Level 1 now keeps order-2 context in every configuration, but no chunk. Its chunk screens with mixture scoring reach +0.005 to +0.006 (with the old screens at most −0.017), so chunks are offered and trialled, and their trials gain nothing (−0.0007 to 0.0000).

## 5. The stack on the whole corpus

The run: `stack --data corpus/gutenberg_all.bin --words --run-window 200000 --conjoin-words --long-trials 5 --grow-pools --pool-budget 524288 --readout3 --screen-mix --fresh-extensions --vocab-halflife 50000 --levels 3 --vocab 512 --upper "--code-bits 8 --write-bits 8 --age-every 1024 --bias-edge --window 100000 --long-trials 20"`. It took 3 h 51 min, against 94 min in [19](19-whole-corpus-stack.md): the screens now offer many more candidates (1,177 trials and 866 binds at level 0).

The first attempt ran out of memory. Removed nodes kept their whole matcher pool, which hardly mattered while few binds were built, but mixture screens build many. Removed nodes now drop their masks, index and caches, keeping only the codes and generations that live nodes may still read. Results are bit-identical.

**Level 0** (bits per byte; both columns from 1,000-byte block means, as in [19](19-whole-corpus-stack.md)):

| Book | [19](19-whole-corpus-stack.md) | This run | WB order 4 | This run − WB4 |
|---|---|---|---|---|
| Austen, *Emma* | 2.095 | 2.038 | 2.008 | +0.03 |
| Austen, *Persuasion* | 1.918 | 1.879 | 1.904 | −0.03 |
| Austen, *Sense and Sensibility* | 1.899 | 1.870 | 1.887 | −0.02 |
| King James Bible | 1.708 | 1.680 | 1.900 | −0.22 |
| Blake, poems | 2.457 | 2.499 | 2.817 | −0.32 |
| Bryant, *Stories to Tell to Children* | 2.098 | 2.065 | 2.293 | −0.23 |
| Burgess, *Buster Bear* | 1.952 | 1.905 | 2.287 | −0.38 |
| Carroll, *Alice* | 2.124 | 2.059 | 2.354 | −0.30 |
| Chesterton, *The Ball and the Cross* | 2.195 | 2.149 | 2.304 | −0.16 |
| Chesterton, *The Wisdom of Father Brown* | 2.128 | 2.089 | 2.246 | −0.16 |
| Chesterton, *Thursday* | 2.094 | 2.066 | 2.198 | −0.13 |
| Edgeworth, *The Parent's Assistant* | 2.055 | 2.009 | 2.130 | −0.12 |
| Melville, *Moby Dick* | 2.251 | 2.211 | 2.342 | −0.13 |
| Milton, *Paradise Lost* | 2.267 | 2.238 | 2.465 | −0.23 |
| Shakespeare, *Julius Caesar* | 2.421 | 2.301 | 2.849 | −0.55 |
| Shakespeare, *Hamlet* | 2.322 | 2.263 | 2.719 | −0.46 |
| Shakespeare, *Macbeth* | 2.281 | 2.251 | 2.689 | −0.44 |
| Whitman, *Leaves of Grass* | 2.236 | 2.201 | 2.408 | −0.21 |
| **Whole stream** | 1.976 | **1.939** | 2.107 | −0.17 |

It beats order-4 counts on 17 of 18 books and improves on [19](19-whole-corpus-stack.md) on 17 (all but Blake). The word-pair conjoin was kept at 71k ticks, then pruned and re-kept several times.

**Level 1** (3.0M word-pieces):

| | Bits per symbol, second half |
|---|---|
| Level 1 | 5.596 |
| Witten-Bell orders 0 / 1 / 2 / 3 | 5.790 / 5.387 / 5.472 / 5.777 |

- **The adaptive vocabulary** gave 2,156 slots a new text over the run, and "other" fell from 54% ([19](19-whole-corpus-stack.md)) to 37%.
- **Level 1 kept order-2 context** (`(byte@2*byte)`, at 271k level-1 ticks), but uses only about half of what order-1 context offers (0.19 of 0.40 bits), against 70% in [19](19-whole-corpus-stack.md). A likely cause is the vocabulary's churn: each reassigned symbol starts its rows over.
- **No chunk and no level 2.** Level 1's chunk screens reached +0.008. Its 28 chunk-related trials gained at most +0.005, and all were rejected.
- **The top-down node** had 33 trials at level 0, the best at +0.0027; none was kept.

## Findings

1. **Fair screens help at every level.**
   - Bytes: grown exact nodes on two books go from 2.079 to 1.991 bits per byte (book A, last 100k), confirmed on a second seed.
   - Austen: level 0 beats order-4 counts over the three novels together (1.903 against 1.944).
   - Whole corpus: level 0 reaches 1.939 bits per byte against 2.107 for order-4 counts (1.976 before), beating them on 17 of 18 books. This run also changed the vocabulary and the levels above, so the step is not attributed to the screens alone.
   - Whole words: the grown brain beats order-2 counts by 0.225 bits per word with binds and holds over earlier words.
2. **Word-piece levels now keep order-2 context,** once extended trials are judged on recent gain. They still keep no chunk, so no third level appears, even on the whole corpus.
3. **The adaptive vocabulary brings content in** ("other" 54% → 37% on the whole corpus; names from the current book) but churns. Level 1 uses a smaller share of its order-1 information than with a fixed vocabulary.
4. **Recency was already there.** A recency edge is worth 0.056 bits per word over a slow-memory bias edge, but the upper levels' fast-forgetting bias edge already does the same job. In growth runs recency candidates were never kept.
5. **What's left for a third level:** the word-piece stream's chunks still add nothing in trials. Whole words, with a good screen, grow non-adjacent structure (holds, binds over earlier words) and beat order-2 counts. That suggests, though it is untested, that the problem is now the units level 1 receives (word-pieces with churning identities) rather than the grammar.

## Code

- `info.rs`: `heldout_gain_mix`.
- `brain.rs`:
  - `RunScreen::make` (mixture with online λ);
  - `Cfg::screen_mix`, `mix_thr`, `fresh_extensions`, `recency`, `recency_kept`, `vocab_halflife`, `bias_mask_bits`, `bias_age`;
  - `Brain::thr`, `add_recency`, `recency_window`, `reassign_symbol`; `judge_t0`.
- `stack.rs`: `VocabMap::lookup` (text-keyed, adaptive), `VocabMap::dump`.
- The stack writes `levelN.vocab.txt`, the vocabulary each level hands up.
- Flags: `--screen-mix`, `--mix-thr`, `--window`, `--fresh-extensions`, `--vocab-halflife`, `--recency`, `--recency-kept`, `--bias-mask-bits`, `--bias-age`.
