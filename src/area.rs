//! An area of matchers: the one local rule set the reader's levels share.
//!
//! Each matcher holds a mask over a 512-bit code. A matcher fires on an input when the overlap passes its
//! threshold, here a Jaccard fraction compared by cross-multiplication (5 x overlap >= th x union, no division).
//! Firing matchers inhibit each other: the best overlap wins (k-WTA, k = 1, lowest index on ties). The winner
//! takes a Hebbian swap (one mask bit outside the input out, one input bit outside the mask in); a matcher that
//! fired but was inhibited takes an anti-Hebbian swap (one shared bit out, one bit outside the input in), which
//! frees it to match something else. When nothing fires, a matcher is recruited with the input as its mask.
//!
//! Candidates come from an inverted index (bit -> matchers holding it), read only at the input's rarest bits: a
//! matcher can reach the threshold only if it holds one of the input's |x| - ceil(th/5 |x|) + 1 rarest bits
//! (prefix filtering), so the lookup reads short lists and never scans the area.
//!
//! The same area serves any level: words by their context codes (kinds), clauses by the codes of their words.

pub const W: usize = 8; // 8 x 64 = 512 bits
pub type Code = [u64; W];

fn pop(c: &Code) -> u32 {
    c.iter().map(|x| x.count_ones()).sum()
}
fn bits_of(c: &Code) -> Vec<u16> {
    let mut v = vec![];
    for (i, &x) in c.iter().enumerate() {
        let mut y = x;
        while y != 0 {
            let b = y.trailing_zeros();
            v.push((i * 64) as u16 + b as u16);
            y &= y - 1;
        }
    }
    v
}
fn has(c: &Code, b: u16) -> bool {
    c[(b >> 6) as usize] >> (b & 63) & 1 == 1
}
fn put(c: &mut Code, b: u16) {
    c[(b >> 6) as usize] |= 1 << (b & 63);
}
fn take(c: &mut Code, b: u16) {
    c[(b >> 6) as usize] &= !(1 << (b & 63));
}
fn mix(a: u64, b: u64) -> u64 {
    let mut z = a ^ b.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[derive(Clone)]
pub struct Area {
    /// The firing threshold, in fifths of Jaccard (2 = 2/5).
    pub th: u64,
    /// Swap one bit per win (Hebbian) and per inhibited firing (anti-Hebbian).
    pub hebb: bool,
    /// Sticky membership and consolidation: an item's incumbent matcher keeps it while it still fires (even if
    /// another fires stronger), and a matcher's swaps grow rarer as it wins (one in 2^(bitlen(wins)/2)).
    pub sticky: bool,
    /// 0 none; 1 a shared bit out, a bit outside the input in; 2 a contested bit (input, winner and loser) out,
    /// down to half the mask's first size; 3 as 2, but only for a loser in a near-tie (its Jaccard at least 3/4 of
    /// the winner's), so the swap happens only where the code is crowded.
    pub anti: u32,
    pub masks: Vec<Code>,
    /// A matcher's name: the item that recruited it (a kind keeps its name for as long as the matcher lives).
    pub names: Vec<u32>,
    pub wins: Vec<u32>,
    size0: Vec<u32>,
    post: Vec<Vec<u32>>,
    stamp: Vec<u64>,
    clock: u64,
    pub recruits: u64,
    pub swaps: u64,
    /// The area's own measure of crowding: presentations that found a winner, those where two or more matchers
    /// fired, and those where the runner-up came within 3/4 of the winner's overlap (a near-tie).
    pub presented: u64,
    pub cofired: u64,
    pub near: u64,
}

impl Area {
    pub fn new(th: u64) -> Self {
        Area { th, hebb: true, sticky: false, anti: 0, masks: vec![], names: vec![], wins: vec![], size0: vec![], post: vec![vec![]; W * 64], stamp: vec![], clock: 0, recruits: 0, swaps: 0, presented: 0, cofired: 0, near: 0 }
    }
    fn unindex(&mut self, m: u32, b: u16) {
        let l = &mut self.post[b as usize];
        if let Some(i) = l.iter().position(|&x| x == m) {
            l.swap_remove(i);
        }
    }
    fn fires(&self, inter: u64, uni: u64) -> bool {
        uni > 0 && 5 * inter >= self.th * uni
    }
    /// Whether matcher m may swap a bit now: always, or with `sticky` one time in 2^(bitlen(wins)/2).
    fn plastic(&self, m: u32, salt: u64) -> bool {
        if !self.sticky {
            return true;
        }
        let sh = (32 - self.wins[m as usize].leading_zeros()) / 2;
        mix(salt, 7 + m as u64) & ((1u64 << sh) - 1) == 0
    }
    /// Present an input; returns the winning matcher's name (`name` if a matcher was recruited for it) and index
    /// (u32::MAX for an empty input). `incumbent` is the matcher that won this item last time (u32::MAX if none).
    /// `salt` picks which bits a swap moves (deterministic). An empty input recruits nothing and is its own name.
    pub fn present(&mut self, x: &Code, name: u32, salt: u64, incumbent: u32) -> (u32, u32) {
        let xb = bits_of(x);
        if xb.is_empty() {
            return (name, u32::MAX);
        }
        // prefix: the rarest |x| - ceil(th |x| / 5) + 1 bits of the input
        let need = (self.th * xb.len() as u64 + 4) / 5;
        let keep = (xb.len() as u64 + 1).saturating_sub(need).max(1) as usize;
        let mut by: Vec<(usize, u16)> = xb.iter().map(|&b| (self.post[b as usize].len(), b)).collect();
        by.sort_unstable();
        self.clock += 1;
        let mut fired: Vec<(u64, u64, u32)> = vec![]; // (inter, uni, matcher)
        for &(_, b) in by.iter().take(keep) {
            for &m in &self.post[b as usize] {
                if self.stamp[m as usize] == self.clock {
                    continue;
                }
                self.stamp[m as usize] = self.clock;
                let mk = &self.masks[m as usize];
                let (mut inter, mut uni) = (0u64, 0u64);
                for i in 0..W {
                    inter += (mk[i] & x[i]).count_ones() as u64;
                    uni += (mk[i] | x[i]).count_ones() as u64;
                }
                if self.fires(inter, uni) {
                    fired.push((inter, uni, m));
                }
            }
        }
        if fired.is_empty() {
            let m = self.masks.len() as u32;
            self.masks.push(*x);
            self.names.push(name);
            self.wins.push(1);
            self.size0.push(xb.len() as u32);
            self.stamp.push(0);
            for &b in &xb {
                self.post[b as usize].push(m);
            }
            self.recruits += 1;
            return (name, m);
        }
        // inhibition: the best Jaccard wins (cross-multiplied), lowest index on ties
        let mut best = 0;
        for i in 1..fired.len() {
            let (a, b) = (fired[i], fired[best]);
            if a.0 * b.1 > b.0 * a.1 || a.0 * b.1 == b.0 * a.1 && a.2 < b.2 {
                best = i;
            }
        }
        if self.sticky {
            if let Some(i) = fired.iter().position(|f| f.2 == incumbent) {
                best = i; // the incumbent keeps the item while it still fires
            }
        }
        let win = fired[best].2;
        self.wins[win as usize] += 1;
        self.presented += 1;
        // near-tie: a loser whose Jaccard is at least 3/4 of the winner's (cross-multiplied)
        let (wi, wu) = (fired[best].0, fired[best].1);
        let near = |f: &(u64, u64, u32)| 4 * f.0 * wu >= 3 * wi * f.1;
        if fired.len() > 1 {
            self.cofired += 1;
            if fired.iter().enumerate().any(|(i, f)| i != best && near(f)) {
                self.near += 1;
            }
        }
        if self.hebb && self.plastic(win, salt) {
            // toward the input: a mask bit the input lacks out, an input bit the mask lacks in
            let mk = self.masks[win as usize];
            let out: Vec<u16> = bits_of(&mk).into_iter().filter(|&b| !has(x, b)).collect();
            let inn: Vec<u16> = xb.iter().copied().filter(|&b| !has(&mk, b)).collect();
            if !out.is_empty() && !inn.is_empty() {
                let o = out[(mix(salt, 1) % out.len() as u64) as usize];
                let n = inn[(mix(salt, 2) % inn.len() as u64) as usize];
                take(&mut self.masks[win as usize], o);
                put(&mut self.masks[win as usize], n);
                self.unindex(win, o);
                self.post[n as usize].push(win);
                self.swaps += 1;
            }
        }
        if self.anti == 2 || self.anti == 3 {
            // pull overlapping patterns apart: each inhibited matcher gives up one bit it contests with the winner
            // on this input (held by the input, the winner and itself), down to half its first size
            let wm = self.masks[win as usize];
            for (i, &(_, _, m)) in fired.iter().enumerate() {
                if i == best || self.anti == 3 && !near(&fired[i]) || !self.plastic(m, salt ^ 0x55) {
                    continue;
                }
                let mk = self.masks[m as usize];
                if 2 * pop(&mk) <= self.size0[m as usize] {
                    continue;
                }
                let contested: Vec<u16> = bits_of(&mk).into_iter().filter(|&b| has(x, b) && has(&wm, b)).collect();
                if contested.is_empty() {
                    continue;
                }
                let o = contested[(mix(salt ^ m as u64, 3) % contested.len() as u64) as usize];
                take(&mut self.masks[m as usize], o);
                self.unindex(m, o);
                self.swaps += 1;
            }
        }
        if self.anti == 1 {
            // away from the input for the inhibited: a shared bit out, a bit outside the input in
            for (i, &(_, _, m)) in fired.iter().enumerate() {
                if i == best {
                    continue;
                }
                let mk = self.masks[m as usize];
                let shared: Vec<u16> = bits_of(&mk).into_iter().filter(|&b| has(x, b)).collect();
                if shared.len() < 2 {
                    continue;
                }
                let o = shared[(mix(salt ^ m as u64, 3) % shared.len() as u64) as usize];
                // the new bit: hashed over the whole code until it falls outside both the input and the mask
                let mut n = (mix(salt ^ m as u64, 4) % (W as u64 * 64)) as u16;
                let mut tries = 0;
                while (has(x, n) || has(&mk, n)) && tries < 64 {
                    n = (mix(n as u64 ^ salt, 5 + tries) % (W as u64 * 64)) as u16;
                    tries += 1;
                }
                if tries == 64 {
                    continue;
                }
                take(&mut self.masks[m as usize], o);
                put(&mut self.masks[m as usize], n);
                self.unindex(m, o);
                self.post[n as usize].push(m);
                self.swaps += 1;
            }
        }
        (self.names[win as usize], win)
    }
    /// Crowding in one line: matchers, presentations, co-firing and near-tie rates (per mille).
    pub fn report(&self) -> String {
        let pm = |x: u64| if self.presented == 0 { 0 } else { 1000 * x / self.presented };
        format!("matchers {} | presented {} | co-fired {}‰ | near-ties {}‰ | swaps {}", self.len(), self.presented, pm(self.cofired), pm(self.near), self.swaps)
    }
    pub fn len(&self) -> usize {
        self.masks.len()
    }
    pub fn size_of(&self, m: usize) -> u32 {
        pop(&self.masks[m])
    }
}
