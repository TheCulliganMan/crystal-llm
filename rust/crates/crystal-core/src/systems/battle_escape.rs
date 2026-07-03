use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::battle::stats::{BattleStatMultiplierTables, apply_stage};
use crate::models::{Pokemon, Stat};
use crate::random::Random;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BattleEscapeRules {
    pub player_speed_multiplier: u16,
    pub enemy_speed_divisor: u16,
    pub failed_attempt_bonus: u16,
    pub rng_roll_values: u16,
}

impl<'de> Deserialize<'de> for BattleEscapeRules {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawBattleEscapeRules {
            player_speed_multiplier: u16,
            enemy_speed_divisor: u16,
            failed_attempt_bonus: u16,
            rng_roll_values: u16,
        }

        let raw = RawBattleEscapeRules::deserialize(deserializer)?;
        let rules = Self {
            player_speed_multiplier: raw.player_speed_multiplier,
            enemy_speed_divisor: raw.enemy_speed_divisor,
            failed_attempt_bonus: raw.failed_attempt_bonus,
            rng_roll_values: raw.rng_roll_values,
        };
        rules.validate_shape().map_err(D::Error::custom)?;
        Ok(rules)
    }
}

impl Default for BattleEscapeRules {
    fn default() -> Self {
        Self {
            player_speed_multiplier: 0,
            enemy_speed_divisor: 0,
            failed_attempt_bonus: 0,
            rng_roll_values: 0,
        }
    }
}

