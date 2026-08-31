use crate::battle::damage::Weather;
use crate::models::{Pokemon, Stat};

/// Every primary ability assigned by the pinned Emerald National Dex data.
///
/// Keep this catalog exhaustive: the Generation III pack verifier compares
/// its generated assignments with this runtime-owned list, so a data-only
/// ability cannot silently ship without an explicit mechanic implementation.
pub const SUPPORTED_GEN3_ABILITIES: &[&str] = &[
    "AIR_LOCK",
    "BATTLE_ARMOR",
    "BLAZE",
    "CHLOROPHYLL",
    "CLEAR_BODY",
    "COLOR_CHANGE",
    "COMPOUND_EYES",
    "CUTE_CHARM",
    "DAMP",
    "DRIZZLE",
    "DROUGHT",
    "EARLY_BIRD",
    "EFFECT_SPORE",
    "FLAME_BODY",
    "FLASH_FIRE",
    "FORECAST",
    "GUTS",
    "HUSTLE",
    "HYPER_CUTTER",
    "ILLUMINATE",
    "IMMUNITY",
    "INNER_FOCUS",
    "INSOMNIA",
    "INTIMIDATE",
    "KEEN_EYE",
    "LEVITATE",
    "LIGHTNING_ROD",
    "LIMBER",
    "LIQUID_OOZE",
    "MAGMA_ARMOR",
    "MAGNET_PULL",
    "MARVEL_SCALE",
    "MINUS",
    "NATURAL_CURE",
    "OBLIVIOUS",
    "OVERGROW",
    "OWN_TEMPO",
    "PICKUP",
    "PLUS",
    "POISON_POINT",
    "PRESSURE",
    "PURE_POWER",
    "ROCK_HEAD",
    "ROUGH_SKIN",
    "RUN_AWAY",
    "SAND_STREAM",
    "SAND_VEIL",
    "SERENE_GRACE",
    "SHADOW_TAG",
    "SHED_SKIN",
    "SHELL_ARMOR",
    "SHIELD_DUST",
    "SOUNDPROOF",
    "SPEED_BOOST",
    "STATIC",
    "STENCH",
    "STURDY",
    "SUCTION_CUPS",
    "SWARM",
    "SWIFT_SWIM",
    "SYNCHRONIZE",
    "THICK_FAT",
    "TORRENT",
    "TRACE",
    "TRUANT",
    "VITAL_SPIRIT",
    "VOLT_ABSORB",
    "WATER_ABSORB",
    "WATER_VEIL",
    "WHITE_SMOKE",
    "WONDER_GUARD",
];

/// These abilities have no observable effect in a Generation III single
/// battle. Lightning Rod only redirects a partner's target, while Plus and
/// Minus require an allied battler. The runtime currently exposes singles,
/// so doing nothing is their canonical behavior rather than a missing hook.
pub const GEN3_SINGLE_BATTLE_NO_EFFECT_ABILITIES: &[&str] = &["LIGHTNING_ROD", "MINUS", "PLUS"];

pub fn ability_blocks_critical_hit(ability: &str) -> bool {
    matches!(ability, "BATTLE_ARMOR" | "SHELL_ARMOR")
}

pub fn ability_suppresses_weather(ability: &str) -> bool {
    ability == "AIR_LOCK"
}

pub fn effective_weather(weather: Weather, attacker: &Pokemon, defender: &Pokemon) -> Weather {
    if ability_suppresses_weather(&attacker.species.ability)
        || ability_suppresses_weather(&defender.species.ability)
    {
        Weather::None
    } else {
        weather
    }
}

pub fn low_hp_boosted_type(ability: &str) -> Option<&'static str> {
    match ability {
        "BLAZE" => Some("FIRE"),
        "OVERGROW" => Some("GRASS"),
        "SWARM" => Some("BUG"),
        "TORRENT" => Some("WATER"),
        _ => None,
    }
}

pub fn has_ground_immunity(ability: &str) -> bool {
    ability == "LEVITATE"
}

pub fn absorbs_move_type(ability: &str, move_type: &str) -> bool {
    matches!(
        (ability, move_type),
        ("FLASH_FIRE", "FIRE") | ("VOLT_ABSORB", "ELECTRIC") | ("WATER_ABSORB", "WATER")
    )
}

pub fn has_wonder_guard(ability: &str) -> bool {
    ability == "WONDER_GUARD"
}

pub fn has_thick_fat(ability: &str) -> bool {
    ability == "THICK_FAT"
}

