use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};

use super::move_data::Move;
use crate::systems::experience::{ExperienceError, GrowthRateCatalog, calculate_experience};
use crate::systems::learnsets::{SpeciesLearnsets, default_moves_for_level};
use crate::world::encounters::TimeOfDay;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", deny_unknown_fields)]
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

pub type PokemonType = String;

pub fn pokemon_type(id: &str) -> PokemonType {
    id.to_string()
}

pub type GrowthRate = String;

pub fn growth_rate(id: &str) -> GrowthRate {
    id.to_string()
}

pub type EggGroup = String;
pub type Ability = String;

pub fn egg_group(id: &str) -> EggGroup {
    id.to_string()
}

pub fn ability(id: &str) -> Ability {
    id.to_string()
}

pub fn max_move_pp(base_pp: u8, pp_ups: u8) -> u8 {
    base_pp.saturating_add((base_pp / 5).saturating_mul(pp_ups.min(3)))
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Dv {
    pub attack: u8,
    pub defense: u8,
    pub speed: u8,
    pub special: u8,
    pub hp: u8,
}

impl<'de> Deserialize<'de> for Dv {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawDv {
            attack: u8,
            defense: u8,
            speed: u8,
            special: u8,
            hp: u8,
        }

        let raw = RawDv::deserialize(deserializer)?;
        let dvs = Self {
            attack: raw.attack,
            defense: raw.defense,
            speed: raw.speed,
            special: raw.special,
            hp: raw.hp,
        };
        validate_dvs(dvs).map_err(serde::de::Error::custom)?;
        Ok(dvs)
    }
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

    /// Return Crystal's one-based Unown letter index derived from the four
    /// non-HP DVs (`A = 1` through `Z = 26`).
    pub const fn unown_letter(self) -> u8 {
        let packed = ((self.attack & 0x06) >> 1) << 6
            | ((self.defense & 0x06) >> 1) << 4
            | ((self.speed & 0x06) >> 1) << 2
            | ((self.special & 0x06) >> 1);
        packed / 10 + 1
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BaseStats {
    pub hp: u16,
    pub attack: u16,
    pub defense: u16,
    pub speed: u16,
    pub special_attack: u16,
    pub special_defense: u16,
}

impl<'de> Deserialize<'de> for BaseStats {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawBaseStats {
            hp: u16,
            attack: u16,
            defense: u16,
            speed: u16,
            special_attack: u16,
            special_defense: u16,
        }

        let raw = RawBaseStats::deserialize(deserializer)?;
        let stats = Self {
            hp: raw.hp,
            attack: raw.attack,
            defense: raw.defense,
            speed: raw.speed,
            special_attack: raw.special_attack,
            special_defense: raw.special_defense,
        };
        validate_base_stats(stats).map_err(serde::de::Error::custom)?;
        Ok(stats)
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PokemonSpecies {
    pub id: String,
    pub int_id: u16,
    pub base_stats: BaseStats,
    pub type1: PokemonType,
    pub type2: PokemonType,
    pub catch_rate: u8,
    pub base_exp: u16,
    pub item1: Option<String>,
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

impl<'de> Deserialize<'de> for PokemonSpecies {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSpecies {
            #[serde(deserialize_with = "required_pokemon_token")]
            id: String,
            int_id: u16,
            base_stats: BaseStats,
            #[serde(deserialize_with = "required_pokemon_token")]
            type1: PokemonType,
            #[serde(deserialize_with = "required_pokemon_token")]
            type2: PokemonType,
            catch_rate: u8,
            base_exp: u16,
            #[serde(deserialize_with = "required_nullable_pokemon_token")]
            item1: Option<String>,
            #[serde(deserialize_with = "required_nullable_pokemon_token")]
            item2: Option<String>,
            gender_ratio: u8,
            unknown1: u8,
            step_cycles_to_hatch: u8,
            unknown2: u8,
            #[serde(deserialize_with = "required_pokemon_token")]
            growth_rate: GrowthRate,
            #[serde(deserialize_with = "required_pokemon_token")]
            egg_group1: EggGroup,
            #[serde(deserialize_with = "required_pokemon_token")]
            egg_group2: EggGroup,
            #[serde(deserialize_with = "required_pokemon_token_vec")]
            tmhm_learnset: Vec<String>,
            #[serde(deserialize_with = "required_pokemon_token")]
            ability: Ability,
            pic_size: u8,
            front_pic: u16,
            back_pic: u16,
            weight: u16,
        }

        let raw = RawSpecies::deserialize(deserializer)?;
        let species = Self {
            id: raw.id,
            int_id: raw.int_id,
            base_stats: raw.base_stats,
            type1: raw.type1,
            type2: raw.type2,
            catch_rate: raw.catch_rate,
            base_exp: raw.base_exp,
            item1: raw.item1,
            item2: raw.item2,
            gender_ratio: raw.gender_ratio,
            unknown1: raw.unknown1,
            step_cycles_to_hatch: raw.step_cycles_to_hatch,
            unknown2: raw.unknown2,
            growth_rate: raw.growth_rate,
            egg_group1: raw.egg_group1,
            egg_group2: raw.egg_group2,
            tmhm_learnset: raw.tmhm_learnset,
            ability: raw.ability,
            pic_size: raw.pic_size,
            front_pic: raw.front_pic,
            back_pic: raw.back_pic,
            weight: raw.weight,
        };
        species
            .validate_compiled_shape()
            .map_err(serde::de::Error::custom)?;
        Ok(species)
    }
}

fn required_pokemon_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if !is_exact_pokemon_token(&value) {
        return Err(serde::de::Error::custom(format!(
            "Pokemon token must be exact ASCII alphanumeric/underscore, found {value:?}"
        )));
    }
    validate_no_reserved_pokemon_token(&value).map_err(serde::de::Error::custom)?;
    Ok(value)
}

fn required_nullable_pokemon_token<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    match value {
        Some(token) if is_exact_pokemon_token(&token) => {
            validate_no_reserved_pokemon_token(&token).map_err(serde::de::Error::custom)?;
            Ok(Some(token))
        }
        Some(token) => Err(serde::de::Error::custom(format!(
            "Pokemon token must be exact ASCII alphanumeric/underscore, found {token:?}"
        ))),
        None => Ok(None),
    }
}

fn required_pokemon_token_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values = Vec::<String>::deserialize(deserializer)?;
    if let Some(token) = values.iter().find(|token| !is_exact_pokemon_token(token)) {
        Err(serde::de::Error::custom(format!(
            "Pokemon token must be exact ASCII alphanumeric/underscore, found {token:?}"
        )))
    } else {
        for token in &values {
            validate_no_reserved_pokemon_token(token).map_err(serde::de::Error::custom)?;
        }
        Ok(values)
    }
}

