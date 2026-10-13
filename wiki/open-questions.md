# Open questions

Ordered roughly by how much they block the next step.

## Grammar and representation

- [ ] **Words with real pools.** With exact nodes, a conjoin of the previous and current chunk is kept and cuts 0.05–0.09 bits per byte. With real pools it gains nothing; the 4,096-cell chunk pool is evicted about every 4 bytes, but the real conjoin never had an extended trial, so this is not yet proven ([13](experiments/13-word-pairs.md)). Pools that grow on demand, or a smaller code for word identity?
- [ ] **Word classes.** Classes of chunks by what follows them added nothing beyond the current chunk ([13](experiments/13-word-pairs.md)). What grouping keeps what the previous word tells about the next? Intermediate targets, where nodes predict their own next state, may supply it.
- [ ] **Compress for text.** Distilling a bind into 32 clusters of predicted successors never paid. The chunk is the first text variable kept on its own merits ([12](experiments/12-words.md)); is it the right thing to compress?
- [ ] **Readout gap.** Hand-wired exact contexts trail Witten-Bell counts of the same order by about 0.22 bits per byte. Better writer rule, or backoff between context orders?
- [ ] **Capacity (now the main limit).** 1,024-cell bind pools cost 0.28 bits per byte against exact tables on 400k bytes. Over 11.7M bytes the gap to order-4 counts stays at 0.4–0.6, and pools are evicted up to once every 3 bytes ([14](experiments/14-full-corpus.md)). Grow a parallel pool when recently used cells are being evicted?
- [ ] **Noise.** Where does duplicate merging live (a sleep phase)? And what overlap rule generalises beyond bytes? The Rust port does 0.2–0.6 bits worse than Python on book B under noise ([reference code](reference-code.md#validation-bits-per-byte-seed-0-unless-noted)), which suggests noisy runs are sensitive to the writer-mask drop rule.

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
