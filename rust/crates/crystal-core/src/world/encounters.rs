use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EncounterSurface {
    Grass,
    Water,
    Rock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WildEncounter {
    pub level: u8,
    pub species: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WildEncounterTable {
    pub morning: Vec<WildEncounter>,
    pub day: Vec<WildEncounter>,
    pub night: Vec<WildEncounter>,
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
            map_name: String,
            #[serde(deserialize_with = "required_nullable")]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedWildEncounter {
    pub encounter: WildEncounter,
    pub slot: usize,
    pub level: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncounterSlotChance {
    pub threshold: u8,
    pub slot: usize,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncounterSlotTables {
    pub grass: Vec<EncounterSlotChance>,
    pub water: Vec<EncounterSlotChance>,
}

impl EncounterSlotTables {
    pub fn table_for_surface(
        &self,
        surface: EncounterSurface,
    ) -> Result<&[EncounterSlotChance], EncounterError> {
        let table = match surface {
            EncounterSurface::Water => &self.water,
            EncounterSurface::Grass | EncounterSurface::Rock => &self.grass,
        };
        if table.is_empty() {
            return Err(EncounterError::MissingEncounterSlotTable { surface });
        }
        Ok(table)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterMusicModifier {
    pub music_id: String,
    pub numerator: u8,
    pub denominator: u8,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterMusicModifiers {
    pub modifiers: Vec<EncounterMusicModifier>,
}

impl EncounterMusicModifiers {
    pub fn modifier_for(&self, music_id: &str) -> Option<&EncounterMusicModifier> {
        self.modifiers
            .iter()
            .find(|modifier| modifier.music_id == music_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldEncounterEntry {
    pub weight: u8,
    pub species: String,
    pub level: u8,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldEncounterTable {
    pub common: Vec<FieldEncounterEntry>,
    pub rare: Vec<FieldEncounterEntry>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct FieldEncounterData {
    pub map_name: String,
    pub headbutt: Option<FieldEncounterTable>,
    pub rock_smash: Option<FieldEncounterTable>,
}

impl<'de> Deserialize<'de> for FieldEncounterData {
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
        struct RawFieldEncounterData {
            map_name: String,
            #[serde(deserialize_with = "required_nullable")]
            headbutt: Option<FieldEncounterTable>,
            #[serde(deserialize_with = "required_nullable")]
            rock_smash: Option<FieldEncounterTable>,
        }

        let raw = RawFieldEncounterData::deserialize(deserializer)?;
        Ok(Self {
            map_name: raw.map_name,
            headbutt: raw.headbutt,
            rock_smash: raw.rock_smash,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldEncounterKind {
    Headbutt,
    RockSmash,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldEncounterRoll {
    pub map_name: String,
    pub kind: FieldEncounterKind,
    pub target_tile_x: i16,
    pub target_tile_y: i16,
    pub score: Option<u8>,
    pub chance_roll: u8,
    pub entry_roll: Option<u8>,
    pub resolved: Option<ResolvedWildEncounter>,
}

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
            music_id: modifier.music_id.clone(),
        });
    }
    Ok(
        ((u16::from(threshold) * u16::from(modifier.numerator)) / u16::from(modifier.denominator))
            as u8,
    )
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

pub fn apply_grass_level_variance(base_level: u8, surface: EncounterSurface, roll_byte: u8) -> u8 {
    if surface != EncounterSurface::Grass {
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
    let level = apply_grass_level_variance(encounter.level, surface, level_roll_byte);
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
        resolved,
        repelled_by: None,
        rng_seed_after: 0,
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
    let table =
        data.headbutt
            .as_ref()
            .ok_or_else(|| EncounterError::MissingFieldEncounterTable {
                map_name: data.map_name.clone(),
                kind: FieldEncounterKind::Headbutt,
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

pub fn select_rock_smash_encounter(
    data: &FieldEncounterData,
    target_tile_x: i16,
    target_tile_y: i16,
    chance_roll: u8,
    entry_roll: u8,
) -> Result<FieldEncounterRoll, EncounterError> {
    let table =
        data.rock_smash
            .as_ref()
            .ok_or_else(|| EncounterError::MissingFieldEncounterTable {
                map_name: data.map_name.clone(),
                kind: FieldEncounterKind::RockSmash,
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
        FieldEncounterData {
            map_name: "Route29".to_string(),
            headbutt: Some(FieldEncounterTable {
                common: vec![FieldEncounterEntry {
                    weight: 100,
                    species: "HOOTHOOT".to_string(),
                    level: 10,
                }],
                rare: vec![FieldEncounterEntry {
                    weight: 100,
                    species: "PINECO".to_string(),
                    level: 10,
                }],
            }),
            rock_smash: Some(FieldEncounterTable {
                common: vec![
                    FieldEncounterEntry {
                        weight: 90,
                        species: "KRABBY".to_string(),
                        level: 15,
                    },
                    FieldEncounterEntry {
                        weight: 10,
                        species: "SHUCKLE".to_string(),
                        level: 15,
                    },
                ],
                rare: Vec::new(),
            }),
        }
    }

    fn slot_tables() -> EncounterSlotTables {
        EncounterSlotTables {
            grass: vec![
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
            water: vec![
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
        }
    }

    fn music_modifiers() -> EncounterMusicModifiers {
        EncounterMusicModifiers {
            modifiers: vec![
                EncounterMusicModifier {
                    music_id: "MUSIC_POKEMON_MARCH".to_string(),
                    numerator: 2,
                    denominator: 1,
                },
                EncounterMusicModifier {
                    music_id: "MUSIC_RUINS_OF_ALPH_RADIO".to_string(),
                    numerator: 2,
                    denominator: 1,
                },
                EncounterMusicModifier {
                    music_id: "MUSIC_POKEMON_LULLABY".to_string(),
                    numerator: 1,
                    denominator: 2,
                },
            ],
        }
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
    fn grass_level_variance_uses_same_thresholds_as_typescript() {
        assert_eq!(
            apply_grass_level_variance(5, EncounterSurface::Water, 255),
            5
        );
        assert_eq!(apply_grass_level_variance(5, EncounterSurface::Grass, 0), 5);
        assert_eq!(
            apply_grass_level_variance(5, EncounterSurface::Grass, 90),
            6
        );
        assert_eq!(
            apply_grass_level_variance(5, EncounterSurface::Grass, 166),
            7
        );
        assert_eq!(
            apply_grass_level_variance(5, EncounterSurface::Grass, 243),
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
        assert_eq!(resolved.level, 7);
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
    fn field_encounters_require_modpack_tables_and_selected_buckets() {
        let mut data = field_data();
        data.headbutt = None;
        assert!(matches!(
            select_headbutt_encounter(&data, 0, 2, 0, 2, 54),
            Err(EncounterError::MissingFieldEncounterTable {
                map_name,
                kind: FieldEncounterKind::Headbutt,
            }) if map_name == "Route29"
        ));

        let mut data = field_data();
        data.headbutt.as_mut().expect("headbutt").rare.clear();
        assert!(matches!(
            select_headbutt_encounter(&data, 0, 2, 0, 2, 54),
            Err(EncounterError::EmptyFieldEncounterEntries {
                map_name,
                kind: FieldEncounterKind::Headbutt,
                bucket: "rare",
            }) if map_name == "Route29"
        ));

        let mut data = field_data();
        data.rock_smash = None;
        assert!(matches!(
            select_rock_smash_encounter(&data, 4, 6, 2, 90),
            Err(EncounterError::MissingFieldEncounterTable {
                map_name,
                kind: FieldEncounterKind::RockSmash,
            }) if map_name == "Route29"
        ));

        let mut data = field_data();
        data.rock_smash.as_mut().expect("rock smash").common.clear();
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
