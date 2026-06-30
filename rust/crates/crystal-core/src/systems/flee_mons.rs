use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct FleeMonTables {
    pub buckets: BTreeMap<String, Vec<String>>,
}

impl<'de> Deserialize<'de> for FleeMonTables {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawFleeMonTables {
            buckets: Option<BTreeMap<String, Vec<String>>>,
            #[serde(default)]
            always: Vec<String>,
            #[serde(default)]
            often: Vec<String>,
            #[serde(default)]
            sometimes: Vec<String>,
        }

        let raw = RawFleeMonTables::deserialize(deserializer)?;
        let mut buckets = raw.buckets.unwrap_or_default();
        if !raw.always.is_empty() {
            buckets.insert("always".to_string(), raw.always);
        }
        if !raw.often.is_empty() {
            buckets.insert("often".to_string(), raw.often);
        }
        if !raw.sometimes.is_empty() {
            buckets.insert("sometimes".to_string(), raw.sometimes);
        }
        if buckets.is_empty() {
            return Err(serde::de::Error::missing_field("buckets"));
        }

        for (bucket_id, species) in &buckets {
            if !is_exact_nonempty_flee_mon_bucket(bucket_id) {
                return Err(serde::de::Error::custom(format!(
                    "flee mons bucket id must be exact lowercase ASCII/underscore, found {bucket_id:?}"
                )));
            }
            if species.is_empty() {
                return Err(serde::de::Error::custom(format!(
                    "flee mons bucket {bucket_id:?} must not be empty"
                )));
            }
            for species_id in species {
                if !is_exact_nonempty_flee_mon_token(species_id) {
                    return Err(serde::de::Error::custom(format!(
                        "flee mons species id must be exact ASCII alphanumeric/underscore, found {species_id:?}"
                    )));
                }
            }
        }

        Ok(Self {
            buckets,
        })
    }
}

