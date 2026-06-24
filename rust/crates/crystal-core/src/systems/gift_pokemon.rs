use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::models::{
    CaptureStorageLocation, Dv, Item, Move, Pokemon, PokemonBuildError, PokemonSpecies,
    PokemonStorage, create_pokemon_from_known_dvs,
};
use crate::systems::learnsets::SpeciesLearnsets;

pub const NO_ITEM: &str = "NO_ITEM";
pub const EGG_NICKNAME: &str = "EGG";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GiftPokemonScript {
    pub species_id: String,
    pub level_token: String,
    pub level: u8,
    pub held_item_id: Option<String>,
    pub nickname_label: Option<String>,
    pub ot_label: Option<String>,
    pub source_script: String,
    pub command_index: usize,
    pub egg: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GiftPokemonRequest {
    pub species_id: String,
    pub level: u8,
    pub held_item_id: Option<String>,
    pub nickname: Option<String>,
    pub original_trainer_name: String,
    pub original_trainer_id: u16,
    pub source_script: String,
    pub command_index: usize,
    pub egg: bool,
    pub dvs: Dv,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GiftPokemonOutcome {
    pub species_id: String,
    pub level: u8,
    pub location: CaptureStorageLocation,
    pub pokemon: Pokemon,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GiftPokemonError {
    UnknownSpecies { species_id: String },
    UnknownHeldItem { item_id: String },
    InvalidLevel { level: u8 },
    StorageFull { species_id: String },
    PokemonBuild { error: PokemonBuildError },
}

pub fn give_gift_pokemon(
    storage: &mut PokemonStorage,
    species: &BTreeMap<String, PokemonSpecies>,
    learnsets: &SpeciesLearnsets,
    moves: &BTreeMap<String, Move>,
    items: &BTreeMap<String, Item>,
    request: GiftPokemonRequest,
) -> Result<GiftPokemonOutcome, GiftPokemonError> {
    if request.level == 0 {
        return Err(GiftPokemonError::InvalidLevel {
            level: request.level,
        });
    }
    let species_data =
        species
            .get(&request.species_id)
            .ok_or_else(|| GiftPokemonError::UnknownSpecies {
                species_id: request.species_id.clone(),
            })?;
    if let Some(item_id) = request.held_item_id.as_deref()
        && !items.contains_key(item_id)
    {
        return Err(GiftPokemonError::UnknownHeldItem {
            item_id: item_id.to_string(),
        });
    }

    let mut pokemon =
        create_pokemon_from_known_dvs(species_data, request.level, request.dvs, learnsets, moves)
            .map_err(|error| GiftPokemonError::PokemonBuild { error })?;
    pokemon.original_trainer_name = request.original_trainer_name.clone();
    pokemon.original_trainer_id = request.original_trainer_id;
    pokemon.item = request.held_item_id.clone();
    if let Some(nickname) = request.nickname.as_deref() {
        pokemon.nickname = nickname.to_string();
    }
    if request.egg {
        pokemon.nickname = EGG_NICKNAME.to_string();
        pokemon.happiness = species_data.step_cycles_to_hatch;
        pokemon.hp = 0;
    } else {
        pokemon.hp = pokemon.max_hp;
        pokemon.status = None;
        pokemon.sleep_turns = 0;
        pokemon.flinching = false;
        pokemon.confusion_turns = 0;
        pokemon.rampage_turns = 0;
    }

    let location =
        storage
            .register_capture(pokemon.clone())
            .map_err(|_| GiftPokemonError::StorageFull {
                species_id: request.species_id.clone(),
            })?;
    Ok(GiftPokemonOutcome {
        species_id: request.species_id,
        level: request.level,
        location,
        pokemon,
        source_script: request.source_script,
        command_index: request.command_index,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BaseStats, GrowthRate, ItemPocket, PokemonType};

    fn species(id: &str) -> PokemonSpecies {
        PokemonSpecies {
            id: id.to_string(),
            int_id: 1,
            base_stats: BaseStats::new(45, 49, 65, 45, 49, 65),
            type1: PokemonType::Grass,
            type2: PokemonType::Grass,
            catch_rate: 45,
            base_exp: 64,
            item1: None,
            item2: None,
            gender_ratio: 127,
            unknown1: 0,
            unknown2: 0,
            growth_rate: GrowthRate::MediumSlow,
            egg_group1: crate::models::EggGroup::Monster,
            egg_group2: crate::models::EggGroup::Monster,
            tmhm_learnset: Vec::new(),
            ability: crate::models::Ability::None,
            pic_size: 0,
            front_pic: 0,
            back_pic: 0,
            weight: 0,
            step_cycles_to_hatch: 20,
        }
    }

    fn item(id: &str) -> Item {
        Item {
            name: id.to_string(),
            description: String::new(),
            effect: "NONE".to_string(),
            price: 0,
            held_effect: "HELD_NONE".to_string(),
            parameter: 0,
            property: String::new(),
            pocket: ItemPocket::Item,
            field_menu: String::new(),
            battle_menu: String::new(),
            script_name: id.to_string(),
            consumable: false,
            tmhm_index: None,
        }
    }

    fn request(species_id: &str, level: u8) -> GiftPokemonRequest {
        GiftPokemonRequest {
            species_id: species_id.to_string(),
            level,
            held_item_id: None,
            nickname: None,
            original_trainer_name: "PLAYER".to_string(),
            original_trainer_id: 1234,
            source_script: "GiftScript".to_string(),
            command_index: 4,
            egg: false,
            dvs: Dv::from_non_hp(10, 10, 10, 10),
        }
    }

    fn learnsets(species_id: &str) -> SpeciesLearnsets {
        [(species_id.to_string(), Vec::new())].into_iter().collect()
    }

    #[test]
    fn gives_exact_species_to_party_with_exact_held_item() {
        let mut storage = PokemonStorage::default();
        let species_map = BTreeMap::from([("CYNDAQUIL".to_string(), species("CYNDAQUIL"))]);
        let items = BTreeMap::from([("BERRY".to_string(), item("BERRY"))]);
        let mut request = request("CYNDAQUIL", 5);
        request.held_item_id = Some("BERRY".to_string());

        let outcome = give_gift_pokemon(
            &mut storage,
            &species_map,
            &learnsets("CYNDAQUIL"),
            &BTreeMap::new(),
            &items,
            request,
        )
        .expect("gift pokemon");

        assert_eq!(outcome.location, CaptureStorageLocation::Party { slot: 0 });
        assert_eq!(outcome.pokemon.species.id, "CYNDAQUIL");
        assert_eq!(outcome.pokemon.item.as_deref(), Some("BERRY"));
        assert_eq!(storage.party.filled_slots(), 1);
    }

    #[test]
    fn rejects_case_changed_species_and_item_ids() {
        let mut storage = PokemonStorage::default();
        let species_map = BTreeMap::from([("CYNDAQUIL".to_string(), species("CYNDAQUIL"))]);
        let items = BTreeMap::from([("BERRY".to_string(), item("BERRY"))]);
        let bad_species = request("cyndaquil", 5);
        let mut bad_item = request("CYNDAQUIL", 5);
        bad_item.held_item_id = Some("berry".to_string());

        assert_eq!(
            give_gift_pokemon(
                &mut storage,
                &species_map,
                &learnsets("CYNDAQUIL"),
                &BTreeMap::new(),
                &items,
                bad_species,
            ),
            Err(GiftPokemonError::UnknownSpecies {
                species_id: "cyndaquil".to_string(),
            })
        );
        assert_eq!(
            give_gift_pokemon(
                &mut storage,
                &species_map,
                &learnsets("CYNDAQUIL"),
                &BTreeMap::new(),
                &items,
                bad_item,
            ),
            Err(GiftPokemonError::UnknownHeldItem {
                item_id: "berry".to_string(),
            })
        );
    }

    #[test]
    fn gives_egg_to_party_with_egg_metadata() {
        let mut storage = PokemonStorage::default();
        let species_map = BTreeMap::from([("TOGEPI".to_string(), species("TOGEPI"))]);
        let mut request = request("TOGEPI", 5);
        request.egg = true;

        let outcome = give_gift_pokemon(
            &mut storage,
            &species_map,
            &learnsets("TOGEPI"),
            &BTreeMap::new(),
            &BTreeMap::new(),
            request,
        )
        .expect("gift egg");

        assert_eq!(outcome.pokemon.nickname, EGG_NICKNAME);
        assert_eq!(outcome.pokemon.hp, 0);
        assert_eq!(outcome.pokemon.happiness, 20);
    }

    #[test]
    fn rejects_missing_learnset_moves_without_creating_zero_pp_gift() {
        let mut storage = PokemonStorage::default();
        let species_map = BTreeMap::from([("CYNDAQUIL".to_string(), species("CYNDAQUIL"))]);
        let learnsets = [(
            "CYNDAQUIL".to_string(),
            vec![crate::systems::learnsets::LearnsetEntry(
                1,
                "TACKLE".to_string(),
            )],
        )]
        .into_iter()
        .collect();

        assert_eq!(
            give_gift_pokemon(
                &mut storage,
                &species_map,
                &learnsets,
                &BTreeMap::new(),
                &BTreeMap::new(),
                request("CYNDAQUIL", 5),
            ),
            Err(GiftPokemonError::PokemonBuild {
                error: PokemonBuildError::UnknownLearnsetMove {
                    species_id: "CYNDAQUIL".to_string(),
                    move_name: "TACKLE".to_string(),
                },
            })
        );
        assert_eq!(storage.party.filled_slots(), 0);
    }
}
