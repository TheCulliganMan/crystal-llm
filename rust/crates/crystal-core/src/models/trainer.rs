use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{Dv, Item, LearnedMove};

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrainerCatalog {
    pub trainers: BTreeMap<String, Trainer>,
}

impl TrainerCatalog {
    pub fn insert(&mut self, trainer: Trainer) -> Result<(), TrainerCatalogError> {
        let trainer_id = trainer_key(&trainer)?;
        if trainer.trainer_class.is_empty() {
            return Err(TrainerCatalogError::MissingTrainerClass {
                trainer_id: trainer.trainer_id,
            });
        } else if !is_exact_nonempty_trainer_token(&trainer.trainer_class) {
            return Err(TrainerCatalogError::InvalidTrainerClass {
                trainer_id: trainer.trainer_id,
                trainer_class: trainer.trainer_class,
            });
        }
        if self.trainers.contains_key(&trainer_id) {
            return Err(TrainerCatalogError::DuplicateTrainerId { trainer_id });
        }
        self.trainers.insert(trainer_id, trainer);
        Ok(())
    }

    pub fn get(&self, trainer_id: &str) -> Option<&Trainer> {
        self.trainers.get(trainer_id)
    }

    pub fn is_empty(&self) -> bool {
        self.trainers.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trainer {
    pub name: String,
    #[serde(deserialize_with = "required_trainer_token")]
    pub trainer_id: String,
    #[serde(deserialize_with = "required_trainer_token")]
    pub trainer_class: String,
    pub party: Vec<TrainerPartyPokemon>,
    pub win_quote: String,
    pub lose_quote: String,
    #[serde(deserialize_with = "required_nullable_trainer_token_vec")]
    pub items: Vec<Option<String>>,
    pub base_reward: u32,
    pub ai_move_flags: u32,
    pub ai_item_switch_flags: u32,
    #[serde(deserialize_with = "required_trainer_token")]
    pub encounter_music: String,
    #[serde(deserialize_with = "required_trainer_token_vec")]
    pub ai_layers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrainerPartyPokemon {
    #[serde(deserialize_with = "required_trainer_token")]
    pub species: String,
    pub level: u8,
    #[serde(deserialize_with = "required_nullable_trainer_token")]
    pub item: Option<String>,
    pub moves: Vec<LearnedMove>,
    pub dvs: Dv,
}

fn required_trainer_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_nonempty_trainer_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "trainer token must be exact ASCII alphanumeric/underscore, found {value:?}"
        )))
    }
}

fn required_nullable_trainer_token<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    match value {
        Some(token) if is_exact_nonempty_trainer_token(&token) => Ok(Some(token)),
        Some(token) => Err(serde::de::Error::custom(format!(
            "trainer token must be exact ASCII alphanumeric/underscore, found {token:?}"
        ))),
        None => Ok(None),
    }
}

fn required_trainer_token_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values = Vec::<String>::deserialize(deserializer)?;
    if let Some(token) = values
        .iter()
        .find(|token| !is_exact_nonempty_trainer_token(token))
    {
        Err(serde::de::Error::custom(format!(
            "trainer token must be exact ASCII alphanumeric/underscore, found {token:?}"
        )))
    } else {
        Ok(values)
    }
}

fn required_nullable_trainer_token_vec<'de, D>(
    deserializer: D,
) -> Result<Vec<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values = Vec::<Option<String>>::deserialize(deserializer)?;
    if let Some(token) = values
        .iter()
        .flatten()
        .find(|token| !is_exact_nonempty_trainer_token(token))
    {
        Err(serde::de::Error::custom(format!(
            "trainer token must be exact ASCII alphanumeric/underscore, found {token:?}"
        )))
    } else {
        Ok(values)
    }
}

impl Default for TrainerPartyPokemon {
    fn default() -> Self {
        Self {
            species: String::new(),
            level: 1,
            item: None,
            moves: Vec::new(),
            dvs: Dv::default(),
        }
    }
}

