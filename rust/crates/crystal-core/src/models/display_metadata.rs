use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpritePaletteDefaultIssue {
    InvalidDefault { sprite_id: String },
}

pub fn sprite_palette_default_issues(
    defaults: &BTreeMap<String, i64>,
) -> Vec<SpritePaletteDefaultIssue> {
    defaults
        .iter()
        .filter(|(sprite_id, palette)| !is_exact_nonempty_display_token(sprite_id) || **palette < 0)
        .map(|(sprite_id, _)| SpritePaletteDefaultIssue::InvalidDefault {
            sprite_id: sprite_id.clone(),
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PokegearTownMapPaletteIssue {
    InvalidEntry { map_name: String },
}

pub fn pokegear_town_map_palette_issues(
    palette_map: &BTreeMap<String, Vec<String>>,
) -> Vec<PokegearTownMapPaletteIssue> {
    palette_map
        .iter()
        .filter(|(name, palettes)| {
            !is_exact_nonempty_display_token(name)
                || palettes.is_empty()
                || palettes
                    .iter()
                    .any(|entry| !is_exact_nonempty_display_token(entry))
        })
        .map(|(map_name, _)| PokegearTownMapPaletteIssue::InvalidEntry {
            map_name: map_name.clone(),
        })
        .collect()
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PokegearLandmarksPayload {
    pub landmarks: Vec<PokegearLandmark>,
    pub map_to_landmark: BTreeMap<String, String>,
}

impl<'de> Deserialize<'de> for PokegearLandmarksPayload {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawPokegearLandmarksPayload {
            landmarks: Vec<PokegearLandmark>,
            map_to_landmark: BTreeMap<String, String>,
        }

        let raw = RawPokegearLandmarksPayload::deserialize(deserializer)?;
        validate_display_token_values("pokegear landmark map", raw.map_to_landmark.keys())?;
        validate_landmark_constant_values(
            "pokegear landmark map reference",
            raw.map_to_landmark.values(),
        )?;
        Ok(Self {
            landmarks: raw.landmarks,
            map_to_landmark: raw.map_to_landmark,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PokegearLandmark {
    pub id: u16,
    #[serde(deserialize_with = "required_landmark_constant")]
    pub constant: String,
    #[serde(deserialize_with = "required_display_token")]
    pub label: String,
    #[serde(deserialize_with = "required_display_text")]
    pub name: String,
    pub x: i16,
    pub y: i16,
    #[serde(deserialize_with = "required_display_token")]
    pub region: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PokegearLandmarkIssue {
    InvalidLandmark {
        constant: String,
    },
    InvalidConstant {
        constant: String,
    },
    InvalidMapEntry {
        map_name: String,
    },
    InvalidLandmarkReference {
        map_name: String,
        landmark_constant: String,
    },
    UnknownMap {
        map_name: String,
    },
    UnknownLandmarkConstant {
        map_name: String,
        landmark_constant: String,
    },
}

pub fn pokegear_landmark_issues(
    payload: &PokegearLandmarksPayload,
    map_names: &BTreeSet<String>,
) -> Vec<PokegearLandmarkIssue> {
    let mut issues = Vec::new();
    let landmark_constants: BTreeSet<&str> = payload
        .landmarks
        .iter()
        .filter(|landmark| is_valid_landmark_constant(&landmark.constant))
        .map(|landmark| landmark.constant.as_str())
        .collect();

    for landmark in &payload.landmarks {
        if !is_exact_nonempty_display_token(&landmark.constant)
            || !is_exact_nonempty_display_token(&landmark.label)
            || !is_exact_nonempty_display_text(&landmark.name)
            || !is_exact_nonempty_display_token(&landmark.region)
        {
            issues.push(PokegearLandmarkIssue::InvalidLandmark {
                constant: landmark.constant.clone(),
            });
        }
        if is_exact_nonempty_display_token(&landmark.constant)
            && !is_valid_landmark_constant(&landmark.constant)
        {
            issues.push(PokegearLandmarkIssue::InvalidConstant {
                constant: landmark.constant.clone(),
            });
        }
    }

    for (map_name, landmark_constant) in &payload.map_to_landmark {
        let invalid_map_name = !is_exact_nonempty_display_token(map_name);
        let invalid_landmark_constant = !is_valid_landmark_constant(landmark_constant);
        if invalid_map_name {
            issues.push(PokegearLandmarkIssue::InvalidMapEntry {
                map_name: map_name.clone(),
            });
        }
        if invalid_landmark_constant {
            issues.push(PokegearLandmarkIssue::InvalidLandmarkReference {
                map_name: map_name.clone(),
                landmark_constant: landmark_constant.clone(),
            });
        }
        if invalid_map_name || invalid_landmark_constant {
            continue;
        }
        if !map_names.contains(map_name) {
            issues.push(PokegearLandmarkIssue::UnknownMap {
                map_name: map_name.clone(),
            });
        }
        if !landmark_constants.contains(landmark_constant.as_str()) {
            issues.push(PokegearLandmarkIssue::UnknownLandmarkConstant {
                map_name: map_name.clone(),
                landmark_constant: landmark_constant.clone(),
            });
        }
    }

    issues
}

fn is_exact_nonempty_display_token(value: &str) -> bool {
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

fn is_valid_landmark_constant(value: &str) -> bool {
    is_exact_nonempty_display_token(value) && value.starts_with("LANDMARK_")
}

fn is_exact_nonempty_display_text(value: &str) -> bool {
    !value.is_empty() && value.trim() == value
}

fn validate_display_token_values<E, I>(field: &str, values: I) -> Result<(), E>
where
    E: serde::de::Error,
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    for value in values {
        let value = value.as_ref();
        if !is_exact_nonempty_display_token(value) {
            return Err(E::custom(format!(
                "{field} must be exact ASCII alphanumeric/underscore, found {value:?}"
            )));
        }
    }
    Ok(())
}

fn validate_landmark_constant_values<E, I>(field: &str, values: I) -> Result<(), E>
where
    E: serde::de::Error,
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    for value in values {
        let value = value.as_ref();
        if !is_valid_landmark_constant(value) {
            return Err(E::custom(format!(
                "{field} must be an exact LANDMARK_ token, found {value:?}"
            )));
        }
    }
    Ok(())
}

fn required_display_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_nonempty_display_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "display token must be exact ASCII alphanumeric/underscore, found {value:?}"
        )))
    }
}

fn required_landmark_constant<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_valid_landmark_constant(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "pokegear landmark constant must be an exact LANDMARK_ token, found {value:?}"
        )))
    }
}

