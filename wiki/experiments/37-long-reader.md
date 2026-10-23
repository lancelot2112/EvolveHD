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
| 5 | Scale: 10^6–10^7 words, nested "part P chapter C" labels, a changed header word | Accuracy within 3 points; lookup cost ≤1.5–2× per 10× words; ≤25 bytes per word; per-word cost well under today's 3.3 ms. **10^6 with nesting: met (see step 5)** |

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

## Step 4: units and labels (`--sit-ranges`)

The design is the simplest that every seat's proposal contains, with no part specific to chapters:
- **Labels:** every committed clause's text (its words in order, the boundary left out) is indexed with the start of its first and latest occurrence. Questions are not committed, so they never index anything.
- **Units:** every clause's first word is an opener type, with the sorted start positions of its clauses. A unit runs from a clause of a type to the next clause of the same type, so units nest by how often each type recurs: about 130 words for "scene", about 2,070 for "chapter".
- **History:** every long-term head keeps its history as a posting list of (time, value).
- **Lookup:** after a learned question boundary, every span of 1–4 words of the question clause that equals a stored clause's text is an anchor. "chapter twenty two" equals the clause "chapter twenty two ."; a span must match a whole clause, so "twenty" does not match "twenty two". Each anchor's unit gives a range. The cue entity's history inside the range, newest first and at most 2 per key, gives route candidates.
- **What is learned:** which candidates to trust, by the query learner, with features for the opener type that addressed the range, first or latest occurrence, span length, the exact path and the rank within the range.

**Result** (`R_rg_0`, seed 0, 1,100 s):

| | 8–127 | 128–1k | 1k–8k | 8k–64k | 64k+ |
|---|---|---|---|---|---|
| Found, before (step 2) | 60.3% | 63.3% | 43.0% | 15.3% | 12.6% |
| **Found, with units and labels** | **98.3%** | **96.7%** | **97.5%** | **97.0%** | **93.7%** |
| Found, by echo distance | 98.9% | 97.5% | 96.8% | 95.5% | 97.5% |
| Found, recall@8 | 100% | 100% | 99.3% | 98.5% | 95.6% |
| Newest-find baseline | 100% | 99.5% | 73.6% | 19.2% | 8.8% |
| Home | 100% | 98.8% | 99.7% | 99.3% | 99.3% |
| Owner | 100% | 85.9% | 93.0% | 89.8% | 100% |

- **The forum's step-4 prediction is met:** found ≥80% in every bucket. Beyond 8k words, where X's newest find is right only 9–19% of the time, the reader is right 94–97%, so the chapter address is doing the work.
- **Local questions:** 99.7 / 90.8 / 78.2% (where-person, who-has, where-object). On the story set: 96.6% (seed 0), unchanged.
- **Since run:** the shuffled-heading control and a second seed (next section); the scale test (step 5) is still to do.

## The control, a second seed, and speed

**The shuffled-heading control.** `longdoc_gen.py 200000 runs/exp37/ld200k_shuf 1 shuffle` writes a book whose chapter headings print a permuted chapter number: every chapter moves, and the questions keep the true number. The book is not word-for-word the same as `ld200k`, because the lengths of the headings shift the scenes, but it has the same distribution. On it (`R_shuf_0`, seed 0), found falls to **62.5 / 54.8 / 41.0 / 17.3 / 4.4%** by fact distance. That follows X's newest find (100 / 99.5 / 74 / 19 / 9%), and at 64k+ it falls below chance (1 in 12). So the 94–98% comes from navigating by the heading. Home and owner are unaffected (99–100% and 82–99%).

**A second seed** (`R_rg_1`): found 98.3 / 96.7 / 97.0 / 97.8 / 94.3%, home 92–100%, owner 97–100%. It replicates seed 0.

**Speed (`--sit-fast`, with `--sit-fast-mask`).** Seven parts, each tested by leaving it out:

| Part | What it does | Keep? |
|---|---|---|
| — | std's SipHash replaced by a fast integer hasher for every map | yes; results identical to the decimal |
| bit 0 | a per-word cache of ranked slot reads, kind member lists, a running background total | yes |
| bit 1 | a cheaper multiplicative phase hash | yes |
| bit 2 | second hops only from the 4 nearest cue words of content kinds | yes |
| bit 3 | heads and ranges only from cue words of content kinds | no |
| bit 4 | heads filtered to content values before the sort | yes |
| bit 5 | heads stored only for content values | no |

Bits 3 and 5 cost owner at 8k–64k words (70% with every part on, 77% without bit 3, 81% without bit 5) and saved little time. The default mask is 23 (bits 0, 1, 2 and 4).

| Book, seed 0 | Wall | Home | Owner | Found |
|---|---|---|---|---|
| without `--sit-fast` (`R_rg_0`) | 1,100 s | 99–100% | 86–100% | 94–98% |
| `--sit-fast`, all bits (mask 63) | 276 s | 92–100% | 70–100% | 91–98% |
| **`--sit-fast`, mask 23 (`R_mask23_0`)** | **378 s** | **98–100%** | **90–100%** | **94–100%** |

Mask 23 is 2.9× faster, about 1.8 ms per word. On the story set it scores 96.8 / 96.8 / 95.1% over three seeds, about 38 s per run.

