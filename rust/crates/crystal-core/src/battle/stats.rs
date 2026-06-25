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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleStatMultiplier {
    pub numerator: i32,
    pub denominator: i32,
}

impl BattleStatMultiplier {
    pub const fn as_fraction(self) -> Fraction {
        Fraction::new(self.numerator, self.denominator)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleStatMultiplierTables {
    pub stat: Vec<BattleStatMultiplier>,
    pub accuracy: Vec<BattleStatMultiplier>,
}

pub fn stage_multiplier(tables: &BattleStatMultiplierTables, stage: i8) -> Option<Fraction> {
    table_stage_multiplier(&tables.stat, stage)
}

pub fn accuracy_stage_multiplier(
    tables: &BattleStatMultiplierTables,
    stage: i8,
) -> Option<Fraction> {
    table_stage_multiplier(&tables.accuracy, stage)
}

pub fn apply_stage(tables: &BattleStatMultiplierTables, value: u16, stage: i8) -> Option<u16> {
    let modifier = stage_multiplier(tables, stage)?;
    Some(modifier.multiply_floor(value as i32).clamp(1, 999) as u16)
}

fn table_stage_multiplier(table: &[BattleStatMultiplier], stage: i8) -> Option<Fraction> {
    if !(-6..=6).contains(&stage) {
        return None;
    }
    table
        .get((stage + 6) as usize)
        .copied()
        .map(BattleStatMultiplier::as_fraction)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tables() -> BattleStatMultiplierTables {
        BattleStatMultiplierTables {
            stat: vec![
                BattleStatMultiplier {
                    numerator: 25,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 28,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 33,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 40,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 50,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 66,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 1,
                    denominator: 1,
                },
                BattleStatMultiplier {
                    numerator: 15,
                    denominator: 10,
                },
                BattleStatMultiplier {
                    numerator: 2,
                    denominator: 1,
                },
                BattleStatMultiplier {
                    numerator: 25,
                    denominator: 10,
                },
                BattleStatMultiplier {
                    numerator: 3,
                    denominator: 1,
                },
                BattleStatMultiplier {
                    numerator: 35,
                    denominator: 10,
                },
                BattleStatMultiplier {
                    numerator: 4,
                    denominator: 1,
                },
            ],
            accuracy: vec![
                BattleStatMultiplier {
                    numerator: 33,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 36,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 43,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 50,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 60,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 75,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 1,
                    denominator: 1,
                },
                BattleStatMultiplier {
                    numerator: 133,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 166,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 2,
                    denominator: 1,
                },
                BattleStatMultiplier {
                    numerator: 233,
                    denominator: 100,
                },
                BattleStatMultiplier {
                    numerator: 133,
                    denominator: 50,
                },
                BattleStatMultiplier {
                    numerator: 3,
                    denominator: 1,
                },
            ],
        }
    }

    #[test]
    fn stage_multiplier_uses_exact_pack_table_without_clamping() {
        let tables = tables();
        assert_eq!(stage_multiplier(&tables, -6), Some(Fraction::new(25, 100)));
        assert_eq!(stage_multiplier(&tables, -1), Some(Fraction::new(66, 100)));
        assert_eq!(stage_multiplier(&tables, 0), Some(Fraction::new(1, 1)));
        assert_eq!(stage_multiplier(&tables, 2), Some(Fraction::new(2, 1)));
        assert_eq!(stage_multiplier(&tables, 6), Some(Fraction::new(4, 1)));
        assert_eq!(stage_multiplier(&tables, 99), None);
    }

    #[test]
    fn apply_stage_floors_and_clamps_like_typescript() {
        let tables = tables();
        assert_eq!(apply_stage(&tables, 100, -1), Some(66));
        assert_eq!(apply_stage(&tables, 100, 2), Some(200));
        assert_eq!(apply_stage(&tables, 900, 6), Some(999));
        assert_eq!(apply_stage(&tables, 1, -6), Some(1));
    }

    #[test]
    fn accuracy_stage_uses_asm_table_not_formula() {
        let tables = tables();
        assert_eq!(
            accuracy_stage_multiplier(&tables, 2),
            Some(Fraction::new(166, 100))
        );
        assert_eq!(
            accuracy_stage_multiplier(&tables, -2),
            Some(Fraction::new(60, 100))
        );
    }
}
