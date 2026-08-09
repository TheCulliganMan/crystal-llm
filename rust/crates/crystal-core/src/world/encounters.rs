use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use thiserror::Error;

use crate::random::{CrystalRandom, CrystalRandomState, DividerSource, Random};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum EncounterSurface {
    Grass,
    Water,
    Rock,
}

impl EncounterSurface {
    pub const fn as_key(self) -> &'static str {
        match self {
            Self::Grass => "grass",
            Self::Water => "water",
            Self::Rock => "rock",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum TimeOfDay {
    Morning,
    Day,
    Night,
}

pub const ENCOUNTER_TIME_KEYS: &[&str] = &["morning", "day", "night"];

impl TimeOfDay {
    pub fn as_key(self) -> &'static str {
        match self {
            Self::Morning => "morning",
            Self::Day => "day",
            Self::Night => "night",
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EncounterError {
    #[error("unknown wild encounter time of day '{0}'")]
    UnknownTimeOfDay(String),
    #[error("wild encounter data for map '{map_name}' is missing {surface:?} rate")]
    MissingEncounterRate {
        map_name: String,
        surface: EncounterSurface,
    },
    #[error(
        "wild encounter data for map '{map_name}' is missing {surface:?} rate for time {time:?}"
    )]
    MissingTimedEncounterRate {
        map_name: String,
        surface: EncounterSurface,
        time: TimeOfDay,
    },
    #[error("wild encounter data for map '{map_name}' is missing {surface:?} table")]
    MissingEncounterTable {
        map_name: String,
        surface: EncounterSurface,
    },
    #[error("wild encounter table for map '{map_name}' has no {surface:?} slots at {time:?}")]
    EmptyEncounterSlots {
        map_name: String,
        surface: EncounterSurface,
        time: TimeOfDay,
    },
    #[error("wild encounter table for map '{map_name}' selected invalid species '{species}'")]
    InvalidEncounterSpecies { map_name: String, species: String },
    #[error("encounter roll {roll} did not resolve for {surface:?} with {slot_count} slots")]
    UnresolvedSlot {
        surface: EncounterSurface,
        slot_count: usize,
        roll: u8,
    },
    #[error("encounter percent roll must be between 1 and 100, got {0}")]
    InvalidPercentRoll(u8),
    #[error("encounter slot table for {surface:?} is missing from the modpack")]
    MissingEncounterSlotTable { surface: EncounterSurface },
    #[error("encounter music modifier for '{music_id}' has invalid denominator 0")]
    InvalidEncounterMusicModifier { music_id: String },
    #[error(
        "encounter music modifier for '{music_id}' overflows encounter threshold byte: {threshold} * {numerator} / {denominator} = {adjusted}"
    )]
    EncounterMusicModifierOverflow {
        music_id: String,
        threshold: u8,
        numerator: u8,
        denominator: u8,
        adjusted: u16,
    },
    #[error("map {map_name} runtime tile bounds overflow supported encounter coordinates")]
    RuntimeTileBoundsOverflow { map_name: String },
    #[error(
        "encounter runtime tile ({x}, {y}) is outside map {map_name} runtime tile bounds {width}x{height}"
    )]
    RuntimeTileOutOfBounds {
        map_name: String,
        x: i16,
        y: i16,
        width: u16,
        height: u16,
    },
    #[error("encounter runtime tile ({x}, {y}) is not aligned to metatile width {metatile_width}")]
    UnalignedRuntimeTile { x: i16, y: i16, metatile_width: i16 },
    #[error("encounter runtime tile ({x}, {y}) has no collision data on map {map_name}")]
    MissingRuntimeCollision { map_name: String, x: i16, y: i16 },
    #[error("{kind:?} field encounter table for map '{map_name}' is missing from the modpack")]
    MissingFieldEncounterTable {
        map_name: String,
        kind: FieldEncounterKind,
    },
    #[error(
        "{kind:?} field encounter table for map '{map_name}' has no entries in bucket '{bucket}'"
    )]
    EmptyFieldEncounterEntries {
        map_name: String,
        kind: FieldEncounterKind,
        bucket: &'static str,
    },
    #[error(
        "{kind:?} field encounter table for map '{map_name}' selected invalid species '{species}'"
    )]
    InvalidFieldEncounterSpecies {
        map_name: String,
        kind: FieldEncounterKind,
        species: String,
    },
    #[error("active repel item '{item_id}' requires an explicit lead party level")]
    ActiveRepelMissingLeadLevel { item_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WildEncounter {
    pub level: u8,
    pub species: String,
}

impl<'de> Deserialize<'de> for WildEncounter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawWildEncounter {
            level: u8,
            #[serde(deserialize_with = "required_encounter_token")]
            species: String,
        }

        let raw = RawWildEncounter::deserialize(deserializer)?;
        if raw.level == 0 {
            return Err(D::Error::custom(format!(
                "wild encounter species {} has level 0",
                raw.species
            )));
        }
        Ok(Self {
            level: raw.level,
            species: raw.species,
        })
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WildEncounterTable {
    pub morning: Vec<WildEncounter>,
    pub day: Vec<WildEncounter>,
    pub night: Vec<WildEncounter>,
}

impl<'de> Deserialize<'de> for WildEncounterTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawWildEncounterTable {
            morning: Vec<WildEncounter>,
            day: Vec<WildEncounter>,
            night: Vec<WildEncounter>,
        }

        let raw = RawWildEncounterTable::deserialize(deserializer)?;
        Ok(Self {
            morning: raw.morning,
            day: raw.day,
            night: raw.night,
        })
    }
}

impl WildEncounterTable {
    pub fn slots(&self, time: TimeOfDay) -> &[WildEncounter] {
        match time {
            TimeOfDay::Morning => &self.morning,
            TimeOfDay::Day => &self.day,
            TimeOfDay::Night => &self.night,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct WildEncounterData {
    pub map_name: String,
    pub grass_rates: Option<BTreeMap<String, u8>>,
    pub water_rate: Option<u8>,
    pub grass: Option<WildEncounterTable>,
    pub water: Option<WildEncounterTable>,
}

impl<'de> Deserialize<'de> for WildEncounterData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        fn required_nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
        where
            D: serde::Deserializer<'de>,
            T: Deserialize<'de>,
        {
            Option::<T>::deserialize(deserializer)
        }

        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawWildEncounterData {
            #[serde(deserialize_with = "required_encounter_token")]
            map_name: String,
            #[serde(deserialize_with = "required_nullable_grass_rates")]
            grass_rates: Option<BTreeMap<String, u8>>,
            #[serde(deserialize_with = "required_nullable")]
            water_rate: Option<u8>,
            #[serde(deserialize_with = "required_nullable")]
            grass: Option<WildEncounterTable>,
            #[serde(deserialize_with = "required_nullable")]
            water: Option<WildEncounterTable>,
        }

        let raw = RawWildEncounterData::deserialize(deserializer)?;
        Ok(Self {
            map_name: raw.map_name,
            grass_rates: raw.grass_rates,
            water_rate: raw.water_rate,
            grass: raw.grass,
            water: raw.water,
        })
    }
}

fn required_encounter_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_nonempty_encounter_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "encounter token must be exact ASCII alphanumeric/underscore, found {value:?}"
        )))
    }
}

