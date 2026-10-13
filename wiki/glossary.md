# Glossary

Each term links to the page that explains it. For the ideas behind the terms, start at [concepts](concepts/README.md).

| Term | Meaning |
|---|---|
| [**Ablation gain**](concepts/shadow-trials.md) | For a kept edge: the target's loss without the edge minus its loss with it, averaged over a block. Used to prune |
| [**Backoff**](concepts/shadow-trials.md) | Ticks a rejected candidate must wait before another trial; doubles on each rejection |
| [**Bind**](concepts/shift-bind.md) | Expand node: one cell per seen combination of its parts' states (shift-bind + k-WTA cleanup). Real binds are matcher pools with a capacity; symbolic binds are exact tables |
| [**Bits per byte**](experiments/11-real-text.md) | Prequential log-loss: −log2 p(byte), scored before the byte is learned from |
| [**Broadcast alarm**](concepts/broadcast.md) | Phasic surprise signal per target: the fast accuracy average falls well below the slow one |
| [**Burn-in**](concepts/shadow-trials.md) | Ticks a trial edge learns before its gain starts being counted |
| [**Channel**](primitives.md#channels) | A typed edge: Drive, Prime, Teach, Gate control, Scalar broadcast, Bus |
| [**Chunk**](concepts/growth-operators.md#chunk) | A recurrent bind reset by a surprise boundary; its state is "the bytes since the last boundary" |
| [**Held-out gain**](concepts/screens.md) | A screen for high-cardinality contexts: counts on the first half of the window, log-loss gain on the second half |
| [**Surprise boundary**](experiments/12-words.md) | An event when the target's bits for the current byte exceed its running mean plus z standard deviations |
| [**Conjoin**](concepts/growth-operators.md#conjoin) | Grammar move: when a source carries information but its edge fails, bind it with the target's own state |
| [**Distill**](concepts/growth-operators.md#distill) | Compress node: a kept bind's cells grouped by what they predict |
| [**Drive**](concepts/drive-and-prime.md) | The only channel that creates winners while sensing |
| [**Echo**](concepts/credit-without-backprop.md#echo) | The local sign that a matcher took part in a loop that was actually used; decides whether global reinforcement applies to it |
| [**Event-gated hold**](concepts/time-delays-and-holds.md) | Gate node: latch a value when a rare event fires and keep it until the next one |
| [**Generation**](concepts/sparse-codes.md) | Per-cell counter bumped when a cell is re-recruited, so edges reset their masks for it |
| [**Granule layer**](concepts/growth-operators.md#granule-layer) | Correct primitive's expansion: one cell per joint value of every k-subset of sources |
| [**Horizon**](concepts/matcher-pools.md#growing-pools) | How recently an evicted cell must have been used for its eviction to count as pressure to grow the pool; in effect, the pool's memory span |
| [**Hysteresis**](concepts/shadow-trials.md) | Different bars for keeping and pruning an edge, so it doesn't flip back and forth |
| [**k-WTA**](concepts/matcher-pools.md) | k-winners-take-all; here usually k = 1 |
| [**Matcher**](engine-model.md) | Fires when the popcount of its masked window of history reaches a threshold |
| [**Opportunity**](concepts/attention.md) | Screen information on an unsolved target that no kept edge has captured; part of the attention score |
| [**Pool**](concepts/matcher-pools.md) | A set of matcher cells sharing an input, with recruitment, LRU eviction and k-WTA |
| [**Prime**](concepts/drive-and-prime.md) | Writer-mask votes that add evidence to existing candidates, without creating winners |
| [**Running count screen**](concepts/screens.md#running-count-screen) | A screen that keeps its own counts from the candidate's proposal on and scores each tick before learning from it; it can use far more data than a window |
| [**Screen**](concepts/screens.md) | A cheap estimate of how much a candidate would add: conditional MI against a shift null, or a held-out predictive gain |
| [**Trial extension**](concepts/shadow-trials.md#extensions-for-many-state-sources) | A trial of a source with more than 4,096 states that is positive but under the bar continues, up to 5 times its normal length |
| [**Shadow trial**](concepts/shadow-trials.md) | A trial edge that learns and is scored but does not vote |
| [**Shift-bind**](concepts/shift-bind.md) | Bind two codes by rotating one by an amount derived from the other, then cleaning up; keeps the pair code unlike either part |
| [**Station**](experiments/09-attention-temporal.md) | One world plus one grown brain, in the attention experiments |
| [**Sticky bit**](concepts/credit-without-backprop.md#sticky-bits) | A mask bit protected from swap rules after positive reinforcement |
| [**Stepping stone**](concepts/stepping-stones.md) | An easier puzzle whose solution builds a part a harder one needs |
| [**Synergy**](concepts/screens.md) | Information a pair carries about the target beyond what its parts carry |
| [**Writer mask**](concepts/writer-mask-readout.md) | Per source cell, bits over the target's code space; the score for a target cell is its code's overlap with the mask |
