# 41 · Forum round six, first tests: a halving sketch, split boundary roles, boundaries judged by use, a per-entity cap, a contrast detector (2026-10-24)

*[All experiments](README.md)* · [← 40](40-standard-set-and-qa2.md)

Forum round six reviewed the equations against the code. It proposed an ordered next iteration, E0 to E6 (project doc `claude/forum-round-six.md`). This experiment runs the first three steps.

**Results** (all single runs at seed 0):
- **E1, the halving sketch, partly holds.**
  - On the full qa2 stream with chains, long-term heads at word 520k fall from 147,847 to 25,698, and the run takes 1,449 s instead of 5,724 s. Accuracy is unchanged.
  - Candidates per word still grow, to 3,124 by the end (16,945 before). A few hub entities collect hundreds of heads each (the largest list reaches 970). Which words they are was not logged in this run.
  - It costs nothing on the story set or the BABILong pilot.
- **E0, splitting the boundary set's jobs, is falsified.** Small qa2 rises from 33% to 74% without the hand ban. But the story set falls to 53.5% and the pilot to 2 / 4%: the veto also removes "?" and "." as binding stops. Part of that collapse was a fault in this experiment's code, found in the second round: the cue walk was tied to the binding stops. With it fixed, E0 reaches 78.7% and 79 / 96%, still under the bars.
- **E2, boundaries judged by use, fails with both judges tried.**
  - The forum's rule is biased toward cutting: "the" scored span 1, cut 543.
  - A symmetric rank judge is biased toward crossing: it drops every stop on small qa2 (13%).
  - The 1-in-8 exploration crossings themselves cost accuracy, because they are written into long-term memory.

- **Second round** (the qa2 runs ban "the" by hand so the chains have facts; the long BABILong runs do not):
  - On full qa2 the hubs are function words ("of", "and", "the", "in", "that"), growing to about 1,000 heads; the people stop near 90.
  - A learned hop-2 fan-out saves time but loses 15–27 points.
  - **A derived per-entity cap (a fair share of the budget) works:**
    - full qa2 4k goes from 45% to 90%, with candidates down 2.7× and the run at 946 s instead of 1,700 s;
    - long BABILong 64k goes from 62.5% to 8 of 8.
  - The halving sketch kills 64k (0 of 8), probably because sparse facts do not recur within its window.
  - A normalized use judge also fails, after a cue-walk fault in this experiment's code was fixed.

## Stacks

- **qa2 and small qa2:** the BABILong stack (experiment 40), plus `--sit-chain`. Small qa2 (`bl_qa2s`: 1,000 plain training samples, 100 test) also has `--sit-lt-budget 131072`. The test is frozen from word 102,303 and read in full.
- **Story set and BABILong pilot:** as in experiment 40 (`runs/exp36/rq30.sh`, `runs/exp38/bl_split.sh bl_pilot`).

Binaries `sit-41a` to `sit-41f` (each adds a flag; earlier flags behave the same). Scripts: `runs/exp41/e1.sh`, `e0.sh`, `e2.sh`, `e2r.sh`. Both cores were shared between two runs at a time.

## E1: the halving sketch (`--sit-sketch-decay`)

Every 2¹⁶ increments, each counter of the path sketch halves. A path then has to recur recently to pass the recurrence gate. The run also logs heads per entity (`HEADS_PER_ENTITY`). The five largest lists were added to that line later (from `sit-41c`), so only the small-qa2 runs below show them.

Full qa2 stream (525,866 words; frozen from word 220,618), chains, no budget:

| | Before (`R_qa2_chain`, exp 40) | Halving sketch | Halving sketch + budget |
|---|---|---|---|
| Run time | 5,724 s | 1,449 s | 1,607 s |
| Heads at word 520k | 147,847 | 25,698 | 40,033 |
| Candidates per word, mean | 4,093 | 1,448 | 1,676 |
| Candidates per word at 520k | 16,945 | 3,124 | 3,763 |
| Time per 20k words at 520k | 732 s | 108 s | 121 s |
| Test 0k / 4k | 16 / 18% | 17 / 16% | 27 / 25% |

The forum's predictions:

