# 10 · All seven primitives as moves, and a give-up rule (2026-10-13)

*[All experiments](README.md)* · [← 09](09-attention-temporal.md) · [11 →](11-real-text.md)

**Concepts:** [Growth operators](../concepts/growth-operators.md) · [Go/NoGo selection](../concepts/go-nogo-selection.md) · [Broadcast](../concepts/broadcast.md) · [Attention](../concepts/attention.md) · [Shadow trials](../concepts/shadow-trials.md)

Questions:
1. Do Correct, Select and Broadcast work as grammar moves, alongside Expand, Associate, Compress and Gate?
2. Can the attention loop give up on hopeless stations without abandoning slow-but-learnable ones?

All tests use the curio_sim station machinery: screened candidates, parallel shadow trials, synergy/conjoin binds, distill, delay taps and event-gated holds, with per-edge vote weights. Unused derived nodes are now garbage-collected every 10k ticks, so the structure budget is recycled.

## How the three new primitives work

| Primitive | Grown when | What it is | How it's tested |
|---|---|---|---|
| **Correct** (cerebellum) | A target has no kept edges and the screens have found nothing for 3 checks (15k ticks) | A shared granule layer: one cell per joint value of every k-subset of the station's sources (fixed random conjunctions), plus one error-driven readout per unexplained target. Readouts move only on their own wrong guesses, like Purkinje cells taught by climbing fibres. | Long shadow trial, evaluated every 2k ticks; kept after two passing windows in a row (many looks, so stricter); rejected after 60k |
| **Select** (basal ganglia) | The station has an action output | Go/NoGo tables per context cell with a critic (δ = reward − V; NoGo plus Go-LTD on dips). Context inputs are proposed by a screen for which source carries reward information *jointly with the action*. | Interleaved trial: alternate ticks with and without the new input, compare reward. Actions can't be shadow-tested, because only the taken action's reward is seen. |
| **Broadcast** (LC-like) | A target's fast accuracy average drops 0.12 below its slow average | A phasic alarm: early re-check of the target's kept edges, cleared backoffs, stale trials dropped, screens restricted to post-alarm data, half burn-in, and 3× learning rate for 4k ticks | Standalone comparison on a world-change scenario |

## 1. Standalone tests (6 seeds unless stated)

**Correct.**

| world | config | target accuracy, last 10k | notes |
|---|---|---|---|
| 3-input, no stepping stone: T = LS2[LS1[A, B], C] | no cerebellum | 0.125 (chance) | nothing grown |
| same | cerebellum, k = 3 | **0.722 ±0.004** (ceiling 0.869) | granules at 15k (20 subsets × 512 cells), kept at 23k; 0.39 → 0.61 → 0.67 → 0.72 per 10k; no false keeps |
| 4-input (boot_sim nostone T8), 160k ticks, 4 seeds | cerebellum, k = 4 | 0.274 ±0.002, still rising | 120 subsets × 4096 = 491,520 cells; kept at 25–37k; 1 false keep on a noise node in 4 seeds |

**Select.** ActionWorld: reward is 1 when the action matches a hidden policy of the previous context C (10% flipped). Ceiling is about 0.86 with 5% exploration.

| config | reward, last 10k |
|---|---|
| bias only (no context) | 0.233 ±0.029 |
| **grown context** | **0.832 ±0.011** |

With grown context, C was trialled from 10k and kept at 18k in 6/6 seeds. One junk context (a delayed copy of its own action or reward) was trialled and rejected in each seed.

**Broadcast.** SwapWorld: SL and D1 swap channels at tick 40k.

| config | ticks after swap until T1 ← D1-carrying source is kept | T1 accuracy 40–50k / 50–60k |
|---|---|---|
| no broadcast | 12k, 12k, 18k, 18k, 18k, 24k (median 18k) | 0.519 / 0.582 |
| broadcast v1 (re-check, backoffs, burn-in, learning rate) | 10k, 16k, 16k, 16k, 16k, 22k (median 16k) | 0.540 / 0.583 |
| **broadcast v2 (+ drop stale trials, screens see only post-alarm data)** | **5k, 5k, 5k, 11k, 17k, 17k (median 8k)** | **0.574 / 0.629** |

