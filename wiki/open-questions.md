# Open questions

Ordered roughly by how much they block the next step.

## Grammar and representation

- [ ] **Words.** Chunking by surprise pays, but its chunks are morph-like runs (15% are whole words), and a previous-chunk context screens negative at a 30k window ([12](experiments/12-words.md)). Next: distill chunks by their contexts into word classes, then screen previous-class holds; intermediate targets, so nodes predict their own next state.
- [ ] **Compress for text.** Distilling a bind into 32 clusters of predicted successors never paid. The chunk is the first text variable kept on its own merits ([12](experiments/12-words.md)); is it the right thing to compress?
- [ ] **Readout gap.** Hand-wired exact contexts trail Witten-Bell counts of the same order by about 0.22 bits per byte. Better writer rule, or backoff between context orders?
- [ ] **Capacity.** 1024-cell bind pools cost 0.28 bits per byte against exact tables. Grow capacity on demand (a parallel pool), or rely on factored binds?
- [ ] **Noise.** Where does duplicate merging live (a sleep phase)? And what overlap rule generalises beyond bytes?

## Scale and build

- [ ] **Screens at scale.** Every check compares every source with every target, and the bind screen compares pairs, so cost grows with the square of the node count. Running counts, local candidates and sampled pairs must make the same keep and reject decisions as window recounts.
- [ ] **Brute force.** The granule layer needs 10,000 cells for a 3-input puzzle and 490,000 for a 4-input one. Is a sparse, sampled granule layer enough?
- [ ] **Throughput.** The Rust port runs a grown 400k-byte text run in about 45 s, against about 430 s in Python, and matches its numbers ([reference code](reference-code.md#validation-bits-per-byte-seed-0-unless-noted)). The controller's screens now take most of the time. Word-level learning likely needs millions of bytes, so incremental screens come next. Chunks and held-out screens are not ported yet.

## Policy

- [ ] **Screens that can say "not enough data".** A held-out screen can't tell no information from too little data for a context with thousands of states ([12](experiments/12-words.md)). Confidence bounds, or a longer window for high-cardinality candidates?
- [ ] **Hand-set thresholds.** Every screen, trial and prune bar was set by hand. Tune them by population search across a family of worlds.
- [ ] **Give-up.** It never fired in 200 blocks with the fallback clause. Test it over long horizons with many hopeless inputs.

## From the engine spec (untested)

- [ ] Simultaneous-write collision policies, and whether the contested-bit rate is a useful signal.
- [ ] Competition groups derived from real mask and code overlap.
- [ ] A leak rate that scales with pool size.
- [ ] Log-shaped against uniform delay sampling.
- [ ] Density control on persistent vectors.
- [ ] A full-width run: many 2048-bit windows learning at once.
