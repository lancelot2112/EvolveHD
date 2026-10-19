# 24 · A story state for the word level (2026-10-19)

*[All experiments](README.md)* · [← 23](23-factored-readout-gate.md)

**Concepts:** [Writer-mask readout](../concepts/writer-mask-readout.md) · [Growth operators](../concepts/growth-operators.md#recency) · [Stepping stones](../concepts/stepping-stones.md)

Question: [23](23-factored-readout-gate.md) found that meaning carries across a story as accumulated state (recency, association, the book's own words), worth about 0.38 bits per word beyond order-2 counts on the whole corpus with 4,096 words. Can the word level use it?

All runs are single-seed.

## The op

`--story K`: a kept edge from a constant "story" node whose rows are keyed by the K most salient recent content symbols. Salience is a count that decays with a half-life of `--story-half` content symbols (default 200); symbols among the 100 commonest, and "other", are not content. The edge scores the salience-weighted mean of its rows. On each word it writes all K rows, or one drawn by salience (`--story-write1`); `--story-age` ages its rows. It is a superposition of recent content words, read out through one row per component, so a row keyed by "whale" learns what tends to follow while whales are on the page. The edge is never pruned; its ablation gain (bits lost when its contribution is removed) is logged every 10k words.

## The ceiling at 512 words

The engine's word streams keep only the 512 commonest words, and most content words are "other" there (22% of the corpus stream). `scripts/meaning_transfer.py` with 512 words, beyond order-2 counts:

| | Austen | Whole corpus |
|---|---|---|
| All long-range contexts (4,096 words, from [23](23-factored-readout-gate.md)) | +0.136 | +0.378 |
| All long-range contexts, 512 words | +0.066 | +0.112 |
| This op in counts (`scripts/story_proto.py`, K 16, half-life 200) | +0.049 | +0.082 |

So the design itself captures most of what 512 words leave, but 512 words leave less than a third of it.

## Austen, 512 words

Level 0 alone (`stack --syms runs/exp21/austen_w512.syms --levels 1`), the settings of [21](21-word-units.md) section 1:

| Run | Bits per word, second half | Story ablation gain, second half |
|---|---|---|
| Baseline | 5.283 | – |
| Bias edge (aging every 1,024) | 5.289 | – |
| K 4 / 16 / 64, write all | 5.271 / 5.281 / 5.285 | +0.006 / +0.010 / +0.015 |
| K 16, half-life 1,000 | 5.285 | +0.010 |
| K 16, rows age every 256 | 5.271 | +0.006 |
| **K 16, write one** | **5.269** | **+0.061** |
| K 16, write one, age 256 | 5.275 | +0.055 |
| Bias edge + K 16 | 5.272 | +0.008 |

Writing one row per word makes the edge carry much more (written all at once, every row drifts toward the same recent-word distribution). But the net gain is at most 0.014 bits per word, within the spread between single-seed runs here; the story runs also never kept the baseline's chunk.

## The whole corpus, 512 words

The settings of [21](21-word-units.md) section 4, level 0 only, 80–100 minutes each:

| Run | Bits per word, second half | Kept |
|---|---|---|
| Baseline | **4.702** | the previous word, binds over words 2–4 back (two using holds), the chunk |
| K 16, write one | 4.705 | story, the previous word, binds on the chunk's previous word |

**No net gain.** The story edge carried a lot early (ablation gain 0.1–0.3 bits per word between 0.1M and 1.4M words, and its running bits per word were 0.026 below the baseline's at 1.5M), then faded to about zero after 2M words.

## Keeping the surprising words

Most of what carries across a story is in words outside the 512 (names, places, things), and those are all "other". What would keeping them be worth? `scripts/surprising_words.py` scores exact word identity in counts: an "other" word costs its slot plus its spelling, from unigram counts over other words. Whole corpus, beyond order-2 counts:

| | Bits per word | Gain |
|---|---|---|
| Baseline (of which spelling "other" words: 3.99) | 8.905 | |
| Story op over the 512 words | 8.828 | +0.077 |
| **Keep recent "other" words** (a decaying cache, mixed into their spelling) | 8.723 | **+0.182** |
| Both | 8.646 | +0.258 |
| Both, with the kept words also as story keys | 8.629 | +0.276 |

On Austen the same rows give +0.046, +0.026, +0.072 and +0.075.

- **Spelling the words outside the vocabulary is 45% of all bits on the whole corpus,** and the engine's scores leave it out: they score the slot "other", not the word.
- **Remembering recent out-of-vocabulary words is the largest gain,** more than twice the story op over the 512.
- **Weighting kept words by surprise adds nothing.** With every kept word counted equally the gains are the same (+0.186, +0.263, +0.275). Every out-of-vocabulary word is already surprising; what matters is keeping them at all.

## Findings

1. **The word level does not use the story state yet:** on the whole corpus, 4.705 bits per word with it against 4.702 without.
2. **Its contribution fades.** It is large in the first half of the corpus and near zero by the end. A likely cause, untested: its rows are written on most words without ever aging, so their masks fill and stop discriminating. Austen rows aged every 256 writes didn't help there, but Austen's ceiling is too small to tell.
3. **Keeping out-of-vocabulary words is worth more than the story op** (+0.18 against +0.08 bits per word in counts, +0.28 together), but the engine's scores don't yet count the words it can't name.
4. **512 words hide most of it.** The long-range information is mostly in content words, and at 512 words most of them are "other": the counts ceiling drops from 0.38 to 0.11 bits per word, and the existing bias edge already covers some of the rest. The prize needs a word level that handles thousands of words, where the readout currently trails order-1 counts ([21](21-word-units.md)).

## Code

- `edge.rs`: `Edge::multi`, `set`, `write_one` (a weighted set of rows per tick).
- `brain.rs`: `Cfg::story`, `story_half`, `story_skip`, `story_age`, `story_write1`; `Brain::story_update`.
- `scripts/story_proto.py` (the op in counts); `scripts/surprising_words.py` (keeping out-of-vocabulary words, scored on exact word identity). Runs: `runs/exp24/`.
- Flags: `--story`, `--story-half`, `--story-skip`, `--story-age`, `--story-write1`.
