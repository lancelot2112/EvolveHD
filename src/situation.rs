//! A learned situation model on ternary memory (experiment 34), the Rust port of `scripts/situation_slots.py`, with
//! relation classes pooled by meaning codes. Nothing about the words is given; inside, only bits and small
//! integers.
//!
//! 1. Clauses from learned boundaries: for each word u, the words before and after it at a stochastically kept
//!    earlier occurrence; hit counts are collision rates, and u is a boundary when lo^3 hi^2 > n^2 pair^3.
//! 2. Facts: each word w is bound to every earlier word s of its clause (back to and including the first
//!    boundary), both ways. The relation is the path of words between them, with its direction.
//! 3. Pooling: a path's code is the OR of its words' meaning codes (`Meaning`, a narrow-window instance, learned
//!    online), each rotated by its place in the path. Paths join the relation class whose prototype they overlap
//!    most (popcount, at least `pool` of the code), else start a class; prototypes are their members' most
//!    frequent bits. So "went to the" and "walked to the" become one relation once "went" and "walked" mean alike.
//!    Paths are re-assigned every `recluster` words as the meanings mature.
//! 4. Fast memory, ternary: a fact (s, class, w) is written at K active elements chosen from s *and* the class
//!    (one slot per entity and relation), phase key + value per element, adding +-mag to the real or imaginary
//!    part, clipped to -1, 0, +1 (mag 1: conflicting writes cancel to 0; mag 2: the newest sign wins). A slowly
//!    drifting context offsets the elements (a slot per learned boundary is redrawn).
//! 5. Slow memory: counts per (context, cue position) over classes (hop 1) and class pairs (hop 2). When the next
//!    word w arrives, each cue word's slots for the most frequent classes are read; a class whose slot ranks w
//!    first earns a count for that context. Hop 2 goes through the best value of each of the 10 most frequent
//!    classes. Prediction reads the slots of the context's best-counted classes.
//!
//! Experiment 36 (the expert forum's changes), each behind a switch, all integer:
//! - `kinds`: learned value kinds. Each word has 8-bit counters over its frames (the word before, the two words
//!   before, the word after), halved together when one saturates. (4-bit counters split the people into two
//!   kinds: a frame is a noisy observation, and a short window follows which verb a person used last.) Words are leader-clustered by the Jaccard
//!   overlap of their frame codes (counters at a quarter of their maximum or more), at least 2/5 by
//!   cross-multiplication, every `recluster` words. A kind keeps its name (a word id) across reclusterings.
//!   Slots are keyed on (entity, class, kind of the value), and C1/C2 count (class, kind). A slot of kind k is read
//!   against the words of kind k only (a typed clean-up), so an empty or noisy slot cannot return a word of
//!   another kind.
//! - `mood`: questions do not write. A clause's bindings are buffered until the boundary that closes it. Each word
//!   has a 4-bit counter of "after this boundary, the store's top proposal was the next word": up by 1 when it
//!   was, down by 1 on half the misses (a stochastic decrement from a hash bit, so it climbs above a third). A
//!   clause closed by a boundary whose counter is 8 or more is a cue, and its buffer is dropped.
//! - `recency`: among one cue word's proposals of one kind, those with at least half the best count go newest first
//!   (the slot's last write time, compared within the entity).
//! - `bounce`: reasoning as a loop of reads instead of the C1/C2 tables. At each step (at most 3) it either stops
//!   or reads one of the current entity's live slots and moves to the value read.
//!   - The slot is chosen by success counts per (context, cue position, step).
//!   - The stop is a 4-bit up/down counter per (context, kind of the current entity, step), stepped up where the
//!     answer was reached and down at an intermediate.
//! - `supersede`: when x's slot of kind k is written under one class, its other live slots of kind k in the same
//!   direction get the reserved NONE value (never proposed), so an entity keeps one live value per (direction,
//!   kind).
//! - `compose`: data bouncing at write time. When a fact (x, c, v) is committed and ((c, k), (c2, k2)) is among
//!   the `compose` most counted two-hop pairs, v's (c2, k2) slot is read then and written to x's composite slot,
//!   which C1 can learn like any class: "emma dropped the cup" also stores where emma is at that moment.
//! - `precision`: rank routes by Go minus NoGo, hits minus misses (a precision ranking, (hits + 1) / (hits + misses
//!   + 2), let rare lucky routes beat well-tested ones: where-person fell from 91% to 61-73%). A miss is
//!   counted for the routes behind the answer actually given (the top proposal) when the next word was something
//!   else: Go and NoGo, the forum's three-factor rule, credited like Select (only the taken action is seen). Ranking by hits alone lets a route through a function word win, because it lands on the
//!   most recent place by chance a third of the time and is never penalized.
//! - `entity_cues`: only words of the larger learned kinds (at least 3 members) are read from as cue entities. A
//!   function word's slots hold whatever was written next to it last ("the" -> the latest place), a recency
//!   buffer that wins by luck a third of the time.
//! - `live`: learn over the cue entity's live slots (two hops: the intermediate's too) instead of the most
//!   frequent classes. The two-hop search over the 10 most frequent classes never saw "the apple was got by",
//!   whose class is rarer.
//! - `query`: the general query learner, in place of the C1/C2 tables, the context hash, the cue positions, the
//!   entity-cue rule and the recency rule. A bitwise W_QK:
//!   - the query code is the last 4 words' frame codes, each in its position's block, plus identity bits, plus 4
//!     bits hashed from the words 1 and 3 back together (a conjunction: the question form, apart from its entity);
//!   - a route's key is a set of features: cue position, the cue word's kind, per hop the class, the kind and the
//!     slot's recency rank within its entity, and the number of hops;
//!   - W holds signed 8-bit weights per (query bit, feature);
//!   - every one- and two-hop route from the cue words' 8 newest live slots is scored by the sum of its weights
//!     under the query, and the best is read;
//!   - learning is mistake-driven and three-factor (a perceptron): when the top route was wrong, the best-scored
//!     route that held the next word is stepped up and the top route down. (Stepping up every route that held it
//!     instead fell to 52%: the frequent routes' weights swamp the rest.)
//! - `replay`: the query learner keeps its mistakes (the query, the routes as they were, the answer) in a ring of
//!   512 and re-learns this many per word, so the rarer two-hop questions train it as often as the common ones.
//! - `fpool`: relations pooled by the words' frame codes (the 8-bit frame counters of `kinds`, all integer)
//!   instead of the meaning module's codes (float inside); the meaning module is then not run. Only distinctive
//!   frame features count: those in at most an eighth of the words' codes (recounted at each recluster), so
//!   "went" and "got" (both after a name) differ by what follows them.
//! - `lookback` N: a window of the last N words (the transformer's context, made explicit). The same query
//!   learner scores routes over it: from a cue word's latest occurrences, the word at an offset, and again from
//!   there (two hops). An arbiter keeps, per (question form: the words 1, 3 and 4 back; both sources' margin buckets),
//!   how often the slots' answer and the window's answer were right where the two disagreed. It answers from the window where the window has been right more often
//!   (cross-multiplied, prior 1 in 2). So the window is read for its answer only where the slots are unsure.
//!   Both sources learn at every word.
//! - `sq_max`: open queries, at most this many at once. An open query is a two-hop pattern ((class, kind), then
//!   (class, kind)) kept answered for every entity while reading, in a derived slot of its own. A committed fact
//!   that matches its first hop moves the entity's pointer and refreshes its slot. One that matches its second
//!   hop refreshes every entity pointing to the fact's subject: "fred went to the kitchen" moves every object fred
//!   holds. The query learner reads derived slots like any other.
//!   - **Which open:** the two-hop patterns that held answers to questions are counted. An answer is the word
//!     after a boundary the mood counter has learned closes a question. The most counted (at least 16) opens
//!     when there is room.
//!   - **Which stay:** each open query starts with a balance of 512. It loses 64 when an answer given through its
//!     slot was wrong. It gains 32 when its slot held an answer
//!     and pays 1 per read or write of upkeep. After 2,000 words, one whose balance falls below 0 closes. So the number open tunes itself under
//!     the cap.
//! - `seed`: salts the codes, keys and context.
use crate::meaning::{Meaning, MeaningCfg};
/// A fast integer hasher (FxHash-style multiply-rotate) for the many small-key maps; std's SipHash is the costlier
/// default meant to resist adversarial keys, which these maps do not face.
#[derive(Default, Clone, Copy)]
pub struct Fx(u64);
impl std::hash::Hasher for Fx {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0.rotate_left(5) ^ b as u64).wrapping_mul(0x51_7C_C1_B7_27_22_0A_95);
        }
    }
    fn write_u8(&mut self, i: u8) {
        self.0 = (self.0.rotate_left(5) ^ i as u64).wrapping_mul(0x51_7C_C1_B7_27_22_0A_95);
    }
    fn write_u32(&mut self, i: u32) {
        self.0 = (self.0.rotate_left(5) ^ i as u64).wrapping_mul(0x51_7C_C1_B7_27_22_0A_95);
    }
    fn write_u64(&mut self, i: u64) {
        self.0 = (self.0.rotate_left(5) ^ i).wrapping_mul(0x51_7C_C1_B7_27_22_0A_95);
    }
    fn write_usize(&mut self, i: usize) {
        self.write_u64(i as u64);
    }
}
type HashMap<K, V> = std::collections::HashMap<K, V, std::hash::BuildHasherDefault<Fx>>;

fn hash(a: u64, b: u64) -> u64 {
    crate::rng::hash2(a, b)
}

#[derive(Clone, Debug)]
pub struct SitCfg {
    pub d: usize,
    pub k: usize,
    pub mag: i8,
    pub cue: usize,
    /// The cue reaches back to the start of the clause (the last boundary before it), at least `cue` words and at most
    /// 12: "what did X find in chapter twenty two ?" then has X in view. 0 = the last `cue` words only.
    pub cue_clause: bool,
    /// Long-term heads: an exact newest-wins table of committed facts, (entity, relation class, value kind) -> (value,
    /// time), that never drifts (symbols, not superposed vectors). Its entries join the route candidates as one-hop
    /// routes with a "long-term" source feature, so the query learner learns when to read them.
    pub heads: bool,
    /// Heads keyed by the exact relation path ("owns the" / "got the"), not the pooled class: only an exact
    /// restatement overwrites a durable fact. All of an entity's heads are offered (up to 24), ranked within kind.
    pub heads_path: bool,
    /// With heads_path: the newest heads offered per value kind of 3 or more members (8 by default), one per smaller
    /// kind, up to 48 in all.
    pub heads_per_kind: u32,
    /// Units and labels (with heads_path). Every committed clause's text (its words, in order) is indexed with its
    /// first and latest position; every clause's first word is an opener type with the start positions of its
    /// clauses; a unit runs from a clause of a type to the next clause of the same type. Every head keeps its history
    /// (time, value). At a learned question boundary, spans of the question clause that equal a stored clause's text
    /// are anchors; the cue entity's history inside each anchor's unit gives candidates, with features for the opener
    /// type, first or latest occurrence, span length, the path and the rank within the range.
    pub ranges: bool,
    /// Nested anchors (with --sit-ranges): two spans of a question that both name clauses give the inner span's
    /// occurrence inside the outer span's unit ("chapter twelve" within "part three"): structure of structure.
    pub nest: bool,
    /// Scoped anchors (with --sit-ranges): every anchor carries the rank of its occurrence from the newest (0 the
    /// latest, 1 the one before, 2 any older), for a nested anchor the rank of the outer span's occurrence; route
    /// features conjoin the opener type with which occurrence and that rank, a nested anchor's outer type and rank
    /// with its inner type, and count how many anchor units contain the candidate. Nested anchors keep the newest 4.
    pub scope: bool,
    /// From this word on, route skill is frozen: no query learning, no open-query credit, and no change to the
    /// question detector (the mood counters). u64::MAX: never.
    pub freeze_from: u64,
    /// Role features for anchors (with --sit-ranges): an opener type enters the route features as its level, not
    /// its word: the bit length of the span of its starts minus the bit length of their count less one (about
    /// log2 of its mean recurrence interval, without a division), so a new book's headings in new words reuse the
    /// skill learned on old ones.
    pub roles: bool,
    /// Skippable words left out of exact relation paths (with --sit-heads-path): a word is skippable when, at
    /// three quarters or more of its (at least 16) occurrences, the word before it has already been seen directly
    /// followed by the word after it ("mary finally lives": "mary lives" occurs too). Modifiers, and in prose
    /// adjectives, then do not split one relation into many paths.
    pub skip: bool,
    /// Stand-in words: a clause whose first word belongs to a small kind (under 3 members) whose contexts are
    /// nearly all contexts of a content kind (its kind code three quarters inside the code of that kind's most
    /// frequent word) has its facts filed under the newest clause-initial word of that kind ("she went to the
    /// garden" after "ella picked up the ring": ella). Pronouns, learned from contexts, nothing named.
    pub anaphora: bool,
    /// Epochs, the level above the top labels (with --sit-ranges): an opener type's epoch restarts where its first
    /// clause text recurs ("part one ." again: a new book; "chapter one ." again: a new part). An anchor looks only
    /// at its occurrences in the current epoch of its type when there are any, so "part three" means this book's
    /// part three, the newest context first.
    pub epoch: bool,
    /// One rule for every level of nesting, any depth (with --sit-ranges --sit-nest): the question's labels are
    /// ordered by the level of their types (the log2 recurrence interval, as in `roles`), and from each label as the
    /// root (its first and its latest occurrence, in its type's current epoch with --sit-epoch) a chain descends: each
    /// lower label's first occurrence inside the current unit narrows the unit to its own ("chapter twelve" in "part
    /// three" in "volume two"). Replaces the pairs of `nest`; a chain's depth enters the route features.
    pub levels: bool,
    /// Kinds from an area of matchers (threshold, inhibition, Hebbian and anti-Hebbian swaps; see `area`) instead
    /// of re-clustering every word from scratch: the same rule the clause level can use.
    pub area_kinds: bool,
    /// A clause's type (for units, epochs and anchors) is the kind of its first word, the word level's output,
    /// instead of the word itself: the level above reads the level below's classes.
    pub open_kind: bool,
    /// Anti-Hebbian learning for the kind area's matchers that fire but are inhibited, which pulls apart patterns
    /// that overlap: 0 none; 1 one shared bit out and a bit outside the input in; 2 one contested bit out (held by
    /// the input, the winner and the loser), the mask shrinking to at most half its first size.
    pub area_anti: u32,
    /// The width of the code words are clustered on (a power of two up to 512): the frame features fold onto
    /// fewer bits, so patterns overlap more (does anti-Hebbian learning let a narrower code serve?).
    pub kind_bits: usize,
    /// The kind area's matchers keep their words while they still fire, and grow less plastic as they win, so
    /// kinds hold still under skill trained on them (see `area`).
    pub area_sticky: bool,
    /// Relation classes by locality-sensitive hashing instead of a search over prototypes: a path's code (its
    /// words' frame bits) gets `lsh_bands` MinHash bands of `lsh_rows` rows each (the least hash of its bits per
    /// row), and two paths share a class when a band collides. A new path costs bands x rows hashes over its bits,
    /// whatever the number of classes; a band that hits gives the class, the other bands are filed under it.
    pub lsh: bool,
    /// Slot reads without a scan: a value's phase at a key's i-th position depends on (value, i) only, so every
    /// value has one fixed code of K phases, and an unbound slot is an estimate of that code. The codes are
    /// indexed by bit sampling (8 bands of 8 positions, 16-bit keys); a read takes the phase of each position
    /// from the unbound slot, looks up the bands it estimates fully, and scores only the values found there.
    pub vlsh: bool,
    /// Long-term heads keyed by (entity, exact path) only, not also by the value's kind at write time: when
    /// kinds are renumbered, a path's old and new heads would otherwise both stay and crowd the per-kind cap.
    pub heads_nokind: bool,
    /// Slot reads from the slot's own write log: every slot (entity, class, kind) keeps the last `slotlog` values
    /// written to it, found by hashing the slot, and a read scores only those (0 = scan the kind).
    pub slotlog: usize,
    /// An entity's newest slots from a recency list kept as slots are written (its last `recent` (class, kind)
    /// slots, move to front), not by gathering and sorting all of its slots at every read (0 = gather all).
    pub recent: usize,
    /// Boundaries learned slowly: a word becomes a boundary at half the maximum openness, as before, and then
    /// stays one while its own openness holds (at least a quarter of its own peak), whatever more open words new
    /// text brings (",", "and" in prose): new boundaries are added, learned ones (". ?") are not erased.
    pub bnd_hyst: bool,
    /// Long-term tier for recurring relations only: a binding enters the heads and their histories only once its
    /// exact path has been seen at least this many times, counted in a fixed count-min sketch (4 rows of 2^16
    /// 16-bit counters, 512 KB, the minimum of the four). Paths seen once or twice (almost every path in prose)
    /// stay in working memory and fade. 0 = store every binding.
    pub lt_min: u32,
    /// Long-term tier kept by credit: a recurring path may enter it for its first `lt_credit` bindings; after
    /// that, only if a route through a long-term head or range of that path has answered a question right (the
    /// next word after a learned question boundary). Relations that never help answer stop being stored. 0 = off.
    pub lt_credit: u32,
    /// Working memory forgets its bookkeeping too: every horizon/2 words, slots not written for `wm_horizon`
    /// words are dropped from the slot tables (their live kinds, write times, write logs, the entity's class
    /// lists), long after their drifting keys have made them unreadable. 0 = keep all.
    pub wm_horizon: u64,
    /// Paths remembered only once they recur (with --sit-lsh): a path is counted in a count-min sketch, and only
    /// a path seen this many times is kept (in the path counts, its class, the bands, the re-pooling). A rarer
    /// path takes the class of a band it hits, or a shared class for rare paths, and is not stored. 0 = keep all.
    pub path_min: u32,
    /// Consolidation forgets one-off paths: at each re-pooling, paths seen fewer than this many times are dropped
    /// (their counts and classes) instead of re-hashed; a path met again is classed afresh. 0 = keep all.
    pub path_forget: u64,
    pub vlsh_band: usize, // positions per band (default 4)
    pub vlsh_hits: usize, // bands a value must match to be a candidate (default 1)
    pub lsh_bands: usize,
    pub lsh_rows: usize,
    /// Speed: slot reads cached within a word (cleared on any write), kind member lists kept with the kinds, the
    /// background total kept as a running sum, and a cheaper multiplicative phase hash for values.
    pub fast: bool,
    /// Which parts of the fast path are on (bits, with `fast`): 0 read cache, member lists and running background;
    /// 1 cheap phase hash; 2 second hops only from the 4 nearest content cue words; 3 heads and ranges only from
    /// content cue words; 4 heads filtered to content values when read; 5 heads stored only for content values.
    /// Default 23 (bits 0, 1, 2, 4): bits 3 and 5 cost long-range accuracy (owner at 8k-64k words) for little speed.
    pub fast_mask: u32,
    pub win: usize,
    pub ntop: usize,
    pub pool: f64,
    pub recluster: usize,
    pub mdim: usize,
    pub mwin: usize,
    pub bdrift: usize,
    /// Per-slot clocks: slot i's phase steps on every `clock` * 2^(i mod 7) words (0 = off), so a write fades
    /// gradually as the clocks of more of its slots tick; proposals from reliable routes are then ranked by
    /// strength, the freshest first.
    pub clock: u64,
    /// Attend each path word's meaning to its clause before pooling (its sense in context).
    pub sense: bool,
    pub sense_mix: f32,
    /// Learned senses per word (at most), and the match below which a context starts a new one.
    pub nsenses: usize,
    pub sense_new: f64,
    pub kinds: bool,
    pub mood: bool,
    /// The mood counter credits memory only beyond local statistics: it moves only when the boundary word's most
    /// frequent successor (a bigram count) would have missed the next word; then up if the store predicted it, down
    /// (half the time) if not. After "?" the answer is retrieval; after "apple" the "." is just the bigram.
    pub mood2: bool,
    /// Clause boundaries by open contexts on both sides: for each word, the number of distinct successors covering
    /// 90% of its uses, and the same for predecessors (from bigram counts); a word is a boundary when the smaller of
    /// the two is at least half the largest such value over all words. Replaces the independence test. Recomputed
    /// every `recluster` words.
    pub bnd2: bool,
    pub recency: bool,
    pub bounce: bool,
    pub supersede: bool,
    /// Write-time composition: the number of most-counted two-hop pairs composed when a fact is committed (0 = off).
    pub compose: usize,
    /// Rank a route by its precision (hits over hits plus misses when it was proposed), not by its hit count.
    pub precision: bool,
    /// Only words of the larger kinds (at least 3 members: people, places, objects) act as cue entities; function
    /// words, each its own kind, are not read from.
    pub entity_cues: bool,
    /// Learn over the cue entity's live slots (and, for two hops, the intermediate's) instead of the most frequent
    /// classes: every relation an entity actually has is searched, however rare its class.
    pub live: bool,
    /// Routes proposed per cue word (most counted live first), and the credibility share: a route competes on
    /// recency if its count times `cred` reaches the best count.
    pub nroutes: usize,
    /// The general query learner (see the module docs) instead of the C1/C2 tables.
    pub query: bool,
    /// The query weights stored sparse at 8 bits instead of a dense 16-bit table. 1: an open-addressing table of
    /// 32-bit keys and 8-bit saturating weights (grown at 3/4 full). 2: per feature, a list of (query bit as 16 bits,
    /// weight as 8 bits), 3 bytes per nonzero weight; a weight that returns to 0 is dropped.
    pub qsparse: u32,
    /// With --sit-qsparse 3: the query weights superposed in one vector of 2^qholo 8-bit cells. A (query bit, feature)
    /// pair is bound to a cell and a sign by a hash; pairs that land together share the cell (crosstalk).
    pub qholo: u32,
    /// Route choice without weights: the question form (the words 1, 3 and 4 back) is an entity in the same store as
    /// the facts. Its slots, keyed on (cue position, the cue word's kind, a route field), hold route symbols (number of
    /// hops; per hop the relation class, the value's kind and the recency rank). A route is scored by how well its
    /// fields match what the question's slots hold. On a mistake, the route that held the answer is written into the
    /// slots (Hebbian) and the wrong route's differing fields are written with opposite phase (anti-Hebbian).
    pub qslots: bool,
    /// The write magnitude of the question slots (the facts use `mag`).
    pub qslot_mag: i8,
    /// The question slots' cell depth: each component is clamped to -c..c (1 = ternary, as the facts).
    pub qslot_clamp: i8,
    /// Which route fields the question slots use (bit i = field i: 0 hops, 1/4 class, 2/5 kind, 3/6 rank; 7 the two
    /// kinds together, 8/9 each hop's class with its kind).
    pub qslot_fields: u32,
    /// Anti-Hebbian writes for the wrong route's differing fields.
    pub qslot_anti: bool,
    /// The question form from the learned kinds of the last 4 words (instead of the identities of the words 1, 3
    /// and 4 back): "what place is mary in ?" and "what place is john in ?" are one question.
    pub qslot_kforms: bool,
    /// Every route field's symbol also carries the route's number of hops (one- and two-hop routes keep their
    /// evidence apart).
    pub qslot_hopconj: bool,
    /// Curiosity (with `qslots` and `lookback`): answer from memory where answers like this one (same question form,
    /// same margin bucket) have been right often enough; otherwise look it up in the lookback window (read the same
    /// weight-free way). Answer from the source that has been right more often in this situation (question form, both
    /// margins, whether the two agree), and answer nothing ("unknown") if that source has not been right often
    /// enough either. "Often enough": right x den > num x n, with a prior of 1 right in 2.
    pub curious: bool,
    /// The "two words before" frame feature uses the kind of the word two back, not its identity: ⟨kind, word before⟩.
    /// Synonyms (picked up / grabbed / took …) then count as one frame instead of diluting it.
    pub kframes: bool,
    /// Slow merges: two established kinds (each with at least 2 members) merge only after `kslow` reclusters in a row
    /// propose it; until then the proposed cluster is split back by the old kinds (0 = merge at once). Splits are free.
    pub kslow: u32,
    /// The cut for the kinds' clustering codes: a frame feature counts at max >> kshift or more (2 = a quarter, the
    /// codes used elsewhere). A lower cut (3 = an eighth) keeps features that synonyms dilute, so objects keep the
    /// frames that tell them from places.
    pub kshift: u32,
    pub curious_num: u32,
    /// The trust counts for the two sources keyed on (form, agree) only, not the margins too.
    pub curious_coarse: bool,
    /// The window also reads and learns at every question boundary where memory is sure (practice), not only when
    /// memory is unsure; its answer is still used only when memory is unsure.
    pub curious_practice: bool,
    pub curious_den: u32,
    /// The query code also carries 4 bits hashed from the words 1 and 4 back together ("?" ... "has" against "?" ...
    /// "is": the two question forms share the words 1 and 3 back).
    pub qform14: bool,
    /// An entity's read list takes each kind's newest value before any kind's second newest (the list is short: 8
    /// slots; without this a clause's function words push an older place out of it).
    pub kindfirst: bool,
    /// Query learner: mistakes kept in a ring of 512 and re-learned, this many per word (0 = off).
    pub replay: usize,
    /// Replay question episodes only: keep the mistakes made answering a question (after a learned question
    /// boundary) and replay `replay` of them at each question, not at every word.
    pub replay_q: bool,
    /// Pool relations by the words' frame codes (8-bit frame counters, integer) instead of the meaning module.
    pub fpool: bool,
    /// Lookback: a window of the last `lookback` words, searched by the same query learner when the slots are unsure
    /// (0 = off).
    pub lookback: usize,
    /// Lookback merged: the window's routes compete with the slots' routes in one pool under the same learner
    /// (no separate arbiter).
    pub merge: bool,
    /// Open queries: the most open at once (0 = off). Each is a two-hop pattern ((class, kind) then (class, kind))
    /// kept answered for every entity as facts arrive, into a derived slot of its own; the system opens and closes
    /// them by their usefulness, so the number open tunes itself under this cap.
    pub sq_max: usize,
    /// How open queries are chosen. 0: top-down only (the patterns that held most question answers). 1: the input
    /// proposes (every committed fact proposes the two-hop chains starting from it, counted) and the higher level
    /// ranks (the proposals' route features scored by the learned query weights under the question codes seen
    /// lately). 2: as 1, and the chain must also have held at least 4 answers.
    pub sq_mode: u32,
    /// Open queries' credit. 0: credit (+32) whenever a query's slot held the answer. 1: credit only for what the
    /// query contributed: +32 when the answer given came through its slot and was right, +16 when the answer given
    /// was wrong and its slot held the right one; nothing when another route answered right anyway.
    pub sq_credit: u32,
    /// Which open queries' derived slots an entity's reader sees (it sees at most 4): 0 the newest written, 1 the
    /// queries with the highest balance, 2 the queries with the most credits.
    pub sq_rank: u32,
    /// Open queries by kinds: also count the chains in which both hops were the entity's newest value of their kind,
    /// as a pattern over the kinds alone (any relation, then any relation): "the newest person, then that person's
    /// newest place". The counts of all the relations that reach a person pool into one.
    pub sq_kinds: bool,
    /// The price of staying open. 0: a query closes when its balance is below 0 after its trial. 1: relative: when its
    /// net payoff per word since opening is below the average of the open queries past their trial (cross-multiplied).
    /// 2: either.
    pub sq_price: u32,
    /// Which answers count toward opening a pattern. 0: every answer it held. 1: only the answers it held when the
    /// answer given was wrong (it would have fixed a mistake): patterns that hold what is already answered right add
    /// nothing.
    pub sq_open: u32,
    pub cred: u32,
    pub seed: u64,
}