- v1 barely helped: the alarm fired within ~200 ticks, but the screens still ranked candidates on 4k ticks of pre-change data, so the trial slot stayed busy with stale candidates.
- Resetting what the screens look at is what made the alarm useful.
- T4's rebind (it needs a new D1×T4 bind) still mostly fails within 40k ticks: 2/6 with v1, 0/6 otherwise. This is open.

## 2. Full budget run with a give-up rule

Seven stations share 200 blocks of 1000 ticks:

| Station | Contents |
|---|---|
| easy | two single-edge puzzles |
| staged | the T2 → T6 → T8 bootstrapping chain |
| temporal | delay and memory puzzles |
| action | Select needed |
| hard | 3-input, needs Correct |
| noise | pure noise |
| bloomer | pure noise until global block 100, then a single-edge puzzle |

**Give-up rule.** After warm-up, a station with no sign of life for 6 consecutive visits goes dormant for 10 blocks, then 20, 40, up to 80. Signs of life are: significant progress, structure grown, or screen opportunity. One quiet recheck sends it back to sleep. The **fallback-aware** variant also treats a pending brute-force move as alive: a dry streak building toward a granule layer, or a cerebellar trial younger than 20k ticks.

Score is the mean normalised accuracy on the 10 learnable targets after one final block (0 = chance, 1 = ceiling); for the action station it is reward. 4 seeds.

| policy | score | targets > 0.8 (of 10) | T8 | hard T | action R | bloomer | blocks: easy / staged / temporal / action / hard / noise / bloomer |
|---|---|---|---|---|---|---|---|
| round-robin | 0.930 ±0.046 | 8.8 | 0.77 | **0.59** | 0.97 | 1.00 | 29 / 29 / 29 / 29 / 28 / 28 / 28 |
| opportunity | **0.953 ±0.010** | 9.0 | 1.01 | 0.52 | 0.94 | 1.00 | 24 / 39 / 24 / 43 / 26 / 22 / 22 |
| opportunity + give-up | 0.910 ±0.034 | 9.0 | 1.00 | 0.15 | 0.94 | 0.99 | 28 / 40 / 37 / 43 / 16 / **10** / 26 |
| opportunity + give-up, fallback-aware | **0.953 ±0.010** | 9.0 | 1.01 | 0.52 | 0.94 | 1.00 | identical to opportunity: never went dormant |

## Findings

1. **All seven primitives now exist as moves, and each does what only it can do:**
   - **Correct** solves puzzles with no stepping stones, which no screen can see.
   - **Select** learns to act from reward.
   - **Broadcast** speeds recovery after the world changes.
2. **Correct is the brute-force fallback, and its cost grows fast with order.** A 3-input puzzle needs about 10k granule cells and is mostly solved within 40k ticks of starting. A 4-input one needs about 490k cells and is still only a third of the way after 145k ticks. That matches the cerebellum: a huge expansion layer, used for mappings with no compositional shortcut. Where stepping stones exist, Expand → Associate → Compress is orders of magnitude cheaper.
3. **Action edges need a different kind of trial.** Shadow trials are free for predictions but impossible for actions, because the counterfactual reward is never seen. Interleaving works, at the price of acting worse on half the trial ticks.
4. **An alarm is only useful if it also resets what the system pays attention to.** Faster learning and early re-checks did little; discarding pre-change evidence from the screens halved rewiring time.
5. **Give-up is a real trade-off: hopeless and slow can't be told apart cheaply.**
   - Plain give-up cut the noise station from 22 blocks to 10, but also abandoned the hard station before its granule layer existed (hard T 0.52 → 0.15), so it lost overall.
   - Making give-up wait for the fallback removed the damage, but within 200 blocks it then never fired. Ruling out brute force costs about 35k station ticks: a 15k dry streak plus 20k of probation.
   - Opportunity-driven attention already pushes the noise station to the end of the run (most of its 22 blocks came after block 135, once the rest was done), so a give-up rule matters only for much longer horizons or many hopeless sources.
