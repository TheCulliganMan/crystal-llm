use serde::{Deserialize, Serialize};

use crate::models::{Party, Pokemon};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepEventRules {
    pub poison_step_interval: u8,
    pub egg_step_trigger: u8,
    pub hatched_egg_happiness: u8,
    pub poison_status: String,
    pub egg_nickname: String,
    pub happiness_step_counter_mask: u8,
    pub happiness_step_counter_target: u8,
}

impl Default for StepEventRules {
    fn default() -> Self {
        Self {
            poison_step_interval: 0,
            egg_step_trigger: 0,
            hatched_egg_happiness: 0,
            poison_status: String::new(),
            egg_nickname: String::new(),
            happiness_step_counter_mask: 0,
            happiness_step_counter_target: 0,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepEventCounters {
    pub step_count: u8,
    pub poison_step_count: u8,
    pub happiness_step_count: u8,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepEventResult {
    pub egg_hatched: bool,
    pub hatched_species: Option<String>,
    pub poison_result: Option<PoisonDamageResult>,
    pub happiness_changed: Vec<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PoisonDamageResult {
    pub damaged_names: Vec<String>,
    pub fainted_names: Vec<String>,
}

pub fn process_step(
    rules: &StepEventRules,
    counters: &mut StepEventCounters,
    party: &mut Party,
) -> StepEventResult {
    counters.poison_step_count = counters.poison_step_count.wrapping_add(1);
    counters.step_count = counters.step_count.wrapping_add(1);

    let mut happiness_changed = Vec::new();
    if counters.step_count == 0 {
        happiness_changed = apply_happiness_step(rules, counters, party);
    }

    if counters.step_count == rules.egg_step_trigger {
        if let Some(hatched_species) = process_egg_step(rules, party) {
            return StepEventResult {
                egg_hatched: true,
                hatched_species: Some(hatched_species),
                poison_result: None,
                happiness_changed,
            };
        }
    }

    let poison_result = process_poison_step(rules, counters, party);
    StepEventResult {
        egg_hatched: false,
        hatched_species: None,
        poison_result,
        happiness_changed,
    }
}

pub fn apply_happiness_step(
    rules: &StepEventRules,
    counters: &mut StepEventCounters,
    party: &mut Party,
) -> Vec<String> {
    counters.happiness_step_count =
        (counters.happiness_step_count.wrapping_add(1)) & rules.happiness_step_counter_mask;
    if counters.happiness_step_count != rules.happiness_step_counter_target {
        return Vec::new();
    }

    let mut changed = Vec::new();
    for pokemon in party.pokemon.iter_mut().flatten() {
        if is_egg(rules, pokemon) {
            continue;
        }
        let before = pokemon.happiness;
        pokemon.happiness = pokemon.happiness.saturating_add(1);
        if pokemon.happiness != before {
            changed.push(pokemon_event_name(pokemon));
        }
    }
    changed
}

pub fn process_egg_step(rules: &StepEventRules, party: &mut Party) -> Option<String> {
    for pokemon in party.pokemon.iter_mut().flatten() {
        if !is_egg(rules, pokemon) {
            continue;
        }
        pokemon.happiness = pokemon.happiness.wrapping_sub(1);
        if pokemon.happiness != 0 {
            return None;
        }
        let species_id = pokemon.species.id.clone();
        pokemon.nickname = species_id.clone();
        pokemon.happiness = rules.hatched_egg_happiness;
        return Some(species_id);
    }
    None
}

pub fn process_poison_step(
    rules: &StepEventRules,
    counters: &mut StepEventCounters,
    party: &mut Party,
) -> Option<PoisonDamageResult> {
    if counters.poison_step_count < rules.poison_step_interval {
        return None;
    }
    counters.poison_step_count = 0;

    let poisoned_before_step: Vec<usize> = party
        .pokemon
        .iter()
        .enumerate()
        .filter_map(|(index, pokemon)| {
            let pokemon = pokemon.as_ref()?;
            (is_poisoned(rules, pokemon) && pokemon.hp > 0).then_some(index)
        })
        .collect();

    let mut result = apply_poison_to_party(rules, party);
    apply_poison_faint_happiness(party, poisoned_before_step);
    if result.damaged_names.is_empty() && result.fainted_names.is_empty() {
        None
    } else {
        result.damaged_names.shrink_to_fit();
        result.fainted_names.shrink_to_fit();
        Some(result)
    }
}

pub fn apply_poison_to_party(rules: &StepEventRules, party: &mut Party) -> PoisonDamageResult {
    let mut result = PoisonDamageResult::default();
    for pokemon in party.pokemon.iter_mut().flatten() {
        if !is_poisoned(rules, pokemon) || pokemon.hp == 0 {
            continue;
        }
        pokemon.hp = pokemon.hp.saturating_sub(1);
        if pokemon.hp == 0 {
            pokemon.status = None;
            result.fainted_names.push(pokemon_event_name(pokemon));
        } else {
            result.damaged_names.push(pokemon_event_name(pokemon));
        }
    }
    result
}

pub fn is_poisoned(rules: &StepEventRules, pokemon: &Pokemon) -> bool {
    pokemon.status.as_deref() == Some(rules.poison_status.as_str())
}

pub fn is_egg(rules: &StepEventRules, pokemon: &Pokemon) -> bool {
    pokemon.nickname == rules.egg_nickname
}

fn apply_poison_faint_happiness(party: &mut Party, poisoned_before_step: Vec<usize>) {
    for index in poisoned_before_step {
        let Some(pokemon) = party.pokemon[index].as_mut() else {
            continue;
        };
        if pokemon.hp > 0 {
            continue;
        }
        pokemon.happiness = pokemon
            .happiness
            .saturating_sub(poison_faint_happiness_delta(pokemon.happiness));
    }
}

fn poison_faint_happiness_delta(happiness: u8) -> u8 {
    if happiness < 200 { 5 } else { 10 }
}

fn pokemon_event_name(pokemon: &Pokemon) -> String {
    if !pokemon.nickname.is_empty() {
        pokemon.nickname.clone()
    } else {
        pokemon.species.id.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BaseStats, Dv, PokemonSpecies};

    fn rules() -> StepEventRules {
        StepEventRules {
            poison_step_interval: 4,
            egg_step_trigger: 0x80,
            hatched_egg_happiness: 0x78,
            poison_status: "POISON".to_string(),
            egg_nickname: "EGG".to_string(),
            happiness_step_counter_mask: 1,
            happiness_step_counter_target: 0,
        }
    }

    fn pokemon(id: &str) -> Pokemon {
        Pokemon::new_for_tests(
            PokemonSpecies::new_for_tests(id, BaseStats::new(45, 49, 49, 45, 65, 65)),
            12,
            Dv::from_non_hp(1, 2, 3, 4),
        )
    }

    fn party_with(entries: Vec<(usize, Pokemon)>) -> Party {
        let mut party = Party::default();
        for (slot, pokemon) in entries {
            party.pokemon[slot] = Some(pokemon);
        }
        party
    }

    #[test]
    fn poison_damage_applies_every_four_steps_to_exact_poison_status() {
        let mut oddish = pokemon("ODDISH");
        oddish.hp = 3;
        oddish.status = Some(rules().poison_status);
        let mut party = party_with(vec![(0, oddish)]);
        let mut counters = StepEventCounters::default();

        for _ in 0..3 {
            assert_eq!(
                process_step(&rules(), &mut counters, &mut party).poison_result,
                None
            );
        }

        let result = process_step(&rules(), &mut counters, &mut party);
        assert_eq!(
            result.poison_result,
            Some(PoisonDamageResult {
                damaged_names: vec!["ODDISH".to_string()],
                fainted_names: Vec::new(),
            })
        );
        assert_eq!(party.pokemon[0].as_ref().expect("pokemon").hp, 2);
        assert_eq!(counters.poison_step_count, 0);
    }

    #[test]
    fn poison_status_is_exact_not_lowercase_or_alias_coerced() {
        let mut grimer = pokemon("GRIMER");
        grimer.hp = 4;
        grimer.status = Some("poison".to_string());
        let mut party = party_with(vec![(0, grimer)]);
        let mut counters = StepEventCounters {
            poison_step_count: 3,
            ..StepEventCounters::default()
        };

        let result = process_step(&rules(), &mut counters, &mut party);
        assert_eq!(result.poison_result, None);
        assert_eq!(party.pokemon[0].as_ref().expect("pokemon").hp, 4);
    }

    #[test]
    fn poison_faint_clears_status_and_reduces_happiness() {
        let mut oddish = pokemon("ODDISH");
        oddish.hp = 1;
        oddish.happiness = 210;
        oddish.status = Some(rules().poison_status);
        let mut party = party_with(vec![(0, oddish)]);
        let mut counters = StepEventCounters {
            poison_step_count: 3,
            ..StepEventCounters::default()
        };

        let result = process_step(&rules(), &mut counters, &mut party);
        assert_eq!(
            result.poison_result,
            Some(PoisonDamageResult {
                damaged_names: Vec::new(),
                fainted_names: vec!["ODDISH".to_string()],
            })
        );
        let pokemon = party.pokemon[0].as_ref().expect("pokemon");
        assert_eq!(pokemon.hp, 0);
        assert_eq!(pokemon.status, None);
        assert_eq!(pokemon.happiness, 200);
    }

    #[test]
    fn egg_step_hatches_only_when_counter_decrements_to_zero() {
        let mut egg = pokemon("TOGEPI");
        egg.nickname = rules().egg_nickname;
        egg.happiness = 1;
        let mut party = party_with(vec![(0, egg)]);
        let mut counters = StepEventCounters {
            step_count: 0x7f,
            ..StepEventCounters::default()
        };

        let result = process_step(&rules(), &mut counters, &mut party);
        assert_eq!(result.egg_hatched, true);
        assert_eq!(result.hatched_species, Some("TOGEPI".to_string()));
        let pokemon = party.pokemon[0].as_ref().expect("pokemon");
        assert_eq!(pokemon.nickname, "TOGEPI");
        assert_eq!(pokemon.happiness, rules().hatched_egg_happiness);
    }

    #[test]
    fn egg_step_wraps_counter_and_stops_at_first_egg() {
        let mut first = pokemon("TOGEPI");
        first.nickname = rules().egg_nickname;
        first.happiness = 0;
        let mut second = pokemon("PICHU");
        second.nickname = rules().egg_nickname;
        second.happiness = 2;
        let mut party = party_with(vec![(0, first), (1, second)]);
        let mut counters = StepEventCounters {
            step_count: 0x7f,
            ..StepEventCounters::default()
        };

        let result = process_step(&rules(), &mut counters, &mut party);
        assert_eq!(result.egg_hatched, false);
        assert_eq!(party.pokemon[0].as_ref().expect("first").happiness, 0xff);
        assert_eq!(party.pokemon[1].as_ref().expect("second").happiness, 2);
    }

    #[test]
    fn egg_hatch_skips_poison_in_count_step_ordering() {
        let mut egg = pokemon("TOGEPI");
        egg.nickname = rules().egg_nickname;
        egg.happiness = 1;
        let mut oddish = pokemon("ODDISH");
        oddish.hp = 3;
        oddish.status = Some(rules().poison_status);
        let mut party = party_with(vec![(0, egg), (1, oddish)]);
        let mut counters = StepEventCounters {
            step_count: 0x7f,
            poison_step_count: 3,
            ..StepEventCounters::default()
        };

        let result = process_step(&rules(), &mut counters, &mut party);
        assert_eq!(result.egg_hatched, true);
        assert_eq!(result.poison_result, None);
        assert_eq!(party.pokemon[1].as_ref().expect("poisoned").hp, 3);
        assert_eq!(counters.poison_step_count, 4);
    }

    #[test]
    fn happiness_step_runs_every_512_steps_and_skips_eggs() {
        let mut chikorita = pokemon("CHIKORITA");
        chikorita.happiness = 70;
        let mut egg = pokemon("TOGEPI");
        egg.nickname = rules().egg_nickname;
        egg.happiness = 70;
        let mut party = party_with(vec![(0, chikorita), (1, egg)]);
        let mut counters = StepEventCounters {
            step_count: 0xff,
            happiness_step_count: 1,
            ..StepEventCounters::default()
        };

        let result = process_step(&rules(), &mut counters, &mut party);
        assert_eq!(result.happiness_changed, vec!["CHIKORITA".to_string()]);
        assert_eq!(party.pokemon[0].as_ref().expect("mon").happiness, 71);
        assert_eq!(party.pokemon[1].as_ref().expect("egg").happiness, 70);
    }
}
