use std::collections::VecDeque;
use std::convert::Infallible;
use web_time::Instant;

use serde::{Deserialize, Serialize};

/// Divider samples are supplied by the frame/hardware adapter.  Keeping the
/// source behind this trait lets deterministic replay feed the exact samples
/// observed from the reference ROM without pretending to emulate CPU timing.
pub trait DividerSource {
    type Error;

    fn next_divider(&mut self) -> Result<u8, Self::Error>;
}

/// Random operations used by the battle engine. Production battle execution
/// supplies [`ExactBattleRandom`]; fixture implementations may supply a
/// deterministic byte stream without becoming part of cartridge state.
pub trait BattleRandomSource {
    fn battle_random_byte(&mut self) -> u8;
    fn crystal_random_byte(&mut self, carry_in: bool) -> u8 {
        let _ = carry_in;
        self.battle_random_byte()
    }
}

impl<S> DividerSource for &mut S
where
    S: DividerSource + ?Sized,
{
    type Error = S::Error;

    fn next_divider(&mut self) -> Result<u8, Self::Error> {
        (**self).next_divider()
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrystalRandomState {
    pub add: u8,
    pub sub: u8,
}

/// A live, process-owned approximation of the Game Boy's free-running DIV
/// register. It samples monotonic host time at the DMG divider's 16384 Hz
/// increment rate; unlike the retired seed facade, no game seed or frame
/// counter is repurposed as hardware timing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveDivider {
    epoch: Instant,
}

impl LiveDivider {
    pub fn new() -> Self {
        Self {
            epoch: Instant::now(),
        }
    }
}

impl Default for LiveDivider {
    fn default() -> Self {
        Self::new()
    }
}

impl DividerSource for LiveDivider {
    type Error = Infallible;

    fn next_divider(&mut self) -> Result<u8, Self::Error> {
        const DIVIDER_HZ: u128 = 16_384;
        const NANOS_PER_SECOND: u128 = 1_000_000_000;
        let ticks = self.epoch.elapsed().as_nanos().saturating_mul(DIVIDER_HZ) / NANOS_PER_SECOND;
        Ok(ticks as u8)
    }
}

/// Records every successful DIV read from an injected source. The recorded
/// bytes are the deterministic boundary persisted in runtime commands; replay
/// feeds them back through [`ReplayDivider`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordingDivider<S> {
    source: S,
    samples: Vec<u8>,
}

impl<S> RecordingDivider<S> {
    pub fn new(source: S) -> Self {
        Self {
            source,
            samples: Vec::new(),
        }
    }

    pub fn samples(&self) -> &[u8] {
        &self.samples
    }

    pub fn into_parts(self) -> (S, Vec<u8>) {
        (self.source, self.samples)
    }
}

impl<S> DividerSource for RecordingDivider<S>
where
    S: DividerSource,
{
    type Error = S::Error;

    fn next_divider(&mut self) -> Result<u8, Self::Error> {
        let sample = self.source.next_divider()?;
        self.samples.push(sample);
        Ok(sample)
    }
}

/// The register and carry flag returned by `Random`.
///
/// Carry is deliberately not part of [`CrystalRandomState`]. It belongs to
/// the LR35902 caller: `Random` consumes the carry that is live at the call
/// boundary and returns the carry produced by its final `sbc` instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrystalRandomOutput {
    pub value: u8,
    pub carry_out: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("divider replay exhausted after {consumed} samples")]
pub struct ReplayDividerExhausted {
    pub consumed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayDivider {
    samples: VecDeque<u8>,
    consumed: usize,
}

impl ReplayDivider {
    pub fn new(samples: impl IntoIterator<Item = u8>) -> Self {
        Self {
            samples: samples.into_iter().collect(),
            consumed: 0,
        }
    }

    pub fn consumed(&self) -> usize {
        self.consumed
    }

    pub fn remaining(&self) -> usize {
        self.samples.len()
    }
}

impl DividerSource for ReplayDivider {
    type Error = ReplayDividerExhausted;