fn required_display_text<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_nonempty_display_text(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "display text must be exact non-empty text, found {value:?}"
        )))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeBundleIssue {
    InvalidJson { error: String },
    NotObject,
    MissingSection { section: String },
    UnknownSection { section: String },
}

pub fn runtime_bundle_issues(value: &str, sections: &[&str]) -> Vec<RuntimeBundleIssue> {
    if value.trim().is_empty() {
        return Vec::new();
    }

    let value: serde_json::Value = match serde_json::from_str(value) {
        Ok(value) => value,
        Err(error) => {
            return vec![RuntimeBundleIssue::InvalidJson {
                error: error.to_string(),
            }];
        }
    };
    let Some(object) = value.as_object() else {
        return vec![RuntimeBundleIssue::NotObject];
    };

    let required_issues = sections
        .iter()
        .filter(|section| {
            !matches!(
                object.get(**section).and_then(serde_json::Value::as_object),
                Some(section_object) if !section_object.is_empty()
            )
        })
        .map(|section| RuntimeBundleIssue::MissingSection {
            section: (*section).to_string(),
        })
        .collect::<Vec<_>>();
    if !required_issues.is_empty() {
        return required_issues;
    }

    let required_sections = sections
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    object
        .keys()
        .filter(|section| !required_sections.contains(section.as_str()))
        .map(|section| RuntimeBundleIssue::UnknownSection {
            section: section.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_palette_default_issues_require_exact_nonnegative_defaults() {
        let defaults = [
            ("".to_string(), 0),
            (" SPRITE_SILVER".to_string(), 0),
            ("SPRITE SILVER".to_string(), 0),
            ("SPRITE_CHRIS".to_string(), 0),
            ("SPRITE_KRIS".to_string(), -1),
        ]
        .into_iter()
        .collect();

        assert_eq!(
            sprite_palette_default_issues(&defaults),
            vec![
                SpritePaletteDefaultIssue::InvalidDefault {
                    sprite_id: String::new(),
                },
                SpritePaletteDefaultIssue::InvalidDefault {
                    sprite_id: " SPRITE_SILVER".to_string(),
                },
                SpritePaletteDefaultIssue::InvalidDefault {
                    sprite_id: "SPRITE SILVER".to_string(),
                },
                SpritePaletteDefaultIssue::InvalidDefault {
                    sprite_id: "SPRITE_KRIS".to_string(),
                },
            ],
        );
    }

    #[test]
    fn pokegear_town_map_palette_issues_require_nonempty_palette_lists() {
        let palette_map = [
            ("".to_string(), vec!["PAL_GREEN".to_string()]),
            (" ROUTE_30".to_string(), vec!["PAL_GREEN".to_string()]),
            ("ROUTE 30".to_string(), vec!["PAL_GREEN".to_string()]),
            ("NEW_BARK_TOWN".to_string(), Vec::new()),
            (
                "ROUTE_29".to_string(),
                vec!["PAL_GREEN".to_string(), String::new()],
            ),
            ("ROUTE_30".to_string(), vec![" PAL_GREEN".to_string()]),
            ("CHERRYGROVE_CITY".to_string(), vec!["PAL_RED".to_string()]),
        ]
        .into_iter()
        .collect();

        assert_eq!(
            pokegear_town_map_palette_issues(&palette_map),
            vec![
                PokegearTownMapPaletteIssue::InvalidEntry {
                    map_name: String::new(),
                },
                PokegearTownMapPaletteIssue::InvalidEntry {
                    map_name: " ROUTE_30".to_string(),
                },
                PokegearTownMapPaletteIssue::InvalidEntry {
                    map_name: "NEW_BARK_TOWN".to_string(),
                },
                PokegearTownMapPaletteIssue::InvalidEntry {
                    map_name: "ROUTE 30".to_string(),
                },
                PokegearTownMapPaletteIssue::InvalidEntry {
                    map_name: "ROUTE_29".to_string(),
                },
                PokegearTownMapPaletteIssue::InvalidEntry {
                    map_name: "ROUTE_30".to_string(),
                },
            ],
        );
    }

    #[test]
    fn display_metadata_issues_reject_reserved_pack_prefix_tokens() {
        let defaults = [("fallback_sprite_chris".to_string(), 0)]
            .into_iter()
            .collect();
        assert_eq!(
            sprite_palette_default_issues(&defaults),
            vec![SpritePaletteDefaultIssue::InvalidDefault {
                sprite_id: "fallback_sprite_chris".to_string(),
            }]
        );

        let palette_map = [(
            "legacy_route_29".to_string(),
            vec!["fallback_palette".to_string()],
        )]
        .into_iter()
        .collect();
        assert_eq!(
            pokegear_town_map_palette_issues(&palette_map),
            vec![PokegearTownMapPaletteIssue::InvalidEntry {
                map_name: "legacy_route_29".to_string(),
            }]
        );
    }

    #[test]
    fn pokegear_landmark_issues_require_exact_constants_and_known_maps() {
        let payload = PokegearLandmarksPayload {
            landmarks: vec![
                PokegearLandmark {
                    id: 1,
                    constant: "LANDMARK_ROUTE_29".to_string(),
                    label: "ROUTE_29".to_string(),
                    name: "Route 29".to_string(),
                    x: 2,
                    y: 3,
                    region: "johto".to_string(),
                },
                PokegearLandmark {
                    id: 2,
                    constant: "ROUTE_30".to_string(),
                    label: " ROUTE_30".to_string(),
                    name: "Route 30".to_string(),
                    x: 4,
                    y: 5,
                    region: "johto".to_string(),
                },
                PokegearLandmark {
                    id: 3,
                    constant: "LANDMARK_ROUTE_31".to_string(),
                    label: "ROUTE 31".to_string(),
                    name: "Route 31".to_string(),
                    x: 6,
                    y: 7,
                    region: "johto".to_string(),
                },
            ],
            map_to_landmark: [
                ("Route29".to_string(), "LANDMARK_ROUTE_30".to_string()),
                ("MissingRoute".to_string(), "LANDMARK_ROUTE_29".to_string()),
                (" Route29".to_string(), "LANDMARK_ROUTE_29".to_string()),
                ("Route 30".to_string(), "LANDMARK_ROUTE_29".to_string()),
                ("Route30".to_string(), " LANDMARK_ROUTE_29".to_string()),
                ("Route31".to_string(), "ROUTE_30".to_string()),
            ]
            .into_iter()
            .collect(),
        };
        let map_names = ["Route29".to_string(), "Route31".to_string()]
            .into_iter()
            .collect();

        assert_eq!(
            pokegear_landmark_issues(&payload, &map_names),
            vec![
                PokegearLandmarkIssue::InvalidLandmark {
                    constant: "ROUTE_30".to_string(),
                },
                PokegearLandmarkIssue::InvalidConstant {
                    constant: "ROUTE_30".to_string(),
                },
                PokegearLandmarkIssue::InvalidLandmark {
                    constant: "LANDMARK_ROUTE_31".to_string(),
                },
                PokegearLandmarkIssue::InvalidMapEntry {
                    map_name: " Route29".to_string(),
                },
                PokegearLandmarkIssue::UnknownMap {
                    map_name: "MissingRoute".to_string(),
                },
                PokegearLandmarkIssue::InvalidMapEntry {
                    map_name: "Route 30".to_string(),
                },
                PokegearLandmarkIssue::UnknownLandmarkConstant {
                    map_name: "Route29".to_string(),
                    landmark_constant: "LANDMARK_ROUTE_30".to_string(),
                },
                PokegearLandmarkIssue::InvalidLandmarkReference {
                    map_name: "Route30".to_string(),
                    landmark_constant: " LANDMARK_ROUTE_29".to_string(),
                },
                PokegearLandmarkIssue::InvalidLandmarkReference {
                    map_name: "Route31".to_string(),
                    landmark_constant: "ROUTE_30".to_string(),
                },
            ],
        );
    }

    #[test]
    fn pokegear_landmark_issues_reject_reserved_pack_prefix_tokens() {
        let payload = PokegearLandmarksPayload {
            landmarks: vec![PokegearLandmark {
                id: 1,
                constant: "fallback_landmark_route_29".to_string(),
                label: "legacy_route_29".to_string(),
                name: "Route 29".to_string(),
                x: 2,
                y: 3,
                region: "fallback_region".to_string(),
            }],
            map_to_landmark: [(
                "legacy_route_29".to_string(),
                "fallback_landmark_route_29".to_string(),
            )]
            .into_iter()
            .collect(),
        };

        assert_eq!(
            pokegear_landmark_issues(&payload, &BTreeSet::new()),
            vec![
                PokegearLandmarkIssue::InvalidLandmark {
                    constant: "fallback_landmark_route_29".to_string(),
                },
                PokegearLandmarkIssue::InvalidMapEntry {
                    map_name: "legacy_route_29".to_string(),
                },
                PokegearLandmarkIssue::InvalidLandmarkReference {
                    map_name: "legacy_route_29".to_string(),
                    landmark_constant: "fallback_landmark_route_29".to_string(),
                },
            ]
        );
    }

    #[test]
    fn pokegear_landmarks_json_rejects_malformed_pack_fields_at_deserialization() {
        let valid_landmark = serde_json::json!({
            "id": 1,
            "constant": "LANDMARK_ROUTE_29",
            "label": "ROUTE_29",
            "name": "Route 29",
            "x": 2,
            "y": 3,
            "region": "JOHTO"
        });
        let cases = [
            (
                "constant",
                serde_json::json!({
                    "landmarks": [{
                        "id": 1,
                        "constant": "ROUTE_29",
                        "label": "ROUTE_29",
                        "name": "Route 29",
                        "x": 2,
                        "y": 3,
                        "region": "JOHTO"
                    }],
                    "map_to_landmark": {}
                }),
            ),
            (
                "label",
                serde_json::json!({
                    "landmarks": [{
                        "id": 1,
                        "constant": "LANDMARK_ROUTE_29",
                        "label": "ROUTE 29",
                        "name": "Route 29",
                        "x": 2,
                        "y": 3,
                        "region": "JOHTO"
                    }],
                    "map_to_landmark": {}
                }),
            ),
            (
                "name",
                serde_json::json!({
                    "landmarks": [{
                        "id": 1,
                        "constant": "LANDMARK_ROUTE_29",
                        "label": "ROUTE_29",
                        "name": " Route 29",
                        "x": 2,
                        "y": 3,
                        "region": "JOHTO"
                    }],
                    "map_to_landmark": {}
                }),
            ),
            (
                "region",
                serde_json::json!({
                    "landmarks": [{
                        "id": 1,
                        "constant": "LANDMARK_ROUTE_29",
                        "label": "ROUTE_29",
                        "name": "Route 29",
                        "x": 2,
                        "y": 3,
                        "region": "JOH TO"
                    }],
                    "map_to_landmark": {}
                }),
            ),
            (
                "map key",
                serde_json::json!({
                    "landmarks": [valid_landmark.clone()],
                    "map_to_landmark": {"Route 29": "LANDMARK_ROUTE_29"}
                }),
            ),
            (
                "map reference",
                serde_json::json!({
                    "landmarks": [valid_landmark],
                    "map_to_landmark": {"Route29": "ROUTE_29"}
                }),
            ),
        ];

        for (label, payload) in cases {
            let error = serde_json::from_value::<PokegearLandmarksPayload>(payload)
                .expect_err("malformed pokegear landmarks must fail during JSON load")
                .to_string();
            assert!(
                error.contains("display")
                    || error.contains("pokegear landmark")
                    || error.contains("LANDMARK_"),
                "{label} produced unexpected error: {error}"
            );
        }
    }

    #[test]
    fn runtime_bundle_issues_require_object_with_nonempty_sections() {
        assert_eq!(
            runtime_bundle_issues("{", &["objects"]),
            vec![RuntimeBundleIssue::InvalidJson {
                error: "EOF while parsing an object at line 1 column 1".to_string(),
            }],
        );
        assert_eq!(
            runtime_bundle_issues("[]", &["objects"]),
            vec![RuntimeBundleIssue::NotObject],
        );
        assert_eq!(
            runtime_bundle_issues(
                r#"{"objects":{"obj":{}},"framesets":{},"oam_sets":{"set":{}}}"#,
                &["objects", "framesets", "oam_sets"],
            ),
            vec![RuntimeBundleIssue::MissingSection {
                section: "framesets".to_string(),
            }],
        );
        assert_eq!(
            runtime_bundle_issues(
                r#"{"objects":{"obj":{}},"framesets":{"fs":{}},"fallback_objects":{"obj":{}}}"#,
                &["objects", "framesets"],
            ),
            vec![RuntimeBundleIssue::UnknownSection {
                section: "fallback_objects".to_string(),
            }],
        );
        assert!(runtime_bundle_issues("", &["objects"]).is_empty());
    }
}