impl Default for Trainer {
    fn default() -> Self {
        Self {
            name: String::new(),
            trainer_id: String::new(),
            trainer_class: String::new(),
            party: Vec::new(),
            win_quote: String::new(),
            lose_quote: String::new(),
            items: Vec::new(),
            base_reward: 0,
            ai_move_flags: 0,
            ai_item_switch_flags: 0,
            encounter_music: String::new(),
            ai_layers: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TrainerCatalogError {
    #[error("trainer is missing explicit trainer_id")]
    MissingTrainerId,
    #[error("trainer id '{trainer_id}' must be an exact nonempty id token")]
    InvalidTrainerId { trainer_id: String },
    #[error("trainer '{trainer_id}' is missing explicit trainer_class")]
    MissingTrainerClass { trainer_id: String },
    #[error("trainer '{trainer_id}' class '{trainer_class}' must be an exact nonempty id token")]
    InvalidTrainerClass {
        trainer_id: String,
        trainer_class: String,
    },
    #[error("trainer id '{trainer_id}' is duplicated")]
    DuplicateTrainerId { trainer_id: String },
}

pub fn trainer_key(trainer: &Trainer) -> Result<String, TrainerCatalogError> {
    if trainer.trainer_id.is_empty() {
        Err(TrainerCatalogError::MissingTrainerId)
    } else if !is_exact_nonempty_trainer_token(&trainer.trainer_id) {
        Err(TrainerCatalogError::InvalidTrainerId {
            trainer_id: trainer.trainer_id.clone(),
        })
    } else {
        Ok(trainer.trainer_id.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrainerCatalogIssue {
    KeyMismatch {
        key: String,
        trainer_id: String,
    },
    InvalidTrainerId {
        trainer_id: String,
    },
    MissingTrainerClass {
        trainer_id: String,
    },
    InvalidTrainerClass {
        trainer_id: String,
        trainer_class: String,
    },
    EmptyParty {
        trainer_id: String,
    },
    InvalidPartySpecies {
        trainer_id: String,
        slot: usize,
        species: String,
    },
    UnknownPartySpecies {
        trainer_id: String,
        slot: usize,
        species: String,
    },
    InvalidPartyItem {
        trainer_id: String,
        slot: usize,
        item_id: String,
    },
    UnknownPartyItem {
        trainer_id: String,
        slot: usize,
        item_id: String,
    },
    InvalidBattleItem {
        trainer_id: String,
        slot: usize,
        item_id: String,
    },
    UnknownBattleItem {
        trainer_id: String,
        slot: usize,
        item_id: String,
    },
    UnusableBattleItem {
        trainer_id: String,
        slot: usize,
        item_id: String,
    },
    InvalidPartyMove {
        trainer_id: String,
        slot: usize,
        move_id: String,
    },
    UnknownPartyMove {
        trainer_id: String,
        slot: usize,
        move_id: String,
    },
}

pub fn trainer_catalog_issues(
    catalog: &TrainerCatalog,
    species_ids: &BTreeSet<String>,
    items: &BTreeMap<String, Item>,
    move_ids: &BTreeSet<String>,
) -> Vec<TrainerCatalogIssue> {
    let mut issues = Vec::new();
    for (key, trainer) in &catalog.trainers {
        if key != &trainer.trainer_id {
            issues.push(TrainerCatalogIssue::KeyMismatch {
                key: key.clone(),
                trainer_id: trainer.trainer_id.clone(),
            });
        }
        if !is_exact_nonempty_trainer_token(&trainer.trainer_id) {
            issues.push(TrainerCatalogIssue::InvalidTrainerId {
                trainer_id: trainer.trainer_id.clone(),
            });
        }
        if trainer.trainer_class.is_empty() {
            issues.push(TrainerCatalogIssue::MissingTrainerClass {
                trainer_id: trainer.trainer_id.clone(),
            });
        } else if !is_exact_nonempty_trainer_token(&trainer.trainer_class) {
            issues.push(TrainerCatalogIssue::InvalidTrainerClass {
                trainer_id: trainer.trainer_id.clone(),
                trainer_class: trainer.trainer_class.clone(),
            });
        }
        if trainer.party.is_empty() {
            issues.push(TrainerCatalogIssue::EmptyParty {
                trainer_id: trainer.trainer_id.clone(),
            });
        }
        for (slot, party_mon) in trainer.party.iter().enumerate() {
            if !is_exact_nonempty_trainer_token(&party_mon.species) {
                issues.push(TrainerCatalogIssue::InvalidPartySpecies {
                    trainer_id: trainer.trainer_id.clone(),
                    slot,
                    species: party_mon.species.clone(),
                });
            } else if !species_ids.contains(&party_mon.species) {
                issues.push(TrainerCatalogIssue::UnknownPartySpecies {
                    trainer_id: trainer.trainer_id.clone(),
                    slot,
                    species: party_mon.species.clone(),
                });
            }
            if let Some(item_id) = party_mon.item.as_deref() {
                if !is_exact_nonempty_trainer_token(item_id) {
                    issues.push(TrainerCatalogIssue::InvalidPartyItem {
                        trainer_id: trainer.trainer_id.clone(),
                        slot,
                        item_id: item_id.to_string(),
                    });
                } else if !items.contains_key(item_id) {
                    issues.push(TrainerCatalogIssue::UnknownPartyItem {
                        trainer_id: trainer.trainer_id.clone(),
                        slot,
                        item_id: item_id.to_string(),
                    });
                }
            }
            for learned_move in &party_mon.moves {
                if !is_exact_nonempty_trainer_token(&learned_move.name) {
                    issues.push(TrainerCatalogIssue::InvalidPartyMove {
                        trainer_id: trainer.trainer_id.clone(),
                        slot,
                        move_id: learned_move.name.clone(),
                    });
                } else if !move_ids.contains(&learned_move.name) {
                    issues.push(TrainerCatalogIssue::UnknownPartyMove {
                        trainer_id: trainer.trainer_id.clone(),
                        slot,
                        move_id: learned_move.name.clone(),
                    });
                }
            }
        }
        for (slot, item_id) in trainer.items.iter().enumerate() {
            let Some(item_id) = item_id.as_deref() else {
                continue;
            };
            if !is_exact_nonempty_trainer_token(item_id) {
                issues.push(TrainerCatalogIssue::InvalidBattleItem {
                    trainer_id: trainer.trainer_id.clone(),
                    slot,
                    item_id: item_id.to_string(),
                });
            } else {
                match items.get(item_id) {
                    Some(item) if !item.battle_usable => {
                        issues.push(TrainerCatalogIssue::UnusableBattleItem {
                            trainer_id: trainer.trainer_id.clone(),
                            slot,
                            item_id: item_id.to_string(),
                        });
                    }
                    Some(_) => {}
                    None => issues.push(TrainerCatalogIssue::UnknownBattleItem {
                        trainer_id: trainer.trainer_id.clone(),
                        slot,
                        item_id: item_id.to_string(),
                    }),
                }
            }
        }
    }
    issues
}

fn is_exact_nonempty_trainer_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && !matches!(
            value.to_ascii_lowercase().as_str(),
            token if token.starts_with("fallback") || token.starts_with("legacy")
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trainer_json(party_entry: &str) -> String {
        format!(
            r#"{{
              "name":"Youngster Joey",
              "trainer_id":"YOUNGSTER_JOEY",
              "trainer_class":"YOUNGSTER",
              "party":[{party_entry}],
              "win_quote":"I won!",
              "lose_quote":"I lost!",
              "items":[],
              "base_reward":4,
              "ai_move_flags":0,
              "ai_item_switch_flags":0,
              "encounter_music":"MUSIC_YOUNGSTER_ENCOUNTER",
              "ai_layers":[]
            }}"#
        )
    }

    #[test]
    fn trainer_party_requires_explicit_modpack_fields() {
        let missing_dvs = trainer_json(
            r#"{
              "species":"RATTATA",
              "level":6,
              "item":null,
              "moves":[]
            }"#,
        );
        let error = serde_json::from_str::<Trainer>(&missing_dvs)
            .expect_err("trainer party DVs must not deserialize from defaults")
            .to_string();

        assert!(error.contains("missing field `dvs`"), "{error}");
    }

    #[test]
    fn trainer_requires_explicit_modpack_fields() {
        let error = serde_json::from_str::<Trainer>(
            r#"{
              "name":"Youngster Joey",
              "trainer_id":"YOUNGSTER_JOEY",
              "trainer_class":"YOUNGSTER",
              "party":[]
            }"#,
        )
        .expect_err("trainer records must not deserialize from defaults")
        .to_string();

        assert!(error.contains("missing field `win_quote`"), "{error}");
    }

    #[test]
    fn trainer_deserializes_when_all_pack_fields_are_explicit() {
        let trainer = serde_json::from_str::<Trainer>(&trainer_json(
            r#"{
              "species":"RATTATA",
              "level":6,
              "item":null,
              "moves":[],
              "dvs":{"attack":0,"defense":0,"speed":0,"special":0,"hp":0}
            }"#,
        ))
        .expect("explicit trainer should deserialize");

        assert_eq!(trainer.trainer_id, "YOUNGSTER_JOEY");
        assert_eq!(trainer.party[0].species, "RATTATA");
        assert_eq!(trainer.party[0].dvs, Dv::default());
    }

    #[test]
    fn trainer_identifier_fields_reject_malformed_tokens_at_deserialization() {
        for (field, value) in [
            ("trainer_id", serde_json::json!("YOUNGSTER JOEY")),
            ("trainer_class", serde_json::json!("YOUNG STER")),
            ("items", serde_json::json!([null, "SUPER POTION"])),
            (
                "encounter_music",
                serde_json::json!("MUSIC YOUNGSTER_ENCOUNTER"),
            ),
            ("ai_layers", serde_json::json!(["BASIC", "SMART AI"])),
        ] {
            let mut trainer = valid_trainer_json();
            trainer[field] = value;

            let error = serde_json::from_value::<Trainer>(trainer)
                .expect_err("malformed trainer identifiers must fail before runtime use")
                .to_string();

            assert!(
                error.contains("trainer token must be"),
                "{field} produced unexpected error: {error}"
            );
        }

        for (field, value) in [
            ("species", serde_json::json!("RAT TATA")),
            ("item", serde_json::json!("BERRY JUICE")),
        ] {
            let mut trainer = valid_trainer_json();
            trainer["party"][0][field] = value;

            let error = serde_json::from_value::<Trainer>(trainer)
                .expect_err("malformed trainer party identifiers must fail before runtime use")
                .to_string();

            assert!(
                error.contains("trainer token must be"),
                "party {field} produced unexpected error: {error}"
            );
        }
    }

    #[test]
    fn trainer_identifier_fields_reject_reserved_pack_prefixes() {
        for (field, value) in [
            ("trainer_id", serde_json::json!("fallback_youngster")),
            ("trainer_class", serde_json::json!("legacy_class")),
            ("items", serde_json::json!([null, "fallback_potion"])),
            (
                "encounter_music",
                serde_json::json!("legacy_encounter_music"),
            ),
            ("ai_layers", serde_json::json!(["BASIC", "fallback_ai"])),
        ] {
            let mut trainer = valid_trainer_json();
            trainer[field] = value;

            let error = serde_json::from_value::<Trainer>(trainer)
                .expect_err("reserved trainer identifiers must fail before runtime use")
                .to_string();

            assert!(
                error.contains("trainer token must be"),
                "{field} produced unexpected error: {error}"
            );
        }

        for (field, value) in [
            ("species", serde_json::json!("fallback_species")),
            ("item", serde_json::json!("legacy_item")),
        ] {
            let mut trainer = valid_trainer_json();
            trainer["party"][0][field] = value;

            let error = serde_json::from_value::<Trainer>(trainer)
                .expect_err("reserved trainer party identifiers must fail before runtime use")
                .to_string();

            assert!(
                error.contains("trainer token must be"),
                "party {field} produced unexpected error: {error}"
            );
        }
    }

    fn valid_trainer_json() -> serde_json::Value {
        serde_json::json!({
            "name": "Youngster Joey",
            "trainer_id": "YOUNGSTER_JOEY",
            "trainer_class": "YOUNGSTER",
            "party": [{
                "species": "RATTATA",
                "level": 6,
                "item": null,
                "moves": [],
                "dvs": { "attack": 0, "defense": 0, "speed": 0, "special": 0, "hp": 0 }
            }],
            "win_quote": "I won!",
            "lose_quote": "I lost!",
            "items": [],
            "base_reward": 4,
            "ai_move_flags": 0,
            "ai_item_switch_flags": 0,
            "encounter_music": "MUSIC_YOUNGSTER_ENCOUNTER",
            "ai_layers": []
        })
    }

    fn test_item(id: &str, battle_usable: bool) -> Item {
        use crate::models::item_pocket;

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
            pocket: item_pocket("ITEM"),
            field_menu: "ITEMMENU_NOUSE".to_string(),
            field_usable: false,
            battle_menu: if battle_usable {
                "ITEMMENU_CLOSE".to_string()
            } else {
                "ITEMMENU_NOUSE".to_string()
            },
            battle_usable,
            script_name: id.to_string(),
            consumable: true,
            tmhm_index: None,
            tmhm_move: None,
        }
    }