    fn next_divider(&mut self) -> Result<u8, Self::Error> {
        let sample = self.samples.pop_front().ok_or(ReplayDividerExhausted {
            consumed: self.consumed,
        })?;
        self.consumed += 1;
        Ok(sample)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RuntimeDividerError {
    #[error(transparent)]
    Replay(#[from] ReplayDividerExhausted),
}

/// Runtime-owned divider input. Live play samples monotonic host time; tests
/// and deterministic local harnesses may inject an exact finite trace. Normal
/// command replay still uses `ReplayDivider` directly at the command boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeDividerSource {
    Live(LiveDivider),
    Replay(ReplayDivider),
}

impl RuntimeDividerSource {
    pub fn live() -> Self {
        Self::Live(LiveDivider::new())
    }

    pub fn replay(samples: impl IntoIterator<Item = u8>) -> Self {
        Self::Replay(ReplayDivider::new(samples))
    }
}

impl Default for RuntimeDividerSource {
    fn default() -> Self {
        Self::live()
    }
}

impl DividerSource for RuntimeDividerSource {
    type Error = RuntimeDividerError;

    fn next_divider(&mut self) -> Result<u8, Self::Error> {
        match self {
            Self::Live(source) => match source.next_divider() {
                Ok(sample) => Ok(sample),
                Err(never) => match never {},
            },
            Self::Replay(source) => source.next_divider().map_err(Into::into),
        }
    }
}

/// Crystal's single-player RNG from `home/random.asm`.
///
/// Each call consumes two divider samples, updates `hRandomAdd` with ADC,
/// updates `hRandomSub` with SBC, and returns the new subtraction byte plus the
/// SBC carry. The caller supplies the carry live at the call boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrystalRandom<S> {
    state: CrystalRandomState,
    source: S,
    random_calls: usize,
    rejected_ranges: usize,
}

/// Adapts the cartridge RNG to the battle engine while retaining divider
/// failures until the outer transactional boundary can reject the operation.
/// A terminal byte is returned after the first failure so rejection loops
/// finish; the computed state is never committed when `divider_error` is set.
pub struct ExactBattleRandom<'a, S>
where
    S: DividerSource + ?Sized,
{
    rng: CrystalRandom<&'a mut S>,
    divider_error: Option<String>,
}

impl<'a, S> ExactBattleRandom<'a, S>
where
    S: DividerSource + ?Sized,
    S::Error: std::fmt::Display,
{
    pub fn new(state: CrystalRandomState, source: &'a mut S) -> Self {
        Self {
            rng: CrystalRandom::new(state, source),
            divider_error: None,
        }
    }

    pub fn state(&self) -> CrystalRandomState {
        self.rng.state()
    }

    pub fn divider_error(&self) -> Option<&str> {
        self.divider_error.as_deref()
    }

    fn remember<T>(&mut self, result: Result<T, S::Error>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                if self.divider_error.is_none() {
                    self.divider_error = Some(error.to_string());
                }
                None
            }
        }
    }
}