6. **Round-robin still beats the adaptive policies on the hard station** (0.59 vs 0.52). Its slow cerebellar climb gives too weak a progress signal per visit. A progress estimate taken from the readout's own learning curve, rather than per-visit accuracy, would credit it better.

## Changes to the spec this supports

1. **Correct:** grow one shared granule layer per area when the screens stay dry, with a readout per unexplained target. Accept with two consecutive passing windows. Subset size k is a grammar parameter, worth tuning by population search.
2. **Select:** propose context inputs by reward–action synergy, and test them with interleaved trials.
3. **Broadcast:** a phasic surprise alarm restarts the screens' evidence window and drops stale trials, as well as boosting plasticity.
4. **Attention:** keep opportunity-driven Select. Give up only after the brute-force fallback has had its probation; give-up is a long-horizon tool.
5. **Housekeeping:** garbage-collect derived nodes nothing uses (never a part of a live node), so the structure budget recycles.

## Follow-up: the two loose ends, fixed

**T4 never rebinding** turned out to be a grammar bug: priority hogging, not a Broadcast problem. Once delay taps and conjoins were added, every newly built node got trial priority *permanently*. T4's single trial slot was monopolised by delay taps (SL@2, SL@4…). Those taps carry the latent's information, but only in conjunction with T4's own state, so additive edges from them always failed. Meanwhile the correct bind (SL*T4) waited and was eventually garbage-collected without ever being tried. T4 had in fact never been solved properly even before the swap (0.38).

Fixes:
- Priority buys one trial, not a permanent slot. Synergy-built binds get one-shot priority for the target they were built for.
- Two concurrent shadow trials per target, six per station.
- The temporal screen scores what an *additive* edge could use (unconditional MI), leaving conjunctions to synergy and conjoin.
- Cleanup never removes a node that hasn't been trialled yet.
- Broadcast v3: a repeat alarm keeps trials started since an earlier alarm.

Swap world, 6 seeds:

| config | T1 rewire after swap | T4 rebind after swap | T4 accuracy 30–40k / 40–50k / 50–60k | T1 40–50k |
|---|---|---|---|---|
| no broadcast | 12k (6/6) | 12k (6/6) | 0.765 / 0.452 / 0.721 | 0.513 |
| **broadcast v3** | **5k (5/6), 11k (1/6)** | **11k (6/6)** | 0.774 / 0.509 / **0.741** | **0.589** |

T4 now reaches 0.77 before the swap, above the earlier hand wiring's 0.71.

Regression checks with the fixes (6 seeds):
- hard T 0.716
- action reward 0.842
- temporal Td/Q 0.868 / 0.385
- staged T2/T6/T8 all 0.869, with T8 solved at 26k instead of ~32k

**Hard station starved by attention** turned out to be the action station wasting budget. Select kept trialling junk context inputs (delayed copies of its own action or reward), and each running trial counted as opportunity. Fixes: a Select trial counts as opportunity only if its screen saw more than 0.05 bits, and rejected contexts back off with doubling.

Budget rerun, 7 stations, 200 blocks, 4 seeds, all fixes:

| policy | score | hard T | blocks: easy / staged / temporal / action / hard / noise / bloomer |
|---|---|---|---|
| round-robin | 0.932 ±0.047 | 0.57 | 29 / 29 / 29 / 29 / 28 / 28 / 28 |
| **opportunity** | **0.966 ±0.004** | **0.70** | 25 / 36 / 24 / 29 / **40** / 24 / 23 |
| opportunity + slope-based progress | 0.950 ±0.029 | 0.51 | 24 / 44 / 26 / 30 / 32 / 22 / 22 |
| + fallback-aware give-up | 0.950 ±0.029 | 0.51 | identical: never went dormant |

- With the Select waste removed, plain opportunity-driven attention gives the slow cerebellar station the most time of any station and now beats round-robin there (0.70 vs 0.57).
- A slope-based progress signal didn't help; it over-credited the staged chain's climb. It is not needed.
- These budget runs used broadcast v2. The v3 change only affects repeat alarms.

## Code

[`reference/python/prims_sim.py`](../../reference/python/prims_sim.py); see [reference code](../reference-code.md) for how to run it.