pub fn physical_attack_multiplier(pokemon: &Pokemon) -> (u16, u16) {
    match pokemon.species.ability.as_str() {
        "PURE_POWER" => (2, 1),
        "GUTS" if pokemon.status.is_some() => (3, 2),
        "HUSTLE" => (3, 2),
        _ => (1, 1),
    }
}

pub fn physical_defense_multiplier(pokemon: &Pokemon) -> (u16, u16) {
    if pokemon.species.ability == "MARVEL_SCALE" && pokemon.status.is_some() {
        (3, 2)
    } else {
        (1, 1)
    }
}

pub fn guts_ignores_burn_penalty(pokemon: &Pokemon) -> bool {
    pokemon.species.ability == "GUTS" && pokemon.status.is_some()
}

pub fn ability_blocks_status(ability: &str, status: &str) -> bool {
    matches!(
        (ability, status),
        ("IMMUNITY", "POISON" | "BAD_POISON")
            | ("INSOMNIA" | "VITAL_SPIRIT", "SLEEP")
            | ("LIMBER", "PARALYSIS")
            | ("MAGMA_ARMOR", "FREEZE")
            | ("WATER_VEIL", "BURN")
    )
}

pub fn ability_blocks_confusion(ability: &str) -> bool {
    ability == "OWN_TEMPO"
}

pub fn ability_blocks_flinching(ability: &str) -> bool {
    ability == "INNER_FOCUS"
}

pub fn ability_blocks_attraction(ability: &str) -> bool {
    ability == "OBLIVIOUS"
}

pub fn ability_blocks_stat_drop(ability: &str, stat: Stat) -> bool {
    matches!(ability, "CLEAR_BODY" | "WHITE_SMOKE")
        || (ability == "HYPER_CUTTER" && stat == Stat::Attack)
        || (ability == "KEEN_EYE" && stat == Stat::Accuracy)
}

pub fn weather_speed_multiplier(ability: &str, weather: Weather) -> u16 {
    match (ability, weather) {
        ("CHLOROPHYLL", Weather::Sun) | ("SWIFT_SWIM", Weather::Rain) => 2,
        _ => 1,
    }
}

pub fn secondary_effect_chance(ability: &str, base_percent: u8) -> u8 {
    if ability == "SERENE_GRACE" {
        base_percent.saturating_mul(2).min(100)
    } else {
        base_percent.min(100)
    }
}

pub fn blocks_opposing_secondary_effects(ability: &str) -> bool {
    ability == "SHIELD_DUST"
}

pub fn ability_accuracy_ratio(
    attacker_ability: &str,
    defender_ability: &str,
    move_type: &str,
    weather: Weather,
) -> (u16, u16) {
    let mut numerator = 1_u16;
    let mut denominator = 1_u16;
    if attacker_ability == "COMPOUND_EYES" {
        numerator *= 13;
        denominator *= 10;
    }
    if attacker_ability == "HUSTLE" && is_physical_move_type(move_type) {
        numerator *= 4;
        denominator *= 5;
    }
    if defender_ability == "SAND_VEIL" && weather == Weather::Sandstorm {
        numerator *= 4;
        denominator *= 5;
    }
    (numerator, denominator)
}

pub fn ability_traps_opponent(
    source_ability: &str,
    target_ability: &str,
    target_types: &[String],
) -> bool {
    match source_ability {
        "SHADOW_TAG" => target_ability != "SHADOW_TAG",
        "MAGNET_PULL" => target_types.iter().any(|type_id| type_id == "STEEL"),
        _ => false,
    }
}

pub fn is_sound_move(move_name: &str) -> bool {
    matches!(
        move_name,
        "GRASS_WHISTLE"
            | "GROWL"
            | "HYPER_VOICE"
            | "METAL_SOUND"
            | "ROAR"
            | "SCREECH"
            | "SING"
            | "SNORE"
            | "SUPERSONIC"
            | "UPROAR"
    )
}

fn is_physical_move_type(move_type: &str) -> bool {
    matches!(
        move_type,
        "NORMAL" | "FIGHTING" | "FLYING" | "POISON" | "GROUND" | "ROCK" | "BUG" | "GHOST" | "STEEL"
    )
}

