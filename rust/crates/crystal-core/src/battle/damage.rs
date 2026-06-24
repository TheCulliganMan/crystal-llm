use serde::{Deserialize, Serialize};

use crate::battle::stats::apply_stage;
use crate::models::{Move, Pokemon, PokemonType, Stat};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypeMultiplier {
    pub numerator: u16,
    pub denominator: u16,
}

impl TypeMultiplier {
    pub const fn one() -> Self {
        Self {
            numerator: 1,
            denominator: 1,
        }
    }

    pub const fn zero() -> Self {
        Self {
            numerator: 0,
            denominator: 1,
        }
    }

    pub fn multiply(self, other: Self) -> Self {
        if self.numerator == 0 || other.numerator == 0 {
            return Self::zero();
        }
        let numerator = self.numerator * other.numerator;
        let denominator = self.denominator * other.denominator;
        let divisor = gcd(numerator, denominator);
        Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        }
    }

    pub const fn apply_floor(self, value: u16) -> u16 {
        ((value as u32 * self.numerator as u32) / self.denominator as u32) as u16
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Weather {
    None,
    Rain,
    Sun,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DamageContext {
    pub is_critical: bool,
    pub is_confusion_damage: bool,
    pub weather: Weather,
    pub random_roll: u8,
}

impl Default for DamageContext {
    fn default() -> Self {
        Self {
            is_critical: false,
            is_confusion_damage: false,
            weather: Weather::None,
            random_roll: 255,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DamageResult {
    pub damage: u16,
    pub type_multiplier: TypeMultiplier,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamageCalculationError {
    MissingStat { pokemon_id: String, stat: Stat },
    MissingStatStage { pokemon_id: String, stat: Stat },
}

pub fn is_physical_type(move_type: PokemonType) -> bool {
    matches!(
        move_type,
        PokemonType::Normal
            | PokemonType::Fighting
            | PokemonType::Flying
            | PokemonType::Ground
            | PokemonType::Rock
            | PokemonType::Bug
            | PokemonType::Ghost
            | PokemonType::Poison
            | PokemonType::Steel
    )
}

pub fn type_effectiveness(move_type: PokemonType, defender_type: PokemonType) -> TypeMultiplier {
    use PokemonType::*;
    match (move_type, defender_type) {
        (Normal, Rock) | (Normal, Steel) => half(),
        (Normal, Ghost) => TypeMultiplier::zero(),
        (Fire, Fire) | (Fire, Water) | (Fire, Rock) | (Fire, Dragon) => half(),
        (Fire, Grass) | (Fire, Ice) | (Fire, Bug) | (Fire, Steel) => double(),
        (Water, Fire) | (Water, Ground) | (Water, Rock) => double(),
        (Water, Water) | (Water, Grass) | (Water, Dragon) => half(),
        (Grass, Water) | (Grass, Ground) | (Grass, Rock) => double(),
        (Grass, Fire)
        | (Grass, Grass)
        | (Grass, Poison)
        | (Grass, Flying)
        | (Grass, Bug)
        | (Grass, Dragon)
        | (Grass, Steel) => half(),
        (Electric, Water) | (Electric, Flying) => double(),
        (Electric, Grass) | (Electric, Electric) | (Electric, Dragon) => half(),
        (Electric, Ground) => TypeMultiplier::zero(),
        (Ice, Grass) | (Ice, Ground) | (Ice, Flying) | (Ice, Dragon) => double(),
        (Ice, Fire) | (Ice, Water) | (Ice, Ice) | (Ice, Steel) => half(),
        (Fighting, Normal)
        | (Fighting, Ice)
        | (Fighting, Rock)
        | (Fighting, Dark)
        | (Fighting, Steel) => double(),
        (Fighting, Poison) | (Fighting, Flying) | (Fighting, PsychicType) | (Fighting, Bug) => {
            half()
        }
        (Fighting, Ghost) => TypeMultiplier::zero(),
        (Poison, Grass) => double(),
        (Poison, Poison) | (Poison, Ground) | (Poison, Rock) | (Poison, Ghost) => half(),
        (Poison, Steel) => TypeMultiplier::zero(),
        (Ground, Fire)
        | (Ground, Electric)
        | (Ground, Poison)
        | (Ground, Rock)
        | (Ground, Steel) => double(),
        (Ground, Grass) | (Ground, Bug) => half(),
        (Ground, Flying) => TypeMultiplier::zero(),
        (Flying, Grass) | (Flying, Fighting) | (Flying, Bug) => double(),
        (Flying, Electric) | (Flying, Rock) | (Flying, Steel) => half(),
        (PsychicType, Fighting) | (PsychicType, Poison) => double(),
        (PsychicType, PsychicType) | (PsychicType, Steel) => half(),
        (PsychicType, Dark) => TypeMultiplier::zero(),
        (Bug, Grass) | (Bug, PsychicType) | (Bug, Dark) => double(),
        (Bug, Fire)
        | (Bug, Fighting)
        | (Bug, Poison)
        | (Bug, Flying)
        | (Bug, Ghost)
        | (Bug, Steel) => half(),
        (Rock, Fire) | (Rock, Ice) | (Rock, Flying) | (Rock, Bug) => double(),
        (Rock, Fighting) | (Rock, Ground) | (Rock, Steel) => half(),
        (Ghost, PsychicType) | (Ghost, Ghost) => double(),
        (Ghost, Normal) => TypeMultiplier::zero(),
        (Ghost, Dark) | (Ghost, Steel) => half(),
        (Dragon, Dragon) => double(),
        (Dragon, Steel) => half(),
        (Dark, PsychicType) | (Dark, Ghost) => double(),
        (Dark, Fighting) | (Dark, Dark) | (Dark, Steel) => half(),
        (Steel, Ice) | (Steel, Rock) => double(),
        (Steel, Fire) | (Steel, Water) | (Steel, Electric) | (Steel, Steel) => half(),
        _ => TypeMultiplier::one(),
    }
}

pub fn calculate_type_effectiveness_multiplier(
    move_type: PokemonType,
    defender_types: &[PokemonType],
) -> TypeMultiplier {
    defender_types
        .iter()
        .copied()
        .fold(TypeMultiplier::one(), |acc, defender_type| {
            let next = type_effectiveness(move_type, defender_type);
            if next.numerator == 0 {
                TypeMultiplier::zero()
            } else {
                acc.multiply(next)
            }
        })
}

pub fn calculate_damage(
    attacker: &Pokemon,
    defender: &Pokemon,
    move_data: &Move,
    context: DamageContext,
) -> Result<DamageResult, DamageCalculationError> {
    if move_data.power == 0 {
        return Ok(DamageResult {
            damage: 0,
            type_multiplier: TypeMultiplier::one(),
        });
    }

    let physical = is_physical_type(move_data.move_type);
    let attack_stat = if physical {
        Stat::Attack
    } else {
        Stat::SpecialAttack
    };
    let defense_stat = if physical {
        Stat::Defense
    } else {
        Stat::SpecialDefense
    };
    let attack_stage = *attacker.stat_boosts.get(&attack_stat).ok_or_else(|| {
        DamageCalculationError::MissingStatStage {
            pokemon_id: attacker.species.id.clone(),
            stat: attack_stat,
        }
    })?;
    let defense_stage = *defender.stat_boosts.get(&defense_stat).ok_or_else(|| {
        DamageCalculationError::MissingStatStage {
            pokemon_id: defender.species.id.clone(),
            stat: defense_stat,
        }
    })?;

    let base_attack = attacker.calculate_stat(attack_stat).ok_or_else(|| {
        DamageCalculationError::MissingStat {
            pokemon_id: attacker.species.id.clone(),
            stat: attack_stat,
        }
    })?;
    let base_defense = defender.calculate_stat(defense_stat).ok_or_else(|| {
        DamageCalculationError::MissingStat {
            pokemon_id: defender.species.id.clone(),
            stat: defense_stat,
        }
    })?;
    let attack_value = if context.is_critical && defense_stage > attack_stage {
        clamp_stat(base_attack)
    } else {
        clamp_stat(apply_stage(base_attack, attack_stage))
    };
    let defense_value = if context.is_critical && defense_stage > attack_stage {
        clamp_stat(base_defense)
    } else {
        clamp_stat(apply_stage(base_defense, defense_stage))
    }
    .max(1);

    let level_factor = ((2 * attacker.level as u16) / 5) + 2;
    let mut damage = (((level_factor as u32 * move_data.power as u32 * attack_value as u32)
        / defense_value as u32)
        / 50) as u16;
    if context.is_critical {
        damage = damage.saturating_mul(2);
    }
    damage = damage.min(997) + 2;

    damage = match (context.weather, move_data.move_type) {
        (Weather::Rain, PokemonType::Water) | (Weather::Sun, PokemonType::Fire) => {
            ((damage as u32 * 3) / 2) as u16
        }
        (Weather::Rain, PokemonType::Fire) | (Weather::Sun, PokemonType::Water) => damage / 2,
        _ => damage,
    };

    if !context.is_confusion_damage
        && (move_data.move_type == attacker.species.type1
            || move_data.move_type == attacker.species.type2)
    {
        damage = ((damage as u32 * 3) / 2) as u16;
    }

    let defender_types = distinct_defender_types(defender);
    let type_multiplier = if context.is_confusion_damage || move_data.name == "STRUGGLE" {
        TypeMultiplier::one()
    } else {
        calculate_type_effectiveness_multiplier(move_data.move_type, &defender_types)
    };
    if type_multiplier.numerator == 0 {
        return Ok(DamageResult {
            damage: 0,
            type_multiplier,
        });
    }
    damage = type_multiplier.apply_floor(damage);

    let roll = context.random_roll.max(1);
    damage = ((damage as u32 * roll as u32) / 255).max(1) as u16;

    Ok(DamageResult {
        damage,
        type_multiplier,
    })
}

fn distinct_defender_types(defender: &Pokemon) -> Vec<PokemonType> {
    let mut types = vec![defender.species.type1];
    if defender.species.type2 != defender.species.type1 {
        types.push(defender.species.type2);
    }
    types
}

fn clamp_stat(value: u16) -> u16 {
    value.clamp(1, 999)
}

const fn half() -> TypeMultiplier {
    TypeMultiplier {
        numerator: 1,
        denominator: 2,
    }
}

const fn double() -> TypeMultiplier {
    TypeMultiplier {
        numerator: 2,
        denominator: 1,
    }
}

const fn gcd(mut a: u16, mut b: u16) -> u16 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::models::{BaseStats, Dv, GrowthRate, PokemonSpecies, create_pokemon_from_known_dvs};
    fn species(id: &str, pokemon_type: PokemonType, stats: BaseStats) -> PokemonSpecies {
        let mut species = PokemonSpecies::new_for_tests(id, stats);
        species.type1 = pokemon_type;
        species.type2 = pokemon_type;
        species.growth_rate = GrowthRate::MediumFast;
        species
    }

    fn pokemon(id: &str, pokemon_type: PokemonType, stats: BaseStats, level: u8) -> Pokemon {
        let learnsets = [(id.to_string(), Vec::new())].into_iter().collect();
        create_pokemon_from_known_dvs(
            &species(id, pokemon_type, stats),
            level,
            Dv::from_non_hp(10, 10, 10, 10),
            &learnsets,
            &BTreeMap::new(),
        )
        .expect("test Pokemon builds from explicit empty learnset")
    }

    fn tackle(move_type: PokemonType, power: u16) -> Move {
        Move {
            name: "TACKLE".to_string(),
            move_type,
            power,
            accuracy: 100,
            pp: 35,
            effect: "NORMAL_HIT".to_string(),
            effect_chance: 0,
            stat: None,
            amount: None,
        }
    }

    #[test]
    fn gen_two_type_chart_includes_steel_dark_and_immunities() {
        assert_eq!(
            calculate_type_effectiveness_multiplier(PokemonType::Ghost, &[PokemonType::Steel]),
            TypeMultiplier {
                numerator: 1,
                denominator: 2
            }
        );
        assert_eq!(
            calculate_type_effectiveness_multiplier(PokemonType::Dark, &[PokemonType::Steel]),
            TypeMultiplier {
                numerator: 1,
                denominator: 2
            }
        );
        assert_eq!(
            calculate_type_effectiveness_multiplier(PokemonType::Electric, &[PokemonType::Ground]),
            TypeMultiplier::zero()
        );
        assert_eq!(
            calculate_type_effectiveness_multiplier(
                PokemonType::Ice,
                &[PokemonType::Grass, PokemonType::Flying]
            ),
            TypeMultiplier {
                numerator: 4,
                denominator: 1
            }
        );
    }

    #[test]
    fn physical_type_split_matches_gen_two() {
        assert!(is_physical_type(PokemonType::Ghost));
        assert!(is_physical_type(PokemonType::Steel));
        assert!(!is_physical_type(PokemonType::Fire));
        assert!(!is_physical_type(PokemonType::Dark));
    }

    #[test]
    fn damage_applies_stab_type_multiplier_and_random_roll_deterministically() {
        let attacker = pokemon(
            "ATTACKER",
            PokemonType::Fire,
            BaseStats::new(80, 84, 78, 100, 109, 85),
            50,
        );
        let defender = pokemon(
            "DEFENDER",
            PokemonType::Grass,
            BaseStats::new(80, 82, 83, 80, 100, 100),
            50,
        );
        let result = calculate_damage(
            &attacker,
            &defender,
            &tackle(PokemonType::Fire, 60),
            DamageContext::default(),
        )
        .expect("damage calculates");

        assert_eq!(
            result.type_multiplier,
            TypeMultiplier {
                numerator: 2,
                denominator: 1
            }
        );
        assert_eq!(result.damage, 90);
    }

    #[test]
    fn zero_effectiveness_returns_zero_damage() {
        let attacker = pokemon(
            "ATTACKER",
            PokemonType::Electric,
            BaseStats::new(35, 55, 40, 90, 50, 50),
            30,
        );
        let defender = pokemon(
            "DEFENDER",
            PokemonType::Ground,
            BaseStats::new(50, 50, 95, 35, 40, 50),
            30,
        );

        let result = calculate_damage(
            &attacker,
            &defender,
            &tackle(PokemonType::Electric, 40),
            DamageContext::default(),
        )
        .expect("damage calculates");

        assert_eq!(result.damage, 0);
        assert_eq!(result.type_multiplier, TypeMultiplier::zero());
    }

    #[test]
    fn damage_requires_explicit_battle_stat_stages_without_zero_fallback() {
        let mut attacker = pokemon(
            "ATTACKER",
            PokemonType::Normal,
            BaseStats::new(80, 84, 78, 100, 109, 85),
            50,
        );
        let defender = pokemon(
            "DEFENDER",
            PokemonType::Normal,
            BaseStats::new(80, 82, 83, 80, 100, 100),
            50,
        );
        attacker.stat_boosts.remove(&Stat::Attack);

        let error = calculate_damage(
            &attacker,
            &defender,
            &tackle(PokemonType::Normal, 60),
            DamageContext::default(),
        )
        .expect_err("missing attack stage must not default to zero");

        assert_eq!(
            error,
            DamageCalculationError::MissingStatStage {
                pokemon_id: "ATTACKER".to_string(),
                stat: Stat::Attack,
            }
        );
    }
}
