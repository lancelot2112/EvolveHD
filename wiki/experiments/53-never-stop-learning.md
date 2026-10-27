# 53 · Never stop learning: a schedule, and an act gate that can't shut itself off (2026-10-26)

*[All experiments](README.md)* · [← 52](52-slow-learning-after-freeze.md)

**The owner's rule:** learning should be quick at first, to get seeded, then slow down, and never stop.

**Results:**
- **Never stopping works, at full rate.** Nothing is frozen: router, act gate, detector and merge judge all keep learning, and each answer is scored before the reader learns from it. Over three seeds, against the frozen stack:
  - full qa2: 88.7 / 67.7%, against 81.3 / 42.3%;
  - long BABILong: 94.4% / 7 of 8 on every seed, against 91.1% / 95.8%;
  - small qa2 97%; story set 97.1%; pilot 100 / 100, 100 / 100 and 98 / 60%.
  - The seed spread on full qa2's 4k test is 3 points (66–69%), against 41 frozen (27–68%).
- **"Fast at first, then slower" by a feature's age hurts.** The schedule (full rate for a feature's first 2^seed updates, then halving to 1 in 4) slowed the router just when the material changed in training: at the book-padded 1k stories. Pilot seed 1: training 1k 88% → 72.5% (seed 6) and 55% (seed 10); test 4k 100% → 54% and 26%. On full qa2 it was about as good as learning 1 in 4 after the freeze (0k 88.7%, 4k 62.7%, three seeds).
- **The act gate must not be able to shut itself off.** Left learning with its old rule (act where paid > charged), it closed in hard 16k–64k documents, where answers are right less than half the time: long BABILong fell to 30% / 0 of 8. A pure contrast rule (twice the rate of acts in general) kept it open but also opened it after "john", which cost the pilot (seed 1: 4k 100% → 58%).
- **The fix is hysteresis:** the gate opens by majority, as before, and once open closes only by contrast. In every run it opens on "?" alone, and frozen it behaves exactly as the old gate.

## The rules (binaries `sit-53a`–`sit-53c`)

- **No freeze:** `--sit-freeze-from` set past the end of the stream (scripts: `FREEZE=999999999`). Everything that froze at the end of training keeps learning: the route learner, the act gate, the mood counter, the contrast detector and the merge judge.
- `--sit-learn-sched seed,floor`: each route feature counts its updates. An update to a feature with n of them is applied one time in 2^min(floor, bitlen(n >> seed)).
- `--sit-act-contrast`: Go after u when acts there are paid at least twice as often as acts in general (at least 16 counted).
- `--sit-act-hyst`: open by majority (paid > charged, at least 16), close by contrast.

## The pilot, seed 1: what each change does

All runs use the act path and relation merging (`runs/exp53/a.sh pilot`). Columns: training 0k / 1k, test 0k / 4k.

| Run | Training 0k / 1k | Test 0k / 4k | Gate opens on |
|---|---|---|---|
| Frozen, old gate (base) | 93.7 / 88.0% | 100 / 100% | ? |
| Frozen, contrast gate | 92.4 / 55.0% | 82 / 58% | ? john |
| Frozen, schedule 6,2 | 92.3 / 72.5% | 97 / 54% | ? |
| Frozen, schedule 10,2 | 93.6 / 55.0% | 94 / 26% | ? |
| Not frozen, contrast gate | 92.4 / 55.0% | 99 / 58% | ? |
| Frozen, hysteresis gate | 93.7 / 88.0% | 100 / 100% | ? |
| **Not frozen, hysteresis gate, full rate** | 93.7 / 88.0% | **100 / 100%** | ? |
| Not frozen, hysteresis gate, schedule 10,2 | 93.6 / 55.0% | 96 / 40% | ? |

Seed 2, hysteresis gate: frozen 95 / 60%, not frozen 98 / 60%. Its training is weaker (1k 59.5%) in both runs, because the training phase is the same until the freeze point.

## The first schedule runs (old gate, schedule 6,2, nothing frozen)

| Test | Seed 0 | Seed 1 | Seed 2 | Mean | For comparison |
|---|---|---|---|---|---|
| Full qa2 0k / 4k | 90 / 65% | 84 / 55% | 92 / 68% | 88.7 / 62.7% | frozen 81.3 / 42.3%; 1 in 4 after the freeze 87.7 / 66.7% (exp 52) |
| Long 16k / 64k (contrast gate) | 86.7 / 100% | 83.3 / 62.5% | 76.7 / 100% | 82.2 / 87.5% | frozen 91.1 / 95.8% (exp 50) |
| Long 16k / 64k (old gate, seed 0) | 30.0% / 0 of 8 | — | — | — | the gate closed |

## The final stack (nothing frozen, full rate, hysteresis gate, merging)

Binary `sit-53c` (rebuilt on the new machine after a container restart; it reproduces experiment 42's small-qa2 and story-set results exactly). Script `runs/exp53/run3.sh`. Flags: `--sit-act --sit-act-hyst --sit-rel-merge` on the length-weighted judge stack, `FREEZE=999999999`.

| Test | Seed 0 | Seed 1 | Seed 2 | Mean | Frozen (exp 50 / 46–48) |
|---|---|---|---|---|---|
| Full qa2 0k | 90% | 87% | 89% | **88.7%** | 81.3% (60–96%) |
| Full qa2 4k | 69% | 68% | 66% | **67.7%** | 42.3% (27–68%) |
| Long 16k (n = 30) | 96.7% | 93.3% | 93.3% | **94.4%** | 91.1% (merge), 93.3% (act path) |
| Long 64k (n = 8) | 7 of 8 | 7 of 8 | 7 of 8 | 87.5% | 95.8% (merge), 87.5% (act path) |
| Small qa2 | 98% | 95% | 98% | 97.0% | 99–100% (seed 0) |
| Story set | 96.6% | 97.3% | 97.5% | 97.1% | 96.1–97.3% (seed 0) |
| Pilot 0k / 4k | 100 / 100% | 100 / 100% | 98 / 60% | — | seed 2 frozen: 95 / 60% |

- **The gate opens on "?" alone in every run,** and answering never switches off: 2,092 predictions on every long run, the same as frozen.
- **Training is identical to the frozen runs** (the freeze point is at the end of training). Every gain is in the test, from learning as it reads.

## What this means

- **"Never stop" holds; "slow down with age" doesn't, at least in this form.** The world changes in the middle of training (book-padded stories arrive), and a learner slowed by age can't follow it. Full-rate learning that never stops is as good as or better than freezing everywhere tested.
- **If learning is to slow down, it should be by surprise, not age:** fast while the router is being wrong more than usual, slow while it is doing as well as it usually does. Not built yet.
- **Anything that keeps learning needs a way back from bad stretches.** The act gate shows it: a rule that compares acting to a fixed 50% shuts itself off in hard material, and one that compares to the average opens too easily. Hysteresis does both jobs.

## Next

1. A learning rate set by surprise (recent error against long-run error), not by age.
2. The same never-frozen stack on the reports and echo streams.
