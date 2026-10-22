# 37 · A reader for any length: the test, the baseline and the plan (2026-10-22)

*[All experiments](README.md)* · [← 36](36-forum-changes.md)

Question: can the reader navigate a context of arbitrary length? It should hold a structure of what it has read (structure of structure), so that accuracy does not fall with distance and a lookup costs about log N. The constraints are unchanged: bit vectors and integers, local learning, no division, no hand-shaped pieces, and the next word as the only teacher. The design comes from the third forum (Claude Docs "The forum, round three: a reader for any length").

## The test

`scripts/exp37/longdoc_gen.py WORDS PREFIX [seed]` writes a book of 99 chapters ("chapter twenty two .") made of scenes ("scene ."). The scenes are like the stories (24 people, 12 places, 12 objects) and carry local questions. Long-range facts are planted at log-spread distances:

- **home:** "X lives in the Y .", stated once and rarely restated, asked as "where does X live ?".
- **owner:** "X owns the Z .", asked as "who owns the Z ?".
- **found:** "X found the Z ." inside chapter N, asked as "what did X find in chapter N ?". It is addressed by the book's structure.

Each long-range question picks a kind uniformly, then a distance bucket uniformly among those with a fact of that kind, then a fact. The dev book is `runs/exp37/ld200k` (204,627 words, seed 1, vocabulary 104).

`scripts/exp37/score_dist.py DOC RUN...` scores top-1 by kind and distance bucket, leaving out the first 10% of the book. It reports two distances:
- **fact distance:** words back to the fact's latest statement;
- **echo distance:** the smaller of that and the words back to the last time the same question was asked with the same answer, since a question followed by its answer restates the fact.

It also reports recall@8 (the answer among the 8 candidates) and the newest-find baseline: answering found with X's latest find, whatever the chapter.

## The baseline

The best story configuration (`--sit-qform14 --sit-kindfirst --sit-sq 64 --sit-sq-credit 1 --sit-sq-rank 1 --sit-qsparse 2 --sit-kslow 3 --sit-kshift 3`), seed 0.

**First run: the learned boundaries failed.** The clause boundary test (left and right neighbours independent) chose every object and most number words, and missed "?". The question mood counter then marked the objects as question boundaries, so clauses such as "john got the apple ." were dropped as questions. Who-has scored 21.3% and where-object 28.1%. Crediting memory only where the bigram would miss (`--sit-mood2`) did not help.

**The fix, `--sit-bnd2`:** a boundary is a word open on both sides. For each word, count the successors that cover 90% of its uses, and the same for predecessors; a word is a boundary when the smaller count is at least half the largest such value over all words. This chooses exactly "." and "?" on both the story set and the book. On the story set (seed 0) it scores 96.8%, against 95.3% before.

**With `--sit-bnd2`** (run `R_b2_0`, 666 s): local questions score 97.8% (where-person), 92.9% (who-has) and 80.8% (where-object). Long range:

| Top-1, by fact distance | 8–127 | 128–1k | 1k–8k | 8k–64k | 64k+ |
|---|---|---|---|---|---|
| home | 53.8% (13) | 22.0% (82) | 8.2% (343) | 9.0% (689) | 7.4% (149) |
| owner | 33.3% (9) | 23.9% (71) | 10.9% (357) | 8.0% (763) | 11.1% (72) |
| found | 1.7% (58) | 13.3% (210) | 10.2% (402) | 5.9% (406) | 8.8% (159) |

| Top-1, by echo distance | 8–127 | 128–1k | 1k–8k | 8k–64k | 64k+ |
|---|---|---|---|---|---|
| home | 27.4% (113) | 8.9% (483) | 8.4% (597) | 2.5% (81) | – |
| owner | 27.6% (192) | 8.7% (658) | 3.7% (404) | 6.7% (15) | – |
| found | 17.2% (93) | 11.2% (277) | 8.2% (403) | 5.8% (381) | 7.4% (81) |

