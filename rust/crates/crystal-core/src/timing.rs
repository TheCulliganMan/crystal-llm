pub const GB_CPU_CYCLES_PER_SECOND: u32 = 4_194_304;
pub const GB_CYCLES_PER_FRAME: u32 = 70_224;
pub const GB_FRAME_RATE: f64 = GB_CPU_CYCLES_PER_SECOND as f64 / GB_CYCLES_PER_FRAME as f64;
pub const GB_FRAME_DURATION_MS: f64 =
    (GB_CYCLES_PER_FRAME as f64 * 1000.0) / GB_CPU_CYCLES_PER_SECOND as f64;
pub const GB_FRAME_DURATION_SECONDS: f64 = GB_FRAME_DURATION_MS / 1000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Frame(pub u64);

impl Frame {
    pub const ZERO: Self = Self(0);

    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }

    pub fn elapsed_seconds(self) -> f64 {
        self.0 as f64 * GB_FRAME_DURATION_SECONDS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_rate_matches_game_boy_cadence() {
        assert!((GB_FRAME_RATE - 59.727500569606).abs() < 0.000000001);
        assert!((GB_FRAME_DURATION_MS - 16.742706298828125).abs() < 0.000000001);
    }

    #[test]
    fn frame_elapsed_seconds_uses_hardware_duration() {
        assert_eq!(Frame::ZERO.next(), Frame(1));
        assert!((Frame(60).elapsed_seconds() - 1.0045623779296875).abs() < 0.000000001);
    }
}
