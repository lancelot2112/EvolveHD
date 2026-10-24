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

- **Far-found errors fall from 792 to 9 of 1,916.** Right-chapter errors (wrong part or wrong book, counting every wrong answer that equals one of X's finds in that chapter) fall from 548 to 5.
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

- **Reading by the index pass matches the full read** (found within 3.3 points) **at about an eighth of the time.** The full read's time is estimated: its total of 4,816 s, less the 2,732 s that book A took in `R_ar_ans`. This assumes book A took as long in both runs; under heavier load, book A took 3,864–3,948 s in other runs here. Predicting every word is not needed to answer.
- **The book level removes most of the earlier-book errors.** Of 2,762 far-found questions, `R_ar_ans` gives book A's answer at the same part and chapter 178 times, and `R_ar_epoch2` 14 times (`why_wrong.py`). These are the errors `--sit-scope` cannot fix here: book A never repeats a part label, so the scope features never learned to tell outer occurrences apart. Earlier-book errors overall fall from 230 to 87, though "same chapter, other part, earlier book" rises from 52 to 73.
- **Home and owner without echoes are weak.**
  - For home, the right answer is often not among the 8 candidates (recall@8 41%).
  - For owner it always is (recall@8 100%), so its low scores (42.9% and 28.6%) are a ranking failure.
  - Owner has only 7 questions.
  - In the in-stream books, nearly every far home or owner question had been asked and answered before. On `ld1m_p`, 4,473 scored home questions at 8k+ words have an earlier echo and 5 have none; for owner it is 4,441 and 2. So their 99–100% was mostly echo.
  - Training on a book with no echoes (`ld1m_pn`, each fact asked once at a log-spread delay) does not fix it. On that book's own questions, home at 8k–64k scores 93% (43) and owner 71% (24). On book C (`R_nec_ans`) they score 29% (17) and 14% (7).
  - A route-logging run is under way.
- **A failure mode found on the way.** In a first `--sit-epoch` run, a bug (the inner span limited to the current epoch) made book C's first answers wrong. The question mood counter then dropped below its threshold, and the reader stopped answering after 84 of 3,000 questions. The question detector keeps adapting while skill is frozen, so a run of wrong answers can switch answering off.

## Step 3: learning skill faster from scratch

- **Question replay** (`R_p1m_rq4`: 4 replays per question of kept question mistakes) does not speed learning. Far found (fact at least 64k back), by 100k-word window from 0 to 1M words:
  - without replay: 59 / 62 / 80 / 98 / 97 / 91 / 98 / 100 / 99 / 99 / 100%;
  - with replay: 62 / 60 / 81 / 94 / 96 / 86 / 94 / 99 / 100 / 99 / 100%.

  By the forum's own falsifier, the slowness is credit starvation: the nested route can only be practised once there are earlier parts to ask about. It is not too few presentations.
- **Merging learned skill between shards** (`--sit-shard-learn`, deltas summed; the 200k book, 4 shards from a 20k-word warm-up, `R_sh4learn`) closes about a third of the gap to the sequential read (36% over the six cells, 15–71% by cell), and pass 1 then costs a full read.

| Stretch (shards 1–3) | Who-has | Owner |
|---|---|---|
| sequential | 97.8 / 95.4 / 92.2% | 98.4 / 99.7 / 99.3% |
| shards, facts only (`R_sh4`) | 86.7 / 82.1 / 74.7% | 78.1 / 87.1 / 59.4% |
| shards + summed skill | 90.4 / 88.4 / 83.1% | 92.6 / 89.2 / 65.5% |

The prediction (within 3–4 points of sequential) is not met.

## Step 4: a new vocabulary

Book C′ (`ld1m_s`) is book C with new words for every person, place, object and heading word ("section", "volume"), after book A. It is read by the index pass with skill frozen and `--sit-epoch` (`R_ps_word2`). Found scores 100 / 88.2 / 94.8 / 91.2% (1k–8k / 8k–64k / 64k–512k / 512k+); home at 8k–64k is 64.7% (17). That is as good as the same-vocabulary book (`R_ar_epoch2`) or better at 8k–512k, and slightly lower at 512k+ (91.2% against 92.4%).

- **The TEM seat's prediction for word-id features (at most 25% at 64k+) is falsified.** The reader keeps learning structure while it reads (kinds, classes, boundaries, opener types, all in `observe_only`). The new words join the old kinds, and the frozen route skill still fits. Frozen actions plus continual structure learning carry a trained reader into new words.
- **The role-features variant (`--sit-roles`) was not run;** it was not needed for this test.

## Step 5: a ladder toward natural text

The 200k book with one factor at a time (`R_lad_*`); the plain book is `R_sc200`:

| Book | Local (where / who / where-obj) | Home | Owner | Found |
|---|---|---|---|---|
| plain | 99.7 / 88.1 / 76.3% | 92–100% | 92–100% | 94–98% |
| pronouns for 50% of repeat subjects | 86.7 / 42.5 / 33.4% | 85–91% | 85–100% | 71–84% |
| + `--sit-anaphora` | 91.8 / 67.9 / 50.2% | 90–97% | 83–94% | 79–90% |
| modifiers in 30% of clauses | 99.4 / 81.8 / 66.9% | 40–45% at 8k+ | 55–77% at 8k+ | 94–100% |
| + `--sit-skip` | 99.6 / 91.7 / 65.6% | 92–100% | 73–100% | 94–100% |
| synonym verbs in 50% of statements | 99.8 / 91.6 / 73.5% | 85–98% | 83–100% | 91–98% |
| plain + `--sit-skip` | 99.7 / 89.4 / 75.4% | 99–100% | 92–100% | 94–98% |

- **Modifiers broke home and owner.** "mary finally lives in the kitchen" made a new exact path, so one relation split into many, and the variants crowded the old fact out of the per-kind cap. With skippable words left out of the path key, home at 8k–64k rises from 40% to 99%. The skippable words are learned from bigram counts; in prose the same rule would drop adjectives.
- **The modifier is not lost.** Questions about it ("who finally found the lamp in chapter M ?", `qual`, `R_lad_mod03q` and `_skip`) score 83–100% (88–90% at 64k+) with or without `--sit-skip`, with recall@8 at 97–100%. Every pair in a clause is bound, so "finally" keeps its person as a value.
- **Pronouns break local questions most,** because "she" becomes an entity. Stand-in resolution recovers about half of the loss. It resolved 2,268 clauses, more than the 1,999 pronouns in the book, so its test also fires on clauses that do not start with a pronoun (at least 269). The count does not show how many pronouns it caught.
- **Synonyms barely hurt.** "discovered" joins the kind of "found".
- **The language seat's predictions** were found and home at 8k+ ≤55% with pronouns (actual 82–91%), and home at 64k+ ≤60% with modifiers (actual 45%). Both fixes were to recover to within 5 points. `--sit-skip` meets it for home at 8k–64k (98.7% against 99.7%) but not at 64k+ (92.1% against 100%); `--sit-anaphora` does not meet it.

## Step 6: natural text (started)

BABILong (RMT-team on Hugging Face) has bAbI facts hidden in PG-19 book text. `babilong_stream.py` turns its samples into one word stream: each document, then its question and answer.

The pilot stream (`bl_pilot`, 308,702 words, vocabulary 19,018):
- training: qa1 0k, 1,000 samples; qa1 1k, 200 samples;
- test, frozen and answer-only: qa1 0k, 100 samples; qa1 4k, 50 samples.

**First runs were too slow.**
- **Cost.** With the default binding window of 8, a word of book text costs about 27 ms (23 ms averaged over words 40k–60k). That compares with about 1 ms on the stream's bAbI-only start, and about 2 ms on this experiment's synthetic books.
- **Where it goes.** A profile put 93% of the time in relation-class lookup, a scan over every class: natural text creates a new class for almost every new path.
- **The class index** removed the scan but not the cost. The shared bits are common, and class lookup plus sorting still took 82% (profiles from `perf`, not saved).
- **With `--sit-win 4`** a word costs about 9 ms (about 10 ms on book text). That run is under way.

**The first BABILong pilot run to finish** used `--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind` (`R_blpR`, 3,913 s).

| Part of the stream | Learning | Top-1 |
|---|---|---|
| training, qa1 0k (plain bAbI), 1,000 documents | on | 93.5% |
| training, qa1 1k (facts in about 1k words of book text), 200 documents | on | 78.0% |
| test, qa1 0k, 100 documents | frozen, answer-only | 0% |
| test, qa1 4k, 50 documents | frozen, answer-only | 0% |

- **While learning, it reads facts out of book text** (78% on the 1k documents, against chance of 1 in 6 places).
- **The test failed before answering anything:** 0 predictions at question boundaries.
  - **The learned boundaries are wrong.** At the end of the run they were ", and of"; "." and "?" had lost their boundary status. The both-sides-open rule compares each word with the most open word, and in book text the most frequent words (",", "and", "of") have far more distinct neighbours than "." or "?". With "?" no longer a boundary, no question is detected, and answer-only never predicts.
  - The rule needs a measure that does not grow with frequency.
- **The index pass is slow on book text: 20–26 ms per word.** By word 300k the long-term tier held 1.75M heads and 2.97M head-history entries. Each word binds to up to 8 earlier words, and almost every binding is a new (entity, path). This is the index growth the CPU and data-structures seats predicted (about 23 entries per clause). It needs their leaner layout: a clause log plus inverted lists, with paths re-parsed at question time.

## Hashing instead of searching

Three lookups searched, and on natural text each grew with the vocabulary:
- **relation classes:** a new path was compared with every class prototype;
- **slot reads:** an unbound slot was cleaned up against every word of its kind;
- **an entity's newest slots:** all of the entity's slots were gathered and sorted.

Each was replaced by a hash, all integer. On the story set (seed 0; base 96.8%):

| Change | Flag | Story set | Cost |
|---|---|---|---|
| relation classes by MinHash banding: 8 bands of 3 rows over the path code's bits; two paths share a class when a band collides; re-pooling becomes re-hashing | `--sit-lsh` | 96.8% | 24 hashes per new path, whatever the number of classes |
| (4 bands of 3 rows) | | 95.3% | |
| slot reads by bit sampling over fixed value codes: a value's phase at a key's i-th position depends on (value, i) only; 16 bands of 4 positions, multi-probing the positions whose unbound component is ambiguous | `--sit-vlsh` | 97.3% | 4.8 values scored per read, against 7.1 kind members |
| (bands of 8 positions) | | 89.9% | the estimate rarely survives a whole band |
| slot reads from the slot's write log: the slot's last 16 values, found by hashing the slot | `--sit-slotlog 16` | 96.6% | 3.4 values scored per read |
| + an entity's newest slots from a 32-entry recency list kept at write time | `--sit-recent 32` | 98.2% | |
| (16 entries) | | 91.6% | |

- **Bit-sampling slot reads do not scale to natural vocabulary.** With 19,018 words, a 4-position band (8 bits) holds about 80 words, and gathering candidates took 56% of the time (about 26 ms per word). Longer bands do not survive the store's noise. The write log is the hash that works there.
- **BABILong pilot speed** (ms per word over words 40k–60k / 60k–80k, where the book text starts):

  | Build | 40k–60k | 60k–80k |
  |---|---|---|
  | search, window 8 | 22.9 | — |
  | search, window 4 | 8.9 | 30.5 |
  | `--sit-lsh --sit-vlsh` | 14.4 | — |
  | `--sit-lsh --sit-slotlog 16` | 3.1 | 8.7 |
  | **+ `--sit-recent 32` `--sit-heads-nokind`** | **1.8** | **4.4** |

  The bAbI-only start costs about 1 ms per word. What is left at 60k–80k: query-row scoring 20%, the MinHash itself 18%, re-hashing all paths at re-pooling 13%.

## Home without echoes, found

Route logging (`SIT_QDBG_FROM`) on the read-then-ask run explains the missing home candidate:
- **When the answer was right,** the "lives in" head was among the person's 8 newest place-valued heads.
- **When it was wrong,** it was missing: newer place heads under the same paths, filed under old kind numbers after kinds were renumbered, had pushed it out of the per-kind cap.
- **The fix:** keying heads and their histories by (entity, exact path) only (`--sit-heads-nokind`, `R_ar_nokind`).
  - Home at 8k–64k: from 41% to 88% (17 questions).
  - Found: unchanged (79.0 / 87.6 / 92.2%).
  - Owner: still 14% (7 questions), with its answer among the candidates: a ranking failure.

## A slow, compressive memory

**The long-term tier was not compressive.** It was an exact, append-only index: every binding got a head and a history entry, and nothing decided what was worth keeping. On book text almost every path is new, so by word 300k it held 1.75M heads and 2.97M history entries. The layers below make what is stored earn its place. All are integer and of fixed or bounded size.

| Flag | What it does |
|---|---|
| `--sit-lt-min 4` | A binding enters the long-term tier only when its entity is a content word (a kind of 3 or more) and its exact path has been seen at least 4 times. Paths are counted in a fixed 512 KB count-min sketch (4 rows of 2^16 16-bit counters, the minimum of the four). |
| `--sit-lt-credit 64` | A path keeps entering it after its first 64 bindings only if a route through one of its heads or ranges has answered a question right (credited by the next word after a learned question boundary). |
| `--sit-wm-horizon 4096` | Working memory drops the bookkeeping of slots not written for 4,096 words: their live kinds, write times, write logs and the entity's class lists. |
| `--sit-path-forget 2` | Each re-pooling of relation classes forgets paths seen only once, instead of re-hashing them. |
| `--sit-path-min` | Paths are stored only once recurring. Story set 95.8% with N = 2; not used. |
| `--sit-bnd-hyst` | A learned boundary stays one while its own openness holds (at least a quarter of its peak), so prose adds boundaries (", a and of") without erasing ". ?". |
| freezing | `--sit-freeze-from` now freezes the question detector (mood counters) too. |

**Story set, seed 0** (each row adds to the one before):

| Stack | All |
|---|---|
| `--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind` | 94.0% |
| + `--sit-bnd-hyst` | 94.0% |
| + `--sit-lt-min 4` | 97.3% |
| + `--sit-lt-credit 64` | 97.5% |
| + `--sit-wm-horizon 4096` | 98.5% |
| + `--sit-path-forget 2` | 97.7%, with 3,694 stored paths against 10,246 |

**The BABILong pilot with the full stack.** The two middle columns use the stack without `--sit-path-forget`.

| Pilot (`bl_pilot`) | Without `--sit-wm-horizon` (`R_blpH2`) | With `--sit-wm-horizon` (`R_blpW2`) | + `--sit-path-forget 2` (`R_blpF`) |
|---|---|---|---|
| training qa1 0k, learning on | 95.0% | 95.0% | 95.0% |
| training qa1 1k (facts in about 1k words of book text), learning on | 96.0% | 97.0% | 96.0% |
| **test qa1 0k, frozen, answer-only** | **100%** | **100%** | **100%** |
| **test qa1 4k, frozen, answer-only** | **100%** | **98.0%** | **100%** |
| run time | 1,875 s | 1,841 s | 803 s |

- **Two things made the test work.**
  - **The boundaries.** Before the fix, "." and "?" were lost, no question was detected, and the test scored 0% (`R_blpR`, `R_blpG`).
  - **The frozen question detector.** In `R_blpH`, frozen route skill with an adapting detector answered only 19 of 50 qa1 4k questions before "?" stopped counting as a question (0%).
- **Store size at word 300k:** 44k heads and 81k history entries, against 1.75M and 2.97M with no gating (`R_blpR`).
- **The cheap read on book text** costs about 13 ms per word without `--sit-path-forget`; 66% of the time went to re-pooling (re-hashing all 955k paths) and 26% to hashing new paths. With it, about 5.5 ms.
- **Small samples:** 100 and 50 test documents, one seed. The 4k documents are about 3,000 words each.

## Packed kind codes and longer BABILong documents

**The speed fix (exact).** Forum round five's superoptimizer found that about 97% of the 5.5 ms per word on book text was one search: the leader clustering of word kinds, run every 2,000 words. Every seen word was compared with every leader, each compare zipping two 512-entry `Vec<bool>` codes.

- **The change:** kind codes are now `[u64; 8]` compared by popcount. The stand-in containment test uses the same packed codes. Kind naming uses a set instead of a list scan.
- **Exactness:** on an 80k-word prefix of the stream, `.top` and `.rank` are byte-identical to the old binary (`eqcheck.sh`).
- **Speed** at word 340k of the same stream: 10.1 s per 20k words, against 140.2 s before (about 14×).
- **The whole 950k-word run took 739 s.** The old binary was at 7 ms per word by word 340k and still slowing; it was stopped there.

**The longer test** (`bl_long`, `R_bl_long`):

- **Setup:**
  - the same training prefix as the pilot (qa1 0k and 1k, learning on);
  - then 30 qa1 documents at 16k (about 13k words each) and 8 at 64k (about 51k words each), frozen and answer-only from word 156,952;
  - the full stack of `R_blpF`, binary `sit-38y`.
- **Results:**

| Part | Top-1 |
|---|---|
| training qa1 0k (learning on) | 95.2% (n 1,000) |
| training qa1 1k (learning on) | 97.5% (n 200) |
| **test qa1 16k, frozen, answer-only** | **93.3% (28 of 30)** |
| **test qa1 64k, frozen, answer-only** | **87.5% (7 of 8)** |

- **Caveats:** one seed, with 30 and 8 test documents, so one document moves the 64k score by 12.5 points.
- **The store is not compressive at this length.** Heads grow about linearly once the test starts:
  - 17k at word 240k;
  - 159k at 400k;
  - 568k at 640k;
  - 1.09M at 940k (about 1.4 heads per word read in the test part).
- **Cost per word grows with it:** from 0.29 ms (word 240k) to 1.5 ms (word 940k).
- **Likely cause (not yet checked):** the `--sit-lt-min` count-min sketch has 4 rows of 2^16 counters. With a 33k-word vocabulary and on the order of a million distinct paths, every counter passes 4, so one-off paths read as recurring. Each new path also gets its first 64 bindings under `--sit-lt-credit`.
  - A sketch sized to the stream, or the forum's single importance rule with a fixed budget, should hold the store flat. That is the next thing to test.

## Lost run

The three-book read-then-ask (`ld3m_pqr`: A, B, then C) hit the 2-hour limit for background jobs and was stopped. It was not restarted, because `--sit-epoch` answers the same question more directly.

## Caveats

- **One seed throughout,** with small samples for home and owner in step 2 (17 and 7 questions).
- **Timings come from a slower machine,** with runs sharing 2 cores.
- **All route choice uses the sparse 8-bit query table.**
