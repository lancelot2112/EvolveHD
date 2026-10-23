# 38 · Read quickly, answer questions: the forum's round-four plan (2026-10-23)

*[All experiments](README.md)* · [← 37](37-long-reader.md)

The owner's goal: a system that can quickly read something and answer questions about it. The fourth forum ([Claude Docs](https://claude.ai/artifact/WazQC5jF9GWmZSYjZoZ6Ne), project doc `claude/forum-round-four.md`) agreed on the direction: read cheaply, answer expensively. Reading is the index pass. Routes are scored only at questions. Skill is trained once and then frozen. The forum's order of work:

1. Fix the two-books label clash.
2. Read, then ask.
3. Learn skill fast from scratch.
4. A new vocabulary.
5. A ladder toward natural text.
6. Natural text.

This page carries out steps 1–5 and starts step 6.

All runs use experiment 37's best configuration (`runs/exp37/F.flags`) plus `--sit-nest`, seed 0. Two caveats on timing:
- **The machine is slower than in experiment 37.** The same binary and flags took 422 s on the 200k book (`R_ns200`, byte-identical predictions to `R_spd_new2`) against 251 s then.
- **Most runs shared 2 cores with one or two others.** Compare times only within a table.

## New pieces

| Flag or option | What it does |
|---|---|
| `--sit-scope` | Every anchor carries its occurrence rank from the newest (0 the latest, 1 the one before, 2 any older). A nested anchor carries the rank of its outer occurrence. New route features conjoin opener type × which occurrence × rank, and a nested anchor's outer type and rank × its inner type. Another feature counts how many of the question's anchor units contain the candidate event. Nested anchors keep the newest 4, not the oldest. |
| `--sit-epoch` | The level above the top labels. An opener type's epoch restarts where its first clause text recurs: "part one ." again starts a new book, "chapter one ." again a new part. An anchor looks only at occurrences in its type's current epoch, when there are any. A nested anchor's inner span is still searched inside the outer unit over all its occurrences. |
| `--sit-answer-only T` | From word T, read without predicting (`observe_only`), except right after a learned question boundary. |
| `--sit-freeze-from T` | From word T, no route learning: no query learning and no open-query credit. |
| `--sit-replay-q` | Replay only the mistakes made answering questions, at questions (with `--sit-replay N`). |
| `--sit-roles` | Opener types enter route features as a level (bit length of the span of their starts minus bit length of their count less one, about log2 of the recurrence interval), not as a word id. |
| `--sit-skip` | Skippable words are left out of the exact relation-path key. A word is skippable when, at at least three quarters of its (at least 16) occurrences, the word before it has already been seen directly followed by the word after it. |
| `--sit-anaphora` | Stand-in first words. A clause whose first word belongs to a small kind (under 3 members) whose kind code lies at least three quarters inside the code of a content kind's most frequent word has its facts filed under that kind's newest clause-initial word. |
| class index | Relation classes are indexed by their prototype bits, so a class lookup counts only classes that share a bit. The predictions are byte-identical (`R_sc200_i`). |
| `longdoc_gen.py` options | `noq` (no questions in the text) with `qend=N` (N long-range questions at the end, each fact at most once); `fresh` (new words for every person, place, object and heading word); `noecho` (each long-range fact asked once, at a log-uniform delay after its statement); `pron=P`, `mod=P`, `syn=P` (the ladder, on a random generator of their own); `qual` (questions about a modifier). |
| scripts | `scripts/exp37/concat_books.py` (one book after another, scored on the last); `scripts/exp38/baselines.py`; `why_wrong.py` (wrong far-found answers by kind of error); `curve.py` (accuracy in windows); `babilong_stream.py`, `score_babilong.py`. |

## Step 1: the two-books label clash

On `ld2m_pq` (book A, then book B, questions in the text), scored on book B:

| Found by fact distance | 8–127 | 128–1k | 1k–8k | 8k–64k | 64k–512k | 512k+ |
|---|---|---|---|---|---|---|
| before (`R_ab_seq`) | 93.8% | 93.8% | 93.9% | 92.2% | 56.6% | 69.1% |
| **`--sit-scope` (`R_ab_scope`)** | **98.9%** | **99.7%** | **99.4%** | **99.3%** | **99.4%** | **100%** |

- **Far-found errors fall from 792 to 9 of 1,916.** Right-chapter, wrong-part errors fall from 548 to 5.
- **Other questions:** home 100% and owner 99.5–100% in every bucket; local questions 100 / 98.7 / 91.7%.
- **The prediction (≥88–90%) is met.**
- **On a single book** (the 200k book, `R_sc200` against `R_ns200`, same load): found 98.3 / 96.7 / 97.3 / 96.8 / 93.7% against 100 / 96.2 / 97.0 / 97.0 / 94.3%. Who-has is 88.1% against 90.6%. The run takes 438 s against 422 s.

