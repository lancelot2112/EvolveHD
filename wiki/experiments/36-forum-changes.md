# 36 · The forum's changes, in Rust: from 70% to 95% on the stories (2026-10-22)

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

## Holding many questions open, and letting the system choose how many (`--sit-sq N`)

An **open query** is a two-hop pattern kept up to date while reading. Written as (class, kind) then (class, kind), an example is "an object's getter, then the getter's place". Each open query has a derived slot per entity:
- **First-hop fact:** a fact that matches the first hop moves the entity's pointer and refreshes its slot.
- **Second-hop fact:** a fact that matches the second hop refreshes every entity pointing to that fact's subject. "fred went to the kitchen" moves every object fred holds.
- **Reading:** the query learner reads derived slots like any other.

`N` is only a ceiling. How many are open is decided by each query's integer balance:
- **The balance:** it starts at 512 and pays 1 per read or write it does. It gets credit when it helps, and loses 64 when an answer given through its slot was wrong.
- **Closing:** a query closes when its balance is below 0 after a 2,000-word trial.
- **Opening:** every 64 words, the best candidate opens if there is room.

All of it is integer, and the judge is the next word.

**Which queries open (top-down, mode 0):** the two-hop patterns that held answers to questions at least 16 times.

**Two changes made it work:**
- **Contribution credit (`--sit-sq-credit 1`).** A query is paid +32 when its slot gave the right answer and +16 when the answer given was wrong but its slot held the right one. It gets nothing when another route answered right anyway. Before, any query whose slot happened to hold the answer was paid. Riders stayed open, and cap 64 fell to 88.6%.
- **Attention to derived slots by balance (`--sit-sq-rank 1`).** An entity's reader sees at most 4 derived slots, now the 4 with the highest balance instead of the 4 newest. With many open, new trial queries had pushed the paying ones out of the reader's view. That is a capacity limit on the read side, not the write side.

**Capacity sweep** (`--sit-sq-credit 1 --sit-sq-rank 1`, 3 seeds; "open" is averaged over every word read, including the first ~15k words before "?" is learned as a question boundary, when none are open):

| Cap | All, seeds 0 / 1 / 2 | Mean | Where-person | Where-object | Who-has | Mean open | Peak open | Opened | Upkeep per word |
|---|---|---|---|---|---|---|---|---|---|
| none | 90.3 / 87.9 / 88.4% | 88.9% | 96.9% | 50.2% | 84.6% | — | — | — | — |
| 1 | 88.9 / 89.6 / 90.3% | 89.6% | 96.6% | 54.7% | 86.7% | 0.38 | 1 | 5 | 0.30 |
| 2 | 91.1 / 90.3 / 91.1% | 90.8% | 97.8% | 56.8% | 87.5% | 0.76 | 2 | 10 | 0.55 |
| 4 | 90.9 / 90.8 / 90.9% | 90.9% | 97.9% | 56.0% | 87.9% | 1.5 | 4 | 16–17 | 0.78–0.87 |
| 8 | 91.1 / 91.1 / 91.4% | 91.2% | 97.9% | 57.6% | 89.2% | 2.9 | 8 | 28–29 | 1.5–1.7 |
| **16** | **91.8 / 91.4 / 92.6%** | **91.9%** | **98.0%** | **62.6%** | **88.7%** | **4.5–4.8** | **15–16** | **44–47** | **2.7** |
| 64 (self-tuned) | 91.8 / 91.4 / 92.6% | 91.9% | 98.0% | 62.6% | 88.7% | 4.5–4.8 | 15–16 | 44–47 | 2.7 |

- **The number open tunes itself.** Above 16 the cap never binds: cap 64 gives the same runs as cap 16, bit for bit. With room, the system holds about 5 on average over the run. Because none are open in the first ~15k words, that is very roughly 12 while queries are possible. It peaks at 15–16 while trials come and go, and closes 32 of the 44–47 opened.
- **Accuracy grows with capacity and then stops.** From none to self-tuned the mean rises by 3 points (88.9 → 91.9%), mostly in where-object: 50.2% becomes 62.6% (seeds 46–58% become 60–64%). That is the two-hop question the open queries keep answered.
- **The price grows with the number open.** At the margin, upkeep is about 0.55 reads and writes per word per open query (0.7–0.8 at caps 1 and 2). The best setting costs 2.7 per word.
- **Without the two fixes, more capacity hurt.** Seed 0 was 91.1% at cap 4 and 88.6% at cap 64.

**Letting the input drive which queries open** (modes 1 and 2, cap 64, 3 seeds):
- **Mode 1:** every committed fact proposes the two-hop chains starting from it (counted). The higher level then ranks the proposals: the chain's route features summed under the learned query weights, over the last 32 question codes. This gave **86.4 / 83.4 / 89.1%**, with 12–37 open on average, 136–249 opened, and 6–9 upkeep per word. The query weights are trained to choose what to *read*, and they turned out to be a poor judge of what is worth *holding*. They rank chains ending in "to", "?" and "." highly, so the set churns.
- **Mode 2:** as mode 1, but a chain must also have held 4 answers. This gave **89.6 / 87.2 / 90.6%**, with about 9 open. That is better than mode 1, but only level with having no open queries (mean 89.1% against 88.9%), and below top-down choice.
- **So for now:** the input drives attention at *read* time. Each word's query, built from the last 4 words, chooses among the open queries' slots and the facts. Which questions to *hold* is chosen by what has answered questions. A bottom-up proposer would need a judge trained for holding, for instance the credit itself fed back to the proposals' features.

