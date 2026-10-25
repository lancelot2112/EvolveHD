# 47 · The act path, stage 3: hearing itself (echo trials) (2026-10-25)

*[All experiments](README.md)* · [← 46](46-act-path.md) · [48 →](48-goal-directed-acts.md)

Stage 3 of the act path (plan: project doc `claude/act-path-plan.md`): the reader says words and hears its own output, tagged as self, starting with echo.

**Results (three seeds for the echo runs; the as-world and no-echo controls, one seed):**
- **With a replay primitive, the reader learns to echo a sentence it just heard:** on the test half, 97.9–98.7% of words and 92.9–94.7% of whole sentences on the story set, and 99.2–99.6% / 96.1–98.1% on small qa2 (three seeds).
- **Echo depends on hearing itself.** Deaf to its own words, it gets the first word right (84–96%) and almost nothing after it (0–2%).
- **The self/world separation matters.** Heard as if the world had said them, its own words wreck QA: small qa2 34% against 95–100%, story set 90.8% against 94–96%. Echo accuracy also falls (22–29% of words).
- **QA with echo trials in the stream:** small qa2 95 / 99 / 99% over three seeds (100% deaf on seed 0; 100% without echo trials). Story set 96.1 / 92.8 / 94.5% (deaf 94.1 / 94.3 / 94.1%; 96.1% without echo trials, with replay). The echo trials themselves cost the story set's where-object questions (75–80% with or without hearing itself, against 88–91% without echo trials).
- **Echo didn't work until three things were added.** Each is part of the design now:
  - self words kept out of the binding window;
  - self words dropped from the history once the act ends;
  - replay, a way to say a word it just heard.

## The setup

**The stream** (`scripts/exp47/echo_stream.py`). After every third fact sentence S:
1. "echo";
2. S as self slots: each id carries bit 31, with the teacher's word in the low bits;
3. S again, said by the world.

Story set: 24,659 → 36,094 words, 893 trials. Small qa2: 112,790 → 188,567 words, 5,741 trials. Answer positions and the freeze point are shifted.

**At a self slot** (harness and reader, binary `sit-47f`):
- The reader predicts as usual. Its top word is what it says.
- **Learning:** every prediction learner (routes, NoGo, the act gate) is credited against the teacher's word.
- **What it hears:** the said word enters the context only. It creates no bindings, bigram counts, kinds or segmentation.
- **The clause window:** self words never enter the window the binding walk reads, so no fact ever binds to a self word.
- **The efference copy:** self words stay in the history only while the act lasts. When the world speaks again, they are dropped.
- **Separate act books:** acting after a word the reader said is paid in its own table, so that what pays after its own "the" says nothing about acting after the world's "the".

**Replay** (`--sit-seq-replay`):
- A ring keeps the world's last 256 words; self words never enter it.
- At each prediction, for each of the 3 newest cue words u, the words that followed u's two newest earlier occurrences in the ring are offered as candidates. Cue position, rank and whether u was self-said are route features.
- Unlike memory's other sources, a replayed word may be a cue word, because copying is the point. Before that change, echo could never produce the sentence's first word, which was in the cue, and the first word scored 0%.

**Modes:**
- normal (hears itself as self);
- `--echo-deaf` (does not hear itself: the context stays at "echo");
- `--echo-as-world` (hears its own word as if the world had said it, so the word enters structure like any input).

The stack is experiment 42's best with the act path (`--sit-act`). Script `runs/exp47/e.sh`; scorer `scripts/exp47/score_echo.py`.

## Results

Seed 0 below. Seeds 1 and 2: story set, hearing itself, 98.7 / 97.9% of words and 94.7 / 92.9% of sentences, QA 92.8 / 94.5%; deaf, later words 0.0 / 0.0%, QA 94.3 / 94.1%. Small qa2, hearing itself, 99.6 / 99.6% of words and 98.1 / 98.1% of sentences, QA 99 / 99%.

**Echo (test half):**

| Stream, mode | Words | First word | Later words | Whole sentences |
|---|---|---|---|---|
| Story set, hears itself | **98.2%** | 99.1% | 98.0% | **93.4%** |
| Story set, deaf | 16.3% | 95.7% | 0.0% | 0.0% |
| Story set, as world | 28.7% | 26.3% | 29.2% | 15.6% |
| Small qa2, hears itself | **99.2%** | 100.0% | 99.1% | **96.1%** |
| Small qa2, deaf | 15.6% | 83.6% | 2.2% | 0.0% |
| Small qa2, as world | 21.7% | 13.1% | 23.4% | 3.2% |

**QA (test):**

| Run | Story set (where-object) | Small qa2 |
|---|---|---|
| Act path, no echo trials (exp 46) | 97.0% (91.4%) | 99% |
| Act path + replay, no echo trials | 96.1% (87.7%) | 100% |
| Echo stream, hears itself | 96.1% (80.2%) | 95% |
| Echo stream, deaf | 94.1% (76.5%) | 100% |
| Echo stream, as world | 90.8% (65.4%) | **34%** |

**Before replay (binary `sit-47d`, story set):**
- The best echo was 47.6% of words with no whole sentences.
- Memory's sources can only propose stored facts and generic continuations, so it said "julie grabbed john the office ." for "julie moved to the office .".
- **Before the clause-window rule** (`sit-47a`), self words leaked into the next sentence's bindings, and the story set fell to 87.6%.

## What this means

- **The output side now has a working motor program:** replay what was heard, word by word, steered by its own last word. That is why hearing itself is necessary: after the first word, the next word is found from the word it just said.
- **Routing self words away from fact memory is essential,** as the owner expected. When its words count as world input, small qa2 falls from 95–100% to 34%, because its mistakes become facts.
- **Echo trials still cost the story set's where-object questions** (80% against 88–91%), with or without hearing itself. The likely cause is the repeated sentences and extra stream words between facts and questions, not the self words: the deaf run loses as much. On small qa2 seeds 1 and 2 give 99%, so seed 0's 95% was noise (n = 100).

## Next

1. **Stage 4, goal-directed acts:** answers, continuations and requests through the same act path, paid only for what plain next-word prediction would miss.
2. Echo of longer spans (two sentences), and delayed echo (after intervening text).
3. Find what the echo trials cost the story set's where-object questions (the repeated sentence between facts and questions is the first suspect).
