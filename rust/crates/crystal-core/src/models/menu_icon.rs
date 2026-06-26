use std::collections::{BTreeMap, BTreeSet};

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
    !value.is_empty() && value.trim() == value
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
}
