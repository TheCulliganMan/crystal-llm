use serde::{Deserialize, Serialize};

use super::pokemon::{PokemonType, Stat};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Move {
    pub name: String,
    #[serde(rename = "type")]
    pub move_type: PokemonType,
    pub power: u16,
    pub accuracy: u8,
    pub pp: u8,
    pub effect: String,
    pub effect_chance: u8,
    pub stat: Option<Stat>,
    pub amount: Option<i8>,
}

#[cfg(test)]
mod tests {
    use super::super::pokemon::pokemon_type;
    use super::*;

    #[test]
    fn parses_existing_json_move_shape() {
        let value: Move = serde_json::from_str(
            r#"{
              "name":"POUND",
              "type":"NORMAL",
              "power":40,
              "accuracy":100,
              "pp":35,
              "effect":"NORMAL_HIT",
              "effect_chance":0,
              "stat":null,
              "amount":null
            }"#,
        )
        .expect("parse move");

        assert_eq!(value.name, "POUND");
        assert_eq!(value.move_type, pokemon_type("NORMAL"));
        assert_eq!(value.pp, 35);
    }

    #[test]
    fn move_type_ids_are_modpack_owned_strings_not_core_enums() {
        let value: Move = serde_json::from_str(
            r#"{
              "name":"AETHER_PULSE",
              "type":"AETHER",
              "power":60,
              "accuracy":100,
              "pp":15,
              "effect":"NORMAL_HIT",
              "effect_chance":0,
              "stat":null,
              "amount":null
            }"#,
        )
        .expect("modded move type ids are exact data");

        assert_eq!(value.move_type, pokemon_type("AETHER"));
    }

    #[test]
    fn move_json_rejects_unknown_modpack_fields() {
        let error = serde_json::from_str::<Move>(
            r#"{
              "name":"POUND",
              "type":"NORMAL",
              "power":40,
              "accuracy":100,
              "pp":35,
              "effect":"NORMAL_HIT",
              "effect_chance":0,
              "stat":null,
              "amount":null,
              "legacy_effect":"hit"
            }"#,
        )
        .expect_err("moves must not accept legacy effect fields")
        .to_string();

        assert!(error.contains("unknown field `legacy_effect`"), "{error}");
    }
}
