use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::pokemon::{Pokemon, PokemonSpecies};

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
}
