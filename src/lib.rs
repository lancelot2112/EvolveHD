//! EvolveHD: a learning system that grows its own network from seven primitives over
//! high-dimensional bit vectors. See `wiki/` for the design and the experiments behind it.
//!
//! Module map (mirrors the Python reference in `reference/python`):
//! - [`rng`]: seeded xoshiro256** generator, the only source of randomness.
//! - [`bits`]: 2048-bit sets and 32-of-2048 codes.
//! - [`info`]: entropy, mutual information and conditional MI over integer series (the screens).
//! - [`pool`]: matcher pools (k-WTA, recruit on novelty, LRU eviction, Hebbian swaps, merging).
//! - [`edge`]: Prime edges: stochastic multi-level writer masks with learned gains.
//! - [`brain`]: nodes, the tick loop and the growth controller for a byte stream.
//! - [`baseline`]: online Witten-Bell n-gram model, the yardstick for text runs.
//! - [`stack`]: levels of brains, each over the chunks of the level below.
//! - [`meaning`]: meaning codes learned from co-occurrence, story state, fast mapping and working memory.
pub mod assoc;
pub mod baseline;
pub mod bits;
pub mod brain;
pub mod edge;
pub mod episodic;
pub mod ctxholo;
pub mod hippo;
pub mod holo;
pub mod info;
pub mod meaning;
pub mod pool;
pub mod rng;
pub mod situation;
pub mod stack;
