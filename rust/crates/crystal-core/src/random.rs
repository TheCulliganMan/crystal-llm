use std::collections::VecDeque;

/// Divider samples are supplied by the frame/hardware adapter.  Keeping the
/// source behind this trait lets deterministic replay feed the exact samples
/// observed from the reference ROM without pretending to emulate CPU timing.
pub trait DividerSource {
    fn next_divider(&mut self) -> u8;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrystalRandomState {
    pub add: u8,
    pub sub: u8,
    pub carry: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayDivider {
    samples: VecDeque<u8>,
    last: u8,
    consumed: usize,
}

impl ReplayDivider {
    pub fn new(samples: impl IntoIterator<Item = u8>) -> Self {
        Self {
            samples: samples.into_iter().collect(),
            last: 0,
            consumed: 0,
        }
    }

    pub fn consumed(&self) -> usize {
        self.consumed
    }
}

impl DividerSource for ReplayDivider {
    fn next_divider(&mut self) -> u8 {
        let sample = self.samples.pop_front().unwrap_or(self.last);
        self.last = sample;
        self.consumed += 1;
        sample
    }
}

/// Crystal's single-player RNG from `home/random.asm`.
///
/// Each call consumes two divider samples, updates `hRandomAdd` with ADC,
/// updates `hRandomSub` with SBC, and returns the new subtraction byte.  The
/// carry flag is stateful because the assembly routine intentionally uses ADC
/// and SBC rather than ADD and SUB.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrystalRandom<S> {
    state: CrystalRandomState,
    source: S,
    random_calls: usize,
    rejected_ranges: usize,
}

impl<S> CrystalRandom<S>
where
    S: DividerSource,
{
    pub fn new(state: CrystalRandomState, source: S) -> Self {
        Self {
            state,
            source,
            random_calls: 0,
            rejected_ranges: 0,
        }
    }

    pub fn state(&self) -> CrystalRandomState {
        self.state
    }

    pub fn source(&self) -> &S {
        &self.source
    }

    pub fn source_mut(&mut self) -> &mut S {
        &mut self.source
    }

    pub fn random_calls(&self) -> usize {
        self.random_calls
    }

    pub fn rejected_ranges(&self) -> usize {
        self.rejected_ranges
    }

    pub fn random(&mut self) -> u8 {
        let divider_add = self.source.next_divider();
        let (partial_add, carry_from_divider) = self.state.add.overflowing_add(divider_add);
        let (add, carry_from_flag) = partial_add.overflowing_add(u8::from(self.state.carry));
        let add_overflow = carry_from_divider || carry_from_flag;
        self.state.add = add;

        let divider_sub = self.source.next_divider();
        let (partial_sub, borrow_from_divider) = self.state.sub.overflowing_sub(divider_sub);
        let (sub, borrow_from_flag) = partial_sub.overflowing_sub(u8::from(add_overflow));
        let sub_underflow = borrow_from_divider || borrow_from_flag;
        self.state.sub = sub;
        self.state.carry = sub_underflow;
        self.random_calls += 1;
        self.state.sub
    }

    /// Implements `RandomRange`, returning a value in `0..max`.
    pub fn random_range(&mut self, max: u8) -> u8 {
        assert!(max != 0, "Crystal RandomRange does not accept zero");
        let remainder = (256u16 % u16::from(max)) as u8;
        loop {
            // RandomRange reads hRandomAdd after Random, not the A register
            // value (which contains hRandomSub on return).
            self.random();
            let value = self.state.add;
            if value.checked_add(remainder).is_none() {
                self.rejected_ranges += 1;
                continue;
            }
            // SimpleDivide returns the remainder in A.  RandomRange therefore
            // returns `random % max`, not the quotient (the latter produces
            // values far outside the requested range for every max < 16).
            return value % max;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Random {
    seed: u32,
    crystal: Option<CrystalRandomState>,
    divider: u16,
}

impl Random {
    pub const fn new(seed: u32) -> Self {
        Self {
            seed,
            crystal: None,
            divider: 0,
        }
    }

    /// Construct the gameplay RNG used by the Rust runtime, matching the
    /// TypeScript HardwareRNG state transition and divider LFSR. `Random::new`
    /// remains a separate deterministic fixture path.
    pub const fn new_crystal(seed: u32) -> Self {
        Self {
            seed,
            crystal: Some(CrystalRandomState {
                add: seed as u8,
                sub: (seed >> 8) as u8,
                carry: false,
            }),
            divider: (seed >> 16) as u16,
        }
    }

    pub const fn seed(self) -> u32 {
        match self.crystal {
            Some(state) => {
                (state.add as u32)
                    | ((state.sub as u32) << 8)
                    | ((self.divider as u32) << 16)
            }
            None => self.seed,
        }
    }

    pub fn randrange(&mut self, max: u32) -> u32 {
        if max == 0 {
            return 0;
        }
        if self.crystal.is_some() {
            return self.crystal_randrange(max);
        }
        self.seed = self.seed.wrapping_mul(9301).wrapping_add(49_297) % 233_280;
        ((self.seed as f64 / 233_280.0) * max as f64).floor() as u32
    }

    /// Return the byte produced by Crystal's `BattleRandom` routine.  Unlike
    /// `RandomRange`, this exposes the subtraction register directly; battle
    /// code uses both the full byte and its high nibble for rejection rolls.
    pub fn battle_random_byte(&mut self) -> u8 {
        if self.crystal.is_some() {
            return self.crystal_random();
        }
        self.randrange(256) as u8
    }

    /// Return the two Crystal RNG bytes produced by one `Random` call.  The
    /// de-novo Pokérus routine checks both `hRandomAdd` and `hRandomSub`.
    pub fn crystal_random_add_sub(&mut self) -> (u8, u8) {
        if self.crystal.is_some() {
            let sub = self.crystal_random();
            let add = self.crystal.expect("crystal RNG state remains present").add;
            (add, sub)
        } else {
            let sub = self.randrange(256) as u8;
            (sub, sub)
        }
    }

    fn crystal_random(&mut self) -> u8 {
        let Some(mut state) = self.crystal else {
            unreachable!("crystal_random called for legacy RNG");
        };
        let divider_add = self.next_divider();
        state.add = state.add.wrapping_add(divider_add);
        state.sub = state.sub.wrapping_sub(divider_add);
        state.carry = false;
        self.crystal = Some(state);
        state.sub
    }

    fn next_divider(&mut self) -> u8 {
        if self.divider == 0 {
            self.divider = 0xace1;
        }
        let feedback = self.divider & 1;
        self.divider >>= 1;
        if feedback != 0 {
            self.divider ^= 0xb400;
        }
        self.divider as u8
    }

    fn crystal_randrange(&mut self, max: u32) -> u32 {
        let mut mask = 1u32;
        while mask < max {
            mask = (mask << 1) | 1;
        }
        let bit_length = 32 - mask.leading_zeros();
        let byte_count = bit_length.div_ceil(8).max(1);
        loop {
            let mut value = 0u32;
            for _ in 0..byte_count {
                value = (value << 8) | u32::from(self.crystal_random());
            }
            value &= mask;
            if value < max {
                return value;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CrystalRandom, CrystalRandomState, Random, ReplayDivider};

    #[test]
    fn crystal_random_updates_add_sub_and_carry_from_divider_samples() {
        let mut rng = CrystalRandom::new(
            CrystalRandomState {
                add: 0x10,
                sub: 0x80,
                carry: false,
            },
            ReplayDivider::new([0x20, 0x30, 0x30, 0x30]),
        );

        assert_eq!(rng.random(), 0x50);
        assert_eq!(
            rng.state(),
            CrystalRandomState {
                add: 0x30,
                sub: 0x50,
                carry: false
            }
        );
        assert_eq!(rng.random(), 0x20);
        assert_eq!(
            rng.state(),
            CrystalRandomState {
                add: 0x60,
                sub: 0x20,
                carry: false
            }
        );
        assert_eq!(rng.random_calls(), 2);
        assert_eq!(rng.source().consumed(), 4);
    }

    #[test]
    fn crystal_random_range_rejects_values_in_the_high_remainder_window() {
        let mut rng = CrystalRandom::new(
            CrystalRandomState {
                add: 0,
                sub: 0,
                carry: false,
            },
            ReplayDivider::new([250, 0, 3, 0, 7, 0]),
        );

        // The first two hRandomAdd values are in the rejected high window;
        // the third wraps to 4 and is the first accepted Crystal sample.
        assert_eq!(rng.random_range(10), 4);
        assert_eq!(rng.random_calls(), 3);
        assert_eq!(rng.rejected_ranges(), 2);
    }

    #[test]
    fn crystal_random_range_returns_simple_divide_remainder() {
        let mut rng = CrystalRandom::new(
            CrystalRandomState {
                add: 0,
                sub: 0,
                carry: false,
            },
            ReplayDivider::new([7, 0]),
        );
        assert_eq!(rng.random_range(3), 1);
    }

    #[test]
    fn battle_random_byte_exposes_the_subtraction_register() {
        let mut rng = Random::new_crystal(0x0000_8010);
        // The TypeScript runtime advances its divider once and applies the
        // same byte to both accumulators.  0xace1 steps to 0xe270, so hRandomSub
        // becomes 0x80 - 0x70 = 0x10.
        assert_eq!(rng.battle_random_byte(), 0x10);
    }

    #[test]
    fn matches_typescript_lcg_sequence() {
        let mut rng = Random::new(1);
        let values: Vec<u32> = (0..8).map(|_| rng.randrange(100)).collect();
        assert_eq!(values, vec![25, 54, 34, 95, 76, 12, 90, 70]);
        assert_eq!(rng.seed(), 164_697);
    }

    #[test]
    fn runtime_crystal_rng_uses_byte_width_state_and_roundtrips_seed() {
        let mut rng = Random::new_crystal(0x1234_5678);
        let before = rng.seed();
        let first = rng.randrange(100);
        let after = rng.seed();
        assert_ne!(before, after);
        assert!(first < 100);

        let mut resumed = Random::new_crystal(after);
        assert_eq!(resumed.randrange(100), rng.randrange(100));
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