/// Sparse 8-bit weights: an open-addressing hash table (linear probing, power-of-two capacity, grown at half full)
/// of 32-bit keys and saturating 8-bit values. A key never updated reads 0.
#[derive(Clone)]
pub struct SparseW {
    keys: Vec<u32>,
    vals: Vec<i8>,
    pub len: usize,
}
impl SparseW {
    const EMPTY: u32 = u32::MAX;
    pub fn new() -> Self {
        SparseW { keys: vec![Self::EMPTY; 1024], vals: vec![0; 1024], len: 0 }
    }
    fn slot(&self, k: u32) -> usize {
        // multiplicative hash (Fibonacci), then linear probing
        let m = self.keys.len() - 1;
        let mut i = ((k as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 32) as usize & m;
        while self.keys[i] != Self::EMPTY && self.keys[i] != k {
            i = (i + 1) & m;
        }
        i
    }
    pub fn get(&self, k: u32) -> i8 {
        let i = self.slot(k);
        if self.keys[i] == k { self.vals[i] } else { 0 }
    }
    pub fn add(&mut self, k: u32, d: i8) {
        if 4 * (self.len + 1) > 3 * self.keys.len() {
            let (ok, ov) = (std::mem::take(&mut self.keys), std::mem::take(&mut self.vals));
            self.keys = vec![Self::EMPTY; ok.len() * 2];
            self.vals = vec![0; ok.len() * 2];
            for (k, v) in ok.into_iter().zip(ov) {
                if k != Self::EMPTY {
                    let i = self.slot(k);
                    self.keys[i] = k;
                    self.vals[i] = v;
                }
            }
        }
        let i = self.slot(k);
        if self.keys[i] != k {
            self.keys[i] = k;
            self.len += 1;
        }
        self.vals[i] = self.vals[i].saturating_add(d);
    }
    /// Bytes held (keys and values, at the table's capacity).
    pub fn bytes(&self) -> usize {
        self.keys.len() * 5
    }
    /// (entries nonzero, entries at the 8-bit limit)
    pub fn census(&self) -> (usize, usize) {
        let mut nz = 0;
        let mut sat = 0;
        for (k, v) in self.keys.iter().zip(&self.vals) {
            if *k != Self::EMPTY && *v != 0 {
                nz += 1;
                if *v == i8::MAX || *v == i8::MIN {
                    sat += 1;
                }
            }
        }
        (nz, sat)
    }
}

/// Sparse 8-bit weights by feature: for each feature, the query bits with a nonzero weight (16 bits) and their
/// weights (8 bits, saturating). A feature's score under a query is the sum of its weights, each times the number of
/// times its bit is in the query. A row whose list grows past a third of the query bits is kept dense (one byte per
/// bit), which is no bigger than its list and scores as one vector dot product; the sums are the same.
#[derive(Clone)]
pub struct RowW {
    bits: Vec<Vec<u16>>,
    vals: Vec<Vec<i8>>,
    dense: Vec<Vec<i8>>,
}
const DENSE_AT: usize = QD / 3;
impl RowW {
    pub fn new(nf: usize) -> Self {
        RowW { bits: vec![vec![]; nf], vals: vec![vec![]; nf], dense: vec![vec![]; nf] }
    }
    pub fn score(&self, fi: u32, qcount: &[u8]) -> i64 {
        let dn = &self.dense[fi as usize];
        if !dn.is_empty() {
            // i8 x u8 products fit i16; sums of QD of them fit i32
            let s: i32 = dn.iter().zip(&qcount[..QD]).map(|(&w, &c)| w as i32 * c as i32).sum();
            return s as i64;
        }
        let (b, v) = (&self.bits[fi as usize], &self.vals[fi as usize]);
        let mut s = 0i64;
        for (&bb, &w) in b.iter().zip(v) {
            s += w as i64 * qcount[bb as usize] as i64;
        }
        s
    }
    pub fn add(&mut self, fi: u32, b: u16, d: i8) {
        let dn = &mut self.dense[fi as usize];
        if !dn.is_empty() {
            dn[b as usize] = dn[b as usize].saturating_add(d);
            return;
        }
        if self.bits[fi as usize].len() >= DENSE_AT {
            let mut row = vec![0i8; QD];
            for (&bb, &w) in self.bits[fi as usize].iter().zip(&self.vals[fi as usize]) {
                row[bb as usize] = w;
            }
            row[b as usize] = row[b as usize].saturating_add(d);
            self.dense[fi as usize] = row;
            self.bits[fi as usize] = vec![];
            self.vals[fi as usize] = vec![];
            return;
        }
        let (bs, vs) = (&mut self.bits[fi as usize], &mut self.vals[fi as usize]);
        match bs.iter().position(|&x| x == b) {
            Some(i) => {
                vs[i] = vs[i].saturating_add(d);
                if vs[i] == 0 {
                    bs.swap_remove(i);
                    vs.swap_remove(i);
                }
            }
            None => {
                bs.push(b);
                vs.push(d);
            }
        }
    }
    /// Bytes held: 3 per weight at the lists' capacity, one per bit for dense rows, plus the lists' headers.
    pub fn bytes(&self) -> usize {
        self.bits.iter().map(|b| b.capacity() * 2).sum::<usize>()
            + self.vals.iter().map(|v| v.capacity()).sum::<usize>()
            + self.dense.iter().map(|v| v.capacity()).sum::<usize>()
            + self.bits.len() * 72
    }
    pub fn census(&self) -> (usize, usize) {
        let all = || self.vals.iter().flatten().chain(self.dense.iter().flatten());
        let nz = all().filter(|&&w| w != 0).count();
        let sat = all().filter(|&&w| w == i8::MAX || w == i8::MIN).count();
        (nz, sat)
    }
    /// A row as dense weights.
    fn row(&self, fi: usize) -> Vec<i16> {
        let mut r = vec![0i16; QD];
        if !self.dense[fi].is_empty() {
            for (x, &w) in r.iter_mut().zip(&self.dense[fi]) {
                *x = w as i16;
            }
        } else {
            for (&b, &w) in self.bits[fi].iter().zip(&self.vals[fi]) {
                r[b as usize] = w as i16;
            }
        }
        r
    }
    /// What was learned since `base`: (feature, bit, change).
    pub fn delta(&self, base: &RowW) -> Vec<(u32, u16, i16)> {
        let mut d = vec![];
        for fi in 0..self.bits.len() {
            let (a, b) = (self.row(fi), base.row(fi));
            for (i, (&x, &y)) in a.iter().zip(&b).enumerate() {
                if x != y {
                    d.push((fi as u32, i as u16, x - y));
                }
            }
        }
        d
    }
    /// Add learned changes (saturating at 8 bits, as learning does).
    pub fn apply(&mut self, d: &[(u32, u16, i16)]) {
        for &(fi, b, mut x) in d {
            while x != 0 {
                let step = x.clamp(-127, 127);
                self.add(fi, b, step as i8);
                x -= step;
            }
        }
    }
    pub fn dense_rows(&self) -> usize {
        self.dense.iter().filter(|v| !v.is_empty()).count()
    }
}

/// A learned sense of a word: a prototype of the contexts it is used in (counts over the identity bits of the
/// clause words around it).
#[derive(Clone)]
struct Sense {
    counts: Vec<u32>,
    n: u32,
}

/// A pooled relation class: a prototype (bit counts) and its name (its first path).
#[derive(Clone)]
struct Proto {
    counts: Vec<u32>,
    bits: Vec<u32>,
    dir: u8,
    n: u32,
}

/// A shard's long-term index (see `export_index`).
#[derive(Clone, Default)]
pub struct LtIndex {
    pub heads: Vec<((u32, u32, u32), (u32, u64, u32))>,
    pub posts: Vec<((u32, u32, u32), Vec<(u64, u32)>)>,
    pub ctext: Vec<(u64, u32, Vec<u64>)>,
    pub opener: Vec<(u32, Vec<u64>)>,
}
impl LtIndex {
    /// The entries at times in [from, to).
    pub fn range(&self, from: u64, to: u64) -> LtIndex {
        let inr = |t: u64| t >= from && t < to;
        LtIndex {
            heads: self.heads.iter().filter(|x| inr(x.1 .1)).copied().collect(),
            posts: self.posts.iter().map(|(k, h)| (*k, h.iter().filter(|x| inr(x.0)).copied().collect::<Vec<_>>())).filter(|x| !x.1.is_empty()).collect(),
            ctext: self.ctext.iter().map(|(h, fw, o)| (*h, *fw, o.iter().filter(|&&x| inr(x)).copied().collect::<Vec<_>>())).filter(|x| !x.2.is_empty()).collect(),
            opener: self.opener.iter().map(|(f, p)| (*f, p.iter().filter(|&&x| inr(x)).copied().collect::<Vec<_>>())).filter(|x| !x.1.is_empty()).collect(),
        }
    }
    /// Entries (heads + postings + clause starts + opener starts).
    pub fn len(&self) -> usize {
        self.heads.len()
            + self.posts.iter().map(|x| x.1.len()).sum::<usize>()
            + self.ctext.iter().map(|x| x.2.len()).sum::<usize>()
            + self.opener.iter().map(|x| x.1.len()).sum::<usize>()
    }
}

#[derive(Clone)]
pub struct Situation {
    pub cfg: SitCfg,
    nw: usize,
    re: Vec<i8>,
    im: Vec<i8>,
    bg: Vec<u64>,
    ctx_off: Vec<u32>,
    ctx_ph: Vec<u8>,
    // segmentation
    seg_draw: HashMap<u32, (u32, u32)>,
    seg_hits: HashMap<u32, [u64; 3]>,
    seg_n: HashMap<u32, u64>,
    boundary: Vec<bool>,
    // paths and classes
    mn: Meaning,
    path_count: HashMap<Vec<u32>, u64>, // dir-tagged path -> uses
    path_class: HashMap<Vec<u32>, u32>,
    protos: Vec<Proto>,
    proto_by_bit: Vec<Vec<u32>>,                    // relation classes by the bits of their prototypes (an inverted index)
    lsh_bucket: HashMap<u64, u32>,                  // --sit-lsh: band key -> relation class
    vband: HashMap<u64, Vec<u32>>,                  // --sit-vlsh: band key -> the values whose code has it
    vcodes: Vec<u128>,                              // --sit-vlsh: each value's code, 2 bits per key position
    slot_log: HashMap<(u32, u32, u32), Vec<u32>>,   // --sit-slotlog: slot -> its last values written, newest last
    recent_slots: HashMap<u32, Vec<(u32, u32)>>,    // --sit-recent: entity -> its last slots written, newest last
    path_sketch: Vec<u16>,                          // --sit-lt-min: count-min sketch of exact paths (4 x 2^16)
    bnd_peak: HashMap<u32, u32>,                    // --sit-bnd-hyst: a boundary's peak openness
    path_seen: Vec<u16>,                            // --sit-path-min: count-min sketch of whole paths
    rare_class: [u32; 2],                           // --sit-path-min: the shared class of rare paths, per direction
    pscratch: Vec<u32>,                             // shared-bit counts per class, for one lookup
    class_count: Vec<u64>,
    senses: HashMap<u32, Vec<Sense>>,
    // slow memory
    c1: HashMap<(u64, usize), HashMap<(u32, u32), u32>>,
    c2: HashMap<(u64, usize), HashMap<((u32, u32), (u32, u32)), u32>>,
    hist: Vec<u32>,
    clause: Vec<u32>,
    seen: Vec<bool>,
    words: Vec<u32>,
    t: u64,
    pub last: Vec<(u32, u32)>, // the last prediction's proposals: (weight, value)
    learning: bool,
    // experiment 36
    pool_pm: u64,                                   // the pooling threshold in thousandths (integer compare)
    frame: Vec<Vec<u16>>,                           // 8-bit frame counters per word
    wcount: Vec<u64>,
    kind: Vec<u32>,                                 // each word's kind (a word id); kn = none yet
    kn: u32,
    ksize: HashMap<u32, u32>,                      // members per kind
    fdf: Vec<u32>,                                  // per frame feature: how many words' frame codes hold it
    fseen: u32,
    qw: Vec<i16>,                                   // the query learner: QD x NF signed 16-bit weights (dense)
    qs: SparseW,                                    // ... or the same weights sparse, 8-bit (--sit-qsparse 1)
    qr: RowW,                                       // ... or per feature, (bit, weight) lists (--sit-qsparse 2)
    qset_memo: std::cell::RefCell<(Vec<u32>, Vec<u8>)>,
    idx_only: bool,                                 // reading without a prediction (observe_only)        // the last query's bit counts (built once per query)
    qh: Vec<i8>,                                    // ... or superposed in one vector of 8-bit cells (--sit-qsparse 3)
    qform: u64,                                     // the current question form (--sit-qslots)
    heads_tab: HashMap<(u32, u32, u32), (u32, u64, u32, u32)>, // --sit-heads: (entity, path or class, kind) -> (value, time, class, index in heads_of)
    heads_of: HashMap<u32, Vec<(u32, u32, u32, u64, u32)>>, // entity -> its heads (path or class, kind, value, time, class), a copy kept in step
    bufp: Vec<u32>,                                 // the exact path of each buffered binding (aligned with buf)
    ctext: HashMap<u64, (Vec<u64>, u32)>,           // --sit-ranges: clause text -> (every start, ascending; first word)
    opener_pos: HashMap<u32, Vec<u64>>,             // first word -> start positions of its clauses (ascending)
    first_text: HashMap<u32, u64>,                  // --sit-epoch: opener type -> the text of its first clause
    epoch_at: HashMap<u32, u64>,                    // --sit-epoch: opener type -> where its current epoch began
    posts: HashMap<(u32, u32, u32), Vec<(u64, u32)>>, // head key -> its history (time, value), ascending
    rcache: std::cell::RefCell<HashMap<(u32, u32, u32), Vec<(u32, i128)>>>, // --sit-fast: slot reads this word
    members: HashMap<u32, Vec<u32>>,                // --sit-fast: kind -> its seen words
    bg_total: i128,                                 // --sit-fast: the background summed over seen words
    pub stat: std::cell::RefCell<[u64; 6]>,         // counters: reads computed, read cache hits, candidates, members scanned, words, query words
    qre: Vec<i8>,                                   // the question slots' own ternary phasor store (an area of its own)
    qim: Vec<i8>,
    qroute: Vec<(usize, u32)>,                      // each route's (cue position, cue word's kind), aligned with qcands
    qsrc: Vec<(u8, u32)>,                           // each route's source (0 slot, 1 head, 2 range) and path key
    key_credit: HashMap<u32, u32>,                  // --sit-lt-credit: path key -> questions answered through it
    key_writes: HashMap<u32, u32>,                  // --sit-lt-credit: path key -> bindings stored for it
    qbits: Vec<u32>,                                // the query code of the current prediction
    qcands: Vec<(Vec<u32>, u32, i64)>,             // its routes: (features, value, score), best first
    ring: Vec<(Vec<u32>, Vec<(Vec<u32>, u32)>, u32)>, // replayed mistakes: (query, routes (features, value), answer)
    ring_at: usize,
    win: Vec<u32>,                                  // the lookback window: the last `lookback` words
    wcands: Vec<(Vec<u32>, u32, i64)>,             // the window's routes for the current prediction
    arb: HashMap<(u64, u32), [u16; 4]>,             // per (question form, slot margin bucket): slot right, slot n, window right, window n
    arb_key: (u64, u32),
    conf_s: HashMap<(u64, u32), [u16; 2]>,          // curiosity: per (form, slot margin): slot right, n
    conf_c: HashMap<(u64, u32), [u16; 3]>,          // per (form, both margins, agree): slot right, window right, n
    cur_ks: (u64, u32),
    cur_kc: Option<(u64, u32)>,
    win_top: Option<u32>,
    practiced: bool,
    pub cur_asked: u64,                             // predictions where memory was unsure and the window was read
    pub cur_window: u64,                            // ... answered from the window
    pub cur_unknown: u64,                           // ... answered "unknown"
    pub cur_q: [u64; 4],                            // at questions: unsure, answered from the window, unknown, all
    slot_top: Option<u32>,
    pub used_window: u64,                           // predictions answered from the window
    pub used_window_q: u64,                         // ... of them right after a question boundary
    pub questions_seen: u64,
    qhops: Vec<Route>,                              // each route's hops, aligned with qcands
    qranks: Vec<(u32, u32)>,                        // each route's recency ranks within kind, aligned with qcands
    after_q: bool,                                  // the current prediction follows a learned question boundary
    sq: Vec<OpenQuery>,                             // the open queries
    sq_cand: HashMap<((u32, u32), (u32, u32)), u32>, // two-hop patterns that answered, counted
    sq_in: HashMap<((u32, u32), (u32, u32)), (u32, u32)>, // chains the input proposed: (count, the last subject)
    qring: Vec<Vec<u32>>,                           // the question codes seen lately (the higher level)
    qring_at: usize,
    sq_ptr: HashMap<(u32, u32), u32>,               // (query id, entity x) -> the intermediate m it points to
    sq_rev: HashMap<(u32, u32), Vec<u32>>,          // (query id, m) -> the entities pointing to m
    sq_next: u32,
    sq_pid: HashMap<((u32, u32), (u32, u32)), u32>, // a pattern's id, kept across closes (its derived class and weights)
    kmerge: HashMap<(u32, u32), u32>,               // slow merges: proposals in a row per pair of old kinds
    kind_area: crate::area::Area,                   // --sit-area-kinds: the matchers that name the kinds
    kpres: Vec<u64>,
    kmatch: Vec<u32>,                               // --sit-area-kinds: the matcher that won each word last                                // --sit-area-kinds: a word's count when last presented
    pub sq_work: u64,                               // maintenance reads and writes
    pub sq_open_sum: u64,                           // open queries summed over words (for the mean)
    pub sq_peak: usize,                             // the most open at once
    pub sq_opened: u64,
    pub sq_closed: u64,
    mood: Vec<u8>,
    succ: HashMap<(u32, u32), u32>,                 // bigram counts (word, next word), for mood2
    skip_n: Vec<(u32, u32)>,
    last_subj: HashMap<u32, u32>,                   // --sit-anaphora: content kind -> its newest clause-initial word
    pub anaphora_resolved: u64,                        // --sit-skip: per word (occurrences, occurrences that were skippable)
    succ_top: HashMap<u32, (u32, u32)>,             // per word: its most frequent successor and that count
    buf: Vec<(u32, u32, u8, u32)>,                  // buffered bindings: (entity, class, direction, value)
    stamp: HashMap<(u32, u32, u32), u64>,           // last write time per slot (entity, class, kind)
    occ: HashMap<(u32, u32), Vec<u32>>,             // live kinds per (entity, class): the slots' valid bits
    dcl: HashMap<(u32, u8), Vec<u32>>,              // classes written per (entity, direction)
    hop: HashMap<(u64, usize, u8), HashMap<(u32, u32), u32>>,
    g2: HashMap<((u32, u32), (u32, u32)), u32>,       // two-hop pairs counted over all contexts
    comp: HashMap<((u32, u32), (u32, u32)), u32>,     // composite classes, numbered from COMP
    miss: HashMap<(u64, usize, Route), u32>,          // NoGo: times a route was proposed and wrong
    routes: Vec<(u64, usize, Route, u32)>,            // the last prediction's routes: (context, cue, route, value)
    halt: HashMap<(u64, u32, u8), u8>,
    pub committed: u64,
    pub dropped: u64,
}

const DRE: [i8; 4] = [1, 0, -1, 0];
const FB: usize = 512;
const QD: usize = 4 * FB + 768; // query bits: 4 positions x frame code, 4 positions x 64 identity bits, 2 x 256 context bits
const NF: usize = 4096;         // route features (hashed)
/// A route: one slot (class, kind), or two (the second with class != u32::MAX).
type Route = (u32, u32, u32, u32);
const COMP: u32 = 1 << 24; // composite class ids start here, apart from the pooled classes
const DER: u32 = 1 << 25; // open queries' derived slots: class DER + query id
const ANY: u32 = u32::MAX - 1; // an open query's hop of any relation: the entity's newest value of that kind

/// An open query: a two-hop pattern kept answered for every entity, with its usefulness.
#[derive(Clone, Debug)]
pub struct OpenQuery {
    pub id: u32,
    pub p1: (u32, u32),
    pub p2: (u32, u32),
    pub util: i32,
    pub credits: u64,
    pub work: u64,
    pub opened_at: u64,
}
const DIM: [i8; 4] = [0, 1, 0, -1];

impl Situation {
    pub fn new(cfg: SitCfg, nw: usize) -> Self {
        let mut mc = MeaningCfg::default();
        mc.d = cfg.mdim;
        mc.win = cfg.mwin;
        mc.skip = 0;
        mc.seed = 21;
        let mut mn = Meaning::new(mc, nw);
        mn.sense_mix = cfg.sense_mix;
        let sd = cfg.seed.wrapping_mul(0x9E37_79B9);
        Situation {
            nw,
            re: vec![0; cfg.d],
            im: vec![0; cfg.d],
            bg: vec![0; nw + 1],
            ctx_off: (0..cfg.k).map(|i| (hash(91 + sd, i as u64) % cfg.d as u64) as u32).collect(),
            ctx_ph: (0..cfg.k).map(|i| (hash(92 + sd, i as u64) & 3) as u8).collect(),
            seg_draw: HashMap::default(),
            seg_hits: HashMap::default(),
            seg_n: HashMap::default(),
            boundary: vec![false; nw],
            mn,
            path_count: HashMap::default(),
            path_class: HashMap::default(),
            protos: vec![],
            proto_by_bit: vec![vec![]; cfg.mdim],
            lsh_bucket: HashMap::default(),
            vband: HashMap::default(),
            vcodes: vec![],
            slot_log: HashMap::default(),
            recent_slots: HashMap::default(),
            path_sketch: if cfg.lt_min > 0 { vec![0; 4 << 16] } else { vec![] },
            bnd_peak: HashMap::default(),
            path_seen: if cfg.path_min > 0 { vec![0; 4 << 16] } else { vec![] },
            rare_class: [u32::MAX; 2],
            pscratch: vec![],
            class_count: vec![],
            senses: HashMap::default(),
            c1: HashMap::default(),
            c2: HashMap::default(),
            hist: vec![],
            clause: vec![],
            seen: vec![false; nw],
            words: vec![],
            t: 0,
            last: vec![],
            learning: true,
            pool_pm: (cfg.pool * 1000.0).round() as u64,
            frame: vec![vec![0; FB]; nw],
            wcount: vec![0; nw],
            kind: vec![nw as u32; nw + 1],
            kn: nw as u32,
            ksize: HashMap::default(),
            fdf: vec![0; FB],
            fseen: 0,
            qw: if cfg.query && cfg.qsparse == 0 { vec![0; QD * NF] } else { vec![] },
            qs: SparseW::new(),
            qr: RowW::new(if cfg.query && cfg.qsparse == 2 { NF } else { 0 }),
            qset_memo: std::cell::RefCell::new((vec![], vec![0u8; QD])),
            idx_only: false,
            qh: if cfg.query && cfg.qsparse == 3 { vec![0; 1 << cfg.qholo] } else { vec![] },
            qform: 0,
            heads_tab: HashMap::default(),
            bufp: vec![],
            ctext: HashMap::default(),
            opener_pos: HashMap::default(),
            first_text: HashMap::default(),
            epoch_at: HashMap::default(),
            posts: HashMap::default(),
            rcache: std::cell::RefCell::new(HashMap::default()),
            members: HashMap::default(),
            bg_total: 0,
            stat: std::cell::RefCell::new([0; 6]),
            heads_of: HashMap::default(),
            qre: if cfg.qslots { vec![0; cfg.d] } else { vec![] },
            qim: if cfg.qslots { vec![0; cfg.d] } else { vec![] },
            qroute: vec![],
            qsrc: vec![],
            key_credit: HashMap::default(),
            key_writes: HashMap::default(),
            qbits: vec![],
            qcands: vec![],
            ring: vec![],
            ring_at: 0,
            win: vec![],
            wcands: vec![],
            arb: HashMap::default(),
            arb_key: (0, 0),
            conf_s: HashMap::default(),
            conf_c: HashMap::default(),
            cur_ks: (0, 0),
            cur_kc: None,
            win_top: None,
            practiced: false,
            cur_asked: 0,
            cur_window: 0,
            cur_unknown: 0,
            cur_q: [0; 4],
            slot_top: None,
            used_window: 0,
            used_window_q: 0,
            questions_seen: 0,
            qhops: vec![],
            qranks: vec![],
            after_q: false,
            sq: vec![],
            sq_cand: HashMap::default(),
            sq_in: HashMap::default(),
            qring: vec![],
            qring_at: 0,
            sq_ptr: HashMap::default(),
            sq_rev: HashMap::default(),
            sq_next: 0,
            sq_pid: HashMap::default(),
            kmerge: HashMap::default(),
            kind_area: {
                let mut a = crate::area::Area::new(2);
                a.anti = cfg.area_anti;
                a.sticky = cfg.area_sticky;
                a
            },
            kpres: vec![0; nw],
            kmatch: vec![u32::MAX; nw],
            sq_work: 0,
            sq_open_sum: 0,
            sq_peak: 0,
            sq_opened: 0,
            sq_closed: 0,
            mood: vec![0; nw],
            succ: HashMap::default(),
            skip_n: vec![(0, 0); nw],
            last_subj: HashMap::default(),
            anaphora_resolved: 0,
            succ_top: HashMap::default(),
            buf: vec![],
            stamp: HashMap::default(),
            occ: HashMap::default(),
            dcl: HashMap::default(),
            hop: HashMap::default(),
            g2: HashMap::default(),
            comp: HashMap::default(),
            miss: HashMap::default(),
            routes: vec![],
            halt: HashMap::default(),
            committed: 0,
            dropped: 0,
            cfg,
        }
    }