**What the open queries are:** many go through function-word or verb intermediates. Examples are "(class 929, kind .) then (class 1, kind office)" and "(class 0, kind moved) then (class 28, kind office)": "the latest place written after this word". They are paid only for what they contribute, so they earn their place. But they are recency buffers, not the clean "object → holder → place" chain.

**Caveats:** the small set only. The constants are designed: the amounts, the 2,000-word trial, the opening count of 16, and the read budget of 4. A self-scaling price, staying open while a query's payoff per word beats the open set's average, would remove most of them.

## A relative price, and letting the chain query through

**The chain query.** The clean two-hop question is "object → its holder → the holder's place". The open queries seldom held it, so `SIT_SQ_FORCE=a,b` (a test switch) held one open from word 2,048. It is a pattern over the kinds of its values only: (any relation, kind person) then (any relation, kind place). With it open, two blocks showed up:
- **The reader saw only the 8 newest slots.** The second hop from the holder read the person's 8 newest slots, and so did the query learner's own two-hop routes. "daniel picked up the key ." writes "picked", "up", "." and "key" into daniel's slots and pushes "office" out of the list, so the chain kept a stale place.
  - **Fix for open queries:** the second hop of an open query finds the newest value of its kind over all the intermediate's slots.
  - **Fix for the reader (`--sit-kindfirst`):** the read list takes each kind's newest value before any kind's second newest.
- **Two question forms gave the same question-form bits.** "who has the X ?" and "where is the X ?" share the words 1 and 3 back, so once the chain answered "where is the X ?" the learner used it for "who has" too: who-has fell from 87.5% to 65%. The fix, `--sit-qform14`, adds 4 query bits hashed from the words 1 and 4 back.

| Seed 0 | All | Where-object | Who-has |
|---|---|---|---|
| chain forced open, before the fixes | 89.9% | 59.3% | 83.8% |
| + second hop over all slots | 89.4% | 74.1% | 65.0% |
| + `--sit-qform14` | **93.3%** | 74.1% | 92.5% |

**With the reader fixed, the system finds the chain by itself.** The query learner's two-hop route reaches the holder's place once it is in view. Self-tuned open queries (cap 64) then add the cases where the holder moved since. Three seeds:

| Configuration (query learner + frame-code pooler) | All, seeds 0 / 1 / 2 | Mean | Where-object | Mean open | Upkeep per word |
|---|---|---|---|---|---|
| open queries, before (previous section) | 91.8 / 91.4 / 92.6% | 91.9% | 60–64% | 4.5–4.8 | 2.7 |
| `--sit-kindfirst`, no open queries | 92.1 / 91.8 / 93.6% | 92.5% | 68–74% | — | — |
| `--sit-kindfirst --sit-qform14`, no open queries | 91.4 / 93.1 / 93.0% | 92.5% | 69–73% | — | — |
| `--sit-kindfirst` + open queries | 94.0 / 94.5 / 96.0% | 94.8% | 73–83% | 2.6–2.7 | 1.5 |
| **`--sit-kindfirst --sit-qform14` + open queries** | **93.6 / 95.8 / 95.8%** | **95.1%** | **77–83%** | **2.5–2.8** | **1.5** |
| the same, relative price (`--sit-sq-price 1`) | 94.1 / 95.0 / 95.5% | 94.9% | 78–79% | 2.5–2.7 | 1.5 |
| the same, either price (`--sit-sq-price 2`) | 94.0 / 95.5 / 96.1% | 95.2% | 77–83% | 2.4–2.7 | 1.5 |

Open queries are `--sit-sq 64 --sit-sq-credit 1 --sit-sq-rank 1`. On seed 0, the where-object cases (`wo_cases.py`) went from 62% (previous section's cap 64) to 74% when the holder has not moved, and from 71% to 94% when the holder moved since.

- **The read budget was the block, not the query machinery.** Fixing the reader alone gains 0.6 points over the best open-query result. Open queries on top add 2.3–2.6 points more, now with fewer open (about 2.6, against 4.8) and half the upkeep (1.5 per word, against 2.7).
- **The relative price works as well as the fixed one.** With `--sit-sq-price 1`, a query past its trial closes when its net payoff per word is below the open set's average, compared by cross-multiplication. It reaches 94.9% against 95.1%, and the number open is the same. That removes the "below 0" threshold. The start balance, the credit amounts relative to upkeep, and the 2,000-word trial are still set by hand.
- **`--sit-qform14` is within noise here** (+0.3 points with open queries, 0.0 without). It was needed when the chain was forced open.
- **Kind-level open queries chosen by count do not work yet** (`--sit-sq-kinds`, which counts the chains whose two hops were both the newest of their kind as a pattern over kinds). Seed 0 gave 90.9% with the reader fixed, with 7 open on average and 17 upkeep per word. The kind patterns that count the most answers run through function words: "(any, kind .) then (any, kind place)" is the latest place written anywhere. Their first hop matches nearly every fact, so each costs about 1 read or write per word. They open first, before the clean person → place chain, which only opened at word 23,104 of 24,659. Counting only the answers a pattern would have fixed (`--sit-sq-open 1`) is data-starved on this set: 2 to 10 counts per pattern, so almost nothing opens.
