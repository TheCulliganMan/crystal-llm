use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use super::pokemon::{PokemonType, Stat};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Move {
    #[serde(deserialize_with = "required_move_token")]
    pub name: String,
    #[serde(rename = "type")]
    #[serde(deserialize_with = "required_move_token")]
    pub move_type: PokemonType,
    pub power: u16,
    pub accuracy: u8,
    pub pp: u8,
    #[serde(deserialize_with = "required_move_token")]
    pub effect: String,
    pub effect_chance: u8,
    pub stat: Option<Stat>,
    pub amount: Option<i8>,
}

impl<'de> Deserialize<'de> for Move {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawMove {
            #[serde(deserialize_with = "required_move_token")]
            name: String,
            #[serde(rename = "type")]
            #[serde(deserialize_with = "required_move_token")]
            move_type: PokemonType,
            power: u16,
            accuracy: u8,
            pp: u8,
            #[serde(deserialize_with = "required_move_token")]
            effect: String,
            effect_chance: u8,
            stat: Option<Stat>,
            amount: Option<i8>,
        }

        let raw = RawMove::deserialize(deserializer)?;
        let move_data = Self {
            name: raw.name,
            move_type: raw.move_type,
            power: raw.power,
            accuracy: raw.accuracy,
            pp: raw.pp,
            effect: raw.effect,
            effect_chance: raw.effect_chance,
            stat: raw.stat,
            amount: raw.amount,
        };
        move_data.validate_shape().map_err(D::Error::custom)?;
        Ok(move_data)
    }
}

impl Move {
    fn validate_shape(&self) -> Result<(), String> {
        if self.pp == 0 {
            return Err(format!("move {} must have positive PP", self.name));
        }
        Ok(())
    }
}

fn required_move_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if !is_exact_nonempty_move_token(&value) {
        return Err(serde::de::Error::custom(format!(
            "move token must be exact ASCII alphanumeric/underscore, found {value:?}"
        )));
    }
    validate_no_reserved_move_token(&value).map_err(serde::de::Error::custom)?;
    Ok(value)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MovePayloadIssue {
    MissingName,
    InvalidName { name: String },
    MissingType,
    InvalidType { move_type: String },
    MissingEffect,
    InvalidEffect { effect: String },
}

pub fn move_payload_issues(move_data: &Move) -> Vec<MovePayloadIssue> {
    let mut issues = Vec::new();

    if move_data.name.trim().is_empty() {
        issues.push(MovePayloadIssue::MissingName);
    } else if !is_valid_move_payload_token(&move_data.name) {
        issues.push(MovePayloadIssue::InvalidName {
            name: move_data.name.clone(),
        });
    }
    if move_data.move_type.trim().is_empty() {
        issues.push(MovePayloadIssue::MissingType);
    } else if !is_valid_move_payload_token(&move_data.move_type) {
        issues.push(MovePayloadIssue::InvalidType {
            move_type: move_data.move_type.clone(),
        });
    }
    if move_data.effect.trim().is_empty() {
        issues.push(MovePayloadIssue::MissingEffect);
    } else if !is_valid_move_payload_token(&move_data.effect) {
        issues.push(MovePayloadIssue::InvalidEffect {
            effect: move_data.effect.clone(),
        });
    }

    issues
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveNameCatalogIssue {
    CountMismatch {
        actual_count: usize,
        expected_count: usize,
    },
    InvalidName {
        index: usize,
    },
}

pub fn move_name_catalog_issues(
    move_names: &[String],
    move_count: usize,
) -> Vec<MoveNameCatalogIssue> {
    let mut issues = Vec::new();

    if !move_names.is_empty() && move_names.len() != move_count {
        issues.push(MoveNameCatalogIssue::CountMismatch {
            actual_count: move_names.len(),
            expected_count: move_count,
        });
    }
    for (index, move_name) in move_names.iter().enumerate() {
        if !is_valid_move_display_name(move_name) {
            issues.push(MoveNameCatalogIssue::InvalidName { index });
        }
    }

    issues
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct MoveNameTable(pub Vec<String>);

impl<'de> Deserialize<'de> for MoveNameTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let names = Vec::<String>::deserialize(deserializer)?;
        if names.is_empty() {
            return Err(D::Error::custom("move names table must not be empty"));
        }
        for (index, name) in names.iter().enumerate() {
            if !is_valid_move_display_name(name) {
                return Err(D::Error::custom(format!(
                    "move names table entry {index} must be an exact move display name, found {name:?}"
                )));
            }
        }
        Ok(Self(names))
    }
}

