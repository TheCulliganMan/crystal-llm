use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::map::MapAttributes;
use crate::models::Item;
use crate::random::Random;
use crate::state::{FishingMemory, FishingRodState, GameState};
use crate::world::encounters::{TimeOfDay, WildEncounter};

pub const ROD_OLD: &str = "OLD_ROD";
pub const ROD_GOOD: &str = "GOOD_ROD";
pub const ROD_SUPER: &str = "SUPER_ROD";
pub const FISHING_RODS: &[&str] = &[ROD_OLD, ROD_GOOD, ROD_SUPER];
pub const FISHGROUP_NONE: &str = "FISHGROUP_NONE";

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FishingCatalog {
    pub groups: BTreeMap<String, FishingGroup>,
    pub time_groups: BTreeMap<String, TimeFishEntry>,
    pub swarm_rules: BTreeMap<String, FishingSwarmRule>,
    pub rod_items: BTreeMap<String, String>,
}

impl<'de> Deserialize<'de> for FishingCatalog {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawFishingCatalog {
            groups: BTreeMap<String, FishingGroup>,
            #[serde(deserialize_with = "required_fishing_time_groups")]
            time_groups: BTreeMap<String, TimeFishEntry>,
            swarm_rules: BTreeMap<String, FishingSwarmRule>,
            rod_items: BTreeMap<String, String>,
        }

        let raw = RawFishingCatalog::deserialize(deserializer)?;
        validate_fishing_token_keys("fishing group", raw.groups.keys())?;
        validate_fishing_token_keys("fishing time group", raw.time_groups.keys())?;
        validate_fishing_token_keys("fishing swarm rule", raw.swarm_rules.keys())?;
        validate_fishing_token_keys("fishing rod item", raw.rod_items.keys())?;
        validate_fishing_token_values("fishing rod item rod", raw.rod_items.values())?;
        Ok(Self {
            groups: raw.groups,
            time_groups: raw.time_groups,
            swarm_rules: raw.swarm_rules,
            rod_items: raw.rod_items,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FishingGroup {
    pub bite_threshold: u8,
    #[serde(deserialize_with = "required_fishing_rod_tables")]
    pub rod_tables: BTreeMap<String, RodTable>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RodTable {
    pub slots: Vec<FishingSlot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FishingSlot {
    pub threshold: u8,
    #[serde(deserialize_with = "required_nullable_fishing_token")]
    pub species: Option<String>,
    pub level: u8,
    #[serde(deserialize_with = "required_nullable_fishing_token")]
    pub time_group: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeFishEntry {
    #[serde(deserialize_with = "required_fishing_token")]
    pub day_species: String,
    pub day_level: u8,
    #[serde(deserialize_with = "required_fishing_token")]
    pub night_species: String,
    pub night_level: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FishingSwarmRule {
    pub daily_flag_bit: u8,
    pub swarm: u8,
    #[serde(deserialize_with = "required_fishing_token")]
    pub base_group: String,
    #[serde(deserialize_with = "required_fishing_token")]
    pub swarm_group: String,
}

fn validate_fishing_token_keys<E, I>(field: &str, values: I) -> Result<(), E>
where
    E: serde::de::Error,
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    validate_fishing_token_values(field, values)
}

fn validate_fishing_token_values<E, I>(field: &str, values: I) -> Result<(), E>
where
    E: serde::de::Error,
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    for value in values {
        let value = value.as_ref();
        if !is_exact_nonempty_fishing_token(value) {
            return Err(E::custom(format!(
                "{field} token must be exact ASCII alphanumeric/underscore, found {value:?}"
            )));
        }
    }
    Ok(())
}

fn required_fishing_time_groups<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<String, TimeFishEntry>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    BTreeMap::<String, TimeFishEntry>::deserialize(deserializer)
}

fn required_fishing_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_nonempty_fishing_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "fishing token must be exact ASCII alphanumeric/underscore, found {value:?}"
        )))
    }
}

fn required_nullable_fishing_token<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    match value {
        Some(token) if is_exact_nonempty_fishing_token(&token) => Ok(Some(token)),
        Some(token) => Err(serde::de::Error::custom(format!(
            "fishing token must be exact ASCII alphanumeric/underscore, found {token:?}"
        ))),
        None => Ok(None),
    }
}

fn required_fishing_rod_tables<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<String, RodTable>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values = BTreeMap::<String, RodTable>::deserialize(deserializer)?;
    validate_fishing_token_keys("fishing rod", values.keys())?;
    Ok(values)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum FishingCatalogIssue {
    MissingCatalog {
        map_name: String,
        group: String,
    },
    MissingRodItems,
    InvalidRodItemId {
        item_id: String,
    },
    InvalidRodItemRod {
        item_id: String,
        rod: String,
    },
    UnknownRodItemRod {
        item_id: String,
        rod: String,
    },
    UnknownRodItemId {
        item_id: String,
    },
    UnusableRodItem {
        item_id: String,
    },
    InvalidMapFishingGroup {
        map_name: String,
        group: String,
    },
    UnknownMapFishingGroup {
        map_name: String,
        group: String,
    },
    InvalidFishingGroupId {
        group_id: String,
    },
    InvalidFishingRod {
        group_id: String,
        rod: String,
    },
    UnknownFishingRod {
        group_id: String,
        rod: String,
    },
    EmptyFishingRodTable {
        group_id: String,
        rod: String,
    },
    InvalidFishingSlotThreshold {
        group_id: String,
        rod: String,
        slot_index: usize,
        threshold: u8,
    },
    UnorderedFishingSlotThreshold {
        group_id: String,
        rod: String,
        slot_index: usize,
        threshold: u8,
        previous: u8,
    },
    IncompleteFishingRodTable {
        group_id: String,
        rod: String,
        last_threshold: u8,
    },
    InvalidFishingSlotLevel {
        group_id: String,
        rod: String,
        slot_index: usize,
        level: u8,
    },
    MissingFishingSlotSpecies {
        group_id: String,
        rod: String,
        slot_index: usize,
    },
    InvalidFishingSpecies {
        group_id: String,
        species: String,
    },
    UnknownFishingSpecies {
        group_id: String,
        species: String,
    },
    UnknownFishingTimeGroup {
        group_id: String,
        time_group: String,
    },
    InvalidFishingTimeGroupSpecies {
        time_group: String,
        species: String,
    },
    UnknownFishingTimeGroupSpecies {
        time_group: String,
        species: String,
    },
    InvalidSwarmFlagBit {
        rule_id: String,
        daily_flag_bit: u8,
    },
    InvalidSwarmBaseGroup {
        rule_id: String,
    },
    UnknownSwarmBaseGroup {
        rule_id: String,
        base_group: String,
    },
    InvalidSwarmGroup {
        rule_id: String,
    },
    UnknownSwarmGroup {
        rule_id: String,
        swarm_group: String,
    },
    DuplicateSwarmRule {
        rule_id: String,
    },
    InvalidFishingTimeGroupId {
        time_group: String,
    },
    InvalidSwarmRuleId {
        rule_id: String,
    },
}