fn is_exact_pokemon_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn validate_no_reserved_pokemon_token(value: &str) -> Result<(), String> {
    let lowered = value.to_ascii_lowercase();
    if lowered.starts_with("fallback") || lowered.starts_with("legacy") {
        return Err(format!(
            "Pokemon token '{value}' uses reserved modpack payload prefix"
        ));
    }
    Ok(())
}

impl PokemonSpecies {
    pub fn new_for_tests(id: impl Into<String>, base_stats: BaseStats) -> Self {
        Self {
            id: id.into(),
            int_id: 0,
            base_stats,
            type1: pokemon_type("NORMAL"),
            type2: pokemon_type("NORMAL"),
            catch_rate: 45,
            base_exp: 64,
            item1: None,
            item2: None,
            gender_ratio: 127,
            unknown1: 0,
            step_cycles_to_hatch: 20,
            unknown2: 0,
            growth_rate: growth_rate("GROWTH_MEDIUM_SLOW"),
            egg_group1: egg_group("EGG_MONSTER"),
            egg_group2: egg_group("EGG_MONSTER"),
            tmhm_learnset: Vec::new(),
            ability: ability("NONE"),
            pic_size: 0,
            front_pic: 0,
            back_pic: 0,
            weight: 0,
        }
    }

    pub fn validate_compiled_shape(&self) -> Result<(), String> {
        validate_exact_token("pokemon.species.id", &self.id)?;
        validate_base_stats(self.base_stats)?;
        if self.int_id == 0 {
            return Err(format!("pokemon.species {} has zero int_id", self.id));
        }
        if self.catch_rate == 0 {
            return Err(format!("pokemon.species {} has zero catch_rate", self.id));
        }
        if self.base_exp == 0 {
            return Err(format!("pokemon.species {} has zero base_exp", self.id));
        }
        if self.step_cycles_to_hatch == 0 {
            return Err(format!(
                "pokemon.species {} has zero step_cycles_to_hatch",
                self.id
            ));
        }
        if self.pic_size > 0x77 {
            return Err(format!(
                "pokemon.species {} pic_size {} is outside packed 4-bit dimensions",
                self.id, self.pic_size
            ));
        }
        validate_exact_token("pokemon.species.type1", &self.type1)?;
        validate_exact_token("pokemon.species.type2", &self.type2)?;
        validate_exact_token("pokemon.species.growth_rate", &self.growth_rate)?;
        validate_exact_token("pokemon.species.egg_group1", &self.egg_group1)?;
        validate_exact_token("pokemon.species.egg_group2", &self.egg_group2)?;
        validate_exact_token("pokemon.species.ability", &self.ability)?;
        for (index, move_id) in self.tmhm_learnset.iter().enumerate() {
            validate_exact_token(&format!("pokemon.species.tmhm_learnset[{index}]"), move_id)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LearnedMove {
    #[serde(deserialize_with = "required_pokemon_token")]
    pub name: String,
    pub current_pp: u8,
    pub pp_ups: u8,
}

impl<'de> Deserialize<'de> for LearnedMove {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawLearnedMove {
            #[serde(deserialize_with = "required_pokemon_token")]
            name: String,
            current_pp: u8,
            pp_ups: u8,
        }

        let raw = RawLearnedMove::deserialize(deserializer)?;
        let learned_move = Self {
            name: raw.name,
            current_pp: raw.current_pp,
            pp_ups: raw.pp_ups,
        };
        learned_move
            .validate_saved_state(0)
            .map_err(serde::de::Error::custom)?;
        Ok(learned_move)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum PokemonBuildError {
    #[error("missing level-up learnset for species '{species_id}'")]
    MissingLearnset { species_id: String },
    #[error("learnset for species '{species_id}' references missing move '{move_name}'")]
    UnknownLearnsetMove {
        species_id: String,
        move_name: String,
    },
    #[error("experience table error: {0}")]
    Experience(#[from] ExperienceError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaughtData {
    /// Low six bits of `MON_CAUGHTLEVEL`.
    pub level: u8,
    /// High two bits of `MON_CAUGHTLEVEL`; gift Pokemon store zero/None.
    pub time_of_day: Option<TimeOfDay>,
    /// High bit of `MON_CAUGHTLOCATION`.
    pub original_trainer_gender: u8,
    /// Low seven bits of `MON_CAUGHTLOCATION`.
    pub location: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MailData {
    pub message: String,
    pub author: String,
    pub nationality: u16,
    pub author_id: u16,
    pub species: String,
    pub mail_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pokemon {
    pub species: PokemonSpecies,
    pub nickname: String,
    #[serde(deserialize_with = "required_nullable_pokemon_token")]
    pub item: Option<String>,
    pub moves: Vec<LearnedMove>,
    #[serde(deserialize_with = "required_nullable_pokemon_token")]
    pub status: Option<String>,
    /// Persistent egg identity; the status field is reserved for battle
    /// conditions and is retained only as a legacy read discriminator.
    #[serde(default)]
    pub is_egg: bool,
    /// Pokérus status byte from the party structure. The low nibble stores
    /// remaining days; the high nibble is the strain and is preserved after
    /// the counter reaches zero.
    pub pokerus: u8,
    #[serde(default)]
    pub caught_data: Option<CaughtData>,
    #[serde(default)]
    pub mail: Option<MailData>,
    pub level: u8,
    pub hp: u16,
    pub max_hp: u16,
    pub dvs: Dv,
    pub sleep_turns: u8,
    pub flinching: bool,
    pub rampage_turns: u8,
    pub confusion_turns: u16,
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

impl<'de> Deserialize<'de> for Pokemon {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawPokemon {
            species: PokemonSpecies,
            nickname: String,
            #[serde(deserialize_with = "required_nullable_pokemon_token")]
            item: Option<String>,
            moves: Vec<LearnedMove>,
            #[serde(deserialize_with = "required_nullable_pokemon_token")]
            status: Option<String>,
            #[serde(default)]
            is_egg: bool,
            pokerus: u8,
            #[serde(default)]
            caught_data: Option<CaughtData>,
            #[serde(default)]
            mail: Option<MailData>,
            level: u8,
            hp: u16,
            max_hp: u16,
            dvs: Dv,
            sleep_turns: u8,
            flinching: bool,
            rampage_turns: u8,
            confusion_turns: u16,
            perish_song_turns: u8,
            focus_energy: bool,
            original_trainer_name: String,
            original_trainer_id: u16,
            experience: i32,
            hp_exp: u16,
            attack_exp: u16,
            defense_exp: u16,
            speed_exp: u16,
            special_exp: u16,
            happiness: u8,
            turns_in_battle: u16,
            stat_boosts: BTreeMap<Stat, i8>,
            attack: u16,
            defense: u16,
            speed: u16,
            special_attack: u16,
            special_defense: u16,
        }

        let raw = RawPokemon::deserialize(deserializer)?;
        let pokemon = Self {
            species: raw.species,
            nickname: raw.nickname,
            item: raw.item,
            moves: raw.moves,
            status: raw.status,
            is_egg: raw.is_egg,
            pokerus: raw.pokerus,
            caught_data: raw.caught_data,
            mail: raw.mail,
            level: raw.level,
            hp: raw.hp,
            max_hp: raw.max_hp,
            dvs: raw.dvs,
            sleep_turns: raw.sleep_turns,
            flinching: raw.flinching,
            rampage_turns: raw.rampage_turns,
            confusion_turns: raw.confusion_turns,
            perish_song_turns: raw.perish_song_turns,
            focus_energy: raw.focus_energy,
            original_trainer_name: raw.original_trainer_name,
            original_trainer_id: raw.original_trainer_id,
            experience: raw.experience,
            hp_exp: raw.hp_exp,
            attack_exp: raw.attack_exp,
            defense_exp: raw.defense_exp,
            speed_exp: raw.speed_exp,
            special_exp: raw.special_exp,
            happiness: raw.happiness,
            turns_in_battle: raw.turns_in_battle,
            stat_boosts: raw.stat_boosts,
            attack: raw.attack,
            defense: raw.defense,
            speed: raw.speed,
            special_attack: raw.special_attack,
            special_defense: raw.special_defense,
        };
        pokemon
            .validate_saved_state()
            .map_err(serde::de::Error::custom)?;
        Ok(pokemon)
    }
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
            is_egg: false,
            pokerus: 0,
            caught_data: None,
            mail: None,
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

    pub fn validate_saved_state(&self) -> Result<(), String> {
        validate_exact_token("pokemon.species.id", &self.species.id)?;
        validate_exact_text("pokemon.nickname", &self.nickname)?;
        validate_exact_text("pokemon.original_trainer_name", &self.original_trainer_name)?;
        if let Some(item) = &self.item {
            validate_exact_token("pokemon.item", item)?;
        }
        if let Some(status) = &self.status {
            validate_exact_token("pokemon.status", status)?;
        }
        if let Some(caught) = &self.caught_data {
            if caught.level > 0x3f {
                return Err(format!(
                    "pokemon.caught_data.level {} is outside six-bit range 0..63",
                    caught.level
                ));
            }
            if caught.original_trainer_gender > 1 {
                return Err(format!(
                    "pokemon.caught_data.original_trainer_gender {} is outside Crystal gender range 0..1",
                    caught.original_trainer_gender
                ));
            }
            if caught.location > 0x7f {
                return Err(format!(
                    "pokemon.caught_data.location {} is outside seven-bit range 0..127",
                    caught.location
                ));
            }
        }
        if let Some(mail) = &self.mail {
            validate_mail_message(&mail.message)?;
            validate_exact_text("pokemon.mail.author", &mail.author)?;
            if mail.nationality > 4 {
                return Err(format!(
                    "pokemon.mail.nationality {} is outside Crystal language range 0..4",
                    mail.nationality
                ));
            }
            validate_exact_token("pokemon.mail.species", &mail.species)?;
            validate_exact_token("pokemon.mail.mail_type", &mail.mail_type)?;
            if !crate::models::item::is_mail_item_id(&mail.mail_type) {
                return Err(format!(
                    "pokemon.mail.mail_type '{}' is not an ASM Mail item",
                    mail.mail_type
                ));
            }
            if self.item.as_deref() != Some(mail.mail_type.as_str()) {
                return Err("pokemon Mail type does not match its held item".to_string());
            }
        }
        if self.level == 0 || self.level > 100 {
            return Err(format!(
                "pokemon.level {} is outside range 1..100",
                self.level
            ));
        }
        if self.max_hp == 0 {
            return Err("pokemon.max_hp must be nonzero".to_string());
        }
        if self.hp > self.max_hp {
            return Err(format!(
                "pokemon.hp {} cannot exceed max_hp {}",
                self.hp, self.max_hp
            ));
        }
        validate_dvs(self.dvs)?;
        self.validate_stat_projection()?;
        if self.experience < 0 {
            return Err(format!(
                "pokemon.experience {} must be nonnegative",
                self.experience
            ));
        }
        if self.moves.len() > 4 {
            return Err(format!(
                "pokemon.moves has {} entries, maximum is 4",
                self.moves.len()
            ));
        }
        for (index, learned_move) in self.moves.iter().enumerate() {
            learned_move.validate_saved_state(index)?;
        }
        validate_stat_boosts(&self.stat_boosts)
    }

    fn validate_stat_projection(&self) -> Result<(), String> {
        let expected = calculate_stats(
            &self.species,
            self.level,
            self.dvs,
            StatExperience {
                hp: self.hp_exp,
                attack: self.attack_exp,
                defense: self.defense_exp,
                speed: self.speed_exp,
                special: self.special_exp,
            },
        );
        for (field, actual, expected) in [
            ("max_hp", self.max_hp, expected.max_hp),
            ("attack", self.attack, expected.attack),
            ("defense", self.defense, expected.defense),
            ("speed", self.speed, expected.speed),
            (
                "special_attack",
                self.special_attack,
                expected.special_attack,
            ),
            (
                "special_defense",
                self.special_defense,
                expected.special_defense,
            ),
        ] {
            if actual != expected {
                return Err(format!(
                    "pokemon.{field} {actual} does not match calculated stat {expected}"
                ));
            }
        }
        Ok(())
    }
}

impl LearnedMove {
    pub fn validate_saved_state(&self, index: usize) -> Result<(), String> {
        validate_exact_token(&format!("pokemon.moves[{index}].name"), &self.name)?;
        if self.pp_ups > 3 {
            return Err(format!(
                "pokemon.moves[{index}].pp_ups {} is outside range 0..3",
                self.pp_ups
            ));
        }
        Ok(())
    }
}

fn validate_dvs(dvs: Dv) -> Result<(), String> {
    for (field, value) in [
        ("attack", dvs.attack),
        ("defense", dvs.defense),
        ("speed", dvs.speed),
        ("special", dvs.special),
        ("hp", dvs.hp),
    ] {
        if value > 15 {
            return Err(format!(
                "pokemon.dvs.{field} {value} is outside range 0..15"
            ));
        }
    }
    let expected_hp = Dv::from_non_hp(dvs.attack, dvs.defense, dvs.speed, dvs.special).hp;
    if dvs.hp != expected_hp {
        return Err(format!(
            "pokemon.dvs.hp {} does not match derived HP DV {}",
            dvs.hp, expected_hp
        ));
    }
    Ok(())
}

fn validate_base_stats(stats: BaseStats) -> Result<(), String> {
    for (field, value) in [
        ("hp", stats.hp),
        ("attack", stats.attack),
        ("defense", stats.defense),
        ("speed", stats.speed),
        ("special_attack", stats.special_attack),
        ("special_defense", stats.special_defense),
    ] {
        if value == 0 {
            return Err(format!("pokemon.base_stats.{field} must be nonzero"));
        }
    }
    Ok(())
}

fn validate_stat_boosts(stat_boosts: &BTreeMap<Stat, i8>) -> Result<(), String> {
    let expected = default_stat_boosts();
    if stat_boosts.len() != expected.len() {
        return Err(format!(
            "pokemon.stat_boosts has {} entries, expected {}",
            stat_boosts.len(),
            expected.len()
        ));
    }
    for stat in expected.keys() {
        let Some(value) = stat_boosts.get(stat) else {
            return Err(format!("pokemon.stat_boosts is missing {:?}", stat));
        };
        if !(-6..=6).contains(value) {
            return Err(format!(
                "pokemon.stat_boosts.{stat:?} {value} is outside range -6..6"
            ));
        }
    }
    Ok(())
}

fn validate_exact_token(field: &str, value: &str) -> Result<(), String> {
    if value.is_empty() || value.trim() != value {
        return Err(format!("{field} has invalid token '{value}'"));
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b':' | b'.'))
    {
        return Err(format!("{field} has invalid token '{value}'"));
    }
    validate_no_reserved_pokemon_token(value)
        .map_err(|_| format!("{field} token '{value}' uses reserved modpack payload prefix"))?;
    Ok(())
}

fn validate_exact_text(field: &str, value: &str) -> Result<(), String> {
    if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
        return Err(format!("{field} has invalid text '{value}'"));
    }
    Ok(())
}

fn validate_mail_message(value: &str) -> Result<(), String> {
    let mut lines = value.split('\n');
    let first = lines.next().unwrap_or_default();
    let second = lines.next();
    if lines.next().is_some()
        || first.chars().count() > 16
        || second.is_some_and(|line| line.chars().count() > 16)
        || value.chars().filter(|character| *character != '\n').count() > 32
        || value
            .chars()
            .any(|character| character.is_control() && character != '\n')
    {
        return Err(format!("pokemon.mail.message has invalid text '{value}'"));
    }
    Ok(())
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
    let exp_modifier = stat_exp_square_root(stat_exp) / 4;
    let interim_value = (base + dv) * 2 + exp_modifier;
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
    growth_rates: &GrowthRateCatalog,
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
            Ok::<LearnedMove, PokemonBuildError>(LearnedMove {
                name,
                current_pp: move_data.pp,
                pp_ups: 0,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Pokemon {
        species: species.clone(),
        nickname: pokemon_species_display_name(&species.id),
        item: None,
        moves: learned_moves,
        status: None,
        is_egg: false,
        pokerus: 0,
        caught_data: None,
        mail: None,
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
        experience: calculate_experience(growth_rates, &species.growth_rate, level)?,
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

fn stat_exp_square_root(stat_exp: u16) -> u16 {
    let stat_exp = u32::from(stat_exp);
    let floor = isqrt(stat_exp);
    let ceiling = floor + u32::from(floor * floor < stat_exp);
    ceiling.min(255) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chikorita() -> PokemonSpecies {
        let mut species =
            PokemonSpecies::new_for_tests("CHIKORITA", BaseStats::new(45, 49, 65, 45, 49, 65));
        species.int_id = 152;
        species
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
    fn species_type_ids_are_modpack_owned_strings_not_core_enums() {
        let mut species = species_json();
        species["type1"] = serde_json::json!("AETHER");
        species["type2"] = serde_json::json!("VOID");

        let parsed = serde_json::from_value::<PokemonSpecies>(species)
            .expect("modded species type ids are exact data");

        assert_eq!(parsed.type1, pokemon_type("AETHER"));
        assert_eq!(parsed.type2, pokemon_type("VOID"));
    }

    #[test]
    fn species_metadata_ids_are_modpack_owned_strings_not_core_enums() {
        let mut species = species_json();
        species["egg_group1"] = serde_json::json!("EGG_CRYSTAL");
        species["egg_group2"] = serde_json::json!("EGG_ANCIENT");
        species["ability"] = serde_json::json!("SHED_SKIN_PLUS");

        let parsed = serde_json::from_value::<PokemonSpecies>(species)
            .expect("modded species metadata ids are exact data");

        assert_eq!(parsed.egg_group1, egg_group("EGG_CRYSTAL"));
        assert_eq!(parsed.egg_group2, egg_group("EGG_ANCIENT"));
        assert_eq!(parsed.ability, ability("SHED_SKIN_PLUS"));
    }

    #[test]
    fn species_identifier_fields_reject_malformed_tokens_at_deserialization() {
        for (field, value) in [
            ("id", serde_json::json!("BULBA SAUR")),
            ("id", serde_json::json!("fallback_BULBASAUR")),
            ("type1", serde_json::json!(" GRASS")),
            ("type1", serde_json::json!("legacy_GRASS")),
            ("type2", serde_json::json!("POI SON")),
            ("item1", serde_json::json!("MIRACLE SEED")),
            ("item2", serde_json::json!(" POISON_BARB")),
            ("growth_rate", serde_json::json!("GROWTH MEDIUM_SLOW")),
            ("egg_group1", serde_json::json!("EGG MONSTER")),
            ("egg_group2", serde_json::json!("EGG_PLANT ")),
            ("tmhm_learnset", serde_json::json!(["HEADBUTT", "MUD SLAP"])),
            ("ability", serde_json::json!("SHED SKIN_PLUS")),
        ] {
            let mut species = species_json();
            species[field] = value;

            let error = serde_json::from_value::<PokemonSpecies>(species)
                .expect_err("malformed Pokemon species identifiers must fail before runtime use")
                .to_string();

            assert!(
                error.contains("Pokemon token must be")
                    || error.contains("uses reserved modpack payload prefix"),
                "{field} produced unexpected error: {error}"
            );
        }
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
            "pp_ups": 0,
            "move": "Tackle"
        }))
        .expect_err("learned moves must not accept display move aliases")
        .to_string();
        assert!(error.contains("unknown field `move`"), "{error}");

        let error = serde_json::from_value::<LearnedMove>(serde_json::json!({
            "name": "TACKLE",
            "current_pp": 35
        }))
        .expect_err("learned moves must declare PP Up stages explicitly")
        .to_string();
        assert!(error.contains("missing field `pp_ups`"), "{error}");
    }

    #[test]
    fn saved_pokemon_identifier_fields_reject_malformed_tokens_at_deserialization() {
        let mut learned_move = serde_json::json!({
            "name": "MUD SLAP",
            "current_pp": 10,
            "pp_ups": 0
        });
        let error = serde_json::from_value::<LearnedMove>(learned_move.clone())
            .expect_err("learned move names must be exact tokens")
            .to_string();
        assert!(error.contains("Pokemon token must be"), "{error}");

        learned_move["name"] = serde_json::json!("legacy_TACKLE");
        let error = serde_json::from_value::<LearnedMove>(learned_move.clone())
            .expect_err("learned move names must reject reserved payload prefixes")
            .to_string();
        assert!(
            error.contains("uses reserved modpack payload prefix"),
            "{error}"
        );

        learned_move["name"] = serde_json::json!("TACKLE");
        let species: PokemonSpecies = serde_json::from_value(species_json()).expect("species");
        let pokemon = Pokemon::new_for_tests(species, 5, Dv::default());
        let mut pokemon_json = serde_json::to_value(pokemon).expect("pokemon json");

        for (field, value) in [
            ("item", serde_json::json!("MIRACLE SEED")),
            ("item", serde_json::json!("fallback_ITEM")),
            ("status", serde_json::json!("BAD POISON")),
            ("status", serde_json::json!("legacy_STATUS")),
        ] {
            let mut candidate = pokemon_json.clone();
            candidate[field] = value;

            let error = serde_json::from_value::<Pokemon>(candidate)
                .expect_err("saved Pokemon identifier fields must fail before runtime use")
                .to_string();

            assert!(
                error.contains("Pokemon token must be")
                    || error.contains("uses reserved modpack payload prefix"),
                "{field} produced unexpected error: {error}"
            );
        }

        pokemon_json["moves"] = serde_json::json!([learned_move]);
        serde_json::from_value::<Pokemon>(pokemon_json).expect("exact learned move token is valid");
    }

    #[test]
    fn pokemon_saved_state_rejects_impossible_runtime_records() {
        let mut pokemon = Pokemon::new_for_tests(chikorita(), 5, Dv::from_non_hp(1, 2, 3, 4));
        pokemon.validate_saved_state().expect("valid Pokemon");

        pokemon.level = 0;
        assert_eq!(
            pokemon.validate_saved_state(),
            Err("pokemon.level 0 is outside range 1..100".to_string())
        );

        pokemon.level = 5;
        pokemon.hp = pokemon.max_hp + 1;
        assert_eq!(
            pokemon.validate_saved_state(),
            Err(format!(
                "pokemon.hp {} cannot exceed max_hp {}",
                pokemon.hp, pokemon.max_hp
            ))
        );

        pokemon.hp = pokemon.max_hp;
        pokemon.dvs.attack = 16;
        assert_eq!(
            pokemon.validate_saved_state(),
            Err("pokemon.dvs.attack 16 is outside range 0..15".to_string())
        );

        pokemon.dvs.attack = 1;
        pokemon.dvs.hp ^= 1;
        assert_eq!(
            pokemon.validate_saved_state(),
            Err("pokemon.dvs.hp 11 does not match derived HP DV 10".to_string())
        );

        pokemon.dvs = Dv::from_non_hp(1, 2, 3, 4);
        pokemon.attack += 1;
        assert_eq!(
            pokemon.validate_saved_state(),
            Err(format!(
                "pokemon.attack {} does not match calculated stat {}",
                pokemon.attack,
                pokemon.attack - 1
            ))
        );

        pokemon.attack -= 1;
        pokemon.moves.push(LearnedMove {
            name: "THUNDER PUNCH".to_string(),
            current_pp: 15,
            pp_ups: 0,
        });
        assert_eq!(
            pokemon.validate_saved_state(),
            Err("pokemon.moves[0].name has invalid token 'THUNDER PUNCH'".to_string())
        );
    }

    #[test]
    fn pokemon_json_requires_explicit_nullable_item_and_status() {
        let pokemon = Pokemon::new_for_tests(chikorita(), 5, Dv::from_non_hp(10, 10, 10, 10));

        let mut missing_item = serde_json::to_value(&pokemon).expect("pokemon json");
        missing_item
            .as_object_mut()
            .expect("pokemon object")
            .remove("item");
        let item_error = serde_json::from_value::<Pokemon>(missing_item)
            .expect_err("missing held item must not deserialize as None")
            .to_string();
        assert!(item_error.contains("missing field `item`"), "{item_error}");

        let mut missing_status = serde_json::to_value(&pokemon).expect("pokemon json");
        missing_status
            .as_object_mut()
            .expect("pokemon object")
            .remove("status");
        let status_error = serde_json::from_value::<Pokemon>(missing_status)
            .expect_err("missing status must not deserialize as healthy")
            .to_string();
        assert!(
            status_error.contains("missing field `status`"),
            "{status_error}"
        );

        let explicit_nulls = serde_json::from_value::<Pokemon>(
            serde_json::to_value(&pokemon).expect("pokemon json"),
        )
        .expect("explicit null item and status are valid");
        assert_eq!(explicit_nulls.item, None);
        assert_eq!(explicit_nulls.status, None);
    }

    #[test]
    fn pokemon_persists_caught_data_mail_and_pokerus_metadata() {
        let mut pokemon = Pokemon::new_for_tests(chikorita(), 5, Dv::from_non_hp(10, 10, 10, 10));
        pokemon.pokerus = 0xb4;
        pokemon.caught_data = Some(CaughtData {
            level: 5,
            time_of_day: Some(TimeOfDay::Day),
            original_trainer_gender: 0,
            location: 18,
        });
        pokemon.mail = Some(MailData {
            message: "DARK CAVE leads\nto another road".to_string(),
            author: "CHRIS".to_string(),
            nationality: 0,
            author_id: 1234,
            species: "SPEAROW".to_string(),
            mail_type: "FLOWER_MAIL".to_string(),
        });
        pokemon.item = Some("FLOWER_MAIL".to_string());
        pokemon
            .validate_saved_state()
            .expect("source mail line break is valid saved text");
        let encoded = serde_json::to_string(&pokemon).expect("serialize metadata");
        let decoded: Pokemon = serde_json::from_str(&encoded).expect("deserialize metadata");
        assert_eq!(decoded.pokerus, 0xb4);
        assert_eq!(decoded.caught_data, pokemon.caught_data);
        assert_eq!(decoded.mail, pokemon.mail);
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
    fn stat_exp_uses_capped_ceiling_square_root() {
        let species = chikorita();
        let dvs = Dv::from_non_hp(15, 15, 15, 15);
        let attack = calculate_stat(species.base_stats, 100, dvs, 10, Stat::Attack);
        assert_eq!(attack, Some(134));
    }

    #[test]
    fn max_stat_exp_uses_the_same_capped_modifier_for_hp() {
        let species = chikorita();
        let dvs = Dv::from_non_hp(15, 15, 15, 15);
        let maxed = calculate_stat(species.base_stats, 100, dvs, u16::MAX, Stat::Hp);
        assert_eq!(maxed, Some(293));
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
        let growth_rates = crate::systems::experience::crystal_growth_rate_catalog_for_tests();
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
                    source_index: 1,
                    name: "TACKLE".to_string(),
                    move_type: pokemon_type("NORMAL"),
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
                    source_index: 1,
                    name: "GROWL".to_string(),
                    move_type: pokemon_type("NORMAL"),
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
            &growth_rates,
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
    fn pokemon_factory_requires_growth_rate_catalog_entry_without_fallback() {
        let growth_rates = crate::systems::experience::crystal_growth_rate_catalog_for_tests();
        let learnsets = [("CHIKORITA".to_string(), Vec::new())]
            .into_iter()
            .collect();
        let moves = BTreeMap::new();

        let mut species = chikorita();
        species.growth_rate = growth_rate("GROWTH_CUSTOM");
        assert_eq!(
            create_pokemon_from_known_dvs(
                &species,
                5,
                Dv::from_non_hp(10, 10, 10, 10),
                &learnsets,
                &moves,
                &growth_rates,
            ),
            Err(PokemonBuildError::Experience(
                ExperienceError::MissingGrowthRate {
                    growth_rate: "GROWTH_CUSTOM".to_string(),
                }
            ))
        );
    }

    #[test]
    fn pokemon_factory_rejects_missing_learnset_or_move_without_pp_fallback() {
        let growth_rates = crate::systems::experience::crystal_growth_rate_catalog_for_tests();
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
                &growth_rates,
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
                &growth_rates,
            ),
            Err(PokemonBuildError::UnknownLearnsetMove {
                species_id: "CHIKORITA".to_string(),
                move_name: "TACKLE".to_string(),
            })
        );
    }

    #[test]
    fn stat_json_rejects_legacy_alias_payloads() {
        let error = serde_json::from_value::<Stat>(serde_json::json!({
            "ATTACK": {
                "legacy_stat": "ATK"
            }
        }))
        .expect_err("stats must not accept legacy object payloads")
        .to_string();
        assert!(
            error.contains("invalid type") || error.contains("unknown variant"),
            "{error}"
        );
    }

    #[test]
    fn pokemon_build_error_json_rejects_unknown_fallback_fields() {
        let error = serde_json::from_value::<PokemonBuildError>(serde_json::json!({
            "UnknownLearnsetMove": {
                "species_id": "CHIKORITA",
                "move_name": "TACKLE",
                "fallback_move_name": "POUND"
            }
        }))
        .expect_err("Pokemon build errors must not accept fallback move names")
        .to_string();
        assert!(
            error.contains("unknown field `fallback_move_name`"),
            "{error}"
        );
    }
}
