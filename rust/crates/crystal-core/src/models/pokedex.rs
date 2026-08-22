use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use super::pokemon::{Pokemon, PokemonSpecies};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePokedexEntry {
    #[serde(deserialize_with = "required_pokedex_id")]
    pub species: String,
    #[serde(deserialize_with = "required_pokedex_text")]
    pub classification: String,
    #[serde(deserialize_with = "required_pokedex_dimension")]
    pub height_digits: u16,
    #[serde(deserialize_with = "required_pokedex_dimension")]
    pub weight_digits: u16,
    #[serde(deserialize_with = "required_pokedex_pages")]
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
        let invalid_species_id = !is_exact_nonempty_pokedex_id(species_id);
        let invalid_record_species = !is_exact_nonempty_pokedex_id(&entry.species);
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
            || !is_exact_nonempty_pokedex_text(&entry.classification)
            || entry.height_digits == 0
            || entry.weight_digits == 0
            || entry.pages.is_empty()
            || entry
                .pages
                .iter()
                .any(|page| !is_exact_nonempty_pokedex_text(page))
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

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimePokedexEntryTable(pub BTreeMap<String, RuntimePokedexEntry>);

impl<'de> Deserialize<'de> for RuntimePokedexEntryTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let entries = BTreeMap::<String, RuntimePokedexEntry>::deserialize(deserializer)?;
        if entries.is_empty() {
            return Err(D::Error::custom("pokedex entry table must not be empty"));
        }
        for (species_id, entry) in &entries {
            if !is_exact_nonempty_pokedex_id(species_id) {
                return Err(D::Error::custom(format!(
                    "pokedex entry key must be exact species id, found {species_id:?}"
                )));
            }
            if entry.species != *species_id {
                return Err(D::Error::custom(format!(
                    "pokedex entry key {species_id:?} must match record species {:?}",
                    entry.species
                )));
            }
        }
        Ok(Self(entries))
    }
}

fn is_exact_nonempty_pokedex_id(value: &str) -> bool {
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

fn is_exact_nonempty_pokedex_text(value: &str) -> bool {
    !value.is_empty() && value.trim() == value
}

fn required_pokedex_id<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_nonempty_pokedex_id(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "pokedex species id must be exact ASCII alphanumeric/underscore, found {value:?}"
        )))
    }
}

fn required_pokedex_text<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_nonempty_pokedex_text(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "pokedex text must be exact non-empty text, found {value:?}"
        )))
    }
}

fn required_pokedex_dimension<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = u16::deserialize(deserializer)?;
    if value > 0 {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(
            "pokedex dimensions must be positive",
        ))
    }
}

fn required_pokedex_pages<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values = Vec::<String>::deserialize(deserializer)?;
    if values.is_empty() {
        return Err(serde::de::Error::custom(
            "pokedex pages must contain at least one page",
        ));
    }
    for value in &values {
        if !is_exact_nonempty_pokedex_text(value) {
            return Err(serde::de::Error::custom(format!(
                "pokedex page must be exact non-empty text, found {value:?}"
            )));
        }
    }
    Ok(values)
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PokedexState {
    pub seen_species: BTreeSet<String>,
    pub caught_species: BTreeSet<String>,
    /// One-based Unown letter indices in the same first-caught order as
    /// Crystal's `wUnownDex` array.
    pub unown_letters: Vec<u8>,
}

