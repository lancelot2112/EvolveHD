# 43 · A cheap test of multi-scale bindings, and forum round eight (2026-10-24)

*[All experiments](README.md)* · [← 42](42-credit-from-the-level-above.md) · [44 →](44-builders-from-nothing.md)

**The owner's question:** "We should think of language as a fractal, nested, recursive structure. Build a fractal memory that can see it at various levels and make connections itself, without worrying about whether it lines up exactly on periods or clauses. Maybe the language-specific approach is what is hurting us."

**Results:**
- **Bindings by text distance, with no binding stops, lose badly:** small qa2 14%, story set 84.4% (where-object 42%), full qa2 30 / 24%.
- **The test was only half a test.** Only the bindings changed. Commits, drift, the cue and the question detector still used the learned boundary set L. On small qa2, "the" became a question word, so every clause ending at "the" was dropped as a question. Experiment 44 replaces every job.
- **Forum round eight** (project doc `claude/forum-round-eight.md`) answered the question and set the next tests (experiment 44).

## The rule (`--sit-scales`)

- No binding stops. Every word binds to every word up to 8 back by exact path.
- Content words (a kind of 3 or more) also bind to content words 9–32 back, in log-spaced distance bands (9–16, 17–32). The relation is the band plus the value's kind.
- Everything else is unchanged: commits, drift, the cue walk and the question detector use L. There is no hand ban and no boundary judge.

Binary `sit-43a`, script `runs/exp43/sc1.sh`. Same stacks as experiment 42; small qa2 and full qa2 with chains, `--sit-lt-budget 131072` and `--sit-entity-fair`. One seed.

| Test | Multi-scale, no stops | Best so far (k90 L + clause judge, exp 42) |
|---|---|---|
| Small qa2 | 14% | 97–100% |
| Story set | 84.4% (where-object 42%) | 98.5% |
| BABILong pilot 0k / 4k | 97 / 92% | 100 / 100% |
| Full qa2 0k / 4k | 30 / 24% | 87 / 34% (hand ban 97 / 65%) |

**Cost.** Small qa2 ran 380 s against about 66 s, with 1,019 candidates per predicted word against 389. On full qa2 (2,997 s), the long-term heads reach the budget (131,071), and the largest entities hold about 1,300–1,400 heads each (about 128 with stops and the cap).

## What the panel said (forum round eight, ten seats)

Every seat that measured it offline found that far bindings by text distance lose:
- the superoptimizer's key study (a stand-in reader that scores below the real one but ranks rules the same): no stops, 8 back exact, scored 32% on small qa2 and 64.9% on the story set, against 85% and 84.4% with L;
- the TEM seat's toy reader: every far binding by text distance lost, even with the level visible.

The paths of true but irrelevant cross-clause bindings recur as often as the facts' paths, so they crowd the ranking. Exact keys must stay; the unit has to come from somewhere, and it should be built from nothing.

The seats proposed six builders and a common account of goals: each level predicts its own next item and is paid only for what the level below missed, against a shadow on the same words; the cap is the upkeep. Experiment 44 tests two of the builders (the recurrent seat's surprise chunks and the TEM seat's collision ledger) and one replacement for the question drop (the write gate).
