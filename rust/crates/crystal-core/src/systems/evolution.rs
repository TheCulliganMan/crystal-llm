use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::models::pokemon::StatExperience;
use crate::models::{LearnedMove, Move, Pokemon, PokemonSpecies, calculate_stats};
use crate::systems::learnsets::{LearnsetEntry, SpeciesLearnsets, level_up_moves_for_species};
use crate::world::encounters::TimeOfDay;

pub const HAPPINESS_TO_EVOLVE: u8 = 220;
pub const TRADE_ANY_ITEM: &str = "-1";
pub const EVERSTONE_ITEM_ID: &str = "EVERSTONE";

pub const METHOD_LEVEL: &str = "LEVEL";
pub const METHOD_ITEM: &str = "ITEM";
pub const METHOD_HAPPINESS: &str = "HAPPINESS";
pub const METHOD_TRADE: &str = "TRADE";
pub const METHOD_STAT: &str = "STAT";
pub const EVOLUTION_METHODS: &[&str] = &[
    METHOD_LEVEL,
    METHOD_ITEM,
    METHOD_HAPPINESS,
    METHOD_TRADE,
    METHOD_STAT,
];

pub const HAPPINESS_ANYTIME: &str = "TR_ANYTIME";
pub const HAPPINESS_MORNDAY: &str = "TR_MORNDAY";
pub const HAPPINESS_NITE: &str = "TR_NITE";
pub const HAPPINESS_WINDOWS: &[&str] = &[HAPPINESS_ANYTIME, HAPPINESS_MORNDAY, HAPPINESS_NITE];

pub const STAT_ATK_GT_DEF: &str = "ATK_GT_DEF";
pub const STAT_ATK_LT_DEF: &str = "ATK_LT_DEF";
pub const STAT_ATK_EQ_DEF: &str = "ATK_EQ_DEF";
pub const STAT_EVOLUTION_RATIOS: &[&str] = &[STAT_ATK_GT_DEF, STAT_ATK_LT_DEF, STAT_ATK_EQ_DEF];

pub fn is_known_evolution_method(method: &str) -> bool {
    EVOLUTION_METHODS.contains(&method)
}

pub fn is_known_happiness_window(window: &str) -> bool {
    HAPPINESS_WINDOWS.contains(&window)
}

pub fn is_known_stat_evolution_ratio(ratio: &str) -> bool {
    STAT_EVOLUTION_RATIOS.contains(&ratio)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvolutionTableIssue {
    MissingSpeciesEvolutions {
        species_id: String,
    },
    InvalidSourceSpecies {
        species_id: String,
    },
    UnknownSourceSpecies {
        species_id: String,
    },
    InvalidTargetSpecies {
        source_species_id: String,
        target_species_id: String,
    },
    UnknownTargetSpecies {
        source_species_id: String,
        target_species_id: String,
    },
    MissingLevel {
        source_species_id: String,
    },
    MissingItem {
        source_species_id: String,
    },
    InvalidItem {
        source_species_id: String,
        item_id: String,
    },
    UnknownItem {
        source_species_id: String,
        item_id: String,
    },
    MissingHappinessWindow {
        source_species_id: String,
    },
    InvalidHappinessWindow {
        source_species_id: String,
        window: String,
    },
    UnknownHappinessWindow {
        source_species_id: String,
        window: String,
    },
    InvalidTradeItem {
        source_species_id: String,
        item_id: String,
    },
    UnknownTradeItem {
        source_species_id: String,
        item_id: String,
    },
    MissingStatLevel {
        source_species_id: String,
    },
    MissingStatRatio {
        source_species_id: String,
    },
    InvalidStatRatio {
        source_species_id: String,
        ratio: String,
    },
    UnknownStatRatio {
        source_species_id: String,
        ratio: String,
    },
    InvalidMethod {
        source_species_id: String,
        method: String,
    },
    UnknownMethod {
        source_species_id: String,
        method: String,
    },
}

pub fn evolution_table_issues(
    table: &EvolutionTable,
    species_ids: &BTreeSet<String>,
    item_ids: &BTreeSet<String>,
) -> Vec<EvolutionTableIssue> {
    let mut issues = Vec::new();
    for species_id in species_ids {
        if !table.0.contains_key(species_id) {
            issues.push(EvolutionTableIssue::MissingSpeciesEvolutions {
                species_id: species_id.clone(),
            });
        }
    }
    for (source_species_id, entries) in &table.0 {
        if !is_exact_nonempty_evolution_token(source_species_id) {
            issues.push(EvolutionTableIssue::InvalidSourceSpecies {
                species_id: source_species_id.clone(),
            });
        } else if !species_ids.contains(source_species_id) {
            issues.push(EvolutionTableIssue::UnknownSourceSpecies {
                species_id: source_species_id.clone(),
            });
        }
        for entry in entries {
            if !is_exact_nonempty_evolution_token(&entry.species) {
                issues.push(EvolutionTableIssue::InvalidTargetSpecies {
                    source_species_id: source_species_id.clone(),
                    target_species_id: entry.species.clone(),
                });
            } else if !species_ids.contains(&entry.species) {
                issues.push(EvolutionTableIssue::UnknownTargetSpecies {
                    source_species_id: source_species_id.clone(),
                    target_species_id: entry.species.clone(),
                });
            }
            match entry.method.as_str() {
                METHOD_LEVEL => {
                    if entry.level.is_none() {
                        issues.push(EvolutionTableIssue::MissingLevel {
                            source_species_id: source_species_id.clone(),
                        });
                    }
                }
                METHOD_ITEM => match entry.item.as_deref() {
                    Some(item_id) if item_ids.contains(item_id) => {}
                    Some(item_id) if !is_exact_nonempty_evolution_token(item_id) => {
                        issues.push(EvolutionTableIssue::InvalidItem {
                            source_species_id: source_species_id.clone(),
                            item_id: item_id.to_string(),
                        });
                    }
                    Some(item_id) => issues.push(EvolutionTableIssue::UnknownItem {
                        source_species_id: source_species_id.clone(),
                        item_id: item_id.to_string(),
                    }),
                    None => issues.push(EvolutionTableIssue::MissingItem {
                        source_species_id: source_species_id.clone(),
                    }),
                },
                METHOD_HAPPINESS => match entry.happiness.as_deref() {
                    Some(window) if is_known_happiness_window(window) => {}
                    Some(window) if !is_exact_nonempty_evolution_token(window) => {
                        issues.push(EvolutionTableIssue::InvalidHappinessWindow {
                            source_species_id: source_species_id.clone(),
                            window: window.to_string(),
                        });
                    }
                    Some(window) => issues.push(EvolutionTableIssue::UnknownHappinessWindow {
                        source_species_id: source_species_id.clone(),
                        window: window.to_string(),
                    }),
                    None => issues.push(EvolutionTableIssue::MissingHappinessWindow {
                        source_species_id: source_species_id.clone(),
                    }),
                },
                METHOD_TRADE => {
                    if let Some(item_id) = entry.held_item.as_deref() {
                        if item_id != TRADE_ANY_ITEM && !is_exact_nonempty_evolution_token(item_id)
                        {
                            issues.push(EvolutionTableIssue::InvalidTradeItem {
                                source_species_id: source_species_id.clone(),
                                item_id: item_id.to_string(),
                            });
                        } else if item_id != TRADE_ANY_ITEM && !item_ids.contains(item_id) {
                            issues.push(EvolutionTableIssue::UnknownTradeItem {
                                source_species_id: source_species_id.clone(),
                                item_id: item_id.to_string(),
                            });
                        }
                    }
                }
                METHOD_STAT => {
                    if entry.level.is_none() {
                        issues.push(EvolutionTableIssue::MissingStatLevel {
                            source_species_id: source_species_id.clone(),
                        });
                    }
                    match entry.stat_ratio.as_deref() {
                        Some(ratio) if is_known_stat_evolution_ratio(ratio) => {}
                        Some(ratio) if !is_exact_nonempty_evolution_token(ratio) => {
                            issues.push(EvolutionTableIssue::InvalidStatRatio {
                                source_species_id: source_species_id.clone(),
                                ratio: ratio.to_string(),
                            });
                        }
                        Some(ratio) => issues.push(EvolutionTableIssue::UnknownStatRatio {
                            source_species_id: source_species_id.clone(),
                            ratio: ratio.to_string(),
                        }),
                        None => issues.push(EvolutionTableIssue::MissingStatRatio {
                            source_species_id: source_species_id.clone(),
                        }),
                    }
                }
                method if !is_exact_nonempty_evolution_token(method) => {
                    issues.push(EvolutionTableIssue::InvalidMethod {
                        source_species_id: source_species_id.clone(),
                        method: method.to_string(),
                    });
                }
                method => issues.push(EvolutionTableIssue::UnknownMethod {
                    source_species_id: source_species_id.clone(),
                    method: method.to_string(),
                }),
            }
        }
    }
    issues
}

fn is_exact_nonempty_evolution_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !has_reserved_pack_prefix(value)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

fn has_reserved_pack_prefix(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("fallback") || value.starts_with("legacy")
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct EvolutionTable(pub BTreeMap<String, Vec<EvolutionEntry>>);

impl<'de> Deserialize<'de> for EvolutionTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let table = BTreeMap::<String, Vec<EvolutionEntry>>::deserialize(deserializer)?;
        for (source_species_id, entries) in &table {
            validate_evolution_pack_token("evolution source species", source_species_id)
                .map_err(serde::de::Error::custom)?;
            for (index, entry) in entries.iter().enumerate() {
                validate_evolution_pack_token(
                    &format!("{source_species_id}[{index}].method"),
                    &entry.method,
                )
                .map_err(serde::de::Error::custom)?;
                validate_evolution_pack_token(
                    &format!("{source_species_id}[{index}].species"),
                    &entry.species,
                )
                .map_err(serde::de::Error::custom)?;
                validate_optional_evolution_pack_token(
                    &format!("{source_species_id}[{index}].item"),
                    entry.item.as_deref(),
                )
                .map_err(serde::de::Error::custom)?;
                validate_optional_evolution_pack_token(
                    &format!("{source_species_id}[{index}].held_item"),
                    entry.held_item.as_deref(),
                )
                .map_err(serde::de::Error::custom)?;
                validate_optional_evolution_pack_token(
                    &format!("{source_species_id}[{index}].happiness"),
                    entry.happiness.as_deref(),
                )
                .map_err(serde::de::Error::custom)?;
                validate_optional_evolution_pack_token(
                    &format!("{source_species_id}[{index}].stat_ratio"),
                    entry.stat_ratio.as_deref(),
                )
                .map_err(serde::de::Error::custom)?;
            }
        }
        Ok(Self(table))
    }
}

