use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::world::encounters::TimeOfDay;

pub const MORN_HOUR: u8 = 4;
pub const DAY_HOUR: u8 = 10;
pub const NITE_HOUR: u8 = 18;
pub const MAX_HOUR: u8 = 24;

pub const RTC_RESET: u8 = 0x80;
pub const RTC_DAYS_EXCEED_255: u8 = 0x40;
pub const RTC_DAYS_EXCEED_139: u8 = 0x20;

pub const B_RAMB_RTC_DH_HIGH: u8 = 0;
pub const B_RAMB_RTC_DH_HALT: u8 = 6;
pub const B_RAMB_RTC_DH_CARRY: u8 = 7;
pub const DEFAULT_RTC_ANCHOR: GameDate = GameDate::new(2000, 1, 1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GameDate {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl<'de> Deserialize<'de> for GameDate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawGameDate {
            year: i32,
            month: u8,
            day: u8,
        }

        let raw = RawGameDate::deserialize(deserializer)?;
        let date = Self {
            year: raw.year,
            month: raw.month,
            day: raw.day,
        };
        date.validate().map_err(D::Error::custom)?;
        Ok(date)
    }
}

impl GameDate {
    pub const fn new(year: i32, month: u8, day: u8) -> Self {
        Self { year, month, day }
    }

    fn validate(&self) -> Result<(), String> {
        if !(1..=12).contains(&self.month) {
            return Err(format!("game date month {} is outside 1..=12", self.month));
        }
        if !(1..=31).contains(&self.day) {
            return Err(format!("game date day {} is outside 1..=31", self.day));
        }
        Ok(())
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClockTime {
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl<'de> Deserialize<'de> for ClockTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawClockTime {
            day: u8,
            hour: u8,
            minute: u8,
            second: u8,
        }

        let raw = RawClockTime::deserialize(deserializer)?;
        let clock = Self {
            day: raw.day,
            hour: raw.hour,
            minute: raw.minute,
            second: raw.second,
        };
        clock
            .validate("time.start_time")
            .map_err(D::Error::custom)?;
        Ok(clock)
    }
}

impl ClockTime {
    pub fn new(day: u8, hour: u8, minute: u8, second: u8) -> Self {
        Self {
            day: day % 7,
            hour: hour % 24,
            minute: minute % 60,
            second: second % 60,
        }
    }

    fn validate(&self, field: &str) -> Result<(), String> {
        validate_clock_field(&format!("{field}.hour"), self.hour, 24)?;
        validate_clock_field(&format!("{field}.minute"), self.minute, 60)?;
        validate_clock_field(&format!("{field}.second"), self.second, 60)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RtcRegisters {
    pub rtc_seconds: u8,
    pub rtc_minutes: u8,
    pub rtc_hours: u8,
    pub rtc_day_lo: u8,
    pub rtc_day_hi: u8,
    pub seconds: u8,
    pub minutes: u8,
    pub hours: u8,
}

impl<'de> Deserialize<'de> for RtcRegisters {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawRtcRegisters {
            rtc_seconds: u8,
            rtc_minutes: u8,
            rtc_hours: u8,
            rtc_day_lo: u8,
            rtc_day_hi: u8,
            seconds: u8,
            minutes: u8,
            hours: u8,
        }

        let raw = RawRtcRegisters::deserialize(deserializer)?;
        let registers = Self {
            rtc_seconds: raw.rtc_seconds,
            rtc_minutes: raw.rtc_minutes,
            rtc_hours: raw.rtc_hours,
            rtc_day_lo: raw.rtc_day_lo,
            rtc_day_hi: raw.rtc_day_hi,
            seconds: raw.seconds,
            minutes: raw.minutes,
            hours: raw.hours,
        };
        registers.validate().map_err(D::Error::custom)?;
        Ok(registers)
    }
}

impl RtcRegisters {
    fn validate(&self) -> Result<(), String> {
        validate_clock_field("time.registers.rtc_seconds", self.rtc_seconds, 60)?;
        validate_clock_field("time.registers.rtc_minutes", self.rtc_minutes, 60)?;
        validate_clock_field("time.registers.rtc_hours", self.rtc_hours, 24)?;
        validate_clock_field("time.registers.seconds", self.seconds, 60)?;
        validate_clock_field("time.registers.minutes", self.minutes, 60)?;
        validate_clock_field("time.registers.hours", self.hours, 24)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TimeState {
    pub rtc_anchor: GameDate,
    pub current_date: GameDate,
    pub start_time: ClockTime,
    pub registers: RtcRegisters,
    pub current_day: u8,
    pub day_of_week: u8,
    pub time_of_day: TimeOfDay,
    pub dst: bool,
    pub rtc_status_flags: u8,
    pub game_time_seconds: u8,
    pub game_time_minutes: u8,
    pub game_time_hours: u8,
    last_rtc_day_count: u16,
}

impl<'de> Deserialize<'de> for TimeState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawTimeState {
            rtc_anchor: GameDate,
            current_date: GameDate,
            start_time: ClockTime,
            registers: RtcRegisters,
            current_day: u8,
            day_of_week: u8,
            time_of_day: TimeOfDay,
            dst: bool,
            rtc_status_flags: u8,
            game_time_seconds: u8,
            game_time_minutes: u8,
            game_time_hours: u8,
            last_rtc_day_count: u16,
        }

        let raw = RawTimeState::deserialize(deserializer)?;
        let state = Self {
            rtc_anchor: raw.rtc_anchor,
            current_date: raw.current_date,
            start_time: raw.start_time,
            registers: raw.registers,
            current_day: raw.current_day,
            day_of_week: raw.day_of_week,
            time_of_day: raw.time_of_day,
            dst: raw.dst,
            rtc_status_flags: raw.rtc_status_flags,
            game_time_seconds: raw.game_time_seconds,
            game_time_minutes: raw.game_time_minutes,
            game_time_hours: raw.game_time_hours,
            last_rtc_day_count: raw.last_rtc_day_count,
        };
        state.validate_saved_state().map_err(D::Error::custom)?;
        Ok(state)
    }
}

impl TimeState {
    pub fn new(anchor: GameDate) -> Self {
        Self {
            rtc_anchor: anchor,
            current_date: anchor,
            start_time: ClockTime::default(),
            registers: RtcRegisters::default(),
            current_day: 0,
            day_of_week: 0,
            time_of_day: TimeOfDay::Night,
            dst: false,
            rtc_status_flags: 0,
            game_time_seconds: 0,
            game_time_minutes: 0,
            game_time_hours: 0,
            last_rtc_day_count: 0,
        }
    }

