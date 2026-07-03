use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::battle::stats::{BattleStatMultiplierTables, apply_stage};
use crate::models::{Move, Pokemon, PokemonType, Stat};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TypeMultiplier {
    pub numerator: u16,
    pub denominator: u16,
}

impl<'de> Deserialize<'de> for TypeMultiplier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawTypeMultiplier {
            numerator: u16,
            denominator: u16,
        }

        let raw = RawTypeMultiplier::deserialize(deserializer)?;
        let multiplier = Self {
            numerator: raw.numerator,
            denominator: raw.denominator,
        };
        validate_type_multiplier("type multiplier", multiplier).map_err(D::Error::custom)?;
        Ok(multiplier)
    }
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
#[serde(deny_unknown_fields)]
pub enum Weather {
    None,
    Rain,
    Sandstorm,
    Sun,
}

impl Weather {
    pub const fn asm_id(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Rain => Some("WEATHER_RAIN"),
            Self::Sandstorm => Some("WEATHER_SANDSTORM"),
            Self::Sun => Some("WEATHER_SUN"),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WeatherModifiers {
    pub type_modifiers: BTreeMap<String, BTreeMap<String, TypeMultiplier>>,
    pub move_effect_modifiers: BTreeMap<String, BTreeMap<String, TypeMultiplier>>,
}

impl<'de> Deserialize<'de> for WeatherModifiers {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawWeatherModifiers {
            type_modifiers: BTreeMap<String, BTreeMap<String, TypeMultiplier>>,
            move_effect_modifiers: BTreeMap<String, BTreeMap<String, TypeMultiplier>>,
        }

        let raw = RawWeatherModifiers::deserialize(deserializer)?;
        let modifiers = Self {
            type_modifiers: raw.type_modifiers,
            move_effect_modifiers: raw.move_effect_modifiers,
        };
        modifiers.validate_shape().map_err(D::Error::custom)?;
        Ok(modifiers)
    }
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
    InvalidMoveType {
        move_type: String,
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
    for (weather, type_modifiers) in &modifiers.type_modifiers {
        if !is_exact_battle_damage_token(weather) {
            issues.push(WeatherModifierIssue::InvalidWeather {
                table: WeatherModifierTableKind::TypeModifiers,
                weather: weather.clone(),
            });
        }
        for (move_type, multiplier) in type_modifiers {
            if !is_exact_battle_damage_token(move_type) {
                issues.push(WeatherModifierIssue::InvalidMoveType {
                    move_type: move_type.clone(),
                });
            }
            push_type_multiplier_issue(
                WeatherModifierTableKind::TypeModifiers,
                *multiplier,
                &mut issues,
            );
        }
    }
    for (weather, move_effect_modifiers) in &modifiers.move_effect_modifiers {
        if !is_exact_battle_damage_token(weather) {
            issues.push(WeatherModifierIssue::InvalidWeather {
                table: WeatherModifierTableKind::MoveEffectModifiers,
                weather: weather.clone(),
            });
        }
        for (move_effect, multiplier) in move_effect_modifiers {
            if !is_exact_battle_damage_token(move_effect) {
                issues.push(WeatherModifierIssue::InvalidMoveEffect {
                    move_effect: move_effect.clone(),
                });
            } else if !move_effect_ids.contains(move_effect.as_str()) {
                issues.push(WeatherModifierIssue::UnknownMoveEffect {
                    move_effect: move_effect.clone(),
                });
            }
            push_type_multiplier_issue(
                WeatherModifierTableKind::MoveEffectModifiers,
                *multiplier,
                &mut issues,
            );
        }
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

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TypeEffectivenessTable {
    pub matchups: BTreeMap<String, BTreeMap<String, TypeMultiplier>>,
    pub foresight_matchups: BTreeMap<String, BTreeMap<String, TypeMultiplier>>,
}

impl<'de> Deserialize<'de> for TypeEffectivenessTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawTypeEffectivenessTable {
            matchups: BTreeMap<String, BTreeMap<String, TypeMultiplier>>,
            foresight_matchups: BTreeMap<String, BTreeMap<String, TypeMultiplier>>,
        }

        let raw = RawTypeEffectivenessTable::deserialize(deserializer)?;
        let table = Self {
            matchups: raw.matchups,
            foresight_matchups: raw.foresight_matchups,
        };
        table.validate_shape().map_err(D::Error::custom)?;
        Ok(table)
    }
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
    InvalidAttacker {
        table: TypeEffectivenessTableKind,
        attacker: PokemonType,
    },
    InvalidDefender {
        table: TypeEffectivenessTableKind,
        defender: PokemonType,
    },
    UnknownAttacker {
        table: TypeEffectivenessTableKind,
        attacker: PokemonType,
    },
    UnknownDefender {
        table: TypeEffectivenessTableKind,
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
    for (attacker, defenders) in &table.matchups {
        push_type_effectiveness_entry_issues(
            TypeEffectivenessTableKind::Matchups,
            attacker,
            defenders,
            &declared_types,
            &mut issues,
        );
    }
    for attacker in &declared_types {
        for defender in &declared_types {
            if !table
                .matchups
                .get(*attacker)
                .is_some_and(|defenders| defenders.contains_key(*defender))
            {
                issues.push(TypeEffectivenessTableIssue::MissingMatchup {
                    attacker: (*attacker).to_string(),
                    defender: (*defender).to_string(),
                });
            }
        }
    }

    for (attacker, defenders) in &table.foresight_matchups {
        push_type_effectiveness_entry_issues(
            TypeEffectivenessTableKind::ForesightMatchups,
            attacker,
            defenders,
            &declared_types,
            &mut issues,
        );
    }

    issues
}

fn push_type_effectiveness_entry_issues(
    table: TypeEffectivenessTableKind,
    attacker: &str,
    defenders: &BTreeMap<String, TypeMultiplier>,
    declared_types: &BTreeSet<&str>,
    issues: &mut Vec<TypeEffectivenessTableIssue>,
) {
    if declared_types.is_empty() {
        return;
    }
    if !is_exact_battle_damage_token(attacker) {
        issues.push(TypeEffectivenessTableIssue::InvalidAttacker {
            table,
            attacker: attacker.to_string(),
        });
    } else if !declared_types.contains(attacker) {
        issues.push(TypeEffectivenessTableIssue::UnknownAttacker {
            table,
            attacker: attacker.to_string(),
        });
    }
    for (defender, multiplier) in defenders {
        if multiplier.denominator == 0 {
            issues.push(TypeEffectivenessTableIssue::InvalidMultiplierDenominator { table });
        }
        if !is_exact_battle_damage_token(defender) {
            issues.push(TypeEffectivenessTableIssue::InvalidDefender {
                table,
                defender: defender.clone(),
            });
        } else if !declared_types.contains(defender.as_str()) {
            issues.push(TypeEffectivenessTableIssue::UnknownDefender {
                table,
                defender: defender.clone(),
            });
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TypeCategories {
    pub physical: Vec<String>,
    pub special: Vec<String>,
}

impl<'de> Deserialize<'de> for TypeCategories {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawTypeCategories {
            physical: Vec<String>,
            special: Vec<String>,
        }

        let raw = RawTypeCategories::deserialize(deserializer)?;
        let categories = Self {
            physical: raw.physical,
            special: raw.special,
        };
        categories.validate_shape().map_err(D::Error::custom)?;
        Ok(categories)
    }
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
    if !is_exact_battle_damage_token(type_id) {
        issues.push(TypeCategoryIssue::InvalidToken {
            table,
            type_id: type_id.to_string(),
        });
    }
}

fn is_exact_battle_damage_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !has_reserved_pack_prefix(value)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn has_reserved_pack_prefix(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("fallback") || value.starts_with("legacy")
}

fn validate_type_multiplier(subject: &str, multiplier: TypeMultiplier) -> Result<(), String> {
    if multiplier.denominator == 0 {
        return Err(format!("{subject} denominator must be nonzero"));
    }
    Ok(())
}

impl WeatherModifiers {
    fn validate_shape(&self) -> Result<(), String> {
        if self.type_modifiers.is_empty() {
            return Err("weather type_modifiers must be explicit".to_string());
        }
        if self.move_effect_modifiers.is_empty() {
            return Err("weather move_effect_modifiers must be explicit".to_string());
        }
        for (weather, type_modifiers) in &self.type_modifiers {
            validate_exact_damage_token("weather type_modifiers weather", weather)?;
            if type_modifiers.is_empty() {
                return Err(format!(
                    "weather {weather} type_modifiers must not be empty"
                ));
            }
            for (move_type, multiplier) in type_modifiers {
                validate_exact_damage_token("weather type modifier move type", move_type)?;
                validate_type_multiplier("weather type modifier", *multiplier)?;
            }
        }
        for (weather, effect_modifiers) in &self.move_effect_modifiers {
            validate_exact_damage_token("weather move_effect_modifiers weather", weather)?;
            if effect_modifiers.is_empty() {
                return Err(format!(
                    "weather {weather} move_effect_modifiers must not be empty"
                ));
            }
            for (move_effect, multiplier) in effect_modifiers {
                validate_exact_damage_token("weather move effect modifier", move_effect)?;
                validate_type_multiplier("weather move effect modifier", *multiplier)?;
            }
        }
        Ok(())
    }
}

impl TypeEffectivenessTable {
    fn validate_shape(&self) -> Result<(), String> {
        if self.matchups.is_empty() {
            return Err("type effectiveness matchups must be explicit".to_string());
        }
        if self.foresight_matchups.is_empty() {
            return Err("type effectiveness foresight_matchups must be explicit".to_string());
        }
        validate_type_effectiveness_shape("matchups", &self.matchups)?;
        validate_type_effectiveness_shape("foresight_matchups", &self.foresight_matchups)?;
        Ok(())
    }
}

fn validate_type_effectiveness_shape(
    table_name: &str,
    table: &BTreeMap<String, BTreeMap<String, TypeMultiplier>>,
) -> Result<(), String> {
    for (attacker, defenders) in table {
        validate_exact_damage_token("type effectiveness attacker", attacker)?;
        if defenders.is_empty() {
            return Err(format!(
                "type effectiveness {table_name} attacker {attacker} has no defenders"
            ));
        }
        for (defender, multiplier) in defenders {
            validate_exact_damage_token("type effectiveness defender", defender)?;
            validate_type_multiplier("type effectiveness matchup", *multiplier)?;
        }
    }
    Ok(())
}

impl TypeCategories {
    fn validate_shape(&self) -> Result<(), String> {
        if self.physical.is_empty() {
            return Err("type_categories physical must be explicit".to_string());
        }
        if self.special.is_empty() {
            return Err("type_categories special must be explicit".to_string());
        }
        let mut physical = BTreeSet::new();
        for type_id in &self.physical {
            validate_exact_damage_token("physical type category", type_id)?;
            if !physical.insert(type_id.as_str()) {
                return Err(format!("physical type category {type_id} is duplicated"));
            }
        }
        let mut special = BTreeSet::new();
        for type_id in &self.special {
            validate_exact_damage_token("special type category", type_id)?;
            if !special.insert(type_id.as_str()) {
                return Err(format!("special type category {type_id} is duplicated"));
            }
            if physical.contains(type_id.as_str()) {
                return Err(format!(
                    "type category {type_id} cannot be both physical and special"
                ));
            }
        }
        Ok(())
    }
}

fn validate_exact_damage_token(subject: &str, value: &str) -> Result<(), String> {
    if !is_exact_battle_damage_token(value) {
        return Err(format!("{subject} {value:?} is not exact"));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DamageContext {
    pub is_critical: bool,
    pub is_confusion_damage: bool,
    pub defender_identified: bool,
    pub weather: Weather,
    pub random_roll: u8,
}

impl Default for DamageContext {
    fn default() -> Self {
        Self {
            is_critical: false,
            is_confusion_damage: false,
            defender_identified: false,
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
#[serde(deny_unknown_fields)]
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
    InvalidWeatherModifierType {
        move_type: PokemonType,
    },
    MissingTypeEffectivenessTable,
    InvalidTypeEffectivenessAttacker {
        attacker: PokemonType,
    },
    InvalidTypeEffectivenessDefender {
        defender: PokemonType,
    },
    MissingTypeEffectiveness {
        attacker: PokemonType,
        defender: PokemonType,
    },
    MissingTypeCategoryTable,
    InvalidTypeCategory {
        move_type: PokemonType,
    },
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
    if !is_exact_battle_damage_token(move_type) {
        return Err(DamageCalculationError::InvalidTypeCategory {
            move_type: move_type.to_string(),
        });
    }
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
    type_effectiveness_from_matchups(&table.matchups, move_type, defender_type)
}

pub fn foresight_type_effectiveness(
    table: &TypeEffectivenessTable,
    move_type: impl AsRef<str>,
    defender_type: impl AsRef<str>,
) -> Result<TypeMultiplier, DamageCalculationError> {
    let move_type = move_type.as_ref();
    let defender_type = defender_type.as_ref();
    validate_type_effectiveness_tokens(move_type, defender_type)?;
    if let Some(multiplier) = table
        .foresight_matchups
        .get(move_type)
        .and_then(|defenders| defenders.get(defender_type))
        .copied()
    {
        return Ok(multiplier);
    }
    type_effectiveness(table, move_type, defender_type)
}

fn type_effectiveness_from_matchups(
    matchups: &BTreeMap<String, BTreeMap<String, TypeMultiplier>>,
    move_type: impl AsRef<str>,
    defender_type: impl AsRef<str>,
) -> Result<TypeMultiplier, DamageCalculationError> {
    if matchups.is_empty() {
        return Err(DamageCalculationError::MissingTypeEffectivenessTable);
    }
    let move_type = move_type.as_ref();
    let defender_type = defender_type.as_ref();
    if !is_exact_battle_damage_token(move_type) {
        return Err(invalid_type_effectiveness_attacker(move_type));
    }
    if !is_exact_battle_damage_token(defender_type) {
        return Err(invalid_type_effectiveness_defender(defender_type));
    }
    matchups
        .get(move_type)
        .and_then(|defenders| defenders.get(defender_type))
        .copied()
        .ok_or_else(|| DamageCalculationError::MissingTypeEffectiveness {
            attacker: move_type.to_string(),
            defender: defender_type.to_string(),
        })
}

fn validate_type_effectiveness_tokens(
    move_type: &str,
    defender_type: &str,
) -> Result<(), DamageCalculationError> {
    if !is_exact_battle_damage_token(move_type) {
        return Err(invalid_type_effectiveness_attacker(move_type));
    }
    if !is_exact_battle_damage_token(defender_type) {
        return Err(invalid_type_effectiveness_defender(defender_type));
    }
    Ok(())
}

fn invalid_type_effectiveness_attacker(move_type: &str) -> DamageCalculationError {
    DamageCalculationError::InvalidTypeEffectivenessAttacker {
        attacker: move_type.to_string(),
    }
}

fn invalid_type_effectiveness_defender(defender_type: &str) -> DamageCalculationError {
    DamageCalculationError::InvalidTypeEffectivenessDefender {
        defender: defender_type.to_string(),
    }
}

pub fn calculate_type_effectiveness_multiplier(
    table: &TypeEffectivenessTable,
    move_type: impl AsRef<str>,
    defender_types: &[PokemonType],
) -> Result<TypeMultiplier, DamageCalculationError> {
    calculate_type_effectiveness_multiplier_with_foresight(table, move_type, defender_types, false)
}

pub fn calculate_type_effectiveness_multiplier_with_foresight(
    table: &TypeEffectivenessTable,
    move_type: impl AsRef<str>,
    defender_types: &[PokemonType],
    defender_identified: bool,
) -> Result<TypeMultiplier, DamageCalculationError> {
    let move_type = move_type.as_ref();
    defender_types
        .iter()
        .try_fold(TypeMultiplier::one(), |acc, defender_type| {
            let next = if defender_identified {
                foresight_type_effectiveness(table, move_type, defender_type)?
            } else {
                type_effectiveness(table, move_type, defender_type)?
            };
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
    let attack_value = apply_burn_attack_penalty(attacker, physical, attack_value);
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
        calculate_type_effectiveness_multiplier_with_foresight(
            type_effectiveness,
            &move_data.move_type,
            &defender_types,
            context.defender_identified,
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
    if !is_exact_battle_damage_token(move_type) {
        return Err(DamageCalculationError::InvalidWeatherModifierType {
            move_type: move_type.to_string(),
        });
    }
    let Some(entry) = weather_modifiers
        .type_modifiers
        .get(weather_id)
        .and_then(|modifiers| modifiers.get(move_type))
    else {
        return Err(DamageCalculationError::MissingWeatherModifier {
            weather,
            move_type: move_type.to_string(),
        });
    };
    Ok(entry.apply_floor(damage))
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

fn apply_burn_attack_penalty(attacker: &Pokemon, physical: bool, attack_value: u16) -> u16 {
    if physical && attacker.status.as_deref() == Some("BURN") {
        (attack_value / 2).max(1)
    } else {
        attack_value
    }
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

    #[test]
    fn damage_error_json_rejects_unknown_fallback_fields() {
        let weather_error = serde_json::from_value::<DamageCalculationError>(serde_json::json!({
            "MissingWeatherModifier": {
                "weather": "Rain",
                "move_type": "WATER",
                "fallback_multiplier": { "numerator": 1, "denominator": 1 }
            }
        }))
        .expect_err("damage errors must not accept fallback multipliers")
        .to_string();
        assert!(
            weather_error.contains("unknown field `fallback_multiplier`"),
            "{weather_error}"
        );

        let category_error = serde_json::from_value::<DamageCalculationError>(serde_json::json!({
            "MissingTypeCategory": {
                "move_type": "FIRE",
                "default_category": "special"
            }
        }))
        .expect_err("damage errors must not accept default type categories")
        .to_string();
        assert!(
            category_error.contains("unknown field `default_category`"),
            "{category_error}"
        );
    }

    #[test]
    fn weather_json_rejects_legacy_alias_payloads() {
        let error = serde_json::from_value::<Weather>(serde_json::json!({
            "Rain": {
                "fallback_weather": "WEATHER_RAIN"
            }
        }))
        .expect_err("weather must not accept object-shaped fallback aliases")
        .to_string();
        assert!(
            error.contains("invalid type") || error.contains("unknown field `fallback_weather`"),
            "{error}"
        );
    }

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
            type_modifiers: [
                (
                    "WEATHER_RAIN".to_string(),
                    [
                        (
                            "WATER".to_string(),
                            TypeMultiplier {
                                numerator: 3,
                                denominator: 2,
                            },
                        ),
                        (
                            "FIRE".to_string(),
                            TypeMultiplier {
                                numerator: 1,
                                denominator: 2,
                            },
                        ),
                    ]
                    .into_iter()
                    .collect(),
                ),
                (
                    "WEATHER_SUN".to_string(),
                    [
                        (
                            "FIRE".to_string(),
                            TypeMultiplier {
                                numerator: 3,
                                denominator: 2,
                            },
                        ),
                        (
                            "WATER".to_string(),
                            TypeMultiplier {
                                numerator: 1,
                                denominator: 2,
                            },
                        ),
                    ]
                    .into_iter()
                    .collect(),
                ),
            ]
            .into_iter()
            .collect(),
            move_effect_modifiers: BTreeMap::new(),
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
            type_modifiers: [
                (
                    " WEATHER_RAIN".to_string(),
                    [(
                        "WATER".to_string(),
                        TypeMultiplier {
                            numerator: 1,
                            denominator: 0,
                        },
                    )]
                    .into_iter()
                    .collect(),
                ),
                (
                    "WEATHER RAIN".to_string(),
                    [("WATER".to_string(), TypeMultiplier::one())]
                        .into_iter()
                        .collect(),
                ),
            ]
            .into_iter()
            .collect(),
            move_effect_modifiers: [
                (
                    String::new(),
                    [(
                        " SOLARBEAM".to_string(),
                        TypeMultiplier {
                            numerator: 1,
                            denominator: 0,
                        },
                    )]
                    .into_iter()
                    .collect(),
                ),
                (
                    "WEATHER SUN".to_string(),
                    [("SOLAR BEAM".to_string(), TypeMultiplier::one())]
                        .into_iter()
                        .collect(),
                ),
            ]
            .into_iter()
            .collect(),
        };
        let unknown_effect_modifiers = WeatherModifiers {
            type_modifiers: [(
                "WEATHER_SUN".to_string(),
                [("FIRE".to_string(), TypeMultiplier::one())]
                    .into_iter()
                    .collect(),
            )]
            .into_iter()
            .collect(),
            move_effect_modifiers: [(
                "WEATHER_SUN".to_string(),
                [("MOONBEAM".to_string(), TypeMultiplier::one())]
                    .into_iter()
                    .collect(),
            )]
            .into_iter()
            .collect(),
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
                    table: WeatherModifierTableKind::TypeModifiers,
                    weather: "WEATHER RAIN".to_string(),
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
                WeatherModifierIssue::InvalidWeather {
                    table: WeatherModifierTableKind::MoveEffectModifiers,
                    weather: "WEATHER SUN".to_string(),
                },
                WeatherModifierIssue::InvalidMoveEffect {
                    move_effect: "SOLAR BEAM".to_string(),
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
        serde_json::from_value(serde_json::json!({
            "matchups": {
                "NORMAL": {
                    "NORMAL": { "numerator": 1, "denominator": 1 },
                    "GHOST": { "numerator": 0, "denominator": 1 }
                },
                "FIGHTING": {
                    "GHOST": { "numerator": 0, "denominator": 1 }
                },
                "GHOST": { "STEEL": { "numerator": 1, "denominator": 2 } },
                "DARK": { "STEEL": { "numerator": 1, "denominator": 2 } },
                "ELECTRIC": { "GROUND": { "numerator": 0, "denominator": 1 } },
                "ICE": {
                    "GRASS": { "numerator": 2, "denominator": 1 },
                    "FLYING": { "numerator": 2, "denominator": 1 }
                },
                "FIRE": { "GRASS": { "numerator": 2, "denominator": 1 } }
            },
            "foresight_matchups": {
                "NORMAL": { "GHOST": { "numerator": 1, "denominator": 1 } },
                "FIGHTING": { "GHOST": { "numerator": 1, "denominator": 1 } }
            }
        }))
        .expect("type effectiveness fixture should parse")
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
                "FIGHT ING".to_string(),
            ],
            special: vec![
                "FIRE".to_string(),
                String::new(),
                "WATER TYPE".to_string(),
                "NORMAL".to_string(),
            ],
        };

        assert_eq!(
            type_category_issues(&categories, true),
            vec![
                TypeCategoryIssue::InvalidToken {
                    table: TypeCategoryTableKind::Physical,
                    type_id: " FIGHTING".to_string(),
                },
                TypeCategoryIssue::InvalidToken {
                    table: TypeCategoryTableKind::Physical,
                    type_id: "FIGHT ING".to_string(),
                },
                TypeCategoryIssue::InvalidToken {
                    table: TypeCategoryTableKind::Special,
                    type_id: String::new(),
                },
                TypeCategoryIssue::InvalidToken {
                    table: TypeCategoryTableKind::Special,
                    type_id: "WATER TYPE".to_string(),
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
    fn battle_damage_tokens_reject_reserved_pack_prefixes() {
        let categories = TypeCategories {
            physical: vec!["fallback_normal".to_string()],
            special: vec!["legacy_fire".to_string()],
        };

        assert_eq!(
            type_category_issues(&categories, true),
            vec![
                TypeCategoryIssue::InvalidToken {
                    table: TypeCategoryTableKind::Physical,
                    type_id: "fallback_normal".to_string(),
                },
                TypeCategoryIssue::InvalidToken {
                    table: TypeCategoryTableKind::Special,
                    type_id: "legacy_fire".to_string(),
                },
            ]
        );
    }

    #[test]
    fn type_effectiveness_table_issues_validate_definitive_rows() {
        let categories = TypeCategories {
            physical: vec!["NORMAL".to_string()],
            special: vec!["FIRE".to_string()],
        };
        let one = TypeMultiplier {
            numerator: 1,
            denominator: 1,
        };
        let zero_denominator = TypeMultiplier {
            numerator: 1,
            denominator: 0,
        };
        let table = TypeEffectivenessTable {
            matchups: BTreeMap::from([
                (
                    "NORMAL".to_string(),
                    BTreeMap::from([("NORMAL".to_string(), zero_denominator)]),
                ),
                (
                    "WATER".to_string(),
                    BTreeMap::from([("FIRE".to_string(), one)]),
                ),
                (
                    "WA TER".to_string(),
                    BTreeMap::from([("FIRE".to_string(), one)]),
                ),
                (
                    "FIRE".to_string(),
                    BTreeMap::from([("WA TER".to_string(), one), ("WATER".to_string(), one)]),
                ),
            ]),
            foresight_matchups: BTreeMap::from([
                (
                    "NORMAL".to_string(),
                    BTreeMap::from([
                        ("WATER".to_string(), zero_denominator),
                        ("WA TER".to_string(), one),
                    ]),
                ),
                (
                    "NO RMAL".to_string(),
                    BTreeMap::from([("NORMAL".to_string(), one)]),
                ),
            ]),
        };

        assert_eq!(
            type_effectiveness_table_issues(&table, &categories, true),
            vec![
                TypeEffectivenessTableIssue::InvalidDefender {
                    table: TypeEffectivenessTableKind::Matchups,
                    defender: pokemon_type("WA TER"),
                },
                TypeEffectivenessTableIssue::UnknownDefender {
                    table: TypeEffectivenessTableKind::Matchups,
                    defender: pokemon_type("WATER"),
                },
                TypeEffectivenessTableIssue::InvalidMultiplierDenominator {
                    table: TypeEffectivenessTableKind::Matchups,
                },
                TypeEffectivenessTableIssue::InvalidAttacker {
                    table: TypeEffectivenessTableKind::Matchups,
                    attacker: pokemon_type("WA TER"),
                },
                TypeEffectivenessTableIssue::UnknownAttacker {
                    table: TypeEffectivenessTableKind::Matchups,
                    attacker: pokemon_type("WATER"),
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
                TypeEffectivenessTableIssue::InvalidAttacker {
                    table: TypeEffectivenessTableKind::ForesightMatchups,
                    attacker: pokemon_type("NO RMAL"),
                },
                TypeEffectivenessTableIssue::InvalidDefender {
                    table: TypeEffectivenessTableKind::ForesightMatchups,
                    defender: pokemon_type("WA TER"),
                },
                TypeEffectivenessTableIssue::InvalidMultiplierDenominator {
                    table: TypeEffectivenessTableKind::ForesightMatchups,
                },
                TypeEffectivenessTableIssue::UnknownDefender {
                    table: TypeEffectivenessTableKind::ForesightMatchups,
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
                pokemon_type("NORMAL"),
                &[pokemon_type("GHOST")]
            )
            .expect("normal ghost immunity calculates"),
            TypeMultiplier::zero()
        );
        assert_eq!(
            calculate_type_effectiveness_multiplier_with_foresight(
                &type_effectiveness_table(),
                pokemon_type("NORMAL"),
                &[pokemon_type("GHOST")],
                true
            )
            .expect("foresight normal ghost override calculates"),
            TypeMultiplier::one()
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
    fn type_effectiveness_rejects_malformed_runtime_types() {
        assert_eq!(
            type_effectiveness(
                &type_effectiveness_table(),
                pokemon_type(" FIRE"),
                pokemon_type("GRASS")
            )
            .expect_err("malformed attacker must not become missing effectiveness"),
            DamageCalculationError::InvalidTypeEffectivenessAttacker {
                attacker: pokemon_type(" FIRE"),
            }
        );
        assert_eq!(
            type_effectiveness(
                &type_effectiveness_table(),
                pokemon_type("FIRE"),
                pokemon_type("GRA SS")
            )
            .expect_err("malformed defender must not become missing effectiveness"),
            DamageCalculationError::InvalidTypeEffectivenessDefender {
                defender: pokemon_type("GRA SS"),
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
    fn physical_type_rejects_malformed_runtime_type() {
        assert_eq!(
            is_physical_type(&type_categories(), pokemon_type("FI RE"))
                .expect_err("malformed type must not become missing category"),
            DamageCalculationError::InvalidTypeCategory {
                move_type: pokemon_type("FI RE"),
            }
        );
    }

    #[test]
    fn weather_modifier_rejects_malformed_runtime_type() {
        assert_eq!(
            apply_weather_type_modifier(
                40,
                Weather::Rain,
                pokemon_type("WA TER"),
                &weather_modifiers(),
            )
            .expect_err("malformed type must not become missing weather modifier"),
            DamageCalculationError::InvalidWeatherModifierType {
                move_type: pokemon_type("WA TER"),
            }
        );
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
    fn burn_halves_physical_attack_damage_for_exact_status_token() {
        let attacker = pokemon(
            "ATTACKER",
            pokemon_type("NORMAL"),
            BaseStats::new(80, 100, 78, 100, 80, 85),
            50,
        );
        let mut burned = attacker.clone();
        burned.status = Some("BURN".to_string());
        let mut lowercase_burn = attacker.clone();
        lowercase_burn.status = Some("burn".to_string());
        let defender = pokemon(
            "DEFENDER",
            pokemon_type("NORMAL"),
            BaseStats::new(80, 82, 83, 80, 100, 100),
            50,
        );
        let tackle = tackle(pokemon_type("NORMAL"), 60);
        let context = DamageContext::default();

        let normal = calculate_damage(
            &attacker,
            &defender,
            &tackle,
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            context,
        )
        .expect("normal physical damage");
        let burned = calculate_damage(
            &burned,
            &defender,
            &tackle,
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            context,
        )
        .expect("burned physical damage");
        let lowercase = calculate_damage(
            &lowercase_burn,
            &defender,
            &tackle,
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            context,
        )
        .expect("lowercase burn physical damage");

        assert!(burned.damage < normal.damage);
        assert_eq!(lowercase.damage, normal.damage);
    }

    #[test]
    fn burn_does_not_reduce_special_damage() {
        let attacker = pokemon(
            "ATTACKER",
            pokemon_type("FIRE"),
            BaseStats::new(80, 100, 78, 100, 100, 85),
            50,
        );
        let mut burned = attacker.clone();
        burned.status = Some("BURN".to_string());
        let defender = pokemon(
            "DEFENDER",
            pokemon_type("GRASS"),
            BaseStats::new(80, 82, 83, 80, 100, 100),
            50,
        );
        let ember = tackle(pokemon_type("FIRE"), 40);
        let context = DamageContext::default();

        let normal = calculate_damage(
            &attacker,
            &defender,
            &ember,
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            context,
        )
        .expect("normal special damage");
        let burned = calculate_damage(
            &burned,
            &defender,
            &ember,
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            context,
        )
        .expect("burned special damage");

        assert_eq!(burned.damage, normal.damage);
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
