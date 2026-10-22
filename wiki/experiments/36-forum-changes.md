# 36 · The forum's changes, in Rust: from 70% to 86% on the stories (2026-10-22)

*[All experiments](README.md)* · [← 35](35-axes-of-memory.md)

**Concepts:** [Go/NoGo selection](../concepts/go-nogo-selection.md) · [Sparse codes](../concepts/sparse-codes.md) · [Credit without backprop](../concepts/credit-without-backprop.md)

Question: do the changes the expert forum agreed on (the document "The composite model: a forum's consensus") lift the situation model on the comprehension stories, and which of them matter? Each change is a switch in `src/situation.rs`. Inside the store everything is integer: counters, shifts and comparisons by cross-multiplication. The class pooling's float division was removed too. Learning is local, and the only teacher is the next word.

Benchmark: the small comprehension set (`runs/exp31c/small/syn300`, 24,659 words, 596 questions in the second half: 435 where-person, 81 where-object, 80 who-has). Scored on the **top answer, with no kind filter** (`scripts/exp36/score.py`). Command: `evolvehd assoc --sit --data IDS --vocab TXT --sit-pool 0.5 [switches] --out PREFIX`.

## Results

| Switches (cumulative) | Where-person | Where-object | Who-has | All, seeds 0 / 1 / 2 |
|---|---|---|---|---|
| base | 88.5% | 29.6% | 12.5% | 70.3 / 71.1 / 70.5% |
| `--sit-kinds` | 90.1% | 33.3% | 18.8% | 72.8 / 72.8 / 73.3% |
| + `--sit-mood` | 90.1% | 34.6% | 18.8% | 73.0 / 73.0 / 73.5% |
| + `--sit-recency` | 91.0% | 34.6% | 52.5% | 78.2 / 77.5 / 78.0% |
| + `--sit-entity-cues` | 91.5% | 35.8% | 53.8% | 78.9% (seed 0) |
| + `--sit-live` (and typed clean-up) | 92.0% | 44.4% | 65.0% | 81.9% (seed 0) |
| **+ `--sit-nroutes 8 --sit-cred 8`** | **93.3%** | **49.4–55.6%** | **82.5–85.0%** | **85.9 / 87.1 / 86.1%** |
| + `--sit-compose 6` | 93.6% | 51.9% | 83.8% | 86.6% (seed 0) |

Columns other than "All" are seed 0, except the bold row (all three seeds). A run takes 54 s, down from 350–430 s: reading an entity's live slots replaces scanning 40 classes for every cue word.