| | 8–127 | 128–1k | 1k–8k | 8k–64k | 64k+ |
|---|---|---|---|---|---|
| Recall@8, home | 100% | 46.3% | 28.0% | 32.1% | 28.9% |
| Recall@8, owner | 77.8% | 49.3% | 30.0% | 22.8% | 27.8% |
| Recall@8, found | 13.8% | 26.7% | 17.9% | 14.8% | 17.6% |
| Newest-find baseline | 100% | 99.5% | 73.6% | 19.2% | 8.8% |

Chance is about 1 in 12 for places and objects, 1 in 24 for people.

- **Everything older than about a thousand words is at chance.** Fact keys drift at every clause boundary, by design: one of a key's 64 positions moves each time. Boundaries come at about 0.22 per word, so a key's surviving share halves every ~200 words. No tuning can move the far buckets.
- **Home and owner are mostly echoes.** Most of their questions were asked and answered within the last 1k words: only 96 home and 15 owner questions have an echo distance of 8k or more. Found is the honest long-range kind.
- **Found fails even next to its fact (1.7%), below the newest-find baseline (100% there).** The reader sees only the last 4 words, "chapter twenty two ?", so the entity of the question is out of view. The newest-find baseline falls to 19% at 8k–64k and 9% beyond, so beyond 8k words only the chapter address can answer.
- **Durable and changing relations share kinds:** "has" and "owns", "lives" and "find", and "found" with "up got dropped".

## The plan (the forum's order of work)

| # | Experiment | Prediction |
|---|---|---|
| 0 | The scorer: echo distance, recall@8, newest-find baseline | Done |
| 1 | The whole question clause as the cue, from the last boundary to "?" | Found at 8–127 from 1.7% to ≥45–60%; home and owner within 3 points |
| 2 | A clause log of symbols (10–24 bytes per clause, posting lists, keys made from ids at read time, never drifting) with newest-wins heads per (entity, relation) as route candidates; no hierarchy | Home and owner ≥85% in every bucket, 64k+ within 5 points of 128–1k, by echo distance too; found about 50% and flat; local within 1 point |
| 3 | Durable relations kept apart from changing ones, with persistence learned per relation class | Owner at 8–1k from 24–33% to ≥85% |
| 4 | Learned header types, levels and labels; postings restricted to a unit's range | 99 ± 3 top-level units on the chapter headings; found ≥80% in every bucket; shuffled labels drop it to about 8% |
| 5 | Scale: 10^6–10^7 words, nested "part P chapter C" labels, a changed header word | Accuracy within 3 points; lookup cost ≤1.5–2× per 10× words; ≤25 bytes per word; per-word cost well under today's 3.3 ms |

If home at 64k+ stays under 70% after step 2, the limit is route credit rather than storage.

**Agreed design:** keep the drifting store as working memory for the current scene. Add a long-term tier of symbols that never drifts, since the superposed store holds about 1,000–4,000 bindings against 34k clauses in this book. Learned header types cut the book into units, with ordered labels indexed where a unit starts. The question slots learn which memory to read (source, level, label, rank, role, stop), with the next word as the teacher. Estimated cost: 2–6 MB at 10^6 words and 200–630 MB at 10^8, O(1) work per word to write.

**Still open:**
- **How levels form:** by recurrence interval, as openers of openers, by surprise rank, as learned skip-list heights, or as header types that keep a register while they earn credit.
- **Consolidate into a stable store, or read the log directly.**
- **How labels are matched:** hash, FM-index, trie, or the oldest occurrence of the cue words.
- **Whether question–answer pairs are committed as facts:** only once scoring is by echo distance.

## Steps 1 and 2: the whole question clause, and long-term heads

All runs below use the baseline configuration plus `--sit-bnd2`, seed 0, on `ld200k`.

