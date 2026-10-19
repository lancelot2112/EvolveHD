# 23 · A factored phrase readout, tested with counts first (2026-10-19)

*[All experiments](README.md)* · [← 22](22-fusion.md)

**Concepts:** [Writer-mask readout](../concepts/writer-mask-readout.md) · [Screens](../concepts/screens.md#estimator-bias) · [Stepping stones](../concepts/stepping-stones.md)

Question: [22](22-fusion.md) concluded that a phrase level predicting one cell per unit is a slot vocabulary in disguise, and that using the overlap between fused codes would need a factored readout: predict the parts of the next unit by role instead of a cell. Before building that into the engine, a gate. With exact counts, how much would such a readout tell the word level about the next word, beyond what the word level can already use?

## The test

`scripts/factored_proto.py`, on the whole-word streams of [21](21-word-units.md) (top 512 words), prequential, single run:
- **Phrases** are cut where a word's surprise (order-2 Witten-Bell) exceeds its running mean by 0.5 sd, as the engine's chunk node cuts.
- **Heads** predict each word from its position j in the current phrase (known when the phrase starts) and the previous phrase:
  - *cell*: the previous phrase as one unit, which is what a cell or slot readout can use;
  - *factored*: one head per role of the previous phrase (its last, second-last and third-last word, and its first word).
- **Skip context** for comparison: the words 2 and 3 back, with no phrases at all. The word level already reaches such context with delays and holds ([20](20-fair-screens.md)).
- **Scoring:** each head is mixed with order-2 word counts, with weights chosen on the first fifth of the scored half and the gain in bits per word reported on the rest.

## Results

| Gain beyond order-2 word counts, bits per word | Austen (440k words) | Whole corpus (2.66M words) |
|---|---|---|
| Cell head | +0.033 | +0.053 |
| Factored heads | +0.046 | +0.069 |
| Skip context (words 2 and 3 back) | +0.058 | +0.073 |
| **Skip context + factored heads** (separate weights) | +0.061 | **+0.076** |
| Skip context + cell head | +0.058 | +0.075 |

At cut thresholds of 0, 1 and 1.5 sd on Austen (phrases averaging 3.4 to 12 words), factored heads add 0.000 to 0.003 bits per word beyond skip context.

- **Factoring beats the single cell,** as expected: on the whole corpus +0.069 against +0.053.
- **But nearly all of it is skip context.** What the previous phrase tells about the next word is, almost entirely, which words came a few positions back. Beyond those words, the factored heads add 0.003 bits per word on the whole corpus.
- **So the gate fails.** Exact counts usually do better than writer masks, so an engine version would likely gain less than 0.003 bits per word, under a third of the 0.01 keep bar. It was not built.

## Findings

1. **At this scale, phrase structure adds almost nothing to word prediction beyond non-adjacent words.** A phrase level, factored or not, would mainly relay context the word level already reaches through delays and holds.
2. **The useful non-adjacent context found so far is at the word level:** holds and binds over earlier words (which beat order-2 counts by 0.21 bits per word in [21](21-word-units.md)) and recency (0.13 bits per word in counts, [19](19-whole-corpus-stack.md)).
3. **Test with counts before building.** A ten-minute prototype answered what an engine change would have taken hours to show.

## Code

`scripts/factored_proto.py`. Results: `runs/exp22/proto_austen.txt`, `runs/exp22/proto_all.txt`.
