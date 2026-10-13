# Reference code

The Python sims in [`reference/python`](../reference/python) are the reference for the Rust port. They need numpy and nothing else. Each file is self-contained apart from the imports listed.

| File | Experiment | What it holds |
|---|---|---|
| `growth_sim.py` | [06](experiments/06-growth-interfaces.md) | Topic world, matcher-pool nodes, writer-mask edges, the first growth controller (trials, prunes, a co-activity-triggered compress node) |
| `graph_sim.py` | [07](experiments/07-graph-growth.md) | 9-node graph world, type-matched candidates, conditional-MI screens, parallel shadow trials, synergy and conjoin binds, a fixed-wiring evaluator for the genetic search |
| `boot_sim.py` | [08](experiments/08-bootstrapping.md) | Staged-depth world, binds of binds, distill, per-edge vote weights |
| `curio_sim.py` | [09](experiments/09-attention-temporal.md) | Stations, delay taps, event-gated holds, attention policies (round-robin, surprise, progress, opportunity) |
| `prims_sim.py` | [10](experiments/10-full-primitives.md) | Correct (granule layer and readout), Select (Go/NoGo with interleaved trials), Broadcast alarms, cleanup, the give-up rule, the 7-station budget |
| `text_sim.py` | [11](experiments/11-real-text.md) | Real-text brain: byte pool, real and symbolic binds, delays, holds, distill clusters, stochastic multi-level writer masks with learned gains, log-loss shadow trials, duplicate merging |
| `ngram.py` | [11](experiments/11-real-text.md) | Online Witten-Bell n-gram baseline (bits per byte) |
| `word_sim.py` | [12](experiments/12-words.md) | Surprise boundaries, chunk nodes (recurrent binds), previous-word holds, held-out screens |

Import chain: `graph_sim` ← `boot_sim` ← `curio_sim` ← `prims_sim`, and `text_sim` ← `word_sim`.

## Running

```bash
pip install numpy
scripts/fetch_corpus.sh            # NLTK Gutenberg zip → corpus/bookA.bin, corpus/bookB.bin
cd reference/python
python3 -c "from text_sim import TextBrain; ..."   # see each experiment page for configs
```

A 400k-byte grown text run takes 6 to 12 minutes in Python; toy-world runs take seconds to a minute.

## The Rust crate

`src/` ports `text_sim.py` and `ngram.py`: same nodes, tick order, controller schedule and defaults. Build and run instructions are in the [repo README](../README.md). Random draws differ from numpy's, so runs agree statistically, not bit for bit.

| Python | Rust module |
|---|---|
| `Matcher.match`, `consolidate` | `pool` |
| `Edge`, `TargetIndex` | `edge` |
| `ent`, `mi`, `cmi` | `info` (plus `ent_nested`: one sort gives both entropies a screen needs) |
| `ByteReal`, `ByteSym`, `Delay`, `Hold`, `BindReal`, `BindSym`, `Distill` | `brain` (`Kind` enum) |
| `TextBrain.tick`, `check`, `screen`, `fill`, `temporal`, `synergy`, `gc`, `summary` | `brain::Brain` |
| `ngram.py` | `baseline` |
| `word_sim.py` (chunks, held-out screen) | not yet ported |

Deliberate differences:
- When a writer mask drops bits, the Python code took the lowest-numbered candidates (a side effect of `np.unique`). Rust samples them at random.
- Removed nodes free their history but keep their frozen state, as Python objects did.

### Validation (bits per byte, seed 0 unless noted)

| System | Book A, last 100k (Python → Rust) | Book B, first 50k after A | Book B, 50–100k after A | Book A whole |
|---|---|---|---|---|
| Witten-Bell order 4 | 1.958 → 1.958 | 2.586 → 2.586 | 2.385 → 2.385 | 2.153 → 2.153 |
| Hand-wired order 4, exact nodes | 2.177 → 2.170 | 2.614 → 2.618 | 2.484 → 2.488 | 2.398 → 2.392 |
| Hand-wired order 4, real pools | 2.735 → 2.731 | 2.988 → 2.987 | 2.920 → 2.914 | 2.843 → 2.840 |
| Grown, exact nodes | 2.329 → 2.307 | 2.688 → 2.647 | 2.544 → 2.513 | 2.637 → 2.628 |
| Grown, real pools, seed 0 | 2.625 → 2.611 | 2.911 → 2.927 | 2.826 → 2.842 | 2.832 → 2.817 |
| Grown, real pools, seed 1 | 2.593 → 2.609 | 2.943 → 2.914 | 2.853 → 2.822 | 2.819 → 2.816 |
| Grown, real pools, fresh on book B | – | 3.620 → 3.621 | 2.939 → 2.945 | – |
| Noisy input, recruit < 10, merge, Hebbian 2% | 3.181 → 3.128 | 3.260 → 3.850 | – | 3.267 → 3.202 |
| Same, seed 1 (Python → Rust); Rust seed 2 | 3.000 → 3.005; 3.126 | 3.180 → 3.398; 3.410 | 3.216 → 3.322; 3.323 | 3.222 → 3.154; 3.184 |

Every clean row agrees within the seed-to-seed spread (about 0.03). The grown structures match too: the same delays, byte pairs, skip pairs and exact 3-byte contexts are kept. **One known gap:** with noisy input, book A matches but book B is worse in Rust: 3.40–3.85 on its first 50k against 3.18–3.26 in Python. The same structure is kept, and recruits and duplicate merges are the same, so the difference is in learning, not growth. The likeliest cause is the mask-drop rule above; it is open.

### Speed

On one core, a full 400k-byte grown run takes about 45 s with real pools (Python: about 430 s) and 32 s with exact nodes (Python: about 390 s), roughly 10 to 12 times faster. Without growth the tick loop runs 100k bytes in about 1 s. With growth, the controller takes most of the time: on book A with exact nodes, 18 s of 32 s, of which holds and delays (temporal) are 8 s, synergy 5 s and the source screen 2 s. Set `EVOLVEHD_PROF=1` to print the breakdown. Screening cost is the next scaling problem ([open questions](open-questions.md)).
