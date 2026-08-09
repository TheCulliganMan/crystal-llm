use crate::random::{
    CrystalRandom, CrystalRandomState, DividerSource, LinkBattleRandom, LinkBattleRandomState,
};
use crate::state::{RoamingMapHistory, RoamingPokemonState};
use crate::systems::special_routines::{
    ROAMING_POKEMON_SLOT_COUNT, RoamingMapLocation, RoamingPokemonCatalog,
    roaming_pokemon_catalog_shape_issues,
};

pub const ROAMING_BATTLE_TYPE: &str = "BATTLETYPE_ROAMING";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoamingEngineState {
    pub slots: [RoamingPokemonState; ROAMING_POKEMON_SLOT_COUNT],
    pub history: RoamingMapHistory,
    pub random_state: CrystalRandomState,
    pub link_random: Option<LinkBattleRandomState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoamingBattleEndInput<'a> {
    pub battle_type: &'a str,
    pub roaming_slot: Option<u8>,
    pub enemy_hp: u16,
    pub battle_result: u8,
    pub link_battle: bool,
    pub current_map: RoamingMapLocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoamingBattleEndOutcome {
    pub route_update_ran: bool,
    pub cleared_roaming_slot: Option<u8>,
    pub saved_roaming_hp: Option<u8>,
    pub battle_random_gate: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoamingEncounterSelection {
    pub roll: u8,
    pub roaming_slot: Option<u8>,
    pub random_state_after: CrystalRandomState,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RoamingEngineError {
    #[error("invalid roaming Pokemon catalog: {error}")]
    InvalidCatalog { error: String },
    #[error("roaming battle requires roaming_slot")]
    MissingRoamingSlot,
    #[error("roaming_slot {slot} is outside slot range 0..3")]
    InvalidRoamingSlot { slot: u8 },
    #[error("non-roaming battle must not declare roaming_slot {slot}")]
    UnexpectedRoamingSlot { slot: u8 },
    #[error("link battle requires persisted link random seeds and count")]
    MissingLinkRandomState,
    #[error("invalid link random state: {error}")]
    InvalidLinkRandomState { error: String },
    #[error("divider source failed: {error}")]
    Divider { error: String },
    #[error("divider replay contains {remaining} unused samples")]
    UnusedDividerSamples { remaining: usize },
}

fn validate_catalog(catalog: &RoamingPokemonCatalog) -> Result<(), RoamingEngineError> {
    if let Some(issue) = roaming_pokemon_catalog_shape_issues(catalog)
        .into_iter()
        .next()
    {
        return Err(RoamingEngineError::InvalidCatalog {
            error: issue.to_string(),
        });
    }
    Ok(())
}

/// `InitializeWorld` runs after WRAM clear and writes the inactive map bytes
/// for all three roam structs while leaving their other freshly-cleared bytes
/// untouched. Burned Tower's later `InitRoamMons` writes only slots 0 and 1.
pub fn initialize_world_roaming_slots(
    catalog: &RoamingPokemonCatalog,
    slots: &[RoamingPokemonState; ROAMING_POKEMON_SLOT_COUNT],
) -> Result<[RoamingPokemonState; ROAMING_POKEMON_SLOT_COUNT], RoamingEngineError> {
    validate_catalog(catalog)?;
    let mut initialized = slots.clone();
    for slot in &mut initialized {
        slot.species = None;
        slot.map_group = catalog.inactive_map.map_group;
        slot.map_number = catalog.inactive_map.map_number;
    }
    Ok(initialized)
}

fn random_byte_with_carry<S>(
    rng: &mut CrystalRandom<&mut S>,
    carry_in: bool,
) -> Result<u8, RoamingEngineError>
where
    S: DividerSource + ?Sized,
    S::Error: std::fmt::Display,
{
    rng.random(carry_in)
        .map(|output| output.value)
        .map_err(|error| RoamingEngineError::Divider {
            error: error.to_string(),
        })
}

fn random_byte<S>(rng: &mut CrystalRandom<&mut S>) -> Result<u8, RoamingEngineError>
where
    S: DividerSource + ?Sized,
    S::Error: std::fmt::Display,
{
    random_byte_with_carry(rng, false)
}

fn jump_roam_mon<S>(
    catalog: &RoamingPokemonCatalog,
    current_map: RoamingMapLocation,
    first_carry: bool,
    rng: &mut CrystalRandom<&mut S>,
) -> Result<RoamingMapLocation, RoamingEngineError>
where
    S: DividerSource + ?Sized,
    S::Error: std::fmt::Display,
{
    let mut carry_in = first_carry;
    loop {
        let index = usize::from(random_byte_with_carry(rng, carry_in)? & catalog.jump_mask);
        let route = &catalog.routes[index];
        let candidate = RoamingMapLocation {
            map_group: route.map_group,
            map_number: route.map_number,
        };
        if candidate != current_map {
            return Ok(candidate);
        }
        // The rejected candidate reached the retry through the equal `cp`
        // pair, which clears carry before the next Random call.
        carry_in = false;
    }
}

fn update_one_roamer<S>(
    slot: &mut RoamingPokemonState,
    catalog: &RoamingPokemonCatalog,
    history: RoamingMapHistory,
    current_map: RoamingMapLocation,
    rng: &mut CrystalRandom<&mut S>,
) -> Result<(), RoamingEngineError>
where
    S: DividerSource + ?Sized,
    S::Error: std::fmt::Display,
{
    if slot.map_group == catalog.inactive_map.map_group {
        return Ok(());
    }
    let Some(route) = catalog.routes.iter().find(|route| {
        route.map_group == slot.map_group && route.map_number == slot.map_number
    }) else {
        return Ok(());
    };

    let destination = loop {
        let roll = random_byte(rng)?;
        if roll & 0x1f == 0 {
            break jump_roam_mon(catalog, current_map, false, rng)?;
        }
        let index = usize::from(roll & 0x03);
        let Some(candidate) = route.connections.get(index).copied() else {
            continue;
        };
        if candidate.map_group == history.last_map_group
            && candidate.map_number == history.last_map_number
        {
            continue;
        }
        break candidate;
    };
    slot.map_group = destination.map_group;
    slot.map_number = destination.map_number;
    Ok(())
}

fn shift_roaming_history(history: &mut RoamingMapHistory, current_map: RoamingMapLocation) {
    history.last_map_number = history.current_map_number;
    history.last_map_group = history.current_map_group;
    history.current_map_number = current_map.map_number;
    history.current_map_group = current_map.map_group;
}

fn update_roam_mons_with_rng<S>(
    catalog: &RoamingPokemonCatalog,
    slots: &mut [RoamingPokemonState; ROAMING_POKEMON_SLOT_COUNT],
    history: &mut RoamingMapHistory,
    current_map: RoamingMapLocation,
    rng: &mut CrystalRandom<&mut S>,
) -> Result<(), RoamingEngineError>
where
    S: DividerSource + ?Sized,
    S::Error: std::fmt::Display,
{
    let prior_history = *history;
    for slot in slots {
        update_one_roamer(slot, catalog, prior_history, current_map, rng)?;
    }
    shift_roaming_history(history, current_map);
    Ok(())
}

pub fn update_roam_mons<S>(
    catalog: &RoamingPokemonCatalog,
    state: &RoamingEngineState,
    current_map: RoamingMapLocation,
    divider: &mut S,
) -> Result<RoamingEngineState, RoamingEngineError>
where
    S: DividerSource + ?Sized,
    S::Error: std::fmt::Display,
{
    validate_catalog(catalog)?;
    let mut next = state.clone();
    let mut rng = CrystalRandom::new(next.random_state, divider);
    update_roam_mons_with_rng(
        catalog,
        &mut next.slots,
        &mut next.history,
        current_map,
        &mut rng,
    )?;
    next.random_state = rng.state();
    Ok(next)
}

/// Exact land-only `CheckEncounterRoamMon` selector. The caller owns the
/// preceding encounter-rate and water gates; once entered, this consumes one
/// ordinary `Random` call with carry clear and returns the selected raw WRAM
/// slot as a zero-based typed index.
pub fn check_encounter_roam_mon<S>(
    slots: &[RoamingPokemonState; ROAMING_POKEMON_SLOT_COUNT],
    current_map: RoamingMapLocation,
    random_state: CrystalRandomState,
    divider: &mut S,
) -> Result<RoamingEncounterSelection, RoamingEngineError>
where
    S: DividerSource + ?Sized,
    S::Error: std::fmt::Display,
{
    let mut rng = CrystalRandom::new(random_state, divider);
    let roll = random_byte(&mut rng)?;
    let raw_slot = roll & 0x03;
    let roaming_slot = if roll < 100 && raw_slot != 0 {
        let slot = raw_slot - 1;
        let candidate = &slots[usize::from(slot)];
        (candidate.species.is_some()
            && candidate.map_group == current_map.map_group
            && candidate.map_number == current_map.map_number)
            .then_some(slot)
    } else {
        None
    };
    Ok(RoamingEncounterSelection {
        roll,
        roaming_slot,
        random_state_after: rng.state(),
    })
}

pub fn check_encounter_roam_mon_replay(
    slots: &[RoamingPokemonState; ROAMING_POKEMON_SLOT_COUNT],
    current_map: RoamingMapLocation,
    random_state: CrystalRandomState,
    divider_trace: &[u8],
) -> Result<RoamingEncounterSelection, RoamingEngineError> {
    let mut divider = crate::random::ReplayDivider::new(divider_trace.iter().copied());
    let selection =
        check_encounter_roam_mon(slots, current_map, random_state, &mut divider)?;
    if divider.remaining() != 0 {
        return Err(RoamingEngineError::UnusedDividerSamples {
            remaining: divider.remaining(),
        });
    }
    Ok(selection)
}

/// Exact `JumpRoamMons`: globally relocate each live slot in source order,
/// then back up the four roaming-map history bytes once.
pub fn jump_roam_mons<S>(
    catalog: &RoamingPokemonCatalog,
    state: &RoamingEngineState,
    current_map: RoamingMapLocation,
    divider: &mut S,
) -> Result<RoamingEngineState, RoamingEngineError>
where
    S: DividerSource + ?Sized,
    S::Error: std::fmt::Display,
{
    validate_catalog(catalog)?;
    let mut next = state.clone();
    let mut rng = CrystalRandom::new(next.random_state, divider);
    for slot in &mut next.slots {
        if slot.map_group == catalog.inactive_map.map_group {
            continue;
        }
        // `cp GROUP_N_A` immediately precedes the first JumpRoamMon call.
        // The carry is therefore the unsigned comparison result, not a
        // caller-selected seed bit.
        let first_carry = slot.map_group < catalog.inactive_map.map_group;
        let destination = jump_roam_mon(catalog, current_map, first_carry, &mut rng)?;
        slot.map_group = destination.map_group;
        slot.map_number = destination.map_number;
    }
    shift_roaming_history(&mut next.history, current_map);
    next.random_state = rng.state();
    Ok(next)
}

pub fn jump_roam_mons_replay(
    catalog: &RoamingPokemonCatalog,
    state: &RoamingEngineState,
    current_map: RoamingMapLocation,
    divider_trace: &[u8],
) -> Result<RoamingEngineState, RoamingEngineError> {
    let mut divider = crate::random::ReplayDivider::new(divider_trace.iter().copied());
    let next = jump_roam_mons(catalog, state, current_map, &mut divider)?;
    if divider.remaining() != 0 {
        return Err(RoamingEngineError::UnusedDividerSamples {
            remaining: divider.remaining(),
        });
    }
    Ok(next)
}

pub fn battle_end_handle_roam_mons<S>(
    catalog: &RoamingPokemonCatalog,
    state: &RoamingEngineState,
    input: RoamingBattleEndInput<'_>,
    divider: &mut S,
) -> Result<(RoamingEngineState, RoamingBattleEndOutcome), RoamingEngineError>
where
    S: DividerSource + ?Sized,
    S::Error: std::fmt::Display,
{
    validate_catalog(catalog)?;
    let mut next = state.clone();
    let mut outcome = RoamingBattleEndOutcome {
        route_update_ran: false,
        cleared_roaming_slot: None,
        saved_roaming_hp: None,
        battle_random_gate: None,
    };

    if input.battle_type == ROAMING_BATTLE_TYPE {
        let slot = input
            .roaming_slot
            .ok_or(RoamingEngineError::MissingRoamingSlot)?;
        let slot_index = usize::from(slot);
        if slot_index >= ROAMING_POKEMON_SLOT_COUNT {
            return Err(RoamingEngineError::InvalidRoamingSlot { slot });
        }
        if input.battle_result & 0x0f == 0 {
            let roaming = &mut next.slots[slot_index];
            roaming.hp = 0;
            roaming.map_group = catalog.inactive_map.map_group;
            roaming.map_number = catalog.inactive_map.map_number;
            roaming.species = None;
            outcome.cleared_roaming_slot = Some(slot);
            return Ok((next, outcome));
        }
        let hp = input.enemy_hp as u8;
        next.slots[slot_index].hp = hp;
        outcome.saved_roaming_hp = Some(hp);
    } else {
        if let Some(slot) = input.roaming_slot {
            return Err(RoamingEngineError::UnexpectedRoamingSlot { slot });
        }
        let gate = if input.link_battle {
            let link_state = next
                .link_random
                .as_mut()
                .ok_or(RoamingEngineError::MissingLinkRandomState)?;
            let mut stream = LinkBattleRandom::from_state(link_state).map_err(|error| {
                RoamingEngineError::InvalidLinkRandomState {
                    error: error.to_string(),
                }
            })?;
            let value = stream.battle_random();
            *link_state = stream.state();
            value
        } else {
            let mut rng = CrystalRandom::new(next.random_state, &mut *divider);
            let value = rng
                .battle_random()
                .map_err(|error| RoamingEngineError::Divider {
                    error: error.to_string(),
                })?;
            next.random_state = rng.state();
            value
        };
        outcome.battle_random_gate = Some(gate);
        if gate & 0x0f != 0 {
            return Ok((next, outcome));
        }
    }

    let mut rng = CrystalRandom::new(next.random_state, divider);
    update_roam_mons_with_rng(
        catalog,
        &mut next.slots,
        &mut next.history,
        input.current_map,
        &mut rng,
    )?;
    next.random_state = rng.state();
    outcome.route_update_ran = true;
    Ok((next, outcome))
}

pub fn battle_end_handle_roam_mons_replay(
    catalog: &RoamingPokemonCatalog,
    state: &RoamingEngineState,
    input: RoamingBattleEndInput<'_>,
    divider_trace: &[u8],
) -> Result<(RoamingEngineState, RoamingBattleEndOutcome), RoamingEngineError> {
    let mut divider = crate::random::ReplayDivider::new(divider_trace.iter().copied());
    let outcome = battle_end_handle_roam_mons(catalog, state, input, &mut divider)?;
    if divider.remaining() != 0 {
        return Err(RoamingEngineError::UnusedDividerSamples {
            remaining: divider.remaining(),
        });
    }
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::special_routines::RoamingPokemonRoute;

    fn catalog() -> RoamingPokemonCatalog {
        let routes = (0..16)
            .map(|index| RoamingPokemonRoute {
                map_group: 1,
                map_number: index + 1,
                connections: if index == 0 {
                    vec![
                        RoamingMapLocation {
                            map_group: 1,
                            map_number: 2,
                        },
                        RoamingMapLocation {
                            map_group: 1,
                            map_number: 3,
                        },
                    ]
                } else {
                    vec![RoamingMapLocation {
                        map_group: 1,
                        map_number: (index + 1) % 16 + 1,
                    }]
                },
            })
            .collect();
        RoamingPokemonCatalog {
            slot_count: 3,
            inactive_map: RoamingMapLocation {
                map_group: 0xfe,
                map_number: 0xfd,
            },
            init_writes: vec![
                crate::systems::special_routines::RoamingPokemonInitWrite {
                    slot: 0,
                    species: "RAIKOU".to_string(),
                    level: 40,
                    map_group: 1,
                    map_number: 1,
                    hp: 0,
                },
                crate::systems::special_routines::RoamingPokemonInitWrite {
                    slot: 1,
                    species: "ENTEI".to_string(),
                    level: 40,
                    map_group: 1,
                    map_number: 2,
                    hp: 0,
                },
            ],
            routes,
            jump_mask: 15,
        }
    }

    fn active(species: &str, map_number: u8) -> RoamingPokemonState {
        RoamingPokemonState {
            species: Some(species.to_string()),
            level: 40,
            map_group: 1,
            map_number,
            hp: 99,
            dvs_be: [0xab, 0xcd],
        }
    }

    fn engine_state() -> RoamingEngineState {
        RoamingEngineState {
            slots: [
                active("RAIKOU", 1),
                active("ENTEI", 1),
                RoamingPokemonState {
                    map_group: catalog().inactive_map.map_group,
                    map_number: catalog().inactive_map.map_number,
                    ..RoamingPokemonState::default()
                },
            ],
            history: RoamingMapHistory {
                current_map_number: 8,
                current_map_group: 1,
                last_map_number: 2,
                last_map_group: 1,
            },
            random_state: CrystalRandomState::default(),
            link_random: None,
        }
    }

    fn trace_for_sub_values(values: impl IntoIterator<Item = u8>) -> Vec<u8> {
        let mut previous = 0u8;
        let mut trace = Vec::new();
        for value in values {
            trace.push(0);
            trace.push(previous.wrapping_sub(value));
            previous = value;
        }
        trace
    }

    #[test]
    fn initialize_world_sets_all_three_catalog_inactive_maps_and_preserves_cleared_payload() {
        let slots = std::array::from_fn(|_| RoamingPokemonState::default());
        let initialized = initialize_world_roaming_slots(&catalog(), &slots)
            .expect("catalog-driven InitializeWorld roam bytes");
        for slot in initialized {
            assert_eq!(slot.species, None);
            assert_eq!(slot.map_group, catalog().inactive_map.map_group);
            assert_eq!(slot.map_number, catalog().inactive_map.map_number);
            assert_eq!(slot.level, 0);
            assert_eq!(slot.hp, 0);
            assert_eq!(slot.dvs_be, [0, 0]);
        }
    }

    #[test]
    fn update_retries_invalid_connection_and_last_map_in_slot_order_then_backs_up_history() {
        let state = engine_state();
        // slot0: 3 is invalid, 4 selects Last map 2, 1 selects map 3.
        // slot1 then consumes 1 and also selects map 3.
        let mut divider = crate::random::ReplayDivider::new(trace_for_sub_values([3, 4, 1, 1]));
        let next = update_roam_mons(
            &catalog(),
            &state,
            RoamingMapLocation {
                map_group: 9,
                map_number: 9,
            },
            &mut divider,
        )
        .expect("exact route update");
        assert_eq!(divider.remaining(), 0);
        assert_eq!(next.slots[0].map_number, 3);
        assert_eq!(next.slots[1].map_number, 3);
        assert_eq!(next.history.last_map_number, 8);
        assert_eq!(next.history.last_map_group, 1);
        assert_eq!(next.history.current_map_number, 9);
        assert_eq!(next.history.current_map_group, 9);
    }

    #[test]
    fn encounter_selector_uses_one_carry_clear_random_and_preserves_exact_slot_identity() {
        let mut state = engine_state();
        state.slots[0].map_group = 7;
        state.slots[0].map_number = 9;
        state.slots[2] = active("SUICUNE", 1);
        state.random_state = CrystalRandomState { add: 0xff, sub: 0 };
        // With carry clear, [0, 255] yields 1 and selects WRAM slot 0. If the
        // selector inherited carry, ADC would overflow and SBC would instead
        // yield 0, proving this source boundary is not BattleRandom/link RNG.
        let selected = check_encounter_roam_mon_replay(
            &state.slots,
            RoamingMapLocation {
                map_group: 7,
                map_number: 9,
            },
            state.random_state,
            &[0, 255],
        )
        .expect("exact roaming encounter selector");
        assert_eq!(selected.roll, 1);
        assert_eq!(selected.roaming_slot, Some(0));
        assert_eq!(selected.random_state_after.add, 0xff);

        state.slots[0].map_group = 1;
        state.slots[0].map_number = 1;
        state.random_state = CrystalRandomState::default();

        for (roll, expected_slot) in [
            (0, None),
            (1, Some(0)),
            (2, Some(1)),
            (3, Some(2)),
            (99, Some(2)),
            (100, None),
        ] {
            let selection = check_encounter_roam_mon_replay(
                &state.slots,
                RoamingMapLocation {
                    map_group: 1,
                    map_number: 1,
                },
                CrystalRandomState::default(),
                &trace_for_sub_values([roll]),
            )
            .expect("selector outcome");
            assert_eq!(selection.roaming_slot, expected_slot, "roll {roll}");
        }
    }

    #[test]
    fn encounter_selector_replay_rejects_short_and_unused_traces_atomically() {
        let state = engine_state();
        let current_map = RoamingMapLocation {
            map_group: 1,
            map_number: 1,
        };
        assert!(matches!(
            check_encounter_roam_mon_replay(
                &state.slots,
                current_map,
                state.random_state,
                &[0]
            ),
            Err(RoamingEngineError::Divider { .. })
        ));
        assert_eq!(
            check_encounter_roam_mon_replay(
                &state.slots,
                current_map,
                state.random_state,
                &[0, 0, 99]
            ),
            Err(RoamingEngineError::UnusedDividerSamples { remaining: 1 })
        );
        assert_eq!(state, engine_state());
    }

    #[test]
    fn global_jump_rejects_the_players_current_map_not_the_roamers_origin() {
        let mut state = engine_state();
        state.slots[1].map_group = catalog().inactive_map.map_group;
        state.slots[1].map_number = catalog().inactive_map.map_number;
        // Zero triggers JumpRoamMon. Index 0 is the player's current map and
        // retries; index 1 is accepted even though another slot originated there.
        let mut divider = crate::random::ReplayDivider::new(trace_for_sub_values([0, 0, 1]));
        let next = update_roam_mons(
            &catalog(),
            &state,
            RoamingMapLocation {
                map_group: 1,
                map_number: 1,
            },
            &mut divider,
        )
        .expect("global jump");
        assert_eq!(next.slots[0].map_number, 2);
        assert_eq!(divider.remaining(), 0);
    }

    #[test]
    fn jump_roam_mons_uses_cp_carry_for_each_live_slot_and_shifts_history_once() {
        let mut state = engine_state();
        state.random_state = CrystalRandomState { add: 0xff, sub: 0 };
        state.slots[2].map_group = catalog().inactive_map.map_group;
        state.slots[2].map_number = catalog().inactive_map.map_number;
        // With carry from `cp GROUP_N_A`, the first [0, 254] sample pair
        // wraps hRandomAdd and yields index 1. A fabricated carry=false entry
        // would yield index 2 instead. Slot 1 then also selects index 1.
        let next = jump_roam_mons_replay(
            &catalog(),
            &state,
            RoamingMapLocation {
                map_group: 9,
                map_number: 9,
            },
            &[0, 254, 0, 0],
        )
        .expect("exact JumpRoamMons");
        assert_eq!(next.slots[0].map_number, 2);
        assert_eq!(next.slots[1].map_number, 2);
        assert_eq!(next.slots[2], state.slots[2]);
        assert_eq!(next.history.last_map_number, 8);
        assert_eq!(next.history.last_map_group, 1);
        assert_eq!(next.history.current_map_number, 9);
        assert_eq!(next.history.current_map_group, 9);
    }

    #[test]
    fn jump_roam_mons_retry_clears_carry_and_replay_is_strict_and_atomic() {
        let mut state = engine_state();
        state.slots[1].map_group = catalog().inactive_map.map_group;
        state.slots[1].map_number = catalog().inactive_map.map_number;
        // First call (carry from cp) selects the current map at index 0;
        // retry enters with carry clear and accepts index 1.
        let next = jump_roam_mons_replay(
            &catalog(),
            &state,
            RoamingMapLocation {
                map_group: 1,
                map_number: 1,
            },
            &trace_for_sub_values([0, 1]),
        )
        .expect("JumpRoamMons current-map retry");
        assert_eq!(next.slots[0].map_number, 2);

        assert!(matches!(
            jump_roam_mons_replay(
                &catalog(),
                &state,
                RoamingMapLocation::default(),
                &[0]
            ),
            Err(RoamingEngineError::Divider { .. })
        ));
        assert_eq!(state, {
            let mut expected = engine_state();
            expected.slots[1].map_group = catalog().inactive_map.map_group;
            expected.slots[1].map_number = catalog().inactive_map.map_number;
            expected
        });
        assert_eq!(
            jump_roam_mons_replay(
                &catalog(),
                &state,
                RoamingMapLocation::default(),
                &[0, 0, 99]
            ),
            Err(RoamingEngineError::UnusedDividerSamples { remaining: 1 })
        );
    }

    #[test]
    fn roaming_win_uses_catalog_inactive_map_and_preserves_level_dvs_with_zero_reads() {
        let state = engine_state();
        let (next, outcome) = battle_end_handle_roam_mons_replay(
            &catalog(),
            &state,
            RoamingBattleEndInput {
                battle_type: ROAMING_BATTLE_TYPE,
                roaming_slot: Some(0),
                enemy_hp: 0x1234,
                battle_result: 0x80,
                link_battle: false,
                current_map: RoamingMapLocation::default(),
            },
            &[],
        )
        .expect("roaming WIN");
        assert_eq!(outcome.cleared_roaming_slot, Some(0));
        assert_eq!(next.slots[0].species, None);
        assert_eq!(next.slots[0].level, 40);
        assert_eq!(next.slots[0].dvs_be, [0xab, 0xcd]);
        assert_eq!(next.slots[0].map_group, catalog().inactive_map.map_group);
        assert_eq!(next.slots[0].map_number, catalog().inactive_map.map_number);
    }

    #[test]
    fn roaming_nonwin_saves_only_low_hp_byte_then_updates_routes() {
        let state = engine_state();
        let (next, outcome) = battle_end_handle_roam_mons_replay(
            &catalog(),
            &state,
            RoamingBattleEndInput {
                battle_type: ROAMING_BATTLE_TYPE,
                roaming_slot: Some(0),
                enemy_hp: 0x1234,
                battle_result: 1,
                link_battle: false,
                current_map: RoamingMapLocation {
                    map_group: 9,
                    map_number: 9,
                },
            },
            &trace_for_sub_values([1, 1]),
        )
        .expect("roaming non-WIN");
        assert_eq!(outcome.saved_roaming_hp, Some(0x34));
        assert_eq!(next.slots[0].hp, 0x34);
        assert!(outcome.route_update_ran);
    }

    #[test]
    fn nonroaming_local_gate_consumes_one_battle_random_and_only_low_nibble_zero_updates() {
        let state = engine_state();
        let (_, nonzero) = battle_end_handle_roam_mons_replay(
            &catalog(),
            &state,
            RoamingBattleEndInput {
                battle_type: "BATTLETYPE_NORMAL",
                roaming_slot: None,
                enemy_hp: 0,
                battle_result: 0,
                link_battle: false,
                current_map: RoamingMapLocation::default(),
            },
            &trace_for_sub_values([0x10, 1, 1]),
        )
        .expect("low nibble zero does not mean full byte zero");
        assert_eq!(nonzero.battle_random_gate, Some(0x10));
        assert!(nonzero.route_update_ran);
    }

    #[test]
    fn link_gate_count_eight_returns_seed_eight_advances_all_ten_then_routes_from_divider() {
        let mut state = engine_state();
        state.link_random = Some(LinkBattleRandomState {
            seeds: [1, 2, 3, 4, 5, 6, 7, 8, 0x10, 10],
            count: 8,
        });
        let (next, outcome) = battle_end_handle_roam_mons_replay(
            &catalog(),
            &state,
            RoamingBattleEndInput {
                battle_type: "BATTLETYPE_LINK",
                roaming_slot: None,
                enemy_hp: 0,
                battle_result: 0,
                link_battle: true,
                current_map: RoamingMapLocation {
                    map_group: 9,
                    map_number: 9,
                },
            },
            &trace_for_sub_values([1, 1]),
        )
        .expect("link gate and ordinary route Random");
        assert_eq!(outcome.battle_random_gate, Some(0x10));
        assert!(outcome.route_update_ran);
        assert_eq!(next.link_random.as_ref().unwrap().count, 0);
        assert_eq!(next.link_random.as_ref().unwrap().seeds[9], 51);
    }

    #[test]
    fn replay_short_and_unused_tail_reject_without_returning_mutated_state() {
        let state = engine_state();
        let input = RoamingBattleEndInput {
            battle_type: "BATTLETYPE_NORMAL",
            roaming_slot: None,
            enemy_hp: 0,
            battle_result: 0,
            link_battle: false,
            current_map: RoamingMapLocation::default(),
        };
        assert!(matches!(
            battle_end_handle_roam_mons_replay(&catalog(), &state, input, &[0]),
            Err(RoamingEngineError::Divider { .. })
        ));
        assert_eq!(state, engine_state());
        assert_eq!(
            battle_end_handle_roam_mons_replay(
                &catalog(),
                &state,
                input,
                &trace_for_sub_values([1]).into_iter().chain([99]).collect::<Vec<_>>(),
            ),
            Err(RoamingEngineError::UnusedDividerSamples { remaining: 1 })
        );
        assert_eq!(state, engine_state());
    }
}