pub fn fishing_catalog_issues(
    catalog: &FishingCatalog,
    referenced_groups: &[(String, String)],
    items: &BTreeMap<String, Item>,
    species_ids: &BTreeSet<String>,
) -> Vec<FishingCatalogIssue> {
    let mut issues = Vec::new();
    if catalog.groups.is_empty() {
        for (map_name, group) in referenced_groups {
            issues.push(FishingCatalogIssue::MissingCatalog {
                map_name: map_name.clone(),
                group: group.clone(),
            });
        }
        return issues;
    }

    if catalog.rod_items.is_empty() {
        issues.push(FishingCatalogIssue::MissingRodItems);
    }
    for (item_id, rod) in &catalog.rod_items {
        if !is_exact_nonempty_fishing_token(item_id) {
            issues.push(FishingCatalogIssue::InvalidRodItemId {
                item_id: item_id.clone(),
            });
        }
        if !is_exact_nonempty_fishing_token(rod) {
            issues.push(FishingCatalogIssue::InvalidRodItemRod {
                item_id: item_id.clone(),
                rod: rod.clone(),
            });
        } else if !is_known_fishing_rod(rod) {
            issues.push(FishingCatalogIssue::UnknownRodItemRod {
                item_id: item_id.clone(),
                rod: rod.clone(),
            });
        }
        if is_exact_nonempty_fishing_token(item_id) {
            match items.get(item_id.as_str()) {
                Some(item) if !item.field_usable => {
                    issues.push(FishingCatalogIssue::UnusableRodItem {
                        item_id: item_id.clone(),
                    });
                }
                Some(_) => {}
                None => issues.push(FishingCatalogIssue::UnknownRodItemId {
                    item_id: item_id.clone(),
                }),
            }
        }
    }

    for (map_name, group) in referenced_groups {
        if !is_exact_nonempty_fishing_token(group) {
            issues.push(FishingCatalogIssue::InvalidMapFishingGroup {
                map_name: map_name.clone(),
                group: group.clone(),
            });
        } else if !catalog.groups.contains_key(group) {
            issues.push(FishingCatalogIssue::UnknownMapFishingGroup {
                map_name: map_name.clone(),
                group: group.clone(),
            });
        }
    }

    for (group_id, group) in &catalog.groups {
        if !is_exact_nonempty_fishing_token(group_id) {
            issues.push(FishingCatalogIssue::InvalidFishingGroupId {
                group_id: group_id.clone(),
            });
        }
        for (rod, table) in &group.rod_tables {
            if !is_exact_nonempty_fishing_token(rod) {
                issues.push(FishingCatalogIssue::InvalidFishingRod {
                    group_id: group_id.clone(),
                    rod: rod.clone(),
                });
            } else if !is_known_fishing_rod(rod) {
                issues.push(FishingCatalogIssue::UnknownFishingRod {
                    group_id: group_id.clone(),
                    rod: rod.clone(),
                });
            }
            if table.slots.is_empty() {
                issues.push(FishingCatalogIssue::EmptyFishingRodTable {
                    group_id: group_id.clone(),
                    rod: rod.clone(),
                });
            }
            let mut previous_threshold = 0;
            for (slot_index, slot) in table.slots.iter().enumerate() {
                if slot.threshold == 0 {
                    issues.push(FishingCatalogIssue::InvalidFishingSlotThreshold {
                        group_id: group_id.clone(),
                        rod: rod.clone(),
                        slot_index,
                        threshold: slot.threshold,
                    });
                }
                if slot.threshold < previous_threshold {
                    issues.push(FishingCatalogIssue::UnorderedFishingSlotThreshold {
                        group_id: group_id.clone(),
                        rod: rod.clone(),
                        slot_index,
                        threshold: slot.threshold,
                        previous: previous_threshold,
                    });
                }
                previous_threshold = slot.threshold;
                if slot.species.is_some() && slot.level == 0 {
                    issues.push(FishingCatalogIssue::InvalidFishingSlotLevel {
                        group_id: group_id.clone(),
                        rod: rod.clone(),
                        slot_index,
                        level: slot.level,
                    });
                }
                if slot.species.is_none() && slot.time_group.is_none() {
                    issues.push(FishingCatalogIssue::MissingFishingSlotSpecies {
                        group_id: group_id.clone(),
                        rod: rod.clone(),
                        slot_index,
                    });
                }
                if let Some(species) = slot.species.as_deref() {
                    if !is_exact_nonempty_fishing_token(species) {
                        issues.push(FishingCatalogIssue::InvalidFishingSpecies {
                            group_id: group_id.clone(),
                            species: species.to_string(),
                        });
                    } else if !species_ids.contains(species) {
                        issues.push(FishingCatalogIssue::UnknownFishingSpecies {
                            group_id: group_id.clone(),
                            species: species.to_string(),
                        });
                    }
                }
                if let Some(time_group) = slot.time_group.as_deref() {
                    let Some(entry) = catalog.time_groups.get(time_group) else {
                        issues.push(FishingCatalogIssue::UnknownFishingTimeGroup {
                            group_id: group_id.clone(),
                            time_group: time_group.to_string(),
                        });
                        continue;
                    };
                    for species in [&entry.day_species, &entry.night_species] {
                        if !is_exact_nonempty_fishing_token(species) {
                            issues.push(FishingCatalogIssue::InvalidFishingSpecies {
                                group_id: group_id.clone(),
                                species: species.clone(),
                            });
                        } else if !species_ids.contains(species.as_str()) {
                            issues.push(FishingCatalogIssue::UnknownFishingSpecies {
                                group_id: group_id.clone(),
                                species: species.clone(),
                            });
                        }
                    }
                }
            }
            if let Some(last_slot) = table.slots.last()
                && last_slot.threshold != u8::MAX
            {
                issues.push(FishingCatalogIssue::IncompleteFishingRodTable {
                    group_id: group_id.clone(),
                    rod: rod.clone(),
                    last_threshold: last_slot.threshold,
                });
            }
        }
    }

    for (time_group, entry) in &catalog.time_groups {
        if !is_exact_nonempty_fishing_token(time_group) {
            issues.push(FishingCatalogIssue::InvalidFishingTimeGroupId {
                time_group: time_group.clone(),
            });
        }
        for species in [&entry.day_species, &entry.night_species] {
            if !is_exact_nonempty_fishing_token(species) {
                issues.push(FishingCatalogIssue::InvalidFishingTimeGroupSpecies {
                    time_group: time_group.clone(),
                    species: species.clone(),
                });
            } else if !species_ids.contains(species.as_str()) {
                issues.push(FishingCatalogIssue::UnknownFishingTimeGroupSpecies {
                    time_group: time_group.clone(),
                    species: species.clone(),
                });
            }
        }
    }

    let mut seen_swarm_rules = BTreeSet::new();
    for (rule_id, rule) in &catalog.swarm_rules {
        if !is_exact_nonempty_fishing_token(rule_id) {
            issues.push(FishingCatalogIssue::InvalidSwarmRuleId {
                rule_id: rule_id.clone(),
            });
        }
        if rule.daily_flag_bit >= u8::BITS as u8 {
            issues.push(FishingCatalogIssue::InvalidSwarmFlagBit {
                rule_id: rule_id.clone(),
                daily_flag_bit: rule.daily_flag_bit,
            });
        }
        if !is_exact_nonempty_fishing_token(&rule.base_group) {
            issues.push(FishingCatalogIssue::InvalidSwarmBaseGroup {
                rule_id: rule_id.clone(),
            });
        } else if !catalog.groups.contains_key(&rule.base_group) {
            issues.push(FishingCatalogIssue::UnknownSwarmBaseGroup {
                rule_id: rule_id.clone(),
                base_group: rule.base_group.clone(),
            });
        }
        if !is_exact_nonempty_fishing_token(&rule.swarm_group) {
            issues.push(FishingCatalogIssue::InvalidSwarmGroup {
                rule_id: rule_id.clone(),
            });
        } else if !catalog.groups.contains_key(&rule.swarm_group) {
            issues.push(FishingCatalogIssue::UnknownSwarmGroup {
                rule_id: rule_id.clone(),
                swarm_group: rule.swarm_group.clone(),
            });
        }
        if !seen_swarm_rules.insert((rule.daily_flag_bit, rule.swarm, rule.base_group.as_str())) {
            issues.push(FishingCatalogIssue::DuplicateSwarmRule {
                rule_id: rule_id.clone(),
            });
        }
    }

    issues
}

