use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fraction {
    pub numerator: i32,
    pub denominator: i32,
}

impl Fraction {
    pub const fn new(numerator: i32, denominator: i32) -> Self {
        Self {
            numerator,
            denominator,
        }
    }

    pub const fn one() -> Self {
        Self::new(1, 1)
    }

    pub fn multiply_floor(self, value: i32) -> i32 {
        (value * self.numerator) / self.denominator
    }
}

pub fn clamp_stage(stage: i8) -> i8 {
    stage.clamp(-6, 6)
}

pub fn stage_multiplier(stage: i8) -> Fraction {
    match clamp_stage(stage) {
        -6 => Fraction::new(25, 100),
        -5 => Fraction::new(28, 100),
        -4 => Fraction::new(33, 100),
        -3 => Fraction::new(40, 100),
        -2 => Fraction::new(50, 100),
        -1 => Fraction::new(66, 100),
        0 => Fraction::one(),
        1 => Fraction::new(15, 10),
        2 => Fraction::new(2, 1),
        3 => Fraction::new(25, 10),
        4 => Fraction::new(3, 1),
        5 => Fraction::new(35, 10),
        6 => Fraction::new(4, 1),
        _ => unreachable!("stage is clamped"),
    }
}

pub fn accuracy_stage_multiplier(stage: i8) -> Fraction {
    let stage = clamp_stage(stage);
    if stage >= 0 {
        Fraction::new(3 + stage as i32, 3)
    } else {
        Fraction::new(3, 3 - stage as i32)
    }
}

pub fn apply_stage(value: u16, stage: i8) -> u16 {
    let modifier = stage_multiplier(stage);
    modifier.multiply_floor(value as i32).clamp(1, 999) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_multiplier_matches_typescript_table() {
        assert_eq!(stage_multiplier(-6), Fraction::new(25, 100));
        assert_eq!(stage_multiplier(-1), Fraction::new(66, 100));
        assert_eq!(stage_multiplier(0), Fraction::new(1, 1));
        assert_eq!(stage_multiplier(2), Fraction::new(2, 1));
        assert_eq!(stage_multiplier(6), Fraction::new(4, 1));
        assert_eq!(stage_multiplier(99), Fraction::new(4, 1));
    }

    #[test]
    fn apply_stage_floors_and_clamps_like_typescript() {
        assert_eq!(apply_stage(100, -1), 66);
        assert_eq!(apply_stage(100, 2), 200);
        assert_eq!(apply_stage(900, 6), 999);
        assert_eq!(apply_stage(1, -6), 1);
    }

    #[test]
    fn accuracy_stage_formula_matches_gen_two() {
        assert_eq!(accuracy_stage_multiplier(2), Fraction::new(5, 3));
        assert_eq!(accuracy_stage_multiplier(-2), Fraction::new(3, 5));
    }
}