    pub fn update_from_datetime(&mut self, date: GameDate, hour: u8, minute: u8, second: u8) {
        self.capture_rtc(date, hour, minute, second);
        self.update_time_registers();
    }

    pub fn set_manual_time(
        &mut self,
        now_date: GameDate,
        now_hour: u8,
        now_minute: u8,
        now_second: u8,
        target: ClockTime,
    ) {
        self.capture_rtc(now_date, now_hour, now_minute, now_second);
        self.set_start_offset(target);
        self.update_time_registers();
    }

    pub fn set_start_offset(&mut self, target: ClockTime) {
        let seconds_raw = i16::from(target.second) - i16::from(self.registers.rtc_seconds);
        let start_seconds = mod_i16(seconds_raw, 60) as u8;
        let carry_minutes = if seconds_raw < 0 { 1 } else { 0 };

        let minutes_raw =
            i16::from(target.minute) - i16::from(self.registers.rtc_minutes) - carry_minutes;
        let start_minutes = mod_i16(minutes_raw, 60) as u8;
        let carry_hours = if minutes_raw < 0 { 1 } else { 0 };

        let hours_raw = i16::from(target.hour) - i16::from(self.registers.rtc_hours) - carry_hours;
        let start_hours = mod_i16(hours_raw, 24) as u8;
        let carry_days = if hours_raw < 0 { 1 } else { 0 };

        let base_day = (self.last_rtc_day_count % 7) as i16;
        let day_raw = i16::from(target.day) - (base_day + carry_days);
        let start_day = mod_i16(day_raw, 256) as u8;

        self.start_time = ClockTime {
            day: start_day,
            hour: start_hours,
            minute: start_minutes,
            second: start_seconds,
        };
    }

    pub fn update_time_registers(&mut self) {
        self.fix_days();
        self.fix_time();
        self.sync_game_time_fields();
        self.time_of_day = time_of_day_for_hour(self.registers.hours);
    }

    fn capture_rtc(&mut self, date: GameDate, hour: u8, minute: u8, second: u8) {
        let day_count = compute_day_count(self.rtc_anchor, date);
        self.last_rtc_day_count = day_count;
        self.current_date = date;
        self.registers.rtc_seconds = second % 60;
        self.registers.rtc_minutes = minute % 60;
        self.registers.rtc_hours = hour % 24;
        self.registers.rtc_day_lo = (day_count & 0xff) as u8;
        self.registers.rtc_day_hi = (((day_count >> 8) & 1) as u8) << B_RAMB_RTC_DH_HIGH;
    }