What each switch is:
- **`kinds`.** Value kinds learned from frames: the word before, the two words before, the word after. They are 8-bit counters, halved together, clustered by Jaccard ≥ 2/5. Slots are keyed on (entity, class, kind). People, places and objects each come out as one kind. With 4-bit frame counters, people split into two kinds by which verb each had used last: a frame is a noisy observation, and experiment 35's rule says noisy observations need more bits.
- **`mood`.** Questions don't write. A clause's bindings are buffered until its boundary. A boundary word's counter steps up when the store predicted the next word and down on half the misses, so it climbs above a one-third hit rate. A clause closed by a high-counter boundary is dropped. The counter found "?" and "is" by itself; "." stays a fact boundary.
- **`recency`.** Among a cue word's credible proposals of one kind, the newest first slot wins.
- **`entity-cues`.** Only words of learned kinds with at least 3 members act as cues. Function words' slots ("the", "?") held whatever was written beside them last, a "latest place mentioned" buffer that won by luck about a third of the time.
- **`live`.** Learn over the cue entity's live slots, and for two hops the intermediate's, instead of the 10 most frequent classes. The two-hop route "object → its getter → the getter's place" was never searched before, because "was got by" is a rarer class. A slot of kind k is read against words of kind k only (typed clean-up).
- **`nroutes`, `cred`.** Propose up to 8 live routes per cue word. A route competes on recency if its count is at least an eighth of the best. The route for the newest fact ("bill grabbed the pen") was a less-counted class than stale ones from earlier stories, so it was cut before recency could choose it.
- **`compose`.** Write-time composition: committing a fact also reads and stores one counted two-hop result. It adds about half a point.
- **`bounce`** (the hop loop with a learned stop, experiment 36's `e6c_policy.py` design) and **`supersede`** (detach writes NONE) did not help here. The bounce loop does not yet use the wider route set. On the box world the same design reaches 100% at every depth (below).
- **`precision`.** Ranking routes by hits over hits plus misses lifted who-has to 77.5% but dropped where-person to 61–73%: rare, lucky routes beat well-tested ones. Go minus NoGo kept where-person and cost where-object. Neither is in the best row.

## Where the remaining errors are

- **Before the boundaries are learned.** "?" becomes a boundary only at word 14,309, a sixth of the way into the scored half (`SIT_DEBUG_BND=1` prints boundary changes). Before that, questions merge with their answers. Scored separately (seed 0, `--sit-live` row): 58.8% before, 86.6% after.
- **Held objects whose holder has not moved** (58 of the 81 where-object questions, `scripts/exp36/wo_cases.py`): 43% right. Stale holders from earlier stories stay live, because nothing resets at a story start (the learned story detector of the Python prototype fired on "took" and "apple" too). Drops are only 6 of the 81.

## The hop loop on the box world

`scripts/exp36/e6c_policy.py` is experiment 35's box world with the oracle stop removed and a frequent distractor relation ("looks at") with its own question. Policies, 3 seeds each:
- *Newest slot*: 54% on where-questions and about 50% on looks-at.
- *Learned Go/NoGo*, per (question, kind of the current entity, relation, recency rank), with a learned stop per (question, kind, steps taken): **100% at every depth from 1 to 5+, and 100% on looks-at, with 0.00 hops of overshoot.**
- *Supersede*, no write times: also 100%.

The policy compares by cross-multiplication and halves its counts with shifts.

## Against a transformer

`scripts/exp36/tf_babi.py`: a 2-layer, 4-head, width-128 causal transformer (426k parameters, context 128), on the same small set.

| Model | Training | All |
|---|---|---|
| this model | one pass, learning while reading | **86–87%** |
| transformer | one pass, one Adam step per 64 words | 12.8% |
| transformer | first half only, many epochs | peak 40.9% at 3 epochs (2.5M positions), then overfits to about 31–34% |

The second half's new verbs and question form are untrained embeddings for the transformer. The full 817k-word benchmark, where a transformer has enough data, has not been run yet.

## Caveats

- **Single seed** except where three are listed.
- **The small set only.**
- **The machinery is designed.** Every learner fills in statistics inside a structure someone chose: clause bindings, slot keys, the mood gate, the frame features, recency within an entity, the thresholds. Discussed in the conversation: the next step is a general query learner, a bitwise W_QK trained by a three-factor rule.

## The general query learner and an integer pooler (`--sit-query`, `--sit-fpool`)

**The query learner** replaces the route tables, the context hash, the cue positions, the entity-cue rule, the recency rule and the credibility thresholds with one learned map, a bitwise W_QK:
- **The query code** is built from the last 4 words: their frame codes, each in its position's block; identity bits; and 4 bits hashed from the words 1 and 3 back.
- **The candidate routes** are every one- and two-hop route from the cue words' newest live slots. Each route is a set of features: the cue position, the cue word's kind, and per hop the class, the kind and the slot's recency rank among the entity's values of the same kind, plus cross-hop conjunctions.
- **The weights** are signed integers per (query bit, feature). A route's score is the sum of its weights under the query, and the best route is read.
- **Learning** is mistake-driven and three-factor: when the top route was wrong, the best-scored route that held the next word steps up and the top route steps down.

**The frame-code pooler** pools relations by the words' 8-bit frame counters, the same counters the kinds use, instead of the meaning module, whose codes are float inside. Only distinctive features count: those in at most an eighth of the words' codes. The meaning module then does not run.

| Configuration (with `--sit-kinds --sit-mood`) | Where-person | Where-object | Who-has | All, seeds 0 / 1 / 2 |
|---|---|---|---|---|
| hand-built routes (best row above) | 93.3% | 49–56% | 83–85% | 85.9 / 87.1 / 86.1% |
| query learner, recency rank over all slots | 92–94% | 31–37% | 63–71% | 82.0 / 80.7 / 81.9% |
| query learner, recency rank within the value's kind | 92–94% | 42–46% | 78–91% | 84.4 / 86.9 / 84.1% |
| **query learner + frame-code pooler (Jaccard ≥ 0.7)** | **96.8–97.0%** | **46–58%** | **81–88%** | **90.3 / 87.9 / 88.4%** |

- **The hand rules are learned now.** The learner finds that "the" and "?" are not worth reading from, and when the newest slot matters. It only needed the recency rank counted among values of the same kind. A clause like "fred picked up the cup" also writes "up", "picked" and "." into the cup's slots, and those had taken the top ranks.
- **The integer pooler is better with the learner than the meaning module was.** It keeps "went / moved / walked / travelled to the" together. It needs a stricter threshold (0.7, giving 1,048 classes; 0.65 gives 84.6%, 0.5 gives 77.9% with the hand-built routes) so that the one-word paths do not swallow the longer ones. So it compresses less: 9,590 paths into 1,048 classes, against 144.
- **What did not help:**
  - replaying kept mistakes: 81.4% and 80.5% with 2 and 4 per word, before the rank fix; 88.6% against 90.3% after it;
  - 16-bit weights: no change, so nothing was saturating;
  - crediting every route that held the answer: 52%, as the frequent routes swamp the rest.
- **It is faster:** a run takes 40 s, against 54 s with the hand-built routes and 350 s for the base.
- **What is still not integer:** the store's output weights handed to the engine, as before. Everything inside the store is integer now, including the pooling.

## Knowing when it doesn't know, and looking it up (`--sit-lookback 128`)

The transformer reads its whole 128-word context exactly every time. The fair counterpart here is a lookback window of the last 128 words, read by the same query learner. Its routes start from a cue word's latest occurrences (up to 3) in the window and take the word at an offset of −6 to +6; from that word's latest 2 occurrences, they take the word at an offset again (two hops). The features are the cue position, the cue word's kind, the occurrence rank, the offset and the value's kind. Both the slots and the window learn at every word.

An arbiter decides which answer to give. It counts, per (question form, both sources' margin buckets), which source was right where the two disagreed, and answers from the window where the window has been right more often. Three seeds, `scripts/exp36/agree.py` on `SIT_WDBG=1` logs:

| Seed | Slots alone | Window alone | Given (arbiter) | Agree: share of questions | Agree: right | Disagree: slots right | Disagree: window right |
|---|---|---|---|---|---|---|---|
| 0 | 87.2% | 79.4% | 88.4% | 73.7% | 97.7% | 58.0% | 28.0% |
| 1 | 89.4% | 57.4% | 89.3% | 55.9% | 96.7% | 80.2% | 7.6% |
| 2 | 86.2% | 83.9% | 86.7% | 78.9% | 97.7% | 43.7% | 32.5% |

- **Agreement is the "I know" signal.** When the slots and the window give the same answer, 96.7–97.7% of those answers are right, over 56–79% of the questions. The score margin is not a usable signal: nearly every answer, right or wrong, has a large margin. For where-object, the agreeing answers are 74–86% right; overall they are 30–38% of the where-object questions.
- **Looking it up is only as good as the reader of the window, and that reader is weak.** Alone it gets 57–84%. It must learn offsets in raw text, which is slow: the transformer needed 2.5M training positions to reach 41%. Where the two disagree, the slots are right more often in all three seeds. The window helps only on where-object, and only on seed 0: 50% right against the slots' 19% there.
- **So the net effect is small:** +0.5 to −1.7 points over the slots alone, across arbiter variants. A first arbiter keyed on the slots' margin and a two-word question form (90.8 / 88.8 / 86.2%) mixed up "who has the X ?" with "where is the X ?". One pool scored by the shared weights overused the window (85.1 / 83.6%).
- **What would make it pay:** a better window reader (the next step), or answering "unknown" on disagreement. Answering only when the two agree would give about 97% right on 56–79% of the questions.
