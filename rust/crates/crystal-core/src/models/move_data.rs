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
    } else if !is_exact_nonempty_move_token(&move_data.name) {
        issues.push(MovePayloadIssue::InvalidName {
            name: move_data.name.clone(),
        });
    }
    if move_data.move_type.trim().is_empty() {
        issues.push(MovePayloadIssue::MissingType);
    } else if !is_exact_nonempty_move_token(&move_data.move_type) {
        issues.push(MovePayloadIssue::InvalidType {
            move_type: move_data.move_type.clone(),
        });
    }
    if move_data.effect.trim().is_empty() {
        issues.push(MovePayloadIssue::MissingEffect);
    } else if !is_exact_nonempty_move_token(&move_data.effect) {
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
        if !is_exact_nonempty_move_token(move_name) {
            issues.push(MoveNameCatalogIssue::InvalidName { index });
        }
    }

    issues
}

fn is_exact_nonempty_move_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
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
    fn move_payload_issues_require_exact_pack_owned_ids_without_effect_enums() {
        let move_data = Move {
            name: "AETHER PULSE".to_string(),
            move_type: pokemon_type("AETHER TYPE"),
            power: 60,
            accuracy: 100,
            pp: 15,
            effect: "MODDED EFFECT".to_string(),
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
                    effect: "MODDED EFFECT".to_string(),
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
                    "KARATE CHOP".to_string()
                ],
                2,
            ),
            vec![
                MoveNameCatalogIssue::CountMismatch {
                    actual_count: 3,
                    expected_count: 2,
                },
                MoveNameCatalogIssue::InvalidName { index: 1 },
                MoveNameCatalogIssue::InvalidName { index: 2 },
            ],
        );
    }

    #[test]
    fn move_name_catalog_issues_allow_absent_partial_pack_names() {
        assert!(move_name_catalog_issues(&[], 251).is_empty());
    }
}
