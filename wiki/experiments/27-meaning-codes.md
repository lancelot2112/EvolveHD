# 27 · Meaning codes in 2,048 bits (2026-10-20)

*[All experiments](README.md)* · [← 26](26-word-map.md)

**Concepts:** [Sparse codes](../concepts/sparse-codes.md) · [Shift-bind](../concepts/shift-bind.md) · [Growth operators](../concepts/growth-operators.md#story-state)

Question: [26](26-word-map.md) found that a low-dimensional word map can't hold meaning: even offline, a line keeps 7% of a word's strongest associates nearby and 32 dimensions 22%. It suggested building similarity in the 2,048-bit space instead. If each word's code is learned as it reads from the company it keeps, do words group, and does the grouping help predict?

Counts prototype (`scripts/meaning_codes.py`), one pass, prequential, one run per configuration, scored on exact-word bits beyond the counts system of [25](25-kept-words.md) as in 26.

## The codes

- **Identity:** every word has a fixed random code, 32 of 2,048 bits.
- **Meaning, learned online:** a counter per bit for each word. When two content words (outside the 100 commonest) occur within 16 content words of each other, each adds the other's identity code to its counters, weighted by the other word's rarity (log of total over its count so far).
- **The meaning code** is the K strongest bits (default 64). *Distinctive bits*: each bit's count is divided by how much that bit is used across all words, so bits busy for every word don't dominate. *Raw*: plain counts.

Two words that keep the same company share bits, whether or not they ever meet. Nothing is placed or moved: the grouping is in the code.

## Do words group?

Nearest words by shared meaning bits, whole corpus, distinctive bits, K 64 (words seen 20+ times):

| Word | Nearest words |
|---|---|
| whale | whale's whales whalemen surface greenland hunted jaw pacific whaling |
| tree | leaf planted vine root boughs olive bringeth palm trees |
| church | gospel saints epistle apostle grace fellowship faith greet paul |
| gold | weighed weight vessels brass charger shekels bowl talents treasures |
| light | thick moon shadow stars clouds dark vanity window sky |
| river | northward border portion southward toward east westward sea jordan |
| jump | jumped catch fox cow horse run tailor gingerbread |
| emma | knightley hartfield herself emma's weston's elton mrs harriet's |
| brutus | beare caesars feare loue cassi heere cas caes giue antony |
| god | know than house good every see great these go |

The groups are topics and registers: whaling, plants, the epistles, temple treasure, the night sky, Biblical geography, nursery tales, Highbury, Shakespeare's spellings. The commonest content words ("god", "love") get generic neighbours: they keep every kind of company.

Locality on the measure of [26](26-word-map.md) (the share of a word's 10 strongest associates among its nearest 2%):

| | Austen | Whole corpus |
|---|---|---|
| Meaning codes, distinctive bits | **0.137** (random 0.038) | **0.297** (random 0.031) |
| Meaning codes, raw counts | 0.101 (0.047) | 0.265 (0.036) |
| Best offline low-dimensional ordering ([26](26-word-map.md)) | 0.052 (0.049), 2-D | 0.224 (0.037), 32-D |
| Best online map ([26](26-word-map.md)) | none clearly above chance | 0.041 (0.025) |

Learned in one pass, the meaning codes group words better than any arrangement in 26, including orderings computed offline in 32 dimensions. Distinctive bits matter: raw codes group less well and their neighbours are more generic (raw "sea": through, along, large, last).

## Does it help predict?

The story state is a decaying sum (half-life 200 content words) of the meaning codes of recent content words; its code is the k most lit bits (k = K). The readouts are those of [26](26-word-map.md): rows keyed by the story code's bits, and a meaning readout in which each story bit spreads its share over the words whose meaning codes contain it (a snapshot of all codes every 20,000 words), so a story about the sea can name a sea word it hasn't used yet. The control builds the story state from identity codes.

Gains in bits per exact word beyond the counts system of 25 (whole corpus, base 8.672; rare words seen fewer than 10 times; words new to the story are content words not in the previous 2,000):

| | All words | Rare words | New to the story |
|---|---|---|---|
| **Story edge keyed by word id ([25](25-kept-words.md))** | **+0.130** | **+0.171** | +0.270 |
| Code rows, meaning K 64 | +0.101 | +0.117 | +0.226 |
| Code rows, meaning K 32 / K 128 | +0.101 / +0.094 | +0.118 / +0.092 | +0.224 / +0.210 |
| Code rows, meaning, raw counts | +0.084 | +0.059 | +0.183 |
| Code rows, identity codes (control) | +0.081 | +0.044 | +0.170 |
| Meaning readout, K 64 | +0.070 | +0.007 | +0.215 |
| Meaning readout, identity codes (control) | +0.057 | +0.009 | +0.099 |
| Story edge + meaning readout, K 32 / 64 / 128 | +0.128 / +0.127 / +0.127 | +0.143 / +0.140 / +0.144 | **+0.278 / +0.276 / +0.279** |
| Story edge + code rows, any | +0.130 | +0.171 | +0.270 |

On Austen (K 64): the story edge gains +0.055; code rows +0.044 (distinctive) / +0.049 (raw); the meaning readout +0.041; story edge + meaning readout **+0.063** (raw: +0.062), worse on rare words (+0.024 against +0.027) and better on words new to the story (+0.143 against +0.111).

- **Meaning makes the compressed code better.** Built from meaning codes, the story code keeps +0.101 of the story edge's +0.130 (identity codes: +0.081), and two thirds of its gain on rare words (identity: a quarter).
- **The meaning readout names words new to the story twice as well as identity codes do** (+0.215 against +0.099).
- **But in a fixed mixture it still doesn't beat the story edge overall on the whole corpus:** with the meaning readout, words new to the story gain (+0.276 against +0.270), rare words lose (+0.140 against +0.171), and the total is 0.002 to 0.004 lower. On Austen the same mixture is +0.007 above the story edge. The mixture gives the readout one weight for every word; it spreads probability over a whole topic, which helps when the next word is new and costs when it isn't. A combination that can tell those cases apart (in the engine, an edge whose gain is learned per context) might keep both; untested.
- **K 32 and 64 do equally well; 128 is worse** for the code rows.

## Attention

A word's meaning code ANDed with the story code where it occurs should leave the bits of the sense in play. The demo takes each of ten words at its first, middle and last occurrence. Mostly the story code shares only 1 to 4 of a word's 64 bits, too few to pick a sense. The clearest case: "court" in Ezekiel ("toward the utter court") keeps 27 lit bits, whose nearest words are breadth, porch, measured, height, length, pillars, chambers: the temple-architecture sense. In a Whitman line ("nor perfume of foreign court") 3 bits are lit and their neighbours are generic. Attention works where the story state is rich in the same topic; a story code of 64 bits from a fast-decaying sum is usually too thin for it.

## Findings

1. **Groupings form in the 2,048-bit space, online, in one pass.** Words that keep the same company share bits: a word's strongest associates are among its nearest 2% ten times as often as chance on the whole corpus, more than any low-dimensional arrangement held, even computed offline ([26](26-word-map.md)).
2. **Distinctive bits matter.** Choosing the bits a word uses more than other words do gives tighter groups and better codes than raw counts.
3. **Meaning improves the compressed story code** (+0.101 against +0.081 for identity codes) and lets lit meaning name words the story hasn't used yet (+0.215 against +0.099 on such words).
4. **It does not yet add to the story edge overall.** A fixed mixture trades rare words for new ones. Whether a learned, context-dependent combination keeps both is the next question.
5. **Sense attention works only where the story code is rich in the word's topic.** With a thin story code most words have only a few lit bits.

## Code

`scripts/meaning_codes.py` (`--K`, `--k`, `--win`, `--raw`, `--identity`; writes `*.groups.txt` with the neighbour lists, locality and the attention demo). Runs: `runs/exp27/`.