    fn fix_days(&mut self) {
        let mut status = 0;
        let mut day_lo = self.registers.rtc_day_lo;
        let mut day_hi = self.registers.rtc_day_hi;

        if day_hi & (1 << B_RAMB_RTC_DH_HIGH) != 0 {
            day_hi &= !(1 << B_RAMB_RTC_DH_HIGH);
            day_lo %= 140;
            status |= RTC_DAYS_EXCEED_255;
        } else if day_lo >= 140 {
            day_lo %= 140;
            status |= RTC_DAYS_EXCEED_139;
        }

        self.registers.rtc_day_lo = day_lo;
        self.registers.rtc_day_hi = day_hi;
        self.rtc_status_flags = status;
    }

    fn fix_time(&mut self) {
        let total_seconds =
            i16::from(self.registers.rtc_seconds) + i16::from(self.start_time.second);
        self.registers.seconds = mod_i16(total_seconds, 60) as u8;
        let carry_minutes = div_floor_i16(total_seconds, 60);

        let total_minutes = i16::from(self.registers.rtc_minutes)
            + i16::from(self.start_time.minute)
            + carry_minutes;
        self.registers.minutes = mod_i16(total_minutes, 60) as u8;
        let carry_hours = div_floor_i16(total_minutes, 60);

        let total_hours =
            i16::from(self.registers.rtc_hours) + i16::from(self.start_time.hour) + carry_hours;
        self.registers.hours = mod_i16(total_hours, 24) as u8;
        let carry_days = div_floor_i16(total_hours, 24);

        let total_days =
            i16::from(self.registers.rtc_day_lo) + i16::from(self.start_time.day) + carry_days;
        self.current_day = mod_i16(total_days, 256) as u8;
    }

    fn sync_game_time_fields(&mut self) {
        self.game_time_seconds = self.registers.seconds;
        self.game_time_minutes = self.registers.minutes;
        self.game_time_hours = self.registers.hours;
        self.day_of_week = self.current_day % 7;
    }

