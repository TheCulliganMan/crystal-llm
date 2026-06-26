use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::pokemon::{Pokemon, PokemonSpecies};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePokedexEntry {
    pub species: String,
    pub classification: String,
    pub height_digits: u16,
    pub weight_digits: u16,
    pub pages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PokedexEntryCatalogIssue {
    InvalidSpeciesId {
        species_id: String,
    },
    SpeciesMismatch {
        species_id: String,
        record_species: String,
    },
    UnknownSpecies {
        species_id: String,
    },
    InvalidEntry {
        species_id: String,
    },
    MissingSpeciesEntry {
        species_id: String,
    },
}

pub fn pokedex_entry_catalog_issues(
    entries: &BTreeMap<String, RuntimePokedexEntry>,
    species_ids: &BTreeSet<String>,
) -> Vec<PokedexEntryCatalogIssue> {
    let mut issues = Vec::new();

    for (species_id, entry) in entries {
        let invalid_species_id = !is_exact_nonempty_pokedex_token(species_id);
        let invalid_record_species = !is_exact_nonempty_pokedex_token(&entry.species);
        if invalid_species_id {
            issues.push(PokedexEntryCatalogIssue::InvalidSpeciesId {
                species_id: species_id.clone(),
            });
        }
        if species_id != &entry.species {
            issues.push(PokedexEntryCatalogIssue::SpeciesMismatch {
                species_id: species_id.clone(),
                record_species: entry.species.clone(),
            });
        }
        if !invalid_species_id && !species_ids.contains(species_id) {
            issues.push(PokedexEntryCatalogIssue::UnknownSpecies {
                species_id: species_id.clone(),
            });
        }
        if invalid_record_species
            || !is_exact_nonempty_pokedex_token(&entry.classification)
            || entry.pages.is_empty()
            || entry
                .pages
                .iter()
                .any(|page| !is_exact_nonempty_pokedex_token(page))
        {
            issues.push(PokedexEntryCatalogIssue::InvalidEntry {
                species_id: species_id.clone(),
            });
        }
    }

    for species_id in species_ids {
        if !entries.contains_key(species_id) {
            issues.push(PokedexEntryCatalogIssue::MissingSpeciesEntry {
                species_id: species_id.clone(),
            });
        }
    }

    issues
}

fn is_exact_nonempty_pokedex_token(value: &str) -> bool {
    !value.is_empty() && value.trim() == value
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PokedexState {
    pub seen_species: BTreeSet<String>,
    pub caught_species: BTreeSet<String>,
}

impl PokedexState {
    pub fn record_seen(&mut self, species: &PokemonSpecies) -> bool {
        self.seen_species.insert(species.id.clone())
    }

    pub fn record_caught(&mut self, species: &PokemonSpecies) -> bool {
        self.record_seen(species);
        self.caught_species.insert(species.id.clone())
    }

    pub fn record_seen_pokemon(&mut self, pokemon: &Pokemon) -> bool {
        self.record_seen(&pokemon.species)
    }

    pub fn record_caught_pokemon(&mut self, pokemon: &Pokemon) -> bool {
        self.record_caught(&pokemon.species)
    }

    pub fn has_seen(&self, species_id: &str) -> bool {
        self.seen_species.contains(species_id)
    }

    pub fn has_caught(&self, species_id: &str) -> bool {
        self.caught_species.contains(species_id)
    }

    pub fn seen_count(&self) -> usize {
        self.seen_species.len()
    }

    pub fn caught_count(&self) -> usize {
        self.caught_species.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BaseStats, pokemon_type};

    fn species(id: &str) -> PokemonSpecies {
        let mut species = PokemonSpecies::new_for_tests(id, BaseStats::new(45, 49, 49, 45, 65, 65));
        species.type1 = pokemon_type("NORMAL");
        species.type2 = pokemon_type("NORMAL");
        species
    }

    #[test]
    fn caught_species_are_also_seen() {
        let mut pokedex = PokedexState::default();
        let chikorita = species("CHIKORITA");

        assert!(pokedex.record_caught(&chikorita));
        assert!(pokedex.has_seen("CHIKORITA"));
        assert!(pokedex.has_caught("CHIKORITA"));
        assert_eq!(pokedex.seen_count(), 1);
        assert_eq!(pokedex.caught_count(), 1);
    }

    #[test]
    fn species_ids_are_exact_modpack_ids() {
        let mut pokedex = PokedexState::default();
        let modded = species("modpack_CHIKORITA");

        pokedex.record_seen(&modded);

        assert!(pokedex.has_seen("modpack_CHIKORITA"));
        assert!(!pokedex.has_seen("MODPACK_CHIKORITA"));
        assert!(!pokedex.has_seen("CHIKORITA"));
    }

    #[test]
    fn pokedex_entry_catalog_issues_require_exact_pack_records() {
        let entries = [
            (
                "CHIKORITA".to_string(),
                RuntimePokedexEntry {
                    species: "chikorita".to_string(),
                    classification: String::new(),
                    height_digits: 4,
                    weight_digits: 64,
                    pages: Vec::new(),
                },
            ),
            (
                "MISSINGNO".to_string(),
                RuntimePokedexEntry {
                    species: "MISSINGNO".to_string(),
                    classification: "GLITCH".to_string(),
                    height_digits: 10,
                    weight_digits: 100,
                    pages: vec!["Unknown data.".to_string()],
                },
            ),
            (
                " CYNDAQUIL".to_string(),
                RuntimePokedexEntry {
                    species: " CYNDAQUIL".to_string(),
                    classification: "Fire Mouse".to_string(),
                    height_digits: 5,
                    weight_digits: 79,
                    pages: vec![" A timid fire Pokemon.".to_string()],
                },
            ),
        ]
        .into_iter()
        .collect();
        let species_ids = ["BAYLEEF".to_string(), "CHIKORITA".to_string()]
            .into_iter()
            .collect();

        assert_eq!(
            pokedex_entry_catalog_issues(&entries, &species_ids),
            vec![
                PokedexEntryCatalogIssue::InvalidSpeciesId {
                    species_id: " CYNDAQUIL".to_string(),
                },
                PokedexEntryCatalogIssue::InvalidEntry {
                    species_id: " CYNDAQUIL".to_string(),
                },
                PokedexEntryCatalogIssue::SpeciesMismatch {
                    species_id: "CHIKORITA".to_string(),
                    record_species: "chikorita".to_string(),
                },
                PokedexEntryCatalogIssue::InvalidEntry {
                    species_id: "CHIKORITA".to_string(),
                },
                PokedexEntryCatalogIssue::UnknownSpecies {
                    species_id: "MISSINGNO".to_string(),
                },
                PokedexEntryCatalogIssue::MissingSpeciesEntry {
                    species_id: "BAYLEEF".to_string(),
                },
            ],
        );
    }
}
