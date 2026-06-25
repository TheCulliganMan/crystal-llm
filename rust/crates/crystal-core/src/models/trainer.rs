use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{Dv, LearnedMove};

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
    pub trainer_id: String,
    pub trainer_class: String,
    pub party: Vec<TrainerPartyPokemon>,
    pub win_quote: String,
    pub lose_quote: String,
    pub items: Vec<Option<String>>,
    pub base_reward: u32,
    pub ai_move_flags: u32,
    pub ai_item_switch_flags: u32,
    pub encounter_music: String,
    pub ai_layers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrainerPartyPokemon {
    pub species: String,
    pub level: u8,
    pub item: Option<String>,
    pub moves: Vec<LearnedMove>,
    pub dvs: Dv,
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
    #[error("trainer '{trainer_id}' is missing explicit trainer_class")]
    MissingTrainerClass { trainer_id: String },
}

pub fn trainer_key(trainer: &Trainer) -> Result<String, TrainerCatalogError> {
    if trainer.trainer_id.is_empty() {
        Err(TrainerCatalogError::MissingTrainerId)
    } else {
        Ok(trainer.trainer_id.clone())
    }
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
    fn trainer_catalog_json_requires_explicit_trainers_map() {
        let error = serde_json::from_str::<TrainerCatalog>(r#"{}"#)
            .expect_err("missing trainer catalog must not default to empty")
            .to_string();

        assert!(error.contains("missing field `trainers`"), "{error}");
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
}