impl FleeMonTables {
    pub fn for_crystal(always: Vec<String>, often: Vec<String>, sometimes: Vec<String>) -> Self {
        Self {
            buckets: BTreeMap::from([
                ("always".to_string(), always),
                ("often".to_string(), often),
                ("sometimes".to_string(), sometimes),
            ]),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.buckets.values().all(Vec::is_empty)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FleeMonCatalogIssue {
    InvalidBucketId { bucket_id: String },
    EmptyBucket { bucket_id: String },
    InvalidSpeciesId { species_id: String },
    UnknownSpecies { species_id: String },
}

pub fn flee_mon_catalog_issues(
    flee_mons: &FleeMonTables,
    species_ids: &BTreeSet<String>,
) -> Vec<FleeMonCatalogIssue> {
    let mut issues = Vec::new();
    for (bucket_id, species) in &flee_mons.buckets {
        if !is_exact_nonempty_flee_mon_bucket(bucket_id) {
            issues.push(FleeMonCatalogIssue::InvalidBucketId {
                bucket_id: bucket_id.clone(),
            });
            continue;
        }
        if species.is_empty() {
            issues.push(FleeMonCatalogIssue::EmptyBucket {
                bucket_id: bucket_id.clone(),
            });
        }
        for species_id in species {
            if !is_exact_nonempty_flee_mon_token(species_id) {
                issues.push(FleeMonCatalogIssue::InvalidSpeciesId {
                    species_id: species_id.clone(),
                });
            } else if !species_ids.contains(species_id) {
                issues.push(FleeMonCatalogIssue::UnknownSpecies {
                    species_id: species_id.clone(),
                });
            }
        }
    }
    issues
}

fn is_exact_nonempty_flee_mon_bucket(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !has_reserved_pack_prefix(value)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
}

fn is_exact_nonempty_flee_mon_token(value: &str) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flee_mon_catalog_issues_require_exact_species_ids() {
        let flee_mons = FleeMonTables::for_crystal(
            vec![
                "RAIKOU".to_string(),
                " raikou".to_string(),
                "RAI KOU".to_string(),
                "raikou".to_string(),
            ],
            vec!["ENTEI".to_string()],
            vec!["Suicune".to_string()],
        );
        let species_ids = ["ENTEI".to_string(), "RAIKOU".to_string()]
            .into_iter()
            .collect();

        assert_eq!(
            flee_mon_catalog_issues(&flee_mons, &species_ids),
            vec![
                FleeMonCatalogIssue::InvalidSpeciesId {
                    species_id: " raikou".to_string(),
                },
                FleeMonCatalogIssue::InvalidSpeciesId {
                    species_id: "RAI KOU".to_string(),
                },
                FleeMonCatalogIssue::UnknownSpecies {
                    species_id: "raikou".to_string(),
                },
                FleeMonCatalogIssue::UnknownSpecies {
                    species_id: "Suicune".to_string(),
                },
            ],
        );
    }

    #[test]
    fn flee_mon_catalog_issues_reject_reserved_pack_prefix_tokens() {
        let flee_mons = FleeMonTables {
            buckets: [
                ("fallback_bucket".to_string(), vec!["RAIKOU".to_string()]),
                (
                    "always".to_string(),
                    vec!["fallback_raikou".to_string(), "legacy_entei".to_string()],
                ),
            ]
            .into_iter()
            .collect(),
        };

        assert_eq!(
            flee_mon_catalog_issues(&flee_mons, &BTreeSet::new()),
            vec![
                FleeMonCatalogIssue::InvalidSpeciesId {
                    species_id: "fallback_raikou".to_string(),
                },
                FleeMonCatalogIssue::InvalidSpeciesId {
                    species_id: "legacy_entei".to_string(),
                },
                FleeMonCatalogIssue::InvalidBucketId {
                    bucket_id: "fallback_bucket".to_string(),
                },
            ]
        );
    }

    #[test]
    fn flee_mon_tables_require_explicit_pack_bucket_map() {
        let missing_bucket = serde_json::from_str::<FleeMonTables>(r#"{}"#)
            .expect_err("flee mon bucket map must be explicit")
            .to_string();

        assert!(missing_bucket.contains("missing field `buckets`"));
    }

    #[test]
    fn flee_mon_tables_reject_unknown_pack_fields() {
        let unknown_field =
            serde_json::from_str::<FleeMonTables>(r#"{"buckets":{"always":[]},"fallback":[]}"#)
                .expect_err("flee mon tables reject unknown pack fields")
                .to_string();

        assert!(unknown_field.contains("unknown field"));
    }

    #[test]
    fn flee_mon_tables_accept_exact_custom_bucket_ids() {
        let flee_mons: FleeMonTables = serde_json::from_str(
            r#"{"buckets":{"always":["RAIKOU"],"deep_cave":["ZUBAT"],"empty":[]}}"#,
        )
        .expect("exact custom bucket ids are pack data");
        let species_ids = ["RAIKOU".to_string(), "ZUBAT".to_string()]
            .into_iter()
            .collect();

        assert_eq!(
            flee_mon_catalog_issues(&flee_mons, &species_ids),
            vec![FleeMonCatalogIssue::EmptyBucket {
                bucket_id: "empty".to_string(),
            }]
        );
    }

    #[test]
    fn flee_mon_tables_reject_malformed_json_tokens() {
        let bucket_error =
            serde_json::from_str::<FleeMonTables>(r#"{"buckets":{"DeepCave":["GOLBAT"]}}"#)
                .expect_err("flee mon bucket ids must be exact during JSON load")
                .to_string();
        assert!(
            bucket_error.contains("flee mons bucket id must be exact lowercase ASCII/underscore"),
            "{bucket_error}"
        );

        let species_error =
            serde_json::from_str::<FleeMonTables>(r#"{"buckets":{"always":["RAI KOU"]}}"#)
                .expect_err("flee mon species ids must be exact during JSON load")
                .to_string();
        assert!(
            species_error
                .contains("flee mons species id must be exact ASCII alphanumeric/underscore"),
            "{species_error}"
        );

        let reserved_bucket_error =
            serde_json::from_str::<FleeMonTables>(r#"{"buckets":{"legacy_bucket":["GOLBAT"]}}"#)
                .expect_err("reserved flee mon bucket ids must reject during JSON load")
                .to_string();
        assert!(
            reserved_bucket_error
                .contains("flee mons bucket id must be exact lowercase ASCII/underscore"),
            "{reserved_bucket_error}"
        );

        let reserved_species_error =
            serde_json::from_str::<FleeMonTables>(r#"{"buckets":{"always":["fallback_raikou"]}}"#)
                .expect_err("reserved flee mon species ids must reject during JSON load")
                .to_string();
        assert!(
            reserved_species_error
                .contains("flee mons species id must be exact ASCII alphanumeric/underscore"),
            "{reserved_species_error}"
        );
    }
}
