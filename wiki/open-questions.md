# Open questions

Ordered roughly by how much they block the next step.

## Grammar and representation

- [x] **Words with real pools.** With fixed pools the word-pair conjoin gained nothing ([13](experiments/13-word-pairs.md)); with growing pools it is kept and held for 11.7M bytes ([15](experiments/15-growing-pools.md)). Pools that grow on demand, or a smaller code for word identity?
- [ ] **Word classes.** Classes of chunks by what follows them added nothing beyond the current chunk ([13](experiments/13-word-pairs.md)). What grouping keeps what the previous word tells about the next? Intermediate targets, where nodes predict their own next state, may supply it.
- [ ] **Compress for text.** Distilling a bind into 32 clusters of predicted successors never paid. The chunk is the first text variable kept on its own merits ([12](experiments/12-words.md)); is it the right thing to compress?
- [x] **Readout gap.** Hand-wired exact contexts trailed Witten-Bell counts of the same order by 0.22 bits per byte; wider masks and writes cut that to 0.14 ([16](experiments/16-readout-and-generation.md)), and per-bit levels with empty rows to 0.016 ([18](experiments/18-sparser-codes.md)). Most of it was counting noise.
- [ ] **Readout capacity for upper levels.** The writer-mask readout's error grows with outcomes per row ([17](experiments/17-level-stack.md)). Readout v3 with 8-bit codes cuts it to 0.24 bits at 512 symbols, and a word-piece level now uses 57–65% of its order-1 information ([18](experiments/18-sparser-codes.md)). Wider vectors or a factored readout could take the rest.
- [ ] **A third level.** The whole corpus didn't give level 1 a chunk ([19](experiments/19-whole-corpus-stack.md)). All three candidate causes were fixed ([20](experiments/20-fair-screens.md)): mixture screens, an adaptive vocabulary, and a recency move (which the bias edge already covered). Level 1 now keeps order-2 context, but its chunks still add nothing in trials, while whole words grow holds and binds that beat order-2 counts. Started from whole words, a stack does climb to a phrase level ([21](experiments/21-word-units.md)), so units were the block for the first climb. Two questions remain: how to cut bytes into units as consistent as words (uncertainty peaks give mostly whole words, but level 1 still keeps no chunk), and how to represent units above that, since phrases are too varied for one slot each (64% "other"), though coverage may not be the only problem. Fused codes (role-rotated parts, superposed) didn't help ([22](experiments/22-fusion.md)): the level above still predicts one cell per unit, and its cells either lump phrases or make nearly every phrase new. A factored readout was tested with counts first and would add only 0.003 bits per word beyond the words a few back ([23](experiments/23-factored-readout-gate.md)). At this scale, is there any structure above words that word-level delays, holds and recency don't already reach?
- [ ] **Beyond a few words.** Generated text loses track after a few words: no structure longer than two word-pieces has been grown ([16](experiments/16-readout-and-generation.md)). Phrase- and sentence-level context is the next level of stepping stones.
- [x] **Capacity.** Fixed pools stayed 0.45 bits per byte behind order-4 counts over the whole corpus ([14](experiments/14-full-corpus.md)). Pools that grow under eviction pressure closed it to 0.02 ([15](experiments/15-growing-pools.md)).
- [ ] **Memory horizon.** A growing pool keeps only contexts that recur within its horizon. On Austen, 400k ticks matched exact nodes where 50k did not, at twice the time. Should the horizon adapt, for example to how far back screens still find information?
- [ ] **Learning speed at size.** Each new source cell needs its own writer-mask rows; the grown full-corpus run took 2 h 14 min against 44 min. Cheaper rows, or rows shared by similar cells?
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