| Prediction | Result |
|---|---|
| Heads at 520k ≤ 30k | **Held** (25,698) |
| Accuracy within ±2 points | **Held** |
| Candidates per word ≤ 150 | **Failed** (3,124) |
| Time ≤ 30 s per 20k words | **Failed** (108 s) |

The budgeted run's 27 / 25% (against 16 / 18% without the budget) is one seed.

Heads per entity, halving sketch, no budget:

| Word | Entities | Median | 90th percentile | Largest |
|---|---|---|---|---|
| 140k | 550 | 1 | 3 | 85 |
| 300k | 3,099 | 2 | 3 | 439 |
| 520k | 6,106 | 1 | 3 | 970 |

**What this means:**
- **Most entities stay at 1–3 heads.** The total grows mostly because book text keeps introducing new entities.
- **The largest lists grow linearly.** Which entities own them on the full stream was not logged. On small qa2 they depend on the stops:
    - with stops "the ." (E2, forum judge), the largest are apple, football, milk, sandra and kitchen, at 129–148 heads;
    - with no stops at all (E2, rank judge), they are the four people (921–974 heads) and "went".
    - A head is one per (entity, exact path). Exact paths through up to 8 words are close to unique, so an entity that appears in many different contexts keeps collecting new heads.
- **The chain walks those lists, so its cost tracks the hubs.** The fix belongs in hop 2's fan-out, or in what makes a path count as the same relation, not in the sketch.

**The halving sketch costs nothing elsewhere:** story set 98.5% (as before), pilot 100 / 100%.

## E0: split the boundary set's jobs (`--sit-bind-indep`)

L, the learned boundary set, still decides commits, drift and the detector. Only L's independent words stop the binding and cue walks. The question drop uses `is_q`, and floor samples update the mood counter after the freeze. Run with `--sit-mood-floor`.

| Test | Base | E0 | Binding stops (E0) |
|---|---|---|---|
| Small qa2, chain + budget | 33% | **74%** | "." |
| + `--sit-bind-drift` (drift only at stops; run by hand, no script) | | 77% | "." |
| Story set | 98.5% | **53.5%** | "." ("?" vetoed) |
| BABILong pilot | 100 / 100% | **2 / 4%** | "of" (". ? , a and" vetoed) |

- **The forum set a falsification bar of story set < 90%, and E0 lands at 53.5%.**
- **Binding across "?" and "." does real damage.** So the veto is the wrong judge even for binding, not only in its wiring.
- **Some of the gap to the hand ban's 100% remains unexplained.** Drift at "the" is not the cause (77% with `--sit-bind-drift`).

## E2: boundaries judged by use (`--sit-bnd-use`)

L proposes. On a hash coin (one occurrence in 8), the walk does the opposite of the current stop decision. Every exact path that crosses a proposed word records the first one it crosses, u. At each prediction the bigram did not already make, the routes' path keys decide.

**The forum's judge:**
- If the top route crosses u, was right, and no route that does not cross u held the word: span(u) += 1.
- If the top route crosses u, was wrong, and a non-crossing route held the word: cut(u) += 1.
- u stops the walk while span ≤ cut. Both halve when span + cut reaches 1,024.

**The rank judge (`--sit-bnd-use-rank`):**
- If the best-ranked holder of the right word crosses u: span(u) += 1.
- If a route crossing some other proposed word u outranks it: cut(u) += 1.

| Test | Forum judge | Rank judge | Halving sketch only |
|---|---|---|---|
| Small qa2 | 33% ("the" span 1 / cut 543; stops "the .") | 13% (the 647 / 273, . 482 / 167; no stops) | — |
| Story set | 93.5% (stops ". ?") | 90.1% (stops ". ?") | 98.5% |
| BABILong pilot | 52 / 44% (all stops kept) | 100 / 72% ("?" stops no more: 498 / 372) | 100 / 100% |

**Why it fails:**
- **The forum's judge is biased toward cutting.** Span and cut test the same condition, "a route that does not cross u holds the word", once negated. With hundreds of routes proposing, that condition almost always holds, so span almost never counts.
- **The rank judge is biased the other way.** Crossing paths are more numerous and more varied, so the best holder crosses u often, whatever u is.
- **Exploration is not free.** The 1-in-8 crossing bindings enter long-term heads. That costs 5 points on the story set and up to 56 on the pilot even when every stop is kept. The forum meant them to stay in working memory, but working-memory routes carry classes, not exact paths, so they cannot be attributed.