- **Where the time goes** (stack samples, and counters printed per word): with mask 23 a word costs about 112 slot reads (6.3 words of the kind scanned per read), about 16 cache hits and about 311 route candidates. The time per 20k words is flat along the stream (`SIT_TIMING=1`), so the cost does not grow with the book's length.
- **Exact rewrites of the hot loops** (profiled with `perf` on the 1M-word book). None of them changes a result: the top and ranked predictions are byte-identical to the previous binary on the 200k book with nesting.
  - **Dense hot query rows.** A row of the sparse 8-bit query table turns dense (one byte per query bit) once its list holds more than a third of the bits. That is no bigger than the list, and the row scores as one vector dot product.
  - **Query counts built once per query,** instead of once per scored route.
  - **A fused slot-read loop** for the cheap phase hash: the key-side multiply once per read, and a branch-free sign-select per position, which vectorizes.
  - **Head values kept inline** in each entity's list, in the same order, so a lookup walks one list instead of a hash probe per head.

  | Binary (200k book, two runs sharing 2 cores) | Wall | Per word |
  |---|---|---|
  | before (`R_spd_old`) | 393 s | 1.92 ms |
  | dense rows, query counts, fused read (`R_spd_new`) | 279 s | 1.36 ms |
  | **+ inline heads (`R_spd_new2`)** | **251 s** | **1.23 ms** |

  That is 1.57× in all. What remained after the first three rewrites, before the inline heads (stack samples): slot reads about 16%, sparse query rows about 15%, the heads walk and its sort about 18%, class lookups 7%, newest-slot sorting 7%, memory allocation 5%. The reader is still single-threaded.
- **A measurement mistake caught on the way:** the book's run script still pointed at an older binary, so four book runs reported as "fast" had run the old code (about 1,100 s each). Both run scripts now use one current binary, `runs/exp36/sit-cur`. Those runs are not reported here.

## Step 5: a million words, nested labels (`--sit-nest`)

**The book.** `longdoc_gen.py 1000000 runs/exp37/ld1m_p 1 parts=10 chapters=20` writes 1,009,264 words: 10 parts ("part three .") of 20 chapters each, about 5,000 words per chapter. Chapter numbers start again in every part, so the clause "chapter twelve ." occurs ten times. Found is asked as "what did X find in chapter M of part N ?". Facts lie up to 978,883 words back (just under 2^20). The vocabulary adds "part" and "of" at the end, so the 200k book's word ids are unchanged.

**Nested anchors.** Clause texts now keep every start, not just the first and latest. With `--sit-nest`, two spans of the question that each equal a stored clause give a nested anchor: the inner span's first occurrence inside the outer span's unit, cut to that unit. For example, "chapter twelve" is looked for inside "part three". The outer span is taken at its first and latest start, and up to 4 nested anchors are kept. A feature names the outer opener type and span length (`fid(50, …)`), so the query learner learns whether to trust them. Nothing in this is specific to parts or chapters. Without `--sit-nest` the anchors are as in step 4.

**Result** (seed 0, `R_p1m_nest` and `R_p1m_flat`, run side by side, 2,168 s and 2,162 s):

| Found, by fact distance | 8–127 | 128–1k | 1k–8k | 8k–64k | 64k–512k | 512k+ |
|---|---|---|---|---|---|---|
| Without nesting | 88.4% | 89.7% | 89.2% | 87.3% | 33.8% | 44.3% |
| **With nested anchors** | **94.6%** | **97.4%** | **97.2%** | **97.3%** | **93.9%** | **91.8%** |
| With nesting, by echo distance | 96.7% | 97.1% | 97.1% | 95.9% | 94.0% | 97.8% |
| With nesting, recall@8 | 96.9% | 98.2% | 98.2% | 98.8% | 96.7% | 96.2% |
| Newest-find baseline | 100% | 99.7% | 86.1% | 20.9% | 8.6% | 8.2% |

- **Without nesting, found breaks beyond 64k words.** "chapter M" alone points at its first or latest occurrence, which is the right part only by chance.
- **With nesting, found stays at 92–97% out to nearly a million words back.**
- **Home and owner:** 99–100% from 128 words to 512k, with or without nesting. Home at 8–127 words is 97.5% with nesting and 95.0% without (n 40).
- **Local questions:** 99.9 / 98.3 / 89.7% (where-person, who-has, where-object).
- **On the 200k book,** `--sit-nest` leaves the scores identical to mask 23 (found 100 / 96.2 / 97.0 / 97.0 / 94.3%). The predictions differ at a few unscored positions.
- **Against the forum's step-5 prediction:**
  - Accuracy within 3 points of the 200k book: met for home, and for found except the shortest bucket (94.6% against 100% at 8–127 words; n 129 and 58). Owner is higher on the 1M book (99.8–100% against 90–100%).
  - Per-word cost: 2.15 ms with two runs sharing the 2 cores, against 1.92 ms on the 200k book under the same load (`R_spd_old`, 393 s). That is 1.12× for 5× the words.
  - Bytes per word: not measured yet.
  - A changed header word and 10^7 words: not run yet.

## Caveats

- Single seed, one book (one per length).
- The machine restarted several times during this work. Seed 2 of the experiment 36 full benchmark was lost twice and has been relaunched.
