use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::models::PokemonSpecies;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LearnsetEntry(pub u8, pub String);

pub type SpeciesLearnsets = BTreeMap<String, Vec<LearnsetEntry>>;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LearnsetError {
    #[error("invalid learnset species id '{species_id}'")]
    InvalidSpecies { species_id: String },
    #[error("missing level-up learnset for species '{species_id}'")]
    MissingSpecies { species_id: String },
    #[error("invalid level-up move id '{move_id}' for species '{species_id}'")]
    InvalidMove { species_id: String, move_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LearnsetCatalogIssue {
    MissingSpeciesLearnset { species_id: String },
    InvalidSpeciesHeldItem { species_id: String, item_id: String },
    UnknownSpeciesHeldItem { species_id: String, item_id: String },
    InvalidTmHmMove { species_id: String, move_id: String },
    UnknownTmHmMove { species_id: String, move_id: String },
    InvalidLearnsetSpecies { species_id: String },
    UnknownLearnsetSpecies { species_id: String },
    InvalidLevelMove { species_id: String, move_id: String },
    UnknownLevelMove { species_id: String, move_id: String },
}

pub fn learnset_catalog_issues(
    species: &BTreeMap<String, PokemonSpecies>,
    learnsets: &SpeciesLearnsets,
    item_ids: &BTreeSet<String>,
    move_ids: &BTreeSet<String>,
) -> Vec<LearnsetCatalogIssue> {
    let mut issues = Vec::new();
    for (species_id, species_data) in species {
        if !learnsets.contains_key(species_id) {
            issues.push(LearnsetCatalogIssue::MissingSpeciesLearnset {
                species_id: species_id.clone(),
            });
        }
        for item_id in [species_data.item1.as_deref(), species_data.item2.as_deref()]
            .into_iter()
            .flatten()
        {
            if !is_exact_nonempty_learnset_token(item_id) {
                issues.push(LearnsetCatalogIssue::InvalidSpeciesHeldItem {
                    species_id: species_id.clone(),
                    item_id: item_id.to_string(),
                });
            } else if !item_ids.contains(item_id) {
                issues.push(LearnsetCatalogIssue::UnknownSpeciesHeldItem {
                    species_id: species_id.clone(),
                    item_id: item_id.to_string(),
                });
            }
        }
        for move_id in &species_data.tmhm_learnset {
            if !is_exact_nonempty_learnset_token(move_id) {
                issues.push(LearnsetCatalogIssue::InvalidTmHmMove {
                    species_id: species_id.clone(),
                    move_id: move_id.clone(),
                });
            } else if !move_ids.contains(move_id) {
                issues.push(LearnsetCatalogIssue::UnknownTmHmMove {
                    species_id: species_id.clone(),
                    move_id: move_id.clone(),
                });
            }
        }
    }
    for (species_id, learnset) in learnsets {
        if !is_exact_nonempty_learnset_token(species_id) {
            issues.push(LearnsetCatalogIssue::InvalidLearnsetSpecies {
                species_id: species_id.clone(),
            });
        } else if !species.contains_key(species_id) {
            issues.push(LearnsetCatalogIssue::UnknownLearnsetSpecies {
                species_id: species_id.clone(),
            });
        }
        for entry in learnset {
            if !is_exact_nonempty_learnset_token(&entry.1) {
                issues.push(LearnsetCatalogIssue::InvalidLevelMove {
                    species_id: species_id.clone(),
                    move_id: entry.1.clone(),
                });
            } else if !move_ids.contains(&entry.1) {
                issues.push(LearnsetCatalogIssue::UnknownLevelMove {
                    species_id: species_id.clone(),
                    move_id: entry.1.clone(),
                });
            }
        }
    }
    issues
}

fn is_exact_nonempty_learnset_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

pub fn level_up_moves_for_species<'a>(
    learnsets: &'a SpeciesLearnsets,
    species_id: &str,
) -> Result<&'a [LearnsetEntry], LearnsetError> {
    validate_learnset_runtime_token(species_id).map_err(|_| LearnsetError::InvalidSpecies {
        species_id: species_id.to_string(),
    })?;
    learnsets
        .get(species_id)
        .map(Vec::as_slice)
        .ok_or_else(|| LearnsetError::MissingSpecies {
            species_id: species_id.to_string(),
        })
}