**The judge needs a normalized comparison.** It should compare credit per binding written, crossing against within, as the superoptimizer and computational-neuroscientist seats wrote it: across · n_within ≤ within · n_across. Exploration bindings should stay out of long-term memory.

## Second round: the hubs, the fan-out, a derived cap, a normalized judge

The owner asked for the next steps. Binaries `sit-41g` to `sit-41k`; scripts `runs/exp41/e4.sh`, `e5.sh`, `e5c.sh`, `e5b.sh`, `e2n.sh`, `e0e2n_cue.sh`.

**The chains need facts to follow, so the qa2 runs here (full and small) ban "the" as a boundary by hand** (`SIT_BND_DENY=2`, the experiment 40 diagnostic, not a learner). Their stack is chains + `--sit-lt-budget 131072`. The long BABILong runs have no ban and no chains: only the budget plus the flag named, through `bl_split.sh`. The normalized-judge runs have no ban either.

### The hubs are function words

On the full qa2 stream (with the ban, chains, budget and halving sketch), the five largest head lists at words 120k, 240k and 520k:

| Word 120k | Word 240k | Word 520k |
|---|---|---|
| john 84, sandra 83, mary 82, daniel 81, football 54 | of 315, and 303, "," 232, in 226, that 172 | of 1,094, and 1,006, the 991, in 784, that 679 |

- **The people stop at about 85–92 heads.**
- **The function words keep growing.** Having heads at all means the content gate passed them, so each is in a kind of 3 or more members. They meet so many different words that their exact paths never repeat.

### A learned hop-2 fan-out fails (`--sit-chain-learn`)

A table from hop 1's relation (its class and direction) to hop-2 classes:
- Counts are Misra-Gries, 4 per row.
- It is credited when a chain is the best-ranked route holding the next word.
- Hop 2 then offers only the row's top 2. A 1-in-16 coin, or a row walked fewer than 32 times, offers everything.

`--sit-chain-learn-kind` uses value kinds instead of classes.

| Run | Test 0k / 4k | Candidates per word | Time |
|---|---|---|---|
| Small qa2, full walk | 97% | 389 | 66 s |
| Small qa2, by class | 82% | 293 | 57 s |
| Small qa2, by value kind (run by hand) | 70% | 272 | 54 s |
| Full qa2, full walk | 98 / 45% | 1,937 | 1,700 s |
| Full qa2, by class | 75 / 23% | 576 | 921 s |

**It saves time and loses accuracy.**
- qa2's answer is the newest place across all of a person's place paths, which spread over several classes.
- The credit comes from every prediction, not only from answers. On small qa2 no question boundary is learned, so there is nothing better to credit from.

### A derived per-entity cap works (`--sit-entity-fair`)

The cap is a fair share of the budget, N >> bl(entities with heads), and at least 4. When a new head arrives on a full list, it replaces that entity's oldest head.
- The cap falls as entities appear: 4,096 with 26 entities, 16 with 6,000.
- A list is not trimmed when the cap falls. It stops growing, because each new head then replaces an old one.

**Full qa2 stream:**

| Run | Test 0k / 4k | Candidates per word (mean / at 520k) | Largest list | Time |
|---|---|---|---|---|
| Halving sketch | 98 / 45% | 1,937 / 4,222 | 1,094 ("of") | 1,700 s |
| Halving sketch + cap | **97 / 90%** | 709 / 926 | 128 ("and") | 946 s |
| Cap, no halving sketch | 97 / 65% | 758 / 1,042 | 128 | 928 s |

**Long BABILong qa1** (budget, answer-only; 16k n = 30, 64k n = 8):

| Run | 16k | 64k | Time |
|---|---|---|---|
| Budget (exp 38, `R_bl_long_bud17` and `bud17b`) | 83.3 / 80.0% | 62.5 / 62.5% | 1,698 / 2,120 s (older binary, other core sharing) |
| + halving sketch + cap | 90.0% | **0.0%** | 810 s |
| + halving sketch | 90.0% | **0.0%** | 826 s |
| + cap | **93.3%** | **100.0%** | 820 s |

