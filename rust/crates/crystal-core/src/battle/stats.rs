use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Fraction {
    pub numerator: i32,
    pub denominator: i32,
}

impl<'de> Deserialize<'de> for Fraction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawFraction {
            numerator: i32,
            denominator: i32,
        }

        let raw = RawFraction::deserialize(deserializer)?;
        if raw.denominator <= 0 {
            return Err(D::Error::custom(format!(
                "fraction denominator {} must be positive",
                raw.denominator
            )));
        }
        Ok(Self {
            numerator: raw.numerator,
            denominator: raw.denominator,
        })
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BattleStatMultiplier {
    pub numerator: i32,
    pub denominator: i32,
}

impl<'de> Deserialize<'de> for BattleStatMultiplier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawBattleStatMultiplier {
            numerator: i32,
            denominator: i32,
        }

        let raw = RawBattleStatMultiplier::deserialize(deserializer)?;
        let multiplier = Self {
            numerator: raw.numerator,
            denominator: raw.denominator,
        };
        validate_battle_stat_multiplier(BattleStatMultiplierTableKind::Stat, 0, multiplier)
            .map_err(|issue| D::Error::custom(format!("{issue:?}")))?;
        Ok(multiplier)
    }
}

impl BattleStatMultiplier {
    pub const fn as_fraction(self) -> Fraction {
        Fraction::new(self.numerator, self.denominator)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BattleStatMultiplierTables {
    pub stat: Vec<BattleStatMultiplier>,
    pub accuracy: Vec<BattleStatMultiplier>,
}

impl<'de> Deserialize<'de> for BattleStatMultiplierTables {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawBattleStatMultiplierTables {
            stat: Vec<BattleStatMultiplier>,
            accuracy: Vec<BattleStatMultiplier>,
        }

        let raw = RawBattleStatMultiplierTables::deserialize(deserializer)?;
        let tables = Self {
            stat: raw.stat,
            accuracy: raw.accuracy,
        };
        let issues = battle_stat_multiplier_table_issues(&tables, true);
        if let Some(issue) = issues.first() {
            return Err(D::Error::custom(format!(
                "invalid battle stat multiplier tables: {issue:?}"
            )));
        }
        Ok(tables)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleStatMultiplierTableKind {
    Stat,
    Accuracy,
}

impl BattleStatMultiplierTableKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stat => "stat",
            Self::Accuracy => "accuracy",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattleStatMultiplierTableIssue {
    InvalidLength {
        table: BattleStatMultiplierTableKind,
        actual: usize,
    },
    InvalidNumerator {
        table: BattleStatMultiplierTableKind,
        stage: i8,
        numerator: i32,
    },
    InvalidDenominator {
        table: BattleStatMultiplierTableKind,
        stage: i8,
        denominator: i32,
    },
}

impl BattleStatMultiplierTableIssue {
    pub fn table(&self) -> BattleStatMultiplierTableKind {
        match self {
            Self::InvalidLength { table, .. }
            | Self::InvalidNumerator { table, .. }
            | Self::InvalidDenominator { table, .. } => *table,
        }
    }
}

pub fn battle_stat_multiplier_table_issues(
    tables: &BattleStatMultiplierTables,
    required: bool,
) -> Vec<BattleStatMultiplierTableIssue> {
    if !required {
        return Vec::new();
    }
    let mut issues = Vec::new();
    push_battle_stat_multiplier_table_issues(
        BattleStatMultiplierTableKind::Stat,
        &tables.stat,
        &mut issues,
    );
    push_battle_stat_multiplier_table_issues(
        BattleStatMultiplierTableKind::Accuracy,
        &tables.accuracy,
        &mut issues,
    );
    issues
}

fn push_battle_stat_multiplier_table_issues(
    table: BattleStatMultiplierTableKind,
    entries: &[BattleStatMultiplier],
    issues: &mut Vec<BattleStatMultiplierTableIssue>,
) {
    if entries.len() != 13 {
        issues.push(BattleStatMultiplierTableIssue::InvalidLength {
            table,
            actual: entries.len(),
        });
    }
    for (index, entry) in entries.iter().enumerate() {
        let stage = index as i8 - 6;
        if let Err(issue) = validate_battle_stat_multiplier(table, stage, *entry) {
            issues.push(issue);
        }
    }
}

fn validate_battle_stat_multiplier(
    table: BattleStatMultiplierTableKind,
    stage: i8,
    entry: BattleStatMultiplier,
) -> Result<(), BattleStatMultiplierTableIssue> {
    if entry.numerator <= 0 {
        return Err(BattleStatMultiplierTableIssue::InvalidNumerator {
            table,
            stage,
            numerator: entry.numerator,
        });
    }
    if entry.denominator <= 0 {
        return Err(BattleStatMultiplierTableIssue::InvalidDenominator {
            table,
            stage,
            denominator: entry.denominator,
        });
    }
    Ok(())
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
    fn battle_stat_multiplier_table_issues_validate_exact_pack_tables() {
        let tables = BattleStatMultiplierTables {
            stat: vec![
                BattleStatMultiplier {
                    numerator: 1,
                    denominator: 1,
                };
                12
            ],
            accuracy: vec![
                BattleStatMultiplier {
                    numerator: 1,
                    denominator: 1,
                },
                BattleStatMultiplier {
                    numerator: 0,
                    denominator: 1,
                },
                BattleStatMultiplier {
                    numerator: 1,
                    denominator: 0,
                },
            ],
        };

        assert_eq!(
            battle_stat_multiplier_table_issues(&tables, true),
            vec![
                BattleStatMultiplierTableIssue::InvalidLength {
                    table: BattleStatMultiplierTableKind::Stat,
                    actual: 12,
                },
                BattleStatMultiplierTableIssue::InvalidLength {
                    table: BattleStatMultiplierTableKind::Accuracy,
                    actual: 3,
                },
                BattleStatMultiplierTableIssue::InvalidNumerator {
                    table: BattleStatMultiplierTableKind::Accuracy,
                    stage: -5,
                    numerator: 0,
                },
                BattleStatMultiplierTableIssue::InvalidDenominator {
                    table: BattleStatMultiplierTableKind::Accuracy,
                    stage: -4,
                    denominator: 0,
                },
            ]
        );
        assert_eq!(battle_stat_multiplier_table_issues(&tables, false), []);
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