impl<S> BattleRandomSource for ExactBattleRandom<'_, S>
where
    S: DividerSource + ?Sized,
    S::Error: std::fmt::Display,
{
    fn battle_random_byte(&mut self) -> u8 {
        if self.divider_error.is_some() {
            return u8::MAX;
        }
        let result = self.rng.battle_random();
        self.remember(result).unwrap_or(u8::MAX)
    }

    fn crystal_random_byte(&mut self, carry_in: bool) -> u8 {
        if self.divider_error.is_some() {
            return u8::MAX;
        }
        let result = self.rng.random(carry_in).map(|output| output.value);
        self.remember(result).unwrap_or(u8::MAX)
    }
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

    pub fn into_parts(self) -> (CrystalRandomState, S) {
        (self.state, self.source)
    }

    pub fn random_calls(&self) -> usize {
        self.random_calls
    }

    pub fn rejected_ranges(&self) -> usize {
        self.rejected_ranges
    }

    pub fn random(&mut self, carry_in: bool) -> Result<CrystalRandomOutput, S::Error> {
        let divider_add = self.source.next_divider()?;
        let (partial_add, carry_from_divider) = self.state.add.overflowing_add(divider_add);
        let (add, carry_from_flag) = partial_add.overflowing_add(u8::from(carry_in));
        let add_overflow = carry_from_divider || carry_from_flag;
        self.state.add = add;

        let divider_sub = self.source.next_divider()?;
        let (partial_sub, borrow_from_divider) = self.state.sub.overflowing_sub(divider_sub);
        let (sub, borrow_from_flag) = partial_sub.overflowing_sub(u8::from(add_overflow));
        let sub_underflow = borrow_from_divider || borrow_from_flag;
        self.state.sub = sub;
        self.random_calls += 1;
        Ok(CrystalRandomOutput {
            value: self.state.sub,
            carry_out: sub_underflow,
        })
    }

    /// Implements the non-link branch of `_BattleRandom`.
    ///
    /// `_BattleRandom` tests `wLinkMode` with `and a`, which clears carry,
    /// before tail-calling `Random` in a local battle.
    pub fn battle_random(&mut self) -> Result<u8, S::Error> {
        self.random(false).map(|output| output.value)
    }

    /// Implements `RandomRange`, returning a value in `0..max`.
    pub fn random_range(&mut self, max: u8) -> Result<u8, S::Error> {
        assert!(max != 0, "Crystal RandomRange does not accept zero");
        let remainder = (256u16 % u16::from(max)) as u8;
        loop {
            // The modulo setup's final `add c` always carries. A rejected
            // sample's `add b` also carries, so every loop iteration enters
            // Random with carry set.
            self.random(true)?;
            // RandomRange reads hRandomAdd after Random, not the A register
            // value (which contains hRandomSub on return).
            let value = self.state.add;
            if value.overflowing_add(remainder).1 {
                self.rejected_ranges += 1;
                continue;
            }
            // SimpleDivide returns the remainder in A.  RandomRange therefore
            // returns `random % max`, not the quotient (the latter produces
            // values far outside the requested range for every max < 16).
            return Ok(value % max);
        }
    }

    /// Implements the script interpreter's distinct `Script_random` loop.
    ///
    /// Its first call inherits carry from the 256/modulus setup. A rejected
    /// hRandomAdd byte reaches the loop through `cp threshold; jr nc`, so the
    /// retry enters `Random` with carry clear rather than set like RandomRange.
    pub fn script_random_range(&mut self, max: u8) -> Result<u8, S::Error> {
        assert!(max != 0, "Crystal Script_random does not accept zero here");
        let threshold = 256u16 - (256u16 % u16::from(max));
        let mut carry_in = true;
        loop {
            self.random(carry_in)?;
            let value = self.state.add;
            if u16::from(value) < threshold {
                return Ok(value % max);
            }
            self.rejected_ranges += 1;
            carry_in = false;
        }
    }
}

pub const LINK_BATTLE_RANDOM_SEED_COUNT: usize = 10;
pub const LINK_BATTLE_RANDOM_OUTPUTS_PER_GENERATION: u8 = 9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LinkBattleRandomStateError {
    #[error("link battle random count {count} is outside the canonical 0..=8 range")]
    InvalidCount { count: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LinkBattleRandomState {
    pub seeds: [u8; LINK_BATTLE_RANDOM_SEED_COUNT],
    pub count: u8,
}

impl<'de> Deserialize<'de> for LinkBattleRandomState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawLinkBattleRandomState {
            seeds: [u8; LINK_BATTLE_RANDOM_SEED_COUNT],
            count: u8,
        }

        let raw = RawLinkBattleRandomState::deserialize(deserializer)?;
        if raw.count >= LINK_BATTLE_RANDOM_OUTPUTS_PER_GENERATION {
            return Err(serde::de::Error::custom(
                LinkBattleRandomStateError::InvalidCount { count: raw.count },
            ));
        }
        Ok(Self {
            seeds: raw.seeds,
            count: raw.count,
        })
    }
}

/// Crystal's link-battle branch of `_BattleRandom`.
///
/// The cartridge stores ten independently evolving seeds, but emits only
/// indices 0 through 8. Returning index 8 resets the cursor and advances all
/// ten seeds with `x = x * 5 + 1 (mod 256)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkBattleRandom {
    seeds: [u8; LINK_BATTLE_RANDOM_SEED_COUNT],
    count: u8,
}

impl LinkBattleRandom {
    pub fn new(
        seeds: [u8; LINK_BATTLE_RANDOM_SEED_COUNT],
        count: u8,
    ) -> Result<Self, LinkBattleRandomStateError> {
        if count >= LINK_BATTLE_RANDOM_OUTPUTS_PER_GENERATION {
            return Err(LinkBattleRandomStateError::InvalidCount { count });
        }
        Ok(Self { seeds, count })
    }

    pub const fn from_fresh_seeds(seeds: [u8; LINK_BATTLE_RANDOM_SEED_COUNT]) -> Self {
        Self { seeds, count: 0 }
    }

