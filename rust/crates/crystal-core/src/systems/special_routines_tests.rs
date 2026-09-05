use super::*;
use crate::models::{
    BaseStats, Dv, LearnedMove, MAX_BOX_MONS, PcBox, PokemonSpecies, Trainer, TrainerPartyPokemon,
    growth_rate, item_pocket, pokemon_type,
};
use crate::random::ReplayDivider;
use crate::state::{
    CardFlipInput, CardFlipPhase, CardFlipState, DayCareInput, MemoryGameButton, MemoryGameInput,
    MemoryGamePhase, MemoryGameState, OverworldMemory, ScriptWarpRequest, SlotMachineInput,
    SlotMachinePhase, SlotMachineState,
};
use crate::systems::experience::{GrowthRateCatalog, crystal_growth_rate_catalog_for_tests};
use crate::systems::phone::PhoneContactRecord;
use crate::world::encounters::WildEncounterTable;
use crate::world::map::Direction;
use crate::world::movement::MovementMode;
use std::sync::LazyLock;

static EMPTY_TEST_LEARNSETS: LazyLock<SpeciesLearnsets> = LazyLock::new(SpeciesLearnsets::new);
static TEST_GROWTH_RATES: LazyLock<GrowthRateCatalog> =
    LazyLock::new(crystal_growth_rate_catalog_for_tests);
static EMPTY_TEST_ITEMS: LazyLock<BTreeMap<String, Item>> = LazyLock::new(BTreeMap::new);
static EMPTY_TEST_CRIES: LazyLock<BTreeMap<String, String>> = LazyLock::new(BTreeMap::new);
static EMPTY_TEST_SPECIES: LazyLock<BTreeMap<String, PokemonSpecies>> =
    LazyLock::new(BTreeMap::new);
static EMPTY_TEST_SPAWNS: LazyLock<BTreeMap<String, RuntimeSpawnPointRef>> =
    LazyLock::new(BTreeMap::new);
static EMPTY_TEST_ROAMERS: LazyLock<RoamingPokemonCatalog> =
    LazyLock::new(RoamingPokemonCatalog::default);
static EMPTY_TEST_BUENA_PASSWORD_CATEGORIES: LazyLock<BuenaPasswordCategories> =
    LazyLock::new(BuenaPasswordCategories::default);
static EMPTY_TEST_BUENA_PRIZES: LazyLock<BuenaPrizeDefinitions> = LazyLock::new(BTreeMap::new);
static EMPTY_TEST_KURT_APRICORN_RECIPES: LazyLock<KurtApricornRecipes> =
    LazyLock::new(BTreeMap::new);
static EMPTY_TEST_DRATINI_MOVE_SETS: LazyLock<DratiniMoveSets> = LazyLock::new(BTreeMap::new);
static EMPTY_TEST_MAGIKARP_LENGTHS: LazyLock<Vec<MagikarpLengthEntry>> = LazyLock::new(Vec::new);
static EMPTY_TEST_TRAINERS: LazyLock<TrainerCatalog> = LazyLock::new(TrainerCatalog::default);
static EMPTY_TEST_PHONE_CONTACTS: LazyLock<PhoneContactCatalog> =
    LazyLock::new(PhoneContactCatalog::default);
static EMPTY_TEST_WILD_ENCOUNTERS: LazyLock<BTreeMap<String, WildEncounterData>> =
    LazyLock::new(BTreeMap::new);
const MODPACK_SPECIAL_ROUTINES_JSON: &str = include_str!(
    "../../../../../apps/web/assets/data/content-packs/core-modular/special_routines/routines.json"
);

fn divider_trace_for_sub_values(values: impl IntoIterator<Item = u8>) -> Vec<u8> {
    divider_trace_for_sub_values_after(0, values)
}

fn divider_trace_for_sub_values_after(
    initial_sub: u8,
    values: impl IntoIterator<Item = u8>,
) -> Vec<u8> {
    let mut previous_sub = initial_sub;
    let mut samples = Vec::new();
    for value in values {
        // These focused traces start from hRandomAdd=hRandomSub=0 and
        // keep the additive DIV sample at zero, so every direct Random
        // call enters without an ADC carry. Choose the subtractive DIV
        // byte that makes hRandomSub equal the requested value.
        samples.push(0);
        samples.push(previous_sub.wrapping_sub(value));
        previous_sub = value;
    }
    samples
}

fn stage_card_flip_state(state: &mut GameState, phase: CardFlipPhase, num_cards_played: u8) {
    let mut discard_pile = vec![false; 24];
    let discarded = usize::from(num_cards_played)
        + usize::from(matches!(
            phase,
            CardFlipPhase::Result | CardFlipPhase::PlayAgain
        ));
    discard_pile[..discarded].fill(true);
    state.script_runtime.card_flip = Some(CardFlipState {
        deck: (0_u8..24).collect(),
        discard_pile,
        phase,
        num_cards_played,
        which_card: 0,
        cursor_x: 2,
        cursor_y: 2,
        face_up_card: matches!(phase, CardFlipPhase::Result | CardFlipPhase::PlayAgain)
            .then_some(num_cards_played),
        payout_remaining: 0,
    });
}

fn stage_slot_machine_betting_state(state: &mut GameState, keep_seven_bias_chance: bool) {
    state.script_runtime.slot_machine = Some(SlotMachineState {
        phase: SlotMachinePhase::Betting,
        lucky: false,
        keep_seven_bias_chance,
        bet: 3,
        bias: None,
        offsets: [14; 3],
        next_reel: 1,
        matched_symbol: None,
        payout_remaining: 0,
    });
}

fn memory_game_distribution_zero_board() -> Vec<u8> {
    [
        (1_u8, 8_usize),
        (2, 2),
        (3, 6),
        (4, 6),
        (5, 6),
        (6, 8),
        (7, 6),
        (8, 3),
    ]
    .into_iter()
    .flat_map(|(card, count)| std::iter::repeat_n(card, count))
    .collect()
}

fn stage_memory_game_board(state: &mut GameState, tries_remaining: u8) {
    state.script_runtime.memory_game = Some(MemoryGameState {
        cards: memory_game_distribution_zero_board(),
        phase: MemoryGamePhase::CheckTriesRemaining,
        distribution: 0,
        counter: 0,
        number_tries_remaining: tries_remaining,
        last_matches: [0; 5],
        num_cards_matched: 0,
        card1: None,
        card1_location: None,
        card2: None,
        card2_location: None,
        cursor_index: 0,
        cursor_active: true,
        card_choice: 0,
        last_card_picked: 0,
    });
}

fn apply_memory_game_input(
    state: &mut GameState,
    context: SpecialRoutineContext<'_>,
    divider: &mut ReplayDivider,
    input: MemoryGameInput,
) -> SpecialRoutineOutcome {
    state.script_runtime.pending_memory_game_input = Some(input);
    apply_random_special_routine_with_context(state, context, "UnusedMemoryGame", divider)
        .expect("source Memory Game phase")
}

fn resolve_memory_game_pair(
    state: &mut GameState,
    context: SpecialRoutineContext<'_>,
    divider: &mut ReplayDivider,
    first_index: usize,
    second_index: usize,
) -> SpecialRoutineOutcome {
    let frame = |state: &mut GameState, divider: &mut ReplayDivider, button| {
        apply_memory_game_input(
            state,
            context,
            divider,
            MemoryGameInput::AdvanceFrame { button },
        )
    };
    frame(state, divider, None);
    state
        .script_runtime
        .memory_game
        .as_mut()
        .unwrap()
        .cursor_index = u8::try_from(first_index).expect("board index fits u8");
    frame(state, divider, Some(MemoryGameButton::A));
    frame(state, divider, None);
    state
        .script_runtime
        .memory_game
        .as_mut()
        .unwrap()
        .cursor_index = u8::try_from(second_index).expect("board index fits u8");
    frame(state, divider, Some(MemoryGameButton::A));
    frame(state, divider, None);
    for frame in 0..64 {
        let outcome = apply_memory_game_input(
            state,
            context,
            divider,
            MemoryGameInput::AdvanceFrame { button: None },
        );
        if frame == 63 {
            return outcome;
        }
    }
    unreachable!("the source delay is exactly 64 frames")
}

fn test_roaming_catalog() -> RoamingPokemonCatalog {
    let locations = [
        RoamingMapLocation {
            map_group: 2,
            map_number: 5,
        },
        RoamingMapLocation {
            map_group: 10,
            map_number: 4,
        },
        RoamingMapLocation {
            map_group: 3,
            map_number: 1,
        },
        RoamingMapLocation {
            map_group: 4,
            map_number: 1,
        },
        RoamingMapLocation {
            map_group: 5,
            map_number: 1,
        },
        RoamingMapLocation {
            map_group: 6,
            map_number: 1,
        },
        RoamingMapLocation {
            map_group: 7,
            map_number: 1,
        },
        RoamingMapLocation {
            map_group: 8,
            map_number: 1,
        },
        RoamingMapLocation {
            map_group: 9,
            map_number: 1,
        },
        RoamingMapLocation {
            map_group: 11,
            map_number: 1,
        },
        RoamingMapLocation {
            map_group: 12,
            map_number: 1,
        },
        RoamingMapLocation {
            map_group: 13,
            map_number: 1,
        },
        RoamingMapLocation {
            map_group: 14,
            map_number: 1,
        },
        RoamingMapLocation {
            map_group: 15,
            map_number: 1,
        },
        RoamingMapLocation {
            map_group: 16,
            map_number: 1,
        },
        RoamingMapLocation {
            map_group: 17,
            map_number: 1,
        },
    ];
    RoamingPokemonCatalog {
        slot_count: ROAMING_POKEMON_SLOT_COUNT as u8,
        inactive_map: RoamingMapLocation {
            map_group: 0xfe,
            map_number: 0xfd,
        },
        init_writes: vec![
            RoamingPokemonInitWrite {
                slot: 0,
                species: "RAIKOU".to_string(),
                level: 40,
                map_group: 2,
                map_number: 5,
                hp: 0,
            },
            RoamingPokemonInitWrite {
                slot: 1,
                species: "ENTEI".to_string(),
                level: 40,
                map_group: 10,
                map_number: 4,
                hp: 0,
            },
        ],
        routes: locations
            .iter()
            .enumerate()
            .map(|(index, location)| RoamingPokemonRoute {
                map_group: location.map_group,
                map_number: location.map_number,
                connections: vec![locations[(index + 1) % locations.len()]],
            })
            .collect(),
        jump_mask: 15,
    }
}

#[test]
fn special_routine_registry_is_exact_and_covers_core_modpack_declarations() {
    assert!(is_known_special_routine("HealParty"));
    assert!(is_known_special_routine("UnusedDummySpecial"));
    assert!(!is_known_special_routine("healparty"));
    assert!(!is_known_special_routine("MODPACK_ONLY_ROUTINE"));

    let routines: BTreeMap<String, serde_json::Value> =
        serde_json::from_str(MODPACK_SPECIAL_ROUTINES_JSON).expect("core special routines json");
    let unknown: Vec<&str> = routines
        .keys()
        .map(String::as_str)
        .filter(|routine| !is_known_special_routine(routine))
        .collect();
    assert_eq!(unknown, Vec::<&str>::new());

    assert_eq!(
        special_routine_catalog_issues(&BTreeSet::from([
            "HealParty".to_string(),
            "Heal Party".to_string(),
            "fallbackHealParty".to_string(),
            "healparty".to_string(),
            String::new(),
        ])),
        vec![
            SpecialRoutineCatalogIssue::EmptyRoutine {
                routine: String::new(),
            },
            SpecialRoutineCatalogIssue::InvalidRoutine {
                routine: "Heal Party".to_string(),
            },
            SpecialRoutineCatalogIssue::InvalidRoutine {
                routine: "fallbackHealParty".to_string(),
            },
            SpecialRoutineCatalogIssue::UnknownRoutine {
                routine: "healparty".to_string(),
            },
        ]
    );
}

#[test]
fn roaming_pokemon_catalog_issues_validate_exact_source_shape_species_and_level() {
    let species = BTreeSet::from(["RAIKOU".to_string()]);
    let mut catalog = test_roaming_catalog();
    catalog.init_writes[0].species = "RAI KOU".to_string();
    catalog.init_writes[0].level = 0;

    assert_eq!(
        roaming_pokemon_catalog_issues(&catalog, &species),
        vec![
            RoamingPokemonCatalogIssue::InvalidInitSpecies {
                slot: 0,
                species: "RAI KOU".to_string(),
            },
            RoamingPokemonCatalogIssue::InvalidInitLevel { slot: 0, level: 0 },
            RoamingPokemonCatalogIssue::UnknownInitSpecies {
                slot: 1,
                species: "ENTEI".to_string(),
            },
        ]
    );
}

#[test]
fn roaming_pokemon_catalog_rejects_half_zero_inactive_map_and_noncanonical_rows() {
    for inactive_map in [
        RoamingMapLocation {
            map_group: 0,
            map_number: 1,
        },
        RoamingMapLocation {
            map_group: 1,
            map_number: 0,
        },
    ] {
        let mut catalog = test_roaming_catalog();
        catalog.inactive_map = inactive_map;
        assert_eq!(
            roaming_pokemon_catalog_shape_issues(&catalog),
            vec![RoamingPokemonCatalogIssue::InvalidInactiveMap]
        );
        let error = serde_json::from_value::<RoamingPokemonCatalog>(
            serde_json::to_value(&catalog).expect("serialize malformed roamer catalog"),
        )
        .expect_err("half-zero inactiveMap must fail at the typed pack boundary")
        .to_string();
        assert!(
            error.contains("inactiveMap must not use the invalid"),
            "{error}"
        );
    }

    let mut duplicate = test_roaming_catalog();
    let repeated = duplicate.routes[0].connections[0];
    duplicate.routes[0].connections.push(repeated);
    assert!(roaming_pokemon_catalog_shape_issues(&duplicate).contains(
        &RoamingPokemonCatalogIssue::DuplicateConnection {
            index: 0,
            map_group: 10,
            map_number: 4,
        }
    ));

    let mut nonzero_hp = test_roaming_catalog();
    nonzero_hp.init_writes[0].hp = 1;
    assert!(
        roaming_pokemon_catalog_issues(
            &nonzero_hp,
            &BTreeSet::from(["RAIKOU".to_string(), "ENTEI".to_string()])
        )
        .contains(&RoamingPokemonCatalogIssue::InvalidInitHp { slot: 0, hp: 1 })
    );
}

#[test]
fn buena_prize_definition_issues_validate_exact_items_and_cost() {
    let item_ids = BTreeSet::from(["ULTRA_BALL".to_string()]);
    let prizes = BTreeMap::from([
        (String::new(), 0),
        ("ULTRA BALL".to_string(), 2),
        ("ultra_ball".to_string(), 2),
        ("ULTRA_BALL".to_string(), 2),
    ]);

    assert_eq!(
        buena_prize_definition_issues(&prizes, &item_ids),
        vec![
            BuenaPrizeDefinitionIssue::EmptyItem {
                item_id: String::new(),
            },
            BuenaPrizeDefinitionIssue::InvalidCost {
                item_id: String::new(),
            },
            BuenaPrizeDefinitionIssue::InvalidItem {
                item_id: "ULTRA BALL".to_string(),
            },
            BuenaPrizeDefinitionIssue::UnknownItem {
                item_id: "ultra_ball".to_string(),
            },
        ]
    );
}

#[test]
fn buena_password_category_issues_validate_exact_options() {
    let species_ids = BTreeSet::from(["PIKACHU".to_string()]);
    let item_ids = BTreeSet::from(["POTION".to_string()]);
    let move_ids = BTreeSet::from(["THUNDERBOLT".to_string()]);
    let categories = BuenaPasswordCategories {
        order: vec![
            String::new(),
            "BUENA MON".to_string(),
            "ITEM".to_string(),
            "MOVE".to_string(),
            "UNKNOWN".to_string(),
        ],
        categories: BTreeMap::from([
            (
                String::new(),
                BuenaPasswordCategoryDefinition {
                    category_type: "buena mon".to_string(),
                    points: 0,
                    options: Vec::new(),
                },
            ),
            (
                "BUENA MON".to_string(),
                BuenaPasswordCategoryDefinition {
                    category_type: BUENA_PASSWORD_CATEGORY_MON.to_string(),
                    points: 1,
                    options: vec![
                        String::new(),
                        "PIKA CHU".to_string(),
                        "pikachu".to_string(),
                        "PIKACHU".to_string(),
                    ],
                },
            ),
            (
                "ITEM".to_string(),
                BuenaPasswordCategoryDefinition {
                    category_type: BUENA_PASSWORD_CATEGORY_ITEM.to_string(),
                    points: 1,
                    options: vec![
                        "POT ION".to_string(),
                        "potion".to_string(),
                        "POTION".to_string(),
                    ],
                },
            ),
            (
                "MOVE".to_string(),
                BuenaPasswordCategoryDefinition {
                    category_type: BUENA_PASSWORD_CATEGORY_MOVE.to_string(),
                    points: 1,
                    options: vec![
                        "THUNDERBOLT ".to_string(),
                        "thunderbolt".to_string(),
                        "THUNDERBOLT".to_string(),
                    ],
                },
            ),
            (
                "UNKNOWN".to_string(),
                BuenaPasswordCategoryDefinition {
                    category_type: "BUENA_UNKNOWN".to_string(),
                    points: 1,
                    options: vec!["TEXT".to_string()],
                },
            ),
        ]),
    };

    assert_eq!(
        buena_password_category_issues(&categories, &species_ids, &item_ids, &move_ids),
        vec![
            BuenaPasswordCategoryIssue::EmptyId { id: String::new() },
            BuenaPasswordCategoryIssue::InvalidId {
                id: "BUENA MON".to_string(),
            },
            BuenaPasswordCategoryIssue::EmptyId { id: String::new() },
            BuenaPasswordCategoryIssue::UnknownOrderedId { id: String::new() },
            BuenaPasswordCategoryIssue::InvalidCategoryType {
                id: String::new(),
                category_type: "buena mon".to_string(),
            },
            BuenaPasswordCategoryIssue::InvalidPoints { id: String::new() },
            BuenaPasswordCategoryIssue::EmptyOptions { id: String::new() },
            BuenaPasswordCategoryIssue::InvalidId {
                id: "BUENA MON".to_string(),
            },
            BuenaPasswordCategoryIssue::UnknownOrderedId {
                id: "BUENA MON".to_string(),
            },
            BuenaPasswordCategoryIssue::EmptyOption {
                id: "BUENA MON".to_string(),
                option_index: 0,
            },
            BuenaPasswordCategoryIssue::InvalidOption {
                id: "BUENA MON".to_string(),
                option_index: 1,
                option: "PIKA CHU".to_string(),
            },
            BuenaPasswordCategoryIssue::UnknownSpecies {
                id: "BUENA MON".to_string(),
                option_index: 2,
                species: "pikachu".to_string(),
            },
            BuenaPasswordCategoryIssue::InvalidOption {
                id: "ITEM".to_string(),
                option_index: 0,
                option: "POT ION".to_string(),
            },
            BuenaPasswordCategoryIssue::UnknownItem {
                id: "ITEM".to_string(),
                option_index: 1,
                item_id: "potion".to_string(),
            },
            BuenaPasswordCategoryIssue::InvalidOption {
                id: "MOVE".to_string(),
                option_index: 0,
                option: "THUNDERBOLT ".to_string(),
            },
            BuenaPasswordCategoryIssue::UnknownMove {
                id: "MOVE".to_string(),
                option_index: 1,
                move_id: "thunderbolt".to_string(),
            },
            BuenaPasswordCategoryIssue::UnknownCategoryType {
                id: "UNKNOWN".to_string(),
                category_type: "BUENA_UNKNOWN".to_string(),
            },
        ]
    );
}

#[test]
fn kurt_apricorn_recipe_issues_validate_exact_items() {
    let item_ids = BTreeSet::from(["BLU_APRICORN".to_string(), "LURE_BALL".to_string()]);
    let recipes = BTreeMap::from([
        (String::new(), String::new()),
        ("BLU APRICORN".to_string(), "LURE BALL".to_string()),
        ("blu_apricorn".to_string(), "lure_ball".to_string()),
        ("BLU_APRICORN".to_string(), "LURE_BALL".to_string()),
    ]);

    assert_eq!(
        kurt_apricorn_recipe_issues(&recipes, &item_ids),
        vec![
            KurtApricornRecipeIssue::EmptyApricorn {
                apricorn: String::new(),
            },
            KurtApricornRecipeIssue::EmptyBall {
                apricorn: String::new(),
            },
            KurtApricornRecipeIssue::InvalidApricorn {
                apricorn: "BLU APRICORN".to_string(),
            },
            KurtApricornRecipeIssue::InvalidBall {
                apricorn: "BLU APRICORN".to_string(),
                ball: "LURE BALL".to_string(),
            },
            KurtApricornRecipeIssue::UnknownApricorn {
                apricorn: "blu_apricorn".to_string(),
            },
            KurtApricornRecipeIssue::UnknownBall {
                apricorn: "blu_apricorn".to_string(),
                ball: "lure_ball".to_string(),
            },
        ]
    );
}

#[test]
fn shuckie_gift_issues_validate_exact_pack_references() {
    let species_ids = BTreeSet::from(["SHUCKLE".to_string()]);
    let item_ids = BTreeSet::from(["BERRY".to_string()]);
    let engine_flags = BTreeSet::from(["ENGINE_GOT_SHUCKIE_TODAY".to_string()]);

    assert_eq!(
        shuckie_gift_issues(
            &ShuckieGiftDefinition {
                species: String::new(),
                level: 0,
                held_item: String::new(),
                nickname: String::new(),
                original_trainer_name: String::new(),
                original_trainer_id: 518,
                got_today_engine_flag: String::new(),
            },
            &species_ids,
            &item_ids,
            &engine_flags,
        ),
        vec![
            ShuckieGiftIssue::EmptySpecies,
            ShuckieGiftIssue::InvalidLevel,
            ShuckieGiftIssue::EmptyHeldItem,
            ShuckieGiftIssue::EmptyName,
            ShuckieGiftIssue::EmptyEngineFlag,
        ]
    );

    assert_eq!(
        shuckie_gift_issues(
            &ShuckieGiftDefinition {
                species: "SHUCK LE".to_string(),
                level: 15,
                held_item: "BE RRY".to_string(),
                nickname: "SHUCKIE".to_string(),
                original_trainer_name: "MANIA".to_string(),
                original_trainer_id: 518,
                got_today_engine_flag: "ENGINE GOT SHUCKIE TODAY".to_string(),
            },
            &species_ids,
            &item_ids,
            &engine_flags,
        ),
        vec![
            ShuckieGiftIssue::InvalidSpecies {
                species: "SHUCK LE".to_string(),
            },
            ShuckieGiftIssue::InvalidHeldItem {
                held_item: "BE RRY".to_string(),
            },
            ShuckieGiftIssue::InvalidEngineFlag {
                engine_flag: "ENGINE GOT SHUCKIE TODAY".to_string(),
            },
        ]
    );
}

fn move_data(name: &str, pp: u8) -> Move {
    Move {
        source_index: 1,
        name: name.to_string(),
        move_type: pokemon_type("NORMAL"),
        power: 40,
        accuracy: 100,
        pp,
        effect: "NORMAL_HIT".to_string(),
        effect_chance: 0,
        stat: None,
        amount: None,
    }
}

fn item_data(id: &str) -> Item {
    Item {
        name: id.to_string(),
        description: String::new(),
        effect: "NONE".to_string(),
        status_heals: Vec::new(),
        revive_hp_percent: None,
        party_revive_hp_percent: None,
        pp_restore_scope: None,
        pp_restore_points: None,
        pp_up_stages: None,
        vitamin_stat: None,
        vitamin_stat_exp: None,
        vitamin_max_stat_exp: None,
        rare_candy_level_gain: None,
        battle_stat_boost_stat: None,
        battle_stat_boost_stages: None,
        battle_escape_mode: None,
        battle_capture_ball: None,
        battle_focus_energy: None,
        battle_stat_drop_guard: None,
        confusion_heal: None,
        repel_steps: None,
        escape_rope_mode: None,
        price: 0,
        held_effect: "HELD_NONE".to_string(),
        parameter: 0,
        property: String::new(),
        pocket: item_pocket("ITEM"),
        field_menu: String::new(),
        field_usable: true,
        battle_menu: String::new(),
        battle_usable: true,
        script_name: id.to_string(),
        consumable: false,
        tmhm_index: None,
        tmhm_move: None,
    }
}

fn pokemon(id: &str) -> Pokemon {
    let mut species = PokemonSpecies::new_for_tests(id, BaseStats::new(45, 49, 49, 45, 65, 65));
    species.growth_rate = growth_rate("GROWTH_MEDIUM_FAST");
    let mut pokemon = Pokemon::new_for_tests(species, 5, Dv::default());
    pokemon.moves = vec![
        LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 1,
            pp_ups: 1,
        },
        LearnedMove {
            name: "GROWL".to_string(),
            current_pp: 0,
            pp_ups: 0,
        },
    ];
    pokemon.hp = 1;
    pokemon.status = Some("PSN".to_string());
    pokemon.sleep_turns = 3;
    pokemon.confusion_turns = 2;
    pokemon.focus_energy = true;
    pokemon
}

fn moves() -> BTreeMap<String, Move> {
    [
        ("TACKLE".to_string(), move_data("TACKLE", 35)),
        ("GROWL".to_string(), move_data("GROWL", 40)),
    ]
    .into_iter()
    .collect()
}

fn species_catalog(ids: &[(&str, u16)]) -> BTreeMap<String, PokemonSpecies> {
    ids.iter()
        .map(|(id, int_id)| {
            let mut species =
                PokemonSpecies::new_for_tests(*id, BaseStats::new(45, 49, 49, 45, 65, 65));
            species.int_id = *int_id;
            (species.id.clone(), species)
        })
        .collect()
}

fn cry_context<'a>(
    move_catalog: &'a BTreeMap<String, Move>,
    cry_by_species: &'a BTreeMap<String, String>,
    species_catalog: &'a BTreeMap<String, PokemonSpecies>,
) -> SpecialRoutineContext<'a> {
    SpecialRoutineContext {
        move_catalog,
        cry_by_species,
        species_catalog,
        learnsets: &EMPTY_TEST_LEARNSETS,
        growth_rates: &TEST_GROWTH_RATES,
        item_catalog: &EMPTY_TEST_ITEMS,
        runtime_spawn_points: &EMPTY_TEST_SPAWNS,
        roaming_pokemon: &EMPTY_TEST_ROAMERS,
        buena_password_categories: &EMPTY_TEST_BUENA_PASSWORD_CATEGORIES,
        buena_prizes: &EMPTY_TEST_BUENA_PRIZES,
        kurt_apricorn_recipes: &EMPTY_TEST_KURT_APRICORN_RECIPES,
        shuckie_gift: None,
        dratini_move_sets: &EMPTY_TEST_DRATINI_MOVE_SETS,
        bug_contest_config: None,
        battle_tower_rules: None,
        magikarp_lengths: &[],
        happiness_data: None,
        trainer_catalog: &EMPTY_TEST_TRAINERS,
        phone_contacts: &EMPTY_TEST_PHONE_CONTACTS,
        wild_encounters: &EMPTY_TEST_WILD_ENCOUNTERS,
        odd_egg_definitions: &[],
        oak_ratings: &[],
    }
}

fn full_context<'a>(
    move_catalog: &'a BTreeMap<String, Move>,
    species_catalog: &'a BTreeMap<String, PokemonSpecies>,
    learnsets: &'a SpeciesLearnsets,
    item_catalog: &'a BTreeMap<String, Item>,
) -> SpecialRoutineContext<'a> {
    SpecialRoutineContext {
        move_catalog,
        cry_by_species: &EMPTY_TEST_CRIES,
        species_catalog,
        learnsets,
        growth_rates: &TEST_GROWTH_RATES,
        item_catalog,
        runtime_spawn_points: &EMPTY_TEST_SPAWNS,
        roaming_pokemon: &EMPTY_TEST_ROAMERS,
        buena_password_categories: &EMPTY_TEST_BUENA_PASSWORD_CATEGORIES,
        buena_prizes: &EMPTY_TEST_BUENA_PRIZES,
        kurt_apricorn_recipes: &EMPTY_TEST_KURT_APRICORN_RECIPES,
        shuckie_gift: None,
        dratini_move_sets: &EMPTY_TEST_DRATINI_MOVE_SETS,
        bug_contest_config: None,
        battle_tower_rules: None,
        magikarp_lengths: &[],
        happiness_data: None,
        trainer_catalog: &EMPTY_TEST_TRAINERS,
        phone_contacts: &EMPTY_TEST_PHONE_CONTACTS,
        wild_encounters: &EMPTY_TEST_WILD_ENCOUNTERS,
        odd_egg_definitions: &[],
        oak_ratings: &[],
    }
}

fn full_context_with_shuckie_gift<'a>(
    move_catalog: &'a BTreeMap<String, Move>,
    species_catalog: &'a BTreeMap<String, PokemonSpecies>,
    learnsets: &'a SpeciesLearnsets,
    item_catalog: &'a BTreeMap<String, Item>,
    shuckie_gift: &'a ShuckieGiftDefinition,
) -> SpecialRoutineContext<'a> {
    SpecialRoutineContext {
        move_catalog,
        cry_by_species: &EMPTY_TEST_CRIES,
        species_catalog,
        learnsets,
        growth_rates: &TEST_GROWTH_RATES,
        item_catalog,
        runtime_spawn_points: &EMPTY_TEST_SPAWNS,
        roaming_pokemon: &EMPTY_TEST_ROAMERS,
        buena_password_categories: &EMPTY_TEST_BUENA_PASSWORD_CATEGORIES,
        buena_prizes: &EMPTY_TEST_BUENA_PRIZES,
        kurt_apricorn_recipes: &EMPTY_TEST_KURT_APRICORN_RECIPES,
        shuckie_gift: Some(shuckie_gift),
        dratini_move_sets: &EMPTY_TEST_DRATINI_MOVE_SETS,
        bug_contest_config: None,
        battle_tower_rules: None,
        magikarp_lengths: &[],
        happiness_data: None,
        trainer_catalog: &EMPTY_TEST_TRAINERS,
        phone_contacts: &EMPTY_TEST_PHONE_CONTACTS,
        wild_encounters: &EMPTY_TEST_WILD_ENCOUNTERS,
        odd_egg_definitions: &[],
        oak_ratings: &[],
    }
}

fn full_context_with_dratini_move_sets<'a>(
    move_catalog: &'a BTreeMap<String, Move>,
    species_catalog: &'a BTreeMap<String, PokemonSpecies>,
    learnsets: &'a SpeciesLearnsets,
    item_catalog: &'a BTreeMap<String, Item>,
    dratini_move_sets: &'a DratiniMoveSets,
) -> SpecialRoutineContext<'a> {
    SpecialRoutineContext {
        move_catalog,
        cry_by_species: &EMPTY_TEST_CRIES,
        species_catalog,
        learnsets,
        growth_rates: &TEST_GROWTH_RATES,
        item_catalog,
        runtime_spawn_points: &EMPTY_TEST_SPAWNS,
        roaming_pokemon: &EMPTY_TEST_ROAMERS,
        buena_password_categories: &EMPTY_TEST_BUENA_PASSWORD_CATEGORIES,
        buena_prizes: &EMPTY_TEST_BUENA_PRIZES,
        kurt_apricorn_recipes: &EMPTY_TEST_KURT_APRICORN_RECIPES,
        shuckie_gift: None,
        dratini_move_sets,
        bug_contest_config: None,
        battle_tower_rules: None,
        magikarp_lengths: &[],
        happiness_data: None,
        trainer_catalog: &EMPTY_TEST_TRAINERS,
        phone_contacts: &EMPTY_TEST_PHONE_CONTACTS,
        wild_encounters: &EMPTY_TEST_WILD_ENCOUNTERS,
        odd_egg_definitions: &[],
        oak_ratings: &[],
    }
}

fn full_context_with_bug_contest_config<'a>(
    move_catalog: &'a BTreeMap<String, Move>,
    species_catalog: &'a BTreeMap<String, PokemonSpecies>,
    learnsets: &'a SpeciesLearnsets,
    item_catalog: &'a BTreeMap<String, Item>,
    bug_contest_config: &'a BugContestConfig,
) -> SpecialRoutineContext<'a> {
    SpecialRoutineContext {
        move_catalog,
        cry_by_species: &EMPTY_TEST_CRIES,
        species_catalog,
        learnsets,
        growth_rates: &TEST_GROWTH_RATES,
        item_catalog,
        runtime_spawn_points: &EMPTY_TEST_SPAWNS,
        roaming_pokemon: &EMPTY_TEST_ROAMERS,
        buena_password_categories: &EMPTY_TEST_BUENA_PASSWORD_CATEGORIES,
        buena_prizes: &EMPTY_TEST_BUENA_PRIZES,
        kurt_apricorn_recipes: &EMPTY_TEST_KURT_APRICORN_RECIPES,
        shuckie_gift: None,
        dratini_move_sets: &EMPTY_TEST_DRATINI_MOVE_SETS,
        bug_contest_config: Some(bug_contest_config),
        battle_tower_rules: None,
        magikarp_lengths: &EMPTY_TEST_MAGIKARP_LENGTHS,
        happiness_data: None,
        trainer_catalog: &EMPTY_TEST_TRAINERS,
        phone_contacts: &EMPTY_TEST_PHONE_CONTACTS,
        wild_encounters: &EMPTY_TEST_WILD_ENCOUNTERS,
        odd_egg_definitions: &[],
        oak_ratings: &[],
    }
}

fn test_bug_contest_encounters() -> Vec<BugContestEncounterEntry> {
    let mut encounters = (0..10)
        .map(|_| BugContestEncounterEntry {
            weight: 10,
            species: "CATERPIE".to_string(),
            min_level: 7,
            max_level: 18,
        })
        .collect::<Vec<_>>();
    encounters.push(BugContestEncounterEntry {
        weight: u8::MAX,
        species: "VENOMOTH".to_string(),
        min_level: 30,
        max_level: 40,
    });
    encounters
}

fn full_context_with_battle_tower_rules<'a>(
    move_catalog: &'a BTreeMap<String, Move>,
    battle_tower_rules: &'a BattleTowerRules,
) -> SpecialRoutineContext<'a> {
    SpecialRoutineContext {
        move_catalog,
        cry_by_species: &EMPTY_TEST_CRIES,
        species_catalog: &EMPTY_TEST_SPECIES,
        learnsets: &EMPTY_TEST_LEARNSETS,
        growth_rates: &TEST_GROWTH_RATES,
        item_catalog: &EMPTY_TEST_ITEMS,
        runtime_spawn_points: &EMPTY_TEST_SPAWNS,
        roaming_pokemon: &EMPTY_TEST_ROAMERS,
        buena_password_categories: &EMPTY_TEST_BUENA_PASSWORD_CATEGORIES,
        buena_prizes: &EMPTY_TEST_BUENA_PRIZES,
        kurt_apricorn_recipes: &EMPTY_TEST_KURT_APRICORN_RECIPES,
        shuckie_gift: None,
        dratini_move_sets: &EMPTY_TEST_DRATINI_MOVE_SETS,
        bug_contest_config: None,
        battle_tower_rules: Some(battle_tower_rules),
        magikarp_lengths: &EMPTY_TEST_MAGIKARP_LENGTHS,
        happiness_data: None,
        trainer_catalog: &EMPTY_TEST_TRAINERS,
        phone_contacts: &EMPTY_TEST_PHONE_CONTACTS,
        wild_encounters: &EMPTY_TEST_WILD_ENCOUNTERS,
        odd_egg_definitions: &[],
        oak_ratings: &[],
    }
}

fn battle_tower_rules_with_banned_species(banned_species: Vec<String>) -> BattleTowerRules {
    BattleTowerRules {
        banned_species: banned_species
            .into_iter()
            .map(|species_id| (species_id, BattleTowerBannedSpeciesRule::default()))
            .collect(),
        required_party_count: 3,
        challenge_streak_length: 7,
        reward_candidates: [
            "HP_UP",
            "PROTEIN",
            "IRON",
            "CARBOS",
            "LUCKY_PUNCH",
            "CALCIUM",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
        excluded_reward_items: vec!["LUCKY_PUNCH".to_string()],
        reward_quantity: 5,
        reward_failure_sentinel: "POTION".to_string(),
        reward_item_values: BTreeMap::from([
            ("POTION".to_string(), 0x12),
            ("HP_UP".to_string(), 0x1a),
            ("PROTEIN".to_string(), 0x1b),
            ("IRON".to_string(), 0x1c),
            ("CARBOS".to_string(), 0x1d),
            ("LUCKY_PUNCH".to_string(), 0x1e),
            ("CALCIUM".to_string(), 0x1f),
        ]),
        minimum_level_group: 1,
        maximum_level_group: 10,
        level_group_size: 10,
        party_count_failure_text: "OnlyThreeMonMayBeEnteredText".to_string(),
        duplicate_species_failure_text: "TheMonMustAllBeDifferentKindsText".to_string(),
        duplicate_held_item_failure_text: "TheMonMustNotHoldTheSameItemsText".to_string(),
        egg_failure_text: "YouCantTakeAnEggText".to_string(),
        trainers: Vec::new(),
        mon_groups: Vec::new(),
    }
}

fn full_context_with_oak_ratings<'a>(
    move_catalog: &'a BTreeMap<String, Move>,
    oak_ratings: &'a [OakRatingEntry],
) -> SpecialRoutineContext<'a> {
    SpecialRoutineContext {
        move_catalog,
        cry_by_species: &EMPTY_TEST_CRIES,
        species_catalog: &EMPTY_TEST_SPECIES,
        learnsets: &EMPTY_TEST_LEARNSETS,
        growth_rates: &TEST_GROWTH_RATES,
        item_catalog: &EMPTY_TEST_ITEMS,
        runtime_spawn_points: &EMPTY_TEST_SPAWNS,
        roaming_pokemon: &EMPTY_TEST_ROAMERS,
        buena_password_categories: &EMPTY_TEST_BUENA_PASSWORD_CATEGORIES,
        buena_prizes: &EMPTY_TEST_BUENA_PRIZES,
        kurt_apricorn_recipes: &EMPTY_TEST_KURT_APRICORN_RECIPES,
        shuckie_gift: None,
        dratini_move_sets: &EMPTY_TEST_DRATINI_MOVE_SETS,
        bug_contest_config: None,
        battle_tower_rules: None,
        magikarp_lengths: &EMPTY_TEST_MAGIKARP_LENGTHS,
        happiness_data: None,
        trainer_catalog: &EMPTY_TEST_TRAINERS,
        phone_contacts: &EMPTY_TEST_PHONE_CONTACTS,
        wild_encounters: &EMPTY_TEST_WILD_ENCOUNTERS,
        odd_egg_definitions: &[],
        oak_ratings,
    }
}

fn full_context_with_odd_egg_definitions<'a>(
    move_catalog: &'a BTreeMap<String, Move>,
    species_catalog: &'a BTreeMap<String, PokemonSpecies>,
    learnsets: &'a SpeciesLearnsets,
    odd_egg_definitions: &'a [OddEggDefinition],
) -> SpecialRoutineContext<'a> {
    SpecialRoutineContext {
        move_catalog,
        cry_by_species: &EMPTY_TEST_CRIES,
        species_catalog,
        learnsets,
        growth_rates: &TEST_GROWTH_RATES,
        item_catalog: &EMPTY_TEST_ITEMS,
        runtime_spawn_points: &EMPTY_TEST_SPAWNS,
        roaming_pokemon: &EMPTY_TEST_ROAMERS,
        buena_password_categories: &EMPTY_TEST_BUENA_PASSWORD_CATEGORIES,
        buena_prizes: &EMPTY_TEST_BUENA_PRIZES,
        kurt_apricorn_recipes: &EMPTY_TEST_KURT_APRICORN_RECIPES,
        shuckie_gift: None,
        dratini_move_sets: &EMPTY_TEST_DRATINI_MOVE_SETS,
        bug_contest_config: None,
        battle_tower_rules: None,
        magikarp_lengths: &EMPTY_TEST_MAGIKARP_LENGTHS,
        happiness_data: None,
        trainer_catalog: &EMPTY_TEST_TRAINERS,
        phone_contacts: &EMPTY_TEST_PHONE_CONTACTS,
        wild_encounters: &EMPTY_TEST_WILD_ENCOUNTERS,
        odd_egg_definitions,
        oak_ratings: &[],
    }
}

fn full_context_with_magikarp_lengths<'a>(
    move_catalog: &'a BTreeMap<String, Move>,
    species_catalog: &'a BTreeMap<String, PokemonSpecies>,
    learnsets: &'a SpeciesLearnsets,
    item_catalog: &'a BTreeMap<String, Item>,
    magikarp_lengths: &'a [MagikarpLengthEntry],
) -> SpecialRoutineContext<'a> {
    SpecialRoutineContext {
        move_catalog,
        cry_by_species: &EMPTY_TEST_CRIES,
        species_catalog,
        learnsets,
        growth_rates: &TEST_GROWTH_RATES,
        item_catalog,
        runtime_spawn_points: &EMPTY_TEST_SPAWNS,
        roaming_pokemon: &EMPTY_TEST_ROAMERS,
        buena_password_categories: &EMPTY_TEST_BUENA_PASSWORD_CATEGORIES,
        buena_prizes: &EMPTY_TEST_BUENA_PRIZES,
        kurt_apricorn_recipes: &EMPTY_TEST_KURT_APRICORN_RECIPES,
        shuckie_gift: None,
        dratini_move_sets: &EMPTY_TEST_DRATINI_MOVE_SETS,
        bug_contest_config: None,
        battle_tower_rules: None,
        magikarp_lengths,
        happiness_data: None,
        trainer_catalog: &EMPTY_TEST_TRAINERS,
        phone_contacts: &EMPTY_TEST_PHONE_CONTACTS,
        wild_encounters: &EMPTY_TEST_WILD_ENCOUNTERS,
        odd_egg_definitions: &[],
        oak_ratings: &[],
    }
}

fn full_context_with_happiness_data<'a>(
    move_catalog: &'a BTreeMap<String, Move>,
    species_catalog: &'a BTreeMap<String, PokemonSpecies>,
    learnsets: &'a SpeciesLearnsets,
    item_catalog: &'a BTreeMap<String, Item>,
    happiness_data: &'a HappinessData,
) -> SpecialRoutineContext<'a> {
    SpecialRoutineContext {
        move_catalog,
        cry_by_species: &EMPTY_TEST_CRIES,
        species_catalog,
        learnsets,
        growth_rates: &TEST_GROWTH_RATES,
        item_catalog,
        runtime_spawn_points: &EMPTY_TEST_SPAWNS,
        roaming_pokemon: &EMPTY_TEST_ROAMERS,
        buena_password_categories: &EMPTY_TEST_BUENA_PASSWORD_CATEGORIES,
        buena_prizes: &EMPTY_TEST_BUENA_PRIZES,
        kurt_apricorn_recipes: &EMPTY_TEST_KURT_APRICORN_RECIPES,
        shuckie_gift: None,
        dratini_move_sets: &EMPTY_TEST_DRATINI_MOVE_SETS,
        bug_contest_config: None,
        battle_tower_rules: None,
        magikarp_lengths: &EMPTY_TEST_MAGIKARP_LENGTHS,
        happiness_data: Some(happiness_data),
        trainer_catalog: &EMPTY_TEST_TRAINERS,
        phone_contacts: &EMPTY_TEST_PHONE_CONTACTS,
        wild_encounters: &EMPTY_TEST_WILD_ENCOUNTERS,
        odd_egg_definitions: &[],
        oak_ratings: &[],
    }
}

fn full_context_with_kurt_apricorn_recipes<'a>(
    move_catalog: &'a BTreeMap<String, Move>,
    species_catalog: &'a BTreeMap<String, PokemonSpecies>,
    learnsets: &'a SpeciesLearnsets,
    item_catalog: &'a BTreeMap<String, Item>,
    kurt_apricorn_recipes: &'a KurtApricornRecipes,
) -> SpecialRoutineContext<'a> {
    SpecialRoutineContext {
        move_catalog,
        cry_by_species: &EMPTY_TEST_CRIES,
        species_catalog,
        learnsets,
        growth_rates: &TEST_GROWTH_RATES,
        item_catalog,
        runtime_spawn_points: &EMPTY_TEST_SPAWNS,
        roaming_pokemon: &EMPTY_TEST_ROAMERS,
        buena_password_categories: &EMPTY_TEST_BUENA_PASSWORD_CATEGORIES,
        buena_prizes: &EMPTY_TEST_BUENA_PRIZES,
        kurt_apricorn_recipes,
        shuckie_gift: None,
        dratini_move_sets: &EMPTY_TEST_DRATINI_MOVE_SETS,
        bug_contest_config: None,
        battle_tower_rules: None,
        magikarp_lengths: &[],
        happiness_data: None,
        trainer_catalog: &EMPTY_TEST_TRAINERS,
        phone_contacts: &EMPTY_TEST_PHONE_CONTACTS,
        wild_encounters: &EMPTY_TEST_WILD_ENCOUNTERS,
        odd_egg_definitions: &[],
        oak_ratings: &[],
    }
}

fn full_context_with_buena_password_categories<'a>(
    move_catalog: &'a BTreeMap<String, Move>,
    species_catalog: &'a BTreeMap<String, PokemonSpecies>,
    learnsets: &'a SpeciesLearnsets,
    item_catalog: &'a BTreeMap<String, Item>,
    buena_password_categories: &'a BuenaPasswordCategories,
) -> SpecialRoutineContext<'a> {
    SpecialRoutineContext {
        move_catalog,
        cry_by_species: &EMPTY_TEST_CRIES,
        species_catalog,
        learnsets,
        growth_rates: &TEST_GROWTH_RATES,
        item_catalog,
        runtime_spawn_points: &EMPTY_TEST_SPAWNS,
        roaming_pokemon: &EMPTY_TEST_ROAMERS,
        buena_password_categories,
        buena_prizes: &EMPTY_TEST_BUENA_PRIZES,
        kurt_apricorn_recipes: &EMPTY_TEST_KURT_APRICORN_RECIPES,
        shuckie_gift: None,
        dratini_move_sets: &EMPTY_TEST_DRATINI_MOVE_SETS,
        bug_contest_config: None,
        battle_tower_rules: None,
        magikarp_lengths: &[],
        happiness_data: None,
        trainer_catalog: &EMPTY_TEST_TRAINERS,
        phone_contacts: &EMPTY_TEST_PHONE_CONTACTS,
        wild_encounters: &EMPTY_TEST_WILD_ENCOUNTERS,
        odd_egg_definitions: &[],
        oak_ratings: &[],
    }
}

fn full_context_with_buena_prizes<'a>(
    move_catalog: &'a BTreeMap<String, Move>,
    species_catalog: &'a BTreeMap<String, PokemonSpecies>,
    learnsets: &'a SpeciesLearnsets,
    item_catalog: &'a BTreeMap<String, Item>,
    buena_prizes: &'a BuenaPrizeDefinitions,
) -> SpecialRoutineContext<'a> {
    SpecialRoutineContext {
        move_catalog,
        cry_by_species: &EMPTY_TEST_CRIES,
        species_catalog,
        learnsets,
        growth_rates: &TEST_GROWTH_RATES,
        item_catalog,
        runtime_spawn_points: &EMPTY_TEST_SPAWNS,
        roaming_pokemon: &EMPTY_TEST_ROAMERS,
        buena_password_categories: &EMPTY_TEST_BUENA_PASSWORD_CATEGORIES,
        buena_prizes,
        kurt_apricorn_recipes: &EMPTY_TEST_KURT_APRICORN_RECIPES,
        shuckie_gift: None,
        dratini_move_sets: &EMPTY_TEST_DRATINI_MOVE_SETS,
        bug_contest_config: None,
        battle_tower_rules: None,
        magikarp_lengths: &[],
        happiness_data: None,
        trainer_catalog: &EMPTY_TEST_TRAINERS,
        phone_contacts: &EMPTY_TEST_PHONE_CONTACTS,
        wild_encounters: &EMPTY_TEST_WILD_ENCOUNTERS,
        odd_egg_definitions: &[],
        oak_ratings: &[],
    }
}

fn full_context_with_roamers<'a>(
    move_catalog: &'a BTreeMap<String, Move>,
    species_catalog: &'a BTreeMap<String, PokemonSpecies>,
    learnsets: &'a SpeciesLearnsets,
    item_catalog: &'a BTreeMap<String, Item>,
    roaming_pokemon: &'a RoamingPokemonCatalog,
) -> SpecialRoutineContext<'a> {
    SpecialRoutineContext {
        move_catalog,
        cry_by_species: &EMPTY_TEST_CRIES,
        species_catalog,
        learnsets,
        growth_rates: &TEST_GROWTH_RATES,
        item_catalog,
        runtime_spawn_points: &EMPTY_TEST_SPAWNS,
        roaming_pokemon,
        buena_password_categories: &EMPTY_TEST_BUENA_PASSWORD_CATEGORIES,
        buena_prizes: &EMPTY_TEST_BUENA_PRIZES,
        kurt_apricorn_recipes: &EMPTY_TEST_KURT_APRICORN_RECIPES,
        shuckie_gift: None,
        dratini_move_sets: &EMPTY_TEST_DRATINI_MOVE_SETS,
        bug_contest_config: None,
        battle_tower_rules: None,
        magikarp_lengths: &[],
        happiness_data: None,
        trainer_catalog: &EMPTY_TEST_TRAINERS,
        phone_contacts: &EMPTY_TEST_PHONE_CONTACTS,
        wild_encounters: &EMPTY_TEST_WILD_ENCOUNTERS,
        odd_egg_definitions: &[],
        oak_ratings: &[],
    }
}

fn spawn_point(
    identifier: u16,
    map_name: &str,
    group_id: i16,
    map_id: i16,
    tile_x: i16,
    tile_y: i16,
) -> RuntimeSpawnPointRef {
    runtime_spawn_point_from_runtime_tile(
        identifier,
        map_name.to_string(),
        map_name.to_string(),
        group_id,
        map_id,
        "GROUP".to_string(),
        TilePosition::new(tile_x, tile_y),
    )
    .expect("test spawn point must be representable")
}

fn trainer_context<'a>(
    move_catalog: &'a BTreeMap<String, Move>,
    species_catalog: &'a BTreeMap<String, PokemonSpecies>,
    learnsets: &'a SpeciesLearnsets,
    trainer_catalog: &'a TrainerCatalog,
) -> SpecialRoutineContext<'a> {
    SpecialRoutineContext {
        move_catalog,
        cry_by_species: &EMPTY_TEST_CRIES,
        species_catalog,
        learnsets,
        growth_rates: &TEST_GROWTH_RATES,
        item_catalog: &EMPTY_TEST_ITEMS,
        runtime_spawn_points: &EMPTY_TEST_SPAWNS,
        roaming_pokemon: &EMPTY_TEST_ROAMERS,
        buena_password_categories: &EMPTY_TEST_BUENA_PASSWORD_CATEGORIES,
        buena_prizes: &EMPTY_TEST_BUENA_PRIZES,
        kurt_apricorn_recipes: &EMPTY_TEST_KURT_APRICORN_RECIPES,
        shuckie_gift: None,
        dratini_move_sets: &EMPTY_TEST_DRATINI_MOVE_SETS,
        bug_contest_config: None,
        battle_tower_rules: None,
        magikarp_lengths: &[],
        happiness_data: None,
        trainer_catalog,
        phone_contacts: &EMPTY_TEST_PHONE_CONTACTS,
        wild_encounters: &EMPTY_TEST_WILD_ENCOUNTERS,
        odd_egg_definitions: &[],
        oak_ratings: &[],
    }
}

fn test_odd_egg_definitions() -> Vec<OddEggDefinition> {
    vec![
        OddEggDefinition {
            species: "PICHU".to_string(),
            moves: vec!["THUNDERSHOCK".to_string()],
            original_trainer_id: 2048,
            dvs: [0, 0, 0, 0],
            probability: 8,
            level: 5,
            experience: 125,
            hatch_cycles: 20,
            nickname: "EGG".to_string(),
            original_trainer_name: "ODD".to_string(),
        },
        OddEggDefinition {
            species: "PICHU".to_string(),
            moves: vec!["THUNDERSHOCK".to_string()],
            original_trainer_id: 256,
            dvs: [2, 10, 10, 10],
            probability: 1,
            level: 5,
            experience: 125,
            hatch_cycles: 20,
            nickname: "EGG".to_string(),
            original_trainer_name: "ODD".to_string(),
        },
        OddEggDefinition {
            species: "CLEFFA".to_string(),
            moves: vec![
                "POUND".to_string(),
                "CHARM".to_string(),
                "DIZZY_PUNCH".to_string(),
            ],
            original_trainer_id: 4096,
            dvs: [0, 0, 0, 0],
            probability: 16,
            level: 5,
            experience: 125,
            hatch_cycles: 20,
            nickname: "EGG".to_string(),
            original_trainer_name: "ODD".to_string(),
        },
        OddEggDefinition {
            species: "CLEFFA".to_string(),
            moves: vec![
                "POUND".to_string(),
                "CHARM".to_string(),
                "DIZZY_PUNCH".to_string(),
            ],
            original_trainer_id: 768,
            dvs: [2, 10, 10, 10],
            probability: 75,
            level: 5,
            experience: 125,
            hatch_cycles: 20,
            nickname: "EGG".to_string(),
            original_trainer_name: "ODD".to_string(),
        },
    ]
}

#[test]
fn play_cur_mon_cry_uses_exact_declared_current_species_cry() {
    let mut state = GameState::default();
    state
        .script_runtime
        .variables
        .insert("wCurPartySpecies".to_string(), "CHIKORITA".to_string());
    let moves = moves();
    let cries = BTreeMap::from([("CHIKORITA".to_string(), "CRY_CHIKORITA".to_string())]);
    let species = BTreeMap::new();

    let outcome = apply_special_routine_with_context(
        &mut state,
        cry_context(&moves, &cries, &species),
        "PlayCurMonCry",
    )
    .expect("play current cry");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::PlayCurMonCry {
            species: "CHIKORITA".to_string(),
            audio_id: "CRY_CHIKORITA".to_string()
        }
    );
    assert_eq!(state.script_runtime.audio_events.len(), 1);
    assert_eq!(
        state.script_runtime.audio_events[0].kind,
        ScriptAudioRuntimeKind::Cry
    );
    assert_eq!(
        state.script_runtime.audio_events[0].audio_id.as_deref(),
        Some("CRY_CHIKORITA")
    );
    assert_eq!(
        state.script_runtime.last_special_routine.as_deref(),
        Some("PlayCurMonCry")
    );

    let mut missing_current = GameState::default();
    missing_current
        .storage
        .register_capture_in_box(0, pokemon("CHIKORITA"))
        .expect("store party mon");
    missing_current.sync_party_from_storage();
    let before_missing_current = missing_current.clone();
    let error = apply_special_routine_with_context(
        &mut missing_current,
        cry_context(&moves, &cries, &species),
        "PlayCurMonCry",
    )
    .expect_err("current cry must require wCurPartySpecies");
    assert_eq!(
        error,
        SpecialRoutineError::MissingCurrentPartySpecies {
            routine: "PlayCurMonCry".to_string()
        }
    );
    assert_eq!(missing_current, before_missing_current);
}

#[test]
fn play_slow_cry_uses_exact_value_species_cry_without_case_coercion() {
    let mut state = GameState::default();
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "LUGIA".to_string());
    let moves = moves();
    let cries = BTreeMap::from([("LUGIA".to_string(), "CRY_LUGIA".to_string())]);
    let species = BTreeMap::new();

    let outcome = apply_special_routine_with_context(
        &mut state,
        cry_context(&moves, &cries, &species),
        "PlaySlowCry",
    )
    .expect("play slow cry");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::PlaySlowCry {
            species: "LUGIA".to_string(),
            audio_id: "CRY_LUGIA".to_string()
        }
    );
    assert_eq!(
        state.script_runtime.audio_events[0].audio_id.as_deref(),
        Some("CRY_LUGIA")
    );

    let mut case_state = GameState::default();
    case_state
        .script_runtime
        .variables
        .insert("_value".to_string(), "lugia".to_string());
    let before = case_state.clone();
    let error = apply_special_routine_with_context(
        &mut case_state,
        cry_context(&moves, &cries, &species),
        "PlaySlowCry",
    )
    .expect_err("case exact cry species");

    assert!(matches!(
        error,
        SpecialRoutineError::MissingCryMetadata { routine, species }
            if routine == "PlaySlowCry" && species == "lugia"
    ));
    assert_eq!(case_state, before);
}

#[test]
fn play_cur_mon_cry_requires_declared_modpack_cry() {
    let mut state = GameState::default();
    state
        .storage
        .register_capture_in_box(0, pokemon("CHIKORITA"))
        .expect("store");
    state.sync_party_from_storage();
    state
        .script_runtime
        .variables
        .insert("wCurPartySpecies".to_string(), "CHIKORITA".to_string());
    let before = state.clone();
    let moves = moves();
    let cries = BTreeMap::new();
    let species = BTreeMap::new();

    let error = apply_special_routine_with_context(
        &mut state,
        cry_context(&moves, &cries, &species),
        "PlayCurMonCry",
    )
    .expect_err("missing cry");

    assert!(matches!(
        error,
        SpecialRoutineError::MissingCryMetadata { routine, species }
            if routine == "PlayCurMonCry" && species == "CHIKORITA"
    ));
    assert_eq!(state, before);
}

#[test]
fn heal_party_restores_hp_status_and_pp_from_exact_move_catalog() {
    let mut state = GameState::default();
    state
        .storage
        .register_capture_in_box(0, pokemon("CHIKORITA"))
        .expect("store first");
    state.sync_party_from_storage();

    let outcome = apply_special_routine(&mut state, &moves(), "HealParty").expect("heal party");

    assert_eq!(outcome.routine, "HealParty");
    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::HealParty {
            healed_slots: vec![0]
        }
    );
    assert_eq!(
        state.script_runtime.last_special_routine.as_deref(),
        Some("HealParty")
    );
    for slot in 0..1 {
        let pokemon = state.storage.party.pokemon[slot]
            .as_ref()
            .expect("party pokemon");
        assert_eq!(pokemon.hp, pokemon.max_hp);
        assert_eq!(pokemon.status, None);
        assert_eq!(pokemon.sleep_turns, 0);
        assert_eq!(pokemon.confusion_turns, 2);
        assert!(pokemon.focus_energy);
        assert_eq!(pokemon.moves[0].current_pp, 42);
        assert_eq!(pokemon.moves[1].current_pp, 40);
        assert_eq!(
            state.party.pokemon[slot]
                .as_ref()
                .expect("projected party")
                .species
                .as_str(),
            pokemon.species.id
        );
    }
}

#[test]
fn special_routines_reject_unknown_or_case_changed_routines_without_mutation() {
    let mut state = GameState::default();
    state
        .storage
        .register_capture_in_box(0, pokemon("CHIKORITA"))
        .expect("store");
    state.sync_party_from_storage();
    let before = state.clone();

    let error = apply_special_routine(&mut state, &moves(), "healparty").expect_err("case exact");

    assert!(matches!(
        error,
        SpecialRoutineError::UnsupportedRoutine { routine } if routine == "healparty"
    ));
    assert_eq!(state, before);
}

#[test]
fn heal_party_rejects_unknown_move_without_partial_healing() {
    let mut state = GameState::default();
    let mut pokemon = pokemon("CHIKORITA");
    pokemon.moves[0].name = "tackle".to_string();
    state
        .storage
        .register_capture_in_box(0, pokemon)
        .expect("store");
    state.sync_party_from_storage();
    let before = state.clone();

    assert_eq!(
        apply_special_routine(&mut state, &moves(), "HealParty")
            .expect_err("unknown party move must not receive a partial heal"),
        SpecialRoutineError::UnknownMove {
            routine: "HealParty".to_string(),
            party_slot: 0,
            move_id: "tackle".to_string(),
        }
    );
    assert_eq!(state, before);
}

#[test]
fn fade_out_music_records_exact_two_frame_music_none_fade() {
    let mut state = GameState::default();
    state.script_runtime.current_music = Some("MUSIC_ROUTE_30".to_string());

    let outcome = apply_special_routine(&mut state, &moves(), "FadeOutMusic").expect("fade music");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::FadeOutMusic {
            audio_id: "MUSIC_NONE".to_string(),
            fade_frames: 2
        }
    );
    assert_eq!(
        state.script_runtime.last_special_routine.as_deref(),
        Some("FadeOutMusic")
    );
    let fade = state
        .script_runtime
        .pending_music_fade
        .as_ref()
        .expect("pending fade");
    assert_eq!(fade.audio_id, "MUSIC_NONE");
    assert_eq!(fade.fade_frames, 2);
    assert_eq!(fade.source_script, "FadeOutMusic");
    assert_eq!(
        state.script_runtime.current_music.as_deref(),
        Some("MUSIC_ROUTE_30")
    );
    assert_eq!(state.script_runtime.audio_events.len(), 1);
    assert_eq!(
        state.script_runtime.audio_events[0].kind,
        ScriptAudioRuntimeKind::FadeMusic
    );
    assert_eq!(
        state.script_runtime.audio_events[0].audio_id.as_deref(),
        Some("MUSIC_NONE")
    );
    assert_eq!(state.script_runtime.audio_events[0].fade_frames, Some(2));
}

#[test]
fn restart_map_music_preserves_the_dont_restart_map_music_flag() {
    let mut state = GameState::default();
    state.script_runtime.map_music_restart_disabled = true;

    let outcome =
        apply_special_routine(&mut state, &moves(), "RestartMapMusic").expect("restart music");

    assert_eq!(outcome.effect, SpecialRoutineEffect::RestartMapMusic);
    assert!(state.script_runtime.map_music_requested);
    assert!(state.script_runtime.map_music_restart_disabled);
    assert_eq!(
        state.script_runtime.last_special_routine.as_deref(),
        Some("RestartMapMusic")
    );
}

#[test]
fn wait_sfx_records_exact_sound_effect_wait_without_audio_id() {
    let mut state = GameState::default();

    let outcome = apply_special_routine(&mut state, &moves(), "WaitSFX").expect("wait sfx");

    assert_eq!(outcome.effect, SpecialRoutineEffect::WaitSfx);
    assert!(state.script_runtime.waiting_for_sound_effect);
    assert_eq!(
        state.script_runtime.last_special_routine.as_deref(),
        Some("WaitSFX")
    );
    assert_eq!(state.script_runtime.audio_events.len(), 1);
    assert_eq!(
        state.script_runtime.audio_events[0].kind,
        ScriptAudioRuntimeKind::WaitForSoundEffect
    );
    assert_eq!(state.script_runtime.audio_events[0].audio_id, None);
    assert_eq!(state.script_runtime.audio_events[0].fade_frames, None);
    assert_eq!(
        state.script_runtime.audio_events[0].source_script,
        "WaitSFX"
    );
}

#[test]
fn play_map_music_preserves_the_dont_restart_map_music_flag() {
    let mut state = GameState::default();
    state.script_runtime.map_music_restart_disabled = true;

    let outcome =
        apply_special_routine(&mut state, &moves(), "PlayMapMusic").expect("play map music");

    assert_eq!(outcome.effect, SpecialRoutineEffect::PlayMapMusic);
    assert!(state.script_runtime.map_music_requested);
    assert!(state.script_runtime.map_music_restart_disabled);
    assert_eq!(
        state.script_runtime.last_special_routine.as_deref(),
        Some("PlayMapMusic")
    );
}

#[test]
fn gameboy_check_sets_exact_cgb_token() {
    let mut state = GameState::default();

    let outcome =
        apply_special_routine(&mut state, &moves(), "GameboyCheck").expect("gameboy check");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::GameboyCheck {
            token: "GBCHECK_CGB".to_string()
        }
    );
    assert_eq!(
        state.script_runtime.last_special_routine.as_deref(),
        Some("GameboyCheck")
    );
    assert_eq!(
        state.script_runtime.script_value.as_deref(),
        Some("GBCHECK_CGB")
    );
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("_value")
            .map(String::as_str),
        Some("GBCHECK_CGB")
    );
}

#[test]
fn mobile_adapter_status_sets_exact_zero_value() {
    let mut state = GameState::default();

    let outcome = apply_special_routine(&mut state, &moves(), "CheckMobileAdapterStatusSpecial")
        .expect("mobile adapter status");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::MobileAdapterStatus {
            value: "0".to_string()
        }
    );
    assert_eq!(
        state.script_runtime.last_special_routine.as_deref(),
        Some("CheckMobileAdapterStatusSpecial")
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("_value")
            .map(String::as_str),
        Some("0")
    );
}

#[test]
fn get_first_pokemon_happiness_uses_first_non_egg_party_member() {
    let mut state = GameState::default();
    let mut egg = pokemon("TOGEPI");
    egg.is_egg = true;
    egg.nickname = "EGG".to_string();
    egg.happiness = 1;
    let mut chikorita = pokemon("CHIKORITA");
    chikorita.nickname = "Leafy".to_string();
    chikorita.happiness = 218;
    state
        .storage
        .register_capture_in_box(0, egg)
        .expect("store egg");
    state
        .storage
        .register_capture_in_box(0, chikorita)
        .expect("store mon");
    state.sync_party_from_storage();

    let outcome =
        apply_special_routine(&mut state, &moves(), "GetFirstPokemonHappiness").expect("happiness");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::FirstPokemonHappiness {
            party_slot: 1,
            species: "CHIKORITA".to_string(),
            nickname: "Leafy".to_string(),
            happiness: 218
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("218"));
    assert_eq!(
        state
            .script_runtime
            .named_buffers
            .get("STRING_BUFFER_3")
            .map(String::as_str),
        Some("Leafy")
    );
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("wCurPartySpecies")
            .map(String::as_str),
        Some("CHIKORITA")
    );
}

#[test]
fn get_first_pokemon_happiness_rejects_all_egg_party_without_mutation() {
    let mut state = GameState::default();
    let mut egg = pokemon("TOGEPI");
    egg.is_egg = true;
    egg.nickname = "EGG".to_string();
    state
        .storage
        .register_capture_in_box(0, egg)
        .expect("store egg");
    state.sync_party_from_storage();
    let before = state.clone();

    let error = apply_special_routine(&mut state, &moves(), "GetFirstPokemonHappiness")
        .expect_err("all egg rejected");

    assert!(matches!(
        error,
        SpecialRoutineError::NoNonEggPartyPokemon { routine }
            if routine == "GetFirstPokemonHappiness"
    ));
    assert_eq!(state, before);
}

#[test]
fn check_first_mon_is_egg_sets_exact_value_and_buffer() {
    let mut state = GameState::default();
    let mut egg = pokemon("TOGEPI");
    egg.is_egg = true;
    egg.nickname = "EGG".to_string();
    state
        .storage
        .register_capture_in_box(0, egg)
        .expect("store egg");
    state.sync_party_from_storage();

    let outcome =
        apply_special_routine(&mut state, &moves(), "CheckFirstMonIsEgg").expect("egg check");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::CheckFirstMonIsEgg {
            species: "EGG".to_string(),
            nickname: "EGG".to_string(),
            is_egg: true
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));
    assert_eq!(
        state
            .script_runtime
            .named_buffers
            .get("STRING_BUFFER_3")
            .map(String::as_str),
        Some("EGG")
    );
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("wCurPartySpecies")
            .map(String::as_str),
        Some("EGG")
    );

    let mut non_egg_state = GameState::default();
    let mut chikorita = pokemon("CHIKORITA");
    chikorita.nickname.clear();
    non_egg_state
        .storage
        .register_capture_in_box(0, chikorita)
        .expect("store mon");
    non_egg_state.sync_party_from_storage();

    let non_egg = apply_special_routine(&mut non_egg_state, &moves(), "CheckFirstMonIsEgg")
        .expect("non-egg check");

    assert_eq!(
        non_egg.effect,
        SpecialRoutineEffect::CheckFirstMonIsEgg {
            species: "CHIKORITA".to_string(),
            nickname: "CHIKORITA".to_string(),
            is_egg: false
        }
    );
    assert_eq!(
        non_egg_state.script_runtime.script_value.as_deref(),
        Some("0")
    );
}

#[test]
fn check_first_mon_is_egg_rejects_empty_party_without_mutation() {
    let mut state = GameState::default();
    let before = state.clone();

    let error = apply_special_routine(&mut state, &moves(), "CheckFirstMonIsEgg")
        .expect_err("empty party rejected");

    assert!(matches!(
        error,
        SpecialRoutineError::EmptyParty { routine } if routine == "CheckFirstMonIsEgg"
    ));
    assert_eq!(state, before);
}

#[test]
fn find_party_mon_that_species_uses_exact_script_value_species() {
    let mut state = GameState::default();
    state
        .storage
        .register_capture_in_box(0, pokemon("CHIKORITA"))
        .expect("store");
    state.sync_party_from_storage();
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "CHIKORITA".to_string());

    let outcome = apply_special_routine(&mut state, &moves(), "FindPartyMonThatSpecies")
        .expect("find species");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::FindPartyMonThatSpecies {
            species: "CHIKORITA".to_string(),
            found: true
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("_value")
            .map(String::as_str),
        Some("1")
    );
    assert_eq!(
        state.script_runtime.last_special_routine.as_deref(),
        Some("FindPartyMonThatSpecies")
    );

    let mut case_state = GameState::default();
    case_state
        .storage
        .register_capture_in_box(0, pokemon("CHIKORITA"))
        .expect("store");
    case_state.sync_party_from_storage();
    case_state.script_runtime.script_value = Some("chikorita".to_string());

    let case_outcome = apply_special_routine(&mut case_state, &moves(), "FindPartyMonThatSpecies")
        .expect("case changed miss");

    assert_eq!(
        case_outcome.effect,
        SpecialRoutineEffect::FindPartyMonThatSpecies {
            species: "chikorita".to_string(),
            found: false
        }
    );
    assert_eq!(case_state.script_runtime.script_value.as_deref(), Some("0"));
}

#[test]
fn find_party_mon_that_species_rejects_missing_value_without_mutation() {
    let mut state = GameState::default();
    state
        .storage
        .register_capture_in_box(0, pokemon("CHIKORITA"))
        .expect("store");
    state.sync_party_from_storage();
    let before = state.clone();

    let error = apply_special_routine(&mut state, &moves(), "FindPartyMonThatSpecies")
        .expect_err("missing species rejected");

    assert!(matches!(
        error,
        SpecialRoutineError::MissingSpeciesValue { routine }
            if routine == "FindPartyMonThatSpecies"
    ));
    assert_eq!(state, before);
}

#[test]
fn party_query_specials_check_exact_levels_happiness_and_ot() {
    let mut state = GameState::default();
    state.player_name = "CHRIS".to_string();
    state.player_id = 0x1234;
    let mut chikorita = pokemon("CHIKORITA");
    chikorita.level = 31;
    chikorita.happiness = 220;
    chikorita.original_trainer_name = "CHRIS".to_string();
    chikorita.original_trainer_id = 0x1234;
    state
        .storage
        .register_capture_in_box(0, chikorita)
        .expect("store matching mon");
    state.sync_party_from_storage();

    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "30".to_string());
    let level =
        apply_special_routine(&mut state, &moves(), "FindPartyMonAboveLevel").expect("find level");

    assert_eq!(
        level.effect,
        SpecialRoutineEffect::FindPartyMonAboveLevel {
            level: 30,
            found: true,
            species: Some("CHIKORITA".to_string())
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));

    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "221".to_string());
    let happiness = apply_special_routine(&mut state, &moves(), "FindPartyMonAtLeastThatHappy")
        .expect("find happiness");

    assert_eq!(
        happiness.effect,
        SpecialRoutineEffect::FindPartyMonAtLeastThatHappy {
            happiness: 221,
            found: false,
            species: None
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));

    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "CHIKORITA".to_string());
    let ot_match =
        apply_special_routine(&mut state, &moves(), "FindPartyMonThatSpeciesYourTrainerID")
            .expect("find species ot");

    assert_eq!(
        ot_match.effect,
        SpecialRoutineEffect::FindPartyMonThatSpeciesYourTrainerId {
            species: "CHIKORITA".to_string(),
            player_name: "CHRIS".to_string(),
            player_id: 0x1234,
            found: true
        }
    );

    state.player_name = "Chris".to_string();
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "CHIKORITA".to_string());
    let case_miss =
        apply_special_routine(&mut state, &moves(), "FindPartyMonThatSpeciesYourTrainerID")
            .expect("exact ot name miss");

    assert_eq!(
        case_miss.effect,
        SpecialRoutineEffect::FindPartyMonThatSpeciesYourTrainerId {
            species: "CHIKORITA".to_string(),
            player_name: "Chris".to_string(),
            player_id: 0x1234,
            found: false
        }
    );
}

#[test]
fn mon_check_and_beasts_check_scan_party_and_pc_with_exact_ot() {
    let mut state = GameState::default();
    state.player_name = "KRIS".to_string();
    state.player_id = 0x2345;
    for species in ["RAIKOU", "ENTEI"] {
        let mut pokemon = pokemon(species);
        pokemon.original_trainer_name = "KRIS".to_string();
        pokemon.original_trainer_id = 0x2345;
        state
            .storage
            .register_capture_in_box(0, pokemon)
            .expect("store beast");
    }
    let mut box0 = PcBox::new(0);
    let mut suicune = pokemon("SUICUNE");
    suicune.original_trainer_name = "KRIS".to_string();
    suicune.original_trainer_id = 0x2345;
    assert!(box0.add_pokemon(suicune));
    state.storage.pc_boxes[0] = box0;
    state.sync_party_from_storage();

    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "SUICUNE".to_string());
    let mon = apply_special_routine(&mut state, &moves(), "MonCheck").expect("mon check");

    assert_eq!(
        mon.effect,
        SpecialRoutineEffect::MonCheck {
            species: "SUICUNE".to_string(),
            player_name: "KRIS".to_string(),
            player_id: 0x2345,
            owned: true
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));

    let beasts = apply_special_routine(&mut state, &moves(), "BeastsCheck").expect("beasts check");

    assert_eq!(
        beasts.effect,
        SpecialRoutineEffect::BeastsCheck {
            player_name: "KRIS".to_string(),
            player_id: 0x2345,
            missing_species: None,
            owned_all: true
        }
    );

    state.player_id = 0x9999;
    let beasts_miss =
        apply_special_routine(&mut state, &moves(), "BeastsCheck").expect("beasts miss");

    assert_eq!(
        beasts_miss.effect,
        SpecialRoutineEffect::BeastsCheck {
            player_name: "KRIS".to_string(),
            player_id: 0x9999,
            missing_species: Some("RAIKOU".to_string()),
            owned_all: false
        }
    );
}

#[test]
fn mon_check_rejects_invalid_pc_box_count_without_mutation() {
    let mut state = GameState::default();
    state.player_name = "KRIS".to_string();
    state.player_id = 1;
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "SUICUNE".to_string());
    let mut pc_box = PcBox::new(0);
    pc_box.count = MAX_BOX_MONS + 1;
    state.storage.pc_boxes[0] = pc_box;
    let before = state.clone();

    let error =
        apply_special_routine(&mut state, &moves(), "MonCheck").expect_err("invalid pc box");

    assert!(matches!(
        error,
        SpecialRoutineError::InvalidPcBoxCount {
            routine,
            box_index: 0,
            count
        } if routine == "MonCheck" && count == MAX_BOX_MONS + 1
    ));
    assert_eq!(state, before);
}

#[test]
fn game_corner_prize_mon_check_dex_records_exact_modpack_species() {
    let mut state = GameState::default();
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "PIKACHU".to_string());
    let moves = moves();
    let cries = BTreeMap::new();
    let species = species_catalog(&[("PIKACHU", 25)]);

    let outcome = apply_special_routine_with_context(
        &mut state,
        cry_context(&moves, &cries, &species),
        "GameCornerPrizeMonCheckDex",
    )
    .expect("game corner prize dex");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::GameCornerPrizeMonCheckDex {
            species: "PIKACHU".to_string(),
            species_int_id: 25,
            already_caught: false,
            recorded_caught: true
        }
    );
    assert!(state.pokedex.has_seen("PIKACHU"));
    assert!(state.pokedex.has_caught("PIKACHU"));
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("wCurPartySpecies")
            .map(String::as_str),
        Some("PIKACHU")
    );
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("wNamedObjectIndex")
            .map(String::as_str),
        Some("25")
    );

    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "PIKACHU".to_string());
    let already = apply_special_routine_with_context(
        &mut state,
        cry_context(&moves, &cries, &species),
        "GameCornerPrizeMonCheckDex",
    )
    .expect("already caught prize dex");

    assert_eq!(
        already.effect,
        SpecialRoutineEffect::GameCornerPrizeMonCheckDex {
            species: "PIKACHU".to_string(),
            species_int_id: 25,
            already_caught: true,
            recorded_caught: false
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));
}

#[test]
fn pokedex_species_specials_reject_unknown_or_case_changed_species_without_mutation() {
    let mut state = GameState::default();
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "pikachu".to_string());
    let moves = moves();
    let cries = BTreeMap::new();
    let species = species_catalog(&[("PIKACHU", 25)]);
    let before = state.clone();

    let error = apply_special_routine_with_context(
        &mut state,
        cry_context(&moves, &cries, &species),
        "GameCornerPrizeMonCheckDex",
    )
    .expect_err("case exact species");

    assert!(matches!(
        error,
        SpecialRoutineError::UnknownSpecies { routine, species }
            if routine == "GameCornerPrizeMonCheckDex" && species == "pikachu"
    ));
    assert_eq!(state, before);
}

#[test]
fn unused_set_seen_mon_records_seen_without_caught_flag() {
    let mut state = GameState::default();
    state.script_runtime.script_value = Some("EEVEE".to_string());
    let moves = moves();
    let cries = BTreeMap::new();
    let species = species_catalog(&[("EEVEE", 133)]);

    let outcome = apply_special_routine_with_context(
        &mut state,
        cry_context(&moves, &cries, &species),
        "UnusedSetSeenMon",
    )
    .expect("set seen mon");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::UnusedSetSeenMon {
            species: "EEVEE".to_string(),
            species_int_id: 133,
            newly_seen: true
        }
    );
    assert!(state.pokedex.has_seen("EEVEE"));
    assert!(!state.pokedex.has_caught("EEVEE"));
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));

    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "EEVEE".to_string());
    let already_seen = apply_special_routine_with_context(
        &mut state,
        cry_context(&moves, &cries, &species),
        "UnusedSetSeenMon",
    )
    .expect("already seen mon");

    assert_eq!(
        already_seen.effect,
        SpecialRoutineEffect::UnusedSetSeenMon {
            species: "EEVEE".to_string(),
            species_int_id: 133,
            newly_seen: false
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));
}

#[test]
fn activate_fishing_swarm_sets_exact_byte_from_script_value() {
    let mut state = GameState::default();
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "511".to_string());

    let outcome = apply_special_routine(&mut state, &moves(), "ActivateFishingSwarm")
        .expect("activate fishing swarm");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::ActivateFishingSwarm { value: 255 }
    );
    assert_eq!(state.fishing.swarm_flag, 255);
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("511"));
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("_value")
            .map(String::as_str),
        Some("511")
    );
    assert_eq!(
        state.script_runtime.last_special_routine.as_deref(),
        Some("ActivateFishingSwarm")
    );
}

#[test]
fn activate_fishing_swarm_requires_value_and_rejects_invalid_without_mutation() {
    let mut missing_state = GameState::default();
    missing_state.fishing.swarm_flag = 7;
    let before_missing = missing_state.clone();

    let missing_error = apply_special_routine(&mut missing_state, &moves(), "ActivateFishingSwarm")
        .expect_err("missing swarm value rejected");
    assert!(matches!(
        missing_error,
        SpecialRoutineError::MissingScriptValue { routine, variable }
            if routine == "ActivateFishingSwarm" && variable == "_value"
    ));
    assert_eq!(missing_state, before_missing);

    let mut invalid_state = GameState::default();
    invalid_state.fishing.swarm_flag = 7;
    invalid_state
        .script_runtime
        .variables
        .insert("_value".to_string(), "ROUTE_32".to_string());
    let before = invalid_state.clone();

    let error = apply_special_routine(&mut invalid_state, &moves(), "ActivateFishingSwarm")
        .expect_err("invalid numeric value rejected");

    assert!(matches!(
        error,
        SpecialRoutineError::InvalidNumericValue { routine, value }
            if routine == "ActivateFishingSwarm" && value == "ROUTE_32"
    ));
    assert_eq!(invalid_state, before);
}

#[test]
fn check_caught_celebi_reads_crystal_caught_battle_result_bit() {
    let mut uncaught_state = GameState::default();

    let uncaught = apply_special_routine(&mut uncaught_state, &moves(), "CheckCaughtCelebi")
        .expect("uncaught celebi");

    assert_eq!(
        uncaught.effect,
        SpecialRoutineEffect::CheckCaughtCelebi { caught: false }
    );
    assert_eq!(
        uncaught_state.script_runtime.script_value.as_deref(),
        Some("0")
    );

    let mut caught_state = GameState::default();
    caught_state.battle_result = 1 << 6;

    let caught = apply_special_routine(&mut caught_state, &moves(), "CheckCaughtCelebi")
        .expect("caught celebi");

    assert_eq!(
        caught.effect,
        SpecialRoutineEffect::CheckCaughtCelebi { caught: true }
    );
    assert_eq!(
        caught_state.script_runtime.script_value.as_deref(),
        Some("1")
    );
    assert_eq!(
        caught_state.script_runtime.last_special_routine.as_deref(),
        Some("CheckCaughtCelebi")
    );
}

#[test]
fn set_player_palette_requires_high_bit_and_updates_exact_palette_bits() {
    let mut state = GameState::default();
    state.player_palette_id = 3;
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "112".to_string());

    let unchanged = apply_special_routine(&mut state, &moves(), "SetPlayerPalette")
        .expect("low-bit palette ignored");

    assert_eq!(
        unchanged.effect,
        SpecialRoutineEffect::SetPlayerPalette {
            raw_value: 112,
            palette_id: 3,
            changed: false
        }
    );
    assert_eq!(state.player_palette_id, 3);
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("_value")
            .map(String::as_str),
        Some("112")
    );

    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "176".to_string());
    state.script_runtime.script_value = Some("176".to_string());
    let changed =
        apply_special_routine(&mut state, &moves(), "SetPlayerPalette").expect("palette changes");

    assert_eq!(
        changed.effect,
        SpecialRoutineEffect::SetPlayerPalette {
            raw_value: 176,
            palette_id: 3,
            changed: true
        }
    );
    assert_eq!(state.player_palette_id, 3);
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("3"));
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("_value")
            .map(String::as_str),
        Some("3")
    );
    assert_eq!(
        state.script_runtime.last_special_routine.as_deref(),
        Some("SetPlayerPalette")
    );
}

#[test]
fn set_player_palette_rejects_invalid_value_without_mutation() {
    let mut missing = GameState::default();
    missing.player_palette_id = 5;
    let before_missing = missing.clone();
    let missing_error = apply_special_routine(&mut missing, &moves(), "SetPlayerPalette")
        .expect_err("missing palette value rejected");
    assert!(matches!(
        missing_error,
        SpecialRoutineError::MissingScriptValue { routine, variable }
            if routine == "SetPlayerPalette" && variable == "_value"
    ));
    assert_eq!(missing, before_missing);

    let mut state = GameState::default();
    state.player_palette_id = 5;
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "PAL_OW_RED".to_string());
    let before = state.clone();

    let error = apply_special_routine(&mut state, &moves(), "SetPlayerPalette")
        .expect_err("invalid palette rejected");

    assert!(matches!(
        error,
        SpecialRoutineError::InvalidNumericValue { routine, value }
            if routine == "SetPlayerPalette" && value == "PAL_OW_RED"
    ));
    assert_eq!(state, before);
}

#[test]
fn snorlax_awake_requires_poke_flute_music_and_adjacent_tile() {
    let mut state = GameState::default();
    state.script_runtime.current_music = Some("MUSIC_POKE_FLUTE_CHANNEL".to_string());
    state.overworld = crate::state::OverworldMemory::Active {
        map_name: "Route11".to_string(),
        tile: crate::world::map::TilePosition::new(68, 20),
        facing: crate::world::map::Direction::Down,
        mode: crate::world::movement::MovementMode::Normal,
    };

    let awake = apply_special_routine(&mut state, &moves(), "SnorlaxAwake").expect("snorlax awake");

    assert_eq!(
        awake.effect,
        SpecialRoutineEffect::SnorlaxAwake {
            music: Some("MUSIC_POKE_FLUTE_CHANNEL".to_string()),
            tile: Some((68, 20)),
            awake: true
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));

    state.script_runtime.current_music = Some("MUSIC_ROUTE_11".to_string());
    let asleep =
        apply_special_routine(&mut state, &moves(), "SnorlaxAwake").expect("snorlax asleep");

    assert_eq!(
        asleep.effect,
        SpecialRoutineEffect::SnorlaxAwake {
            music: Some("MUSIC_ROUTE_11".to_string()),
            tile: Some((68, 20)),
            awake: false
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));
}

#[test]
fn snorlax_awake_rejects_packed_coordinate_candidates() {
    let mut state = GameState::default();
    state.script_runtime.current_music = Some("MUSIC_POKE_FLUTE_CHANNEL".to_string());
    state.overworld = crate::state::OverworldMemory::Active {
        map_name: "Route11".to_string(),
        tile: crate::world::map::TilePosition::new(67, 17),
        facing: crate::world::map::Direction::Down,
        mode: crate::world::movement::MovementMode::Normal,
    };

    let outcome =
        apply_special_routine(&mut state, &moves(), "SnorlaxAwake").expect("snorlax check");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::SnorlaxAwake {
            music: Some("MUSIC_POKE_FLUTE_CHANNEL".to_string()),
            tile: Some((67, 17)),
            awake: false
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));
    assert_eq!(
        state.script_runtime.last_special_routine.as_deref(),
        Some("SnorlaxAwake")
    );
}

#[test]
fn time_specials_update_exact_time_state_and_script_values() {
    let mut state = GameState::default();
    state.time.current_day = 5;
    state.time.day_of_week = 5;
    state
        .script_runtime
        .variables
        .insert("wTempDayOfWeek".to_string(), "5".to_string());

    let day = apply_special_routine(&mut state, &moves(), "SetDayOfWeek").expect("set day of week");

    assert_eq!(day.effect, SpecialRoutineEffect::SetDayOfWeek { day: 5 });
    assert_eq!(state.time.current_day, 5);
    assert_eq!(state.time.day_of_week, 5);
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));

    let set_dst =
        apply_special_routine(&mut state, &moves(), "InitialSetDSTFlag").expect("set dst flag");

    assert_eq!(set_dst.effect, SpecialRoutineEffect::InitialSetDstFlag);
    assert!(state.time.dst);

    let clear_dst =
        apply_special_routine(&mut state, &moves(), "InitialClearDSTFlag").expect("clear dst flag");

    assert_eq!(clear_dst.effect, SpecialRoutineEffect::InitialClearDstFlag);
    assert!(!state.time.dst);
    assert_eq!(
        state.script_runtime.last_special_routine.as_deref(),
        Some("InitialClearDSTFlag")
    );
}

#[test]
fn set_day_of_week_requires_a_valid_temporary_weekday_register() {
    let mut missing = GameState::default();
    missing.time.current_day = 3;
    missing.time.day_of_week = 3;
    assert_eq!(
        apply_special_routine(&mut missing, &moves(), "SetDayOfWeek"),
        Err(SpecialRoutineError::MissingScriptValue {
            routine: "SetDayOfWeek".to_string(),
            variable: "wTempDayOfWeek".to_string(),
        })
    );
    assert_eq!(missing.time.current_day, 3);
    assert_eq!(missing.time.day_of_week, 3);

    let mut invalid = GameState::default();
    invalid.time.current_day = 4;
    invalid.time.day_of_week = 4;
    invalid
        .script_runtime
        .variables
        .insert("wTempDayOfWeek".to_string(), "7".to_string());
    assert_eq!(
        apply_special_routine(&mut invalid, &moves(), "SetDayOfWeek"),
        Err(SpecialRoutineError::InvalidNumericValue {
            routine: "SetDayOfWeek".to_string(),
            value: "7".to_string(),
        })
    );
    assert_eq!(invalid.time.current_day, 4);
    assert_eq!(invalid.time.day_of_week, 4);
}

#[test]
fn update_time_special_recomputes_registers_and_time_of_day() {
    let mut state = GameState::default();
    state.time.start_time = crate::systems::time::ClockTime::new(2, 9, 30, 15);
    state.time.registers.rtc_day_lo = 3;
    state.time.registers.rtc_hours = 8;
    state.time.registers.rtc_minutes = 45;
    state.time.registers.rtc_seconds = 50;

    let outcome = apply_special_routine(&mut state, &moves(), "UpdateTime").expect("update time");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::UpdateTime {
            hour: 18,
            minute: 16,
            second: 5,
            day_of_week: 5,
            time_of_day: TimeOfDay::Night
        }
    );
    assert_eq!(state.time.registers.hours, 18);
    assert_eq!(state.time.registers.minutes, 16);
    assert_eq!(state.time.registers.seconds, 5);
    assert_eq!(state.time.game_time_hours, 0);
    assert_eq!(state.time.game_time_minutes, 0);
    assert_eq!(state.time.game_time_seconds, 0);
    assert_eq!(state.time.current_day, 5);
}

#[test]
fn unused_two_day_timer_updates_remaining_days_from_start_day() {
    let mut state = GameState::default();
    state.unused_two_day_timer.active = true;
    state.unused_two_day_timer.remaining_days = 2;
    state.unused_two_day_timer.start_day = 9;
    state.time.current_day = 10;

    let outcome = apply_special_routine(&mut state, &moves(), "UnusedCheckUnusedTwoDayTimer")
        .expect("unused two-day timer");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::UnusedCheckUnusedTwoDayTimer {
            start_day: 9,
            current_day: 10,
            elapsed_days: 1,
            remaining_days: 1,
        }
    );
    assert_eq!(state.unused_two_day_timer.remaining_days, 1);
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));

    state.time.current_day = 12;
    let expired = apply_special_routine(&mut state, &moves(), "UnusedCheckUnusedTwoDayTimer")
        .expect("expired unused two-day timer");
    assert_eq!(
        expired.effect,
        SpecialRoutineEffect::UnusedCheckUnusedTwoDayTimer {
            start_day: 9,
            current_day: 12,
            elapsed_days: 3,
            remaining_days: 0,
        }
    );
    assert_eq!(state.unused_two_day_timer.remaining_days, 0);
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));
}

#[test]
fn sample_kenji_break_countdown_uses_exact_divider_and_preserves_script_value() {
    let mut state = GameState::default();
    state.random_state = CrystalRandomState { add: 0xff, sub: 0 };
    state.script_runtime.script_value = Some("preserved".to_string());
    let mut divider = ReplayDivider::new([0, 200]);

    let outcome = apply_random_special_routine(
        &mut state,
        &moves(),
        "SampleKenjiBreakCountdown",
        &mut divider,
    )
    .expect("sample kenji countdown");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::SampleKenjiBreakCountdown {
            value: 3,
            random_state_after: CrystalRandomState { add: 0xff, sub: 56 },
        }
    );
    assert_eq!(state.kenji_break_timer, 3);
    assert_eq!(
        state.random_state,
        CrystalRandomState { add: 0xff, sub: 56 }
    );
    assert_eq!(
        state.script_runtime.script_value.as_deref(),
        Some("preserved")
    );
    assert!(!state.script_runtime.variables.contains_key("_value"));
    assert_eq!(divider.consumed(), 2);
}

#[test]
fn random_special_divider_exhaustion_is_atomic() {
    let mut state = GameState::default();
    state.kenji_break_timer = 6;
    state.random_state = CrystalRandomState { add: 9, sub: 10 };
    let before = state.clone();
    let mut divider = ReplayDivider::new([200]);

    assert_eq!(
        apply_random_special_routine(
            &mut state,
            &moves(),
            "SampleKenjiBreakCountdown",
            &mut divider,
        ),
        Err(RandomSpecialRoutineError::Divider(
            crate::random::ReplayDividerExhausted { consumed: 1 }
        ))
    );

    assert_eq!(state, before);
    assert_eq!(divider.consumed(), 1);
}

#[test]
fn lucky_number_show_flag_and_reset_use_daily_lucky_number() {
    let mut state = GameState::default();
    state.lucky_number_show_flag = true;
    state.time.current_day = 6;
    state.script_runtime.script_value = Some("preserved".to_string());

    let check = apply_special_routine(&mut state, &moves(), "CheckLuckyNumberShowFlag")
        .expect("check lucky flag");

    assert_eq!(
        check.effect,
        SpecialRoutineEffect::CheckLuckyNumberShowFlag { flag: true }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));

    state.script_runtime.script_value = Some("preserved".to_string());
    let mut divider = ReplayDivider::new([0, 255, 0, 255]);
    let reset = apply_random_special_routine(
        &mut state,
        &moves(),
        "ResetLuckyNumberShowFlag",
        &mut divider,
    )
    .expect("reset lucky flag");

    assert_eq!(
        reset.effect,
        SpecialRoutineEffect::ResetLuckyNumberShowFlag {
            lucky_number: 513,
            lucky_number_day: 6,
            random_state_after: CrystalRandomState { add: 2, sub: 2 },
        }
    );
    assert!(!state.lucky_number_show_flag);
    assert_eq!(state.lucky_number_day, Some(6));
    assert_eq!(state.lucky_id_number, 513);
    assert_eq!(state.random_state, CrystalRandomState { add: 2, sub: 2 });
    assert_eq!(
        state.script_runtime.script_value.as_deref(),
        Some("preserved")
    );
    assert_eq!(divider.consumed(), 4);

    let before_random_state = state.random_state;
    let mut same_day_divider = ReplayDivider::new([]);
    let reset_same_day = apply_random_special_routine(
        &mut state,
        &moves(),
        "ResetLuckyNumberShowFlag",
        &mut same_day_divider,
    )
    .expect("same-day reset");

    assert_eq!(
        reset_same_day.effect,
        SpecialRoutineEffect::ResetLuckyNumberShowFlag {
            lucky_number: 513,
            lucky_number_day: 6,
            random_state_after: before_random_state,
        }
    );
    assert_eq!(state.random_state, before_random_state);
    assert_eq!(same_day_divider.consumed(), 0);

    let printed = apply_special_routine(&mut state, &moves(), "PrintTodaysLuckyNumber")
        .expect("print lucky number");

    assert_eq!(
        printed.effect,
        SpecialRoutineEffect::PrintTodaysLuckyNumber {
            lucky_number: 513,
            formatted: "00513".to_string()
        }
    );
    assert_eq!(state.random_state, before_random_state);
    assert_eq!(
        state
            .script_runtime
            .named_buffers
            .get("STRING_BUFFER_3")
            .map(String::as_str),
        Some("00513")
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("00513"));
}

#[test]
fn check_for_lucky_number_winners_scans_party_and_pc_with_pc_tie_priority() {
    let mut state = GameState::default();
    state.lucky_number_day = None;
    state.time.current_day = 2;
    state.lucky_id_number = 45_123;
    let mut party_match = pokemon("CHIKORITA");
    party_match.original_trainer_id = 31_123;
    state
        .storage
        .register_capture_in_box(0, party_match)
        .expect("store party match");
    let mut pc_tie = pokemon("TOTODILE");
    pc_tie.original_trainer_id = 51_123;
    let mut box0 = PcBox::new(0);
    assert!(box0.add_pokemon(pc_tie));
    state.storage.pc_boxes[0] = box0;
    state.current_pc_box = 0;
    state.sync_party_from_storage();

    let outcome = apply_special_routine(&mut state, &moves(), "CheckForLuckyNumberWinners")
        .expect("check lucky winners");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::CheckForLuckyNumberWinners {
            lucky_number: 45_123,
            tier: 2,
            source: Some(LuckyNumberWinnerSource::Pc),
            species: Some("TOTODILE".to_string()),
            text_label: Some("LuckyNumberMatchPCText".to_string())
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("2"));
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("wCurPartySpecies")
            .map(String::as_str),
        Some("TOTODILE")
    );
}

#[test]
fn check_for_lucky_number_winners_rejects_invalid_current_pc_box_without_mutation() {
    let mut state = GameState::default();
    state.lucky_number_day = Some(1);
    state.time.current_day = 1;
    state.lucky_id_number = 12_345;
    state.current_pc_box = 3;
    state
        .storage
        .register_capture_in_box(0, pokemon("CHIKORITA"))
        .expect("store party");
    state.storage.pc_boxes.truncate(1);
    let before = state.clone();

    let error = apply_special_routine(&mut state, &moves(), "CheckForLuckyNumberWinners")
        .expect_err("invalid current box rejected");

    assert!(matches!(
        error,
        SpecialRoutineError::InvalidCurrentPcBox {
            routine,
            current_pc_box: 3,
            box_count: 1
        } if routine == "CheckForLuckyNumberWinners"
    ));
    assert_eq!(state, before);
}

#[test]
fn check_for_lucky_number_winners_rejects_invalid_pc_box_count_without_mutation() {
    let mut state = GameState::default();
    state.lucky_number_day = Some(1);
    state.time.current_day = 1;
    state.lucky_id_number = 12_345;
    state
        .storage
        .register_capture_in_box(0, pokemon("CHIKORITA"))
        .expect("store party");
    let mut pc_box = PcBox::new(0);
    pc_box.count = MAX_BOX_MONS + 1;
    state.storage.pc_boxes[0] = pc_box;
    let before = state.clone();

    let error = apply_special_routine(&mut state, &moves(), "CheckForLuckyNumberWinners")
        .expect_err("invalid box count rejected");

    assert!(matches!(
        error,
        SpecialRoutineError::InvalidPcBoxCount {
            routine,
            box_index: 0,
            count
        } if routine == "CheckForLuckyNumberWinners" && count == MAX_BOX_MONS + 1
    ));
    assert_eq!(state, before);
}

#[test]
fn money_display_specials_write_exact_buffers_values_and_runtime_events() {
    let mut state = GameState::default();
    state.money = 12_345;
    state.coins = 321;

    let money =
        apply_special_routine(&mut state, &moves(), "PlaceMoneyTopRight").expect("place money");

    assert_eq!(
        money.effect,
        SpecialRoutineEffect::PlaceMoneyTopRight {
            money: 12_345,
            formatted: "012345".to_string()
        }
    );
    assert_eq!(
        state
            .script_runtime
            .named_buffers
            .get("STRING_BUFFER_1")
            .map(String::as_str),
        Some("012345")
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("12345"));
    assert_eq!(state.script_runtime.money_events.len(), 1);
    assert_eq!(
        state.script_runtime.money_events[0].kind,
        ScriptMoneyRuntimeKind::PlaceMoneyTopRight
    );
    assert_eq!(state.script_runtime.money_events[0].money, 12_345);
    assert_eq!(state.script_runtime.money_events[0].coins, None);

    let balance = apply_special_routine(&mut state, &moves(), "DisplayMoneyAndCoinBalance")
        .expect("display money and coins");

    assert_eq!(
        balance.effect,
        SpecialRoutineEffect::DisplayMoneyAndCoinBalance {
            money: 12_345,
            coins: 321,
            formatted_money: "012345".to_string(),
            formatted_coins: "0321".to_string()
        }
    );
    assert_eq!(
        state
            .script_runtime
            .named_buffers
            .get("STRING_BUFFER_1")
            .map(String::as_str),
        Some("012345")
    );
    assert_eq!(
        state
            .script_runtime
            .named_buffers
            .get("STRING_BUFFER_2")
            .map(String::as_str),
        Some("0321")
    );
    assert_eq!(state.script_runtime.money_events.len(), 2);
    assert_eq!(
        state.script_runtime.money_events[1].kind,
        ScriptMoneyRuntimeKind::DisplayMoneyAndCoinBalance
    );
    assert_eq!(state.script_runtime.money_events[1].money, 12_345);
    assert_eq!(state.script_runtime.money_events[1].coins, Some(321));
    assert_eq!(
        state.script_runtime.last_special_routine.as_deref(),
        Some("DisplayMoneyAndCoinBalance")
    );

    let coin_case = apply_special_routine(&mut state, &moves(), "DisplayCoinCaseBalance")
        .expect("display coin case");

    assert_eq!(
        coin_case.effect,
        SpecialRoutineEffect::DisplayCoinCaseBalance {
            coins: 321,
            formatted_coins: "0321".to_string()
        }
    );
    assert_eq!(
        state
            .script_runtime
            .named_buffers
            .get("STRING_BUFFER_1")
            .map(String::as_str),
        Some("0321")
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("321"));
    assert_eq!(state.script_runtime.money_events.len(), 3);
    assert_eq!(
        state.script_runtime.money_events[2].kind,
        ScriptMoneyRuntimeKind::DisplayCoinCaseBalance
    );
    assert_eq!(state.script_runtime.money_events[2].money, 0);
    assert_eq!(state.script_runtime.money_events[2].coins, Some(321));
}

#[test]
fn gs_healings_reports_exact_saved_counter() {
    let mut state = GameState::default();
    state.gs_healings = 12;
    state.trainer_rankings_healings = 34;

    let outcome = apply_special_routine(&mut state, &moves(), "GSHealings").expect("gs healings");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::GsHealings { healings: 12 }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("12"));
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("_value")
            .map(String::as_str),
        Some("12")
    );
    assert_eq!(
        state.script_runtime.last_special_routine.as_deref(),
        Some("GSHealings")
    );

    let trainer_rankings =
        apply_special_routine(&mut state, &moves(), "StubbedTrainerRankings_Healings")
            .expect("trainer rankings healings");

    assert_eq!(
        trainer_rankings.effect,
        SpecialRoutineEffect::TrainerRankingsHealings { healings: 34 }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("34"));
    assert_eq!(
        state.script_runtime.last_special_routine.as_deref(),
        Some("StubbedTrainerRankings_Healings")
    );
}

#[test]
fn reset_records_exact_reset_request_and_clears_script_variables() {
    let mut state = GameState::default();
    state
        .script_runtime
        .variables
        .insert("old".to_string(), "value".to_string());
    state.script_runtime.script_value = Some("old".to_string());

    let outcome = apply_special_routine(&mut state, &moves(), "Reset").expect("reset");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::Reset {
            value: "$0".to_string()
        }
    );
    assert!(state.script_runtime.reset_requested);
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("$0"));
    assert_eq!(state.script_runtime.variables.len(), 1);
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("_value")
            .map(String::as_str),
        Some("$0")
    );
    assert_eq!(
        state.script_runtime.last_special_routine.as_deref(),
        Some("Reset")
    );
}

#[test]
fn ho_oh_chamber_requires_ho_oh_and_unleashed_beast_flags() {
    let mut state = GameState::default();
    state
        .storage
        .register_capture_in_box(0, pokemon("HO_OH"))
        .expect("store ho-oh");
    state.sync_party_from_storage();
    state
        .flags
        .set_event_flag("EVENT_UNLEASHED_SUICUNE", true)
        .expect("set suicune");
    state
        .flags
        .set_event_flag("EVENT_UNLEASHED_RAIKOU", true)
        .expect("set raikou");

    let closed =
        apply_special_routine(&mut state, &moves(), "HoOhChamber").expect("closed chamber");

    assert_eq!(
        closed.effect,
        SpecialRoutineEffect::HoOhChamber {
            has_ho_oh: true,
            suicune_unleashed: true,
            raikou_unleashed: true,
            entei_unleashed: false,
            open: false
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));

    state
        .flags
        .set_event_flag("EVENT_UNLEASHED_ENTEI", true)
        .expect("set entei");
    let open = apply_special_routine(&mut state, &moves(), "HoOhChamber").expect("open chamber");

    assert_eq!(
        open.effect,
        SpecialRoutineEffect::HoOhChamber {
            has_ho_oh: true,
            suicune_unleashed: true,
            raikou_unleashed: true,
            entei_unleashed: true,
            open: true
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));
}

#[test]
fn unown_chambers_match_asm_flag_and_map_requirements() {
    let mut state = GameState::default();
    let move_catalog = moves();
    let water_stone = item_data("WATER_STONE");
    let item_catalog = BTreeMap::from([(water_stone.script_name.clone(), water_stone.clone())]);
    let context = full_context(
        &move_catalog,
        &EMPTY_TEST_SPECIES,
        &EMPTY_TEST_LEARNSETS,
        &item_catalog,
    );

    let closed = apply_special_routine_with_context(&mut state, context, "OmanyteChamber")
        .expect("closed Omanyte chamber");
    assert_eq!(
        closed.effect,
        SpecialRoutineEffect::UnownChamber {
            chamber: "OMANYTE".to_string(),
            open: false,
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));
    assert!(
        !state
            .flags
            .is_event_flag_set("EVENT_WALL_OPENED_IN_OMANYTE_CHAMBER")
            .expect("read Omanyte flag")
    );

    state
        .bag
        .add_item(&water_stone, 1)
        .expect("add Water Stone");
    let open = apply_special_routine_with_context(&mut state, context, "OmanyteChamber")
        .expect("open Omanyte chamber");
    assert_eq!(
        open.effect,
        SpecialRoutineEffect::UnownChamber {
            chamber: "OMANYTE".to_string(),
            open: true,
        }
    );
    assert!(
        state
            .flags
            .is_event_flag_set("EVENT_WALL_OPENED_IN_OMANYTE_CHAMBER")
            .expect("read Omanyte flag")
    );

    state.overworld = OverworldMemory::Active {
        map_name: "RuinsOfAlphAerodactylChamber".to_string(),
        tile: TilePosition::new(1, 1),
        facing: Direction::Down,
        mode: MovementMode::Normal,
    };
    let aerodactyl = apply_special_routine_with_context(&mut state, context, "AerodactylChamber")
        .expect("open Aerodactyl chamber");
    assert_eq!(
        aerodactyl.effect,
        SpecialRoutineEffect::UnownChamber {
            chamber: "AERODACTYL".to_string(),
            open: true,
        }
    );

    state.overworld = OverworldMemory::Active {
        map_name: "RuinsOfAlphAerodactylChamber".to_string(),
        tile: TilePosition::new(1, 1),
        facing: Direction::Down,
        mode: MovementMode::Normal,
    };
    let kabuto = apply_special_routine_with_context(&mut state, context, "KabutoChamber")
        .expect("closed Kabuto chamber outside its map");
    assert_eq!(
        kabuto.effect,
        SpecialRoutineEffect::UnownChamber {
            chamber: "KABUTO".to_string(),
            open: false,
        }
    );
}

#[test]
fn graphics_commands_record_exact_kind_without_fade_payload() {
    let cases = [
        (
            "ClearBGPalettesBufferScreen",
            ScriptGraphicsRuntimeKind::ClearBgPalettesBufferScreen,
        ),
        (
            "ClearBGPalettes",
            ScriptGraphicsRuntimeKind::ClearBgPalettes,
        ),
        ("UpdateTimePals", ScriptGraphicsRuntimeKind::UpdateTimePals),
        ("ClearTilemap", ScriptGraphicsRuntimeKind::ClearTilemap),
        (
            "LoadMapPalettes",
            ScriptGraphicsRuntimeKind::LoadMapPalettes,
        ),
        ("RefreshSprites", ScriptGraphicsRuntimeKind::RefreshSprites),
        ("UpdateSprites", ScriptGraphicsRuntimeKind::UpdateSprites),
        (
            "ReloadSpritesNoPalettes",
            ScriptGraphicsRuntimeKind::ReloadSpritesNoPalettes,
        ),
    ];

    for (routine, kind) in cases {
        let mut state = GameState::default();

        let outcome =
            apply_special_routine(&mut state, &moves(), routine).expect("graphics command");

        assert_eq!(
            outcome.effect,
            SpecialRoutineEffect::GraphicsCommand { kind }
        );
        assert_eq!(
            state.script_runtime.last_special_routine.as_deref(),
            Some(routine)
        );
        assert_eq!(state.script_runtime.graphics_events.len(), 1);
        let event = &state.script_runtime.graphics_events[0];
        assert_eq!(event.kind, kind);
        assert_eq!(event.color, None);
        assert_eq!(event.direction, None);
        assert_eq!(event.frames, None);
        assert_eq!(event.source_script, routine);
    }
}

#[test]
fn screen_fades_record_exact_color_direction_and_frames() {
    let cases = [
        (
            "FadeOutToWhite",
            ScriptFadeColor::White,
            ScriptFadeDirection::Out,
        ),
        (
            "FadeInFromWhite",
            ScriptFadeColor::White,
            ScriptFadeDirection::In,
        ),
        (
            "FadeOutToBlack",
            ScriptFadeColor::Black,
            ScriptFadeDirection::Out,
        ),
        (
            "FadeInFromBlack",
            ScriptFadeColor::Black,
            ScriptFadeDirection::In,
        ),
    ];

    for (routine, color, direction) in cases {
        let mut state = GameState::default();

        let outcome = apply_special_routine(&mut state, &moves(), routine).expect("screen fade");

        assert_eq!(
            outcome.effect,
            SpecialRoutineEffect::ScreenFade {
                color,
                direction,
                frames: 8
            }
        );
        assert_eq!(
            state.script_runtime.last_special_routine.as_deref(),
            Some(routine)
        );
        assert_eq!(
            state.script_runtime.pending_screen_fade,
            Some(ScriptScreenFade {
                color,
                direction,
                frames: 8,
                source_script: routine.to_string(),
                command_index: 0
            })
        );
        assert_eq!(state.script_runtime.graphics_events.len(), 1);
        assert_eq!(
            state.script_runtime.graphics_events[0].kind,
            ScriptGraphicsRuntimeKind::ScreenFade
        );
        assert_eq!(state.script_runtime.graphics_events[0].color, Some(color));
        assert_eq!(
            state.script_runtime.graphics_events[0].direction,
            Some(direction)
        );
        assert_eq!(state.script_runtime.graphics_events[0].frames, Some(8));
    }
}

#[test]
fn pc_and_display_specials_record_exact_runtime_requests() {
    let mut state = GameState::default();
    state
        .storage
        .register_capture_in_box(0, pokemon("CHIKORITA"))
        .expect("store party mon");
    state.sync_party_from_storage();
    state.current_pc_box = 3;
    let chikorita = species_catalog(&[("CHIKORITA", 152)])
        .remove("CHIKORITA")
        .expect("species");
    let cyndaquil = species_catalog(&[("CYNDAQUIL", 155)])
        .remove("CYNDAQUIL")
        .expect("species");
    state.pokedex.record_seen(&chikorita);
    state.pokedex.record_caught(&cyndaquil);
    state.script_runtime.script_value = Some("73".to_string());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "73".to_string());
    for host_mirror in [
        "_pc_context",
        "_pc_party_count",
        "_pc_current_box",
        "_oak_rating_label",
        "_oak_seen_count",
        "_oak_owned_count",
        "_town_map_current_map",
    ] {
        state
            .script_runtime
            .variables
            .insert(host_mirror.to_string(), "stale".to_string());
    }

    let pc =
        apply_special_routine(&mut state, &moves(), "PokemonCenterPC").expect("pokemon center pc");

    assert_eq!(
        pc.effect,
        SpecialRoutineEffect::PokemonCenterPc {
            party_count: 1,
            current_pc_box: 3
        }
    );
    assert_eq!(
        state.script_runtime.active_menu.as_deref(),
        Some("PokemonCenterPC")
    );
    for host_mirror in ["_pc_context", "_pc_party_count", "_pc_current_box"] {
        assert!(!state.script_runtime.variables.contains_key(host_mirror));
    }
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("73"));

    let player_pc =
        apply_special_routine(&mut state, &moves(), "PlayersHousePC").expect("players pc");

    assert_eq!(
        player_pc.effect,
        SpecialRoutineEffect::PlayersHousePc { party_count: 1 }
    );
    assert_eq!(
        state.script_runtime.active_menu.as_deref(),
        Some("PlayersHousePC")
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));

    state.script_runtime.script_value = Some("73".to_string());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "73".to_string());

    let move_catalog = moves();
    let oak_ratings = vec![OakRatingEntry {
        caught_count_limit: 9,
        fanfare: "SFX_DEX_FANFARE_LESS_THAN_20".to_string(),
        text_label: "OakRating01".to_string(),
    }];
    let oak = apply_special_routine_with_context(
        &mut state,
        full_context_with_oak_ratings(&move_catalog, &oak_ratings),
        "ProfOaksPCBoot",
    )
    .expect("oak pc boot");

    assert_eq!(
        oak.effect,
        SpecialRoutineEffect::ProfOaksPcBoot {
            seen_count: 2,
            caught_count: 1,
            rating_label: "OakRating01".to_string()
        }
    );
    assert_eq!(
        state
            .script_runtime
            .named_buffers
            .get("STRING_BUFFER_3")
            .map(String::as_str),
        Some("2")
    );
    for host_mirror in ["_oak_rating_label", "_oak_seen_count", "_oak_owned_count"] {
        assert!(!state.script_runtime.variables.contains_key(host_mirror));
    }
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("73"));

    let mut missing_oak_ratings = state.clone();
    let missing_error =
        apply_special_routine(&mut missing_oak_ratings, &move_catalog, "ProfOaksPCBoot")
            .expect_err("Oak ratings are pack data");
    assert!(matches!(
        missing_error,
        SpecialRoutineError::MissingOakRatingTable { routine }
            if routine == "ProfOaksPCBoot"
    ));

    state.overworld = crate::state::OverworldMemory::Active {
        map_name: "NewBarkTown".to_string(),
        tile: crate::world::map::TilePosition::new(4, 5),
        facing: crate::world::map::Direction::Down,
        mode: crate::world::movement::MovementMode::Normal,
    };
    let town_map =
        apply_special_routine(&mut state, &moves(), "OverworldTownMap").expect("town map");

    assert_eq!(
        town_map.effect,
        SpecialRoutineEffect::OverworldTownMap {
            map_name: Some("NewBarkTown".to_string())
        }
    );
    assert!(
        !state
            .script_runtime
            .variables
            .contains_key("_town_map_current_map")
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("73"));

    state.pokedex.unown_letters = vec![1, 2, 26];
    let printer =
        apply_special_routine(&mut state, &moves(), "UnownPrinter").expect("unown printer");

    assert_eq!(
        printer.effect,
        SpecialRoutineEffect::UnownPrinter {
            letters: vec![1, 2, 26]
        }
    );
    assert_eq!(
        state.script_runtime.active_menu.as_deref(),
        Some("UnownPrinter")
    );
    assert!(
        !state
            .script_runtime
            .variables
            .contains_key("_unown_printer_unlocked")
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("73"));

    state.pokedex.unown_letters.clear();
    let unavailable =
        apply_special_routine(&mut state, &moves(), "UnownPrinter").expect("empty Unown Printer");
    assert_eq!(
        unavailable.effect,
        SpecialRoutineEffect::UnownPrinter { letters: vec![] }
    );
    assert!(state.script_runtime.active_menu.is_none());
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("73"));
}

#[test]
fn map_radio_and_name_rival_require_exact_script_values() {
    let mut state = GameState::default();

    let missing_radio = apply_special_routine(&mut state, &moves(), "MapRadio")
        .expect_err("map radio requires exact selector");
    assert!(matches!(
        missing_radio,
        SpecialRoutineError::MissingScriptValue { routine, variable }
            if routine == "MapRadio" && variable == "_value"
    ));

    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "MAPRADIO_UNOWN".to_string());
    state
        .script_runtime
        .variables
        .insert("_map_radio_station".to_string(), "stale".to_string());
    let radio = apply_special_routine(&mut state, &moves(), "MapRadio").expect("map radio");

    assert_eq!(
        radio.effect,
        SpecialRoutineEffect::MapRadio {
            station: "MAPRADIO_UNOWN".to_string()
        }
    );
    assert!(!state.script_runtime.variables.contains_key("_value"));
    assert!(
        !state
            .script_runtime
            .variables
            .contains_key("_map_radio_station")
    );

    let missing_rival = apply_special_routine(&mut state, &moves(), "NameRival")
        .expect_err("rival name requires script value");
    assert!(matches!(
        missing_rival,
        SpecialRoutineError::MissingScriptValue { routine, variable }
            if routine == "NameRival" && variable == "_rival_name"
    ));

    state
        .script_runtime
        .variables
        .insert("_rival_name".to_string(), "     ".to_string());
    let blank_rival = apply_special_routine(&mut state, &moves(), "NameRival")
        .expect_err("blank rival name is invalid definitive content");
    assert!(matches!(
        blank_rival,
        SpecialRoutineError::MissingScriptValue { routine, variable }
            if routine == "NameRival" && variable == "_rival_name"
    ));

    state
        .script_runtime
        .variables
        .insert("_rival_name".to_string(), "SILVER".to_string());
    let rival = apply_special_routine(&mut state, &moves(), "NameRival").expect("name rival");

    assert_eq!(
        rival.effect,
        SpecialRoutineEffect::NameRival {
            rival_name: "SILVER".to_string()
        }
    );
    assert_eq!(
        state
            .script_runtime
            .named_buffers
            .get("STRING_BUFFER_1")
            .map(String::as_str),
        Some("SILVER")
    );
}

#[test]
fn move_deletion_requires_exact_party_and_move_slots() {
    let mut state = GameState::default();
    state
        .storage
        .register_capture_in_box(0, pokemon("CHIKORITA"))
        .expect("store party mon");
    state.sync_party_from_storage();

    let missing = apply_special_routine(&mut state, &moves(), "MoveDeletion")
        .expect_err("selection required");
    assert!(matches!(
        missing,
        SpecialRoutineError::MissingScriptValue { routine, variable }
            if routine == "MoveDeletion" && variable == "_party_slot"
    ));

    state
        .script_runtime
        .variables
        .insert("_party_slot".to_string(), "0".to_string());
    state
        .script_runtime
        .variables
        .insert("_move_slot".to_string(), "1".to_string());
    state.script_runtime.script_value = Some("73".to_string());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "73".to_string());
    let deletion =
        apply_special_routine(&mut state, &moves(), "MoveDeletion").expect("delete move");

    assert_eq!(
        deletion.effect,
        SpecialRoutineEffect::MoveDeletion {
            party_slot: 0,
            species: "CHIKORITA".to_string(),
            deleted_move: "GROWL".to_string(),
            remaining_moves: 1
        }
    );
    assert_eq!(
        state.storage.party.pokemon[0]
            .as_ref()
            .expect("party slot")
            .moves
            .iter()
            .map(|move_slot| move_slot.name.as_str())
            .collect::<Vec<_>>(),
        vec!["TACKLE"]
    );
    assert!(!state.script_runtime.variables.contains_key("_deleted_move"));
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("73"));
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("_value")
            .map(String::as_str),
        Some("73")
    );

    let one_move_left = apply_special_routine(&mut state, &moves(), "MoveDeletion")
        .expect_err("only move cannot be deleted");
    assert!(matches!(
        one_move_left,
        SpecialRoutineError::CannotDeleteOnlyMove {
            routine,
            party_slot: 0
        } if routine == "MoveDeletion"
    ));
}

#[test]
fn visual_specials_record_exact_runtime_graphics_events() {
    let cases = [
        (
            "BattleTowerFade",
            ScriptGraphicsRuntimeKind::BattleTowerFade,
        ),
        (
            "UpdatePlayerSprite",
            ScriptGraphicsRuntimeKind::UpdatePlayerSprite,
        ),
        (
            "HealMachineAnim",
            ScriptGraphicsRuntimeKind::HealMachineAnim,
        ),
        ("SurfStartStep", ScriptGraphicsRuntimeKind::SurfStartStep),
        (
            "LoadUsedSpritesGFX",
            ScriptGraphicsRuntimeKind::LoadUsedSpritesGfx,
        ),
        (
            "ToggleMaptileDecorations",
            ScriptGraphicsRuntimeKind::ToggleMaptileDecorations,
        ),
        (
            "ToggleDecorationsVisibility",
            ScriptGraphicsRuntimeKind::ToggleDecorationsVisibility,
        ),
        ("MagnetTrain", ScriptGraphicsRuntimeKind::MagnetTrain),
        ("Diploma", ScriptGraphicsRuntimeKind::Diploma),
        ("PrintDiploma", ScriptGraphicsRuntimeKind::PrintDiploma),
        (
            "DisplayUnownWords",
            ScriptGraphicsRuntimeKind::DisplayUnownWords,
        ),
    ];

    for (routine, kind) in cases {
        let mut state = GameState::default();
        if routine == "ToggleMaptileDecorations" {
            state.overworld = OverworldMemory::Active {
                map_name: "PlayersHouse2F".to_string(),
                tile: TilePosition::new(3, 3),
                facing: Direction::Down,
                mode: MovementMode::Normal,
            };
        }
        state.script_runtime.script_value = Some("73".to_string());
        state
            .script_runtime
            .variables
            .insert("_value".to_string(), "73".to_string());

        let outcome =
            apply_special_routine(&mut state, &moves(), routine).expect("visual special routine");

        assert_eq!(
            outcome.effect,
            SpecialRoutineEffect::RuntimeVisualCommand { kind }
        );
        assert_eq!(state.script_runtime.active_menu.as_deref(), Some(routine));
        assert!(
            !state
                .script_runtime
                .variables
                .contains_key("_visual_special")
        );
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("73"));
        assert_eq!(
            state
                .script_runtime
                .variables
                .get("_value")
                .map(String::as_str),
            Some("73")
        );
        assert_eq!(
            state.script_runtime.last_special_routine.as_deref(),
            Some(routine)
        );
        assert_eq!(state.script_runtime.graphics_events.len(), 1);
        let event = &state.script_runtime.graphics_events[0];
        assert_eq!(event.kind, kind);
        assert_eq!(event.source_script, routine);
        assert_eq!(event.command, "special");
        assert_eq!(event.color, None);
        assert_eq!(event.direction, None);
        assert_eq!(event.frames, None);
    }
}

#[test]
fn unown_puzzle_runs_exact_state_machine_instead_of_visual_command() {
    let mut state = GameState::default();
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "2".to_string());
    let mut divider = ReplayDivider::new(divider_trace_for_sub_values(0_u8..16));

    let opened = apply_random_special_routine(&mut state, &moves(), "UnownPuzzle", &mut divider)
        .expect("open puzzle");

    let SpecialRoutineEffect::UnownPuzzle {
        puzzle_id,
        solved,
        layout,
        holding_piece,
        random_state_after,
    } = opened.effect
    else {
        panic!("expected UnownPuzzle effect");
    };
    assert_eq!(puzzle_id, "AERODACTYL");
    assert!(!solved);
    assert_eq!(holding_piece, None);
    assert_eq!(random_state_after, CrystalRandomState { add: 0, sub: 15 });
    assert_eq!(state.random_state, random_state_after);
    assert_eq!(divider.consumed(), 32);
    assert_eq!(layout.len(), 6);
    assert_eq!(
        layout
            .iter()
            .flatten()
            .filter(|piece| **piece != 0)
            .copied()
            .collect::<BTreeSet<_>>(),
        (1_u8..=16).collect::<BTreeSet<_>>()
    );
    for y in 1..5 {
        for x in 1..5 {
            assert_eq!(layout[y][x], 0);
        }
    }
    for (index, (x, y)) in UNOWN_START_POSITIONS.iter().copied().enumerate() {
        assert_eq!(layout[y][x], index as u8 + 1);
    }
    assert_eq!(
        state.script_runtime.active_menu.as_deref(),
        Some("UnownPuzzle")
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));
    let mut layout_array = [[0_u8; 6]; 6];
    for (y, row) in layout.iter().enumerate() {
        for (x, value) in row.iter().enumerate() {
            layout_array[y][x] = *value;
        }
    }
    let expected_layout = encode_unown_layout(&layout_array);
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("unown_layout_AERODACTYL")
            .map(String::as_str),
        Some(expected_layout.as_str())
    );
    assert!(state.script_runtime.graphics_events.is_empty());
}

#[test]
fn unown_puzzle_divider_exhaustion_is_atomic() {
    let mut state = GameState::default();
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "UNOWNPUZZLE_KABUTO".to_string());
    let before = state.clone();
    let mut samples = divider_trace_for_sub_values(0_u8..16);
    samples.pop();
    let mut divider = ReplayDivider::new(samples);

    assert_eq!(
        apply_random_special_routine(&mut state, &moves(), "UnownPuzzle", &mut divider,),
        Err(RandomSpecialRoutineError::Divider(
            crate::random::ReplayDividerExhausted { consumed: 31 }
        ))
    );
    assert_eq!(state, before);
    assert_eq!(divider.consumed(), 31);
}

#[test]
fn unown_puzzle_headless_actions_persist_and_detect_solved_layouts() {
    let mut state = GameState::default();
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "UNOWNPUZZLE_KABUTO".to_string());
    state.script_runtime.variables.insert(
        "unown_layout_KABUTO".to_string(),
        encode_unown_layout(&UNOWN_TARGET_LAYOUT),
    );
    state
        .script_runtime
        .variables
        .insert("unown_action".to_string(), "pickup".to_string());
    state
        .script_runtime
        .variables
        .insert("unown_x".to_string(), "1".to_string());
    state
        .script_runtime
        .variables
        .insert("unown_y".to_string(), "1".to_string());

    let mut pickup_divider = ReplayDivider::new([]);
    let pickup =
        apply_random_special_routine(&mut state, &moves(), "UnownPuzzle", &mut pickup_divider)
            .expect("pickup piece");
    assert_eq!(pickup_divider.consumed(), 0);

    assert!(matches!(
        pickup.effect,
        SpecialRoutineEffect::UnownPuzzle {
            puzzle_id,
            solved: false,
            holding_piece: Some(1),
            ..
        } if puzzle_id == "KABUTO"
    ));
    assert!(!state.script_runtime.variables.contains_key("unown_action"));
    assert!(!state.script_runtime.variables.contains_key("unown_x"));
    assert!(!state.script_runtime.variables.contains_key("unown_y"));
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("unown_holding_piece_KABUTO")
            .map(String::as_str),
        Some("1")
    );

    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "UNOWNPUZZLE_KABUTO".to_string());
    state
        .script_runtime
        .variables
        .insert("unown_action".to_string(), "place".to_string());
    state
        .script_runtime
        .variables
        .insert("unown_x".to_string(), "1".to_string());
    state
        .script_runtime
        .variables
        .insert("unown_y".to_string(), "1".to_string());

    let mut place_divider = ReplayDivider::new([]);
    let place =
        apply_random_special_routine(&mut state, &moves(), "UnownPuzzle", &mut place_divider)
            .expect("place piece");
    assert_eq!(place_divider.consumed(), 0);

    assert!(matches!(
        place.effect,
        SpecialRoutineEffect::UnownPuzzle {
            puzzle_id,
            solved: true,
            holding_piece: None,
            ..
        } if puzzle_id == "KABUTO"
    ));
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("wSolvedUnownPuzzle")
            .map(String::as_str),
        Some("1")
    );
    assert!(
        !state
            .script_runtime
            .variables
            .contains_key("unown_moves_KABUTO")
    );
    assert!(
        !state
            .script_runtime
            .variables
            .contains_key("unown_holding_piece_KABUTO")
    );
}

#[test]
fn unown_puzzle_does_not_migrate_unscoped_host_state() {
    let mut state = GameState::default();
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "UNOWNPUZZLE_KABUTO".to_string());
    state.script_runtime.variables.insert(
        "unown_layout".to_string(),
        encode_unown_layout(&UNOWN_TARGET_LAYOUT),
    );
    state
        .script_runtime
        .variables
        .insert("unown_action".to_string(), "noop".to_string());
    let mut divider = ReplayDivider::new([]);

    let error = apply_random_special_routine(&mut state, &moves(), "UnownPuzzle", &mut divider)
        .expect_err("unscoped host puzzle state is not a cartridge state boundary");

    assert!(matches!(
        error,
        RandomSpecialRoutineError::Routine(
            SpecialRoutineError::InvalidUnownPuzzleState { routine, message }
        ) if routine == "UnownPuzzle" && message.contains("KABUTO has no active layout")
    ));
    assert!(state.script_runtime.variables.contains_key("unown_layout"));
    assert!(
        !state
            .script_runtime
            .variables
            .contains_key("unown_layout_KABUTO")
    );
    assert_eq!(divider.consumed(), 0);
}

#[test]
fn unown_puzzle_rejects_bare_host_puzzle_aliases() {
    let mut state = GameState::default();
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "KABUTO".to_string());
    let before = state.clone();
    let mut divider = ReplayDivider::new([]);

    let error = apply_random_special_routine(&mut state, &moves(), "UnownPuzzle", &mut divider)
        .expect_err("bare host puzzle alias is not a source constant");

    assert!(matches!(
        error,
        RandomSpecialRoutineError::Routine(
            SpecialRoutineError::InvalidUnownPuzzleState { routine, message }
        ) if routine == "UnownPuzzle" && message.contains("unknown puzzle id 'KABUTO'")
    ));
    assert_eq!(state, before);
    assert_eq!(divider.consumed(), 0);
}

#[test]
fn unown_puzzle_rejects_impossible_restored_piece_inventories() {
    let mut state = GameState::default();
    let mut duplicate = UNOWN_TARGET_LAYOUT;
    duplicate[1][2] = 1;
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "UNOWNPUZZLE_HO_OH".to_string());
    state.script_runtime.variables.insert(
        "unown_layout_HOOH".to_string(),
        encode_unown_layout(&duplicate),
    );
    state
        .script_runtime
        .variables
        .insert("unown_action".to_string(), "noop".to_string());

    let mut divider = ReplayDivider::new([]);
    let error = apply_random_special_routine(&mut state, &moves(), "UnownPuzzle", &mut divider)
        .expect_err("duplicate piece must fail");

    assert!(matches!(
        error,
        RandomSpecialRoutineError::Routine(
            SpecialRoutineError::InvalidUnownPuzzleState { routine, message }
        )
            if routine == "UnownPuzzle"
                && message == "piece 1 appears more than once in the puzzle state"
    ));
}

#[test]
fn toggle_decorations_visibility_uses_equipped_room_decoration_sprites() {
    let mut state = GameState::default();
    state
        .script_runtime
        .memory
        .insert("wDecoConsole".to_string(), "DECO_N64".to_string());
    state.script_runtime.memory.insert(
        "wDecoLeftOrnament".to_string(),
        "DECO_PIKACHU_DOLL".to_string(),
    );
    state.script_runtime.memory.insert(
        "wDecoRightOrnament".to_string(),
        "DECO_SURF_PIKACHU_DOLL".to_string(),
    );
    state.script_runtime.memory.insert(
        "wDecoBigDoll".to_string(),
        "DECO_BIG_LAPRAS_DOLL".to_string(),
    );

    let outcome = apply_special_routine(&mut state, &moves(), "ToggleDecorationsVisibility")
        .expect("toggle decorations visibility");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::RuntimeVisualCommand {
            kind: ScriptGraphicsRuntimeKind::ToggleDecorationsVisibility
        }
    );
    assert_eq!(
        state.script_runtime.variable_sprites,
        BTreeMap::from([
            (
                "SPRITE_BIG_DOLL".to_string(),
                "SPRITE_BIG_LAPRAS".to_string()
            ),
            ("SPRITE_CONSOLE".to_string(), "SPRITE_N64".to_string()),
            ("SPRITE_DOLL_1".to_string(), "SPRITE_PIKACHU".to_string()),
            (
                "SPRITE_DOLL_2".to_string(),
                "SPRITE_SURFING_PIKACHU".to_string(),
            ),
        ])
    );
    for event_flag in [
        "EVENT_PLAYERS_HOUSE_2F_CONSOLE",
        "EVENT_PLAYERS_HOUSE_2F_DOLL_1",
        "EVENT_PLAYERS_HOUSE_2F_DOLL_2",
        "EVENT_PLAYERS_HOUSE_2F_BIG_DOLL",
    ] {
        assert_eq!(state.flags.is_event_flag_set(event_flag), Ok(false));
    }

    for memory in [
        "wDecoConsole",
        "wDecoLeftOrnament",
        "wDecoRightOrnament",
        "wDecoBigDoll",
    ] {
        state
            .script_runtime
            .memory
            .insert(memory.to_string(), "0".to_string());
    }
    apply_special_routine(&mut state, &moves(), "ToggleDecorationsVisibility")
        .expect("hide removed decorations");
    assert!(state.script_runtime.variable_sprites.is_empty());
    for event_flag in [
        "EVENT_PLAYERS_HOUSE_2F_CONSOLE",
        "EVENT_PLAYERS_HOUSE_2F_DOLL_1",
        "EVENT_PLAYERS_HOUSE_2F_DOLL_2",
        "EVENT_PLAYERS_HOUSE_2F_BIG_DOLL",
    ] {
        assert_eq!(state.flags.is_event_flag_set(event_flag), Ok(true));
    }
}

#[test]
fn toggle_maptile_decorations_uses_all_equipped_tile_decorations() {
    let mut state = GameState::default();
    state.overworld = OverworldMemory::Active {
        map_name: "PlayersHouse2F".to_string(),
        tile: TilePosition::new(3, 3),
        facing: Direction::Down,
        mode: MovementMode::Normal,
    };
    for (key, value) in [
        ("wDecoBed", "DECO_PIKACHU_BED"),
        ("wDecoPlant", "DECO_TROPICPLANT"),
        ("wDecoPoster", "DECO_CLEFAIRY_POSTER"),
        ("wDecoCarpet", "DECO_BLUE_CARPET"),
    ] {
        state
            .script_runtime
            .memory
            .insert(key.to_string(), value.to_string());
    }

    let outcome = apply_special_routine(&mut state, &moves(), "ToggleMaptileDecorations")
        .expect("toggle map-tile decorations");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::RuntimeVisualCommand {
            kind: ScriptGraphicsRuntimeKind::ToggleMaptileDecorations
        }
    );
    assert_eq!(
        state.map_block_overrides.get("PlayersHouse2F"),
        Some(&BTreeMap::from([
            ((0, 0), 0x0b),
            ((0, 1), 0x0c),
            ((0, 2), 0x1e),
            ((1, 1), 0x0d),
            ((2, 1), 0x0c),
            ((3, 0), 0x24),
            ((3, 2), 0x21),
        ]))
    );
    assert_eq!(
        state.flags.is_event_flag_set("EVENT_PLAYERS_ROOM_POSTER"),
        Ok(false)
    );

    for memory in ["wDecoBed", "wDecoPlant", "wDecoPoster", "wDecoCarpet"] {
        state
            .script_runtime
            .memory
            .insert(memory.to_string(), "0".to_string());
    }
    apply_special_routine(&mut state, &moves(), "ToggleMaptileDecorations")
        .expect("remove tile decorations");
    assert!(state.map_block_overrides["PlayersHouse2F"].is_empty());
    assert_eq!(
        state.flags.is_event_flag_set("EVENT_PLAYERS_ROOM_POSTER"),
        Ok(true)
    );
}

#[test]
fn decoration_callbacks_reject_invalid_equipped_state_atomically() {
    let mut state = GameState::default();
    state.overworld = OverworldMemory::Active {
        map_name: "PlayersHouse2F".to_string(),
        tile: TilePosition::new(3, 3),
        facing: Direction::Down,
        mode: MovementMode::Normal,
    };
    state
        .script_runtime
        .memory
        .insert("wDecoPoster".to_string(), "DECO_PIKACHU_DOLL".to_string());
    let before = state.clone();

    assert!(matches!(
        apply_special_routine(&mut state, &moves(), "ToggleMaptileDecorations"),
        Err(SpecialRoutineError::InvalidState { ref routine, ref message })
            if routine == "ToggleMaptileDecorations"
                && message == "wDecoPoster has invalid equipped decoration DECO_PIKACHU_DOLL"
    ));
    assert_eq!(state, before);
}

#[test]
fn check_pokerus_is_a_query_and_leaves_story_mutations_to_the_standard_script() {
    let mut state = GameState::default();
    let mut infected = pokemon("CHIKORITA");
    infected.status = Some("POKERUS".to_string());
    state
        .storage
        .register_capture_in_box(0, infected)
        .expect("store infected mon");
    state.sync_party_from_storage();

    let outcome =
        apply_special_routine(&mut state, &moves(), "CheckPokerus").expect("check pokerus");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::CheckPokerus {
            found: true,
            newly_discovered: true
        }
    );
    assert_eq!(
        state.flags.is_engine_flag_set("ENGINE_CAUGHT_POKERUS"),
        Ok(false)
    );
    assert!(state.script_runtime.special_phone_call.is_none());
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));

    state
        .flags
        .set_engine_flag("ENGINE_CAUGHT_POKERUS", true)
        .expect("standard script sets caught Pokerus flag");
    state.script_runtime.special_phone_call = Some("SPECIALCALL_POKERUS".to_string());

    let second = apply_special_routine(&mut state, &moves(), "CheckPokerus").expect("check again");

    assert_eq!(
        second.effect,
        SpecialRoutineEffect::CheckPokerus {
            found: true,
            newly_discovered: false
        }
    );
    assert_eq!(
        state.script_runtime.special_phone_call.as_deref(),
        Some("SPECIALCALL_POKERUS")
    );
}

#[test]
fn happiness_data_issues_validate_exact_change_and_service_tables() {
    let data = HappinessData {
        changes: BTreeMap::from([
            (
                1,
                HappinessChangeEntry {
                    code: "HAPPINESS CHANGE".to_string(),
                    low: 1,
                    mid: 1,
                    high: 1,
                },
            ),
            (
                2,
                HappinessChangeEntry {
                    code: String::new(),
                    low: 1,
                    mid: 1,
                    high: 1,
                },
            ),
        ]),
        services: BTreeMap::from([
            (String::new(), Vec::new()),
            (
                "Haircut Brothers".to_string(),
                vec![HappinessServiceOutcome {
                    roll_weight: 1,
                    script_value: 0,
                    change_code: 9,
                }],
            ),
        ]),
    };

    assert_eq!(
        happiness_data_issues(&HappinessData {
            changes: BTreeMap::new(),
            services: BTreeMap::new(),
        }),
        vec![
            HappinessDataIssue::EmptyChanges,
            HappinessDataIssue::EmptyServices,
        ],
    );
    assert_eq!(
        happiness_data_issues(&data),
        vec![
            HappinessDataIssue::InvalidChangeCode {
                code: "HAPPINESS CHANGE".to_string(),
                change_code: 1,
            },
            HappinessDataIssue::EmptyChangeCode { change_code: 2 },
            HappinessDataIssue::EmptyServiceRoutine {
                routine: String::new(),
            },
            HappinessDataIssue::EmptyServiceOutcomes {
                routine: String::new(),
            },
            HappinessDataIssue::InvalidServiceRoutine {
                routine: "Haircut Brothers".to_string(),
            },
            HappinessDataIssue::UnknownServiceChange {
                routine: "Haircut Brothers".to_string(),
                change_code: 9,
            },
        ],
    );
}

#[test]
fn oak_rating_table_issues_validate_exact_order_and_coverage() {
    let entries = vec![
        OakRatingEntry {
            caught_count_limit: 3,
            fanfare: String::new(),
            text_label: "OakRating01".to_string(),
        },
        OakRatingEntry {
            caught_count_limit: 2,
            fanfare: "SFX_DEX_FANFARE_20_49".to_string(),
            text_label: "Oak Rating02".to_string(),
        },
    ];

    assert_eq!(
        oak_rating_table_issues(&entries, 5),
        vec![
            OakRatingTableIssue::InvalidFanfare {
                index: 0,
                fanfare: String::new(),
            },
            OakRatingTableIssue::InvalidTextLabel {
                index: 1,
                text_label: "Oak Rating02".to_string(),
            },
            OakRatingTableIssue::InvalidOrder {
                index: 1,
                caught_count_limit: 2,
                previous_limit: 3,
            },
            OakRatingTableIssue::IncompleteCoverage {
                pokemon_count: 5,
                last_caught_count_limit: 2,
            },
        ],
    );
    assert_eq!(oak_rating_table_issues(&[], 5), []);
}

#[test]
fn magikarp_length_table_issues_validate_divisors_and_order() {
    let entries = vec![
        MagikarpLengthEntry {
            threshold: 100,
            divisor: 0,
        },
        MagikarpLengthEntry {
            threshold: 100,
            divisor: 10,
        },
    ];

    assert_eq!(
        magikarp_length_table_issues(&entries),
        vec![
            MagikarpLengthTableIssue::InvalidEntryCount { actual: 2 },
            MagikarpLengthTableIssue::InvalidDivisor {
                index: 0,
                threshold: 100,
            },
            MagikarpLengthTableIssue::InvalidThresholdOrder {
                index: 1,
                threshold: 100,
                previous_threshold: 100,
            },
        ],
    );
}

#[test]
fn battle_tower_rules_issues_validate_exact_rules_and_banned_species() {
    let rules = BattleTowerRules {
        banned_species: BTreeMap::from([
            (
                "MEWTWO".to_string(),
                BattleTowerBannedSpeciesRule::default(),
            ),
            ("ME W".to_string(), BattleTowerBannedSpeciesRule::default()),
        ]),
        required_party_count: 0,
        challenge_streak_length: 0,
        reward_candidates: vec!["HP_UP".to_string(), "LUCKY_PUNCH".to_string()],
        excluded_reward_items: vec!["LUCKY_PUNCH".to_string()],
        reward_quantity: 5,
        reward_failure_sentinel: "POTION".to_string(),
        reward_item_values: BTreeMap::from([
            ("POTION".to_string(), 0x12),
            ("HP_UP".to_string(), 0x1a),
            ("LUCKY_PUNCH".to_string(), 0x1e),
        ]),
        minimum_level_group: 0,
        maximum_level_group: 10,
        level_group_size: 0,
        party_count_failure_text: String::new(),
        duplicate_species_failure_text: "DuplicateSpeciesText".to_string(),
        duplicate_held_item_failure_text: "Duplicate HeldItemText".to_string(),
        egg_failure_text: "EggText".to_string(),
        trainers: Vec::new(),
        mon_groups: Vec::new(),
    };
    let species_ids = BTreeSet::from(["MEW".to_string()]);

    assert_eq!(
        battle_tower_rules_issues(&rules, &species_ids),
        vec![
            BattleTowerRulesIssue::MissingRequiredPartyCount,
            BattleTowerRulesIssue::MissingChallengeStreakLength,
            BattleTowerRulesIssue::MissingLevelGroupSize,
            BattleTowerRulesIssue::InvalidLevelGroupRange,
            BattleTowerRulesIssue::MissingTrainerRoster,
            BattleTowerRulesIssue::MissingMonGroups,
            BattleTowerRulesIssue::InvalidFailureText {
                field: BattleTowerFailureTextField::PartyCount,
                text_id: String::new(),
            },
            BattleTowerRulesIssue::InvalidFailureText {
                field: BattleTowerFailureTextField::DuplicateHeldItem,
                text_id: "Duplicate HeldItemText".to_string(),
            },
            BattleTowerRulesIssue::InvalidBannedSpecies {
                species_id: "ME W".to_string(),
            },
            BattleTowerRulesIssue::UnknownBannedSpecies {
                species_id: "MEWTWO".to_string(),
            },
        ],
    );
    assert_eq!(
        BattleTowerFailureTextField::DuplicateHeldItem.subject(),
        "battle_tower_rules:duplicateHeldItemFailureText",
    );
}

#[test]
fn battle_tower_rules_deserialization_requires_compiled_roster_tables() {
    let mut rules = battle_tower_rules_with_banned_species(vec![]);
    rules.trainers = vec![BattleTowerTrainerDefinition {
        index: 0,
        trainer_class: "GENTLEMAN".to_string(),
        name: "EDWARD@".to_string(),
        sprite_constant: "SPRITE_GENTLEMAN".to_string(),
        female: false,
    }];
    rules.mon_groups = vec![vec![BattleTowerMonDefinition {
        species: "PERSIAN".to_string(),
        moves: vec![
            "SLASH".to_string(),
            "BITE".to_string(),
            "SCREECH".to_string(),
            "NO_MOVE".to_string(),
        ],
        stat_exp: vec![0; 5],
        dvs: vec![15; 4],
        pp: vec![20, 25, 40, 0],
        pokerus: vec![0; 3],
        level: 10,
        status: vec![0; 2],
        stats: vec![35, 35, 20, 20, 30, 20, 20],
        nickname: "PERSIAN".to_string(),
        ..BattleTowerMonDefinition::default()
    }]];
    let mut value = serde_json::to_value(&rules).expect("serialize exact rules");
    value
        .as_object_mut()
        .expect("rules object")
        .remove("trainers");
    let missing = serde_json::from_value::<BattleTowerRules>(value)
        .expect_err("missing trainer roster must fail pack decoding");
    assert!(missing.to_string().contains("missing field `trainers`"));

    let mut value = serde_json::to_value(&rules).expect("serialize exact rules");
    value["monGroups"] = serde_json::json!([]);
    let empty = serde_json::from_value::<BattleTowerRules>(value)
        .expect_err("empty Pokemon groups must fail pack decoding");
    assert!(
        empty
            .to_string()
            .contains("battle tower Pokemon groups must be present and nonempty")
    );

    for (field, malformed) in [
        ("moves", serde_json::json!(["SLASH", "BITE", "SCREECH"])),
        ("statExp", serde_json::json!([0, 0, 0, 0])),
        ("dvs", serde_json::json!([15, 15, 15])),
        ("pp", serde_json::json!([20, 25, 40])),
        ("pokerus", serde_json::json!([0, 0])),
        ("status", serde_json::json!([0])),
        ("stats", serde_json::json!([35, 35, 20, 20, 30, 20])),
    ] {
        let mut value = serde_json::to_value(&rules).expect("serialize exact rules");
        value["monGroups"][0][0][field] = malformed;
        let error = serde_json::from_value::<BattleTowerRules>(value)
            .expect_err("truncated ASM Pokemon record must fail pack decoding");
        assert!(
            error.to_string().contains(field),
            "unexpected {field} error: {error}"
        );
    }

    for (field, malformed) in [
        ("status", serde_json::json!([9, 0])),
        ("status", serde_json::json!([0, 1])),
        ("dvs", serde_json::json!([16, 15, 15, 15])),
        ("level", serde_json::json!(0)),
        ("experience", serde_json::json!(0x0100_0000_u32)),
    ] {
        let mut value = serde_json::to_value(&rules).expect("serialize exact rules");
        value["monGroups"][0][0][field] = malformed;
        let error = serde_json::from_value::<BattleTowerRules>(value)
            .expect_err("invalid ASM Pokemon record value must fail pack decoding");
        assert!(
            error.to_string().contains(field),
            "unexpected {field} error: {error}"
        );
    }
}

#[test]
fn battle_tower_status_bytes_decode_exact_crystal_bit_layout() {
    for (raw, expected_status, expected_sleep) in [
        (0x00, None, 0),
        (0x07, Some("SLEEP"), 7),
        (0x08, Some("POISON"), 0),
        (0x10, Some("BURN"), 0),
        (0x20, Some("FREEZE"), 0),
        (0x40, Some("PARALYSIS"), 0),
    ] {
        let (status, sleep_turns) =
            decode_battle_tower_status(&[raw, 0]).expect("valid Crystal status byte");
        assert_eq!(status.as_deref(), expected_status);
        assert_eq!(sleep_turns, expected_sleep);
    }
    for raw in [0x09, 0x18, 0x80] {
        assert!(
            decode_battle_tower_status(&[raw, 0]).is_err(),
            "mixed or reserved status byte {raw:#04x} must be rejected"
        );
    }
    assert!(decode_battle_tower_status(&[0, 1]).is_err());
}

#[test]
fn odd_egg_definition_issues_validate_exact_pack_rows() {
    let definitions = vec![
        OddEggDefinition {
            species: "CLE FFA".to_string(),
            moves: vec!["POU ND".to_string(), " ".to_string()],
            original_trainer_id: 768,
            dvs: [2, 10, 10, 10],
            probability: 0,
            level: 101,
            experience: 125,
            hatch_cycles: 20,
            nickname: " EGG".to_string(),
            original_trainer_name: String::new(),
        },
        OddEggDefinition {
            species: "CLEFFA".to_string(),
            moves: vec![
                "POUND".to_string(),
                "CHARM".to_string(),
                "DIZZY_PUNCH".to_string(),
                "SING".to_string(),
                "PRESENT".to_string(),
            ],
            original_trainer_id: 768,
            dvs: [2, 10, 10, 10],
            probability: 25,
            level: 5,
            experience: 125,
            hatch_cycles: 20,
            nickname: "EGG".to_string(),
            original_trainer_name: "ODD".to_string(),
        },
    ];
    let species_ids = BTreeSet::from(["CLEFFA".to_string()]);
    let move_ids = BTreeSet::from(["POUND".to_string()]);

    assert_eq!(
        odd_egg_definition_issues(&definitions, &species_ids, &move_ids),
        vec![
            OddEggDefinitionIssue::InvalidProbabilityTotal {
                total_probability: 25,
            },
            OddEggDefinitionIssue::InvalidSpecies {
                index: 0,
                species_id: "CLE FFA".to_string(),
            },
            OddEggDefinitionIssue::InvalidMove {
                index: 0,
                move_index: 0,
                move_id: "POU ND".to_string(),
            },
            OddEggDefinitionIssue::InvalidMove {
                index: 0,
                move_index: 1,
                move_id: " ".to_string(),
            },
            OddEggDefinitionIssue::InvalidProbability { index: 0 },
            OddEggDefinitionIssue::InvalidLevel {
                index: 0,
                level: 101,
            },
            OddEggDefinitionIssue::InvalidNickname {
                index: 0,
                nickname: " EGG".to_string(),
            },
            OddEggDefinitionIssue::InvalidOriginalTrainerName {
                index: 0,
                original_trainer_name: String::new(),
            },
            OddEggDefinitionIssue::InvalidMoveCount {
                index: 1,
                move_count: 5,
            },
            OddEggDefinitionIssue::UnknownMove {
                index: 1,
                move_index: 1,
                move_id: "CHARM".to_string(),
            },
            OddEggDefinitionIssue::UnknownMove {
                index: 1,
                move_index: 2,
                move_id: "DIZZY_PUNCH".to_string(),
            },
            OddEggDefinitionIssue::UnknownMove {
                index: 1,
                move_index: 3,
                move_id: "SING".to_string(),
            },
            OddEggDefinitionIssue::UnknownMove {
                index: 1,
                move_index: 4,
                move_id: "PRESENT".to_string(),
            },
        ],
    );
}

#[test]
fn dratini_move_set_issues_validate_exact_move_rows() {
    let move_sets = BTreeMap::from([
        (0, Vec::new()),
        (
            1,
            vec![
                String::new(),
                "EXTREMESPEED ".to_string(),
                "EXTREME SPEED".to_string(),
                "EXTREMESPEED".to_string(),
            ],
        ),
        (2, vec!["SURF".to_string()]),
    ]);
    let move_ids = BTreeSet::from(["SURF".to_string()]);

    assert_eq!(
        dratini_move_set_issues(&move_sets, &move_ids),
        vec![
            DratiniMoveSetIssue::EmptyMoveSet { mode: 0 },
            DratiniMoveSetIssue::InvalidMove {
                mode: 1,
                move_index: 0,
                move_id: String::new(),
            },
            DratiniMoveSetIssue::InvalidMove {
                mode: 1,
                move_index: 1,
                move_id: "EXTREMESPEED ".to_string(),
            },
            DratiniMoveSetIssue::InvalidMove {
                mode: 1,
                move_index: 2,
                move_id: "EXTREME SPEED".to_string(),
            },
            DratiniMoveSetIssue::UnknownMove {
                mode: 1,
                move_index: 3,
                move_id: "EXTREMESPEED".to_string(),
            },
        ],
    );
}

#[test]
fn bug_contest_config_issues_validate_exact_flags_and_counts() {
    let config = BugContestConfig {
        park_balls: 0,
        timer_minutes: 20,
        timer_seconds: 60,
        selected_contestant_count: 4,
        contestant_flags: vec![
            String::new(),
            "EVENT_BUG_CONTESTANT_1".to_string(),
            "EVENT_BUG_CONTESTANT_1".to_string(),
            "EVENT_MISSING".to_string(),
            "EVENT BUG".to_string(),
        ],
        encounters: test_bug_contest_encounters(),
    };
    let event_flags = BTreeSet::from(["EVENT_BUG_CONTESTANT_1".to_string()]);

    assert_eq!(
        bug_contest_config_issues(&config, &event_flags),
        vec![
            BugContestConfigIssue::MissingParkBalls,
            BugContestConfigIssue::InvalidTimerSeconds { timer_seconds: 60 },
            BugContestConfigIssue::InvalidContestantFlag {
                index: 0,
                flag: String::new(),
            },
            BugContestConfigIssue::DuplicateContestantFlag {
                index: 2,
                flag: "EVENT_BUG_CONTESTANT_1".to_string(),
            },
            BugContestConfigIssue::UnknownContestantFlag {
                index: 3,
                flag: "EVENT_MISSING".to_string(),
            },
            BugContestConfigIssue::InvalidContestantFlag {
                index: 4,
                flag: "EVENT BUG".to_string(),
            },
        ],
    );

    let too_few_flags = BugContestConfig {
        selected_contestant_count: 6,
        ..config
    };
    assert!(
        bug_contest_config_issues(&too_few_flags, &event_flags).contains(
            &BugContestConfigIssue::SelectedContestantCountExceedsFlags {
                selected_contestant_count: 6,
                contestant_flag_count: 5,
            }
        )
    );
}

#[test]
fn happiness_services_require_exact_party_slot_and_apply_exact_change_tables() {
    let mut state = GameState::default();
    let mut mon = pokemon("CHIKORITA");
    mon.happiness = 70;
    mon.nickname = "Leafy".to_string();
    state
        .storage
        .register_capture_in_box(0, mon)
        .expect("store mon");
    state.sync_party_from_storage();
    let move_catalog = moves();
    let species_catalog = BTreeMap::new();
    let item_catalog = BTreeMap::new();
    let happiness_data = HappinessData {
        changes: BTreeMap::from([
            (
                9,
                HappinessChangeEntry {
                    code: "HAPPINESS_OLDERCUT1".to_string(),
                    low: 1,
                    mid: 1,
                    high: 1,
                },
            ),
            (
                10,
                HappinessChangeEntry {
                    code: "HAPPINESS_OLDERCUT2".to_string(),
                    low: 3,
                    mid: 3,
                    high: 1,
                },
            ),
            (
                11,
                HappinessChangeEntry {
                    code: "HAPPINESS_OLDERCUT3".to_string(),
                    low: 5,
                    mid: 5,
                    high: 2,
                },
            ),
            (
                12,
                HappinessChangeEntry {
                    code: "HAPPINESS_YOUNGCUT1".to_string(),
                    low: 1,
                    mid: 1,
                    high: 1,
                },
            ),
            (
                13,
                HappinessChangeEntry {
                    code: "HAPPINESS_YOUNGCUT2".to_string(),
                    low: 3,
                    mid: 3,
                    high: 1,
                },
            ),
            (
                14,
                HappinessChangeEntry {
                    code: "HAPPINESS_YOUNGCUT3".to_string(),
                    low: 10,
                    mid: 10,
                    high: 4,
                },
            ),
            (
                18,
                HappinessChangeEntry {
                    code: "HAPPINESS_GROOMING".to_string(),
                    low: 3,
                    mid: 3,
                    high: 1,
                },
            ),
        ]),
        services: BTreeMap::from([
            (
                "OlderHaircutBrother".to_string(),
                vec![
                    HappinessServiceOutcome {
                        roll_weight: 76,
                        script_value: 2,
                        change_code: 9,
                    },
                    HappinessServiceOutcome {
                        roll_weight: 128,
                        script_value: 3,
                        change_code: 10,
                    },
                    HappinessServiceOutcome {
                        roll_weight: 255,
                        script_value: 4,
                        change_code: 11,
                    },
                ],
            ),
            (
                "YoungerHaircutBrother".to_string(),
                vec![
                    HappinessServiceOutcome {
                        roll_weight: 154,
                        script_value: 2,
                        change_code: 12,
                    },
                    HappinessServiceOutcome {
                        roll_weight: 76,
                        script_value: 3,
                        change_code: 13,
                    },
                    HappinessServiceOutcome {
                        roll_weight: 255,
                        script_value: 4,
                        change_code: 14,
                    },
                ],
            ),
            (
                "DaisysGrooming".to_string(),
                vec![HappinessServiceOutcome {
                    roll_weight: 255,
                    script_value: 2,
                    change_code: 18,
                }],
            ),
        ]),
    };

    let missing = apply_special_routine_with_context(
        &mut state,
        full_context_with_happiness_data(
            &move_catalog,
            &species_catalog,
            &EMPTY_TEST_LEARNSETS,
            &item_catalog,
            &happiness_data,
        ),
        "OlderHaircutBrother",
    )
    .expect_err("party slot required");
    assert!(matches!(
        missing,
        SpecialRoutineError::MissingScriptValue { routine, variable }
            if routine == "OlderHaircutBrother" && variable == "_party_slot"
    ));

    state
        .script_runtime
        .variables
        .insert("_party_slot".to_string(), "0".to_string());
    let happiness_before = state.storage.party.pokemon[0]
        .as_ref()
        .expect("party pokemon")
        .happiness;
    let missing_roll = apply_special_routine_with_context(
        &mut state,
        full_context_with_happiness_data(
            &move_catalog,
            &species_catalog,
            &EMPTY_TEST_LEARNSETS,
            &item_catalog,
            &happiness_data,
        ),
        "OlderHaircutBrother",
    )
    .expect_err("rng roll required");
    assert!(matches!(
        missing_roll,
        SpecialRoutineError::MissingScriptValue { routine, variable }
            if routine == "OlderHaircutBrother" && variable == "_rng_roll"
    ));
    assert_eq!(
        state.storage.party.pokemon[0]
            .as_ref()
            .expect("party pokemon")
            .happiness,
        happiness_before
    );
    assert_eq!(state.script_runtime.script_value, None);

    state
        .script_runtime
        .variables
        .insert("_rng_roll".to_string(), "0".to_string());
    let older = apply_special_routine_with_context(
        &mut state,
        full_context_with_happiness_data(
            &move_catalog,
            &species_catalog,
            &EMPTY_TEST_LEARNSETS,
            &item_catalog,
            &happiness_data,
        ),
        "OlderHaircutBrother",
    )
    .expect("older haircut");

    assert_eq!(
        older.effect,
        SpecialRoutineEffect::HappinessService {
            party_slot: 0,
            species: "CHIKORITA".to_string(),
            old_happiness: 70,
            new_happiness: 71,
            script_value: 2,
            change_code: 9
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("2"));
    assert_eq!(
        state
            .script_runtime
            .named_buffers
            .get("STRING_BUFFER_3")
            .map(String::as_str),
        Some("Leafy")
    );

    state
        .script_runtime
        .variables
        .insert("_rng_roll".to_string(), "200".to_string());
    let younger = apply_special_routine_with_context(
        &mut state,
        full_context_with_happiness_data(
            &move_catalog,
            &species_catalog,
            &EMPTY_TEST_LEARNSETS,
            &item_catalog,
            &happiness_data,
        ),
        "YoungerHaircutBrother",
    )
    .expect("younger haircut");

    assert_eq!(
        younger.effect,
        SpecialRoutineEffect::HappinessService {
            party_slot: 0,
            species: "CHIKORITA".to_string(),
            old_happiness: 71,
            new_happiness: 74,
            script_value: 3,
            change_code: 13
        }
    );

    let daisy = apply_special_routine_with_context(
        &mut state,
        full_context_with_happiness_data(
            &move_catalog,
            &species_catalog,
            &EMPTY_TEST_LEARNSETS,
            &item_catalog,
            &happiness_data,
        ),
        "DaisysGrooming",
    )
    .expect("daisy grooming");

    assert_eq!(
        daisy.effect,
        SpecialRoutineEffect::HappinessService {
            party_slot: 0,
            species: "CHIKORITA".to_string(),
            old_happiness: 74,
            new_happiness: 77,
            script_value: 2,
            change_code: 18
        }
    );
    assert_eq!(
        state.storage.party.pokemon[0]
            .as_ref()
            .expect("party mon")
            .happiness,
        77
    );
}

#[test]
fn happiness_services_require_modpack_data_without_change_table_fallback() {
    let mut state = GameState::default();
    state
        .storage
        .register_capture_in_box(0, pokemon("CHIKORITA"))
        .expect("store");
    state.sync_party_from_storage();
    state
        .script_runtime
        .variables
        .insert("_party_slot".to_string(), "0".to_string());
    let before = state.clone();

    let error = apply_special_routine(&mut state, &moves(), "DaisysGrooming")
        .expect_err("missing happiness data rejected");

    assert!(matches!(
        error,
        SpecialRoutineError::MissingHappinessData { routine }
            if routine == "DaisysGrooming"
    ));
    assert_eq!(state, before);
}

#[test]
fn name_rater_poke_seer_and_move_tutor_use_exact_script_inputs() {
    let mut state = GameState::default();
    let mut mon = pokemon("CHIKORITA");
    mon.nickname = "Leafy".to_string();
    mon.original_trainer_name = "KRIS".to_string();
    mon.original_trainer_id = 0x2222;
    mon.moves = vec![LearnedMove {
        name: "TACKLE".to_string(),
        current_pp: 35,
        pp_ups: 0,
    }];
    state
        .storage
        .register_capture_in_box(0, mon)
        .expect("store mon");
    state.sync_party_from_storage();

    state
        .script_runtime
        .variables
        .insert("_party_slot".to_string(), "0".to_string());
    state
        .script_runtime
        .variables
        .insert("_selected_nickname".to_string(), "Chiko".to_string());
    state.script_runtime.script_value = Some("73".to_string());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "73".to_string());
    let renamed = apply_special_routine(&mut state, &moves(), "NameRater").expect("name rater");

    assert_eq!(
        renamed.effect,
        SpecialRoutineEffect::NameRater {
            party_slot: 0,
            species: "CHIKORITA".to_string(),
            old_nickname: "Leafy".to_string(),
            new_nickname: "Chiko".to_string()
        }
    );
    assert_eq!(
        state.storage.party.pokemon[0]
            .as_ref()
            .expect("party mon")
            .nickname,
        "Chiko"
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("73"));
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("_value")
            .map(String::as_str),
        Some("73")
    );

    let seer = apply_special_routine(&mut state, &moves(), "PokeSeer").expect("poke seer");

    assert_eq!(
        seer.effect,
        SpecialRoutineEffect::PokeSeer {
            party_slot: 0,
            species: "CHIKORITA".to_string(),
            nickname: "Chiko".to_string(),
            original_trainer_name: "KRIS".to_string(),
            original_trainer_id: 0x2222
        }
    );
    assert!(
        !state
            .script_runtime
            .variables
            .contains_key("_poke_seer_ot_id")
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("73"));
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("_value")
            .map(String::as_str),
        Some("73")
    );

    let mut move_catalog = moves();
    move_catalog.insert("EMBER".to_string(), move_data("EMBER", 25));
    let happiness_data = HappinessData {
        changes: BTreeMap::from([(
            5,
            HappinessChangeEntry {
                code: "HAPPINESS_LEARNMOVE".to_string(),
                low: 1,
                mid: 1,
                high: 1,
            },
        )]),
        services: BTreeMap::from([(
            "MoveTutor".to_string(),
            vec![HappinessServiceOutcome {
                roll_weight: 255,
                script_value: 1,
                change_code: 5,
            }],
        )]),
    };
    let species_catalog = BTreeMap::new();
    let item_catalog = BTreeMap::new();
    let context = full_context_with_happiness_data(
        &move_catalog,
        &species_catalog,
        &EMPTY_TEST_LEARNSETS,
        &item_catalog,
        &happiness_data,
    );
    state
        .script_runtime
        .variables
        .insert("_move".to_string(), "ember".to_string());
    let lower = apply_special_routine_with_context(&mut state, context, "MoveTutor")
        .expect_err("lowercase move is not coerced");
    assert!(matches!(
        lower,
        SpecialRoutineError::UnknownMove { routine, move_id, .. }
            if routine == "MoveTutor" && move_id == "ember"
    ));

    state
        .script_runtime
        .variables
        .insert("_move".to_string(), "EMBER".to_string());
    let taught =
        apply_special_routine_with_context(&mut state, context, "MoveTutor").expect("move tutor");

    assert_eq!(
        taught.effect,
        SpecialRoutineEffect::MoveTutor {
            party_slot: 0,
            species: "CHIKORITA".to_string(),
            move_name: "EMBER".to_string(),
            learned: true
        }
    );
    assert_eq!(
        state.storage.party.pokemon[0]
            .as_ref()
            .expect("party mon")
            .moves
            .iter()
            .map(|known| known.name.as_str())
            .collect::<Vec<_>>(),
        vec!["TACKLE", "EMBER"]
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("_value")
            .map(String::as_str),
        Some("0")
    );

    let repeat =
        apply_special_routine_with_context(&mut state, context, "MoveTutor").expect("move known");
    assert_eq!(
        repeat.effect,
        SpecialRoutineEffect::MoveTutor {
            party_slot: 0,
            species: "CHIKORITA".to_string(),
            move_name: "EMBER".to_string(),
            learned: false
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("255"));
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("_value")
            .map(String::as_str),
        Some("255")
    );
}

#[test]
fn name_rater_treats_a_blank_selected_name_as_unchanged() {
    let mut state = GameState::default();
    let mon = pokemon("CHIKORITA");
    state
        .storage
        .register_capture_in_box(0, mon)
        .expect("store mon");
    state.sync_party_from_storage();
    state
        .script_runtime
        .variables
        .insert("_party_slot".to_string(), "0".to_string());
    state
        .script_runtime
        .variables
        .insert("_selected_nickname".to_string(), "          ".to_string());

    let rated = apply_special_routine(&mut state, &moves(), "NameRater")
        .expect("rate default species name");

    assert_eq!(
        state.storage.party.pokemon[0]
            .as_ref()
            .expect("party mon")
            .nickname,
        "CHIKORITA",
        "ASM treats an all-space naming result as unchanged"
    );
    assert_eq!(
        rated.effect,
        SpecialRoutineEffect::NameRater {
            party_slot: 0,
            species: "CHIKORITA".to_string(),
            old_nickname: "CHIKORITA".to_string(),
            new_nickname: "CHIKORITA".to_string(),
        }
    );
    assert_eq!(
        state
            .script_runtime
            .named_buffers
            .get("STRING_BUFFER_1")
            .map(String::as_str),
        Some("CHIKORITA")
    );
}

#[test]
fn photo_studio_never_prints_an_egg_or_coerces_an_invalid_party_slot_to_cancel() {
    let mut state = GameState::default();
    let mut egg = pokemon("CHIKORITA");
    egg.is_egg = true;
    state
        .storage
        .register_capture_in_box(0, egg)
        .expect("store Egg");
    state.sync_party_from_storage();
    state
        .script_runtime
        .variables
        .insert("_party_slot".to_string(), "0".to_string());

    let refused = apply_special_routine(&mut state, &moves(), "PhotoStudio")
        .expect("Photo Studio Egg branch");

    assert_eq!(
        refused.effect,
        SpecialRoutineEffect::PhotoStudio {
            party_slot: Some(0),
            species: Some("CHIKORITA".to_string()),
        }
    );
    assert!(state.script_runtime.active_pokemon_picture.is_none());
    assert_eq!(state.script_runtime.script_value, None);

    state
        .script_runtime
        .variables
        .insert("_party_slot".to_string(), "1".to_string());
    let before = state.clone();
    let error = apply_special_routine(&mut state, &moves(), "PhotoStudio")
        .expect_err("an empty party slot cannot become a source cancellation");
    assert!(matches!(
        error,
        SpecialRoutineError::InvalidPartySlot {
            routine,
            party_slot: 1,
        } if routine == "PhotoStudio"
    ));
    assert_eq!(state, before);
}

#[test]
fn active_service_specials_record_exact_state_backed_requests() {
    let mut state = GameState::default();
    state.money = 1200;
    state.moms_money = 345;
    state.coins = 99;
    state.link_battle_stats.wins = 7;
    state.link_battle_stats.losses = 3;
    state.link_battle_stats.draws = 1;
    state
        .storage
        .register_capture_in_box(0, pokemon("CHIKORITA"))
        .expect("store photo mon");
    state.sync_party_from_storage();
    state.script_runtime.script_value = Some("73".to_string());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "73".to_string());

    let bank = apply_special_routine(&mut state, &moves(), "BankOfMom").expect("bank");
    assert_eq!(
        bank.effect,
        SpecialRoutineEffect::BankOfMom {
            initialized: false,
            money: 1200,
            moms_money: 345
        }
    );
    assert!(!state.script_runtime.variables.contains_key("_bank_money"));
    assert!(!state.script_runtime.variables.contains_key("_mom_money"));
    assert_eq!(state.script_runtime.active_menu, None);
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("73"));
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("_value")
            .map(String::as_str),
        Some("73")
    );

    let move_catalog = moves();
    let coin_case = item_data("COIN_CASE");
    let item_catalog = BTreeMap::from([("COIN_CASE".to_string(), coin_case.clone())]);
    state.bag.add_item(&coin_case, 1).expect("add coin case");
    state
        .script_runtime
        .variables
        .insert("_coin_case_balance".to_string(), "stale".to_string());
    let context = full_context(
        &move_catalog,
        &EMPTY_TEST_SPECIES,
        &EMPTY_TEST_LEARNSETS,
        &item_catalog,
    );

    state.script_runtime.pending_slot_machine_input =
        Some(SlotMachineInput::Enter { lucky: false });
    let mut slot_divider = ReplayDivider::new(divider_trace_for_sub_values(1_u8..=64));
    apply_random_special_routine_with_context(
        &mut state,
        context,
        "SlotMachine",
        &mut slot_divider,
    )
    .expect("enter slot cabinet");
    assert!(
        !state
            .script_runtime
            .variables
            .contains_key("_coin_case_balance")
    );
    state.script_runtime.pending_slot_machine_input = Some(SlotMachineInput::Start {
        bet: 3,
        lucky: false,
    });
    let slot = apply_random_special_routine_with_context(
        &mut state,
        context,
        "SlotMachine",
        &mut slot_divider,
    )
    .expect("slot");
    let SpecialRoutineEffect::SlotMachineStarted {
        coins_before,
        bet,
        coins,
        random_state_after,
        ..
    } = slot.effect
    else {
        panic!("slot special returned non-slot effect");
    };
    assert_eq!(coins_before, 99);
    assert_eq!(bet, 3);
    assert_eq!(coins, 99 - 3);
    assert_eq!(state.coins, coins);
    assert_eq!(state.random_state, random_state_after);
    assert_eq!(
        slot_divider.consumed(),
        4,
        "first start samples the cabinet streak mode, then Slots_InitBias"
    );
    assert_eq!(
        state.script_runtime.script_value.as_deref(),
        Some(coins.to_string().as_str())
    );
    for reel in 1..=3 {
        let offsets = state
            .script_runtime
            .slot_machine
            .as_ref()
            .expect("active slot machine")
            .offsets;
        state.script_runtime.pending_slot_machine_input =
            Some(SlotMachineInput::StopReel { reel, offsets });
        let stopped = apply_random_special_routine_with_context(
            &mut state,
            context,
            "SlotMachine",
            &mut slot_divider,
        )
        .expect("stop slot reel");
        let SpecialRoutineEffect::SlotMachineReelStopped {
            reel: stopped_reel, ..
        } = stopped.effect
        else {
            panic!("slot special returned non-stop effect");
        };
        assert_eq!(stopped_reel, reel);
    }
    state.script_runtime.pending_slot_machine_input = Some(SlotMachineInput::ResolveResult);
    let result = apply_random_special_routine_with_context(
        &mut state,
        context,
        "SlotMachine",
        &mut slot_divider,
    )
    .expect("resolve stopped slot reels");
    let SpecialRoutineEffect::SlotMachineResult { payout, .. } = result.effect else {
        panic!("slot special returned non-result effect");
    };
    for expected_paid in 1..=payout {
        state.script_runtime.pending_slot_machine_input = Some(SlotMachineInput::PayoutFrame);
        let payout_step = apply_random_special_routine_with_context(
            &mut state,
            context,
            "SlotMachine",
            &mut slot_divider,
        )
        .expect("slot payout step");
        let SpecialRoutineEffect::SlotMachinePayout {
            payout_remaining,
            coins,
            ..
        } = payout_step.effect
        else {
            panic!("slot special returned non-payout effect");
        };
        assert_eq!(payout_remaining, payout - expected_paid);
        assert_eq!(coins, 99 - 3 + expected_paid);
    }

    stage_card_flip_state(&mut state, CardFlipPhase::Shuffled, 0);
    state.script_runtime.pending_card_flip_input = Some(CardFlipInput::ResumeAfterShuffle);
    let mut card_divider = ReplayDivider::new([]);
    let start = apply_random_special_routine_with_context(
        &mut state,
        context,
        "CardFlip",
        &mut card_divider,
    )
    .expect("card start");
    let SpecialRoutineEffect::CardFlipStarted {
        coins: staked_coins,
        ..
    } = start.effect
    else {
        panic!("card flip special returned non-start effect");
    };
    assert!(
        !state
            .script_runtime
            .variables
            .contains_key("_coin_case_balance")
    );
    state.script_runtime.pending_card_flip_input = Some(CardFlipInput::Reveal {
        which_card: 0,
        cursor_x: 2,
        cursor_y: 2,
    });
    let card = apply_random_special_routine_with_context(
        &mut state,
        context,
        "CardFlip",
        &mut card_divider,
    )
    .expect("card reveal");
    let SpecialRoutineEffect::CardFlipRevealed {
        coins_before,
        card_index,
        card_name,
        payout,
        coins,
        random_state_after,
        ..
    } = card.effect
    else {
        panic!("card flip special returned non-card effect");
    };
    assert_eq!(coins_before, staked_coins);
    assert!(card_index < 24);
    assert!(!card_name.is_empty());
    assert_eq!(coins, coins_before);
    assert_eq!(state.coins, coins);
    assert_eq!(state.random_state, random_state_after);
    assert_eq!(card_divider.consumed(), 0);
    for expected in 1..=payout {
        state.script_runtime.pending_card_flip_input = Some(CardFlipInput::PayoutFrame);
        let payout_step = apply_random_special_routine_with_context(
            &mut state,
            context,
            "CardFlip",
            &mut card_divider,
        )
        .expect("card payout step");
        let SpecialRoutineEffect::CardFlipPayout { coins, .. } = payout_step.effect else {
            panic!("card flip special returned non-payout effect");
        };
        assert_eq!(coins, coins_before + expected);
    }

    stage_memory_game_board(&mut state, 5);
    state.random_state = CrystalRandomState::default();
    let mut memory_divider = ReplayDivider::new(divider_trace_for_sub_values(1_u8..=15));
    let memory = resolve_memory_game_pair(&mut state, context, &mut memory_divider, 0, 1);
    let SpecialRoutineEffect::UnusedMemoryGame {
        matched,
        symbol,
        first_index,
        second_index,
        coins,
        random_state_after,
        ..
    } = memory.effect
    else {
        panic!("memory game returned non-memory effect");
    };
    assert!(matched);
    assert_eq!(symbol.as_deref(), Some("1"));
    assert_eq!((first_index, second_index), (0, 1));
    assert_eq!(coins, state.coins);
    assert_eq!(state.random_state, random_state_after);
    assert_eq!(memory_divider.consumed(), 0);
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));
    let stored_memory = state
        .script_runtime
        .memory_game
        .as_ref()
        .expect("stored Memory Game state");
    assert_eq!(&stored_memory.cards[..2], &[u8::MAX, u8::MAX]);
    assert_eq!(stored_memory.number_tries_remaining, 4);
    assert_eq!(state.script_runtime.active_menu, None);

    state.script_runtime.script_value = Some("73".to_string());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "73".to_string());
    let link_record =
        apply_special_routine(&mut state, &moves(), "DisplayLinkRecord").expect("link record");
    assert_eq!(
        link_record.effect,
        SpecialRoutineEffect::DisplayLinkRecord {
            wins: 7,
            losses: 3,
            draws: 1
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("73"));
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("_value")
            .map(String::as_str),
        Some("73")
    );
    assert!(
        !state
            .script_runtime
            .variables
            .contains_key("_link_battle_wins")
    );
    assert!(
        !state
            .script_runtime
            .variables
            .contains_key("_link_battle_losses")
    );
    assert!(
        !state
            .script_runtime
            .variables
            .contains_key("_link_battle_draws")
    );

    let trainer_house =
        apply_special_routine(&mut state, &moves(), "TrainerHouse").expect("trainer house");
    assert_eq!(
        trainer_house.effect,
        SpecialRoutineEffect::TrainerHouse { enabled: false }
    );
    assert_eq!(state.pending_special_battle_type, None);

    let before_missing_photo_slot = state.clone();
    let error = apply_special_routine(&mut state, &moves(), "PhotoStudio")
        .expect_err("photo studio requires an explicit party slot");
    assert!(matches!(
        error,
        SpecialRoutineError::MissingScriptValue { routine, variable }
            if routine == "PhotoStudio" && variable == "_party_slot"
    ));
    assert_eq!(state, before_missing_photo_slot);

    state
        .script_runtime
        .variables
        .insert("_party_slot".to_string(), "0".to_string());
    let photo = apply_special_routine(&mut state, &moves(), "PhotoStudio").expect("photo");
    assert_eq!(
        photo.effect,
        SpecialRoutineEffect::PhotoStudio {
            party_slot: Some(0),
            species: Some("CHIKORITA".to_string())
        }
    );
    assert_eq!(
        state.script_runtime.active_pokemon_picture.as_deref(),
        Some("CHIKORITA")
    );

    state.script_runtime.script_value = Some("1".to_string());
    let cancel = apply_special_routine(&mut state, &moves(), "Menu_ChallengeExplanationCancel")
        .expect("open challenge explanation menu");
    assert_eq!(
        cancel.effect,
        SpecialRoutineEffect::BattleTowerChallengeExplanationCancel {
            english: true,
            selection: None,
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("4"));
}

#[test]
fn game_corner_services_reject_missing_or_invalid_runtime_state_atomically() {
    let move_catalog = moves();
    let coin_case = item_data("COIN_CASE");
    let item_catalog = BTreeMap::from([("COIN_CASE".to_string(), coin_case.clone())]);
    let context = full_context(
        &move_catalog,
        &EMPTY_TEST_SPECIES,
        &EMPTY_TEST_LEARNSETS,
        &item_catalog,
    );
    let mut state = GameState::default();
    state.coins = 99;
    state.bag.add_item(&coin_case, 1).expect("add coin case");

    let before = state.clone();
    let mut divider = ReplayDivider::new([]);
    let error =
        apply_random_special_routine_with_context(&mut state, context, "SlotMachine", &mut divider)
            .expect_err("slot action is required");
    assert!(matches!(
        error,
        RandomSpecialRoutineError::Routine(SpecialRoutineError::MissingScriptValue {
            routine: error_routine,
            variable: error_variable,
        }) if error_routine == "SlotMachine" && error_variable == "pending_slot_machine_input"
    ));
    assert_eq!(state, before);

    let before = state.clone();
    let mut divider = ReplayDivider::new([]);
    let error =
        apply_random_special_routine_with_context(&mut state, context, "CardFlip", &mut divider)
            .expect_err("Card Flip requires a typed source input");
    assert!(matches!(
        error,
        RandomSpecialRoutineError::Routine(SpecialRoutineError::MissingScriptValue {
            routine: error_routine,
            variable: error_variable,
        }) if error_routine == "CardFlip" && error_variable == "pending_card_flip_input"
    ));
    assert_eq!(state, before);

    state.script_runtime.pending_memory_game_input =
        Some(MemoryGameInput::Enter { menu_cursor_y: 4 });
    let before = state.clone();
    let mut divider = ReplayDivider::new([]);
    let error = apply_random_special_routine_with_context(
        &mut state,
        context,
        "UnusedMemoryGame",
        &mut divider,
    )
    .expect_err("Memory Game requires an exact source menu row");
    assert!(matches!(
        error,
        RandomSpecialRoutineError::Routine(SpecialRoutineError::InvalidState {
            routine: error_routine,
            message,
        }) if error_routine == "UnusedMemoryGame" && message.contains("row 1..=3")
    ));
    assert_eq!(state, before);

    state.script_runtime.pending_slot_machine_input = Some(SlotMachineInput::Start {
        bet: 4,
        lucky: false,
    });
    let before = state.clone();
    let mut divider = ReplayDivider::new([]);
    assert!(matches!(
        apply_random_special_routine_with_context(&mut state, context, "SlotMachine", &mut divider,),
        Err(RandomSpecialRoutineError::Routine(
            SpecialRoutineError::InvalidState { .. }
        ))
    ));
    assert_eq!(state, before);
}

#[test]
fn slot_machine_stakes_before_paying_each_award_coin_separately() {
    let move_catalog = moves();
    let coin_case = item_data("COIN_CASE");
    let item_catalog = BTreeMap::from([("COIN_CASE".to_string(), coin_case.clone())]);
    let context = full_context(
        &move_catalog,
        &EMPTY_TEST_SPECIES,
        &EMPTY_TEST_LEARNSETS,
        &item_catalog,
    );
    let mut state = GameState::default();
    state.coins = 99;
    state.bag.add_item(&coin_case, 1).expect("add coin case");
    stage_slot_machine_betting_state(&mut state, false);
    state.script_runtime.pending_slot_machine_input = Some(SlotMachineInput::Start {
        bet: 3,
        lucky: false,
    });
    let mut divider = ReplayDivider::new(std::iter::repeat_n([0_u8, 255_u8], 32).flatten());

    let start =
        apply_random_special_routine_with_context(&mut state, context, "SlotMachine", &mut divider)
            .expect("start slot spin");
    let SpecialRoutineEffect::SlotMachineStarted { coins, bias, .. } = start.effect else {
        panic!("expected staged slot start");
    };
    assert_eq!(bias.as_deref(), Some("SEVEN"));
    assert_eq!(coins, 96, "starting the reels commits only the stake");
    assert_eq!(state.coins, 96);
    for reel in 1..=3 {
        state.script_runtime.pending_slot_machine_input = Some(SlotMachineInput::StopReel {
            reel,
            offsets: [14; 3],
        });
        let stop = apply_random_special_routine_with_context(
            &mut state,
            context,
            "SlotMachine",
            &mut divider,
        )
        .expect("stop staged slot reel");
        let SpecialRoutineEffect::SlotMachineReelStopped { .. } = stop.effect else {
            panic!("expected staged reel stop");
        };
    }
    state.script_runtime.pending_slot_machine_input = Some(SlotMachineInput::ResolveResult);
    let result =
        apply_random_special_routine_with_context(&mut state, context, "SlotMachine", &mut divider)
            .expect("resolve staged slot result");
    let SpecialRoutineEffect::SlotMachineResult { payout, .. } = result.effect else {
        panic!("expected staged slot result");
    };
    assert_eq!(payout, 300);
    assert_eq!(
        state
            .script_runtime
            .slot_machine
            .as_ref()
            .map(|machine| machine.payout_remaining),
        Some(300)
    );

    state.script_runtime.pending_slot_machine_input = Some(SlotMachineInput::PayoutFrame);
    let payout_step =
        apply_random_special_routine_with_context(&mut state, context, "SlotMachine", &mut divider)
            .expect("first slot payout coin");
    assert!(matches!(
        payout_step.effect,
        SpecialRoutineEffect::SlotMachinePayout {
            coins_before: 96,
            payout_remaining: 299,
            coins: 97,
            ..
        }
    ));
    assert_eq!(state.coins, 97);
}

#[test]
fn slot_machine_uses_the_source_result_replay_and_quit_boundaries() {
    let move_catalog = moves();
    let coin_case = item_data("COIN_CASE");
    let item_catalog = BTreeMap::from([("COIN_CASE".to_string(), coin_case.clone())]);
    let context = full_context(
        &move_catalog,
        &EMPTY_TEST_SPECIES,
        &EMPTY_TEST_LEARNSETS,
        &item_catalog,
    );
    let mut state = GameState::default();
    state.coins = 10;
    state.bag.add_item(&coin_case, 1).expect("add coin case");
    stage_slot_machine_betting_state(&mut state, false);
    let machine = state.script_runtime.slot_machine.as_mut().unwrap();
    machine.phase = SlotMachinePhase::Result;
    machine.next_reel = 4;
    state.script_runtime.pending_slot_machine_input = Some(SlotMachineInput::AcknowledgeResult);
    let mut divider = ReplayDivider::new([]);

    let acknowledged =
        apply_random_special_routine_with_context(&mut state, context, "SlotMachine", &mut divider)
            .expect("acknowledge source result");
    assert!(matches!(
        acknowledged.effect,
        SpecialRoutineEffect::SlotMachineResultAcknowledged {
            can_play_again: true,
            ..
        }
    ));
    assert_eq!(
        state
            .script_runtime
            .slot_machine
            .as_ref()
            .map(|game| game.phase),
        Some(SlotMachinePhase::PlayAgain)
    );

    state.script_runtime.pending_slot_machine_input = Some(SlotMachineInput::Continue);
    let continued =
        apply_random_special_routine_with_context(&mut state, context, "SlotMachine", &mut divider)
            .expect("accept source replay prompt");
    assert!(matches!(
        continued.effect,
        SpecialRoutineEffect::SlotMachineReplayAccepted { .. }
    ));
    state.script_runtime.pending_slot_machine_input = Some(SlotMachineInput::Quit);
    let exited =
        apply_random_special_routine_with_context(&mut state, context, "SlotMachine", &mut divider)
            .expect("quit from source betting prompt");
    assert!(matches!(
        exited.effect,
        SpecialRoutineEffect::SlotMachineExited { .. }
    ));

    let mut empty = state;
    empty.coins = 0;
    let machine = empty.script_runtime.slot_machine.as_mut().unwrap();
    machine.phase = SlotMachinePhase::Result;
    machine.next_reel = 4;
    machine.payout_remaining = 0;
    empty.script_runtime.pending_slot_machine_input = Some(SlotMachineInput::AcknowledgeResult);
    let ran_out =
        apply_random_special_routine_with_context(&mut empty, context, "SlotMachine", &mut divider)
            .expect("source exits automatically after result at zero coins");
    assert!(matches!(
        ran_out.effect,
        SpecialRoutineEffect::SlotMachineResultAcknowledged {
            can_play_again: false,
            ..
        }
    ));
    assert_eq!(
        empty
            .script_runtime
            .slot_machine
            .as_ref()
            .map(|game| game.phase),
        Some(SlotMachinePhase::Quit)
    );
    assert_eq!(divider.consumed(), 0);
}

#[test]
fn slot_reel_two_samples_skip_rng_only_after_a_visible_seven_check() {
    let mut no_seven_divider = ReplayDivider::new([0_u8, 255_u8]);
    let mut no_seven_rng = CrystalRandom::new(
        crate::random::CrystalRandomState::default(),
        &mut no_seven_divider,
    );
    let mut offsets = [7, 14, 14];
    let state_before = no_seven_rng.state();
    assert_eq!(
        slot_stop_reel2(&mut offsets, None, 3, &mut no_seven_rng)
            .expect("normal reel-two stop")
            .mode,
        SlotStopMode::Normal
    );
    let state_after = no_seven_rng.state();
    drop(no_seven_rng);
    assert_eq!(
        no_seven_divider.consumed(),
        0,
        "Slots_StopReel2 returns before Random when reel one has no visible seven"
    );
    assert_eq!(state_after, state_before);

    let mut seven_divider = ReplayDivider::new([0_u8, 255_u8]);
    let mut seven_rng = CrystalRandom::new(
        crate::random::CrystalRandomState::default(),
        &mut seven_divider,
    );
    let mut offsets = [14, 14, 14];
    assert_eq!(
        slot_stop_reel2(&mut offsets, None, 3, &mut seven_rng)
            .expect("eligible reel-two skip check")
            .mode,
        SlotStopMode::SkipToSeven
    );
    drop(seven_rng);
    assert_eq!(seven_divider.consumed(), 2);
}

#[test]
fn slot_reel_three_special_modes_escape_a_match_and_return_exact_actor_counts() {
    let cases = [
        (120_u8, SlotStopMode::Slow, 29_u8),
        (60_u8, SlotStopMode::Golem, 14_u8),
        (0_u8, SlotStopMode::Chansey, 7_u8),
    ];
    for (roll, expected_mode, expected_count) in cases {
        let mut divider = ReplayDivider::new(divider_trace_for_sub_values([roll]));
        let mut rng =
            CrystalRandom::new(crate::random::CrystalRandomState::default(), &mut divider);
        let mut offsets = [14, 14, 14];
        let resolution = slot_stop_reel3(&mut offsets, Some(SlotSymbol::Seven), 3, &mut rng)
            .expect("resolve reel-three special mode");
        drop(rng);

        assert_eq!(resolution.mode, expected_mode);
        assert_eq!(
            resolution.animation_start_offset, 0,
            "the source action keeps spinning until the already-matched seven window is gone"
        );
        assert_eq!(resolution.animation_count, expected_count);
        assert_eq!(offsets[2], 14);
        assert_eq!(divider.consumed(), 2);
    }
}

#[test]
fn card_flip_shuffle_consumes_exact_retry_stream_and_places_remaining_zero() {
    let move_catalog = moves();
    let coin_case = item_data("COIN_CASE");
    let item_catalog = BTreeMap::from([("COIN_CASE".to_string(), coin_case.clone())]);
    let context = full_context(
        &move_catalog,
        &EMPTY_TEST_SPECIES,
        &EMPTY_TEST_LEARNSETS,
        &item_catalog,
    );
    let mut state = GameState::default();
    state.coins = 99;
    state.bag.add_item(&coin_case, 1).expect("add coin case");
    state.script_runtime.pending_card_flip_input = Some(CardFlipInput::Start);
    // $1f is out of range, then the second zero collides with the
    // already occupied first slot. The 23 accepted positions are 0..22,
    // leaving the implicit face zero in slot 23 exactly as the ASM does.
    let samples = [31, 0, 0].into_iter().chain(1_u8..=22).collect::<Vec<_>>();
    let mut divider = ReplayDivider::new(divider_trace_for_sub_values(samples));

    let outcome =
        apply_random_special_routine_with_context(&mut state, context, "CardFlip", &mut divider)
            .expect("exact Card Flip shuffle");

    let SpecialRoutineEffect::CardFlipStarted {
        deck,
        random_state_after,
        ..
    } = outcome.effect
    else {
        panic!("expected CardFlip effect");
    };
    assert_eq!(
        deck,
        (1_u8..=23)
            .rev()
            .chain(std::iter::once(0))
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
    );
    assert_eq!(random_state_after, CrystalRandomState { add: 0, sub: 22 });
    assert_eq!(divider.consumed(), 50);
}

#[test]
fn card_flip_stakes_and_shuffles_before_card_selection_then_reveals_without_second_stake() {
    let move_catalog = moves();
    let coin_case = item_data("COIN_CASE");
    let item_catalog = BTreeMap::from([("COIN_CASE".to_string(), coin_case.clone())]);
    let context = full_context(
        &move_catalog,
        &EMPTY_TEST_SPECIES,
        &EMPTY_TEST_LEARNSETS,
        &item_catalog,
    );
    let mut state = GameState::default();
    state.coins = 3;
    state.bag.add_item(&coin_case, 1).expect("add coin case");
    state.script_runtime.pending_card_flip_input = Some(CardFlipInput::Start);
    let mut shuffle_divider = ReplayDivider::new(divider_trace_for_sub_values(0_u8..=22));

    apply_random_special_routine_with_context(
        &mut state,
        context,
        "CardFlip",
        &mut shuffle_divider,
    )
    .expect("Card Flip pre-choice transaction");

    assert_eq!(
        state.coins, 0,
        "the three-coin stake precedes card selection"
    );
    assert_eq!(
        state
            .script_runtime
            .card_flip
            .as_ref()
            .expect("active Card Flip")
            .deck
            .len(),
        24
    );
    assert_eq!(
        state
            .script_runtime
            .card_flip
            .as_ref()
            .expect("active Card Flip")
            .discard_pile,
        vec![false; 24]
    );

    state.script_runtime.pending_card_flip_input = Some(CardFlipInput::Reveal {
        which_card: 0,
        cursor_x: 5,
        cursor_y: 7,
    });
    let mut reveal_divider = ReplayDivider::new([]);
    let reveal = apply_random_special_routine_with_context(
        &mut state,
        context,
        "CardFlip",
        &mut reveal_divider,
    )
    .expect("Card Flip reveal");
    let SpecialRoutineEffect::CardFlipRevealed { payout, coins, .. } = reveal.effect else {
        panic!("expected Card Flip reveal effect");
    };
    assert_eq!(payout, 72);
    assert_eq!(
        coins, 0,
        "reveal must not pay the result before its source loop"
    );
    assert_eq!(state.coins, 0);
    assert_eq!(
        state
            .script_runtime
            .card_flip
            .as_ref()
            .map(|game| game.num_cards_played),
        Some(0),
        "the source increments wCardFlipNumCardsPlayed only after accepting Play Again"
    );
    let discard_pile = state
        .script_runtime
        .card_flip
        .as_ref()
        .expect("active Card Flip")
        .discard_pile
        .clone();
    assert!(!discard_pile[0], "deck position is not the discard key");
    assert_eq!(
        discard_pile[23], true,
        "wDiscardPile is indexed by the revealed face identity"
    );

    for expected in 1..=payout {
        state.script_runtime.pending_card_flip_input = Some(CardFlipInput::PayoutFrame);
        let payout_step = apply_random_special_routine_with_context(
            &mut state,
            context,
            "CardFlip",
            &mut reveal_divider,
        )
        .expect("Card Flip one-coin payout step");
        let SpecialRoutineEffect::CardFlipPayout {
            payout_remaining,
            coins,
            ..
        } = payout_step.effect
        else {
            panic!("expected Card Flip payout effect");
        };
        assert_eq!(coins, expected);
        assert_eq!(payout_remaining, payout - expected);
    }

    state.script_runtime.pending_card_flip_input = Some(CardFlipInput::PayoutFrame);
    let before_overpay = state.clone();
    let overpay = apply_random_special_routine_with_context(
        &mut state,
        context,
        "CardFlip",
        &mut reveal_divider,
    );
    assert!(matches!(
        overpay,
        Err(RandomSpecialRoutineError::Routine(
            SpecialRoutineError::InvalidState { message, .. }
        )) if message.contains("positive result payout")
    ));
    assert_eq!(state, before_overpay);

    state.script_runtime.pending_card_flip_input = Some(CardFlipInput::AcknowledgeResult);
    apply_random_special_routine_with_context(&mut state, context, "CardFlip", &mut reveal_divider)
        .expect("acknowledge the source result page");
    state.script_runtime.pending_card_flip_input = Some(CardFlipInput::Continue);
    apply_random_special_routine_with_context(&mut state, context, "CardFlip", &mut reveal_divider)
        .expect("the next source round deducts another stake without resetting the deck");
    assert_eq!(
        state
            .script_runtime
            .card_flip
            .as_ref()
            .map(|game| game.num_cards_played),
        Some(1)
    );
}

#[test]
fn card_flip_rejects_a_card_outside_the_current_source_pair() {
    let move_catalog = moves();
    let coin_case = item_data("COIN_CASE");
    let item_catalog = BTreeMap::from([("COIN_CASE".to_string(), coin_case.clone())]);
    let context = full_context(
        &move_catalog,
        &EMPTY_TEST_SPECIES,
        &EMPTY_TEST_LEARNSETS,
        &item_catalog,
    );
    let mut state = GameState::default();
    state.coins = 9;
    state.bag.add_item(&coin_case, 1).expect("add coin case");
    stage_card_flip_state(&mut state, CardFlipPhase::ChooseCard, 0);
    let mut divider = ReplayDivider::new([]);
    state.script_runtime.pending_card_flip_input = Some(CardFlipInput::Reveal {
        which_card: 2,
        cursor_x: 2,
        cursor_y: 2,
    });
    let before = state.clone();
    let result =
        apply_random_special_routine_with_context(&mut state, context, "CardFlip", &mut divider);

    assert!(matches!(
        result,
        Err(RandomSpecialRoutineError::Routine(
            SpecialRoutineError::InvalidState { message, .. }
        )) if message.contains("source card/bet grids")
    ));
    assert_eq!(state, before);
}

#[test]
fn card_flip_reshuffles_only_after_the_twelfth_played_card() {
    let move_catalog = moves();
    let coin_case = item_data("COIN_CASE");
    let item_catalog = BTreeMap::from([("COIN_CASE".to_string(), coin_case.clone())]);
    let context = full_context(
        &move_catalog,
        &EMPTY_TEST_SPECIES,
        &EMPTY_TEST_LEARNSETS,
        &item_catalog,
    );
    let mut state = GameState::default();
    state.coins = 9;
    state.bag.add_item(&coin_case, 1).expect("add coin case");
    stage_card_flip_state(&mut state, CardFlipPhase::PlayAgain, 10);
    state.script_runtime.pending_card_flip_input = Some(CardFlipInput::Continue);
    let mut divider = ReplayDivider::new([]);
    let ordinary =
        apply_random_special_routine_with_context(&mut state, context, "CardFlip", &mut divider)
            .expect("the eleventh round keeps the current deck");
    assert!(matches!(
        ordinary.effect,
        SpecialRoutineEffect::CardFlipStarted { .. }
    ));
    assert_eq!(
        state
            .script_runtime
            .card_flip
            .as_ref()
            .map(|game| game.num_cards_played),
        Some(11)
    );
    assert_eq!(divider.consumed(), 0);

    stage_card_flip_state(&mut state, CardFlipPhase::PlayAgain, 11);
    state.script_runtime.pending_card_flip_input = Some(CardFlipInput::Continue);
    let mut reshuffle_divider = ReplayDivider::new(divider_trace_for_sub_values_after(
        state.random_state.sub,
        0_u8..=22,
    ));
    let outcome = apply_random_special_routine_with_context(
        &mut state,
        context,
        "CardFlip",
        &mut reshuffle_divider,
    )
    .expect("the twelfth played card triggers the source reshuffle");
    assert!(matches!(
        outcome.effect,
        SpecialRoutineEffect::CardFlipShuffled { .. }
    ));
    assert_eq!(
        state
            .script_runtime
            .card_flip
            .as_ref()
            .map(|game| game.num_cards_played),
        Some(0)
    );
    assert_eq!(
        state
            .script_runtime
            .card_flip
            .as_ref()
            .map(|game| game.discard_pile.clone()),
        Some(vec![false; 24])
    );
    assert_eq!(reshuffle_divider.consumed(), 46);
}

#[test]
fn card_flip_shuffles_before_reporting_an_insufficient_stake() {
    let move_catalog = moves();
    let coin_case = item_data("COIN_CASE");
    let item_catalog = BTreeMap::from([("COIN_CASE".to_string(), coin_case.clone())]);
    let context = full_context(
        &move_catalog,
        &EMPTY_TEST_SPECIES,
        &EMPTY_TEST_LEARNSETS,
        &item_catalog,
    );
    let mut state = GameState::default();
    state.coins = 2;
    state.bag.add_item(&coin_case, 1).expect("add coin case");
    state.script_runtime.pending_card_flip_input = Some(CardFlipInput::Start);
    let mut divider = ReplayDivider::new(divider_trace_for_sub_values(0_u8..=22));

    let result =
        apply_random_special_routine_with_context(&mut state, context, "CardFlip", &mut divider)
            .expect("Card Flip insufficient stake check");

    assert!(matches!(
        result.effect,
        SpecialRoutineEffect::GameCornerGameUnavailable {
            reason: GameCornerUnavailableReason::InsufficientStake,
            ..
        }
    ));
    assert_eq!(state.coins, 2);
    assert_eq!(divider.consumed(), 46);
    let card_flip = state
        .script_runtime
        .card_flip
        .as_ref()
        .expect("the source shuffles before DeductCoins");
    assert_eq!(card_flip.deck.len(), 24);
    assert_eq!(card_flip.phase, CardFlipPhase::Quit);
}

#[test]
fn memory_game_uses_frame_joypad_input_with_source_cursor_latency() {
    let move_catalog = moves();
    let coin_case = item_data("COIN_CASE");
    let item_catalog = BTreeMap::from([("COIN_CASE".to_string(), coin_case.clone())]);
    let context = full_context(
        &move_catalog,
        &EMPTY_TEST_SPECIES,
        &EMPTY_TEST_LEARNSETS,
        &item_catalog,
    );
    let mut state = GameState::default();
    state.coins = 99;
    state.bag.add_item(&coin_case, 1).expect("add coin case");
    stage_memory_game_board(&mut state, 5);
    let mut divider = ReplayDivider::new([]);

    let started = apply_memory_game_input(
        &mut state,
        context,
        &mut divider,
        MemoryGameInput::AdvanceFrame { button: None },
    );
    assert!(matches!(
        started.effect,
        SpecialRoutineEffect::UnusedMemoryGameTryStarted {
            tries_remaining: 4,
            ..
        }
    ));

    let pressed = apply_memory_game_input(
        &mut state,
        context,
        &mut divider,
        MemoryGameInput::AdvanceFrame {
            button: Some(MemoryGameButton::A),
        },
    );
    assert!(matches!(
        pressed.effect,
        SpecialRoutineEffect::UnusedMemoryGameFrameAdvanced {
            phase: MemoryGamePhase::PickCard1,
            cursor_index: 0,
            ..
        }
    ));
    assert_eq!(
        state
            .script_runtime
            .memory_game
            .as_ref()
            .unwrap()
            .card_choice,
        1
    );

    let picked = apply_memory_game_input(
        &mut state,
        context,
        &mut divider,
        MemoryGameInput::AdvanceFrame { button: None },
    );
    assert!(
        matches!(picked.effect, SpecialRoutineEffect::UnusedMemoryGameFirstCardPicked { card_index: 0, ref symbol, tries_remaining: 4, .. } if symbol == "1")
    );

    apply_memory_game_input(
        &mut state,
        context,
        &mut divider,
        MemoryGameInput::AdvanceFrame {
            button: Some(MemoryGameButton::A),
        },
    );
    let rejected = apply_memory_game_input(
        &mut state,
        context,
        &mut divider,
        MemoryGameInput::AdvanceFrame { button: None },
    );
    assert!(matches!(
        rejected.effect,
        SpecialRoutineEffect::UnusedMemoryGameCardRejected {
            pick: 2,
            card_index: 0,
            tries_remaining: 4,
            ..
        }
    ));
    assert_eq!(
        state
            .script_runtime
            .memory_game
            .as_ref()
            .unwrap()
            .card_choice,
        1
    );
    assert_eq!(divider.consumed(), 0);
}

#[test]
fn memory_game_cursor_obeys_source_bounds_and_ignores_b() {
    let move_catalog = moves();
    let coin_case = item_data("COIN_CASE");
    let item_catalog = BTreeMap::from([("COIN_CASE".to_string(), coin_case.clone())]);
    let context = full_context(
        &move_catalog,
        &EMPTY_TEST_SPECIES,
        &EMPTY_TEST_LEARNSETS,
        &item_catalog,
    );
    let mut state = GameState::default();
    state.coins = 99;
    state.bag.add_item(&coin_case, 1).expect("add coin case");
    stage_memory_game_board(&mut state, 5);
    state.script_runtime.memory_game.as_mut().unwrap().phase = MemoryGamePhase::PickCard1;
    let mut divider = ReplayDivider::new([]);

    for button in [
        MemoryGameButton::Left,
        MemoryGameButton::Up,
        MemoryGameButton::B,
    ] {
        apply_memory_game_input(
            &mut state,
            context,
            &mut divider,
            MemoryGameInput::AdvanceFrame {
                button: Some(button),
            },
        );
        assert_eq!(
            state
                .script_runtime
                .memory_game
                .as_ref()
                .unwrap()
                .cursor_index,
            0
        );
    }
    for _ in 0..8 {
        apply_memory_game_input(
            &mut state,
            context,
            &mut divider,
            MemoryGameInput::AdvanceFrame {
                button: Some(MemoryGameButton::Right),
            },
        );
    }
    assert_eq!(
        state
            .script_runtime
            .memory_game
            .as_ref()
            .unwrap()
            .cursor_index,
        8
    );
    apply_memory_game_input(
        &mut state,
        context,
        &mut divider,
        MemoryGameInput::AdvanceFrame {
            button: Some(MemoryGameButton::Right),
        },
    );
    assert_eq!(
        state
            .script_runtime
            .memory_game
            .as_ref()
            .unwrap()
            .cursor_index,
        8
    );
    for _ in 0..4 {
        apply_memory_game_input(
            &mut state,
            context,
            &mut divider,
            MemoryGameInput::AdvanceFrame {
                button: Some(MemoryGameButton::Down),
            },
        );
    }
    assert_eq!(
        state
            .script_runtime
            .memory_game
            .as_ref()
            .unwrap()
            .cursor_index,
        44
    );
    apply_memory_game_input(
        &mut state,
        context,
        &mut divider,
        MemoryGameInput::AdvanceFrame {
            button: Some(MemoryGameButton::Down),
        },
    );
    assert_eq!(
        state
            .script_runtime
            .memory_game
            .as_ref()
            .unwrap()
            .cursor_index,
        44
    );
}

#[test]
fn memory_game_matches_after_the_exact_source_delay() {
    let move_catalog = moves();
    let coin_case = item_data("COIN_CASE");
    let item_catalog = BTreeMap::from([("COIN_CASE".to_string(), coin_case.clone())]);
    let context = full_context(
        &move_catalog,
        &EMPTY_TEST_SPECIES,
        &EMPTY_TEST_LEARNSETS,
        &item_catalog,
    );
    let mut state = GameState::default();
    state.coins = 99;
    state.bag.add_item(&coin_case, 1).expect("add coin case");
    stage_memory_game_board(&mut state, 5);
    let board_before = state
        .script_runtime
        .memory_game
        .as_ref()
        .unwrap()
        .cards
        .clone();
    let mut divider = ReplayDivider::new([]);

    let outcome = resolve_memory_game_pair(&mut state, context, &mut divider, 0, 1);
    assert!(matches!(
        outcome.effect,
        SpecialRoutineEffect::UnusedMemoryGame {
            matched: true,
            first_index: 0,
            second_index: 1,
            tries_remaining: 4,
            ..
        }
    ));
    let game = state.script_runtime.memory_game.as_ref().unwrap();
    assert_eq!(&game.cards[..2], &[u8::MAX, u8::MAX]);
    assert_eq!(&game.cards[2..], &board_before[2..]);
    assert_eq!(game.last_matches, [1, 0, 0, 0, 0]);
    assert_eq!(game.num_cards_matched, 2);
    assert_eq!(game.last_card_picked, 1);
    assert_eq!(divider.consumed(), 0);
}

#[test]
fn memory_game_reveal_acknowledgement_automatically_restarts_stub_loop() {
    let move_catalog = moves();
    let coin_case = item_data("COIN_CASE");
    let item_catalog = BTreeMap::from([("COIN_CASE".to_string(), coin_case.clone())]);
    let context = full_context(
        &move_catalog,
        &EMPTY_TEST_SPECIES,
        &EMPTY_TEST_LEARNSETS,
        &item_catalog,
    );
    let mut state = GameState::default();
    state.coins = 99;
    state.bag.add_item(&coin_case, 1).expect("add coin case");
    stage_memory_game_board(&mut state, 0);
    let mut divider = ReplayDivider::new(divider_trace_for_sub_values(0_u8..39));

    let ready = apply_memory_game_input(
        &mut state,
        context,
        &mut divider,
        MemoryGameInput::AdvanceFrame { button: None },
    );
    assert!(matches!(
        ready.effect,
        SpecialRoutineEffect::UnusedMemoryGameRevealReady { .. }
    ));
    let ignored = apply_memory_game_input(
        &mut state,
        context,
        &mut divider,
        MemoryGameInput::AdvanceFrame {
            button: Some(MemoryGameButton::B),
        },
    );
    assert!(matches!(
        ignored.effect,
        SpecialRoutineEffect::UnusedMemoryGameFrameAdvanced {
            phase: MemoryGamePhase::RevealAll,
            ..
        }
    ));
    let revealed = apply_memory_game_input(
        &mut state,
        context,
        &mut divider,
        MemoryGameInput::AdvanceFrame {
            button: Some(MemoryGameButton::A),
        },
    );
    assert!(matches!(
        revealed.effect,
        SpecialRoutineEffect::UnusedMemoryGameRoundEnded { .. }
    ));
    assert_eq!(
        state.script_runtime.memory_game.as_ref().unwrap().phase,
        MemoryGamePhase::RevealAllAcknowledgement
    );
    assert_eq!(
        state.script_runtime.memory_game.as_ref().unwrap().counter,
        45
    );

    let restarted = apply_memory_game_input(
        &mut state,
        context,
        &mut divider,
        MemoryGameInput::AdvanceFrame {
            button: Some(MemoryGameButton::A),
        },
    );
    assert!(matches!(
        restarted.effect,
        SpecialRoutineEffect::UnusedMemoryGameFrameAdvanced {
            phase: MemoryGamePhase::RestartGame,
            ..
        }
    ));
    assert_eq!(
        state
            .script_runtime
            .memory_game
            .as_ref()
            .unwrap()
            .card_choice,
        1
    );
    assert!(
        state
            .script_runtime
            .memory_game
            .as_ref()
            .unwrap()
            .cursor_active
    );
    assert_eq!(divider.consumed(), 0);
    apply_memory_game_input(
        &mut state,
        context,
        &mut divider,
        MemoryGameInput::AdvanceFrame { button: None },
    );
    assert_eq!(
        state.script_runtime.memory_game.as_ref().unwrap().phase,
        MemoryGamePhase::ResetBoard
    );
    let initialized = apply_memory_game_input(
        &mut state,
        context,
        &mut divider,
        MemoryGameInput::AdvanceFrame { button: None },
    );
    assert!(matches!(
        initialized.effect,
        SpecialRoutineEffect::UnusedMemoryGameStarted {
            tries_remaining: 0,
            ..
        }
    ));
    assert_eq!(
        state.script_runtime.memory_game.as_ref().unwrap().phase,
        MemoryGamePhase::InitBoardTilemapAndCursor
    );
    assert!(
        !state
            .script_runtime
            .memory_game
            .as_ref()
            .unwrap()
            .cursor_active
    );
    assert_eq!(
        state
            .script_runtime
            .memory_game
            .as_ref()
            .unwrap()
            .card_choice,
        0
    );
    assert_eq!(divider.consumed(), 78);
}

#[test]
fn memory_game_initializes_and_places_the_exact_asm_board() {
    let move_catalog = moves();
    let coin_case = item_data("COIN_CASE");
    let item_catalog = BTreeMap::from([("COIN_CASE".to_string(), coin_case.clone())]);
    let context = full_context(
        &move_catalog,
        &EMPTY_TEST_SPECIES,
        &EMPTY_TEST_LEARNSETS,
        &item_catalog,
    );
    for (menu_cursor_y, sampled_cards, expected_counts) in [
        (1, 39_usize, vec![8_usize, 2, 6, 6, 6, 8, 6, 3]),
        (2, 36, vec![8, 2, 6, 4, 9, 8, 6, 2]),
        (3, 33, vec![8, 2, 7, 2, 12, 8, 4, 2]),
    ] {
        let mut state = GameState::default();
        state.coins = 99;
        state.bag.add_item(&coin_case, 1).expect("add coin case");
        state
            .script_runtime
            .variables
            .insert("_coin_case_balance".to_string(), "stale".to_string());
        let mut divider = ReplayDivider::new(divider_trace_for_sub_values(
            0_u8..u8::try_from(sampled_cards).expect("sample count fits byte"),
        ));

        let entered = apply_memory_game_input(
            &mut state,
            context,
            &mut divider,
            MemoryGameInput::Enter { menu_cursor_y },
        );
        assert!(matches!(
            entered.effect,
            SpecialRoutineEffect::UnusedMemoryGameEntered { .. }
        ));
        assert!(
            !state
                .script_runtime
                .variables
                .contains_key("_coin_case_balance")
        );
        assert_eq!(divider.consumed(), 0);
        apply_memory_game_input(
            &mut state,
            context,
            &mut divider,
            MemoryGameInput::AdvanceFrame { button: None },
        );
        assert_eq!(divider.consumed(), 0);
        let initialized = apply_memory_game_input(
            &mut state,
            context,
            &mut divider,
            MemoryGameInput::AdvanceFrame { button: None },
        );
        assert!(matches!(
            initialized.effect,
            SpecialRoutineEffect::UnusedMemoryGameStarted {
                tries_remaining: 0,
                ..
            }
        ));

        let board = state
            .script_runtime
            .memory_game
            .as_ref()
            .unwrap()
            .cards
            .clone();
        let counts = (1_u8..=8)
            .map(|card| board.iter().filter(|value| **value == card).count())
            .collect::<Vec<_>>();
        assert_eq!(counts, expected_counts);
        assert_eq!(divider.consumed(), sampled_cards * 2);

        for expected_index in 0..45 {
            let placed = apply_memory_game_input(
                &mut state,
                context,
                &mut divider,
                MemoryGameInput::AdvanceFrame { button: None },
            );
            assert!(
                matches!(placed.effect, SpecialRoutineEffect::UnusedMemoryGameCardPlaced { card_index, delay_frames: 3, .. } if card_index == expected_index)
            );
        }
        let ready = apply_memory_game_input(
            &mut state,
            context,
            &mut divider,
            MemoryGameInput::AdvanceFrame { button: None },
        );
        assert!(matches!(
            ready.effect,
            SpecialRoutineEffect::UnusedMemoryGameCursorReady {
                cursor_index: 0,
                tries_remaining: 5,
                ..
            }
        ));
    }
}

#[test]
fn game_corner_games_require_coins_and_coin_case() {
    let move_catalog = moves();
    let coin_case = item_data("COIN_CASE");
    let item_catalog = BTreeMap::from([("COIN_CASE".to_string(), coin_case.clone())]);
    let context = full_context(
        &move_catalog,
        &EMPTY_TEST_SPECIES,
        &EMPTY_TEST_LEARNSETS,
        &item_catalog,
    );

    let mut no_coins = GameState::default();
    no_coins
        .bag
        .add_item(&coin_case, 1)
        .expect("coin case for no coins");
    no_coins.script_runtime.pending_slot_machine_input =
        Some(SlotMachineInput::Enter { lucky: false });
    let mut divider = ReplayDivider::new([]);
    let no_coins_outcome = apply_random_special_routine_with_context(
        &mut no_coins,
        context,
        "SlotMachine",
        &mut divider,
    )
    .expect("no coins is handled by script text");
    assert_eq!(
        no_coins_outcome.effect,
        SpecialRoutineEffect::GameCornerGameUnavailable {
            game: "SlotMachine".to_string(),
            reason: GameCornerUnavailableReason::NoCoins,
        }
    );
    assert_eq!(no_coins.script_runtime.active_menu, None);

    let mut card_flip_no_coins = GameState::default();
    card_flip_no_coins.script_runtime.pending_card_flip_input = Some(CardFlipInput::Start);
    let before = card_flip_no_coins.clone();
    let mut divider = ReplayDivider::new([]);
    let no_coins_card_flip = apply_random_special_routine_with_context(
        &mut card_flip_no_coins,
        context,
        "CardFlip",
        &mut divider,
    )
    .expect("the shared wrapper rejects zero coins before entering Card Flip");
    assert_eq!(
        no_coins_card_flip.effect,
        SpecialRoutineEffect::GameCornerGameUnavailable {
            game: "CardFlip".to_string(),
            reason: GameCornerUnavailableReason::NoCoins,
        }
    );
    assert_eq!(divider.consumed(), 0);
    assert_eq!(card_flip_no_coins.coins, before.coins);
    assert_eq!(card_flip_no_coins.bag, before.bag);
    assert!(card_flip_no_coins.script_runtime.card_flip.is_none());

    let mut no_coin_case = GameState::default();
    no_coin_case.coins = 10;
    no_coin_case.script_runtime.pending_card_flip_input = Some(CardFlipInput::Start);
    let mut divider = ReplayDivider::new([]);
    let missing_case = apply_random_special_routine_with_context(
        &mut no_coin_case,
        context,
        "CardFlip",
        &mut divider,
    )
    .expect("missing coin case is handled by script text");
    assert_eq!(
        missing_case.effect,
        SpecialRoutineEffect::GameCornerGameUnavailable {
            game: "CardFlip".to_string(),
            reason: GameCornerUnavailableReason::MissingCoinCase,
        }
    );
    assert_eq!(no_coin_case.script_runtime.active_menu, None);
}

#[test]
fn random_phone_wild_mon_uses_caller_map_grass_bucket() {
    let move_catalog = moves();
    let species_catalog = species_catalog(&[
        ("PIDGEY", 16),
        ("RATTATA", 19),
        ("SENTRET", 161),
        ("HOOTHOOT", 163),
    ]);
    let phone_contacts = PhoneContactCatalog(BTreeMap::from([(
        "PHONE_BIRDKEEPER_VANCE".to_string(),
        PhoneContactRecord {
            contact_id: "PHONE_BIRDKEEPER_VANCE".to_string(),
            trainer_class: Some("BIRD_KEEPER".to_string()),
            trainer_label: Some("VANCE1".to_string()),
            lines: vec!["Vance:".to_string()],
            primary_label: "VANCE".to_string(),
            map_constant: Some("ROUTE_44".to_string()),
            callee_time_mask: 0xff,
            callee_script: None,
            caller_time_mask: 0xff,
            caller_script: None,
        },
    )]));
    let wild_encounters = BTreeMap::from([(
        "ROUTE_44".to_string(),
        WildEncounterData {
            map_name: "ROUTE_44".to_string(),
            grass_rates: Some(BTreeMap::from([
                ("morning".to_string(), 30),
                ("day".to_string(), 30),
                ("night".to_string(), 30),
            ])),
            water_rate: None,
            swarm_overrides: BTreeMap::new(),
            zones: Vec::new(),
            grass: Some(WildEncounterTable {
                morning: vec![
                    WildEncounter {
                        level: 20,
                        species: "PIDGEY".to_string(),
                    };
                    4
                ],
                day: vec![
                    WildEncounter {
                        level: 20,
                        species: "PIDGEY".to_string(),
                    },
                    WildEncounter {
                        level: 21,
                        species: "RATTATA".to_string(),
                    },
                    WildEncounter {
                        level: 22,
                        species: "SENTRET".to_string(),
                    },
                    WildEncounter {
                        level: 23,
                        species: "HOOTHOOT".to_string(),
                    },
                ],
                night: vec![
                    WildEncounter {
                        level: 20,
                        species: "HOOTHOOT".to_string(),
                    };
                    4
                ],
            }),
            water: None,
        },
    )]);
    let mut context = full_context(
        &move_catalog,
        &species_catalog,
        &EMPTY_TEST_LEARNSETS,
        &EMPTY_TEST_ITEMS,
    );
    context.phone_contacts = &phone_contacts;
    context.wild_encounters = &wild_encounters;

    let mut state = GameState::default();
    state.time.time_of_day = TimeOfDay::Day;
    state.script_runtime.variables.insert(
        "VAR_CALLERID".to_string(),
        "PHONE_BIRDKEEPER_VANCE".to_string(),
    );
    let mut divider = ReplayDivider::new([0, 255]);
    let outcome = apply_random_special_routine_with_context(
        &mut state,
        context,
        "RandomPhoneWildMon",
        &mut divider,
    )
    .expect("random phone wild mon");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::RandomPhoneWildMon {
            contact_id: "PHONE_BIRDKEEPER_VANCE".to_string(),
            map_name: "ROUTE_44".to_string(),
            time_of_day: TimeOfDay::Day,
            species: "RATTATA".to_string(),
            random_state_after: CrystalRandomState { add: 0, sub: 1 },
        }
    );
    assert_eq!(
        state
            .script_runtime
            .named_buffers
            .get("STRING_BUFFER_4")
            .map(String::as_str),
        Some("RATTATA")
    );
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("wNamedObjectIndex")
            .map(String::as_str),
        Some("19")
    );
    assert_eq!(divider.consumed(), 2);
}

#[test]
fn random_unseen_wild_mon_preserves_morning_slot_selection() {
    let move_catalog = moves();
    let species_catalog = species_catalog(&[
        ("PIDGEY", 16),
        ("RATTATA", 19),
        ("SENTRET", 161),
        ("HOOTHOOT", 163),
        ("LARVITAR", 246),
        ("PHANPY", 231),
        ("SKARMORY", 227),
    ]);
    let phone_contacts = PhoneContactCatalog(BTreeMap::from([(
        "PHONE_HIKER_PARRY".to_string(),
        PhoneContactRecord {
            contact_id: "PHONE_HIKER_PARRY".to_string(),
            trainer_class: Some("HIKER".to_string()),
            trainer_label: Some("PARRY1".to_string()),
            lines: vec!["Parry:".to_string()],
            primary_label: "PARRY".to_string(),
            map_constant: Some("ROUTE_45".to_string()),
            callee_time_mask: 0xff,
            callee_script: None,
            caller_time_mask: 0xff,
            caller_script: None,
        },
    )]));
    let wild_encounters = BTreeMap::from([(
        "ROUTE_45".to_string(),
        WildEncounterData {
            map_name: "ROUTE_45".to_string(),
            grass_rates: Some(BTreeMap::from([
                ("morning".to_string(), 30),
                ("day".to_string(), 30),
                ("night".to_string(), 30),
            ])),
            water_rate: None,
            swarm_overrides: BTreeMap::new(),
            zones: Vec::new(),
            grass: Some(WildEncounterTable {
                morning: vec![
                    WildEncounter {
                        level: 20,
                        species: "PIDGEY".to_string(),
                    },
                    WildEncounter {
                        level: 20,
                        species: "RATTATA".to_string(),
                    },
                    WildEncounter {
                        level: 20,
                        species: "SENTRET".to_string(),
                    },
                    WildEncounter {
                        level: 20,
                        species: "HOOTHOOT".to_string(),
                    },
                    WildEncounter {
                        level: 20,
                        species: "LARVITAR".to_string(),
                    },
                    WildEncounter {
                        level: 20,
                        species: "PHANPY".to_string(),
                    },
                    WildEncounter {
                        level: 20,
                        species: "SKARMORY".to_string(),
                    },
                ],
                day: vec![
                    WildEncounter {
                        level: 20,
                        species: "PIDGEY".to_string(),
                    };
                    7
                ],
                night: vec![
                    WildEncounter {
                        level: 20,
                        species: "HOOTHOOT".to_string(),
                    };
                    7
                ],
            }),
            water: None,
        },
    )]);
    let mut context = full_context(
        &move_catalog,
        &species_catalog,
        &EMPTY_TEST_LEARNSETS,
        &EMPTY_TEST_ITEMS,
    );
    context.phone_contacts = &phone_contacts;
    context.wild_encounters = &wild_encounters;

    let mut state = GameState::default();
    state.time.time_of_day = TimeOfDay::Night;
    state.random_state = CrystalRandomState { add: 0xff, sub: 0 };
    state
        .script_runtime
        .variables
        .insert("VAR_CALLERID".to_string(), "PHONE_HIKER_PARRY".to_string());
    let mut divider = ReplayDivider::new([1, 255, 0, 254]);
    let outcome = apply_random_special_routine_with_context(
        &mut state,
        context,
        "RandomUnseenWildMon",
        &mut divider,
    )
    .expect("random unseen wild mon");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::RandomUnseenWildMon {
            contact_id: "PHONE_HIKER_PARRY".to_string(),
            map_name: "ROUTE_45".to_string(),
            species: Some("PHANPY".to_string()),
            already_seen: false,
            script_value: 0,
            random_state_after: CrystalRandomState { add: 0, sub: 2 },
        }
    );
    assert_eq!(divider.consumed(), 4);
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));
    assert_eq!(
        state
            .script_runtime
            .named_buffers
            .get("STRING_BUFFER_1")
            .map(String::as_str),
        Some("PHANPY")
    );
}

#[test]
fn random_phone_wild_mon_requires_exact_caller_context() {
    let move_catalog = moves();
    let species_catalog = species_catalog(&[("PIDGEY", 16)]);
    let state = &mut GameState::default();
    let mut divider = ReplayDivider::new([]);
    let error = apply_random_special_routine_with_context(
        state,
        full_context(
            &move_catalog,
            &species_catalog,
            &EMPTY_TEST_LEARNSETS,
            &EMPTY_TEST_ITEMS,
        ),
        "RandomPhoneWildMon",
        &mut divider,
    )
    .expect_err("caller id is required");

    assert_eq!(
        error,
        RandomSpecialRoutineError::Routine(SpecialRoutineError::MissingCallerId {
            routine: "RandomPhoneWildMon".to_string()
        })
    );
    assert!(state.script_runtime.named_buffers.is_empty());
}

#[test]
fn random_phone_mon_uses_exact_caller_trainer_party() {
    let move_catalog = moves();
    let species_catalog = species_catalog(&[("PIDGEY", 16), ("FEAROW", 22), ("PIDGEOT", 18)]);
    let phone_contacts = PhoneContactCatalog(BTreeMap::from([(
        "PHONE_BIRDKEEPER_VANCE".to_string(),
        PhoneContactRecord {
            contact_id: "PHONE_BIRDKEEPER_VANCE".to_string(),
            trainer_class: Some("BIRD_KEEPER".to_string()),
            trainer_label: Some("VANCE1".to_string()),
            lines: vec!["Vance:".to_string()],
            primary_label: "VANCE".to_string(),
            map_constant: Some("ROUTE_44".to_string()),
            callee_time_mask: 0xff,
            callee_script: None,
            caller_time_mask: 0xff,
            caller_script: None,
        },
    )]));
    let mut trainer_catalog = TrainerCatalog::default();
    trainer_catalog
        .insert(Trainer {
            name: "VANCE".to_string(),
            trainer_id: "VANCE1".to_string(),
            trainer_class: "BIRD_KEEPER".to_string(),
            party: vec![
                TrainerPartyPokemon {
                    species: "PIDGEY".to_string(),
                    level: 25,
                    ..TrainerPartyPokemon::default()
                },
                TrainerPartyPokemon {
                    species: "FEAROW".to_string(),
                    level: 27,
                    ..TrainerPartyPokemon::default()
                },
                TrainerPartyPokemon {
                    species: "PIDGEOT".to_string(),
                    level: 29,
                    ..TrainerPartyPokemon::default()
                },
            ],
            win_quote: "Won".to_string(),
            lose_quote: "Lost".to_string(),
            items: Vec::new(),
            base_reward: 1,
            ai_move_flags: 0,
            ai_item_switch_flags: 0,
            encounter_music: "TRAINER_MUSIC".to_string(),
            ai_layers: Vec::new(),
        })
        .expect("trainer catalog");
    let mut context = full_context(
        &move_catalog,
        &species_catalog,
        &EMPTY_TEST_LEARNSETS,
        &EMPTY_TEST_ITEMS,
    );
    context.phone_contacts = &phone_contacts;
    context.trainer_catalog = &trainer_catalog;

    let mut state = GameState::default();
    state.script_runtime.variables.insert(
        "VAR_CALLERID".to_string(),
        "PHONE_BIRDKEEPER_VANCE".to_string(),
    );
    let mut divider = ReplayDivider::new([0, 249, 0, 6]);
    let outcome = apply_random_special_routine_with_context(
        &mut state,
        context,
        "RandomPhoneMon",
        &mut divider,
    )
    .expect("random phone trainer mon");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::RandomPhoneMon {
            contact_id: "PHONE_BIRDKEEPER_VANCE".to_string(),
            trainer_id: "VANCE1".to_string(),
            species: "FEAROW".to_string(),
            party_index: 1,
            random_state_after: CrystalRandomState { add: 0, sub: 1 },
        }
    );
    assert_eq!(
        state
            .script_runtime
            .named_buffers
            .get("STRING_BUFFER_4")
            .map(String::as_str),
        Some("FEAROW")
    );
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("wNamedObjectIndex")
            .map(String::as_str),
        Some("22")
    );
    assert_eq!(divider.consumed(), 4);
}

#[test]
fn asm_ret_only_specials_return_noop_without_runtime_mutation() {
    for routine in [
        "UnusedDummySpecial",
        "UnusedBattleTowerDummySpecial1",
        "UnusedBattleTowerDummySpecial2",
    ] {
        let mut state = GameState::default();
        let before = state.clone();

        let outcome = apply_special_routine(&mut state, &moves(), routine).expect("noop special");

        assert_eq!(outcome.routine, routine);
        assert_eq!(outcome.effect, SpecialRoutineEffect::Noop);
        assert_eq!(state, before);
    }
}

#[test]
fn unused_find_item_in_pc_or_bag_checks_pc_before_bag() {
    let move_catalog = moves();
    let potion = item_data("POTION");
    let item_catalog = BTreeMap::from([("POTION".to_string(), potion.clone())]);
    let context = full_context(
        &move_catalog,
        &EMPTY_TEST_SPECIES,
        &EMPTY_TEST_LEARNSETS,
        &item_catalog,
    );
    let mut state = GameState::default();
    state.script_runtime.script_value = Some("POTION".to_string());
    state.bag.add_item(&potion, 1).expect("add bag potion");
    state.bag.add_pc_item(&potion, 2).expect("add pc potion");

    let pc_first =
        apply_special_routine_with_context(&mut state, context, "UnusedFindItemInPCOrBag")
            .expect("find item");

    assert_eq!(
        pc_first.effect,
        SpecialRoutineEffect::UnusedFindItemInPcOrBag {
            item_id: "POTION".to_string(),
            found_in_pc: true,
            found_in_bag: false,
            script_value: 1,
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));

    state.bag.pc_items.clear();
    state.script_runtime.script_value = Some("POTION".to_string());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "POTION".to_string());
    let bag_fallback =
        apply_special_routine_with_context(&mut state, context, "UnusedFindItemInPCOrBag")
            .expect("find bag item");

    assert_eq!(
        bag_fallback.effect,
        SpecialRoutineEffect::UnusedFindItemInPcOrBag {
            item_id: "POTION".to_string(),
            found_in_pc: false,
            found_in_bag: true,
            script_value: 1,
        }
    );

    state.bag.items.clear();
    state.script_runtime.script_value = Some("POTION".to_string());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "POTION".to_string());
    let missing =
        apply_special_routine_with_context(&mut state, context, "UnusedFindItemInPCOrBag")
            .expect("missing item handled");

    assert_eq!(
        missing.effect,
        SpecialRoutineEffect::UnusedFindItemInPcOrBag {
            item_id: "POTION".to_string(),
            found_in_pc: false,
            found_in_bag: false,
            script_value: 0,
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));
}

#[test]
fn function11ba38_checks_for_another_usable_party_mon() {
    let mut state = GameState::default();
    let selected = pokemon("CHIKORITA");
    let mut other = pokemon("CYNDAQUIL");
    other.hp = 12;
    state
        .storage
        .register_capture_in_box(0, selected)
        .expect("store selected");
    state
        .storage
        .register_capture_in_box(0, other)
        .expect("store other");
    state.sync_party_from_storage();
    state
        .script_runtime
        .variables
        .insert("_party_slot".to_string(), "0".to_string());
    let before_alias = state.clone();
    let alias_error = apply_special_routine(&mut state, &moves(), "Function11ba38")
        .expect_err("selected party helper must not accept party slot alias");
    assert!(matches!(
        alias_error,
        SpecialRoutineError::MissingScriptValue { routine, variable }
            if routine == "Function11ba38" && variable == "_selected_party_index"
    ));
    assert_eq!(state, before_alias);
    state.script_runtime.variables.remove("_party_slot");
    state
        .script_runtime
        .variables
        .insert("_selected_party_index".to_string(), "0".to_string());

    let usable =
        apply_special_routine(&mut state, &moves(), "Function11ba38").expect("another usable mon");

    assert_eq!(
        usable.effect,
        SpecialRoutineEffect::Function11ba38 {
            selected_party_slot: 0,
            other_usable_party_mon: true,
            script_value: 0,
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));

    state.storage.party.pokemon[1]
        .as_mut()
        .expect("other party mon")
        .hp = 0;
    state.sync_party_from_storage();
    let last_usable =
        apply_special_routine(&mut state, &moves(), "Function11ba38").expect("last usable mon");

    assert_eq!(
        last_usable.effect,
        SpecialRoutineEffect::Function11ba38 {
            selected_party_slot: 0,
            other_usable_party_mon: false,
            script_value: 1,
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));
}

#[test]
fn inactive_declared_specials_reject_without_runtime_mutation() {
    let cases = [
        "Function11ac3e",
        "TradeCornerHoldMon",
        "Function11b5e8",
        "Function11b7e5",
        "Function11b879",
        "Function11b920",
        "Function11b93b",
        "Function11c1ab",
        "Function17d2b6",
        "Function17d2ce",
        "Function102142",
    ];

    for routine in cases {
        let mut state = GameState::default();
        let before = state.clone();

        let error = apply_special_routine(&mut state, &moves(), routine)
            .expect_err("inactive declared routine must reject");

        assert!(matches!(
            error,
            SpecialRoutineError::InactiveDeclaredRoutine { routine: rejected }
                if rejected == routine
        ));
        assert_eq!(state, before);
    }
}

#[test]
fn every_modpack_declared_special_has_an_exact_rust_branch() {
    let declared: BTreeMap<String, serde_json::Value> =
        serde_json::from_str(MODPACK_SPECIAL_ROUTINES_JSON).expect("special routines json");
    let mut missing = Vec::new();

    for routine in declared.keys() {
        let mut state = GameState::default();
        let result = apply_special_routine(&mut state, &moves(), routine);
        if matches!(
            result,
            Err(SpecialRoutineError::UnsupportedRoutine { routine: unsupported })
                if unsupported == *routine
        ) {
            missing.push(routine.clone());
        }
    }

    assert!(
        missing.is_empty(),
        "modpack special routines missing exact Rust branches: {missing:?}"
    );
}

#[test]
fn shuckie_routines_create_and_return_exact_mania_shuckle() {
    let mut state = GameState::default();
    let mut move_catalog = moves();
    move_catalog.insert("CONSTRICT".to_string(), move_data("CONSTRICT", 35));
    let species = species_catalog(&[("SHUCKLE", 213)]);
    let learnsets = [(
        "SHUCKLE".to_string(),
        vec![crate::systems::learnsets::LearnsetEntry(
            1,
            "CONSTRICT".to_string(),
        )],
    )]
    .into_iter()
    .collect();
    let items = BTreeMap::from([("BERRY".to_string(), item_data("BERRY"))]);
    let shuckie_gift = ShuckieGiftDefinition {
        species: "SHUCKLE".to_string(),
        level: 15,
        held_item: "BERRY".to_string(),
        nickname: "SHUCKIE".to_string(),
        original_trainer_name: "MANIA".to_string(),
        original_trainer_id: 518,
        got_today_engine_flag: "ENGINE_GOT_SHUCKIE_TODAY".to_string(),
    };
    state.random_state = CrystalRandomState { add: 0xff, sub: 0 };
    let mut divider = ReplayDivider::new([0, 1, 0, 0]);

    let outcome = apply_random_special_routine_with_context(
        &mut state,
        full_context_with_shuckie_gift(&move_catalog, &species, &learnsets, &items, &shuckie_gift),
        "GiveShuckle",
        &mut divider,
    )
    .expect("give shuckle");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::GiveShuckle {
            stored: true,
            random_state_after: CrystalRandomState { add: 0, sub: 0xfe }
        }
    );
    assert_eq!(divider.consumed(), 4);
    let shuckie = state.storage.party.pokemon[0].as_ref().expect("shuckie");
    assert_eq!(shuckie.species.id, "SHUCKLE");
    assert_eq!(shuckie.item.as_deref(), Some("BERRY"));
    assert_eq!(shuckie.nickname, "SHUCKIE");
    assert_eq!(shuckie.original_trainer_name, "MANIA");
    assert_eq!(shuckie.original_trainer_id, 518);
    assert_eq!(shuckie.dvs, Dv::from_non_hp(15, 15, 15, 14));
    assert_eq!(
        state.flags.is_engine_flag_set("ENGINE_GOT_SHUCKIE_TODAY"),
        Ok(true)
    );

    state
        .script_runtime
        .variables
        .insert("_selected_party_index".to_string(), "0".to_string());
    state
        .script_runtime
        .variables
        .insert("_selection_cancelled".to_string(), "0".to_string());
    let returned = apply_special_routine_with_context(
        &mut state,
        full_context_with_shuckie_gift(&move_catalog, &species, &learnsets, &items, &shuckie_gift),
        "ReturnShuckie",
    )
    .expect("return");

    assert_eq!(
        returned.effect,
        SpecialRoutineEffect::ReturnShuckie {
            party_slot: Some(0),
            result: 2
        }
    );
    assert_eq!(state.storage.party.filled_slots(), 0);
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("2"));
}

#[test]
fn shuckie_routines_require_explicit_modpack_gift_data_without_builtin_fallback() {
    let mut state = GameState::default();
    let before = state.clone();

    let mut divider = ReplayDivider::new([]);
    let error = apply_random_special_routine_with_context(
        &mut state,
        full_context(
            &moves(),
            &EMPTY_TEST_SPECIES,
            &EMPTY_TEST_LEARNSETS,
            &EMPTY_TEST_ITEMS,
        ),
        "GiveShuckle",
        &mut divider,
    )
    .expect_err("missing Shuckie gift rejected");

    assert_eq!(
        error,
        RandomSpecialRoutineError::Routine(SpecialRoutineError::MissingShuckieGift {
            routine: "GiveShuckle".to_string()
        })
    );
    assert_eq!(state, before);
}

#[test]
fn give_dratini_replaces_last_dratini_moves_from_exact_move_catalog() {
    let mut state = GameState::default();
    state
        .storage
        .register_capture_in_box(0, pokemon("DRATINI"))
        .expect("store");
    state
        .storage
        .register_capture_in_box(0, pokemon("DRATINI"))
        .expect("store");
    state.sync_party_from_storage();
    let mut move_catalog = moves();
    for (name, pp) in [
        ("WRAP", 20),
        ("THUNDER_WAVE", 20),
        ("TWISTER", 20),
        ("EXTREMESPEED", 5),
        ("LEER", 30),
    ] {
        move_catalog.insert(name.to_string(), move_data(name, pp));
    }
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "0".to_string());
    let species = BTreeMap::new();
    let learnsets = SpeciesLearnsets::new();
    let items = BTreeMap::new();
    let dratini_move_sets = BTreeMap::from([
        (
            0,
            vec![
                "WRAP".to_string(),
                "THUNDER_WAVE".to_string(),
                "TWISTER".to_string(),
                "EXTREMESPEED".to_string(),
            ],
        ),
        (
            1,
            vec![
                "WRAP".to_string(),
                "LEER".to_string(),
                "THUNDER_WAVE".to_string(),
                "TWISTER".to_string(),
            ],
        ),
    ]);

    let outcome = apply_special_routine_with_context(
        &mut state,
        full_context_with_dratini_move_sets(
            &move_catalog,
            &species,
            &learnsets,
            &items,
            &dratini_move_sets,
        ),
        "GiveDratini",
    )
    .expect("dratini");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::GiveDratini {
            party_slot: Some(1),
            mode: 0,
            move_names: vec![
                "WRAP".to_string(),
                "THUNDER_WAVE".to_string(),
                "TWISTER".to_string(),
                "EXTREMESPEED".to_string()
            ],
            learned: true
        }
    );
    assert_eq!(
        state.storage.party.pokemon[1]
            .as_ref()
            .expect("dratini")
            .moves
            .iter()
            .map(|known| known.name.as_str())
            .collect::<Vec<_>>(),
        vec!["WRAP", "THUNDER_WAVE", "TWISTER", "EXTREMESPEED"]
    );
}

#[test]
fn give_dratini_requires_explicit_modpack_move_sets_without_builtin_fallback() {
    let mut state = GameState::default();
    state
        .storage
        .register_capture_in_box(0, pokemon("DRATINI"))
        .expect("store");
    state.sync_party_from_storage();
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "0".to_string());
    let before = state.clone();

    let error = apply_special_routine(&mut state, &moves(), "GiveDratini")
        .expect_err("missing Dratini move sets rejected");

    assert_eq!(
        error,
        SpecialRoutineError::MissingDratiniMoveSets {
            routine: "GiveDratini".to_string()
        }
    );
    assert_eq!(state, before);
}

#[test]
fn kurt_selection_removes_exact_apricorn_and_records_script_values() {
    let mut state = GameState::default();
    let item = item_data("RED_APRICORN");
    let items = BTreeMap::from([("RED_APRICORN".to_string(), item.clone())]);
    state.bag.add_item(&item, 3).expect("add apricorn");
    state.script_runtime.variables.insert(
        "_kurt_apricorn_type".to_string(),
        "RED_APRICORN".to_string(),
    );
    state
        .script_runtime
        .variables
        .insert("_kurt_apricorn_quantity".to_string(), "2".to_string());
    let moves = moves();
    let species = BTreeMap::new();
    let learnsets = SpeciesLearnsets::new();
    let recipes = BTreeMap::from([("RED_APRICORN".to_string(), "LEVEL_BALL".to_string())]);

    let outcome = apply_special_routine_with_context(
        &mut state,
        full_context_with_kurt_apricorn_recipes(&moves, &species, &learnsets, &items, &recipes),
        "SelectApricornForKurt",
    )
    .expect("kurt apricorn");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::SelectApricornForKurt {
            apricorn: Some("RED_APRICORN".to_string()),
            quantity: 2
        }
    );
    assert_eq!(state.bag.quantity(&item), 1);
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("VAR_KURT_APRICORNS")
            .map(String::as_str),
        Some("2")
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));

    let mut unselected = GameState::default();
    unselected.bag.add_item(&item, 3).expect("add apricorn");
    unselected
        .script_runtime
        .variables
        .insert("_kurt_apricorn_quantity".to_string(), "2".to_string());
    let no_selection = apply_special_routine_with_context(
        &mut unselected,
        full_context_with_kurt_apricorn_recipes(&moves, &species, &learnsets, &items, &recipes),
        "SelectApricornForKurt",
    )
    .expect("missing apricorn selection is a cancelled selection");
    assert_eq!(
        no_selection.effect,
        SpecialRoutineEffect::SelectApricornForKurt {
            apricorn: None,
            quantity: 0
        }
    );
    assert_eq!(unselected.bag.quantity(&item), 3);
}

#[test]
fn kurt_selection_requires_explicit_modpack_recipe_data_without_builtin_apricorn_fallback() {
    let mut state = GameState::default();
    let item = item_data("RED_APRICORN");
    let items = BTreeMap::from([("RED_APRICORN".to_string(), item.clone())]);
    state.bag.add_item(&item, 3).expect("add apricorn");
    state.script_runtime.variables.insert(
        "_kurt_apricorn_type".to_string(),
        "RED_APRICORN".to_string(),
    );
    state
        .script_runtime
        .variables
        .insert("_kurt_apricorn_quantity".to_string(), "2".to_string());
    let before = state.clone();
    let moves = moves();
    let species = BTreeMap::new();
    let learnsets = SpeciesLearnsets::new();

    let error = apply_special_routine_with_context(
        &mut state,
        full_context(&moves, &species, &learnsets, &items),
        "SelectApricornForKurt",
    )
    .expect_err("missing Kurt apricorn recipes rejected");

    assert_eq!(
        error,
        SpecialRoutineError::MissingKurtApricornRecipes {
            routine: "SelectApricornForKurt".to_string()
        }
    );
    assert_eq!(state, before);
}

#[test]
fn kurt_selection_requires_exact_quantity_without_one_apricorn_fallback() {
    let mut state = GameState::default();
    let item = item_data("RED_APRICORN");
    let items = BTreeMap::from([("RED_APRICORN".to_string(), item.clone())]);
    state.bag.add_item(&item, 3).expect("add apricorn");
    state.script_runtime.variables.insert(
        "_kurt_apricorn_type".to_string(),
        "RED_APRICORN".to_string(),
    );
    let before = state.clone();
    let moves = moves();
    let species = BTreeMap::new();
    let learnsets = SpeciesLearnsets::new();
    let recipes = BTreeMap::from([("RED_APRICORN".to_string(), "LEVEL_BALL".to_string())]);

    let error = apply_special_routine_with_context(
        &mut state,
        full_context_with_kurt_apricorn_recipes(&moves, &species, &learnsets, &items, &recipes),
        "SelectApricornForKurt",
    )
    .expect_err("missing kurt quantity rejected");

    assert!(matches!(
        error,
        SpecialRoutineError::MissingScriptValue { routine, variable }
            if routine == "SelectApricornForKurt"
                && variable == "_kurt_apricorn_quantity"
    ));
    assert_eq!(state, before);
}

#[test]
fn bills_grandfather_and_init_roam_mons_write_saveable_state() {
    let mut state = GameState::default();
    state
        .storage
        .register_capture_in_box(0, pokemon("PIKACHU"))
        .expect("store");
    state.sync_party_from_storage();
    state
        .script_runtime
        .variables
        .insert("_selected_party_index".to_string(), "0".to_string());

    let bill = apply_special_routine(&mut state, &moves(), "BillsGrandfather").expect("bill");

    assert_eq!(
        bill.effect,
        SpecialRoutineEffect::BillsGrandfather {
            party_slot: Some(0),
            species: Some("PIKACHU".to_string())
        }
    );
    assert_eq!(
        state
            .script_runtime
            .named_buffers
            .get("STRING_BUFFER_1")
            .map(String::as_str),
        Some("PIKACHU")
    );

    let move_catalog = moves();
    let species = species_catalog(&[("RAIKOU", 243), ("ENTEI", 244)]);
    let learnsets = SpeciesLearnsets::new();
    let items = BTreeMap::new();
    let roaming_catalog = test_roaming_catalog();
    state.roaming_pokemon[0].dvs_be = [0x12, 0x34];
    state.roaming_pokemon[1].dvs_be = [0x56, 0x78];
    state.roaming_pokemon[2] = RoamingPokemonState {
        species: Some("SUICUNE".to_string()),
        level: 40,
        map_group: 7,
        map_number: 1,
        hp: 99,
        dvs_be: [0x9a, 0xbc],
    };
    let roamers = apply_special_routine_with_context(
        &mut state,
        full_context_with_roamers(
            &move_catalog,
            &species,
            &learnsets,
            &items,
            &roaming_catalog,
        ),
        "InitRoamMons",
    )
    .expect("init roamers");

    assert_eq!(
        state.roaming_pokemon,
        [
            RoamingPokemonState {
                species: Some("RAIKOU".to_string()),
                level: 40,
                map_group: 2,
                map_number: 5,
                hp: 0,
                dvs_be: [0x12, 0x34],
            },
            RoamingPokemonState {
                species: Some("ENTEI".to_string()),
                level: 40,
                map_group: 10,
                map_number: 4,
                hp: 0,
                dvs_be: [0x56, 0x78],
            },
            RoamingPokemonState {
                species: Some("SUICUNE".to_string()),
                level: 40,
                map_group: 7,
                map_number: 1,
                hp: 99,
                dvs_be: [0x9a, 0xbc],
            },
        ]
    );
    assert_eq!(
        roamers.effect,
        SpecialRoutineEffect::InitRoamMons {
            roamers: state.roaming_pokemon.clone()
        }
    );
}

#[test]
fn init_roam_mons_requires_explicit_modpack_roamer_data_without_beast_fallback() {
    let mut state = GameState::default();
    let move_catalog = moves();
    let species = species_catalog(&[("RAIKOU", 243), ("ENTEI", 244)]);
    let learnsets = SpeciesLearnsets::new();
    let items = BTreeMap::new();
    let before = state.clone();

    let error = apply_special_routine_with_context(
        &mut state,
        full_context(&move_catalog, &species, &learnsets, &items),
        "InitRoamMons",
    )
    .expect_err("missing roamer data rejects");

    assert_eq!(
        error,
        SpecialRoutineError::MissingRoamingPokemonDefinitions {
            routine: "InitRoamMons".to_string()
        }
    );
    assert_eq!(state, before);
}

#[test]
fn bills_grandfather_requires_exact_selection_without_slot_zero_fallback() {
    let mut state = GameState::default();
    state
        .storage
        .register_capture_in_box(0, pokemon("PIKACHU"))
        .expect("store");
    state.sync_party_from_storage();
    let before = state.clone();

    let error = apply_special_routine(&mut state, &moves(), "BillsGrandfather")
        .expect_err("missing bill selection rejected");

    assert!(matches!(
        error,
        SpecialRoutineError::MissingScriptValue { routine, variable }
            if routine == "BillsGrandfather" && variable == "_selected_party_index"
    ));
    assert_eq!(state, before);
}

#[test]
fn mystery_gift_specials_use_exact_save_item_and_clear_pending_reward() {
    let mut state = GameState::default();
    let item = item_data("NUGGET");
    let items = BTreeMap::from([("NUGGET".to_string(), item.clone())]);
    let moves = moves();
    let species = BTreeMap::new();
    let learnsets = SpeciesLearnsets::new();

    let unlock = apply_special_routine(&mut state, &moves, "UnlockMysteryGift")
        .expect("unlock mystery gift");

    assert_eq!(
        unlock.effect,
        SpecialRoutineEffect::UnlockMysteryGift {
            newly_unlocked: true
        }
    );
    assert!(state.mystery_gift_unlocked);

    state.mystery_gift.stored_item = Some("NUGGET".to_string());
    state.mystery_gift.backup_item = Some("NUGGET".to_string());
    let check =
        apply_special_routine(&mut state, &moves, "CheckMysteryGift").expect("check mystery gift");
    assert_eq!(
        check.effect,
        SpecialRoutineEffect::CheckMysteryGift {
            has_pending_item: true
        }
    );

    let received = apply_special_routine_with_context(
        &mut state,
        full_context(&moves, &species, &learnsets, &items),
        "GetMysteryGiftItem",
    )
    .expect("receive mystery gift");

    assert_eq!(
        received.effect,
        SpecialRoutineEffect::GetMysteryGiftItem {
            item_id: Some("NUGGET".to_string()),
            received: true
        }
    );
    assert_eq!(state.bag.quantity(&item), 1);
    assert_eq!(state.mystery_gift.stored_item, None);
    assert_eq!(state.mystery_gift.backup_item, None);
    assert_eq!(
        state.script_runtime.audio_events[0].audio_id.as_deref(),
        Some("SFX_ITEM")
    );
}

#[test]
fn buena_password_and_prize_are_exact_saveable_accounting() {
    assert_eq!(
        BUENA_PASSWORD_CATEGORY_TYPES,
        &[
            BUENA_PASSWORD_CATEGORY_MON,
            BUENA_PASSWORD_CATEGORY_ITEM,
            BUENA_PASSWORD_CATEGORY_MOVE,
            BUENA_PASSWORD_CATEGORY_STRING
        ]
    );
    assert!(is_known_buena_password_category_type(
        BUENA_PASSWORD_CATEGORY_MON
    ));
    assert!(!is_known_buena_password_category_type("buena_mon"));

    let mut state = GameState::default();
    state.time.current_day = 3;
    let moves = moves();
    let species = BTreeMap::new();
    let learnsets = SpeciesLearnsets::new();
    let items = BTreeMap::new();
    let mut buena_password_categories = BuenaPasswordCategories {
        order: vec![
            "JohtoStarters".to_string(),
            "Beverages".to_string(),
            "HealingItems".to_string(),
        ],
        categories: BTreeMap::from([
            (
                "JohtoStarters".to_string(),
                BuenaPasswordCategoryDefinition {
                    category_type: "BUENA_MON".to_string(),
                    points: 10,
                    options: vec![
                        "CYNDAQUIL".to_string(),
                        "TOTODILE".to_string(),
                        "CHIKORITA".to_string(),
                    ],
                },
            ),
            (
                "Beverages".to_string(),
                BuenaPasswordCategoryDefinition {
                    category_type: "BUENA_ITEM".to_string(),
                    points: 12,
                    options: vec![
                        "FRESH_WATER".to_string(),
                        "SODA_POP".to_string(),
                        "LEMONADE".to_string(),
                    ],
                },
            ),
            (
                "HealingItems".to_string(),
                BuenaPasswordCategoryDefinition {
                    category_type: "BUENA_ITEM".to_string(),
                    points: 12,
                    options: vec![
                        "POTION".to_string(),
                        "ANTIDOTE".to_string(),
                        "PARLYZ_HEAL".to_string(),
                    ],
                },
            ),
        ]),
    };
    for index in 3..11 {
        let category_id = format!("Category{index}");
        buena_password_categories.order.push(category_id.clone());
        buena_password_categories.categories.insert(
            category_id,
            BuenaPasswordCategoryDefinition {
                category_type: "BUENA_STRING".to_string(),
                points: 1,
                options: vec![
                    format!("A{index}"),
                    format!("B{index}"),
                    format!("C{index}"),
                ],
            },
        );
    }
    // Reject masked category 15, accept category 0, reject masked option
    // 3, then accept option 0.
    let mut divider = ReplayDivider::new(divider_trace_for_sub_values([15, 0, 3, 0]));

    let first = apply_random_special_routine_with_context(
        &mut state,
        full_context_with_buena_password_categories(
            &moves,
            &species,
            &learnsets,
            &items,
            &buena_password_categories,
        ),
        "BuenasPassword",
        &mut divider,
    )
    .expect("generate password");

    assert_eq!(
        first.effect,
        SpecialRoutineEffect::BuenasPassword {
            category: "JohtoStarters".to_string(),
            category_type: "BUENA_MON".to_string(),
            options: vec![
                "CYNDAQUIL".to_string(),
                "TOTODILE".to_string(),
                "CHIKORITA".to_string(),
            ],
            correct: "CYNDAQUIL".to_string(),
            guess: None,
            matched: false,
            random_state_after: CrystalRandomState::default()
        }
    );
    assert_eq!(divider.consumed(), 8);
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));
    for host_mirror in ["_buena_category", "_buena_category_type", "_buena_password"] {
        assert!(
            !state.script_runtime.variables.contains_key(host_mirror),
            "BuenasPassword must not persist the immediate {host_mirror} result"
        );
    }

    let mut padded_guess = state.clone();
    padded_guess
        .script_runtime
        .variables
        .insert("BUENA_PASSWORD".to_string(), " TOTODILE".to_string());
    let before_padded_guess = padded_guess.clone();
    let mut padded_divider = ReplayDivider::new([]);
    let padded_guess_error = apply_random_special_routine_with_context(
        &mut padded_guess,
        full_context_with_buena_password_categories(
            &moves,
            &species,
            &learnsets,
            &items,
            &buena_password_categories,
        ),
        "BuenasPassword",
        &mut padded_divider,
    )
    .expect_err("padded Buena password guess rejected");
    assert_eq!(
        padded_guess_error,
        RandomSpecialRoutineError::Routine(SpecialRoutineError::InvalidBuenaPasswordGuess {
            routine: "BuenasPassword".to_string(),
            guess: " TOTODILE".to_string(),
        })
    );
    assert_eq!(padded_guess, before_padded_guess);

    state
        .script_runtime
        .variables
        .insert("BUENA_PASSWORD".to_string(), "TOTODILE".to_string());
    let mut repeat_divider = ReplayDivider::new([]);
    let correct = apply_random_special_routine_with_context(
        &mut state,
        full_context_with_buena_password_categories(
            &moves,
            &species,
            &learnsets,
            &items,
            &buena_password_categories,
        ),
        "BuenasPassword",
        &mut repeat_divider,
    )
    .expect("match password");
    assert_eq!(
        correct.effect,
        SpecialRoutineEffect::BuenasPassword {
            category: "JohtoStarters".to_string(),
            category_type: "BUENA_MON".to_string(),
            options: vec![
                "CYNDAQUIL".to_string(),
                "TOTODILE".to_string(),
                "CHIKORITA".to_string(),
            ],
            correct: "CYNDAQUIL".to_string(),
            guess: Some("TOTODILE".to_string()),
            matched: false,
            random_state_after: CrystalRandomState::default()
        }
    );
    assert_eq!(repeat_divider.consumed(), 0);
    assert!(
        !state
            .script_runtime
            .variables
            .contains_key("BUENA_PASSWORD")
    );

    let item = item_data("RARE_CANDY");
    let items = BTreeMap::from([("RARE_CANDY".to_string(), item.clone())]);
    let buena_prizes = BTreeMap::from([("RARE_CANDY".to_string(), 3)]);
    state.blue_card_balance = 10;
    state
        .script_runtime
        .variables
        .insert("_selected_prize".to_string(), "RARE_CANDY".to_string());
    state
        .script_runtime
        .variables
        .insert("_selected_prize_quantity".to_string(), "2".to_string());

    let prize = apply_special_routine_with_context(
        &mut state,
        full_context_with_buena_prizes(&moves, &species, &learnsets, &items, &buena_prizes),
        "BuenaPrize",
    )
    .expect("buena prize");

    assert_eq!(
        prize.effect,
        SpecialRoutineEffect::BuenaPrize {
            item_id: "RARE_CANDY".to_string(),
            quantity: 2,
            points_spent: 6,
            balance: 4
        }
    );
    assert_eq!(state.blue_card_balance, 4);
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("VAR_BLUECARDBALANCE")
            .map(String::as_str),
        Some("4")
    );
    assert_eq!(state.bag.quantity(&item), 2);
}

#[test]
fn buena_password_requires_explicit_modpack_category_data_without_builtin_table_fallback() {
    let mut state = GameState::default();
    state.time.current_day = 3;
    let before = state.clone();

    let move_catalog = moves();
    let mut divider = ReplayDivider::new([]);
    let error = apply_random_special_routine_with_context(
        &mut state,
        full_context(
            &move_catalog,
            &EMPTY_TEST_SPECIES,
            &EMPTY_TEST_LEARNSETS,
            &EMPTY_TEST_ITEMS,
        ),
        "BuenasPassword",
        &mut divider,
    )
    .expect_err("missing Buena password categories reject");

    assert_eq!(
        error,
        RandomSpecialRoutineError::Routine(SpecialRoutineError::MissingBuenaPasswordCategories {
            routine: "BuenasPassword".to_string()
        })
    );
    assert_eq!(state, before);
}

#[test]
fn buena_prize_requires_explicit_modpack_prize_data_without_builtin_table_fallback() {
    let mut state = GameState::default();
    state.blue_card_balance = 10;
    state
        .script_runtime
        .variables
        .insert("_selected_prize".to_string(), "RARE_CANDY".to_string());
    state
        .script_runtime
        .variables
        .insert("_selected_prize_quantity".to_string(), "1".to_string());
    let item = item_data("RARE_CANDY");
    let items = BTreeMap::from([("RARE_CANDY".to_string(), item)]);
    let species = BTreeMap::new();
    let learnsets = SpeciesLearnsets::new();
    let before = state.clone();

    let error = apply_special_routine_with_context(
        &mut state,
        full_context(&moves(), &species, &learnsets, &items),
        "BuenaPrize",
    )
    .expect_err("missing Buena prizes reject");

    assert_eq!(
        error,
        SpecialRoutineError::MissingBuenaPrizeDefinitions {
            routine: "BuenaPrize".to_string()
        }
    );
    assert_eq!(state, before);
}

#[test]
fn celebi_shrine_records_pending_special_battle_type() {
    let mut state = GameState::default();

    let outcome =
        apply_special_routine(&mut state, &moves(), "CelebiShrineEvent").expect("celebi shrine");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::CelebiShrineEvent {
            battle_type: "BATTLETYPE_CELEBI".to_string()
        }
    );
    assert_eq!(
        state.pending_special_battle_type.as_deref(),
        Some("BATTLETYPE_CELEBI")
    );
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("battle_type")
            .map(String::as_str),
        Some("BATTLETYPE_CELEBI")
    );
}

#[test]
fn magikarp_length_updates_exact_record_and_house_sign_buffer() {
    let mut state = GameState::default();
    state.player_id = 0x1234;
    state.magikarp_record.best_feet = 3;
    state.magikarp_record.best_inches = 0;
    let mut magikarp = pokemon("MAGIKARP");
    magikarp.original_trainer_name = "KRIS".to_string();
    magikarp.dvs = Dv::from_non_hp(10, 10, 10, 10);
    state
        .storage
        .register_capture_in_box(0, magikarp)
        .expect("store");
    state.sync_party_from_storage();
    state
        .script_runtime
        .variables
        .insert("_selected_party_index".to_string(), "0".to_string());
    let move_catalog = moves();
    let species_catalog = BTreeMap::new();
    let item_catalog = BTreeMap::new();
    let magikarp_lengths = vec![
        MagikarpLengthEntry {
            threshold: 110,
            divisor: 1,
        },
        MagikarpLengthEntry {
            threshold: 310,
            divisor: 2,
        },
        MagikarpLengthEntry {
            threshold: 710,
            divisor: 4,
        },
        MagikarpLengthEntry {
            threshold: 2710,
            divisor: 20,
        },
        MagikarpLengthEntry {
            threshold: 7710,
            divisor: 50,
        },
        MagikarpLengthEntry {
            threshold: 17710,
            divisor: 100,
        },
        MagikarpLengthEntry {
            threshold: 32710,
            divisor: 150,
        },
        MagikarpLengthEntry {
            threshold: 47710,
            divisor: 150,
        },
        MagikarpLengthEntry {
            threshold: 57710,
            divisor: 100,
        },
        MagikarpLengthEntry {
            threshold: 62710,
            divisor: 50,
        },
        MagikarpLengthEntry {
            threshold: 64710,
            divisor: 20,
        },
        MagikarpLengthEntry {
            threshold: 65210,
            divisor: 5,
        },
        MagikarpLengthEntry {
            threshold: 65410,
            divisor: 2,
        },
        MagikarpLengthEntry {
            threshold: 65510,
            divisor: 1,
        },
    ];
    state
        .script_runtime
        .variables
        .insert("_selection_cancelled".to_string(), "0".to_string());

    let outcome = apply_special_routine_with_context(
        &mut state,
        full_context_with_magikarp_lengths(
            &move_catalog,
            &species_catalog,
            &EMPTY_TEST_LEARNSETS,
            &item_catalog,
            &magikarp_lengths,
        ),
        "CheckMagikarpLength",
    )
    .expect("measure");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::CheckMagikarpLength {
            party_slot: 0,
            species: "MAGIKARP".to_string(),
            feet: 3,
            inches: 5,
            result: 3
        }
    );
    assert_eq!(state.magikarp_record.best_feet, 3);
    assert_eq!(state.magikarp_record.best_inches, 5);
    assert_eq!(state.magikarp_record.best_owner_name, "KRIS");
    assert_eq!(
        state
            .script_runtime
            .named_buffers
            .get("STRING_BUFFER_1")
            .map(String::as_str),
        Some("3'5\"")
    );

    let sign = apply_special_routine(&mut state, &moves(), "MagikarpHouseSign").expect("sign");

    assert_eq!(
        sign.effect,
        SpecialRoutineEffect::MagikarpHouseSign {
            feet: 3,
            inches: 5,
            formatted: "3'5\"".to_string()
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("3'5\""));
}

#[test]
fn selected_party_specials_require_exact_selection_without_slot_zero_fallback() {
    let mut shuckie_state = GameState::default();
    shuckie_state
        .storage
        .register_capture_in_box(0, pokemon("SHUCKLE"))
        .expect("store shuckie");
    shuckie_state.sync_party_from_storage();
    let before_shuckie = shuckie_state.clone();
    let move_catalog = moves();
    let species = BTreeMap::new();
    let learnsets = SpeciesLearnsets::new();
    let items = BTreeMap::new();
    let shuckie_gift = ShuckieGiftDefinition {
        species: "SHUCKLE".to_string(),
        level: 15,
        held_item: "BERRY".to_string(),
        nickname: "SHUCKIE".to_string(),
        original_trainer_name: "MANIA".to_string(),
        original_trainer_id: 518,
        got_today_engine_flag: "ENGINE_GOT_SHUCKIE_TODAY".to_string(),
    };
    let missing_shuckie_cancel = apply_special_routine_with_context(
        &mut shuckie_state,
        full_context_with_shuckie_gift(&move_catalog, &species, &learnsets, &items, &shuckie_gift),
        "ReturnShuckie",
    )
    .expect_err("missing shuckie cancellation input rejected");
    assert!(matches!(
        missing_shuckie_cancel,
        SpecialRoutineError::MissingScriptValue { routine, variable }
            if routine == "ReturnShuckie" && variable == "_selection_cancelled"
    ));
    assert_eq!(shuckie_state, before_shuckie);

    shuckie_state
        .script_runtime
        .variables
        .insert("_selection_cancelled".to_string(), "0".to_string());
    let before_shuckie_selection = shuckie_state.clone();
    let shuckie_error = apply_special_routine_with_context(
        &mut shuckie_state,
        full_context_with_shuckie_gift(&move_catalog, &species, &learnsets, &items, &shuckie_gift),
        "ReturnShuckie",
    )
    .expect_err("missing shuckie selection rejected");
    assert!(matches!(
        shuckie_error,
        SpecialRoutineError::MissingScriptValue { routine, variable }
            if routine == "ReturnShuckie" && variable == "_selected_party_index"
    ));
    assert_eq!(shuckie_state, before_shuckie_selection);

    let mut magikarp_state = GameState::default();
    magikarp_state
        .storage
        .register_capture_in_box(0, pokemon("MAGIKARP"))
        .expect("store magikarp");
    magikarp_state.sync_party_from_storage();
    let before_magikarp = magikarp_state.clone();
    let missing_magikarp_cancel =
        apply_special_routine(&mut magikarp_state, &moves(), "CheckMagikarpLength")
            .expect_err("missing magikarp cancellation input rejected");
    assert!(matches!(
        missing_magikarp_cancel,
        SpecialRoutineError::MissingScriptValue { routine, variable }
            if routine == "CheckMagikarpLength" && variable == "_selection_cancelled"
    ));
    assert_eq!(magikarp_state, before_magikarp);

    magikarp_state
        .script_runtime
        .variables
        .insert("_selection_cancelled".to_string(), "0".to_string());
    let before_magikarp_selection = magikarp_state.clone();
    let magikarp_error =
        apply_special_routine(&mut magikarp_state, &moves(), "CheckMagikarpLength")
            .expect_err("missing magikarp selection rejected");
    assert!(matches!(
        magikarp_error,
        SpecialRoutineError::MissingScriptValue { routine, variable }
            if routine == "CheckMagikarpLength" && variable == "_selected_party_index"
    ));
    assert_eq!(magikarp_state, before_magikarp_selection);
}

#[test]
fn day_care_deposit_inspect_and_withdraw_are_saveable_actions() {
    let mut state = GameState::default();
    let mut chikorita = pokemon("CHIKORITA");
    chikorita.experience =
        calculate_experience(&TEST_GROWTH_RATES, "GROWTH_MEDIUM_FAST", 5).unwrap() + 50;
    state
        .storage
        .register_capture_in_box(0, chikorita)
        .expect("store");
    state
        .storage
        .register_capture_in_box(0, pokemon("CYNDAQUIL"))
        .expect("store second");
    state.sync_party_from_storage();
    state.script_runtime.pending_day_care_input = Some(DayCareInput::Deposit { party_slot: 0 });

    let move_catalog = moves();
    let species = BTreeMap::new();
    let learnsets = BTreeMap::from([
        ("CHIKORITA".to_string(), Vec::new()),
        ("CYNDAQUIL".to_string(), Vec::new()),
    ]);
    let items = BTreeMap::new();
    let context = full_context(&move_catalog, &species, &learnsets, &items);
    let mut divider = ReplayDivider::new([]);
    let deposit =
        apply_random_special_routine_with_context(&mut state, context, "DayCareMan", &mut divider)
            .expect("deposit");

    assert_eq!(
        deposit.effect,
        SpecialRoutineEffect::DayCareInteraction {
            caretaker: "man".to_string(),
            action: "deposit".to_string(),
            success: true,
            pokemon: Some("CHIKORITA".to_string()),
            level: Some(5),
            reason: None,
        }
    );
    assert_eq!(state.storage.party.filled_slots(), 1);
    assert_eq!(
        state
            .day_care
            .man
            .pokemon
            .as_ref()
            .map(|pokemon| pokemon.species.id.as_str()),
        Some("CHIKORITA")
    );

    let inspect =
        apply_special_routine_with_context(&mut state, context, "DayCareMon1").expect("inspect");
    assert_eq!(
        inspect.effect,
        SpecialRoutineEffect::DayCareMon {
            caretaker: "man".to_string(),
            occupied: true,
            pokemon: Some("CHIKORITA".to_string()),
            level: Some(5)
        }
    );

    state.script_runtime.pending_day_care_input = Some(DayCareInput::Withdraw {});
    state.money = 100;
    let withdraw =
        apply_random_special_routine_with_context(&mut state, context, "DayCareMan", &mut divider)
            .expect("withdraw");

    assert_eq!(
        withdraw.effect,
        SpecialRoutineEffect::DayCareInteraction {
            caretaker: "man".to_string(),
            action: "withdraw".to_string(),
            success: true,
            pokemon: Some("CHIKORITA".to_string()),
            level: Some(5),
            reason: None,
        }
    );
    // Withdrawal returns the resident to the party, restoring the two
    // Pokémon present before the deposit.
    assert_eq!(state.storage.party.filled_slots(), 2);
    assert!(state.day_care.man.pokemon.is_none());
    assert!(!state.day_care.man.active);
    assert_eq!(
        state.storage.party.pokemon[1]
            .as_ref()
            .map(|pokemon| pokemon.experience),
        Some(calculate_experience(&TEST_GROWTH_RATES, "GROWTH_MEDIUM_FAST", 5).unwrap())
    );
    state
        .validate_saved_state()
        .expect("Day Care withdraw leaves save-valid state");
}

#[test]
fn day_care_withdraw_derives_level_shifts_moves_and_resets_experience_to_level_floor() {
    let mut resident = pokemon("CHIKORITA");
    resident.level = 5;
    resident.experience =
        calculate_experience(&TEST_GROWTH_RATES, "GROWTH_MEDIUM_FAST", 7).unwrap() + 100;
    resident.moves = ["MOVE1", "MOVE2", "MOVE3", "MOVE4"]
        .into_iter()
        .map(|name| LearnedMove {
            name: name.to_string(),
            current_pp: 1,
            pp_ups: 0,
        })
        .collect();
    let mut state = GameState::default();
    state.day_care.man.pokemon = Some(resident);
    state.day_care.man.active = true;
    state.money = 300;
    state.script_runtime.pending_day_care_input = Some(DayCareInput::Withdraw {});
    let move_catalog = ["MOVE1", "MOVE2", "MOVE3", "MOVE4", "SIX", "SEVEN"]
        .into_iter()
        .map(|name| (name.to_string(), move_data(name, 20)))
        .collect::<BTreeMap<_, _>>();
    let species = BTreeMap::new();
    let learnsets = BTreeMap::from([(
        "CHIKORITA".to_string(),
        vec![
            crate::systems::learnsets::LearnsetEntry(6, "SIX".to_string()),
            crate::systems::learnsets::LearnsetEntry(7, "SEVEN".to_string()),
        ],
    )]);
    let items = BTreeMap::new();
    let context = full_context(&move_catalog, &species, &learnsets, &items);

    apply_random_special_routine_with_context(
        &mut state,
        context,
        "DayCareMan",
        &mut ReplayDivider::new([]),
    )
    .expect("withdraw leveled resident");

    let pokemon = state.storage.party.pokemon[0]
        .as_ref()
        .expect("withdrawn resident");
    assert_eq!(pokemon.level, 7);
    assert_eq!(
        pokemon.experience,
        calculate_experience(&TEST_GROWTH_RATES, "GROWTH_MEDIUM_FAST", 7).unwrap()
    );
    assert_eq!(
        pokemon
            .moves
            .iter()
            .map(|learned| learned.name.as_str())
            .collect::<Vec<_>>(),
        vec!["MOVE3", "MOVE4", "SIX", "SEVEN"]
    );
    assert_eq!(state.money, 0);
}

#[test]
fn day_care_open_sets_the_source_active_bit_without_touching_script_results() {
    let mut state = GameState::default();
    state.script_runtime.script_value = Some("77".to_string());
    state.script_runtime.pending_day_care_input = Some(DayCareInput::Open {});

    let mut divider = ReplayDivider::new([]);
    let opened = apply_random_special_routine(&mut state, &moves(), "DayCareLady", &mut divider)
        .expect("open Day-Care lady interaction");

    assert_eq!(
        opened.effect,
        SpecialRoutineEffect::DayCareInteraction {
            caretaker: "lady".to_string(),
            action: "open".to_string(),
            success: true,
            pokemon: None,
            level: None,
            reason: None,
        }
    );
    assert!(state.day_care.lady.active);
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("77"));
    state
        .validate_saved_state()
        .expect("source Day-Care active bit is saveable without a resident");
}

#[test]
fn day_care_man_outside_does_not_invent_the_inside_intro_active_bit() {
    let mut state = GameState::default();
    assert!(!state.day_care.man.active);

    let outcome = apply_random_special_routine(
        &mut state,
        &moves(),
        "DayCareManOutside",
        &mut ReplayDivider::new([]),
    )
    .expect("outside man reports that no Egg is ready");

    assert!(matches!(
        outcome.effect,
        SpecialRoutineEffect::DayCareInteraction {
            caretaker,
            action,
            success: false,
            pokemon: None,
            level: None,
            reason: Some(reason),
        } if caretaker == "man" && action == "collect_egg" && reason == "no_egg"
    ));
    assert!(!state.day_care.man.active);
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("TRUE"));
    assert!(!state.script_runtime.variables.contains_key("_value"));
}

#[test]
fn day_care_interaction_requires_explicit_action_without_inspect_fallback() {
    let mut state = GameState::default();
    state
        .storage
        .register_capture_in_box(0, pokemon("CHIKORITA"))
        .expect("store");
    state.sync_party_from_storage();
    let before = state.clone();

    let mut divider = ReplayDivider::new([]);
    let error = apply_random_special_routine(&mut state, &moves(), "DayCareMan", &mut divider)
        .expect_err("missing day care action rejected");

    assert!(matches!(
        error,
        RandomSpecialRoutineError::Routine(SpecialRoutineError::MissingScriptValue { routine, variable })
            if routine == "DayCareMan" && variable == "pending_day_care_input"
    ));
    assert_eq!(state, before);
}

#[test]
fn day_care_withdraw_from_full_party_preserves_resident() {
    let mut state = GameState::default();
    for index in 0..crate::models::PARTY_SIZE {
        state
            .storage
            .register_capture_in_box(
                0,
                pokemon(if index % 2 == 0 {
                    "CHIKORITA"
                } else {
                    "CYNDAQUIL"
                }),
            )
            .expect("store full-party Pokemon");
    }
    state.sync_party_from_storage();
    state.day_care.man.pokemon = Some(pokemon("TOTODILE"));
    state.day_care.man.active = true;
    state.script_runtime.pending_day_care_input = Some(DayCareInput::Withdraw {});

    let before = state.day_care.man.clone();
    let mut divider = ReplayDivider::new([]);
    let outcome = apply_random_special_routine(&mut state, &moves(), "DayCareMan", &mut divider)
        .expect("full-party withdrawal should be a handled refusal");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::DayCareInteraction {
            caretaker: "man".to_string(),
            action: "withdraw".to_string(),
            success: false,
            pokemon: Some("TOTODILE".to_string()),
            level: Some(5),
            reason: Some("party_full".to_string()),
        }
    );
    assert_eq!(state.day_care.man, before);
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));
}

#[test]
fn day_care_rejects_unknown_caretaker_without_man_fallback_or_mutation() {
    let mut state = GameState::default();
    state.day_care.man.active = false;
    state.day_care.lady.active = true;
    let before = state.clone();

    let error = set_day_care_active(&mut state, "DayCareMan", "invalid", true)
        .expect_err("unknown caretaker must reject");

    assert!(matches!(
        error,
        SpecialRoutineError::InvalidDayCareCaretaker { routine, caretaker }
            if routine == "DayCareMan" && caretaker == "invalid"
    ));
    assert_eq!(state, before);
}

#[test]
fn day_care_compatibility_requires_matching_groups_and_opposite_gender() {
    let mut state = GameState::default();
    let mut female = pokemon("CHIKORITA");
    female.dvs.attack = 0;
    let mut male = pokemon("CHIKORITA");
    male.dvs.attack = 15;
    male.dvs.defense = 1;
    state.day_care.man.pokemon = Some(female);
    state.day_care.lady.pokemon = Some(male);
    let mut divider = ReplayDivider::new(divider_trace_for_sub_values([150, 0x12, 0x34]));
    let mut rng = CrystalRandom::new(state.random_state, &mut divider);
    update_day_care_compatibility(&mut state, &mut rng).expect("compatibility RNG");
    assert!(state.day_care.compatibility_score > 0);

    state.day_care.lady.pokemon.as_mut().unwrap().dvs.attack = 0;
    update_day_care_compatibility(&mut state, &mut rng).expect("zero compatibility");
    assert_eq!(state.day_care.compatibility_score, 0);

    state.day_care.lady.pokemon.as_mut().unwrap().dvs.attack = 15;
    state
        .day_care
        .lady
        .pokemon
        .as_mut()
        .unwrap()
        .species
        .egg_group1 = "EGG_DRAGON".to_string();
    state
        .day_care
        .lady
        .pokemon
        .as_mut()
        .unwrap()
        .species
        .egg_group2 = "EGG_DRAGON".to_string();
    update_day_care_compatibility(&mut state, &mut rng).expect("mismatched egg groups");
    assert_eq!(state.day_care.compatibility_score, 0);
}

#[test]
fn day_care_initial_countdown_retry_inherits_comparison_carry() {
    let mut state = GameState::default();
    let mut female = pokemon("CHIKORITA");
    female.dvs.attack = 0;
    let mut male = pokemon("CHIKORITA");
    male.dvs.attack = 15;
    male.dvs.defense = 1;
    state.day_care.man.pokemon = Some(female);
    state.day_care.lady.pokemon = Some(male);
    // First Random returns 149 and `cp 150` sets carry. The retry must enter
    // carry-set; with these samples that increments hRandomAdd to 1 while
    // returning the accepted subtraction byte 150.
    let mut divider = ReplayDivider::new([0, 107, 0, 255, 0, 0, 0, 0]);
    let mut rng = CrystalRandom::new(state.random_state, &mut divider);

    update_day_care_compatibility(&mut state, &mut rng).expect("countdown retry");

    assert_eq!(state.day_care.steps_until_next_egg, 150);
    assert_eq!(divider.consumed(), 8);
}

#[test]
fn day_care_init_breeding_prebuilds_the_egg_before_any_steps() {
    let mut state = GameState::default();
    let mut female = pokemon("CHIKORITA");
    female.dvs.attack = 0;
    let mut male = pokemon("CHIKORITA");
    male.dvs.attack = 15;
    male.dvs.defense = 1;
    state.day_care.man.pokemon = Some(female);
    state.day_care.lady.pokemon = Some(male);
    let mut divider = ReplayDivider::new(divider_trace_for_sub_values([150, 0x12, 0x34]));
    let mut rng = CrystalRandom::new(state.random_state, &mut divider);

    update_day_care_compatibility(&mut state, &mut rng).expect("initialize breeding");

    assert_eq!(state.day_care.steps_until_next_egg, 150);
    assert!(!state.day_care.egg_present);
    assert!(state.day_care.egg.as_ref().is_some_and(|egg| egg.is_egg));
    assert_eq!(divider.consumed(), 6);

    let mut no_divider = ReplayDivider::new([]);
    let mut no_rng = CrystalRandom::new(state.random_state, &mut no_divider);
    let outcome = day_care_collect_egg(&mut state, "DayCareManOutside", &mut no_rng)
        .expect("a prebuilt but unready egg is not collectible");
    assert!(!outcome.success);
    assert_eq!(outcome.reason.as_deref(), Some("no_egg"));
    assert!(state.day_care.egg.is_some());
}

#[test]
fn day_care_ff_compatibility_sentinel_never_initializes_or_rolls() {
    let mut state = GameState::default();
    let mut female = pokemon("CHIKORITA");
    female.dvs.attack = 0;
    let mut male = pokemon("CHIKORITA");
    male.dvs.attack = 15;
    // Equal defense and low three special DV bits produce Crystal's $ff
    // incompatibility sentinel even though species/groups/genders match.
    female.dvs.defense = 7;
    female.dvs.special = 5;
    male.dvs.defense = 7;
    male.dvs.special = 13;
    state.day_care.man.pokemon = Some(female);
    state.day_care.lady.pokemon = Some(male);
    let mut divider = ReplayDivider::new([]);
    let mut rng = CrystalRandom::new(state.random_state, &mut divider);

    update_day_care_compatibility(&mut state, &mut rng).expect("sentinel compatibility");
    advance_day_care_step(&mut state, &TEST_GROWTH_RATES, &mut rng)
        .expect("sentinel Day Care step");

    assert_eq!(state.day_care.compatibility_score, 255);
    assert_eq!(state.day_care.steps_until_next_egg, 0);
    assert!(!state.day_care.egg_present);
    assert_eq!(divider.consumed(), 0);
}

#[test]
fn day_care_deposit_is_atomic_when_countdown_trace_is_exhausted() {
    let mut state = GameState::default();
    let mut female = pokemon("CHIKORITA");
    female.dvs.attack = 0;
    let mut male = pokemon("CHIKORITA");
    male.dvs.attack = 15;
    male.dvs.defense = 1;
    state.day_care.man.pokemon = Some(female);
    state
        .storage
        .register_capture_in_box(0, male)
        .expect("store second parent");
    state.sync_party_from_storage();
    state.script_runtime.pending_day_care_input = Some(DayCareInput::Deposit { party_slot: 0 });
    let before = state.clone();
    let mut divider = ReplayDivider::new([0, 107]);

    let error = apply_random_special_routine(&mut state, &moves(), "DayCareLady", &mut divider)
        .expect_err("rejected countdown roll requires another exact Random call");

    assert!(matches!(error, RandomSpecialRoutineError::Divider(_)));
    assert_eq!(state, before);
    assert_eq!(divider.consumed(), 2);
}

#[test]
fn day_care_step_preserves_countdown_and_low_compatibility_carry() {
    let mut state = GameState {
        random_state: CrystalRandomState { add: 255, sub: 0 },
        ..GameState::default()
    };
    state.day_care.man.pokemon = Some(pokemon("CHIKORITA"));
    state.day_care.lady.pokemon = Some(pokemon("CYNDAQUIL"));
    state.day_care.compatibility_score = 51;
    state.day_care.steps_until_next_egg = 1;
    // The lady-level comparison feeds carry into the countdown Random. With
    // hRandomAdd=255 that carry overflows and makes hRandomSub 255. The low
    // compatibility tier also feeds carry into the probability Random.
    let mut divider = ReplayDivider::new([0, 0, 0, 245]);
    let mut rng = CrystalRandom::new(state.random_state, &mut divider);

    advance_day_care_step(&mut state, &TEST_GROWTH_RATES, &mut rng).expect("day-care step");

    assert_eq!(state.day_care.steps_until_next_egg, 255);
    assert!(!state.day_care.egg_present);
    assert_eq!(rng.state(), CrystalRandomState { add: 1, sub: 10 });
    assert_eq!(divider.consumed(), 4);
}

#[test]
fn day_care_countdown_random_clears_carry_on_the_exact_max_exp_step() {
    let mut state = GameState {
        random_state: CrystalRandomState { add: 255, sub: 0 },
        ..GameState::default()
    };
    state.day_care.man.pokemon = Some(pokemon("CHIKORITA"));
    let mut lady = pokemon("CYNDAQUIL");
    lady.experience = 0x4f_ffff;
    state.day_care.lady.pokemon = Some(lady);
    state.day_care.compatibility_score = 177;
    state.day_care.steps_until_next_egg = 1;
    // The three-byte increment reaches $500000, then `cp HIGH($500000)`
    // clears carry before the countdown Random call. With hRandomAdd=$ff,
    // incorrectly retaining the earlier level-comparison carry changes both
    // generated bytes.
    let mut divider = ReplayDivider::new([0, 0, 0, 156]);
    let mut rng = CrystalRandom::new(state.random_state, &mut divider);

    advance_day_care_step(&mut state, &TEST_GROWTH_RATES, &mut rng).expect("day-care step");

    assert_eq!(
        state.day_care.lady.pokemon.as_ref().unwrap().experience,
        0x50_0000
    );
    assert_eq!(state.day_care.steps_until_next_egg, 0);
    assert_eq!(rng.state(), CrystalRandomState { add: 255, sub: 100 });
    assert_eq!(divider.consumed(), 4);
}

#[test]
fn day_care_source_cap_allows_exp_to_continue_until_the_next_high_byte_wrap() {
    let mut state = GameState::default();
    let mut resident = pokemon("CHIKORITA");
    resident.experience = 0x4f_ffff;
    state.day_care.man.pokemon = Some(resident);
    let mut divider = ReplayDivider::new([]);
    let mut rng = CrystalRandom::new(state.random_state, &mut divider);

    advance_day_care_step(&mut state, &TEST_GROWTH_RATES, &mut rng).expect("cap boundary step");
    advance_day_care_step(&mut state, &TEST_GROWTH_RATES, &mut rng)
        .expect("post-boundary source increment");

    assert_eq!(
        state.day_care.man.pokemon.as_ref().unwrap().experience,
        0x50_0001
    );
    assert_eq!(divider.consumed(), 0);
}

#[test]
fn day_care_second_dv_roll_inherits_first_random_carry() {
    let mut state = GameState::default();
    let mut female = pokemon("CHIKORITA");
    female.dvs.attack = 0;
    let mut male = pokemon("CHIKORITA");
    male.dvs.attack = 15;
    male.dvs.defense = 1;
    state.day_care.man.pokemon = Some(female);
    state.day_care.lady.pokemon = Some(male);
    // Countdown -> 200, first DV -> 216 with carry set and hRandomAdd=255.
    // The inherited carry overflows hRandomAdd, making the second DV byte
    // 215 instead of 216.
    let mut divider = ReplayDivider::new([0, 56, 255, 240, 0, 0]);
    let mut rng = CrystalRandom::new(state.random_state, &mut divider);

    update_day_care_compatibility(&mut state, &mut rng).expect("initialize breeding");

    let egg = state.day_care.egg.as_ref().expect("generated egg");
    assert_eq!(egg.dvs.attack, 13);
    assert_eq!(egg.dvs.speed, 13);
    assert_eq!(egg.dvs.defense, 0, "male child inherits from its mother");
    assert_eq!(rng.state(), CrystalRandomState { add: 0, sub: 215 });
    assert_eq!(divider.consumed(), 6);
}

#[test]
fn day_care_steps_advance_residents_and_raise_egg_present() {
    let mut state = GameState::default();
    let mut female = pokemon("CHIKORITA");
    female.dvs.attack = 0;
    let mut male = pokemon("CHIKORITA");
    male.dvs.attack = 15;
    male.dvs.defense = 1;
    state.day_care.man.pokemon = Some(female);
    state.day_care.lady.pokemon = Some(male);
    let mut divider = ReplayDivider::new(divider_trace_for_sub_values([200, 0x12, 0x34, 200, 0]));
    let mut rng = CrystalRandom::new(state.random_state, &mut divider);
    update_day_care_compatibility(&mut state, &mut rng).expect("compatibility");

    let man_experience = state.day_care.man.pokemon.as_ref().unwrap().experience;
    let lady_experience = state.day_care.lady.pokemon.as_ref().unwrap().experience;
    for _ in 0..512 {
        advance_day_care_step(&mut state, &TEST_GROWTH_RATES, &mut rng).expect("day-care step");
    }
    assert_eq!(state.day_care.man.pokemon.as_ref().unwrap().level, 5);
    assert_eq!(state.day_care.lady.pokemon.as_ref().unwrap().level, 5);
    assert_eq!(
        state.day_care.man.pokemon.as_ref().unwrap().experience,
        man_experience + 512
    );
    assert_eq!(
        state.day_care.lady.pokemon.as_ref().unwrap().experience,
        lady_experience + 512
    );
    assert!(state.day_care.egg_present);
    assert_eq!(
        state
            .day_care
            .egg
            .as_ref()
            .map(|egg| egg.species.id.as_str()),
        Some("CHIKORITA")
    );
    assert!(state.day_care.egg.as_ref().is_some_and(|egg| egg.is_egg));
    assert_eq!(
        state.day_care.egg.as_ref().unwrap().dvs.defense,
        1,
        "female child inherits from its father"
    );
    assert_eq!(divider.consumed(), 10);
}

#[test]
fn day_care_nidoran_f_consumes_species_roll_before_dv_rolls() {
    let mut state = GameState::default();
    let mut ditto = pokemon("DITTO");
    let mut nidoran_f = pokemon("NIDORAN_F");
    ditto.dvs.defense = 1;
    ditto.dvs.special = 1;
    nidoran_f.dvs.defense = 2;
    nidoran_f.dvs.special = 2;
    state.day_care.man.pokemon = Some(ditto);
    state.day_care.lady.pokemon = Some(nidoran_f);
    let mut divider =
        ReplayDivider::new(divider_trace_for_sub_values([200, 200, 0x12, 0x34, 200, 0]));
    let mut rng = CrystalRandom::new(state.random_state, &mut divider);
    update_day_care_compatibility(&mut state, &mut rng).expect("compatibility");
    state.day_care.steps_until_next_egg = 1;
    advance_day_care_step(&mut state, &TEST_GROWTH_RATES, &mut rng).expect("day-care step");

    assert_eq!(
        state
            .day_care
            .egg
            .as_ref()
            .map(|egg| egg.species.id.as_str()),
        Some("NIDORAN_M")
    );
    assert!(state.day_care.egg.as_ref().is_some_and(|egg| egg.is_egg));
    assert_eq!(divider.consumed(), 12);
}

#[test]
fn day_care_ditto_uses_distinct_move_and_dv_inheritance_parents() {
    let mut state = GameState::default();
    let mut ditto = pokemon("DITTO");
    ditto.dvs.defense = 1;
    ditto.dvs.special = 1;
    ditto.moves[0].name = "DITTO_MOVE".to_string();
    let mut male = pokemon("CHIKORITA");
    male.dvs.attack = 15;
    male.dvs.defense = 9;
    male.dvs.special = 6;
    male.moves[0].name = "FATHER_MOVE".to_string();
    state.day_care.man.pokemon = Some(ditto);
    state.day_care.lady.pokemon = Some(male);
    let mut divider = ReplayDivider::new(divider_trace_for_sub_values([200, 0x12, 0x34]));
    let mut rng = CrystalRandom::new(state.random_state, &mut divider);

    update_day_care_compatibility(&mut state, &mut rng).expect("initialize breeding");

    let egg = state.day_care.egg.as_ref().expect("prebuilt egg");
    assert_eq!(egg.moves[0].name, "FATHER_MOVE");
    assert_eq!(egg.dvs.defense, 1);
    assert_eq!(egg.dvs.special, 1);
}

#[test]
fn day_care_step_does_not_consult_the_growth_rate_table() {
    let mut state = GameState::default();
    let mut resident = pokemon("CHIKORITA");
    resident.species.growth_rate = "GROWTH_UNKNOWN".to_string();
    state.day_care.man.pokemon = Some(resident);
    let experience_before = state.day_care.man.pokemon.as_ref().unwrap().experience;

    let mut divider = ReplayDivider::new([]);
    let mut rng = CrystalRandom::new(state.random_state, &mut divider);
    advance_day_care_step(&mut state, &TEST_GROWTH_RATES, &mut rng)
        .expect("the source increments the stored experience bytes directly");

    assert_eq!(
        state.day_care.man.pokemon.as_ref().unwrap().experience,
        experience_before + 1
    );
    assert_eq!(divider.consumed(), 0);
}

#[test]
fn bug_contest_setup_selects_contestants_and_drops_off_party_backup() {
    let mut state = GameState::default();
    state
        .storage
        .register_capture_in_box(0, pokemon("CHIKORITA"))
        .expect("lead");
    state
        .storage
        .register_capture_in_box(0, pokemon("CYNDAQUIL"))
        .expect("backup");
    state.sync_party_from_storage();
    let move_catalog = moves();
    let species_catalog = BTreeMap::new();
    let item_catalog = BTreeMap::new();
    let bug_contest_config = BugContestConfig {
        park_balls: 20,
        timer_minutes: 20,
        timer_seconds: 0,
        selected_contestant_count: 5,
        contestant_flags: vec![
            "EVENT_BUG_CATCHING_CONTESTANT_1A".to_string(),
            "EVENT_BUG_CATCHING_CONTESTANT_2A".to_string(),
            "EVENT_BUG_CATCHING_CONTESTANT_3A".to_string(),
            "EVENT_BUG_CATCHING_CONTESTANT_4A".to_string(),
            "EVENT_BUG_CATCHING_CONTESTANT_5A".to_string(),
            "EVENT_BUG_CATCHING_CONTESTANT_6A".to_string(),
            "EVENT_BUG_CATCHING_CONTESTANT_7A".to_string(),
            "EVENT_BUG_CATCHING_CONTESTANT_8A".to_string(),
            "EVENT_BUG_CATCHING_CONTESTANT_9A".to_string(),
            "EVENT_BUG_CATCHING_CONTESTANT_10A".to_string(),
        ],
        encounters: test_bug_contest_encounters(),
    };

    let balls = apply_special_routine_with_context(
        &mut state,
        full_context_with_bug_contest_config(
            &move_catalog,
            &species_catalog,
            &EMPTY_TEST_LEARNSETS,
            &item_catalog,
            &bug_contest_config,
        ),
        "GiveParkBalls",
    )
    .expect("park balls");

    assert_eq!(
        balls.effect,
        SpecialRoutineEffect::GiveParkBalls { balls: 20 }
    );
    assert_eq!(state.bug_contest.park_balls_remaining, 20);
    assert!(state.bug_contest.timer_active);
    assert_eq!(
        state.flags.is_engine_flag_set("ENGINE_BUG_CONTEST_TIMER"),
        Ok(true)
    );

    // 250 is rejected, the repeated 0 selects an already chosen flag,
    // and 0/25/50/75/100 then select canonical quotients 0..4.
    let mut contestant_divider =
        ReplayDivider::new(divider_trace_for_sub_values([250, 0, 0, 25, 50, 75, 100]));
    state
        .script_runtime
        .variables
        .insert("_bug_contestant_flags".to_string(), "stale".to_string());
    let contestants = apply_random_special_routine_with_context(
        &mut state,
        full_context_with_bug_contest_config(
            &move_catalog,
            &species_catalog,
            &EMPTY_TEST_LEARNSETS,
            &item_catalog,
            &bug_contest_config,
        ),
        "SelectRandomBugContestContestants",
        &mut contestant_divider,
    )
    .expect("contestants");
    assert_eq!(
        contestants.effect,
        SpecialRoutineEffect::SelectRandomBugContestContestants {
            flags: vec![
                "EVENT_BUG_CATCHING_CONTESTANT_1A".to_string(),
                "EVENT_BUG_CATCHING_CONTESTANT_2A".to_string(),
                "EVENT_BUG_CATCHING_CONTESTANT_3A".to_string(),
                "EVENT_BUG_CATCHING_CONTESTANT_4A".to_string(),
                "EVENT_BUG_CATCHING_CONTESTANT_5A".to_string(),
            ],
            random_state_after: CrystalRandomState { add: 0, sub: 100 }
        }
    );
    assert_eq!(contestant_divider.consumed(), 14);
    assert_eq!(state.random_state, CrystalRandomState { add: 0, sub: 100 });
    assert_eq!(state.bug_contest.selected_contestant_flags.len(), 5);
    assert!(
        !state
            .script_runtime
            .variables
            .contains_key("_bug_contestant_flags")
    );
    assert_eq!(
        state
            .flags
            .is_event_flag_set("EVENT_BUG_CATCHING_CONTESTANT_5A"),
        Ok(true)
    );

    let drop = apply_special_routine(&mut state, &moves(), "ContestDropOffMons").expect("drop off");
    assert_eq!(
        drop.effect,
        SpecialRoutineEffect::ContestDropOffMons {
            result: 0,
            backup_count: 1,
            second_party_species: Some("CYNDAQUIL".to_string())
        }
    );
    assert_eq!(state.storage.party.filled_slots(), 1);
    assert_eq!(state.bug_contest.party_backup.len(), 1);

    let returned =
        apply_special_routine(&mut state, &moves(), "ContestReturnMons").expect("return mons");
    assert_eq!(
        returned.effect,
        SpecialRoutineEffect::ContestReturnMons { restored_count: 2 }
    );
    assert_eq!(state.storage.party.filled_slots(), 2);
    assert!(state.bug_contest.party_backup.is_empty());
}

#[test]
fn bug_contest_timer_matches_elapsed_asm_arithmetic() {
    let mut state = GameState::default();

    let not_started = apply_special_routine(&mut state, &moves(), "CheckBugContestTimer")
        .expect_err("timer check must require a started timer");
    assert!(matches!(
        not_started,
        SpecialRoutineError::BugContestTimerNotStarted { .. }
    ));

    let started =
        apply_special_routine(&mut state, &moves(), "StartBugContestTimer").expect("start");
    assert_eq!(
        started.effect,
        SpecialRoutineEffect::BugContestTimer {
            active: true,
            minutes_remaining: 20,
            seconds_remaining: 0,
        }
    );
    assert_eq!(state.bug_contest.timer_start_time.unwrap().second, 0);

    state.time.registers.minutes = 19;
    state.time.registers.seconds = 30;
    let remaining =
        apply_special_routine(&mut state, &moves(), "CheckBugContestTimer").expect("check");
    assert_eq!(
        remaining.effect,
        SpecialRoutineEffect::BugContestTimer {
            active: true,
            minutes_remaining: 0,
            seconds_remaining: 30,
        }
    );

    state.time.registers.minutes = 20;
    state.time.registers.seconds = 0;
    let exact_boundary =
        apply_special_routine(&mut state, &moves(), "CheckBugContestTimer").expect("boundary");
    assert_eq!(
        exact_boundary.effect,
        SpecialRoutineEffect::BugContestTimer {
            active: true,
            minutes_remaining: 0,
            seconds_remaining: 0,
        }
    );

    state.time.registers.seconds = 1;
    let expired =
        apply_special_routine(&mut state, &moves(), "CheckBugContestTimer").expect("expired");
    assert_eq!(
        expired.effect,
        SpecialRoutineEffect::BugContestTimer {
            active: false,
            minutes_remaining: 0,
            seconds_remaining: 0,
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));
}

#[test]
fn bug_contest_caught_mon_is_registered_and_judging_records_rank() {
    let mut state = GameState::default();
    state.time.time_of_day = crate::world::encounters::TimeOfDay::Day;
    state.player_gender = 1;
    state
        .storage
        .register_capture_in_box(0, pokemon("CHIKORITA"))
        .expect("lead");
    state.sync_party_from_storage();
    state.bug_contest.caught_mon = Some(pokemon("SCYTHER"));

    let result = apply_special_routine(&mut state, &moves(), "CheckPartyFullAfterContest")
        .expect("contest catch");

    assert_eq!(
        result.effect,
        SpecialRoutineEffect::CheckPartyFullAfterContest {
            result: 0,
            species: Some("SCYTHER".to_string())
        }
    );
    assert_eq!(state.storage.party.filled_slots(), 2);
    assert_eq!(
        state.storage.party.pokemon[1]
            .as_ref()
            .and_then(|pokemon| pokemon.caught_data.as_ref()),
        Some(&CaughtData {
            level: 5,
            time_of_day: Some(crate::world::encounters::TimeOfDay::Day),
            original_trainer_gender: 1,
            location: 0x13,
        })
    );
    assert!(state.bug_contest.caught_mon.is_none());
}

#[test]
fn bug_contest_full_selected_box_discards_catch_without_alternate_box_routing() {
    let mut state = GameState::default();
    state.time.time_of_day = crate::world::encounters::TimeOfDay::Day;
    state.player_gender = 0;
    for index in 0..6 {
        assert!(
            state
                .storage
                .party
                .add_pokemon(pokemon(&format!("PARTY_{index}")))
        );
    }
    let mut full_current_box = PcBox::new(0);
    for index in 0..MAX_BOX_MONS {
        let mut boxed = pokemon(&format!("BOXED_{index}"));
        if index == 0 {
            boxed.caught_data = Some(crate::models::pokemon::CaughtData {
                level: 5,
                time_of_day: Some(crate::world::encounters::TimeOfDay::Morning),
                original_trainer_gender: 1,
                location: 2,
            });
        }
        assert!(full_current_box.add_pokemon(boxed));
    }
    state.storage.pc_boxes = vec![full_current_box, PcBox::new(1)];
    state.current_pc_box = 0;
    state.sync_party_from_storage();
    state.bug_contest.caught_mon = Some(pokemon("SCYTHER"));

    let result = apply_special_routine(&mut state, &moves(), "CheckPartyFullAfterContest")
        .expect("full current box still returns BOXED_MON");

    assert_eq!(
        result.effect,
        SpecialRoutineEffect::CheckPartyFullAfterContest {
            result: 1,
            species: Some("SCYTHER".to_string())
        }
    );
    assert_eq!(state.storage.pc_boxes[0].filled_slots(), MAX_BOX_MONS);
    assert_eq!(state.storage.pc_boxes[1].filled_slots(), 0);
    assert_eq!(
        state.storage.pc_boxes[0].pokemon[0]
            .as_ref()
            .and_then(|pokemon| pokemon.caught_data.as_ref())
            .map(|caught| (
                caught.level,
                caught.time_of_day,
                caught.original_trainer_gender,
                caught.location,
            )),
        Some((5, Some(crate::world::encounters::TimeOfDay::Day), 0, 0x13,))
    );
    assert!(state.bug_contest.caught_mon.is_none());
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));
}

#[test]
fn magikarp_length_requires_modpack_table_without_constant_fallback() {
    let mut state = GameState::default();
    let magikarp = pokemon("MAGIKARP");
    state
        .storage
        .register_capture_in_box(0, magikarp)
        .expect("store");
    state.sync_party_from_storage();
    state
        .script_runtime
        .variables
        .insert("_selected_party_index".to_string(), "0".to_string());
    state
        .script_runtime
        .variables
        .insert("_selection_cancelled".to_string(), "0".to_string());
    let before = state.clone();

    let error = apply_special_routine(&mut state, &moves(), "CheckMagikarpLength")
        .expect_err("missing Magikarp length table rejected");

    assert!(matches!(
        error,
        SpecialRoutineError::MissingMagikarpLengthTable { routine }
            if routine == "CheckMagikarpLength"
    ));
    assert_eq!(state, before);
}

#[test]
fn bug_contest_setup_requires_modpack_config_without_constant_fallback() {
    let mut state = GameState::default();
    let before = state.clone();

    let error = apply_special_routine(&mut state, &moves(), "GiveParkBalls")
        .expect_err("missing Bug-Catching Contest config rejected");

    assert!(matches!(
        error,
        SpecialRoutineError::MissingBugContestConfig { routine }
            if routine == "GiveParkBalls"
    ));
    assert_eq!(state, before);
}

#[test]
fn bug_contest_judging_scores_caught_mon_and_contestants() {
    let mut state = GameState::default();
    state.player_name = "KRIS".to_string();
    state.bug_contest.caught_mon = Some(pokemon("SCYTHER"));
    let config = BugContestConfig {
        park_balls: 20,
        timer_minutes: 20,
        timer_seconds: 0,
        selected_contestant_count: 5,
        contestant_flags: (1..=10)
            .map(|index| format!("EVENT_BUG_CATCHING_CONTESTANT_{index}A"))
            .collect(),
        encounters: test_bug_contest_encounters(),
    };
    state.bug_contest.selected_contestant_flags = config.contestant_flags[..5].to_vec();
    let move_catalog = moves();
    // Five active AI contestants each consume one accepted placement
    // byte and one score-perturbation byte.
    let mut divider = ReplayDivider::new([0; 20]);
    let judging = apply_random_special_routine_with_context(
        &mut state,
        full_context_with_bug_contest_config(
            &move_catalog,
            &EMPTY_TEST_SPECIES,
            &EMPTY_TEST_LEARNSETS,
            &EMPTY_TEST_ITEMS,
            &config,
        ),
        "BugContestJudging",
        &mut divider,
    )
    .expect("judging");
    let SpecialRoutineEffect::BugContestJudging {
        rank,
        placements,
        random_state_after,
    } = judging.effect
    else {
        panic!("expected BugContestJudging effect");
    };
    assert_eq!(state.script_runtime.script_value, Some(rank.to_string()));
    assert_eq!(placements.len(), 3);
    assert_eq!(random_state_after, CrystalRandomState::default());
    assert_eq!(state.random_state, random_state_after);
    assert_eq!(divider.consumed(), 20);
}

#[test]
fn bug_contest_player_score_adds_only_low_stat_bytes_like_contest_score() {
    let mut caught = pokemon("SCYTHER");
    caught.max_hp = 0x0102;
    caught.attack = 0x0103;
    caught.defense = 0x0204;
    caught.speed = 0x0305;
    caught.special_attack = 0x0406;
    caught.special_defense = 0x0507;
    caught.hp = 0x0108;
    caught.dvs = Dv::default();
    caught.item = Some("BERRY".to_string());

    // ContestScore reads the low byte (`+ 1`) of every big-endian stat,
    // including current HP before shifting it right three times:
    // 2*4 + 3 + 4 + 5 + 6 + 7 + 0 DV points + 8/8 + 1 held item.
    assert_eq!(bug_contest_player_score(&caught), 35);
}

#[test]
fn bug_contest_judging_rejects_masked_three_and_preserves_random_order() {
    let config = BugContestConfig {
        park_balls: 20,
        timer_minutes: 20,
        timer_seconds: 0,
        selected_contestant_count: 5,
        contestant_flags: (1..=10)
            .map(|index| format!("EVENT_BUG_CATCHING_CONTESTANT_{index}A"))
            .collect(),
        encounters: test_bug_contest_encounters(),
    };
    let mut state = GameState::default();
    // Keep the last three contestants active so the source winner table is
    // complete; Kipp's placement byte still exercises the rejected masked 3.
    state.bug_contest.selected_contestant_flags = config.contestant_flags[..7].to_vec();
    let move_catalog = moves();
    let mut divider = ReplayDivider::new(divider_trace_for_sub_values([0, 0, 0, 0, 3, 2, 7]));

    let judging = apply_random_special_routine_with_context(
        &mut state,
        full_context_with_bug_contest_config(
            &move_catalog,
            &EMPTY_TEST_SPECIES,
            &EMPTY_TEST_LEARNSETS,
            &EMPTY_TEST_ITEMS,
            &config,
        ),
        "BugContestJudging",
        &mut divider,
    )
    .expect("judge three active contestants");

    let SpecialRoutineEffect::BugContestJudging {
        placements,
        random_state_after,
        ..
    } = judging.effect
    else {
        panic!("expected BugContestJudging effect");
    };
    assert!(placements.iter().any(|placement| {
        placement.winner_id == 11 && placement.species == "KAKUNA" && placement.score == 266
    }));
    assert_eq!(random_state_after, CrystalRandomState { add: 0, sub: 7 });
    assert_eq!(divider.consumed(), 14);
}

#[test]
fn bug_contest_judging_rejects_an_incomplete_winner_table_without_placeholder_names() {
    let config = BugContestConfig {
        park_balls: 20,
        timer_minutes: 20,
        timer_seconds: 0,
        selected_contestant_count: 5,
        contestant_flags: (1..=10)
            .map(|index| format!("EVENT_BUG_CATCHING_CONTESTANT_{index}A"))
            .collect(),
        encounters: test_bug_contest_encounters(),
    };
    let mut state = GameState::default();
    state.bug_contest.selected_contestant_flags = config.contestant_flags[..9].to_vec();
    let before = state.clone();
    let move_catalog = moves();
    let mut divider = ReplayDivider::new([]);

    let error = apply_random_special_routine_with_context(
        &mut state,
        full_context_with_bug_contest_config(
            &move_catalog,
            &EMPTY_TEST_SPECIES,
            &EMPTY_TEST_LEARNSETS,
            &EMPTY_TEST_ITEMS,
            &config,
        ),
        "BugContestJudging",
        &mut divider,
    )
    .expect_err("judging cannot present uninitialized winner slots");

    assert!(matches!(
        error,
        RandomSpecialRoutineError::Routine(SpecialRoutineError::InvalidState { routine, message })
            if routine == "BugContestJudging"
                && message.contains("three initialized winner slots")
    ));
    assert_eq!(state, before);
    assert_eq!(divider.consumed(), 0);
}

#[test]
fn link_request_timeout_same_room_and_close_update_saveable_session() {
    let mut state = GameState::default();
    state.script_runtime.script_value = Some("77".to_string());

    let trade = apply_special_routine(&mut state, &moves(), "SetBitsForLinkTradeRequest")
        .expect("trade request");
    assert_eq!(
        trade.effect,
        SpecialRoutineEffect::LinkAction { action: 1, room: 1 }
    );
    assert_eq!(state.link_session.chosen_cable_club_room, 1);
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("77"));

    state.link_session.serial_connection_status = LinkSerialConnectionStatus::UsingInternalClock;
    let friend =
        apply_special_routine(&mut state, &moves(), "WaitForLinkedFriend").expect("friend ready");
    assert_eq!(
        friend.effect,
        SpecialRoutineEffect::LinkedFriendResult {
            success: true,
            link_mode: 0,
            delay_frames: 50
        }
    );
    assert_eq!(
        state.link_session.serial_connection_status,
        LinkSerialConnectionStatus::UsingInternalClock
    );

    let quick_save = apply_special_routine(&mut state, &moves(), "TryQuickSave")
        .expect("pre-room quick-save boundary");
    assert_eq!(
        quick_save.effect,
        SpecialRoutineEffect::QuickSave {
            requested: true,
            delay_frames: 30
        }
    );
    let saved = serde_json::to_value(&state).expect("serialize pre-room handshake");
    let reloaded: GameState = serde_json::from_value(saved).expect("reload pre-room handshake");
    assert_eq!(
        reloaded.link_session.serial_connection_status,
        LinkSerialConnectionStatus::UsingInternalClock
    );
    assert_eq!(reloaded.link_session.link_mode, 0);

    state
        .script_runtime
        .variables
        .insert("_other_player_room".to_string(), "1".to_string());
    let same = apply_special_routine(&mut state, &moves(), "CheckBothSelectedSameRoom")
        .expect("same room");
    assert_eq!(
        same.effect,
        SpecialRoutineEffect::LinkResult {
            success: true,
            link_mode: 2,
            delay_frames: 1
        }
    );
    assert_eq!(state.link_session.link_mode, 2);

    let close = apply_special_routine(&mut state, &moves(), "CloseLink").expect("close");
    assert_eq!(
        close.effect,
        SpecialRoutineEffect::LinkDelay {
            link_mode: 0,
            delay_frames: 6
        }
    );
    assert_eq!(state.link_session.link_mode, 0);
    assert_eq!(state.link_session.player_link_action, 1);
    assert_eq!(state.link_session.chosen_cable_club_room, 1);
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));

    let exited = apply_special_routine(&mut state, &moves(), "WaitForOtherPlayerToExit")
        .expect("other player exit");
    assert_eq!(
        exited.effect,
        SpecialRoutineEffect::LinkDelay {
            link_mode: 0,
            delay_frames: 12
        }
    );
    assert_eq!(state.link_session.player_link_action, 1);
    assert_eq!(state.link_session.chosen_cable_club_room, 1);
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));

    state.link_session.chosen_cable_club_room = 2;
    let timeout = apply_special_routine(&mut state, &moves(), "CheckLinkTimeout_Receptionist")
        .expect("timeout");
    assert_eq!(
        timeout.effect,
        SpecialRoutineEffect::LinkResult {
            success: false,
            link_mode: 0,
            delay_frames: 5
        }
    );
    assert_eq!(state.link_session.player_link_action, 1);
    assert_eq!(state.link_session.chosen_cable_club_room, 2);

    state.link_session.chosen_cable_club_room = 2;
    state
        .script_runtime
        .variables
        .insert("_other_player_link_mode".to_string(), "3".to_string());
    state.link_session.serial_connection_status = LinkSerialConnectionStatus::UsingExternalClock;
    let connected = apply_special_routine(&mut state, &moves(), "CheckLinkTimeout_Receptionist")
        .expect("connected");
    assert_eq!(
        connected.effect,
        SpecialRoutineEffect::LinkResult {
            success: true,
            link_mode: 0,
            delay_frames: 2
        }
    );
    assert_eq!(state.link_session.player_link_action, 1);
    assert_eq!(state.link_session.other_player_link_mode, 3);
}

#[test]
fn link_peer_inputs_are_required_without_session_fallbacks() {
    let cases = [
        ("CheckBothSelectedSameRoom", "_other_player_room"),
        ("AskRememberPassword", "_yes_no_result"),
    ];

    for (routine, variable) in cases {
        let mut state = GameState::default();
        state.link_session.chosen_cable_club_room = 2;
        state.link_session.player_link_action = 1;
        let before = state.clone();

        let error = apply_special_routine(&mut state, &moves(), routine)
            .expect_err("missing peer input must reject");

        assert!(matches!(
            error,
            SpecialRoutineError::MissingScriptValue {
                routine: rejected_routine,
                variable: rejected_variable
            } if rejected_routine == routine && rejected_variable == variable
        ));
        assert_eq!(state, before);
    }

    let mut state = GameState::default();
    state.link_session.serial_connection_status = LinkSerialConnectionStatus::UsingInternalClock;
    let before = state.clone();
    let error = apply_special_routine(&mut state, &moves(), "CheckLinkTimeout_Receptionist")
        .expect_err("non-timeout handshake must include other player mode");
    assert!(matches!(
        error,
        SpecialRoutineError::MissingScriptValue { routine, variable }
            if routine == "CheckLinkTimeout_Receptionist"
                && variable == "_other_player_link_mode"
    ));
    assert_eq!(state, before);

    let mut state = GameState::default();
    state
        .script_runtime
        .variables
        .insert("_yes_no_result".to_string(), "true".to_string());
    let before = state.clone();
    let error = apply_special_routine(&mut state, &moves(), "AskRememberPassword")
        .expect_err("boolean script inputs must not accept string aliases");
    assert!(matches!(
        error,
        SpecialRoutineError::InvalidNumericValue { routine, value }
            if routine == "AskRememberPassword" && value == "true"
    ));
    assert_eq!(state, before);

    let mut state = GameState::default();
    state
        .script_runtime
        .variables
        .insert("_remember_password".to_string(), "1".to_string());
    let before = state.clone();
    let error = apply_special_routine(&mut state, &moves(), "AskRememberPassword")
        .expect_err("remember password output must not alias yes/no input");
    assert!(matches!(
        error,
        SpecialRoutineError::MissingScriptValue { routine, variable }
            if routine == "AskRememberPassword" && variable == "_yes_no_result"
    ));
    assert_eq!(state, before);

    let mut state = GameState::default();
    state
        .script_runtime
        .variables
        .insert("_selection_cancelled".to_string(), "true".to_string());
    let before = state.clone();
    let error = apply_special_routine(&mut state, &moves(), "CheckMagikarpLength")
        .expect_err("selection cancellation must use exact numeric script values");
    assert!(matches!(
        error,
        SpecialRoutineError::InvalidNumericValue { routine, value }
            if routine == "CheckMagikarpLength" && value == "true"
    ));
    assert_eq!(state, before);
}

#[test]
fn cable_club_chris_branch_uses_serial_clock_ownership_not_player_gender() {
    let mut external = GameState::default();
    external.link_session.serial_connection_status = LinkSerialConnectionStatus::UsingExternalClock;
    external.player_gender = crate::state::PLAYER_GENDER_FEMALE;

    let outcome = apply_special_routine(&mut external, &moves(), "CableClubCheckWhichChris")
        .expect("external-clock player branch");

    assert_eq!(external.script_runtime.script_value.as_deref(), Some("1"));
    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::CableClubCheckWhichChris {
            external_clock_player: true
        }
    );

    let mut internal = GameState::default();
    internal.link_session.serial_connection_status = LinkSerialConnectionStatus::UsingInternalClock;
    internal.player_gender = crate::state::PLAYER_GENDER_MALE;

    apply_special_routine(&mut internal, &moves(), "CableClubCheckWhichChris")
        .expect("internal-clock player branch");
    assert_eq!(internal.script_runtime.script_value.as_deref(), Some("0"));
}

#[test]
fn link_rooms_time_capsule_quick_save_and_gender_branch_are_stateful() {
    let mut failed = GameState::default();
    failed.link_session.link_mode = 2;
    failed.link_session.player_link_action = 1;
    failed.link_session.chosen_cable_club_room = 1;
    failed.link_session.other_player_link_mode = 1;
    failed.link_session.serial_connection_status = LinkSerialConnectionStatus::UsingExternalClock;
    failed.script_runtime.script_value = Some("77".to_string());
    let failure = apply_special_routine(&mut failed, &moves(), "FailedLinkToPast")
        .expect("Time Capsule failure");
    assert_eq!(
        failure.effect,
        SpecialRoutineEffect::LinkDelay {
            link_mode: 2,
            delay_frames: 40
        }
    );
    assert_eq!(failed.link_session.link_mode, 2);
    assert_eq!(failed.link_session.player_link_action, 1);
    assert_eq!(failed.link_session.chosen_cable_club_room, 1);
    assert_eq!(failed.link_session.other_player_link_mode, 1);
    assert_eq!(failed.script_runtime.script_value.as_deref(), Some("77"));
    assert_eq!(
        failed.link_session.serial_connection_status,
        LinkSerialConnectionStatus::UsingExternalClock
    );

    let mut state = GameState::default();
    state.script_runtime.script_value = Some("77".to_string());
    let trade_center =
        apply_special_routine(&mut state, &moves(), "TradeCenter").expect("trade center");
    assert_eq!(
        trade_center.effect,
        SpecialRoutineEffect::LinkRoom {
            room: "TradeCenter".to_string(),
            link_mode: 2,
            session: true
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("77"));
    let colosseum = apply_special_routine(&mut state, &moves(), "Colosseum").expect("colosseum");
    assert_eq!(
        colosseum.effect,
        SpecialRoutineEffect::LinkRoom {
            room: "Colosseum".to_string(),
            link_mode: 3,
            session: true
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("77"));
    let capsule = apply_special_routine(&mut state, &moves(), "EnterTimeCapsule").expect("capsule");
    assert_eq!(
        capsule.effect,
        SpecialRoutineEffect::LinkRoom {
            room: "TimeCapsule".to_string(),
            link_mode: 1,
            session: false
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("77"));

    state.link_session.player_link_action = 2;
    state.link_session.chosen_cable_club_room = 2;
    state.link_session.link_mode = 3;
    let room_capsule =
        apply_special_routine(&mut state, &moves(), "TimeCapsule").expect("Time Capsule room");
    assert_eq!(
        room_capsule.effect,
        SpecialRoutineEffect::LinkRoom {
            room: "TimeCapsule".to_string(),
            link_mode: 1,
            session: true
        }
    );
    assert_eq!(state.link_session.link_mode, 1);
    assert_eq!(state.link_session.player_link_action, 2);
    assert_eq!(state.link_session.chosen_cable_club_room, 2);
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("77"));

    let mut celebi = pokemon("CELEBI");
    celebi.species.int_id = 251;
    celebi.nickname = "ILEX".to_string();
    state
        .storage
        .register_capture_in_box(0, celebi)
        .expect("store celebi");
    state.sync_party_from_storage();
    let compat = apply_special_routine(&mut state, &moves(), "CheckTimeCapsuleCompatibility")
        .expect("compat");
    assert_eq!(
        compat.effect,
        SpecialRoutineEffect::TimeCapsuleCompatibility {
            result_code: 1,
            mon_name: Some("CELEBI".to_string()),
            move_name: None
        }
    );

    let save = apply_special_routine(&mut state, &moves(), "TryQuickSave").expect("save");
    assert_eq!(
        save.effect,
        SpecialRoutineEffect::QuickSave {
            requested: true,
            delay_frames: 30
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));

    let ask = apply_special_routine(&mut state, &moves(), "AskMobileOrCable").expect("ask cable");
    assert_eq!(
        ask.effect,
        SpecialRoutineEffect::AskMobileOrCable {
            selection: ".Cable".to_string()
        }
    );

    state.link_session.serial_connection_status = LinkSerialConnectionStatus::UsingInternalClock;
    let chris =
        apply_special_routine(&mut state, &moves(), "CableClubCheckWhichChris").expect("chris");
    assert_eq!(
        chris.effect,
        SpecialRoutineEffect::CableClubCheckWhichChris {
            external_clock_player: false
        }
    );
}

#[test]
fn time_capsule_checks_party_wide_species_mail_then_gen_two_moves() {
    let mut species_before_mail = GameState::default();
    let mut mailed_kanto = pokemon("PIKACHU");
    mailed_kanto.species.int_id = 25;
    mailed_kanto.item = Some("FLOWER_MAIL".to_string());
    let mut johto = pokemon("CHIKORITA");
    johto.species.int_id = 152;
    species_before_mail
        .storage
        .register_capture_in_box(0, mailed_kanto)
        .expect("store mailed Kanto Pokemon");
    species_before_mail
        .storage
        .register_capture_in_box(0, johto)
        .expect("store Johto Pokemon");
    species_before_mail.sync_party_from_storage();

    let species = apply_special_routine(
        &mut species_before_mail,
        &moves(),
        "CheckTimeCapsuleCompatibility",
    )
    .expect("check species before mail");
    assert!(matches!(
        species.effect,
        SpecialRoutineEffect::TimeCapsuleCompatibility {
            result_code: 1,
            mon_name: Some(ref name),
            move_name: None,
        } if name == "CHIKORITA"
    ));

    let mut move_state = GameState::default();
    let mut sketch_user = pokemon("PIKACHU");
    sketch_user.species.int_id = 25;
    sketch_user.nickname = "INKY".to_string();
    sketch_user.moves[0].name = "SKETCH".to_string();
    move_state
        .storage
        .register_capture_in_box(0, sketch_user)
        .expect("store move-incompatible Pokemon");
    move_state.sync_party_from_storage();
    let mut move_catalog = moves();
    let mut sketch = move_data("SKETCH", 1);
    sketch.source_index = 166;
    move_catalog.insert("SKETCH".to_string(), sketch);

    let move_result = apply_special_routine(
        &mut move_state,
        &move_catalog,
        "CheckTimeCapsuleCompatibility",
    )
    .expect("check Gen II move");
    assert_eq!(
        move_result.effect,
        SpecialRoutineEffect::TimeCapsuleCompatibility {
            result_code: 2,
            mon_name: Some("PIKACHU".to_string()),
            move_name: Some("SKETCH".to_string()),
        }
    );
    assert_eq!(
        move_state
            .script_runtime
            .named_buffers
            .get("STRING_BUFFER_2")
            .map(String::as_str),
        Some("SKETCH")
    );
    assert_eq!(
        move_state
            .script_runtime
            .named_buffers
            .get("STRING_BUFFER_1")
            .map(String::as_str),
        Some("PIKACHU")
    );
}

#[test]
fn battle_tower_rules_actions_and_records_are_saveable() {
    let mut state = GameState::default();
    state.time.current_day = 9;
    state.time.day_of_week = state.time.current_day % 7;
    state
        .storage
        .register_capture_in_box(0, pokemon("CHIKORITA"))
        .expect("store chikorita");
    state
        .storage
        .register_capture_in_box(0, pokemon("CYNDAQUIL"))
        .expect("store cyndaquil");
    state
        .storage
        .register_capture_in_box(0, pokemon("TOTODILE"))
        .expect("store totodile");
    state.sync_party_from_storage();

    let move_catalog = moves();
    let battle_tower_rules = battle_tower_rules_with_banned_species(vec![]);
    let rules = apply_special_routine_with_context(
        &mut state,
        full_context_with_battle_tower_rules(&move_catalog, &battle_tower_rules),
        "CheckForBattleTowerRules",
    )
    .expect("rules");
    assert_eq!(
        rules.effect,
        SpecialRoutineEffect::CheckForBattleTowerRules { failures: vec![] }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));

    state.script_runtime.script_value = Some("BATTLETOWERACTION_SET_EXPLANATION_READ".to_string());
    let explanation =
        apply_special_routine(&mut state, &move_catalog, "BattleTowerAction").expect("set read");
    assert_eq!(
        explanation.effect,
        SpecialRoutineEffect::BattleTowerAction {
            action: "BATTLETOWERACTION_SET_EXPLANATION_READ".to_string(),
            value: "1".to_string(),
            truthy: true
        }
    );
    assert!(state.battle_tower.explanation_read);

    state.battle_tower.level_group = 5;
    state.script_runtime.script_value = Some("BATTLETOWERACTION_SAVELEVELGROUP".to_string());
    let level_group = apply_special_routine(&mut state, &move_catalog, "BattleTowerAction")
        .expect("save level group");
    assert_eq!(
        level_group.effect,
        SpecialRoutineEffect::BattleTowerAction {
            action: "BATTLETOWERACTION_SAVELEVELGROUP".to_string(),
            value: "1".to_string(),
            truthy: true
        }
    );
    assert_eq!(state.battle_tower.level_group, 5);

    state.script_runtime.script_value = Some("BATTLETOWERACTION_LOADLEVELGROUP".to_string());
    let loaded_level_group = apply_special_routine(&mut state, &move_catalog, "BattleTowerAction")
        .expect("load level group");
    assert_eq!(
        loaded_level_group.effect,
        SpecialRoutineEffect::BattleTowerAction {
            action: "BATTLETOWERACTION_LOADLEVELGROUP".to_string(),
            value: "1".to_string(),
            truthy: true
        }
    );
    assert!(
        !state
            .script_runtime
            .variables
            .contains_key("_battle_tower_level_group")
    );
    assert!(!state.script_runtime.variables.contains_key("_value"));
    assert!(!state.script_runtime.variables.contains_key("_truthy"));

    state.battle_tower.reward_item = "HP_UP".to_string();
    state.script_runtime.script_value = Some("BATTLETOWERACTION_SAVEOPTIONS".to_string());
    let save_options = apply_special_routine(&mut state, &move_catalog, "BattleTowerAction")
        .expect("save options");
    assert_eq!(
        save_options.effect,
        SpecialRoutineEffect::BattleTowerAction {
            action: "BATTLETOWERACTION_SAVEOPTIONS".to_string(),
            value: "1".to_string(),
            truthy: true
        }
    );
    assert_eq!(state.battle_tower.reward_item, "HP_UP");

    state
        .script_runtime
        .variables
        .insert("_battle_result".to_string(), "0".to_string());
    // Opponent loading has already written the in-progress state and current
    // opponent number to SRAM before BattleTowerBattle can return a result.
    state.battle_tower.challenge_state = 2;
    state.battle_tower.beaten_trainers = 1;
    let battle = apply_special_routine_with_context(
        &mut state,
        full_context_with_battle_tower_rules(&move_catalog, &battle_tower_rules),
        "BattleTowerBattle",
    )
    .expect("battle");
    assert_eq!(
        battle.effect,
        SpecialRoutineEffect::BattleTowerBattle {
            result_code: 0,
            beaten_trainers: 1,
            challenge_state: 2
        }
    );
    assert_eq!(
        state.script_runtime.script_value.as_deref(),
        Some("0"),
        "BattleTowerBattle must leave wScriptVar equal to the source battle result so `ifnotequal $0` keeps a win in the room loop"
    );
    assert_eq!(
        state
            .script_runtime
            .memory
            .get("wNrOfBeatenBattleTowerTrainers")
            .map(String::as_str),
        Some("1"),
        "the beaten count must mirror WRAM where the room script's readmem command reads it"
    );

    state.script_runtime.script_value = Some("BATTLETOWERACTION_1D".to_string());
    let reward = apply_special_routine_with_context(
        &mut state,
        full_context_with_battle_tower_rules(&move_catalog, &battle_tower_rules),
        "BattleTowerAction",
    )
    .expect("reward");
    assert_eq!(
        reward.effect,
        SpecialRoutineEffect::BattleTowerAction {
            action: "BATTLETOWERACTION_1D".to_string(),
            value: "1".to_string(),
            truthy: true
        }
    );
    assert_eq!(state.battle_tower.record_streaks, vec![1]);
    assert_eq!(state.battle_tower.record_days, vec![9]);

    let menu = apply_special_routine_with_context(
        &mut state,
        full_context_with_battle_tower_rules(&move_catalog, &battle_tower_rules),
        "BattleTowerRoomMenu",
    )
    .expect("menu");
    assert_eq!(
        menu.effect,
        SpecialRoutineEffect::BattleTowerRoomMenu {
            level_groups: vec![1, 2, 3, 4],
            selection: None,
            rejection: None,
            cancelled: false,
        }
    );
    assert_eq!(
        state.script_runtime.active_menu.as_deref(),
        Some("BattleTowerRoomMenu")
    );
    state.script_runtime.variables.insert(
        "battle_tower_mon_history".to_string(),
        "0:0;0:1".to_string(),
    );
    apply_special_routine(&mut state, &move_catalog, "Function170114")
        .expect("initialize Battle Tower challenge RAM");
    assert!(
        !state
            .script_runtime
            .variables
            .contains_key("battle_tower_mon_history")
    );
    state
        .validate_saved_state()
        .expect("Battle Tower room menu leaves save-valid state");
}

#[test]
fn battle_tower_reward_selection_and_capacity_follow_the_source_actions() {
    let move_catalog = moves();
    let rules = battle_tower_rules_with_banned_species(vec![]);
    let mut state = GameState::default();
    state.script_runtime.script_value = Some("BATTLETOWERACTION_CHOOSEREWARD".to_string());
    let mut divider = ReplayDivider::new(divider_trace_for_sub_values([4, 1]));
    let chosen = apply_random_special_routine_with_context(
        &mut state,
        full_context_with_battle_tower_rules(&move_catalog, &rules),
        "BattleTowerAction",
        &mut divider,
    )
    .expect("retry excluded Lucky Punch and choose Protein");
    assert_eq!(divider.consumed(), 4);
    assert_eq!(state.battle_tower.reward_item, "PROTEIN");
    assert!(
        !state
            .script_runtime
            .variables
            .contains_key("battle_tower_reward_item")
    );
    assert_eq!(
        chosen.effect,
        SpecialRoutineEffect::BattleTowerAction {
            action: "BATTLETOWERACTION_CHOOSEREWARD".to_string(),
            value: "BATTLETOWERACTION_CHOOSEREWARD".to_string(),
            truthy: true,
        }
    );

    let item_catalog = BTreeMap::from([("PROTEIN".to_string(), item_data("PROTEIN"))]);
    for index in 0..crate::models::ITEM_POCKET_CAPACITY {
        state.bag.items.insert(format!("ITEM_{index}"), 1);
    }
    let mut context = full_context_with_battle_tower_rules(&move_catalog, &rules);
    context.item_catalog = &item_catalog;
    state.script_runtime.script_value = Some("BATTLETOWERACTION_GIVEREWARD".to_string());
    let full = apply_special_routine_with_context(&mut state, context, "BattleTowerAction")
        .expect("check full item pocket");
    assert!(matches!(
        full.effect,
        SpecialRoutineEffect::BattleTowerAction { ref value, truthy: false, .. }
            if value == "POTION"
    ));
    assert_eq!(state.bag.quantity(&item_catalog["PROTEIN"]), 0);

    state.bag.items.remove("ITEM_0");
    state.bag.items.insert("PROTEIN".to_string(), 94);
    state.script_runtime.script_value = Some("BATTLETOWERACTION_GIVEREWARD".to_string());
    let stacking = apply_special_routine_with_context(&mut state, context, "BattleTowerAction")
        .expect("existing reward stack has room for five");
    assert!(matches!(
        stacking.effect,
        SpecialRoutineEffect::BattleTowerAction { ref value, truthy: true, .. }
            if value == "PROTEIN"
    ));
    assert_eq!(state.bag.quantity(&item_catalog["PROTEIN"]), 94);
}

#[test]
fn battle_tower_save_actions_persist_existing_state_without_synthetic_inputs() {
    let mut missing_action = GameState::default();
    missing_action.battle_tower.save_file_flags = 0x55;
    let before_missing_action = missing_action.clone();
    let missing_action_error =
        apply_special_routine(&mut missing_action, &moves(), "BattleTowerAction")
            .expect_err("missing battle tower action rejected");
    assert!(matches!(
        missing_action_error,
        SpecialRoutineError::MissingScriptValue { routine, variable }
            if routine == "BattleTowerAction" && variable == "_value"
    ));
    assert_eq!(missing_action, before_missing_action);

    for blank_action in ["", "   ", "; BATTLETOWER COMMENT"] {
        let mut state = GameState::default();
        state.battle_tower.save_file_flags = 0x55;
        state
            .script_runtime
            .variables
            .insert("_value".to_string(), blank_action.to_string());
        let before = state.clone();

        let error = apply_special_routine(&mut state, &moves(), "BattleTowerAction")
            .expect_err("blank battle tower action rejected");

        assert!(matches!(
            error,
            SpecialRoutineError::MissingScriptValue { routine, variable }
                if routine == "BattleTowerAction" && variable == "_value action token"
        ));
        assert_eq!(state, before);
    }

    for padded_action in [
        " BATTLETOWERACTION_SET_EXPLANATION_READ",
        "BATTLETOWERACTION_SET_EXPLANATION_READ ",
        "BATTLETOWERACTION_SET_EXPLANATION_READ ; comment",
    ] {
        let mut state = GameState::default();
        state.battle_tower.save_file_flags = 0x55;
        state
            .script_runtime
            .variables
            .insert("_value".to_string(), padded_action.to_string());
        let before = state.clone();

        let error = apply_special_routine(&mut state, &moves(), "BattleTowerAction")
            .expect_err("padded battle tower action rejected");

        assert_eq!(
            error,
            SpecialRoutineError::UnhandledBattleTowerAction {
                routine: "BattleTowerAction".to_string(),
                action: padded_action.to_string(),
            }
        );
        assert_eq!(state, before);
    }

    for action_token in [
        "BATTLETOWERACTION_SAVELEVELGROUP",
        "BATTLETOWERACTION_SAVEOPTIONS",
    ] {
        let mut state = GameState::default();
        state.battle_tower.level_group = 3;
        state.battle_tower.reward_item = "CARBOS".to_string();
        state
            .script_runtime
            .variables
            .insert("_value".to_string(), action_token.to_string());
        let outcome = apply_special_routine(&mut state, &moves(), "BattleTowerAction")
            .expect("source save action uses already-selected durable state");

        assert!(matches!(
            outcome.effect,
            SpecialRoutineEffect::BattleTowerAction {
                ref action,
                ref value,
                truthy: true,
            } if action == action_token && value == "1"
        ));
        assert_eq!(state.battle_tower.level_group, 3);
        assert_eq!(state.battle_tower.reward_item, "CARBOS");
        assert!(
            !state
                .script_runtime
                .variables
                .contains_key("_battle_tower_level_group")
        );
        assert!(
            !state
                .script_runtime
                .variables
                .contains_key("_selected_reward")
        );
    }
}

#[test]
fn battle_tower_action_reads_the_live_script_accumulator_not_a_stale_host_value() {
    let mut state = GameState::default();
    state.script_runtime.script_value = Some("BATTLETOWERACTION_GET_CHALLENGE_STATE".to_string());
    state.script_runtime.variables.insert(
        "_value".to_string(),
        "BATTLETOWERACTION_SET_EXPLANATION_READ".to_string(),
    );

    let outcome = apply_special_routine(&mut state, &moves(), "BattleTowerAction")
        .expect("read current wScriptVar action");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::BattleTowerAction {
            action: "BATTLETOWERACTION_GET_CHALLENGE_STATE".to_string(),
            value: "0".to_string(),
            truthy: false,
        }
    );
    assert!(!state.battle_tower.explanation_read);
}

#[test]
fn battle_tower_level_check_rejects_saved_level_group_without_clamp_fallback() {
    let move_catalog = moves();
    let rules = battle_tower_rules_with_banned_species(vec![]);
    let mut empty_party = GameState::default();
    empty_party.battle_tower.level_group = rules.minimum_level_group;
    empty_party.script_runtime.variables.insert(
        "_value".to_string(),
        "BATTLETOWERACTION_LEVEL_CHECK".to_string(),
    );
    let before_empty_party = empty_party.clone();

    let empty_party_error = apply_special_routine_with_context(
        &mut empty_party,
        full_context_with_battle_tower_rules(&move_catalog, &rules),
        "BattleTowerAction",
    )
    .expect_err("empty party must not pass as level zero");
    assert!(matches!(
        empty_party_error,
        SpecialRoutineError::EmptyParty { routine } if routine == "BattleTowerAction"
    ));
    assert_eq!(empty_party, before_empty_party);

    for level_group in [0, rules.maximum_level_group + 1] {
        let mut state = GameState::default();
        state.battle_tower.level_group = level_group;
        state
            .storage
            .register_capture_in_box(0, pokemon("CHIKORITA"))
            .expect("party capture");
        state.sync_party_from_storage();
        state.script_runtime.variables.insert(
            "_value".to_string(),
            "BATTLETOWERACTION_LEVEL_CHECK".to_string(),
        );
        let before = state.clone();

        let error = apply_special_routine_with_context(
            &mut state,
            full_context_with_battle_tower_rules(&move_catalog, &rules),
            "BattleTowerAction",
        )
        .expect_err("saved level group must be exact pack-owned state");

        assert!(matches!(
            error,
            SpecialRoutineError::InvalidBattleTowerLevelGroup {
                routine,
                level_group: rejected,
                minimum,
                maximum,
            } if routine == "BattleTowerAction"
                && rejected == level_group
                && minimum == rules.minimum_level_group
                && maximum == rules.maximum_level_group
        ));
        assert_eq!(state, before);
    }
}

#[test]
fn battle_tower_ubers_check_requires_exact_modpack_rules_without_builtin_species() {
    let move_catalog = moves();

    let mut missing_rules = GameState::default();
    missing_rules
        .storage
        .register_capture_in_box(0, pokemon("MEWTWO"))
        .expect("party capture");
    missing_rules.sync_party_from_storage();
    missing_rules.script_runtime.variables.insert(
        "_value".to_string(),
        "BATTLETOWERACTION_UBERS_CHECK".to_string(),
    );
    let before_missing_rules = missing_rules.clone();
    let missing_rules_error =
        apply_special_routine(&mut missing_rules, &move_catalog, "BattleTowerAction")
            .expect_err("ubers check requires modpack rules");
    assert!(matches!(
        missing_rules_error,
        SpecialRoutineError::MissingBattleTowerRules { routine }
            if routine == "BattleTowerAction"
    ));
    assert_eq!(missing_rules, before_missing_rules);

    let exact_rules = battle_tower_rules_with_banned_species(vec!["MEWTWO".to_string()]);
    let mut exact_match = before_missing_rules.clone();
    let exact_outcome = apply_special_routine_with_context(
        &mut exact_match,
        full_context_with_battle_tower_rules(&move_catalog, &exact_rules),
        "BattleTowerAction",
    )
    .expect("exact banned species rule applies");
    assert_eq!(
        exact_outcome.effect,
        SpecialRoutineEffect::BattleTowerAction {
            action: "BATTLETOWERACTION_UBERS_CHECK".to_string(),
            value: "1".to_string(),
            truthy: true
        }
    );

    let lowercase_rules = battle_tower_rules_with_banned_species(vec!["mewtwo".to_string()]);
    let mut lowercase_mismatch = before_missing_rules;
    let lowercase_outcome = apply_special_routine_with_context(
        &mut lowercase_mismatch,
        full_context_with_battle_tower_rules(&move_catalog, &lowercase_rules),
        "BattleTowerAction",
    )
    .expect("lowercase modpack id is not coerced");
    assert_eq!(
        lowercase_outcome.effect,
        SpecialRoutineEffect::BattleTowerAction {
            action: "BATTLETOWERACTION_UBERS_CHECK".to_string(),
            value: "0".to_string(),
            truthy: false
        }
    );
}

#[test]
fn battle_tower_rule_check_requires_pack_rules_and_uses_exact_failure_text() {
    let move_catalog = moves();
    let mut missing_rules = GameState::default();
    missing_rules
        .storage
        .register_capture_in_box(0, pokemon("CHIKORITA"))
        .expect("first");
    missing_rules.sync_party_from_storage();
    let before_missing_rules = missing_rules.clone();

    let error = apply_special_routine(
        &mut missing_rules,
        &move_catalog,
        "CheckForBattleTowerRules",
    )
    .expect_err("battle tower rules are pack owned");

    assert!(matches!(
        error,
        SpecialRoutineError::MissingBattleTowerRules { routine }
            if routine == "CheckForBattleTowerRules"
    ));
    assert_eq!(missing_rules, before_missing_rules);

    let mut exact_rules = battle_tower_rules_with_banned_species(vec![]);
    exact_rules.required_party_count = 2;
    exact_rules.party_count_failure_text = "CUSTOM_BATTLE_TOWER_PARTY_COUNT".to_string();
    let mut custom_failure = before_missing_rules;
    let outcome = apply_special_routine_with_context(
        &mut custom_failure,
        full_context_with_battle_tower_rules(&move_catalog, &exact_rules),
        "CheckForBattleTowerRules",
    )
    .expect("custom rules apply");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::CheckForBattleTowerRules {
            failures: vec!["CUSTOM_BATTLE_TOWER_PARTY_COUNT".to_string()]
        }
    );
}

#[test]
fn battle_tower_challenge_menu_returns_the_source_choice_without_mutating_save_flags() {
    let mut state = GameState::default();
    state.battle_tower.explanation_read = true;
    state.script_runtime.script_value = Some("1".to_string());
    state.script_runtime.variables.insert(
        "_battle_tower_challenge_choice".to_string(),
        "2".to_string(),
    );

    let outcome = apply_special_routine(&mut state, &moves(), "Menu_ChallengeExplanationCancel")
        .expect("choose Battle Tower explanation");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::BattleTowerChallengeExplanationCancel {
            english: true,
            selection: Some(2),
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("2"));
    assert!(state.battle_tower.explanation_read);
}

#[test]
fn battle_tower_rule_check_reports_every_failed_source_check_in_order() {
    let mut state = GameState::default();
    state
        .storage
        .register_capture_in_box(0, pokemon("CHIKORITA"))
        .expect("first duplicate");
    state
        .storage
        .register_capture_in_box(0, pokemon("CHIKORITA"))
        .expect("second duplicate");
    state.sync_party_from_storage();
    let move_catalog = moves();
    let rules = battle_tower_rules_with_banned_species(vec![]);

    let outcome = apply_special_routine_with_context(
        &mut state,
        full_context_with_battle_tower_rules(&move_catalog, &rules),
        "CheckForBattleTowerRules",
    )
    .expect("check every Battle Tower rule");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::CheckForBattleTowerRules {
            failures: vec![
                "OnlyThreeMonMayBeEnteredText".to_string(),
                "TheMonMustAllBeDifferentKindsText".to_string(),
            ],
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));
}

#[test]
fn battle_tower_room_menu_commits_the_selected_level_group() {
    let mut state = GameState::default();
    for species in ["CHIKORITA", "CYNDAQUIL", "TOTODILE"] {
        state
            .storage
            .register_capture_in_box(0, pokemon(species))
            .expect("register Battle Tower party member");
    }
    state.sync_party_from_storage();
    state.hall_of_fame.count = 1;
    state
        .flags
        .engine_flags
        .insert("STATUSFLAGS_HALL_OF_FAME_F".to_string(), true);
    state
        .script_runtime
        .variables
        .insert("_battle_tower_room_selection".to_string(), "2".to_string());
    let move_catalog = moves();
    let rules = battle_tower_rules_with_banned_species(vec![]);

    let outcome = apply_special_routine_with_context(
        &mut state,
        full_context_with_battle_tower_rules(&move_catalog, &rules),
        "BattleTowerRoomMenu",
    )
    .expect("select the level-20 Battle Room");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::BattleTowerRoomMenu {
            level_groups: (1..=10).collect(),
            selection: Some(2),
            rejection: None,
            cancelled: false,
        }
    );
    assert_eq!(state.battle_tower.level_group, 2);
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));
    assert!(state.script_runtime.active_menu.is_none());
}

#[test]
fn battle_tower_room_menu_rejects_source_level_and_uber_mismatches_and_cancels_with_ten() {
    let move_catalog = moves();
    let rules = battle_tower_rules_with_banned_species(vec!["CHIKORITA".to_string()]);

    let mut over_level = GameState::default();
    let mut level_30 = pokemon("TOTODILE");
    level_30.level = 30;
    over_level
        .storage
        .register_capture_in_box(0, level_30)
        .expect("over-level party member");
    over_level.sync_party_from_storage();
    over_level
        .script_runtime
        .variables
        .insert("_battle_tower_room_selection".to_string(), "2".to_string());
    let rejected = apply_special_routine_with_context(
        &mut over_level,
        full_context_with_battle_tower_rules(&move_catalog, &rules),
        "BattleTowerRoomMenu",
    )
    .expect("reject over-level party");
    assert!(matches!(
        rejected.effect,
        SpecialRoutineEffect::BattleTowerRoomMenu {
            rejection: Some(BattleTowerRoomMenuRejection::PartyMonTopsThisLevel),
            ..
        }
    ));
    assert_eq!(over_level.battle_tower.level_group, 0);
    assert!(over_level.script_runtime.active_menu.is_some());

    let mut uber = GameState::default();
    uber.storage
        .register_capture_in_box(0, pokemon("CHIKORITA"))
        .expect("restricted party member");
    uber.sync_party_from_storage();
    uber.script_runtime
        .variables
        .insert("_battle_tower_room_selection".to_string(), "1".to_string());
    let rejected = apply_special_routine_with_context(
        &mut uber,
        full_context_with_battle_tower_rules(&move_catalog, &rules),
        "BattleTowerRoomMenu",
    )
    .expect("reject restricted species below level 70 room");
    assert!(matches!(
        rejected.effect,
        SpecialRoutineEffect::BattleTowerRoomMenu {
            rejection: Some(BattleTowerRoomMenuRejection::UberRestriction { ref species }),
            ..
        } if species == "CHIKORITA"
    ));

    let mut cancelled = GameState::default();
    cancelled
        .script_runtime
        .variables
        .insert("_battle_tower_room_cancelled".to_string(), "1".to_string());
    let outcome = apply_special_routine_with_context(
        &mut cancelled,
        full_context_with_battle_tower_rules(&move_catalog, &rules),
        "BattleTowerRoomMenu",
    )
    .expect("confirm Battle Tower room cancellation");
    assert!(matches!(
        outcome.effect,
        SpecialRoutineEffect::BattleTowerRoomMenu {
            cancelled: true,
            ..
        }
    ));
    assert_eq!(cancelled.script_runtime.script_value.as_deref(), Some("10"));
    assert!(cancelled.script_runtime.active_menu.is_none());
}

#[test]
fn battle_tower_rejects_invalid_parties_and_mobile_prompt_is_explicit() {
    let mut state = GameState::default();
    state
        .storage
        .register_capture_in_box(0, pokemon("CHIKORITA"))
        .expect("first");
    state
        .storage
        .register_capture_in_box(0, pokemon("CHIKORITA"))
        .expect("duplicate");
    state
        .storage
        .register_capture_in_box(0, pokemon("TOTODILE"))
        .expect("third");
    state.sync_party_from_storage();

    let move_catalog = moves();
    let battle_tower_rules = battle_tower_rules_with_banned_species(vec![]);
    let rules = apply_special_routine_with_context(
        &mut state,
        full_context_with_battle_tower_rules(&move_catalog, &battle_tower_rules),
        "CheckForBattleTowerRules",
    )
    .expect("rules");
    assert_eq!(
        rules.effect,
        SpecialRoutineEffect::CheckForBattleTowerRules {
            failures: vec!["TheMonMustAllBeDifferentKindsText".to_string()]
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));

    let mobile =
        apply_special_routine(&mut state, &moves(), "BattleTowerMobileError").expect("mobile");
    assert_eq!(mobile.effect, SpecialRoutineEffect::BattleTowerMobileError);
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));

    state
        .script_runtime
        .variables
        .insert("_yes_no_result".to_string(), "0".to_string());
    let remember =
        apply_special_routine(&mut state, &moves(), "AskRememberPassword").expect("remember");
    assert_eq!(
        remember.effect,
        SpecialRoutineEffect::AskRememberPassword { remember: false }
    );
    assert!(
        !state
            .script_runtime
            .variables
            .contains_key("_remember_password")
    );
    assert!(
        !state
            .script_runtime
            .variables
            .contains_key("_yes_no_result")
    );
}

#[test]
fn mobile_handshakes_export_leaderboard_and_session_state() {
    let mut state = GameState::default();
    state.battle_tower.record_streaks = vec![7, 3];
    state.battle_tower.record_outcomes = vec![true, false];
    state.battle_tower.record_days = vec![4, 5];
    state.script_runtime.variables.insert(
        "_mobile_login_password".to_string(),
        "SEVENTEEN-CHARS!!".to_string(),
    );
    state
        .script_runtime
        .variables
        .insert("_mobile_battle_timer".to_string(), "1,2,3".to_string());
    state
        .script_runtime
        .variables
        .insert("_mobile_adapter_status".to_string(), "ready".to_string());
    state.script_runtime.variables.insert(
        "_mobile_adapter_secondary_status".to_string(),
        "standby".to_string(),
    );

    let init = apply_special_routine(&mut state, &moves(), "Function1011f1").expect("mobile init");
    assert_eq!(
        init.effect,
        SpecialRoutineEffect::MobileHandshake {
            routine: "Function1011f1".to_string(),
            mode: "init".to_string(),
            link_mode: 4,
            serial_status: LinkSerialConnectionStatus::NotEstablished,
            handshakes: 1
        }
    );
    assert_eq!(state.mobile_link.mode.as_deref(), Some("init"));
    assert_eq!(state.mobile_link.battle_timer, [1, 2, 3]);
    assert_eq!(state.mobile_link.login_password, "SEVENTEEN-CHARS!!");
    assert_eq!(state.mobile_link.leaderboard.len(), 2);
    assert!(
        !state
            .script_runtime
            .variables
            .contains_key("mobile_handshakes")
    );
    assert!(
        !state
            .script_runtime
            .variables
            .contains_key("battle_tower_leaderboard_count")
    );

    let battle =
        apply_special_routine(&mut state, &moves(), "Function101225").expect("mobile battle");
    assert_eq!(
        battle.effect,
        SpecialRoutineEffect::MobileHandshake {
            routine: "Function101225".to_string(),
            mode: "battle".to_string(),
            link_mode: 4,
            serial_status: LinkSerialConnectionStatus::UsingExternalClock,
            handshakes: 2
        }
    );
    assert_eq!(
        state.link_session.serial_connection_status,
        LinkSerialConnectionStatus::UsingExternalClock
    );

    let leaderboard =
        apply_special_routine(&mut state, &moves(), "Function1700ba").expect("leaderboard");
    assert_eq!(
        leaderboard.effect,
        SpecialRoutineEffect::BattleTowerLeaderboard {
            records: vec![
                MobileBattleTowerRecord {
                    streak: 7,
                    outcome: "win".to_string(),
                    day: 4
                },
                MobileBattleTowerRecord {
                    streak: 3,
                    outcome: "loss".to_string(),
                    day: 5
                }
            ],
            acknowledged: true
        }
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));
    assert!(!state.script_runtime.variables.contains_key("_value"));

    let end = apply_special_routine(&mut state, &moves(), "Function101220").expect("mobile end");
    assert_eq!(end.effect, SpecialRoutineEffect::MobileSessionEnded);
    assert!(state.mobile_link.terminated);
    assert_eq!(state.link_session.link_mode, 0);
}

#[test]
fn mobile_flags_and_party_selection_are_explicit_state() {
    let mut state = GameState::default();
    state
        .script_runtime
        .variables
        .insert("_selected_party_indexes".to_string(), "2,4,5".to_string());

    let selected =
        apply_special_routine(&mut state, &moves(), "Mobile_SelectThreeMons").expect("select mons");
    assert_eq!(
        selected.effect,
        SpecialRoutineEffect::MobileSelectThreeMons {
            indexes: vec![2, 4, 5]
        }
    );
    assert_eq!(state.battle_tower.selected_party_indexes, vec![2, 4, 5]);

    for (routine, flag) in [
        ("Function103780", "function103780"),
        ("Function1037c2", "function1037c2"),
        ("Function1037eb", "function1037eb"),
        ("Function10383c", "function10383c"),
        ("Function10387b", "function10387b"),
    ] {
        let outcome = apply_special_routine(&mut state, &moves(), routine).expect("flag");
        assert_eq!(
            outcome.effect,
            SpecialRoutineEffect::BattleTowerMobileFlag {
                flag: flag.to_string()
            }
        );
        assert!(state.battle_tower.mobile_flags.contains(flag));
    }
}

#[test]
fn mobile_handshake_and_party_selection_require_exact_inputs_without_defaults() {
    let handshake_cases = [
        "_mobile_login_password",
        "_mobile_battle_timer",
        "_mobile_adapter_status",
        "_mobile_adapter_secondary_status",
    ];

    for missing_variable in handshake_cases {
        let mut state = GameState::default();
        state.script_runtime.variables.insert(
            "_mobile_login_password".to_string(),
            "SEVENTEEN-CHARS!!".to_string(),
        );
        state
            .script_runtime
            .variables
            .insert("_mobile_battle_timer".to_string(), "1,2,3".to_string());
        state
            .script_runtime
            .variables
            .insert("_mobile_adapter_status".to_string(), "ready".to_string());
        state.script_runtime.variables.insert(
            "_mobile_adapter_secondary_status".to_string(),
            "standby".to_string(),
        );
        state.script_runtime.variables.remove(missing_variable);
        let before = state.clone();

        let error = apply_special_routine(&mut state, &moves(), "Function1011f1")
            .expect_err("missing mobile handshake input rejected");

        assert!(matches!(
            error,
            SpecialRoutineError::MissingScriptValue { routine, variable }
                if routine == "Function1011f1" && variable == missing_variable
        ));
        assert_eq!(state, before);
    }

    let mut aliased_timer = GameState::default();
    aliased_timer.script_runtime.variables.insert(
        "_mobile_login_password".to_string(),
        "SEVENTEEN-CHARS!!".to_string(),
    );
    aliased_timer
        .script_runtime
        .variables
        .insert("_mobile_battle_timer".to_string(), "+1,2,3".to_string());
    aliased_timer
        .script_runtime
        .variables
        .insert("_mobile_adapter_status".to_string(), "ready".to_string());
    aliased_timer.script_runtime.variables.insert(
        "_mobile_adapter_secondary_status".to_string(),
        "standby".to_string(),
    );
    let before_aliased_timer = aliased_timer.clone();
    let timer_error = apply_special_routine(&mut aliased_timer, &moves(), "Function1011f1")
        .expect_err("aliased mobile timer rejected");
    assert!(matches!(
        timer_error,
        SpecialRoutineError::InvalidNumericValue { routine, value }
            if routine == "Function1011f1" && value == "+1"
    ));
    assert_eq!(aliased_timer, before_aliased_timer);

    let mut padded_password = GameState::default();
    padded_password.script_runtime.variables.insert(
        "_mobile_login_password".to_string(),
        " SEVENTEEN-CHARS!".to_string(),
    );
    padded_password
        .script_runtime
        .variables
        .insert("_mobile_battle_timer".to_string(), "1,2,3".to_string());
    padded_password
        .script_runtime
        .variables
        .insert("_mobile_adapter_status".to_string(), "ready".to_string());
    padded_password.script_runtime.variables.insert(
        "_mobile_adapter_secondary_status".to_string(),
        "standby".to_string(),
    );
    let before_padded_password = padded_password.clone();
    let password_error = apply_special_routine(&mut padded_password, &moves(), "Function1011f1")
        .expect_err("padded mobile password rejected");
    assert!(matches!(
        password_error,
        SpecialRoutineError::InvalidMobilePassword { routine }
            if routine == "Function1011f1"
    ));
    assert_eq!(padded_password, before_padded_password);

    let mut selection = GameState::default();
    selection.battle_tower.selected_party_indexes = vec![9, 8, 7];
    let before_selection = selection.clone();
    let selection_error = apply_special_routine(&mut selection, &moves(), "Mobile_SelectThreeMons")
        .expect_err("missing mobile party selection rejected");
    assert!(matches!(
        selection_error,
        SpecialRoutineError::MissingScriptValue { routine, variable }
            if routine == "Mobile_SelectThreeMons" && variable == "_selected_party_indexes"
    ));
    assert_eq!(selection, before_selection);

    selection
        .script_runtime
        .variables
        .insert("_selected_party_indexes".to_string(), "2,+4,5".to_string());
    let before_aliased_selection = selection.clone();
    let selection_alias_error =
        apply_special_routine(&mut selection, &moves(), "Mobile_SelectThreeMons")
            .expect_err("aliased selected party indexes rejected");
    assert!(matches!(
        selection_alias_error,
        SpecialRoutineError::InvalidNumericValue { routine, value }
            if routine == "Mobile_SelectThreeMons" && value == "2,+4,5"
    ));
    assert_eq!(selection, before_aliased_selection);
}

#[test]
fn give_odd_egg_uses_exact_table_entry_and_modpack_move_pp() {
    let mut state = GameState::default();
    let mut species = species_catalog(&[("CLEFFA", 173), ("PICHU", 172)]);
    species
        .get_mut("CLEFFA")
        .expect("cleffa")
        .step_cycles_to_hatch = 99;
    let learnsets = [
        ("CLEFFA".to_string(), Vec::new()),
        ("PICHU".to_string(), Vec::new()),
    ]
    .into_iter()
    .collect();
    let moves = [
        ("POUND".to_string(), move_data("POUND", 35)),
        ("THUNDERSHOCK".to_string(), move_data("THUNDERSHOCK", 20)),
        ("CHARM".to_string(), move_data("CHARM", 20)),
        ("DIZZY_PUNCH".to_string(), move_data("DIZZY_PUNCH", 10)),
    ]
    .into_iter()
    .collect::<BTreeMap<_, _>>();
    let odd_egg_definitions = test_odd_egg_definitions();
    let mut divider = ReplayDivider::new(divider_trace_for_sub_values([0x40]));

    let outcome = apply_random_special_routine_with_context(
        &mut state,
        full_context_with_odd_egg_definitions(&moves, &species, &learnsets, &odd_egg_definitions),
        "GiveOddEgg",
        &mut divider,
    )
    .expect("odd egg");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::GiveOddEgg {
            table_index: 3,
            species: "CLEFFA".to_string(),
            party_slot: 0,
            shiny: true,
            random_state_after: CrystalRandomState { add: 0, sub: 0x40 }
        }
    );
    assert_eq!(divider.consumed(), 2);
    let egg = state.storage.party.pokemon[0].as_ref().expect("egg");
    assert_eq!(egg.species.id, "CLEFFA");
    assert_eq!(egg.nickname, "EGG");
    assert_eq!(egg.level, 5);
    assert_eq!(egg.hp, 0);
    assert_eq!(egg.happiness, 20);
    assert_eq!(egg.experience, 125);
    assert_eq!(egg.original_trainer_name, "ODD");
    assert_eq!(egg.original_trainer_id, 768);
    assert_eq!(egg.dvs, Dv::from_non_hp(2, 10, 10, 10));
    assert_eq!(
        egg.moves,
        vec![
            LearnedMove {
                name: "POUND".to_string(),
                current_pp: 35,
                pp_ups: 0
            },
            LearnedMove {
                name: "CHARM".to_string(),
                current_pp: 20,
                pp_ups: 0
            },
            LearnedMove {
                name: "DIZZY_PUNCH".to_string(),
                current_pp: 10,
                pp_ups: 0
            },
        ]
    );
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("wCurPartySpecies")
            .map(String::as_str),
        Some("CLEFFA")
    );
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("wCurPartyMon")
            .map(String::as_str),
        Some("0")
    );
}

#[test]
fn give_odd_egg_requires_modpack_definitions_without_builtin_table_fallback() {
    let mut state = GameState::default();
    let species = species_catalog(&[("CLEFFA", 173), ("PICHU", 172)]);
    let learnsets = [
        ("CLEFFA".to_string(), Vec::new()),
        ("PICHU".to_string(), Vec::new()),
    ]
    .into_iter()
    .collect();
    let moves = [
        ("POUND".to_string(), move_data("POUND", 35)),
        ("THUNDERSHOCK".to_string(), move_data("THUNDERSHOCK", 20)),
        ("CHARM".to_string(), move_data("CHARM", 20)),
        ("DIZZY_PUNCH".to_string(), move_data("DIZZY_PUNCH", 10)),
    ]
    .into_iter()
    .collect::<BTreeMap<_, _>>();
    let before = state.clone();

    let mut divider = ReplayDivider::new([]);
    let error = apply_random_special_routine_with_context(
        &mut state,
        full_context(&moves, &species, &learnsets, &BTreeMap::new()),
        "GiveOddEgg",
        &mut divider,
    )
    .expect_err("missing Odd Egg definitions reject");

    assert_eq!(
        error,
        RandomSpecialRoutineError::Routine(SpecialRoutineError::MissingOddEggDefinitions {
            routine: "GiveOddEgg".to_string()
        })
    );
    assert_eq!(state, before);
}

#[test]
fn give_odd_egg_rejects_full_party_without_pc_fallback() {
    let mut state = GameState::default();
    for _ in 0..6 {
        assert!(state.storage.party.add_pokemon(pokemon("CHIKORITA")));
    }
    state.storage.pc_boxes = vec![PcBox::new(0), PcBox::new(1)];
    state.sync_party_from_storage();
    let species = species_catalog(&[("CLEFFA", 173), ("PICHU", 172)]);
    let learnsets = [
        ("CLEFFA".to_string(), Vec::new()),
        ("PICHU".to_string(), Vec::new()),
    ]
    .into_iter()
    .collect();
    let moves = [
        ("POUND".to_string(), move_data("POUND", 35)),
        ("THUNDERSHOCK".to_string(), move_data("THUNDERSHOCK", 20)),
        ("CHARM".to_string(), move_data("CHARM", 20)),
        ("DIZZY_PUNCH".to_string(), move_data("DIZZY_PUNCH", 10)),
    ]
    .into_iter()
    .collect::<BTreeMap<_, _>>();
    let odd_egg_definitions = test_odd_egg_definitions();

    let before_random_state = state.random_state;
    let mut divider = ReplayDivider::new([1, 2]);
    let error = apply_random_special_routine_with_context(
        &mut state,
        full_context_with_odd_egg_definitions(&moves, &species, &learnsets, &odd_egg_definitions),
        "GiveOddEgg",
        &mut divider,
    )
    .expect_err("full party rejects odd egg");

    assert_eq!(
        error,
        RandomSpecialRoutineError::Routine(SpecialRoutineError::GiftStorageFull {
            routine: "GiveOddEgg".to_string(),
            species: "PICHU".to_string()
        })
    );
    assert_eq!(divider.consumed(), 0);
    assert_eq!(state.random_state, before_random_state);
    assert_eq!(state.storage.pc_boxes[0].filled_slots(), 0);
    assert_eq!(state.storage.pc_boxes[1].filled_slots(), 0);
}

#[test]
fn warp_to_spawn_point_only_clears_runtime_status_bits() {
    let mut state = GameState::default();
    state.last_spawn_map_constant = Some("GOLDENROD_CITY".to_string());
    state.overworld = OverworldMemory::Active {
        map_name: "Route29".to_string(),
        tile: TilePosition::new(7, 4),
        facing: Direction::Left,
        mode: MovementMode::Bike,
    };
    state.bug_contest.timer_active = true;
    state.bug_contest.timer_start_time = Some(current_bug_contest_time(&state));
    state
        .flags
        .set_engine_flag("ENGINE_BUG_CONTEST_TIMER", true)
        .expect("Bug Contest timer flag");
    state
        .flags
        .set_engine_flag("ENGINE_SAFARI_ZONE", true)
        .expect("Safari Game flag");
    state.script_runtime.script_value = Some("73".to_string());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "73".to_string());
    state.script_runtime.pending_script_warp = Some(ScriptWarpRequest {
        target_map: "GoldenrodPokecenter1F".to_string(),
        tile: TilePosition::new(6, 4),
        facing: None,
        source_script: "FlyScript".to_string(),
        command_index: 4,
    });
    let before_overworld = state.overworld.clone();
    let before_pending_warp = state.script_runtime.pending_script_warp.clone();

    let outcome = apply_special_routine(&mut state, &moves(), "WarpToSpawnPoint")
        .expect("status cleanup does not require a spawn catalog");

    assert_eq!(outcome.routine, "WarpToSpawnPoint");
    assert!(!state.bug_contest.timer_active);
    assert_eq!(state.bug_contest.timer_start_time, None);
    assert_eq!(
        state.flags.is_engine_flag_set("ENGINE_BUG_CONTEST_TIMER"),
        Ok(false)
    );
    assert_eq!(
        state.flags.is_engine_flag_set("ENGINE_SAFARI_ZONE"),
        Ok(false)
    );
    assert_eq!(
        state.last_spawn_map_constant.as_deref(),
        Some("GOLDENROD_CITY")
    );
    assert_eq!(state.overworld, before_overworld);
    assert_eq!(
        state.script_runtime.pending_script_warp,
        before_pending_warp
    );
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("73"));
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("_value")
            .map(String::as_str),
        Some("73")
    );
}

#[test]
fn load_battle_tower_opponent_uses_exact_pack_trainer_and_sprite() {
    let mut state = GameState::default();
    state
        .storage
        .register_capture_in_box(0, pokemon("PERSIAN"))
        .expect("store player party mon");
    state.sync_party_from_storage();
    state.script_runtime.script_value = Some("BATTLETOWERBATTLEROOM_GENTLEMAN".to_string());
    let species = species_catalog(&[("PERSIAN", 53)]);
    let learnsets = [(
        "PERSIAN".to_string(),
        vec![crate::systems::learnsets::LearnsetEntry(
            1,
            "SCRATCH".to_string(),
        )],
    )]
    .into_iter()
    .collect();
    let moves = [("SCRATCH".to_string(), move_data("SCRATCH", 35))]
        .into_iter()
        .collect::<BTreeMap<_, _>>();
    let mut rules = battle_tower_rules_with_banned_species(vec![]);
    rules.required_party_count = 1;
    rules.trainers = (0..70)
        .map(|index| BattleTowerTrainerDefinition {
            index,
            trainer_class: "GENTLEMAN".to_string(),
            name: if index == 0 {
                "EDWARD@".to_string()
            } else {
                format!("TRAINER{index}@")
            },
            sprite_constant: "SPRITE_GENTLEMAN".to_string(),
            female: false,
        })
        .collect();
    let tower_mon = BattleTowerMonDefinition {
        species: "PERSIAN".to_string(),
        item: None,
        moves: vec![
            "SCRATCH".to_string(),
            "0".to_string(),
            "NO_MOVE".to_string(),
            "NO_MOVE".to_string(),
        ],
        original_trainer_id: 1234,
        experience: 35937,
        stat_exp: vec![0; 5],
        dvs: vec![8; 4],
        pp: vec![35, 0, 0, 0],
        happiness: 70,
        pokerus: vec![0xb4, 0xa1, 0xaa],
        level: 33,
        status: vec![5, 0],
        stats: vec![91, 91, 57, 48, 88, 52, 52],
        nickname: "PERSIAN".to_string(),
    };
    rules.mon_groups = vec![vec![tower_mon; 21]];
    let catalog = TrainerCatalog::default();
    let mut context = trainer_context(&moves, &species, &learnsets, &catalog);
    context.battle_tower_rules = Some(&rules);

    let mut blocked = state.clone();
    blocked.pending_static_wild_terminal = Some(crate::state::PendingStaticWildBattleTerminal {
        origin_map_name: "ROUTE_40".to_string(),
        source_script: "RockSmashScript".to_string(),
        startbattle_command_index: 12,
        resume_command_index: 13,
        battle_type: "BATTLETYPE_NORMAL".to_string(),
        species: "SHUCKLE".to_string(),
        level: 15,
        pay_day_payout: 0,
        battle_result: 0,
        win_cleanup_applied: false,
    });
    let blocked_before = blocked.clone();
    let mut empty_divider = ReplayDivider::new([]);
    let blocked_error = apply_random_special_routine_with_context(
        &mut blocked,
        context,
        "LoadOpponentTrainerAndPokemonWithOTSprite",
        &mut empty_divider,
    )
    .expect_err("Battle Tower load cannot overwrite a pending static-wild terminal");
    assert!(matches!(
        blocked_error,
        RandomSpecialRoutineError::Routine(SpecialRoutineError::InvalidState {
            routine,
            message,
        }) if routine == "LoadOpponentTrainerAndPokemonWithOTSprite"
            && message.contains("pending static-wild terminal")
    ));
    assert_eq!(empty_divider.consumed(), 0);
    assert_eq!(blocked, blocked_before);

    let mut divider = ReplayDivider::new([0; 4]);
    let outcome = apply_random_special_routine_with_context(
        &mut state,
        context,
        "LoadOpponentTrainerAndPokemonWithOTSprite",
        &mut divider,
    )
    .expect("load opponent");

    assert_eq!(
        outcome.effect,
        SpecialRoutineEffect::LoadOpponentTrainerAndPokemonWithOtSprite {
            trainer_id: "BATTLE_TOWER_0".to_string(),
            trainer_class: "GENTLEMAN".to_string(),
            trainer_name: "EDWARD@".to_string(),
            party_size: 1,
            sprite_constant: "SPRITE_GENTLEMAN".to_string(),
            target_object: "BATTLETOWERBATTLEROOM_GENTLEMAN".to_string(),
            random_state_after: CrystalRandomState::default()
        }
    );
    assert_eq!(divider.consumed(), 4);
    assert_eq!(
        state.battle_tower.loaded_trainer_id.as_deref(),
        Some("BATTLE_TOWER_0")
    );
    for key in [
        "other_trainer_class",
        "other_trainer_id",
        "other_trainer_name",
        "other_trainer_party_count",
        "battle_tower_target_object",
    ] {
        assert!(!state.script_runtime.variables.contains_key(key), "{key}");
    }
    assert_eq!(
        state.battle_tower.challenge_state, 0,
        "opponent selection precedes CopyBTTrainer_FromBT_OT_TowBT_OTTemp"
    );
    assert_eq!(
        state.battle_tower.beaten_trainers, 0,
        "opponent selection must not increment SRAM before BattleTowerBattle starts"
    );
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("_battle_tower_opponent_pending")
            .map(String::as_str),
        Some("1")
    );
    match &state.battle {
        BattleMemory::Trainer {
            battle_type,
            trainer_class,
            trainer_id,
            trainer_name,
            enemy_party,
            encounter_music,
            ai_layers,
            ..
        } => {
            assert_eq!(battle_type, "BATTLETYPE_BATTLE_TOWER");
            assert_eq!(trainer_class, "GENTLEMAN");
            assert_eq!(trainer_id, "BATTLE_TOWER_0");
            assert_eq!(trainer_name, "EDWARD@");
            assert_eq!(enemy_party.len(), 1);
            assert_eq!(enemy_party[0].species.id, "PERSIAN");
            assert_eq!(enemy_party[0].level, 33);
            assert_eq!(enemy_party[0].pokerus, 0xb4);
            assert_eq!(enemy_party[0].status.as_deref(), Some("SLEEP"));
            assert_eq!(enemy_party[0].sleep_turns, 5);
            assert_eq!(
                enemy_party[0].caught_data,
                Some(CaughtData {
                    level: 33,
                    time_of_day: Some(TimeOfDay::Day),
                    original_trainer_gender: 1,
                    location: 42,
                })
            );
            assert_eq!(
                enemy_party[0].moves,
                vec![LearnedMove {
                    name: "SCRATCH".to_string(),
                    current_pp: 35,
                    pp_ups: 0,
                }]
            );
            assert_eq!(encounter_music, "MUSIC_BATTLE_TOWER_THEME");
            assert!(ai_layers.is_empty());
        }
        other => panic!("expected battle tower trainer battle, got {other:?}"),
    }

    let player = state.storage.party.pokemon[0]
        .as_mut()
        .expect("Battle Tower player party");
    player.hp = 1;
    let max_hp = player.max_hp;
    let started = apply_special_routine_with_context(&mut state, context, "BattleTowerBattle")
        .expect("start prepared Battle Tower battle");
    assert_eq!(
        started.effect,
        SpecialRoutineEffect::BattleTowerBattleStarted
    );
    assert_eq!(state.battle_tower.challenge_state, 2);
    assert_eq!(state.battle_tower.beaten_trainers, 1);
    assert!(
        !state
            .script_runtime
            .variables
            .contains_key("_battle_tower_opponent_pending")
    );
    assert_eq!(
        state
            .script_runtime
            .memory
            .get("wNrOfBeatenBattleTowerTrainers")
            .map(String::as_str),
        Some("1")
    );
    assert_eq!(
        state.storage.party.pokemon[0]
            .as_ref()
            .expect("healed Battle Tower party")
            .hp,
        max_hp
    );
}

#[test]
fn battle_tower_battle_and_opponent_load_require_exact_source_state() {
    let mut battle_state = GameState::default();
    battle_state.battle_result = 1;
    battle_state.battle_tower.quick_saved = true;
    battle_state.battle_tower.beaten_trainers = 3;
    let before_battle = battle_state.clone();
    let battle_error = apply_special_routine(&mut battle_state, &moves(), "BattleTowerBattle")
        .expect_err("missing loaded Battle Tower opponent rejected");
    assert!(matches!(
        battle_error,
        SpecialRoutineError::InvalidState { routine, message }
            if routine == "BattleTowerBattle"
                && message.contains("loaded Battle Tower opponent")
    ));
    assert_eq!(battle_state, before_battle);

    let mut load_state = GameState::default();
    let before_load = load_state.clone();
    let catalog = TrainerCatalog::default();
    let mut divider = ReplayDivider::new([]);
    let load_error = apply_random_special_routine_with_context(
        &mut load_state,
        trainer_context(
            &moves(),
            &BTreeMap::new(),
            &SpeciesLearnsets::new(),
            &catalog,
        ),
        "LoadOpponentTrainerAndPokemonWithOTSprite",
        &mut divider,
    )
    .expect_err("missing battle tower target object rejected");
    assert!(matches!(
        load_error,
        RandomSpecialRoutineError::Routine(SpecialRoutineError::MissingScriptValue { routine, variable })
            if routine == "LoadOpponentTrainerAndPokemonWithOTSprite"
                && variable == "_value"
    ));
    assert_eq!(load_state, before_load);
}

#[test]
fn load_battle_tower_opponent_rejects_missing_compiled_roster() {
    let mut state = GameState::default();
    state.script_runtime.script_value = Some("BATTLETOWERBATTLEROOM_GENTLEMAN".to_string());
    let before = state.clone();
    let catalog = TrainerCatalog::default();

    let mut divider = ReplayDivider::new([]);
    let error = apply_random_special_routine_with_context(
        &mut state,
        trainer_context(
            &moves(),
            &BTreeMap::new(),
            &SpeciesLearnsets::new(),
            &catalog,
        ),
        "LoadOpponentTrainerAndPokemonWithOTSprite",
        &mut divider,
    )
    .expect_err("missing compiled Battle Tower roster");

    assert_eq!(
        error,
        RandomSpecialRoutineError::Routine(SpecialRoutineError::MissingBattleTowerRules {
            routine: "LoadOpponentTrainerAndPokemonWithOTSprite".to_string()
        })
    );
    assert_eq!(state, before);

    let mut incomplete = GameState::default();
    incomplete.script_runtime.script_value = Some("BATTLETOWERBATTLEROOM_GENTLEMAN".to_string());
    let before_incomplete = incomplete.clone();
    let rules = battle_tower_rules_with_banned_species(vec![]);
    let move_catalog = moves();
    let species = BTreeMap::new();
    let learnsets = SpeciesLearnsets::new();
    let mut context = trainer_context(&move_catalog, &species, &learnsets, &catalog);
    context.battle_tower_rules = Some(&rules);
    let mut divider = ReplayDivider::new([]);
    let error = apply_random_special_routine_with_context(
        &mut incomplete,
        context,
        "LoadOpponentTrainerAndPokemonWithOTSprite",
        &mut divider,
    )
    .expect_err("empty compiled Battle Tower roster");
    assert_eq!(
        error,
        RandomSpecialRoutineError::Routine(SpecialRoutineError::InvalidBattleTowerRules {
            routine: "LoadOpponentTrainerAndPokemonWithOTSprite".to_string(),
            message: "compiled trainer roster and Pokemon groups are required".to_string(),
        })
    );
    assert_eq!(incomplete, before_incomplete);
}

#[test]
fn runtime_spawn_point_from_runtime_tile_preserves_exact_coordinate_fields() {
    let spawn = runtime_spawn_point_from_runtime_tile(
        7,
        "ROUTE_29".to_string(),
        "Route29".to_string(),
        1,
        2,
        "GROUP_ROUTE_29".to_string(),
        TilePosition::new(4, 6),
    )
    .expect("runtime tile can form spawn point");

    assert_eq!(spawn.tile_x, 4);
    assert_eq!(spawn.tile_y, 6);
    assert_eq!(spawn.metatile_x, 2);
    assert_eq!(spawn.metatile_y, 3);
    assert_eq!(spawn.subtile_x, 0);
    assert_eq!(spawn.subtile_y, 0);
    assert_eq!(
        checked_runtime_spawn_expected_tile(&spawn),
        Some(TilePosition::new(4, 6))
    );
    let odd_spawn = runtime_spawn_point_from_runtime_tile(
        7,
        "ROUTE_29".to_string(),
        "Route29".to_string(),
        1,
        2,
        "GROUP_ROUTE_29".to_string(),
        TilePosition::new(5, 7),
    )
    .expect("odd runtime tile can form spawn point");
    assert_eq!(odd_spawn.tile_x, 5);
    assert_eq!(odd_spawn.tile_y, 7);
    assert_eq!(odd_spawn.metatile_x, 2);
    assert_eq!(odd_spawn.metatile_y, 3);
    assert_eq!(odd_spawn.subtile_x, 1);
    assert_eq!(odd_spawn.subtile_y, 1);
    assert!(
        runtime_spawn_point_from_runtime_tile(
            7,
            "ROUTE_29".to_string(),
            "Route29".to_string(),
            1,
            2,
            "GROUP_ROUTE_29".to_string(),
            TilePosition::new(-1, 7),
        )
        .is_none()
    );
}

#[test]
fn runtime_spawn_point_catalog_issues_validate_exact_pack_records() {
    let spawn_points = [
        (
            "1".to_string(),
            RuntimeSpawnPointRef {
                identifier: 0,
                map_constant: "MISSING_MAP".to_string(),
                map_name: "MissingMap".to_string(),
                group_name: String::new(),
                ..spawn_point(0, "MISSING_MAP", 1, 1, 0, 0)
            },
        ),
        (
            "2".to_string(),
            RuntimeSpawnPointRef {
                identifier: 2,
                map_constant: "ROUTE_29".to_string(),
                map_name: "WrongMap".to_string(),
                group_name: "GROUP_ROUTE_29".to_string(),
                ..spawn_point(2, "ROUTE_29", 1, 2, 4, 4)
            },
        ),
        (
            "3 4".to_string(),
            RuntimeSpawnPointRef {
                identifier: 3,
                map_constant: "ROUTE 29".to_string(),
                map_name: "Route 29".to_string(),
                group_name: "GROUP ROUTE_29".to_string(),
                ..spawn_point(3, "ROUTE_29", 1, 3, 6, 6)
            },
        ),
        (
            "4".to_string(),
            RuntimeSpawnPointRef {
                identifier: 4,
                map_constant: "ROUTE_29".to_string(),
                map_name: "Route29".to_string(),
                group_name: "GROUP_ROUTE_29".to_string(),
                ..spawn_point(4, "ROUTE_29", 1, 2, 8, 8)
            },
        ),
    ]
    .into_iter()
    .collect();
    let runtime_map_names = [("ROUTE_29".to_string(), "Route29".to_string())]
        .into_iter()
        .collect();

    assert_eq!(
        runtime_spawn_point_catalog_issues(&spawn_points, &runtime_map_names),
        vec![
            RuntimeSpawnPointCatalogIssue::InvalidSpawnPoint {
                key: "1".to_string(),
            },
            RuntimeSpawnPointCatalogIssue::IdentifierMismatch {
                key: "1".to_string(),
                identifier: 0,
            },
            RuntimeSpawnPointCatalogIssue::UnknownMap {
                key: "1".to_string(),
                map_constant: "MISSING_MAP".to_string(),
            },
            RuntimeSpawnPointCatalogIssue::MapMismatch {
                key: "2".to_string(),
                map_name: "WrongMap".to_string(),
                metadata_name: "Route29".to_string(),
            },
            RuntimeSpawnPointCatalogIssue::InvalidSpawnPoint {
                key: "3 4".to_string(),
            },
            RuntimeSpawnPointCatalogIssue::IdentifierMismatch {
                key: "3 4".to_string(),
                identifier: 3,
            },
            RuntimeSpawnPointCatalogIssue::DuplicateMapBinding {
                key: "4".to_string(),
                existing_key: "2".to_string(),
                group_id: 1,
                map_id: 2,
            },
        ],
    );
}

#[test]
fn runtime_spawn_point_catalog_issues_reject_reserved_pack_prefix_tokens() {
    let spawn_points = [(
        "fallback_1".to_string(),
        RuntimeSpawnPointRef {
            identifier: 1,
            map_constant: "legacy_ROUTE_29".to_string(),
            map_name: "Route29".to_string(),
            group_name: "fallback_GROUP_ROUTE_29".to_string(),
            ..spawn_point(1, "ROUTE_29", 1, 1, 0, 0)
        },
    )]
    .into_iter()
    .collect();
    let runtime_map_names = [("ROUTE_29".to_string(), "Route29".to_string())]
        .into_iter()
        .collect();

    assert_eq!(
        runtime_spawn_point_catalog_issues(&spawn_points, &runtime_map_names),
        vec![
            RuntimeSpawnPointCatalogIssue::InvalidSpawnPoint {
                key: "fallback_1".to_string(),
            },
            RuntimeSpawnPointCatalogIssue::IdentifierMismatch {
                key: "fallback_1".to_string(),
                identifier: 1,
            },
        ],
    );
}

#[test]
fn runtime_spawn_point_catalog_rejects_identifiers_past_num_spawns() {
    let spawn_points = [(
        "28".to_string(),
        RuntimeSpawnPointRef {
            identifier: 28,
            map_constant: "ROUTE_29".to_string(),
            map_name: "Route29".to_string(),
            group_name: "GROUP_ROUTE_29".to_string(),
            ..spawn_point(28, "ROUTE_29", 1, 1, 0, 0)
        },
    )]
    .into_iter()
    .collect();
    let runtime_map_names = [("ROUTE_29".to_string(), "Route29".to_string())]
        .into_iter()
        .collect();

    assert_eq!(
        runtime_spawn_point_catalog_issues(&spawn_points, &runtime_map_names),
        vec![RuntimeSpawnPointCatalogIssue::IdentifierOutOfRange {
            key: "28".to_string(),
            identifier: 28,
        }]
    );
}

#[test]
fn runtime_spawn_point_catalog_issues_reject_inconsistent_tile_fields() {
    let spawn_points = [(
        "1".to_string(),
        RuntimeSpawnPointRef {
            identifier: 1,
            map_constant: "ROUTE_29".to_string(),
            map_name: "Route29".to_string(),
            group_name: "GROUP_ROUTE_29".to_string(),
            tile_x: 10,
            tile_y: 8,
            metatile_x: 4,
            metatile_y: 4,
            subtile_x: 0,
            subtile_y: 0,
            ..spawn_point(1, "ROUTE_29", 1, 1, 8, 8)
        },
    )]
    .into_iter()
    .collect();
    let runtime_map_names = [("ROUTE_29".to_string(), "Route29".to_string())]
        .into_iter()
        .collect();

    assert_eq!(
        runtime_spawn_point_catalog_issues(&spawn_points, &runtime_map_names),
        vec![RuntimeSpawnPointCatalogIssue::CoordinateMismatch {
            key: "1".to_string(),
            tile_x: 10,
            tile_y: 8,
            expected_tile_x: 8,
            expected_tile_y: 8,
        }],
    );
}

#[test]
fn runtime_spawn_point_catalog_issues_reject_out_of_range_subtiles() {
    let spawn_points = [(
        "1".to_string(),
        RuntimeSpawnPointRef {
            identifier: 1,
            map_constant: "ROUTE_29".to_string(),
            map_name: "Route29".to_string(),
            group_name: "GROUP_ROUTE_29".to_string(),
            tile_x: 3,
            tile_y: 0,
            metatile_x: 0,
            metatile_y: 0,
            subtile_x: METATILE_WIDTH,
            subtile_y: 0,
            ..spawn_point(1, "ROUTE_29", 1, 1, 0, 0)
        },
    )]
    .into_iter()
    .collect();
    let runtime_map_names = [("ROUTE_29".to_string(), "Route29".to_string())]
        .into_iter()
        .collect();

    assert_eq!(
        runtime_spawn_point_catalog_issues(&spawn_points, &runtime_map_names),
        vec![RuntimeSpawnPointCatalogIssue::InvalidSubtile {
            key: "1".to_string(),
            subtile_x: METATILE_WIDTH,
            subtile_y: 0,
            metatile_width: METATILE_WIDTH,
        }],
    );
}

#[test]
fn runtime_spawn_point_catalog_issues_reject_overflowing_runtime_tile() {
    let spawn_points = [(
        "1".to_string(),
        RuntimeSpawnPointRef {
            identifier: 1,
            map_constant: "ROUTE_29".to_string(),
            map_name: "Route29".to_string(),
            group_id: 1,
            map_id: 1,
            tile_x: 0,
            tile_y: 0,
            group_name: "GROUP_ROUTE_29".to_string(),
            metatile_x: i16::MAX,
            metatile_y: 0,
            subtile_x: 0,
            subtile_y: 0,
        },
    )]
    .into_iter()
    .collect();
    let runtime_map_names = [("ROUTE_29".to_string(), "Route29".to_string())]
        .into_iter()
        .collect();

    assert_eq!(
        runtime_spawn_point_catalog_issues(&spawn_points, &runtime_map_names),
        vec![RuntimeSpawnPointCatalogIssue::CoordinateOverflow {
            key: "1".to_string(),
            metatile_x: i16::MAX,
            metatile_y: 0,
            subtile_x: 0,
            subtile_y: 0,
        }],
    );
}

#[test]
fn special_routine_issue_json_rejects_unknown_fallback_fields() {
    let tower_error = serde_json::from_value::<BattleTowerRulesIssue>(serde_json::json!({
        "InvalidFailureText": {
            "field": "PartyCount",
            "text_id": "BattleTowerPartyCountText",
            "default_text_id": "BattleTowerDefaultText"
        }
    }))
    .expect_err("default battle tower failure text must be rejected")
    .to_string();
    assert!(
        tower_error.contains("unknown field `default_text_id`"),
        "{tower_error}"
    );

    let odd_egg_error = serde_json::from_value::<OddEggDefinitionIssue>(serde_json::json!({
        "UnknownSpecies": {
            "index": 0,
            "species_id": "MODMON",
            "fallback_species_id": "PICHU"
        }
    }))
    .expect_err("fallback odd egg species must be rejected")
    .to_string();
    assert!(
        odd_egg_error.contains("unknown field `fallback_species_id`"),
        "{odd_egg_error}"
    );

    let dratini_error = serde_json::from_value::<DratiniMoveSetIssue>(serde_json::json!({
        "UnknownMove": {
            "mode": 1,
            "move_index": 0,
            "move_id": "MOD_MOVE",
            "legacy_move_id": "EXTREMESPEED"
        }
    }))
    .expect_err("legacy dratini move must be rejected")
    .to_string();
    assert!(
        dratini_error.contains("unknown field `legacy_move_id`"),
        "{dratini_error}"
    );

    let routine_error = serde_json::from_value::<SpecialRoutineError>(serde_json::json!({
        "UnknownSpecies": {
            "routine": "SpecialMonCheck",
            "species": "MODMON",
            "fallback_species": "PIKACHU"
        }
    }))
    .expect_err("special routine errors must not accept fallback species")
    .to_string();
    assert!(
        routine_error.contains("unknown field `fallback_species`"),
        "{routine_error}"
    );

    let source_error =
        serde_json::from_str::<LuckyNumberWinnerSource>(r#"{"party":{"legacy_box":0}}"#)
            .expect_err("lucky number sources must not accept legacy aliases")
            .to_string();
    assert!(
        source_error.contains("invalid type")
            || source_error.contains("unknown field `legacy_box`"),
        "{source_error}"
    );
}