pub fn move_makes_contact(move_name: &str) -> bool {
    matches!(
        move_name,
        "BIDE"
            | "BIND"
            | "BITE"
            | "BODY_SLAM"
            | "CLAMP"
            | "COMET_PUNCH"
            | "CONSTRICT"
            | "COUNTER"
            | "CRABHAMMER"
            | "CROSS_CHOP"
            | "CRUNCH"
            | "CUT"
            | "DIG"
            | "DIZZY_PUNCH"
            | "DOUBLE_EDGE"
            | "DOUBLE_KICK"
            | "DRILL_PECK"
            | "FALSE_SWIPE"
            | "FIRE_PUNCH"
            | "FLAIL"
            | "FLAME_WHEEL"
            | "FLY"
            | "FRUSTRATION"
            | "FURY_ATTACK"
            | "FURY_CUTTER"
            | "FURY_SWIPES"
            | "GUILLOTINE"
            | "HEADBUTT"
            | "HI_JUMP_KICK"
            | "HORN_ATTACK"
            | "HORN_DRILL"
            | "HYPER_FANG"
            | "ICE_PUNCH"
            | "IRON_TAIL"
            | "JUMP_KICK"
            | "KARATE_CHOP"
            | "LEECH_LIFE"
            | "LICK"
            | "LOW_KICK"
            | "MACH_PUNCH"
            | "MEGAHORN"
            | "MEGA_KICK"
            | "MEGA_PUNCH"
            | "METAL_CLAW"
            | "OUTRAGE"
            | "PECK"
            | "PETAL_DANCE"
            | "POUND"
            | "PURSUIT"
            | "QUICK_ATTACK"
            | "RAGE"
            | "RAPID_SPIN"
            | "RETURN"
            | "REVERSAL"
            | "ROCK_SMASH"
            | "ROLLOUT"
            | "ROLLING_KICK"
            | "SCRATCH"
            | "SEISMIC_TOSS"
            | "SLAM"
            | "SLASH"
            | "SPARK"
            | "STEEL_WING"
            | "STOMP"
            | "STRENGTH"
            | "STRUGGLE"
            | "SUBMISSION"
            | "SUPER_FANG"
            | "TACKLE"
            | "TAKE_DOWN"
            | "THIEF"
            | "THRASH"
            | "TRIPLE_KICK"
            | "VINE_WHIP"
            | "VITAL_THROW"
            | "WATERFALL"
            | "WING_ATTACK"
            | "WRAP"
    )
}

