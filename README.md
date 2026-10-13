# EvolveHD

Evolves networks that learn using high-level primitive composition.

EvolveHD is a learning system built from high-dimensional bit vectors. Matcher cells read them, fire on a popcount threshold, and adjust slowly with Hebbian and anti-Hebbian bit swaps. On top of that engine sits a **growth grammar**. A network starts as a few input pools and grows the rest inline, during one lifetime, from seven primitives: Expand, Associate, Compress, Gate, Select, Correct, Broadcast. A structure is added only when a screen measures a pressure for it, and kept only while a shadow trial shows it pays.

The goal is a robust general learner that uses the strengths of a computer, not a brain simulation.

## Where to read

- **[wiki/](wiki/README.md)**: the design, every experiment with its numbers, one page per concept, lessons and open questions.
- **[reference/python/](reference/python)**: the Python simulations behind the experiments. They need numpy and nothing else.
- **[src/](src)**: the Rust crate, a port of the text experiment (`reference/python/text_sim.py`).

## Quick start

```bash
scripts/fetch_corpus.sh                      # NLTK Gutenberg → corpus/bookA.bin (Persuasion), corpus/bookB.bin (Thursday)
cargo build --release
cargo test --release

# one grown run on book A then book B
./target/release/evolvehd text --data corpus/bookA.bin,corpus/bookB.bin --out run.json
#   --symbolic        exact (unlimited) nodes instead of real matcher pools
#   --oracle 4        hand-wired order-4 contexts, no growth
#   --noise 0.25,4    drop 25% of input bits and add 4 spurious ones
#   --merge 8 --hebb 0.02   duplicate merging and rationed Hebbian swaps (needed under noise)

./target/release/evolvehd ngram --data corpus/bookA.bin,corpus/bookB.bin --order 4   # Witten-Bell baseline
./target/release/evolvehd table --corpus corpus --out-dir runs                       # the standard table
```

`run.json` holds bits per byte and accuracy per 1,000 bytes, the growth log (screens, trials, keeps, prunes, cleanup), the final nodes and kept edges with their learned gains.

## Crate layout

| Module | Holds |
|---|---|
| `rng` | Seeded xoshiro256**: the only source of randomness |
| `bits` | 2048-bit sets and 32-of-2048 codes |
| `pool` | Matcher pools: k-WTA, recruit on novelty, LRU eviction, Hebbian swaps, duplicate merging |
| `edge` | Prime edges: stochastic multi-level writer masks with learned gains, owner table |
| `info` | Entropy, MI and conditional MI with a circular-shift null: the screens |
| `brain` | Nodes (byte, delay, hold, real and exact binds, distill), the tick loop and the growth controller |
| `baseline` | Online Witten-Bell n-gram model |

See [wiki/reference-code.md](wiki/reference-code.md) for the Python ↔ Rust map and the port's validation numbers.

## License

MIT, see [LICENSE](LICENSE).