**What this means:**
- **The cap is the fix for both chain cost and long-document memory:**
  - on full qa2, 4k goes from 45% to 90% with the halving sketch, and to 65% without it;
  - candidates fall by about 2.7× and the largest list stays flat;
  - long BABILong at 64k goes from 62.5% to 8 of 8.
  - The forum's bar for the budget was 64k ≥ 7/8. It is met on one seed, with n = 8.
- **The halving sketch conflicts with long documents.**
  - In a 64k document a fact's path recurs too rarely to reach 4 within the halving window (2¹⁶ sketch increments, mostly book text), so facts stop entering the long-term store.
  - On full qa2 the same sketch keeps book one-offs out and helps 4k (90% against 65%).
  - A rate set by the sketch's increments is the wrong clock for a sparse fact.

### The use judge, normalized (`--sit-bnd-use-norm`), and the cue walk

The normalized judge (runs with `--sit-sketch-decay`, no ban):
- span(u) counts predictions whose best-ranked holder of the right word crosses u;
- cut(u) counts bindings written across u;
- u stops the walk while span · bindings ≤ hits · cut, so crossing bindings must earn no more per binding than bindings do overall.

| Test | Normalized judge | Stops kept |
|---|---|---|
| Small qa2 | 19% | "the" ("." crossed: 2,025 / 57,136 against 24,564 / 1,447,524 overall) |
| Story set | 17.6% | none |
| Pilot | 0 / 0% | ". , a and of" ("?" crossed) |

**Crossing "." earns more per binding than the average binding,** so the judge removes it. Those runs exposed a fault in this experiment's code: E0 and the use judges tied the question's cue walk to the binding stops. Once "?" stopped ending a binding, the cue ran into the previous sentence and no question boundary was learned. The cue walk now stops at L (`sit-41k`).

Rerun with the cue on L:

| Test | E0 (split roles) | Normalized judge |
|---|---|---|
| Small qa2 | 78% | 27% (no stops) |
| Story set | 78.7% | 80.4% (stop "?"; where-object 28.4%) |
| Pilot | 79 / 96% | 98 / 100% (stops ". , a and of") |

- **The cue fix explains part of the earlier collapses** (story set 53.5% → 78.7%; pilot 2 / 4% → 79 / 96%).
- **No judge passes yet.** Judges that count hits favour crossing even where it costs accuracy: the story set's where-object falls to 28% when "." is crossed.
- **What crossing costs is not visible in the routes that hit.** It is the wrong answers that crossing bindings cause elsewhere, and the clause structure they break.

## Third round: the contrast detector, and the fan-out credited only at answers

The owner asked whether the learned fan-out keeps improving with training. It does not. The fair test credits it only at answers, which needs a question detector, so this round builds E3 first. Binaries `sit-41l` to `sit-41n`; script `runs/exp41/e3b.sh`. The first version, `e3.sh` (`sit-41l`), was stopped. As with the second round, the qa2 runs ban "the" by hand and use chains, the budget and the halving sketch; the story set and the pilot do not.

### The contrast detector (`--sit-q-contrast`)

For every word type u, it counts the predictions after u that the bigram did not already make, and how often the store's top proposal was right among them. It compares that rate with the same rate over every word.
- Q(u) holds when u has at least 16 such predictions and its rate is at least twice the overall rate.
- No boundary is required.
- It keeps counting after the freeze. The overall rate stops at the freeze.

**The first version (`sit-41l`) also committed and dropped clauses at Q words.** It learned Q = "milk went apple football", dropped 9,724 fact clauses, and scored 26% on small qa2. It was stopped. In the second version (`sit-41m`), a clause is dropped only at a learned boundary that is also a Q word.

| Test | Before | Contrast detector | Question words learned |
|---|---|---|---|
| Small qa2 | 97% (old detector: none learned) | **99%** | milk went apple ? football |
| Full qa2 + per-entity cap, 0k / 4k | 97 / 90% | 96 / 83% | milk went apple ? football |
| Story set | 98.5% | 97.3% | ? apple football milk book key pen hat cup |
| BABILong pilot, 0k / 4k | 100 / 100% | 88 / 96% | sandra went daniel mary ? john object |

