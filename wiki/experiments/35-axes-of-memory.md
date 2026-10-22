# 35 · Precision, spread and compression: testing the trade-offs (2026-10-21)

*[All experiments](README.md)* · [← 33](33-learned-map.md)

**Concepts:** [Sparse codes](../concepts/sparse-codes.md) · [Capacity and budget](../concepts/capacity-and-budget.md) · [Time delays and holds](../concepts/time-delays-and-holds.md) · [Go/NoGo selection](../concepts/go-nogo-selection.md)

Question: the trade-off tables (the "Holographic memory trade-offs" doc, from experiments 32 to 34) suggested that three axes of a memory move together:
- **precision per cell:** 1 bit, ternary, 4, 8, 16, 32 bits;
- **spread:** one cell per item, locally distributed (K of D elements), fully distributed;
- **compression:** lossy (pooling, classes) or exact (storing facts).

The claim: learning statistics wants counters in few cells, and storing facts wants few levels spread over a few elements. Each of the seven primitives (Expand, Associate, Compress, Gate, Select, Correct, Broadcast) sits at its own point on these axes. Before running anything, seven predictions were written down. Each is tested here by a simulation in which every cell learns only from what reaches it: a counter or a trace element, stepped up or down.

Code: `scripts/exp35/` (numpy; logs were written to `runs/exp35/`, which is not tracked; one seed each unless stated). `trace.py` is the shared sparse 4-phase trace: L levels per part, real and imaginary, clipped. With 2 levels each part holds the sign of the last write.

## Scorecard