    pub fn validate_saved_state(&self) -> Result<(), String> {
        validate_clock_field("time.start_time.hour", self.start_time.hour, 24)?;
        validate_clock_field("time.start_time.minute", self.start_time.minute, 60)?;
        validate_clock_field("time.start_time.second", self.start_time.second, 60)?;
        validate_clock_field("time.registers.rtc_seconds", self.registers.rtc_seconds, 60)?;
        validate_clock_field("time.registers.rtc_minutes", self.registers.rtc_minutes, 60)?;
        validate_clock_field("time.registers.rtc_hours", self.registers.rtc_hours, 24)?;
        validate_clock_field("time.registers.seconds", self.registers.seconds, 60)?;
        validate_clock_field("time.registers.minutes", self.registers.minutes, 60)?;
        validate_clock_field("time.registers.hours", self.registers.hours, 24)?;
        if self.game_time_seconds != self.registers.seconds {
            return Err(format!(
                "time.game_time_seconds {} does not match registers.seconds {}",
                self.game_time_seconds, self.registers.seconds
            ));
        }
        if self.game_time_minutes != self.registers.minutes {
            return Err(format!(
                "time.game_time_minutes {} does not match registers.minutes {}",
                self.game_time_minutes, self.registers.minutes
            ));
        }
        if self.game_time_hours != self.registers.hours {
            return Err(format!(
                "time.game_time_hours {} does not match registers.hours {}",
                self.game_time_hours, self.registers.hours
            ));
        }
        let expected_day_of_week = self.current_day % 7;
        if self.day_of_week != expected_day_of_week {
            return Err(format!(
                "time.day_of_week {} does not match current_day modulo 7 {}",
                self.day_of_week, expected_day_of_week
            ));
        }
        let expected_time_of_day = time_of_day_for_hour(self.registers.hours);
        if self.time_of_day != expected_time_of_day {
            return Err(format!(
                "time.time_of_day {:?} does not match registers.hours {} ({:?})",
                self.time_of_day, self.registers.hours, expected_time_of_day
            ));
        }
        Ok(())
    }
}

fn validate_clock_field(field: &str, value: u8, exclusive_max: u8) -> Result<(), String> {
    if value >= exclusive_max {
        return Err(format!(
            "{field} {value} is outside clock range 0..{}",
            exclusive_max - 1
        ));
    }
    Ok(())
}

impl Default for TimeState {
    fn default() -> Self {
        Self::new(DEFAULT_RTC_ANCHOR)
    }
}

pub fn time_of_day_for_hour(hour: u8) -> TimeOfDay {
    let hour = hour % MAX_HOUR;
    if hour < MORN_HOUR {
        TimeOfDay::Night
    } else if hour < DAY_HOUR {
        TimeOfDay::Morning
    } else if hour < NITE_HOUR {
        TimeOfDay::Day
    } else {
        TimeOfDay::Night
    }
}

pub fn compute_day_count(anchor: GameDate, date: GameDate) -> u16 {
    let anchor_days = days_from_civil(anchor.year, anchor.month, anchor.day);
    let date_days = days_from_civil(date.year, date.month, date.day);
    date_days
        .saturating_sub(anchor_days)
        .clamp(0, u16::MAX as i64) as u16
}

fn days_from_civil(year: i32, month: u8, day: u8) -> i64 {
    let year = year - i32::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let month = i32::from(month);
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + i32::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    i64::from(era * 146097 + doe)
}

fn mod_i16(value: i16, modulus: i16) -> i16 {
    value.rem_euclid(modulus)
}

fn div_floor_i16(value: i16, divisor: i16) -> i16 {
    value.div_euclid(divisor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_from_datetime_sets_rtc_registers_and_time_of_day() {
        let mut state = TimeState::new(GameDate::new(2024, 1, 1));
        state.update_from_datetime(GameDate::new(2024, 1, 1), 12, 0, 0);

        assert_eq!(state.registers.rtc_hours, 12);
        assert_eq!(state.registers.rtc_minutes, 0);
        assert_eq!(state.registers.rtc_seconds, 0);
        assert_eq!(state.time_of_day, TimeOfDay::Day);
        assert_eq!(state.game_time_hours, 12);
        assert!(!state.dst);
    }

    #[test]
    fn time_of_day_thresholds_match_crystal() {
        assert_eq!(time_of_day_for_hour(0), TimeOfDay::Night);
        assert_eq!(time_of_day_for_hour(4), TimeOfDay::Morning);
        assert_eq!(time_of_day_for_hour(9), TimeOfDay::Morning);
        assert_eq!(time_of_day_for_hour(10), TimeOfDay::Day);
        assert_eq!(time_of_day_for_hour(17), TimeOfDay::Day);
        assert_eq!(time_of_day_for_hour(18), TimeOfDay::Night);
        assert_eq!(time_of_day_for_hour(23), TimeOfDay::Night);
    }

    #[test]
    fn set_manual_time_offsets_from_current_rtc() {
        let mut state = TimeState::new(GameDate::new(2024, 1, 1));
        state.set_manual_time(
            GameDate::new(2024, 1, 1),
            12,
            0,
            0,
            ClockTime::new(0, 20, 30, 15),
        );

        assert_eq!(state.registers.hours, 20);
        assert_eq!(state.registers.minutes, 30);
        assert_eq!(state.registers.seconds, 15);
        assert_eq!(state.time_of_day, TimeOfDay::Night);
    }

    #[test]
    fn byte_start_offset_carries_minutes_forward() {
        let mut state = TimeState::new(GameDate::new(2024, 1, 1));
        state.update_from_datetime(GameDate::new(2024, 1, 1), 12, 0, 0);
        state.start_time.second = 255;
        state.fix_time();

        assert_eq!(state.registers.seconds, 15);
        assert_eq!(state.registers.minutes, 4);
    }

    #[test]
    fn explicit_negative_offset_rolls_minutes_back() {
        assert_eq!(mod_i16(-30, 60), 30);
        assert_eq!(div_floor_i16(-30, 60), -1);
    }

    #[test]
    fn day_count_and_rtc_status_flags_match_day_thresholds() {
        let mut state = TimeState::new(GameDate::new(2024, 1, 1));
        state.update_from_datetime(GameDate::new(2024, 5, 20), 12, 0, 0);
        assert_eq!(state.registers.rtc_day_lo, 0);
        assert_eq!(state.rtc_status_flags, RTC_DAYS_EXCEED_139);

        state.update_from_datetime(GameDate::new(2024, 9, 14), 12, 0, 0);
        assert_eq!(state.rtc_status_flags, RTC_DAYS_EXCEED_255);
    }

    #[test]
    fn date_before_anchor_resets_day_count_to_zero() {
        let mut state = TimeState::new(GameDate::new(2024, 1, 2));
        state.update_from_datetime(GameDate::new(2024, 1, 1), 6, 0, 0);

        assert_eq!(state.registers.rtc_day_lo, 0);
        assert_eq!(state.current_day, 0);
        assert_eq!(state.time_of_day, TimeOfDay::Morning);
    }
}
