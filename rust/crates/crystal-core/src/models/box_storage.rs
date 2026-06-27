use serde::{Deserialize, Serialize};

use super::party::Party;
use super::pokemon::Pokemon;

pub const MAX_PC_BOXES: usize = 14;
pub const MAX_BOX_MONS: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PcBox {
    pub name: String,
    pub pokemon: [Option<Pokemon>; MAX_BOX_MONS],
    pub nicknames: [String; MAX_BOX_MONS],
    pub original_trainer_names: [String; MAX_BOX_MONS],
    pub original_trainer_ids: [u16; MAX_BOX_MONS],
    pub count: usize,
    pub slot_species: [u16; MAX_BOX_MONS + 1],
}

impl PcBox {
    pub fn new(index: usize) -> Self {
        Self {
            name: format_default_box_name(index),
            pokemon: [const { None }; MAX_BOX_MONS],
            nicknames: std::array::from_fn(|_| String::new()),
            original_trainer_names: std::array::from_fn(|_| String::new()),
            original_trainer_ids: [0; MAX_BOX_MONS],
            count: 0,
            slot_species: [0; MAX_BOX_MONS + 1],
        }
    }

    pub fn filled_slots(&self) -> usize {
        self.pokemon.iter().filter(|slot| slot.is_some()).count()
    }

    pub fn has_space(&self) -> bool {
        self.filled_slots() < MAX_BOX_MONS
    }

    pub fn next_open_slot(&self) -> Option<usize> {
        self.pokemon.iter().position(Option::is_none)
    }

    pub fn add_pokemon(&mut self, pokemon: Pokemon) -> bool {
        let Some(slot) = self.next_open_slot() else {
            return false;
        };
        self.set_slot(slot, Some(pokemon));
        true
    }

    pub fn set_slot(&mut self, index: usize, pokemon: Option<Pokemon>) {
        assert!(index < MAX_BOX_MONS, "box slot {index} is out of range");
        let previous_filled = self.pokemon[index].is_some();
        let next_filled = pokemon.is_some();
        self.write_slot_metadata(index, pokemon.as_ref());
        self.pokemon[index] = pokemon;
        match (previous_filled, next_filled) {
            (false, true) => self.count += 1,
            (true, false) => self.count = self.count.saturating_sub(1),
            _ => {}
        }
        self.count = self.count.min(MAX_BOX_MONS);
    }

    fn write_slot_metadata(&mut self, index: usize, pokemon: Option<&Pokemon>) {
        if let Some(pokemon) = pokemon {
            self.nicknames[index] = pokemon.nickname.clone();
            self.original_trainer_names[index] = pokemon.original_trainer_name.clone();
            self.original_trainer_ids[index] = pokemon.original_trainer_id;
            self.slot_species[index] = pokemon.species.int_id;
        } else {
            self.nicknames[index].clear();
            self.original_trainer_names[index].clear();
            self.original_trainer_ids[index] = 0;
            self.slot_species[index] = 0;
        }
    }

    pub fn validate_metadata(&self) -> Result<(), String> {
        validate_pc_box_name(&self.name)?;
        let filled = self.filled_slots();
        if self.count != filled {
            return Err(format!(
                "box count {} must match filled pokemon slots {}",
                self.count, filled
            ));
        }
        if self.slot_species[MAX_BOX_MONS] != 0 {
            return Err(format!(
                "slot_species terminator {} must be 0",
                self.slot_species[MAX_BOX_MONS]
            ));
        }
        for index in 0..MAX_BOX_MONS {
            match &self.pokemon[index] {
                Some(pokemon) => {
                    pokemon
                        .validate_saved_state()
                        .map_err(|error| format!("slot {index}: {error}"))?;
                    if self.nicknames[index] != pokemon.nickname {
                        return Err(format!("slot {index} nickname metadata mismatch"));
                    }
                    if self.original_trainer_names[index] != pokemon.original_trainer_name {
                        return Err(format!(
                            "slot {index} original trainer name metadata mismatch"
                        ));
                    }
                    if self.original_trainer_ids[index] != pokemon.original_trainer_id {
                        return Err(format!(
                            "slot {index} original trainer id metadata mismatch"
                        ));
                    }
                    if self.slot_species[index] != pokemon.species.int_id {
                        return Err(format!("slot {index} species metadata mismatch"));
                    }
                }
                None => {
                    if !self.nicknames[index].is_empty()
                        || !self.original_trainer_names[index].is_empty()
                        || self.original_trainer_ids[index] != 0
                        || self.slot_species[index] != 0
                    {
                        return Err(format!("empty slot {index} must have empty metadata"));
                    }
                }
            }
        }
        Ok(())
    }
}