pub fn move_targets_opponent(move_name: &str) -> bool {
    !matches!(
        move_name,
        "ACID_ARMOR"
            | "AGILITY"
            | "AMNESIA"
            | "BARRIER"
            | "BATON_PASS"
            | "BELLY_DRUM"
            | "BIDE"
            | "CONVERSION"
            | "DEFENSE_CURL"
            | "DETECT"
            | "DESTINY_BOND"
            | "DOUBLE_TEAM"
            | "ENDURE"
            | "FOCUS_ENERGY"
            | "GROWTH"
            | "HARDEN"
            | "HAZE"
            | "HEAL_BELL"
            | "LIGHT_SCREEN"
            | "MEDITATE"
            | "MILK_DRINK"
            | "MINIMIZE"
            | "MIST"
            | "MOONLIGHT"
            | "MORNING_SUN"
            | "PERISH_SONG"
            | "PROTECT"
            | "RAIN_DANCE"
            | "RECOVER"
            | "REFLECT"
            | "REST"
            | "SAFEGUARD"
            | "SANDSTORM"
            | "SHARPEN"
            | "SPLASH"
            | "SUBSTITUTE"
            | "SUNNY_DAY"
            | "SWORDS_DANCE"
            | "SYNTHESIS"
            | "TELEPORT"
            | "WITHDRAW"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BaseStats, Dv, PokemonSpecies, pokemon_type};

    fn pokemon_with_ability(ability: &str) -> Pokemon {
        let mut species =
            PokemonSpecies::new_for_tests("TESTMON", BaseStats::new(50, 50, 50, 50, 50, 50));
        species.type1 = pokemon_type("NORMAL");
        species.type2 = pokemon_type("NORMAL");
        species.ability = ability.to_string();
        Pokemon::new_for_tests(species, 20, Dv::from_non_hp(10, 10, 10, 10))
    }

    #[test]
    fn supported_catalog_is_sorted_and_unique() {
        assert_eq!(SUPPORTED_GEN3_ABILITIES.len(), 71);
        assert!(
            SUPPORTED_GEN3_ABILITIES
                .windows(2)
                .all(|pair| pair[0] < pair[1])
        );
        assert!(
            GEN3_SINGLE_BATTLE_NO_EFFECT_ABILITIES
                .iter()
                .all(|ability| SUPPORTED_GEN3_ABILITIES.contains(ability))
        );
    }

    #[test]
    fn defensive_immunity_and_prevention_families_are_exact() {
        assert!(ability_blocks_critical_hit("BATTLE_ARMOR"));
        assert!(ability_blocks_critical_hit("SHELL_ARMOR"));
        assert!(ability_suppresses_weather("AIR_LOCK"));
        assert!(has_ground_immunity("LEVITATE"));
        assert!(has_wonder_guard("WONDER_GUARD"));
        assert!(has_thick_fat("THICK_FAT"));
        assert!(absorbs_move_type("FLASH_FIRE", "FIRE"));
        assert!(absorbs_move_type("VOLT_ABSORB", "ELECTRIC"));
        assert!(absorbs_move_type("WATER_ABSORB", "WATER"));
        assert!(ability_blocks_confusion("OWN_TEMPO"));
        assert!(ability_blocks_flinching("INNER_FOCUS"));
        assert!(ability_blocks_attraction("OBLIVIOUS"));
        for (ability, status) in [
            ("IMMUNITY", "POISON"),
            ("IMMUNITY", "BAD_POISON"),
            ("INSOMNIA", "SLEEP"),
            ("VITAL_SPIRIT", "SLEEP"),
            ("LIMBER", "PARALYSIS"),
            ("MAGMA_ARMOR", "FREEZE"),
            ("WATER_VEIL", "BURN"),
        ] {
            assert!(ability_blocks_status(ability, status), "{ability}/{status}");
        }
    }

    #[test]
    fn damage_stat_weather_and_accuracy_families_are_exact() {
        for (ability, move_type) in [
            ("BLAZE", "FIRE"),
            ("OVERGROW", "GRASS"),
            ("SWARM", "BUG"),
            ("TORRENT", "WATER"),
        ] {
            assert_eq!(low_hp_boosted_type(ability), Some(move_type));
        }
        let mut guts = pokemon_with_ability("GUTS");
        guts.status = Some("BURN".to_string());
        assert_eq!(physical_attack_multiplier(&guts), (3, 2));
        assert!(guts_ignores_burn_penalty(&guts));
        assert_eq!(
            physical_attack_multiplier(&pokemon_with_ability("PURE_POWER")),
            (2, 1)
        );
        assert_eq!(
            physical_attack_multiplier(&pokemon_with_ability("HUSTLE")),
            (3, 2)
        );
        let mut marvel_scale = pokemon_with_ability("MARVEL_SCALE");
        marvel_scale.status = Some("POISON".to_string());
        assert_eq!(physical_defense_multiplier(&marvel_scale), (3, 2));
        assert_eq!(weather_speed_multiplier("CHLOROPHYLL", Weather::Sun), 2);
        assert_eq!(weather_speed_multiplier("SWIFT_SWIM", Weather::Rain), 2);
        assert_eq!(
            ability_accuracy_ratio("COMPOUND_EYES", "NONE", "WATER", Weather::None),
            (13, 10)
        );
        assert_eq!(
            ability_accuracy_ratio("HUSTLE", "NONE", "NORMAL", Weather::None),
            (4, 5)
        );
        assert_eq!(
            ability_accuracy_ratio("NONE", "SAND_VEIL", "WATER", Weather::Sandstorm),
            (4, 5)
        );
        assert_eq!(secondary_effect_chance("SERENE_GRACE", 30), 60);
        assert!(blocks_opposing_secondary_effects("SHIELD_DUST"));
    }

    #[test]
    fn stat_drop_and_trapping_families_are_exact() {
        assert!(ability_blocks_stat_drop("CLEAR_BODY", Stat::Speed));
        assert!(ability_blocks_stat_drop("WHITE_SMOKE", Stat::Defense));
        assert!(ability_blocks_stat_drop("HYPER_CUTTER", Stat::Attack));
        assert!(!ability_blocks_stat_drop("HYPER_CUTTER", Stat::Defense));
        assert!(ability_blocks_stat_drop("KEEN_EYE", Stat::Accuracy));
        assert!(ability_traps_opponent(
            "SHADOW_TAG",
            "PRESSURE",
            &["NORMAL".to_string()]
        ));
        assert!(!ability_traps_opponent(
            "SHADOW_TAG",
            "SHADOW_TAG",
            &["NORMAL".to_string()]
        ));
        assert!(ability_traps_opponent(
            "MAGNET_PULL",
            "PRESSURE",
            &["STEEL".to_string()]
        ));
    }

    #[test]
    fn generation_three_sound_contact_and_pressure_move_tables_are_wired() {
        assert!(is_sound_move("HYPER_VOICE"));
        assert!(is_sound_move("PERISH_SONG") == false);
        assert!(move_makes_contact("TACKLE"));
        assert!(!move_makes_contact("SURF"));
        assert!(move_targets_opponent("TACKLE"));
        assert!(!move_targets_opponent("HEAL_BELL"));
    }
}
