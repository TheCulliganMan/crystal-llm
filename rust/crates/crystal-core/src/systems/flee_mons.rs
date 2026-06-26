use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FleeMonTables {
    pub always: Vec<String>,
    pub often: Vec<String>,
    pub sometimes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FleeMonCatalogIssue {
    InvalidSpeciesId { species_id: String },
    UnknownSpecies { species_id: String },
}

pub fn flee_mon_catalog_issues(
    flee_mons: &FleeMonTables,
    species_ids: &BTreeSet<String>,
) -> Vec<FleeMonCatalogIssue> {
    flee_mons
        .always
        .iter()
        .chain(flee_mons.often.iter())
        .chain(flee_mons.sometimes.iter())
        .filter_map(|species_id| {
            if !is_exact_nonempty_flee_mon_token(species_id) {
                Some(FleeMonCatalogIssue::InvalidSpeciesId {
                    species_id: species_id.clone(),
                })
            } else if !species_ids.contains(species_id) {
                Some(FleeMonCatalogIssue::UnknownSpecies {
                    species_id: species_id.clone(),
                })
            } else {
                None
            }
        })
        .collect()
}

fn is_exact_nonempty_flee_mon_token(value: &str) -> bool {
    !value.is_empty() && value.trim() == value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flee_mon_catalog_issues_require_exact_species_ids() {
        let flee_mons = FleeMonTables {
            always: vec![
                "RAIKOU".to_string(),
                " raikou".to_string(),
                "raikou".to_string(),
            ],
            often: vec!["ENTEI".to_string()],
            sometimes: vec!["Suicune".to_string()],
        };
        let species_ids = ["ENTEI".to_string(), "RAIKOU".to_string()]
            .into_iter()
            .collect();

        assert_eq!(
            flee_mon_catalog_issues(&flee_mons, &species_ids),
            vec![
                FleeMonCatalogIssue::InvalidSpeciesId {
                    species_id: " raikou".to_string(),
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
    fn flee_mon_tables_require_explicit_pack_fields() {
        let missing_bucket = serde_json::from_str::<FleeMonTables>(r#"{"always":[],"often":[]}"#)
            .expect_err("flee mon buckets must all be explicit")
            .to_string();

        assert!(missing_bucket.contains("missing field `sometimes`"));
    }

    #[test]
    fn flee_mon_tables_reject_unknown_pack_fields() {
        let unknown_field = serde_json::from_str::<FleeMonTables>(
            r#"{"always":[],"often":[],"sometimes":[],"fallback":[]}"#,
        )
        .expect_err("flee mon tables reject unknown pack fields")
        .to_string();

        assert!(unknown_field.contains("unknown field"));
    }
}
