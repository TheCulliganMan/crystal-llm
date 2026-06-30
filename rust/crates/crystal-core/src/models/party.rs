use serde::{Deserialize, Deserializer, Serialize};

use super::pokemon::Pokemon;

pub const PARTY_SIZE: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Party {
    pub pokemon: [Option<Pokemon>; PARTY_SIZE],
}

impl<'de> Deserialize<'de> for Party {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawParty {
            pokemon: [Option<Pokemon>; PARTY_SIZE],
        }

        let raw = RawParty::deserialize(deserializer)?;
        let party = Self {
            pokemon: raw.pokemon,
        };
        party
            .validate_saved_state()
            .map_err(serde::de::Error::custom)?;
        Ok(party)
    }
}

impl Default for Party {
    fn default() -> Self {
        Self {
            pokemon: [const { None }; PARTY_SIZE],
        }
    }
}

impl Party {
    pub fn filled_slots(&self) -> usize {
        self.pokemon.iter().filter(|slot| slot.is_some()).count()
    }

    pub fn has_space(&self) -> bool {
        self.filled_slots() < PARTY_SIZE
    }

    pub fn next_open_slot(&self) -> Option<usize> {
        self.pokemon.iter().position(Option::is_none)
    }

    pub fn add_pokemon(&mut self, pokemon: Pokemon) -> bool {
        let Some(slot) = self.next_open_slot() else {
            return false;
        };
        self.pokemon[slot] = Some(pokemon);
        true
    }

    pub fn validate_saved_state(&self) -> Result<(), String> {
        let mut first_empty_slot = None;
        for (index, pokemon) in self.pokemon.iter().enumerate() {
            match pokemon {
                Some(pokemon) => {
                    if let Some(empty_index) = first_empty_slot {
                        return Err(format!(
                            "party slot {index} is filled after empty slot {empty_index}"
                        ));
                    }
                    pokemon
                        .validate_saved_state()
                        .map_err(|error| format!("party slot {index}: {error}"))?;
                }
                None => {
                    first_empty_slot.get_or_insert(index);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::pokemon::{BaseStats, Dv, PokemonSpecies, growth_rate};

    fn pokemon(id: &str) -> Pokemon {
        Pokemon::new_for_tests(
            PokemonSpecies::new_for_tests(id, BaseStats::new(45, 49, 49, 45, 65, 65)),
            5,
            Dv::default(),
        )
    }

    #[test]
    fn party_slot_helpers_match_typescript_behavior() {
        let mut party = Party::default();
        assert_eq!(party.filled_slots(), 0);
        assert!(party.has_space());
        assert_eq!(party.next_open_slot(), Some(0));

        assert!(party.add_pokemon(pokemon("CHIKORITA")));
        assert_eq!(party.filled_slots(), 1);
        assert_eq!(party.next_open_slot(), Some(1));

        for i in 0..5 {
            assert!(party.add_pokemon(pokemon(&format!("MON_{i}"))));
        }
        assert_eq!(party.filled_slots(), PARTY_SIZE);
        assert!(!party.has_space());
        assert_eq!(party.next_open_slot(), None);
        assert!(!party.add_pokemon(pokemon("EXTRA")));
    }

    #[test]
    fn default_party_has_exact_game_party_capacity() {
        assert_eq!(Party::default().pokemon.len(), PARTY_SIZE);
    }

    #[test]
    fn saved_party_rejects_filled_slots_after_empty_slots() {
        let mut party = Party::default();
        party.pokemon[1] = Some(pokemon("CHIKORITA"));

        assert_eq!(
            party.validate_saved_state(),
            Err("party slot 1 is filled after empty slot 0".to_string())
        );
    }

    #[test]
    fn test_fixture_uses_growth_rate_default() {
        let mon = pokemon("CYNDAQUIL");
        assert_eq!(mon.species.growth_rate, growth_rate("GROWTH_MEDIUM_SLOW"));
    }
}
