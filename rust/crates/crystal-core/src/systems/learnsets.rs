use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LearnsetEntry(pub u8, pub String);

pub type SpeciesLearnsets = BTreeMap<String, Vec<LearnsetEntry>>;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LearnsetError {
    #[error("missing level-up learnset for species '{species_id}'")]
    MissingSpecies { species_id: String },
}

pub fn level_up_moves_for_species<'a>(
    learnsets: &'a SpeciesLearnsets,
    species_id: &str,
) -> Result<&'a [LearnsetEntry], LearnsetError> {
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
    if level == 0 || max_moves == 0 {
        return Ok(Vec::new());
    }

    let mut slots: Vec<String> = Vec::new();
    for LearnsetEntry(learn_level, move_name) in level_up_moves_for_species(learnsets, species_id)?
    {
        if *learn_level > level {
            continue;
        }
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

#[cfg(test)]
mod tests {
    use super::*;

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
    }
}
