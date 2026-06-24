use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::move_data::Move;
use crate::systems::experience::calculate_experience;
use crate::systems::learnsets::{SpeciesLearnsets, default_moves_for_level};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Stat {
    Hp,
    Attack,
    Defense,
    Speed,
    SpecialAttack,
    SpecialDefense,
    Accuracy,
    Evasion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PokemonType {
    Normal,
    Fighting,
    Flying,
    Poison,
    Ground,
    Rock,
    Bug,
    Ghost,
    Steel,
    Fire,
    Water,
    Grass,
    Electric,
    PsychicType,
    Ice,
    Dragon,
    Dark,
    CurseType,
    None,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GrowthRate {
    #[serde(rename = "GROWTH_MEDIUM_FAST")]
    MediumFast,
    #[serde(rename = "GROWTH_SLIGHTLY_FAST")]
    SlightlyFast,
    #[serde(rename = "GROWTH_SLIGHTLY_SLOW")]
    SlightlySlow,
    #[serde(rename = "GROWTH_MEDIUM_SLOW")]
    MediumSlow,
    #[serde(rename = "GROWTH_FAST")]
    Fast,
    #[serde(rename = "GROWTH_SLOW")]
    Slow,
    #[serde(rename = "GROWTH_ERRATIC")]
    Erratic,
    #[serde(rename = "GROWTH_FLUCTUATING")]
    Fluctuating,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EggGroup {
    #[serde(rename = "EGG_NONE")]
    None,
    #[serde(rename = "EGG_MONSTER")]
    Monster,
    #[serde(rename = "EGG_WATER_1")]
    Water1,
    #[serde(rename = "EGG_BUG")]
    Bug,
    #[serde(rename = "EGG_FLYING")]
    Flying,
    #[serde(rename = "EGG_GROUND")]
    Ground,
    #[serde(rename = "EGG_FAIRY")]
    Fairy,
    #[serde(rename = "EGG_PLANT")]
    Plant,
    #[serde(rename = "EGG_HUMANSHAPE")]
    Humanshape,
    #[serde(rename = "EGG_WATER_3")]
    Water3,
    #[serde(rename = "EGG_MINERAL")]
    Mineral,
    #[serde(rename = "EGG_INDETERMINATE")]
    Indeterminate,
    #[serde(rename = "EGG_WATER_2")]
    Water2,
    #[serde(rename = "EGG_DITTO")]
    Ditto,
    #[serde(rename = "EGG_DRAGON")]
    Dragon,
    #[serde(rename = "EGG_UNDISCOVERED")]
    Undiscovered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Ability {
    None,
    Guts,
    LightBall,
    ThickClub,
}

impl Default for Ability {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dv {
    pub attack: u8,
    pub defense: u8,
    pub speed: u8,
    pub special: u8,
    pub hp: u8,
}

impl Dv {
    pub fn from_non_hp(attack: u8, defense: u8, speed: u8, special: u8) -> Self {
        let mut hp = 0;
        if attack % 2 == 1 {
            hp += 8;
        }
        if defense % 2 == 1 {
            hp += 4;
        }
        if speed % 2 == 1 {
            hp += 2;
        }
        if special % 2 == 1 {
            hp += 1;
        }
        Self {
            attack,
            defense,
            speed,
            special,
            hp,
        }
    }

    pub const fn for_stat(self, stat: Stat) -> Option<u8> {
        match stat {
            Stat::Hp => Some(self.hp),
            Stat::Attack => Some(self.attack),
            Stat::Defense => Some(self.defense),
            Stat::Speed => Some(self.speed),
            Stat::SpecialAttack | Stat::SpecialDefense => Some(self.special),
            Stat::Accuracy | Stat::Evasion => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaseStats {
    pub hp: u16,
    pub attack: u16,
    pub defense: u16,
    pub speed: u16,
    pub special_attack: u16,
    pub special_defense: u16,
}

impl BaseStats {
    pub const fn new(
        hp: u16,
        attack: u16,
        defense: u16,
        speed: u16,
        special_attack: u16,
        special_defense: u16,
    ) -> Self {
        Self {
            hp,
            attack,
            defense,
            speed,
            special_attack,
            special_defense,
        }
    }

    pub const fn for_stat(self, stat: Stat) -> Option<u16> {
        match stat {
            Stat::Hp => Some(self.hp),
            Stat::Attack => Some(self.attack),
            Stat::Defense => Some(self.defense),
            Stat::Speed => Some(self.speed),
            Stat::SpecialAttack => Some(self.special_attack),
            Stat::SpecialDefense => Some(self.special_defense),
            Stat::Accuracy | Stat::Evasion => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PokemonSpecies {
    pub id: String,
    pub int_id: u16,
    pub base_stats: BaseStats,
    pub type1: PokemonType,
    pub type2: PokemonType,
    pub catch_rate: u8,
    pub base_exp: u16,
    #[serde(deserialize_with = "required_nullable_string")]
    pub item1: Option<String>,
    #[serde(deserialize_with = "required_nullable_string")]
    pub item2: Option<String>,
    pub gender_ratio: u8,
    pub unknown1: u8,
    pub step_cycles_to_hatch: u8,
    pub unknown2: u8,
    pub growth_rate: GrowthRate,
    pub egg_group1: EggGroup,
    pub egg_group2: EggGroup,
    pub tmhm_learnset: Vec<String>,
    pub ability: Ability,
    pub pic_size: u8,
    pub front_pic: u16,
    pub back_pic: u16,
    pub weight: u16,
}

fn required_nullable_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)
}

impl PokemonSpecies {
    pub fn new_for_tests(id: impl Into<String>, base_stats: BaseStats) -> Self {
        Self {
            id: id.into(),
            int_id: 0,
            base_stats,
            type1: PokemonType::Normal,
            type2: PokemonType::Normal,
            catch_rate: 45,
            base_exp: 64,
            item1: None,
            item2: None,
            gender_ratio: 127,
            unknown1: 0,
            step_cycles_to_hatch: 20,
            unknown2: 0,
            growth_rate: GrowthRate::MediumSlow,
            egg_group1: EggGroup::Monster,
            egg_group2: EggGroup::Monster,
            tmhm_learnset: Vec::new(),
            ability: Ability::None,
            pic_size: 0,
            front_pic: 0,
            back_pic: 0,
            weight: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LearnedMove {
    pub name: String,
    pub current_pp: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum PokemonBuildError {
    #[error("missing level-up learnset for species '{species_id}'")]
    MissingLearnset { species_id: String },
    #[error("learnset for species '{species_id}' references missing move '{move_name}'")]
    UnknownLearnsetMove {
        species_id: String,
        move_name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pokemon {
    pub species: PokemonSpecies,
    pub nickname: String,
    pub item: Option<String>,
    pub moves: Vec<LearnedMove>,
    #[serde(default)]
    pub status: Option<String>,
    pub level: u8,
    pub hp: u16,
    pub max_hp: u16,
    pub dvs: Dv,
    pub sleep_turns: u8,
    pub flinching: bool,
    pub rampage_turns: u8,
    pub confusion_turns: u8,
    pub perish_song_turns: u8,
    pub focus_energy: bool,
    pub original_trainer_name: String,
    pub original_trainer_id: u16,
    pub experience: i32,
    pub hp_exp: u16,
    pub attack_exp: u16,
    pub defense_exp: u16,
    pub speed_exp: u16,
    pub special_exp: u16,
    pub happiness: u8,
    pub turns_in_battle: u16,
    pub stat_boosts: BTreeMap<Stat, i8>,
    pub attack: u16,
    pub defense: u16,
    pub speed: u16,
    pub special_attack: u16,
    pub special_defense: u16,
}

impl Pokemon {
    pub fn new_for_tests(species: PokemonSpecies, level: u8, dvs: Dv) -> Self {
        let stats = calculate_stats(&species, level, dvs, StatExperience::default());
        Self {
            nickname: pokemon_species_display_name(&species.id),
            species,
            item: None,
            moves: Vec::new(),
            status: None,
            level,
            hp: stats.max_hp,
            max_hp: stats.max_hp,
            dvs,
            sleep_turns: 0,
            flinching: false,
            rampage_turns: 0,
            confusion_turns: 0,
            perish_song_turns: 0,
            focus_energy: false,
            original_trainer_name: "PLAYER".to_string(),
            original_trainer_id: 0,
            experience: 0,
            hp_exp: 0,
            attack_exp: 0,
            defense_exp: 0,
            speed_exp: 0,
            special_exp: 0,
            happiness: 70,
            turns_in_battle: 0,
            stat_boosts: default_stat_boosts(),
            attack: stats.attack,
            defense: stats.defense,
            speed: stats.speed,
            special_attack: stats.special_attack,
            special_defense: stats.special_defense,
        }
    }

    pub fn stat_exp_for_stat(&self, stat: Stat) -> Option<u16> {
        Some(match stat {
            Stat::Hp => self.hp_exp,
            Stat::Attack => self.attack_exp,
            Stat::Defense => self.defense_exp,
            Stat::Speed => self.speed_exp,
            Stat::SpecialAttack | Stat::SpecialDefense => self.special_exp,
            Stat::Accuracy | Stat::Evasion => return None,
        })
    }

    pub fn calculate_stat(&self, stat: Stat) -> Option<u16> {
        calculate_stat(
            self.species.base_stats,
            self.level,
            self.dvs,
            self.stat_exp_for_stat(stat)?,
            stat,
        )
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatExperience {
    pub hp: u16,
    pub attack: u16,
    pub defense: u16,
    pub speed: u16,
    pub special: u16,
}

impl StatExperience {
    pub const fn for_stat(self, stat: Stat) -> Option<u16> {
        match stat {
            Stat::Hp => Some(self.hp),
            Stat::Attack => Some(self.attack),
            Stat::Defense => Some(self.defense),
            Stat::Speed => Some(self.speed),
            Stat::SpecialAttack | Stat::SpecialDefense => Some(self.special),
            Stat::Accuracy | Stat::Evasion => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalculatedStats {
    pub max_hp: u16,
    pub attack: u16,
    pub defense: u16,
    pub speed: u16,
    pub special_attack: u16,
    pub special_defense: u16,
}

pub fn pokemon_species_display_name(id: &str) -> String {
    match id {
        "FARFETCH_D" => "FARFETCH'D".to_string(),
        "HO_OH" => "HO-OH".to_string(),
        "MR__MIME" => "MR.MIME".to_string(),
        "NIDORAN_F" => "NIDORAN♀".to_string(),
        "NIDORAN_M" => "NIDORAN♂".to_string(),
        exact_id => exact_id.to_string(),
    }
}

pub fn default_stat_boosts() -> BTreeMap<Stat, i8> {
    [
        Stat::Hp,
        Stat::Attack,
        Stat::Defense,
        Stat::Speed,
        Stat::SpecialAttack,
        Stat::SpecialDefense,
        Stat::Accuracy,
        Stat::Evasion,
    ]
    .into_iter()
    .map(|stat| (stat, 0))
    .collect()
}

pub fn calculate_stats(
    species: &PokemonSpecies,
    level: u8,
    dvs: Dv,
    stat_exp: StatExperience,
) -> CalculatedStats {
    CalculatedStats {
        max_hp: calculate_stat(species.base_stats, level, dvs, stat_exp.hp, Stat::Hp)
            .expect("HP is calculable"),
        attack: calculate_stat(
            species.base_stats,
            level,
            dvs,
            stat_exp.attack,
            Stat::Attack,
        )
        .expect("attack is calculable"),
        defense: calculate_stat(
            species.base_stats,
            level,
            dvs,
            stat_exp.defense,
            Stat::Defense,
        )
        .expect("defense is calculable"),
        speed: calculate_stat(species.base_stats, level, dvs, stat_exp.speed, Stat::Speed)
            .expect("speed is calculable"),
        special_attack: calculate_stat(
            species.base_stats,
            level,
            dvs,
            stat_exp.special,
            Stat::SpecialAttack,
        )
        .expect("special attack is calculable"),
        special_defense: calculate_stat(
            species.base_stats,
            level,
            dvs,
            stat_exp.special,
            Stat::SpecialDefense,
        )
        .expect("special defense is calculable"),
    }
}

pub fn calculate_stat(
    base_stats: BaseStats,
    level: u8,
    dvs: Dv,
    stat_exp: u16,
    stat: Stat,
) -> Option<u16> {
    let base = base_stats.for_stat(stat)?;
    let dv = dvs.for_stat(stat)? as u16;
    let mut exp_modifier = isqrt(stat_exp as u32).min(255) / 4;
    if stat == Stat::Hp && stat_exp == u16::MAX {
        exp_modifier += 1;
    }
    let interim_value = (base + dv) * 2 + exp_modifier as u16;
    let main_stat_component = (interim_value * level as u16) / 100;
    if stat == Stat::Hp {
        Some(main_stat_component + level as u16 + 10)
    } else {
        Some(main_stat_component + 5)
    }
}

pub fn create_pokemon_from_known_dvs(
    species: &PokemonSpecies,
    level: u8,
    dvs: Dv,
    learnsets: &SpeciesLearnsets,
    moves: &BTreeMap<String, Move>,
) -> Result<Pokemon, PokemonBuildError> {
    let stats = calculate_stats(species, level, dvs, StatExperience::default());
    if !learnsets.contains_key(&species.id) {
        return Err(PokemonBuildError::MissingLearnset {
            species_id: species.id.clone(),
        });
    }
    let learned_moves = default_moves_for_level(learnsets, &species.id, level, 4)
        .map_err(|_| PokemonBuildError::MissingLearnset {
            species_id: species.id.clone(),
        })?
        .into_iter()
        .map(|name| {
            let move_data =
                moves
                    .get(&name)
                    .ok_or_else(|| PokemonBuildError::UnknownLearnsetMove {
                        species_id: species.id.clone(),
                        move_name: name.clone(),
                    })?;
            Ok(LearnedMove {
                name,
                current_pp: move_data.pp,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Pokemon {
        species: species.clone(),
        nickname: pokemon_species_display_name(&species.id),
        item: None,
        moves: learned_moves,
        status: None,
        level,
        hp: stats.max_hp,
        max_hp: stats.max_hp,
        dvs,
        sleep_turns: 0,
        flinching: false,
        rampage_turns: 0,
        confusion_turns: 0,
        perish_song_turns: 0,
        focus_energy: false,
        original_trainer_name: "PLAYER".to_string(),
        original_trainer_id: 0,
        experience: calculate_experience(species.growth_rate, level),
        hp_exp: 0,
        attack_exp: 0,
        defense_exp: 0,
        speed_exp: 0,
        special_exp: 0,
        happiness: 70,
        turns_in_battle: 0,
        stat_boosts: default_stat_boosts(),
        attack: stats.attack,
        defense: stats.defense,
        speed: stats.speed,
        special_attack: stats.special_attack,
        special_defense: stats.special_defense,
    })
}

pub fn isqrt(n: u32) -> u32 {
    let mut x = (n as f64).sqrt().floor() as u32;
    if (x + 1).saturating_mul(x + 1) <= n {
        x += 1;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chikorita() -> PokemonSpecies {
        PokemonSpecies::new_for_tests("CHIKORITA", BaseStats::new(45, 49, 65, 45, 49, 65))
    }

    #[test]
    fn display_names_match_asm_special_cases() {
        assert_eq!(pokemon_species_display_name("FARFETCH_D"), "FARFETCH'D");
        assert_eq!(pokemon_species_display_name("HO_OH"), "HO-OH");
        assert_eq!(pokemon_species_display_name("MR__MIME"), "MR.MIME");
        assert_eq!(pokemon_species_display_name("NIDORAN_F"), "NIDORAN♀");
        assert_eq!(pokemon_species_display_name("NIDORAN_M"), "NIDORAN♂");
        assert_eq!(pokemon_species_display_name("CHIKORITA"), "CHIKORITA");
    }

    #[test]
    fn display_names_do_not_repair_case_changed_species_ids() {
        assert_eq!(pokemon_species_display_name("ho_oh"), "ho_oh");
        assert_eq!(pokemon_species_display_name("chikorita"), "chikorita");
    }

    fn species_json() -> serde_json::Value {
        serde_json::json!({
            "id":"BULBASAUR",
            "int_id":1,
            "base_stats":{
                "hp":45,
                "attack":49,
                "defense":49,
                "speed":45,
                "special_attack":65,
                "special_defense":65
            },
            "type1":"GRASS",
            "type2":"POISON",
            "catch_rate":45,
            "base_exp":64,
            "item1":null,
            "item2":null,
            "gender_ratio":31,
            "unknown1":0,
            "step_cycles_to_hatch":20,
            "unknown2":0,
            "growth_rate":"GROWTH_MEDIUM_SLOW",
            "egg_group1":"EGG_MONSTER",
            "egg_group2":"EGG_PLANT",
            "tmhm_learnset":["HEADBUTT"],
            "ability":"NONE",
            "pic_size":0,
            "front_pic":0,
            "back_pic":0,
            "weight":150
        })
    }

    #[test]
    fn species_json_requires_explicit_nullable_held_items() {
        let mut species = species_json();
        species
            .as_object_mut()
            .expect("species object")
            .remove("item1");
        let error = serde_json::from_value::<PokemonSpecies>(species)
            .expect_err("missing item1 must not deserialize as None")
            .to_string();
        assert!(error.contains("missing field `item1`"), "{error}");
    }

    #[test]
    fn species_json_requires_explicit_exported_stat_metadata() {
        let mut species = species_json();
        species
            .as_object_mut()
            .expect("species object")
            .remove("weight");
        let error = serde_json::from_value::<PokemonSpecies>(species)
            .expect_err("missing weight must not default to zero")
            .to_string();
        assert!(error.contains("missing field `weight`"), "{error}");
    }

    #[test]
    fn species_json_rejects_unknown_modpack_fields() {
        let mut species = species_json();
        species["legacy_name"] = serde_json::json!("Bulbasaur");
        let error = serde_json::from_value::<PokemonSpecies>(species)
            .expect_err("species must not accept legacy display-name fields")
            .to_string();
        assert!(error.contains("unknown field `legacy_name`"), "{error}");

        let error = serde_json::from_value::<BaseStats>(serde_json::json!({
            "hp":45,
            "attack":49,
            "defense":49,
            "speed":45,
            "special_attack":65,
            "special_defense":65,
            "special":65
        }))
        .expect_err("base stats must not accept legacy combined special")
        .to_string();
        assert!(error.contains("unknown field `special`"), "{error}");

        let error = serde_json::from_value::<LearnedMove>(serde_json::json!({
            "name": "TACKLE",
            "current_pp": 35,
            "move": "Tackle"
        }))
        .expect_err("learned moves must not accept display move aliases")
        .to_string();
        assert!(error.contains("unknown field `move`"), "{error}");
    }

    #[test]
    fn hp_dv_is_derived_from_low_bits_of_other_dvs() {
        assert_eq!(Dv::from_non_hp(1, 1, 1, 1).hp, 15);
        assert_eq!(Dv::from_non_hp(2, 3, 4, 5).hp, 5);
        assert_eq!(Dv::from_non_hp(2, 4, 6, 8).hp, 0);
    }

    #[test]
    fn stat_formula_matches_typescript_port() {
        let species = chikorita();
        let dvs = Dv::from_non_hp(10, 10, 10, 10);
        let stats = calculate_stats(&species, 5, dvs, StatExperience::default());
        assert_eq!(
            stats,
            CalculatedStats {
                max_hp: 19,
                attack: 10,
                defense: 12,
                speed: 10,
                special_attack: 10,
                special_defense: 12,
            }
        );
    }

    #[test]
    fn hp_stat_exp_uses_gen_two_max_exp_adjustment() {
        let species = chikorita();
        let dvs = Dv::from_non_hp(15, 15, 15, 15);
        let maxed = calculate_stat(species.base_stats, 100, dvs, u16::MAX, Stat::Hp);
        assert_eq!(maxed, Some(294));
    }

    #[test]
    fn default_stat_boosts_include_battle_modifiable_stats() {
        let boosts = default_stat_boosts();
        assert_eq!(boosts.len(), 8);
        assert_eq!(boosts[&Stat::Accuracy], 0);
        assert_eq!(boosts[&Stat::Evasion], 0);
    }

    #[test]
    fn pokemon_factory_uses_learnsets_move_pp_stats_and_experience() {
        let species = chikorita();
        let learnsets = [(
            "CHIKORITA".to_string(),
            vec![
                crate::systems::learnsets::LearnsetEntry(1, "TACKLE".to_string()),
                crate::systems::learnsets::LearnsetEntry(1, "GROWL".to_string()),
                crate::systems::learnsets::LearnsetEntry(8, "RAZOR_LEAF".to_string()),
            ],
        )]
        .into_iter()
        .collect();
        let moves = [
            (
                "TACKLE".to_string(),
                Move {
                    name: "TACKLE".to_string(),
                    move_type: PokemonType::Normal,
                    power: 35,
                    accuracy: 95,
                    pp: 35,
                    effect: "NORMAL_HIT".to_string(),
                    effect_chance: 0,
                    stat: None,
                    amount: None,
                },
            ),
            (
                "GROWL".to_string(),
                Move {
                    name: "GROWL".to_string(),
                    move_type: PokemonType::Normal,
                    power: 0,
                    accuracy: 100,
                    pp: 40,
                    effect: "ATTACK_DOWN".to_string(),
                    effect_chance: 0,
                    stat: Some(Stat::Attack),
                    amount: Some(-1),
                },
            ),
        ]
        .into_iter()
        .collect();

        let pokemon = create_pokemon_from_known_dvs(
            &species,
            5,
            Dv::from_non_hp(10, 10, 10, 10),
            &learnsets,
            &moves,
        )
        .expect("pokemon builds from exact learnset and moves");

        assert_eq!(pokemon.nickname, "CHIKORITA");
        assert_eq!(pokemon.moves.len(), 2);
        assert_eq!(pokemon.moves[0].name, "TACKLE");
        assert_eq!(pokemon.moves[0].current_pp, 35);
        assert_eq!(pokemon.moves[1].current_pp, 40);
        assert_eq!(pokemon.max_hp, 19);
        assert_eq!(pokemon.happiness, 70);
        assert_eq!(pokemon.experience, 135);
    }

    #[test]
    fn pokemon_factory_rejects_missing_learnset_or_move_without_pp_fallback() {
        let species = chikorita();
        let learnsets = [(
            "CHIKORITA".to_string(),
            vec![crate::systems::learnsets::LearnsetEntry(
                1,
                "TACKLE".to_string(),
            )],
        )]
        .into_iter()
        .collect();

        assert_eq!(
            create_pokemon_from_known_dvs(
                &species,
                5,
                Dv::from_non_hp(10, 10, 10, 10),
                &SpeciesLearnsets::new(),
                &BTreeMap::new(),
            ),
            Err(PokemonBuildError::MissingLearnset {
                species_id: "CHIKORITA".to_string(),
            })
        );
        assert_eq!(
            create_pokemon_from_known_dvs(
                &species,
                5,
                Dv::from_non_hp(10, 10, 10, 10),
                &learnsets,
                &BTreeMap::new(),
            ),
            Err(PokemonBuildError::UnknownLearnsetMove {
                species_id: "CHIKORITA".to_string(),
                move_name: "TACKLE".to_string(),
            })
        );
    }
}