fn is_exact_nonempty_fishing_token(value: &str) -> bool {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FishingOutcome {
    pub bite: bool,
    pub encounter: Option<WildEncounter>,
    pub group: Option<String>,
    pub bite_roll: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FishingSession {
    pub rod: String,
    pub outcome: FishingOutcome,
    pub start_frame: u64,
    pub bite_delay_frames: u64,
    pub group: Option<String>,
    pub cast_frames: u64,
    pub bites_remaining: u8,
    pub resolved: bool,
    pub resolution: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FishingRolledSession {
    pub session: FishingSession,
    pub bite_roll: u8,
    pub slot_roll: u8,
    pub rng_seed_after: u32,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum FishingError {
    #[error("cannot fish while using a surfing movement mode")]
    CannotFishWhileSurfing,
    #[error("cannot fish because the facing tile is outside the map")]
    FacingTileOutOfBounds,
    #[error("cannot fish because the facing tile is not water")]
    FacingTileIsNotWater,
    #[error("invalid fishing rod '{rod}'")]
    InvalidRod { rod: String },
    #[error("unknown fishing rod '{rod}'")]
    UnknownRod { rod: String },
    #[error("saved fishing.rod_index {index} is outside compiled Crystal rod range")]
    SavedRodIndexOutOfRange { index: u8 },
    #[error("saved fishing.rod_state {rod_state:?} requires fishing.rod_index")]
    SavedFishingStateMissingRodIndex { rod_state: FishingRodState },
    #[error(
        "saved fishing.rod_index {rod_index} resolves to {rod}, which is missing from compiled fishing rod tables"
    )]
    SavedRodTableMissing { rod_index: u8, rod: String },
    #[error("saved fishing.daily_flags1 bit {bit} is missing from compiled fishing swarm rules")]
    SavedDailyFlagMissingSwarmRule { bit: u32 },
    #[error("saved fishing.swarm_flag {swarm_flag} is missing from compiled fishing swarm rules")]
    SavedSwarmFlagMissingSwarmRule { swarm_flag: u8 },
    #[error("invalid fishing group '{group}'")]
    InvalidGroup { group: String },
    #[error("fishing group {group} is not defined")]
    UnknownGroup { group: String },
    #[error("fishing group '{group}' is missing the {rod} encounter table")]
    MissingRodTable { group: String, rod: String },
    #[error("fishing time group {time_group} is not defined")]
    MissingTimeGroup { time_group: String },
    #[error("fishing slot in {group}/{rod} resolved without a species")]
    MissingSlotSpecies { group: String, rod: String },
    #[error("fishing slot in {group}/{rod} resolved invalid species '{species}'")]
    InvalidSpecies {
        group: String,
        rod: String,
        species: String,
    },
    #[error("fishing slot roll {slot_roll} did not resolve within {group}/{rod}")]
    UnresolvedSlot {
        group: String,
        rod: String,
        slot_roll: u8,
    },
    #[error("item id '{item_id}' is not a fishing rod item")]
    UnknownRodItemId { item_id: String },
    #[error("item id '{item_id}' is not an exact fishing rod item id")]
    InvalidRodItemId { item_id: String },
}

pub fn percent_to_byte(percent: u8) -> u8 {
    ((u16::from(percent.min(100)) * 0xff) / 100) as u8
}

pub fn threshold(percent: u8, add_one: bool) -> u8 {
    let base = percent_to_byte(percent);
    if add_one {
        base.saturating_add(1)
    } else {
        base
    }
}

pub fn resolve_group_token(
    group: Option<&str>,
    state: Option<&GameState>,
    catalog: Option<&FishingCatalog>,
) -> Option<String> {
    let group = group?;
    if group.is_empty() || group == FISHGROUP_NONE {
        return None;
    }
    let (Some(state), Some(catalog)) = (state, catalog) else {
        return Some(group.to_string());
    };
    for rule in catalog.swarm_rules.values() {
        if rule.daily_flag_bit >= u8::BITS as u8 {
            continue;
        }
        let flag_mask = 1u8 << rule.daily_flag_bit;
        if state.fishing.daily_flags1 & flag_mask != 0
            && state.fishing.swarm_flag == rule.swarm
            && group == rule.base_group
        {
            return Some(rule.swarm_group.clone());
        }
    }
    Some(group.to_string())
}

pub fn resolve_group_for_map(
    attributes: &BTreeMap<String, MapAttributes>,
    map_name: &str,
    state: Option<&GameState>,
    catalog: Option<&FishingCatalog>,
) -> Option<String> {
    let group = attributes.get(map_name)?.fishing_group.as_deref();
    resolve_group_token(group, state, catalog)
}

pub fn roll_fishing_encounter(
    state: &GameState,
    catalog: &FishingCatalog,
    group: Option<&str>,
    rod: &str,
    time_of_day: TimeOfDay,
    bite_roll: u8,
    slot_roll: u8,
) -> Result<FishingOutcome, FishingError> {
    validate_rod(rod)?;
    let Some(group_name) = resolve_group_token(group, Some(state), Some(catalog)) else {
        return Ok(FishingOutcome {
            bite: false,
            encounter: None,
            group: None,
            bite_roll: 0,
        });
    };
    if !is_exact_nonempty_fishing_token(&group_name) {
        return Err(FishingError::InvalidGroup { group: group_name });
    }
    let fishing_group =
        catalog
            .groups
            .get(&group_name)
            .ok_or_else(|| FishingError::UnknownGroup {
                group: group_name.clone(),
            })?;
    if bite_roll >= fishing_group.bite_threshold {
        return Ok(FishingOutcome {
            bite: false,
            encounter: None,
            group: Some(group_name),
            bite_roll,
        });
    }
    let table = fishing_group
        .rod_tables
        .get(rod)
        .filter(|table| !table.slots.is_empty())
        .ok_or_else(|| FishingError::MissingRodTable {
            group: group_name.clone(),
            rod: rod.to_string(),
        })?;
    for slot in &table.slots {
        if slot_roll <= slot.threshold {
            let (species, level) = match (&slot.species, &slot.time_group) {
                (Some(species), _) => (species.clone(), slot.level),
                (None, Some(time_group)) => {
                    resolve_time_group(catalog, time_group.as_str(), time_of_day)?
                }
                (None, None) => {
                    return Err(FishingError::MissingSlotSpecies {
                        group: group_name,
                        rod: rod.to_string(),
                    });
                }
            };
            if !is_exact_nonempty_fishing_token(&species) {
                return Err(FishingError::InvalidSpecies {
                    group: group_name,
                    rod: rod.to_string(),
                    species,
                });
            }
            return Ok(FishingOutcome {
                bite: true,
                encounter: Some(WildEncounter { level, species }),
                group: Some(group_name),
                bite_roll,
            });
        }
    }
    Err(FishingError::UnresolvedSlot {
        group: group_name,
        rod: rod.to_string(),
        slot_roll,
    })
}

pub fn roll_fishing_encounter_from_rng(
    state: &GameState,
    catalog: &FishingCatalog,
    group: Option<&str>,
    rod: &str,
    time_of_day: TimeOfDay,
    rng: &mut Random,
) -> Result<FishingOutcome, FishingError> {
    let bite_roll = rng.randrange(256) as u8;
    let slot_roll = rng.randrange(256) as u8;
    roll_fishing_encounter(
        state,
        catalog,
        group,
        rod,
        time_of_day,
        bite_roll,
        slot_roll,
    )
}

pub fn do_fishing(
    state: &mut GameState,
    catalog: &FishingCatalog,
    group: Option<&str>,
    rod: &str,
    time_of_day: TimeOfDay,
    bite_roll: u8,
    slot_roll: u8,
) -> Result<FishingSession, FishingError> {
    let outcome = roll_fishing_encounter(
        state,
        catalog,
        group,
        rod,
        time_of_day,
        bite_roll,
        slot_roll,
    )?;
    let session = FishingSession {
        rod: rod.to_string(),
        outcome: outcome.clone(),
        start_frame: state.frame_counter,
        bite_delay_frames: 0,
        group: outcome.group.clone(),
        cast_frames: 40,
        bites_remaining: 1,
        resolved: false,
        resolution: None,
    };
    state.fishing.rod_state = FishingRodState::Waiting;
    state.fishing.rod_index = Some(rod_index(rod)?);
    state.fishing.bites_remaining = 1;
    state.fishing.result = 0;
    Ok(session)
}

pub fn do_fishing_from_rng(
    state: &mut GameState,
    catalog: &FishingCatalog,
    group: Option<&str>,
    rod: &str,
    time_of_day: TimeOfDay,
    rng: &mut Random,
) -> Result<FishingRolledSession, FishingError> {
    let bite_roll = rng.randrange(256) as u8;
    let slot_roll = rng.randrange(256) as u8;
    let session = do_fishing(
        state,
        catalog,
        group,
        rod,
        time_of_day,
        bite_roll,
        slot_roll,
    )?;
    Ok(FishingRolledSession {
        session,
        bite_roll,
        slot_roll,
        rng_seed_after: rng.seed(),
    })
}

pub fn fishing_rod_for_item_id<'a>(
    catalog: &'a FishingCatalog,
    item_id: &str,
) -> Result<&'a str, FishingError> {
    if !is_exact_nonempty_fishing_token(item_id) {
        return Err(FishingError::InvalidRodItemId {
            item_id: item_id.to_string(),
        });
    }
    let rod = catalog
        .rod_items
        .get(item_id)
        .ok_or_else(|| FishingError::UnknownRodItemId {
            item_id: item_id.to_string(),
        })?;
    validate_rod(rod)?;
    Ok(rod.as_str())
}

