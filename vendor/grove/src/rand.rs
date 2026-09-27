//! Addressed randomness — the reason one seed can decide a whole plant.
//!
//! A stream is the wrong shape for a plant. Grove used to consume one PCG
//! stream as the tree grew, so the numbers a twig got depended on how many
//! branches had been drawn before it. That is fine for growing one fixed tree
//! and fatal for a clock: hide the branches a sapling has not sprouted yet and
//! every branch that remains gets different numbers — the whole tree
//! reshuffles instead of ageing.
//!
//! So randomness here is *addressed*, never consumed. There is no stream and
//! no state: every value is `hash(seed, key, slot)`, where the **key** is the
//! thing asking (a branch, a leaf site, a tip — its identity, hashed from its
//! path through the plant) and the **slot** is what the number is for. Ask for
//! the same address from anywhere, at any age, at any level of detail, in any
//! order, and the answer is the same. That is `same seed, same plant,
//! everywhere`, made structural instead of hoped for.

/// One deterministic value at an address.
///
/// Not cryptography — a geometry hash: cheap, and mixed well enough that
/// neighbouring keys and slots look unrelated.
pub fn hash(seed: u32, key: u32, slot: u32) -> u32 {
    let mut x = seed.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    x ^= key.wrapping_mul(2_654_435_761);
    x = x.wrapping_mul(1_597_334_677);
    x ^= slot.wrapping_mul(374_761_393).wrapping_add(0x9E37_79B9);
    // PCG-style output permutation: the bits get mixed, not merely shifted.
    let y = ((x >> ((x >> 28).wrapping_add(4))) ^ x).wrapping_mul(277_803_737);
    (y >> 22) ^ y
}

/// The value at an address as 0..1.
pub fn unit(seed: u32, key: u32, slot: u32) -> f32 {
    (hash(seed, key, slot) as f32) / (u32::MAX as f32)
}

/// A branch's identity, hashed from its parent's: path, not order. `kind`
/// separates a fork from a lateral so the two never collide, and `index` says
/// which child it is. Nothing in a key depends on how much of the plant has
/// been built, so a branch keeps its key as the plant ages and as coarser
/// LODs drop generations.
pub fn child_key(parent: u32, kind: u32, index: u32) -> u32 {
    hash(parent, kind.wrapping_mul(0x0501_1A1B) ^ 0xB17E_5EED, index.wrapping_add(1))
}

/// The draws belonging to one key: stateless, so the order they are asked in
/// cannot matter.
#[derive(Debug, Clone, Copy)]
pub struct Rnd {
    seed: u32,
    key: u32,
}

impl Rnd {
    pub fn new(seed: u32, key: u32) -> Self {
        Self { seed, key }
    }

    /// The value at a named slot, 0..1.
    pub fn at(&self, slot: u32) -> f32 {
        unit(self.seed, self.key, slot)
    }

    /// The value at a named slot, -1..1.
    pub fn signed(&self, slot: u32) -> f32 {
        self.at(slot) * 2.0 - 1.0
    }

    /// A smooth -1..1 wander sampled at `t` (0..1) from `controls` values in
    /// slots `slot..slot+controls`. A branch's wander is a curve the seed
    /// decided, not one value per segment: sample it at six points or at sixty
    /// and it is the same wander, so a coarse LOD bends the way LOD0 does.
    pub fn noise(&self, slot: u32, controls: u32, t: f32) -> f32 {
        let c = controls.max(2);
        let f = t.clamp(0.0, 1.0) * (c - 1) as f32;
        let i = (f.floor() as u32).min(c - 2);
        let u = f - i as f32;
        // Smoothstep between control values: no kinks at the joins.
        let w = u * u * (3.0 - 2.0 * u);
        let a = self.signed(slot + i);
        let b = self.signed(slot + i + 1);
        a + (b - a) * w
    }

    /// A key derived from this one for a sub-thing that has no branch of its
    /// own — a leaf site along a twig, a cluster, a socket's contents.
    pub fn sub(&self, kind: u32, index: u32) -> Rnd {
        Rnd { seed: self.seed, key: child_key(self.key, kind, index) }
    }
}
