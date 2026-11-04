//! Seeded randomness: every choice the Floor makes comes from a seed, so
//! the same seed always gives the same office, bit for bit.

/// FNV-1a over `text`: a stable seed from words.
#[must_use]
pub fn hash(text: &str) -> u32 {
    let mut h: u32 = 2_166_136_261;
    for byte in text.bytes() {
        h ^= u32::from(byte);
        h = h.wrapping_mul(16_777_619);
    }
    h
}

/// A small, fast, seeded generator (mulberry32).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rng(pub u32);

impl Rng {
    /// A generator seeded from words.
    #[must_use]
    pub fn of(text: &str) -> Self {
        Self(hash(text))
    }

    /// The next number in `[0, 1)` (24 bits, so it never rounds up to 1).
    pub fn next_f32(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x6d2b_79f5);
        let mut t = self.0;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 0x3d));
        ((t ^ (t >> 14)) >> 8) as f32 / 16_777_216.0
    }

    /// A number in `[low, high)`.
    pub fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.next_f32()
    }

    /// A whole number in `0..n` (0 when `n` is 0).
    pub fn int(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        ((self.next_f32() * n as f32) as usize).min(n - 1)
    }

    /// True with chance `p`.
    pub fn chance(&mut self, p: f32) -> bool {
        self.next_f32() < p
    }

    /// One of `items` (the first when it has none to choose between).
    ///
    /// # Panics
    ///
    /// Never for a non-empty slice; callers pass constant, non-empty lists.
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        let at = self.int(items.len());
        &items[at]
    }
}

#[cfg(test)]
mod tests {
    use super::{Rng, hash};

    #[test]
    fn the_same_seed_gives_the_same_numbers() {
        let mut a = Rng::of("office|1");
        let mut b = Rng::of("office|1");
        for _ in 0..100 {
            assert_eq!(a.next_f32().to_bits(), b.next_f32().to_bits());
        }
        assert_ne!(hash("a"), hash("b"));
    }

    #[test]
    fn numbers_stay_in_their_range() {
        let mut r = Rng(7);
        for _ in 0..10_000 {
            let x = r.next_f32();
            assert!((0.0..1.0).contains(&x));
            assert!(r.int(5) < 5);
        }
        assert_eq!(r.int(0), 0);
    }
}