pub fn default_moves_for_level(
    learnsets: &SpeciesLearnsets,
    species_id: &str,
    level: u8,
    max_moves: usize,
) -> Result<Vec<String>, LearnsetError> {
    validate_learnset_runtime_token(species_id).map_err(|_| LearnsetError::InvalidSpecies {
        species_id: species_id.to_string(),
    })?;
    if level == 0 || max_moves == 0 {
        return Ok(Vec::new());
    }

    let mut slots: Vec<String> = Vec::new();
    for LearnsetEntry(learn_level, move_name) in level_up_moves_for_species(learnsets, species_id)?
    {
        if *learn_level > level {
            continue;
        }
        validate_learnset_runtime_token(move_name).map_err(|_| LearnsetError::InvalidMove {
            species_id: species_id.to_string(),
            move_id: move_name.clone(),
        })?;
        if let Some(index) = slots.iter().position(|known| known == move_name) {
            slots.remove(index);
        }
        slots.push(move_name.clone());
        if slots.len() > max_moves {
            slots.remove(0);
        }
    }
    Ok(slots)
}

fn validate_learnset_runtime_token(value: &str) -> Result<(), ()> {
    if is_exact_nonempty_learnset_token(value) {
        Ok(())
    } else {
        Err(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BaseStats, growth_rate, pokemon_type};

    fn species(id: &str) -> PokemonSpecies {
        let mut species = PokemonSpecies::new_for_tests(id, BaseStats::new(45, 49, 65, 45, 49, 65));
        species.growth_rate = growth_rate("GROWTH_MEDIUM_FAST");
        species.type1 = pokemon_type("GRASS");
        species.type2 = pokemon_type("GRASS");
        species
    }

    #[test]
    fn learnset_catalog_issues_validate_exact_modpack_ids_without_coercion() {
        let mut chikorita = species("CHIKORITA");
        chikorita.item1 = Some(" BERRY".to_string());
        chikorita.item2 = Some("MIRACLE SEED".to_string());
        chikorita.tmhm_learnset = vec![
            " HEADBUTT".to_string(),
            "HEAD BUTT".to_string(),
            "headbutt".to_string(),
            "CUT".to_string(),
        ];
        let mut bayleef = species("BAYLEEF");
        bayleef.tmhm_learnset.clear();
        let species = [
            ("CHIKORITA".to_string(), chikorita),
            ("BAYLEEF".to_string(), bayleef),
        ]
        .into_iter()
        .collect();
        let learnsets = [
            (
                "CHIKORITA".to_string(),
                vec![
                    LearnsetEntry(1, "TACKLE".to_string()),
                    LearnsetEntry(8, "razor_leaf".to_string()),
                ],
            ),
            (
                " BAYLEEF".to_string(),
                vec![
                    LearnsetEntry(1, "TACKLE ".to_string()),
                    LearnsetEntry(1, "RAZOR LEAF".to_string()),
                    LearnsetEntry(1, "tackle".to_string()),
                ],
            ),
            (
                "BAY LEEF".to_string(),
                vec![LearnsetEntry(1, "TACKLE".to_string())],
            ),
            (
                "bayleef".to_string(),
                vec![LearnsetEntry(1, "TACKLE".to_string())],
            ),
        ]
        .into_iter()
        .collect();
        let item_ids = ["MIRACLE_SEED".to_string()].into_iter().collect();
        let move_ids = ["TACKLE".to_string(), "CUT".to_string()]
            .into_iter()
            .collect();

        assert_eq!(
            learnset_catalog_issues(&species, &learnsets, &item_ids, &move_ids),
            vec![
                LearnsetCatalogIssue::MissingSpeciesLearnset {
                    species_id: "BAYLEEF".to_string(),
                },
                LearnsetCatalogIssue::InvalidSpeciesHeldItem {
                    species_id: "CHIKORITA".to_string(),
                    item_id: " BERRY".to_string(),
                },
                LearnsetCatalogIssue::InvalidSpeciesHeldItem {
                    species_id: "CHIKORITA".to_string(),
                    item_id: "MIRACLE SEED".to_string(),
                },
                LearnsetCatalogIssue::InvalidTmHmMove {
                    species_id: "CHIKORITA".to_string(),
                    move_id: " HEADBUTT".to_string(),
                },
                LearnsetCatalogIssue::InvalidTmHmMove {
                    species_id: "CHIKORITA".to_string(),
                    move_id: "HEAD BUTT".to_string(),
                },
                LearnsetCatalogIssue::UnknownTmHmMove {
                    species_id: "CHIKORITA".to_string(),
                    move_id: "headbutt".to_string(),
                },
                LearnsetCatalogIssue::InvalidLearnsetSpecies {
                    species_id: " BAYLEEF".to_string(),
                },
                LearnsetCatalogIssue::InvalidLevelMove {
                    species_id: " BAYLEEF".to_string(),
                    move_id: "TACKLE ".to_string(),
                },
                LearnsetCatalogIssue::InvalidLevelMove {
                    species_id: " BAYLEEF".to_string(),
                    move_id: "RAZOR LEAF".to_string(),
                },
                LearnsetCatalogIssue::UnknownLevelMove {
                    species_id: " BAYLEEF".to_string(),
                    move_id: "tackle".to_string(),
                },
                LearnsetCatalogIssue::InvalidLearnsetSpecies {
                    species_id: "BAY LEEF".to_string(),
                },
                LearnsetCatalogIssue::UnknownLevelMove {
                    species_id: "CHIKORITA".to_string(),
                    move_id: "razor_leaf".to_string(),
                },
                LearnsetCatalogIssue::UnknownLearnsetSpecies {
                    species_id: "bayleef".to_string(),
                },
            ]
        );
    }

    #[test]
    fn default_moves_follow_typescript_slot_replacement_behavior() {
        let learnsets: SpeciesLearnsets = [(
            "TESTMON".to_string(),
            vec![
                LearnsetEntry(1, "TACKLE".to_string()),
                LearnsetEntry(4, "GROWL".to_string()),
                LearnsetEntry(7, "LEECH_SEED".to_string()),
                LearnsetEntry(10, "VINE_WHIP".to_string()),
                LearnsetEntry(15, "POISONPOWDER".to_string()),
                LearnsetEntry(15, "SLEEP_POWDER".to_string()),
                LearnsetEntry(20, "TACKLE".to_string()),
            ],
        )]
        .into_iter()
        .collect();

        assert_eq!(
            default_moves_for_level(&learnsets, "TESTMON", 20, 4).expect("explicit learnset"),
            vec!["VINE_WHIP", "POISONPOWDER", "SLEEP_POWDER", "TACKLE"]
        );
        assert_eq!(
            default_moves_for_level(&learnsets, "testmon", 15, 4),
            Err(LearnsetError::MissingSpecies {
                species_id: "testmon".to_string(),
            })
        );
        assert_eq!(
            default_moves_for_level(&learnsets, "TEST MON", 15, 4),
            Err(LearnsetError::InvalidSpecies {
                species_id: "TEST MON".to_string(),
            })
        );
    }

    #[test]
    fn default_moves_reject_malformed_runtime_move_ids() {
        let learnsets: SpeciesLearnsets = [(
            "TESTMON".to_string(),
            vec![
                LearnsetEntry(1, "TACKLE".to_string()),
                LearnsetEntry(4, "VINE WHIP".to_string()),
            ],
        )]
        .into_iter()
        .collect();

        assert_eq!(
            default_moves_for_level(&learnsets, "TESTMON", 5, 4),
            Err(LearnsetError::InvalidMove {
                species_id: "TESTMON".to_string(),
                move_id: "VINE WHIP".to_string(),
            })
        );
    }

    #[test]
    fn level_up_moves_reject_malformed_species_before_missing_lookup() {
        let learnsets = SpeciesLearnsets::new();

        assert_eq!(
            level_up_moves_for_species(&learnsets, "TEST MON"),
            Err(LearnsetError::InvalidSpecies {
                species_id: "TEST MON".to_string(),
            })
        );
    }

    #[test]
    fn invalid_level_or_capacity_returns_no_moves() {
        let learnsets = SpeciesLearnsets::new();
        assert!(
            default_moves_for_level(&learnsets, "ANY", 0, 4)
                .expect("level zero does not require move data")
                .is_empty()
        );
        assert!(
            default_moves_for_level(&learnsets, "ANY", 5, 0)
                .expect("zero capacity does not require move data")
                .is_empty()
        );
        assert_eq!(
            default_moves_for_level(&learnsets, "AN Y", 0, 4),
            Err(LearnsetError::InvalidSpecies {
                species_id: "AN Y".to_string(),
            })
        );
    }
}