impl BattleEscapeRules {
    fn validate_shape(&self) -> Result<(), String> {
        if let Some(issue) = battle_escape_rules_issues(self, true).into_iter().next() {
            return Err(format!("invalid battle escape rules: {issue:?}"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum BattleEscapeRulesIssue {
    Missing,
    MissingPlayerSpeedMultiplier,
    MissingEnemySpeedDivisor,
    InvalidRngRollValues { rng_roll_values: u16 },
}

pub fn battle_escape_rules_issues(
    rules: &BattleEscapeRules,
    required: bool,
) -> Vec<BattleEscapeRulesIssue> {
    if !required {
        return Vec::new();
    }
    if rules == &BattleEscapeRules::default() {
        return vec![BattleEscapeRulesIssue::Missing];
    }

    let mut issues = Vec::new();
    if rules.player_speed_multiplier == 0 {
        issues.push(BattleEscapeRulesIssue::MissingPlayerSpeedMultiplier);
    }
    if rules.enemy_speed_divisor == 0 {
        issues.push(BattleEscapeRulesIssue::MissingEnemySpeedDivisor);
    }
    if rules.rng_roll_values == 0 || rules.rng_roll_values > u16::from(u8::MAX) + 1 {
        issues.push(BattleEscapeRulesIssue::InvalidRngRollValues {
            rng_roll_values: rules.rng_roll_values,
        });
    }
    issues
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleEscapeAttempt {
    pub escaped: bool,
    pub chance: u16,
    pub roll: Option<u8>,
    pub attempts_before: u8,
    pub attempts_after: u8,
    pub rng_seed_after: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum BattleEscapeError {
    MissingStat { side: EscapeSide, stat: Stat },
    MissingStatStage { side: EscapeSide, stat: Stat },
    MissingStatMultiplier { side: EscapeSide, stage: i8 },
    InvalidRules { message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum EscapeSide {
    Player,
    Enemy,
}

pub fn attempt_wild_battle_escape(
    player: &Pokemon,
    enemy: &Pokemon,
    stat_multipliers: &BattleStatMultiplierTables,
    rules: &BattleEscapeRules,
    attempts_before: u8,
    rng: &mut Random,
) -> Result<BattleEscapeAttempt, BattleEscapeError> {
    let player_speed = escape_speed(EscapeSide::Player, player, stat_multipliers)?;
    let enemy_speed = escape_speed(EscapeSide::Enemy, enemy, stat_multipliers)?;
    let chance = escape_chance(player_speed, enemy_speed, attempts_before, rules)?;
    if player_speed >= enemy_speed || chance >= rules.rng_roll_values {
        return Ok(BattleEscapeAttempt {
            escaped: true,
            chance,
            roll: None,
            attempts_before,
            attempts_after: attempts_before,
            rng_seed_after: rng.seed(),
        });
    }
    let roll = rng.randrange(u32::from(rules.rng_roll_values)) as u8;
    let escaped = u16::from(roll) < chance;
    Ok(BattleEscapeAttempt {
        escaped,
        chance,
        roll: Some(roll),
        attempts_before,
        attempts_after: if escaped {
            attempts_before
        } else {
            attempts_before.saturating_add(1)
        },
        rng_seed_after: rng.seed(),
    })
}

pub fn escape_chance(
    player_speed: u16,
    enemy_speed: u16,
    attempts_before: u8,
    rules: &BattleEscapeRules,
) -> Result<u16, BattleEscapeError> {
    if rules.player_speed_multiplier == 0 {
        return Err(BattleEscapeError::InvalidRules {
            message: "player_speed_multiplier must be nonzero".to_string(),
        });
    }
    if rules.enemy_speed_divisor == 0 {
        return Err(BattleEscapeError::InvalidRules {
            message: "enemy_speed_divisor must be nonzero".to_string(),
        });
    }
    if rules.rng_roll_values == 0 || rules.rng_roll_values > u16::from(u8::MAX) + 1 {
        return Err(BattleEscapeError::InvalidRules {
            message: "rng_roll_values must be in 1..=256".to_string(),
        });
    }
    let enemy_divisor = (enemy_speed / rules.enemy_speed_divisor).max(1);
    Ok(
        ((u32::from(player_speed) * u32::from(rules.player_speed_multiplier))
            / u32::from(enemy_divisor))
        .saturating_add(u32::from(attempts_before) * u32::from(rules.failed_attempt_bonus))
        .min(u32::from(u16::MAX)) as u16,
    )
}

fn escape_speed(
    side: EscapeSide,
    pokemon: &Pokemon,
    stat_multipliers: &BattleStatMultiplierTables,
) -> Result<u16, BattleEscapeError> {
    let base = pokemon
        .calculate_stat(Stat::Speed)
        .ok_or(BattleEscapeError::MissingStat {
            side,
            stat: Stat::Speed,
        })?;
    let stage =
        *pokemon
            .stat_boosts
            .get(&Stat::Speed)
            .ok_or(BattleEscapeError::MissingStatStage {
                side,
                stat: Stat::Speed,
            })?;
    apply_stage(stat_multipliers, base, stage)
        .ok_or(BattleEscapeError::MissingStatMultiplier { side, stage })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::stats::BattleStatMultiplier;
    use crate::models::{BaseStats, Dv, PokemonSpecies};

    fn stat_multipliers() -> BattleStatMultiplierTables {
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
            accuracy: vec![],
        }
    }

    fn pokemon(id: &str, speed: u16) -> Pokemon {
        Pokemon::new_for_tests(
            PokemonSpecies::new_for_tests(id, BaseStats::new(45, 49, 49, speed, 65, 65)),
            20,
            Dv::from_non_hp(10, 10, 10, 10),
        )
    }

    fn escape_rules() -> BattleEscapeRules {
        BattleEscapeRules {
            player_speed_multiplier: 32,
            enemy_speed_divisor: 4,
            failed_attempt_bonus: 30,
            rng_roll_values: 256,
        }
    }

    #[test]
    fn battle_escape_rules_issues_require_definitive_rules_when_pokemon_exist() {
        assert_eq!(
            battle_escape_rules_issues(&BattleEscapeRules::default(), true),
            vec![BattleEscapeRulesIssue::Missing]
        );
        assert_eq!(
            battle_escape_rules_issues(&BattleEscapeRules::default(), false),
            []
        );

        let rules = BattleEscapeRules {
            player_speed_multiplier: 0,
            enemy_speed_divisor: 0,
            failed_attempt_bonus: 30,
            rng_roll_values: 257,
        };
        assert_eq!(
            battle_escape_rules_issues(&rules, true),
            vec![
                BattleEscapeRulesIssue::MissingPlayerSpeedMultiplier,
                BattleEscapeRulesIssue::MissingEnemySpeedDivisor,
                BattleEscapeRulesIssue::InvalidRngRollValues {
                    rng_roll_values: 257,
                },
            ],
        );
    }

    #[test]
    fn faster_player_escapes_without_rng_or_attempt_increment() {
        let player = pokemon("RATTATA", 120);
        let enemy = pokemon("GEODUDE", 20);
        let mut rng = Random::new(7);

        let outcome = attempt_wild_battle_escape(
            &player,
            &enemy,
            &stat_multipliers(),
            &escape_rules(),
            2,
            &mut rng,
        )
        .expect("escape resolves");

        assert!(outcome.escaped);
        assert_eq!(outcome.roll, None);
        assert_eq!(outcome.attempts_before, 2);
        assert_eq!(outcome.attempts_after, 2);
        assert_eq!(outcome.rng_seed_after, 7);
    }

    #[test]
    fn slower_player_uses_rng_and_increments_only_on_failure() {
        let player = pokemon("GEODUDE", 20);
        let enemy = pokemon("RATTATA", 120);
        let mut rng = Random::new(1);

        let outcome = attempt_wild_battle_escape(
            &player,
            &enemy,
            &stat_multipliers(),
            &escape_rules(),
            0,
            &mut rng,
        )
        .expect("escape resolves");

        assert_eq!(outcome.chance, 38);
        assert_eq!(outcome.roll, Some(64));
        assert!(!outcome.escaped);
        assert_eq!(outcome.attempts_after, 1);
        assert_eq!(outcome.rng_seed_after, rng.seed());
    }

    #[test]
    fn escape_requires_exact_speed_stage_data() {
        let mut player = pokemon("GEODUDE", 20);
        player.stat_boosts.remove(&Stat::Speed);
        let enemy = pokemon("RATTATA", 120);
        let mut rng = Random::new(1);

        assert_eq!(
            attempt_wild_battle_escape(
                &player,
                &enemy,
                &stat_multipliers(),
                &escape_rules(),
                0,
                &mut rng
            ),
            Err(BattleEscapeError::MissingStatStage {
                side: EscapeSide::Player,
                stat: Stat::Speed
            })
        );
    }

    #[test]
    fn escape_rejects_missing_rule_divisor_without_formula_fallback() {
        let player = pokemon("GEODUDE", 20);
        let enemy = pokemon("RATTATA", 120);
        let mut rng = Random::new(1);
        let mut rules = escape_rules();
        rules.enemy_speed_divisor = 0;

        assert_eq!(
            attempt_wild_battle_escape(&player, &enemy, &stat_multipliers(), &rules, 0, &mut rng),
            Err(BattleEscapeError::InvalidRules {
                message: "enemy_speed_divisor must be nonzero".to_string()
            })
        );
    }

    #[test]
    fn escape_rejects_missing_player_speed_multiplier_without_zero_chance_fallback() {
        let player = pokemon("GEODUDE", 20);
        let enemy = pokemon("RATTATA", 120);
        let mut rng = Random::new(1);
        let mut rules = escape_rules();
        rules.player_speed_multiplier = 0;

        assert_eq!(
            attempt_wild_battle_escape(&player, &enemy, &stat_multipliers(), &rules, 0, &mut rng),
            Err(BattleEscapeError::InvalidRules {
                message: "player_speed_multiplier must be nonzero".to_string()
            })
        );
    }

    #[test]
    fn battle_escape_serialized_variants_reject_unknown_fallback_fields() {
        let issue_error = serde_json::from_value::<BattleEscapeRulesIssue>(serde_json::json!({
            "InvalidRngRollValues": {
                "rng_roll_values": 0,
                "default_rng_roll_values": 256
            }
        }))
        .expect_err("default rng roll values must be rejected")
        .to_string();
        assert!(
            issue_error.contains("unknown field `default_rng_roll_values`"),
            "{issue_error}"
        );

        let escape_error = serde_json::from_value::<BattleEscapeError>(serde_json::json!({
            "MissingStat": {
                "side": "player",
                "stat": "Speed",
                "fallback_stat": "Speed"
            }
        }))
        .expect_err("fallback stat must be rejected")
        .to_string();
        assert!(
            escape_error.contains("unknown field `fallback_stat`"),
            "{escape_error}"
        );
    }
}