pub fn fishing_bite(
    state: &mut GameState,
    session: &mut FishingSession,
    current_frame: u64,
) -> Option<bool> {
    let elapsed = current_frame.saturating_sub(session.start_frame);
    let bite_frame = session.cast_frames + session.bite_delay_frames;
    if elapsed < bite_frame {
        return None;
    }
    if session.resolved {
        return session.resolution;
    }
    if !session.outcome.bite || session.outcome.encounter.is_none() {
        state.fishing.rod_state = FishingRodState::Idle;
        state.fishing.rod_index = None;
        state.fishing.bites_remaining = 0;
        state.fishing.result = if session.group.is_none() { 0 } else { 2 };
        session.resolved = true;
        session.resolution = Some(false);
        return Some(false);
    }
    state.fishing.rod_state = FishingRodState::Bite;
    state.fishing.result = 1;
    session.resolved = true;
    session.resolution = Some(true);
    Some(true)
}

pub fn fishing_battle_trigger(state: &mut GameState) {
    state.fishing.rod_state = FishingRodState::Battle;
    state.fishing.bites_remaining = 0;
}

fn resolve_time_group(
    catalog: &FishingCatalog,
    time_group: &str,
    time_of_day: TimeOfDay,
) -> Result<(String, u8), FishingError> {
    let entry =
        catalog
            .time_groups
            .get(time_group)
            .ok_or_else(|| FishingError::MissingTimeGroup {
                time_group: time_group.to_string(),
            })?;
    if time_of_day == TimeOfDay::Night {
        Ok((entry.night_species.clone(), entry.night_level))
    } else {
        Ok((entry.day_species.clone(), entry.day_level))
    }
}

pub fn is_known_fishing_rod(rod: &str) -> bool {
    FISHING_RODS.contains(&rod)
}

pub fn validate_rod(rod: &str) -> Result<(), FishingError> {
    if !is_exact_nonempty_fishing_token(rod) {
        return Err(FishingError::InvalidRod {
            rod: rod.to_string(),
        });
    }
    rod_index(rod).map(|_| ())
}

pub fn saved_fishing_rod_for_index(index: u8) -> Result<&'static str, FishingError> {
    match index {
        0 => Ok(ROD_OLD),
        1 => Ok(ROD_GOOD),
        2 => Ok(ROD_SUPER),
        _ => Err(FishingError::SavedRodIndexOutOfRange { index }),
    }
}

pub fn validate_saved_fishing_state(
    fishing: &FishingMemory,
) -> Result<Option<&'static str>, FishingError> {
    if let Some(rod_index) = fishing.rod_index {
        saved_fishing_rod_for_index(rod_index).map(Some)
    } else if fishing.rod_state != FishingRodState::Idle {
        Err(FishingError::SavedFishingStateMissingRodIndex {
            rod_state: fishing.rod_state,
        })
    } else {
        Ok(None)
    }
}

pub fn validate_saved_fishing_references<F, G, H>(
    fishing: &FishingMemory,
    has_rod_table: F,
    has_daily_flag_bit: G,
    has_swarm_flag: H,
) -> Result<(), FishingError>
where
    F: Fn(&str) -> bool,
    G: Fn(u32) -> bool,
    H: Fn(u8) -> bool,
{
    if let Some(rod) = validate_saved_fishing_state(fishing)? {
        if !has_rod_table(rod) {
            let rod_index = fishing.rod_index.expect("validated fishing rod index");
            return Err(FishingError::SavedRodTableMissing {
                rod_index,
                rod: rod.to_string(),
            });
        }
    }
    validate_saved_fishing_swarm_state(fishing, has_daily_flag_bit, has_swarm_flag)
}

pub fn validate_saved_fishing_swarm_state<F, G>(
    fishing: &FishingMemory,
    has_daily_flag_bit: F,
    has_swarm_flag: G,
) -> Result<(), FishingError>
where
    F: Fn(u32) -> bool,
    G: Fn(u8) -> bool,
{
    if fishing.daily_flags1 != 0 {
        for bit in 0..u8::BITS {
            let mask = 1u8 << bit;
            if fishing.daily_flags1 & mask != 0 && !has_daily_flag_bit(bit) {
                return Err(FishingError::SavedDailyFlagMissingSwarmRule { bit });
            }
        }
    }
    if fishing.swarm_flag != 0 && !has_swarm_flag(fishing.swarm_flag) {
        return Err(FishingError::SavedSwarmFlagMissingSwarmRule {
            swarm_flag: fishing.swarm_flag,
        });
    }
    Ok(())
}

