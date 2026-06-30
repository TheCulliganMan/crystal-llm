use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize, de::Error as _};

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct MenuIconTable(pub BTreeMap<String, String>);

impl<'de> Deserialize<'de> for MenuIconTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let values = BTreeMap::<String, String>::deserialize(deserializer)?;
        if values.is_empty() {
            return Err(D::Error::custom("menu icon table must not be empty"));
        }
        for (species_id, icon) in &values {
            if !is_exact_nonempty_menu_icon_token(species_id) {
                return Err(D::Error::custom(format!(
                    "invalid menu icon species id: {species_id}"
                )));
            }
            if !is_exact_nonempty_menu_icon_token(icon) {
                return Err(D::Error::custom(format!(
                    "invalid menu icon value for species id: {species_id}"
                )));
            }
        }
        Ok(Self(values))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuIconCatalogIssue {
    InvalidSpeciesId { species_id: String },
    UnknownSpecies { species_id: String },
    InvalidIcon { species_id: String },
    MissingSpeciesIcon { species_id: String },
}

pub fn menu_icon_catalog_issues(
    menu_icons: &BTreeMap<String, String>,
    species_ids: &BTreeSet<String>,
) -> Vec<MenuIconCatalogIssue> {
    let mut issues = Vec::new();

    for (species_id, icon) in menu_icons {
        if !is_exact_nonempty_menu_icon_token(species_id) {
            issues.push(MenuIconCatalogIssue::InvalidSpeciesId {
                species_id: species_id.clone(),
            });
        } else if species_id != "EGG" && !species_ids.contains(species_id) {
            issues.push(MenuIconCatalogIssue::UnknownSpecies {
                species_id: species_id.clone(),
            });
        }
        if !is_exact_nonempty_menu_icon_token(icon) {
            issues.push(MenuIconCatalogIssue::InvalidIcon {
                species_id: species_id.clone(),
            });
        }
    }

    for species_id in species_ids {
        if !menu_icons.contains_key(species_id) {
            issues.push(MenuIconCatalogIssue::MissingSpeciesIcon {
                species_id: species_id.clone(),
            });
        }
    }

    issues
}

fn is_exact_nonempty_menu_icon_token(value: &str) -> bool {
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
    fn menu_icon_catalog_issues_require_exact_species_icons() {
        let menu_icons = [
            ("CHIKORITA".to_string(), String::new()),
            ("EGG".to_string(), "ICON_EGG".to_string()),
            (" CYNDAQUIL".to_string(), "ICON_CYNDAQUIL".to_string()),
            ("NEW MON".to_string(), "ICON_NEW MON".to_string()),
            ("TOTODILE".to_string(), " ICON_TOTODILE".to_string()),
            ("missingno".to_string(), "ICON_GLITCH".to_string()),
        ]
        .into_iter()
        .collect();
        let species_ids = [
            "BAYLEEF".to_string(),
            "CHIKORITA".to_string(),
            "TOTODILE".to_string(),
        ]
        .into_iter()
        .collect();

        assert_eq!(
            menu_icon_catalog_issues(&menu_icons, &species_ids),
            vec![
                MenuIconCatalogIssue::InvalidSpeciesId {
                    species_id: " CYNDAQUIL".to_string(),
                },
                MenuIconCatalogIssue::InvalidIcon {
                    species_id: "CHIKORITA".to_string(),
                },
                MenuIconCatalogIssue::InvalidSpeciesId {
                    species_id: "NEW MON".to_string(),
                },
                MenuIconCatalogIssue::InvalidIcon {
                    species_id: "NEW MON".to_string(),
                },
                MenuIconCatalogIssue::InvalidIcon {
                    species_id: "TOTODILE".to_string(),
                },
                MenuIconCatalogIssue::UnknownSpecies {
                    species_id: "missingno".to_string(),
                },
                MenuIconCatalogIssue::MissingSpeciesIcon {
                    species_id: "BAYLEEF".to_string(),
                },
            ],
        );
    }

    #[test]
    fn menu_icon_catalog_issues_reject_reserved_pack_prefix_tokens() {
        let menu_icons = [(
            "fallback_chikorita".to_string(),
            "legacy_icon_chikorita".to_string(),
        )]
        .into_iter()
        .collect();

        assert_eq!(
            menu_icon_catalog_issues(&menu_icons, &BTreeSet::new()),
            vec![
                MenuIconCatalogIssue::InvalidSpeciesId {
                    species_id: "fallback_chikorita".to_string(),
                },
                MenuIconCatalogIssue::InvalidIcon {
                    species_id: "fallback_chikorita".to_string(),
                },
            ]
        );
    }
}