impl EvolutionTable {
    pub fn entries_for(&self, species_id: &str) -> Result<&[EvolutionEntry], EvolutionError> {
        self.0.get(species_id).map(Vec::as_slice).ok_or_else(|| {
            EvolutionError::MissingEvolutionData {
                species_id: species_id.to_string(),
            }
        })
    }

    pub fn contains_item_evolution(&self, item_id: &str) -> bool {
        self.0
            .values()
            .flatten()
            .any(|entry| entry.method == METHOD_ITEM && entry.item.as_deref() == Some(item_id))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvolutionEntry {
    pub method: String,
    pub species: String,
    #[serde(deserialize_with = "required_nullable_u8")]
    pub level: Option<u8>,
    #[serde(deserialize_with = "required_nullable_string")]
    pub item: Option<String>,
    #[serde(deserialize_with = "required_nullable_string")]
    pub held_item: Option<String>,
    #[serde(deserialize_with = "required_nullable_string")]
    pub happiness: Option<String>,
    #[serde(deserialize_with = "required_nullable_string")]
    pub stat_ratio: Option<String>,
}

fn required_nullable_u8<'de, D>(deserializer: D) -> Result<Option<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<u8>::deserialize(deserializer)
}

fn required_nullable_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)
}

fn validate_optional_evolution_pack_token(field: &str, value: Option<&str>) -> Result<(), String> {
    if let Some(value) = value {
        validate_evolution_pack_token(field, value)?;
    }
    Ok(())
}

fn validate_evolution_pack_token(field: &str, value: &str) -> Result<(), String> {
    if is_exact_nonempty_evolution_token(value) {
        Ok(())
    } else {
        Err(format!(
            "{field} must be exact ASCII alphanumeric/underscore/dash, found {value:?}"
        ))
    }
}

impl EvolutionEntry {
    pub fn level(species: impl Into<String>, level: u8) -> Self {
        Self {
            method: METHOD_LEVEL.to_string(),
            species: species.into(),
            level: Some(level),
            item: None,
            held_item: None,
            happiness: None,
            stat_ratio: None,
        }
    }

    pub fn item(species: impl Into<String>, item: impl Into<String>) -> Self {
        Self {
            method: METHOD_ITEM.to_string(),
            species: species.into(),
            level: None,
            item: Some(item.into()),
            held_item: None,
            happiness: None,
            stat_ratio: None,
        }
    }

    pub fn happiness(species: impl Into<String>, window: impl Into<String>) -> Self {
        Self {
            method: METHOD_HAPPINESS.to_string(),
            species: species.into(),
            level: None,
            item: None,
            held_item: None,
            happiness: Some(window.into()),
            stat_ratio: None,
        }
    }

    pub fn trade(species: impl Into<String>, held_item: Option<impl Into<String>>) -> Self {
        Self {
            method: METHOD_TRADE.to_string(),
            species: species.into(),
            level: None,
            item: None,
            held_item: held_item.map(Into::into),
            happiness: None,
            stat_ratio: None,
        }
    }