**What this means:**
- **"?" becomes a question word everywhere,** including qa2, where it is not a learned boundary.
- **Content words qualify too,** because memory predicts the word after a person or an object well.
- On small qa2 that helps, through credit at answers. Elsewhere it costs 1–12 points:
  - in answer-only reading, the pilot makes 2,145 predictions instead of 547;
  - the extra Q words take the routes the real questions use.
- **The twice-overall bar is a constant, and it lets in too much.**

### The fan-out, credited only at answers (`--sit-chain-learn-q`)

Accuracy per block of 100 training questions, then the test:

| Run | Blocks 1–10 | Test |
|---|---|---|
| Small qa2, full walk + detector | 48 84 93 94 96 99 100 99 98 98 | 99% |
| Small qa2, fan-out credited at answers | 53 87 86 81 77 82 78 76 67 76 | 72% |
| Small qa2, the same following all 4 of a row's entries (`SIT_CL_TOP=4`, run by hand) | 53 87 86 81 80 85 77 82 74 76 | 90% |
| Full qa2, full walk + detector + cap | 61 89 96 97 95 97 100 99 99 99 | 96 / 83% |
| Full qa2, fan-out credited at answers | 62 90 87 84 82 77 81 75 67 75 | 76 / 60% |

**The decline is the same whatever the credit:**
- The fan-out matches or beats the full walk for the first one or two blocks, during its warm-up.
- After that it falls. Crediting only at answers does not change the shape, and neither does following all 4 of a row's entries.
- A row holds 4 classes. When more classes answer than a row can hold, the Misra-Gries wear-down churns the row.
- The saving is small at best: 349 candidates per word against 436 on small qa2, and 341 against 723 on full qa2, for 20–24 points of accuracy.

**The derived per-entity cap is the better tool for chain cost.**

## Fourth round: boundaries credited from the level above (`--sit-bnd-use-q`)

The owner's principle: predictions stay local, and credit comes from a higher-level prediction, flowing down to the lower decisions it depended on.
- Here the higher-level prediction is the answer after a question word (contrast detector).
- Only answers vote. A right answer gives span += 1 to every proposed word its top route crossed; a wrong answer gives cut += 1.
- No hand ban. Binary `sit-41o`, script `runs/exp41/e2q.sh`, with the halving sketch.

| Test | Contrast detector alone (`qc2`) | + boundaries from answers | Stops | Span / cut |
|---|---|---|---|---|
| Small qa2 | 99% (with "the" banned) | 22% | the . | the 0 / 0, . 0 / 4 |
| Story set | 97.3% | 92.4% (where-object 67.9%) | . | . 2 / 7, ? 65 / 5 |
| Pilot, 0k / 4k | 88 / 96% | 76 / 88% | . ? , a and of | . 106 / 473, ? 0 / 4 |

**Two failure modes:**
- **Bootstrapping.** Without the ban, the facts are broken, so the answers after "?" are wrong. "?" is then never learned as a question word (Q = milk went apple football). With no answers at "?", the judge receives no votes about "the" (0 / 0). The higher-level signal needs the lower level to be roughly right first.
- **Shortcuts are credited.** On the story set, answers whose routes cross "?" are right 65 times out of 70.
  - The binding from a question's entity to the previous answer often still holds, because the person has not moved.
  - So "?" stops being a stop, and where-object falls from 86% to 68%.
  - Credit from the outcome rewards a route that is right for the wrong reason.

## Next

1. **Adopt the per-entity cap.** Choose a clock for recurrence that suits sparse facts, for example one that counts sentences or the entity's own sightings rather than all sketch increments, so qa2's 4k gain and qa1's 64k survive together. Rerun across three seeds.
2. **Top-down credit needs a signal that exists before the boundaries are right, and that does not reward shortcuts.** For example, use the unit level's own prediction of the next clause, which is available on any text, rather than question answers. This is the owner's "high-level prediction credits the lower level", one level up from words.
3. **The detector needs a sharper contrast,** one that keeps "?" and leaves out content words. For example, require the rate after u to beat the rate after u's own kind, not after every word.
4. **The boundary judge needs the cost of crossing,** not only its hits: wrong top answers that a crossing binding caused, counted per binding written. Until then "the" is banned by hand in the chain runs.
