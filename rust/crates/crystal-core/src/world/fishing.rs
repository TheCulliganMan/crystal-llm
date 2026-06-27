use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::map::MapAttributes;
use crate::state::{FishingRodState, GameState};
use crate::world::encounters::{TimeOfDay, WildEncounter};

pub const ROD_OLD: &str = "OLD_ROD";
pub const ROD_GOOD: &str = "GOOD_ROD";
pub const ROD_SUPER: &str = "SUPER_ROD";
pub const FISHING_RODS: &[&str] = &[ROD_OLD, ROD_GOOD, ROD_SUPER];
pub const FISHGROUP_NONE: &str = "FISHGROUP_NONE";

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FishingCatalog {
    pub groups: BTreeMap<String, FishingGroup>,
    pub time_groups: Vec<TimeFishEntry>,
    pub swarm_rules: Vec<FishingSwarmRule>,
    pub rod_items: Vec<FishingRodItemRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FishingGroup {
    pub bite_threshold: u8,
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
    pub species: Option<String>,
    pub level: u8,
    pub time_group: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeFishEntry {
    pub day_species: String,
    pub day_level: u8,
    pub night_species: String,
    pub night_level: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FishingSwarmRule {
    pub daily_flag_bit: u8,
    pub swarm: u8,
    pub base_group: String,
    pub swarm_group: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FishingRodItemRule {
    pub item_id: String,
    pub rod: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FishingCatalogIssue {
    MissingCatalog {
        map_name: String,
        group: String,
    },
    MissingRodItems,
    InvalidRodItemId {
        item_id: String,
    },
    DuplicateRodItemId {
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
        time_group: usize,
    },
    InvalidFishingTimeGroupSpecies {
        index: usize,
        species: String,
    },
    UnknownFishingTimeGroupSpecies {
        index: usize,
        species: String,
    },
    InvalidSwarmFlagBit {
        index: usize,
        daily_flag_bit: u8,
    },
    InvalidSwarmBaseGroup {
        index: usize,
    },
    UnknownSwarmBaseGroup {
        index: usize,
        base_group: String,
    },
    InvalidSwarmGroup {
        index: usize,
    },
    UnknownSwarmGroup {
        index: usize,
        swarm_group: String,
    },
    DuplicateSwarmRule {
        index: usize,
    },
}

pub fn fishing_catalog_issues(
    catalog: &FishingCatalog,
    referenced_groups: &[(String, String)],
    item_ids: &BTreeSet<String>,
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
    let mut rod_item_ids = BTreeSet::new();
    for rule in &catalog.rod_items {
        if !is_exact_nonempty_fishing_token(&rule.item_id) {
            issues.push(FishingCatalogIssue::InvalidRodItemId {
                item_id: rule.item_id.clone(),
            });
        }
        if !rod_item_ids.insert(rule.item_id.as_str()) {
            issues.push(FishingCatalogIssue::DuplicateRodItemId {
                item_id: rule.item_id.clone(),
            });
        }
        if !is_exact_nonempty_fishing_token(&rule.rod) {
            issues.push(FishingCatalogIssue::InvalidRodItemRod {
                item_id: rule.item_id.clone(),
                rod: rule.rod.clone(),
            });
        } else if !is_known_fishing_rod(&rule.rod) {
            issues.push(FishingCatalogIssue::UnknownRodItemRod {
                item_id: rule.item_id.clone(),
                rod: rule.rod.clone(),
            });
        }
        if is_exact_nonempty_fishing_token(&rule.item_id)
            && !item_ids.contains(rule.item_id.as_str())
        {
            issues.push(FishingCatalogIssue::UnknownRodItemId {
                item_id: rule.item_id.clone(),
            });
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
                if let Some(time_group) = slot.time_group {
                    let Some(entry) = catalog.time_groups.get(time_group) else {
                        issues.push(FishingCatalogIssue::UnknownFishingTimeGroup {
                            group_id: group_id.clone(),
                            time_group,
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

    for (index, entry) in catalog.time_groups.iter().enumerate() {
        for species in [&entry.day_species, &entry.night_species] {
            if !is_exact_nonempty_fishing_token(species) {
                issues.push(FishingCatalogIssue::InvalidFishingTimeGroupSpecies {
                    index,
                    species: species.clone(),
                });
            } else if !species_ids.contains(species.as_str()) {
                issues.push(FishingCatalogIssue::UnknownFishingTimeGroupSpecies {
                    index,
                    species: species.clone(),
                });
            }
        }
    }

    let mut seen_swarm_rules = BTreeSet::new();
    for (index, rule) in catalog.swarm_rules.iter().enumerate() {
        if rule.daily_flag_bit >= u8::BITS as u8 {
            issues.push(FishingCatalogIssue::InvalidSwarmFlagBit {
                index,
                daily_flag_bit: rule.daily_flag_bit,
            });
        }
        if !is_exact_nonempty_fishing_token(&rule.base_group) {
            issues.push(FishingCatalogIssue::InvalidSwarmBaseGroup { index });
        } else if !catalog.groups.contains_key(&rule.base_group) {
            issues.push(FishingCatalogIssue::UnknownSwarmBaseGroup {
                index,
                base_group: rule.base_group.clone(),
            });
        }
        if !is_exact_nonempty_fishing_token(&rule.swarm_group) {
            issues.push(FishingCatalogIssue::InvalidSwarmGroup { index });
        } else if !catalog.groups.contains_key(&rule.swarm_group) {
            issues.push(FishingCatalogIssue::UnknownSwarmGroup {
                index,
                swarm_group: rule.swarm_group.clone(),
            });
        }
        if !seen_swarm_rules.insert((rule.daily_flag_bit, rule.swarm, rule.base_group.as_str())) {
            issues.push(FishingCatalogIssue::DuplicateSwarmRule { index });
        }
    }

    issues
}

fn is_exact_nonempty_fishing_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
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

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum FishingError {
    #[error("invalid fishing rod '{rod}'")]
    InvalidRod { rod: String },
    #[error("unknown fishing rod '{rod}'")]
    UnknownRod { rod: String },
    #[error("invalid fishing group '{group}'")]
    InvalidGroup { group: String },
    #[error("fishing group {group} is not defined")]
    UnknownGroup { group: String },
    #[error("fishing group '{group}' is missing the {rod} encounter table")]
    MissingRodTable { group: String, rod: String },
    #[error("fishing time group {time_group} is not defined")]
    MissingTimeGroup { time_group: usize },
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
    for rule in &catalog.swarm_rules {
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
            let (species, level) = match (&slot.species, slot.time_group) {
                (Some(species), _) => (species.clone(), slot.level),
                (None, Some(time_group)) => resolve_time_group(catalog, time_group, time_of_day)?,
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

pub fn fishing_rod_for_item_id<'a>(
    catalog: &'a FishingCatalog,
    item_id: &str,
) -> Result<&'a str, FishingError> {
    if !is_exact_nonempty_fishing_token(item_id) {
        return Err(FishingError::InvalidRodItemId {
            item_id: item_id.to_string(),
        });
    }
    let rule = catalog
        .rod_items
        .iter()
        .find(|rule| rule.item_id == item_id)
        .ok_or_else(|| FishingError::UnknownRodItemId {
            item_id: item_id.to_string(),
        })?;
    validate_rod(&rule.rod)?;
    Ok(rule.rod.as_str())
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
    time_group: usize,
    time_of_day: TimeOfDay,
) -> Result<(String, u8), FishingError> {
    let entry = catalog
        .time_groups
        .get(time_group)
        .ok_or(FishingError::MissingTimeGroup { time_group })?;
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
                                    time_group: Some(0),
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
            time_groups: vec![TimeFishEntry {
                day_species: "CORSOLA".to_string(),
                day_level: 20,
                night_species: "STARYU".to_string(),
                night_level: 20,
            }],
            swarm_rules: vec![FishingSwarmRule {
                daily_flag_bit: 2,
                swarm: 1,
                base_group: "FISHGROUP_QWILFISH".to_string(),
                swarm_group: "FISHGROUP_QWILFISH_SWARM".to_string(),
            }],
            rod_items: vec![FishingRodItemRule {
                item_id: "MOD_GOOD_ROD_ITEM".to_string(),
                rod: ROD_GOOD.to_string(),
            }],
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
                &BTreeSet::new(),
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
        catalog.rod_items = vec![
            FishingRodItemRule {
                item_id: "OLD_ROD".to_string(),
                rod: "OLD_ROD".to_string(),
            },
            FishingRodItemRule {
                item_id: "OLD_ROD".to_string(),
                rod: "BAD_ROD".to_string(),
            },
        ];
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
                                time_group: Some(9),
                            },
                        ],
                    },
                )]
                .into_iter()
                .collect(),
            },
        );
        catalog.time_groups[0].night_species = "MISSINGNO".to_string();
        catalog.swarm_rules = vec![
            FishingSwarmRule {
                daily_flag_bit: 8,
                swarm: 1,
                base_group: " FISHGROUP_LAKE".to_string(),
                swarm_group: String::new(),
            },
            FishingSwarmRule {
                daily_flag_bit: 1,
                swarm: 1,
                base_group: "FISHGROUP_MISSING".to_string(),
                swarm_group: "FISHGROUP_SWARM".to_string(),
            },
            FishingSwarmRule {
                daily_flag_bit: 1,
                swarm: 1,
                base_group: "FISHGROUP_MISSING".to_string(),
                swarm_group: "FISHGROUP_SWARM".to_string(),
            },
        ];
        let item_ids = BTreeSet::from(["OLD_ROD".to_string()]);
        let species_ids = BTreeSet::from(["MAGIKARP".to_string(), "CORSOLA".to_string()]);

        assert_eq!(
            fishing_catalog_issues(&catalog, &referenced_groups, &item_ids, &species_ids),
            vec![
                FishingCatalogIssue::DuplicateRodItemId {
                    item_id: "OLD_ROD".to_string(),
                },
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
                    time_group: 9,
                },
                FishingCatalogIssue::UnknownFishingSpecies {
                    group_id: "FISHGROUP_LAKE".to_string(),
                    species: "MISSINGNO".to_string(),
                },
                FishingCatalogIssue::UnknownFishingTimeGroupSpecies {
                    index: 0,
                    species: "MISSINGNO".to_string(),
                },
                FishingCatalogIssue::InvalidSwarmFlagBit {
                    index: 0,
                    daily_flag_bit: 8,
                },
                FishingCatalogIssue::InvalidSwarmBaseGroup { index: 0 },
                FishingCatalogIssue::InvalidSwarmGroup { index: 0 },
                FishingCatalogIssue::UnknownSwarmBaseGroup {
                    index: 1,
                    base_group: "FISHGROUP_MISSING".to_string(),
                },
                FishingCatalogIssue::UnknownSwarmGroup {
                    index: 1,
                    swarm_group: "FISHGROUP_SWARM".to_string(),
                },
                FishingCatalogIssue::UnknownSwarmBaseGroup {
                    index: 2,
                    base_group: "FISHGROUP_MISSING".to_string(),
                },
                FishingCatalogIssue::UnknownSwarmGroup {
                    index: 2,
                    swarm_group: "FISHGROUP_SWARM".to_string(),
                },
                FishingCatalogIssue::DuplicateSwarmRule { index: 2 },
            ],
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
            time_groups: vec![TimeFishEntry {
                day_species: " MAGIKARP".to_string(),
                day_level: 10,
                night_species: "STAR YU".to_string(),
                night_level: 10,
            }],
            swarm_rules: vec![FishingSwarmRule {
                daily_flag_bit: 1,
                swarm: 1,
                base_group: "FISHGROUP BAD".to_string(),
                swarm_group: "FISHGROUP SWARM".to_string(),
            }],
            rod_items: vec![
                FishingRodItemRule {
                    item_id: " OLD_ROD".to_string(),
                    rod: ROD_OLD.to_string(),
                },
                FishingRodItemRule {
                    item_id: "OLD ROD".to_string(),
                    rod: ROD_OLD.to_string(),
                },
                FishingRodItemRule {
                    item_id: "MISSING_ROD".to_string(),
                    rod: ROD_OLD.to_string(),
                },
                FishingRodItemRule {
                    item_id: "MISSING_ROD_2".to_string(),
                    rod: "OLD ROD".to_string(),
                },
            ],
        };
        let referenced_groups = vec![
            ("LAKE".to_string(), " FISHGROUP_BAD".to_string()),
            ("COVE".to_string(), "FISHGROUP BAD".to_string()),
            ("POND".to_string(), "FISHGROUP_MISSING".to_string()),
        ];
        let item_ids = BTreeSet::from(["OLD_ROD".to_string()]);
        let species_ids = BTreeSet::from(["MAGIKARP".to_string()]);

        assert_eq!(
            fishing_catalog_issues(&catalog, &referenced_groups, &item_ids, &species_ids),
            vec![
                FishingCatalogIssue::InvalidRodItemId {
                    item_id: " OLD_ROD".to_string(),
                },
                FishingCatalogIssue::InvalidRodItemId {
                    item_id: "OLD ROD".to_string(),
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
                    index: 0,
                    species: " MAGIKARP".to_string(),
                },
                FishingCatalogIssue::InvalidFishingTimeGroupSpecies {
                    index: 0,
                    species: "STAR YU".to_string(),
                },
                FishingCatalogIssue::InvalidSwarmBaseGroup { index: 0 },
                FishingCatalogIssue::InvalidSwarmGroup { index: 0 },
            ],
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
              "time_groups":[],
              "swarm_rules":[],
              "rod_items":[]
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
    fn fishing_rejects_malformed_runtime_time_group_species() {
        let mut catalog = catalog();
        catalog.time_groups[0].night_species = "STAR YU".to_string();

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
