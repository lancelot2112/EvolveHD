# 27 · Meaning codes in 2,048 bits (2026-10-20)

*[All experiments](README.md)* · [← 26](26-word-map.md) · [28 →](28-width-fast-working-memory.md)

**Concepts:** [Sparse codes](../concepts/sparse-codes.md) · [Shift-bind](../concepts/shift-bind.md) · [Growth operators](../concepts/growth-operators.md#story-state)

Question: [26](26-word-map.md) found that a low-dimensional word map can't hold meaning: even offline, a line keeps 7% of a word's strongest associates nearby and 32 dimensions 22%. It suggested building similarity in the 2,048-bit space instead. If each word's code is learned as it reads from the company it keeps, do words group, and does the grouping help predict?

Counts prototype (`scripts/meaning_codes.py`), one pass, prequential, one run per configuration, scored on exact-word bits beyond the counts system of [25](25-kept-words.md) as in 26.

## The codes

- **Identity:** every word has a fixed random code, 32 of 2,048 bits.
- **Meaning, learned online:** a counter per bit for each word. When two content words (outside the 100 commonest) occur within 16 content words of each other, each adds the other's identity code to its counters, weighted by the other word's rarity (log of total over its count so far).
- **The meaning code** is the K strongest bits (default 64). *Distinctive bits*: each bit's count is divided by how much that bit is used across all words, so bits busy for every word don't dominate. *Raw*: plain counts.

Two words that keep the same company share bits, whether or not they ever meet. Nothing is placed or moved: the grouping is in the code. (A word with fewer than K non-zero counters uses fewer bits. The 100 commonest words get no code.)

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
| love | yet might can thing excellent myself own truth much |

The groups are topics and registers: whaling, plants, the epistles, temple treasure, the night sky, Biblical geography, nursery tales, Highbury, Shakespeare's spellings. Common content words such as "love" get generic neighbours: they keep every kind of company. (The neighbours are measured on the final codes, after the whole pass.)

Locality as in [26](26-word-map.md): the share of a word's 10 strongest associates among its nearest 2% (here of the words seen 20+ times, by shared bits). The comparisons with 26 are approximate: its offline orderings used words seen 3+ times, and its online maps measured within 1% of the line.

| | Austen | Whole corpus |
|---|---|---|
| Meaning codes, distinctive bits | **0.137** (random 0.038) | **0.297** (random 0.031) |
| Meaning codes, raw counts | 0.101 (0.047) | 0.265 (0.036) |
| Best offline low-dimensional ordering ([26](26-word-map.md)) | 0.052 (0.049), 2-D | 0.224 (0.037), 32-D |
| Best online map by ratio ([26](26-word-map.md); within 1% of the line) | none clearly above chance | 0.041 (0.025) |

Learned in one pass, the meaning codes group words better than any arrangement tried in 26, including the heuristic orderings computed offline in 32 dimensions. Distinctive bits matter: raw codes group less well and their neighbours are more generic (raw "sea": through, along, large, last).

## Does it help predict?

The story state is a decaying sum (half-life 200 content words) of the meaning codes of recent content words; its code is the k most lit bits (k = 64, and 128 in the K 128 run). The readouts are those of [26](26-word-map.md): rows keyed by the story code's bits, and a meaning readout in which each story bit spreads its share over the words whose codes contain it, in proportion to their frequency (a snapshot of all codes every 20,000 words), so a story about the sea can name a sea word it hasn't used yet. The control does the same with random codes of the same size (64 bits) for the same words: compression without meaning.

Gains in bits per exact word beyond the counts system of 25 (whole corpus, base 8.672; rare words seen fewer than 10 times; words new to the story are content words not in the previous 2,000):

| | All words | Rare words | New to the story |
|---|---|---|---|
| **Story edge keyed by word id ([25](25-kept-words.md))** | **+0.130** | **+0.171** | +0.270 |
| Code rows, meaning K 64 | +0.101 | +0.117 | +0.226 |
| Code rows, meaning K 32 / K 128 | +0.101 / +0.094 | +0.118 / +0.092 | +0.224 / +0.210 |
| Code rows, meaning, raw counts | +0.084 | +0.059 | +0.183 |
| Code rows, random codes (control) | +0.080 | +0.038 | +0.165 |
| Readout, meaning K 64 | +0.070 | +0.007 | +0.215 |
| Readout, random codes (control) | +0.066 | +0.068 | +0.214 |
| Story edge + readout, meaning K 32 / 64 / 128 | +0.128 / +0.127 / +0.127 | +0.143 / +0.140 / +0.144 | +0.278 / +0.276 / +0.279 |
| Story edge + readout, random codes | +0.127 | +0.165 | +0.280 |
| Story edge + code rows, any (the fit gives code rows weight 0) | +0.130 | +0.171 | +0.270 |

On Austen (K 64): the story edge gains +0.055; code rows +0.044 (distinctive), +0.049 (raw), +0.048 (random codes); the readout +0.041 (meaning, both versions) and +0.041 (random codes); story edge + readout +0.063 (distinctive), +0.062 (raw), +0.059 (random codes).

- **Meaning improves the compressed story code on the whole corpus.** Its code rows keep +0.101 of the story edge's +0.130 against +0.080 for random codes of the same size, and two thirds of its gain on rare words against a fifth. On Austen there is no such difference.
- **The readout's gain is not meaning.** Random codes name words new to the story as well (+0.214 against +0.215) and rare words much better (+0.068 against +0.007): the lit bits mostly point back at recent words, and a rare word's meaning code, learned from few contexts, is scattered. With either readout, the story edge's gain moves from rare to new words and the total drops by 0.002 to 0.004 (single runs, a coarse weight grid).
- **Nothing beats the story edge overall.** With fixed mixture weights, every combination on the whole corpus is at or below it; on Austen the best is +0.007 above it, within what one run and a coarse grid can resolve. A combination whose weight depends on context (in the engine, an edge with a learned gain) might do better; untested.
- **K 32 and 64 do equally well for the code rows; K 128 (with a 128-bit story code) is worse.**

## Attention

A word's meaning code ANDed with the story code where it occurs should leave the bits of the sense in play. The demo takes ten words at three of their first 400 occurrences (first, 200th or middle, last kept) and ANDs the story code there with the word's final meaning code; the story code already includes the word's own code. Of the 30 cases, the AND keeps 0 to 27 of 64 bits, typically about 5. When enough bits are lit they mostly mark the book or register of the passage: "fire", "heart" and "charge" in Biblical passages get Biblical neighbours, and "state" and "ball" get Austen-like neighbours wherever they occur. In two cases they pick the sense in play: "court" in Ezekiel ("toward the utter court", 27 bits: breadth, porch, measured, height, length, pillars, chambers) and "deck" in *Moby Dick* ("seen from the pequod's deck", 13 bits: ship, whales, cabin, lowered, boat). Elsewhere ("court" in Whitman, 3 bits; "light" in *Emma*, 1 bit) too little is lit to say anything. The story code's width and half-life were not varied.

## Findings

1. **Groupings form in the 2,048-bit space, online, in one pass.** Words that keep the same company share bits: a word's strongest associates are among its nearest 2% ten times as often as chance on the whole corpus, more than any arrangement tried in [26](26-word-map.md), including heuristic orderings computed offline in 32 dimensions.
2. **Distinctive bits matter for grouping.** Choosing the bits a word uses more than other words do gives tighter groups than raw counts (0.297 against 0.265), and on the whole corpus better code rows; on Austen raw counts do as well.
3. **Meaning improves the compressed story code on the whole corpus** (+0.101 against +0.080 for random codes of the same size), not on Austen.
4. **Naming words new to the story is not meaning's doing:** random codes do it as well, and rare words better. Nothing yet adds to the story edge overall.
5. **Sense attention shows only occasionally.** ANDed with the story code, a word's lit bits mostly mark the book it is in; twice ("court" in Ezekiel, "deck" in *Moby Dick*) they mark the sense.

## Code

`scripts/meaning_codes.py` (`--K`, `--k`, `--win`, `--raw`, `--identity` for the random-code control; writes `*.groups.txt` with the neighbour lists, locality and the attention demo). Runs: `runs/exp27/` (the controls are `all_id64` and `a_id64`; `all_id` used 32-bit codes and gave codes to every word, a handicap, and is superseded).