fn validate_pc_box_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.trim() != name || name.chars().any(char::is_control) {
        return Err(format!("box name has invalid text '{name}'"));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PokemonStorage {
    pub party: Party,
    pub pc_boxes: Vec<PcBox>,
}

impl Default for PokemonStorage {
    fn default() -> Self {
        Self {
            party: Party::default(),
            pc_boxes: Vec::new(),
        }
    }
}

impl PokemonStorage {
    pub fn validate_metadata(&self) -> Result<(), String> {
        self.party.validate_saved_state()?;
        if self.pc_boxes.len() > MAX_PC_BOXES {
            return Err(format!(
                "PC storage has {} boxes, maximum is {MAX_PC_BOXES}",
                self.pc_boxes.len()
            ));
        }
        for (index, pc_box) in self.pc_boxes.iter().enumerate() {
            pc_box
                .validate_metadata()
                .map_err(|error| format!("pc_boxes[{index}] {error}"))?;
        }
        Ok(())
    }

    pub fn has_capture_space(&self) -> bool {
        self.party.has_space()
            || self.pc_boxes.len() < MAX_PC_BOXES
            || self.pc_boxes.iter().any(PcBox::has_space)
    }

    pub fn register_capture(&mut self, pokemon: Pokemon) -> Result<CaptureStorageLocation, String> {
        if self.party.add_pokemon(pokemon.clone()) {
            let slot = self.party.filled_slots() - 1;
            return Ok(CaptureStorageLocation::Party { slot });
        }

        for box_index in 0..self.pc_boxes.len() {
            self.ensure_canonical_box(box_index);
            if self.pc_boxes[box_index].add_pokemon(pokemon.clone()) {
                let slot = self.pc_boxes[box_index].filled_slots() - 1;
                return Ok(CaptureStorageLocation::Pc { box_index, slot });
            }
        }

        if self.pc_boxes.len() < MAX_PC_BOXES {
            let box_index = self.pc_boxes.len();
            self.ensure_canonical_box(box_index);
            if self.pc_boxes[box_index].add_pokemon(pokemon) {
                return Ok(CaptureStorageLocation::Pc { box_index, slot: 0 });
            }
        }

        Err("party and PC boxes are full; cannot store captured Pokemon".to_string())
    }

    fn ensure_canonical_box(&mut self, index: usize) {
        while self.pc_boxes.len() <= index {
            let next = self.pc_boxes.len();
            self.pc_boxes.push(PcBox::new(next));
        }
        if self.pc_boxes[index].name.trim().is_empty() {
            self.pc_boxes[index].name = format_default_box_name(index);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptureStorageLocation {
    Party { slot: usize },
    Pc { box_index: usize, slot: usize },
}

pub fn format_default_box_name(index: usize) -> String {
    format!("BOX {:02}", index + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::party::PARTY_SIZE;
    use crate::models::pokemon::{BaseStats, Dv, PokemonSpecies};

    fn pokemon(id: &str, int_id: u16) -> Pokemon {
        let mut species = PokemonSpecies::new_for_tests(id, BaseStats::new(45, 49, 49, 45, 65, 65));
        species.int_id = int_id;
        Pokemon::new_for_tests(species, 5, Dv::default())
    }

    #[test]
    fn pc_box_updates_metadata_with_slots() {
        let mut pc_box = PcBox::new(0);
        let mon = pokemon("CHIKORITA", 152);

        assert!(pc_box.add_pokemon(mon.clone()));

        assert_eq!(pc_box.count, 1);
        assert_eq!(pc_box.nicknames[0], mon.nickname);
        assert_eq!(pc_box.original_trainer_names[0], mon.original_trainer_name);
        assert_eq!(pc_box.original_trainer_ids[0], mon.original_trainer_id);
        assert_eq!(pc_box.slot_species[0], 152);
        assert_eq!(pc_box.slot_species[MAX_BOX_MONS], 0);
        pc_box.validate_metadata().expect("valid box metadata");
    }

    #[test]
    fn storage_metadata_validates_all_saved_pc_boxes() {
        let mut storage = PokemonStorage::default();
        let mut pc_box = PcBox::new(0);
        pc_box.count = 1;
        storage.pc_boxes.push(pc_box);

        assert_eq!(
            storage.validate_metadata(),
            Err("pc_boxes[0] box count 1 must match filled pokemon slots 0".to_string())
        );

        let mut storage = PokemonStorage::default();
        let mut pc_box = PcBox::new(0);
        pc_box.slot_species[MAX_BOX_MONS] = 0xff;
        storage.pc_boxes.push(pc_box);
        assert_eq!(
            storage.validate_metadata(),
            Err("pc_boxes[0] slot_species terminator 255 must be 0".to_string())
        );

        let mut storage = PokemonStorage::default();
        let mut pc_box = PcBox::new(0);
        pc_box.name = " BOX 01".to_string();
        storage.pc_boxes.push(pc_box);
        assert_eq!(
            storage.validate_metadata(),
            Err("pc_boxes[0] box name has invalid text ' BOX 01'".to_string())
        );
    }

    #[test]
    fn capture_storage_uses_party_before_pc_boxes() {
        let mut storage = PokemonStorage::default();
        let first = storage
            .register_capture(pokemon("CHIKORITA", 152))
            .expect("store in party");
        assert_eq!(first, CaptureStorageLocation::Party { slot: 0 });

        for index in 0..(PARTY_SIZE - 1) {
            storage
                .register_capture(pokemon(&format!("PARTY_{index}"), index as u16))
                .expect("fill party");
        }

        let pc = storage
            .register_capture(pokemon("TOTODILE", 158))
            .expect("store in pc");
        assert_eq!(
            pc,
            CaptureStorageLocation::Pc {
                box_index: 0,
                slot: 0
            }
        );
        assert_eq!(storage.pc_boxes[0].name, "BOX 01");
    }

    #[test]
    fn capture_storage_reports_full_when_party_and_all_boxes_are_full() {
        let mut storage = PokemonStorage::default();
        for index in 0..PARTY_SIZE {
            assert!(
                storage
                    .party
                    .add_pokemon(pokemon(&format!("PARTY_{index}"), index as u16))
            );
        }
        for box_index in 0..MAX_PC_BOXES {
            let mut pc_box = PcBox::new(box_index);
            for slot in 0..MAX_BOX_MONS {
                assert!(pc_box.add_pokemon(pokemon(
                    &format!("BOX_{box_index}_{slot}"),
                    (slot + 1) as u16,
                )));
            }
            storage.pc_boxes.push(pc_box);
        }

        assert!(!storage.has_capture_space());
        assert!(storage.register_capture(pokemon("EXTRA", 999)).is_err());
    }
}