    pub fn from_state(state: &LinkBattleRandomState) -> Result<Self, LinkBattleRandomStateError> {
        Self::new(state.seeds, state.count)
    }

    pub const fn state(&self) -> LinkBattleRandomState {
        LinkBattleRandomState {
            seeds: self.seeds,
            count: self.count,
        }
    }

    pub const fn seeds(&self) -> &[u8; LINK_BATTLE_RANDOM_SEED_COUNT] {
        &self.seeds
    }

    pub const fn count(&self) -> u8 {
        self.count
    }

    pub fn battle_random(&mut self) -> u8 {
        let value = self.seeds[usize::from(self.count)];
        let next_count = self.count + 1;
        if next_count < LINK_BATTLE_RANDOM_OUTPUTS_PER_GENERATION {
            self.count = next_count;
            return value;
        }

        self.count = 0;
        for seed in &mut self.seeds {
            *seed = seed.wrapping_mul(5).wrapping_add(1);
        }
        value
    }
}

/// Deterministic LCG retained only for isolated test fixtures.
/// Production code uses [`CrystalRandom`] and [`LinkBattleRandom`].
#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Random {
    seed: u32,
}

#[cfg(any(test, feature = "test-fixtures"))]
impl Random {
    /// Construct the deterministic LCG used by isolated test fixtures.
    ///
    /// The constructor is deliberately absent from production builds so a
    /// missing divider source cannot silently fall back to the LCG.
    #[cfg(any(test, feature = "test-fixtures"))]
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
        self.fixture_lcg_randrange(max)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    fn fixture_lcg_randrange(&mut self, max: u32) -> u32 {
        self.seed = self.seed.wrapping_mul(9301).wrapping_add(49_297) % 233_280;
        ((self.seed as f64 / 233_280.0) * max as f64).floor() as u32
    }

    #[cfg(not(any(test, feature = "test-fixtures")))]
    fn fixture_lcg_randrange(&mut self, _max: u32) -> u32 {
        unreachable!("production code cannot construct the fixture LCG")
    }

    /// Return a fixture byte. Exact replay code uses
    /// [`CrystalRandom::battle_random`] or [`LinkBattleRandom::battle_random`].
    pub fn battle_random_byte(&mut self) -> u8 {
        self.randrange(256) as u8
    }

    /// Return the same fixture byte in both accumulator positions.
    pub fn crystal_random_add_sub(&mut self) -> (u8, u8) {
        let value = self.randrange(256) as u8;
        (value, value)
    }
}

