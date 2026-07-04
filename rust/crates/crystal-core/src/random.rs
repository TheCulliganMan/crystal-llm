#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Random {
    seed: u32,
}

impl Random {
    pub const fn new(seed: u32) -> Self {
        Self { seed }
    }

    pub const fn seed(self) -> u32 {
        self.seed
    }

    pub fn randrange(&mut self, max: u32) -> u32 {
        if max == 0 {
            return 0;
        }
        self.seed = self.seed.wrapping_mul(9301).wrapping_add(49_297) % 233_280;
        ((self.seed as f64 / 233_280.0) * max as f64).floor() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::Random;

    #[test]
    fn matches_typescript_lcg_sequence() {
        let mut rng = Random::new(1);
        let values: Vec<u32> = (0..8).map(|_| rng.randrange(100)).collect();
        assert_eq!(values, vec![25, 54, 34, 95, 76, 12, 90, 70]);
        assert_eq!(rng.seed(), 164_697);
    }

    #[test]
    fn zero_upper_bound_is_total() {
        let mut rng = Random::new(7);
        assert_eq!(rng.randrange(0), 0);
        assert_eq!(rng.seed(), 7);
    }

    #[test]
    fn high_seed_uses_wrapping_lcg_arithmetic() {
        let mut rng = Random::new(0x1234_5678);
        assert_eq!(rng.randrange(16), 7);
        assert_eq!(rng.seed(), 116_457);
    }
}
