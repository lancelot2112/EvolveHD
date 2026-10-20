# 25 · Keeping the words outside the vocabulary (2026-10-19)

*[All experiments](README.md)* · [← 24](24-story-state.md)

**Concepts:** [Writer-mask readout](../concepts/writer-mask-readout.md) · [Growth operators](../concepts/growth-operators.md#recency) · [Stepping stones](../concepts/stepping-stones.md)

Question: [24](24-story-state.md) found, in counts, that most of what carries across a story is in the words outside the 512-word vocabulary, and that the engine's scores leave them out. If the word level keeps those words and is scored on the exact word, what does it gain?

All runs are single-seed.

## Scoring the exact word

`wordslots` reads full word ids (`scripts/word_ids.py`: each word's frequency rank; the 512 commonest are the same symbols as in [21](21-word-units.md)–[24](24-story-state.md)). The word level predicts a symbol as before; an "other" word also pays its spelling: unigram counts over the words outside the 512, with an escape to 1 in 100,000 for words never seen. Bits are bits per exact word.

## Three ways to keep them

- **Temporary slots** (`--slots S`): S more symbols, handed to words outside the 512 on first sight (least recently used slot first, or lowest decaying use count with `--slot-half`). A slot handed to a new word gets a fresh code and generation (`Brain::renew_sym`; the target index updates that one cell, `TargetIndex::change_cell`).
- **The story state names the "other" word** (`--keep-half H`): its spelling mixes the unigram counts with a decaying memory of recent words outside the 512 (half-life H such words), with the mixing weight chosen online (the best of a grid so far, so no look-ahead).
- **Kept words as story keys:** the story edge of [24](24-story-state.md), keyed by word id, so words outside the 512 are keys too.

## Austen

| Run | Symbols | Spelling | Exact word |
|---|---|---|---|
| Baseline | 5.283 | 2.348 | 7.630 |
| 64 slots (LRU) | 5.524 | 2.138 | 7.662 |
| 64 slots, use-count eviction | 5.541 | 2.123 | 7.664 |
| 256 slots, use-count eviction | 5.961 | 1.901 | 7.862 |
| 64 slots + story | 5.535 | 2.138 | 7.673 |
| Story keyed by word id | 5.278 | 2.348 | 7.626 |
| Memory names "other" words, H 50 / 200 / 1,000 | 5.283 | 2.313 / 2.311 / 2.314 | 7.596 / 7.594 / 7.597 |
| **Memory (H 200) + story keyed by word id** | 5.266 | 2.311 | **7.577** |

- **Slots make things worse,** at every size. A word in a slot saves its spelling, but the symbol it now needs is harder to predict than "other": a slot means a different word each time it changes hands, so the context rows can't learn it. Counts models lose too (order 2: 7.483 against 7.449).
- **Letting the story state name the word works,** with little dependence on the half-life.

## The whole corpus

The settings of [21](21-word-units.md) section 4, level 0 only, 90 minutes each:

| Run | Symbols | Spelling | Exact word |
|---|---|---|---|
| Baseline | 4.702 | 3.694 | 8.395 |
| Memory names "other" words (H 200) | 4.702 | 3.443 | 8.145 |
| **Memory + story keyed by word id** | **4.655** | 3.443 | **8.098** |
| Witten-Bell order 2 (symbols), same spelling as the baseline | 4.921 | 3.694 | 8.364 |

- **Exact-word bits drop by 0.30 per word:** 0.25 from naming "other" words out of the memory, 0.047 from the story edge.
- **Keyed by word id, the story edge helps, and keeps helping.** Keyed by the 512 words only ([24](24-story-state.md)), it ended at 4.705 against 4.702, its contribution fading to zero. Keyed by word id, words outside the 512 included, it ends at 4.655, and its ablation gain stays above 0.2 bits per word to the end (0.47 on average over the second half). With one seed per run, the key change is the likely cause but was not isolated.
- **The engine is now 0.27 bits per exact word ahead of order-2 counts** (8.098 against 8.364), up from 0.22 on symbols alone.

## Findings

1. **Scored on the exact word, words outside the vocabulary cost 44% of the bits** (3.69 of 8.40 per word on the whole corpus), and our earlier scores left them out.
2. **Keeping them pays: 0.30 bits per word on the whole corpus.** The story state's memory of recent words names most of them, and those words, as story keys, make the story edge work.
3. **Giving them slots of their own doesn't.** A slot that changes owner is a symbol no context can learn.
4. **Weighting kept words by surprise isn't needed:** everything outside the vocabulary is already surprising ([24](24-story-state.md)).

## Code

- `main.rs`: `wordslots --data IDS --base 512 [--slots S --slot-half H] [--keep-half H]` (exact-word scoring; the memory and slots are computed from the word stream).
- `brain.rs`: `Brain::story_keys` (story keys per tick), `renew_sym`; story salience is a map over keys.
- `edge.rs`: `TargetIndex::change_cell` and the change log (rows update only the cells whose codes changed).
- `scripts/word_ids.py`. Runs: `runs/exp25/`.