#[cfg(any(test, feature = "test-fixtures"))]
impl BattleRandomSource for Random {
    fn battle_random_byte(&mut self) -> u8 {
        Random::battle_random_byte(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BattleRandomSource, CrystalRandom, CrystalRandomOutput, CrystalRandomState,
        ExactBattleRandom, LinkBattleRandom, Random, RecordingDivider, ReplayDivider,
        ReplayDividerExhausted,
    };

    #[test]
    fn exact_battle_random_reads_the_divider_and_updates_cartridge_state() {
        let mut divider = ReplayDivider::new([0x20, 0x10]);
        let mut rng = ExactBattleRandom::new(
            CrystalRandomState {
                add: 0x10,
                sub: 0x80,
            },
            &mut divider,
        );

        assert_eq!(rng.battle_random_byte(), 0x70);
        assert_eq!(
            rng.state(),
            CrystalRandomState {
                add: 0x30,
                sub: 0x70
            }
        );
        assert_eq!(rng.divider_error(), None);
        drop(rng);
        assert_eq!(divider.consumed(), 2);
    }

    #[test]
    fn exact_battle_random_latches_exhaustion_without_reading_again() {
        let mut divider = ReplayDivider::new([0x20]);
        let mut rng = ExactBattleRandom::new(CrystalRandomState::default(), &mut divider);

        assert_eq!(rng.battle_random_byte(), u8::MAX);
        assert_eq!(rng.battle_random_byte(), u8::MAX);
        assert_eq!(
            rng.divider_error(),
            Some("divider replay exhausted after 1 samples")
        );
        drop(rng);
        assert_eq!(divider.consumed(), 1);
    }

    #[test]
    fn crystal_random_updates_add_sub_and_carry_from_divider_samples() {
        let mut rng = CrystalRandom::new(
            CrystalRandomState {
                add: 0x10,
                sub: 0x80,
            },
            ReplayDivider::new([0x20, 0x30, 0x30, 0x30]),
        );

        assert_eq!(
            rng.random(false).expect("complete divider trace"),
            CrystalRandomOutput {
                value: 0x50,
                carry_out: false,
            }
        );
        assert_eq!(
            rng.state(),
            CrystalRandomState {
                add: 0x30,
                sub: 0x50,
            }
        );
        assert_eq!(
            rng.random(false).expect("complete divider trace"),
            CrystalRandomOutput {
                value: 0x20,
                carry_out: false,
            }
        );
        assert_eq!(
            rng.state(),
            CrystalRandomState {
                add: 0x60,
                sub: 0x20,
            }
        );
        assert_eq!(rng.random_calls(), 2);
        assert_eq!(rng.source().consumed(), 4);
    }

    #[test]
    fn crystal_random_carry_is_owned_by_the_call_boundary_not_rng_state() {
        let mut rng = CrystalRandom::new(
            CrystalRandomState { add: 0xff, sub: 0 },
            ReplayDivider::new([0, 0, 0, 0]),
        );

        assert_eq!(
            rng.random(true).expect("complete divider trace"),
            CrystalRandomOutput {
                value: 0xff,
                carry_out: true,
            }
        );
        assert_eq!(rng.state(), CrystalRandomState { add: 0, sub: 0xff });

        assert_eq!(
            rng.random(false).expect("complete divider trace"),
            CrystalRandomOutput {
                value: 0xff,
                carry_out: false,
            }
        );
        assert_eq!(rng.state(), CrystalRandomState { add: 0, sub: 0xff });
    }

    #[test]
    fn battle_random_clears_carry_before_calling_random() {
        let mut rng = CrystalRandom::new(
            CrystalRandomState { add: 0xff, sub: 0 },
            ReplayDivider::new([0, 0]),
        );

        assert_eq!(rng.battle_random().expect("complete divider trace"), 0);
        assert_eq!(rng.state(), CrystalRandomState { add: 0xff, sub: 0 });
    }

    #[test]
    fn replay_divider_exhaustion_is_an_error_and_never_repeats_the_last_sample() {
        let mut rng = CrystalRandom::new(
            CrystalRandomState {
                add: 0x10,
                sub: 0x80,
            },
            ReplayDivider::new([0x20]),
        );

        assert_eq!(
            rng.random(false).expect_err("second DIV read is absent"),
            ReplayDividerExhausted { consumed: 1 }
        );
        assert_eq!(
            rng.state(),
            CrystalRandomState {
                add: 0x30,
                sub: 0x80
            }
        );
        assert_eq!(rng.source().consumed(), 1);
        assert_eq!(rng.source().remaining(), 0);

        assert_eq!(
            rng.random(false)
                .expect_err("exhausted trace stays exhausted"),
            ReplayDividerExhausted { consumed: 1 }
        );
        assert_eq!(
            rng.state(),
            CrystalRandomState {
                add: 0x30,
                sub: 0x80
            }
        );
        assert_eq!(rng.source().consumed(), 1);
    }

    #[test]
    fn recording_divider_captures_only_successful_source_reads_for_replay() {
        let source = ReplayDivider::new([0x12, 0x34]);
        let mut rng = CrystalRandom::new(
            CrystalRandomState { add: 1, sub: 2 },
            RecordingDivider::new(source),
        );

        assert_eq!(
            rng.random(false).expect("complete source trace"),
            CrystalRandomOutput {
                value: 0xce,
                carry_out: true,
            }
        );
        assert_eq!(rng.source().samples(), &[0x12, 0x34]);

        assert_eq!(
            rng.random(false)
                .expect_err("underlying source is exhausted"),
            ReplayDividerExhausted { consumed: 2 }
        );
        assert_eq!(rng.source().samples(), &[0x12, 0x34]);
    }

    #[test]
    fn crystal_random_range_rejects_values_in_the_high_remainder_window() {
        let mut rng = CrystalRandom::new(
            CrystalRandomState { add: 0, sub: 0x80 },
            ReplayDivider::new([250, 1, 3, 2, 0, 3]),
        );

        // RandomRange enters every Random call with carry set. The first two
        // hRandomAdd values (251 and 255) are in the rejected high window;
        // the third wraps to 0 and is accepted.
        assert_eq!(rng.random_range(10).expect("complete divider trace"), 0);
        assert_eq!(rng.random_calls(), 3);
        assert_eq!(rng.rejected_ranges(), 2);
        assert_eq!(rng.source().consumed(), 6);
        assert_eq!(rng.state(), CrystalRandomState { add: 0, sub: 0x79 });
    }

    #[test]
    fn crystal_random_range_returns_simple_divide_remainder() {
        let mut rng = CrystalRandom::new(
            CrystalRandomState { add: 0, sub: 0 },
            ReplayDivider::new([6, 0]),
        );
        assert_eq!(rng.random_range(3).expect("complete divider trace"), 1);
    }

    #[test]
    fn script_random_retries_with_carry_cleared_by_threshold_compare() {
        let mut rng = CrystalRandom::new(
            CrystalRandomState::default(),
            ReplayDivider::new([249, 0, 5, 0, 1, 0]),
        );

        // Script_random enters its first Random with carry set. hRandomAdd=250
        // is rejected for bound 10; `cp 250` clears carry, so the retry reaches
        // 255 rather than wrapping to zero. The third sample then wraps and is
        // accepted.
        assert_eq!(rng.script_random_range(10).expect("complete trace"), 0);
        assert_eq!(rng.random_calls(), 3);
        assert_eq!(rng.rejected_ranges(), 2);
        assert_eq!(rng.source().consumed(), 6);
        assert_eq!(rng.state().add, 0);
    }

    #[test]
    fn link_battle_random_emits_nine_values_then_advances_all_ten_seeds() {
        let mut rng =
            LinkBattleRandom::new([0, 1, 2, 3, 4, 5, 6, 7, 8, 9], 0).expect("fresh link stream");

        let generation: Vec<u8> = (0..9).map(|_| rng.battle_random()).collect();

        assert_eq!(generation, vec![0, 1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(rng.count(), 0);
        assert_eq!(rng.seeds(), &[1, 6, 11, 16, 21, 26, 31, 36, 41, 46]);
        assert_eq!(rng.battle_random(), 1);
        assert_eq!(rng.count(), 1);
    }

    #[test]
    fn link_battle_random_rejects_the_unreachable_tenth_output_index() {
        assert!(LinkBattleRandom::new([0; 10], 9).is_err());
    }

    #[test]
    fn production_rng_boundaries_have_no_seed_packed_or_fixture_constructors() {
        fn standalone_occurrences(source: &str, needle: &str) -> usize {
            source
                .match_indices(needle)
                .filter(|(index, _)| {
                    source[..*index]
                        .chars()
                        .next_back()
                        .is_none_or(|previous| !previous.is_ascii_alphanumeric() && previous != '_')
                })
                .count()
        }

        let boundaries = [
            (
                "crystal-assets/game_data",
                include_str!("../../crystal-assets/src/game_data.rs"),
                0,
            ),
            (
                "crystal-bevy/lib",
                include_str!("../../crystal-bevy/src/lib.rs"),
                0,
            ),
            (
                "crystal-bevy/battle_entry",
                include_str!("../../crystal-bevy/src/bevy_shell/battle_entry.rs"),
                0,
            ),
            (
                "crystal-bevy/credits",
                include_str!("../../crystal-bevy/src/bevy_shell/credits.rs"),
                0,
            ),
            (
                "crystal-bevy/economy",
                include_str!("../../crystal-bevy/src/bevy_shell/economy.rs"),
                0,
            ),
            (
                "crystal-bevy/script_callbacks",
                include_str!("../../crystal-bevy/src/bevy_shell/script_callbacks.rs"),
                0,
            ),
            ("crystal-core/state", include_str!("state.rs"), 0),
            (
                "crystal-core/special_routines",
                include_str!("systems/special_routines.rs"),
                0,
            ),
        ];

        let mut total = 0;
        for (boundary, source, expected_seed_packed_calls) in boundaries {
            let seed_packed_calls = standalone_occurrences(source, "Random::new_crystal(");
            assert_eq!(
                seed_packed_calls, expected_seed_packed_calls,
                "update the explicit RNG migration inventory for {boundary}"
            );
            assert_eq!(
                standalone_occurrences(source, "Random::new("),
                0,
                "production boundary {boundary} must never fall back to the fixture LCG"
            );
            total += seed_packed_calls;
        }
        assert_eq!(
            total, 0,
            "all seed-packed production boundaries are inventoried"
        );
    }

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
