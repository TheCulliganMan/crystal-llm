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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypeEffectivenessEntry {
    pub attacker: PokemonType,
    pub defender: PokemonType,
    pub multiplier: TypeMultiplier,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TypeEffectivenessTable {
    /// Rows before the source table's `-2` Foresight sentinel, in ROM order.
    pub matchups: Vec<TypeEffectivenessEntry>,
    /// Ghost-immunity rows after `-2`, skipped when the target is identified.
    pub foresight_matchups: Vec<TypeEffectivenessEntry>,
}

impl<'de> Deserialize<'de> for TypeEffectivenessTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawTypeEffectivenessTable {
            matchups: Vec<TypeEffectivenessEntry>,
            foresight_matchups: Vec<TypeEffectivenessEntry>,
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
    DuplicateMatchup {
        table: TypeEffectivenessTableKind,
        attacker: PokemonType,
        defender: PokemonType,
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
    let mut normal_pairs = BTreeSet::new();
    for entry in &table.matchups {
        push_type_effectiveness_entry_issues(
            TypeEffectivenessTableKind::Matchups,
            entry,
            &declared_types,
            &mut issues,
        );
        if !normal_pairs.insert((entry.attacker.as_str(), entry.defender.as_str())) {
            issues.push(TypeEffectivenessTableIssue::DuplicateMatchup {
                table: TypeEffectivenessTableKind::Matchups,
                attacker: entry.attacker.clone(),
                defender: entry.defender.clone(),
            });
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
    if declared_types.is_empty() {
        return;
    }
    if !is_exact_battle_damage_token(&entry.attacker) {
        issues.push(TypeEffectivenessTableIssue::InvalidAttacker {
            table,
            attacker: entry.attacker.clone(),
        });
    } else if !declared_types.contains(entry.attacker.as_str()) {
        issues.push(TypeEffectivenessTableIssue::UnknownAttacker {
            table,
            attacker: entry.attacker.clone(),
        });
    }
    if entry.multiplier.denominator == 0 {
        issues.push(TypeEffectivenessTableIssue::InvalidMultiplierDenominator { table });
    }
    if !is_exact_battle_damage_token(&entry.defender) {
        issues.push(TypeEffectivenessTableIssue::InvalidDefender {
            table,
            defender: entry.defender.clone(),
        });
    } else if !declared_types.contains(entry.defender.as_str()) {
        issues.push(TypeEffectivenessTableIssue::UnknownDefender {
            table,
            defender: entry.defender.clone(),
        });
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
    table: &[TypeEffectivenessEntry],
) -> Result<(), String> {
    let mut pairs = BTreeSet::new();
    for entry in table {
        validate_exact_damage_token("type effectiveness attacker", &entry.attacker)?;
        validate_exact_damage_token("type effectiveness defender", &entry.defender)?;
        validate_type_multiplier("type effectiveness matchup", entry.multiplier)?;
        if !pairs.insert((entry.attacker.as_str(), entry.defender.as_str())) {
            return Err(format!(
                "type effectiveness {table_name} duplicates {}/{}",
                entry.attacker, entry.defender
            ));
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
    /// Apply Crystal's post-stage/status 12.5% badge boost to the selected attacking stat.
    #[serde(default)]
    pub attacker_badge_boost: bool,
    /// Apply Crystal's post-stage 12.5% badge boost to the selected defending stat.
    #[serde(default)]
    pub defender_badge_boost: bool,
    /// Apply Crystal's 12.5% badge boost to damage after weather and before
    /// STAB. The ROM adds at least one damage and saturates at 16 bits.
    #[serde(default)]
    pub attacker_type_badge_boost: bool,
    /// The original battler is Ditto holding Metal Powder. This cannot be
    /// inferred from the effective transformed Pokemon used for damage stats.
    #[serde(default)]
    pub defender_metal_powder: bool,
    /// The original attacking species and held item activate Thick Club or
    /// Light Ball. Transform copies battle stats/species but not this check.
    #[serde(default)]
    pub attacker_species_item_boost: bool,
    /// Reflect or Light Screen is active for the selected damage category.
    /// Crystal wraps the 16-bit doubled Defense, then jointly truncates both
    /// selected damage stats into the command's byte registers.
    #[serde(default)]
    pub defender_screen: bool,
    /// `_BattleRandom` battles use `LINK_COLOSSEUM`; in that mode the ROM's
    /// `TruncateHL_BC` returns after only one paired shift and exposes the low
    /// bytes even when a stat remains wider than eight bits.
    #[serde(default)]
    pub link_colosseum: bool,
    /// Percentage added by a matching Gen II type-boosting held item. The ROM
    /// applies this to the post-division quotient before critical damage.
    #[serde(default)]
    pub held_type_boost_percent: u8,
    /// Multiplier applied to the capped damage command result before the STAB
    /// command. Triple Kick uses 1, 2, then 3 for its successive kicks.
    #[serde(default = "default_pre_stab_multiplier")]
    pub pre_stab_multiplier: u8,
    /// Multiplier applied after STAB and type effectiveness but before damage
    /// variation. Fury Cutter and Rollout mutate `wCurDamage` at this point.
    #[serde(default = "default_post_type_damage_multiplier")]
    pub post_type_damage_multiplier: u16,
    /// Crystal's separate Rage counter. Rage multiplies post-effectiveness
    /// damage by counter + 1 before damage variation.
    #[serde(default)]
    pub rage_counter: u8,
    /// Apply the Attack penalty from a cached burned battle stat even if the
    /// major status byte has since been cleared by the enemy item AI.
    #[serde(default)]
    pub attacker_burn_penalty: bool,
    /// `CalcPlayerStats` applies BadgeStatBoosts before its later Burn
    /// mutation; initial loading and the level-up path use the reverse order.
    #[serde(default)]
    pub attacker_badge_before_status: bool,
    /// The live attacking word from `wPlayerStats`/`wEnemyStats`. It already
    /// includes the current stat level, status penalty, and badge modification.
    #[serde(default)]
    pub attacker_loaded_stat: Option<u16>,
    /// The live defending word from `wPlayerStats`/`wEnemyStats`. Screens are
    /// applied later by `DamageStats` and are not part of this value.
    #[serde(default)]
    pub defender_loaded_stat: Option<u16>,
}

impl Default for DamageContext {
    fn default() -> Self {
        Self {
            is_critical: false,
            is_confusion_damage: false,
            defender_identified: false,
            weather: Weather::None,
            random_roll: 255,
            attacker_badge_boost: false,
            defender_badge_boost: false,
            attacker_type_badge_boost: false,
            defender_metal_powder: false,
            attacker_species_item_boost: false,
            defender_screen: false,
            link_colosseum: false,
            held_type_boost_percent: 0,
            pre_stab_multiplier: default_pre_stab_multiplier(),
            post_type_damage_multiplier: default_post_type_damage_multiplier(),
            rage_counter: 0,
            attacker_burn_penalty: false,
            attacker_badge_before_status: false,
            attacker_loaded_stat: None,
            defender_loaded_stat: None,
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
    type_effectiveness_from_rows(
        table.matchups.iter().chain(table.foresight_matchups.iter()),
        !table.matchups.is_empty() || !table.foresight_matchups.is_empty(),
        move_type,
        defender_type,
    )
}

pub fn foresight_type_effectiveness(
    table: &TypeEffectivenessTable,
    move_type: impl AsRef<str>,
    defender_type: impl AsRef<str>,
) -> Result<TypeMultiplier, DamageCalculationError> {
    let move_type = move_type.as_ref();
    let defender_type = defender_type.as_ref();
    type_effectiveness_from_rows(
        table.matchups.iter(),
        !table.matchups.is_empty(),
        move_type,
        defender_type,
    )
}

fn type_effectiveness_from_rows<'a>(
    mut rows: impl Iterator<Item = &'a TypeEffectivenessEntry>,
    has_rows: bool,
    move_type: impl AsRef<str>,
    defender_type: impl AsRef<str>,
) -> Result<TypeMultiplier, DamageCalculationError> {
    if !has_rows {
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
    Ok(rows
        .find(|entry| entry.attacker == move_type && entry.defender == defender_type)
        .map_or_else(TypeMultiplier::one, |entry| entry.multiplier))
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

fn apply_type_effectiveness_rows(
    table: &TypeEffectivenessTable,
    move_type: &str,
    defender_types: &[PokemonType],
    defender_identified: bool,
    mut damage: u16,
) -> Result<u16, DamageCalculationError> {
    if table.matchups.is_empty() {
        return Err(DamageCalculationError::MissingTypeEffectivenessTable);
    }
    validate_exact_damage_token("type effectiveness attacker", move_type)
        .map_err(|_| invalid_type_effectiveness_attacker(move_type))?;
    for defender_type in defender_types {
        validate_exact_damage_token("type effectiveness defender", defender_type)
            .map_err(|_| invalid_type_effectiveness_defender(defender_type))?;
    }
    let rows = table.matchups.iter().chain(
        (!defender_identified)
            .then_some(table.foresight_matchups.iter())
            .into_iter()
            .flatten(),
    );
    for entry in rows {
        if entry.attacker != move_type || !defender_types.contains(&entry.defender) {
            continue;
        }
        if entry.multiplier.numerator == 0 {
            return Ok(0);
        }
        damage = entry.multiplier.apply_floor(damage).max(1);
    }
    Ok(damage)
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

    let base_attack = match attack_stat {
        Stat::Attack => attacker.attack,
        Stat::SpecialAttack => attacker.special_attack,
        _ => unreachable!("damage selected a non-attacking stat"),
    };
    let base_defense = match defense_stat {
        Stat::Defense => defender.defense,
        Stat::SpecialDefense => defender.special_defense,
        _ => unreachable!("damage selected a non-defending stat"),
    };
    // CheckDamageStatsCritical returns carry (keep boosted live stats and the
    // already-applied screen) only when the defender's stage is strictly
    // lower than the attacker's. Equality therefore reloads both raw party
    // stats, discarding stat levels, status penalties, badge boosts, and the
    // already-applied Reflect/Light Screen just like a higher Defense stage.
    let critical_ignores_stages = context.is_critical && defense_stage >= attack_stage;
    let mut attack_value = if critical_ignores_stages {
        clamp_stat(base_attack)
    } else if let Some(loaded) = context.attacker_loaded_stat {
        clamp_stat(loaded)
    } else {
        clamp_stat(
            apply_stage(stat_multipliers, base_attack, attack_stage).ok_or(
                DamageCalculationError::MissingStatMultiplier {
                    stage: attack_stage,
                },
            )?,
        )
    };
    if !critical_ignores_stages && context.attacker_loaded_stat.is_none() {
        if context.attacker_badge_boost && context.attacker_badge_before_status {
            attack_value = attack_value.saturating_add(attack_value / 8).min(999);
        }
        attack_value = apply_burn_attack_penalty(
            attacker,
            physical,
            attack_value,
            context.attacker_burn_penalty,
        );
        if context.attacker_badge_boost && !context.attacker_badge_before_status {
            attack_value = attack_value.saturating_add(attack_value / 8).min(999);
        }
    }
    // HitSelfInConfusion loads the current battle Attack directly and skips
    // DamageStats, so Thick Club/Light Ball stat boosts are not applied. The
    // later DamageCalc type-item lookup still uses the selected move type.
    let attack_value = if context.is_confusion_damage || !context.attacker_species_item_boost {
        attack_value
    } else {
        attack_value.wrapping_mul(2)
    };
    let mut defense_value = if critical_ignores_stages {
        clamp_stat(base_defense)
    } else if let Some(loaded) = context.defender_loaded_stat {
        clamp_stat(loaded)
    } else {
        clamp_stat(
            apply_stage(stat_multipliers, base_defense, defense_stage).ok_or(
                DamageCalculationError::MissingStatMultiplier {
                    stage: defense_stage,
                },
            )?,
        )
    };
    if context.defender_badge_boost
        && !critical_ignores_stages
        && context.defender_loaded_stat.is_none()
    {
        defense_value = defense_value.saturating_add(defense_value / 8).min(999);
    }
    if context.defender_screen && !critical_ignores_stages {
        defense_value = defense_value.wrapping_mul(2);
    }
    let (mut attack_value, mut defense_value) =
        truncate_damage_stats(attack_value, defense_value, context.link_colosseum);
    if context.defender_metal_powder
        || (defender.species.id == "DITTO" && defender.item.as_deref() == Some("METAL_POWDER"))
    {
        (attack_value, defense_value) =
            apply_metal_powder_damage_stats(attack_value, defense_value);
    }
    if move_data.effect == "SELFDESTRUCT" {
        defense_value = (defense_value / 2).max(1);
    }

    let level_factor = ((2 * attacker.level as u16) / 5) + 2;
    let mut base_damage = ((level_factor as u32 * move_data.power as u32 * attack_value as u32)
        / defense_value as u32)
        / 50;
    base_damage = apply_held_type_boost_to_quotient(base_damage, context.held_type_boost_percent);
    if context.is_critical {
        base_damage = base_damage.saturating_mul(2);
    }
    // DamageCalc caps the full-width quotient at 997 before adding the
    // minimum two damage. Narrowing first can wrap a large quotient below the
    // cap and produce tiny damage for otherwise maximal attacks.
    let mut damage = base_damage.min(997) as u16 + 2;
    damage = damage.saturating_mul(u16::from(context.pre_stab_multiplier.max(1)));

    damage = apply_weather_type_modifier(
        damage,
        context.weather,
        &move_data.move_type,
        weather_modifiers,
    )?;

    if context.attacker_type_badge_boost {
        damage = damage.saturating_add((damage / 8).max(1));
    }

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
    damage = apply_type_effectiveness_rows(
        type_effectiveness,
        &move_data.move_type,
        &defender_types,
        context.defender_identified,
        damage,
    )?;

    damage = damage
        .checked_mul(context.post_type_damage_multiplier.max(1))
        .unwrap_or(u16::MAX);

    if move_data.effect == "RAGE" && context.rage_counter != 0 {
        damage = u16::try_from(
            u32::from(damage)
                .saturating_mul(u32::from(context.rage_counter) + 1)
                .min(u32::from(u16::MAX)),
        )
        .expect("Rage damage was capped to u16");
    }

    let roll = context.random_roll.max(1);
    damage = ((damage as u32 * roll as u32) / 255).max(1) as u16;

    Ok(DamageResult {
        damage,
        type_multiplier,
    })
}

const fn default_pre_stab_multiplier() -> u8 {
    1
}

const fn default_post_type_damage_multiplier() -> u16 {
    1
}

pub(crate) fn truncate_damage_stats(
    mut attack: u16,
    mut defense: u16,
    link_colosseum: bool,
) -> (u16, u16) {
    while attack > u16::from(u8::MAX) || defense > u16::from(u8::MAX) {
        attack = (attack >> 2).max(1);
        defense = (defense >> 2).max(1);
        if link_colosseum {
            // The link-only early return leaves the command's B/C byte
            // registers authoritative. DamageCalc subsequently promotes a
            // zero Defense byte to one, while a zero Attack byte stays zero.
            return (attack & 0xff, (defense & 0xff).max(1));
        }
    }
    (attack, defense)
}

pub(crate) fn apply_metal_powder_damage_stats(attack: u16, defense: u16) -> (u16, u16) {
    let defense = defense.min(u16::from(u8::MAX));
    let sum = defense + defense / 2;
    if sum <= u16::from(u8::MAX) {
        return (attack, sum.max(1));
    }
    let attack = (attack >> 1).max(1);
    let wrapped = (sum & u16::from(u8::MAX)) as u8;
    let defense = u16::from((wrapped >> 1) | 0x80);
    (attack, defense.max(1))
}

pub(crate) fn apply_held_type_boost_to_quotient(quotient: u32, parameter: u8) -> u32 {
    if parameter == 0 {
        return quotient;
    }
    let multiplier = u32::from(parameter.wrapping_add(100));
    quotient.saturating_mul(multiplier) / 100
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
    let Some(modifiers) = weather_modifiers.type_modifiers.get(weather_id) else {
        return Err(DamageCalculationError::MissingWeatherModifier {
            weather,
            move_type: move_type.to_string(),
        });
    };
    // BattleCommand_DamageStats only branches for FIRE and WATER under rain
    // or sun. Every other valid type retains the incoming damage unchanged;
    // the sparse exported table represents those exceptional branches, not
    // an exhaustive matrix requiring a synthetic ×1 entry for every type.
    Ok(modifiers
        .get(move_type)
        .map_or(damage, |entry| entry.apply_floor(damage)))
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

fn apply_burn_attack_penalty(
    attacker: &Pokemon,
    physical: bool,
    attack_value: u16,
    cached_penalty: bool,
) -> u16 {
    if physical && (attacker.status.as_deref() == Some("BURN") || cached_penalty) {
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
            "matchups": [
                { "attacker": "GHOST", "defender": "STEEL", "multiplier": { "numerator": 1, "denominator": 2 } },
                { "attacker": "DARK", "defender": "STEEL", "multiplier": { "numerator": 1, "denominator": 2 } },
                { "attacker": "ELECTRIC", "defender": "GROUND", "multiplier": { "numerator": 0, "denominator": 1 } },
                { "attacker": "ICE", "defender": "GRASS", "multiplier": { "numerator": 2, "denominator": 1 } },
                { "attacker": "ICE", "defender": "FLYING", "multiplier": { "numerator": 2, "denominator": 1 } },
                { "attacker": "FIRE", "defender": "GRASS", "multiplier": { "numerator": 2, "denominator": 1 } }
            ],
            "foresight_matchups": [
                { "attacker": "NORMAL", "defender": "GHOST", "multiplier": { "numerator": 0, "denominator": 1 } },
                { "attacker": "FIGHTING", "defender": "GHOST", "multiplier": { "numerator": 0, "denominator": 1 } }
            ]
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
            matchups: vec![
                TypeEffectivenessEntry {
                    attacker: "FIRE".to_string(),
                    defender: "WA TER".to_string(),
                    multiplier: one,
                },
                TypeEffectivenessEntry {
                    attacker: "FIRE".to_string(),
                    defender: "WATER".to_string(),
                    multiplier: one,
                },
                TypeEffectivenessEntry {
                    attacker: "NORMAL".to_string(),
                    defender: "NORMAL".to_string(),
                    multiplier: zero_denominator,
                },
                TypeEffectivenessEntry {
                    attacker: "WA TER".to_string(),
                    defender: "FIRE".to_string(),
                    multiplier: one,
                },
                TypeEffectivenessEntry {
                    attacker: "WATER".to_string(),
                    defender: "FIRE".to_string(),
                    multiplier: one,
                },
            ],
            foresight_matchups: vec![
                TypeEffectivenessEntry {
                    attacker: "NO RMAL".to_string(),
                    defender: "NORMAL".to_string(),
                    multiplier: one,
                },
                TypeEffectivenessEntry {
                    attacker: "NORMAL".to_string(),
                    defender: "WA TER".to_string(),
                    multiplier: one,
                },
                TypeEffectivenessEntry {
                    attacker: "NORMAL".to_string(),
                    defender: "WATER".to_string(),
                    multiplier: zero_denominator,
                },
            ],
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
            source_index: 1,
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
    fn dual_type_effectiveness_floors_after_each_matching_asm_row() {
        let mut attacker = pokemon(
            "ATTACKER",
            pokemon_type("NORMAL"),
            BaseStats::new(80, 100, 78, 100, 80, 85),
            50,
        );
        attacker.attack = 1;
        let mut defender = pokemon(
            "DEFENDER",
            pokemon_type("GRASS"),
            BaseStats::new(80, 82, 100, 80, 100, 100),
            50,
        );
        defender.species.type2 = pokemon_type("POISON");
        defender.defense = 1;
        let mut table = type_effectiveness_table();
        table.matchups.extend([
            TypeEffectivenessEntry {
                attacker: "GROUND".to_string(),
                defender: "GRASS".to_string(),
                multiplier: TypeMultiplier {
                    numerator: 1,
                    denominator: 2,
                },
            },
            TypeEffectivenessEntry {
                attacker: "GROUND".to_string(),
                defender: "POISON".to_string(),
                multiplier: TypeMultiplier {
                    numerator: 2,
                    denominator: 1,
                },
            },
        ]);

        let result = calculate_damage(
            &attacker,
            &defender,
            &tackle(pokemon_type("GROUND"), 3),
            &stat_multipliers(),
            &type_categories(),
            &table,
            &weather_modifiers(),
            DamageContext::default(),
        )
        .expect("dual-type damage");

        // The Ground rows encounter Grass before Poison: 3 / 2 = 1, then 1 * 2 = 2.
        assert_eq!(result.damage, 2);
        assert_eq!(result.type_multiplier, TypeMultiplier::one());
    }

    #[test]
    fn absent_type_effectiveness_rows_are_neutral() {
        let multiplier = calculate_type_effectiveness_multiplier(
            &type_effectiveness_table(),
            pokemon_type("NORMAL"),
            &[pokemon_type("GRASS")],
        )
        .expect("sparse source table treats an absent row as neutral");

        assert_eq!(multiplier, TypeMultiplier::one());
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
    fn weather_modifier_keeps_unlisted_valid_types_neutral() {
        assert_eq!(
            apply_weather_type_modifier(
                40,
                Weather::Sun,
                pokemon_type("NORMAL"),
                &weather_modifiers(),
            )
            .expect("ordinary types are neutral under sun"),
            40
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
    fn abilities_do_not_change_damage() {
        let mut attacker = pokemon(
            "ATTACKER",
            pokemon_type("WATER"),
            BaseStats::new(80, 84, 78, 100, 109, 85),
            50,
        );
        let defender = pokemon(
            "DEFENDER",
            pokemon_type("NORMAL"),
            BaseStats::new(80, 82, 83, 80, 100, 100),
            50,
        );
        let water_move = tackle(pokemon_type("WATER"), 60);
        let normal = calculate_damage(
            &attacker,
            &defender,
            &water_move,
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            DamageContext::default(),
        )
        .expect("ordinary damage");

        attacker.species.ability = "TORRENT".to_string();
        attacker.hp = attacker.max_hp / 3;
        let torrent = calculate_damage(
            &attacker,
            &defender,
            &water_move,
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            DamageContext::default(),
        )
        .expect("Torrent damage");
        attacker.hp = attacker.max_hp / 3 + 1;
        let above_threshold = calculate_damage(
            &attacker,
            &defender,
            &water_move,
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            DamageContext::default(),
        )
        .expect("above-threshold damage");

        assert_eq!(torrent.damage, normal.damage);
        assert_eq!(above_threshold.damage, normal.damage);

        for ability in ["PURE_POWER", "HUSTLE"] {
            let mut ability_attacker = attacker.clone();
            ability_attacker.species.ability = ability.to_string();
            assert_eq!(
                calculate_damage(
                    &ability_attacker,
                    &defender,
                    &tackle(pokemon_type("NORMAL"), 60),
                    &stat_multipliers(),
                    &type_categories(),
                    &type_effectiveness_table(),
                    &weather_modifiers(),
                    DamageContext::default(),
                )
                .expect("offensive ability damage"),
                calculate_damage(
                    &attacker,
                    &defender,
                    &tackle(pokemon_type("NORMAL"), 60),
                    &stat_multipliers(),
                    &type_categories(),
                    &type_effectiveness_table(),
                    &weather_modifiers(),
                    DamageContext::default(),
                )
                .expect("neutral physical damage"),
                "{ability} changed physical damage"
            );
        }

        for (ability, move_type) in [
            ("THICK_FAT", "FIRE"),
            ("LEVITATE", "GROUND"),
            ("WONDER_GUARD", "NORMAL"),
            ("WATER_ABSORB", "WATER"),
            ("VOLT_ABSORB", "ELECTRIC"),
            ("FLASH_FIRE", "FIRE"),
        ] {
            let mut ability_defender = defender.clone();
            ability_defender.species.ability = ability.to_string();
            let move_data = tackle(pokemon_type(move_type), 60);
            assert_eq!(
                calculate_damage(
                    &attacker,
                    &ability_defender,
                    &move_data,
                    &stat_multipliers(),
                    &type_categories(),
                    &type_effectiveness_table(),
                    &weather_modifiers(),
                    DamageContext::default(),
                )
                .expect("defensive ability damage"),
                calculate_damage(
                    &attacker,
                    &defender,
                    &move_data,
                    &stat_multipliers(),
                    &type_categories(),
                    &type_effectiveness_table(),
                    &weather_modifiers(),
                    DamageContext::default(),
                )
                .expect("neutral comparison damage"),
                "{ability} changed damage"
            );
        }

        let mut air_lock_attacker = attacker.clone();
        air_lock_attacker.species.ability = "AIR_LOCK".to_string();
        let rain = DamageContext {
            weather: Weather::Rain,
            ..DamageContext::default()
        };
        assert_eq!(
            calculate_damage(
                &air_lock_attacker,
                &defender,
                &water_move,
                &stat_multipliers(),
                &type_categories(),
                &type_effectiveness_table(),
                &weather_modifiers(),
                rain,
            )
            .expect("Air Lock weather damage"),
            calculate_damage(
                &attacker,
                &defender,
                &water_move,
                &stat_multipliers(),
                &type_categories(),
                &type_effectiveness_table(),
                &weather_modifiers(),
                rain,
            )
            .expect("neutral weather damage")
        );
    }

    #[test]
    fn damage_uses_copied_battle_stats_instead_of_recalculating_species_stats() {
        let mut attacker = pokemon(
            "DITTO",
            pokemon_type("NORMAL"),
            BaseStats::new(48, 48, 48, 48, 48, 48),
            50,
        );
        attacker.attack = 200;
        let defender = pokemon(
            "DEFENDER",
            pokemon_type("NORMAL"),
            BaseStats::new(80, 82, 83, 80, 100, 100),
            50,
        );
        let result = calculate_damage(
            &attacker,
            &defender,
            &tackle(pokemon_type("NORMAL"), 60),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            DamageContext::default(),
        )
        .expect("copied battle Attack is used");

        assert!(result.damage > 40);
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
        burned.species.ability = "GUTS".to_string();
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
    fn screens_apply_inside_critical_hit_damage() {
        let mut attacker = pokemon(
            "ATTACKER",
            pokemon_type("NORMAL"),
            BaseStats::new(80, 100, 78, 100, 80, 85),
            50,
        );
        attacker.stat_boosts.insert(Stat::Attack, 1);
        let defender = pokemon(
            "DEFENDER",
            pokemon_type("NORMAL"),
            BaseStats::new(80, 82, 100, 80, 100, 100),
            50,
        );
        let move_data = tackle(pokemon_type("NORMAL"), 60);
        let critical = DamageContext {
            is_critical: true,
            ..DamageContext::default()
        };
        let screened = DamageContext {
            defender_screen: true,
            ..critical
        };

        let plain = calculate_damage(
            &attacker,
            &defender,
            &move_data,
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            critical,
        )
        .expect("critical damage without screen");
        let screened = calculate_damage(
            &attacker,
            &defender,
            &move_data,
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            screened,
        )
        .expect("critical damage with screen");

        assert!(screened.damage < plain.damage);
    }

    #[test]
    fn equal_stage_critical_ignores_the_defenders_screen() {
        let attacker = pokemon(
            "ATTACKER",
            pokemon_type("NORMAL"),
            BaseStats::new(80, 100, 78, 100, 80, 85),
            50,
        );
        let defender = pokemon(
            "DEFENDER",
            pokemon_type("NORMAL"),
            BaseStats::new(80, 82, 100, 80, 100, 100),
            50,
        );
        let move_data = tackle(pokemon_type("NORMAL"), 60);
        let plain = calculate_damage(
            &attacker,
            &defender,
            &move_data,
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            DamageContext {
                is_critical: true,
                ..DamageContext::default()
            },
        )
        .expect("equal-stage critical without screen");
        let screened = calculate_damage(
            &attacker,
            &defender,
            &move_data,
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            DamageContext {
                is_critical: true,
                defender_screen: true,
                ..DamageContext::default()
            },
        )
        .expect("equal-stage critical with screen");

        assert_eq!(screened.damage, plain.damage);
    }

    #[test]
    fn equal_stage_critical_bypass_discards_badge_boosts_and_burn_penalty() {
        let attacker = pokemon(
            "ATTACKER",
            pokemon_type("NORMAL"),
            BaseStats::new(80, 100, 78, 100, 80, 85),
            50,
        );
        let mut burned_attacker = attacker.clone();
        burned_attacker.status = Some("BURN".to_string());
        let defender = pokemon(
            "DEFENDER",
            pokemon_type("NORMAL"),
            BaseStats::new(80, 82, 100, 80, 100, 100),
            50,
        );
        let move_data = tackle(pokemon_type("NORMAL"), 60);
        let calculate = |attacker: &Pokemon, context: DamageContext| {
            calculate_damage(
                attacker,
                &defender,
                &move_data,
                &stat_multipliers(),
                &type_categories(),
                &type_effectiveness_table(),
                &weather_modifiers(),
                context,
            )
            .expect("equal-stage critical damage")
            .damage
        };
        let critical = DamageContext {
            is_critical: true,
            ..DamageContext::default()
        };
        let plain = calculate(&attacker, critical);

        assert_eq!(
            calculate(
                &attacker,
                DamageContext {
                    attacker_badge_boost: true,
                    ..critical
                },
            ),
            plain,
        );
        assert_eq!(
            calculate(
                &attacker,
                DamageContext {
                    defender_badge_boost: true,
                    ..critical
                },
            ),
            plain,
        );
        assert_eq!(
            calculate(
                &burned_attacker,
                DamageContext {
                    attacker_burn_penalty: true,
                    ..critical
                },
            ),
            plain,
        );
    }

    #[test]
    fn badge_stat_boost_applies_after_the_stat_level_multiplier() {
        let mut attacker = pokemon(
            "ATTACKER",
            pokemon_type("NORMAL"),
            BaseStats::new(80, 100, 78, 100, 80, 85),
            50,
        );
        attacker.attack = 100;
        attacker.stat_boosts.insert(Stat::Attack, -4);
        let mut defender = pokemon(
            "DEFENDER",
            pokemon_type("NORMAL"),
            BaseStats::new(80, 82, 100, 80, 100, 100),
            50,
        );
        defender.defense = 10;

        let result = calculate_damage(
            &attacker,
            &defender,
            &tackle(pokemon_type("NORMAL"), 250),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            DamageContext {
                attacker_badge_boost: true,
                ..DamageContext::default()
            },
        )
        .expect("badge-boosted staged damage");

        // 100 at -4 is 33, then BadgeStatBoosts adds floor(33 / 8) = 4.
        assert_eq!(result.damage, 613);
    }

    #[test]
    fn critical_stage_bypass_reloads_defense_after_the_screen_step() {
        let attacker = pokemon(
            "ATTACKER",
            pokemon_type("NORMAL"),
            BaseStats::new(80, 100, 78, 100, 80, 85),
            50,
        );
        let mut defender = pokemon(
            "DEFENDER",
            pokemon_type("NORMAL"),
            BaseStats::new(80, 82, 100, 80, 100, 100),
            50,
        );
        defender.stat_boosts.insert(Stat::Defense, 1);
        let move_data = tackle(pokemon_type("NORMAL"), 60);
        let plain = calculate_damage(
            &attacker,
            &defender,
            &move_data,
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            DamageContext {
                is_critical: true,
                ..DamageContext::default()
            },
        )
        .expect("stage-bypassing critical without screen");
        let screened = calculate_damage(
            &attacker,
            &defender,
            &move_data,
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            DamageContext {
                is_critical: true,
                defender_screen: true,
                ..DamageContext::default()
            },
        )
        .expect("stage-bypassing critical with screen");

        assert_eq!(screened.damage, plain.damage);
    }

    #[test]
    fn non_link_screen_defense_repeats_gen_two_paired_stat_truncation() {
        let attacker = pokemon(
            "ATTACKER",
            pokemon_type("NORMAL"),
            BaseStats::new(80, 100, 78, 100, 80, 85),
            50,
        );
        let mut defender = pokemon(
            "DEFENDER",
            pokemon_type("NORMAL"),
            BaseStats::new(80, 82, 100, 80, 100, 100),
            50,
        );
        defender.defense = 512;
        let move_data = tackle(pokemon_type("NORMAL"), 60);

        let plain = calculate_damage(
            &attacker,
            &defender,
            &move_data,
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            DamageContext::default(),
        )
        .expect("damage without wrapped screen");
        let screened = calculate_damage(
            &attacker,
            &defender,
            &move_data,
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            DamageContext {
                defender_screen: true,
                ..DamageContext::default()
            },
        )
        .expect("damage with wrapped screen");

        assert!(screened.damage < plain.damage);
    }

    #[test]
    fn link_colosseum_screen_exposes_the_single_shift_defense_wrap_bug() {
        let attacker = pokemon(
            "ATTACKER",
            pokemon_type("NORMAL"),
            BaseStats::new(80, 100, 78, 100, 80, 85),
            50,
        );
        let mut defender = pokemon(
            "DEFENDER",
            pokemon_type("NORMAL"),
            BaseStats::new(80, 82, 100, 80, 100, 100),
            50,
        );
        defender.defense = 512;
        let move_data = tackle(pokemon_type("NORMAL"), 60);

        let ordinary = calculate_damage(
            &attacker,
            &defender,
            &move_data,
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            DamageContext {
                defender_screen: true,
                ..DamageContext::default()
            },
        )
        .expect("ordinary paired truncation");
        let link = calculate_damage(
            &attacker,
            &defender,
            &move_data,
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            DamageContext {
                defender_screen: true,
                link_colosseum: true,
                ..DamageContext::default()
            },
        )
        .expect("link single-shift truncation");

        assert!(link.damage > ordinary.damage);
    }

    #[test]
    fn held_type_parameter_boosts_the_quotient_before_minimum_damage() {
        let attacker = pokemon(
            "ATTACKER",
            pokemon_type("FIRE"),
            BaseStats::new(80, 100, 78, 100, 80, 85),
            50,
        );
        let defender = pokemon(
            "DEFENDER",
            pokemon_type("NORMAL"),
            BaseStats::new(80, 82, 100, 80, 100, 100),
            50,
        );
        let move_data = tackle(pokemon_type("NORMAL"), 80);
        let plain = calculate_damage(
            &attacker,
            &defender,
            &move_data,
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            DamageContext::default(),
        )
        .expect("plain damage");
        let boosted = calculate_damage(
            &attacker,
            &defender,
            &move_data,
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            DamageContext {
                held_type_boost_percent: 10,
                ..DamageContext::default()
            },
        )
        .expect("type-item damage");

        assert_eq!(
            boosted.damage,
            ((u32::from(plain.damage - 2) * 110) / 100) as u16 + 2
        );
    }

    #[test]
    fn held_type_parameter_addition_wraps_in_the_multiplier_byte() {
        let attacker = pokemon(
            "ATTACKER",
            pokemon_type("FIRE"),
            BaseStats::new(80, 100, 78, 100, 80, 85),
            50,
        );
        let defender = pokemon(
            "DEFENDER",
            pokemon_type("NORMAL"),
            BaseStats::new(80, 82, 100, 80, 100, 100),
            50,
        );
        let move_data = tackle(pokemon_type("NORMAL"), 80);
        let plain = calculate_damage(
            &attacker,
            &defender,
            &move_data,
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            DamageContext::default(),
        )
        .expect("plain damage");
        let wrapped = calculate_damage(
            &attacker,
            &defender,
            &move_data,
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            DamageContext {
                held_type_boost_percent: 200,
                ..DamageContext::default()
            },
        )
        .expect("wrapped type-item damage");

        // DamageCalc executes `add 100` in A: 200 + 100 wraps to 44.
        assert_eq!(
            wrapped.damage,
            ((u32::from(plain.damage - 2) * 44) / 100) as u16 + 2
        );
    }

    #[test]
    fn full_width_base_damage_is_capped_before_narrowing() {
        let mut attacker = pokemon(
            "ATTACKER",
            pokemon_type("NORMAL"),
            BaseStats::new(80, 100, 78, 100, 80, 85),
            100,
        );
        attacker.level = 255;
        attacker.attack = 999;
        let mut defender = pokemon(
            "DEFENDER",
            pokemon_type("NORMAL"),
            BaseStats::new(80, 82, 100, 80, 100, 100),
            50,
        );
        defender.defense = 1;

        let result = calculate_damage(
            &attacker,
            &defender,
            &tackle(pokemon_type("NORMAL"), 255),
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            DamageContext {
                is_critical: true,
                ..DamageContext::default()
            },
        )
        .expect("maximal damage calculation");

        assert_eq!(result.damage, 1498);
    }

    #[test]
    fn rage_counter_multiplies_damage_after_type_effectiveness() {
        let attacker = pokemon(
            "ATTACKER",
            pokemon_type("NORMAL"),
            BaseStats::new(80, 100, 78, 100, 80, 85),
            50,
        );
        let defender = pokemon(
            "DEFENDER",
            pokemon_type("NORMAL"),
            BaseStats::new(80, 82, 100, 80, 100, 100),
            50,
        );
        let mut rage = tackle(pokemon_type("NORMAL"), 20);
        rage.name = "RAGE".to_string();
        rage.effect = "RAGE".to_string();

        let initial = calculate_damage(
            &attacker,
            &defender,
            &rage,
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            DamageContext::default(),
        )
        .expect("initial Rage damage");
        let built = calculate_damage(
            &attacker,
            &defender,
            &rage,
            &stat_multipliers(),
            &type_categories(),
            &type_effectiveness_table(),
            &weather_modifiers(),
            DamageContext {
                rage_counter: 2,
                ..DamageContext::default()
            },
        )
        .expect("built Rage damage");

        assert_eq!(built.damage, initial.damage * 3);
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
