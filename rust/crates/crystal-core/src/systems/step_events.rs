use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::models::{Party, Pokemon};
use crate::state::GameState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

impl<'de> Deserialize<'de> for StepEventRules {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawStepEventRules {
            poison_step_interval: u8,
            egg_step_trigger: u8,
            hatched_egg_happiness: u8,
            poison_status: String,
            egg_nickname: String,
            happiness_step_counter_mask: u8,
            happiness_step_counter_target: u8,
        }

        let raw = RawStepEventRules::deserialize(deserializer)?;
        let rules = Self {
            poison_step_interval: raw.poison_step_interval,
            egg_step_trigger: raw.egg_step_trigger,
            hatched_egg_happiness: raw.hatched_egg_happiness,
            poison_status: raw.poison_status,
            egg_nickname: raw.egg_nickname,
            happiness_step_counter_mask: raw.happiness_step_counter_mask,
            happiness_step_counter_target: raw.happiness_step_counter_target,
        };
        rules.validate_shape().map_err(D::Error::custom)?;
        Ok(rules)
    }
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

impl StepEventRules {
    fn validate_shape(&self) -> Result<(), String> {
        if let Some(issue) = step_event_rules_issues(self).into_iter().next() {
            return Err(format!("invalid step event rules: {issue:?}"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum StepEventRulesIssue {
    MissingPoisonStepInterval,
    InvalidPoisonStatus { poison_status: String },
    InvalidEggNickname { egg_nickname: String },
    HappinessTargetOutsideMask { target: u8, mask: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum StepEventError {
    #[error("step event rules are missing")]
    MissingRules,
    #[error("step event rules are invalid: {issue:?}")]
    InvalidRules { issue: StepEventRulesIssue },
}

pub fn step_event_rules_issues(rules: &StepEventRules) -> Vec<StepEventRulesIssue> {
    let mut issues = Vec::new();
    if rules.poison_step_interval == 0 {
        issues.push(StepEventRulesIssue::MissingPoisonStepInterval);
    }
    if !is_exact_step_event_token(&rules.poison_status) {
        issues.push(StepEventRulesIssue::InvalidPoisonStatus {
            poison_status: rules.poison_status.clone(),
        });
    }
    if !is_exact_step_event_token(&rules.egg_nickname) {
        issues.push(StepEventRulesIssue::InvalidEggNickname {
            egg_nickname: rules.egg_nickname.clone(),
        });
    }
    if rules.happiness_step_counter_target > rules.happiness_step_counter_mask {
        issues.push(StepEventRulesIssue::HappinessTargetOutsideMask {
            target: rules.happiness_step_counter_target,
            mask: rules.happiness_step_counter_mask,
        });
    }
    issues
}

pub fn require_step_event_rules(rules: &StepEventRules) -> Result<(), StepEventError> {
    if rules == &StepEventRules::default() {
        return Err(StepEventError::MissingRules);
    }
    if let Some(issue) = step_event_rules_issues(rules).into_iter().next() {
        return Err(StepEventError::InvalidRules { issue });
    }
    Ok(())
}

fn is_exact_step_event_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !has_reserved_pack_prefix(value)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn has_reserved_pack_prefix(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("fallback") || value.starts_with("legacy")
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

pub fn process_step_checked(
    rules: &StepEventRules,
    counters: &mut StepEventCounters,
    party: &mut Party,
) -> Result<StepEventResult, StepEventError> {
    require_step_event_rules(rules)?;
    Ok(process_step(rules, counters, party))
}

pub fn process_overworld_step(state: &mut GameState, rules: &StepEventRules) -> StepEventResult {
    crate::systems::special_routines::advance_day_care_step(state);
    let result = process_step(rules, &mut state.step_events, &mut state.storage.party);
    if state.repel_steps_remaining == 0 {
        state.active_repel_item = None;
    } else {
        state.tick_repel_step_after_movement();
    }
    state.sync_party_from_storage();
    result
}

pub fn process_overworld_step_checked(
    state: &mut GameState,
    rules: &StepEventRules,
) -> Result<StepEventResult, StepEventError> {
    require_step_event_rules(rules)?;
    Ok(process_overworld_step(state, rules))
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
    let mut hatched = None;
    for pokemon in party.pokemon.iter_mut().flatten() {
        if !is_egg(rules, pokemon) {
            continue;
        }
        pokemon.happiness = pokemon.happiness.wrapping_sub(1);
        if pokemon.happiness == 0 && hatched.is_none() {
            let species_id = pokemon.species.id.clone();
            pokemon.nickname = crate::models::pokemon_species_display_name(&species_id);
            pokemon.happiness = rules.hatched_egg_happiness;
            pokemon.hp = pokemon.max_hp;
            pokemon.status = None;
            pokemon.sleep_turns = 0;
            pokemon.flinching = false;
            pokemon.confusion_turns = 0;
            pokemon.rampage_turns = 0;
            pokemon.perish_song_turns = 0;
            hatched = Some(species_id);
        }
    }
    hatched
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
    let _ = rules;
    pokemon.status.as_deref() == Some("EGG")
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

    #[test]
    fn step_event_rules_issues_validate_exact_pack_tokens() {
        assert_eq!(
            step_event_rules_issues(&StepEventRules::default()),
            [
                StepEventRulesIssue::MissingPoisonStepInterval,
                StepEventRulesIssue::InvalidPoisonStatus {
                    poison_status: String::new()
                },
                StepEventRulesIssue::InvalidEggNickname {
                    egg_nickname: String::new()
                },
            ]
        );

        let rules = StepEventRules {
            poison_step_interval: 0,
            egg_step_trigger: 0x80,
            hatched_egg_happiness: 0x78,
            poison_status: "BAD POISON".to_string(),
            egg_nickname: " EGG".to_string(),
            happiness_step_counter_mask: 1,
            happiness_step_counter_target: 2,
        };
        assert_eq!(
            step_event_rules_issues(&rules),
            vec![
                StepEventRulesIssue::MissingPoisonStepInterval,
                StepEventRulesIssue::InvalidPoisonStatus {
                    poison_status: "BAD POISON".to_string(),
                },
                StepEventRulesIssue::InvalidEggNickname {
                    egg_nickname: " EGG".to_string(),
                },
                StepEventRulesIssue::HappinessTargetOutsideMask { target: 2, mask: 1 },
            ],
        );
    }

    #[test]
    fn step_event_rules_issues_reject_reserved_pack_prefix_tokens() {
        let rules = StepEventRules {
            poison_step_interval: 4,
            egg_step_trigger: 0x80,
            hatched_egg_happiness: 0x78,
            poison_status: "fallback_poison".to_string(),
            egg_nickname: "legacy_egg".to_string(),
            happiness_step_counter_mask: 1,
            happiness_step_counter_target: 0,
        };

        assert_eq!(
            step_event_rules_issues(&rules),
            vec![
                StepEventRulesIssue::InvalidPoisonStatus {
                    poison_status: "fallback_poison".to_string(),
                },
                StepEventRulesIssue::InvalidEggNickname {
                    egg_nickname: "legacy_egg".to_string(),
                },
            ]
        );
    }

    #[test]
    fn checked_step_processing_rejects_missing_or_invalid_rules_before_mutation() {
        let mut party = Party::default();
        let mut counters = StepEventCounters::default();
        assert_eq!(
            process_step_checked(&StepEventRules::default(), &mut counters, &mut party),
            Err(StepEventError::MissingRules)
        );
        assert_eq!(counters, StepEventCounters::default());
        assert_eq!(party, Party::default());

        let mut bad_rules = rules();
        bad_rules.poison_step_interval = 0;
        let mut oddish = pokemon("ODDISH");
        oddish.hp = 3;
        oddish.status = Some("POISON".to_string());
        party = party_with(vec![(0, oddish)]);
        let before_party = party.clone();
        assert_eq!(
            process_step_checked(&bad_rules, &mut counters, &mut party),
            Err(StepEventError::InvalidRules {
                issue: StepEventRulesIssue::MissingPoisonStepInterval,
            })
        );
        assert_eq!(counters, StepEventCounters::default());
        assert_eq!(party, before_party);
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
        egg.status = Some("EGG".to_string());
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
    fn egg_step_wraps_counter_and_processes_all_eggs() {
        let mut first = pokemon("TOGEPI");
        first.nickname = rules().egg_nickname;
        first.status = Some("EGG".to_string());
        first.happiness = 0;
        let mut second = pokemon("PICHU");
        second.nickname = rules().egg_nickname;
        second.status = Some("EGG".to_string());
        second.happiness = 2;
        let mut party = party_with(vec![(0, first), (1, second)]);
        let mut counters = StepEventCounters {
            step_count: 0x7f,
            ..StepEventCounters::default()
        };

        let result = process_step(&rules(), &mut counters, &mut party);
        assert_eq!(result.egg_hatched, false);
        assert_eq!(party.pokemon[0].as_ref().expect("first").happiness, 0xff);
        assert_eq!(party.pokemon[1].as_ref().expect("second").happiness, 1);
    }

    #[test]
    fn normal_pokemon_nicknamed_egg_is_not_treated_as_an_egg() {
        let mut normal = pokemon("TOGEPI");
        normal.nickname = rules().egg_nickname;
        normal.happiness = 50;
        assert!(!is_egg(&rules(), &normal));
    }

    #[test]
    fn egg_status_is_authoritative_even_with_a_custom_nickname() {
        let mut egg = pokemon("TOGEPI");
        egg.nickname = "HATCHLING".to_string();
        egg.status = Some("EGG".to_string());
        assert!(is_egg(&rules(), &egg));
    }

    #[test]
    fn egg_hatch_skips_poison_in_count_step_ordering() {
        let mut egg = pokemon("TOGEPI");
        egg.nickname = rules().egg_nickname;
        egg.status = Some("EGG".to_string());
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
        egg.status = Some("EGG".to_string());
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

    #[test]
    fn overworld_step_processes_party_events_repel_and_party_sync() {
        let mut state = GameState::default();
        let mut oddish = pokemon("ODDISH");
        oddish.hp = 3;
        oddish.status = Some(rules().poison_status);
        state.storage.party.pokemon[0] = Some(oddish);
        state.step_events.poison_step_count = 3;
        state.repel_steps_remaining = 1;
        state.active_repel_item = Some("REPEL".to_string());

        let result = process_overworld_step(&mut state, &rules());

        assert_eq!(
            result.poison_result,
            Some(PoisonDamageResult {
                damaged_names: vec!["ODDISH".to_string()],
                fainted_names: Vec::new(),
            })
        );
        assert_eq!(state.storage.party.pokemon[0].as_ref().unwrap().hp, 2);
        assert_eq!(state.party.pokemon[0].as_ref().unwrap().species, "ODDISH");
        assert_eq!(state.repel_steps_remaining, 0);
        assert_eq!(state.active_repel_item, None);

        state.active_repel_item = Some("REPEL".to_string());
        let _ = process_overworld_step(&mut state, &rules());
        assert_eq!(state.repel_steps_remaining, 0);
        assert_eq!(state.active_repel_item, None);
    }

    #[test]
    fn step_event_issue_json_rejects_unknown_fallback_fields() {
        let error = serde_json::from_value::<StepEventRulesIssue>(serde_json::json!({
            "InvalidPoisonStatus": {
                "poison_status": "PSN",
                "fallback_poison_status": "POISON"
            }
        }))
        .expect_err("fallback poison status must be rejected")
        .to_string();
        assert!(
            error.contains("unknown field `fallback_poison_status`"),
            "{error}"
        );
    }
}