## Step 2: read, then ask

**The test.** Book C (`ld1m_r`, seed 3, 1,044,804 words) has no questions in its text and 3,000 long-range questions at its end: 2,964 found, 24 home, 12 owner, each fact asked once. The stream `ld2m_pr` is book A (`ld1m_p`, read in full with its questions and learning on) and then book C, scored on C's questions. With `--sit-answer-only` and `--sit-freeze-from` at the start of C, the reader reads C by the index pass and answers at the end with skill frozen. The reference reads C in full, predicting every word, frozen only for the end questions.

| Book C | Found 1k–8k / 8k–64k / 64k–512k / 512k+ | Home 8k–64k | Owner 8k–64k | Time for book C |
|---|---|---|---|---|
| question-only prior (`baselines.py`) | 6.7 / 11.3 / 8.0 / 8.0% | 11.8% | 14.3% | — |
| newest value (X's latest find) | 93.3 / 16.7 / 7.6 / 8.9% | 100% | 100% | — |
| BM25 over 128-word chunks | 0 / 4.3 / 6.2 / 7.4% | 11.8% | 0% | — |
| the structure read by rule (ceiling) | 100% everywhere | 100% | 100% | — |
| full read (`R_ar_full`) | 80.0 / 74.7 / 81.7 / 88.3% | 41.2% | 42.9% | about 2,080 s |
| **index pass, answers only at questions (`R_ar_ans`)** | **80.0 / 73.7 / 85.0 / 88.3%** | **41.2%** | **100%** | **256 s (0.25 ms per word)** |
| + `--sit-epoch` (`R_ar_epoch2`) | 100 / 79.0 / 87.6 / 92.4% | 41.2% | 28.6% | 393 s |

Question counts: found 15 / 186 / 1,442 / 1,320, home 17, owner 7.

- **Reading by the index pass matches the full read** (found within ±3 points) **at about an eighth of the time.** The full read's time is estimated: its total of 4,816 s, less the 2,732 s that book A took in `R_ar_ans`. Predicting every word is not needed to answer.
- **The book level removes most of the earlier-book errors.** Of 2,762 far-found questions, `R_ar_ans` gives book A's answer at the same part and chapter 178 times, and `R_ar_epoch2` 14 times (`why_wrong.py`). These are the errors `--sit-scope` cannot fix here: book A never repeats a label, so the scope features never learned to tell occurrences apart.
- **Home and owner without echoes are weak.** The right answer is often not among the 8 candidates (home recall@8 41%).
  - In the in-stream books, nearly every far home or owner question had been asked and answered before. On `ld1m_p`, 4,473 home questions at 8k+ words have an earlier echo and 5 have none. So their 99–100% was mostly echo.
  - Training on a book with no echoes (`ld1m_pn`, each fact asked once at a log-spread delay) does not fix it. On that book's own questions, home at 8k–64k scores 93% (43) and owner 71% (24). On book C (`R_nec_ans`) they score 29% (17) and 14% (7).
  - A route-logging run is under way.
- **A failure mode found on the way.** In a first `--sit-epoch` run, a bug (the inner span limited to the current epoch) made book C's first answers wrong. The question mood counter then dropped below its threshold, and the reader stopped answering after 84 of 3,000 questions. The question detector keeps adapting while skill is frozen, so a run of wrong answers can switch answering off.

## Step 3: learning skill faster from scratch

- **Question replay** (`R_p1m_rq4`: 4 replays per question of kept question mistakes) does not speed learning. Far found (fact at least 64k back), by 100k-word window from 0 to 1M words:
  - without replay: 59 / 62 / 80 / 98 / 97 / 91 / 98 / 100 / 99 / 99 / 100%;
  - with replay: 62 / 60 / 81 / 94 / 96 / 87 / 94 / 99 / 100 / 99 / 100%.

  By the forum's own falsifier, the slowness is credit starvation: the nested route can only be practised once there are earlier parts to ask about. It is not too few presentations.
- **Merging learned skill between shards** (`--sit-shard-learn`, deltas summed; the 200k book, 4 shards from a 20k-word warm-up, `R_sh4learn`) closes about 40% of the gap to the sequential read, and pass 1 then costs a full read.

| Stretch (shards 1–3) | Who-has | Owner |
|---|---|---|
| sequential | 97.8 / 95.4 / 92.2% | 98.4 / 99.7 / 99.3% |
| shards, facts only (`R_sh4`) | 86.7 / 82.1 / 74.7% | 78.1 / 87.1 / 59.4% |
| shards + summed skill | 90.4 / 88.4 / 83.1% | 92.6 / 89.2 / 65.5% |

The prediction (within 3–4 points of sequential) is not met.

## Step 4: a new vocabulary

Book C′ (`ld1m_s`) is book C with new words for every person, place, object and heading word ("section", "volume"), after book A. It is read by the index pass with skill frozen and `--sit-epoch` (`R_ps_word2`). Found scores 100 / 88.2 / 94.8 / 91.2% (1k–8k / 8k–64k / 64k–512k / 512k+); home at 8k–64k is 64.7% (17). That is as good as the same-vocabulary book (`R_ar_epoch2`), or better.

- **The TEM seat's prediction for word-id features (at most 25% at 64k+) is falsified.** The reader keeps learning structure while it reads (kinds, classes, boundaries, opener types, all in `observe_only`). The new words join the old kinds, and the frozen route skill still fits. Frozen actions plus continual structure learning carry a trained reader into new words.
- **The role-features variant (`--sit-roles`) was not run.** It cannot improve on this.

## Step 5: a ladder toward natural text

The 200k book with one factor at a time (`R_lad_*`); the plain book is `R_sc200`:

| Book | Local (where / who / where-obj) | Home | Owner | Found |
|---|---|---|---|---|
| plain | 99.7 / 88.1 / 76.3% | 92–100% | 91–100% | 94–98% |
| pronouns for 50% of repeat subjects | 86.7 / 42.5 / 33.4% | 85–91% | 85–100% | 71–84% |
| + `--sit-anaphora` | 91.8 / 67.9 / 50.2% | 90–97% | 83–94% | 79–90% |
| modifiers in 30% of clauses | 99.4 / 81.8 / 66.9% | 40–45% at 8k+ | 55–77% at 8k+ | 94–100% |
| + `--sit-skip` | 99.6 / 91.7 / 65.6% | 92–100% | 73–100% | 94–100% |
| synonym verbs in 50% of statements | 99.8 / 91.6 / 73.5% | 85–98% | 83–100% | 91–98% |
| plain + `--sit-skip` | 99.7 / 89.4 / 75.4% | 99–100% | 91–100% | 94–98% |

- **Modifiers broke home and owner.** "mary finally lives in the kitchen" made a new exact path, so one relation split into many, and the variants crowded the old fact out of the per-kind cap. With skippable words left out of the path key, home at 8k–64k rises from 40% to 99%. The skippable words are learned from bigram counts; in prose the same rule would drop adjectives.
- **The modifier is not lost.** Questions about it ("who finally found the lamp in chapter M ?", `qual`, `R_lad_mod03q` and `_skip`) score 83–100% (88–90% at 64k+) with or without `--sit-skip`, with recall@8 at 97–100%. Every pair in a clause is bound, so "finally" keeps its person as a value.
- **Pronouns break local questions most,** because "she" becomes an entity. Stand-in resolution recovers about half of the loss. It resolved 2,268 clauses, which is fewer than the pronouns present, so its test misses many.
- **Synonyms barely hurt.** "discovered" joins the kind of "found".
- **The language seat's predictions** were found and home at 8k+ ≤55% with pronouns (actual 82–91%), and home at 64k+ ≤60% with modifiers (actual 45%). Both fixes were to recover to within 5 points: met by `--sit-skip` for home, not met by `--sit-anaphora`.

## Step 6: natural text (started)

BABILong (RMT-team on Hugging Face) has bAbI facts hidden in PG-19 book text. `babilong_stream.py` turns its samples into one word stream: each document, then its question and answer.

The pilot stream (`bl_pilot`, 308,702 words, vocabulary 19,018):
- training: qa1 0k, 1,000 samples; qa1 1k, 200 samples;
- test, frozen and answer-only: qa1 0k, 100 samples; qa1 4k, 50 samples.

**First runs were too slow.** With the default binding window of 8, a word costs about 23 ms in the book text, against about 1 ms in the synthetic books. 93% of the time went to relation-class lookup, a scan over every class; natural text creates a new class for almost every new path. The class index removed the scan but not the cost: the shared bits are common, and class lookup plus sorting still took 82%. With `--sit-win 4` a word costs about 8 ms. That run is under way.

## Lost run

The three-book read-then-ask (`ld3m_pqr`: A, B, then C) hit the 2-hour limit for background jobs and was stopped. It was not restarted, because `--sit-epoch` answers the same question more directly.

## Caveats

- **One seed throughout,** with small samples for home and owner in step 2 (17 and 7 questions).
- **Timings come from a slower machine,** with runs sharing 2 cores.
- **All route choice uses the sparse 8-bit query table.**