fn is_exact_nonempty_move_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn is_valid_move_payload_token(value: &str) -> bool {
    is_exact_nonempty_move_token(value) && validate_no_reserved_move_token(value).is_ok()
}

fn is_valid_move_display_name(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value.chars().all(|character| !character.is_control())
        && validate_no_reserved_move_token(value).is_ok()
}

fn validate_no_reserved_move_token(value: &str) -> Result<(), String> {
    let lowered = value.to_ascii_lowercase();
    if lowered.starts_with("fallback") || lowered.starts_with("legacy") {
        return Err(format!(
            "move token '{value}' uses reserved modpack payload prefix"
        ));
    }
    Ok(())
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

    #[test]
    fn move_identifier_fields_reject_malformed_tokens_at_deserialization() {
        for (field, value) in [
            ("name", serde_json::json!("AETHER PULSE")),
            ("type", serde_json::json!(" AETHER")),
            ("type", serde_json::json!("legacy_AETHER")),
            ("effect", serde_json::json!("MODDED EFFECT")),
            ("effect", serde_json::json!("fallback_EFFECT")),
        ] {
            let mut move_json = valid_move_json();
            move_json[field] = value;

            let error = serde_json::from_value::<Move>(move_json)
                .expect_err("malformed move identifiers must fail before runtime use")
                .to_string();

            assert!(
                error.contains("move token must be")
                    || error.contains("uses reserved modpack payload prefix"),
                "{field} produced unexpected error: {error}"
            );
        }
    }

    #[test]
    fn move_effect_field_rejects_enum_object_values() {
        let mut move_json = valid_move_json();
        move_json["effect"] = serde_json::json!({ "kind": "NORMAL_HIT" });

        let error = serde_json::from_value::<Move>(move_json)
            .expect_err("move effects must be exact modpack strings, not enum objects")
            .to_string();

        assert!(
            error.contains("invalid type: map")
                || error.contains("invalid type: enum")
                || error.contains("expected a string"),
            "{error}"
        );
    }

    #[test]
    fn move_payload_issues_require_exact_pack_owned_ids_without_effect_enums() {
        let move_data = Move {
            name: "AETHER PULSE".to_string(),
            move_type: pokemon_type("AETHER TYPE"),
            power: 60,
            accuracy: 100,
            pp: 15,
            effect: "fallback_EFFECT".to_string(),
            effect_chance: 0,
            stat: None,
            amount: None,
        };

        assert_eq!(
            move_payload_issues(&move_data),
            vec![
                MovePayloadIssue::InvalidName {
                    name: "AETHER PULSE".to_string(),
                },
                MovePayloadIssue::InvalidType {
                    move_type: "AETHER TYPE".to_string(),
                },
                MovePayloadIssue::InvalidEffect {
                    effect: "fallback_EFFECT".to_string(),
                },
            ],
        );
    }

    #[test]
    fn move_payload_issues_accept_custom_exact_effect_ids() {
        let move_data = Move {
            name: "AETHER_PULSE".to_string(),
            move_type: pokemon_type("AETHER"),
            power: 60,
            accuracy: 100,
            pp: 15,
            effect: "MODDED_EFFECT".to_string(),
            effect_chance: 0,
            stat: None,
            amount: None,
        };

        assert!(move_payload_issues(&move_data).is_empty());
    }

    #[test]
    fn move_name_catalog_issues_require_exact_nonempty_display_names() {
        assert_eq!(
            move_name_catalog_issues(
                &[
                    "POUND".to_string(),
                    String::new(),
                    "KARATE CHOP".to_string(),
                    "legacy_POUND".to_string()
                ],
                2,
            ),
            vec![
                MoveNameCatalogIssue::CountMismatch {
                    actual_count: 4,
                    expected_count: 2,
                },
                MoveNameCatalogIssue::InvalidName { index: 1 },
                MoveNameCatalogIssue::InvalidName { index: 2 },
                MoveNameCatalogIssue::InvalidName { index: 3 },
            ],
        );
    }

    #[test]
    fn move_name_catalog_issues_allow_absent_partial_pack_names() {
        assert!(move_name_catalog_issues(&[], 251).is_empty());
    }

    fn valid_move_json() -> serde_json::Value {
        serde_json::json!({
            "name": "AETHER_PULSE",
            "type": "AETHER",
            "power": 60,
            "accuracy": 100,
            "pp": 15,
            "effect": "MODDED_EFFECT",
            "effect_chance": 0,
            "stat": null,
            "amount": null
        })
    }
}