    #[test]
    fn trainer_catalog_json_requires_explicit_trainers_map() {
        let error = serde_json::from_str::<TrainerCatalog>(r#"{}"#)
            .expect_err("missing trainer catalog must not default to empty")
            .to_string();

        assert!(error.contains("missing field `trainers`"), "{error}");
    }

    #[test]
    fn trainer_catalog_insert_rejects_non_token_identity_without_coercion() {
        let mut catalog = TrainerCatalog::default();
        let trainer_id_error = catalog
            .insert(Trainer {
                trainer_id: "YOUNGSTER JOEY".to_string(),
                trainer_class: "YOUNGSTER".to_string(),
                ..Trainer::default()
            })
            .expect_err("trainer ids must be id tokens");
        assert_eq!(
            trainer_id_error,
            TrainerCatalogError::InvalidTrainerId {
                trainer_id: "YOUNGSTER JOEY".to_string(),
            }
        );

        let trainer_class_error = catalog
            .insert(Trainer {
                trainer_id: "YOUNGSTER_JOEY".to_string(),
                trainer_class: "YOUNG STER".to_string(),
                ..Trainer::default()
            })
            .expect_err("trainer classes must be id tokens");
        assert_eq!(
            trainer_class_error,
            TrainerCatalogError::InvalidTrainerClass {
                trainer_id: "YOUNGSTER_JOEY".to_string(),
                trainer_class: "YOUNG STER".to_string(),
            }
        );

        let reserved_trainer_id_error = catalog
            .insert(Trainer {
                trainer_id: "fallback_youngster".to_string(),
                trainer_class: "YOUNGSTER".to_string(),
                ..Trainer::default()
            })
            .expect_err("reserved trainer ids must not enter the catalog");
        assert_eq!(
            reserved_trainer_id_error,
            TrainerCatalogError::InvalidTrainerId {
                trainer_id: "fallback_youngster".to_string(),
            }
        );

        let reserved_trainer_class_error = catalog
            .insert(Trainer {
                trainer_id: "YOUNGSTER_JOEY".to_string(),
                trainer_class: "legacy_class".to_string(),
                ..Trainer::default()
            })
            .expect_err("reserved trainer classes must not enter the catalog");
        assert_eq!(
            reserved_trainer_class_error,
            TrainerCatalogError::InvalidTrainerClass {
                trainer_id: "YOUNGSTER_JOEY".to_string(),
                trainer_class: "legacy_class".to_string(),
            }
        );

        catalog
            .insert(Trainer {
                trainer_id: "YOUNGSTER_JOEY".to_string(),
                trainer_class: "YOUNGSTER".to_string(),
                ..Trainer::default()
            })
            .expect("first trainer id is unique");
        let duplicate_error = catalog
            .insert(Trainer {
                trainer_id: "YOUNGSTER_JOEY".to_string(),
                trainer_class: "YOUNGSTER".to_string(),
                ..Trainer::default()
            })
            .expect_err("trainer catalog must not overwrite duplicate ids");
        assert_eq!(
            duplicate_error,
            TrainerCatalogError::DuplicateTrainerId {
                trainer_id: "YOUNGSTER_JOEY".to_string(),
            }
        );
    }

    #[test]
    fn trainer_json_rejects_unknown_modpack_fields() {
        let error = serde_json::from_str::<Trainer>(&trainer_json(
            r#"{
              "species":"RATTATA",
              "level":6,
              "item":null,
              "moves":[],
              "dvs":{"attack":0,"defense":0,"speed":0,"special":0,"hp":0},
              "fallback_moves":["TACKLE"]
            }"#,
        ))
        .expect_err("trainer party records must not accept fallback moves")
        .to_string();
        assert!(error.contains("unknown field `fallback_moves`"), "{error}");

        let error =
            serde_json::from_str::<TrainerCatalog>(r#"{"trainers":{},"legacy_trainers":[]}"#)
                .expect_err("trainer catalogs must not accept legacy trainer lists")
                .to_string();
        assert!(error.contains("unknown field `legacy_trainers`"), "{error}");
    }

    #[test]
    fn trainer_catalog_issues_validate_exact_pack_references() {
        let mut trainer = Trainer {
            trainer_id: " YOUNGSTER_JOEY".to_string(),
            trainer_class: "YOUNGSTER ".to_string(),
            party: vec![
                TrainerPartyPokemon {
                    species: " RATTATA".to_string(),
                    level: 6,
                    item: Some(" BERRY".to_string()),
                    moves: vec![LearnedMove {
                        name: " TACKLE".to_string(),
                        current_pp: 35,
                        pp_ups: 0,
                    }],
                    dvs: Dv::default(),
                },
                TrainerPartyPokemon {
                    species: "rattata".to_string(),
                    level: 6,
                    item: Some("berry".to_string()),
                    moves: vec![LearnedMove {
                        name: "tackle".to_string(),
                        current_pp: 35,
                        pp_ups: 0,
                    }],
                    dvs: Dv::default(),
                },
            ],
            items: vec![Some(" POTION".to_string()), Some("potion".to_string())],
            ..Trainer::default()
        };
        let trainer_id = trainer.trainer_id.clone();
        let catalog = TrainerCatalog {
            trainers: [("YOUNGSTER_JOEY".to_string(), trainer.clone())]
                .into_iter()
                .collect(),
        };
        let species_ids = BTreeSet::from(["RATTATA".to_string()]);
        let items = BTreeMap::from([
            ("BERRY".to_string(), test_item("BERRY", false)),
            ("POTION".to_string(), test_item("POTION", true)),
        ]);
        let move_ids = BTreeSet::from(["TACKLE".to_string()]);

        assert_eq!(
            trainer_catalog_issues(&catalog, &species_ids, &items, &move_ids),
            vec![
                TrainerCatalogIssue::KeyMismatch {
                    key: "YOUNGSTER_JOEY".to_string(),
                    trainer_id: trainer_id.clone(),
                },
                TrainerCatalogIssue::InvalidTrainerId {
                    trainer_id: trainer_id.clone(),
                },
                TrainerCatalogIssue::InvalidTrainerClass {
                    trainer_id: trainer_id.clone(),
                    trainer_class: "YOUNGSTER ".to_string(),
                },
                TrainerCatalogIssue::InvalidPartySpecies {
                    trainer_id: trainer_id.clone(),
                    slot: 0,
                    species: " RATTATA".to_string(),
                },
                TrainerCatalogIssue::InvalidPartyItem {
                    trainer_id: trainer_id.clone(),
                    slot: 0,
                    item_id: " BERRY".to_string(),
                },
                TrainerCatalogIssue::InvalidPartyMove {
                    trainer_id: trainer_id.clone(),
                    slot: 0,
                    move_id: " TACKLE".to_string(),
                },
                TrainerCatalogIssue::UnknownPartySpecies {
                    trainer_id: trainer_id.clone(),
                    slot: 1,
                    species: "rattata".to_string(),
                },
                TrainerCatalogIssue::UnknownPartyItem {
                    trainer_id: trainer_id.clone(),
                    slot: 1,
                    item_id: "berry".to_string(),
                },
                TrainerCatalogIssue::UnknownPartyMove {
                    trainer_id: trainer_id.clone(),
                    slot: 1,
                    move_id: "tackle".to_string(),
                },
                TrainerCatalogIssue::InvalidBattleItem {
                    trainer_id: trainer_id.clone(),
                    slot: 0,
                    item_id: " POTION".to_string(),
                },
                TrainerCatalogIssue::UnknownBattleItem {
                    trainer_id,
                    slot: 1,
                    item_id: "potion".to_string(),
                },
            ]
        );

        trainer.trainer_id = "YOUNGSTER_JOEY".to_string();
        trainer.trainer_class = String::new();
        trainer.party.clear();
        trainer.items.clear();
        let catalog = TrainerCatalog {
            trainers: [(trainer.trainer_id.clone(), trainer)]
                .into_iter()
                .collect(),
        };

        assert_eq!(
            trainer_catalog_issues(&catalog, &species_ids, &items, &move_ids),
            vec![
                TrainerCatalogIssue::MissingTrainerClass {
                    trainer_id: "YOUNGSTER_JOEY".to_string(),
                },
                TrainerCatalogIssue::EmptyParty {
                    trainer_id: "YOUNGSTER_JOEY".to_string(),
                },
            ]
        );
    }

    #[test]
    fn trainer_catalog_issues_reject_battle_items_that_are_not_battle_usable() {
        let trainer = Trainer {
            trainer_id: "YOUNGSTER_JOEY".to_string(),
            trainer_class: "YOUNGSTER".to_string(),
            party: vec![TrainerPartyPokemon {
                species: "RATTATA".to_string(),
                level: 6,
                item: Some("BERRY".to_string()),
                moves: vec![LearnedMove {
                    name: "TACKLE".to_string(),
                    current_pp: 35,
                    pp_ups: 0,
                }],
                dvs: Dv::default(),
            }],
            items: vec![Some("POTION".to_string())],
            ..Trainer::default()
        };
        let catalog = TrainerCatalog {
            trainers: [(trainer.trainer_id.clone(), trainer)]
                .into_iter()
                .collect(),
        };
        let species_ids = BTreeSet::from(["RATTATA".to_string()]);
        let items = BTreeMap::from([
            ("BERRY".to_string(), test_item("BERRY", false)),
            ("POTION".to_string(), test_item("POTION", false)),
        ]);
        let move_ids = BTreeSet::from(["TACKLE".to_string()]);

        assert_eq!(
            trainer_catalog_issues(&catalog, &species_ids, &items, &move_ids),
            vec![TrainerCatalogIssue::UnusableBattleItem {
                trainer_id: "YOUNGSTER_JOEY".to_string(),
                slot: 0,
                item_id: "POTION".to_string(),
            }]
        );
    }

    #[test]
    fn trainer_catalog_issues_reject_reserved_pack_references() {
        let trainer = Trainer {
            trainer_id: "fallback_youngster".to_string(),
            trainer_class: "legacy_class".to_string(),
            party: vec![TrainerPartyPokemon {
                species: "fallback_rattata".to_string(),
                level: 6,
                item: Some("legacy_berry".to_string()),
                moves: vec![LearnedMove {
                    name: "fallback_tackle".to_string(),
                    current_pp: 35,
                    pp_ups: 0,
                }],
                dvs: Dv::default(),
            }],
            items: vec![Some("legacy_potion".to_string())],
            ..Trainer::default()
        };
        let trainer_id = trainer.trainer_id.clone();
        let catalog = TrainerCatalog {
            trainers: [(trainer.trainer_id.clone(), trainer)]
                .into_iter()
                .collect(),
        };

        assert_eq!(
            trainer_catalog_issues(
                &catalog,
                &BTreeSet::from(["fallback_rattata".to_string()]),
                &BTreeMap::from([
                    ("legacy_berry".to_string(), test_item("legacy_berry", false)),
                    (
                        "legacy_potion".to_string(),
                        test_item("legacy_potion", true)
                    ),
                ]),
                &BTreeSet::from(["fallback_tackle".to_string()])
            ),
            vec![
                TrainerCatalogIssue::InvalidTrainerId {
                    trainer_id: trainer_id.clone(),
                },
                TrainerCatalogIssue::InvalidTrainerClass {
                    trainer_id: trainer_id.clone(),
                    trainer_class: "legacy_class".to_string(),
                },
                TrainerCatalogIssue::InvalidPartySpecies {
                    trainer_id: trainer_id.clone(),
                    slot: 0,
                    species: "fallback_rattata".to_string(),
                },
                TrainerCatalogIssue::InvalidPartyItem {
                    trainer_id: trainer_id.clone(),
                    slot: 0,
                    item_id: "legacy_berry".to_string(),
                },
                TrainerCatalogIssue::InvalidPartyMove {
                    trainer_id: trainer_id.clone(),
                    slot: 0,
                    move_id: "fallback_tackle".to_string(),
                },
                TrainerCatalogIssue::InvalidBattleItem {
                    trainer_id,
                    slot: 0,
                    item_id: "legacy_potion".to_string(),
                },
            ]
        );
    }
}