impl<'de> Deserialize<'de> for PokedexState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawPokedexState {
            seen_species: BTreeSet<String>,
            caught_species: BTreeSet<String>,
            unown_letters: Vec<u8>,
        }

        let raw = RawPokedexState::deserialize(deserializer)?;
        let state = Self {
            seen_species: raw.seen_species,
            caught_species: raw.caught_species,
            unown_letters: raw.unown_letters,
        };
        state.validate_shape().map_err(D::Error::custom)?;
        Ok(state)
    }
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
        let newly_caught_species = self.record_caught(&pokemon.species);
        if pokemon.species.id == "UNOWN" {
            let letter = pokemon.dvs.unown_letter();
            if !self.unown_letters.contains(&letter) {
                self.unown_letters.push(letter);
            }
        }
        newly_caught_species
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

    pub fn unown_count(&self) -> usize {
        self.unown_letters.len()
    }

    fn validate_shape(&self) -> Result<(), String> {
        for species in &self.seen_species {
            if !is_exact_nonempty_pokedex_id(species) {
                return Err(format!("pokedex seen species {species:?} is not exact"));
            }
        }
        for species in &self.caught_species {
            if !is_exact_nonempty_pokedex_id(species) {
                return Err(format!("pokedex caught species {species:?} is not exact"));
            }
            if !self.seen_species.contains(species) {
                return Err(format!(
                    "pokedex caught species {species} is not present in seen species"
                ));
            }
        }
        let mut unique_letters = BTreeSet::new();
        for &letter in &self.unown_letters {
            if !(1..=26).contains(&letter) {
                return Err(format!(
                    "pokedex Unown letter index {letter} is outside 1..=26"
                ));
            }
            if !unique_letters.insert(letter) {
                return Err(format!("pokedex Unown letter index {letter} is duplicated"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum PokedexSaveError {
    #[error("saved {path} {species} is missing from compiled pack pokemon")]
    MissingSpecies { path: &'static str, species: String },
    #[error("saved {path} {species} does not match compiled species id {compiled_species}")]
    SpeciesMismatch {
        path: &'static str,
        species: String,
        compiled_species: String,
    },
    #[error("saved pokedex.caught_species {species} is not present in saved pokedex.seen_species")]
    CaughtSpeciesNotSeen { species: String },
    #[error("saved pokedex.unown_letters contains invalid one-based letter index {letter}")]
    InvalidUnownLetter { letter: u8 },
    #[error("saved pokedex.unown_letters repeats one-based letter index {letter}")]
    DuplicateUnownLetter { letter: u8 },
    #[error("saved pokedex.unown_letters is nonempty but UNOWN is not caught")]
    UnownLettersWithoutCaughtSpecies,
}

pub fn validate_saved_pokedex_references<F>(
    pokedex: &PokedexState,
    compiled_species_id: F,
) -> Result<(), PokedexSaveError>
where
    F: Fn(&str) -> Option<String>,
{
    for species in &pokedex.seen_species {
        validate_saved_pokedex_species_reference(
            "pokedex.seen_species",
            species,
            &compiled_species_id,
        )?;
    }
    for species in &pokedex.caught_species {
        validate_saved_pokedex_species_reference(
            "pokedex.caught_species",
            species,
            &compiled_species_id,
        )?;
        if !pokedex.seen_species.contains(species) {
            return Err(PokedexSaveError::CaughtSpeciesNotSeen {
                species: species.clone(),
            });
        }
    }
    let mut unique_letters = BTreeSet::new();
    for &letter in &pokedex.unown_letters {
        if !(1..=26).contains(&letter) {
            return Err(PokedexSaveError::InvalidUnownLetter { letter });
        }
        if !unique_letters.insert(letter) {
            return Err(PokedexSaveError::DuplicateUnownLetter { letter });
        }
    }
    if !pokedex.unown_letters.is_empty() && !pokedex.caught_species.contains("UNOWN") {
        return Err(PokedexSaveError::UnownLettersWithoutCaughtSpecies);
    }
    Ok(())
}

fn validate_saved_pokedex_species_reference<F>(
    path: &'static str,
    species: &str,
    compiled_species_id: &F,
) -> Result<(), PokedexSaveError>
where
    F: Fn(&str) -> Option<String>,
{
    let compiled_species =
        compiled_species_id(species).ok_or_else(|| PokedexSaveError::MissingSpecies {
            path,
            species: species.to_string(),
        })?;
    if compiled_species != species {
        return Err(PokedexSaveError::SpeciesMismatch {
            path,
            species: species.to_string(),
            compiled_species,
        });
    }
    Ok(())
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
    fn saved_unown_letters_are_unique_one_based_forms_of_caught_unown() {
        let mut pokedex = PokedexState::default();
        let unown = species("UNOWN");
        pokedex.record_caught(&unown);
        pokedex.unown_letters = vec![1, 26];

        validate_saved_pokedex_references(&pokedex, |species| Some(species.to_string()))
            .expect("valid saved Unown letters");

        pokedex.unown_letters.push(1);
        assert_eq!(
            validate_saved_pokedex_references(&pokedex, |species| Some(species.to_string())),
            Err(PokedexSaveError::DuplicateUnownLetter { letter: 1 })
        );
        pokedex.unown_letters = vec![27];
        assert_eq!(
            validate_saved_pokedex_references(&pokedex, |species| Some(species.to_string())),
            Err(PokedexSaveError::InvalidUnownLetter { letter: 27 })
        );
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
    fn validate_saved_pokedex_references_rejects_missing_species() {
        let mut pokedex = PokedexState::default();
        pokedex.seen_species.insert("CHIKORITA".to_string());

        let error = validate_saved_pokedex_references(&pokedex, |_| None)
            .expect_err("saved seen species must exist in compiled Pokemon table");

        assert_eq!(
            error,
            PokedexSaveError::MissingSpecies {
                path: "pokedex.seen_species",
                species: "CHIKORITA".to_string(),
            }
        );
    }

    #[test]
    fn validate_saved_pokedex_references_rejects_mismatched_species_payload() {
        let mut pokedex = PokedexState::default();
        pokedex.seen_species.insert("CHIKORITA".to_string());

        let error = validate_saved_pokedex_references(&pokedex, |_| Some("CYNDAQUIL".to_string()))
            .expect_err("saved species key must match compiled Pokemon payload id");

        assert_eq!(
            error,
            PokedexSaveError::SpeciesMismatch {
                path: "pokedex.seen_species",
                species: "CHIKORITA".to_string(),
                compiled_species: "CYNDAQUIL".to_string(),
            }
        );
    }

    #[test]
    fn validate_saved_pokedex_references_rejects_caught_without_seen() {
        let mut pokedex = PokedexState::default();
        pokedex.caught_species.insert("CHIKORITA".to_string());

        let error =
            validate_saved_pokedex_references(&pokedex, |species| Some(species.to_string()))
                .expect_err("caught species must also be saved as seen");

        assert_eq!(
            error,
            PokedexSaveError::CaughtSpeciesNotSeen {
                species: "CHIKORITA".to_string(),
            }
        );
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
            (
                "TOTODILE ALT".to_string(),
                RuntimePokedexEntry {
                    species: "TOTODILE ALT".to_string(),
                    classification: "Big Jaw".to_string(),
                    height_digits: 6,
                    weight_digits: 95,
                    pages: vec!["Its well-developed jaws are powerful.".to_string()],
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
                PokedexEntryCatalogIssue::InvalidSpeciesId {
                    species_id: "TOTODILE ALT".to_string(),
                },
                PokedexEntryCatalogIssue::InvalidEntry {
                    species_id: "TOTODILE ALT".to_string(),
                },
                PokedexEntryCatalogIssue::MissingSpeciesEntry {
                    species_id: "BAYLEEF".to_string(),
                },
            ],
        );
    }

    #[test]
    fn pokedex_entry_catalog_issues_reject_reserved_pack_prefix_ids() {
        let entries = [(
            "fallback_chikorita".to_string(),
            RuntimePokedexEntry {
                species: "legacy_chikorita".to_string(),
                classification: "Leaf".to_string(),
                height_digits: 4,
                weight_digits: 64,
                pages: vec!["A sweet aroma wafts from its leaf.".to_string()],
            },
        )]
        .into_iter()
        .collect();

        assert_eq!(
            pokedex_entry_catalog_issues(&entries, &BTreeSet::new()),
            vec![
                PokedexEntryCatalogIssue::InvalidSpeciesId {
                    species_id: "fallback_chikorita".to_string(),
                },
                PokedexEntryCatalogIssue::SpeciesMismatch {
                    species_id: "fallback_chikorita".to_string(),
                    record_species: "legacy_chikorita".to_string(),
                },
                PokedexEntryCatalogIssue::InvalidEntry {
                    species_id: "fallback_chikorita".to_string(),
                },
            ]
        );
    }

    #[test]
    fn pokedex_entry_catalog_issues_rejects_zero_dimensions() {
        let entries = [(
            "CHIKORITA".to_string(),
            RuntimePokedexEntry {
                species: "CHIKORITA".to_string(),
                classification: "Leaf".to_string(),
                height_digits: 0,
                weight_digits: 0,
                pages: vec!["A sweet aroma wafts from its leaf.".to_string()],
            },
        )]
        .into_iter()
        .collect();
        let species_ids = ["CHIKORITA".to_string()].into_iter().collect();

        assert_eq!(
            pokedex_entry_catalog_issues(&entries, &species_ids),
            vec![PokedexEntryCatalogIssue::InvalidEntry {
                species_id: "CHIKORITA".to_string(),
            }],
        );
    }

    #[test]
    fn pokedex_entry_json_rejects_malformed_pack_fields_at_deserialization() {
        let cases = [
            (
                "species",
                serde_json::json!({
                    "species": "CHIKO RITA",
                    "classification": "Leaf",
                    "heightDigits": 211,
                    "weightDigits": 141,
                    "pages": ["A sweet aroma gently wafts from its leaf."]
                }),
            ),
            (
                "classification",
                serde_json::json!({
                    "species": "CHIKORITA",
                    "classification": " Leaf",
                    "heightDigits": 211,
                    "weightDigits": 141,
                    "pages": ["A sweet aroma gently wafts from its leaf."]
                }),
            ),
            (
                "empty pages",
                serde_json::json!({
                    "species": "CHIKORITA",
                    "classification": "Leaf",
                    "heightDigits": 211,
                    "weightDigits": 141,
                    "pages": []
                }),
            ),
            (
                "page",
                serde_json::json!({
                    "species": "CHIKORITA",
                    "classification": "Leaf",
                    "heightDigits": 211,
                    "weightDigits": 141,
                    "pages": ["A sweet aroma gently wafts from its leaf. "]
                }),
            ),
            (
                "height",
                serde_json::json!({
                    "species": "CHIKORITA",
                    "classification": "Leaf",
                    "heightDigits": 0,
                    "weightDigits": 141,
                    "pages": ["A sweet aroma gently wafts from its leaf."]
                }),
            ),
            (
                "weight",
                serde_json::json!({
                    "species": "CHIKORITA",
                    "classification": "Leaf",
                    "heightDigits": 211,
                    "weightDigits": 0,
                    "pages": ["A sweet aroma gently wafts from its leaf."]
                }),
            ),
        ];

        for (label, payload) in cases {
            let error = serde_json::from_value::<RuntimePokedexEntry>(payload)
                .expect_err("malformed pokedex entry fields must fail during JSON load")
                .to_string();
            assert!(
                error.contains("pokedex"),
                "{label} produced unexpected error: {error}"
            );
        }
    }
}