**Step 1, `--sit-cue-clause`.** The cue reaches back to the start of the clause: at least 4 words, at most 12. On the story set it scores 98.2% (seed 0; 96.8% before). On the book, found at 8–127 words rises only from 1.7% to 12.1% (recall@8 from 14% to 36%), well short of the forum's ≥45%. With X in view, the reader still answers X's latest find only 12% of the time. About 85% of found questions lie beyond the drift horizon, so the route learner almost never sees a candidate that holds the answer: credit is starved.

**Step 2, `--sit-heads`.** An exact newest-wins table of committed facts that never drifts: (entity, key, value kind) → (value, time, class), stored as symbols. Its entries are offered as one-hop route candidates with a long-term source feature. The design changes along the way, each tried on the book:

| Change | Home 128–1k / 8k–64k / 64k+ | Owner 1k–8k / 8k–64k / 64k+ | Found 8–127 / 1k–8k |
|---|---|---|---|
| clause cue only (step 1) | 20.7 / 7.5 / 4.0% | 10.4 / 8.1 / 11.1% | 12.1 / 7.7% |
| heads keyed by pooled class, newest 2 per kind | 15.9 / 9.1 / 9.4% | 89.4 / 61.5 / 11.1% | 29.3 / 10.2% |
| keyed by the exact relation path (`--sit-heads-path`), newest 24 | 74.4 / 12.6 / 6.7% | 96.9 / 48.6 / 6.9% | 46.6 / 19.9% |
| newest 8 per value kind, 48 in all | 96.3 / 73.0 / 22.1% | 95.5 / 96.7 / 100% | 27.6 / 25.1% |
| + the value's kind read fresh, + an exact-path feature | 95.1 / 81.6 / 38.9% | 100 / 95.8 / 100% | 55.2 / 43.3% |
| **+ singleton kinds offer their newest head only** | **96.3 / 99.4 / 100%** | **95.2 / 92.7 / 100%** | **60.3 / 43.0%** |

The changes, in order:
- **Pooled-class keys.** "owns the" shares a class with "got the", so a later pick-up overwrote the owner fact, and the further back the fact, the likelier that was.
- **Exact-path keys** stop that: only a restatement overwrites a fact.
- **The cap had to be per value kind.** Each function word is a kind of its own, and a person collects a head to "." for every distinct clause tail ("went to the kitchen", "went to the garden", …). Those crowded out old durable heads such as "lives in the".
- **The value's kind is read at lookup time,** since kinds settle after early heads are written.
- **The exact-path feature** lets the learner tell "found the" from "got the", which share a kind and a class.

**The final row** (`R_hk10_0`, 1,091 s):
- **Home:** 96–100% in every bucket, and 98.8–99.7% by echo distance.
- **Owner:** 89–100% in every bucket.
- **Recall@8:** 100% for both, everywhere.
- **Local questions:** 99.6 / 93.1 / 82.0% (where-person, who-has, where-object).

This meets the forum's step-2 prediction: ≥85% in every bucket, 64k+ within 5 points of 128–1k.

- **Found is unsolved.** It scores 60.3 / 63.3 / 43.0 / 15.3 / 12.6% by fact distance, against a newest-find baseline of 100 / 99.5 / 73.6 / 19.2 / 8.8%. Its recall@8 is 98.3 / 94.3 / 82.6 / 58.1 / 51.6%. Below about 8k words the newest find is usually the answer, and the reader still misses it about 40% of the time. Beyond that, only the chapter address can answer (step 4).
- **The cost on the story set:** 96.5% (seed 0) against 98.2% without heads. Facts from earlier stories never fade, and the learner sometimes trusts them.
- **Step 3** (durable relations apart from changing ones) was needed in part for step 2: exact-path keys and the exact-path feature keep "owns" apart from "got" without learning persistence per class.

## Caveats

- Single seed, one book.
- The machine restarted several times during this work. Seed 2 of the experiment 36 full benchmark was lost twice and has been relaunched.