fn rod_index(rod: &str) -> Result<u8, FishingError> {
    match rod {
        ROD_OLD => Ok(0),
        ROD_GOOD => Ok(1),
        ROD_SUPER => Ok(2),
        _ => Err(FishingError::UnknownRod {
            rod: rod.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ITEM_POCKET_KEY_ITEM, item_pocket};

    fn catalog() -> FishingCatalog {
        FishingCatalog {
            groups: [(
                "FISHGROUP_LAKE".to_string(),
                FishingGroup {
                    bite_threshold: threshold(50, true),
                    rod_tables: [(
                        ROD_GOOD.to_string(),
                        RodTable {
                            slots: vec![
                                FishingSlot {
                                    threshold: threshold(35, false),
                                    species: Some("MAGIKARP".to_string()),
                                    level: 20,
                                    time_group: None,
                                },
                                FishingSlot {
                                    threshold: threshold(100, false),
                                    species: None,
                                    level: 0,
                                    time_group: Some("TIME_GROUP_0".to_string()),
                                },
                            ],
                        },
                    )]
                    .into_iter()
                    .collect(),
                },
            )]
            .into_iter()
            .collect(),
            time_groups: [(
                "TIME_GROUP_0".to_string(),
                TimeFishEntry {
                    day_species: "CORSOLA".to_string(),
                    day_level: 20,
                    night_species: "STARYU".to_string(),
                    night_level: 20,
                },
            )]
            .into_iter()
            .collect(),
            swarm_rules: [(
                "SWARM_RULE_0".to_string(),
                FishingSwarmRule {
                    daily_flag_bit: 2,
                    swarm: 1,
                    base_group: "FISHGROUP_QWILFISH".to_string(),
                    swarm_group: "FISHGROUP_QWILFISH_SWARM".to_string(),
                },
            )]
            .into_iter()
            .collect(),
            rod_items: [("MOD_GOOD_ROD_ITEM".to_string(), ROD_GOOD.to_string())]
                .into_iter()
                .collect(),
        }
    }

    fn test_item(id: &str, field_usable: bool) -> Item {
        Item {
            name: id.to_string(),
            description: "Test item".to_string(),
            effect: "NONE".to_string(),
            status_heals: Vec::new(),
            revive_hp_percent: None,
            party_revive_hp_percent: None,
            pp_restore_scope: None,
            pp_restore_points: None,
            pp_up_stages: None,
            vitamin_stat: None,
            vitamin_stat_exp: None,
            vitamin_max_stat_exp: None,
            rare_candy_level_gain: None,
            battle_stat_boost_stat: None,
            battle_stat_boost_stages: None,
            battle_escape_mode: None,
            battle_capture_ball: None,
            battle_focus_energy: None,
            battle_stat_drop_guard: None,
            battle_stat_drop_guard_turns: None,
            confusion_heal: None,
            repel_steps: None,
            escape_rope_mode: None,
            price: 0,
            held_effect: "HELD_NONE".to_string(),
            parameter: 0,
            property: "NO_LIMITS".to_string(),
            pocket: item_pocket(ITEM_POCKET_KEY_ITEM),
            field_menu: if field_usable {
                "ITEMMENU_CLOSE".to_string()
            } else {
                "ITEMMENU_NOUSE".to_string()
            },
            field_usable,
            battle_menu: "ITEMMENU_NOUSE".to_string(),
            battle_usable: false,
            script_name: id.to_string(),
            consumable: false,
            tmhm_index: None,
            tmhm_move: None,
        }
    }

    #[test]
    fn exported_fishing_rod_set_is_exact() {
        assert_eq!(FISHING_RODS, &[ROD_OLD, ROD_GOOD, ROD_SUPER]);
        assert!(is_known_fishing_rod(ROD_OLD));
        assert!(is_known_fishing_rod(ROD_GOOD));
        assert!(is_known_fishing_rod(ROD_SUPER));
        assert!(!is_known_fishing_rod("old_rod"));
        assert!(!is_known_fishing_rod("GREAT_ROD"));
        assert_eq!(
            validate_rod("old_rod"),
            Err(FishingError::UnknownRod {
                rod: "old_rod".to_string(),
            })
        );
        assert_eq!(
            validate_rod("OLD ROD"),
            Err(FishingError::InvalidRod {
                rod: "OLD ROD".to_string(),
            })
        );
    }

    #[test]
    fn fishing_uses_exact_rod_and_group_ids_without_normalization() {
        let state = GameState::default();
        let catalog = catalog();
        assert_eq!(
            roll_fishing_encounter(
                &state,
                &catalog,
                Some("fishgroup_lake"),
                ROD_GOOD,
                TimeOfDay::Day,
                0,
                0,
            ),
            Err(FishingError::UnknownGroup {
                group: "fishgroup_lake".to_string(),
            })
        );
        assert_eq!(
            roll_fishing_encounter(
                &state,
                &catalog,
                Some("FISHGROUP LAKE"),
                ROD_GOOD,
                TimeOfDay::Day,
                0,
                0,
            ),
            Err(FishingError::InvalidGroup {
                group: "FISHGROUP LAKE".to_string(),
            })
        );
        assert_eq!(
            roll_fishing_encounter(
                &state,
                &catalog,
                Some("FISHGROUP_LAKE"),
                "good_rod",
                TimeOfDay::Day,
                0,
                0,
            ),
            Err(FishingError::UnknownRod {
                rod: "good_rod".to_string(),
            })
        );
    }

    #[test]
    fn fishing_catalog_issues_validate_definitive_catalog() {
        let referenced_groups = vec![
            ("ROUTE_32".to_string(), "FISHGROUP_LAKE".to_string()),
            ("ROUTE_42".to_string(), "FISHGROUP_MISSING".to_string()),
        ];
        assert_eq!(
            fishing_catalog_issues(
                &FishingCatalog::default(),
                &referenced_groups,
                &BTreeMap::new(),
                &BTreeSet::new(),
            ),
            vec![
                FishingCatalogIssue::MissingCatalog {
                    map_name: "ROUTE_32".to_string(),
                    group: "FISHGROUP_LAKE".to_string(),
                },
                FishingCatalogIssue::MissingCatalog {
                    map_name: "ROUTE_42".to_string(),
                    group: "FISHGROUP_MISSING".to_string(),
                },
            ],
        );

        let mut catalog = catalog();
        catalog.rod_items = [("OLD_ROD".to_string(), "BAD_ROD".to_string())]
            .into_iter()
            .collect();
        catalog.groups.insert(
            "FISHGROUP_BAD".to_string(),
            FishingGroup {
                bite_threshold: threshold(50, true),
                rod_tables: [(
                    "BAD_ROD".to_string(),
                    RodTable {
                        slots: vec![
                            FishingSlot {
                                threshold: threshold(35, false),
                                species: Some("MISSINGNO".to_string()),
                                level: 20,
                                time_group: None,
                            },
                            FishingSlot {
                                threshold: threshold(100, false),
                                species: None,
                                level: 0,
                                time_group: Some("TIME_GROUP_9".to_string()),
                            },
                        ],
                    },
                )]
                .into_iter()
                .collect(),
            },
        );
        catalog
            .time_groups
            .get_mut("TIME_GROUP_0")
            .expect("time group")
            .night_species = "MISSINGNO".to_string();
        catalog.swarm_rules = [
            (
                "SWARM_RULE_0".to_string(),
                FishingSwarmRule {
                    daily_flag_bit: 8,
                    swarm: 1,
                    base_group: " FISHGROUP_LAKE".to_string(),
                    swarm_group: String::new(),
                },
            ),
            (
                "SWARM_RULE_1".to_string(),
                FishingSwarmRule {
                    daily_flag_bit: 1,
                    swarm: 1,
                    base_group: "FISHGROUP_MISSING".to_string(),
                    swarm_group: "FISHGROUP_SWARM".to_string(),
                },
            ),
            (
                "SWARM_RULE_2".to_string(),
                FishingSwarmRule {
                    daily_flag_bit: 1,
                    swarm: 1,
                    base_group: "FISHGROUP_MISSING".to_string(),
                    swarm_group: "FISHGROUP_SWARM".to_string(),
                },
            ),
        ]
        .into_iter()
        .collect();
        let items = BTreeMap::from([("OLD_ROD".to_string(), test_item("OLD_ROD", true))]);
        let species_ids = BTreeSet::from(["MAGIKARP".to_string(), "CORSOLA".to_string()]);

        assert_eq!(
            fishing_catalog_issues(&catalog, &referenced_groups, &items, &species_ids),
            vec![
                FishingCatalogIssue::UnknownRodItemRod {
                    item_id: "OLD_ROD".to_string(),
                    rod: "BAD_ROD".to_string(),
                },
                FishingCatalogIssue::UnknownMapFishingGroup {
                    map_name: "ROUTE_42".to_string(),
                    group: "FISHGROUP_MISSING".to_string(),
                },
                FishingCatalogIssue::UnknownFishingRod {
                    group_id: "FISHGROUP_BAD".to_string(),
                    rod: "BAD_ROD".to_string(),
                },
                FishingCatalogIssue::UnknownFishingSpecies {
                    group_id: "FISHGROUP_BAD".to_string(),
                    species: "MISSINGNO".to_string(),
                },
                FishingCatalogIssue::UnknownFishingTimeGroup {
                    group_id: "FISHGROUP_BAD".to_string(),
                    time_group: "TIME_GROUP_9".to_string(),
                },
                FishingCatalogIssue::UnknownFishingSpecies {
                    group_id: "FISHGROUP_LAKE".to_string(),
                    species: "MISSINGNO".to_string(),
                },
                FishingCatalogIssue::UnknownFishingTimeGroupSpecies {
                    time_group: "TIME_GROUP_0".to_string(),
                    species: "MISSINGNO".to_string(),
                },
                FishingCatalogIssue::InvalidSwarmFlagBit {
                    rule_id: "SWARM_RULE_0".to_string(),
                    daily_flag_bit: 8,
                },
                FishingCatalogIssue::InvalidSwarmBaseGroup {
                    rule_id: "SWARM_RULE_0".to_string(),
                },
                FishingCatalogIssue::InvalidSwarmGroup {
                    rule_id: "SWARM_RULE_0".to_string(),
                },
                FishingCatalogIssue::UnknownSwarmBaseGroup {
                    rule_id: "SWARM_RULE_1".to_string(),
                    base_group: "FISHGROUP_MISSING".to_string(),
                },
                FishingCatalogIssue::UnknownSwarmGroup {
                    rule_id: "SWARM_RULE_1".to_string(),
                    swarm_group: "FISHGROUP_SWARM".to_string(),
                },
                FishingCatalogIssue::UnknownSwarmBaseGroup {
                    rule_id: "SWARM_RULE_2".to_string(),
                    base_group: "FISHGROUP_MISSING".to_string(),
                },
                FishingCatalogIssue::UnknownSwarmGroup {
                    rule_id: "SWARM_RULE_2".to_string(),
                    swarm_group: "FISHGROUP_SWARM".to_string(),
                },
                FishingCatalogIssue::DuplicateSwarmRule {
                    rule_id: "SWARM_RULE_2".to_string(),
                },
            ],
        );
    }

    #[test]
    fn fishing_catalog_issues_reject_unusable_rod_items() {
        let mut catalog = catalog();
        catalog.swarm_rules.clear();
        let items = BTreeMap::from([(
            "MOD_GOOD_ROD_ITEM".to_string(),
            test_item("MOD_GOOD_ROD_ITEM", false),
        )]);
        let species_ids = BTreeSet::from([
            "MAGIKARP".to_string(),
            "CORSOLA".to_string(),
            "STARYU".to_string(),
        ]);

        assert_eq!(
            fishing_catalog_issues(&catalog, &[], &items, &species_ids),
            vec![FishingCatalogIssue::UnusableRodItem {
                item_id: "MOD_GOOD_ROD_ITEM".to_string(),
            }],
        );
    }

    #[test]
    fn fishing_catalog_issues_reject_malformed_tokens_and_unusable_tables() {
        let catalog = FishingCatalog {
            groups: [
                (
                    " BAD".to_string(),
                    FishingGroup {
                        bite_threshold: threshold(50, true),
                        rod_tables: [(ROD_OLD.to_string(), RodTable { slots: Vec::new() })]
                            .into_iter()
                            .collect(),
                    },
                ),
                (
                    "BAD GROUP".to_string(),
                    FishingGroup {
                        bite_threshold: threshold(50, true),
                        rod_tables: [(ROD_OLD.to_string(), RodTable { slots: Vec::new() })]
                            .into_iter()
                            .collect(),
                    },
                ),
                (
                    "FISHGROUP_BAD".to_string(),
                    FishingGroup {
                        bite_threshold: threshold(50, true),
                        rod_tables: [(
                            " OLD_ROD".to_string(),
                            RodTable {
                                slots: vec![
                                    FishingSlot {
                                        threshold: 0,
                                        species: Some(" MAGIKARP".to_string()),
                                        level: 5,
                                        time_group: None,
                                    },
                                    FishingSlot {
                                        threshold: 10,
                                        species: Some("MAGI KARP".to_string()),
                                        level: 0,
                                        time_group: None,
                                    },
                                    FishingSlot {
                                        threshold: 5,
                                        species: None,
                                        level: 0,
                                        time_group: None,
                                    },
                                ],
                            },
                        )]
                        .into_iter()
                        .collect(),
                    },
                ),
            ]
            .into_iter()
            .collect(),
            time_groups: [(
                "TIME_GROUP_0".to_string(),
                TimeFishEntry {
                    day_species: " MAGIKARP".to_string(),
                    day_level: 10,
                    night_species: "STAR YU".to_string(),
                    night_level: 10,
                },
            )]
            .into_iter()
            .collect(),
            swarm_rules: [(
                "SWARM_RULE_0".to_string(),
                FishingSwarmRule {
                    daily_flag_bit: 1,
                    swarm: 1,
                    base_group: "FISHGROUP BAD".to_string(),
                    swarm_group: "FISHGROUP SWARM".to_string(),
                },
            )]
            .into_iter()
            .collect(),
            rod_items: [
                (" OLD_ROD".to_string(), ROD_OLD.to_string()),
                ("OLD ROD".to_string(), ROD_OLD.to_string()),
                ("MISSING_ROD".to_string(), ROD_OLD.to_string()),
                ("MISSING_ROD_2".to_string(), "OLD ROD".to_string()),
            ]
            .into_iter()
            .collect(),
        };
        let referenced_groups = vec![
            ("LAKE".to_string(), " FISHGROUP_BAD".to_string()),
            ("COVE".to_string(), "FISHGROUP BAD".to_string()),
            ("POND".to_string(), "FISHGROUP_MISSING".to_string()),
        ];
        let items = BTreeMap::from([("OLD_ROD".to_string(), test_item("OLD_ROD", true))]);
        let species_ids = BTreeSet::from(["MAGIKARP".to_string()]);

        assert_eq!(
            fishing_catalog_issues(&catalog, &referenced_groups, &items, &species_ids),
            vec![
                FishingCatalogIssue::InvalidRodItemId {
                    item_id: " OLD_ROD".to_string(),
                },
                FishingCatalogIssue::UnknownRodItemId {
                    item_id: "MISSING_ROD".to_string(),
                },
                FishingCatalogIssue::InvalidRodItemRod {
                    item_id: "MISSING_ROD_2".to_string(),
                    rod: "OLD ROD".to_string(),
                },
                FishingCatalogIssue::UnknownRodItemId {
                    item_id: "MISSING_ROD_2".to_string(),
                },
                FishingCatalogIssue::InvalidRodItemId {
                    item_id: "OLD ROD".to_string(),
                },
                FishingCatalogIssue::InvalidMapFishingGroup {
                    map_name: "LAKE".to_string(),
                    group: " FISHGROUP_BAD".to_string(),
                },
                FishingCatalogIssue::InvalidMapFishingGroup {
                    map_name: "COVE".to_string(),
                    group: "FISHGROUP BAD".to_string(),
                },
                FishingCatalogIssue::UnknownMapFishingGroup {
                    map_name: "POND".to_string(),
                    group: "FISHGROUP_MISSING".to_string(),
                },
                FishingCatalogIssue::InvalidFishingGroupId {
                    group_id: " BAD".to_string(),
                },
                FishingCatalogIssue::EmptyFishingRodTable {
                    group_id: " BAD".to_string(),
                    rod: ROD_OLD.to_string(),
                },
                FishingCatalogIssue::InvalidFishingGroupId {
                    group_id: "BAD GROUP".to_string(),
                },
                FishingCatalogIssue::EmptyFishingRodTable {
                    group_id: "BAD GROUP".to_string(),
                    rod: ROD_OLD.to_string(),
                },
                FishingCatalogIssue::InvalidFishingRod {
                    group_id: "FISHGROUP_BAD".to_string(),
                    rod: " OLD_ROD".to_string(),
                },
                FishingCatalogIssue::InvalidFishingSlotThreshold {
                    group_id: "FISHGROUP_BAD".to_string(),
                    rod: " OLD_ROD".to_string(),
                    slot_index: 0,
                    threshold: 0,
                },
                FishingCatalogIssue::InvalidFishingSpecies {
                    group_id: "FISHGROUP_BAD".to_string(),
                    species: " MAGIKARP".to_string(),
                },
                FishingCatalogIssue::InvalidFishingSlotLevel {
                    group_id: "FISHGROUP_BAD".to_string(),
                    rod: " OLD_ROD".to_string(),
                    slot_index: 1,
                    level: 0,
                },
                FishingCatalogIssue::InvalidFishingSpecies {
                    group_id: "FISHGROUP_BAD".to_string(),
                    species: "MAGI KARP".to_string(),
                },
                FishingCatalogIssue::UnorderedFishingSlotThreshold {
                    group_id: "FISHGROUP_BAD".to_string(),
                    rod: " OLD_ROD".to_string(),
                    slot_index: 2,
                    threshold: 5,
                    previous: 10,
                },
                FishingCatalogIssue::MissingFishingSlotSpecies {
                    group_id: "FISHGROUP_BAD".to_string(),
                    rod: " OLD_ROD".to_string(),
                    slot_index: 2,
                },
                FishingCatalogIssue::IncompleteFishingRodTable {
                    group_id: "FISHGROUP_BAD".to_string(),
                    rod: " OLD_ROD".to_string(),
                    last_threshold: 5,
                },
                FishingCatalogIssue::InvalidFishingTimeGroupSpecies {
                    time_group: "TIME_GROUP_0".to_string(),
                    species: " MAGIKARP".to_string(),
                },
                FishingCatalogIssue::InvalidFishingTimeGroupSpecies {
                    time_group: "TIME_GROUP_0".to_string(),
                    species: "STAR YU".to_string(),
                },
                FishingCatalogIssue::InvalidSwarmBaseGroup {
                    rule_id: "SWARM_RULE_0".to_string(),
                },
                FishingCatalogIssue::InvalidSwarmGroup {
                    rule_id: "SWARM_RULE_0".to_string(),
                },
            ],
        );
    }

    #[test]
    fn fishing_catalog_issues_reject_reserved_pack_prefix_tokens() {
        let catalog = FishingCatalog {
            groups: [(
                "fallback_group".to_string(),
                FishingGroup {
                    bite_threshold: threshold(50, true),
                    rod_tables: [(
                        "legacy_rod".to_string(),
                        RodTable {
                            slots: vec![FishingSlot {
                                threshold: 255,
                                species: Some("fallback_magikarp".to_string()),
                                level: 10,
                                time_group: Some("legacy_time_group".to_string()),
                            }],
                        },
                    )]
                    .into_iter()
                    .collect(),
                },
            )]
            .into_iter()
            .collect(),
            time_groups: [(
                "legacy_time_group".to_string(),
                TimeFishEntry {
                    day_species: "fallback_staryu".to_string(),
                    day_level: 10,
                    night_species: "legacy_corsola".to_string(),
                    night_level: 10,
                },
            )]
            .into_iter()
            .collect(),
            swarm_rules: [(
                "fallback_swarm_rule".to_string(),
                FishingSwarmRule {
                    daily_flag_bit: 1,
                    swarm: 1,
                    base_group: "fallback_group".to_string(),
                    swarm_group: "legacy_group".to_string(),
                },
            )]
            .into_iter()
            .collect(),
            rod_items: [("fallback_old_rod".to_string(), "legacy_rod".to_string())]
                .into_iter()
                .collect(),
        };

        assert_eq!(
            fishing_catalog_issues(&catalog, &[], &BTreeMap::new(), &BTreeSet::new()),
            vec![
                FishingCatalogIssue::InvalidRodItemId {
                    item_id: "fallback_old_rod".to_string(),
                },
                FishingCatalogIssue::InvalidRodItemRod {
                    item_id: "fallback_old_rod".to_string(),
                    rod: "legacy_rod".to_string(),
                },
                FishingCatalogIssue::InvalidFishingGroupId {
                    group_id: "fallback_group".to_string(),
                },
                FishingCatalogIssue::InvalidFishingRod {
                    group_id: "fallback_group".to_string(),
                    rod: "legacy_rod".to_string(),
                },
                FishingCatalogIssue::InvalidFishingSpecies {
                    group_id: "fallback_group".to_string(),
                    species: "fallback_magikarp".to_string(),
                },
                FishingCatalogIssue::InvalidFishingSpecies {
                    group_id: "fallback_group".to_string(),
                    species: "fallback_staryu".to_string(),
                },
                FishingCatalogIssue::InvalidFishingSpecies {
                    group_id: "fallback_group".to_string(),
                    species: "legacy_corsola".to_string(),
                },
                FishingCatalogIssue::InvalidFishingTimeGroupId {
                    time_group: "legacy_time_group".to_string(),
                },
                FishingCatalogIssue::InvalidFishingTimeGroupSpecies {
                    time_group: "legacy_time_group".to_string(),
                    species: "fallback_staryu".to_string(),
                },
                FishingCatalogIssue::InvalidFishingTimeGroupSpecies {
                    time_group: "legacy_time_group".to_string(),
                    species: "legacy_corsola".to_string(),
                },
                FishingCatalogIssue::InvalidSwarmRuleId {
                    rule_id: "fallback_swarm_rule".to_string(),
                },
                FishingCatalogIssue::InvalidSwarmBaseGroup {
                    rule_id: "fallback_swarm_rule".to_string(),
                },
                FishingCatalogIssue::InvalidSwarmGroup {
                    rule_id: "fallback_swarm_rule".to_string(),
                },
            ]
        );
    }

    #[test]
    fn no_fishing_group_returns_no_bite_without_catalog_lookup() {
        let outcome = roll_fishing_encounter(
            &GameState::default(),
            &FishingCatalog::default(),
            Some(FISHGROUP_NONE),
            ROD_GOOD,
            TimeOfDay::Day,
            0,
            0,
        )
        .expect("no group");

        assert_eq!(
            outcome,
            FishingOutcome {
                bite: false,
                encounter: None,
                group: None,
                bite_roll: 0,
            }
        );
    }

    #[test]
    fn bite_roll_and_slot_roll_resolve_pack_owned_encounter() {
        let outcome = roll_fishing_encounter(
            &GameState::default(),
            &catalog(),
            Some("FISHGROUP_LAKE"),
            ROD_GOOD,
            TimeOfDay::Day,
            0,
            231,
        )
        .expect("roll");

        assert_eq!(
            outcome.encounter,
            Some(WildEncounter {
                species: "CORSOLA".to_string(),
                level: 20,
            })
        );
    }

    #[test]
    fn fishing_rng_helper_consumes_bite_and_slot_rolls_in_core() {
        let mut helper_state = GameState {
            frame_counter: 11,
            ..GameState::default()
        };
        let mut explicit_state = helper_state.clone();
        let catalog = catalog();
        let mut helper_rng = Random::new(1);
        let mut explicit_rng = Random::new(1);
        let bite_roll = explicit_rng.randrange(256) as u8;
        let slot_roll = explicit_rng.randrange(256) as u8;

        let helper = do_fishing_from_rng(
            &mut helper_state,
            &catalog,
            Some("FISHGROUP_LAKE"),
            ROD_GOOD,
            TimeOfDay::Day,
            &mut helper_rng,
        )
        .expect("helper session");
        let explicit = do_fishing(
            &mut explicit_state,
            &catalog,
            Some("FISHGROUP_LAKE"),
            ROD_GOOD,
            TimeOfDay::Day,
            bite_roll,
            slot_roll,
        )
        .expect("explicit session");

        assert_eq!(helper.session, explicit);
        assert_eq!(helper.bite_roll, bite_roll);
        assert_eq!(helper.slot_roll, slot_roll);
        assert_eq!(helper.rng_seed_after, explicit_rng.seed());
        assert_eq!(helper_rng.seed(), explicit_rng.seed());
    }

    #[test]
    fn fishing_rejects_malformed_runtime_slot_species() {
        let mut catalog = catalog();
        catalog
            .groups
            .get_mut("FISHGROUP_LAKE")
            .expect("group")
            .rod_tables
            .get_mut(ROD_GOOD)
            .expect("rod")
            .slots[0]
            .species = Some("MAGI KARP".to_string());

        assert_eq!(
            roll_fishing_encounter(
                &GameState::default(),
                &catalog,
                Some("FISHGROUP_LAKE"),
                ROD_GOOD,
                TimeOfDay::Day,
                0,
                0,
            ),
            Err(FishingError::InvalidSpecies {
                group: "FISHGROUP_LAKE".to_string(),
                rod: ROD_GOOD.to_string(),
                species: "MAGI KARP".to_string(),
            })
        );
    }

    #[test]
    fn fishing_catalog_json_rejects_unknown_modpack_fields() {
        let error = serde_json::from_str::<FishingCatalog>(
            r#"{
              "groups":{
                "FISHGROUP_LAKE":{
                  "bite_threshold":128,
                  "rod_tables":{
                    "GOOD_ROD":{
                      "slots":[
                        {"threshold":255,"species":"MAGIKARP","level":20,"time_group":null,"fallback_species":"TENTACOOL"}
                      ]
                    }
                  }
                }
              },
              "time_groups":{},
              "swarm_rules":{},
              "rod_items":{}
            }"#,
        )
        .expect_err("fishing slots must not accept fallback species")
        .to_string();

        assert!(
            error.contains("unknown field `fallback_species`"),
            "{error}"
        );
    }

    #[test]
    fn fishing_catalog_json_rejects_malformed_pack_tokens_at_deserialization() {
        let cases = [
            (
                "missing time groups",
                serde_json::json!({
                    "groups": {},
                    "swarm_rules": {},
                    "rod_items": {}
                }),
            ),
            (
                "missing swarm rules",
                serde_json::json!({
                    "groups": {},
                    "time_groups": {},
                    "rod_items": {}
                }),
            ),
            (
                "missing rod items",
                serde_json::json!({
                    "groups": {},
                    "time_groups": {},
                    "swarm_rules": {}
                }),
            ),
            (
                "legacy time group list",
                serde_json::json!({
                    "groups": {},
                    "time_groups": [],
                    "swarm_rules": {},
                    "rod_items": {}
                }),
            ),
            (
                "group key",
                serde_json::json!({
                    "groups": {
                        "FISH GROUP_LAKE": {
                            "bite_threshold": 128,
                            "rod_tables": {}
                        }
                    },
                    "time_groups": {},
                    "swarm_rules": {},
                    "rod_items": {}
                }),
            ),
            (
                "rod key",
                serde_json::json!({
                    "groups": {
                        "FISHGROUP_LAKE": {
                            "bite_threshold": 128,
                            "rod_tables": {
                                "GOOD ROD": {"slots": []}
                            }
                        }
                    },
                    "time_groups": {},
                    "swarm_rules": {},
                    "rod_items": {}
                }),
            ),
            (
                "slot species",
                serde_json::json!({
                    "groups": {
                        "FISHGROUP_LAKE": {
                            "bite_threshold": 128,
                            "rod_tables": {
                                "GOOD_ROD": {
                                    "slots": [{"threshold": 255, "species": "MAGI KARP", "level": 20, "time_group": null}]
                                }
                            }
                        }
                    },
                    "time_groups": {},
                    "swarm_rules": {},
                    "rod_items": {}
                }),
            ),
            (
                "slot time group",
                serde_json::json!({
                    "groups": {
                        "FISHGROUP_LAKE": {
                            "bite_threshold": 128,
                            "rod_tables": {
                                "GOOD_ROD": {
                                    "slots": [{"threshold": 255, "species": null, "level": 20, "time_group": "TIME GROUP"}]
                                }
                            }
                        }
                    },
                    "time_groups": {},
                    "swarm_rules": {},
                    "rod_items": {}
                }),
            ),
            (
                "time group key",
                serde_json::json!({
                    "groups": {},
                    "time_groups": {
                        "TIME GROUP": {"day_species": "MAGIKARP", "day_level": 10, "night_species": "STARYU", "night_level": 20}
                    },
                    "swarm_rules": {},
                    "rod_items": {}
                }),
            ),
            (
                "time group species",
                serde_json::json!({
                    "groups": {},
                    "time_groups": {
                        "TIME_GROUP": {"day_species": "MAGI KARP", "day_level": 10, "night_species": "STARYU", "night_level": 20}
                    },
                    "swarm_rules": {},
                    "rod_items": {}
                }),
            ),
            (
                "swarm rule key",
                serde_json::json!({
                    "groups": {},
                    "time_groups": {},
                    "swarm_rules": {
                        "SWARM RULE": {"daily_flag_bit": 0, "swarm": 1, "base_group": "FISHGROUP_LAKE", "swarm_group": "FISHGROUP_SWARM"}
                    },
                    "rod_items": {}
                }),
            ),
            (
                "swarm group",
                serde_json::json!({
                    "groups": {},
                    "time_groups": {},
                    "swarm_rules": {
                        "SWARM_RULE": {"daily_flag_bit": 0, "swarm": 1, "base_group": "FISHGROUP LAKE", "swarm_group": "FISHGROUP_SWARM"}
                    },
                    "rod_items": {}
                }),
            ),
            (
                "rod item key",
                serde_json::json!({
                    "groups": {},
                    "time_groups": {},
                    "swarm_rules": {},
                    "rod_items": {"GOOD ROD": "GOOD_ROD"}
                }),
            ),
            (
                "rod item value",
                serde_json::json!({
                    "groups": {},
                    "time_groups": {},
                    "swarm_rules": {},
                    "rod_items": {"GOOD_ROD": "GOOD ROD"}
                }),
            ),
        ];

        for (label, payload) in cases {
            let error = serde_json::from_value::<FishingCatalog>(payload)
                .expect_err("malformed fishing catalog tokens must fail during JSON load")
                .to_string();

            assert!(
                (error.contains("fishing")
                    && error.contains("token must be exact ASCII alphanumeric/underscore"))
                    || error.contains("missing field")
                    || error.contains("invalid type"),
                "{label} produced unexpected error: {error}"
            );
        }
    }

    #[test]
    fn fishing_catalog_issue_json_rejects_unknown_fallback_fields() {
        let error = serde_json::from_value::<FishingCatalogIssue>(serde_json::json!({
            "UnknownFishingSpecies": {
                "group_id": "FISHGROUP_LAKE",
                "species": "MODKARP",
                "fallback_species": "MAGIKARP"
            }
        }))
        .expect_err("fallback species must be rejected")
        .to_string();

        assert!(
            error.contains("unknown field `fallback_species`"),
            "{error}"
        );
    }

    #[test]
    fn night_time_group_uses_night_species() {
        let outcome = roll_fishing_encounter(
            &GameState::default(),
            &catalog(),
            Some("FISHGROUP_LAKE"),
            ROD_GOOD,
            TimeOfDay::Night,
            0,
            231,
        )
        .expect("roll");

        assert_eq!(
            outcome.encounter,
            Some(WildEncounter {
                species: "STARYU".to_string(),
                level: 20,
            })
        );
    }

    #[test]
    fn fishing_rejects_reserved_runtime_slot_species() {
        let mut catalog = catalog();
        catalog
            .groups
            .get_mut("FISHGROUP_LAKE")
            .expect("group")
            .rod_tables
            .get_mut(ROD_GOOD)
            .expect("rod")
            .slots[0]
            .species = Some("fallback_magikarp".to_string());

        assert_eq!(
            roll_fishing_encounter(
                &GameState::default(),
                &catalog,
                Some("FISHGROUP_LAKE"),
                ROD_GOOD,
                TimeOfDay::Day,
                0,
                0,
            ),
            Err(FishingError::InvalidSpecies {
                group: "FISHGROUP_LAKE".to_string(),
                rod: ROD_GOOD.to_string(),
                species: "fallback_magikarp".to_string(),
            })
        );
    }

    #[test]
    fn fishing_rejects_malformed_runtime_time_group_species() {
        let mut catalog = catalog();
        catalog
            .time_groups
            .get_mut("TIME_GROUP_0")
            .expect("time group")
            .night_species = "STAR YU".to_string();

        assert_eq!(
            roll_fishing_encounter(
                &GameState::default(),
                &catalog,
                Some("FISHGROUP_LAKE"),
                ROD_GOOD,
                TimeOfDay::Night,
                0,
                231,
            ),
            Err(FishingError::InvalidSpecies {
                group: "FISHGROUP_LAKE".to_string(),
                rod: ROD_GOOD.to_string(),
                species: "STAR YU".to_string(),
            })
        );
    }

    #[test]
    fn swarm_flags_remap_exact_base_groups() {
        let mut state = GameState::default();
        state.fishing.daily_flags1 = 1 << 2;
        state.fishing.swarm_flag = 1;
        let catalog = catalog();

        assert_eq!(
            resolve_group_token(Some("FISHGROUP_QWILFISH"), Some(&state), Some(&catalog)),
            Some("FISHGROUP_QWILFISH_SWARM".to_string())
        );
        assert_eq!(
            resolve_group_token(Some("fishgroup_qwilfish"), Some(&state), Some(&catalog)),
            Some("fishgroup_qwilfish".to_string())
        );
    }

    #[test]
    fn saved_fishing_references_require_compiled_rod_tables() {
        let mut memory = FishingMemory {
            rod_state: FishingRodState::Waiting,
            rod_index: Some(1),
            daily_flags1: 1 << 2,
            swarm_flag: 1,
            ..FishingMemory::default()
        };

        assert_eq!(
            validate_saved_fishing_references(&memory, |_| false, |_| true, |_| true),
            Err(FishingError::SavedRodTableMissing {
                rod_index: 1,
                rod: ROD_GOOD.to_string(),
            })
        );
        assert_eq!(
            validate_saved_fishing_references(
                &memory,
                |rod| rod == ROD_GOOD,
                |bit| bit == 2,
                |swarm| swarm == 1
            ),
            Ok(())
        );

        memory.daily_flags1 = 1 << 3;
        assert_eq!(
            validate_saved_fishing_references(
                &memory,
                |rod| rod == ROD_GOOD,
                |bit| bit == 2,
                |_| true
            ),
            Err(FishingError::SavedDailyFlagMissingSwarmRule { bit: 3 })
        );
    }

    #[test]
    fn fishing_session_updates_runtime_memory_and_bite_state() {
        let mut state = GameState {
            frame_counter: 7,
            ..GameState::default()
        };
        let mut session = do_fishing(
            &mut state,
            &catalog(),
            Some("FISHGROUP_LAKE"),
            ROD_GOOD,
            TimeOfDay::Day,
            0,
            0,
        )
        .expect("session");

        assert_eq!(state.fishing.rod_state, FishingRodState::Waiting);
        assert_eq!(state.fishing.rod_index, Some(1));
        assert_eq!(fishing_bite(&mut state, &mut session, 46), None);
        assert_eq!(fishing_bite(&mut state, &mut session, 47), Some(true));
        assert_eq!(state.fishing.rod_state, FishingRodState::Bite);
        assert_eq!(state.fishing.result, 1);

        fishing_battle_trigger(&mut state);
        assert_eq!(state.fishing.rod_state, FishingRodState::Battle);
        assert_eq!(state.fishing.bites_remaining, 0);
    }

    #[test]
    fn fishing_rod_items_are_resolved_from_exact_pack_rules() {
        let catalog = catalog();

        assert_eq!(
            fishing_rod_for_item_id(&catalog, "MOD_GOOD_ROD_ITEM").expect("mod rod rule"),
            ROD_GOOD
        );
        assert_eq!(
            fishing_rod_for_item_id(&catalog, "GOOD_ROD")
                .expect_err("canonical item id is not inferred"),
            FishingError::UnknownRodItemId {
                item_id: "GOOD_ROD".to_string(),
            }
        );
        assert_eq!(
            fishing_rod_for_item_id(&catalog, "GOOD ROD")
                .expect_err("malformed rod item id is invalid"),
            FishingError::InvalidRodItemId {
                item_id: "GOOD ROD".to_string(),
            }
        );
    }
}