| # | Prediction | Verdict |
|---|---|---|
| 1 | The bits a cell needs ≈ log₂ of the noisy events it must average, capped by how often the value changes. With clean observations, 1 bit or ternary wins at every rate | **Holds.** Amended: it holds for up/down counters; up-only counters saturate once they have seen about 2^b × 32 noisy examples |
| 2 | Best active elements K ≈ 4 × levels², whatever the memory's size | **Roughly holds** (≈ 3L²). New: once K is tuned, depth varies by up to a third with no trend in the levels |
| 3 | With exact cues, one fact per cell beats superposition; with noisy cues, spread codes win; a fully distributed signal is the cheap way to change everything | **First two hold.** The third: a context id in a hash is just as cheap; what spreading buys is a *graded* change |
| 4 | OR prototypes saturate; 4-bit counter prototypes read back at 1 bit need no reclustering | **Holds** |
| 5 | A sense is a read-time gate (a mask on the word's shared code), not an address | **Wrong as framed.** A mask loses; a sense that *adds* its own counts to the word's (shrinkage) wins |
| 6 | The fast memory is an evolving state: its own dynamics give order, and reads bounced through it answer chains of any length | **Holds for order and chains.** Fails for recency from drift; recency needs a write time per slot |
| 7 | Put together on the stories, the winning pieces lift the situation model | **No gain.** Gated recency ties the base (66.6% vs 66.3%); every other combination loses. The learned classes merge relations that storage then overwrites ([E7](#e7--the-full-system-on-the-stories)) |

## E1 · Bits are a time constant

`e1_bits.py`. Every cell is a saturating counter with S states (1 bit = 2, ternary = 3, 4 bits = 16 …).

**(a) Tracking a changing value.** A variable holds one of 8 values and changes every T events on average. Each event shows the true value with probability q, otherwise a random one. Each value has a cell, stepped up when it is shown and down otherwise. The read is the highest cell.

Accuracy, by counter width:

| T | q | 1 bit | ternary | 2 bits | 3 bits | 4 bits | 8 bits | 32 bits |
|---|---|---|---|---|---|---|---|---|
| 8 | 1.0 | **89.1%** | 84.8% | 80.6% | 68.8% | 56.6% | 47.5% | 47.5% |
| 128 | 1.0 | **99.3%** | 99.0% | 98.7% | 97.4% | 94.9% | 59.4% | 49.9% |
| 512 | 1.0 | **99.8%** | 99.7% | 99.7% | 99.3% | 98.7% | 82.6% | 50.1% |
| 8 | 0.6 | 58.5% | 62.5% | **66.9%** | 63.9% | 60.6% | 60.0% | 60.2% |
| 32 | 0.6 | 63.3% | 71.0% | 80.7% | **85.3%** | 79.4% | 71.7% | 71.4% |
| 128 | 0.6 | 64.6% | 73.2% | 85.1% | **94.9%** | 92.9% | 74.3% | 74.6% |
| 512 | 0.6 | 64.9% | 73.8% | 86.2% | 97.8% | **98.0%** | 81.3% | 74.9% |
| never changes | 0.35 | 43.1% | 49.0% | 56.6% | 67.0% | 70.3% | 70.6% | 70.6% |

- **Clean observations: the last one is the answer.** 1 bit wins at every change rate. A wide counter lags each change by about half as many events as the value had lasted (the old cell falls while the new one climbs), so 32 bits sits near 50% even when changes are 512 events apart.
- **Noisy observations: the best width grows with how long the value stays put,** from 2 bits (changes every 8) to 4 bits (every 512). Wider counters only add lag. At q = 0.35, everything from 4 bits up is within 0.3 points of the best.
- **A caveat on the rule.** A symmetric up/down counter climbs only for a value shown more than half the time, so at q = 0.35 even 32 bits stays at 70.6%. Rarer features would need a larger step up than down.

**(b) Learning fixed classes,** with the test of the trade-offs doc (`learn_counters.py`): 20 classes in pairs sharing 75% of their bits; examples keep half the bits and add 64 noise bits.

| Counter | 4 examples | 16 | 64 | 256 | 1,024 |
|---|---|---|---|---|---|
| up-only 1 bit (= OR) | 61% | 22% | 7% | 5% | 3% |
| up-only 4 bits | 89% | 100% | 100% | 92% | **3%** |
| up-only 8 bits | 89% | 100% | 100% | 100% | 100% |
| up/down ternary | 81% | 88% | 84% | 88% | 86% |
| up/down 3 bits | 82% | 94% | 97% | 97% | 98% |
| up/down 4 bits | 82% | 94% | 98% | **100%** | **100%** |
| up/down 32 bits | 82% | 94% | 98% | 100% | 100% |

- **Up-only counters saturate.** Noise bits reach the ceiling after about (2^b − 1) ÷ (noise rate) examples: for 4 bits, 15 × 32 ≈ 500. OR is the 1-bit case. Seen here up to 4 bits; 6 bits and wider would saturate past this test (8 bits at about 8,000 examples).
- **Up/down counters never saturate,** and 4 bits already equals 32. Their width sets how many examples they average, not how long they can run. So the earlier "4 bits = full counters" holds for up/down counters only.

## E2 · Spreading vs levels

`e2_kl.py`. Random keys used once, values from 1,000, recall among 50 candidates. Depth is the number of writes back at which recall stays at 80%, smoothed over 3 neighbouring ages.

| Levels L | 3 × L² | Best K at 16,384 elements | Depth there | Best K at 65,536 | Depth there |
|---|---|---|---|---|---|
| 2 | 12 | 8 | 842 | 12 | 4,278 |
| 3 | 27 | 24 | 993 | 16 | 4,880 |
| 4 | 48 | 32 | 888 | — | — |
| 5 | 75 | 96 | 1,021 | 96 | 3,620 |
| 8 | 192 | 192 | 1,013 | 256 | 4,169 |
| 16 | 768 | 768 | 859 | — | — |

- **Best K ≈ 3L², the same at both sizes.** The grid steps by about 1.5×, and the curves near the peak are flat and noisy, so read this as "within a factor of two".
- **Once K is tuned, depth shows no trend in L:** 842 to 1,021 at 16,384 elements, 3,620 to 4,880 at 65,536. So per bit stored, fewer levels win: 2 levels hold about as much as 16 at a quarter of the bits.

## E3 · One cell, a few, or all

`e3_local.py`. Same bit budget for every store (a ternary trace of 16,384 elements: 51,937 bits). Keys are codes of 64 (element, phase) pairs; a noisy cue has a fraction f of its pairs redrawn. Recall must find the exact value among all 1,000 (full clean-up).

| Store | f = 0 | 0.1 | 0.25 | 0.5 | 0.75 |
|---|---|---|---|---|---|
| hash table of value ids (one cell per fact) | **1,098** | 0 | 0 | 0 | 0 |
| CAM: 8 sampled key elements + value per entry | 311 | 311 | 311 | 305 | 0 |
| sparse 1-bit trace, K = 16 | 349 | 3 | 0 | 0 | 0 |
| sparse ternary trace, K = 64 | 482 | 381 | 169 | 0 | 0 |
| dense trace, 16 levels, every element (6,492 elements) | 99 | 91 | 73 | 39 | 0 |

80 queries per age, except the dense row (16).

- **Exact cue and exact recall: one cell per fact wins,** by 2.3×. Recall among all 1,000 values needs more margin than E2's recall among 50. Part of the gap is also an untuned trace: K = 64 here, where E2 found K ≈ 24 best for ternary (993 deep vs 717 at K = 64, among 50).
- **Noisy cue: the hash table breaks at once,** because any change to the cue changes its address. The ternary trace degrades gracefully. A content-addressable table that stores part of each key is the most robust (305 of its 311 at f = 0.5), but holds little: 425 entries fit the budget, oldest replaced.
- **Fully distributed content is the worst store,** 5× shallower than sparse ternary at the same bits. Of the traces it degrades the most gently relative to its own depth (39 of its 99 at f = 0.5), but from far less.
- **Broadcast.** Making every old fact unreadable at a story boundary costs O(1) in both: change the context id in the hash, or redraw all the context slots of the trace. What the trace adds is a graded reset. Redraw part of the context, and old stories fade instead of vanishing. Recall of a fact written b story boundaries ago, cued in the current context (20 facts per story):

| Context slots redrawn per boundary | b = 1 | 2 | 3 | 5 | 9 |
|---|---|---|---|---|---|
| 8 of 64 | 100% | 100% | 100% | 90% | 20% |
| 16 of 64 | 95% | 95% | 70% | 10% | 5% |
| 32 of 64 | 95% | 10% | 0% | 0% | 0% |
| 64 of 64 | 0% | 0% | 0% | 0% | 0% |

The redraw rate sets how many stories back stay readable; redrawing everything is the hash's all-or-nothing reset.

## E4 · Prototypes: OR or counters

`e4_protos.py`. Online pooling: 16 latent classes of 64 bits in 2,048, in pairs sharing half their bits; items keep 60% of their class's bits and add 32 noise bits. An item joins the prototype it overlaps most (at least 0.3 of its bits), else starts one. Only the chosen prototype's memory is updated.

| Prototype memory | 250 items | 1,000 | 4,000 | 16,000 | Prototypes at 16,000 |
|---|---|---|---|---|---|
| OR | 77% | 6% | 5% | 6% | 18 |
| OR, rebuilt from the last 2,000 items every 2,000 | 77% | 6% | 7% | 5% | 17 |
| 4-bit up-only, top 64 bits | 100% | 100% | 100% | 100% | 51 |
| 4-bit up/down, top 64 | 100% | 100% | 100% | 100% | 31 |
| 32-bit, top 64 | 100% | 100% | 100% | 100% | 22 |
| 32-bit, read as weights (no binarizing) | 100% | 100% | 100% | 100% | 22 |

Accuracy: a new item's nearest prototype carries its class.

- **OR saturates within a few hundred items, and reclustering cannot help:** 2,000 items are already enough to fill every prototype.
- **Learning in counters and reading at 1 bit loses nothing** (top 64 bits vs the counts as weights).
- **The up-only counters fragment late** (51 prototypes for 16 classes) as their noise bits saturate, the E1 effect; up/down less so.

## E5 · A sense: address, gate or shrinkage

`e5_senses.py`. 10 words with 2 senses each; the two senses share half their 64 bits (the word's core). Each example of word w in sense s keeps half the sense's bits and adds 64 noise bits; the sense is known from context while learning. All memories are 4-bit up/down counters.

| Code of (w, s) | 4 examples per word | 8 | 16 | 64 | 256 |
|---|---|---|---|---|---|
| plain: one code per word, senses mixed | 52% | 42% | 52% | 38% | 52% |
| address: separate counters per sense | 82% | 89% | 96% | 100% | 100% |
| gate: the word's shared top bits, masked to where this sense's counter ≥ the other's | 58% | 67% | 78% | 78% | 83% |
| **shrinkage: top bits of shared + 2 × sense counts** | **86%** | **91%** | 95% | 100% | 100% |

Accuracy: the right (word, sense) of 20 is nearest. Bit recall (share of the true sense bits in the learned code), from 4 to 256 examples: address 37% → 89%, shrinkage 42% → 92%, gate 29% → 40%.

- **A hard mask throws away the shared core.** Each core bit is in one sense's mask by a coin flip, so half the core goes.
- **Shrinkage wins at few examples and ties later.** A sense starts from the word and moves toward its own evidence. In the situation model, this means a sense should add to the word's code, not replace it in the key.

## E6 · State, not edges

**(a) Order from the state's own dynamics** (`e6a_order.py`). Both stores use the same bits: a ternary vector of 16,384 elements, items with 64 active pairs.
- **State:** every step the whole vector shifts by one element, then the new item is added. No pairs are bound.
- **Edge:** binds each item to the next.

| Lag n | State: after X | Edge: after X | State: two after X | Edge: two after X (two reads) | State: which is more recent |
|---|---|---|---|---|---|
| 128 | 97% | 97% | 97% | 92% | 97% |
| 256 | 92% | 92% | 92% | 85% | 98% |
| 384 | 72% | 85% | 74% | 72% | 90% |
| 512 | 46% | 57% | 49% | 30% | 84% |
| 768 | 5% | 35% | 11% | 16% | 62% |

- **The state gives order for free.** Any lag is one read once X is found, so "two after X" is as easy as "after X". The edge store compounds its errors over two reads, and cannot say which of two items is more recent at all.
- **Its price is a scan.** Finding X costs one read per lag looked at, and the state fades sooner than the edge store (384 steps back: 72% vs 85%; 768 back: 5% vs 35%).
- **Caveat on the recency column:** when Y had appeared again since, the script counted the query as right without testing it (about 10% of queries at lag 512, 14% at 768), so the long-lag recency figures are inflated; 62% at 768 is close to chance.

**(b) Chains by bouncing reads** (`e6b_recurse.py`). A world of 6 people, 6 places, 6 objects and 4 boxes. People move, pick up, drop, and put things into boxes they hold. "Where is X?" takes 1 hop (a person, a dropped thing), 2 (held), 3 (in a held box), 4 (in a box in a held box), or more; the 5 column counts 5 or more.
- **Store:** one ternary slot per (entity, relation) — in-place, held-by, inside — with K = 128 in 65,536 elements and newest write wins. Old relations are never erased: a dropped apple still names its last holder.
- **Answering:** read the current entity's slots, follow one, repeat until a place is reached.

| Which slot to follow | 1 hop | 2 | 3 | 4 | 5+ | All |
|---|---|---|---|---|---|---|
| **the slot written last (an exact 16-bit write time per slot)** | 100% | 100% | 100% | 100% | 100% | **100%** |
| fixed order: inside, then held-by, then in-place | 99% | 92% | 98% | 93% | 98% | 97% |
| the strongest slot, no drift | 100% | 60% | 50% | 52% | 40% | 68% |
| learned success per (relation, strength rank) | 100% | 64% | 50% | 49% | — | 70% |
| the strongest slot, but stop after 2 hops | 100% | 49% | 10% | 3% | 5% | 45% |
| the strongest slot, one of the 128 context slots redrawn with probability 1/64 per event | 100% | 45% | 7% | 1% | 4% | 44% |

- **Bouncing reads answer chains of any length,** if each hop follows the right relation. A fixed two-hop step, as in the situation model, cannot go further.
- **The right relation is the newest one.** With an exact write time per slot it is 100%. The fixed order does nearly as well only because of how this world is built: a thing put in a box never leaves it, and whoever drops something stands where it lands until they move.
- **Drift as the recency signal fails.** It makes the newest relation strongest, but it also fades every fact that has not changed in a while, and a 3-hop chain needs all three. Graded fading suits the end of a story, not facts that persist.
- **So time is where 16 bits belong.** A write time is an ordinal, not a count: it needs log₂(the longest fact life) bits, in one cell per slot.

## E7 · The full system on the stories

`e7_full.py`, on the small comprehension set of the situation model (experiment 34, write-up to come; Rust code in `src/situation.rs`) (24,659 words, `runs/exp31c/small/syn300`; 596 questions scored in the second half). The base is the Python situation model (`scripts/situation_slots.py`) with pooling at cosine 0.5, made faster and given four switches, one per finding above:
- **recency:** a 16-bit write time per slot. For each cue word, its proposals are ranked by when its own slot was last written (E6b). *Gated:* proposals below half the cue word's best count are dropped, and the rest compete on recency.
- **both:** learn the two-hop counts at every word, not only when no one-hop class held it.
- **compose:** data bouncing at write time. When a fact (x, s1, y) is written and (s1, s2) is among the 6 most counted two-hop pairs, y's s2 slot is read and the result is written into x's composite slot. So "emma dropped the cup" also stores where emma is at that moment.
- **bitproto:** meaning and classes from 4-bit counters (halved together when one saturates), read at a quarter of their maximum, paths as the OR of their words' codes; prototypes are never rebuilt, paths are re-assigned to them every 2,000 words (E4).

| Configuration | Where-person | Where-object | Who-has | All |
|---|---|---|---|---|
| base (30 classes) | 76.8% | 38.3% | 37.5% | **66.3%** |
| + recency, the cue's newest proposal only | 69.2% | 45.7% | 31.2% | 60.9% |
| + recency, all proposals newest first | 68.7% | 39.5% | 30.0% | 59.6% |
| + recency gated by count | 76.8% | 38.3% | 40.0% | **66.6%** |
| + gated recency + compose | 73.3% | 39.5% | 33.8% | 63.4% |
| + recency + both + compose | 44.4% | 42.0% | 42.5% | 43.8% |
| bitproto (174 classes) | 66.7% | 39.5% | 33.8% | 58.6% |
| all four switches, bitproto | 37.2% | 38.3% | 37.5% | 37.4% |

First candidate of the answer's kind. For comparison, the Rust situation model scores 76% on the same set (OR meaning codes, 144 classes). The row "the cue's newest proposal only" came from an earlier version of the recency switch, replaced by the rows below it.

- **None of the pieces that won alone helps here.** Gated recency ties the base (+2.5 points on who-has, within noise at 80 questions). Every other combination loses; recency with two-hop learning at every word and composition loses most (43.8%, and 37.4% with bit-local prototypes too).
- **The bottleneck is the classes, not the choice between them.** At the moment a where-person question is asked, the person's current place is in any of their 30 class slots (all of them) only 65% of the time (where-object 30%, who-has 54%; `--oracle`). With only 30 classes, a person's "went to" and "got the" facts can share a class, so the newer one overwrites the place (the likeliest reading; class contents were not inspected). Recency then picks the newest slot of a merged relation, which is often the wrong one. (The same oracle over two reads finds the answer 100% of the time, but with 30 × 30 routes a place is reachable by chance, so that number means nothing.)
- **So the order of work is fixed by the axes.** Compression may only merge what storage may let overwrite each other. Exact storage, recency and recursion all assume relations that are kept apart; on top of over-merged classes they amplify the merge. The bit-local prototypes made finer classes (174), and scored lower too (their purity was not measured): the pooling threshold and code reading were not tuned for them.

## Findings

1. **Precision is a time constant, and the counter's rule matters more than its width.** With clean observations, the last observation is the best estimate. With noisy ones, the right width is the number of events worth averaging before the value changes; here that stopped at about 4 bits. Up-only counters saturate after about 2^b × 32 noisy examples; up/down counters never do. Nothing learned with an up/down counter needed more than 4 bits.
2. **The one quantity that wanted 16 bits was time.** A write time per slot is an ordinal: it needs log₂ of the longest fact life, one cell per slot, kept exactly. It made chained questions 100%, where graded fading of the context (drift) made them fail.
3. **Spread and levels trade off at a fixed exchange rate.** The best number of active elements is about 3 × levels², and once it is tuned the depth varies by up to a third with no trend in the levels. So per bit, fewer levels win. Fully distributed content is the worst store; full spreading suits signals meant for everything, like a context reset.
4. **Exact cues favour one cell per fact; noisy cues favour spread codes.** A hash table is the deepest exact store, and useless once a cue drifts. Superposition's real advantage is tolerance, not capacity.
5. **Learn with counters, read at 1 bit.** OR prototypes collapse within hundreds of items, and reclustering cannot rescue them. Counter prototypes read back as their top bits lose nothing against reading the counts.
6. **A sense adds to its word, it does not mask it.** Shrinkage (shared counts plus the sense's own) learns fastest; a hard mask discards the shared core.
7. **The state can carry order and chains by itself.** A shifting vector gives order, lag and recency without binding pairs. Reads bounced through the slots answer chains of any length, given the right relation at each hop.
8. **Integration is gated by compression.** On the stories, none of the winning pieces helped, because the learned classes merge relations that storage then overwrites. The next step is not more machinery on top. It is classes kept apart where their facts must not overwrite each other: finer pooling, or splitting a class when its slots keep flipping between values of different kinds.

The seven primitives also lack a home for **time**: the shifting state and the per-slot write time did jobs nothing else did. They belong with Expand, as an expansion along time (time cells), with one exact cell per slot for when it was written.

All single seed. E1 to E6 are synthetic; E7 is the small set only.

## After review

An expert forum reviewed these results (the document "The composite model: a forum's consensus"). Two conclusions above are withdrawn and one is narrowed:
- **The E7 oracle read the wrong word.** It took the second word before the answer, which is "in" for "what place is X in ?" (227 of the 435 where-person questions). Its 65% "place in the slots" figure is not evidence of merged classes. The merge is shown instead by a class dump: one of the 30 Python classes holds *went/moved/walked to the* together with *got/grabbed/dropped/gave/handed the*. A symbolic replay shows the same: with classes separated, object → holder → place answers 95.1% of where-object questions, against 9.9% merged.
- **"Time is where 16 bits belong" is withdrawn.** Both scripts compare write times only within one entity, so a small rank does the same job. Once a detach clears the old slot, no time is needed at all.
- **"Superposition's advantage is tolerance" is narrowed.** At the same bits, a banded locality-sensitive hash with tags is 1.9–3× deeper than the ternary trace up to 25% cue noise.
- **Questions are written as facts.** The model writes question clauses into memory like any other clause. Before a who-has question, the possessor slot holds the true holder 71% of the time; with read-only questions it would be 92.5% (replay).
- **The kind-first score supplies the answer type.** Without it, who-has is 12.5%, not 37.5%.