    /// Diagnostics: the overlap (shared bits / k) of word w's sense in two contexts, and of its plain code with each.
    pub fn sense_overlap(&mut self, w: u32, c1: &[u32], c2: &[u32]) -> (f64, f64, f64) {
        let (a, b) = if self.cfg.nsenses > 0 {
            (self.sense_code(w, c1, false), self.sense_code(w, c2, false))
        } else {
            (self.mn.sense(w, c1), self.mn.sense(w, c2))
        };
        let p = self.mn.code_or_ident(w);
        let ov = |x: &[u32], y: &[u32]| x.iter().filter(|b| y.contains(b)).count() as f64 / x.len().max(1) as f64;
        (ov(&a, &b), ov(&a, &p), ov(&b, &p))
    }
    pub fn n_senses(&self, w: u32) -> usize {
        self.senses.get(&w).map(|v| v.len()).unwrap_or(0)
    }
    pub fn boundaries(&self) -> Vec<u32> {
        (0..self.nw as u32).filter(|&w| self.boundary[w as usize]).collect()
    }
    pub fn n_classes(&self) -> usize {
        self.protos.len()
    }
    pub fn n_paths(&self) -> usize {
        self.path_count.len()
    }
    /// The classes with the most uses, each with its most used paths (dir-tagged: first element 0 forward, 1 back).
    pub fn classes(&self, top: usize) -> Vec<Vec<Vec<u32>>> {
        let mut by: HashMap<u32, Vec<(u64, Vec<u32>)>> = HashMap::default();
        for (p, &c) in &self.path_count {
            if let Some(&k) = self.path_class.get(p) {
                by.entry(k).or_default().push((c, p.clone()));
            }
        }
        let mut v: Vec<(u64, Vec<Vec<u32>>)> = by
            .into_values()
            .map(|mut m| {
                m.sort_by(|a, b| b.0.cmp(&a.0));
                (m.iter().map(|x| x.0).sum(), m.into_iter().take(6).map(|x| x.1).collect())
            })
            .collect();
        v.sort_by(|a, b| b.0.cmp(&a.0));
        v.into_iter().filter(|x| x.1.len() > 1).take(top).map(|x| x.1).collect()
    }

    // ---- segmentation ----
    fn seg_update(&mut self, x: u32, u: u32, c: u32) {
        let d = self.seg_draw.get(&u).copied();
        let n = {
            let n = self.seg_n.entry(u).or_insert(0);
            *n += 1;
            *n
        };
        if let Some((dx, dc)) = d {
            let h = self.seg_hits.entry(u).or_insert([0; 3]);
            h[0] += (dx == x) as u64;
            h[1] += (dc == c) as u64;
            h[2] += (dx == x && dc == c) as u64;
        }
        if d.is_none() || hash(self.t, u as u64 + 5151) % 16 == 0 {
            self.seg_draw.insert(u, (x, c));
        }
        if n >= 64 && n % 32 == 0 {
            let [hp, hn, hj] = *self.seg_hits.get(&u).unwrap_or(&[0; 3]);
            let (lo, hi) = (hp.min(hn) as u128, hp.max(hn) as u128);
            let (n, hj) = (n as u128, hj as u128);
            let nb = 20 * hi <= 19 * n && lo * lo * lo * hi * hi > n * n * hj * hj * hj;
            if nb != self.boundary[u as usize] && std::env::var("SIT_DEBUG_BND").is_ok() {
                eprintln!("BND {} word {} -> {}", self.t, u, nb);
            }
            if !self.cfg.bnd2 {
                self.boundary[u as usize] = nb;
            }
        }
        if n >= 4096 {
            self.seg_n.insert(u, n / 2);
            if let Some(h) = self.seg_hits.get_mut(&u) {
                h.iter_mut().for_each(|x| *x /= 2);
            }
        }
    }

