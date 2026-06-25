use std::collections::BTreeMap;

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
    #[error("unknown fishing rod '{rod}'")]
    UnknownRod { rod: String },
    #[error("fishing group {group} is not defined")]
    UnknownGroup { group: String },
    #[error("fishing group '{group}' is missing the {rod} encounter table")]
    MissingRodTable { group: String, rod: String },
    #[error("fishing time group {time_group} is not defined")]
    MissingTimeGroup { time_group: usize },
    #[error("fishing slot in {group}/{rod} resolved without a species")]
    MissingSlotSpecies { group: String, rod: String },
    #[error("fishing slot roll {slot_roll} did not resolve within {group}/{rod}")]
    UnresolvedSlot {
        group: String,
        rod: String,
        slot_roll: u8,
    },
    #[error("item id '{item_id}' is not a fishing rod item")]
    UnknownRodItemId { item_id: String },
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
    current_frame: Option<u64>,
) -> Option<bool> {
    let frame = current_frame.unwrap_or(state.frame_counter);
    let elapsed = frame.saturating_sub(session.start_frame);
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
        assert_eq!(fishing_bite(&mut state, &mut session, Some(46)), None);
        assert_eq!(fishing_bite(&mut state, &mut session, Some(47)), Some(true));
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
    }
}
