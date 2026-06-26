use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::battle::stats::{BattleStatMultiplierTables, apply_stage};
use crate::models::{Move, Pokemon, PokemonType, Stat};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypeMultiplier {
    pub numerator: u16,
    pub denominator: u16,
}

impl TypeMultiplier {
    pub const fn one() -> Self {
        Self {
            numerator: 1,
            denominator: 1,
        }
    }

    pub const fn zero() -> Self {
        Self {
            numerator: 0,
            denominator: 1,
        }
    }

    pub fn multiply(self, other: Self) -> Self {
        if self.numerator == 0 || other.numerator == 0 {
            return Self::zero();
        }
        let numerator = self.numerator * other.numerator;
        let denominator = self.denominator * other.denominator;
        let divisor = gcd(numerator, denominator);
        Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        }
    }

    pub const fn apply_floor(self, value: u16) -> u16 {
        ((value as u32 * self.numerator as u32) / self.denominator as u32) as u16
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Weather {
    None,
    Rain,
    Sun,
}

impl Weather {
    pub const fn asm_id(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Rain => Some("WEATHER_RAIN"),
            Self::Sun => Some("WEATHER_SUN"),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeatherModifiers {
    pub type_modifiers: Vec<WeatherTypeModifier>,
    pub move_effect_modifiers: Vec<WeatherMoveEffectModifier>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeatherTypeModifier {
    pub weather: String,
    pub move_type: PokemonType,
    pub multiplier: TypeMultiplier,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeatherMoveEffectModifier {
    pub weather: String,
    pub move_effect: String,
    pub multiplier: TypeMultiplier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeatherModifierTableKind {
    TypeModifiers,
    MoveEffectModifiers,
}

impl WeatherModifierTableKind {
    pub const fn subject(self) -> &'static str {
        match self {
            Self::TypeModifiers => "weather_modifiers:type_modifiers",
            Self::MoveEffectModifiers => "weather_modifiers:move_effect_modifiers",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WeatherModifierIssue {
    MissingTypeModifiers,
    MissingMoveEffectModifiers,
    InvalidWeather {
        table: WeatherModifierTableKind,
        weather: String,
    },
    InvalidMoveEffect {
        move_effect: String,
    },
    UnknownMoveEffect {
        move_effect: String,
    },
    InvalidMultiplierDenominator {
        table: WeatherModifierTableKind,
    },
}

pub fn weather_modifier_issues(
    modifiers: &WeatherModifiers,
    moves: &BTreeMap<String, Move>,
    required: bool,
) -> Vec<WeatherModifierIssue> {
    if !required {
        return Vec::new();
    }
    let mut issues = Vec::new();
    if modifiers.type_modifiers.is_empty() {
        issues.push(WeatherModifierIssue::MissingTypeModifiers);
    }
    if modifiers.move_effect_modifiers.is_empty() {
        issues.push(WeatherModifierIssue::MissingMoveEffectModifiers);
    }
    let move_effect_ids: BTreeSet<&str> = moves
        .values()
        .map(|move_data| move_data.effect.as_str())
        .collect();
    for entry in &modifiers.type_modifiers {
        if entry.weather.trim().is_empty() || entry.weather.trim() != entry.weather {
            issues.push(WeatherModifierIssue::InvalidWeather {
                table: WeatherModifierTableKind::TypeModifiers,
                weather: entry.weather.clone(),
            });
        }
        push_type_multiplier_issue(
            WeatherModifierTableKind::TypeModifiers,
            entry.multiplier,
            &mut issues,
        );
    }
    for entry in &modifiers.move_effect_modifiers {
        if entry.weather.trim().is_empty() || entry.weather.trim() != entry.weather {
            issues.push(WeatherModifierIssue::InvalidWeather {
                table: WeatherModifierTableKind::MoveEffectModifiers,
                weather: entry.weather.clone(),
            });
        }
        if entry.move_effect.trim().is_empty() || entry.move_effect.trim() != entry.move_effect {
            issues.push(WeatherModifierIssue::InvalidMoveEffect {
                move_effect: entry.move_effect.clone(),
            });
        } else if !move_effect_ids.contains(entry.move_effect.as_str()) {
            issues.push(WeatherModifierIssue::UnknownMoveEffect {
                move_effect: entry.move_effect.clone(),
            });
        }
        push_type_multiplier_issue(
            WeatherModifierTableKind::MoveEffectModifiers,
            entry.multiplier,
            &mut issues,
        );
    }
    issues
}

fn push_type_multiplier_issue(
    table: WeatherModifierTableKind,
    multiplier: TypeMultiplier,
    issues: &mut Vec<WeatherModifierIssue>,
) {
    if multiplier.denominator == 0 {
        issues.push(WeatherModifierIssue::InvalidMultiplierDenominator { table });
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypeEffectivenessTable {
    pub matchups: Vec<TypeEffectivenessEntry>,
    pub foresight_matchups: Vec<TypeEffectivenessEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypeEffectivenessEntry {
    pub attacker: PokemonType,
    pub defender: PokemonType,
    pub multiplier: TypeMultiplier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeEffectivenessTableKind {
    Matchups,
    ForesightMatchups,
}

impl TypeEffectivenessTableKind {
    pub const fn subject(self) -> &'static str {
        match self {
            Self::Matchups => "type_effectiveness:matchups",
            Self::ForesightMatchups => "type_effectiveness:foresight_matchups",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeEffectivenessTableIssue {
    MissingMatchups,
    MissingForesightMatchups,
    InvalidMultiplierDenominator {
        table: TypeEffectivenessTableKind,
    },
    UnknownAttacker {
        table: TypeEffectivenessTableKind,
        attacker: PokemonType,
    },
    UnknownDefender {
        table: TypeEffectivenessTableKind,
        defender: PokemonType,
    },
    DuplicateMatchup {
        table: TypeEffectivenessTableKind,
        attacker: PokemonType,
        defender: PokemonType,
    },
    MissingMatchup {
        attacker: String,
        defender: String,
    },
}

pub fn type_effectiveness_table_issues(
    table: &TypeEffectivenessTable,
    categories: &TypeCategories,
    required: bool,
) -> Vec<TypeEffectivenessTableIssue> {
    if !required {
        return Vec::new();
    }

    let mut issues = Vec::new();
    if table.matchups.is_empty() {
        issues.push(TypeEffectivenessTableIssue::MissingMatchups);
    }
    if table.foresight_matchups.is_empty() {
        issues.push(TypeEffectivenessTableIssue::MissingForesightMatchups);
    }

    let declared_types: BTreeSet<&str> = categories
        .physical
        .iter()
        .chain(categories.special.iter())
        .map(String::as_str)
        .collect();
    let mut matchup_pairs = BTreeSet::new();
    for entry in &table.matchups {
        push_type_effectiveness_entry_issues(
            TypeEffectivenessTableKind::Matchups,
            entry,
            &declared_types,
            &mut issues,
        );
        if !matchup_pairs.insert((entry.attacker.as_str(), entry.defender.as_str())) {
            issues.push(TypeEffectivenessTableIssue::DuplicateMatchup {
                table: TypeEffectivenessTableKind::Matchups,
                attacker: entry.attacker.clone(),
                defender: entry.defender.clone(),
            });
        }
    }
    for attacker in &declared_types {
        for defender in &declared_types {
            if !matchup_pairs.contains(&(*attacker, *defender)) {
                issues.push(TypeEffectivenessTableIssue::MissingMatchup {
                    attacker: (*attacker).to_string(),
                    defender: (*defender).to_string(),
                });
            }
        }
    }

    let mut foresight_pairs = BTreeSet::new();
    for entry in &table.foresight_matchups {
        push_type_effectiveness_entry_issues(
            TypeEffectivenessTableKind::ForesightMatchups,
            entry,
            &declared_types,
            &mut issues,
        );
        if !foresight_pairs.insert((entry.attacker.as_str(), entry.defender.as_str())) {
            issues.push(TypeEffectivenessTableIssue::DuplicateMatchup {
                table: TypeEffectivenessTableKind::ForesightMatchups,
                attacker: entry.attacker.clone(),
                defender: entry.defender.clone(),
            });
        }
    }

    issues
}

fn push_type_effectiveness_entry_issues(
    table: TypeEffectivenessTableKind,
    entry: &TypeEffectivenessEntry,
    declared_types: &BTreeSet<&str>,
    issues: &mut Vec<TypeEffectivenessTableIssue>,
) {
    if entry.multiplier.denominator == 0 {
        issues.push(TypeEffectivenessTableIssue::InvalidMultiplierDenominator { table });
    }
    if declared_types.is_empty() {
        return;
    }
    if !declared_types.contains(entry.attacker.as_str()) {
        issues.push(TypeEffectivenessTableIssue::UnknownAttacker {
            table,
            attacker: entry.attacker.clone(),
        });
    }
    if !declared_types.contains(entry.defender.as_str()) {
        issues.push(TypeEffectivenessTableIssue::UnknownDefender {
            table,
            defender: entry.defender.clone(),
        });
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypeCategories {
    pub physical: Vec<String>,
    pub special: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeCategoryTableKind {
    Physical,
    Special,
}

impl TypeCategoryTableKind {
    pub const fn subject(self) -> &'static str {
        match self {
            Self::Physical => "type_categories:physical",
            Self::Special => "type_categories:special",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeCategoryIssue {
    MissingPhysical,
    MissingSpecial,
    InvalidToken {
        table: TypeCategoryTableKind,
        type_id: String,
    },
    Overlap {
        type_id: String,
    },
}

pub fn type_category_issues(categories: &TypeCategories, required: bool) -> Vec<TypeCategoryIssue> {
    if !required {
        return Vec::new();
    }
    let mut issues = Vec::new();
    if categories.physical.is_empty() {
        issues.push(TypeCategoryIssue::MissingPhysical);
    }
    if categories.special.is_empty() {
        issues.push(TypeCategoryIssue::MissingSpecial);
    }
    for type_id in &categories.physical {
        push_type_category_token_issue(TypeCategoryTableKind::Physical, type_id, &mut issues);
    }
    for type_id in &categories.special {
        push_type_category_token_issue(TypeCategoryTableKind::Special, type_id, &mut issues);
    }
    for type_id in &categories.physical {
        if categories.special.iter().any(|entry| entry == type_id) {
            issues.push(TypeCategoryIssue::Overlap {
                type_id: type_id.clone(),
            });
        }
    }
    issues
}

fn push_type_category_token_issue(
    table: TypeCategoryTableKind,
    type_id: &str,
    issues: &mut Vec<TypeCategoryIssue>,
) {
    if type_id.trim().is_empty() || type_id.trim() != type_id {
        issues.push(TypeCategoryIssue::InvalidToken {
            table,
            type_id: type_id.to_string(),
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DamageContext {
    pub is_critical: bool,
    pub is_confusion_damage: bool,
    pub weather: Weather,
    pub random_roll: u8,
}

impl Default for DamageContext {
    fn default() -> Self {
        Self {
            is_critical: false,
            is_confusion_damage: false,
            weather: Weather::None,
            random_roll: 255,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DamageResult {
    pub damage: u16,
    pub type_multiplier: TypeMultiplier,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamageCalculationError {
    MissingStat {
        pokemon_id: String,
        stat: Stat,
    },
    MissingStatStage {
        pokemon_id: String,
        stat: Stat,
    },
    MissingStatMultiplier {
        stage: i8,
    },
    MissingWeatherModifier {
        weather: Weather,
        move_type: PokemonType,
    },
    MissingTypeEffectivenessTable,
    MissingTypeEffectiveness {
        attacker: PokemonType,
        defender: PokemonType,
    },
    MissingTypeCategoryTable,
    MissingTypeCategory {
        move_type: PokemonType,
    },
}

pub fn is_physical_type(
    categories: &TypeCategories,
    move_type: impl AsRef<str>,
) -> Result<bool, DamageCalculationError> {
    if categories.physical.is_empty() || categories.special.is_empty() {
        return Err(DamageCalculationError::MissingTypeCategoryTable);
    }
    let move_type = move_type.as_ref();
    if categories.physical.iter().any(|entry| entry == move_type) {
        return Ok(true);
    }
    if categories.special.iter().any(|entry| entry == move_type) {
        return Ok(false);
    }
    Err(DamageCalculationError::MissingTypeCategory {
        move_type: move_type.to_string(),
    })
}

pub fn type_effectiveness(
    table: &TypeEffectivenessTable,
    move_type: impl AsRef<str>,
    defender_type: impl AsRef<str>,
) -> Result<TypeMultiplier, DamageCalculationError> {
    if table.matchups.is_empty() {
        return Err(DamageCalculationError::MissingTypeEffectivenessTable);
    }
    let move_type = move_type.as_ref();
    let defender_type = defender_type.as_ref();
    table
        .matchups
        .iter()
        .find(|entry| entry.attacker == move_type && entry.defender == defender_type)
        .map(|entry| entry.multiplier)
        .ok_or_else(|| DamageCalculationError::MissingTypeEffectiveness {
            attacker: move_type.to_string(),
            defender: defender_type.to_string(),
        })
}

pub fn calculate_type_effectiveness_multiplier(
    table: &TypeEffectivenessTable,
    move_type: impl AsRef<str>,
    defender_types: &[PokemonType],
) -> Result<TypeMultiplier, DamageCalculationError> {
    let move_type = move_type.as_ref();
    defender_types
        .iter()
        .try_fold(TypeMultiplier::one(), |acc, defender_type| {
            let next = type_effectiveness(table, move_type, defender_type)?;
            if next.numerator == 0 {
                Ok(TypeMultiplier::zero())
            } else {
                Ok(acc.multiply(next))
            }
        })
}

pub fn calculate_damage(
    attacker: &Pokemon,
    defender: &Pokemon,
    move_data: &Move,
    stat_multipliers: &BattleStatMultiplierTables,
    type_categories: &TypeCategories,
    type_effectiveness: &TypeEffectivenessTable,
    weather_modifiers: &WeatherModifiers,
    context: DamageContext,
) -> Result<DamageResult, DamageCalculationError> {
    if move_data.power == 0 {
        return Ok(DamageResult {
            damage: 0,
            type_multiplier: TypeMultiplier::one(),
        });
    }

    let physical = is_physical_type(type_categories, &move_data.move_type)?;
    let attack_stat = if physical {
        Stat::Attack
    } else {
        Stat::SpecialAttack
    };
    let defense_stat = if physical {
        Stat::Defense
    } else {
        Stat::SpecialDefense
    };
    let attack_stage = *attacker.stat_boosts.get(&attack_stat).ok_or_else(|| {
        DamageCalculationError::MissingStatStage {
            pokemon_id: attacker.species.id.clone(),
            stat: attack_stat,
        }
    })?;
    let defense_stage = *defender.stat_boosts.get(&defense_stat).ok_or_else(|| {
        DamageCalculationError::MissingStatStage {
            pokemon_id: defender.species.id.clone(),
            stat: defense_stat,
        }
    })?;

    let base_attack = attacker.calculate_stat(attack_stat).ok_or_else(|| {
        DamageCalculationError::MissingStat {
            pokemon_id: attacker.species.id.clone(),
            stat: attack_stat,
        }
    })?;
    let base_defense = defender.calculate_stat(defense_stat).ok_or_else(|| {
        DamageCalculationError::MissingStat {
            pokemon_id: defender.species.id.clone(),
            stat: defense_stat,
        }
    })?;
    let attack_value = if context.is_critical && defense_stage > attack_stage {
        clamp_stat(base_attack)
    } else {
        clamp_stat(
            apply_stage(stat_multipliers, base_attack, attack_stage).ok_or(
                DamageCalculationError::MissingStatMultiplier {
                    stage: attack_stage,
                },
            )?,
        )
    };
    let defense_value = if context.is_critical && defense_stage > attack_stage {
        clamp_stat(base_defense)
    } else {
        clamp_stat(
            apply_stage(stat_multipliers, base_defense, defense_stage).ok_or(
                DamageCalculationError::MissingStatMultiplier {
                    stage: defense_stage,
                },
            )?,
        )
    }
    .max(1);

    let level_factor = ((2 * attacker.level as u16) / 5) + 2;
    let mut damage = (((level_factor as u32 * move_data.power as u32 * attack_value as u32)
        / defense_value as u32)
        / 50) as u16;
    if context.is_critical {
        damage = damage.saturating_mul(2);
    }
    damage = damage.min(997) + 2;

    damage = apply_weather_type_modifier(
        damage,
        context.weather,
        &move_data.move_type,
        weather_modifiers,
    )?;

    if !context.is_confusion_damage
        && (move_data.move_type == attacker.species.type1
            || move_data.move_type == attacker.species.type2)
    {
        damage = ((damage as u32 * 3) / 2) as u16;
    }

    let defender_types = distinct_defender_types(defender);
    let type_multiplier = if context.is_confusion_damage || move_data.name == "STRUGGLE" {
        TypeMultiplier::one()
    } else {
        calculate_type_effectiveness_multiplier(
            type_effectiveness,
            &move_data.move_type,
            &defender_types,
        )?
    };
    if type_multiplier.numerator == 0 {
        return Ok(DamageResult {
            damage: 0,
            type_multiplier,
        });
    }
    damage = type_multiplier.apply_floor(damage);

    let roll = context.random_roll.max(1);
    damage = ((damage as u32 * roll as u32) / 255).max(1) as u16;

    Ok(DamageResult {
        damage,
        type_multiplier,
    })
}

pub fn apply_weather_type_modifier(
    damage: u16,
    weather: Weather,
    move_type: impl AsRef<str>,
    weather_modifiers: &WeatherModifiers,
) -> Result<u16, DamageCalculationError> {
    let Some(weather_id) = weather.asm_id() else {
        return Ok(damage);
    };
    let move_type = move_type.as_ref();
    let Some(entry) = weather_modifiers
        .type_modifiers
        .iter()
        .find(|entry| entry.weather == weather_id && entry.move_type == move_type)
    else {
        return Err(DamageCalculationError::MissingWeatherModifier {
            weather,
            move_type: move_type.to_string(),
        });
    };
    Ok(entry.multiplier.apply_floor(damage))
}

fn distinct_defender_types(defender: &Pokemon) -> Vec<PokemonType> {
    let mut types = vec![defender.species.type1.clone()];
    if defender.species.type2 != defender.species.type1 {
        types.push(defender.species.type2.clone());
    }
    types
}

fn clamp_stat(value: u16) -> u16 {
    value.clamp(1, 999)
}

const fn gcd(mut a: u16, mut b: u16) -> u16 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::battle::stats::BattleStatMultiplier;
    use crate::models::{
        BaseStats, Dv, PokemonSpecies, create_pokemon_from_known_dvs, growth_rate, pokemon_type,
    };
    use crate::systems::experience::crystal_growth_rate_catalog_for_tests;

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

    fn weather_modifiers() -> WeatherModifiers {
        WeatherModifiers {
            type_modifiers: vec![
                WeatherTypeModifier {
                    weather: "WEATHER_RAIN".to_string(),
                    move_type: pokemon_type("WATER"),
                    multiplier: TypeMultiplier {
                        numerator: 3,
                        denominator: 2,
                    },
                },
                WeatherTypeModifier {
                    weather: "WEATHER_RAIN".to_string(),
                    move_type: pokemon_type("FIRE"),
                    multiplier: TypeMultiplier {
                        numerator: 1,
                        denominator: 2,
                    },
                },
                WeatherTypeModifier {
                    weather: "WEATHER_SUN".to_string(),
                    move_type: pokemon_type("FIRE"),
                    multiplier: TypeMultiplier {
                        numerator: 3,
                        denominator: 2,
                    },
                },
                WeatherTypeModifier {
                    weather: "WEATHER_SUN".to_string(),
                    move_type: pokemon_type("WATER"),
                    multiplier: TypeMultiplier {
                        numerator: 1,
                        denominator: 2,
                    },
                },
            ],
            move_effect_modifiers: vec![],
        }
    }

    #[test]
    fn weather_modifier_issues_validate_exact_pack_ids() {
        let mut moves = BTreeMap::new();
        let mut solarbeam = tackle(pokemon_type("GRASS"), 120);
        solarbeam.name = "SOLARBEAM".to_string();
        solarbeam.effect = "SOLARBEAM".to_string();
        moves.insert(solarbeam.name.clone(), solarbeam);
        let modifiers = WeatherModifiers {
            type_modifiers: vec![WeatherTypeModifier {
                weather: " WEATHER_RAIN".to_string(),
                move_type: pokemon_type("WATER"),
                multiplier: TypeMultiplier {
                    numerator: 1,
                    denominator: 0,
                },
            }],
            move_effect_modifiers: vec![WeatherMoveEffectModifier {
                weather: String::new(),
                move_effect: " SOLARBEAM".to_string(),
                multiplier: TypeMultiplier {
                    numerator: 1,
                    denominator: 0,
                },
            }],
        };
        let unknown_effect_modifiers = WeatherModifiers {
            type_modifiers: vec![WeatherTypeModifier {
                weather: "WEATHER_SUN".to_string(),
                move_type: pokemon_type("FIRE"),
                multiplier: TypeMultiplier::one(),
            }],
            move_effect_modifiers: vec![WeatherMoveEffectModifier {
                weather: "WEATHER_SUN".to_string(),
                move_effect: "MOONBEAM".to_string(),
                multiplier: TypeMultiplier::one(),
            }],
        };

        assert_eq!(
            weather_modifier_issues(&modifiers, &moves, true),
            vec![
                WeatherModifierIssue::InvalidWeather {
                    table: WeatherModifierTableKind::TypeModifiers,
                    weather: " WEATHER_RAIN".to_string(),
                },
                WeatherModifierIssue::InvalidMultiplierDenominator {
                    table: WeatherModifierTableKind::TypeModifiers,
                },
                WeatherModifierIssue::InvalidWeather {
                    table: WeatherModifierTableKind::MoveEffectModifiers,
                    weather: String::new(),
                },
                WeatherModifierIssue::InvalidMoveEffect {
                    move_effect: " SOLARBEAM".to_string(),
                },
                WeatherModifierIssue::InvalidMultiplierDenominator {
                    table: WeatherModifierTableKind::MoveEffectModifiers,
                },
            ]
        );
        assert_eq!(
            weather_modifier_issues(&unknown_effect_modifiers, &moves, true),
            vec![WeatherModifierIssue::UnknownMoveEffect {
                move_effect: "MOONBEAM".to_string(),
            }]
        );
        assert_eq!(
            weather_modifier_issues(&WeatherModifiers::default(), &moves, true),
            vec![
                WeatherModifierIssue::MissingTypeModifiers,
                WeatherModifierIssue::MissingMoveEffectModifiers,
            ]
        );
        assert_eq!(weather_modifier_issues(&modifiers, &moves, false), []);
    }

    fn type_effectiveness_table() -> TypeEffectivenessTable {
        TypeEffectivenessTable {
            matchups: vec![
                TypeEffectivenessEntry {
                    attacker: pokemon_type("NORMAL"),
                    defender: pokemon_type("NORMAL"),
                    multiplier: TypeMultiplier::one(),
                },
                TypeEffectivenessEntry {
                    attacker: pokemon_type("GHOST"),
                    defender: pokemon_type("STEEL"),
                    multiplier: TypeMultiplier {
                        numerator: 1,
                        denominator: 2,
                    },
                },
                TypeEffectivenessEntry {
                    attacker: pokemon_type("DARK"),
                    defender: pokemon_type("STEEL"),
                    multiplier: TypeMultiplier {
                        numerator: 1,
                        denominator: 2,
                    },
                },
                TypeEffectivenessEntry {
                    attacker: pokemon_type("ELECTRIC"),
                    defender: pokemon_type("GROUND"),
                    multiplier: TypeMultiplier::zero(),
                },
                TypeEffectivenessEntry {
                    attacker: pokemon_type("ICE"),
                    defender: pokemon_type("GRASS"),
                    multiplier: TypeMultiplier {
                        numerator: 2,
                        denominator: 1,
                    },
                },
                TypeEffectivenessEntry {
                    attacker: pokemon_type("ICE"),
                    defender: pokemon_type("FLYING"),
                    multiplier: TypeMultiplier {
                        numerator: 2,
                        denominator: 1,
                    },
                },
                TypeEffectivenessEntry {
                    attacker: pokemon_type("FIRE"),
                    defender: pokemon_type("GRASS"),
                    multiplier: TypeMultiplier {
                        numerator: 2,
                        denominator: 1,
                    },
                },
            ],
            foresight_matchups: vec![
                TypeEffectivenessEntry {
                    attacker: pokemon_type("NORMAL"),
                    defender: pokemon_type("GHOST"),
                    multiplier: TypeMultiplier::zero(),
                },
                TypeEffectivenessEntry {
                    attacker: pokemon_type("FIGHTING"),
                    defender: pokemon_type("GHOST"),
                    multiplier: TypeMultiplier::zero(),
                },
            ],
        }
    }

    fn type_categories() -> TypeCategories {
        TypeCategories {
            physical: vec![
                "NORMAL".to_string(),
                "FIGHTING".to_string(),
                "FLYING".to_string(),
                "POISON".to_string(),
                "GROUND".to_string(),
                "ROCK".to_string(),
                "BUG".to_string(),
                "GHOST".to_string(),
                "STEEL".to_string(),
            ],
            special: vec![
                "FIRE".to_string(),
                "WATER".to_string(),
                "GRASS".to_string(),
                "ELECTRIC".to_string(),
                "PSYCHIC_TYPE".to_string(),
                "ICE".to_string(),
                "DRAGON".to_string(),
                "DARK".to_string(),
            ],
        }
    }

    #[test]
    fn type_category_issues_validate_exact_pack_tokens() {
        let categories = TypeCategories {
            physical: vec![
                "NORMAL".to_string(),
                "fire".to_string(),
                " FIGHTING".to_string(),
            ],
            special: vec!["FIRE".to_string(), String::new(), "NORMAL".to_string()],
        };

        assert_eq!(
            type_category_issues(&categories, true),
            vec![
                TypeCategoryIssue::InvalidToken {
                    table: TypeCategoryTableKind::Physical,
                    type_id: " FIGHTING".to_string(),
                },
                TypeCategoryIssue::InvalidToken {
                    table: TypeCategoryTableKind::Special,
                    type_id: String::new(),
                },
                TypeCategoryIssue::Overlap {
                    type_id: "NORMAL".to_string(),
                },
            ]
        );
        assert_eq!(
            type_category_issues(&TypeCategories::default(), true),
            vec![
                TypeCategoryIssue::MissingPhysical,
                TypeCategoryIssue::MissingSpecial,
            ]
        );
        assert_eq!(type_category_issues(&categories, false), []);
    }

    #[test]
    fn type_effectiveness_table_issues_validate_definitive_rows() {
        let categories = TypeCategories {
            physical: vec!["NORMAL".to_string()],
            special: vec!["FIRE".to_string()],
        };
        let table = TypeEffectivenessTable {
            matchups: vec![
                TypeEffectivenessEntry {
                    attacker: pokemon_type("NORMAL"),
                    defender: pokemon_type("NORMAL"),
                    multiplier: TypeMultiplier {
                        numerator: 1,
                        denominator: 0,
                    },
                },
                TypeEffectivenessEntry {
                    attacker: pokemon_type("NORMAL"),
                    defender: pokemon_type("NORMAL"),
                    multiplier: TypeMultiplier::one(),
                },
                TypeEffectivenessEntry {
                    attacker: pokemon_type("WATER"),
                    defender: pokemon_type("FIRE"),
                    multiplier: TypeMultiplier::one(),
                },
                TypeEffectivenessEntry {
                    attacker: pokemon_type("FIRE"),
                    defender: pokemon_type("WATER"),
                    multiplier: TypeMultiplier::one(),
                },
            ],
            foresight_matchups: vec![
                TypeEffectivenessEntry {
                    attacker: pokemon_type("NORMAL"),
                    defender: pokemon_type("WATER"),
                    multiplier: TypeMultiplier {
                        numerator: 1,
                        denominator: 0,
                    },
                },
                TypeEffectivenessEntry {
                    attacker: pokemon_type("NORMAL"),
                    defender: pokemon_type("WATER"),
                    multiplier: TypeMultiplier::one(),
                },
            ],
        };

        assert_eq!(
            type_effectiveness_table_issues(&table, &categories, true),
            vec![
                TypeEffectivenessTableIssue::InvalidMultiplierDenominator {
                    table: TypeEffectivenessTableKind::Matchups,
                },
                TypeEffectivenessTableIssue::DuplicateMatchup {
                    table: TypeEffectivenessTableKind::Matchups,
                    attacker: pokemon_type("NORMAL"),
                    defender: pokemon_type("NORMAL"),
                },
                TypeEffectivenessTableIssue::UnknownAttacker {
                    table: TypeEffectivenessTableKind::Matchups,
                    attacker: pokemon_type("WATER"),
                },
                TypeEffectivenessTableIssue::UnknownDefender {
                    table: TypeEffectivenessTableKind::Matchups,
                    defender: pokemon_type("WATER"),
                },
                TypeEffectivenessTableIssue::MissingMatchup {
                    attacker: "FIRE".to_string(),
                    defender: "FIRE".to_string(),
                },
                TypeEffectivenessTableIssue::MissingMatchup {
                    attacker: "FIRE".to_string(),
                    defender: "NORMAL".to_string(),
                },
                TypeEffectivenessTableIssue::MissingMatchup {
                    attacker: "NORMAL".to_string(),
                    defender: "FIRE".to_string(),
                },
                TypeEffectivenessTableIssue::InvalidMultiplierDenominator {
                    table: TypeEffectivenessTableKind::ForesightMatchups,
                },
                TypeEffectivenessTableIssue::UnknownDefender {
                    table: TypeEffectivenessTableKind::ForesightMatchups,
                    defender: pokemon_type("WATER"),
                },
                TypeEffectivenessTableIssue::UnknownDefender {
                    table: TypeEffectivenessTableKind::ForesightMatchups,
                    defender: pokemon_type("WATER"),
                },
                TypeEffectivenessTableIssue::DuplicateMatchup {
                    table: TypeEffectivenessTableKind::ForesightMatchups,
                    attacker: pokemon_type("NORMAL"),
                    defender: pokemon_type("WATER"),
                },
            ],
        );
        assert_eq!(
            type_effectiveness_table_issues(
                &TypeEffectivenessTable::default(),
                &TypeCategories::default(),
                true
            ),
            vec![
                TypeEffectivenessTableIssue::MissingMatchups,
                TypeEffectivenessTableIssue::MissingForesightMatchups,
            ],
        );
        assert_eq!(
            type_effectiveness_table_issues(&table, &categories, false),
            []
        );
    }

    fn species(id: &str, pokemon_type: PokemonType, stats: BaseStats) -> PokemonSpecies {
        let mut species = PokemonSpecies::new_for_tests(id, stats);
        species.type1 = pokemon_type.clone();
        species.type2 = pokemon_type;
        species.growth_rate = growth_rate("GROWTH_MEDIUM_FAST");
        species
    }

    fn pokemon(id: &str, pokemon_type: PokemonType, stats: BaseStats, level: u8) -> Pokemon {
        let growth_rates = crystal_growth_rate_catalog_for_tests();
        let learnsets = [(id.to_string(), Vec::new())].into_iter().collect();
        create_pokemon_from_known_dvs(
            &species(id, pokemon_type, stats),
            level,
            Dv::from_non_hp(10, 10, 10, 10),
            &learnsets,
            &BTreeMap::new(),
            &growth_rates,
        )
        .expect("test Pokemon builds from explicit empty learnset")
    }

    fn tackle(move_type: PokemonType, power: u16) -> Move {
        Move {
            name: "TACKLE".to_string(),
            move_type,
            power,
            accuracy: 100,
            pp: 35,
            effect: "NORMAL_HIT".to_string(),
            effect_chance: 0,
            stat: None,
            amount: None,
        }
    }

    #[test]
    fn gen_two_type_chart_includes_steel_dark_and_immunities() {
        assert_eq!(
            calculate_type_effectiveness_multiplier(
                &type_effectiveness_table(),
                pokemon_type("GHOST"),
                &[pokemon_type("STEEL")]
            )
            .expect("type effectiveness calculates"),
            TypeMultiplier {
                numerator: 1,
                denominator: 2
            }
        );
        assert_eq!(
            calculate_type_effectiveness_multiplier(
                &type_effectiveness_table(),
                pokemon_type("DARK"),
                &[pokemon_type("STEEL")]
            )
            .expect("type effectiveness calculates"),
            TypeMultiplier {
                numerator: 1,
                denominator: 2
            }
        );
        assert_eq!(
            calculate_type_effectiveness_multiplier(
                &type_effectiveness_table(),
                pokemon_type("ELECTRIC"),
                &[pokemon_type("GROUND")]
            )
            .expect("type effectiveness calculates"),
            TypeMultiplier::zero()
        );
        assert_eq!(
            calculate_type_effectiveness_multiplier(
                &type_effectiveness_table(),
                pokemon_type("ICE"),
                &[pokemon_type("GRASS"), pokemon_type("FLYING")]
            )
            .expect("type effectiveness calculates"),
            TypeMultiplier {
                numerator: 4,
                denominator: 1
            }
        );
    }

    #[test]
    fn type_effectiveness_requires_explicit_neutral_rows() {
        let error = calculate_type_effectiveness_multiplier(
            &type_effectiveness_table(),
            pokemon_type("NORMAL"),
            &[pokemon_type("GRASS")],
        )
        .expect_err("missing neutral row must not default to one");

        assert_eq!(
            error,
            DamageCalculationError::MissingTypeEffectiveness {
                attacker: pokemon_type("NORMAL"),
                defender: pokemon_type("GRASS"),
            }
        );
    }

    #[test]
    fn physical_type_split_matches_gen_two() {
        assert!(is_physical_type(&type_categories(), pokemon_type("GHOST")).expect("known type"));
        assert!(is_physical_type(&type_categories(), pokemon_type("STEEL")).expect("known type"));
        assert!(!is_physical_type(&type_categories(), pokemon_type("FIRE")).expect("known type"));
        assert!(!is_physical_type(&type_categories(), pokemon_type("DARK")).expect("known type"));
    }

    #[test]
    fn damage_applies_stab_type_multiplier_and_random_roll_deterministically() {
        let attacker = pokemon(
            "ATTACKER",
            pokemon_type("FIRE"),
            BaseStats::new(80, 84, 78, 100, 109, 85),
            50,
        );
        let defender = pokemon(
            "DEFENDER",
            pokemon_type("GRASS"),
            BaseStats::new(80, 82, 83, 80, 100, 100),
            50,
        );
        let result = calculate_damage(
            &attacker,
            &defender,
            &tackle(pokemon_type("FIRE"), 60),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            DamageContext::default(),
        )
        .expect("damage calculates");

        assert_eq!(
            result.type_multiplier,
            TypeMultiplier {
                numerator: 2,
                denominator: 1
            }
        );
        assert_eq!(result.damage, 90);
    }

    #[test]
    fn zero_effectiveness_returns_zero_damage() {
        let attacker = pokemon(
            "ATTACKER",
            pokemon_type("ELECTRIC"),
            BaseStats::new(35, 55, 40, 90, 50, 50),
            30,
        );
        let defender = pokemon(
            "DEFENDER",
            pokemon_type("GROUND"),
            BaseStats::new(50, 50, 95, 35, 40, 50),
            30,
        );

        let result = calculate_damage(
            &attacker,
            &defender,
            &tackle(pokemon_type("ELECTRIC"), 40),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            DamageContext::default(),
        )
        .expect("damage calculates");

        assert_eq!(result.damage, 0);
        assert_eq!(result.type_multiplier, TypeMultiplier::zero());
    }

    #[test]
    fn damage_requires_explicit_battle_stat_stages_without_zero_fallback() {
        let mut attacker = pokemon(
            "ATTACKER",
            pokemon_type("NORMAL"),
            BaseStats::new(80, 84, 78, 100, 109, 85),
            50,
        );
        let defender = pokemon(
            "DEFENDER",
            pokemon_type("NORMAL"),
            BaseStats::new(80, 82, 83, 80, 100, 100),
            50,
        );
        attacker.stat_boosts.remove(&Stat::Attack);

        let error = calculate_damage(
            &attacker,
            &defender,
            &tackle(pokemon_type("NORMAL"), 60),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            DamageContext::default(),
        )
        .expect_err("missing attack stage must not default to zero");

        assert_eq!(
            error,
            DamageCalculationError::MissingStatStage {
                pokemon_id: "ATTACKER".to_string(),
                stat: Stat::Attack,
            }
        );
    }
}