    pub fn stat(species: impl Into<String>, level: u8, ratio: impl Into<String>) -> Self {
        Self {
            method: METHOD_STAT.to_string(),
            species: species.into(),
            level: Some(level),
            item: None,
            held_item: None,
            happiness: None,
            stat_ratio: Some(ratio.into()),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum LinkMode {
    #[default]
    None,
    Link,
    TimeCapsule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvolutionContext<'a> {
    pub species: &'a BTreeMap<String, PokemonSpecies>,
    pub moves: &'a BTreeMap<String, Move>,
    pub learnsets: &'a SpeciesLearnsets,
    pub time_of_day: TimeOfDay,
    pub current_item: Option<&'a str>,
    pub force_evolution: bool,
    pub link_mode: LinkMode,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EvolutionReport {
    pub target_species: Option<String>,
    pub events: Vec<EvolutionEvent>,
    pub pending_move_learns: Vec<LearnedMove>,
    /// Exact pre-evolution state retained only while Crystal's animation can
    /// still be cancelled with B. Forced item/trade evolutions omit it.
    pub cancel_snapshot: Option<Box<Pokemon>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvolutionEvent {
    Text(&'static str),
    ItemConsumed(String),
    MoveLearned(String),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EvolutionError {
    #[error("evolution requires a Pokemon species id")]
    MissingSpeciesId,
    #[error("missing evolution table entry for species {species_id}")]
    MissingEvolutionData { species_id: String },
    #[error("missing level-up learnset for species {species_id}")]
    MissingLearnset { species_id: String },
    #[error("evolution target species {species_id} was not loaded")]
    MissingTargetSpecies { species_id: String },
    #[error("invalid evolution species id {species_id}")]
    InvalidSpeciesId { species_id: String },
    #[error("item evolution for {species_id} is missing required item")]
    MissingRequiredItem { species_id: String },
    #[error("invalid evolution item {item_id} for {species_id}")]
    InvalidItem { species_id: String, item_id: String },
    #[error("level evolution for {species_id} is missing required level")]
    MissingRequiredLevel { species_id: String },
    #[error("happiness evolution for {species_id} is missing required window")]
    MissingHappinessWindow { species_id: String },
    #[error("invalid happiness window {window} for {species_id}")]
    InvalidHappinessWindow { species_id: String, window: String },
    #[error("stat evolution for {species_id} is missing required ratio")]
    MissingStatRatio { species_id: String },
    #[error("invalid stat ratio {ratio} for {species_id}")]
    InvalidStatRatio { species_id: String, ratio: String },
    #[error("unknown happiness window {window} for {species_id}")]
    UnknownHappinessWindow { species_id: String, window: String },
    #[error("unknown stat ratio {ratio} for {species_id}")]
    UnknownStatRatio { species_id: String, ratio: String },
    #[error("invalid evolution method {method} for {species_id}")]
    InvalidMethod { species_id: String, method: String },
    #[error("unknown evolution method {method} for {species_id}")]
    UnknownMethod { species_id: String, method: String },
    #[error("invalid move id {move_id} in evolution learnset")]
    InvalidMoveId { move_id: String },
    #[error("missing move data for evolution move {move_id}")]
    MissingMoveData { move_id: String },
    #[error("evolution cannot be cancelled")]
    NotCancellable,
    #[error("cannot cancel evolution into {target_species}: current species is {current_species}")]
    CancelTargetMismatch {
        target_species: String,
        current_species: String,
    },
}

pub fn find_evolution_candidate<'a>(
    pokemon: &Pokemon,
    table: &'a EvolutionTable,
    context: &EvolutionContext<'_>,
) -> Result<Option<&'a EvolutionEntry>, EvolutionError> {
    let species_id = pokemon.species.id.as_str();
    if species_id.is_empty() {
        return Err(EvolutionError::MissingSpeciesId);
    }
    validate_evolution_runtime_token(species_id).map_err(|_| EvolutionError::InvalidSpeciesId {
        species_id: species_id.to_string(),
    })?;

    let entries = table.entries_for(species_id)?;
    for entry in entries {
        validate_evolution_runtime_token(&entry.method).map_err(|_| {
            EvolutionError::InvalidMethod {
                species_id: species_id.to_string(),
                method: entry.method.clone(),
            }
        })?;
        validate_evolution_runtime_token(&entry.species).map_err(|_| {
            EvolutionError::InvalidSpeciesId {
                species_id: entry.species.clone(),
            }
        })?;
        match entry.method.as_str() {
            METHOD_ITEM => {
                if !context.force_evolution || context.link_mode != LinkMode::None {
                    continue;
                }
                let required =
                    entry
                        .item
                        .as_deref()
                        .ok_or_else(|| EvolutionError::MissingRequiredItem {
                            species_id: species_id.to_string(),
                        })?;
                validate_evolution_runtime_token(required).map_err(|_| {
                    EvolutionError::InvalidItem {
                        species_id: species_id.to_string(),
                        item_id: required.to_string(),
                    }
                })?;
                if Some(required) == context.current_item {
                    return Ok(Some(entry));
                }
            }
            METHOD_LEVEL => {
                let required_level =
                    entry
                        .level
                        .ok_or_else(|| EvolutionError::MissingRequiredLevel {
                            species_id: species_id.to_string(),
                        })?;
                if context.force_evolution
                    || required_level > pokemon.level
                    || is_holding_everstone(pokemon)
                {
                    continue;
                }
                return Ok(Some(entry));
            }
            METHOD_HAPPINESS => {
                if context.force_evolution
                    || pokemon.happiness < HAPPINESS_TO_EVOLVE
                    || is_holding_everstone(pokemon)
                {
                    continue;
                }
                let window = entry.happiness.as_deref().ok_or_else(|| {
                    EvolutionError::MissingHappinessWindow {
                        species_id: species_id.to_string(),
                    }
                })?;
                validate_evolution_runtime_token(window).map_err(|_| {
                    EvolutionError::InvalidHappinessWindow {
                        species_id: species_id.to_string(),
                        window: window.to_string(),
                    }
                })?;
                match window {
                    HAPPINESS_ANYTIME => return Ok(Some(entry)),
                    HAPPINESS_MORNDAY if context.time_of_day != TimeOfDay::Night => {
                        return Ok(Some(entry));
                    }
                    HAPPINESS_MORNDAY => {}
                    HAPPINESS_NITE if context.time_of_day == TimeOfDay::Night => {
                        return Ok(Some(entry));
                    }
                    HAPPINESS_NITE => {}
                    _ if !is_known_happiness_window(window) => {
                        return Err(EvolutionError::UnknownHappinessWindow {
                            species_id: species_id.to_string(),
                            window: window.to_string(),
                        });
                    }
                    _ => {}
                }
            }
            METHOD_STAT => {
                let required_level =
                    entry
                        .level
                        .ok_or_else(|| EvolutionError::MissingRequiredLevel {
                            species_id: species_id.to_string(),
                        })?;
                if context.force_evolution
                    || required_level > pokemon.level
                    || is_holding_everstone(pokemon)
                {
                    continue;
                }
                let ratio = entry.stat_ratio.as_deref().ok_or_else(|| {
                    EvolutionError::MissingStatRatio {
                        species_id: species_id.to_string(),
                    }
                })?;
                validate_evolution_runtime_token(ratio).map_err(|_| {
                    EvolutionError::InvalidStatRatio {
                        species_id: species_id.to_string(),
                        ratio: ratio.to_string(),
                    }
                })?;
                let matches_ratio = match ratio {
                    STAT_ATK_GT_DEF => pokemon.attack > pokemon.defense,
                    STAT_ATK_LT_DEF => pokemon.attack < pokemon.defense,
                    STAT_ATK_EQ_DEF => pokemon.attack == pokemon.defense,
                    _ => {
                        return Err(EvolutionError::UnknownStatRatio {
                            species_id: species_id.to_string(),
                            ratio: ratio.to_string(),
                        });
                    }
                };
                if matches_ratio {
                    return Ok(Some(entry));
                }
            }
            METHOD_TRADE => {
                if context.link_mode == LinkMode::None || is_holding_everstone(pokemon) {
                    continue;
                }
                let Some(required) = entry.held_item.as_deref() else {
                    return Ok(Some(entry));
                };
                if required == TRADE_ANY_ITEM {
                    return Ok(Some(entry));
                }
                validate_evolution_runtime_token(required).map_err(|_| {
                    EvolutionError::InvalidItem {
                        species_id: species_id.to_string(),
                        item_id: required.to_string(),
                    }
                })?;
                if context.link_mode == LinkMode::TimeCapsule {
                    continue;
                }
                if pokemon.item.as_deref() == Some(required) {
                    return Ok(Some(entry));
                }
            }
            method => {
                return Err(EvolutionError::UnknownMethod {
                    species_id: species_id.to_string(),
                    method: method.to_string(),
                });
            }
        }
    }
    Ok(None)
}

fn validate_evolution_runtime_token(value: &str) -> Result<(), ()> {
    if is_exact_nonempty_evolution_token(value) {
        Ok(())
    } else {
        Err(())
    }
}

pub fn evolve_pokemon(
    pokemon: &mut Pokemon,
    entry: &EvolutionEntry,
    context: &EvolutionContext<'_>,
    include_intro: bool,
) -> Result<EvolutionReport, EvolutionError> {
    validate_evolution_runtime_token(&entry.species).map_err(|_| {
        EvolutionError::InvalidSpeciesId {
            species_id: entry.species.clone(),
        }
    })?;
    let target_species = context.species.get(&entry.species).ok_or_else(|| {
        EvolutionError::MissingTargetSpecies {
            species_id: entry.species.clone(),
        }
    })?;
    let move_learns =
        evolution_moves_for(&target_species.id, pokemon.level, &pokemon.moves, context)?;
    let cancel_snapshot =
        (include_intro && !context.force_evolution && context.link_mode == LinkMode::None)
            .then(|| Box::new(pokemon.clone()));

    let mut events = Vec::new();
    if include_intro {
        events.push(EvolutionEvent::Text("EvolvingText"));
    }

    let old_species_id = pokemon.species.id.clone();
    let old_max_hp = pokemon.max_hp;
    let old_hp = pokemon.hp;
    if !pokemon.nickname.trim().is_empty()
        && pokemon
            .nickname
            .trim()
            .eq_ignore_ascii_case(&old_species_id)
    {
        pokemon.nickname = target_species.id.to_uppercase();
    }
    pokemon.species = target_species.clone();
    refresh_evolved_stats(pokemon, old_max_hp, old_hp);
    events.push(EvolutionEvent::Text("EvolvedIntoText"));

    if entry.method == METHOD_TRADE {
        if let Some(required) = entry.held_item.as_deref() {
            if required != TRADE_ANY_ITEM {
                pokemon.item = None;
                events.push(EvolutionEvent::ItemConsumed(required.to_string()));
            }
        }
    }

    pokemon.moves.extend(move_learns.learned.clone());
    for learned in move_learns.learned {
        events.push(EvolutionEvent::MoveLearned(learned.name));
    }

    Ok(EvolutionReport {
        target_species: Some(target_species.id.clone()),
        events,
        pending_move_learns: move_learns.pending,
        cancel_snapshot,
    })
}

/// Resolve Crystal's cancellable animation boundary. The cartridge does not
/// write the new species, stats, held item, nickname, or moves until after the
/// animation returns without carry, so cancellation restores the exact state
/// retained immediately before the provisional evolution mutation.
pub fn cancel_evolution(
    pokemon: &mut Pokemon,
    report: &mut EvolutionReport,
) -> Result<(), EvolutionError> {
    let target_species = report
        .target_species
        .clone()
        .ok_or(EvolutionError::NotCancellable)?;
    let source = report
        .cancel_snapshot
        .take()
        .ok_or(EvolutionError::NotCancellable)?;
    if pokemon.species.id != target_species {
        report.cancel_snapshot = Some(source);
        return Err(EvolutionError::CancelTargetMismatch {
            target_species,
            current_species: pokemon.species.id.clone(),
        });
    }

    *pokemon = *source;
    report.target_species = None;
    report.pending_move_learns.clear();
    report
        .events
        .retain(|event| matches!(event, EvolutionEvent::Text(text) if *text == "EvolvingText"));
    report
        .events
        .push(EvolutionEvent::Text("StoppedEvolvingText"));
    Ok(())
}

pub fn check_and_evolve(
    pokemon: &mut Pokemon,
    table: &EvolutionTable,
    context: &EvolutionContext<'_>,
    include_intro: bool,
) -> Result<EvolutionReport, EvolutionError> {
    let Some(entry) = find_evolution_candidate(pokemon, table, context)? else {
        return Ok(EvolutionReport::default());
    };
    evolve_pokemon(pokemon, entry, context, include_intro)
}

fn refresh_evolved_stats(pokemon: &mut Pokemon, old_max_hp: u16, old_hp: u16) {
    let stats = calculate_stats(
        &pokemon.species,
        pokemon.level,
        pokemon.dvs,
        StatExperience {
            hp: pokemon.hp_exp,
            attack: pokemon.attack_exp,
            defense: pokemon.defense_exp,
            speed: pokemon.speed_exp,
            special: pokemon.special_exp,
        },
    );
    pokemon.max_hp = stats.max_hp;
    pokemon.attack = stats.attack;
    pokemon.defense = stats.defense;
    pokemon.speed = stats.speed;
    pokemon.special_attack = stats.special_attack;
    pokemon.special_defense = stats.special_defense;

    let hp_delta = i32::from(stats.max_hp) - i32::from(old_max_hp);
    pokemon.hp = (i32::from(old_hp) + hp_delta).clamp(0, i32::from(stats.max_hp)) as u16;
}

struct EvolutionMoveLearnResult {
    learned: Vec<LearnedMove>,
    pending: Vec<LearnedMove>,
}

fn evolution_moves_for(
    species_id: &str,
    level: u8,
    known_moves: &[LearnedMove],
    context: &EvolutionContext<'_>,
) -> Result<EvolutionMoveLearnResult, EvolutionError> {
    if level == 0 {
        return Ok(EvolutionMoveLearnResult {
            learned: Vec::new(),
            pending: Vec::new(),
        });
    }
    let mut current = known_moves.to_vec();
    let mut learned = Vec::new();
    let mut pending = Vec::new();
    for LearnsetEntry(learn_level, move_name) in
        level_up_moves_for_species(context.learnsets, species_id).map_err(|_| {
            EvolutionError::MissingLearnset {
                species_id: species_id.to_string(),
            }
        })?
    {
        if *learn_level != level || current.iter().any(|known| known.name == *move_name) {
            continue;
        }
        validate_evolution_runtime_token(move_name).map_err(|_| EvolutionError::InvalidMoveId {
            move_id: move_name.clone(),
        })?;
        let move_data =
            context
                .moves
                .get(move_name)
                .ok_or_else(|| EvolutionError::MissingMoveData {
                    move_id: move_name.clone(),
                })?;
        let entry = LearnedMove {
            name: move_name.clone(),
            current_pp: move_data.pp,
            pp_ups: 0,
        };
        if current.len() < 4 {
            current.push(entry.clone());
            learned.push(entry);
        } else {
            pending.push(entry);
        }
    }
    Ok(EvolutionMoveLearnResult { learned, pending })
}

fn is_holding_everstone(pokemon: &Pokemon) -> bool {
    pokemon.item.as_deref() == Some(EVERSTONE_ITEM_ID)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BaseStats, Dv, growth_rate, pokemon_type};

    fn species(id: &str, hp: u16, attack: u16, defense: u16) -> PokemonSpecies {
        let mut species =
            PokemonSpecies::new_for_tests(id, BaseStats::new(hp, attack, defense, 45, 65, 65));
        species.growth_rate = growth_rate("GROWTH_MEDIUM_FAST");
        species.type1 = pokemon_type("NORMAL");
        species.type2 = pokemon_type("NORMAL");
        species
    }

    fn pokemon(id: &str, level: u8) -> Pokemon {
        Pokemon::new_for_tests(
            species(id, 40, 40, 40),
            level,
            Dv::from_non_hp(10, 10, 10, 10),
        )
    }

    fn move_data(name: &str, pp: u8) -> Move {
        Move {
            source_index: 1,
            name: name.to_string(),
            move_type: pokemon_type("NORMAL"),
            power: 40,
            accuracy: 100,
            pp,
            effect: "NORMAL_HIT".to_string(),
            effect_chance: 0,
            stat: None,
            amount: None,
        }
    }

    #[test]
    fn evolution_table_issues_validate_exact_modpack_ids_without_coercion() {
        let species_ids = ["CHIKORITA".to_string(), "BAYLEEF".to_string()]
            .into_iter()
            .collect();
        let item_ids = ["THUNDERSTONE".to_string()].into_iter().collect();
        let table = EvolutionTable(
            [
                (
                    " chikorita".to_string(),
                    vec![EvolutionEntry::level("BAYLEEF", 16)],
                ),
                (
                    "chikorita".to_string(),
                    vec![EvolutionEntry::level("BAYLEEF", 16)],
                ),
                (
                    "CHIKORITA".to_string(),
                    vec![
                        EvolutionEntry {
                            method: METHOD_LEVEL.to_string(),
                            species: "BAYLEEF".to_string(),
                            level: None,
                            item: None,
                            held_item: None,
                            happiness: None,
                            stat_ratio: None,
                        },
                        EvolutionEntry::item(" BAYLEEF", " THUNDERSTONE"),
                        EvolutionEntry::item("BAY LEEF", "THUNDER STONE"),
                        EvolutionEntry::item("bayleef", "thunderstone"),
                        EvolutionEntry::happiness("BAYLEEF", " MORNING"),
                        EvolutionEntry::happiness("BAYLEEF", "TR MORNDAY"),
                        EvolutionEntry::happiness("BAYLEEF", "MORNING"),
                        EvolutionEntry::trade("BAYLEEF", Some(" kings_rock")),
                        EvolutionEntry::trade("BAYLEEF", Some("KINGS ROCK")),
                        EvolutionEntry::trade("BAYLEEF", Some("kings_rock")),
                        EvolutionEntry {
                            method: METHOD_STAT.to_string(),
                            species: "BAYLEEF".to_string(),
                            level: None,
                            item: None,
                            held_item: None,
                            happiness: None,
                            stat_ratio: Some(" ATTACKIER".to_string()),
                        },
                        EvolutionEntry {
                            method: METHOD_STAT.to_string(),
                            species: "BAYLEEF".to_string(),
                            level: None,
                            item: None,
                            held_item: None,
                            happiness: None,
                            stat_ratio: Some("ATK GT_DEF".to_string()),
                        },
                        EvolutionEntry {
                            method: METHOD_STAT.to_string(),
                            species: "BAYLEEF".to_string(),
                            level: None,
                            item: None,
                            held_item: None,
                            happiness: None,
                            stat_ratio: Some("ATTACKIER".to_string()),
                        },
                        EvolutionEntry {
                            method: " MOON_PHASE".to_string(),
                            species: "BAYLEEF".to_string(),
                            level: None,
                            item: None,
                            held_item: None,
                            happiness: None,
                            stat_ratio: None,
                        },
                        EvolutionEntry {
                            method: "MOON PHASE".to_string(),
                            species: "BAYLEEF".to_string(),
                            level: None,
                            item: None,
                            held_item: None,
                            happiness: None,
                            stat_ratio: None,
                        },
                        EvolutionEntry {
                            method: "MOON_PHASE".to_string(),
                            species: "BAYLEEF".to_string(),
                            level: None,
                            item: None,
                            held_item: None,
                            happiness: None,
                            stat_ratio: None,
                        },
                    ],
                ),
            ]
            .into_iter()
            .collect(),
        );

        assert_eq!(
            evolution_table_issues(&table, &species_ids, &item_ids),
            vec![
                EvolutionTableIssue::MissingSpeciesEvolutions {
                    species_id: "BAYLEEF".to_string(),
                },
                EvolutionTableIssue::InvalidSourceSpecies {
                    species_id: " chikorita".to_string(),
                },
                EvolutionTableIssue::MissingLevel {
                    source_species_id: "CHIKORITA".to_string(),
                },
                EvolutionTableIssue::InvalidTargetSpecies {
                    source_species_id: "CHIKORITA".to_string(),
                    target_species_id: " BAYLEEF".to_string(),
                },
                EvolutionTableIssue::InvalidItem {
                    source_species_id: "CHIKORITA".to_string(),
                    item_id: " THUNDERSTONE".to_string(),
                },
                EvolutionTableIssue::InvalidTargetSpecies {
                    source_species_id: "CHIKORITA".to_string(),
                    target_species_id: "BAY LEEF".to_string(),
                },
                EvolutionTableIssue::InvalidItem {
                    source_species_id: "CHIKORITA".to_string(),
                    item_id: "THUNDER STONE".to_string(),
                },
                EvolutionTableIssue::UnknownTargetSpecies {
                    source_species_id: "CHIKORITA".to_string(),
                    target_species_id: "bayleef".to_string(),
                },
                EvolutionTableIssue::UnknownItem {
                    source_species_id: "CHIKORITA".to_string(),
                    item_id: "thunderstone".to_string(),
                },
                EvolutionTableIssue::InvalidHappinessWindow {
                    source_species_id: "CHIKORITA".to_string(),
                    window: " MORNING".to_string(),
                },
                EvolutionTableIssue::InvalidHappinessWindow {
                    source_species_id: "CHIKORITA".to_string(),
                    window: "TR MORNDAY".to_string(),
                },
                EvolutionTableIssue::UnknownHappinessWindow {
                    source_species_id: "CHIKORITA".to_string(),
                    window: "MORNING".to_string(),
                },
                EvolutionTableIssue::InvalidTradeItem {
                    source_species_id: "CHIKORITA".to_string(),
                    item_id: " kings_rock".to_string(),
                },
                EvolutionTableIssue::InvalidTradeItem {
                    source_species_id: "CHIKORITA".to_string(),
                    item_id: "KINGS ROCK".to_string(),
                },
                EvolutionTableIssue::UnknownTradeItem {
                    source_species_id: "CHIKORITA".to_string(),
                    item_id: "kings_rock".to_string(),
                },
                EvolutionTableIssue::MissingStatLevel {
                    source_species_id: "CHIKORITA".to_string(),
                },
                EvolutionTableIssue::InvalidStatRatio {
                    source_species_id: "CHIKORITA".to_string(),
                    ratio: " ATTACKIER".to_string(),
                },
                EvolutionTableIssue::MissingStatLevel {
                    source_species_id: "CHIKORITA".to_string(),
                },
                EvolutionTableIssue::InvalidStatRatio {
                    source_species_id: "CHIKORITA".to_string(),
                    ratio: "ATK GT_DEF".to_string(),
                },
                EvolutionTableIssue::MissingStatLevel {
                    source_species_id: "CHIKORITA".to_string(),
                },
                EvolutionTableIssue::UnknownStatRatio {
                    source_species_id: "CHIKORITA".to_string(),
                    ratio: "ATTACKIER".to_string(),
                },
                EvolutionTableIssue::InvalidMethod {
                    source_species_id: "CHIKORITA".to_string(),
                    method: " MOON_PHASE".to_string(),
                },
                EvolutionTableIssue::InvalidMethod {
                    source_species_id: "CHIKORITA".to_string(),
                    method: "MOON PHASE".to_string(),
                },
                EvolutionTableIssue::UnknownMethod {
                    source_species_id: "CHIKORITA".to_string(),
                    method: "MOON_PHASE".to_string(),
                },
                EvolutionTableIssue::UnknownSourceSpecies {
                    species_id: "chikorita".to_string(),
                },
            ]
        );
    }

    #[test]
    fn evolution_table_issues_reject_reserved_pack_prefix_tokens() {
        let species_ids = ["CHIKORITA".to_string(), "BAYLEEF".to_string()]
            .into_iter()
            .collect();
        let item_ids = BTreeSet::new();
        let table = EvolutionTable(
            [(
                "fallback_chikorita".to_string(),
                vec![
                    EvolutionEntry::level("legacy_bayleef", 16),
                    EvolutionEntry::item("BAYLEEF", "fallback_stone"),
                    EvolutionEntry {
                        method: "legacy_method".to_string(),
                        species: "BAYLEEF".to_string(),
                        level: None,
                        item: None,
                        held_item: None,
                        happiness: None,
                        stat_ratio: None,
                    },
                ],
            )]
            .into_iter()
            .collect(),
        );

        assert_eq!(
            evolution_table_issues(&table, &species_ids, &item_ids),
            vec![
                EvolutionTableIssue::MissingSpeciesEvolutions {
                    species_id: "BAYLEEF".to_string(),
                },
                EvolutionTableIssue::MissingSpeciesEvolutions {
                    species_id: "CHIKORITA".to_string(),
                },
                EvolutionTableIssue::InvalidSourceSpecies {
                    species_id: "fallback_chikorita".to_string(),
                },
                EvolutionTableIssue::InvalidTargetSpecies {
                    source_species_id: "fallback_chikorita".to_string(),
                    target_species_id: "legacy_bayleef".to_string(),
                },
                EvolutionTableIssue::InvalidItem {
                    source_species_id: "fallback_chikorita".to_string(),
                    item_id: "fallback_stone".to_string(),
                },
                EvolutionTableIssue::InvalidMethod {
                    source_species_id: "fallback_chikorita".to_string(),
                    method: "legacy_method".to_string(),
                },
            ]
        );
    }

    fn context<'a>(
        species: &'a BTreeMap<String, PokemonSpecies>,
        moves: &'a BTreeMap<String, Move>,
        learnsets: &'a SpeciesLearnsets,
    ) -> EvolutionContext<'a> {
        EvolutionContext {
            species,
            moves,
            learnsets,
            time_of_day: TimeOfDay::Day,
            current_item: None,
            force_evolution: false,
            link_mode: LinkMode::None,
        }
    }

    #[test]
    fn level_evolution_updates_species_stats_hp_and_nickname_exactly() {
        let mut species_map = BTreeMap::new();
        species_map.insert("BULBASAUR".to_string(), species("BULBASAUR", 40, 40, 40));
        species_map.insert("IVYSAUR".to_string(), species("IVYSAUR", 60, 60, 60));
        let moves = BTreeMap::new();
        let learnsets = [("IVYSAUR".to_string(), Vec::new())].into_iter().collect();
        let table = EvolutionTable(
            [(
                "BULBASAUR".to_string(),
                vec![EvolutionEntry::level("IVYSAUR", 16)],
            )]
            .into_iter()
            .collect(),
        );
        let mut pokemon = pokemon("BULBASAUR", 16);
        pokemon.hp = pokemon.max_hp - 3;
        let old_max_hp = pokemon.max_hp;
        let report = check_and_evolve(
            &mut pokemon,
            &table,
            &context(&species_map, &moves, &learnsets),
            true,
        )
        .expect("evolve");

        assert_eq!(report.target_species, Some("IVYSAUR".to_string()));
        assert_eq!(pokemon.species.id, "IVYSAUR");
        assert_eq!(pokemon.nickname, "IVYSAUR");
        assert_eq!(pokemon.hp, pokemon.max_hp - 3);
        assert!(pokemon.max_hp > old_max_hp);
        assert_eq!(
            report.events,
            vec![
                EvolutionEvent::Text("EvolvingText"),
                EvolutionEvent::Text("EvolvedIntoText"),
            ]
        );
    }

    #[test]
    fn level_evolution_is_blocked_by_exact_everstone_only() {
        let species_map = [("IVYSAUR".to_string(), species("IVYSAUR", 60, 60, 60))]
            .into_iter()
            .collect();
        let moves = BTreeMap::new();
        let learnsets = SpeciesLearnsets::new();
        let table = EvolutionTable(
            [(
                "BULBASAUR".to_string(),
                vec![EvolutionEntry::level("IVYSAUR", 16)],
            )]
            .into_iter()
            .collect(),
        );

        let mut blocked = pokemon("BULBASAUR", 16);
        blocked.item = Some("EVERSTONE".to_string());
        assert_eq!(
            find_evolution_candidate(&blocked, &table, &context(&species_map, &moves, &learnsets))
                .expect("candidate"),
            None
        );

        let mut exact_only = pokemon("BULBASAUR", 16);
        exact_only.item = Some("everstone".to_string());
        assert_eq!(
            find_evolution_candidate(
                &exact_only,
                &table,
                &context(&species_map, &moves, &learnsets)
            )
            .expect("candidate")
            .map(|entry| entry.species.as_str()),
            Some("IVYSAUR")
        );
    }

    #[test]
    fn item_happiness_stat_and_trade_evolutions_use_exact_context() {
        let species_map: BTreeMap<_, _> = [
            ("RAICHU".to_string(), species("RAICHU", 60, 60, 60)),
            ("ESPEON".to_string(), species("ESPEON", 65, 65, 55)),
            ("UMBREON".to_string(), species("UMBREON", 65, 55, 65)),
            ("HITMONLEE".to_string(), species("HITMONLEE", 50, 80, 40)),
            ("STEELIX".to_string(), species("STEELIX", 75, 85, 200)),
        ]
        .into_iter()
        .collect();
        let moves = BTreeMap::new();
        let learnsets = SpeciesLearnsets::new();
        let table = EvolutionTable(
            [
                (
                    "PIKACHU".to_string(),
                    vec![EvolutionEntry::item("RAICHU", "THUNDERSTONE")],
                ),
                (
                    "EEVEE".to_string(),
                    vec![
                        EvolutionEntry::happiness("ESPEON", HAPPINESS_MORNDAY),
                        EvolutionEntry::happiness("UMBREON", HAPPINESS_NITE),
                    ],
                ),
                (
                    "TYROGUE".to_string(),
                    vec![EvolutionEntry::stat("HITMONLEE", 20, STAT_ATK_GT_DEF)],
                ),
                (
                    "ONIX".to_string(),
                    vec![EvolutionEntry::trade("STEELIX", Some("METAL_COAT"))],
                ),
            ]
            .into_iter()
            .collect(),
        );

        let mut item_context = context(&species_map, &moves, &learnsets);
        item_context.current_item = Some("THUNDERSTONE");
        item_context.force_evolution = true;
        assert_eq!(
            find_evolution_candidate(&pokemon("PIKACHU", 20), &table, &item_context)
                .expect("item")
                .map(|entry| entry.species.as_str()),
            Some("RAICHU")
        );

        let mut eevee = pokemon("EEVEE", 20);
        eevee.happiness = HAPPINESS_TO_EVOLVE;
        let mut night_context = context(&species_map, &moves, &learnsets);
        night_context.time_of_day = TimeOfDay::Night;
        assert_eq!(
            find_evolution_candidate(&eevee, &table, &night_context)
                .expect("night")
                .map(|entry| entry.species.as_str()),
            Some("UMBREON")
        );

        let mut tyrogue = pokemon("TYROGUE", 20);
        tyrogue.attack = 20;
        tyrogue.defense = 10;
        assert_eq!(
            find_evolution_candidate(&tyrogue, &table, &context(&species_map, &moves, &learnsets))
                .expect("stat")
                .map(|entry| entry.species.as_str()),
            Some("HITMONLEE")
        );

        let mut onix = pokemon("ONIX", 30);
        onix.item = Some("METAL_COAT".to_string());
        let mut link_context = context(&species_map, &moves, &learnsets);
        link_context.link_mode = LinkMode::Link;
        assert_eq!(
            find_evolution_candidate(&onix, &table, &link_context)
                .expect("trade")
                .map(|entry| entry.species.as_str()),
            Some("STEELIX")
        );
    }

    #[test]
    fn trade_evolution_removes_required_item_and_learns_current_level_moves() {
        let species_map: BTreeMap<_, _> =
            [("STEELIX".to_string(), species("STEELIX", 75, 85, 200))]
                .into_iter()
                .collect();
        let moves = [("IRON_TAIL".to_string(), move_data("IRON_TAIL", 15))]
            .into_iter()
            .collect();
        let learnsets = [(
            "STEELIX".to_string(),
            vec![LearnsetEntry(30, "IRON_TAIL".to_string())],
        )]
        .into_iter()
        .collect();
        let entry = EvolutionEntry::trade("STEELIX", Some("METAL_COAT"));
        let mut context = context(&species_map, &moves, &learnsets);
        context.link_mode = LinkMode::Link;
        let mut onix = pokemon("ONIX", 30);
        onix.item = Some("METAL_COAT".to_string());

        let report = evolve_pokemon(&mut onix, &entry, &context, false).expect("evolve");

        assert_eq!(onix.item, None);
        assert_eq!(
            onix.moves,
            vec![LearnedMove {
                name: "IRON_TAIL".to_string(),
                current_pp: 15,
                pp_ups: 0,
            }]
        );
        assert_eq!(
            report.events,
            vec![
                EvolutionEvent::Text("EvolvedIntoText"),
                EvolutionEvent::ItemConsumed("METAL_COAT".to_string()),
                EvolutionEvent::MoveLearned("IRON_TAIL".to_string()),
            ]
        );
    }

    #[test]
    fn evolved_species_same_level_move_queues_replacement_when_moves_are_full() {
        let species_map: BTreeMap<_, _> =
            [("DRAGONITE".to_string(), species("DRAGONITE", 91, 134, 95))]
                .into_iter()
                .collect();
        let moves = [
            ("WRAP".to_string(), move_data("WRAP", 20)),
            ("LEER".to_string(), move_data("LEER", 30)),
            ("THUNDER_WAVE".to_string(), move_data("THUNDER_WAVE", 20)),
            ("TWISTER".to_string(), move_data("TWISTER", 20)),
            ("WING_ATTACK".to_string(), move_data("WING_ATTACK", 35)),
        ]
        .into_iter()
        .collect();
        let learnsets = [(
            "DRAGONITE".to_string(),
            vec![LearnsetEntry(55, "WING_ATTACK".to_string())],
        )]
        .into_iter()
        .collect();
        let entry = EvolutionEntry::level("DRAGONITE", 55);
        let context = context(&species_map, &moves, &learnsets);
        let mut dragonair = pokemon("DRAGONAIR", 55);
        dragonair.moves = ["WRAP", "LEER", "THUNDER_WAVE", "TWISTER"]
            .into_iter()
            .map(|move_id| LearnedMove {
                name: move_id.to_string(),
                current_pp: moves[move_id].pp,
                pp_ups: 0,
            })
            .collect();
        let moves_before = dragonair.moves.clone();

        let report = evolve_pokemon(&mut dragonair, &entry, &context, true).expect("evolve");

        assert_eq!(dragonair.species.id, "DRAGONITE");
        assert_eq!(dragonair.moves, moves_before);
        assert_eq!(
            report.pending_move_learns,
            vec![LearnedMove {
                name: "WING_ATTACK".to_string(),
                current_pp: 35,
                pp_ups: 0,
            }]
        );
        assert!(!report.events.iter().any(
            |event| matches!(event, EvolutionEvent::MoveLearned(move_id) if move_id == "WING_ATTACK")
        ));
    }

    #[test]
    fn cancelling_animation_restores_exact_species_stats_item_and_moves() {
        let species_map: BTreeMap<_, _> =
            [("DRAGONITE".to_string(), species("DRAGONITE", 91, 134, 95))]
                .into_iter()
                .collect();
        let moves = [("WING_ATTACK".to_string(), move_data("WING_ATTACK", 35))]
            .into_iter()
            .collect();
        let learnsets = [(
            "DRAGONITE".to_string(),
            vec![LearnsetEntry(55, "WING_ATTACK".to_string())],
        )]
        .into_iter()
        .collect();
        let entry = EvolutionEntry::level("DRAGONITE", 55);
        let context = context(&species_map, &moves, &learnsets);
        let mut dragonair = pokemon("DRAGONAIR", 55);
        dragonair.nickname = "DRAGONAIR".to_string();
        dragonair.item = Some("BERRY".to_string());
        dragonair.moves = vec![LearnedMove {
            name: "WRAP".to_string(),
            current_pp: 17,
            pp_ups: 2,
        }];
        dragonair.hp = 73;
        dragonair.attack = 88;
        dragonair.defense = 79;
        dragonair.speed = 91;
        dragonair.special_attack = 84;
        dragonair.special_defense = 86;
        let before = dragonair.clone();

        let mut report =
            evolve_pokemon(&mut dragonair, &entry, &context, true).expect("start evolution");
        assert_ne!(dragonair, before, "evolution is provisionally committed");

        cancel_evolution(&mut dragonair, &mut report).expect("cancel evolution");

        assert_eq!(dragonair, before);
        assert_eq!(report.target_species, None);
        assert!(report.pending_move_learns.is_empty());
        assert_eq!(
            report.events,
            vec![
                EvolutionEvent::Text("EvolvingText"),
                EvolutionEvent::Text("StoppedEvolvingText"),
            ]
        );
    }

    #[test]
    fn forced_evolution_cannot_be_cancelled() {
        let species_map: BTreeMap<_, _> =
            [("VAPOREON".to_string(), species("VAPOREON", 130, 65, 60))]
                .into_iter()
                .collect();
        let moves = BTreeMap::new();
        let learnsets = [("VAPOREON".to_string(), Vec::new())].into_iter().collect();
        let entry = EvolutionEntry::item("VAPOREON", "WATER_STONE");
        let mut context = context(&species_map, &moves, &learnsets);
        context.force_evolution = true;
        context.current_item = Some("WATER_STONE");
        let mut eevee = pokemon("EEVEE", 25);

        let mut report =
            evolve_pokemon(&mut eevee, &entry, &context, true).expect("force evolution");

        assert_eq!(
            cancel_evolution(&mut eevee, &mut report),
            Err(EvolutionError::NotCancellable)
        );
        assert_eq!(eevee.species.id, "VAPOREON");
    }

    #[test]
    fn evolution_requires_explicit_learnset_for_target_species() {
        let species_map: BTreeMap<_, _> = [("IVYSAUR".to_string(), species("IVYSAUR", 60, 60, 60))]
            .into_iter()
            .collect();
        let moves = BTreeMap::new();
        let learnsets = SpeciesLearnsets::new();
        let entry = EvolutionEntry::level("IVYSAUR", 16);
        let context = context(&species_map, &moves, &learnsets);
        let mut pokemon = pokemon("BULBASAUR", 16);

        assert_eq!(
            evolve_pokemon(&mut pokemon, &entry, &context, false),
            Err(EvolutionError::MissingLearnset {
                species_id: "IVYSAUR".to_string(),
            })
        );
        assert_eq!(pokemon.species.id, "BULBASAUR");
        assert!(pokemon.moves.is_empty());
    }

    #[test]
    fn evolution_rejects_malformed_target_species_before_mutation() {
        let species_map = BTreeMap::new();
        let moves = BTreeMap::new();
        let learnsets = SpeciesLearnsets::new();
        let entry = EvolutionEntry::level("IVY SAUR", 16);
        let context = context(&species_map, &moves, &learnsets);
        let mut pokemon = pokemon("BULBASAUR", 16);

        assert_eq!(
            evolve_pokemon(&mut pokemon, &entry, &context, false),
            Err(EvolutionError::InvalidSpeciesId {
                species_id: "IVY SAUR".to_string(),
            })
        );
        assert_eq!(pokemon.species.id, "BULBASAUR");
        assert!(pokemon.moves.is_empty());
    }

    #[test]
    fn evolution_rejects_reserved_target_species_before_mutation() {
        let species_map = BTreeMap::new();
        let moves = BTreeMap::new();
        let learnsets = SpeciesLearnsets::new();
        let entry = EvolutionEntry::level("fallback_ivysaur", 16);
        let context = context(&species_map, &moves, &learnsets);
        let mut pokemon = pokemon("BULBASAUR", 16);

        assert_eq!(
            evolve_pokemon(&mut pokemon, &entry, &context, false),
            Err(EvolutionError::InvalidSpeciesId {
                species_id: "fallback_ivysaur".to_string(),
            })
        );
        assert_eq!(pokemon.species.id, "BULBASAUR");
        assert!(pokemon.moves.is_empty());
    }

    #[test]
    fn evolution_rejects_malformed_learned_move_before_mutation() {
        let species_map: BTreeMap<_, _> = [("IVYSAUR".to_string(), species("IVYSAUR", 60, 60, 60))]
            .into_iter()
            .collect();
        let moves = BTreeMap::new();
        let learnsets = [(
            "IVYSAUR".to_string(),
            vec![LearnsetEntry(16, "VINE WHIP".to_string())],
        )]
        .into_iter()
        .collect();
        let entry = EvolutionEntry::level("IVYSAUR", 16);
        let context = context(&species_map, &moves, &learnsets);
        let mut pokemon = pokemon("BULBASAUR", 16);

        assert_eq!(
            evolve_pokemon(&mut pokemon, &entry, &context, false),
            Err(EvolutionError::InvalidMoveId {
                move_id: "VINE WHIP".to_string(),
            })
        );
        assert_eq!(pokemon.species.id, "BULBASAUR");
        assert!(pokemon.moves.is_empty());
    }

    #[test]
    fn missing_evolution_table_entry_is_error_not_empty_default() {
        let species_map = BTreeMap::new();
        let moves = BTreeMap::new();
        let learnsets = SpeciesLearnsets::new();
        let table = EvolutionTable::default();
        let context = context(&species_map, &moves, &learnsets);

        assert_eq!(
            find_evolution_candidate(&pokemon("FINAL_MON", 50), &table, &context),
            Err(EvolutionError::MissingEvolutionData {
                species_id: "FINAL_MON".to_string(),
            })
        );
    }

    #[test]
    fn evolution_data_is_exact_not_case_or_alias_coerced() {
        let species_map = [("RAICHU".to_string(), species("RAICHU", 60, 60, 60))]
            .into_iter()
            .collect();
        let moves = BTreeMap::new();
        let learnsets = SpeciesLearnsets::new();
        let table = EvolutionTable(
            [(
                "PIKACHU".to_string(),
                vec![EvolutionEntry::item("RAICHU", "THUNDERSTONE")],
            )]
            .into_iter()
            .collect(),
        );
        let mut context = context(&species_map, &moves, &learnsets);
        context.current_item = Some("thunderstone");
        context.force_evolution = true;

        assert_eq!(
            find_evolution_candidate(&pokemon("PIKACHU", 20), &table, &context).expect("candidate"),
            None
        );
    }

    fn evolution_entry_json() -> serde_json::Value {
        serde_json::json!({
            "method":"LEVEL",
            "species":"IVYSAUR",
            "level":16,
            "item":null,
            "held_item":null,
            "happiness":null,
            "stat_ratio":null
        })
    }

    #[test]
    fn evolution_json_requires_explicit_nullable_method_fields() {
        let mut missing_item = evolution_entry_json();
        missing_item
            .as_object_mut()
            .expect("evolution object")
            .remove("item");
        let error = serde_json::from_value::<EvolutionEntry>(missing_item)
            .expect_err("missing item must not deserialize as None")
            .to_string();
        assert!(error.contains("missing field `item`"), "{error}");

        let mut missing_level = evolution_entry_json();
        missing_level
            .as_object_mut()
            .expect("evolution object")
            .remove("level");
        let error = serde_json::from_value::<EvolutionEntry>(missing_level)
            .expect_err("missing level must not deserialize as None")
            .to_string();
        assert!(error.contains("missing field `level`"), "{error}");

        let table_error = serde_json::from_str::<EvolutionTable>(
            r#"{"entries":{"CHIKORITA":[]},"fallback_entries":{}}"#,
        )
        .expect_err("evolution tables must be the compiler-emitted species map")
        .to_string();
        assert!(
            table_error.contains("invalid type") || table_error.contains("invalid value"),
            "{table_error}"
        );
    }

    #[test]
    fn evolution_table_json_rejects_malformed_pack_tokens() {
        for (field, value, expected) in [
            (
                "source",
                serde_json::json!("fallback_bulbasaur"),
                "evolution source species",
            ),
            (
                "method",
                serde_json::json!("legacy_level"),
                "BULBASAUR[0].method",
            ),
            (
                "species",
                serde_json::json!("fallback_ivysaur"),
                "BULBASAUR[0].species",
            ),
            (
                "item",
                serde_json::json!("legacy_leaf_stone"),
                "BULBASAUR[0].item",
            ),
            (
                "held_item",
                serde_json::json!("fallback_trade_item"),
                "BULBASAUR[0].held_item",
            ),
            (
                "happiness",
                serde_json::json!("legacy_happiness"),
                "BULBASAUR[0].happiness",
            ),
            (
                "stat_ratio",
                serde_json::json!("fallback_stat_ratio"),
                "BULBASAUR[0].stat_ratio",
            ),
        ] {
            let entry = evolution_entry_json();
            let payload = if field == "source" {
                let mut payload = serde_json::Map::new();
                payload.insert(
                    value.as_str().expect("source string").to_string(),
                    serde_json::json!([entry]),
                );
                serde_json::Value::Object(payload)
            } else {
                let mut entry = entry;
                entry[field] = value;
                serde_json::json!({ "BULBASAUR": [entry] })
            };
            let error = serde_json::from_value::<EvolutionTable>(payload)
                .expect_err("malformed evolution tokens must fail during JSON load")
                .to_string();
            assert!(
                error.contains(expected),
                "{field} produced unexpected error: {error}"
            );
        }
    }

    #[test]
    fn evolution_vocabularies_are_exact_pack_values() {
        assert_eq!(
            EVOLUTION_METHODS,
            &[
                METHOD_LEVEL,
                METHOD_ITEM,
                METHOD_HAPPINESS,
                METHOD_TRADE,
                METHOD_STAT
            ]
        );
        assert!(is_known_evolution_method(METHOD_LEVEL));
        assert!(!is_known_evolution_method("level"));

        assert_eq!(
            HAPPINESS_WINDOWS,
            &[HAPPINESS_ANYTIME, HAPPINESS_MORNDAY, HAPPINESS_NITE]
        );
        assert!(is_known_happiness_window(HAPPINESS_MORNDAY));
        assert!(!is_known_happiness_window("MORNING"));

        assert_eq!(
            STAT_EVOLUTION_RATIOS,
            &[STAT_ATK_GT_DEF, STAT_ATK_LT_DEF, STAT_ATK_EQ_DEF]
        );
        assert!(is_known_stat_evolution_ratio(STAT_ATK_EQ_DEF));
        assert!(!is_known_stat_evolution_ratio("ATTACKIER"));
    }

    #[test]
    fn unknown_evolution_facts_are_errors_not_fallbacks() {
        let species_map = BTreeMap::new();
        let moves = BTreeMap::new();
        let learnsets = SpeciesLearnsets::new();
        let context = context(&species_map, &moves, &learnsets);
        let mut pokemon = pokemon("EEVEE", 20);
        pokemon.happiness = HAPPINESS_TO_EVOLVE;

        let bad_window = EvolutionTable(
            [(
                "EEVEE".to_string(),
                vec![EvolutionEntry::happiness("ESPEON", "MORNINGISH")],
            )]
            .into_iter()
            .collect(),
        );
        assert_eq!(
            find_evolution_candidate(&pokemon, &bad_window, &context),
            Err(EvolutionError::UnknownHappinessWindow {
                species_id: "EEVEE".to_string(),
                window: "MORNINGISH".to_string(),
            })
        );

        let malformed_window = EvolutionTable(
            [(
                "EEVEE".to_string(),
                vec![EvolutionEntry::happiness("ESPEON", "MORN DAY")],
            )]
            .into_iter()
            .collect(),
        );
        assert_eq!(
            find_evolution_candidate(&pokemon, &malformed_window, &context),
            Err(EvolutionError::InvalidHappinessWindow {
                species_id: "EEVEE".to_string(),
                window: "MORN DAY".to_string(),
            })
        );

        let bad_method = EvolutionTable(
            [(
                "EEVEE".to_string(),
                vec![EvolutionEntry {
                    method: "MOON_PHASE".to_string(),
                    species: "UMBREON".to_string(),
                    level: None,
                    item: None,
                    held_item: None,
                    happiness: None,
                    stat_ratio: None,
                }],
            )]
            .into_iter()
            .collect(),
        );
        assert_eq!(
            find_evolution_candidate(&pokemon, &bad_method, &context),
            Err(EvolutionError::UnknownMethod {
                species_id: "EEVEE".to_string(),
                method: "MOON_PHASE".to_string(),
            })
        );

        let malformed_method = EvolutionTable(
            [(
                "EEVEE".to_string(),
                vec![EvolutionEntry {
                    method: "MOON PHASE".to_string(),
                    species: "UMBREON".to_string(),
                    level: None,
                    item: None,
                    held_item: None,
                    happiness: None,
                    stat_ratio: None,
                }],
            )]
            .into_iter()
            .collect(),
        );
        assert_eq!(
            find_evolution_candidate(&pokemon, &malformed_method, &context),
            Err(EvolutionError::InvalidMethod {
                species_id: "EEVEE".to_string(),
                method: "MOON PHASE".to_string(),
            })
        );
    }

    #[test]
    fn malformed_evolution_requirements_are_errors_not_defaulted() {
        let species_map = BTreeMap::new();
        let moves = BTreeMap::new();
        let learnsets = SpeciesLearnsets::new();
        let context = context(&species_map, &moves, &learnsets);

        let missing_level = EvolutionTable(
            [(
                "BULBASAUR".to_string(),
                vec![EvolutionEntry {
                    method: METHOD_LEVEL.to_string(),
                    species: "IVYSAUR".to_string(),
                    level: None,
                    item: None,
                    held_item: None,
                    happiness: None,
                    stat_ratio: None,
                }],
            )]
            .into_iter()
            .collect(),
        );
        assert_eq!(
            find_evolution_candidate(&pokemon("BULBASAUR", 16), &missing_level, &context),
            Err(EvolutionError::MissingRequiredLevel {
                species_id: "BULBASAUR".to_string(),
            })
        );

        let mut eevee = pokemon("EEVEE", 20);
        eevee.happiness = HAPPINESS_TO_EVOLVE;
        let missing_window = EvolutionTable(
            [(
                "EEVEE".to_string(),
                vec![EvolutionEntry {
                    method: METHOD_HAPPINESS.to_string(),
                    species: "ESPEON".to_string(),
                    level: None,
                    item: None,
                    held_item: None,
                    happiness: None,
                    stat_ratio: None,
                }],
            )]
            .into_iter()
            .collect(),
        );
        assert_eq!(
            find_evolution_candidate(&eevee, &missing_window, &context),
            Err(EvolutionError::MissingHappinessWindow {
                species_id: "EEVEE".to_string(),
            })
        );

        let malformed_item = EvolutionTable(
            [(
                "BULBASAUR".to_string(),
                vec![EvolutionEntry::item("IVYSAUR", "LEAF STONE")],
            )]
            .into_iter()
            .collect(),
        );
        let item_context = EvolutionContext {
            species: context.species,
            moves: context.moves,
            learnsets: context.learnsets,
            time_of_day: context.time_of_day,
            current_item: Some("LEAF_STONE"),
            force_evolution: true,
            link_mode: context.link_mode,
        };
        assert_eq!(
            find_evolution_candidate(&pokemon("BULBASAUR", 16), &malformed_item, &item_context,),
            Err(EvolutionError::InvalidItem {
                species_id: "BULBASAUR".to_string(),
                item_id: "LEAF STONE".to_string(),
            })
        );

        let missing_stat_level = EvolutionTable(
            [(
                "TYROGUE".to_string(),
                vec![EvolutionEntry {
                    method: METHOD_STAT.to_string(),
                    species: "HITMONLEE".to_string(),
                    level: None,
                    item: None,
                    held_item: None,
                    happiness: None,
                    stat_ratio: Some(STAT_ATK_GT_DEF.to_string()),
                }],
            )]
            .into_iter()
            .collect(),
        );
        assert_eq!(
            find_evolution_candidate(&pokemon("TYROGUE", 20), &missing_stat_level, &context),
            Err(EvolutionError::MissingRequiredLevel {
                species_id: "TYROGUE".to_string(),
            })
        );

        let missing_ratio = EvolutionTable(
            [(
                "TYROGUE".to_string(),
                vec![EvolutionEntry {
                    method: METHOD_STAT.to_string(),
                    species: "HITMONLEE".to_string(),
                    level: Some(20),
                    item: None,
                    held_item: None,
                    happiness: None,
                    stat_ratio: None,
                }],
            )]
            .into_iter()
            .collect(),
        );
        assert_eq!(
            find_evolution_candidate(&pokemon("TYROGUE", 20), &missing_ratio, &context),
            Err(EvolutionError::MissingStatRatio {
                species_id: "TYROGUE".to_string(),
            })
        );

        let malformed_stat_ratio = EvolutionTable(
            [(
                "TYROGUE".to_string(),
                vec![EvolutionEntry {
                    method: METHOD_STAT.to_string(),
                    species: "HITMONLEE".to_string(),
                    level: Some(20),
                    item: None,
                    held_item: None,
                    happiness: None,
                    stat_ratio: Some("ATK GT_DEF".to_string()),
                }],
            )]
            .into_iter()
            .collect(),
        );
        assert_eq!(
            find_evolution_candidate(&pokemon("TYROGUE", 20), &malformed_stat_ratio, &context),
            Err(EvolutionError::InvalidStatRatio {
                species_id: "TYROGUE".to_string(),
                ratio: "ATK GT_DEF".to_string(),
            })
        );
    }
}