fn required_nullable_grass_rates<'de, D>(
    deserializer: D,
) -> Result<Option<BTreeMap<String, u8>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<BTreeMap<String, u8>>::deserialize(deserializer)?;
    if let Some(rates) = &value {
        if let Some(key) = rates
            .keys()
            .find(|key| !is_exact_nonempty_encounter_token(key))
        {
            return Err(serde::de::Error::custom(format!(
                "encounter token must be exact ASCII alphanumeric/underscore, found {key:?}"
            )));
        }
    }
    Ok(value)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedWildEncounter {
    pub encounter: WildEncounter,
    pub slot: usize,
    pub level: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncounterSlotChance {
    pub threshold: u8,
    pub slot: usize,
}

impl<'de> Deserialize<'de> for EncounterSlotChance {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct RawEncounterSlotChance {
            threshold: u8,
            slot: usize,
        }

        let raw = RawEncounterSlotChance::deserialize(deserializer)?;
        if raw.threshold == 0 || raw.threshold > 100 {
            return Err(D::Error::custom(format!(
                "encounter slot threshold {} is outside 1..100",
                raw.threshold
            )));
        }
        Ok(Self {
            threshold: raw.threshold,
            slot: raw.slot,
        })
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncounterSlotTables {
    pub tables: BTreeMap<String, Vec<EncounterSlotChance>>,
}

impl<'de> Deserialize<'de> for EncounterSlotTables {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct RawEncounterSlotTables {
            tables: BTreeMap<String, Vec<EncounterSlotChance>>,
        }

        let raw = RawEncounterSlotTables::deserialize(deserializer)?;
        let tables = raw.tables;
        for surface_id in tables.keys() {
            if !is_exact_nonempty_encounter_token(surface_id) {
                return Err(serde::de::Error::custom(format!(
                    "encounter token must be exact ASCII alphanumeric/underscore, found {surface_id:?}"
                )));
            }
        }
        let tables = Self { tables };
        if tables != Self::default() {
            let issues = encounter_slot_table_issues(&tables, true);
            if let Some(issue) = issues.first() {
                return Err(serde::de::Error::custom(format!(
                    "invalid encounter slot tables: {issue:?}"
                )));
            }
        }
        Ok(tables)
    }
}

impl EncounterSlotTables {
    pub fn for_crystal(grass: Vec<EncounterSlotChance>, water: Vec<EncounterSlotChance>) -> Self {
        Self {
            tables: BTreeMap::from([
                (EncounterSurface::Grass.as_key().to_string(), grass),
                (EncounterSurface::Water.as_key().to_string(), water),
            ]),
        }
    }

    pub fn table_for_surface(
        &self,
        surface: EncounterSurface,
    ) -> Result<&[EncounterSlotChance], EncounterError> {
        let table = self
            .tables
            .get(surface.as_key())
            .ok_or(EncounterError::MissingEncounterSlotTable { surface })?;
        if table.is_empty() {
            return Err(EncounterError::MissingEncounterSlotTable { surface });
        }
        Ok(table)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncounterSlotTableIssue {
    InvalidSurfaceId {
        surface_id: String,
    },
    MissingTable {
        surface: EncounterSurface,
    },
    InvalidThreshold {
        surface: EncounterSurface,
        threshold: u8,
    },
    UnorderedThreshold {
        surface: EncounterSurface,
        threshold: u8,
        previous: u8,
    },
    DuplicateSlotIndex {
        surface: EncounterSurface,
        slot: usize,
    },
    IncompleteTable {
        surface: EncounterSurface,
    },
    InvalidCustomThreshold {
        surface_id: String,
        threshold: u8,
    },
    UnorderedCustomThreshold {
        surface_id: String,
        threshold: u8,
        previous: u8,
    },
    DuplicateCustomSlotIndex {
        surface_id: String,
        slot: usize,
    },
    EmptyCustomTable {
        surface_id: String,
    },
    IncompleteCustomTable {
        surface_id: String,
    },
}

impl EncounterSlotTableIssue {
    pub fn surface(&self) -> Option<EncounterSurface> {
        match self {
            Self::MissingTable { surface }
            | Self::InvalidThreshold { surface, .. }
            | Self::UnorderedThreshold { surface, .. }
            | Self::DuplicateSlotIndex { surface, .. }
            | Self::IncompleteTable { surface } => Some(*surface),
            Self::InvalidSurfaceId { .. }
            | Self::InvalidCustomThreshold { .. }
            | Self::UnorderedCustomThreshold { .. }
            | Self::DuplicateCustomSlotIndex { .. }
            | Self::EmptyCustomTable { .. }
            | Self::IncompleteCustomTable { .. } => None,
        }
    }
}

pub fn encounter_slot_table_issues(
    tables: &EncounterSlotTables,
    required: bool,
) -> Vec<EncounterSlotTableIssue> {
    if !required {
        return Vec::new();
    }
    let mut issues = Vec::new();
    push_required_encounter_slot_table_issues(EncounterSurface::Grass, tables, &mut issues);
    push_required_encounter_slot_table_issues(EncounterSurface::Water, tables, &mut issues);
    for (surface_id, table) in &tables.tables {
        if surface_id == EncounterSurface::Grass.as_key()
            || surface_id == EncounterSurface::Water.as_key()
        {
            continue;
        }
        if !is_exact_nonempty_encounter_token(surface_id) {
            issues.push(EncounterSlotTableIssue::InvalidSurfaceId {
                surface_id: surface_id.clone(),
            });
            continue;
        }
        push_custom_encounter_slot_table_issues(surface_id, table, &mut issues);
    }
    issues
}

fn push_required_encounter_slot_table_issues(
    surface: EncounterSurface,
    tables: &EncounterSlotTables,
    issues: &mut Vec<EncounterSlotTableIssue>,
) {
    let Some(table) = tables.tables.get(surface.as_key()) else {
        issues.push(EncounterSlotTableIssue::MissingTable { surface });
        return;
    };
    push_encounter_slot_table_issues(surface, table, issues);
}

fn push_encounter_slot_table_issues(
    surface: EncounterSurface,
    table: &[EncounterSlotChance],
    issues: &mut Vec<EncounterSlotTableIssue>,
) {
    if table.is_empty() {
        issues.push(EncounterSlotTableIssue::MissingTable { surface });
        return;
    }
    let mut previous_threshold = 0;
    let mut slots = BTreeSet::new();
    for entry in table {
        if entry.threshold == 0 || entry.threshold > 100 {
            issues.push(EncounterSlotTableIssue::InvalidThreshold {
                surface,
                threshold: entry.threshold,
            });
        }
        if entry.threshold < previous_threshold {
            issues.push(EncounterSlotTableIssue::UnorderedThreshold {
                surface,
                threshold: entry.threshold,
                previous: previous_threshold,
            });
        }
        previous_threshold = entry.threshold;
        if !slots.insert(entry.slot) {
            issues.push(EncounterSlotTableIssue::DuplicateSlotIndex {
                surface,
                slot: entry.slot,
            });
        }
    }
    if previous_threshold != 100 {
        issues.push(EncounterSlotTableIssue::IncompleteTable { surface });
    }
}

fn push_custom_encounter_slot_table_issues(
    surface_id: &str,
    table: &[EncounterSlotChance],
    issues: &mut Vec<EncounterSlotTableIssue>,
) {
    if table.is_empty() {
        issues.push(EncounterSlotTableIssue::EmptyCustomTable {
            surface_id: surface_id.to_string(),
        });
        return;
    }
    let mut previous_threshold = 0;
    let mut slots = BTreeSet::new();
    for entry in table {
        if entry.threshold == 0 || entry.threshold > 100 {
            issues.push(EncounterSlotTableIssue::InvalidCustomThreshold {
                surface_id: surface_id.to_string(),
                threshold: entry.threshold,
            });
        }
        if entry.threshold < previous_threshold {
            issues.push(EncounterSlotTableIssue::UnorderedCustomThreshold {
                surface_id: surface_id.to_string(),
                threshold: entry.threshold,
                previous: previous_threshold,
            });
        }
        previous_threshold = entry.threshold;
        if !slots.insert(entry.slot) {
            issues.push(EncounterSlotTableIssue::DuplicateCustomSlotIndex {
                surface_id: surface_id.to_string(),
                slot: entry.slot,
            });
        }
    }
    if previous_threshold != 100 {
        issues.push(EncounterSlotTableIssue::IncompleteCustomTable {
            surface_id: surface_id.to_string(),
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterMusicModifier {
    pub numerator: u8,
    pub denominator: u8,
}

impl<'de> Deserialize<'de> for EncounterMusicModifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawEncounterMusicModifier {
            numerator: u8,
            denominator: u8,
        }

        let raw = RawEncounterMusicModifier::deserialize(deserializer)?;
        if raw.denominator == 0 {
            return Err(D::Error::custom(
                "encounter music modifier denominator must be nonzero",
            ));
        }
        Ok(Self {
            numerator: raw.numerator,
            denominator: raw.denominator,
        })
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterMusicModifiers {
    pub modifiers: BTreeMap<String, EncounterMusicModifier>,
}

impl<'de> Deserialize<'de> for EncounterMusicModifiers {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawEncounterMusicModifiers {
            modifiers: BTreeMap<String, EncounterMusicModifier>,
        }

        let raw = RawEncounterMusicModifiers::deserialize(deserializer)?;
        for music_id in raw.modifiers.keys() {
            if !is_exact_nonempty_encounter_token(music_id) {
                return Err(serde::de::Error::custom(format!(
                    "encounter token must be exact ASCII alphanumeric/underscore, found {music_id:?}"
                )));
            }
        }
        Ok(Self {
            modifiers: raw.modifiers,
        })
    }
}

impl EncounterMusicModifiers {
    pub fn modifier_for(&self, music_id: &str) -> Option<&EncounterMusicModifier> {
        self.modifiers.get(music_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncounterMusicModifierIssue {
    MissingTable,
    MissingMusicId { music_id: String },
    InvalidMusicId { music_id: String },
    UnknownMusicId { music_id: String },
    InvalidRatio { music_id: String },
}

pub fn encounter_music_modifier_issues(
    modifiers: &EncounterMusicModifiers,
    music_ids: &BTreeSet<String>,
    required: bool,
) -> Vec<EncounterMusicModifierIssue> {
    if !required {
        return Vec::new();
    }
    if modifiers.modifiers.is_empty() {
        return vec![EncounterMusicModifierIssue::MissingTable];
    }
    let mut issues = Vec::new();
    for (music_id, modifier) in &modifiers.modifiers {
        if music_id.is_empty() {
            issues.push(EncounterMusicModifierIssue::MissingMusicId {
                music_id: music_id.clone(),
            });
        } else if !is_exact_nonempty_encounter_token(music_id) {
            issues.push(EncounterMusicModifierIssue::InvalidMusicId {
                music_id: music_id.clone(),
            });
        } else if !music_ids.contains(music_id) {
            issues.push(EncounterMusicModifierIssue::UnknownMusicId {
                music_id: music_id.clone(),
            });
        }
        if modifier.denominator == 0 {
            issues.push(EncounterMusicModifierIssue::InvalidRatio {
                music_id: music_id.clone(),
            });
        }
    }
    issues
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FieldEncounterEntry {
    pub weight: u8,
    pub species: String,
    pub level: u8,
    pub sleep_turns_by_time: BTreeMap<TimeOfDay, u8>,
}

impl<'de> Deserialize<'de> for FieldEncounterEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawFieldEncounterEntry {
            weight: u8,
            #[serde(deserialize_with = "required_encounter_token")]
            species: String,
            level: u8,
            sleep_turns_by_time: BTreeMap<TimeOfDay, u8>,
        }

        let raw = RawFieldEncounterEntry::deserialize(deserializer)?;
        if raw.weight == 0 {
            return Err(D::Error::custom(format!(
                "field encounter species {} has weight 0",
                raw.species
            )));
        }
        if raw.level == 0 {
            return Err(D::Error::custom(format!(
                "field encounter species {} has level 0",
                raw.species
            )));
        }
        if let Some((time, turns)) = raw
            .sleep_turns_by_time
            .iter()
            .find(|(_, turns)| **turns == 0 || **turns > 7)
        {
            return Err(D::Error::custom(format!(
                "field encounter species {} has invalid {:?} sleep counter {} (expected 1..=7)",
                raw.species, time, turns
            )));
        }
        Ok(Self {
            weight: raw.weight,
            species: raw.species,
            level: raw.level,
            sleep_turns_by_time: raw.sleep_turns_by_time,
        })
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FieldEncounterTable {
    pub common: Vec<FieldEncounterEntry>,
    pub rare: Vec<FieldEncounterEntry>,
}

impl<'de> Deserialize<'de> for FieldEncounterTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawFieldEncounterTable {
            common: Vec<FieldEncounterEntry>,
            rare: Vec<FieldEncounterEntry>,
        }

        let raw = RawFieldEncounterTable::deserialize(deserializer)?;
        if raw.common.is_empty() {
            return Err(D::Error::custom("field encounter common table is empty"));
        }
        Ok(Self {
            common: raw.common,
            rare: raw.rare,
        })
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FieldEncounterData {
    pub map_name: String,
    pub tables: BTreeMap<String, FieldEncounterTable>,
}

impl<'de> Deserialize<'de> for FieldEncounterData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawFieldEncounterData {
            #[serde(deserialize_with = "required_encounter_token")]
            map_name: String,
            tables: BTreeMap<String, FieldEncounterTable>,
        }

        let raw = RawFieldEncounterData::deserialize(deserializer)?;
        let tables = raw.tables;
        for kind in tables.keys() {
            if !is_exact_nonempty_encounter_token(kind) {
                return Err(serde::de::Error::custom(format!(
                    "encounter token must be exact ASCII alphanumeric/underscore, found {kind:?}"
                )));
            }
        }
        Ok(Self {
            map_name: raw.map_name,
            tables,
        })
    }
}

impl FieldEncounterData {
    pub fn for_crystal(
        map_name: impl Into<String>,
        headbutt: Option<FieldEncounterTable>,
        rock_smash: Option<FieldEncounterTable>,
    ) -> Self {
        let mut tables = BTreeMap::new();
        if let Some(headbutt) = headbutt {
            tables.insert(FieldEncounterKind::Headbutt.as_key().to_string(), headbutt);
        }
        if let Some(rock_smash) = rock_smash {
            tables.insert(
                FieldEncounterKind::RockSmash.as_key().to_string(),
                rock_smash,
            );
        }
        Self {
            map_name: map_name.into(),
            tables,
        }
    }

    pub fn table(&self, kind: FieldEncounterKind) -> Option<&FieldEncounterTable> {
        self.tables.get(kind.as_key())
    }

    pub fn table_mut(&mut self, kind: FieldEncounterKind) -> Option<&mut FieldEncounterTable> {
        self.tables.get_mut(kind.as_key())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FieldEncounterKind {
    Headbutt,
    RockSmash,
}

impl FieldEncounterKind {
    pub const fn as_key(self) -> &'static str {
        match self {
            Self::Headbutt => "headbutt",
            Self::RockSmash => "rock_smash",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldEncounterRoll {
    #[serde(deserialize_with = "required_encounter_token")]
    pub map_name: String,
    pub kind: FieldEncounterKind,
    pub target_tile_x: i16,
    pub target_tile_y: i16,
    pub score: Option<u8>,
    pub chance_roll: u8,
    pub entry_roll: Option<u8>,
    pub resolved: Option<ResolvedWildEncounter>,
}

/// The memory-independent result of Crystal's `RockMonEncounter` routine.
///
/// `chance_roll == None` means the current map had no valid rock encounter
/// table. That path consumes no divider samples. A present chance roll with no
/// entry roll is the ordinary 60% miss; the entry roll exists only after the
/// chance roll succeeds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RockMonEncounterOutcome {
    pub chance_roll: Option<u8>,
    pub entry_roll: Option<u8>,
    pub resolved: Option<ResolvedWildEncounter>,
    pub random_state_after: CrystalRandomState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RockMonEncounterError<E> {
    Encounter(EncounterError),
    Divider(E),
}

impl<E> From<EncounterError> for RockMonEncounterError<E> {
    fn from(error: EncounterError) -> Self {
        Self::Encounter(error)
    }
}

impl<E: std::fmt::Display> std::fmt::Display for RockMonEncounterError<E> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Encounter(error) => error.fmt(formatter),
            Self::Divider(error) => write!(formatter, "RockMonEncounter divider source: {error}"),
        }
    }
}

impl<E: std::fmt::Debug + std::fmt::Display> std::error::Error for RockMonEncounterError<E> {}

pub fn resolve_encounter_time_key(value: impl AsRef<str>) -> Result<TimeOfDay, EncounterError> {
    match value.as_ref() {
        "morning" => Ok(TimeOfDay::Morning),
        "day" => Ok(TimeOfDay::Day),
        "night" => Ok(TimeOfDay::Night),
        _ => Err(EncounterError::UnknownTimeOfDay(value.as_ref().to_string())),
    }
}

pub fn percent_to_byte(value: f64) -> u8 {
    if value <= 0.0 {
        return 0;
    }
    if value >= 100.0 {
        return 255;
    }
    ((value * 255.0) / 100.0).floor() as u8
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WildEncounterCatalogIssue {
    InvalidMap {
        map_name: String,
    },
    UnknownMap {
        map_name: String,
    },
    InvalidSpecies {
        map_name: String,
        species_id: String,
    },
    UnknownSpecies {
        map_name: String,
        species_id: String,
    },
    InvalidGrassRateTime {
        map_name: String,
        time_key: String,
    },
    UnknownGrassRateTime {
        map_name: String,
        time_key: String,
    },
    MissingGrassRate {
        map_name: String,
        time_key: &'static str,
    },
    EmptyGrassSlots {
        map_name: String,
        time_key: &'static str,
    },
    MissingGrassTable {
        map_name: String,
    },
    MissingWaterRate {
        map_name: String,
    },
    EmptyWaterSlots {
        map_name: String,
        time_key: &'static str,
    },
    MissingWaterTable {
        map_name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldEncounterCatalogIssue {
    InvalidMap {
        map_name: String,
    },
    UnknownMap {
        map_name: String,
    },
    InvalidSpecies {
        map_name: String,
        species_id: String,
    },
    UnknownSpecies {
        map_name: String,
        species_id: String,
    },
    InvalidKind {
        map_name: String,
        kind: String,
    },
    EmptyBucket {
        map_name: String,
        kind: String,
        bucket: &'static str,
    },
    ZeroWeight {
        map_name: String,
        kind: String,
        bucket: &'static str,
        entry_index: usize,
        species_id: String,
    },
    InvalidSleepTurns {
        map_name: String,
        kind: String,
        bucket: &'static str,
        entry_index: usize,
        species_id: String,
        time: TimeOfDay,
        sleep_turns: u8,
    },
    UnexpectedSleepRule {
        map_name: String,
        kind: String,
        bucket: &'static str,
        entry_index: usize,
        species_id: String,
        time: TimeOfDay,
    },
    InvalidWeightTotal {
        map_name: String,
        kind: String,
        bucket: &'static str,
        total_weight: u16,
    },
}

pub fn wild_encounter_catalog_issues(
    encounters: &BTreeMap<String, WildEncounterData>,
    map_ids: &BTreeSet<String>,
    species_ids: &BTreeSet<String>,
) -> Vec<WildEncounterCatalogIssue> {
    let mut issues = Vec::new();
    for (map_name, encounter_data) in encounters {
        if !is_exact_nonempty_encounter_token(map_name) {
            issues.push(WildEncounterCatalogIssue::InvalidMap {
                map_name: map_name.clone(),
            });
        } else if !map_ids.contains(map_name) {
            issues.push(WildEncounterCatalogIssue::UnknownMap {
                map_name: map_name.clone(),
            });
        }
        for species_id in wild_encounter_species(encounter_data) {
            if !is_exact_nonempty_encounter_token(&species_id) {
                issues.push(WildEncounterCatalogIssue::InvalidSpecies {
                    map_name: map_name.clone(),
                    species_id,
                });
            } else if !species_ids.contains(&species_id) {
                issues.push(WildEncounterCatalogIssue::UnknownSpecies {
                    map_name: map_name.clone(),
                    species_id,
                });
            }
        }
        push_wild_encounter_table_issues(map_name, encounter_data, &mut issues);
    }
    issues
}

pub fn field_encounter_catalog_issues(
    encounters: &BTreeMap<String, FieldEncounterData>,
    map_ids: &BTreeSet<String>,
    species_ids: &BTreeSet<String>,
) -> Vec<FieldEncounterCatalogIssue> {
    let mut issues = Vec::new();
    for (map_name, encounter_data) in encounters {
        if !is_exact_nonempty_encounter_token(map_name) {
            issues.push(FieldEncounterCatalogIssue::InvalidMap {
                map_name: map_name.clone(),
            });
        } else if !map_ids.contains(map_name) {
            issues.push(FieldEncounterCatalogIssue::UnknownMap {
                map_name: map_name.clone(),
            });
        }
        for species_id in field_encounter_species(encounter_data) {
            if !is_exact_nonempty_encounter_token(&species_id) {
                issues.push(FieldEncounterCatalogIssue::InvalidSpecies {
                    map_name: map_name.clone(),
                    species_id,
                });
            } else if !species_ids.contains(&species_id) {
                issues.push(FieldEncounterCatalogIssue::UnknownSpecies {
                    map_name: map_name.clone(),
                    species_id,
                });
            }
        }
        push_field_encounter_table_issues(map_name, encounter_data, &mut issues);
    }
    issues
}

fn wild_encounter_species(data: &WildEncounterData) -> BTreeSet<String> {
    let mut species = BTreeSet::new();
    for table in [data.grass.as_ref(), data.water.as_ref()]
        .into_iter()
        .flatten()
    {
        for encounter in table
            .morning
            .iter()
            .chain(table.day.iter())
            .chain(table.night.iter())
        {
            species.insert(encounter.species.clone());
        }
    }
    species
}

fn is_exact_nonempty_encounter_token(value: &str) -> bool {
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

fn field_encounter_species(data: &FieldEncounterData) -> BTreeSet<String> {
    let mut species = BTreeSet::new();
    for table in data.tables.values() {
        for encounter in table.common.iter().chain(table.rare.iter()) {
            species.insert(encounter.species.clone());
        }
    }
    species
}

fn push_wild_encounter_table_issues(
    map_name: &str,
    encounters: &WildEncounterData,
    issues: &mut Vec<WildEncounterCatalogIssue>,
) {
    if let Some(rates) = encounters.grass_rates.as_ref() {
        for time_key in rates.keys() {
            if !is_exact_nonempty_encounter_token(time_key) {
                issues.push(WildEncounterCatalogIssue::InvalidGrassRateTime {
                    map_name: map_name.to_string(),
                    time_key: time_key.clone(),
                });
            } else if !ENCOUNTER_TIME_KEYS.contains(&time_key.as_str()) {
                issues.push(WildEncounterCatalogIssue::UnknownGrassRateTime {
                    map_name: map_name.to_string(),
                    time_key: time_key.clone(),
                });
            }
        }
    }

    if let Some(grass) = encounters.grass.as_ref() {
        for time_key in ENCOUNTER_TIME_KEYS {
            let time_of_day =
                resolve_encounter_time_key(time_key).expect("core encounter time key must resolve");
            let rate = encounters
                .grass_rates
                .as_ref()
                .and_then(|rates| rates.get(*time_key))
                .copied();
            if rate.is_none() {
                issues.push(WildEncounterCatalogIssue::MissingGrassRate {
                    map_name: map_name.to_string(),
                    time_key,
                });
            }
            if rate.is_some_and(|rate| rate > 0) && grass.slots(time_of_day).is_empty() {
                issues.push(WildEncounterCatalogIssue::EmptyGrassSlots {
                    map_name: map_name.to_string(),
                    time_key,
                });
            }
        }
    } else if encounters
        .grass_rates
        .as_ref()
        .is_some_and(|rates| rates.values().any(|rate| *rate > 0))
    {
        issues.push(WildEncounterCatalogIssue::MissingGrassTable {
            map_name: map_name.to_string(),
        });
    }

    if let Some(water) = encounters.water.as_ref() {
        if encounters.water_rate.is_none() {
            issues.push(WildEncounterCatalogIssue::MissingWaterRate {
                map_name: map_name.to_string(),
            });
        }
        if encounters.water_rate.is_some_and(|rate| rate > 0) {
            for time_key in ENCOUNTER_TIME_KEYS {
                let time_of_day = resolve_encounter_time_key(time_key)
                    .expect("core encounter time key must resolve");
                if water.slots(time_of_day).is_empty() {
                    issues.push(WildEncounterCatalogIssue::EmptyWaterSlots {
                        map_name: map_name.to_string(),
                        time_key,
                    });
                }
            }
        }
    } else if encounters.water_rate.is_some_and(|rate| rate > 0) {
        issues.push(WildEncounterCatalogIssue::MissingWaterTable {
            map_name: map_name.to_string(),
        });
    }
}

fn push_field_encounter_table_issues(
    map_name: &str,
    encounters: &FieldEncounterData,
    issues: &mut Vec<FieldEncounterCatalogIssue>,
) {
    for kind in encounters.tables.keys() {
        if !is_exact_nonempty_encounter_token(kind)
            || (kind != FieldEncounterKind::Headbutt.as_key()
                && kind != FieldEncounterKind::RockSmash.as_key())
        {
            issues.push(FieldEncounterCatalogIssue::InvalidKind {
                map_name: map_name.to_string(),
                kind: kind.clone(),
            });
        }
    }

    if let Some(headbutt) = encounters.table(FieldEncounterKind::Headbutt) {
        push_field_encounter_bucket_issues(
            map_name,
            FieldEncounterKind::Headbutt.as_key(),
            "common",
            &headbutt.common,
            issues,
        );
        push_field_encounter_bucket_issues(
            map_name,
            FieldEncounterKind::Headbutt.as_key(),
            "rare",
            &headbutt.rare,
            issues,
        );
    }
    if let Some(rock_smash) = encounters.table(FieldEncounterKind::RockSmash) {
        push_field_encounter_bucket_issues(
            map_name,
            FieldEncounterKind::RockSmash.as_key(),
            "common",
            &rock_smash.common,
            issues,
        );
    }
}

fn push_field_encounter_bucket_issues(
    map_name: &str,
    kind: &'static str,
    bucket: &'static str,
    entries: &[FieldEncounterEntry],
    issues: &mut Vec<FieldEncounterCatalogIssue>,
) {
    if entries.is_empty() {
        issues.push(FieldEncounterCatalogIssue::EmptyBucket {
            map_name: map_name.to_string(),
            kind: kind.to_string(),
            bucket,
        });
        return;
    }

    for (entry_index, entry) in entries.iter().enumerate() {
        if entry.weight == 0 {
            issues.push(FieldEncounterCatalogIssue::ZeroWeight {
                map_name: map_name.to_string(),
                kind: kind.to_string(),
                bucket,
                entry_index,
                species_id: entry.species.clone(),
            });
        }
        for (&time, &sleep_turns) in &entry.sleep_turns_by_time {
            if !(1..=7).contains(&sleep_turns) {
                issues.push(FieldEncounterCatalogIssue::InvalidSleepTurns {
                    map_name: map_name.to_string(),
                    kind: kind.to_string(),
                    bucket,
                    entry_index,
                    species_id: entry.species.clone(),
                    time,
                    sleep_turns,
                });
            }
            if kind != FieldEncounterKind::Headbutt.as_key() {
                issues.push(FieldEncounterCatalogIssue::UnexpectedSleepRule {
                    map_name: map_name.to_string(),
                    kind: kind.to_string(),
                    bucket,
                    entry_index,
                    species_id: entry.species.clone(),
                    time,
                });
            }
        }
    }

    let total_weight: u16 = entries.iter().map(|entry| u16::from(entry.weight)).sum();
    if total_weight != 100 {
        issues.push(FieldEncounterCatalogIssue::InvalidWeightTotal {
            map_name: map_name.to_string(),
            kind: kind.to_string(),
            bucket,
            total_weight,
        });
    }
}

pub fn table_for_surface(
    data: &WildEncounterData,
    surface: EncounterSurface,
    time: TimeOfDay,
) -> Result<&[WildEncounter], EncounterError> {
    let table = match surface {
        EncounterSurface::Water => data.water.as_ref(),
        EncounterSurface::Grass | EncounterSurface::Rock => data.grass.as_ref(),
    };
    table
        .map(|slots| slots.slots(time))
        .ok_or_else(|| EncounterError::MissingEncounterTable {
            map_name: data.map_name.clone(),
            surface,
        })
}

pub fn base_encounter_rate(
    data: &WildEncounterData,
    surface: EncounterSurface,
    time: TimeOfDay,
) -> Result<u8, EncounterError> {
    match surface {
        EncounterSurface::Water => {
            data.water_rate
                .ok_or_else(|| EncounterError::MissingEncounterRate {
                    map_name: data.map_name.clone(),
                    surface,
                })
        }
        EncounterSurface::Grass | EncounterSurface::Rock => {
            let rates =
                data.grass_rates
                    .as_ref()
                    .ok_or_else(|| EncounterError::MissingEncounterRate {
                        map_name: data.map_name.clone(),
                        surface,
                    })?;
            rates.get(time.as_key()).copied().ok_or_else(|| {
                EncounterError::MissingTimedEncounterRate {
                    map_name: data.map_name.clone(),
                    surface,
                    time,
                }
            })
        }
    }
}

pub fn apply_encounter_music_effect(
    threshold: u8,
    music_token: Option<&str>,
    modifiers: &EncounterMusicModifiers,
) -> Result<u8, EncounterError> {
    let Some(music_id) = music_token else {
        return Ok(threshold);
    };
    let Some(modifier) = modifiers.modifier_for(music_id) else {
        return Ok(threshold);
    };
    if modifier.denominator == 0 {
        return Err(EncounterError::InvalidEncounterMusicModifier {
            music_id: music_id.to_string(),
        });
    }
    let adjusted =
        (u16::from(threshold) * u16::from(modifier.numerator)) / u16::from(modifier.denominator);
    u8::try_from(adjusted).map_err(|_| EncounterError::EncounterMusicModifierOverflow {
        music_id: music_id.to_string(),
        threshold,
        numerator: modifier.numerator,
        denominator: modifier.denominator,
        adjusted,
    })
}

pub fn apply_cleanse_tag_effect(threshold: u8, has_cleanse_tag: bool) -> u8 {
    if has_cleanse_tag {
        threshold >> 1
    } else {
        threshold
    }
}

pub fn encounter_threshold(
    data: &WildEncounterData,
    surface: EncounterSurface,
    time: TimeOfDay,
    music_token: Option<&str>,
    music_modifiers: &EncounterMusicModifiers,
    has_cleanse_tag: bool,
) -> Result<u8, EncounterError> {
    let threshold = percent_to_byte(f64::from(base_encounter_rate(data, surface, time)?));
    let threshold = apply_encounter_music_effect(threshold, music_token, music_modifiers)?;
    Ok(apply_cleanse_tag_effect(threshold, has_cleanse_tag))
}

pub fn passes_encounter_roll(threshold: u8, roll_byte: u8) -> bool {
    threshold > 0 && roll_byte < threshold
}

pub fn random_percent_from_bytes<I>(bytes: I) -> Option<u8>
where
    I: IntoIterator<Item = u8>,
{
    bytes
        .into_iter()
        .find(|value| *value < 100)
        .map(|value| value + 1)
}

pub fn choose_slot_from_percent(
    slot_tables: &EncounterSlotTables,
    surface: EncounterSurface,
    slot_count: usize,
    roll_percent: u8,
) -> Result<usize, EncounterError> {
    if !(1..=100).contains(&roll_percent) {
        return Err(EncounterError::InvalidPercentRoll(roll_percent));
    }
    for entry in slot_tables.table_for_surface(surface)? {
        if roll_percent <= entry.threshold && entry.slot < slot_count {
            return Ok(entry.slot);
        }
    }
    Err(EncounterError::UnresolvedSlot {
        surface,
        slot_count,
        roll: roll_percent,
    })
}

pub fn apply_surf_level_variance(base_level: u8, surface: EncounterSurface, roll_byte: u8) -> u8 {
    if surface != EncounterSurface::Water {
        return base_level;
    }
    let mut extra = 0;
    for threshold in [35.0, 65.0, 85.0, 95.0] {
        if roll_byte < percent_to_byte(threshold) {
            break;
        }
        extra += 1;
    }
    base_level.saturating_add(extra)
}

pub fn select_wild_encounter(
    data: &WildEncounterData,
    slot_tables: &EncounterSlotTables,
    surface: EncounterSurface,
    time: TimeOfDay,
    slot_percent_roll: u8,
    level_roll_byte: u8,
) -> Result<Option<ResolvedWildEncounter>, EncounterError> {
    let table = table_for_surface(data, surface, time)?;
    if table.is_empty() {
        return Err(EncounterError::EmptyEncounterSlots {
            map_name: data.map_name.clone(),
            surface,
            time,
        });
    }
    let slot = choose_slot_from_percent(slot_tables, surface, table.len(), slot_percent_roll)?;
    let encounter = table[slot].clone();
    if !is_exact_nonempty_encounter_token(&encounter.species) {
        return Err(EncounterError::InvalidEncounterSpecies {
            map_name: data.map_name.clone(),
            species: encounter.species,
        });
    }
    let level = apply_surf_level_variance(encounter.level, surface, level_roll_byte);
    Ok(Some(ResolvedWildEncounter {
        encounter,
        slot,
        level,
    }))
}

pub fn require_encounter_table_for_surface(
    data: &WildEncounterData,
    surface: EncounterSurface,
    time: TimeOfDay,
) -> Result<(), EncounterError> {
    let table = table_for_surface(data, surface, time)?;
    if table.is_empty() {
        return Err(EncounterError::EmptyEncounterSlots {
            map_name: data.map_name.clone(),
            surface,
            time,
        });
    }
    Ok(())
}

pub fn select_sweet_scent_encounter(
    data: &WildEncounterData,
    slot_tables: &EncounterSlotTables,
    surface: EncounterSurface,
    time: TimeOfDay,
    tile: crate::world::map::TilePosition,
    slot_percent_roll: u8,
    level_roll_byte: u8,
) -> Result<crate::world::session::WildEncounterRoll, EncounterError> {
    let resolved = select_wild_encounter(
        data,
        slot_tables,
        surface,
        time,
        slot_percent_roll,
        level_roll_byte,
    )?;
    Ok(crate::world::session::WildEncounterRoll {
        map_name: data.map_name.clone(),
        tile,
        surface,
        time,
        threshold: 255,
        encounter_roll: 0,
        slot_percent_roll: Some(slot_percent_roll),
        level_roll: Some(level_roll_byte),
        roaming_slot: None,
        resolved,
        repelled_by: None,
    })
}

pub fn tree_score(tile_x: i16, tile_y: i16, player_id: u16) -> u8 {
    let value = i32::from(tile_y) * (i32::from(tile_x) + 1) + i32::from(tile_x);
    let coord_score = value.div_euclid(5).rem_euclid(10) as u8;
    let trainer_score = (player_id % 10) as u8;
    (coord_score + 10 - trainer_score) % 10
}

pub fn select_headbutt_encounter(
    data: &FieldEncounterData,
    target_tile_x: i16,
    target_tile_y: i16,
    player_id: u16,
    chance_roll: u8,
    entry_roll: u8,
) -> Result<FieldEncounterRoll, EncounterError> {
    let score = tree_score(target_tile_x, target_tile_y, player_id);
    let table = data.table(FieldEncounterKind::Headbutt).ok_or_else(|| {
        EncounterError::MissingFieldEncounterTable {
            map_name: data.map_name.clone(),
            kind: FieldEncounterKind::Headbutt,
        }
    })?;
    let entries = match score {
        0 if chance_roll < 8 => Some(("rare", table.rare.as_slice())),
        1..=4 if chance_roll < 5 => Some(("common", table.common.as_slice())),
        5..=9 if chance_roll == 0 => Some(("common", table.common.as_slice())),
        _ => None,
    };
    let resolved = match entries {
        Some((bucket, entries)) => Some(choose_weighted_field_entry(
            data,
            FieldEncounterKind::Headbutt,
            bucket,
            entries,
            entry_roll,
        )?),
        None => None,
    };
    Ok(FieldEncounterRoll {
        map_name: data.map_name.clone(),
        kind: FieldEncounterKind::Headbutt,
        target_tile_x,
        target_tile_y,
        score: Some(score),
        chance_roll,
        entry_roll: resolved.as_ref().map(|_| entry_roll),
        resolved,
    })
}

pub fn roll_headbutt_encounter(
    data: &FieldEncounterData,
    target_tile_x: i16,
    target_tile_y: i16,
    player_id: u16,
    rng: &mut Random,
) -> Result<FieldEncounterRoll, EncounterError> {
    let chance_roll = rng.randrange(10) as u8;
    let entry_roll = rng.randrange(100) as u8;
    select_headbutt_encounter(
        data,
        target_tile_x,
        target_tile_y,
        player_id,
        chance_roll,
        entry_roll,
    )
}

pub fn select_rock_smash_encounter(
    data: &FieldEncounterData,
    target_tile_x: i16,
    target_tile_y: i16,
    chance_roll: u8,
    entry_roll: u8,
) -> Result<FieldEncounterRoll, EncounterError> {
    let table = data.table(FieldEncounterKind::RockSmash).ok_or_else(|| {
        EncounterError::MissingFieldEncounterTable {
            map_name: data.map_name.clone(),
            kind: FieldEncounterKind::RockSmash,
        }
    })?;
    let resolved = if chance_roll < 4 {
        Some(choose_weighted_field_entry(
            data,
            FieldEncounterKind::RockSmash,
            "common",
            table.common.as_slice(),
            entry_roll,
        )?)
    } else {
        None
    };
    Ok(FieldEncounterRoll {
        map_name: data.map_name.clone(),
        kind: FieldEncounterKind::RockSmash,
        target_tile_x,
        target_tile_y,
        score: None,
        chance_roll,
        entry_roll: resolved.as_ref().map(|_| entry_roll),
        resolved,
    })
}

/// Resolve the RNG-owned portion of `RockMonEncounter` from
/// `engine/events/treemons.asm`.
///
/// A missing map or a map without a rock table is the cartridge's
/// `GetTreeMonSet`/`GetTreeMons` failure path: it succeeds as a no-encounter
/// result without reading DIV. On eligible maps the range-100 selection is
/// conditional on the range-10 result being in `0..4`.
pub fn resolve_rock_mon_encounter<S>(
    data: Option<&FieldEncounterData>,
    random_state: CrystalRandomState,
    divider: &mut S,
) -> Result<RockMonEncounterOutcome, RockMonEncounterError<S::Error>>
where
    S: DividerSource + ?Sized,
{
    let Some(data) = data else {
        return Ok(RockMonEncounterOutcome {
            chance_roll: None,
            entry_roll: None,
            resolved: None,
            random_state_after: random_state,
        });
    };
    let Some(table) = data.table(FieldEncounterKind::RockSmash) else {
        return Ok(RockMonEncounterOutcome {
            chance_roll: None,
            entry_roll: None,
            resolved: None,
            random_state_after: random_state,
        });
    };

    let mut rng = CrystalRandom::new(random_state, divider);
    let chance_roll = rng
        .random_range(10)
        .map_err(RockMonEncounterError::Divider)?;
    if chance_roll >= 4 {
        return Ok(RockMonEncounterOutcome {
            chance_roll: Some(chance_roll),
            entry_roll: None,
            resolved: None,
            random_state_after: rng.state(),
        });
    }

    let entry_roll = rng
        .random_range(100)
        .map_err(RockMonEncounterError::Divider)?;
    let resolved = choose_weighted_field_entry(
        data,
        FieldEncounterKind::RockSmash,
        "common",
        table.common.as_slice(),
        entry_roll,
    )?;
    Ok(RockMonEncounterOutcome {
        chance_roll: Some(chance_roll),
        entry_roll: Some(entry_roll),
        resolved: Some(resolved),
        random_state_after: rng.state(),
    })
}

fn choose_weighted_field_entry(
    data: &FieldEncounterData,
    kind: FieldEncounterKind,
    bucket: &'static str,
    entries: &[FieldEncounterEntry],
    entry_roll: u8,
) -> Result<ResolvedWildEncounter, EncounterError> {
    if entries.is_empty() {
        return Err(EncounterError::EmptyFieldEncounterEntries {
            map_name: data.map_name.clone(),
            kind,
            bucket,
        });
    }
    let mut remaining = entry_roll % 100;
    for (slot, entry) in entries.iter().enumerate() {
        if entry.weight == 0 {
            continue;
        }
        if remaining < entry.weight {
            if !is_exact_nonempty_encounter_token(&entry.species) {
                return Err(EncounterError::InvalidFieldEncounterSpecies {
                    map_name: data.map_name.clone(),
                    kind,
                    species: entry.species.clone(),
                });
            }
            return Ok(ResolvedWildEncounter {
                encounter: WildEncounter {
                    level: entry.level,
                    species: entry.species.clone(),
                },
                slot,
                level: entry.level,
            });
        }
        remaining = remaining.saturating_sub(entry.weight);
    }
    Err(EncounterError::EmptyFieldEncounterEntries {
        map_name: data.map_name.clone(),
        kind,
        bucket,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_data() -> WildEncounterData {
        WildEncounterData {
            map_name: "ROUTE_29".to_string(),
            grass_rates: Some(
                [
                    ("morning".to_string(), 30),
                    ("day".to_string(), 20),
                    ("night".to_string(), 10),
                ]
                .into_iter()
                .collect(),
            ),
            water_rate: Some(15),
            grass: Some(WildEncounterTable {
                morning: vec![
                    WildEncounter {
                        level: 2,
                        species: "PIDGEY".to_string(),
                    },
                    WildEncounter {
                        level: 3,
                        species: "RATTATA".to_string(),
                    },
                ],
                day: vec![WildEncounter {
                    level: 4,
                    species: "SENTRET".to_string(),
                }],
                night: Vec::new(),
            }),
            water: Some(WildEncounterTable {
                morning: vec![WildEncounter {
                    level: 10,
                    species: "MAGIKARP".to_string(),
                }],
                day: Vec::new(),
                night: Vec::new(),
            }),
        }
    }

    fn field_data() -> FieldEncounterData {
        FieldEncounterData::for_crystal(
            "Route29",
            Some(FieldEncounterTable {
                common: vec![FieldEncounterEntry {
                    weight: 100,
                    species: "HOOTHOOT".to_string(),
                    level: 10,
                    sleep_turns_by_time: Default::default(),
                }],
                rare: vec![FieldEncounterEntry {
                    weight: 100,
                    species: "PINECO".to_string(),
                    level: 10,
                    sleep_turns_by_time: Default::default(),
                }],
            }),
            Some(FieldEncounterTable {
                common: vec![
                    FieldEncounterEntry {
                        weight: 90,
                        species: "KRABBY".to_string(),
                        level: 15,
                        sleep_turns_by_time: Default::default(),
                    },
                    FieldEncounterEntry {
                        weight: 10,
                        species: "SHUCKLE".to_string(),
                        level: 15,
                        sleep_turns_by_time: Default::default(),
                    },
                ],
                rare: Vec::new(),
            }),
        )
    }

    #[test]
    fn field_encounter_sleep_rules_are_required_and_byte_validated() {
        let missing = serde_json::from_value::<FieldEncounterEntry>(serde_json::json!({
            "weight": 100,
            "species": "CATERPIE",
            "level": 5
        }))
        .expect_err("field encounter sleep rules must be explicit")
        .to_string();
        assert!(missing.contains("sleep_turns_by_time"), "{missing}");

        let invalid = serde_json::from_value::<FieldEncounterEntry>(serde_json::json!({
            "weight": 100,
            "species": "CATERPIE",
            "level": 5,
            "sleep_turns_by_time": { "night": 0 }
        }))
        .expect_err("zero cannot represent an asleep tree monster")
        .to_string();
        assert!(invalid.contains("expected 1..=7"), "{invalid}");

        let valid = serde_json::from_value::<FieldEncounterEntry>(serde_json::json!({
            "weight": 100,
            "species": "CATERPIE",
            "level": 5,
            "sleep_turns_by_time": { "night": 7 }
        }))
        .expect("canonical tree sleep rule");
        assert_eq!(valid.sleep_turns_by_time.get(&TimeOfDay::Night), Some(&7));
    }

    #[test]
    fn field_encounter_catalog_rejects_sleep_rules_on_rock_smash_entries() {
        let mut data = field_data();
        data.table_mut(FieldEncounterKind::RockSmash)
            .expect("rock smash")
            .common[0]
            .sleep_turns_by_time
            .insert(TimeOfDay::Night, 7);
        let encounters = [("Route29".to_string(), data)].into_iter().collect();
        let map_ids = ["Route29".to_string()].into_iter().collect();
        let species_ids = [
            "HOOTHOOT".to_string(),
            "PINECO".to_string(),
            "KRABBY".to_string(),
            "SHUCKLE".to_string(),
        ]
        .into_iter()
        .collect();

        assert_eq!(
            field_encounter_catalog_issues(&encounters, &map_ids, &species_ids),
            vec![FieldEncounterCatalogIssue::UnexpectedSleepRule {
                map_name: "Route29".to_string(),
                kind: "rock_smash".to_string(),
                bucket: "common",
                entry_index: 0,
                species_id: "KRABBY".to_string(),
                time: TimeOfDay::Night,
            }]
        );
    }

    fn slot_tables() -> EncounterSlotTables {
        EncounterSlotTables::for_crystal(
            vec![
                EncounterSlotChance {
                    threshold: 30,
                    slot: 0,
                },
                EncounterSlotChance {
                    threshold: 60,
                    slot: 1,
                },
                EncounterSlotChance {
                    threshold: 80,
                    slot: 2,
                },
                EncounterSlotChance {
                    threshold: 90,
                    slot: 3,
                },
                EncounterSlotChance {
                    threshold: 95,
                    slot: 4,
                },
                EncounterSlotChance {
                    threshold: 99,
                    slot: 5,
                },
                EncounterSlotChance {
                    threshold: 100,
                    slot: 6,
                },
            ],
            vec![
                EncounterSlotChance {
                    threshold: 60,
                    slot: 0,
                },
                EncounterSlotChance {
                    threshold: 90,
                    slot: 1,
                },
                EncounterSlotChance {
                    threshold: 100,
                    slot: 2,
                },
            ],
        )
    }

    fn music_modifiers() -> EncounterMusicModifiers {
        EncounterMusicModifiers {
            modifiers: BTreeMap::from([
                (
                    "MUSIC_POKEMON_MARCH".to_string(),
                    EncounterMusicModifier {
                        numerator: 2,
                        denominator: 1,
                    },
                ),
                (
                    "MUSIC_RUINS_OF_ALPH_RADIO".to_string(),
                    EncounterMusicModifier {
                        numerator: 2,
                        denominator: 1,
                    },
                ),
                (
                    "MUSIC_POKEMON_LULLABY".to_string(),
                    EncounterMusicModifier {
                        numerator: 1,
                        denominator: 2,
                    },
                ),
            ]),
        }
    }

    #[test]
    fn encounter_slot_table_issues_validate_exact_threshold_tables() {
        let tables = EncounterSlotTables::for_crystal(
            vec![
                EncounterSlotChance {
                    threshold: 60,
                    slot: 0,
                },
                EncounterSlotChance {
                    threshold: 50,
                    slot: 0,
                },
                EncounterSlotChance {
                    threshold: 0,
                    slot: 1,
                },
            ],
            Vec::new(),
        );

        assert_eq!(
            encounter_slot_table_issues(&tables, true),
            vec![
                EncounterSlotTableIssue::UnorderedThreshold {
                    surface: EncounterSurface::Grass,
                    threshold: 50,
                    previous: 60,
                },
                EncounterSlotTableIssue::DuplicateSlotIndex {
                    surface: EncounterSurface::Grass,
                    slot: 0,
                },
                EncounterSlotTableIssue::InvalidThreshold {
                    surface: EncounterSurface::Grass,
                    threshold: 0,
                },
                EncounterSlotTableIssue::UnorderedThreshold {
                    surface: EncounterSurface::Grass,
                    threshold: 0,
                    previous: 50,
                },
                EncounterSlotTableIssue::IncompleteTable {
                    surface: EncounterSurface::Grass,
                },
                EncounterSlotTableIssue::MissingTable {
                    surface: EncounterSurface::Water,
                },
            ]
        );
        assert_eq!(encounter_slot_table_issues(&tables, false), []);
    }

    #[test]
    fn encounter_slot_tables_accept_exact_custom_surface_keys_without_ignoring_bad_tables() {
        let mut tables = EncounterSlotTables::for_crystal(
            vec![EncounterSlotChance {
                threshold: 100,
                slot: 0,
            }],
            vec![EncounterSlotChance {
                threshold: 100,
                slot: 0,
            }],
        );
        tables.tables.insert(
            "volcanic_ash".to_string(),
            vec![
                EncounterSlotChance {
                    threshold: 25,
                    slot: 0,
                },
                EncounterSlotChance {
                    threshold: 100,
                    slot: 1,
                },
            ],
        );

        assert_eq!(encounter_slot_table_issues(&tables, true), []);

        tables.tables.insert("volcanic ash".to_string(), Vec::new());
        tables.tables.insert(
            "deep_cave".to_string(),
            vec![EncounterSlotChance {
                threshold: 99,
                slot: 0,
            }],
        );

        assert_eq!(
            encounter_slot_table_issues(&tables, true),
            vec![
                EncounterSlotTableIssue::IncompleteCustomTable {
                    surface_id: "deep_cave".to_string(),
                },
                EncounterSlotTableIssue::InvalidSurfaceId {
                    surface_id: "volcanic ash".to_string(),
                },
            ]
        );
    }

    #[test]
    fn encounter_music_modifier_issues_validate_exact_music_ids() {
        let modifiers = EncounterMusicModifiers {
            modifiers: BTreeMap::from([
                (
                    "MUSIC_POKEMON_MARCH".to_string(),
                    EncounterMusicModifier {
                        numerator: 2,
                        denominator: 1,
                    },
                ),
                (
                    "MUSIC POKEMON MARCH".to_string(),
                    EncounterMusicModifier {
                        numerator: 1,
                        denominator: 1,
                    },
                ),
                (
                    "music_pokemon_march".to_string(),
                    EncounterMusicModifier {
                        numerator: 1,
                        denominator: 0,
                    },
                ),
            ]),
        };
        let music_ids = ["MUSIC_POKEMON_MARCH".to_string()].into_iter().collect();

        assert_eq!(
            encounter_music_modifier_issues(&modifiers, &music_ids, true),
            vec![
                EncounterMusicModifierIssue::InvalidMusicId {
                    music_id: "MUSIC POKEMON MARCH".to_string(),
                },
                EncounterMusicModifierIssue::UnknownMusicId {
                    music_id: "music_pokemon_march".to_string(),
                },
                EncounterMusicModifierIssue::InvalidRatio {
                    music_id: "music_pokemon_march".to_string(),
                },
            ]
        );
        assert_eq!(
            encounter_music_modifier_issues(&EncounterMusicModifiers::default(), &music_ids, true),
            vec![EncounterMusicModifierIssue::MissingTable]
        );
        assert_eq!(
            encounter_music_modifier_issues(&EncounterMusicModifiers::default(), &music_ids, false),
            []
        );
    }

    #[test]
    fn wild_encounter_catalog_issues_validate_exact_ids_and_required_tables() {
        let mut route = sample_data();
        route.grass_rates = Some(
            [
                ("morning".to_string(), 10),
                ("day".to_string(), 10),
                ("night".to_string(), 10),
                (" night".to_string(), 5),
                ("late night".to_string(), 5),
                ("dusk".to_string(), 5),
            ]
            .into_iter()
            .collect(),
        );
        route.grass.as_mut().expect("grass").day.clear();
        route.water_rate = Some(10);
        route.water.as_mut().expect("water").night.clear();
        route.grass.as_mut().expect("grass").morning[0].species = " PIDGEY".to_string();
        route.grass.as_mut().expect("grass").morning[1].species = "pidgey".to_string();
        route
            .water
            .as_mut()
            .expect("water")
            .morning
            .push(WildEncounter {
                level: 10,
                species: "MISSING NO".to_string(),
            });
        let mut missing_table = sample_data();
        missing_table.map_name = "ROUTE_30".to_string();
        missing_table.grass = None;
        missing_table.water = None;
        missing_table.water_rate = Some(5);
        let invalid_map = WildEncounterData {
            map_name: " route_29".to_string(),
            grass_rates: None,
            water_rate: None,
            grass: None,
            water: None,
        };
        let encounters = [
            (" route_29".to_string(), invalid_map),
            ("route_29".to_string(), route),
            ("ROUTE_30".to_string(), missing_table),
        ]
        .into_iter()
        .collect();
        let map_ids = ["ROUTE_29".to_string(), "ROUTE_30".to_string()]
            .into_iter()
            .collect();
        let species_ids = ["PIDGEY".to_string(), "RATTATA".to_string()]
            .into_iter()
            .collect();

        assert_eq!(
            wild_encounter_catalog_issues(&encounters, &map_ids, &species_ids),
            vec![
                WildEncounterCatalogIssue::InvalidMap {
                    map_name: " route_29".to_string(),
                },
                WildEncounterCatalogIssue::MissingGrassTable {
                    map_name: "ROUTE_30".to_string(),
                },
                WildEncounterCatalogIssue::MissingWaterTable {
                    map_name: "ROUTE_30".to_string(),
                },
                WildEncounterCatalogIssue::UnknownMap {
                    map_name: "route_29".to_string(),
                },
                WildEncounterCatalogIssue::InvalidSpecies {
                    map_name: "route_29".to_string(),
                    species_id: " PIDGEY".to_string(),
                },
                WildEncounterCatalogIssue::UnknownSpecies {
                    map_name: "route_29".to_string(),
                    species_id: "MAGIKARP".to_string(),
                },
                WildEncounterCatalogIssue::InvalidSpecies {
                    map_name: "route_29".to_string(),
                    species_id: "MISSING NO".to_string(),
                },
                WildEncounterCatalogIssue::UnknownSpecies {
                    map_name: "route_29".to_string(),
                    species_id: "pidgey".to_string(),
                },
                WildEncounterCatalogIssue::InvalidGrassRateTime {
                    map_name: "route_29".to_string(),
                    time_key: " night".to_string(),
                },
                WildEncounterCatalogIssue::UnknownGrassRateTime {
                    map_name: "route_29".to_string(),
                    time_key: "dusk".to_string(),
                },
                WildEncounterCatalogIssue::InvalidGrassRateTime {
                    map_name: "route_29".to_string(),
                    time_key: "late night".to_string(),
                },
                WildEncounterCatalogIssue::EmptyGrassSlots {
                    map_name: "route_29".to_string(),
                    time_key: "day",
                },
                WildEncounterCatalogIssue::EmptyGrassSlots {
                    map_name: "route_29".to_string(),
                    time_key: "night",
                },
                WildEncounterCatalogIssue::EmptyWaterSlots {
                    map_name: "route_29".to_string(),
                    time_key: "day",
                },
                WildEncounterCatalogIssue::EmptyWaterSlots {
                    map_name: "route_29".to_string(),
                    time_key: "night",
                },
            ]
        );
    }

    #[test]
    fn field_encounter_catalog_issues_validate_exact_ids_and_weights() {
        let mut data = field_data();
        data.table_mut(FieldEncounterKind::Headbutt)
            .expect("headbutt")
            .common[0]
            .species = " PIDGEY".to_string();
        data.table_mut(FieldEncounterKind::RockSmash)
            .expect("rock smash")
            .common[0]
            .species = "PINE CO".to_string();
        data.table_mut(FieldEncounterKind::RockSmash)
            .expect("rock smash")
            .common[1]
            .species = "pidgey".to_string();
        data.table_mut(FieldEncounterKind::Headbutt)
            .expect("headbutt")
            .common[0]
            .weight = 0;
        data.table_mut(FieldEncounterKind::Headbutt)
            .expect("headbutt")
            .rare
            .clear();
        let invalid_map = FieldEncounterData::for_crystal(" route_29", None, None);
        let encounters = [
            (" route_29".to_string(), invalid_map),
            ("route_29".to_string(), data),
        ]
        .into_iter()
        .collect();
        let map_ids = ["Route29".to_string()].into_iter().collect();
        let species_ids = ["HOOTHOOT".to_string(), "PINECO".to_string()]
            .into_iter()
            .collect();

        assert_eq!(
            field_encounter_catalog_issues(&encounters, &map_ids, &species_ids),
            vec![
                FieldEncounterCatalogIssue::InvalidMap {
                    map_name: " route_29".to_string(),
                },
                FieldEncounterCatalogIssue::UnknownMap {
                    map_name: "route_29".to_string(),
                },
                FieldEncounterCatalogIssue::InvalidSpecies {
                    map_name: "route_29".to_string(),
                    species_id: " PIDGEY".to_string(),
                },
                FieldEncounterCatalogIssue::InvalidSpecies {
                    map_name: "route_29".to_string(),
                    species_id: "PINE CO".to_string(),
                },
                FieldEncounterCatalogIssue::UnknownSpecies {
                    map_name: "route_29".to_string(),
                    species_id: "pidgey".to_string(),
                },
                FieldEncounterCatalogIssue::ZeroWeight {
                    map_name: "route_29".to_string(),
                    kind: "headbutt".to_string(),
                    bucket: "common",
                    entry_index: 0,
                    species_id: " PIDGEY".to_string(),
                },
                FieldEncounterCatalogIssue::InvalidWeightTotal {
                    map_name: "route_29".to_string(),
                    kind: "headbutt".to_string(),
                    bucket: "common",
                    total_weight: 0,
                },
                FieldEncounterCatalogIssue::EmptyBucket {
                    map_name: "route_29".to_string(),
                    kind: "headbutt".to_string(),
                    bucket: "rare",
                },
            ]
        );
    }

    #[test]
    fn encounter_catalog_issues_reject_reserved_pack_prefix_tokens() {
        let mut wild = sample_data();
        wild.map_name = "fallback_route_29".to_string();
        wild.grass.as_mut().expect("grass").morning[0].species = "legacy_pidgey".to_string();
        wild.grass_rates
            .as_mut()
            .expect("grass rates")
            .insert("fallback_morning".to_string(), 30);
        let wild_encounters = [("fallback_route_29".to_string(), wild)]
            .into_iter()
            .collect();

        let wild_species_ids = [
            "RATTATA".to_string(),
            "SENTRET".to_string(),
            "MAGIKARP".to_string(),
        ]
        .into_iter()
        .collect();
        assert_eq!(
            wild_encounter_catalog_issues(&wild_encounters, &BTreeSet::new(), &wild_species_ids),
            vec![
                WildEncounterCatalogIssue::InvalidMap {
                    map_name: "fallback_route_29".to_string(),
                },
                WildEncounterCatalogIssue::InvalidSpecies {
                    map_name: "fallback_route_29".to_string(),
                    species_id: "legacy_pidgey".to_string(),
                },
                WildEncounterCatalogIssue::InvalidGrassRateTime {
                    map_name: "fallback_route_29".to_string(),
                    time_key: "fallback_morning".to_string(),
                },
                WildEncounterCatalogIssue::EmptyGrassSlots {
                    map_name: "fallback_route_29".to_string(),
                    time_key: "night",
                },
                WildEncounterCatalogIssue::EmptyWaterSlots {
                    map_name: "fallback_route_29".to_string(),
                    time_key: "day",
                },
                WildEncounterCatalogIssue::EmptyWaterSlots {
                    map_name: "fallback_route_29".to_string(),
                    time_key: "night",
                },
            ]
        );

        let mut field = field_data();
        field
            .table_mut(FieldEncounterKind::Headbutt)
            .expect("headbutt")
            .common[0]
            .species = "fallback_hoothoot".to_string();
        let field_encounters = [("legacy_route_29".to_string(), field)]
            .into_iter()
            .collect();

        let field_species_ids = [
            "PINECO".to_string(),
            "KRABBY".to_string(),
            "SHUCKLE".to_string(),
        ]
        .into_iter()
        .collect();
        assert_eq!(
            field_encounter_catalog_issues(&field_encounters, &BTreeSet::new(), &field_species_ids),
            vec![
                FieldEncounterCatalogIssue::InvalidMap {
                    map_name: "legacy_route_29".to_string(),
                },
                FieldEncounterCatalogIssue::InvalidSpecies {
                    map_name: "legacy_route_29".to_string(),
                    species_id: "fallback_hoothoot".to_string(),
                },
            ]
        );
    }

    #[test]
    fn time_keys_are_exact_pack_values_without_aliases_or_defaults() {
        assert_eq!(ENCOUNTER_TIME_KEYS, &["morning", "day", "night"]);
        assert_eq!(
            resolve_encounter_time_key("morning").unwrap(),
            TimeOfDay::Morning
        );
        assert_eq!(resolve_encounter_time_key("day").unwrap(), TimeOfDay::Day);
        assert_eq!(
            resolve_encounter_time_key("night").unwrap(),
            TimeOfDay::Night
        );
        assert!(resolve_encounter_time_key("").is_err());
        assert!(resolve_encounter_time_key("morn").is_err());
        assert!(resolve_encounter_time_key("NIGHT").is_err());
        assert!(resolve_encounter_time_key("dawn").is_err());
    }

    #[test]
    fn percent_to_byte_matches_javascript_flooring() {
        assert_eq!(percent_to_byte(-1.0), 0);
        assert_eq!(percent_to_byte(0.0), 0);
        assert_eq!(percent_to_byte(35.0), 89);
        assert_eq!(percent_to_byte(50.0), 127);
        assert_eq!(percent_to_byte(100.0), 255);
        assert_eq!(percent_to_byte(120.0), 255);
    }

    #[test]
    fn encounter_rates_and_effects_match_existing_threshold_rules() {
        let data = sample_data();

        assert_eq!(
            base_encounter_rate(&data, EncounterSurface::Grass, TimeOfDay::Morning).unwrap(),
            30
        );
        assert_eq!(
            base_encounter_rate(&data, EncounterSurface::Water, TimeOfDay::Morning).unwrap(),
            15
        );
        assert_eq!(
            encounter_threshold(
                &data,
                EncounterSurface::Grass,
                TimeOfDay::Morning,
                Some("MUSIC_POKEMON_LULLABY"),
                &music_modifiers(),
                true,
            )
            .unwrap(),
            percent_to_byte(30.0) >> 2
        );
        assert!(passes_encounter_roll(10, 9));
        assert!(!passes_encounter_roll(10, 10));
        assert!(!passes_encounter_roll(0, 0));
    }

    #[test]
    fn encounter_music_modifier_overflow_rejects_pack_ratio_without_threshold_truncation() {
        let modifiers = EncounterMusicModifiers {
            modifiers: BTreeMap::from([(
                "MUSIC_POKEMON_MARCH".to_string(),
                EncounterMusicModifier {
                    numerator: 2,
                    denominator: 1,
                },
            )]),
        };

        assert_eq!(
            apply_encounter_music_effect(200, Some("MUSIC_POKEMON_MARCH"), &modifiers),
            Err(EncounterError::EncounterMusicModifierOverflow {
                music_id: "MUSIC_POKEMON_MARCH".to_string(),
                threshold: 200,
                numerator: 2,
                denominator: 1,
                adjusted: 400,
            })
        );
    }

    #[test]
    fn slot_probabilities_match_grass_and_water_boundaries() {
        let slot_tables = slot_tables();
        assert_eq!(
            choose_slot_from_percent(&slot_tables, EncounterSurface::Grass, 7, 1).unwrap(),
            0
        );
        assert_eq!(
            choose_slot_from_percent(&slot_tables, EncounterSurface::Grass, 7, 30).unwrap(),
            0
        );
        assert_eq!(
            choose_slot_from_percent(&slot_tables, EncounterSurface::Grass, 7, 31).unwrap(),
            1
        );
        assert_eq!(
            choose_slot_from_percent(&slot_tables, EncounterSurface::Grass, 7, 99).unwrap(),
            5
        );
        assert_eq!(
            choose_slot_from_percent(&slot_tables, EncounterSurface::Grass, 7, 100).unwrap(),
            6
        );
        assert_eq!(
            choose_slot_from_percent(&slot_tables, EncounterSurface::Water, 3, 60).unwrap(),
            0
        );
        assert_eq!(
            choose_slot_from_percent(&slot_tables, EncounterSurface::Water, 3, 61).unwrap(),
            1
        );
        assert_eq!(
            choose_slot_from_percent(&slot_tables, EncounterSurface::Water, 3, 91).unwrap(),
            2
        );
        assert!(matches!(
            choose_slot_from_percent(&slot_tables, EncounterSurface::Water, 0, 50),
            Err(EncounterError::UnresolvedSlot {
                surface: EncounterSurface::Water,
                slot_count: 0,
                roll: 50
            })
        ));
    }

    #[test]
    fn random_percent_rejects_bytes_outside_percent_range() {
        assert_eq!(random_percent_from_bytes([100, 255, 42]), Some(43));
        assert_eq!(random_percent_from_bytes([100, 255]), None);
    }

    #[test]
    fn surf_level_variance_uses_crystals_thresholds() {
        assert_eq!(
            apply_surf_level_variance(5, EncounterSurface::Grass, 255),
            5
        );
        assert_eq!(apply_surf_level_variance(5, EncounterSurface::Water, 0), 5);
        assert_eq!(
            apply_surf_level_variance(5, EncounterSurface::Water, 90),
            6
        );
        assert_eq!(
            apply_surf_level_variance(5, EncounterSurface::Water, 166),
            7
        );
        assert_eq!(
            apply_surf_level_variance(5, EncounterSurface::Water, 243),
            9
        );
    }

    #[test]
    fn select_encounter_uses_time_surface_slot_and_level_rolls() {
        let data = sample_data();
        let encounter = select_wild_encounter(
            &data,
            &slot_tables(),
            EncounterSurface::Grass,
            TimeOfDay::Morning,
            31,
            255,
        )
        .unwrap()
        .expect("encounter selected");

        assert_eq!(encounter.slot, 1);
        assert_eq!(encounter.encounter.species, "RATTATA");
        assert_eq!(encounter.level, 7);
    }

    #[test]
    fn wild_encounter_rejects_malformed_runtime_species() {
        let mut data = sample_data();
        data.grass.as_mut().expect("grass").morning[0].species = "PID GEY".to_string();

        assert_eq!(
            select_wild_encounter(
                &data,
                &slot_tables(),
                EncounterSurface::Grass,
                TimeOfDay::Morning,
                30,
                0,
            ),
            Err(EncounterError::InvalidEncounterSpecies {
                map_name: "ROUTE_29".to_string(),
                species: "PID GEY".to_string(),
            })
        );
    }

    #[test]
    fn sweet_scent_selects_from_exact_surface_table_without_rate_roll() {
        let data = sample_data();
        let roll = select_sweet_scent_encounter(
            &data,
            &slot_tables(),
            EncounterSurface::Grass,
            TimeOfDay::Morning,
            crate::world::map::TilePosition::new(4, 6),
            31,
            255,
        )
        .expect("sweet scent encounter");

        assert_eq!(roll.map_name, "ROUTE_29");
        assert_eq!(roll.tile, crate::world::map::TilePosition::new(4, 6));
        assert_eq!(roll.surface, EncounterSurface::Grass);
        assert_eq!(roll.time, TimeOfDay::Morning);
        assert_eq!(roll.threshold, 255);
        assert_eq!(roll.encounter_roll, 0);
        assert_eq!(roll.slot_percent_roll, Some(31));
        assert_eq!(roll.level_roll, Some(255));
        let resolved = roll.resolved.expect("resolved");
        assert_eq!(resolved.slot, 1);
        assert_eq!(resolved.encounter.species, "RATTATA");
        assert_eq!(resolved.level, 3);
    }

    #[test]
    fn sweet_scent_requires_exact_surface_table() {
        let mut data = sample_data();
        data.water = None;

        assert!(matches!(
            require_encounter_table_for_surface(
                &data,
                EncounterSurface::Water,
                TimeOfDay::Morning
            ),
            Err(EncounterError::MissingEncounterTable {
                map_name,
                surface: EncounterSurface::Water,
            }) if map_name == "ROUTE_29"
        ));
    }

    #[test]
    fn headbutt_field_encounter_uses_tree_score_chance_and_weighted_entry() {
        let data = field_data();
        assert_eq!(tree_score(0, 2, 0), 0);
        assert_eq!(tree_score(10, 10, 3), 1);

        let rare = select_headbutt_encounter(&data, 0, 2, 0, 2, 54).expect("rare headbutt roll");
        assert_eq!(rare.kind, FieldEncounterKind::Headbutt);
        assert_eq!(rare.score, Some(0));
        assert_eq!(
            rare.resolved.expect("rare headbutt").encounter.species,
            "PINECO"
        );

        let missed =
            select_headbutt_encounter(&data, 0, 2, 0, 8, 54).expect("missed headbutt roll");
        assert_eq!(missed.resolved, None);
        assert_eq!(missed.entry_roll, None);
    }

    #[test]
    fn field_encounter_rejects_malformed_runtime_species() {
        let mut data = field_data();
        data.table_mut(FieldEncounterKind::Headbutt)
            .expect("headbutt")
            .rare[0]
            .species = "PIN ECO".to_string();

        assert_eq!(
            select_headbutt_encounter(&data, 0, 2, 0, 2, 54),
            Err(EncounterError::InvalidFieldEncounterSpecies {
                map_name: "Route29".to_string(),
                kind: FieldEncounterKind::Headbutt,
                species: "PIN ECO".to_string(),
            })
        );
    }

    #[test]
    fn rock_smash_field_encounter_uses_chance_and_common_weights() {
        let data = field_data();

        let krabby = select_rock_smash_encounter(&data, 4, 6, 2, 89).expect("krabby rock smash");
        assert_eq!(krabby.kind, FieldEncounterKind::RockSmash);
        assert_eq!(krabby.score, None);
        assert_eq!(
            krabby.resolved.expect("rock smash").encounter.species,
            "KRABBY"
        );

        let shuckle = select_rock_smash_encounter(&data, 4, 6, 2, 90).expect("shuckle rock smash");
        assert_eq!(
            shuckle.resolved.expect("rock smash").encounter.species,
            "SHUCKLE"
        );

        let missed =
            select_rock_smash_encounter(&data, 4, 6, 4, 90).expect("missed rock smash roll");
        assert_eq!(missed.resolved, None);
        assert_eq!(missed.entry_roll, None);
    }

    #[test]
    fn rock_mon_encounter_uses_zero_divider_reads_without_a_rock_table() {
        let random_state = crate::random::CrystalRandomState {
            add: 0x12,
            sub: 0x34,
        };
        let mut missing_map_divider = crate::random::ReplayDivider::new([]);
        let missing_map = resolve_rock_mon_encounter(None, random_state, &mut missing_map_divider)
            .expect("a map absent from RockMonMaps is a zero-read miss");

        assert_eq!(missing_map.chance_roll, None);
        assert_eq!(missing_map.entry_roll, None);
        assert_eq!(missing_map.resolved, None);
        assert_eq!(missing_map.random_state_after, random_state);
        assert_eq!(missing_map_divider.consumed(), 0);

        let ineligible = FieldEncounterData::for_crystal("IcePathB3F", None, None);
        let mut ineligible_divider = crate::random::ReplayDivider::new([]);
        let no_rock_table =
            resolve_rock_mon_encounter(Some(&ineligible), random_state, &mut ineligible_divider)
                .expect("a map without a rock set is a zero-read miss");

        assert_eq!(no_rock_table, missing_map);
        assert_eq!(ineligible_divider.consumed(), 0);
    }

    #[test]
    fn rock_mon_encounter_miss_consumes_only_the_chance_random_range() {
        let data = field_data();
        let mut divider = crate::random::ReplayDivider::new([3, 0]);

        let outcome = resolve_rock_mon_encounter(
            Some(&data),
            crate::random::CrystalRandomState::default(),
            &mut divider,
        )
        .expect("chance roll 4 is a Rock Smash miss");

        // RandomRange enters Random with carry set, so hRandomAdd becomes
        // 0 + 3 + 1 = 4. No entry roll is permitted after that miss.
        assert_eq!(outcome.chance_roll, Some(4));
        assert_eq!(outcome.entry_roll, None);
        assert_eq!(outcome.resolved, None);
        assert_eq!(
            outcome.random_state_after,
            crate::random::CrystalRandomState { add: 4, sub: 0 }
        );
        assert_eq!(divider.consumed(), 2);
        assert_eq!(divider.remaining(), 0);
    }

    #[test]
    fn rock_mon_encounter_selects_the_exact_ninety_ten_boundary() {
        let data = field_data();

        let mut krabby_divider = crate::random::ReplayDivider::new([255, 0, 88, 0]);
        let krabby = resolve_rock_mon_encounter(
            Some(&data),
            crate::random::CrystalRandomState::default(),
            &mut krabby_divider,
        )
        .expect("entry roll 89 selects Krabby");
        assert_eq!(krabby.chance_roll, Some(0));
        assert_eq!(krabby.entry_roll, Some(89));
        assert_eq!(
            krabby
                .resolved
                .as_ref()
                .expect("Krabby encounter")
                .encounter,
            WildEncounter {
                level: 15,
                species: "KRABBY".to_string(),
            }
        );
        assert_eq!(
            krabby.random_state_after,
            crate::random::CrystalRandomState { add: 89, sub: 255 }
        );
        assert_eq!(krabby_divider.consumed(), 4);

        let mut shuckle_divider = crate::random::ReplayDivider::new([255, 0, 89, 0]);
        let shuckle = resolve_rock_mon_encounter(
            Some(&data),
            crate::random::CrystalRandomState::default(),
            &mut shuckle_divider,
        )
        .expect("entry roll 90 selects Shuckle");
        assert_eq!(shuckle.chance_roll, Some(0));
        assert_eq!(shuckle.entry_roll, Some(90));
        assert_eq!(
            shuckle
                .resolved
                .as_ref()
                .expect("Shuckle encounter")
                .encounter,
            WildEncounter {
                level: 15,
                species: "SHUCKLE".to_string(),
            }
        );
        assert_eq!(
            shuckle.random_state_after,
            crate::random::CrystalRandomState { add: 90, sub: 255 }
        );
        assert_eq!(shuckle_divider.consumed(), 4);
    }

    #[test]
    fn rock_mon_encounter_retries_random_ranges_with_carry_set() {
        let data = field_data();

        // For RandomRange(10), hRandomAdd 250 is rejected. The retry also
        // enters Random with carry set and produces chance roll 4.
        let mut chance_divider = crate::random::ReplayDivider::new([249, 0, 9, 0]);
        let chance_retry = resolve_rock_mon_encounter(
            Some(&data),
            crate::random::CrystalRandomState::default(),
            &mut chance_divider,
        )
        .expect("range-10 rejection retries");
        assert_eq!(chance_retry.chance_roll, Some(4));
        assert_eq!(chance_retry.entry_roll, None);
        assert_eq!(
            chance_retry.random_state_after,
            crate::random::CrystalRandomState { add: 4, sub: 255 }
        );
        assert_eq!(chance_divider.consumed(), 4);

        // The first range-100 hRandomAdd is 200 and must be rejected. The
        // second iteration's carry produces the exact Shuckle boundary 90.
        let mut entry_divider = crate::random::ReplayDivider::new([255, 0, 199, 0, 145, 0]);
        let entry_retry = resolve_rock_mon_encounter(
            Some(&data),
            crate::random::CrystalRandomState::default(),
            &mut entry_divider,
        )
        .expect("range-100 rejection retries");
        assert_eq!(entry_retry.chance_roll, Some(0));
        assert_eq!(entry_retry.entry_roll, Some(90));
        assert_eq!(
            entry_retry
                .resolved
                .as_ref()
                .expect("Shuckle after range rejection")
                .encounter
                .species,
            "SHUCKLE"
        );
        assert_eq!(
            entry_retry.random_state_after,
            crate::random::CrystalRandomState { add: 90, sub: 254 }
        );
        assert_eq!(entry_divider.consumed(), 6);
    }

    #[test]
    fn headbutt_field_encounter_roll_helper_consumes_runtime_rng_in_core() {
        let data = field_data();
        let mut helper_rng = Random::new(1);
        let mut explicit_rng = Random::new(1);
        let chance_roll = explicit_rng.randrange(10) as u8;
        let entry_roll = explicit_rng.randrange(100) as u8;

        let helper = roll_headbutt_encounter(&data, 0, 2, 0, &mut helper_rng).expect("helper roll");
        let explicit =
            select_headbutt_encounter(&data, 0, 2, 0, chance_roll, entry_roll).expect("explicit");

        assert_eq!(helper, explicit);
        assert_eq!(helper_rng.seed(), explicit_rng.seed());

    }

    #[test]
    fn field_encounters_accept_odd_runtime_targets() {
        let data = field_data();

        let headbutt =
            select_headbutt_encounter(&data, 1, 0, 0, 2, 54).expect("odd headbutt target");
        assert_eq!(headbutt.target_tile_x, 1);
        assert_eq!(headbutt.target_tile_y, 0);

        let rock_smash =
            select_rock_smash_encounter(&data, 0, 1, 2, 90).expect("odd rock smash target");
        assert_eq!(rock_smash.target_tile_x, 0);
        assert_eq!(rock_smash.target_tile_y, 1);

        let mut rng = Random::new(0x1234_5678);
        let headbutt_roll =
            roll_headbutt_encounter(&data, 1, 0, 0, &mut rng).expect("odd headbutt roll");
        assert_eq!(headbutt_roll.target_tile_x, 1);
        assert_ne!(rng.seed(), 0x1234_5678);

    }

    #[test]
    fn field_encounters_require_modpack_tables_and_selected_buckets() {
        let mut data = field_data();
        data.tables.remove(FieldEncounterKind::Headbutt.as_key());
        assert!(matches!(
            select_headbutt_encounter(&data, 0, 2, 0, 2, 54),
            Err(EncounterError::MissingFieldEncounterTable {
                map_name,
                kind: FieldEncounterKind::Headbutt,
            }) if map_name == "Route29"
        ));

        let mut data = field_data();
        data.table_mut(FieldEncounterKind::Headbutt)
            .expect("headbutt")
            .rare
            .clear();
        assert!(matches!(
            select_headbutt_encounter(&data, 0, 2, 0, 2, 54),
            Err(EncounterError::EmptyFieldEncounterEntries {
                map_name,
                kind: FieldEncounterKind::Headbutt,
                bucket: "rare",
            }) if map_name == "Route29"
        ));

        let mut data = field_data();
        data.tables.remove(FieldEncounterKind::RockSmash.as_key());
        assert!(matches!(
            select_rock_smash_encounter(&data, 4, 6, 2, 90),
            Err(EncounterError::MissingFieldEncounterTable {
                map_name,
                kind: FieldEncounterKind::RockSmash,
            }) if map_name == "Route29"
        ));

        let mut data = field_data();
        data.table_mut(FieldEncounterKind::RockSmash)
            .expect("rock smash")
            .common
            .clear();
        assert!(matches!(
            select_rock_smash_encounter(&data, 4, 6, 2, 90),
            Err(EncounterError::EmptyFieldEncounterEntries {
                map_name,
                kind: FieldEncounterKind::RockSmash,
                bucket: "common",
            }) if map_name == "Route29"
        ));
    }

    #[test]
    fn missing_rates_are_errors_instead_of_zero_encounter_defaults() {
        let mut data = sample_data();
        data.water_rate = None;
        assert!(matches!(
            base_encounter_rate(&data, EncounterSurface::Water, TimeOfDay::Morning),
            Err(EncounterError::MissingEncounterRate {
                map_name,
                surface: EncounterSurface::Water,
            }) if map_name == "ROUTE_29"
        ));

        let mut data = sample_data();
        data.grass_rates = None;
        assert!(matches!(
            base_encounter_rate(&data, EncounterSurface::Grass, TimeOfDay::Morning),
            Err(EncounterError::MissingEncounterRate {
                map_name,
                surface: EncounterSurface::Grass,
            }) if map_name == "ROUTE_29"
        ));

        let mut data = sample_data();
        data.grass_rates
            .as_mut()
            .expect("sample grass rates")
            .remove("day");
        assert!(matches!(
            base_encounter_rate(&data, EncounterSurface::Grass, TimeOfDay::Day),
            Err(EncounterError::MissingTimedEncounterRate {
                map_name,
                surface: EncounterSurface::Grass,
                time: TimeOfDay::Day,
            }) if map_name == "ROUTE_29"
        ));
    }

    #[test]
    fn missing_tables_are_errors_instead_of_empty_table_defaults() {
        let mut data = sample_data();
        data.grass = None;
        assert!(matches!(
            table_for_surface(&data, EncounterSurface::Grass, TimeOfDay::Morning),
            Err(EncounterError::MissingEncounterTable {
                map_name,
                surface: EncounterSurface::Grass,
            }) if map_name == "ROUTE_29"
        ));

        let mut data = sample_data();
        data.water = None;
        assert!(matches!(
            table_for_surface(&data, EncounterSurface::Water, TimeOfDay::Morning),
            Err(EncounterError::MissingEncounterTable {
                map_name,
                surface: EncounterSurface::Water,
            }) if map_name == "ROUTE_29"
        ));
    }

    #[test]
    fn wild_encounter_json_requires_explicit_rates_and_tables() {
        let missing_rate = serde_json::from_str::<WildEncounterData>(
            r#"{
              "map_name":"Route29",
              "grass_rates":{"morning":10,"day":10,"night":10},
              "grass":{"morning":[],"day":[],"night":[]},
              "water":{"morning":[],"day":[],"night":[]}
            }"#,
        )
        .expect_err("water_rate must be explicit, even when null")
        .to_string();

        assert!(
            missing_rate.contains("missing field `water_rate`"),
            "{missing_rate}"
        );
    }

    #[test]
    fn wild_encounter_table_json_requires_every_time_bucket() {
        let missing_time = serde_json::from_str::<WildEncounterData>(
            r#"{
              "map_name":"Route29",
              "grass_rates":{"morning":10,"day":10,"night":10},
              "water_rate":null,
              "grass":{"morning":[],"day":[]},
              "water":{"morning":[],"day":[],"night":[]}
            }"#,
        )
        .expect_err("night encounters must be explicit, even when empty")
        .to_string();

        assert!(
            missing_time.contains("missing field `night`"),
            "{missing_time}"
        );
    }

    #[test]
    fn wild_encounter_json_rejects_unknown_modpack_fields() {
        let error = serde_json::from_str::<WildEncounterData>(
            r#"{
              "map_name":"Route29",
              "grass_rates":{"morning":10,"day":10,"night":10},
              "water_rate":null,
              "grass":{
                "morning":[{"level":3,"species":"RATTATA","display":"Rattata"}],
                "day":[],
                "night":[]
              },
              "water":{"morning":[],"day":[],"night":[]}
            }"#,
        )
        .expect_err("wild encounter slots must not accept display aliases")
        .to_string();

        assert!(error.contains("unknown field `display`"), "{error}");
    }

    #[test]
    fn encounter_json_rejects_malformed_pack_tokens_at_deserialization() {
        let wild_cases = [
            (
                "wild map",
                r#"{
                  "map_name":"Route 29",
                  "grass_rates":{"morning":10,"day":10,"night":10},
                  "water_rate":null,
                  "grass":{"morning":[],"day":[],"night":[]},
                  "water":{"morning":[],"day":[],"night":[]}
                }"#,
            ),
            (
                "wild species",
                r#"{
                  "map_name":"Route29",
                  "grass_rates":{"morning":10,"day":10,"night":10},
                  "water_rate":null,
                  "grass":{"morning":[{"level":3,"species":"RAT TATA"}],"day":[],"night":[]},
                  "water":{"morning":[],"day":[],"night":[]}
                }"#,
            ),
            (
                "grass rate key",
                r#"{
                  "map_name":"Route29",
                  "grass_rates":{"mor ning":10,"day":10,"night":10},
                  "water_rate":null,
                  "grass":{"morning":[],"day":[],"night":[]},
                  "water":{"morning":[],"day":[],"night":[]}
                }"#,
            ),
        ];
        for (label, payload) in wild_cases {
            let error = serde_json::from_str::<WildEncounterData>(payload)
                .expect_err("malformed wild encounter tokens must fail during JSON load")
                .to_string();
            assert!(
                error.contains("encounter token must be"),
                "{label} produced unexpected error: {error}"
            );
        }

        let slot_error = serde_json::from_value::<EncounterSlotTables>(serde_json::json!({
            "tables": {
                "deep water": [{"threshold": 100, "slot": 0}]
            }
        }))
        .expect_err("malformed encounter slot table keys must fail during JSON load")
        .to_string();
        assert!(
            slot_error.contains("encounter token must be"),
            "{slot_error}"
        );

        let music_error = serde_json::from_value::<EncounterMusicModifiers>(serde_json::json!({
            "modifiers": {
                "MUSIC ROUTE_29": {"numerator": 1, "denominator": 2}
            }
        }))
        .expect_err("malformed encounter music ids must fail during JSON load")
        .to_string();
        assert!(
            music_error.contains("encounter token must be"),
            "{music_error}"
        );

        let field_cases = [
            (
                "field missing map",
                serde_json::json!({
                    "tables": {}
                }),
            ),
            (
                "field map",
                serde_json::json!({
                    "map_name": "Route 29",
                    "tables": {}
                }),
            ),
            (
                "field missing tables",
                serde_json::json!({
                    "map_name": "Route29"
                }),
            ),
            (
                "field legacy headbutt alias",
                serde_json::json!({
                    "map_name": "Route29",
                    "headbutt": {
                        "common": [{"weight": 100, "species": "AIPOM", "level": 10, "sleep_turns_by_time": {}}],
                        "rare": []
                    }
                }),
            ),
            (
                "field legacy rock smash alias",
                serde_json::json!({
                    "map_name": "Route29",
                    "rock_smash": {
                        "common": [{"weight": 100, "species": "KRABBY", "level": 10, "sleep_turns_by_time": {}}],
                        "rare": []
                    }
                }),
            ),
            (
                "field kind",
                serde_json::json!({
                    "map_name": "Route29",
                    "tables": {
                        "head butt": {
                            "common": [{"weight": 100, "species": "AIPOM", "level": 10, "sleep_turns_by_time": {}}],
                            "rare": []
                        }
                    }
                }),
            ),
            (
                "field species",
                serde_json::json!({
                    "map_name": "Route29",
                    "tables": {
                        "headbutt": {
                            "common": [{"weight": 100, "species": "AIP OM", "level": 10, "sleep_turns_by_time": {}}],
                            "rare": []
                        }
                    }
                }),
            ),
        ];
        for (label, payload) in field_cases {
            let error = serde_json::from_value::<FieldEncounterData>(payload)
                .expect_err("malformed field encounter tokens must fail during JSON load")
                .to_string();
            assert!(
                error.contains("encounter token must be")
                    || error.contains("missing field `map_name`")
                    || error.contains("missing field `tables`")
                    || error.contains("unknown field"),
                "{label} produced unexpected error: {error}"
            );
        }
    }

    #[test]
    fn encounter_discriminants_reject_legacy_alias_payloads() {
        let surface_error =
            serde_json::from_str::<EncounterSurface>(r#"{"grass":{"fallback_surface":"land"}}"#)
                .expect_err("encounter surfaces must not accept object-shaped aliases")
                .to_string();
        assert!(
            surface_error.contains("invalid type")
                || surface_error.contains("unknown field `fallback_surface`"),
            "{surface_error}"
        );

        let time_error = serde_json::from_str::<TimeOfDay>(r#"{"day":{"legacy_time":"DAY"}}"#)
            .expect_err("encounter times must not accept object-shaped aliases")
            .to_string();
        assert!(
            time_error.contains("invalid type")
                || time_error.contains("unknown field `legacy_time`"),
            "{time_error}"
        );

        let kind_error = serde_json::from_str::<FieldEncounterKind>(
            r#"{"headbutt":{"normalized_kind":"Headbutt"}}"#,
        )
        .expect_err("field encounter kinds must not accept normalized aliases")
        .to_string();
        assert!(
            kind_error.contains("invalid type")
                || kind_error.contains("unknown field `normalized_kind`"),
            "{kind_error}"
        );
    }

    #[test]
    fn selected_empty_table_is_an_error_instead_of_no_encounter() {
        let data = sample_data();
        assert!(matches!(
            select_wild_encounter(
                &data,
                &slot_tables(),
                EncounterSurface::Grass,
                TimeOfDay::Night,
                1,
                0
            ),
            Err(EncounterError::EmptyEncounterSlots {
                map_name,
                surface: EncounterSurface::Grass,
                time: TimeOfDay::Night,
            }) if map_name == "ROUTE_29"
        ));
    }
}