    // ---- pooling ----
    /// A path's code: the OR of its words' meaning codes, each rotated by its place in the path.
    fn path_code(&mut self, path: &[u32]) -> Vec<u32> {
        let d = self.cfg.mdim;
        let mut out: Vec<u32> = vec![];
        // with senses, the key is [dir, s, w, path...]: each path word's code is its sense in the clause (the
        // context: the two ends and the other path words)
        // with senses the key is [dir, n, path..., the sense each path word took]
        let (words, sids): (Vec<u32>, Vec<u32>) = if self.cfg.sense {
            let n = path[1] as usize;
            (path[2..2 + n].to_vec(), path[2 + n..].to_vec())
        } else {
            (path[1..].to_vec(), vec![])
        };
        for (i, &w) in words.iter().enumerate() {
            let c = match sids.get(i) {
                Some(&k) if k != u32::MAX => self.sense_mask(w, k as usize),
                _ if self.cfg.fpool => self.frame_bits(w),
                _ => self.mn.code_or_ident(w),
            };
            out.extend(c.iter().map(|&b| ((b as usize + i * 7919) % d) as u32));
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// A word's sense in a context: a sparse subset of its own meaning code. The context (the identity bits of
    /// the clause words around it) picks the word's learned sense whose context prototype it matches best (mean
    /// count over the context's bits, relative to the sense's uses), or starts one (up to `nsenses`); when
    /// learning, that sense's prototype counts the context. The sense's code is the half of the word's meaning
    /// code most counted in its prototype.
    fn sense_code(&mut self, w: u32, ctx: &[u32], learn: bool) -> Vec<u32> {
        match self.sense_pick(w, ctx, learn) {
            Some(i) => self.sense_mask(w, i),
            None => self.mn.code_or_ident(w),
        }
    }

    /// Which of w's learned senses the context picks (learning: the sense counts the context, or a new one
    /// starts); None while there is no context or no sense yet.
    fn sense_pick(&mut self, w: u32, ctx: &[u32], learn: bool) -> Option<usize> {
        let mut cb: Vec<u32> = ctx.iter().flat_map(|&c| self.mn.ident(c).to_vec()).collect();
        cb.sort_unstable();
        cb.dedup();
        if cb.is_empty() {
            return None;
        }
        let d = self.cfg.mdim;
        let ss = self.senses.entry(w).or_default();
        let mut best = (0.0f64, usize::MAX);
        for (i, sn) in ss.iter().enumerate() {
            let m = cb.iter().map(|&b| sn.counts[b as usize] as f64).sum::<f64>() / (sn.n.max(1) as f64 * cb.len() as f64);
            if m > best.0 {
                best = (m, i);
            }
        }
        let i = if best.1 == usize::MAX || (best.0 < self.cfg.sense_new && ss.len() < self.cfg.nsenses) {
            if !learn {
                return if best.1 == usize::MAX { None } else { Some(best.1) };
            }
            ss.push(Sense { counts: vec![0; d], n: 0 });
            ss.len() - 1
        } else {
            best.1
        };
        if learn {
            let sn = &mut ss[i];
            sn.n += 1;
            for &b in &cb {
                sn.counts[b as usize] += 1;
            }
        }
        Some(i)
    }

    /// Sense i of w: the half of w's meaning code most counted in that sense's context prototype (the plain code
    /// while the sense is young).
    fn sense_mask(&mut self, w: u32, i: usize) -> Vec<u32> {
        let plain = self.mn.code_or_ident(w);
        let Some(sn) = self.senses.get(&w).and_then(|v| v.get(i)) else { return plain };
        if sn.n < 4 || plain.is_empty() {
            return plain;
        }
        let mut v: Vec<(u32, u32)> = plain.iter().map(|&b| (sn.counts[b as usize], b)).collect();
        v.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let keep = (plain.len() + 1) / 2;
        let mut out: Vec<u32> = v.into_iter().take(keep).filter(|x| x.0 > 0).map(|x| x.1).collect();
        if out.is_empty() {
            return plain;
        }
        out.sort_unstable();
        out
    }

    /// The relation class of a dir-tagged path (an empty path, adjacency, is its own class per direction).
    /// --sit-lsh: a path's band keys (MinHash bands over its code; an empty code is one key of its own).
    fn lsh_keys(&self, path: &[u32], code: &[u32], dir: u8) -> Vec<u64> {
        if code.is_empty() || self.pool_pm == 0 {
            return vec![hash(0xB0A7, path.iter().fold(0x51, |h, &x| hash(h, x as u64)))];
        }
        (0..self.cfg.lsh_bands)
            .map(|b| {
                (0..self.cfg.lsh_rows).fold(hash(dir as u64 + 0x15A, b as u64), |h, r| {
                    let seed = hash(b as u64 * 977 + r as u64, 0x3E1);
                    let m = code.iter().map(|&x| hash(x as u64, seed)).min().unwrap_or(0);
                    hash(h, m)
                })
            })
            .collect()
    }
    /// --sit-path-min: the class of a path not (yet) recurring: a stored class its bands hit, else the shared
    /// class of rare paths; nothing about the path is stored.
    fn class_lookup(&mut self, path: &[u32]) -> u32 {
        if let Some(&c) = self.path_class.get(path) {
            return c;
        }
        let dir = path[0] as u8;
        let bare = if self.cfg.sense { 2 } else { 1 };
        let code = if path.len() <= bare { vec![] } else { self.path_code(path) };
        let keys = self.lsh_keys(path, &code, dir);
        if let Some(c) = keys.iter().find_map(|k| self.lsh_bucket.get(k).copied()) {
            return c;
        }
        let d = (dir & 1) as usize;
        if self.rare_class[d] == u32::MAX || self.rare_class[d] as usize >= self.protos.len() {
            self.protos.push(Proto { counts: vec![], bits: vec![], dir, n: 0 });
            self.class_count.push(0);
            self.rare_class[d] = self.protos.len() as u32 - 1;
        }
        self.rare_class[d]
    }
    fn class_of(&mut self, path: &[u32]) -> u32 {
        if let Some(&c) = self.path_class.get(path) {
            return c;
        }
        let dir = path[0] as u8;
        let bare = if self.cfg.sense { 2 } else { 1 };
        let code = if path.len() <= bare { vec![] } else { self.path_code(path) };
        if self.cfg.lsh {
            let keys = self.lsh_keys(path, &code, dir);
            let c = match keys.iter().find_map(|k| self.lsh_bucket.get(k).copied()) {
                Some(c) => c,
                None => {
                    self.protos.push(Proto { counts: vec![], bits: vec![], dir, n: 0 });
                    self.class_count.push(0);
                    self.protos.len() as u32 - 1
                }
            };
            for k in keys {
                self.lsh_bucket.entry(k).or_insert(c);
            }
            self.protos[c as usize].n += 1;
            self.path_class.insert(path.to_vec(), c);
            return c;
        }
        // the best overlap (shared bits over the larger size), as a fraction (shared, size), compared by
        // cross-multiplication: no floats, no division
        let mut best = (0u64, 1u64, usize::MAX);
        if !code.is_empty() && self.pool_pm > 0 {
            // only classes sharing a bit can pass the overlap test: count shared bits through the inverted index,
            // then take them in class order (the same choice as a scan over all classes)
            if self.pscratch.len() < self.protos.len() {
                self.pscratch.resize(self.protos.len(), 0);
            }
            let mut touched: Vec<u32> = vec![];
            for &b in &code {
                for &pi in &self.proto_by_bit[b as usize] {
                    if self.pscratch[pi as usize] == 0 {
                        touched.push(pi);
                    }
                    self.pscratch[pi as usize] += 1;
                }
            }
            touched.sort_unstable();
            for &pi in &touched {
                let i = pi as usize;
                let sh = std::mem::take(&mut self.pscratch[i]) as u64;
                let p = &self.protos[i];
                if p.dir != dir || p.bits.is_empty() {
                    continue;
                }
                let den = code.len().max(p.bits.len()) as u64;
                if sh * 1000 >= self.pool_pm * den && (best.2 == usize::MAX || sh * best.1 > best.0 * den) {
                    best = (sh, den, i);
                }
            }
        }
        let c = if best.2 == usize::MAX {
            self.protos.push(Proto { counts: vec![0; self.cfg.mdim], bits: code.clone(), dir, n: 0 });
            let id = self.protos.len() as u32 - 1;
            for &b in &code {
                self.proto_by_bit[b as usize].push(id);
            }
            self.class_count.push(0);
            self.protos.len() - 1
        } else {
            best.2
        };
        let p = &mut self.protos[c];
        p.n += 1;
        for &b in &code {
            p.counts[b as usize] += 1;
        }
        if p.n % 8 == 1 && !code.is_empty() {
            // the prototype: the code-sized set of its most counted bits
            let k = code.len();
            let mut idx: Vec<u32> = (0..p.counts.len() as u32).filter(|&i| p.counts[i as usize] > 0).collect();
            if idx.len() > k {
                idx.select_nth_unstable_by(k - 1, |&x, &y| p.counts[y as usize].cmp(&p.counts[x as usize]));
                idx.truncate(k);
            }
            idx.sort_unstable();
            let old = std::mem::replace(&mut p.bits, idx.clone());
            for b in old {
                let l = &mut self.proto_by_bit[b as usize];
                if let Some(at) = l.iter().position(|&x| x == c as u32) {
                    l.swap_remove(at);
                }
            }
            for b in idx {
                self.proto_by_bit[b as usize].push(c as u32);
            }
        }
        self.path_class.insert(path.to_vec(), c as u32);
        c as u32
    }

    /// Re-assign every path, most used first, as the meanings have matured; classes keep their numbers where
    /// their most used path lands again.
    fn recluster(&mut self) {
        if self.cfg.fpool {
            self.recount_fdf();
        }
        if self.cfg.path_forget > 0 {
            let n = self.cfg.path_forget;
            self.path_count.retain(|_, c| *c >= n);
        }
        let mut paths: Vec<(u64, Vec<u32>)> = self.path_count.iter().map(|(p, &c)| (c, p.clone())).collect();
        paths.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        // each old class's most used path, to carry its counts over to wherever that path lands now
        let mut rep: HashMap<u32, Vec<u32>> = HashMap::default();
        for (_, p) in &paths {
            if let Some(&c) = self.path_class.get(p) {
                rep.entry(c).or_insert_with(|| p.clone());
            }
        }
        self.protos.clear();
        self.lsh_bucket.clear();
        self.rare_class = [u32::MAX; 2];
        for l in self.proto_by_bit.iter_mut() {
            l.clear();
        }
        self.class_count.clear();
        self.path_class.clear();
        self.learning = false;
        for (c, p) in paths {
            let k = self.class_of(&p) as usize;
            self.class_count[k] += c;
        }
        self.learning = true;
        let raw: HashMap<u32, u32> = rep.into_iter().map(|(old, p)| (old, self.path_class[&p])).collect();
        // keep class numbers stable: a new class takes the number of the (lowest) old class whose most used path
        // landed in it, so slots written before the re-assignment stay addressed; the rest get unused numbers
        let nnew = self.protos.len();
        let mut fin: Vec<Option<u32>> = vec![None; nnew];
        let mut olds: Vec<(u32, u32)> = raw.iter().map(|(&o, &k)| (o, k)).collect();
        olds.sort_unstable();
        let mut used: std::collections::HashSet<u32> = std::collections::HashSet::new();
        for &(o, k) in &olds {
            if fin[k as usize].is_none() && !used.contains(&o) {
                fin[k as usize] = Some(o);
                used.insert(o);
            }
        }
        let mut next = 0u32;
        for f in fin.iter_mut() {
            if f.is_none() {
                while used.contains(&next) {
                    next += 1;
                }
                *f = Some(next);
                used.insert(next);
            }
        }
        let fin: Vec<u32> = fin.into_iter().map(|x| x.unwrap()).collect();
        let size = fin.iter().copied().max().map_or(0, |m| m as usize + 1);
        let mut protos: Vec<Proto> = (0..size).map(|_| Proto { counts: vec![], bits: vec![], dir: 255, n: 0 }).collect();
        let mut counts = vec![0u64; size];
        for (k, p) in std::mem::take(&mut self.protos).into_iter().enumerate() {
            counts[fin[k] as usize] = self.class_count[k];
            protos[fin[k] as usize] = p;
        }
        self.protos = protos;
        self.class_count = counts;
        for l in self.proto_by_bit.iter_mut() {
            l.clear();
        }
        for (i, p) in self.protos.iter().enumerate() {
            for &b in &p.bits {
                self.proto_by_bit[b as usize].push(i as u32);
            }
        }
        for v in self.path_class.values_mut() {
            *v = fin[*v as usize];
        }
        let map: HashMap<u32, u32> = raw.into_iter().map(|(o, k)| (o, fin[k as usize])).collect();
        let c1 = std::mem::take(&mut self.c1);
        for (k, m) in c1 {
            let e = self.c1.entry(k).or_default();
            for ((c, kd), n) in m {
                if let Some(&c2) = map.get(&c) {
                    *e.entry((c2, kd)).or_insert(0) += n;
                }
            }
        }
        let c2 = std::mem::take(&mut self.c2);
        for (k, m) in c2 {
            let e = self.c2.entry(k).or_default();
            for (((a, ka), (b, kb)), n) in m {
                if let (Some(&a2), Some(&b2)) = (map.get(&a), map.get(&b)) {
                    *e.entry(((a2, ka), (b2, kb))).or_insert(0) += n;
                }
            }
        }
        let miss = std::mem::take(&mut self.miss);
        for ((cx, j, (a, ka, b, kb)), n) in miss {
            let b2 = if b == u32::MAX { Some(b) } else { map.get(&b).copied() };
            if let (Some(&a2), Some(b2)) = (map.get(&a), b2) {
                *self.miss.entry((cx, j, (a2, ka, b2, kb))).or_insert(0) += n;
            }
        }
        let mut closing = vec![];
        for o in self.sq.iter_mut() {
            let mp = |c: u32| if c == ANY { Some(ANY) } else { map.get(&c).copied() };
            match (mp(o.p1.0), mp(o.p2.0)) {
                (Some(a), Some(b)) => {
                    o.p1.0 = a;
                    o.p2.0 = b;
                }
                _ => closing.push(o.id),
            }
        }
        for id in closing {
            self.sq_close(id);
        }
        // the patterns' ids follow their classes
        let mp = |c: u32| if c == ANY { Some(ANY) } else { map.get(&c).copied() };
        let pid = std::mem::take(&mut self.sq_pid);
        for (((a, ka), (b, kb)), id) in pid {
            if let (Some(a2), Some(b2)) = (mp(a), mp(b)) {
                self.sq_pid.entry(((a2, ka), (b2, kb))).or_insert(id);
            }
        }
        self.sq_cand.retain(|p, _| p.0 .0 == ANY);
        self.sq_in.clear();
        let hop = std::mem::take(&mut self.hop);
        for (k, m) in hop {
            let e = self.hop.entry(k).or_default();
            for ((c, kd), n) in m {
                if let Some(&c2) = map.get(&c) {
                    *e.entry((c2, kd)).or_insert(0) += n;
                }
            }
        }
        let g2 = std::mem::take(&mut self.g2);
        for (((a, ka), (b, kb)), n) in g2 {
            if let (Some(&a2), Some(&b2)) = (map.get(&a), map.get(&b)) {
                *self.g2.entry(((a2, ka), (b2, kb))).or_insert(0) += n;
            }
        }
        let comp = std::mem::take(&mut self.comp);
        for (((a, ka), (b, kb)), id) in comp {
            if let (Some(&a2), Some(&b2)) = (map.get(&a), map.get(&b)) {
                self.comp.entry(((a2, ka), (b2, kb))).or_insert(id);
            }
        }
        // the valid bits and write times follow their classes (numbers mostly unchanged); classes merged into
        // another number lose their slots
        let occ = std::mem::take(&mut self.occ);
        for ((e, c), ks) in occ {
            if c >= COMP || map.get(&c) == Some(&c) {
                self.occ.insert((e, c), ks);
            }
        }
        if self.cfg.heads {
            // the heads follow their classes (the newer entry wins where two classes merge)
            let hd = std::mem::take(&mut self.heads_tab);
            for ((e, key, k), (v, t, c, _)) in hd {
                // a path key stays (whatever happens to its class); a class key follows its class
                let c2 = match map.get(&c) {
                    Some(&c2) => c2,
                    None if key & 0x8000_0000 != 0 => c,
                    None => continue,
                };
                let key2 = if key & 0x8000_0000 != 0 { key } else { c2 };
                let ent = self.heads_tab.entry((e, key2, k)).or_insert((v, t, c2, u32::MAX));
                if t > ent.1 {
                    *ent = (v, t, c2, u32::MAX);
                }
            }
            self.heads_of.clear();
            for (&(e, key, k), ent) in self.heads_tab.iter_mut() {
                let l = self.heads_of.entry(e).or_default();
                ent.3 = l.len() as u32;
                l.push((key, k, ent.0, ent.1, ent.2));
            }
        }
        let stamp = std::mem::take(&mut self.stamp);
        for ((e, c, k), t) in stamp {
            if c >= COMP || map.get(&c) == Some(&c) {
                self.stamp.insert((e, c, k), t);
            }
        }
        for cs in self.dcl.values_mut() {
            cs.retain(|c| *c >= COMP || map.get(c) == Some(c));
        }
        if self.cfg.kinds {
            if self.cfg.area_kinds {
                self.area_kinds();
            } else {
                self.recluster_kinds();
            }
        }
    }

    /// bnd2: boundaries as the words open on both sides (see the config field).
    fn open_boundaries(&mut self) {
        let mut sc: HashMap<u32, Vec<u32>> = HashMap::default();
        let mut pc: HashMap<u32, Vec<u32>> = HashMap::default();
        for (&(a, b), &c) in &self.succ {
            sc.entry(a).or_default().push(c);
            pc.entry(b).or_default().push(c);
        }
        // the number of the largest counts that cover 90% of the total (integer compare)
        let k90 = |v: &mut Vec<u32>| -> u32 {
            v.sort_unstable_by(|a, b| b.cmp(a));
            let tot: u64 = v.iter().map(|&x| x as u64).sum();
            let mut s = 0u64;
            for (i, &x) in v.iter().enumerate() {
                s += x as u64;
                if 10 * s >= 9 * tot {
                    return i as u32 + 1;
                }
            }
            v.len() as u32
        };
        let mut open: Vec<(u32, u32)> = vec![];
        for (w, mut v) in sc {
            if let Some(mut p) = pc.remove(&w) {
                let (a, b) = (k90(&mut v), k90(&mut p));
                open.push((w, a.min(b)));
            }
        }
        let mx = open.iter().map(|x| x.1).max().unwrap_or(0);
        if mx < 4 {
            return; // too early to tell
        }
        for (w, m) in open {
            let nb = if self.cfg.bnd_hyst && self.boundary[w as usize] {
                let pk = self.bnd_peak.entry(w).or_insert(m);
                *pk = (*pk).max(m);
                4 * m >= *pk
            } else {
                2 * m >= mx
            };
            if nb != self.boundary[w as usize] && std::env::var("SIT_DEBUG_BND").is_ok() {
                eprintln!("BND2 {} word {} -> {} (open {} of max {})", self.t, w, nb, m, mx);
            }
            self.boundary[w as usize] = nb;
        }
    }

    // ---- kinds ----
    fn fb(a: u64, b: u64) -> usize {
        (hash(a, b) % FB as u64) as usize
    }
    fn frame_bump(&mut self, w: u32, f: usize) {
        let r = &mut self.frame[w as usize];
        r[f] += 1;
        if r[f] > 255 {
            r.iter_mut().for_each(|x| *x >>= 1); // 8-bit: halve together when one saturates
        }
    }
    /// A word's frame code as bit indices (an integer meaning code: its 8-bit frame counters at a quarter of their
    /// maximum or more); a few identity bits while it has no frames yet.
    fn frame_bits(&self, w: u32) -> Vec<u32> {
        let r = &self.frame[w as usize];
        let mx = *r.iter().max().unwrap_or(&0);
        if mx == 0 {
            return (0..4u64).map(|i| (hash(w as u64 + 77, i) % FB as u64) as u32).collect();
        }
        let th = (mx >> 2).max(1);
        // keep the distinctive features: those in at most an eighth of the words' codes (integer compare)
        (0..FB as u32).filter(|&b| r[b as usize] >= th && (self.fseen == 0 || self.fdf[b as usize] * 8 <= self.fseen)).collect()
    }
    /// Recount, per frame feature, how many words' frame codes hold it.
    fn recount_fdf(&mut self) {
        let mut df = vec![0u32; FB];
        let mut nseen = 0;
        for w in 0..self.nw as u32 {
            if self.wcount[w as usize] == 0 {
                continue;
            }
            nseen += 1;
            for (b, on) in self.frame_code(w).into_iter().enumerate() {
                if on {
                    df[b] += 1;
                }
            }
        }
        self.fdf = df;
        self.fseen = nseen;
    }
    /// The code the kinds are clustered on: frame features at max >> kshift or more.
    fn kind_code(&self, w: u32) -> [u64; FB / 64] {
        // packed: bit b of the code is frame feature b at max >> kshift or more (compared by popcount)
        let r = &self.frame[w as usize];
        let th = (*r.iter().max().unwrap_or(&0) >> self.cfg.kshift).max(1);
        let mut c = [0u64; FB / 64];
        let fold = self.cfg.kind_bits.clamp(1, FB) - 1; // a power of two: fold feature b onto bit b mod kind_bits
        for (b, &x) in r.iter().enumerate() {
            if x >= th {
                let b = b & fold;
                c[b >> 6] |= 1u64 << (b & 63);
            }
        }
        c
    }
    fn frame_code(&self, w: u32) -> Vec<bool> {
        let r = &self.frame[w as usize];
        let th = (*r.iter().max().unwrap_or(&0) >> 2).max(1);
        r.iter().map(|&x| x >= th).collect()
    }
    /// Leader clustering of the words by their frame codes (Jaccard >= 2/5, cross-multiplied), most frequent first;
    /// each new cluster takes the old kind name most of its members had, so slots and counts keep their meaning.
    fn recluster_kinds(&mut self) {
        let mut order: Vec<u32> = (0..self.nw as u32).filter(|&w| self.wcount[w as usize] > 0).collect();
        order.sort_by(|a, b| self.wcount[*b as usize].cmp(&self.wcount[*a as usize]).then(a.cmp(b)));
        let mut leaders: Vec<(u32, [u64; FB / 64], Vec<u32>)> = vec![];
        for &w in &order {
            let c = self.kind_code(w);
            let mut placed = false;
            for l in leaders.iter_mut() {
                let (mut inter, mut uni) = (0u64, 0u64);
                for (x, y) in c.iter().zip(&l.1) {
                    inter += (x & y).count_ones() as u64;
                    uni += (x | y).count_ones() as u64;
                }
                if uni > 0 && 5 * inter >= 2 * uni {
                    l.2.push(w);
                    placed = true;
                    break;
                }
            }
            if !placed {
                leaders.push((w, c, vec![w]));
            }
        }
        if self.cfg.kslow > 0 {
            // slow merges: a cluster joining two or more established old kinds (2+ members each in it) is split back
            // by old kind until the same merge has been proposed kslow reclusters in a row
            let mut proposed: Vec<(u32, u32)> = vec![];
            let mut out: Vec<(u32, [u64; FB / 64], Vec<u32>)> = vec![];
            for (lw, code, ms) in leaders.into_iter() {
                let mut by: HashMap<u32, Vec<u32>> = HashMap::default();
                for &w in &ms {
                    by.entry(self.kind[w as usize]).or_default().push(w);
                }
                let mut est: Vec<u32> = by.iter().filter(|(k, v)| **k != self.kn && v.len() >= 2).map(|(k, _)| *k).collect();
                est.sort_unstable();
                let mut ok = true;
                for i in 0..est.len() {
                    for j in i + 1..est.len() {
                        let pr = (est[i], est[j]);
                        proposed.push(pr);
                        if self.kmerge.get(&pr).copied().unwrap_or(0) + 1 < self.cfg.kslow {
                            ok = false;
                        }
                    }
                }
                if ok || est.len() < 2 {
                    out.push((lw, code, ms));
                } else {
                    // split back: each established old kind keeps its members; the rest stay with the first
                    let first = est[0];
                    for (k, mut v) in by.into_iter() {
                        if est.contains(&k) && k != first {
                            v.sort_unstable();
                            out.push((v[0], code, v));
                        }
                    }
                    let rest: Vec<u32> = ms.iter().copied().filter(|w| !est.contains(&self.kind[*w as usize]) || self.kind[*w as usize] == first).collect();
                    out.push((lw, code, rest));
                }
            }
            let mut km: HashMap<(u32, u32), u32> = HashMap::default();
            for pr in proposed {
                km.insert(pr, self.kmerge.get(&pr).copied().unwrap_or(0) + 1);
            }
            self.kmerge = km; // proposals not repeated this time start again from zero
            leaders = out;
        }
        leaders.sort_by(|a, b| b.2.len().cmp(&a.2.len()).then(a.0.cmp(&b.0)));
        let mut used: std::collections::HashSet<u32> = std::collections::HashSet::new(); // same choices as a list + contains, without the O(L^2)
        let mut next_free = 0u32;
        let mut newk = self.kind.clone();
        for (lw, _, ms) in &leaders {
            let mut votes: HashMap<u32, u32> = HashMap::default();
            for &w in ms {
                if self.kind[w as usize] != self.kn {
                    *votes.entry(self.kind[w as usize]).or_insert(0) += 1;
                }
            }
            let mut v: Vec<(u32, u32)> = votes.into_iter().collect();
            v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            let name = match v.iter().map(|x| x.0).find(|o| !used.contains(o)) {
                Some(o) => o,
                None if !used.contains(lw) => *lw,
                None => {
                    while used.contains(&next_free) {
                        next_free += 1;
                    }
                    next_free
                }
            };
            used.insert(name);
            for &w in ms {
                newk[w as usize] = name;
            }
        }
        if std::env::var("SIT_KDBG").is_ok() {
            let mut g: Vec<String> = leaders.iter().filter(|l| l.2.len() >= 2).map(|l| format!("{:?}", l.2)).collect();
            g.sort();
            eprintln!("KDBG {} kinds of 2+: {}", self.t, g.join(" "));
        }
        self.kind = newk;
        self.finish_kinds();
    }
    /// An area of matchers in place of leader clustering (--sit-area-kinds): the words whose context codes changed
    /// since the last presentation (all, the first time), most frequent first; a word's kind is the name of the
    /// matcher that wins it. The matchers persist and learn by bit swaps, so kinds keep their names by themselves.
    fn area_kinds(&mut self) {
        let mut order: Vec<u32> = (0..self.nw as u32).filter(|&w| self.wcount[w as usize] > 0 && self.wcount[w as usize] != self.kpres[w as usize]).collect();
        order.sort_by(|a, b| self.wcount[*b as usize].cmp(&self.wcount[*a as usize]).then(a.cmp(b)));
        for &w in &order {
            let c = self.kind_code(w);
            let salt = hash(self.t, w as u64);
            let (name, m) = self.kind_area.present(&c, w, salt, self.kmatch[w as usize]);
            self.kind[w as usize] = name;
            self.kmatch[w as usize] = m;
            self.kpres[w as usize] = self.wcount[w as usize];
        }
        self.finish_kinds();
    }
    fn finish_kinds(&mut self) {
        if self.fm(0) {
            self.members.clear();
            for &w in &self.words {
                self.members.entry(self.kind[w as usize]).or_default().push(w);
            }
            self.rcache.get_mut().clear();
        }
        self.ksize.clear();
        for w in 0..self.nw {
            if self.wcount[w] > 0 {
                *self.ksize.entry(self.kind[w]).or_insert(0) += 1;
            }
        }
    }
    /// The classes of e's live slots (of kind kd, if given).
    fn live_classes(&self, e: u32, kd: Option<u32>) -> Vec<u32> {
        let mut v: Vec<u32> = vec![];
        for dir in 0..4u8 {
            if let Some(cs) = self.dcl.get(&(e, dir)) {
                for &c in cs {
                    if !v.contains(&c) && kd.map_or(true, |k| self.occ.get(&(e, c)).map_or(false, |ks| ks.contains(&k))) {
                        v.push(c);
                    }
                }
            }
        }
        v
    }
    /// Whether w may act as a cue entity (with --sit-entity-cues: a word of a kind with at least 3 members).
    fn is_entity(&self, w: u32) -> bool {
        !self.cfg.entity_cues || self.ksize.get(&self.kind[w as usize]).copied().unwrap_or(0) >= 3
    }
    fn kk(&self, v: u32) -> u32 {
        if self.cfg.kinds { self.kind[v as usize] } else { self.kn }
    }
    /// The kinds, as groups of words (diagnostics).
    pub fn kind_groups(&self) -> Vec<Vec<u32>> {
        let mut by: HashMap<u32, Vec<u32>> = HashMap::default();
        for w in 0..self.nw as u32 {
            if self.wcount[w as usize] > 0 {
                by.entry(self.kind[w as usize]).or_default().push(w);
            }
        }
        let mut v: Vec<Vec<u32>> = by.into_values().collect();
        v.sort_by(|a, b| b.len().cmp(&a.len()).then(a[0].cmp(&b[0])));
        v
    }
    /// The kind area's crowding report (with --sit-area-kinds).
    pub fn kind_area_report(&self) -> String {
        self.kind_area.report()
    }
    /// The boundaries whose mood counter marks them as closing questions.
    pub fn question_boundaries(&self) -> Vec<u32> {
        (0..self.nw as u32).filter(|&w| self.boundary[w as usize] && self.mood[w as usize] >= 8).collect()
    }

    // ---- fast ternary memory ----
    /// The slot (entity e, class c, value kind kd): K elements and phases, offset by the context.
    fn key(&self, e: u32, c: u32, kd: u32) -> (Vec<u32>, Vec<u8>) {
        let (d, k) = (self.cfg.d as u64, self.cfg.k);
        let mut idx = Vec::with_capacity(k);
        let mut ph = Vec::with_capacity(k);
        let sc = if kd == self.kn { c as u64 } else { c as u64 * (self.nw as u64 + 1) + kd as u64 + (1 << 40) };
        let salt = self.cfg.seed.wrapping_mul(0x5851_F42D);
        for i in 0..k {
            let h = hash(e as u64 * 1_000_003 + sc + salt, i as u64);
            idx.push(((h % d) as u32 + self.ctx_off[i]) % d as u32);
            ph.push(((h >> 40) as u8 + self.ctx_ph[i]) & 3);
        }
        (idx, ph)
    }
    #[inline]
    fn vph(&self, v: u32, e: u32) -> u8 {
        if self.fm(1) {
            // a cheaper phase: two multiplies and a fold
            let h = (v as u64 ^ self.cfg.seed.wrapping_mul(0x1000_0001).wrapping_add(0x7777)).wrapping_mul(0x9E37_79B9_7F4A_7C15)
                ^ (e as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93);
            return ((h ^ (h >> 31)).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 62) as u8;
        }
        (hash(v as u64 + 0x7777 + self.cfg.seed.wrapping_mul(0x1000_0001), e as u64) & 3) as u8
    }
    /// --sit-vlsh: a value's phase at a key's i-th position, from its stored code.
    fn vc(&self, v: u32, i: usize) -> u8 {
        match self.vcodes.get(v as usize) {
            Some(&c) if i < 64 => ((c >> (2 * i)) & 3) as u8,
            _ => self.vcode(v, i),
        }
    }
    /// --sit-vlsh: a value's phase at a key's i-th position (its fixed code).
    fn vcode(&self, v: u32, i: usize) -> u8 {
        let h = (v as u64 ^ self.cfg.seed.wrapping_mul(0x1000_0001).wrapping_add(0x5A5A)).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ (i as u64 + 1).wrapping_mul(0xD6E8_FEB8_6659_FD93);
        ((h ^ (h >> 31)).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 62) as u8
    }
    /// --sit-vlsh: the key of band b for a code (its phases at the band's positions).
    fn vband_key(b: usize, ph: &[u8]) -> u64 {
        hash(b as u64 + 0xBA4D, ph.iter().fold(0u64, |h, &p| h << 2 | p as u64))
    }
    /// --sit-vlsh: file a newly seen value's code under its bands.
    fn vband_add(&mut self, v: u32) {
        let code: Vec<u8> = (0..self.cfg.k).map(|i| self.vcode(v, i)).collect();
        if self.vcodes.len() <= v as usize {
            self.vcodes.resize(v as usize + 1, 0);
        }
        self.vcodes[v as usize] = code.iter().take(64).enumerate().fold(0u128, |c, (i, &p)| c | (p as u128) << (2 * i));
        let w = self.cfg.vlsh_band;
        for b in 0..self.cfg.k / w {
            self.vband.entry(Self::vband_key(b, &code[b * w..b * w + w])).or_default().push(v);
        }
    }
    fn write(&mut self, e: u32, c: u32, kd: u32, v: u32) {
        let (idx, ph) = self.key(e, c, kd);
        let m = self.cfg.mag;
        for (i, (&x, &kp)) in idx.iter().zip(&ph).enumerate() {
            let p = ((kp + if self.cfg.vlsh { self.vcode(v, i) } else { self.vph(v, x) }) & 3) as usize;
            let x = x as usize;
            self.re[x] = (self.re[x] + m * DRE[p]).clamp(-1, 1);
            self.im[x] = (self.im[x] + m * DIM[p]).clamp(-1, 1);
        }
        self.bg[v as usize] += 1;
        if self.cfg.slotlog > 0 {
            let l = self.slot_log.entry((e, c, kd)).or_default();
            if let Some(at) = l.iter().position(|&x| x == v) {
                l.remove(at);
            }
            l.push(v);
            if l.len() > self.cfg.slotlog {
                l.remove(0);
            }
        }
        if self.fm(0) {
            if self.seen[v as usize] {
                self.bg_total += 1;
            }
            self.rcache.get_mut().clear();
        }
    }
    /// The best value in e's slot for class c (by excess over its background, by subtraction), and its excess.
    fn read_best(&self, e: u32, c: u32, kd: u32, excl: &[u32]) -> Option<(u32, i128)> {
        if self.fm(0) {
            // the slot's ranked values, read once per word (the background share over all the kind's words, so the
            // exclusions only filter the ranking)
            if let Some(r) = self.rcache.borrow().get(&(e, c, kd)) {
                self.stat.borrow_mut()[1] += 1;
                return r.iter().find(|x| !excl.contains(&x.0)).copied();
            }
            self.stat.borrow_mut()[0] += 1;
            let r = self.read_ranked(e, c, kd);
            let out = r.iter().find(|x| !excl.contains(&x.0)).copied();
            self.rcache.borrow_mut().insert((e, c, kd), r);
            return out;
        }
        let (idx, ph) = self.key(e, c, kd);
        let z: Vec<(i32, i32)> = idx
            .iter()
            .zip(&ph)
            .map(|(&x, &kp)| {
                let (a, b) = (self.re[x as usize] as i32, self.im[x as usize] as i32);
                match kp & 3 {
                    0 => (a, b),
                    1 => (b, -a),
                    2 => (-a, -b),
                    _ => (-b, a),
                }
            })
            .collect();
        // with kinds, a slot of kind kd is cleaned up against the words of that kind only
        let typed = self.cfg.kinds && kd != self.kn;
        let sc: Vec<(u32, i64)> = self
            .words
            .iter()
            .filter(|w| !excl.contains(w) && (!typed || self.kind[**w as usize] == kd))
            .map(|&v| {
                let s: i64 = idx
                    .iter()
                    .zip(&z)
                    .map(|(&x, &(zr, zi))| match self.vph(v, x) {
                        0 => zr,
                        1 => zi,
                        2 => -zr,
                        _ => -zi,
                    } as i64)
                    .sum();
                (v, s)
            })
            .filter(|x| x.1 > 0)
            .collect();
        let st: i128 = sc.iter().map(|x| x.1 as i128).sum();
        let bt: i128 = self.words.iter().map(|&w| self.bg[w as usize] as i128).sum::<i128>().max(1);
        sc.into_iter()
            .map(|(v, s)| (v, s as i128 * bt - self.bg[v as usize] as i128 * st))
            .filter(|x| x.1 > 0)
            .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
    }

    /// --sit-fast: all of a slot's values with positive excess, best first (over the kind's member list).
    fn read_ranked(&self, e: u32, c: u32, kd: u32) -> Vec<(u32, i128)> {
        let (idx, ph) = self.key(e, c, kd);
        let z: Vec<(i32, i32)> = idx
            .iter()
            .zip(&ph)
            .map(|(&x, &kp)| {
                let (a, b) = (self.re[x as usize] as i32, self.im[x as usize] as i32);
                match kp & 3 {
                    0 => (a, b),
                    1 => (b, -a),
                    2 => (-a, -b),
                    _ => (-b, a),
                }
            })
            .collect();
        let typed = self.cfg.kinds && kd != self.kn;
        let empty: Vec<u32> = vec![];
        let found: Vec<u32>;
        let logged: Vec<u32>;
        let cands: &Vec<u32> = if self.cfg.slotlog > 0 {
            logged = self.slot_log.get(&(e, c, kd)).cloned().unwrap_or_default();
            &logged
        } else if self.cfg.vlsh {
            // the values whose code matches a fully estimated band of the unbound slot
            // each position's possible phases: those its unbound component favours most (one, or two when the
            // real and imaginary parts tie, all four when both are zero)
            let est: Vec<Vec<u8>> = z
                .iter()
                .map(|&(a, b)| {
                    let s = [a, b, -a, -b];
                    let m = *s.iter().max().unwrap();
                    (0..4u8).filter(|&p| s[p as usize] == m).collect()
                })
                .collect();
            let mut f: Vec<u32> = vec![];
            let bw = self.cfg.vlsh_band;
            for b in 0..self.cfg.k / bw {
                // probe every combination of the band's possible phases, when there are at most 16
                let opts = &est[b * bw..b * bw + bw];
                let n: usize = opts.iter().map(|o| o.len()).product();
                if n > 16 {
                    continue;
                }
                for mut r in 0..n {
                    let band: Vec<u8> = opts
                        .iter()
                        .map(|o| {
                            let p = o[r % o.len()];
                            r /= o.len();
                            p
                        })
                        .collect();
                    if let Some(ws) = self.vband.get(&Self::vband_key(b, &band)) {
                        f.extend(ws.iter().filter(|&&w| !typed || self.kind[w as usize] == kd));
                    }
                }
            }
            f.sort_unstable();
            // a candidate: a value matching at least vlsh_hits bands
            let mut g: Vec<u32> = vec![];
            let mut i = 0;
            while i < f.len() {
                let mut j = i;
                while j < f.len() && f[j] == f[i] {
                    j += 1;
                }
                if j - i >= self.cfg.vlsh_hits {
                    g.push(f[i]);
                }
                i = j;
            }
            found = g;
            &found
        } else if typed {
            self.members.get(&kd).unwrap_or(&empty)
        } else {
            &self.words
        };
        self.stat.borrow_mut()[3] += cands.len() as u64;
        let mut sc: Vec<(u32, i64)> = if self.cfg.vlsh {
            cands
                .iter()
                .map(|&v| {
                    let s: i64 = z
                        .iter()
                        .enumerate()
                        .map(|(i, &(zr, zi))| match self.vc(v, i) {
                            0 => zr,
                            1 => zi,
                            2 => -zr,
                            _ => -zi,
                        } as i64)
                        .sum();
                    (v, s)
                })
                .filter(|x| x.1 > 0)
                .collect()
        } else if self.fm(1) {
            // the cheap phase, unrolled over the key: the key-side multiply once per read, then per value one
            // multiply-xor-shift per position and a sign-select of the unbound slot (vectorizes)
            let s0 = self.cfg.seed.wrapping_mul(0x1000_0001).wrapping_add(0x7777);
            let xm: Vec<u64> = idx.iter().map(|&x| (x as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93)).collect();
            let zr: Vec<i32> = z.iter().map(|x| x.0).collect();
            let zi: Vec<i32> = z.iter().map(|x| x.1).collect();
            cands
                .iter()
                .map(|&v| {
                    let a = (v as u64 ^ s0).wrapping_mul(0x9E37_79B9_7F4A_7C15);
                    let mut s = 0i32;
                    for i in 0..xm.len() {
                        let h = a ^ xm[i];
                        let p = ((h ^ (h >> 31)).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 62) as i32;
                        let w = if p & 1 == 0 { zr[i] } else { zi[i] };
                        let m = -(p >> 1);
                        s += (w ^ m) - m;
                    }
                    (v, s as i64)
                })
                .filter(|x| x.1 > 0)
                .collect()
        } else {
            cands
            .iter()
            .map(|&v| {
                let s: i64 = idx
                    .iter()
                    .zip(&z)
                    .map(|(&x, &(zr, zi))| match self.vph(v, x) {
                        0 => zr,
                        1 => zi,
                        2 => -zr,
                        _ => -zi,
                    } as i64)
                    .sum();
                (v, s)
            })
            .filter(|x| x.1 > 0)
            .collect()
        };
        let st: i128 = sc.iter().map(|x| x.1 as i128).sum();
        let bt = self.bg_total.max(1);
        let mut out: Vec<(u32, i128)> = sc.drain(..).map(|(v, s)| (v, s as i128 * bt - self.bg[v as usize] as i128 * st)).filter(|x| x.1 > 0).collect();
        out.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        out
    }
    fn ctx_key(&self) -> u64 {
        let n = self.hist.len();
        let at = |j: usize| if j < n { self.hist[n - 1 - j] as u64 } else { u64::MAX };
        hash(hash(at(0), at(2)), at(3))
    }

    // ---- the general query learner ----
    /// The query code: the cue words' frame codes (8-bit frame counters at a quarter of their max), each in the
    /// block of its position, plus 4 identity bits per position.
    fn query_bits(&self, cue: &[u32]) -> Vec<u32> {
        let mut q = vec![];
        for (j, &u) in cue.iter().enumerate().take(4) {
            for (b, on) in self.frame_code(u).into_iter().enumerate() {
                if on {
                    q.push((j * FB + b) as u32);
                }
            }
            for i in 0..4u64 {
                q.push((4 * FB + j * 64 + (hash(u as u64 + 31, i) % 64) as usize) as u32);
            }
        }
        // conjunctive context bits: a hash of the words 1 and 3 back together, skipping the entity between them
        // (the question form)
        let w0 = cue.first().copied().unwrap_or(u32::MAX) as u64;
        let w2 = cue.get(2).copied().unwrap_or(u32::MAX) as u64;
        let cx = hash(hash(w0, 7), w2);
        for i in 0..4u64 {
            q.push((4 * FB + 256 + (hash(cx, i) % 256) as usize) as u32);
        }
        if self.cfg.qform14 {
            let w3 = cue.get(3).copied().unwrap_or(u32::MAX) as u64;
            let cy = hash(hash(w0, 11), w3);
            for i in 0..4u64 {
                q.push((4 * FB + 512 + (hash(cy, i) % 256) as usize) as u32);
            }
        }
        q
    }
    fn fid(tag: u64, a: u64, b: u64) -> u32 {
        (hash(hash(tag, a), b) % NF as u64) as u32
    }
    /// A route's features: the cue position, the cue word's kind, and per hop its class, its kind, its recency rank
    /// within the entity, plus the number of hops.
    fn route_feats(&self, j: usize, e: u32, hops: &[(u32, u32, u32)]) -> Vec<u32> {
        let mut f = vec![Self::fid(1, j as u64, 0), Self::fid(2, self.kk(e) as u64, 0), Self::fid(6, hops.len() as u64, 0)];
        for (s, &(c, k, r)) in hops.iter().enumerate() {
            f.push(Self::fid(3, s as u64, c as u64));
            f.push(Self::fid(4, s as u64, k as u64));
            f.push(Self::fid(5, s as u64, r.min(3) as u64));
            f.push(Self::fid(7, s as u64 * 1_000_003 + c as u64, k as u64));
        }
        if hops.len() == 2 {
            // conjunctions across the two hops: the pair of classes, the pair of kinds, the pair of ranks
            f.push(Self::fid(8, hops[0].0 as u64 * 1_000_003 + hops[1].0 as u64, hops[0].1 as u64 * 1009 + hops[1].1 as u64));
            f.push(Self::fid(9, hops[0].1 as u64, hops[1].1 as u64));
            f.push(Self::fid(10, hops[0].2.min(3) as u64, hops[1].2.min(3) as u64));
        }
        f
    }
    /// e's live slots, newest first (at most n): (class, kind, rank).
    fn newest_slots(&self, e: u32, n: usize) -> Vec<(u32, u32, u32)> {
        let mut v: Vec<(u64, u32, u32)> = vec![];
        if self.cfg.recent > 0 {
            for &(c, k) in self.recent_slots.get(&e).map(|l| l.as_slice()).unwrap_or(&[]) {
                if self.occ.get(&(e, c)).map_or(false, |ks| ks.contains(&k)) {
                    v.push((self.stamp_of(e, c, k), c, k));
                }
            }
        }
        for c in self.live_classes(e, None).into_iter().filter(|_| self.cfg.recent == 0) {
            if let Some(ks) = self.occ.get(&(e, c)) {
                for &k in ks {
                    v.push((self.stamp_of(e, c, k), c, k));
                }
            }
        }
        v.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
        // the newest few of each kind, ranked within their kind (a slot's recency among the entity's values of the
        // same kind: the current holder is rank 0 even when the clause also wrote "up" or "." beside it)
        if self.cfg.sq_rank > 0 {
            // derived slots ranked by their query's balance or credits (stable: ties keep the newest first)
            let val = |c: u32| -> i64 {
                self.sq.iter().find(|o| o.id + DER == c).map_or(i64::MIN, |o| if self.cfg.sq_rank == 1 { o.util as i64 } else { o.credits as i64 })
            };
            let mut der: Vec<(u64, u32, u32)> = v.iter().filter(|x| x.1 >= DER).copied().collect();
            der.sort_by(|a, b| val(b.1).cmp(&val(a.1)));
            let mut it = der.into_iter();
            for x in v.iter_mut() {
                if x.1 >= DER {
                    *x = it.next().unwrap();
                }
            }
        }
        if self.cfg.kindfirst {
            // each kind's newest first, then the second newest of each (stable: newest first within a rank)
            let mut rk: HashMap<u32, u32> = HashMap::default();
            let mut tagged: Vec<(u32, (u64, u32, u32))> = v
                .iter()
                .map(|x| {
                    if x.1 >= DER {
                        return (0, *x);
                    }
                    let r = rk.entry(x.2).or_insert(0);
                    *r += 1;
                    (*r - 1, *x)
                })
                .collect();
            tagged.sort_by_key(|t| t.0);
            v = tagged.into_iter().map(|t| t.1).collect();
        }
        let mut seen_k: HashMap<u32, u32> = HashMap::default();
        let mut out = vec![];
        let mut derived = 0;
        for x in v {
            if x.1 >= DER {
                // open queries' derived slots have a budget of their own (they must not crowd out the facts)
                if derived < 4 {
                    out.push((x.1, x.2, 0));
                    derived += 1;
                }
                continue;
            }
            let r = seen_k.entry(x.2).or_insert(0);
            if *r < 2 && out.len() < n + derived {
                out.push((x.1, x.2, *r));
            }
            *r += 1;
        }
        out
    }
    /// Score features against the current query: the sum of W[q bit, feature].
    fn qscore(&self, q: &[u32], f: &[u32], cache: &mut HashMap<u32, i64>) -> i64 {
        // the query as counts per bit (a bit two hashes land on counts twice, as in the dense table)
        let mut memo = self.qset_memo.borrow_mut();
        if self.cfg.qsparse == 2 && memo.0 != q {
            memo.0.clear();
            memo.0.extend_from_slice(q);
            memo.1.iter_mut().for_each(|x| *x = 0);
            for &b in q {
                memo.1[b as usize] += 1;
            }
        }
        let qset = &memo.1;
        f.iter()
            .map(|&fi| {
                *cache.entry(fi).or_insert_with(|| {
                    if self.cfg.qsparse == 3 {
                        q.iter().map(|&b| { let (i, sg) = self.qbind(b, fi); sg * self.qh[i] as i64 }).sum()
                    } else if self.cfg.qsparse == 2 {
                        self.qr.score(fi, qset)
                    } else if self.cfg.qsparse == 1 {
                        q.iter().map(|&b| self.qs.get(b * NF as u32 + fi) as i64).sum()
                    } else {
                        q.iter().map(|&b| self.qw[b as usize * NF + fi as usize] as i64).sum()
                    }
                })
            })
            .sum()
    }
    // ---- route choice without weights: the question as a key in the store (--sit-qslots) ----
    /// The question form of a cue: the words 1, 3 and 4 back (not the entity between them).
    fn qform_of(cue: &[u32]) -> u64 {
        let w = |i: usize| cue.get(i).copied().unwrap_or(u32::MAX) as u64;
        hash(hash(hash(w(0), 71), w(2)), w(3))
    }
    /// A route's fields as (field, symbol): the number of hops, then per hop the class, the kind and the rank.
    fn route_fields(hops: &[(u32, u32, u32)], hopconj: bool) -> [(u32, u32); 10] {
        let none = (u32::MAX, u32::MAX, u32::MAX);
        let h1 = hops[0];
        let h2 = hops.get(1).copied().unwrap_or(none);
        let salt = if hopconj { 0xF1E1D + 977 * hops.len() as u64 } else { 0xF1E1D };
        let sym = |f: u64, x: u32| (hash(f + salt, x as u64) & 0xFFFF_FFFF) as u32;
        [
            (0, sym(0, hops.len() as u32)),
            (1, sym(1, h1.0)),
            (2, sym(2, h1.1)),
            (3, sym(3, h1.2.min(3))),
            (4, sym(4, h2.0)),
            (5, sym(5, h2.1)),
            (6, sym(6, if h2.2 == u32::MAX { u32::MAX } else { h2.2.min(3) })),
            // conjunctions: the two kinds; each hop's class with its kind
            (7, sym(7, (hash(h1.1 as u64, h2.1 as u64) & 0xFFFF_FFFF) as u32)),
            (8, sym(8, (hash(h1.0 as u64, h1.1 as u64) & 0xFFFF_FFFF) as u32)),
            (9, sym(9, (hash(h2.0 as u64, h2.1 as u64) & 0xFFFF_FFFF) as u32)),
        ]
    }
    /// The question slot's key: the form, the cue position, the cue kind and the field, hashed to K positions.
    fn qslot_key(&self, form: u64, j: usize, ke: u32, field: u32) -> (Vec<u32>, Vec<u8>) {
        // a stable key: no context drift and no clocks (this is long-term memory, unlike the episodic facts)
        let (d, k) = (self.cfg.d as u64, self.cfg.k);
        let base = hash(hash(form, j as u64 * 1_000_003 + ke as u64), field as u64 + 0xA11E);
        let mut idx = Vec::with_capacity(k);
        let mut ph = Vec::with_capacity(k);
        for i in 0..k {
            let h = hash(base, i as u64);
            idx.push((h % d) as u32);
            ph.push((h >> 40) as u8 & 3);
        }
        (idx, ph)
    }
    /// How strongly a slot holds a symbol: the phase-aligned sum over the slot's positions (-K..K, unbinding).
    fn qslot_sim(&self, idx: &[u32], ph: &[u8], sym: u32) -> i64 {
        idx.iter()
            .zip(ph)
            .map(|(&x, &kp)| {
                let (a, b) = (self.qre[x as usize] as i64, self.qim[x as usize] as i64);
                let (zr, zi) = match kp & 3 {
                    0 => (a, b),
                    1 => (b, -a),
                    2 => (-a, -b),
                    _ => (-b, a),
                };
                match self.vph(sym, x) {
                    0 => zr,
                    1 => zi,
                    2 => -zr,
                    _ => -zi,
                }
            })
            .sum()
    }
    /// A route's score from the question's slots: the sum over its fields of how strongly each slot holds the field.
    fn qslot_score(&self, j: usize, ke: u32, hops: &[(u32, u32, u32)], memo: &mut HashMap<(usize, u32, u32, u32), i64>) -> i64 {
        let form = self.qform;
        let mask = self.cfg.qslot_fields;
        Self::route_fields(hops, self.cfg.qslot_hopconj)
            .iter()
            .filter(|x| mask >> x.0 & 1 == 1)
            .map(|&(f, sym)| {
                *memo.entry((j, ke, f, sym)).or_insert_with(|| {
                    let (idx, ph) = self.qslot_key(form, j, ke, f);
                    self.qslot_sim(&idx, &ph, sym)
                })
            })
            .sum()
    }
    /// Write a symbol into a question slot, with the given phase offset (0 = Hebbian, 2 = anti-Hebbian).
    fn qslot_write(&mut self, form: u64, j: usize, ke: u32, field: u32, sym: u32, off: u8) {
        if field < 32 && self.cfg.qslot_fields >> field & 1 == 0 {
            return;
        }
        self.qslot_write_raw(form, j, ke, field, sym, off);
    }
    fn qslot_write_raw(&mut self, form: u64, j: usize, ke: u32, field: u32, sym: u32, off: u8) {
        let (idx, ph) = self.qslot_key(form, j, ke, field);
        let (m, cl) = (self.cfg.qslot_mag, self.cfg.qslot_clamp);
        for (&x, &kp) in idx.iter().zip(&ph) {
            let p = ((kp + self.vph(sym, x) + off) & 3) as usize;
            let x = x as usize;
            self.qre[x] = (self.qre[x] + m * DRE[p]).clamp(-cl, cl);
            self.qim[x] = (self.qim[x] + m * DIM[p]).clamp(-cl, cl);
        }
    }

    /// A window route's score: with `qslots`, weight-free: each feature, by its index in the route's feature list,
    /// matched against a question slot (form, "window", list length, index); otherwise the query weights.
    fn wscore(&self, q: &[u32], f: &[u32], cache: &mut HashMap<u32, i64>, memo: &mut HashMap<(usize, usize, u32), i64>) -> i64 {
        if !self.cfg.qslots {
            return self.qscore(q, f, cache);
        }
        f.iter()
            .enumerate()
            .map(|(i, &fi)| {
                *memo.entry((f.len(), i, fi)).or_insert_with(|| {
                    let (idx, ph) = self.qslot_key(self.qform, 99, f.len() as u32, 100 + i as u32);
                    self.qslot_sim(&idx, &ph, fi)
                })
            })
            .sum()
    }
    /// Weight-free learning for the window, on a mistake: the route that held the answer is written into the
    /// question's window slots, the wrong top route's differing features with opposite phase.
    fn window_hebb(&mut self, wc: &[(Vec<u32>, u32, i64)], w: u32) {
        let Some(top) = wc.first() else { return };
        if top.1 == w {
            return;
        }
        let Some(good) = wc.iter().find(|x| x.1 == w) else { return };
        let (gf, tf) = (good.0.clone(), top.0.clone());
        let form = self.qform;
        for (i, &fi) in gf.iter().enumerate() {
            self.qslot_write_raw(form, 99, gf.len() as u32, 100 + i as u32, fi, 0);
        }
        for (i, &fi) in tf.iter().enumerate() {
            if tf.len() != gf.len() || gf[i] != fi {
                self.qslot_write_raw(form, 99, tf.len() as u32, 100 + i as u32, fi, 2);
            }
        }
    }

    /// The cell and sign a (query bit, feature) pair is bound to in the superposed vector.
    fn qbind(&self, b: u32, fi: u32) -> (usize, i64) {
        let h = hash(b as u64 * NF as u64 + fi as u64, 0x5eed);
        ((h as usize) & (self.qh.len() - 1), if (h >> 63) & 1 == 1 { 1 } else { -1 })
    }
    fn qupdate(&mut self, q: &[u32], f: &[u32], d: i16) {
        for &b in q {
            for &fi in f {
                if self.cfg.qsparse == 3 {
                    let (i, sg) = self.qbind(b, fi);
                    self.qh[i] = self.qh[i].saturating_add((sg * d as i64) as i8);
                } else if self.cfg.qsparse == 2 {
                    self.qr.add(fi, b as u16, d as i8);
                } else if self.cfg.qsparse == 1 {
                    self.qs.add(b * NF as u32 + fi, d as i8);
                } else {
                    let x = &mut self.qw[b as usize * NF + fi as usize];
                    *x = x.saturating_add(d);
                }
            }
        }
    }
    /// The query learner's candidates: every one- and two-hop route from the cue words' newest live slots, scored.
    fn query_recall(&mut self, cue: &[u32]) -> Vec<(u32, u32)> {
        let q = self.query_bits(cue);
        self.qform = if self.cfg.qslot_kforms {
            cue.iter().take(4).fold(0x9F0u64, |h, &u| hash(h, self.kk(u) as u64))
        } else {
            Self::qform_of(cue)
        };
        let mut memo: HashMap<(usize, u32, u32, u32), i64> = HashMap::default();
        let mut qroute: Vec<(usize, u32)> = vec![];
        let mut csrc: Vec<(u8, u32)> = vec![];
        self.after_q = self.hist.last().map_or(false, |&u| self.boundary[u as usize] && self.mood[u as usize] >= 8);
        if self.cfg.sq_max > 0 && self.cfg.sq_mode > 0 && self.after_q {
            if self.qring.len() < 32 {
                self.qring.push(q.clone());
            } else {
                self.qring[self.qring_at] = q.clone();
            }
            self.qring_at = (self.qring_at + 1) % 32;
        }
        if let Ok(at) = std::env::var("SIT_SQDBG") {
            if at.split(',').any(|x| x.parse::<u64>().ok() == Some(self.t)) {
                for o in &self.sq {
                    for &e in cue.iter().take(3) {
                        let m = self.sq_ptr.get(&(o.id, e)).copied();
                        let d = self.read_best(e, DER + o.id, o.p2.1, &[]);
                        let ms = m.map(|m| format!("{:?}", self.newest_slots(m, 8))).unwrap_or_default();
                        eprintln!("SQDBG t {} q{} cue {} ptr {:?} derived {:?} | ptr's newest slots {}", self.t, o.id, e, m, d, ms);
                    }
                }
            }
        }
        let mut cache: HashMap<u32, i64> = HashMap::default();
        let mut cands: Vec<(Vec<u32>, u32, i64, u64, Route, (u32, u32))> = vec![];
        let anchors = if self.cfg.ranges && self.after_q { self.anchors(cue) } else { vec![] };
        let dbg = std::env::var("SIT_QDBG").is_ok() && std::env::var("SIT_QDBG_FROM").ok().and_then(|x| x.parse::<u64>().ok()).map_or(true, |f| self.t >= f);
        let mut desc: Vec<String> = vec![];
        for (j, &e) in cue.iter().enumerate() {
            if !self.seen[e as usize] {
                continue;
            }
            let is_content_e = self.ksize.get(&self.kk(e)).copied().unwrap_or(0) >= 3;
            let content_cue = !self.fm(3) || is_content_e;
            if self.cfg.heads && content_cue {
                // long-term heads: the newest two per value kind, ranked within kind, newest first
                let mut hs: Vec<(u64, u32, u32, u32, u32)> = self
                    .heads_of
                    .get(&e)
                    .map(|v| {
                        v.iter()
                            .map(|&(key, k, val, t, c)| {
                                // the value's kind as it is now (kinds settle after a head is written)
                                (t, c, if self.cfg.heads_path { self.kk(val) } else { k }, val, key)
                            })
                            // fast: only content values (kinds of 3 or more), before the sort
                            .filter(|h| !self.fm(4) || self.ksize.get(&h.2).copied().unwrap_or(0) >= 3)
                            .collect()
                    })
                    .unwrap_or_default();
                let (per_kind, cap) = if self.cfg.heads_path { (self.cfg.heads_per_kind, 48) } else { (2, 8) };
                hs.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
                let mut seen_k: HashMap<u32, u32> = HashMap::default();
                let mut taken = 0;
                for (t, c, k, val, key) in hs {
                    let r = seen_k.entry(k).or_insert(0);
                    let rr = *r;
                    *r += 1;
                    // content kinds (3 or more members) offer up to per_kind heads; singleton kinds ("." "the" "to",
                    // whose heads multiply with every distinct clause tail) offer their newest only
                    let lim = if self.cfg.heads_path && self.ksize.get(&k).copied().unwrap_or(0) < 3 { 1 } else { per_kind };
                    if rr >= lim || cue.contains(&val) {
                        continue;
                    }
                    let mut f1 = self.route_feats(j, e, &[(c, k, rr)]);
                    f1.push(Self::fid(41, 1, 0)); // the source: long-term heads
                    f1.push(Self::fid(42, k as u64, rr.min(7) as u64));
                    f1.push(Self::fid(43, j as u64, c as u64));
                    if self.cfg.heads_path {
                        f1.push(Self::fid(44, key as u64, 0)); // the exact relation path ("found the" is not "got the")
                    }
                    let ke = self.kk(e);
                    let sc = if self.cfg.qslots { self.qslot_score(j, ke, &[(c, k, rr)], &mut memo) } else { self.qscore(&q, &f1, &mut cache) };
                    if dbg {
                        desc.push(format!("j{}:{}>H c{}k{}r{}={}", j, e, c, k, rr, val));
                    }
                    cands.push((f1, val, sc, t, (c, k, u32::MAX, u32::MAX), (rr, u32::MAX)));
                    csrc.push((1, key));
                    qroute.push((j, ke));
                    taken += 1;
                    if taken >= cap {
                        break;
                    }
                }
            }
            if self.cfg.ranges && self.after_q && !anchors.is_empty() && content_cue {
                if let Some(keys) = self.heads_of.get(&e) {
                    for &(key, k0, _, _, c) in keys {
                        let Some(hist) = self.posts.get(&(e, key, k0)) else { continue };
                        for &(a, b, fw, which, len, ofw, olen, orank) in &anchors {
                            // the history inside the unit [a, b), newest first, at most 2
                            let lo = hist.partition_point(|x| x.0 < a);
                            let hi = hist.partition_point(|x| x.0 < b);
                            for (rank, &(t, val)) in hist[lo..hi].iter().rev().take(2).enumerate() {
                                if cue.contains(&val) {
                                    continue;
                                }
                                let k = self.kk(val);
                                if self.ksize.get(&k).copied().unwrap_or(0) < 3 {
                                    continue; // only content values
                                }
                                let mut f1 = self.route_feats(j, e, &[(c, k, rank as u32)]);
                                f1.push(Self::fid(45, 1, 0)); // the source: a unit's range
                                f1.push(Self::fid(46, which as u64, len as u64));
                                let (fwf, ofwf) = (self.opener_feat(fw), if which >= 2 { self.opener_feat(ofw) } else { 0 });
                                f1.push(Self::fid(48, fwf, 0)); // the opener type that addressed it
                                if which >= 2 {
                                    f1.push(Self::fid(50, ofwf, olen as u64)); // nested inside this outer type
                                }
                                f1.push(Self::fid(44, key as u64, 1));
                                f1.push(Self::fid(49, rank as u64, k as u64));
                                if self.cfg.scope {
                                    // which occurrence addressed it, conjoined with the opener type
                                    f1.push(Self::fid(51, fwf, (which * 3 + orank) as u64));
                                    if which >= 2 {
                                        // the outer type and its occurrence, with the inner type
                                        f1.push(Self::fid(52, (ofwf << 20) ^ fwf, orank as u64));
                                    }
                                    // how many of the question's anchor units hold this event
                                    let n_in = anchors.iter().filter(|x| x.0 <= t && t < x.1).count();
                                    f1.push(Self::fid(53, n_in.min(4) as u64, which as u64));
                                }
                                let ke = self.kk(e);
                                let sc = if self.cfg.qslots { self.qslot_score(j, ke, &[(c, k, rank as u32)], &mut memo) } else { self.qscore(&q, &f1, &mut cache) };
                                if dbg {
                                    desc.push(format!("j{}:{}>R{}-{} c{}k{}={}", j, e, a, b, c, k, val));
                                }
                                cands.push((f1, val, sc, t, (c, k, u32::MAX, u32::MAX), (rank as u32, u32::MAX)));
                                csrc.push((2, key));
                                qroute.push((j, ke));
                            }
                        }
                    }
                }
            }
            for (c, k, r) in self.newest_slots(e, 8) {
                let Some((m, _)) = self.read_best(e, c, k, cue) else { continue };
                let f1 = self.route_feats(j, e, &[(c, k, r)]);
                let ke = self.kk(e);
                let sc = if self.cfg.qslots { self.qslot_score(j, ke, &[(c, k, r)], &mut memo) } else { self.qscore(&q, &f1, &mut cache) };
                if dbg {
                    desc.push(format!("j{}:{}>c{}k{}r{}={}", j, e, c, k, r, m));
                }
                cands.push((f1, m, sc, self.stamp_of(e, c, k), (c, k, u32::MAX, u32::MAX), (r, u32::MAX)));
                csrc.push((0, 0));
                qroute.push((j, ke));
                if self.fm(2) && (j >= 4 || !is_content_e) {
                    continue; // fast: second hops only from the 4 nearest cue words, and only content ones
                }
                for (c2, k2, r2) in self.newest_slots(m, 8) {
                    let mut ex = cue.to_vec();
                    ex.push(m);
                    let Some((v, _)) = self.read_best(m, c2, k2, &ex) else { continue };
                    let f2 = self.route_feats(j, e, &[(c, k, r), (c2, k2, r2)]);
                    let sc = if self.cfg.qslots { self.qslot_score(j, ke, &[(c, k, r), (c2, k2, r2)], &mut memo) } else { self.qscore(&q, &f2, &mut cache) };
                    if dbg {
                        desc.push(format!("j{}:{}>c{}k{}r{}={}>c{}k{}r{}={}", j, e, c, k, r, m, c2, k2, r2, v));
                    }
                    cands.push((f2, v, sc, self.stamp_of(e, c, k), (c, k, c2, k2), (r, r2)));
                    csrc.push((0, 0));
                    qroute.push((j, ke));
                }
            }
        }
        // best score first; ties: the newest first slot, then the shorter route (stable sort keeps one-hop first)
        if dbg {
            let mut ix: Vec<usize> = (0..cands.len()).collect();
            ix.sort_by(|&a, &b| cands[b].2.cmp(&cands[a].2).then(cands[b].3.cmp(&cands[a].3)));
            let parts: Vec<String> = ix.iter().take(10).map(|&i| format!("{}[{}]", desc[i], cands[i].2)).collect();
            eprintln!("QROUTES {} {}", self.t, parts.join(" "));
        }
        {
            let mut st = self.stat.borrow_mut();
            st[2] += cands.len() as u64;
            st[5] += 1;
        }
        // sort (stable), keeping each route's (cue position, cue kind) aligned
        let mut order: Vec<usize> = (0..cands.len()).collect();
        order.sort_by(|&a, &b| cands[b].2.cmp(&cands[a].2).then(cands[b].3.cmp(&cands[a].3)));
        let mut slots: Vec<Option<(Vec<u32>, u32, i64, u64, Route, (u32, u32))>> = cands.into_iter().map(Some).collect();
        let cands: Vec<(Vec<u32>, u32, i64, u64, Route, (u32, u32))> = order.iter().map(|&i| slots[i].take().unwrap()).collect();
        self.qroute = order.iter().map(|&i| qroute[i]).collect();
        self.qsrc = order.iter().map(|&i| csrc[i]).collect();
        let lo = cands.iter().map(|x| x.2).min().unwrap_or(0);
        let props: Vec<(u32, u32)> = cands.iter().map(|x| ((x.2 - lo + 1).min(u32::MAX as i64) as u32, x.1)).collect();
        if dbg && !cands.is_empty() {
            let vals: Vec<String> = cands.iter().take(12).map(|x| format!("{}:{}", x.1, x.2)).collect();
            eprintln!("QDBG {} n {} top {}", self.t, cands.len(), vals.join(" "));
        }
        self.qbits = q;
        self.qhops = cands.iter().map(|x| x.4).collect();
        self.qranks = cands.iter().map(|x| x.5).collect();
        self.qcands = cands.into_iter().map(|x| (x.0, x.1, x.2)).collect();
        props
    }
    /// Learn the query from word w: a mistake-driven (perceptron) three-factor update. When the top route was
    /// wrong, the best-scored route that held w is stepped up (Go) and the top route down (NoGo), for every
    /// (query bit, route feature) pair: pre-synaptic the query, post-synaptic the route, the modulator right or wrong.
    fn query_learn(&mut self, w: u32) {
        let cands = std::mem::take(&mut self.qcands);
        let q = std::mem::take(&mut self.qbits);
        let hops = std::mem::take(&mut self.qhops);
        let ranks = std::mem::take(&mut self.qranks);
        let qroute = std::mem::take(&mut self.qroute);
        let qsrc = std::mem::take(&mut self.qsrc);
        if self.cfg.lt_credit > 0 && self.after_q {
            // credit the path behind the best-ranked long-term route that held the answer
            if let Some(i) = cands.iter().position(|x| x.1 == w) {
                if let Some(&(src, key)) = qsrc.get(i) {
                    if src > 0 {
                        *self.key_credit.entry(key).or_insert(0) += 1;
                    }
                }
            }
        }
        if self.cfg.sq_max > 0 && self.after_q {
            // which two-hop patterns answered (candidates to open), and which open queries' slots did (credit), at
            // the answers to questions (after a boundary the mood counter has learned closes a question)
            let mut credited: Vec<(u32, i32)> = vec![];
            let top_right = cands.first().map_or(false, |x| x.1 == w);
            for (i, (cd, h)) in cands.iter().zip(&hops).enumerate() {
                if cd.1 != w {
                    continue;
                }
                if h.2 != u32::MAX && h.0 < COMP && h.2 < COMP && (self.cfg.sq_open == 0 || !top_right) {
                    *self.sq_cand.entry(((h.0, h.1), (h.2, h.3))).or_insert(0) += 1;
                    if self.cfg.sq_kinds && ranks.get(i) == Some(&(0, 0)) {
                        *self.sq_cand.entry(((ANY, h.1), (ANY, h.3))).or_insert(0) += 1;
                    }
                }
                if h.0 >= DER && !credited.iter().any(|c| c.0 == h.0 - DER) {
                    let amt = if self.cfg.sq_credit == 0 {
                        32
                    } else if i == 0 {
                        32 // it gave the answer, and the answer was right
                    } else if !top_right {
                        16 // the answer given was wrong, and it held the right one
                    } else {
                        0 // another route answered right anyway
                    };
                    if amt > 0 {
                        credited.push((h.0 - DER, amt));
                    }
                }
            }
            for (id, amt) in credited {
                if let Some(o) = self.sq.iter_mut().find(|o| o.id == id) {
                    o.util = (o.util + amt).min(32768); // a credit of 32 is worth 32 reads and writes
                    o.credits += 1;
                }
            }
            // blame: the answer given came through an open query's slot and was wrong
            if let (Some(top), Some(h)) = (cands.first(), hops.first()) {
                if top.1 != w && h.0 >= DER {
                    if let Some(o) = self.sq.iter_mut().find(|o| o.id == h.0 - DER) {
                        o.util -= 64;
                    }
                }
            }
        }
        if self.cfg.qslots {
            // Hebbian on a mistake: write the route that held the answer into the question's slots, and the wrong
            // route's differing fields with opposite phase
            if let Some(top) = cands.first() {
                if top.1 != w {
                    if let Some(gi) = cands.iter().position(|x| x.1 == w) {
                        let route_of = |i: usize| -> Vec<(u32, u32, u32)> {
                            let (h, r) = (hops[i], ranks[i]);
                            let mut v = vec![(h.0, h.1, r.0)];
                            if h.2 != u32::MAX {
                                v.push((h.2, h.3, r.1));
                            }
                            v
                        };
                        let form = self.qform;
                        let (gj, gk) = qroute[gi];
                        let gf = Self::route_fields(&route_of(gi), self.cfg.qslot_hopconj);
                        for &(f, sym) in &gf {
                            self.qslot_write(form, gj, gk, f, sym, 0);
                        }
                        let (tj, tk) = qroute[0];
                        let tf = Self::route_fields(&route_of(0), self.cfg.qslot_hopconj);
                        for (i, &(f, sym)) in tf.iter().enumerate() {
                            if self.cfg.qslot_anti && ((tj, tk) != (gj, gk) || gf[i].1 != sym) {
                                self.qslot_write(form, tj, tk, f, sym, 2);
                            }
                        }
                    }
                }
            }
            return;
        }
        if let Some(top) = cands.first() {
            if top.1 != w {
                if let Some(good) = cands.iter().find(|x| x.1 == w) {
                    let (gf, tf) = (good.0.clone(), top.0.clone());
                    self.qupdate(&q, &gf, 1);
                    self.qupdate(&q, &tf, -1);
                    // keep the mistake for replay: the routes as they were, with their answer
                    if self.cfg.replay > 0 && (!self.cfg.replay_q || self.after_q) {
                        let ep = (q.clone(), cands.iter().map(|x| (x.0.clone(), x.1)).collect(), w);
                        if self.ring.len() < 512 {
                            self.ring.push(ep);
                        } else {
                            self.ring[self.ring_at] = ep;
                        }
                        self.ring_at = (self.ring_at + 1) % 512;
                    }
                }
            }
        }
        // replay: re-score a few kept mistakes under the current weights, and learn again from those still wrong
        for i in 0..self.cfg.replay {
            if self.ring.is_empty() || (self.cfg.replay_q && !self.after_q) {
                break;
            }
            let at = (hash(self.t, 4242 + i as u64) % self.ring.len() as u64) as usize;
            let (rq, routes, rw) = self.ring[at].clone();
            let mut cache: HashMap<u32, i64> = HashMap::default();
            let mut best: Option<(i64, usize)> = None;
            let mut good: Option<(i64, usize)> = None;
            for (r, (f, v)) in routes.iter().enumerate() {
                let sc = self.qscore(&rq, f, &mut cache);
                if best.map_or(true, |b| sc > b.0) {
                    best = Some((sc, r));
                }
                if *v == rw && good.map_or(true, |g| sc > g.0) {
                    good = Some((sc, r));
                }
            }
            if let (Some((_, b)), Some((_, g))) = (best, good) {
                if routes[b].1 != rw {
                    let (gf, bf) = (routes[g].0.clone(), routes[b].0.clone());
                    self.qupdate(&rq, &gf, 1);
                    self.qupdate(&rq, &bf, -1);
                }
            }
        }
    }

    /// The window's routes: from each cue word's latest occurrences (up to 3) in the lookback window, the word at
    /// an offset of -6..+6 (one hop), and from that word's latest 2 occurrences, the word at an offset again (two
    /// hops). Features: cue position, the cue word's kind, the occurrence rank, the offset, the value's kind, and for
    /// two hops the second occurrence rank, offset, the intermediate's kind and the pair of offsets.
    fn window_cands(&self, cue: &[u32], q: &[u32]) -> Vec<(Vec<u32>, u32, i64)> {
        let n = self.win.len();
        let occ = |x: u32, k: usize| -> Vec<usize> {
            let mut v = vec![];
            for i in (0..n).rev() {
                if self.win[i] == x {
                    v.push(i);
                    if v.len() >= k {
                        break;
                    }
                }
            }
            v
        };
        let mut memo: HashMap<(usize, usize, u32), i64> = HashMap::default();
        let mut cache: HashMap<u32, i64> = HashMap::default();
        let mut out: Vec<(Vec<u32>, u32, i64)> = vec![];
        for (j, &e) in cue.iter().enumerate() {
            let ke = self.kk(e) as u64;
            for (r, &i) in occ(e, 3).iter().enumerate() {
                for d in -6i64..=6 {
                    let p = i as i64 + d;
                    if d == 0 || p < 0 || p >= n as i64 {
                        continue;
                    }
                    let m = self.win[p as usize];
                    let km = self.kk(m) as u64;
                    let f1 = vec![
                        Self::fid(20, j as u64, 0), Self::fid(21, ke, 0), Self::fid(22, r as u64, 0),
                        Self::fid(23, (d + 6) as u64, 0), Self::fid(24, km, 0), Self::fid(29, (d + 6) as u64, km),
                    ];
                    let sc = self.wscore(q, &f1, &mut cache, &mut memo);
                    out.push((f1.clone(), m, sc));
                    if j != 1 && j != 0 {
                        continue; // two hops only from the nearest cue words (cost)
                    }
                    for (r2, &i2) in occ(m, 2).iter().enumerate() {
                        for d2 in -6i64..=6 {
                            let p2 = i2 as i64 + d2;
                            if d2 == 0 || p2 < 0 || p2 >= n as i64 {
                                continue;
                            }
                            let v = self.win[p2 as usize];
                            let kv = self.kk(v) as u64;
                            let mut f2 = f1.clone();
                            f2.pop();
                            f2.extend([
                                Self::fid(25, r2 as u64, 0), Self::fid(26, (d2 + 6) as u64, 0), Self::fid(27, kv, 0),
                                Self::fid(28, (d + 6) as u64, (d2 + 6) as u64), Self::fid(30, km, kv),
                            ]);
                            let sc = self.wscore(q, &f2, &mut cache, &mut memo);
                            out.push((f2, v, sc));
                        }
                    }
                }
            }
        }
        out.sort_by(|a, b| b.2.cmp(&a.2));
        out
    }
    /// The margin of a ranked candidate list: how far the top score is above the best score for another value,
    /// as a bucket (0 = no margin, else the bit length of the margin, up to 15).
    fn margin_bucket(c: &[(Vec<u32>, u32, i64)]) -> u32 {
        let Some(top) = c.first() else { return 0 };
        let other = c.iter().find(|x| x.1 != top.1).map(|x| x.2);
        match other {
            None => 15,
            Some(o) => {
                let m = (top.2 - o).max(0) as u64;
                (64 - m.leading_zeros()).min(15)
            }
        }
    }

    // ---- open queries ----
    /// Close the open queries that have stopped paying for their upkeep, and open the most counted candidate
    /// pattern if there is room: so the number open tunes itself.
    fn sq_manage(&mut self) {
        let t = self.t;
        // diagnostics: SIT_SQ_FORCE=a,b holds the kind-level chain (any, kind of a) then (any, kind of b) open from
        // word 2,048 on, and never closes it (a test of what the chain is worth once open)
        if let Ok(fw) = std::env::var("SIT_SQ_FORCE") {
            let w: Vec<u32> = fw.split(',').filter_map(|x| x.parse().ok()).collect();
            if w.len() == 2 && t == 2048 {
                let id = self.sq_next;
                self.sq_next += 1;
                let (ka, kb) = (self.kk(w[0]), self.kk(w[1]));
                self.sq.push(OpenQuery { id, p1: (ANY, ka), p2: (ANY, kb), util: 1 << 30, credits: 0, work: 0, opened_at: t });
                self.sq_opened += 1;
            }
        }
        // the open set's net payoff and age, over the queries past their trial (for the relative price)
        let (mut sn, mut sa) = (0i64, 0i64);
        for o in self.sq.iter().filter(|o| t - o.opened_at >= 2000) {
            sn += o.util as i64 - 512;
            sa += (t - o.opened_at) as i64;
        }
        let price = self.cfg.sq_price;
        let mut closing: Vec<u32> = self
            .sq
            .iter()
            .filter(|o| {
                let age = (t - o.opened_at) as i64;
                if age < 2000 {
                    return false;
                }
                let below_avg = (o.util as i64 - 512) * sa < sn * age; // its payoff per word below the set's
                match price {
                    0 => o.util < 0,
                    1 => below_avg,
                    _ => o.util < 0 || below_avg,
                }
            })
            .map(|o| o.id)
            .collect();
        closing.sort_unstable();
        for id in closing {
            self.sq_close(id);
        }
        if self.sq.len() < self.cfg.sq_max {
            let open: Vec<((u32, u32), (u32, u32))> = self.sq.iter().map(|o| (o.p1, o.p2)).collect();
            let best = if self.cfg.sq_mode == 0 {
                self.sq_cand.iter().filter(|(p, &n)| n >= 16 && !open.contains(p)).max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0))).map(|(p, _)| *p)
            } else {
                self.sq_pick(&open)
            };
            if let Some(p) = best {
                self.sq_cand.insert(p, 0);
                self.sq_in.remove(&p);
                // a pattern keeps its id across closes, so its derived class (and the weights learned on it) return
                let id = match self.sq_pid.get(&p) {
                    Some(&id) => id,
                    None => {
                        let id = self.sq_next;
                        self.sq_next += 1;
                        self.sq_pid.insert(p, id);
                        id
                    }
                };
                self.sq.push(OpenQuery { id, p1: p.0, p2: p.1, util: 512, credits: 0, work: 0, opened_at: t });
                self.sq_opened += 1;
            }
        }
    }
    /// The input's proposals (counted at least 16 times; with mode 2, also at least 4 answers held) ranked by the
    /// higher level: the chain's route features summed under the learned query weights, over the question codes
    /// seen lately, at the best cue position. The best positive one, or none.
    fn sq_pick(&mut self, open: &[((u32, u32), (u32, u32))]) -> Option<((u32, u32), (u32, u32))> {
        if self.qring.is_empty() {
            return None;
        }
        if self.sq_in.len() > 65536 {
            // forget slowly: halve the counts and drop the ones that reach zero
            self.sq_in.retain(|_, v| {
                v.0 >>= 1;
                v.0 > 0
            });
        }
        let mut cands: Vec<(((u32, u32), (u32, u32)), u32)> = self
            .sq_in
            .iter()
            .filter(|(p, v)| v.0 >= 16 && !open.contains(p) && (self.cfg.sq_mode < 2 || self.sq_cand.get(p).copied().unwrap_or(0) >= 4))
            .map(|(p, v)| (*p, v.1))
            .collect();
        cands.sort_unstable();
        let mut best: Option<(i64, ((u32, u32), (u32, u32)))> = None;
        for (p, x) in cands {
            let hops = [(p.0 .0, p.0 .1, 0), (p.1 .0, p.1 .1, 0)];
            let mut top = i64::MIN;
            for j in 0..4 {
                let f = self.route_feats(j, x, &hops);
                let mut cache: HashMap<u32, i64> = HashMap::default();
                let mut s = 0i64;
                for q in &self.qring {
                    cache.clear();
                    s += self.qscore(q, &f, &mut cache);
                }
                top = top.max(s);
            }
            if top > 0 && best.map_or(true, |b| top > b.0) {
                best = Some((top, p));
            }
        }
        best.map(|b| b.1)
    }
    fn sq_close(&mut self, id: u32) {
        self.sq.retain(|o| o.id != id);
        self.sq_ptr.retain(|k, _| k.0 != id);
        self.sq_rev.retain(|k, _| k.0 != id);
        // purge its derived slots from every entity: a closed query's slots are no longer live (they had stayed in
        // the live lists, growing the per-word cost and filling the reader's derived budget with stale values)
        let cs = DER + id;
        self.occ.retain(|k, _| k.1 != cs);
        self.stamp.retain(|k, _| k.1 != cs);
        for v in self.dcl.values_mut() {
            v.retain(|&c| c != cs);
        }
        self.sq_closed += 1;
    }
    /// A committed fact (x, class c, kind kd) = v, checked against every open query: it may move x's pointer (the
    /// query's first hop) and refresh x's derived slot, or change an intermediate's second hop and refresh every
    /// entity pointing to it. A fact of the first hop's kind under another class detaches x.
    fn sq_on_fact(&mut self, x: u32, c: u32, kd: u32, v: u32) {
        let qs: Vec<(u32, (u32, u32), (u32, u32))> = self.sq.iter().map(|o| (o.id, o.p1, o.p2)).collect();
        let hit = |p: (u32, u32)| kd == p.1 && c < COMP && (p.0 == ANY || p.0 == c);
        for (id, p1, p2) in qs {
            let mut work = 0u64;
            if hit(p1) {
                if let Some(old) = self.sq_ptr.insert((id, x), v) {
                    if let Some(r) = self.sq_rev.get_mut(&(id, old)) {
                        r.retain(|&y| y != x);
                    }
                }
                self.sq_rev.entry((id, v)).or_default().push(x);
                work += 1;
                if let Some(z) = self.sq_hop2(v, p2, &[v, x]) {
                    self.sq_write(x, id, p2.1, z);
                    work += 2;
                }
            } else if kd == p1.1 && c < COMP && p1.0 != ANY {
                // detach: x took a value of that kind under another relation
                if let Some(old) = self.sq_ptr.remove(&(id, x)) {
                    if let Some(r) = self.sq_rev.get_mut(&(id, old)) {
                        r.retain(|&y| y != x);
                    }
                    work += 1;
                }
            }
            if hit(p2) {
                let xs: Vec<u32> = self.sq_rev.get(&(id, x)).cloned().unwrap_or_default();
                for xx in xs {
                    self.sq_write(xx, id, p2.1, v);
                    work += 1;
                }
            }
            if work > 0 {
                self.sq_work += work;
                if let Some(o) = self.sq.iter_mut().find(|o| o.id == id) {
                    o.work += work;
                    o.util -= work as i32; // upkeep: one unit per read or write
                }
            }
        }
    }
    /// An open query's second hop from the intermediate m: its slot (class, kind), or for a hop of any relation, m's
    /// newest value of that kind.
    fn sq_hop2(&self, m: u32, p2: (u32, u32), excl: &[u32]) -> Option<u32> {
        if p2.0 == ANY {
            // m's newest value of that kind, over all its live slots (not only the newest few: "daniel picked up the
            // key ." writes four values into daniel's slots and would push his place out of a short list)
            let mut best: Option<(u64, u32)> = None;
            for c in self.live_classes(m, None) {
                if c >= COMP {
                    continue;
                }
                if self.occ.get(&(m, c)).map_or(false, |ks| ks.contains(&p2.1)) {
                    let st = self.stamp_of(m, c, p2.1);
                    if best.map_or(true, |b| st > b.0) {
                        best = Some((st, c));
                    }
                }
            }
            let (_, c2) = best?;
            self.read_best(m, c2, p2.1, excl).map(|x| x.0)
        } else if self.live(m, p2.0, p2.1) {
            self.read_best(m, p2.0, p2.1, excl).map(|x| x.0)
        } else {
            None
        }
    }
    fn sq_write(&mut self, x: u32, id: u32, kd: u32, z: u32) {
        let cs = DER + id;
        self.write(x, cs, kd, z);
        let ks = self.occ.entry((x, cs)).or_default();
        if !ks.contains(&kd) {
            ks.push(kd);
        }
        let cl = self.dcl.entry((x, 3)).or_default();
        if !cl.contains(&cs) {
            cl.push(cs);
        }
        self.stamp.insert((x, cs, kd), self.t);
        self.touch(x, cs, kd);
    }
    /// --sit-recent: (class, kind) is the entity's newest slot.
    fn touch(&mut self, e: u32, c: u32, k: u32) {
        if self.cfg.recent == 0 {
            return;
        }
        let l = self.recent_slots.entry(e).or_default();
        if let Some(at) = l.iter().position(|&x| x == (c, k)) {
            l.remove(at);
        }
        l.push((c, k));
        if l.len() > self.cfg.recent {
            l.remove(0);
        }
    }
    /// The most counted candidate patterns (diagnostics): (pattern, count).
    pub fn sq_top_cands(&self, n: usize) -> Vec<(((u32, u32), (u32, u32)), u32)> {
        let mut v: Vec<_> = self.sq_cand.iter().map(|(p, c)| (*p, *c)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        v.truncate(n);
        v
    }
    /// The query weights' footprint (diagnostics): (stored entries, nonzero, at the 8-bit limit (sparse) or beyond 8
    /// bits (dense), bytes).
    pub fn qw_census(&self) -> (usize, usize, usize, usize) {
        if self.cfg.qsparse == 3 {
            let nz = self.qh.iter().filter(|&&x| x != 0).count();
            let sat = self.qh.iter().filter(|&&x| x == i8::MAX || x == i8::MIN).count();
            (self.qh.len(), nz, sat, self.qh.len())
        } else if self.cfg.qsparse == 2 {
            let (nz, sat) = self.qr.census();
            (nz, nz, sat, self.qr.bytes())
        } else if self.cfg.qsparse == 1 {
            let (nz, sat) = self.qs.census();
            (self.qs.len, nz, sat, self.qs.bytes())
        } else {
            let nz = self.qw.iter().filter(|&&x| x != 0).count();
            let big = self.qw.iter().filter(|&&x| x > 127 || x < -128).count();
            (self.qw.len(), nz, big, self.qw.len() * 2)
        }
    }
    pub fn words_seen(&self) -> u64 {
        self.t
    }
    /// The open queries now (diagnostics).
    pub fn open_queries(&self) -> Vec<OpenQuery> {
        self.sq.clone()
    }

    fn live(&self, e: u32, c: u32, kd: u32) -> bool {
        !self.cfg.kinds || self.occ.get(&(e, c)).map(|v| v.contains(&kd)).unwrap_or(false)
    }
    /// --sit-wm-horizon: drop the bookkeeping of slots not written for the horizon.
    fn wm_forget(&mut self) {
        let cut = self.t.saturating_sub(self.cfg.wm_horizon);
        let old: Vec<(u32, u32, u32)> = self.stamp.iter().filter(|(_, &t)| t < cut).map(|(&k, _)| k).collect();
        for &(e, c, k) in &old {
            self.stamp.remove(&(e, c, k));
            self.slot_log.remove(&(e, c, k));
            if let Some(ks) = self.occ.get_mut(&(e, c)) {
                ks.retain(|&x| x != k);
                if ks.is_empty() {
                    self.occ.remove(&(e, c));
                }
            }
        }
        let occ = &self.occ;
        for ((e, _), cs) in self.dcl.iter_mut() {
            cs.retain(|&c| occ.contains_key(&(*e, c)));
        }
        self.dcl.retain(|_, cs| !cs.is_empty());
        let stamp = &self.stamp;
        for (e, l) in self.recent_slots.iter_mut() {
            l.retain(|&(c, k)| stamp.contains_key(&(*e, c, k)));
        }
        self.recent_slots.retain(|_, l| !l.is_empty());
        self.rcache.get_mut().clear();
    }
    fn stamp_of(&self, e: u32, c: u32, kd: u32) -> u64 {
        self.stamp.get(&(e, c, kd)).copied().unwrap_or(0)
    }

    /// The candidates for the next word: (value, weight), best first.
    pub fn recall(&mut self) -> Vec<(u32, f64)> {
        if self.fm(0) {
            self.rcache.get_mut().clear(); // keys may have drifted since the last read
        }
        let ctx = self.ctx_key();
        let cue: Vec<u32> = self.cue_words();
        if self.cfg.query {
            let mut props = self.query_recall(&cue);
            let mut given: Option<Vec<(u32, u32)>> = None; // what is answered, when it differs from props
            let at_q = self.hist.last().map_or(false, |&u| self.boundary[u as usize] && self.mood[u as usize] >= 8);
            if self.cfg.curious {
                // does memory know? answers like this one (form, margin bucket) right often enough
                let (num, den) = (self.cfg.curious_num as u64, self.cfg.curious_den as u64);
                let often = |r: u16, n: u16| (r as u64 + 1) * den > num * (n as u64 + 2);
                let ms = Self::margin_bucket(&self.qcands);
                self.cur_ks = (self.qform, ms);
                self.slot_top = self.qcands.first().map(|x| x.1);
                self.cur_kc = None;
                self.win_top = None;
                if at_q {
                    self.cur_q[3] += 1;
                }
                let a = *self.conf_s.get(&self.cur_ks).unwrap_or(&[0; 2]);
                let unsure = !often(a[0], a[1]);
                if !unsure && self.cfg.curious_practice && at_q && self.cfg.lookback > 0 {
                    // practice: look it up anyway, to learn to look things up (the answer stays memory's)
                    let q = self.qbits.clone();
                    self.wcands = self.window_cands(&cue, &q);
                    self.win_top = self.wcands.first().map(|x| x.1);
                    self.practiced = true;
                }
                if unsure {
                    // unsure: go and look it up in the window
                    self.cur_asked += 1;
                    let q = self.qbits.clone();
                    self.wcands = self.window_cands(&cue, &q);
                    self.win_top = self.wcands.first().map(|x| x.1);
                    let mw = Self::margin_bucket(&self.wcands);
                    let agree = self.win_top.is_some() && self.win_top == self.slot_top;
                    let kc = if self.cfg.curious_coarse { (self.qform, agree as u32) } else { (self.qform, ms * 64 + mw * 2 + agree as u32) };
                    self.cur_kc = Some(kc);
                    let c = *self.conf_c.get(&kc).unwrap_or(&[0; 3]);
                    // the source right more often here (cross-multiplied; same denominator), if often enough
                    let win_better = self.win_top.is_some() && c[1] > c[0];
                    let best_r = if win_better { c[1] } else { c[0] };
                    if at_q {
                        self.cur_q[0] += 1;
                    }
                    if !often(best_r, c[2]) {
                        self.cur_unknown += 1;
                        if at_q {
                            self.cur_q[2] += 1;
                        }
                        given = Some(vec![]);
                    } else if win_better {
                        self.cur_window += 1;
                        if at_q {
                            self.cur_q[1] += 1;
                        }
                        let lo = self.wcands.iter().map(|x| x.2).min().unwrap_or(0);
                        props = self.wcands.iter().map(|x| ((x.2 - lo + 1).min(u32::MAX as i64) as u32, x.1)).collect();
                    }
                }
            }
            if self.cfg.lookback > 0 && !self.cfg.curious {
                // the slots' answer, its margin, and the question form; then the window's answer; answer from the
                // window where, for this form and margin, the window has been right more often than the slots
                let q = self.qbits.clone();
                self.wcands = self.window_cands(&cue, &q);
                let w0 = cue.first().copied().unwrap_or(u32::MAX) as u64;
                let w2 = cue.get(2).copied().unwrap_or(u32::MAX) as u64;
                let w3 = cue.get(3).copied().unwrap_or(u32::MAX) as u64;
                // the key: the question form, and both sources' margin buckets
                let key = (hash(hash(hash(w0, 7), w2), w3), Self::margin_bucket(&self.qcands) * 16 + Self::margin_bucket(&self.wcands));
                self.arb_key = key;
                self.slot_top = self.qcands.first().map(|x| x.1);
                let a = *self.arb.get(&key).unwrap_or(&[0; 4]);
                // window accuracy > slot accuracy, by cross-multiplication, with a prior of 1 right in 2; merged:
                // the window wins where its best route outscores the slots' best under the shared weights
                let use_win = !self.wcands.is_empty()
                    && if self.cfg.merge {
                        self.qcands.first().map_or(true, |s0| self.wcands[0].2 > s0.2)
                    } else {
                        (a[2] as u64 + 1) * (a[1] as u64 + 2) > (a[0] as u64 + 1) * (a[3] as u64 + 2)
                    };
                if use_win {
                    self.used_window += 1;
                    if let Some(&u) = self.hist.last() {
                        if self.boundary[u as usize] && self.mood[u as usize] >= 8 {
                            self.used_window_q += 1;
                        }
                    }
                    let lo = self.wcands.iter().map(|x| x.2).min().unwrap_or(0);
                    props = self.wcands.iter().map(|x| ((x.2 - lo + 1).min(u32::MAX as i64) as u32, x.1)).collect();
                }
                if let Some(&u) = self.hist.last() {
                    if self.boundary[u as usize] && self.mood[u as usize] >= 8 {
                        self.questions_seen += 1;
                    }
                }
                if std::env::var("SIT_WDBG").is_ok() {
                    eprintln!("WDBG {} slot {} win {} bucket {} use {}", self.t, self.slot_top.map_or(-1, |x| x as i64), self.wcands.first().map_or(-1, |x| x.1 as i64), key.1, use_win as u8);
                }
            }
            // the store's own prediction (for the boundaries' mood counters), and what is answered
            self.last = props.clone();
            if let Some(g) = given {
                props = g;
            }
            let tot: f64 = props.iter().map(|x| x.0 as f64).sum();
            let mut out: Vec<(u32, f64)> = vec![];
            for (n, v) in props {
                match out.iter_mut().find(|x| x.0 == v) {
                    Some(x) => x.1 += n as f64 / tot.max(1.0),
                    None => out.push((v, n as f64 / tot.max(1.0))),
                }
            }
            return out;
        }
        let mut props: Vec<(u32, u32, i128)> = vec![];
        self.routes.clear();
        for (j, &e) in cue.iter().enumerate() {
            if !self.is_entity(e) {
                continue;
            }
            if self.cfg.bounce {
                // a loop of reads: stop, or follow the best-counted live slot of the current entity
                let (mut cur, mut lastc) = (e, 0u32);
                for step in 0..3u8 {
                    if step > 0 && *self.halt.get(&(ctx, self.kk(cur), step)).unwrap_or(&7) >= 8 {
                        props.push((lastc, cur, 0));
                        break;
                    }
                    let Some(h) = self.hop.get(&(ctx, j, step)) else { break };
                    let mut v: Vec<((u32, u32), u32)> = h.iter().map(|(&c, &n)| (c, n)).filter(|x| x.1 >= 2 && self.live(cur, x.0 .0, x.0 .1)).collect();
                    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                    v.truncate(6);
                    if v.is_empty() {
                        break;
                    }
                    if self.cfg.recency {
                        let maxn = v[0].1;
                        v.retain(|x| 2 * x.1 >= maxn);
                        v.sort_by(|a, b| self.stamp_of(cur, b.0 .0, b.0 .1).cmp(&self.stamp_of(cur, a.0 .0, a.0 .1)).then(b.1.cmp(&a.1)));
                    }
                    let ((c, kd), n) = v[0];
                    lastc = n;
                    let mut ex = cue.clone();
                    ex.push(cur);
                    match self.read_best(cur, c, kd, &ex) {
                        Some((val, _)) => cur = val,
                        None => break,
                    }
                }
                continue;
            }
            // (kind, first-slot recency, hits, value, misses, route)
            let mut cand: Vec<(u32, u64, u32, u32, u32, Route)> = vec![];
            if let Some(c1) = self.c1.get(&(ctx, j)) {
                let mut v: Vec<((u32, u32), u32)> = c1.iter().map(|(&c, &n)| (c, n)).filter(|x| x.1 >= 2).collect();
                v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                for &((c, kd), n) in v.iter().filter(|x| self.live(e, x.0 .0, x.0 .1)).take(self.cfg.nroutes.max(3)) {
                    if let Some((val, _)) = self.read_best(e, c, kd, &cue) {
                        let r: Route = (c, kd, u32::MAX, u32::MAX);
                        cand.push((kd, self.stamp_of(e, c, kd), n, val, *self.miss.get(&(ctx, j, r)).unwrap_or(&0), r));
                    }
                }
            }
            if let Some(c2) = self.c2.get(&(ctx, j)) {
                let mut v: Vec<(((u32, u32), (u32, u32)), u32)> = c2.iter().map(|(&c, &n)| (c, n)).filter(|x| x.1 >= 2).collect();
                v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                for &(((c1_, k1), (c2_, k2)), n) in v.iter().filter(|x| self.live(e, x.0 .0 .0, x.0 .0 .1)).take(self.cfg.nroutes.max(2)) {
                    if let Some((m, _)) = self.read_best(e, c1_, k1, &[e]) {
                        if !self.live(m, c2_, k2) {
                            continue;
                        }
                        if let Some((val, _)) = self.read_best(m, c2_, k2, &cue) {
                            let r: Route = (c1_, k1, c2_, k2);
                            cand.push((k2, self.stamp_of(e, c1_, k1), n, val, *self.miss.get(&(ctx, j, r)).unwrap_or(&0), r));
                        }
                    }
                }
            }
            for x in &cand {
                self.routes.push((ctx, j, x.5, x.3));
            }
            if let Ok(from) = std::env::var("SIT_DEBUG_FROM") {
                let last = *self.hist.last().unwrap_or(&0) as usize;
                if self.t >= from.parse::<u64>().unwrap_or(u64::MAX) && self.boundary[last] {
                    for x in &cand {
                        eprintln!("DBG {} j{} e{} route {:?} hits {} misses {} -> {} stamp {}", self.t, j, e, x.5, x.2, x.4, x.3, x.1);
                    }
                }
            }
            // a route's weight: its hits and misses; with --sit-precision ranked by hits / (hits + misses) (compared by
            // cross-multiplication), else by hits
            let better = |a: (u32, u32), b: (u32, u32), prec: bool| -> bool {
                // Go minus NoGo: hits minus misses (the striatal rule); a precision ranking let rare, lucky
                // routes win over well-tested ones
                if prec { (a.0 as i64 - a.1 as i64) > (b.0 as i64 - b.1 as i64) } else { a.0 > b.0 }
            };
            let prec = self.cfg.precision;
            if self.cfg.recency && !cand.is_empty() {
                // per kind: credible proposals (at least half the best), the newest first
                let mut bestw = (cand[0].2, cand[0].4);
                for x in &cand {
                    if better((x.2, x.4), bestw, prec) {
                        bestw = (x.2, x.4);
                    }
                }
                let half = |x: (u32, u32)| -> bool {
                    if prec { 2 * (x.0 as i64 - x.1 as i64) >= (bestw.0 as i64 - bestw.1 as i64) } else { self.cfg.cred * x.0 >= bestw.0 }
                };
                cand.retain(|x| half((x.2, x.4)));
                let mut kinds: Vec<u32> = cand.iter().map(|x| x.0).collect();
                kinds.sort_unstable();
                kinds.dedup();
                for kd in kinds {
                    let mut g: Vec<&(u32, u64, u32, u32, u32, Route)> = cand.iter().filter(|x| x.0 == kd).collect();
                    g.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)));
                    let mut gw = (g[0].2, g[0].4);
                    for x in &g {
                        if better((x.2, x.4), gw, prec) {
                            gw = (x.2, x.4);
                        }
                    }
                    // the group ranks by its best weight; inside it, newest first (kept by the stable sort)
                    for x in g {
                        props.push((gw.0, x.3, gw.1 as i128));
                    }
                }
            } else {
                props.extend(cand.into_iter().map(|x| (x.2, x.3, x.4 as i128)));
            }
        }
        // routes reliable enough (counts at least half the best), then the freshest (strongest) reading first
        if self.cfg.precision {
            // the third field holds the misses: best precision first, compared by cross-multiplication
            props.sort_by(|a, b| {
                let (ha, ma, hb, mb) = (a.0 as i128, a.2, b.0 as i128, b.2);
                (hb - mb).cmp(&(ha - ma)).then(b.0.cmp(&a.0))
            });
        } else {
            props.sort_by(|a, b| b.0.cmp(&a.0));
        }
        let props: Vec<(u32, u32)> = props.into_iter().map(|x| (x.0, x.1)).collect();
        self.last = props.clone();
        let tot: f64 = props.iter().map(|x| x.0 as f64).sum();
        let mut out: Vec<(u32, f64)> = vec![];
        for (n, v) in props {
            match out.iter_mut().find(|x| x.0 == v) {
                Some(x) => x.1 += n as f64 / tot.max(1.0),
                None => out.push((v, n as f64 / tot.max(1.0))),
            }
        }
        out
    }

    /// Whether part `b` of the fast path is on.
    #[inline]
    fn fm(&self, b: u32) -> bool {
        self.cfg.fast && (self.cfg.fast_mask >> b) & 1 == 1
    }
    /// A word sequence's hash (for clause texts and question spans).
    fn text_hash(ws: &[u32]) -> u64 {
        ws.iter().fold(0x7E47u64, |h, &w| hash(h, w as u64))
    }
    /// Read a word without predicting it (a shard's index pass, or a lead-in): everything is learned as usual except
    /// what needs a prediction (mood, route credit).
    pub fn observe_only(&mut self, w: u32) {
        self.last.clear();
        self.idx_only = true;
        self.observe(w);
        self.idx_only = false;
    }
    /// The query weights learned since `base` (a copy of this reader from earlier): for merging shards.
    pub fn learned_since(&self, base: &RowW) -> Vec<(u32, u16, i16)> {
        self.qr.delta(base)
    }
    /// A copy of the query weights (a shard's starting point, to measure what it learned).
    pub fn query_weights(&self) -> RowW {
        self.qr.clone()
    }
    /// Add query weights another shard learned.
    pub fn add_learned(&mut self, d: &[(u32, u16, i16)]) {
        self.qr.apply(d);
    }
    /// Whether the next word answers a question: the last word is a learned question boundary.
    pub fn at_question(&self) -> bool {
        self.hist.last().map_or(false, |&u| self.boundary[u as usize] && self.mood[u as usize] >= 8)
    }
    /// The time (words read).
    pub fn time(&self) -> u64 {
        self.t
    }
    /// Move the clock forward to `t` (a shard's reader starting further along the book; the words in between are
    /// read by other shards).
    pub fn jump_to(&mut self, t: u64) {
        self.t = self.t.max(t);
    }
    /// The long-term index written at times in [from, to): heads, head histories, clause texts and opener starts,
    /// all symbols stamped with positions (for merging the shards of a sharded read).
    pub fn export_index(&self, from: u64, to: u64) -> LtIndex {
        let inr = |t: u64| t >= from && t < to;
        let mut ix = LtIndex::default();
        for (&k, &(v, t, c, _)) in &self.heads_tab {
            if inr(t) {
                ix.heads.push((k, (v, t, c)));
            }
        }
        for (&k, h) in &self.posts {
            let e: Vec<(u64, u32)> = h.iter().filter(|x| inr(x.0)).copied().collect();
            if !e.is_empty() {
                ix.posts.push((k, e));
            }
        }
        for (&h, (occ, fw)) in &self.ctext {
            let e: Vec<u64> = occ.iter().filter(|&&x| inr(x)).copied().collect();
            if !e.is_empty() {
                ix.ctext.push((h, *fw, e));
            }
        }
        for (&fw, ps) in &self.opener_pos {
            let e: Vec<u64> = ps.iter().filter(|&&x| inr(x)).copied().collect();
            if !e.is_empty() {
                ix.opener.push((fw, e));
            }
        }
        ix.heads.sort_unstable_by_key(|x| x.0);
        ix.posts.sort_unstable_by_key(|x| x.0);
        ix.ctext.sort_unstable_by_key(|x| x.0);
        ix.opener.sort_unstable_by_key(|x| x.0);
        ix
    }
    /// Add a long-term index written before this reader's own entries (time order is kept: install the earlier
    /// shards first, before reading). Heads: the newer value wins.
    pub fn install_index(&mut self, ix: &LtIndex) {
        for &((e, key, k), (v, t, c)) in &ix.heads {
            if let Some(ent) = self.heads_tab.get_mut(&(e, key, k)) {
                if t > ent.1 {
                    (ent.0, ent.1, ent.2) = (v, t, c);
                    self.heads_of.get_mut(&e).unwrap()[ent.3 as usize] = (key, k, v, t, c);
                }
            } else {
                let l = self.heads_of.entry(e).or_default();
                self.heads_tab.insert((e, key, k), (v, t, c, l.len() as u32));
                l.push((key, k, v, t, c));
            }
        }
        for (k, h) in &ix.posts {
            self.posts.entry(*k).or_default().extend_from_slice(h);
        }
        for (h, fw, occ) in &ix.ctext {
            self.ctext.entry(*h).or_insert((vec![], *fw)).0.extend_from_slice(occ);
        }
        for (fw, ps) in &ix.opener {
            self.opener_pos.entry(*fw).or_default().extend_from_slice(ps);
        }
    }
    /// An opener type as a route feature: its word, or with --sit-roles its level (see `roles`).
    fn opener_feat(&self, fw: u32) -> u64 {
        if !self.cfg.roles {
            return fw as u64;
        }
        let bl = |x: u64| 64 - x.leading_zeros() as u64;
        let lvl = self.opener_pos.get(&fw).map_or(0, |v| {
            if v.len() < 2 {
                return 0;
            }
            bl(v[v.len() - 1] - v[0]).saturating_sub(bl(v.len() as u64 - 1))
        });
        1 << 40 | lvl
    }
    /// The end of the unit that starts at `st` with a clause whose first word is `fw`: the next clause with that
    /// first word.
    fn unit_end(&self, fw: u32, st: u64) -> u64 {
        self.opener_pos
            .get(&fw)
            .and_then(|v| {
                let at = v.partition_point(|&x| x <= st);
                v.get(at).copied()
            })
            .unwrap_or(u64::MAX)
    }
    /// The anchors of a cue (newest word first): spans of 1-4 words of the cue, in text order, that equal a stored
    /// clause's text, each with its unit (from the clause to the next clause with the same first word). A span gives
    /// its first and its latest occurrence; two spans give a nested anchor: the inner span's first occurrence inside
    /// the outer span's unit ("chapter twelve" inside "part three"), cut to it. As (start, end, first word, which:
    /// 0 first / 1 latest / 2 nested, span length, the outer span's first word or MAX, the outer span's length).
    fn anchors(&self, cue: &[u32]) -> Vec<(u64, u64, u32, u32, u32, u32, u32, u32)> {
        let text: Vec<u32> = cue.iter().rev().copied().collect();
        let mut spans: Vec<(&[u64], u32, u32, &[u64])> = vec![]; // (occurrences in the epoch, type, length, all)
        for len in 1..=4usize.min(text.len()) {
            for i in 0..=text.len() - len {
                let h = Self::text_hash(&text[i..i + len]);
                if let Some((occ, fw)) = self.ctext.get(&h) {
                    let mut o: &[u64] = occ;
                    if self.cfg.epoch {
                        // the occurrences in the current epoch of this type, if any
                        if let Some(&e) = self.epoch_at.get(fw) {
                            let at = occ.partition_point(|&x| x < e);
                            if at < occ.len() {
                                o = &occ[at..];
                            }
                        }
                    }
                    spans.push((o, *fw, len as u32, occ));
                }
            }
        }
        let mut out = vec![];
        for &(occ, fw, len, _) in &spans {
            let (first, latest) = (occ[0], *occ.last().unwrap());
            out.push((first, self.unit_end(fw, first), fw, 0, len, u32::MAX, 0, (occ.len() - 1).min(2) as u32));
            if latest != first {
                out.push((latest, self.unit_end(fw, latest), fw, 1, len, u32::MAX, 0, 0));
            }
        }
        out.truncate(6); // as without nesting
        // nested: an inner span's occurrence inside an outer span's unit (each outer at its first and latest start)
        let mut nested = vec![];
        if self.cfg.levels && self.cfg.nest {
            // the same rule at every level: labels by the level of their types, outermost first; from each root a
            // chain descends through every lower label found inside the current unit
            // a type's level: the log2 of its mean recurrence interval, counting the open one up to now, so a
            // type seen once is as high as the time since it was seen allows
            let now = self.t;
            let lev = |fw: u32| -> u64 {
                let bl = |x: u64| 64 - x.leading_zeros() as u64;
                self.opener_pos.get(&fw).map_or(0, |v| if v.is_empty() { 0 } else { bl(now - v[0]).saturating_sub(bl(v.len() as u64)) })
            };
            let mut order: Vec<usize> = (0..spans.len()).collect();
            order.sort_by_key(|&i| (std::cmp::Reverse(lev(spans[i].1)), i));
            for (oi, &r) in order.iter().enumerate() {
                let (oc, rfw, rlen, _) = spans[r];
                for (ai, &a) in [oc[0], *oc.last().unwrap()].iter().enumerate() {
                    if ai == 1 && oc.len() == 1 {
                        break;
                    }
                    let orank = if ai == 1 { 0 } else { (oc.len() - 1).min(2) as u32 };
                    let (mut lo, mut hi) = (a, self.unit_end(rfw, a));
                    let (mut depth, mut ifw, mut ilen) = (1u32, rfw, rlen);
                    let mut used = vec![rfw];
                    for &i in &order[oi + 1..] {
                        let (_, fw, len, all) = spans[i];
                        if used.contains(&fw) {
                            continue;
                        }
                        let at = all.partition_point(|&x| x < lo);
                        if let Some(&b) = all.get(at) {
                            if b < hi {
                                lo = b;
                                hi = self.unit_end(fw, b).min(hi);
                                depth += 1;
                                ifw = fw;
                                ilen = len;
                                used.push(fw);
                                // every depth is an anchor of its own: route credit picks how deep to trust
                                nested.push((lo, hi, ifw, depth, ilen, rfw, rlen, orank));
                            }
                        }
                    }
                    let _ = (ifw, ilen);
                    if std::env::var("SIT_LDBG").is_ok() {
                        let sp: Vec<String> = order.iter().map(|&i| format!("{}:{}x{}@L{}", spans[i].1, spans[i].2, spans[i].3.len(), lev(spans[i].1))).collect();
                        eprintln!("LDBG t {} root {} at {} rank {} -> [{}, {}) depth {} inner {} | spans {}", self.t, rfw, a, orank, lo, hi, depth, ifw, sp.join(" "));
                    }
                }
            }
        }
        for &(oc, ofw, olen, _) in spans.iter().filter(|_| self.cfg.nest && !self.cfg.levels) {
            for &(_, ifw, ilen, ic) in &spans {
                // the inner span is looked for inside the outer unit: all its occurrences, not just this epoch's
                if std::ptr::eq(oc, ic) || ofw == ifw {
                    continue;
                }
                for (ai, &a) in [oc[0], *oc.last().unwrap()].iter().enumerate() {
                    let ea = self.unit_end(ofw, a);
                    let at = ic.partition_point(|&x| x < a);
                    let orank = if ai == 1 { 0 } else { (oc.len() - 1).min(2) as u32 };
                    if let Some(&b) = ic.get(at) {
                        if b < ea {
                            nested.push((b, self.unit_end(ifw, b).min(ea), ifw, 2, ilen, ofw, olen, orank));
                        }
                    }
                    if oc.len() == 1 {
                        break;
                    }
                }
            }
        }
        nested.sort_unstable();
        nested.dedup();
        if self.cfg.scope {
            nested.reverse(); // keep the newest
        }
        nested.truncate(if self.cfg.levels { 8 } else { 4 });
        out.extend(nested);
        out
    }
    /// The cue words, newest first: the last `cue` words, or with `cue_clause` back to the clause start (at least
    /// `cue`, at most 12). The newest word may itself be a boundary ("?" before an answer); the walk stops at the
    /// first boundary behind it.
    fn cue_words(&self) -> Vec<u32> {
        let n = self.hist.len();
        let base = self.cfg.cue.min(n);
        if !self.cfg.cue_clause {
            return (0..base).map(|j| self.hist[n - 1 - j]).collect();
        }
        let mut out = vec![];
        for j in 0..n.min(12) {
            let u = self.hist[n - 1 - j];
            if j > 0 && j >= base && self.boundary[u as usize] {
                break;
            }
            out.push(u);
        }
        out
    }
    /// Learn from word w (after it has been predicted).
    pub fn observe(&mut self, w: u32) {
        if self.fm(0) {
            self.rcache.get_mut().clear();
        }
        if self.t % 20000 == 0 && std::env::var("SIT_TIMING").is_ok() {
            // elapsed time and the counters since the last report, with the sizes of the growing tables
            thread_local!(static LAST: std::cell::RefCell<(std::time::Instant, [u64; 6])> = std::cell::RefCell::new((std::time::Instant::now(), [0; 6])));
            let st = *self.stat.borrow();
            LAST.with(|l| {
                let mut l = l.borrow_mut();
                let dt = l.0.elapsed().as_millis();
                let q = (st[5] - l.1[5]).max(1);
                eprintln!(
                    "TIMING t {} {} ms / 20k words | per word: reads {} hits {} cands {} | heads {} posts {} ctext {} sq {} occ {} stamp {} classes {}",
                    self.t, dt, (st[0] - l.1[0]) / q, (st[1] - l.1[1]) / q, (st[2] - l.1[2]) / q,
                    self.heads_tab.len(), self.posts.values().map(|v| v.len()).sum::<usize>(), self.ctext.len(), self.sq.len(),
                    self.occ.len(), self.stamp.len(), self.class_count.len()
                );
                *l = (std::time::Instant::now(), st);
            });
        }
        let ctx = self.ctx_key();
        let cue: Vec<u32> = self.cue_words();
        self.wcount[w as usize] += 1;
        // NoGo: every route proposed for this word that was wrong
        // (only the routes behind the answer actually given, the top proposal, are blamed: the bandit's rule,
        // only the taken action's outcome is seen)
        let given = self.last.first().map(|x| x.1);
        for &(cx, j, r, v) in &std::mem::take(&mut self.routes) {
            if Some(v) == given && v != w {
                *self.miss.entry((cx, j, r)).or_insert(0) += 1;
            }
        }
        // mood: after a boundary, was the store's top proposal this word? (up on a hit, down on half the misses)
        // (not when reading without predicting: there was no proposal)
        // (nor once skill is frozen: the question detector is part of the skill, so a run of wrong answers in a hard
        // test cannot switch answering off)
        if let Some(&u) = self.hist.last().filter(|_| !self.idx_only && self.t < self.cfg.freeze_from) {
            if self.boundary[u as usize] {
                let ok = self.last.first().map(|x| x.1) == Some(w);
                let trivial = self.cfg.mood2 && self.succ_top.get(&u).map_or(false, |t| t.0 == w);
                let m = &mut self.mood[u as usize];
                if trivial {
                    // the bigram alone predicts it: no evidence about memory either way
                } else if ok {
                    *m = (*m + 1).min(15);
                } else if hash(self.t, u as u64 + 777) & 1 == 0 {
                    *m = m.saturating_sub(1);
                }
            }
        }
        // skippable words: was the word before last seen directly followed by this one?
        if self.cfg.skip && self.hist.len() >= 2 {
            let (a, u) = (self.hist[self.hist.len() - 2], self.hist[self.hist.len() - 1]);
            let e = &mut self.skip_n[u as usize];
            e.0 += 1;
            if self.succ.get(&(a, w)).copied().unwrap_or(0) > 0 {
                e.1 += 1;
            }
        }
        // the bigram counts behind mood2 and bnd2 (after the mood update, so they hold only the words before this one)
        if self.cfg.mood2 || self.cfg.bnd2 || self.cfg.skip {
            if let Some(&u) = self.hist.last() {
                let c = self.succ.entry((u, w)).or_insert(0);
                *c += 1;
                let c = *c;
                let t = self.succ_top.entry(u).or_insert((w, 0));
                if c > t.1 {
                    *t = (w, c);
                }
            }
        }
        if self.cfg.query && self.cfg.curious {
            let halve = |a: &mut [u16]| {
                if a[a.len() - 1] > 1000 {
                    a.iter_mut().for_each(|x| *x >>= 1);
                }
            };
            let sr = (self.slot_top == Some(w)) as u16;
            if std::env::var("SIT_CDBG").is_ok() && (self.cur_kc.is_some() || self.practiced) {
                eprintln!("CDBG {} slot {:?} win {:?} truth {} unsure {}", self.t - 1, self.slot_top, self.win_top, w, self.cur_kc.is_some() as u8);
            }
            let a = self.conf_s.entry(self.cur_ks).or_insert([0; 2]);
            a[0] += sr;
            a[1] += 1;
            halve(a);
            if let Some(kc) = self.cur_kc.take() {
                let wr = (self.win_top == Some(w)) as u16;
                let c = self.conf_c.entry(kc).or_insert([0; 3]);
                c[0] += sr;
                c[1] += wr;
                c[2] += 1;
                halve(c);
                let wc = std::mem::take(&mut self.wcands);
                self.window_hebb(&wc, w);
            } else if self.practiced {
                let wc = std::mem::take(&mut self.wcands);
                self.window_hebb(&wc, w);
            }
            self.practiced = false;
        }
        if self.cfg.query {
            if self.cfg.lookback > 0 && !self.cfg.curious {
                let wc = std::mem::take(&mut self.wcands);
                let q = self.qbits.clone();
                if self.cfg.merge {
                    // one pool: when the answer given was wrong, the best-scored route of either source that held w
                    // steps up and the given route steps down; the slots' own update then follows as usual
                    let given_win = self.last.first().map(|x| x.1) == wc.first().map(|x| x.1)
                        && wc.first().map_or(false, |t| self.qcands.first().map_or(true, |s0| t.2 > s0.2));
                    if given_win {
                        let top = wc[0].clone();
                        if top.1 != w {
                            let gw = wc.iter().find(|x| x.1 == w).map(|x| (x.0.clone(), x.2));
                            let gs = self.qcands.iter().find(|x| x.1 == w).map(|x| (x.0.clone(), x.2));
                            let good = match (gw, gs) {
                                (Some(a), Some(b)) => Some(if a.1 >= b.1 { a.0 } else { b.0 }),
                                (Some(a), None) => Some(a.0),
                                (None, Some(b)) => Some(b.0),
                                _ => None,
                            };
                            if let Some(gf) = good {
                                self.qupdate(&q, &gf, 1);
                                self.qupdate(&q, &top.0, -1);
                            }
                        }
                    } else if self.slot_top != Some(w) {
                        // the slots answered wrong: if the window held w, step its best such route up
                        if let Some(g) = wc.iter().find(|x| x.1 == w) {
                            let gf = g.0.clone();
                            self.qupdate(&q, &gf, 1);
                        }
                    }
                } else if let Some(top) = wc.first() {
                    if top.1 != w {
                        if let Some(good) = wc.iter().find(|x| x.1 == w) {
                            let (gf, tf) = (good.0.clone(), top.0.clone());
                            self.qupdate(&q, &gf, 1);
                            self.qupdate(&q, &tf, -1);
                        }
                    }
                }
                // the arbiter's counts: was each source right here?
                // (only where the two disagree: elsewhere the choice does not matter)
                let wt = wc.first().map(|x| x.1);
                if wt.is_some() && wt != self.slot_top {
                    let a = self.arb.entry(self.arb_key).or_insert([0; 4]);
                    a[0] += (self.slot_top == Some(w)) as u16;
                    a[1] += 1;
                    a[2] += (wt == Some(w)) as u16;
                    a[3] += 1;
                    if a[1] > 1000 {
                        a.iter_mut().for_each(|x| *x >>= 1);
                    }
                }
            }
            if self.t >= self.cfg.freeze_from {
                // frozen: the proposals are dropped unlearned
                self.qcands.clear();
                self.qbits.clear();
                self.qhops.clear();
                self.qranks.clear();
                self.qroute.clear();
            } else {
                self.query_learn(w);
            }
        }
        // which classes answer this context: those whose slot (of a cue word) holds w
        if !self.cfg.query && self.seen[w as usize] && !self.class_count.is_empty() {
            let mut top: Vec<(u64, u32)> = self.class_count.iter().enumerate().map(|(i, &c)| (c, i as u32)).collect();
            top.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
            let mut top: Vec<u32> = top.into_iter().take(self.cfg.ntop).map(|x| x.1).collect();
            top.extend(self.comp.values().copied());
            let kw = self.kk(w);
            for (j, &e) in cue.iter().enumerate() {
                if e == w || !self.seen[e as usize] || !self.is_entity(e) {
                    continue;
                }
                let pool1: Vec<u32> = if self.cfg.live { self.live_classes(e, Some(kw)) } else { top.clone() };
                let hits: Vec<u32> = pool1.iter().copied().filter(|&c| self.read_best(e, c, kw, &[e]).map(|x| x.0) == Some(w)).collect();
                if !hits.is_empty() {
                    let m = self.c1.entry((ctx, j)).or_default();
                    for &c in &hits {
                        *m.entry((c, kw)).or_insert(0) += 1;
                    }
                    if self.cfg.bounce {
                        let h = self.hop.entry((ctx, j, 0)).or_default();
                        for &c in &hits {
                            *h.entry((c, kw)).or_insert(0) += 1;
                        }
                        let x = self.halt.entry((ctx, kw, 1)).or_insert(7);
                        *x = (*x + 1).min(15);
                    }
                } else {
                    let t10: Vec<u32> = if self.cfg.live { self.live_classes(e, None) } else { top.iter().take(10).copied().collect() };
                    for &c1_ in &t10 {
                        let k1s: Vec<u32> = if self.cfg.kinds { self.occ.get(&(e, c1_)).cloned().unwrap_or_default() } else { vec![self.kn] };
                        for k1 in k1s {
                            let Some((m, _)) = self.read_best(e, c1_, k1, &[e]) else { continue };
                            if m == w {
                                continue;
                            }
                            let t2: Vec<u32> = if self.cfg.live { self.live_classes(m, Some(kw)) } else { t10.clone() };
                            for &c2_ in &t2 {
                                if self.read_best(m, c2_, kw, &[m]).map(|x| x.0) == Some(w) {
                                    *self.c2.entry((ctx, j)).or_default().entry(((c1_, k1), (c2_, kw))).or_insert(0) += 1;
                                    *self.g2.entry(((c1_, k1), (c2_, kw))).or_insert(0) += 1;
                                    if self.cfg.bounce {
                                        *self.hop.entry((ctx, j, 0)).or_default().entry((c1_, k1)).or_insert(0) += 1;
                                        *self.hop.entry((ctx, j, 1)).or_default().entry((c2_, kw)).or_insert(0) += 1;
                                        let km = self.kk(m);
                                        let x = self.halt.entry((ctx, km, 1)).or_insert(7);
                                        *x = x.saturating_sub(1); // m was not the answer
                                        let x = self.halt.entry((ctx, kw, 2)).or_insert(7);
                                        *x = (*x + 1).min(15);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // segmentation, meaning, kinds' frames, then the facts of w's clause (buffered until its boundary)
        let n = self.hist.len();
        if n >= 2 {
            let (x, u) = (self.hist[n - 2], self.hist[n - 1]);
            self.seg_update(x, u, w);
        }
        if !(self.cfg.fpool && !self.cfg.sense) {
            self.mn.observe(w); // the meaning module (float inside) is only needed for its codes
        }
        if (self.cfg.kinds || self.cfg.fpool) && n >= 1 {
            let u = self.hist[n - 1];
            self.frame_bump(w, Self::fb(1, u as u64));
            if n >= 2 {
                let two = if self.cfg.kframes { (1u64 << 40) + self.kk(self.hist[n - 2]) as u64 } else { self.hist[n - 2] as u64 };
                let f = Self::fb(2, two * 100_003 + u as u64);
                self.frame_bump(w, f);
            }
            self.frame_bump(u, Self::fb(3, w as u64));
        }
        let mut path: Vec<u32> = vec![];
        let back: Vec<u32> = self.clause.iter().rev().take(self.cfg.win).copied().collect();
        for s in back {
            if s != w {
                for dir in 0..2u32 {
                    let mut p = vec![dir];
                    if self.cfg.sense {
                        p.push(path.len() as u32);
                        p.extend(&path);
                        for (pi, &pw) in path.iter().enumerate() {
                            let ctx: Vec<u32> = [s, w].iter().chain(path.iter().enumerate().filter(|(k, _)| *k != pi).map(|(_, x)| x)).copied().collect();
                            let k = self.sense_pick(pw, &ctx, dir == 0).map(|k| k as u32).unwrap_or(u32::MAX);
                            p.push(k);
                        }
                    } else {
                        p.extend(&path);
                    }
                    let rare = if self.cfg.path_min > 0 && self.cfg.lsh {
                        let ph = p.iter().fold(0x77u64, |h, &x| hash(h, x as u64));
                        let mut m = u16::MAX;
                        for r in 0..4u64 {
                            let i = (r << 16 | hash(ph, r + 0x91) & 0xFFFF) as usize;
                            self.path_seen[i] = self.path_seen[i].saturating_add(1);
                            m = m.min(self.path_seen[i]);
                        }
                        (m as u32) < self.cfg.path_min && !self.path_class.contains_key(&p)
                    } else {
                        false
                    };
                    let c = if rare {
                        self.class_lookup(&p)
                    } else {
                        *self.path_count.entry(p.clone()).or_insert(0) += 1;
                        self.class_of(&p)
                    };
                    self.class_count[c as usize] += 1;
                    if dir == 0 {
                        self.buf.push((s, c, 0, w));
                    } else {
                        self.buf.push((w, c, 1, s));
                    }
                    let skippable = |x: u32| {
                        let e = self.skip_n.get(x as usize).copied().unwrap_or((0, 0));
                        self.cfg.skip && e.0 >= 16 && e.1 * 4 >= e.0 * 3
                    };
                    let pk = p.iter().enumerate().filter(|(i, &x)| *i == 0 || !skippable(x)).fold(0x9A7, |h, (_, &x)| hash(h, x as u64));
                    self.bufp.push((hash(pk, 77) & 0x7FFF_FFFF) as u32);
                }
            }
            path.push(s);
            if self.boundary[s as usize] {
                break;
            }
        }
        if self.boundary[w as usize] || self.buf.len() > 64 {
            if self.cfg.mood && self.boundary[w as usize] && self.mood[w as usize] >= 8 {
                self.buf.clear(); // a question: a cue, not a fact
                self.bufp.clear();
                self.dropped += 1;
            } else {
                if self.cfg.ranges && self.boundary[w as usize] {
                    // the clause's words (in order, the boundary left out), its start, and its first word
                    let mut ws: Vec<u32> = vec![];
                    for &u in self.clause.iter().rev() {
                        if self.boundary[u as usize] {
                            break;
                        }
                        ws.push(u);
                    }
                    ws.reverse();
                    if let Some(&fw0) = ws.first() {
                        // the clause's type: its first word, or with --sit-open-kind the kind the word area gives it
                        let fw = if self.cfg.open_kind { self.kk(fw0) } else { fw0 };
                        let st = self.t.saturating_sub(ws.len() as u64);
                        let h = Self::text_hash(&ws);
                        self.ctext.entry(h).or_insert((vec![], fw)).0.push(st);
                        self.opener_pos.entry(fw).or_default().push(st);
                        if self.cfg.epoch {
                            let f = *self.first_text.entry(fw).or_insert(h);
                            if f == h {
                                self.epoch_at.insert(fw, st); // the type's first text again: a new epoch
                            }
                        }
                    }
                }
                if self.cfg.anaphora {
                    self.resolve_standin();
                }
                self.commit();
                self.committed += 1;
            }
        }
        self.clause.push(w);
        if self.clause.len() > 16 {
            self.clause.drain(..8);
        }
        self.hist.push(w);
        if self.cfg.lookback > 0 {
            self.win.push(w);
            if self.win.len() > self.cfg.lookback {
                self.win.remove(0);
            }
        }
        if self.hist.len() > 64 {
            self.hist.drain(..32);
        }
        if !self.seen[w as usize] {
            self.seen[w as usize] = true;
            self.words.push(w);
            if self.cfg.vlsh {
                self.vband_add(w);
            }
            if self.fm(0) {
                self.bg_total += self.bg[w as usize] as i128;
                self.members.entry(self.kind[w as usize]).or_default().push(w);
            }
        }
        // the clocks: slot i steps its phase every clock * 2^(i mod 7) words
        if self.cfg.clock > 0 {
            for i in 0..self.cfg.k {
                let per = self.cfg.clock << (i % 7);
                if (self.t + 1) % per == 0 {
                    self.ctx_ph[i] = (self.ctx_ph[i] + 1) & 3;
                }
            }
        }
        // the context drifts at learned boundaries
        if self.boundary[w as usize] {
            for j in 0..self.cfg.bdrift {
                let h = hash(self.t * 31 + j as u64, 97);
                let i = (h % self.cfg.k as u64) as usize;
                self.ctx_off[i] = (hash(h, 98) % self.cfg.d as u64) as u32;
                self.ctx_ph[i] = (hash(h, 99) & 3) as u8;
            }
        }
        self.t += 1;
        if self.cfg.wm_horizon > 0 && self.t % (self.cfg.wm_horizon / 2).max(1) == 0 {
            self.wm_forget();
        }
        if self.cfg.sq_max > 0 {
            self.sq_open_sum += self.sq.len() as u64;
            self.sq_peak = self.sq_peak.max(self.sq.len());
            if self.t % 64 == 0 {
                self.sq_manage();
            }
        }
        if self.cfg.bnd2 && self.cfg.recluster > 0 && self.t % self.cfg.recluster as u64 == 0 {
            self.open_boundaries();
        }
        if self.cfg.recluster > 0 && self.t % self.cfg.recluster as u64 == 0 {
            self.recluster();
        }
    }

    /// Write the buffered bindings of a clause into their slots.
    /// --sit-anaphora: file the clause's facts under the referent of a stand-in first word (see `anaphora`).
    fn resolve_standin(&mut self) {
        let mut fw = None;
        for &u in self.clause.iter().rev() {
            if self.boundary[u as usize] {
                break;
            }
            fw = Some(u);
        }
        let Some(fw) = fw else { return };
        let k = self.kk(fw);
        let size = |k: u32| self.ksize.get(&k).copied().unwrap_or(0);
        if size(k) >= 3 {
            self.last_subj.insert(k, fw);
            return;
        }
        let code = self.kind_code(fw);
        let n: u64 = code.iter().map(|b| b.count_ones() as u64).sum();
        if n == 0 {
            return;
        }
        // the content kind whose leading word's contexts hold this word's
        let mut best: Option<(u64, u32)> = None;
        for (&c, ws) in &self.members {
            if c == k || size(c) < 3 {
                continue;
            }
            let Some(&lead) = ws.iter().max_by_key(|&&x| (self.wcount[x as usize], std::cmp::Reverse(x))) else { continue };
            let lc = self.kind_code(lead);
            let inter: u64 = code.iter().zip(&lc).map(|(a, b)| (a & b).count_ones() as u64).sum();
            if 4 * inter >= 3 * n && best.map_or(true, |b| inter > b.0) {
                best = Some((inter, c));
            }
        }
        let Some((_, c)) = best else { return };
        let Some(&ante) = self.last_subj.get(&c) else { return };
        for b in self.buf.iter_mut() {
            if b.0 == fw {
                b.0 = ante;
            }
            if b.3 == fw {
                b.3 = ante;
            }
        }
        self.anaphora_resolved += 1;
    }
    fn commit(&mut self) {
        let buf = std::mem::take(&mut self.buf);
        let bufp = std::mem::take(&mut self.bufp);
        for (bi, (x, c, dir, v)) in buf.into_iter().enumerate() {
            let kd = self.kk(v);
            if self.cfg.supersede {
                let others: Vec<u32> = self.dcl.get(&(x, dir as u8)).cloned().unwrap_or_default();
                for c2 in others {
                    if c2 == c {
                        continue;
                    }
                    if let Some(ks) = self.occ.get_mut(&(x, c2)) {
                        if let Some(pos) = ks.iter().position(|&q| q == kd) {
                            ks.remove(pos);
                            let none = self.nw as u32;
                            self.write(x, c2, kd, none); // detach: the old relation now reads as nothing
                        }
                    }
                }
            }
            self.write(x, c, kd, v);
            let ks = self.occ.entry((x, c)).or_default();
            if !ks.contains(&kd) {
                ks.push(kd);
            }
            let cl = self.dcl.entry((x, dir as u8)).or_default();
            if !cl.contains(&c) {
                cl.push(c);
            }
            self.stamp.insert((x, c, kd), self.t);
            self.touch(x, c, kd);
            // fast: long-term heads only for content values (a value kind of 3 or more members); heads to "." or "the"
            // multiply with every distinct clause tail and only grow the lists every lookup walks
            let content_val = !self.fm(5) || kd == self.kn || self.ksize.get(&kd).copied().unwrap_or(0) >= 3;
            let mut recurring = true;
            if self.cfg.lt_min > 0 && self.cfg.heads && c < COMP && content_val && self.ksize.get(&self.kk(x)).copied().unwrap_or(0) < 3 {
                recurring = false; // long-term facts are about content words (a kind of 3 or more)
            } else if self.cfg.lt_min > 0 && self.cfg.heads && c < COMP && content_val {
                // count the path in the sketch; a relation is long-term once it recurs
                let pk = bufp.get(bi).copied().unwrap_or(c) as u64;
                let mut m = u16::MAX;
                for r in 0..4u64 {
                    let i = (r << 16 | hash(pk, r + 0x5C) & 0xFFFF) as usize;
                    self.path_sketch[i] = self.path_sketch[i].saturating_add(1);
                    m = m.min(self.path_sketch[i]);
                }
                recurring = m as u32 >= self.cfg.lt_min;
            }
            if recurring && self.cfg.lt_credit > 0 && self.cfg.heads && c < COMP && content_val {
                // kept by credit: a path's first lt_credit bindings, then only if it has helped answer
                let key = if self.cfg.heads_path { bufp.get(bi).copied().unwrap_or(c) | 0x8000_0000 } else { c };
                if self.key_credit.get(&key).copied().unwrap_or(0) == 0 {
                    let n = self.key_writes.entry(key).or_insert(0);
                    *n += 1;
                    recurring = *n <= self.cfg.lt_credit;
                }
            }
            if self.cfg.heads && c < COMP && content_val && recurring {
                let key = if self.cfg.heads_path { bufp.get(bi).copied().unwrap_or(c) | 0x8000_0000 } else { c };
                let hk = if self.cfg.heads_nokind { self.kn } else { kd };
                let now = self.t;
                if let Some(ent) = self.heads_tab.get_mut(&(x, key, hk)) {
                    (ent.0, ent.1, ent.2) = (v, now, c);
                    self.heads_of.get_mut(&x).unwrap()[ent.3 as usize] = (key, hk, v, now, c);
                } else {
                    let l = self.heads_of.entry(x).or_default();
                    self.heads_tab.insert((x, key, hk), (v, now, c, l.len() as u32));
                    l.push((key, hk, v, now, c));
                }
                if self.cfg.ranges {
                    self.posts.entry((x, key, hk)).or_default().push((self.t, v));
                }
            }
            if self.cfg.sq_max > 0 {
                self.sq_on_fact(x, c, kd, v);
                if self.cfg.sq_mode > 0 && c < COMP && self.seen[v as usize] {
                    // the input proposes: the chains that start from this fact (x, c) = v, then v's newest slots
                    for (c2, k2, _) in self.newest_slots(v, 8) {
                        if c2 < COMP {
                            let e = self.sq_in.entry(((c, kd), (c2, k2))).or_insert((0, x));
                            e.0 = e.0.saturating_add(1);
                            e.1 = x;
                        }
                    }
                }
            }
            // data bouncing at write time: (x, c, v) and a counted pair ((c, kd), (c2, k2)) -> read v's (c2, k2)
            // slot now and keep the result in x's composite slot ("emma dropped the cup": where emma is now)
            if self.cfg.compose > 0 && c < COMP {
                let mut pairs: Vec<(((u32, u32), (u32, u32)), u32)> = self.g2.iter().map(|(&p, &n)| (p, n)).collect();
                pairs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                for &((p1, p2), _) in pairs.iter().take(self.cfg.compose) {
                    if p1 != (c, kd) {
                        continue;
                    }
                    if let Some((z, _)) = self.read_best(v, p2.0, p2.1, &[v, x]) {
                        let next = COMP + self.comp.len() as u32;
                        let cs = *self.comp.entry((p1, p2)).or_insert(next);
                        self.write(x, cs, p2.1, z);
                        let ks = self.occ.entry((x, cs)).or_default();
                        if !ks.contains(&p2.1) {
                            ks.push(p2.1);
                        }
                        let cl = self.dcl.entry((x, 2)).or_default();
                        if !cl.contains(&cs) {
                            cl.push(cs);
                        }
                        self.stamp.insert((x, cs, p2.1), self.t);
                        self.touch(x, cs, p2.1);
                    }
                }
            }
        }
    }
}
