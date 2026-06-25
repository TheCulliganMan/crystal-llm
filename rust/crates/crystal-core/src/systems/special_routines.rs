use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::battle::start::materialize_trainer_party;
use crate::models::{
    CaptureStorageLocation, Dv, Item, LearnedMove, MAX_BOX_MONS, Move, Pokemon, PokemonSpecies,
    TrainerCatalog, create_pokemon_from_known_dvs,
};
use crate::random::Random;
use crate::state::{
    BattleMemory, EventFlagError, GameState, LinkSerialConnectionStatus, MobileBattleTowerRecord,
    OverworldMemory, RoamingPokemonState, ScriptAudioRuntimeEvent, ScriptAudioRuntimeKind,
    ScriptFadeColor, ScriptFadeDirection, ScriptGraphicsRuntimeEvent, ScriptGraphicsRuntimeKind,
    ScriptMapRuntimeEvent, ScriptMapRuntimeKind, ScriptMoneyRuntimeEvent, ScriptMoneyRuntimeKind,
    ScriptMusicFade, ScriptScreenFade, ScriptWarpRequest,
};
use crate::systems::experience::GrowthRateCatalog;
use crate::systems::learnsets::SpeciesLearnsets;
use crate::world::encounters::TimeOfDay;
use crate::world::map::{Direction, TilePosition};
use crate::world::movement::MovementMode;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecialRoutineOutcome {
    pub routine: String,
    pub effect: SpecialRoutineEffect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SpecialRoutineEffect {
    HealParty {
        healed_slots: Vec<usize>,
    },
    FadeOutMusic {
        audio_id: String,
        fade_frames: u16,
    },
    WaitSfx,
    PlayMapMusic,
    RestartMapMusic,
    PlayCurMonCry {
        species: String,
        audio_id: String,
    },
    PlaySlowCry {
        species: String,
        audio_id: String,
    },
    GameboyCheck {
        token: String,
    },
    MobileAdapterStatus {
        value: String,
    },
    FirstPokemonHappiness {
        party_slot: usize,
        species: String,
        nickname: String,
        happiness: u8,
    },
    CheckFirstMonIsEgg {
        species: String,
        nickname: String,
        is_egg: bool,
    },
    FindPartyMonThatSpecies {
        species: String,
        found: bool,
    },
    FindPartyMonThatSpeciesYourTrainerId {
        species: String,
        player_name: String,
        player_id: u16,
        found: bool,
    },
    FindPartyMonAboveLevel {
        level: u8,
        found: bool,
        species: Option<String>,
    },
    FindPartyMonAtLeastThatHappy {
        happiness: u8,
        found: bool,
        species: Option<String>,
    },
    MonCheck {
        species: String,
        player_name: String,
        player_id: u16,
        owned: bool,
    },
    BeastsCheck {
        player_name: String,
        player_id: u16,
        missing_species: Option<String>,
        owned_all: bool,
    },
    GameCornerPrizeMonCheckDex {
        species: String,
        species_int_id: u16,
        already_caught: bool,
        recorded_caught: bool,
    },
    UnusedSetSeenMon {
        species: String,
        species_int_id: u16,
        newly_seen: bool,
    },
    ActivateFishingSwarm {
        value: u8,
    },
    CheckCaughtCelebi {
        caught: bool,
    },
    SetPlayerPalette {
        raw_value: i64,
        palette_id: u8,
        changed: bool,
    },
    SnorlaxAwake {
        music: Option<String>,
        tile: Option<(i16, i16)>,
        awake: bool,
    },
    SetDayOfWeek {
        day: u8,
    },
    InitialSetDstFlag,
    InitialClearDstFlag,
    UpdateTime {
        hour: u8,
        minute: u8,
        second: u8,
        day_of_week: u8,
        time_of_day: TimeOfDay,
    },
    SampleKenjiBreakCountdown {
        value: u8,
        rng_seed_after: u32,
    },
    CheckLuckyNumberShowFlag {
        flag: bool,
    },
    ResetLuckyNumberShowFlag {
        lucky_number: u16,
        lucky_number_day: u8,
        rng_seed_after: u32,
    },
    CheckForLuckyNumberWinners {
        lucky_number: u16,
        tier: u8,
        source: Option<LuckyNumberWinnerSource>,
        species: Option<String>,
        text_label: Option<String>,
    },
    PlaceMoneyTopRight {
        money: u32,
        formatted: String,
    },
    DisplayMoneyAndCoinBalance {
        money: u32,
        coins: u16,
        formatted_money: String,
        formatted_coins: String,
    },
    DisplayCoinCaseBalance {
        coins: u16,
        formatted_coins: String,
    },
    PrintTodaysLuckyNumber {
        lucky_number: u16,
        formatted: String,
    },
    GsHealings {
        healings: u16,
    },
    TrainerRankingsHealings {
        healings: u16,
    },
    Reset {
        value: String,
    },
    HoOhChamber {
        has_ho_oh: bool,
        suicune_unleashed: bool,
        raikou_unleashed: bool,
        entei_unleashed: bool,
        open: bool,
    },
    GraphicsCommand {
        kind: ScriptGraphicsRuntimeKind,
    },
    ScreenFade {
        color: ScriptFadeColor,
        direction: ScriptFadeDirection,
        frames: u16,
    },
    PokemonCenterPc {
        party_count: usize,
        current_pc_box: usize,
    },
    PlayersHousePc {
        party_count: usize,
    },
    ProfOaksPcBoot {
        seen_count: usize,
        caught_count: usize,
        rating_label: String,
    },
    OverworldTownMap {
        map_name: Option<String>,
    },
    UnownPrinter {
        unlocked: bool,
    },
    MapRadio {
        station: String,
    },
    NameRival {
        rival_name: String,
    },
    MoveDeletion {
        party_slot: usize,
        species: String,
        deleted_move: String,
        remaining_moves: usize,
    },
    RuntimeVisualCommand {
        kind: ScriptGraphicsRuntimeKind,
    },
    CheckPokerus {
        found: bool,
        newly_discovered: bool,
    },
    HappinessService {
        party_slot: usize,
        species: String,
        old_happiness: u8,
        new_happiness: u8,
        script_value: u8,
        change_code: u8,
        rng_seed_after: u32,
    },
    NameRater {
        party_slot: usize,
        species: String,
        old_nickname: String,
        new_nickname: String,
    },
    PokeSeer {
        party_slot: usize,
        species: String,
        nickname: String,
        original_trainer_name: String,
        original_trainer_id: u16,
    },
    MoveTutor {
        party_slot: usize,
        species: String,
        move_name: String,
        learned: bool,
    },
    GiveShuckle {
        stored: bool,
        rng_seed_after: u32,
    },
    ReturnShuckie {
        party_slot: Option<usize>,
        result: u8,
    },
    GiveDratini {
        party_slot: Option<usize>,
        mode: u8,
        move_names: Vec<String>,
        learned: bool,
    },
    BillsGrandfather {
        party_slot: Option<usize>,
        species: Option<String>,
    },
    SelectApricornForKurt {
        apricorn: Option<String>,
        quantity: u16,
    },
    InitRoamMons {
        roamers: Vec<RoamingPokemonState>,
    },
    CheckMysteryGift {
        has_pending_item: bool,
    },
    GetMysteryGiftItem {
        item_id: Option<String>,
        received: bool,
    },
    UnlockMysteryGift {
        newly_unlocked: bool,
    },
    BuenasPassword {
        category: String,
        category_type: String,
        correct: String,
        guess: Option<String>,
        matched: bool,
        rng_seed_after: u32,
    },
    BuenaPrize {
        item_id: String,
        quantity: u16,
        points_spent: u8,
        balance: u8,
    },
    CelebiShrineEvent {
        battle_type: String,
    },
    CheckMagikarpLength {
        party_slot: usize,
        species: String,
        feet: u8,
        inches: u8,
        result: u8,
    },
    MagikarpHouseSign {
        feet: u8,
        inches: u8,
        formatted: String,
    },
    DayCareInteraction {
        caretaker: String,
        action: String,
        success: bool,
        pokemon: Option<String>,
    },
    DayCareMon {
        caretaker: String,
        occupied: bool,
        pokemon: Option<String>,
        level: Option<u8>,
    },
    GiveParkBalls {
        balls: u8,
    },
    SelectRandomBugContestContestants {
        flags: Vec<String>,
        rng_seed_after: u32,
    },
    ContestDropOffMons {
        result: u8,
        backup_count: usize,
        second_party_species: Option<String>,
    },
    ContestReturnMons {
        restored_count: usize,
    },
    CheckPartyFullAfterContest {
        result: u8,
        species: Option<String>,
    },
    BugContestJudging {
        rank: u8,
    },
    LinkAction {
        action: u8,
        room: u8,
    },
    LinkResult {
        success: bool,
        link_mode: u8,
    },
    LinkRoom {
        room: String,
        link_mode: u8,
    },
    TimeCapsuleCompatibility {
        result_code: u8,
        mon_name: Option<String>,
        move_name: Option<String>,
    },
    QuickSave {
        requested: bool,
    },
    AskMobileOrCable {
        selection: String,
    },
    CableClubCheckWhichChris {
        male_player: bool,
    },
    BattleTowerAction {
        action: String,
        value: String,
        truthy: bool,
    },
    CheckForBattleTowerRules {
        failure: Option<String>,
    },
    BattleTowerRoomMenu {
        records: Vec<BattleTowerRecentRecord>,
    },
    BattleTowerBattle {
        result_code: u8,
        beaten_trainers: u8,
        challenge_state: u8,
    },
    BattleTowerMobileError,
    LoadOpponentTrainerAndPokemonWithOtSprite {
        trainer_id: String,
        trainer_class: String,
        trainer_name: String,
        party_size: usize,
        sprite_constant: String,
        target_object: String,
    },
    AskRememberPassword {
        remember: bool,
    },
    MobileHandshake {
        routine: String,
        mode: String,
        link_mode: u8,
        serial_status: LinkSerialConnectionStatus,
        handshakes: u32,
    },
    MobileSessionEnded,
    BattleTowerMobileFlag {
        flag: String,
    },
    MobileSelectThreeMons {
        indexes: Vec<usize>,
    },
    BattleTowerLeaderboard {
        records: Vec<MobileBattleTowerRecord>,
        acknowledged: bool,
    },
    WarpToSpawnPoint {
        spawn_identifier: u16,
        map_name: String,
        tile: TilePosition,
    },
    GiveOddEgg {
        table_index: usize,
        species: String,
        party_slot: usize,
        shiny: bool,
        rng_seed_after: u32,
    },
    BankOfMom {
        money: u32,
        moms_money: u32,
    },
    SlotMachine {
        coins: u16,
    },
    CardFlip {
        coins: u16,
    },
    TrainerHouse {
        wins: u16,
        losses: u16,
        draws: u16,
    },
    PhotoStudio {
        party_slot: Option<usize>,
        species: Option<String>,
    },
    BattleTowerChallengeExplanationCancel,
    DisplayLinkRecord {
        wins: u16,
        losses: u16,
        draws: u16,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleTowerRecentRecord {
    pub day: u8,
    pub wins: u8,
    pub result: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
pub enum SpecialRoutineError {
    #[error("unsupported exact special routine {routine}")]
    UnsupportedRoutine { routine: String },
    #[error("declared special routine {routine} is inactive in the definitive modpack scripts")]
    InactiveDeclaredRoutine { routine: String },
    #[error(
        "special routine {routine} references unknown move {move_id} in party slot {party_slot}"
    )]
    UnknownMove {
        routine: String,
        party_slot: usize,
        move_id: String,
    },
    #[error("special routine {routine} requires a populated party slot")]
    EmptyParty { routine: String },
    #[error("special routine {routine} requires at least one non-egg party Pokemon")]
    NoNonEggPartyPokemon { routine: String },
    #[error("special routine {routine} requires _value species identifier")]
    MissingSpeciesValue { routine: String },
    #[error("special routine {routine} requires script variable {variable}")]
    MissingScriptValue { routine: String, variable: String },
    #[error("special routine {routine} requires current party species")]
    MissingCurrentPartySpecies { routine: String },
    #[error("special routine {routine} species {species} has no declared cry in the modpack")]
    MissingCryMetadata { routine: String, species: String },
    #[error("special routine {routine} references unknown species {species}")]
    UnknownSpecies { routine: String, species: String },
    #[error("special routine {routine} has invalid numeric value {value}")]
    InvalidNumericValue { routine: String, value: String },
    #[error(
        "special routine {routine} current PC box {current_pc_box} is invalid for {box_count} boxes"
    )]
    InvalidCurrentPcBox {
        routine: String,
        current_pc_box: usize,
        box_count: usize,
    },
    #[error("special routine {routine} PC box {box_index} count {count} exceeds box capacity")]
    InvalidPcBoxCount {
        routine: String,
        box_index: usize,
        count: usize,
    },
    #[error("special routine {routine} failed to read event flag: {error}")]
    EventFlag {
        routine: String,
        error: EventFlagError,
    },
    #[error("special routine {routine} party slot {party_slot} is invalid")]
    InvalidPartySlot { routine: String, party_slot: usize },
    #[error(
        "special routine {routine} move slot {move_slot} is invalid for party slot {party_slot}"
    )]
    InvalidMoveSlot {
        routine: String,
        party_slot: usize,
        move_slot: usize,
    },
    #[error("special routine {routine} cannot delete the only move from party slot {party_slot}")]
    CannotDeleteOnlyMove { routine: String, party_slot: usize },
    #[error("special routine {routine} references unknown item {item_id}")]
    UnknownItem { routine: String, item_id: String },
    #[error("special routine {routine} could not build gift Pokemon: {error}")]
    GiftPokemonBuild { routine: String, error: String },
    #[error("special routine {routine} could not store gift Pokemon {species}")]
    GiftStorageFull { routine: String, species: String },
    #[error("special routine {routine} has unhandled Battle Tower action {action}")]
    UnhandledBattleTowerAction { routine: String, action: String },
    #[error("special routine {routine} mobile password exceeds 17 bytes")]
    MobilePasswordTooLong { routine: String },
    #[error("special routine {routine} has invalid mobile battle timer {value}")]
    InvalidMobileBattleTimer { routine: String, value: String },
    #[error("special routine {routine} references invalid day-care caretaker {caretaker}")]
    InvalidDayCareCaretaker { routine: String, caretaker: String },
    #[error("special routine {routine} requires Odd Egg definitions from the modpack")]
    MissingOddEggDefinitions { routine: String },
    #[error("special routine {routine} odd egg probability table is invalid")]
    InvalidOddEggTable { routine: String },
    #[error("special routine {routine} requires Buena password categories from the modpack")]
    MissingBuenaPasswordCategories { routine: String },
    #[error("special routine {routine} has invalid Buena password category index {index}")]
    InvalidBuenaPasswordCategoryIndex { routine: String, index: usize },
    #[error("special routine {routine} has invalid Buena password option index {index}")]
    InvalidBuenaPasswordOptionIndex { routine: String, index: usize },
    #[error("special routine {routine} requires Buena prize definitions from the modpack")]
    MissingBuenaPrizeDefinitions { routine: String },
    #[error("special routine {routine} requires Kurt apricorn recipes from the modpack")]
    MissingKurtApricornRecipes { routine: String },
    #[error("special routine {routine} requires Shuckie gift data from the modpack")]
    MissingShuckieGift { routine: String },
    #[error("special routine {routine} requires Dratini move sets from the modpack")]
    MissingDratiniMoveSets { routine: String },
    #[error("special routine {routine} requires Bug-Catching Contest config from the modpack")]
    MissingBugContestConfig { routine: String },
    #[error("special routine {routine} has invalid Bug-Catching Contest config: {message}")]
    InvalidBugContestConfig { routine: String, message: String },
    #[error("special routine {routine} requires Battle Tower rules from the modpack")]
    MissingBattleTowerRules { routine: String },
    #[error("special routine {routine} has invalid Battle Tower rules: {message}")]
    InvalidBattleTowerRules { routine: String, message: String },
    #[error("special routine {routine} requires Oak rating entries from the modpack")]
    MissingOakRatingTable { routine: String },
    #[error("special routine {routine} has invalid Oak rating table: {message}")]
    InvalidOakRatingTable { routine: String, message: String },
    #[error("special routine {routine} requires Magikarp length table from the modpack")]
    MissingMagikarpLengthTable { routine: String },
    #[error("special routine {routine} has invalid Magikarp length table: {message}")]
    InvalidMagikarpLengthTable { routine: String, message: String },
    #[error("special routine {routine} requires happiness data from the modpack")]
    MissingHappinessData { routine: String },
    #[error("special routine {routine} has invalid happiness data: {message}")]
    InvalidHappinessData { routine: String, message: String },
    #[error("special routine {routine} requires roaming Pokemon definitions from the modpack")]
    MissingRoamingPokemonDefinitions { routine: String },
    #[error("special routine {routine} requires runtime spawn points from the modpack")]
    MissingRuntimeSpawnPoints { routine: String },
    #[error("special routine {routine} references unknown spawn point {spawn_identifier}")]
    UnknownSpawnPoint {
        routine: String,
        spawn_identifier: u16,
    },
    #[error("special routine {routine} cannot resolve spawn for map group {group_id} map {map_id}")]
    UnknownSpawnMap {
        routine: String,
        group_id: i16,
        map_id: i16,
    },
    #[error("special routine {routine} references unknown Battle Tower trainer {trainer_id}")]
    UnknownBattleTowerTrainer { routine: String, trainer_id: String },
    #[error(
        "special routine {routine} could not materialize Battle Tower trainer {trainer_id}: {error}"
    )]
    BattleTowerTrainerBuild {
        routine: String,
        trainer_id: String,
        error: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LuckyNumberWinnerSource {
    Party,
    Pc,
}

#[derive(Debug, Clone, Copy)]
pub struct SpecialRoutineContext<'a> {
    pub move_catalog: &'a BTreeMap<String, Move>,
    pub cry_by_species: &'a BTreeMap<String, String>,
    pub species_catalog: &'a BTreeMap<String, PokemonSpecies>,
    pub learnsets: &'a SpeciesLearnsets,
    pub growth_rates: &'a GrowthRateCatalog,
    pub item_catalog: &'a BTreeMap<String, Item>,
    pub runtime_spawn_points: &'a BTreeMap<String, RuntimeSpawnPointRef>,
    pub roaming_pokemon: &'a [RoamingPokemonDefinition],
    pub buena_password_categories: &'a [BuenaPasswordCategoryDefinition],
    pub buena_prizes: &'a [BuenaPrizeDefinition],
    pub kurt_apricorn_recipes: &'a [KurtApricornRecipe],
    pub shuckie_gift: Option<&'a ShuckieGiftDefinition>,
    pub dratini_move_sets: &'a [DratiniMoveSetDefinition],
    pub bug_contest_config: Option<&'a BugContestConfig>,
    pub battle_tower_rules: Option<&'a BattleTowerRules>,
    pub magikarp_lengths: &'a [MagikarpLengthEntry],
    pub happiness_data: Option<&'a HappinessData>,
    pub trainer_catalog: &'a TrainerCatalog,
    pub odd_egg_definitions: &'a [OddEggDefinition],
    pub oak_ratings: &'a [OakRatingEntry],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BugContestConfig {
    pub park_balls: u8,
    pub timer_minutes: u8,
    pub timer_seconds: u8,
    pub selected_contestant_count: usize,
    pub contestant_flags: Vec<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BattleTowerRules {
    pub banned_species: Vec<String>,
    pub required_party_count: usize,
    pub challenge_streak_length: u8,
    pub minimum_level_group: u8,
    pub maximum_level_group: u8,
    pub level_group_size: u8,
    pub party_count_failure_text: String,
    pub duplicate_species_failure_text: String,
    pub duplicate_held_item_failure_text: String,
    pub egg_failure_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OakRatingEntry {
    pub caught_count_limit: usize,
    pub fanfare: String,
    pub text_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OddEggDefinition {
    pub species: String,
    pub moves: Vec<String>,
    pub original_trainer_id: u16,
    pub dvs: [u8; 4],
    pub probability: u16,
    pub level: u8,
    pub experience: i32,
    pub hatch_cycles: u8,
    pub nickname: String,
    pub original_trainer_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MagikarpLengthEntry {
    pub threshold: u16,
    pub divisor: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HappinessData {
    pub changes: Vec<HappinessChangeEntry>,
    pub services: Vec<HappinessServiceTable>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HappinessChangeEntry {
    pub code: String,
    pub change_code: u8,
    pub low: i16,
    pub mid: i16,
    pub high: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HappinessServiceTable {
    pub routine: String,
    pub outcomes: Vec<HappinessServiceOutcome>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HappinessServiceOutcome {
    pub roll_weight: u8,
    pub script_value: u8,
    pub change_code: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DratiniMoveSetDefinition {
    pub mode: u8,
    pub moves: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShuckieGiftDefinition {
    pub species: String,
    pub level: u8,
    pub held_item: String,
    pub nickname: String,
    pub original_trainer_name: String,
    pub original_trainer_id: u16,
    pub got_today_engine_flag: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuenaPasswordCategoryDefinition {
    pub id: String,
    pub category_type: String,
    pub points: u8,
    pub options: Vec<String>,
}

pub const BUENA_PASSWORD_CATEGORY_MON: &str = "BUENA_MON";
pub const BUENA_PASSWORD_CATEGORY_ITEM: &str = "BUENA_ITEM";
pub const BUENA_PASSWORD_CATEGORY_MOVE: &str = "BUENA_MOVE";
pub const BUENA_PASSWORD_CATEGORY_STRING: &str = "BUENA_STRING";
pub const BUENA_PASSWORD_CATEGORY_TYPES: &[&str] = &[
    BUENA_PASSWORD_CATEGORY_MON,
    BUENA_PASSWORD_CATEGORY_ITEM,
    BUENA_PASSWORD_CATEGORY_MOVE,
    BUENA_PASSWORD_CATEGORY_STRING,
];

pub fn is_known_buena_password_category_type(category_type: &str) -> bool {
    BUENA_PASSWORD_CATEGORY_TYPES.contains(&category_type)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KurtApricornRecipe {
    pub apricorn: String,
    pub ball: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuenaPrizeDefinition {
    pub item_id: String,
    pub cost: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoamingPokemonDefinition {
    pub species: String,
    pub level: u8,
    pub map_group: u16,
    pub map_number: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeSpawnPointRef {
    pub identifier: u16,
    pub map_constant: String,
    pub map_name: String,
    pub group_id: i16,
    pub map_id: i16,
    pub tile_x: i16,
    pub tile_y: i16,
    pub group_name: String,
    pub metatile_x: i16,
    pub metatile_y: i16,
    pub subtile_x: i16,
    pub subtile_y: i16,
}

pub const EXECUTABLE_SPECIAL_ROUTINES: &[&str] = &[
    "WarpToSpawnPoint",
    "HealParty",
    "FadeOutMusic",
    "WaitSFX",
    "PlayMapMusic",
    "RestartMapMusic",
    "PlayCurMonCry",
    "PlaySlowCry",
    "GameboyCheck",
    "CheckMobileAdapterStatusSpecial",
    "GetFirstPokemonHappiness",
    "CheckFirstMonIsEgg",
    "FindPartyMonThatSpecies",
    "FindPartyMonThatSpeciesYourTrainerID",
    "FindPartyMonAboveLevel",
    "FindPartyMonAtLeastThatHappy",
    "MonCheck",
    "BeastsCheck",
    "GameCornerPrizeMonCheckDex",
    "UnusedSetSeenMon",
    "ActivateFishingSwarm",
    "CheckCaughtCelebi",
    "SetPlayerPalette",
    "SnorlaxAwake",
    "SetDayOfWeek",
    "InitialSetDSTFlag",
    "InitialClearDSTFlag",
    "UpdateTime",
    "SampleKenjiBreakCountdown",
    "CheckLuckyNumberShowFlag",
    "ResetLuckyNumberShowFlag",
    "CheckForLuckyNumberWinners",
    "PlaceMoneyTopRight",
    "DisplayMoneyAndCoinBalance",
    "DisplayCoinCaseBalance",
    "PrintTodaysLuckyNumber",
    "GSHealings",
    "StubbedTrainerRankings_Healings",
    "Reset",
    "HoOhChamber",
    "ClearBGPalettesBufferScreen",
    "ClearBGPalettes",
    "UpdateTimePals",
    "ClearTilemap",
    "LoadMapPalettes",
    "RefreshSprites",
    "UpdateSprites",
    "ReloadSpritesNoPalettes",
    "FadeOutToWhite",
    "FadeInFromWhite",
    "FadeOutToBlack",
    "FadeInFromBlack",
    "PokemonCenterPC",
    "PlayersHousePC",
    "ProfOaksPCBoot",
    "OverworldTownMap",
    "UnownPrinter",
    "MapRadio",
    "NameRival",
    "MoveDeletion",
    "BattleTowerFade",
    "UpdatePlayerSprite",
    "HealMachineAnim",
    "SurfStartStep",
    "LoadUsedSpritesGFX",
    "ToggleMaptileDecorations",
    "ToggleDecorationsVisibility",
    "MagnetTrain",
    "Diploma",
    "PrintDiploma",
    "UnownPuzzle",
    "OmanyteChamber",
    "DisplayUnownWords",
    "CheckPokerus",
    "OlderHaircutBrother",
    "YoungerHaircutBrother",
    "DaisysGrooming",
    "NameRater",
    "PokeSeer",
    "MoveTutor",
    "BankOfMom",
    "SlotMachine",
    "CardFlip",
    "DisplayLinkRecord",
    "TrainerHouse",
    "PhotoStudio",
    "GiveShuckle",
    "ReturnShuckie",
    "GiveDratini",
    "BillsGrandfather",
    "SelectApricornForKurt",
    "InitRoamMons",
    "CheckMysteryGift",
    "GetMysteryGiftItem",
    "UnlockMysteryGift",
    "BuenasPassword",
    "BuenaPrize",
    "CelebiShrineEvent",
    "CheckMagikarpLength",
    "MagikarpHouseSign",
    "DayCareMan",
    "DayCareLady",
    "DayCareManOutside",
    "DayCareMon1",
    "DayCareMon2",
    "GiveParkBalls",
    "SelectRandomBugContestContestants",
    "ContestDropOffMons",
    "ContestReturnMons",
    "CheckPartyFullAfterContest",
    "BugContestJudging",
    "SetBitsForLinkTradeRequest",
    "SetBitsForBattleRequest",
    "SetBitsForTimeCapsuleRequest",
    "WaitForLinkedFriend",
    "CheckLinkTimeout_Receptionist",
    "CheckBothSelectedSameRoom",
    "CloseLink",
    "WaitForOtherPlayerToExit",
    "FailedLinkToPast",
    "TradeCenter",
    "Colosseum",
    "EnterTimeCapsule",
    "TimeCapsule",
    "CheckTimeCapsuleCompatibility",
    "TryQuickSave",
    "AskMobileOrCable",
    "CableClubCheckWhichChris",
    "BattleTowerAction",
    "CheckForBattleTowerRules",
    "BattleTowerRoomMenu",
    "BattleTowerBattle",
    "BattleTowerMobileError",
    "LoadOpponentTrainerAndPokemonWithOTSprite",
    "AskRememberPassword",
    "Function1700ba",
    "Function1011f1",
    "Function101220",
    "Function101225",
    "Function101231",
    "Function103780",
    "Function1037c2",
    "Function1037eb",
    "Function10383c",
    "Function10387b",
    "Mobile_SelectThreeMons",
    "GiveOddEgg",
    "Menu_ChallengeExplanationCancel",
];

pub const INACTIVE_DECLARED_SPECIAL_ROUTINES: &[&str] = &[
    "UnusedCheckUnusedTwoDayTimer",
    "UnusedFindItemInPCOrBag",
    "UnusedDummySpecial",
    "UnusedMemoryGame",
    "RandomUnseenWildMon",
    "RandomPhoneWildMon",
    "RandomPhoneMon",
    "Function11ac3e",
    "TradeCornerHoldMon",
    "Function11b5e8",
    "Function11b7e5",
    "Function11b879",
    "Function11b920",
    "Function11b93b",
    "Function170114",
    "Function1704e1",
    "UnusedBattleTowerDummySpecial1",
    "Function11ba38",
    "Function11c1ab",
    "Function17d2b6",
    "Function17d2ce",
    "Function102142",
    "UnusedBattleTowerDummySpecial2",
];

pub fn is_known_special_routine(routine: &str) -> bool {
    EXECUTABLE_SPECIAL_ROUTINES.contains(&routine)
        || INACTIVE_DECLARED_SPECIAL_ROUTINES.contains(&routine)
}

pub fn apply_special_routine(
    state: &mut GameState,
    move_catalog: &BTreeMap<String, Move>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let empty_cries = BTreeMap::new();
    let empty_species = BTreeMap::new();
    let empty_learnsets = SpeciesLearnsets::new();
    let empty_growth_rates = GrowthRateCatalog::new();
    let empty_items = BTreeMap::new();
    let empty_spawn_points = BTreeMap::new();
    let empty_trainers = TrainerCatalog::default();
    apply_special_routine_with_context(
        state,
        SpecialRoutineContext {
            move_catalog,
            cry_by_species: &empty_cries,
            species_catalog: &empty_species,
            learnsets: &empty_learnsets,
            growth_rates: &empty_growth_rates,
            item_catalog: &empty_items,
            runtime_spawn_points: &empty_spawn_points,
            roaming_pokemon: &[],
            buena_password_categories: &[],
            buena_prizes: &[],
            kurt_apricorn_recipes: &[],
            shuckie_gift: None,
            dratini_move_sets: &[],
            bug_contest_config: None,
            battle_tower_rules: None,
            magikarp_lengths: &[],
            happiness_data: None,
            trainer_catalog: &empty_trainers,
            odd_egg_definitions: &[],
            oak_ratings: &[],
        },
        routine,
    )
}

pub fn apply_special_routine_with_context(
    state: &mut GameState,
    context: SpecialRoutineContext<'_>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    match routine {
        "WarpToSpawnPoint" => warp_to_spawn_point(state, context.runtime_spawn_points, routine),
        "HealParty" => heal_party(state, context.move_catalog, routine),
        "FadeOutMusic" => fade_out_music(state, routine),
        "WaitSFX" => wait_sfx(state, routine),
        "PlayMapMusic" => play_map_music(state, routine),
        "RestartMapMusic" => restart_map_music(state, routine),
        "PlayCurMonCry" => play_cur_mon_cry(state, context.cry_by_species, routine),
        "PlaySlowCry" => play_slow_cry(state, context.cry_by_species, routine),
        "GameboyCheck" => gameboy_check(state, routine),
        "CheckMobileAdapterStatusSpecial" => mobile_adapter_status(state, routine),
        "GetFirstPokemonHappiness" => get_first_pokemon_happiness(state, routine),
        "CheckFirstMonIsEgg" => check_first_mon_is_egg(state, routine),
        "FindPartyMonThatSpecies" => find_party_mon_that_species(state, routine),
        "FindPartyMonThatSpeciesYourTrainerID" => {
            find_party_mon_that_species_your_trainer_id(state, routine)
        }
        "FindPartyMonAboveLevel" => find_party_mon_above_level(state, routine),
        "FindPartyMonAtLeastThatHappy" => find_party_mon_at_least_that_happy(state, routine),
        "MonCheck" => mon_check(state, routine),
        "BeastsCheck" => beasts_check(state, routine),
        "GameCornerPrizeMonCheckDex" => {
            game_corner_prize_mon_check_dex(state, context.species_catalog, routine)
        }
        "UnusedSetSeenMon" => unused_set_seen_mon(state, context.species_catalog, routine),
        "ActivateFishingSwarm" => activate_fishing_swarm(state, routine),
        "CheckCaughtCelebi" => check_caught_celebi(state, routine),
        "SetPlayerPalette" => set_player_palette(state, routine),
        "SnorlaxAwake" => snorlax_awake(state, routine),
        "SetDayOfWeek" => set_day_of_week(state, routine),
        "InitialSetDSTFlag" => initial_set_dst_flag(state, routine),
        "InitialClearDSTFlag" => initial_clear_dst_flag(state, routine),
        "UpdateTime" => update_time(state, routine),
        "SampleKenjiBreakCountdown" => sample_kenji_break_countdown(state, routine),
        "CheckLuckyNumberShowFlag" => check_lucky_number_show_flag(state, routine),
        "ResetLuckyNumberShowFlag" => reset_lucky_number_show_flag(state, routine),
        "CheckForLuckyNumberWinners" => check_for_lucky_number_winners(state, routine),
        "PlaceMoneyTopRight" => place_money_top_right(state, routine),
        "DisplayMoneyAndCoinBalance" => display_money_and_coin_balance(state, routine),
        "DisplayCoinCaseBalance" => display_coin_case_balance(state, routine),
        "PrintTodaysLuckyNumber" => print_todays_lucky_number(state, routine),
        "GSHealings" => gs_healings(state, routine),
        "StubbedTrainerRankings_Healings" => trainer_rankings_healings(state, routine),
        "Reset" => reset_special(state, routine),
        "HoOhChamber" => ho_oh_chamber(state, routine),
        "ClearBGPalettesBufferScreen" => graphics_command(
            state,
            routine,
            ScriptGraphicsRuntimeKind::ClearBgPalettesBufferScreen,
        ),
        "ClearBGPalettes" => {
            graphics_command(state, routine, ScriptGraphicsRuntimeKind::ClearBgPalettes)
        }
        "UpdateTimePals" => {
            graphics_command(state, routine, ScriptGraphicsRuntimeKind::UpdateTimePals)
        }
        "ClearTilemap" => graphics_command(state, routine, ScriptGraphicsRuntimeKind::ClearTilemap),
        "LoadMapPalettes" => {
            graphics_command(state, routine, ScriptGraphicsRuntimeKind::LoadMapPalettes)
        }
        "RefreshSprites" => {
            graphics_command(state, routine, ScriptGraphicsRuntimeKind::RefreshSprites)
        }
        "UpdateSprites" => {
            graphics_command(state, routine, ScriptGraphicsRuntimeKind::UpdateSprites)
        }
        "ReloadSpritesNoPalettes" => graphics_command(
            state,
            routine,
            ScriptGraphicsRuntimeKind::ReloadSpritesNoPalettes,
        ),
        "FadeOutToWhite" => screen_fade(
            state,
            routine,
            ScriptFadeColor::White,
            ScriptFadeDirection::Out,
        ),
        "FadeInFromWhite" => screen_fade(
            state,
            routine,
            ScriptFadeColor::White,
            ScriptFadeDirection::In,
        ),
        "FadeOutToBlack" => screen_fade(
            state,
            routine,
            ScriptFadeColor::Black,
            ScriptFadeDirection::Out,
        ),
        "FadeInFromBlack" => screen_fade(
            state,
            routine,
            ScriptFadeColor::Black,
            ScriptFadeDirection::In,
        ),
        "PokemonCenterPC" => pokemon_center_pc(state, routine),
        "PlayersHousePC" => players_house_pc(state, routine),
        "ProfOaksPCBoot" => prof_oaks_pc_boot(state, context.oak_ratings, routine),
        "OverworldTownMap" => overworld_town_map(state, routine),
        "UnownPrinter" => unown_printer(state, routine),
        "MapRadio" => map_radio(state, routine),
        "NameRival" => name_rival(state, routine),
        "MoveDeletion" => move_deletion(state, routine),
        "BattleTowerFade" => {
            visual_command(state, routine, ScriptGraphicsRuntimeKind::BattleTowerFade)
        }
        "UpdatePlayerSprite" => visual_command(
            state,
            routine,
            ScriptGraphicsRuntimeKind::UpdatePlayerSprite,
        ),
        "HealMachineAnim" => {
            visual_command(state, routine, ScriptGraphicsRuntimeKind::HealMachineAnim)
        }
        "SurfStartStep" => visual_command(state, routine, ScriptGraphicsRuntimeKind::SurfStartStep),
        "LoadUsedSpritesGFX" => visual_command(
            state,
            routine,
            ScriptGraphicsRuntimeKind::LoadUsedSpritesGfx,
        ),
        "ToggleMaptileDecorations" => visual_command(
            state,
            routine,
            ScriptGraphicsRuntimeKind::ToggleMaptileDecorations,
        ),
        "ToggleDecorationsVisibility" => visual_command(
            state,
            routine,
            ScriptGraphicsRuntimeKind::ToggleDecorationsVisibility,
        ),
        "MagnetTrain" => visual_command(state, routine, ScriptGraphicsRuntimeKind::MagnetTrain),
        "Diploma" => visual_command(state, routine, ScriptGraphicsRuntimeKind::Diploma),
        "PrintDiploma" => visual_command(state, routine, ScriptGraphicsRuntimeKind::PrintDiploma),
        "UnownPuzzle" => visual_command(state, routine, ScriptGraphicsRuntimeKind::UnownPuzzle),
        "OmanyteChamber" => {
            visual_command(state, routine, ScriptGraphicsRuntimeKind::OmanyteChamber)
        }
        "DisplayUnownWords" => {
            visual_command(state, routine, ScriptGraphicsRuntimeKind::DisplayUnownWords)
        }
        "CheckPokerus" => check_pokerus(state, routine),
        "OlderHaircutBrother" => older_haircut_brother(state, context.happiness_data, routine),
        "YoungerHaircutBrother" => younger_haircut_brother(state, context.happiness_data, routine),
        "DaisysGrooming" => daisys_grooming(state, context.happiness_data, routine),
        "NameRater" => name_rater(state, routine),
        "PokeSeer" => poke_seer(state, routine),
        "MoveTutor" => move_tutor(state, context.move_catalog, routine),
        "BankOfMom" => bank_of_mom(state, routine),
        "SlotMachine" => slot_machine(state, routine),
        "CardFlip" => card_flip(state, routine),
        "DisplayLinkRecord" => display_link_record(state, routine),
        "TrainerHouse" => trainer_house(state, routine),
        "PhotoStudio" => photo_studio(state, routine),
        "GiveShuckle" => give_shuckle(state, context, routine),
        "ReturnShuckie" => return_shuckie(state, context.shuckie_gift, routine),
        "GiveDratini" => give_dratini(
            state,
            context.move_catalog,
            context.dratini_move_sets,
            routine,
        ),
        "BillsGrandfather" => bills_grandfather(state, routine),
        "SelectApricornForKurt" => select_apricorn_for_kurt(
            state,
            context.item_catalog,
            context.kurt_apricorn_recipes,
            routine,
        ),
        "InitRoamMons" => init_roam_mons(
            state,
            context.species_catalog,
            context.roaming_pokemon,
            routine,
        ),
        "CheckMysteryGift" => check_mystery_gift(state, routine),
        "GetMysteryGiftItem" => get_mystery_gift_item(state, context.item_catalog, routine),
        "UnlockMysteryGift" => unlock_mystery_gift(state, routine),
        "BuenasPassword" => buenas_password(state, context.buena_password_categories, routine),
        "BuenaPrize" => buena_prize(state, context.item_catalog, context.buena_prizes, routine),
        "CelebiShrineEvent" => celebi_shrine_event(state, routine),
        "CheckMagikarpLength" => check_magikarp_length(state, context.magikarp_lengths, routine),
        "MagikarpHouseSign" => magikarp_house_sign(state, routine),
        "DayCareMan" => day_care_interaction(state, routine, "man"),
        "DayCareLady" => day_care_interaction(state, routine, "lady"),
        "DayCareManOutside" => day_care_man_outside(state, routine),
        "DayCareMon1" => day_care_mon(state, routine, "man"),
        "DayCareMon2" => day_care_mon(state, routine, "lady"),
        "GiveParkBalls" => give_park_balls(state, context.bug_contest_config, routine),
        "SelectRandomBugContestContestants" => {
            select_random_bug_contest_contestants(state, context.bug_contest_config, routine)
        }
        "ContestDropOffMons" => contest_drop_off_mons(state, routine),
        "ContestReturnMons" => contest_return_mons(state, routine),
        "CheckPartyFullAfterContest" => check_party_full_after_contest(state, routine),
        "BugContestJudging" => bug_contest_judging(state, routine),
        "SetBitsForLinkTradeRequest" => set_bits_for_link_request(state, routine, 1),
        "SetBitsForBattleRequest" => set_bits_for_link_request(state, routine, 2),
        "SetBitsForTimeCapsuleRequest" => set_bits_for_link_request(state, routine, 0),
        "WaitForLinkedFriend" => wait_for_linked_friend(state, routine),
        "CheckLinkTimeout_Receptionist" => check_link_timeout_receptionist(state, routine),
        "CheckBothSelectedSameRoom" => check_both_selected_same_room(state, routine),
        "CloseLink" => close_link(state, routine),
        "WaitForOtherPlayerToExit" => wait_for_other_player_to_exit(state, routine),
        "FailedLinkToPast" => failed_link_to_past(state, routine),
        "TradeCenter" => link_room(state, routine, "TradeCenter", 2),
        "Colosseum" => link_room(state, routine, "Colosseum", 3),
        "EnterTimeCapsule" => link_room(state, routine, "TimeCapsule", 1),
        "TimeCapsule" => time_capsule(state, routine),
        "CheckTimeCapsuleCompatibility" => check_time_capsule_compatibility(state, routine),
        "TryQuickSave" => try_quick_save(state, routine),
        "AskMobileOrCable" => ask_mobile_or_cable(state, routine),
        "CableClubCheckWhichChris" => cable_club_check_which_chris(state, routine),
        "BattleTowerAction" => battle_tower_action(state, context.battle_tower_rules, routine),
        "CheckForBattleTowerRules" => {
            check_for_battle_tower_rules(state, context.battle_tower_rules, routine)
        }
        "BattleTowerRoomMenu" => battle_tower_room_menu(state, routine),
        "BattleTowerBattle" => battle_tower_battle(state, context.battle_tower_rules, routine),
        "BattleTowerMobileError" => battle_tower_mobile_error(state, routine),
        "LoadOpponentTrainerAndPokemonWithOTSprite" => {
            load_opponent_trainer_and_pokemon_with_ot_sprite(state, context, routine)
        }
        "AskRememberPassword" => ask_remember_password(state, routine),
        "Function1700ba" => battle_tower_leaderboard(state, routine),
        "Function1011f1" => mobile_handshake(
            state,
            routine,
            "init",
            MOBILE_LINK_MODE,
            LinkSerialConnectionStatus::NotEstablished,
        ),
        "Function101220" => mobile_session_end(state, routine),
        "Function101225" => mobile_handshake(
            state,
            routine,
            "battle",
            MOBILE_LINK_MODE,
            LinkSerialConnectionStatus::UsingExternalClock,
        ),
        "Function101231" => mobile_handshake(
            state,
            routine,
            "trade",
            MOBILE_LINK_MODE,
            LinkSerialConnectionStatus::UsingExternalClock,
        ),
        "Function103780" => battle_tower_mobile_flag(state, routine, "function103780"),
        "Function1037c2" => battle_tower_mobile_flag(state, routine, "function1037c2"),
        "Function1037eb" => battle_tower_mobile_flag(state, routine, "function1037eb"),
        "Function10383c" => battle_tower_mobile_flag(state, routine, "function10383c"),
        "Function10387b" => battle_tower_mobile_flag(state, routine, "function10387b"),
        "Mobile_SelectThreeMons" => mobile_select_three_mons(state, routine),
        "GiveOddEgg" => give_odd_egg(
            state,
            context.species_catalog,
            context.learnsets,
            context.growth_rates,
            context.move_catalog,
            context.odd_egg_definitions,
            routine,
        ),
        "Menu_ChallengeExplanationCancel" => {
            battle_tower_challenge_explanation_cancel(state, routine)
        }
        "UnusedCheckUnusedTwoDayTimer"
        | "UnusedFindItemInPCOrBag"
        | "UnusedDummySpecial"
        | "UnusedMemoryGame" => inactive_declared_routine(routine),
        "RandomUnseenWildMon"
        | "RandomPhoneWildMon"
        | "RandomPhoneMon"
        | "Function11ac3e"
        | "TradeCornerHoldMon"
        | "Function11b5e8"
        | "Function11b7e5"
        | "Function11b879"
        | "Function11b920"
        | "Function11b93b"
        | "Function170114"
        | "Function1704e1"
        | "UnusedBattleTowerDummySpecial1"
        | "Function11ba38"
        | "Function11c1ab"
        | "Function17d2b6"
        | "Function17d2ce"
        | "Function102142"
        | "UnusedBattleTowerDummySpecial2" => inactive_declared_routine(routine),
        exact => Err(SpecialRoutineError::UnsupportedRoutine {
            routine: exact.to_string(),
        }),
    }
}

fn inactive_declared_routine(routine: &str) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    Err(SpecialRoutineError::InactiveDeclaredRoutine {
        routine: routine.to_string(),
    })
}

fn heal_party(
    state: &mut GameState,
    move_catalog: &BTreeMap<String, Move>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let mut party = state.storage.party.clone();
    let mut healed_slots = Vec::new();
    for (party_slot, slot) in party.pokemon.iter_mut().enumerate() {
        let Some(pokemon) = slot.as_mut() else {
            continue;
        };
        heal_pokemon(pokemon, move_catalog, routine, party_slot)?;
        healed_slots.push(party_slot);
    }
    state.storage.party = party;
    state.sync_party_from_storage();
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::HealParty { healed_slots },
    })
}

fn warp_to_spawn_point(
    state: &mut GameState,
    spawn_points: &BTreeMap<String, RuntimeSpawnPointRef>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    if spawn_points.is_empty() {
        return Err(SpecialRoutineError::MissingRuntimeSpawnPoints {
            routine: routine.to_string(),
        });
    }
    let spawn = resolve_spawn_point(state, spawn_points, routine)?;
    let tile = TilePosition::new(spawn.tile_x, spawn.tile_y);
    state.last_spawn_identifier = Some(spawn.identifier);
    state.overworld = OverworldMemory::Active {
        map_name: spawn.map_name.clone(),
        tile,
        facing: Direction::Down,
        mode: MovementMode::Normal,
    };
    state.script_runtime.pending_script_warp = Some(ScriptWarpRequest {
        target_map: spawn.map_name.clone(),
        tile,
        facing: None,
        source_script: routine.to_string(),
        command_index: 0,
    });
    state.script_runtime.map_events.push(ScriptMapRuntimeEvent {
        command: "special".to_string(),
        kind: ScriptMapRuntimeKind::Warp,
        target_map: Some(spawn.map_name.clone()),
        tile: Some(tile),
        facing: None,
        map_setup: None,
        source_script: routine.to_string(),
        command_index: 0,
    });
    state.script_runtime.variables.insert(
        "wDefaultSpawnpoint".to_string(),
        spawn.identifier.to_string(),
    );
    state
        .script_runtime
        .variables
        .insert("wLastSpawnMapGroup".to_string(), spawn.group_id.to_string());
    state
        .script_runtime
        .variables
        .insert("wLastSpawnMapNumber".to_string(), spawn.map_id.to_string());
    state
        .script_runtime
        .variables
        .insert("wMapGroup".to_string(), spawn.group_id.to_string());
    state
        .script_runtime
        .variables
        .insert("wMapNumber".to_string(), spawn.map_id.to_string());
    state
        .script_runtime
        .variables
        .insert("wXCoord".to_string(), spawn.tile_x.to_string());
    state
        .script_runtime
        .variables
        .insert("wYCoord".to_string(), spawn.tile_y.to_string());
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::WarpToSpawnPoint {
            spawn_identifier: spawn.identifier,
            map_name: spawn.map_name.clone(),
            tile,
        },
    })
}

fn resolve_spawn_point<'a>(
    state: &GameState,
    spawn_points: &'a BTreeMap<String, RuntimeSpawnPointRef>,
    routine: &str,
) -> Result<&'a RuntimeSpawnPointRef, SpecialRoutineError> {
    let group = optional_i16_script_variable(state, routine, "wLastSpawnMapGroup")?.or(
        optional_i16_script_variable(state, routine, "_last_spawn_map_group")?,
    );
    let map_id = optional_i16_script_variable(state, routine, "wLastSpawnMapNumber")?.or(
        optional_i16_script_variable(state, routine, "_last_spawn_map_number")?,
    );
    if let (Some(group_id), Some(map_id)) = (group, map_id)
        && let Some(spawn) = spawn_points
            .values()
            .find(|spawn| spawn.group_id == group_id && spawn.map_id == map_id)
    {
        return Ok(spawn);
    }
    if let Some(spawn_identifier) = state.last_spawn_identifier {
        return spawn_points
            .get(&spawn_identifier.to_string())
            .ok_or_else(|| SpecialRoutineError::UnknownSpawnPoint {
                routine: routine.to_string(),
                spawn_identifier,
            });
    }
    if let (Some(group_id), Some(map_id)) = (group, map_id) {
        return Err(SpecialRoutineError::UnknownSpawnMap {
            routine: routine.to_string(),
            group_id,
            map_id,
        });
    }
    spawn_points
        .get("0")
        .ok_or_else(|| SpecialRoutineError::UnknownSpawnPoint {
            routine: routine.to_string(),
            spawn_identifier: 0,
        })
}

fn fade_out_music(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    const MUSIC_NONE: &str = "MUSIC_NONE";
    const FADE_FRAMES: u16 = 2;
    state.script_runtime.pending_music_fade = Some(ScriptMusicFade {
        audio_id: MUSIC_NONE.to_string(),
        fade_frames: FADE_FRAMES,
        source_script: routine.to_string(),
        command_index: 0,
    });
    state
        .script_runtime
        .audio_events
        .push(ScriptAudioRuntimeEvent {
            command: "special".to_string(),
            kind: ScriptAudioRuntimeKind::FadeMusic,
            audio_id: Some(MUSIC_NONE.to_string()),
            fade_frames: Some(FADE_FRAMES),
            source_script: routine.to_string(),
            command_index: 0,
        });
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::FadeOutMusic {
            audio_id: MUSIC_NONE.to_string(),
            fade_frames: FADE_FRAMES,
        },
    })
}

fn wait_sfx(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    state.script_runtime.waiting_for_sound_effect = true;
    state
        .script_runtime
        .audio_events
        .push(ScriptAudioRuntimeEvent {
            command: "special".to_string(),
            kind: ScriptAudioRuntimeKind::WaitForSoundEffect,
            audio_id: None,
            fade_frames: None,
            source_script: routine.to_string(),
            command_index: 0,
        });
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::WaitSfx,
    })
}

fn play_map_music(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    state.script_runtime.map_music_requested = true;
    state.script_runtime.map_music_restart_disabled = false;
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::PlayMapMusic,
    })
}

fn restart_map_music(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    state.script_runtime.map_music_requested = true;
    state.script_runtime.map_music_restart_disabled = false;
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::RestartMapMusic,
    })
}

fn play_cur_mon_cry(
    state: &mut GameState,
    cry_by_species: &BTreeMap<String, String>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let species = state
        .script_runtime
        .variables
        .get("wCurPartySpecies")
        .cloned()
        .or_else(|| {
            state
                .storage
                .party
                .pokemon
                .iter()
                .flatten()
                .next()
                .map(|pokemon| pokemon.species.id.clone())
        })
        .ok_or_else(|| SpecialRoutineError::MissingCurrentPartySpecies {
            routine: routine.to_string(),
        })?;
    play_cry_for_species(state, cry_by_species, routine, species, true)
}

fn play_slow_cry(
    state: &mut GameState,
    cry_by_species: &BTreeMap<String, String>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let species = required_species_value(state, routine)?;
    play_cry_for_species(state, cry_by_species, routine, species, false)
}

fn play_cry_for_species(
    state: &mut GameState,
    cry_by_species: &BTreeMap<String, String>,
    routine: &str,
    species: String,
    current_mon: bool,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let audio_id = cry_by_species.get(&species).cloned().ok_or_else(|| {
        SpecialRoutineError::MissingCryMetadata {
            routine: routine.to_string(),
            species: species.clone(),
        }
    })?;
    if current_mon {
        state
            .script_runtime
            .variables
            .insert("wCurPartySpecies".to_string(), species.clone());
    }
    state
        .script_runtime
        .audio_events
        .push(ScriptAudioRuntimeEvent {
            command: "special".to_string(),
            kind: ScriptAudioRuntimeKind::Cry,
            audio_id: Some(audio_id.clone()),
            fade_frames: None,
            source_script: routine.to_string(),
            command_index: 0,
        });
    state.script_runtime.waiting_for_sound_effect = false;
    state.script_runtime.last_special_routine = Some(routine.to_string());
    let effect = if current_mon {
        SpecialRoutineEffect::PlayCurMonCry { species, audio_id }
    } else {
        SpecialRoutineEffect::PlaySlowCry { species, audio_id }
    };
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect,
    })
}

fn gameboy_check(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    const TOKEN: &str = "GBCHECK_CGB";
    state.script_runtime.script_value = Some(TOKEN.to_string());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), TOKEN.to_string());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::GameboyCheck {
            token: TOKEN.to_string(),
        },
    })
}

fn mobile_adapter_status(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    const VALUE: &str = "0";
    state.script_runtime.script_value = Some(VALUE.to_string());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), VALUE.to_string());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::MobileAdapterStatus {
            value: VALUE.to_string(),
        },
    })
}

fn get_first_pokemon_happiness(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let (party_slot, pokemon) = state
        .storage
        .party
        .pokemon
        .iter()
        .enumerate()
        .find_map(|(index, slot)| {
            let pokemon = slot.as_ref()?;
            (pokemon.species.id != "EGG").then_some((index, pokemon))
        })
        .ok_or_else(|| SpecialRoutineError::NoNonEggPartyPokemon {
            routine: routine.to_string(),
        })?;
    let species = pokemon.species.id.clone();
    let nickname = pokemon_nickname_or_species(pokemon);
    let happiness = pokemon.happiness;
    state
        .script_runtime
        .named_buffers
        .insert("STRING_BUFFER_3".to_string(), nickname.clone());
    state.script_runtime.script_value = Some(happiness.to_string());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), happiness.to_string());
    state
        .script_runtime
        .variables
        .insert("wCurPartySpecies".to_string(), species.clone());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::FirstPokemonHappiness {
            party_slot,
            species,
            nickname,
            happiness,
        },
    })
}

fn check_first_mon_is_egg(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let pokemon =
        state.storage.party.pokemon[0]
            .as_ref()
            .ok_or_else(|| SpecialRoutineError::EmptyParty {
                routine: routine.to_string(),
            })?;
    let species = pokemon.species.id.clone();
    let nickname = pokemon_nickname_or_species(pokemon);
    let is_egg = species == "EGG";
    let value = if is_egg { "1" } else { "0" };
    state
        .script_runtime
        .named_buffers
        .insert("STRING_BUFFER_3".to_string(), nickname.clone());
    state.script_runtime.script_value = Some(value.to_string());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), value.to_string());
    state
        .script_runtime
        .variables
        .insert("wCurPartySpecies".to_string(), species.clone());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::CheckFirstMonIsEgg {
            species,
            nickname,
            is_egg,
        },
    })
}

fn find_party_mon_that_species(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let species = required_species_value(state, routine)?;
    let found = state.storage.party.pokemon.iter().any(|slot| {
        slot.as_ref()
            .is_some_and(|pokemon| pokemon.species.id == species)
    });
    let value = if found { "1" } else { "0" };
    state.script_runtime.script_value = Some(value.to_string());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), value.to_string());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::FindPartyMonThatSpecies { species, found },
    })
}

fn find_party_mon_that_species_your_trainer_id(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let species = required_species_value(state, routine)?;
    let player_name = state.player_name.clone();
    let player_id = state.player_id;
    let found = state.storage.party.pokemon.iter().any(|slot| {
        slot.as_ref().is_some_and(|pokemon| {
            pokemon_matches_species_and_ot(pokemon, &species, &player_name, player_id)
        })
    });
    set_script_bool_value(state, found);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::FindPartyMonThatSpeciesYourTrainerId {
            species,
            player_name,
            player_id,
            found,
        },
    })
}

fn find_party_mon_above_level(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let level = required_u8_script_value(state, routine)?;
    let species = state
        .storage
        .party
        .pokemon
        .iter()
        .flatten()
        .find(|pokemon| pokemon.species.id != "EGG" && pokemon.level > level)
        .map(|pokemon| pokemon.species.id.clone());
    let found = species.is_some();
    set_script_bool_value(state, found);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::FindPartyMonAboveLevel {
            level,
            found,
            species,
        },
    })
}

fn find_party_mon_at_least_that_happy(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let happiness = required_u8_script_value(state, routine)?;
    let species = state
        .storage
        .party
        .pokemon
        .iter()
        .flatten()
        .find(|pokemon| pokemon.species.id != "EGG" && pokemon.happiness >= happiness)
        .map(|pokemon| pokemon.species.id.clone());
    let found = species.is_some();
    set_script_bool_value(state, found);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::FindPartyMonAtLeastThatHappy {
            happiness,
            found,
            species,
        },
    })
}

fn mon_check(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let species = required_species_value(state, routine)?;
    let player_name = state.player_name.clone();
    let player_id = state.player_id;
    let owned = storage_owns_species_with_ot(state, &species, &player_name, player_id, routine)?;
    set_script_bool_value(state, owned);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::MonCheck {
            species,
            player_name,
            player_id,
            owned,
        },
    })
}

fn beasts_check(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let player_name = state.player_name.clone();
    let player_id = state.player_id;
    let mut missing_species = None;
    for species in ["RAIKOU", "ENTEI", "SUICUNE"] {
        if !storage_owns_species_with_ot(state, species, &player_name, player_id, routine)? {
            missing_species = Some(species.to_string());
            break;
        }
    }
    let owned_all = missing_species.is_none();
    set_script_bool_value(state, owned_all);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::BeastsCheck {
            player_name,
            player_id,
            missing_species,
            owned_all,
        },
    })
}

fn game_corner_prize_mon_check_dex(
    state: &mut GameState,
    species_catalog: &BTreeMap<String, PokemonSpecies>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let species_id = required_species_value(state, routine)?;
    let species = required_species_metadata(species_catalog, routine, &species_id)?;
    let already_caught = state.pokedex.has_caught(&species_id);
    let recorded_caught = if already_caught {
        false
    } else {
        state.pokedex.record_caught(species)
    };
    state
        .script_runtime
        .variables
        .insert("wCurPartySpecies".to_string(), species_id.clone());
    state
        .script_runtime
        .variables
        .insert("wNamedObjectIndex".to_string(), species.int_id.to_string());
    set_script_bool_value(state, !already_caught);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::GameCornerPrizeMonCheckDex {
            species: species_id,
            species_int_id: species.int_id,
            already_caught,
            recorded_caught,
        },
    })
}

fn unused_set_seen_mon(
    state: &mut GameState,
    species_catalog: &BTreeMap<String, PokemonSpecies>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let species_id = required_species_value(state, routine)?;
    let species = required_species_metadata(species_catalog, routine, &species_id)?;
    let newly_seen = state.pokedex.record_seen(species);
    set_script_bool_value(state, newly_seen);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::UnusedSetSeenMon {
            species: species_id,
            species_int_id: species.int_id,
            newly_seen,
        },
    })
}

fn activate_fishing_swarm(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let numeric = required_numeric_script_value(state, routine)?;
    let value = (numeric & 0xff) as u8;
    state.fishing.swarm_flag = value;
    state.script_runtime.script_value = Some(numeric.to_string());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), numeric.to_string());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::ActivateFishingSwarm { value },
    })
}

fn check_caught_celebi(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    const BATTLE_RESULT_CAUGHT_F: u8 = 1 << 6;
    let caught = (state.battle_result & BATTLE_RESULT_CAUGHT_F) != 0;
    let value = if caught { "1" } else { "0" };
    state.script_runtime.script_value = Some(value.to_string());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), value.to_string());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::CheckCaughtCelebi { caught },
    })
}

fn set_player_palette(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let raw_value = required_numeric_script_value(state, routine)?;
    let changed = (raw_value & 0x80) != 0;
    if changed {
        state.player_palette_id = ((raw_value >> 4) & 0x7) as u8;
    }
    let palette_id = state.player_palette_id;
    state.script_runtime.script_value = Some(palette_id.to_string());
    if changed {
        state
            .script_runtime
            .variables
            .insert("_value".to_string(), palette_id.to_string());
    }
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::SetPlayerPalette {
            raw_value,
            palette_id,
            changed,
        },
    })
}

fn snorlax_awake(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    const POKE_FLUTE_CHANNEL: &str = "MUSIC_POKE_FLUTE_CHANNEL";
    let music = state.script_runtime.current_music.clone();
    let tile = match state.overworld {
        crate::state::OverworldMemory::Active { tile, .. } => Some((tile.x, tile.y)),
        crate::state::OverworldMemory::Inactive => None,
    };
    let awake = music.as_deref() == Some(POKE_FLUTE_CHANNEL)
        && tile.is_some_and(|(x, y)| snorlax_tile_is_adjacent(x, y));
    let value = if awake { "1" } else { "0" };
    state.script_runtime.script_value = Some(value.to_string());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), value.to_string());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::SnorlaxAwake { music, tile, awake },
    })
}

fn snorlax_tile_is_adjacent(x: i16, y: i16) -> bool {
    const PROXIMITY_COORDS: &[(i32, i32)] = &[(33, 8), (34, 10), (35, 10), (36, 8), (36, 9)];
    let x = i32::from(x);
    let y = i32::from(y);
    PROXIMITY_COORDS.iter().any(|&(px, py)| px == x && py == y)
        || [1, 3].into_iter().any(|x_offset| {
            [1, 3].into_iter().any(|y_offset| {
                let normalized_x = x - x_offset;
                let normalized_y = y - y_offset;
                normalized_x % 2 == 0
                    && normalized_y % 2 == 0
                    && PROXIMITY_COORDS
                        .iter()
                        .any(|&(px, py)| px == normalized_x / 2 && py == normalized_y / 2)
            })
        })
}

fn set_day_of_week(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    const HEADLESS_DAY_OF_WEEK: u8 = 0;
    state.time.day_of_week = HEADLESS_DAY_OF_WEEK;
    state.time.current_day = HEADLESS_DAY_OF_WEEK;
    state.script_runtime.script_value = Some("1".to_string());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "1".to_string());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::SetDayOfWeek {
            day: HEADLESS_DAY_OF_WEEK,
        },
    })
}

fn initial_set_dst_flag(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    state.time.dst = true;
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::InitialSetDstFlag,
    })
}

fn initial_clear_dst_flag(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    state.time.dst = false;
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::InitialClearDstFlag,
    })
}

fn update_time(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    state.time.update_time_registers();
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::UpdateTime {
            hour: state.time.game_time_hours,
            minute: state.time.game_time_minutes,
            second: state.time.game_time_seconds,
            day_of_week: state.time.day_of_week,
            time_of_day: state.time.time_of_day,
        },
    })
}

fn sample_kenji_break_countdown(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let mut rng = Random::new(state.rng_seed);
    let value = 3 + rng.randrange(4) as u8;
    state.rng_seed = rng.seed();
    state.kenji_break_timer = value;
    state.script_runtime.script_value = Some(value.to_string());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), value.to_string());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::SampleKenjiBreakCountdown {
            value,
            rng_seed_after: state.rng_seed,
        },
    })
}

fn check_lucky_number_show_flag(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let flag = state.lucky_number_show_flag;
    set_script_bool_value(state, flag);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::CheckLuckyNumberShowFlag { flag },
    })
}

fn reset_lucky_number_show_flag(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    state.lucky_number_show_flag = false;
    let lucky_number = ensure_lucky_number(state);
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::ResetLuckyNumberShowFlag {
            lucky_number,
            lucky_number_day: state.time.current_day,
            rng_seed_after: state.rng_seed,
        },
    })
}

fn check_for_lucky_number_winners(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    if state.storage.party.filled_slots() == 0 {
        set_script_numeric_value(state, 0);
        state.script_runtime.last_special_routine = Some(routine.to_string());
        return Ok(SpecialRoutineOutcome {
            routine: routine.to_string(),
            effect: SpecialRoutineEffect::CheckForLuckyNumberWinners {
                lucky_number: state.lucky_id_number,
                tier: 0,
                source: None,
                species: None,
                text_label: None,
            },
        });
    }

    let lucky_number = ensure_lucky_number(state);
    let mut best_tier = 0;
    let mut best_source = None;
    let mut best_species = None;

    for pokemon in state.storage.party.pokemon.iter().flatten() {
        consider_lucky_number_match(
            pokemon,
            LuckyNumberWinnerSource::Party,
            lucky_number,
            &mut best_tier,
            &mut best_source,
            &mut best_species,
        );
    }

    let box_order = lucky_number_pc_box_order(state, routine)?;
    for box_index in box_order {
        let pc_box = &state.storage.pc_boxes[box_index];
        if pc_box.count > MAX_BOX_MONS {
            return Err(SpecialRoutineError::InvalidPcBoxCount {
                routine: routine.to_string(),
                box_index,
                count: pc_box.count,
            });
        }
        for pokemon in pc_box.pokemon.iter().take(pc_box.count).flatten() {
            consider_lucky_number_match(
                pokemon,
                LuckyNumberWinnerSource::Pc,
                lucky_number,
                &mut best_tier,
                &mut best_source,
                &mut best_species,
            );
        }
    }

    let text_label = best_source.map(|source| match source {
        LuckyNumberWinnerSource::Party => "LuckyNumberMatchPartyText".to_string(),
        LuckyNumberWinnerSource::Pc => "LuckyNumberMatchPCText".to_string(),
    });
    if let Some(species) = best_species.as_ref() {
        state
            .script_runtime
            .variables
            .insert("wCurPartySpecies".to_string(), species.clone());
    }
    set_script_numeric_value(state, best_tier);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::CheckForLuckyNumberWinners {
            lucky_number,
            tier: best_tier,
            source: best_source,
            species: best_species,
            text_label,
        },
    })
}

fn ensure_lucky_number(state: &mut GameState) -> u16 {
    let current_day = state.time.current_day;
    if state.lucky_number_day != Some(current_day) {
        let mut rng = Random::new(state.rng_seed);
        let high = rng.randrange(256) as u16;
        let low = rng.randrange(256) as u16;
        state.rng_seed = rng.seed();
        state.lucky_id_number = (high << 8) | low;
        state.lucky_number_day = Some(current_day);
    }
    state.lucky_id_number
}

fn lucky_number_pc_box_order(
    state: &GameState,
    routine: &str,
) -> Result<Vec<usize>, SpecialRoutineError> {
    let box_count = state.storage.pc_boxes.len();
    if box_count == 0 {
        return Ok(Vec::new());
    }
    let current = state.current_pc_box & 0xf;
    if current >= box_count {
        return Err(SpecialRoutineError::InvalidCurrentPcBox {
            routine: routine.to_string(),
            current_pc_box: state.current_pc_box,
            box_count,
        });
    }
    Ok(std::iter::once(current)
        .chain((0..box_count).filter(move |index| *index != current))
        .collect())
}

fn consider_lucky_number_match(
    pokemon: &Pokemon,
    source: LuckyNumberWinnerSource,
    lucky_number: u16,
    best_tier: &mut u8,
    best_source: &mut Option<LuckyNumberWinnerSource>,
    best_species: &mut Option<String>,
) {
    if pokemon.species.id.is_empty() || pokemon.species.id == "EGG" {
        return;
    }
    let tier = lucky_number_tier(pokemon.original_trainer_id, lucky_number);
    if tier == 0 {
        return;
    }
    if *best_tier == 0
        || tier < *best_tier
        || (tier == *best_tier
            && *best_source == Some(LuckyNumberWinnerSource::Party)
            && source == LuckyNumberWinnerSource::Pc)
    {
        *best_tier = tier;
        *best_source = Some(source);
        *best_species = Some(pokemon.species.id.clone());
    }
}

fn lucky_number_tier(trainer_id: u16, lucky_number: u16) -> u8 {
    match lucky_number_suffix_match_len(trainer_id, lucky_number) {
        5.. => 1,
        3..=4 => 2,
        2 => 3,
        _ => 0,
    }
}

fn lucky_number_suffix_match_len(trainer_id: u16, lucky_number: u16) -> u8 {
    let trainer = format!("{:05}", trainer_id);
    let lucky = format!("{:05}", lucky_number);
    trainer
        .bytes()
        .rev()
        .zip(lucky.bytes().rev())
        .take_while(|(left, right)| left == right)
        .count() as u8
}

fn place_money_top_right(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let money = state.money;
    let formatted = format_money(money);
    state
        .script_runtime
        .named_buffers
        .insert("STRING_BUFFER_1".to_string(), formatted.clone());
    state
        .script_runtime
        .money_events
        .push(ScriptMoneyRuntimeEvent {
            command: "special".to_string(),
            kind: ScriptMoneyRuntimeKind::PlaceMoneyTopRight,
            money,
            coins: None,
            source_script: routine.to_string(),
            command_index: 0,
        });
    set_script_u32_value(state, money);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::PlaceMoneyTopRight { money, formatted },
    })
}

fn display_money_and_coin_balance(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let money = state.money;
    let coins = state.coins;
    let formatted_money = format_money(money);
    let formatted_coins = format_coins(coins);
    state
        .script_runtime
        .named_buffers
        .insert("STRING_BUFFER_1".to_string(), formatted_money.clone());
    state
        .script_runtime
        .named_buffers
        .insert("STRING_BUFFER_2".to_string(), formatted_coins.clone());
    state
        .script_runtime
        .money_events
        .push(ScriptMoneyRuntimeEvent {
            command: "special".to_string(),
            kind: ScriptMoneyRuntimeKind::DisplayMoneyAndCoinBalance,
            money,
            coins: Some(coins),
            source_script: routine.to_string(),
            command_index: 0,
        });
    set_script_u32_value(state, money);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::DisplayMoneyAndCoinBalance {
            money,
            coins,
            formatted_money,
            formatted_coins,
        },
    })
}

fn display_coin_case_balance(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let coins = state.coins;
    let formatted_coins = format_coins(coins);
    state
        .script_runtime
        .named_buffers
        .insert("STRING_BUFFER_1".to_string(), formatted_coins.clone());
    state
        .script_runtime
        .money_events
        .push(ScriptMoneyRuntimeEvent {
            command: "special".to_string(),
            kind: ScriptMoneyRuntimeKind::DisplayCoinCaseBalance,
            money: 0,
            coins: Some(coins),
            source_script: routine.to_string(),
            command_index: 0,
        });
    set_script_u32_value(state, u32::from(coins));
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::DisplayCoinCaseBalance {
            coins,
            formatted_coins,
        },
    })
}

fn print_todays_lucky_number(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let lucky_number = ensure_lucky_number(state);
    let formatted = format!("{lucky_number:05}");
    state
        .script_runtime
        .named_buffers
        .insert("STRING_BUFFER_3".to_string(), formatted.clone());
    state.script_runtime.script_value = Some(formatted.clone());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), formatted.clone());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::PrintTodaysLuckyNumber {
            lucky_number,
            formatted,
        },
    })
}

fn gs_healings(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let healings = state.gs_healings;
    set_script_u32_value(state, u32::from(healings));
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::GsHealings { healings },
    })
}

fn trainer_rankings_healings(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let healings = state.trainer_rankings_healings;
    set_script_u32_value(state, u32::from(healings));
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::TrainerRankingsHealings { healings },
    })
}

fn reset_special(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    const VALUE: &str = "$0";
    state.script_runtime.variables.clear();
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), VALUE.to_string());
    state.script_runtime.script_value = Some(VALUE.to_string());
    state.script_runtime.reset_requested = true;
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::Reset {
            value: VALUE.to_string(),
        },
    })
}

fn ho_oh_chamber(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let has_ho_oh = state
        .storage
        .party
        .pokemon
        .iter()
        .flatten()
        .any(|pokemon| pokemon.species.id == "HO_OH");
    let suicune_unleashed = read_event_flag(state, routine, "EVENT_UNLEASHED_SUICUNE")?;
    let raikou_unleashed = read_event_flag(state, routine, "EVENT_UNLEASHED_RAIKOU")?;
    let entei_unleashed = read_event_flag(state, routine, "EVENT_UNLEASHED_ENTEI")?;
    let open = has_ho_oh && suicune_unleashed && raikou_unleashed && entei_unleashed;
    set_script_bool_value(state, open);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::HoOhChamber {
            has_ho_oh,
            suicune_unleashed,
            raikou_unleashed,
            entei_unleashed,
            open,
        },
    })
}

fn pokemon_center_pc(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let party_count = state.storage.party.filled_slots();
    let current_pc_box = state.current_pc_box;
    state.script_runtime.active_menu = Some("PokemonCenterPC".to_string());
    state
        .script_runtime
        .variables
        .insert("_pc_context".to_string(), "PokemonCenterPC".to_string());
    state
        .script_runtime
        .variables
        .insert("_pc_party_count".to_string(), party_count.to_string());
    state
        .script_runtime
        .variables
        .insert("_pc_current_box".to_string(), current_pc_box.to_string());
    set_script_bool_value(state, party_count > 0);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::PokemonCenterPc {
            party_count,
            current_pc_box,
        },
    })
}

fn players_house_pc(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let party_count = state.storage.party.filled_slots();
    state.script_runtime.active_menu = Some("PlayersHousePC".to_string());
    state
        .script_runtime
        .variables
        .insert("_pc_context".to_string(), "PlayersHousePC".to_string());
    state
        .script_runtime
        .variables
        .insert("_pc_party_count".to_string(), party_count.to_string());
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::PlayersHousePc { party_count },
    })
}

fn prof_oaks_pc_boot(
    state: &mut GameState,
    oak_ratings: &[OakRatingEntry],
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let seen_count = state.pokedex.seen_count();
    let caught_count = state.pokedex.caught_count();
    let rating = oak_rating(oak_ratings, caught_count, routine)?;
    let rating_label = rating.text_label.clone();
    state.script_runtime.active_menu = Some("ProfOaksPCBoot".to_string());
    state
        .script_runtime
        .named_buffers
        .insert("STRING_BUFFER_3".to_string(), seen_count.to_string());
    state
        .script_runtime
        .named_buffers
        .insert("STRING_BUFFER_4".to_string(), caught_count.to_string());
    state
        .script_runtime
        .variables
        .insert("_oak_rating_label".to_string(), rating_label.clone());
    state
        .script_runtime
        .variables
        .insert("_oak_seen_count".to_string(), seen_count.to_string());
    state
        .script_runtime
        .variables
        .insert("_oak_owned_count".to_string(), caught_count.to_string());
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::ProfOaksPcBoot {
            seen_count,
            caught_count,
            rating_label,
        },
    })
}

fn overworld_town_map(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let map_name = match &state.overworld {
        crate::state::OverworldMemory::Active { map_name, .. } => Some(map_name.clone()),
        crate::state::OverworldMemory::Inactive => None,
    };
    state.script_runtime.active_menu = Some("OverworldTownMap".to_string());
    if let Some(map_name) = &map_name {
        state
            .script_runtime
            .variables
            .insert("_town_map_current_map".to_string(), map_name.clone());
    }
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::OverworldTownMap { map_name },
    })
}

fn unown_printer(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    state.script_runtime.active_menu = Some("UnownPrinter".to_string());
    state
        .script_runtime
        .variables
        .insert("_unown_printer_unlocked".to_string(), "1".to_string());
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::UnownPrinter { unlocked: true },
    })
}

fn map_radio(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let station = required_string_script_variable(state, routine, "_value")?;
    state.script_runtime.active_menu = Some("MapRadio".to_string());
    state
        .script_runtime
        .variables
        .insert("_map_radio_station".to_string(), station.clone());
    state.script_runtime.script_value = Some(station.clone());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::MapRadio { station },
    })
}

fn name_rival(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let provided = required_string_script_variable(state, routine, "_rival_name")?;
    let rival_name = if provided.chars().all(|value| value == ' ') || provided.is_empty() {
        "SILVER".to_string()
    } else {
        provided
    };
    state
        .script_runtime
        .variables
        .insert("_rival_name".to_string(), rival_name.clone());
    state
        .script_runtime
        .named_buffers
        .insert("STRING_BUFFER_1".to_string(), rival_name.clone());
    state.script_runtime.script_value = Some(rival_name.clone());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::NameRival { rival_name },
    })
}

fn move_deletion(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let party_slot = required_usize_script_variable(state, routine, "_party_slot")?;
    let move_slot = required_usize_script_variable(state, routine, "_move_slot")?;
    let (species, deleted_move, remaining_moves) = {
        let Some(slot) = state.storage.party.pokemon.get_mut(party_slot) else {
            return Err(SpecialRoutineError::InvalidPartySlot {
                routine: routine.to_string(),
                party_slot,
            });
        };
        let Some(pokemon) = slot.as_mut() else {
            return Err(SpecialRoutineError::InvalidPartySlot {
                routine: routine.to_string(),
                party_slot,
            });
        };
        if pokemon.species.id == "EGG" {
            let remaining_moves = pokemon.moves.len();
            ("EGG".to_string(), String::new(), remaining_moves)
        } else {
            if pokemon.moves.len() <= 1 {
                return Err(SpecialRoutineError::CannotDeleteOnlyMove {
                    routine: routine.to_string(),
                    party_slot,
                });
            }
            if move_slot >= pokemon.moves.len() {
                return Err(SpecialRoutineError::InvalidMoveSlot {
                    routine: routine.to_string(),
                    party_slot,
                    move_slot,
                });
            }
            let species = pokemon.species.id.clone();
            let deleted_move = pokemon.moves.remove(move_slot).name;
            let remaining_moves = pokemon.moves.len();
            (species, deleted_move, remaining_moves)
        }
    };
    if species == "EGG" {
        state.sync_party_from_storage();
        set_script_bool_value(state, false);
        state.script_runtime.last_special_routine = Some(routine.to_string());
        return Ok(SpecialRoutineOutcome {
            routine: routine.to_string(),
            effect: SpecialRoutineEffect::MoveDeletion {
                party_slot,
                species,
                deleted_move,
                remaining_moves,
            },
        });
    }
    state.sync_party_from_storage();
    state
        .script_runtime
        .variables
        .insert("_deleted_move".to_string(), deleted_move.clone());
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::MoveDeletion {
            party_slot,
            species,
            deleted_move,
            remaining_moves,
        },
    })
}

fn visual_command(
    state: &mut GameState,
    routine: &str,
    kind: ScriptGraphicsRuntimeKind,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    state.script_runtime.active_menu = Some(routine.to_string());
    state
        .script_runtime
        .variables
        .insert("_visual_special".to_string(), routine.to_string());
    state
        .script_runtime
        .graphics_events
        .push(ScriptGraphicsRuntimeEvent {
            command: "special".to_string(),
            kind,
            color: None,
            direction: None,
            frames: None,
            source_script: routine.to_string(),
            command_index: 0,
        });
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::RuntimeVisualCommand { kind },
    })
}

fn check_pokerus(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    const POKERUS_STATUS: &str = "POKERUS";
    const ENGINE_FLAG: &str = "ENGINE_CAUGHT_POKERUS";
    const SPECIAL_CALL: &str = "SPECIALCALL_POKERUS";
    let already_discovered = state
        .flags
        .is_engine_flag_set(ENGINE_FLAG)
        .map_err(|error| SpecialRoutineError::EventFlag {
            routine: routine.to_string(),
            error,
        })?;
    let found = state.storage.party.pokemon.iter().flatten().any(|pokemon| {
        pokemon.species.id != "EGG" && pokemon.status.as_deref() == Some(POKERUS_STATUS)
    });
    let newly_discovered = found && !already_discovered;
    if newly_discovered {
        state
            .flags
            .set_engine_flag(ENGINE_FLAG, true)
            .map_err(|error| SpecialRoutineError::EventFlag {
                routine: routine.to_string(),
                error,
            })?;
        if !state
            .script_runtime
            .special_phone_calls
            .iter()
            .any(|call| call == SPECIAL_CALL)
        {
            state
                .script_runtime
                .special_phone_calls
                .push(SPECIAL_CALL.to_string());
        }
    }
    set_script_bool_value(state, found);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::CheckPokerus {
            found,
            newly_discovered,
        },
    })
}

fn older_haircut_brother(
    state: &mut GameState,
    happiness_data: Option<&HappinessData>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    apply_happiness_service(state, routine, happiness_data)
}

fn younger_haircut_brother(
    state: &mut GameState,
    happiness_data: Option<&HappinessData>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    apply_happiness_service(state, routine, happiness_data)
}

fn daisys_grooming(
    state: &mut GameState,
    happiness_data: Option<&HappinessData>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    apply_happiness_service(state, routine, happiness_data)
}

fn apply_happiness_service(
    state: &mut GameState,
    routine: &str,
    happiness_data: Option<&HappinessData>,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let happiness_data = require_happiness_data(happiness_data, routine)?;
    let outcomes = happiness_service_outcomes(happiness_data, routine)?;
    let party_slot = required_usize_script_variable(state, routine, "_party_slot")?;
    let roll = optional_u8_script_variable(state, routine, "_rng_roll")?.unwrap_or_else(|| {
        let mut rng = Random::new(state.rng_seed);
        let value = rng.randrange(256) as u8;
        state.rng_seed = rng.seed();
        value
    });
    let selected = select_happiness_service_outcome(outcomes, roll);
    let (species, nickname, old_happiness, new_happiness) = {
        let pokemon = required_party_pokemon_mut(state, routine, party_slot)?;
        let species = pokemon.species.id.clone();
        let nickname = pokemon_nickname_or_species(pokemon);
        let old_happiness = pokemon.happiness;
        let delta = happiness_delta(happiness_data, selected.change_code, old_happiness, routine)?;
        let new_happiness = apply_signed_happiness_delta(old_happiness, delta);
        pokemon.happiness = new_happiness;
        (species, nickname, old_happiness, new_happiness)
    };
    state.sync_party_from_storage();
    state
        .script_runtime
        .named_buffers
        .insert("STRING_BUFFER_3".to_string(), nickname);
    set_script_numeric_value(state, selected.script_value);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::HappinessService {
            party_slot,
            species,
            old_happiness,
            new_happiness,
            script_value: selected.script_value,
            change_code: selected.change_code,
            rng_seed_after: state.rng_seed,
        },
    })
}

fn name_rater(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let party_slot = required_usize_script_variable(state, routine, "_party_slot")?;
    let new_nickname = required_string_script_variable(state, routine, "_selected_nickname")?;
    let (species, old_nickname) = {
        let pokemon = required_party_pokemon_mut(state, routine, party_slot)?;
        let species = pokemon.species.id.clone();
        let old_nickname = pokemon.nickname.clone();
        pokemon.nickname = new_nickname.clone();
        (species, old_nickname)
    };
    state.sync_party_from_storage();
    state
        .script_runtime
        .named_buffers
        .insert("STRING_BUFFER_1".to_string(), new_nickname.clone());
    state.script_runtime.script_value = Some(new_nickname.clone());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::NameRater {
            party_slot,
            species,
            old_nickname,
            new_nickname,
        },
    })
}

fn poke_seer(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let party_slot = required_usize_script_variable(state, routine, "_party_slot")?;
    let pokemon = required_party_pokemon(state, routine, party_slot)?;
    let species = pokemon.species.id.clone();
    let nickname = pokemon_nickname_or_species(pokemon);
    let original_trainer_name = pokemon.original_trainer_name.clone();
    let original_trainer_id = pokemon.original_trainer_id;
    state
        .script_runtime
        .named_buffers
        .insert("STRING_BUFFER_1".to_string(), nickname.clone());
    state
        .script_runtime
        .named_buffers
        .insert("STRING_BUFFER_2".to_string(), original_trainer_name.clone());
    state.script_runtime.variables.insert(
        "_poke_seer_ot_id".to_string(),
        original_trainer_id.to_string(),
    );
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::PokeSeer {
            party_slot,
            species,
            nickname,
            original_trainer_name,
            original_trainer_id,
        },
    })
}

fn move_tutor(
    state: &mut GameState,
    move_catalog: &BTreeMap<String, Move>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let party_slot = required_usize_script_variable(state, routine, "_party_slot")?;
    let move_name = required_string_script_variable(state, routine, "_move")?;
    let move_data =
        move_catalog
            .get(&move_name)
            .ok_or_else(|| SpecialRoutineError::UnknownMove {
                routine: routine.to_string(),
                party_slot,
                move_id: move_name.clone(),
            })?;
    let (species, learned) = {
        let pokemon = required_party_pokemon_mut(state, routine, party_slot)?;
        let species = pokemon.species.id.clone();
        let learned = if pokemon.moves.iter().any(|known| known.name == move_name) {
            false
        } else {
            pokemon.moves.push(LearnedMove {
                name: move_name.clone(),
                current_pp: move_data.pp,
                pp_ups: 0,
            });
            true
        };
        (species, learned)
    };
    state.sync_party_from_storage();
    set_script_bool_value(state, learned);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::MoveTutor {
            party_slot,
            species,
            move_name,
            learned,
        },
    })
}

fn bank_of_mom(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    state.script_runtime.active_menu = Some(routine.to_string());
    state
        .script_runtime
        .variables
        .insert("_bank_money".to_string(), state.money.to_string());
    state
        .script_runtime
        .variables
        .insert("_mom_money".to_string(), state.moms_money.to_string());
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::BankOfMom {
            money: state.money,
            moms_money: state.moms_money,
        },
    })
}

fn slot_machine(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    coin_game_service(state, routine, true)
}

fn card_flip(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    coin_game_service(state, routine, false)
}

fn coin_game_service(
    state: &mut GameState,
    routine: &str,
    slot_machine: bool,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    state.script_runtime.active_menu = Some(routine.to_string());
    state
        .script_runtime
        .variables
        .insert("_coin_case_balance".to_string(), state.coins.to_string());
    set_script_u32_value(state, u32::from(state.coins));
    state.script_runtime.last_special_routine = Some(routine.to_string());
    let effect = if slot_machine {
        SpecialRoutineEffect::SlotMachine { coins: state.coins }
    } else {
        SpecialRoutineEffect::CardFlip { coins: state.coins }
    };
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect,
    })
}

fn display_link_record(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let stats = state.link_battle_stats;
    state.script_runtime.active_menu = Some(routine.to_string());
    state
        .script_runtime
        .variables
        .insert("_link_battle_wins".to_string(), stats.wins.to_string());
    state
        .script_runtime
        .variables
        .insert("_link_battle_losses".to_string(), stats.losses.to_string());
    state
        .script_runtime
        .variables
        .insert("_link_battle_draws".to_string(), stats.draws.to_string());
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::DisplayLinkRecord {
            wins: stats.wins,
            losses: stats.losses,
            draws: stats.draws,
        },
    })
}

fn trainer_house(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let stats = state.link_battle_stats;
    state.script_runtime.active_menu = Some(routine.to_string());
    state
        .script_runtime
        .variables
        .insert("_trainer_house_wins".to_string(), stats.wins.to_string());
    state.script_runtime.variables.insert(
        "_trainer_house_losses".to_string(),
        stats.losses.to_string(),
    );
    state
        .script_runtime
        .variables
        .insert("_trainer_house_draws".to_string(), stats.draws.to_string());
    state.pending_special_battle_type = Some("BATTLETYPE_TRAINER_HOUSE".to_string());
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::TrainerHouse {
            wins: stats.wins,
            losses: stats.losses,
            draws: stats.draws,
        },
    })
}

fn photo_studio(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let selected_slot = optional_usize_script_variable(state, routine, "_party_slot")?;
    let party_slot = selected_slot.or_else(|| {
        state
            .storage
            .party
            .pokemon
            .iter()
            .position(|slot| slot.is_some())
    });
    let species = party_slot.and_then(|slot| {
        state.storage.party.pokemon[slot]
            .as_ref()
            .map(|pokemon| pokemon.species.id.clone())
    });
    state.script_runtime.active_menu = Some(routine.to_string());
    if let Some(species) = &species {
        state.script_runtime.active_pokemon_picture = Some(species.clone());
        state
            .script_runtime
            .named_buffers
            .insert("STRING_BUFFER_1".to_string(), species.clone());
    }
    set_script_bool_value(state, species.is_some());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::PhotoStudio {
            party_slot,
            species,
        },
    })
}

fn battle_tower_challenge_explanation_cancel(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    state.battle_tower.explanation_read = false;
    state.script_runtime.active_menu = Some(routine.to_string());
    set_script_bool_value(state, false);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::BattleTowerChallengeExplanationCancel,
    })
}

fn give_shuckle(
    state: &mut GameState,
    context: SpecialRoutineContext<'_>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let gift = context
        .shuckie_gift
        .ok_or_else(|| SpecialRoutineError::MissingShuckieGift {
            routine: routine.to_string(),
        })?;
    let species = required_species_metadata(context.species_catalog, routine, &gift.species)?;
    if !context.item_catalog.contains_key(&gift.held_item) {
        return Err(SpecialRoutineError::UnknownItem {
            routine: routine.to_string(),
            item_id: gift.held_item.clone(),
        });
    }
    let dvs = sample_dvs(state);
    let mut pokemon = create_pokemon_from_known_dvs(
        species,
        gift.level,
        dvs,
        context.learnsets,
        context.move_catalog,
        context.growth_rates,
    )
    .map_err(|error| SpecialRoutineError::GiftPokemonBuild {
        routine: routine.to_string(),
        error: error.to_string(),
    })?;
    pokemon.item = Some(gift.held_item.clone());
    pokemon.nickname = gift.nickname.clone();
    pokemon.original_trainer_name = gift.original_trainer_name.clone();
    pokemon.original_trainer_id = gift.original_trainer_id;

    if !state.storage.party.add_pokemon(pokemon) {
        set_script_bool_value(state, false);
        state.script_runtime.last_special_routine = Some(routine.to_string());
        return Ok(SpecialRoutineOutcome {
            routine: routine.to_string(),
            effect: SpecialRoutineEffect::GiveShuckle {
                stored: false,
                rng_seed_after: state.rng_seed,
            },
        });
    }

    state.sync_party_from_storage();
    state
        .flags
        .set_engine_flag(&gift.got_today_engine_flag, true)
        .map_err(|error| SpecialRoutineError::EventFlag {
            routine: routine.to_string(),
            error,
        })?;
    state
        .script_runtime
        .variables
        .insert("wCurPartySpecies".to_string(), gift.species.clone());
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::GiveShuckle {
            stored: true,
            rng_seed_after: state.rng_seed,
        },
    })
}

fn return_shuckie(
    state: &mut GameState,
    shuckie_gift: Option<&ShuckieGiftDefinition>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    const SHUCKIE_WRONG_MON: u8 = 0;
    const SHUCKIE_REFUSED: u8 = 1;
    const SHUCKIE_RETURNED: u8 = 2;
    const SHUCKIE_HAPPY: u8 = 3;
    const SHUCKIE_FAINTED: u8 = 4;
    let gift = shuckie_gift.ok_or_else(|| SpecialRoutineError::MissingShuckieGift {
        routine: routine.to_string(),
    })?;

    let selection_cancelled = state
        .script_runtime
        .variables
        .get("_selection_cancelled")
        .is_some_and(|value| value == "1" || value == "true");
    let party_slot = if selection_cancelled {
        0
    } else {
        required_selected_party_slot(state, routine)?
    };

    let result = if selection_cancelled {
        SHUCKIE_REFUSED
    } else if party_slot >= state.storage.party.pokemon.len() {
        SHUCKIE_REFUSED
    } else {
        let Some(mon) = state.storage.party.pokemon[party_slot].as_ref() else {
            set_script_numeric_value(state, SHUCKIE_REFUSED);
            state.script_runtime.last_special_routine = Some(routine.to_string());
            return Ok(SpecialRoutineOutcome {
                routine: routine.to_string(),
                effect: SpecialRoutineEffect::ReturnShuckie {
                    party_slot: Some(party_slot),
                    result: SHUCKIE_REFUSED,
                },
            });
        };
        state
            .script_runtime
            .variables
            .insert("wCurPartySpecies".to_string(), mon.species.id.clone());
        if mon.species.id != gift.species
            || mon.original_trainer_id != gift.original_trainer_id
            || mon.original_trainer_name != gift.original_trainer_name
        {
            SHUCKIE_WRONG_MON
        } else if mon.hp == 0 {
            SHUCKIE_FAINTED
        } else if mon.happiness >= 150 {
            SHUCKIE_HAPPY
        } else {
            remove_party_member(state, party_slot);
            SHUCKIE_RETURNED
        }
    };

    set_script_numeric_value(state, result);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::ReturnShuckie {
            party_slot: (!selection_cancelled).then_some(party_slot),
            result,
        },
    })
}

fn give_dratini(
    state: &mut GameState,
    move_catalog: &BTreeMap<String, Move>,
    move_sets: &[DratiniMoveSetDefinition],
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    if move_sets.is_empty() {
        return Err(SpecialRoutineError::MissingDratiniMoveSets {
            routine: routine.to_string(),
        });
    }
    let mode = required_u8_script_value(state, routine)?;
    let Some(move_set) = move_sets.iter().find(|move_set| move_set.mode == mode) else {
        set_script_numeric_value(state, mode);
        state.script_runtime.last_special_routine = Some(routine.to_string());
        return Ok(SpecialRoutineOutcome {
            routine: routine.to_string(),
            effect: SpecialRoutineEffect::GiveDratini {
                party_slot: None,
                mode,
                move_names: Vec::new(),
                learned: false,
            },
        });
    };

    let party_slot = state
        .storage
        .party
        .pokemon
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, slot)| {
            let mon = slot.as_ref()?;
            (mon.species.id == "DRATINI").then_some(index)
        });
    let Some(party_slot) = party_slot else {
        set_script_numeric_value(state, mode);
        state.script_runtime.last_special_routine = Some(routine.to_string());
        return Ok(SpecialRoutineOutcome {
            routine: routine.to_string(),
            effect: SpecialRoutineEffect::GiveDratini {
                party_slot: None,
                mode,
                move_names: Vec::new(),
                learned: false,
            },
        });
    };

    let mut learned = Vec::with_capacity(move_set.moves.len());
    for move_name in &move_set.moves {
        let move_data =
            move_catalog
                .get(move_name)
                .ok_or_else(|| SpecialRoutineError::UnknownMove {
                    routine: routine.to_string(),
                    party_slot,
                    move_id: move_name.clone(),
                })?;
        learned.push(LearnedMove {
            name: move_name.clone(),
            current_pp: move_data.pp,
            pp_ups: 0,
        });
    }
    let pokemon = required_party_pokemon_mut(state, routine, party_slot)?;
    pokemon.moves = learned;
    state
        .script_runtime
        .variables
        .insert("wCurPartySpecies".to_string(), "DRATINI".to_string());
    set_script_numeric_value(state, mode);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::GiveDratini {
            party_slot: Some(party_slot),
            mode,
            move_names: move_set.moves.clone(),
            learned: true,
        },
    })
}

fn bills_grandfather(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let manual_species = state
        .script_runtime
        .variables
        .get("_selected_species")
        .cloned();
    let (party_slot, species) = if let Some(species) = manual_species {
        (None, Some(species))
    } else {
        let party_slot = required_selected_party_slot(state, routine)?;
        let species = state
            .storage
            .party
            .pokemon
            .get(party_slot)
            .and_then(Option::as_ref)
            .map(|pokemon| pokemon.species.id.clone());
        (Some(party_slot), species)
    };

    if let Some(species) = species.as_ref() {
        state
            .script_runtime
            .variables
            .insert("wCurPartySpecies".to_string(), species.clone());
        state
            .script_runtime
            .named_buffers
            .insert("STRING_BUFFER_1".to_string(), species.replace('_', " "));
        state
            .script_runtime
            .named_buffers
            .insert("STRING_BUFFER_3".to_string(), species.replace('_', " "));
        state
            .script_runtime
            .variables
            .insert("wNamedObjectIndex".to_string(), species.clone());
        state.script_runtime.script_value = Some(species.clone());
        state
            .script_runtime
            .variables
            .insert("_value".to_string(), species.clone());
    } else {
        set_script_numeric_value(state, 0);
    }
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::BillsGrandfather {
            party_slot,
            species,
        },
    })
}

fn select_apricorn_for_kurt(
    state: &mut GameState,
    item_catalog: &BTreeMap<String, Item>,
    recipes: &[KurtApricornRecipe],
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    if recipes.is_empty() {
        return Err(SpecialRoutineError::MissingKurtApricornRecipes {
            routine: routine.to_string(),
        });
    }
    let selected = state
        .script_runtime
        .variables
        .get("_kurt_apricorn_type")
        .cloned()
        .or_else(|| {
            recipes
                .iter()
                .find(|recipe| bag_quantity_by_id(state, item_catalog, &recipe.apricorn) > 0)
                .map(|recipe| recipe.apricorn.clone())
        });
    let Some(apricorn) = selected else {
        set_script_bool_value(state, false);
        state.script_runtime.last_special_routine = Some(routine.to_string());
        return Ok(SpecialRoutineOutcome {
            routine: routine.to_string(),
            effect: SpecialRoutineEffect::SelectApricornForKurt {
                apricorn: None,
                quantity: 0,
            },
        });
    };
    if !recipes.iter().any(|recipe| recipe.apricorn == apricorn) {
        return Err(SpecialRoutineError::UnknownItem {
            routine: routine.to_string(),
            item_id: apricorn,
        });
    }
    let item = item_catalog
        .get(&apricorn)
        .ok_or_else(|| SpecialRoutineError::UnknownItem {
            routine: routine.to_string(),
            item_id: apricorn.clone(),
        })?;
    let quantity = required_u16_script_variable(state, routine, "_kurt_apricorn_quantity")?;
    let removed = state.bag.remove_item(item, quantity).map_err(|error| {
        SpecialRoutineError::GiftPokemonBuild {
            routine: routine.to_string(),
            error,
        }
    })?;
    if !removed {
        set_script_bool_value(state, false);
        state.script_runtime.last_special_routine = Some(routine.to_string());
        return Ok(SpecialRoutineOutcome {
            routine: routine.to_string(),
            effect: SpecialRoutineEffect::SelectApricornForKurt {
                apricorn: None,
                quantity: 0,
            },
        });
    }

    state
        .script_runtime
        .variables
        .insert("_kurt_apricorn_type".to_string(), apricorn.clone());
    state
        .script_runtime
        .variables
        .insert("_kurt_apricorn_quantity".to_string(), quantity.to_string());
    state
        .script_runtime
        .variables
        .insert("VAR_KURT_APRICORNS".to_string(), quantity.to_string());
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::SelectApricornForKurt {
            apricorn: Some(apricorn),
            quantity,
        },
    })
}

fn init_roam_mons(
    state: &mut GameState,
    species_catalog: &BTreeMap<String, PokemonSpecies>,
    definitions: &[RoamingPokemonDefinition],
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    if definitions.is_empty() {
        return Err(SpecialRoutineError::MissingRoamingPokemonDefinitions {
            routine: routine.to_string(),
        });
    }
    let mut roamers = Vec::with_capacity(definitions.len());
    for definition in definitions {
        required_species_metadata(species_catalog, routine, &definition.species)?;
        roamers.push(RoamingPokemonState {
            species: definition.species.clone(),
            level: definition.level,
            map_group: definition.map_group,
            map_number: definition.map_number,
            hp: 0,
            dvs: 0,
        });
    }
    state.roaming_pokemon = roamers.clone();
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::InitRoamMons { roamers },
    })
}

fn check_mystery_gift(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let has_pending_item = state.mystery_gift.stored_item.is_some();
    set_script_bool_value(state, has_pending_item);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::CheckMysteryGift { has_pending_item },
    })
}

fn get_mystery_gift_item(
    state: &mut GameState,
    item_catalog: &BTreeMap<String, Item>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let Some(item_id) = state.mystery_gift.stored_item.clone() else {
        set_script_bool_value(state, false);
        state.script_runtime.last_special_routine = Some(routine.to_string());
        return Ok(SpecialRoutineOutcome {
            routine: routine.to_string(),
            effect: SpecialRoutineEffect::GetMysteryGiftItem {
                item_id: None,
                received: false,
            },
        });
    };
    let item = item_catalog
        .get(&item_id)
        .ok_or_else(|| SpecialRoutineError::UnknownItem {
            routine: routine.to_string(),
            item_id: item_id.clone(),
        })?;
    let added =
        state
            .bag
            .add_item(item, 1)
            .map_err(|error| SpecialRoutineError::GiftPokemonBuild {
                routine: routine.to_string(),
                error,
            })?;
    if !added {
        set_script_bool_value(state, false);
        state.script_runtime.last_special_routine = Some(routine.to_string());
        return Ok(SpecialRoutineOutcome {
            routine: routine.to_string(),
            effect: SpecialRoutineEffect::GetMysteryGiftItem {
                item_id: Some(item_id),
                received: false,
            },
        });
    }

    state.mystery_gift.stored_item = None;
    state.mystery_gift.backup_item = None;
    state
        .script_runtime
        .named_buffers
        .insert("STRING_BUFFER_1".to_string(), item.name.clone());
    state
        .script_runtime
        .audio_events
        .push(ScriptAudioRuntimeEvent {
            command: "special".to_string(),
            kind: ScriptAudioRuntimeKind::SoundEffect,
            audio_id: Some("SFX_ITEM".to_string()),
            fade_frames: None,
            source_script: routine.to_string(),
            command_index: 0,
        });
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::GetMysteryGiftItem {
            item_id: Some(item_id),
            received: true,
        },
    })
}

fn unlock_mystery_gift(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let newly_unlocked = !state.mystery_gift_unlocked;
    if newly_unlocked {
        state.mystery_gift_unlocked = true;
        state.mystery_gift.stored_item = None;
        state.mystery_gift.backup_item = None;
    }
    set_script_bool_value(state, newly_unlocked);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::UnlockMysteryGift { newly_unlocked },
    })
}

fn buenas_password(
    state: &mut GameState,
    categories: &[BuenaPasswordCategoryDefinition],
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let (category, correct) = ensure_buenas_password(state, categories, routine)?;
    let guess = state
        .script_runtime
        .variables
        .get("BUENA_PASSWORD")
        .cloned()
        .or_else(|| {
            state
                .script_runtime
                .variables
                .get("_selected_password")
                .cloned()
        });
    let matched = guess.as_deref() == Some(correct.as_str());
    set_script_bool_value(state, matched);
    state
        .script_runtime
        .variables
        .insert("_buena_category".to_string(), category.id.clone());
    state.script_runtime.variables.insert(
        "_buena_category_type".to_string(),
        category.category_type.clone(),
    );
    state
        .script_runtime
        .variables
        .insert("_buena_password".to_string(), correct.to_string());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::BuenasPassword {
            category: category.id.clone(),
            category_type: category.category_type.clone(),
            correct,
            guess,
            matched,
            rng_seed_after: state.rng_seed,
        },
    })
}

fn buena_prize(
    state: &mut GameState,
    item_catalog: &BTreeMap<String, Item>,
    buena_prizes: &[BuenaPrizeDefinition],
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    if buena_prizes.is_empty() {
        return Err(SpecialRoutineError::MissingBuenaPrizeDefinitions {
            routine: routine.to_string(),
        });
    }
    let selected = required_string_script_variable(state, routine, "_selected_prize")?;
    let quantity = optional_u16_script_variable(state, routine, "_selected_prize_quantity")?
        .ok_or_else(|| SpecialRoutineError::MissingScriptValue {
            routine: routine.to_string(),
            variable: "_selected_prize_quantity".to_string(),
        })?;
    let Some(prize) = buena_prizes.iter().find(|prize| prize.item_id == selected) else {
        return Err(SpecialRoutineError::UnknownItem {
            routine: routine.to_string(),
            item_id: selected,
        });
    };
    let points_spent = prize.cost.checked_mul(quantity as u8).ok_or_else(|| {
        SpecialRoutineError::InvalidNumericValue {
            routine: routine.to_string(),
            value: quantity.to_string(),
        }
    })?;
    if points_spent > state.blue_card_balance {
        set_script_bool_value(state, false);
        state.script_runtime.last_special_routine = Some(routine.to_string());
        return Ok(SpecialRoutineOutcome {
            routine: routine.to_string(),
            effect: SpecialRoutineEffect::BuenaPrize {
                item_id: prize.item_id.clone(),
                quantity,
                points_spent,
                balance: state.blue_card_balance,
            },
        });
    }
    let item =
        item_catalog
            .get(&prize.item_id)
            .ok_or_else(|| SpecialRoutineError::UnknownItem {
                routine: routine.to_string(),
                item_id: prize.item_id.clone(),
            })?;
    let added = state.bag.add_item(item, quantity).map_err(|error| {
        SpecialRoutineError::GiftPokemonBuild {
            routine: routine.to_string(),
            error,
        }
    })?;
    if !added {
        set_script_bool_value(state, false);
        state.script_runtime.last_special_routine = Some(routine.to_string());
        return Ok(SpecialRoutineOutcome {
            routine: routine.to_string(),
            effect: SpecialRoutineEffect::BuenaPrize {
                item_id: prize.item_id.clone(),
                quantity,
                points_spent,
                balance: state.blue_card_balance,
            },
        });
    }
    state.blue_card_balance -= points_spent;
    state.script_runtime.variables.insert(
        "_blue_card_balance".to_string(),
        state.blue_card_balance.to_string(),
    );
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::BuenaPrize {
            item_id: prize.item_id.clone(),
            quantity,
            points_spent,
            balance: state.blue_card_balance,
        },
    })
}

fn celebi_shrine_event(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    const BATTLE_TYPE: &str = "BATTLETYPE_CELEBI";
    state.pending_special_battle_type = Some(BATTLE_TYPE.to_string());
    state
        .script_runtime
        .variables
        .insert("battle_type".to_string(), BATTLE_TYPE.to_string());
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::CelebiShrineEvent {
            battle_type: BATTLE_TYPE.to_string(),
        },
    })
}

fn check_magikarp_length(
    state: &mut GameState,
    magikarp_lengths: &[MagikarpLengthEntry],
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    const NOT_MAGIKARP: u8 = 0;
    const REFUSED: u8 = 1;
    const TOO_SHORT: u8 = 2;
    const BEAT_RECORD: u8 = 3;

    if state
        .script_runtime
        .variables
        .get("_selection_cancelled")
        .is_some_and(|value| value == "1" || value == "true")
    {
        set_script_numeric_value(state, REFUSED);
        state.script_runtime.last_special_routine = Some(routine.to_string());
        return Ok(SpecialRoutineOutcome {
            routine: routine.to_string(),
            effect: SpecialRoutineEffect::CheckMagikarpLength {
                party_slot: 0,
                species: String::new(),
                feet: 0,
                inches: 0,
                result: REFUSED,
            },
        });
    }

    let party_slot = required_selected_party_slot(state, routine)?;
    let pokemon = required_party_pokemon(state, routine, party_slot)?;
    let species = pokemon.species.id.clone();
    if species != "MAGIKARP" {
        set_script_numeric_value(state, NOT_MAGIKARP);
        state.script_runtime.last_special_routine = Some(routine.to_string());
        return Ok(SpecialRoutineOutcome {
            routine: routine.to_string(),
            effect: SpecialRoutineEffect::CheckMagikarpLength {
                party_slot,
                species,
                feet: 0,
                inches: 0,
                result: NOT_MAGIKARP,
            },
        });
    }

    let (feet, inches) =
        calculate_magikarp_length(pokemon, state.player_id, magikarp_lengths, routine)?;
    let owner_name = pokemon.original_trainer_name.clone();
    state.magikarp_record.current_feet = feet;
    state.magikarp_record.current_inches = inches;
    let formatted = format_magikarp_length(feet, inches);
    state
        .script_runtime
        .named_buffers
        .insert("STRING_BUFFER_1".to_string(), formatted);

    let result = if feet > state.magikarp_record.best_feet
        || (feet == state.magikarp_record.best_feet && inches > state.magikarp_record.best_inches)
    {
        state.magikarp_record.best_feet = feet;
        state.magikarp_record.best_inches = inches;
        state.magikarp_record.best_owner_name = owner_name;
        BEAT_RECORD
    } else {
        TOO_SHORT
    };
    set_script_numeric_value(state, result);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::CheckMagikarpLength {
            party_slot,
            species,
            feet,
            inches,
            result,
        },
    })
}

fn magikarp_house_sign(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let feet = state.magikarp_record.best_feet;
    let inches = state.magikarp_record.best_inches;
    state.magikarp_record.current_feet = feet;
    state.magikarp_record.current_inches = inches;
    let formatted = format_magikarp_length(feet, inches);
    state
        .script_runtime
        .named_buffers
        .insert("STRING_BUFFER_1".to_string(), formatted.clone());
    state.script_runtime.script_value = Some(formatted.clone());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), formatted.clone());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::MagikarpHouseSign {
            feet,
            inches,
            formatted,
        },
    })
}

fn day_care_interaction(
    state: &mut GameState,
    routine: &str,
    caretaker: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let action = required_string_script_variable(state, routine, "_day_care_action")?;
    let outcome = match action.as_str() {
        "deposit" => {
            set_day_care_active(state, routine, caretaker, true)?;
            day_care_deposit(state, routine, caretaker)?
        }
        "withdraw" => {
            set_day_care_active(state, routine, caretaker, true)?;
            day_care_withdraw(state, routine, caretaker)?
        }
        "inspect" => {
            set_day_care_active(state, routine, caretaker, true)?;
            day_care_inspect_interaction(state, routine, caretaker)?
        }
        exact => {
            return Err(SpecialRoutineError::MissingScriptValue {
                routine: routine.to_string(),
                variable: format!("unsupported _day_care_action {exact}"),
            });
        }
    };
    set_script_bool_value(state, outcome.success);
    state.day_care.last_interaction = Some(outcome.clone());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::DayCareInteraction {
            caretaker: outcome.caretaker,
            action: outcome.action,
            success: outcome.success,
            pokemon: outcome.pokemon,
        },
    })
}

fn day_care_man_outside(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    set_day_care_active(state, routine, "man", true)?;
    let success = state.day_care.egg_present;
    let outcome = crate::state::DayCareInteractionState {
        caretaker: "man".to_string(),
        action: "collect_egg".to_string(),
        success,
        pokemon: None,
        level: None,
        reason: (!success).then(|| "no_egg".to_string()),
    };
    state.day_care.last_interaction = Some(outcome.clone());
    state.script_runtime.script_value = Some(if success { "FALSE" } else { "TRUE" }.to_string());
    state.script_runtime.variables.insert(
        "_value".to_string(),
        state
            .script_runtime
            .script_value
            .clone()
            .expect("script value"),
    );
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::DayCareInteraction {
            caretaker: outcome.caretaker,
            action: outcome.action,
            success: outcome.success,
            pokemon: outcome.pokemon,
        },
    })
}

fn day_care_mon(
    state: &mut GameState,
    routine: &str,
    caretaker: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let resident = day_care_resident(state, routine, caretaker)?;
    let pokemon = resident
        .pokemon
        .as_ref()
        .map(|pokemon| pokemon.species.id.clone());
    let level = resident.pokemon.as_ref().map(|pokemon| pokemon.level);
    let occupied = pokemon.is_some();
    state.script_runtime.named_buffers.insert(
        "STRING_BUFFER_1".to_string(),
        pokemon.clone().unwrap_or_default(),
    );
    set_script_bool_value(state, occupied);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::DayCareMon {
            caretaker: caretaker.to_string(),
            occupied,
            pokemon,
            level,
        },
    })
}

fn give_park_balls(
    state: &mut GameState,
    bug_contest_config: Option<&BugContestConfig>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let config = require_bug_contest_config(bug_contest_config, routine)?;
    state.bug_contest.park_balls_remaining = config.park_balls;
    state.bug_contest.caught_mon = None;
    state.bug_contest.caught_species = None;
    state.bug_contest.caught_level = None;
    state.bug_contest.party_backup.clear();
    state.bug_contest.timer_active = true;
    state.bug_contest.timer_minutes_remaining = config.timer_minutes;
    state.bug_contest.timer_seconds_remaining = config.timer_seconds;
    state.bug_contest.last_rank = None;
    state.bug_contest.last_result = None;
    state
        .flags
        .set_engine_flag("ENGINE_BUG_CONTEST_TIMER", true)
        .map_err(|error| SpecialRoutineError::EventFlag {
            routine: routine.to_string(),
            error,
        })?;
    set_script_numeric_value(state, config.park_balls);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::GiveParkBalls {
            balls: config.park_balls,
        },
    })
}

fn select_random_bug_contest_contestants(
    state: &mut GameState,
    bug_contest_config: Option<&BugContestConfig>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let config = require_bug_contest_config(bug_contest_config, routine)?;
    let mut rng = Random::new(state.rng_seed);
    let mut chosen = Vec::new();
    while chosen.len() < config.selected_contestant_count {
        let candidate = rng.randrange(config.contestant_flags.len() as u32) as usize;
        if !chosen.contains(&candidate) {
            chosen.push(candidate);
        }
    }
    chosen.sort_unstable();
    let mut flags = Vec::with_capacity(chosen.len());
    for flag in &config.contestant_flags {
        state.flags.set_event_flag(flag, false).map_err(|error| {
            SpecialRoutineError::EventFlag {
                routine: routine.to_string(),
                error,
            }
        })?;
    }
    for index in chosen {
        let flag = &config.contestant_flags[index];
        state
            .flags
            .set_event_flag(flag, true)
            .map_err(|error| SpecialRoutineError::EventFlag {
                routine: routine.to_string(),
                error,
            })?;
        flags.push(flag.to_string());
    }
    state.rng_seed = rng.seed();
    state.bug_contest.selected_contestant_flags = flags.clone();
    state
        .script_runtime
        .variables
        .insert("_bug_contestant_flags".to_string(), flags.join(","));
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::SelectRandomBugContestContestants {
            flags,
            rng_seed_after: state.rng_seed,
        },
    })
}

fn require_bug_contest_config<'a>(
    bug_contest_config: Option<&'a BugContestConfig>,
    routine: &str,
) -> Result<&'a BugContestConfig, SpecialRoutineError> {
    let config =
        bug_contest_config.ok_or_else(|| SpecialRoutineError::MissingBugContestConfig {
            routine: routine.to_string(),
        })?;
    if config.park_balls == 0 {
        return Err(SpecialRoutineError::InvalidBugContestConfig {
            routine: routine.to_string(),
            message: "park_balls must be positive".to_string(),
        });
    }
    if config.timer_seconds > 59 {
        return Err(SpecialRoutineError::InvalidBugContestConfig {
            routine: routine.to_string(),
            message: format!(
                "timer_seconds must be 0..=59, found {}",
                config.timer_seconds
            ),
        });
    }
    if config.selected_contestant_count == 0 {
        return Err(SpecialRoutineError::InvalidBugContestConfig {
            routine: routine.to_string(),
            message: "selected_contestant_count must be positive".to_string(),
        });
    }
    if config.contestant_flags.len() < config.selected_contestant_count {
        return Err(SpecialRoutineError::InvalidBugContestConfig {
            routine: routine.to_string(),
            message: format!(
                "selected_contestant_count {} exceeds {} contestant flags",
                config.selected_contestant_count,
                config.contestant_flags.len()
            ),
        });
    }
    if config
        .contestant_flags
        .iter()
        .any(|flag| flag.trim().is_empty() || flag.trim() != flag)
    {
        return Err(SpecialRoutineError::InvalidBugContestConfig {
            routine: routine.to_string(),
            message: "contestant_flags must be exact non-empty ids".to_string(),
        });
    }
    let mut unique_flags = BTreeSet::new();
    if config
        .contestant_flags
        .iter()
        .any(|flag| !unique_flags.insert(flag))
    {
        return Err(SpecialRoutineError::InvalidBugContestConfig {
            routine: routine.to_string(),
            message: "contestant_flags must not contain duplicates".to_string(),
        });
    }
    Ok(config)
}

fn contest_drop_off_mons(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let lead = required_party_pokemon(state, routine, 0)?.clone();
    if lead.hp == 0 {
        set_script_numeric_value(state, 1);
        state.bug_contest.last_result = Some(1);
        state.script_runtime.last_special_routine = Some(routine.to_string());
        return Ok(SpecialRoutineOutcome {
            routine: routine.to_string(),
            effect: SpecialRoutineEffect::ContestDropOffMons {
                result: 1,
                backup_count: 0,
                second_party_species: None,
            },
        });
    }
    let backup: Vec<Pokemon> = state
        .storage
        .party
        .pokemon
        .iter()
        .skip(1)
        .filter_map(Clone::clone)
        .collect();
    let second_party_species = backup.first().map(|pokemon| pokemon.species.id.clone());
    state.bug_contest.party_backup = backup;
    state.bug_contest.second_party_species = second_party_species.clone();
    state.bug_contest.caught_mon = None;
    state.bug_contest.caught_species = None;
    state.bug_contest.caught_level = None;
    state.storage.party.pokemon = [const { None }; 6];
    state.storage.party.pokemon[0] = Some(lead);
    state.sync_party_from_storage();
    set_script_numeric_value(state, 0);
    state.bug_contest.last_result = Some(0);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::ContestDropOffMons {
            result: 0,
            backup_count: state.bug_contest.party_backup.len(),
            second_party_species,
        },
    })
}

fn contest_return_mons(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let lead = required_party_pokemon(state, routine, 0)?.clone();
    let mut restored = vec![lead];
    restored.extend(state.bug_contest.party_backup.clone());
    state.storage.party.pokemon = [const { None }; 6];
    for (index, pokemon) in restored.into_iter().take(6).enumerate() {
        state.storage.party.pokemon[index] = Some(pokemon);
    }
    state.bug_contest.party_backup.clear();
    state.bug_contest.second_party_species = None;
    state.sync_party_from_storage();
    let restored_count = state.storage.party.filled_slots();
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::ContestReturnMons { restored_count },
    })
}

fn check_party_full_after_contest(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    const CAUGHT_MON: u8 = 0;
    const BOXED_MON: u8 = 1;
    const NO_CATCH: u8 = 2;
    let Some(contest_mon) = state.bug_contest.caught_mon.take() else {
        state.bug_contest.caught_species = None;
        state.bug_contest.caught_level = None;
        set_script_numeric_value(state, NO_CATCH);
        state.bug_contest.last_result = Some(NO_CATCH);
        state.script_runtime.last_special_routine = Some(routine.to_string());
        return Ok(SpecialRoutineOutcome {
            routine: routine.to_string(),
            effect: SpecialRoutineEffect::CheckPartyFullAfterContest {
                result: NO_CATCH,
                species: None,
            },
        });
    };
    let species = contest_mon.species.id.clone();
    let had_party_space = state.storage.party.has_space();
    state.storage.register_capture(contest_mon).map_err(|_| {
        SpecialRoutineError::GiftStorageFull {
            routine: routine.to_string(),
            species: species.clone(),
        }
    })?;
    state.bug_contest.caught_species = None;
    state.bug_contest.caught_level = None;
    state.sync_party_from_storage();
    let result = if had_party_space {
        CAUGHT_MON
    } else {
        BOXED_MON
    };
    set_script_numeric_value(state, result);
    state.bug_contest.last_result = Some(result);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::CheckPartyFullAfterContest {
            result,
            species: Some(species),
        },
    })
}

fn bug_contest_judging(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let rank = required_u8_script_variable(state, routine, "_bug_contest_rank")?;
    state.bug_contest.last_rank = Some(rank);
    state
        .script_runtime
        .named_buffers
        .insert("STRING_BUFFER_3".to_string(), rank.to_string());
    set_script_numeric_value(state, rank);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::BugContestJudging { rank },
    })
}

fn set_bits_for_link_request(
    state: &mut GameState,
    routine: &str,
    action: u8,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    state.link_session.player_link_action = action;
    state.link_session.chosen_cable_club_room = action;
    set_script_numeric_value(state, action);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::LinkAction {
            action,
            room: action,
        },
    })
}

fn wait_for_linked_friend(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let ready = required_bool_script_variable(state, routine, "_link_friend_ready")?;
    if ready {
        state.link_session.friend_ready = true;
        state.link_session.serial_connection_status =
            LinkSerialConnectionStatus::UsingExternalClock;
    }
    state.link_session.last_result = ready;
    set_script_bool_value(state, ready);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::LinkResult {
            success: ready,
            link_mode: state.link_session.link_mode,
        },
    })
}

fn check_link_timeout_receptionist(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let timeout = required_bool_script_variable(state, routine, "_link_timeout")?;
    if timeout {
        reset_link_state(state);
        set_script_bool_value(state, false);
        state.script_runtime.last_special_routine = Some(routine.to_string());
        return Ok(SpecialRoutineOutcome {
            routine: routine.to_string(),
            effect: SpecialRoutineEffect::LinkResult {
                success: false,
                link_mode: 0,
            },
        });
    }
    let other_mode = required_u8_script_variable(state, routine, "_other_player_link_mode")?;
    state.link_session.player_link_action = state.link_session.chosen_cable_club_room;
    state.link_session.other_player_link_mode = other_mode;
    state.link_session.serial_connection_status = LinkSerialConnectionStatus::UsingExternalClock;
    state.link_session.last_result = true;
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::LinkResult {
            success: true,
            link_mode: state.link_session.link_mode,
        },
    })
}

fn check_both_selected_same_room(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let other_room = required_u8_script_variable(state, routine, "_other_player_room")?;
    let success = other_room == state.link_session.chosen_cable_club_room;
    if success {
        state.link_session.link_mode = state.link_session.chosen_cable_club_room.saturating_add(1);
    }
    state.link_session.last_result = success;
    set_script_bool_value(state, success);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::LinkResult {
            success,
            link_mode: state.link_session.link_mode,
        },
    })
}

fn close_link(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    reset_link_state(state);
    set_script_bool_value(state, false);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::LinkResult {
            success: false,
            link_mode: 0,
        },
    })
}

fn wait_for_other_player_to_exit(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    reset_link_state(state);
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::LinkResult {
            success: true,
            link_mode: 0,
        },
    })
}

fn failed_link_to_past(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    state.link_session.failed_link_to_past = true;
    state.link_session.link_mode = 1;
    state.link_session.serial_connection_status = LinkSerialConnectionStatus::NotEstablished;
    set_script_bool_value(state, false);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::LinkResult {
            success: false,
            link_mode: state.link_session.link_mode,
        },
    })
}

fn link_room(
    state: &mut GameState,
    routine: &str,
    room: &str,
    link_mode: u8,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    state.link_session.link_mode = link_mode;
    state.link_session.active_room = Some(room.to_string());
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::LinkRoom {
            room: room.to_string(),
            link_mode,
        },
    })
}

fn time_capsule(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    state.link_session.player_link_action = 0;
    state.link_session.chosen_cable_club_room = 0;
    state.link_session.active_room = Some("TimeCapsule".to_string());
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::LinkRoom {
            room: "TimeCapsule".to_string(),
            link_mode: state.link_session.link_mode,
        },
    })
}

fn check_time_capsule_compatibility(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let mut result_code = 0;
    let mut mon_name = None;
    let move_name = None;
    for pokemon in state.storage.party.pokemon.iter().flatten() {
        if pokemon.species.int_id >= 152 {
            result_code = 1;
            mon_name = Some(pokemon.nickname.clone());
            break;
        }
        if pokemon
            .item
            .as_deref()
            .is_some_and(|item| item.contains("MAIL"))
        {
            result_code = 3;
            mon_name = Some(pokemon.nickname.clone());
            break;
        }
    }
    if let Some(mon_name) = mon_name.as_ref() {
        state
            .script_runtime
            .named_buffers
            .insert("STRING_BUFFER_3".to_string(), mon_name.clone());
    }
    set_script_numeric_value(state, result_code);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::TimeCapsuleCompatibility {
            result_code,
            mon_name,
            move_name,
        },
    })
}

fn try_quick_save(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    state.link_session.quick_save_requested = true;
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::QuickSave { requested: true },
    })
}

fn ask_mobile_or_cable(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    const SELECTION: &str = ".Cable";
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), SELECTION.to_string());
    state.script_runtime.script_value = Some(SELECTION.to_string());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::AskMobileOrCable {
            selection: SELECTION.to_string(),
        },
    })
}

fn cable_club_check_which_chris(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let gender = required_string_script_variable(state, routine, "_player_gender")?;
    let male_player = match gender.as_str() {
        "MALE" => true,
        "FEMALE" => false,
        _ => {
            return Err(SpecialRoutineError::InvalidNumericValue {
                routine: routine.to_string(),
                value: gender,
            });
        }
    };
    set_script_bool_value(state, male_player);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::CableClubCheckWhichChris { male_player },
    })
}

const BATTLETOWER_NO_CHALLENGE: u8 = 0;
const BATTLETOWER_SAVED_AND_LEFT: u8 = 1;
const BATTLETOWER_CHALLENGE_IN_PROGRESS: u8 = 2;
const BATTLETOWER_WON_CHALLENGE: u8 = 3;
const BATTLETOWER_RECEIVED_REWARD: u8 = 4;
const SAVE_FILE_FLAG_YOURS: u8 = 0x1;
const SAVE_FILE_FLAG_EXPLANATION: u8 = 0x2;

fn battle_tower_action(
    state: &mut GameState,
    battle_tower_rules: Option<&BattleTowerRules>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let raw_action = required_raw_script_value(state, routine)?;
    let action = raw_action
        .split(';')
        .next()
        .expect("split always yields the original string")
        .split_whitespace()
        .next()
        .ok_or_else(|| SpecialRoutineError::MissingScriptValue {
            routine: routine.to_string(),
            variable: "_value action token".to_string(),
        })?
        .to_string();
    let action_key = action.clone();
    let (value, truthy) = match action_key.as_str() {
        "BATTLETOWERACTION_CHECKSAVEFILEISYOURS" => {
            state.battle_tower.save_file_flags |= SAVE_FILE_FLAG_YOURS;
            ("1".to_string(), true)
        }
        "BATTLETOWERACTION_CHECK_EXPLANATION_READ" => (
            u8::from(state.battle_tower.explanation_read).to_string(),
            state.battle_tower.explanation_read,
        ),
        "BATTLETOWERACTION_SET_EXPLANATION_READ" => {
            state.battle_tower.explanation_read = true;
            state.battle_tower.save_file_flags |= SAVE_FILE_FLAG_EXPLANATION;
            ("1".to_string(), true)
        }
        "BATTLETOWERACTION_GET_CHALLENGE_STATE" => {
            sync_battle_tower_beaten_count(state);
            (
                state.battle_tower.challenge_state.to_string(),
                state.battle_tower.challenge_state != BATTLETOWER_NO_CHALLENGE,
            )
        }
        "BATTLETOWERACTION_RESETDATA" => {
            let rules =
                battle_tower_rules.ok_or_else(|| SpecialRoutineError::MissingBattleTowerRules {
                    routine: routine.to_string(),
                })?;
            validate_battle_tower_rules(rules, routine)?;
            reset_battle_tower_trainer_records(state, rules.challenge_streak_length);
            state.battle_tower.challenge_state = BATTLETOWER_NO_CHALLENGE;
            state.battle_tower.reward_given = false;
            state.battle_tower.quick_saved = false;
            state.battle_tower.record_state = 0;
            state.battle_tower.record_reset_counter = 0;
            state.battle_tower.record_last_day = None;
            state.battle_tower.level_group = 0;
            sync_battle_tower_beaten_count(state);
            ("0".to_string(), false)
        }
        "BATTLETOWERACTION_SAVELEVELGROUP" => {
            let level_group =
                required_u8_script_variable(state, routine, "_battle_tower_level_group")?;
            state.battle_tower.level_group = level_group;
            ("1".to_string(), true)
        }
        "BATTLETOWERACTION_LOADLEVELGROUP" => {
            state.script_runtime.variables.insert(
                "_battle_tower_level_group".to_string(),
                state.battle_tower.level_group.to_string(),
            );
            ("1".to_string(), true)
        }
        "BATTLETOWERACTION_SAVEOPTIONS" => {
            let selected = required_string_script_variable(state, routine, "_selected_reward")?;
            state.battle_tower.reward_item = selected;
            state.battle_tower.reward_given = false;
            ("1".to_string(), true)
        }
        "BATTLETOWERACTION_CHOOSEREWARD" => (state.battle_tower.reward_item.clone(), true),
        "BATTLETOWERACTION_SAVE_AND_QUIT" => {
            state.battle_tower.challenge_state = BATTLETOWER_SAVED_AND_LEFT;
            state.battle_tower.quick_saved = true;
            state.battle_tower.record_last_day = Some(state.time.current_day);
            state.battle_tower.record_state = state.battle_tower.record_state.max(1);
            sync_battle_tower_beaten_count(state);
            ("1".to_string(), true)
        }
        "BATTLETOWERACTION_CHALLENGECANCELED" => {
            state.battle_tower.challenge_state = BATTLETOWER_NO_CHALLENGE;
            state.battle_tower.quick_saved = false;
            state.battle_tower.reward_given = false;
            state.battle_tower.beaten_trainers = 0;
            sync_battle_tower_beaten_count(state);
            ("1".to_string(), true)
        }
        "BATTLETOWERACTION_06" => {
            state.battle_tower.quick_saved = false;
            state.battle_tower.record_state = 0;
            state.battle_tower.record_last_day = None;
            state.battle_tower.record_reset_counter = 0;
            ("1".to_string(), true)
        }
        "BATTLETOWERACTION_0A" => ("1".to_string(), true),
        "BATTLETOWERACTION_GSBALL" => {
            let value = if state.battle_tower.gs_ball_flag {
                0x0b
            } else {
                0
            };
            (value.to_string(), value != 0)
        }
        "BATTLETOWERACTION_1C" => {
            let rules =
                battle_tower_rules.ok_or_else(|| SpecialRoutineError::MissingBattleTowerRules {
                    routine: routine.to_string(),
                })?;
            validate_battle_tower_rules(rules, routine)?;
            state.battle_tower.challenge_state = BATTLETOWER_WON_CHALLENGE;
            state.battle_tower.reward_given = false;
            state.battle_tower.beaten_trainers = state
                .battle_tower
                .beaten_trainers
                .saturating_add(1)
                .min(rules.challenge_streak_length);
            state.battle_tower.record_last_day = Some(state.time.current_day);
            state.battle_tower.record_state = state.battle_tower.record_state.max(1);
            sync_battle_tower_beaten_count(state);
            ("1".to_string(), true)
        }
        "BATTLETOWERACTION_GIVEREWARD" => (state.battle_tower.reward_item.clone(), true),
        "BATTLETOWERACTION_1D" => {
            let rules =
                battle_tower_rules.ok_or_else(|| SpecialRoutineError::MissingBattleTowerRules {
                    routine: routine.to_string(),
                })?;
            validate_battle_tower_rules(rules, routine)?;
            state.battle_tower.challenge_state = BATTLETOWER_RECEIVED_REWARD;
            state.battle_tower.reward_given = true;
            record_battle_tower_run(
                state,
                state.battle_tower.beaten_trainers,
                true,
                rules.challenge_streak_length,
            );
            sync_battle_tower_beaten_count(state);
            ("1".to_string(), true)
        }
        "BATTLETOWERACTION_05" => {
            let status = battle_tower_record_status(state);
            (status.to_string(), status != 0)
        }
        "BATTLETOWERACTION_11" => {
            state.battle_tower.leaderboard_acknowledged = false;
            ("0".to_string(), false)
        }
        "BATTLETOWERACTION_12" => {
            state.battle_tower.leaderboard_acknowledged = true;
            ("1".to_string(), true)
        }
        "BATTLETOWERACTION_13" => (
            u8::from(state.battle_tower.leaderboard_acknowledged).to_string(),
            state.battle_tower.leaderboard_acknowledged,
        ),
        "BATTLETOWERACTION_14" => {
            let value = state.battle_tower.save_file_flags & SAVE_FILE_FLAG_YOURS != 0;
            (u8::from(value).to_string(), value)
        }
        "BATTLETOWERACTION_15" => {
            state.battle_tower.save_file_flags |= SAVE_FILE_FLAG_YOURS;
            ("1".to_string(), true)
        }
        "BATTLETOWERACTION_16" => {
            state.battle_tower.record_last_day = Some(state.time.current_day);
            state.battle_tower.record_reset_counter = 0;
            ("1".to_string(), true)
        }
        "BATTLETOWERACTION_17" => {
            let expired = battle_tower_timer_expired(state, 11);
            if expired {
                state.battle_tower.record_last_day = None;
                state.battle_tower.record_reset_counter = 0;
            }
            (u8::from(expired).to_string(), expired)
        }
        "BATTLETOWERACTION_LEVEL_CHECK" => {
            let rules =
                battle_tower_rules.ok_or_else(|| SpecialRoutineError::MissingBattleTowerRules {
                    routine: routine.to_string(),
                })?;
            validate_battle_tower_rules(rules, routine)?;
            let level_cap = state
                .battle_tower
                .level_group
                .clamp(rules.minimum_level_group, rules.maximum_level_group)
                * rules.level_group_size;
            let highest = state
                .storage
                .party
                .pokemon
                .iter()
                .flatten()
                .map(|pokemon| pokemon.level)
                .max()
                .unwrap_or(0);
            if highest > level_cap {
                (highest.to_string(), true)
            } else {
                ("0".to_string(), false)
            }
        }
        "BATTLETOWERACTION_UBERS_CHECK" => {
            let rules =
                battle_tower_rules.ok_or_else(|| SpecialRoutineError::MissingBattleTowerRules {
                    routine: routine.to_string(),
                })?;
            if rules
                .banned_species
                .iter()
                .any(|species| species.is_empty())
            {
                return Err(SpecialRoutineError::InvalidBattleTowerRules {
                    routine: routine.to_string(),
                    message: "bannedSpecies entries must be non-empty exact species ids"
                        .to_string(),
                });
            }
            validate_battle_tower_rules(rules, routine)?;
            let banned = state.storage.party.pokemon.iter().flatten().any(|pokemon| {
                rules
                    .banned_species
                    .iter()
                    .any(|banned| pokemon.species.id.as_str() == banned.as_str())
            });
            (u8::from(banned).to_string(), banned)
        }
        _ => {
            return Err(SpecialRoutineError::UnhandledBattleTowerAction {
                routine: routine.to_string(),
                action,
            });
        }
    };
    state.script_runtime.script_value = Some(value.clone());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), value.clone());
    state
        .script_runtime
        .variables
        .insert("_truthy".to_string(), u8::from(truthy).to_string());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::BattleTowerAction {
            action: action_key,
            value,
            truthy,
        },
    })
}

fn check_for_battle_tower_rules(
    state: &mut GameState,
    battle_tower_rules: Option<&BattleTowerRules>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let rules = battle_tower_rules.ok_or_else(|| SpecialRoutineError::MissingBattleTowerRules {
        routine: routine.to_string(),
    })?;
    validate_battle_tower_rules(rules, routine)?;
    let failure = battle_tower_rule_failure(state, rules);
    state.battle_tower.last_rule_failure = failure.clone();
    state.script_runtime.variables.insert(
        "battle_tower_rule_failure".to_string(),
        failure.clone().unwrap_or_default(),
    );
    set_script_bool_value(state, failure.is_some());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::CheckForBattleTowerRules { failure },
    })
}

fn battle_tower_room_menu(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let count = state
        .battle_tower
        .record_days
        .len()
        .min(state.battle_tower.record_streaks.len())
        .min(state.battle_tower.record_outcomes.len());
    let records = (0..count)
        .map(|index| BattleTowerRecentRecord {
            day: state.battle_tower.record_days[index],
            wins: state.battle_tower.record_streaks[index],
            result: if state.battle_tower.record_outcomes[index] {
                "win".to_string()
            } else {
                "loss".to_string()
            },
        })
        .collect::<Vec<_>>();
    state.script_runtime.active_menu = Some("BattleTowerRoomMenu".to_string());
    state
        .script_runtime
        .variables
        .insert("$a".to_string(), "FALSE".to_string());
    set_script_numeric_value(state, 0);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::BattleTowerRoomMenu { records },
    })
}

fn battle_tower_battle(
    state: &mut GameState,
    battle_tower_rules: Option<&BattleTowerRules>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let result_code = required_u8_script_variable(state, routine, "_battle_result")?;
    state.battle_tower.quick_saved = false;
    if result_code != 0 {
        state.battle_tower.challenge_state = BATTLETOWER_NO_CHALLENGE;
        state.battle_tower.reward_given = false;
        state.battle_tower.beaten_trainers = 0;
        sync_battle_tower_beaten_count(state);
        set_script_numeric_value(state, result_code);
        state.script_runtime.last_special_routine = Some(routine.to_string());
        return Ok(SpecialRoutineOutcome {
            routine: routine.to_string(),
            effect: SpecialRoutineEffect::BattleTowerBattle {
                result_code,
                beaten_trainers: state.battle_tower.beaten_trainers,
                challenge_state: state.battle_tower.challenge_state,
            },
        });
    }

    let rules = battle_tower_rules.ok_or_else(|| SpecialRoutineError::MissingBattleTowerRules {
        routine: routine.to_string(),
    })?;
    validate_battle_tower_rules(rules, routine)?;
    state.battle_tower.challenge_state = BATTLETOWER_CHALLENGE_IN_PROGRESS;
    state.battle_tower.beaten_trainers = state
        .battle_tower
        .beaten_trainers
        .saturating_add(1)
        .min(rules.challenge_streak_length);
    state.battle_tower.record_state = state.battle_tower.record_state.max(1);
    if state.battle_tower.beaten_trainers >= rules.challenge_streak_length {
        state.battle_tower.challenge_state = BATTLETOWER_WON_CHALLENGE;
        state.battle_tower.record_last_day = Some(state.time.current_day);
    }
    sync_battle_tower_beaten_count(state);
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::BattleTowerBattle {
            result_code,
            beaten_trainers: state.battle_tower.beaten_trainers,
            challenge_state: state.battle_tower.challenge_state,
        },
    })
}

fn battle_tower_mobile_error(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    set_script_bool_value(state, false);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::BattleTowerMobileError,
    })
}

fn load_opponent_trainer_and_pokemon_with_ot_sprite(
    state: &mut GameState,
    context: SpecialRoutineContext<'_>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let trainer_id = required_string_script_variable(state, routine, "_battle_tower_trainer_id")?;
    let sprite_constant =
        required_string_script_variable(state, routine, "_battle_tower_sprite_constant")?;
    let target_object =
        required_string_script_variable(state, routine, "_battle_tower_target_object")?;
    let trainer = context.trainer_catalog.get(&trainer_id).ok_or_else(|| {
        SpecialRoutineError::UnknownBattleTowerTrainer {
            routine: routine.to_string(),
            trainer_id: trainer_id.clone(),
        }
    })?;
    let enemy_party = materialize_trainer_party(
        trainer,
        context.species_catalog,
        context.learnsets,
        context.move_catalog,
        context.growth_rates,
    )
    .map_err(|error| SpecialRoutineError::BattleTowerTrainerBuild {
        routine: routine.to_string(),
        trainer_id: trainer_id.clone(),
        error: error.to_string(),
    })?;
    let enemy_pokemon = enemy_party.first().cloned().ok_or_else(|| {
        SpecialRoutineError::BattleTowerTrainerBuild {
            routine: routine.to_string(),
            trainer_id: trainer_id.clone(),
            error: "empty trainer party".to_string(),
        }
    })?;

    state.battle_tower.loaded_trainer_id = Some(trainer.trainer_id.clone());
    state.battle_tower.last_sprite_constant = Some(sprite_constant.clone());
    state.battle = BattleMemory::Trainer {
        battle_type: "BATTLETYPE_BATTLE_TOWER".to_string(),
        trainer_class: trainer.trainer_class.clone(),
        trainer_id: trainer.trainer_id.clone(),
        trainer_name: trainer.name.clone(),
        event_flag: String::new(),
        seen_text: String::new(),
        win_text: trainer.win_quote.clone(),
        loss_text: trainer.lose_quote.clone(),
        callback: String::new(),
        source_script: routine.to_string(),
        enemy_pokemon,
        enemy_party: enemy_party.clone(),
        reward: trainer.base_reward,
        encounter_music: trainer.encounter_music.clone(),
        ai_move_flags: trainer.ai_move_flags,
        ai_item_switch_flags: trainer.ai_item_switch_flags,
        ai_layers: trainer.ai_layers.clone(),
    };
    state.battle_result = 0;
    state.battle_active_party_index = None;
    state.battle_active_enemy_party_index = Some(0);
    state.battle_rewarded_enemy_party_indices.clear();
    state.script_runtime.variables.insert(
        "other_trainer_class".to_string(),
        trainer.trainer_class.clone(),
    );
    state
        .script_runtime
        .variables
        .insert("other_trainer_id".to_string(), trainer.trainer_id.clone());
    state
        .script_runtime
        .variables
        .insert("other_trainer_name".to_string(), trainer.name.clone());
    state.script_runtime.variables.insert(
        "other_trainer_party_count".to_string(),
        enemy_party.len().to_string(),
    );
    state.script_runtime.variables.insert(
        "battle_tower_sprite_constant".to_string(),
        sprite_constant.clone(),
    );
    state.script_runtime.variables.insert(
        "battle_tower_target_object".to_string(),
        target_object.clone(),
    );
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::LoadOpponentTrainerAndPokemonWithOtSprite {
            trainer_id: trainer.trainer_id.clone(),
            trainer_class: trainer.trainer_class.clone(),
            trainer_name: trainer.name.clone(),
            party_size: enemy_party.len(),
            sprite_constant,
            target_object,
        },
    })
}

fn ask_remember_password(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let remember = optional_bool_script_variable(state, routine, "_yes_no_result")?
        .or(optional_bool_script_variable(
            state,
            routine,
            "_remember_password",
        )?)
        .ok_or_else(|| SpecialRoutineError::MissingScriptValue {
            routine: routine.to_string(),
            variable: "_yes_no_result".to_string(),
        })?;
    set_script_bool_value(state, remember);
    state.script_runtime.variables.insert(
        "_remember_password".to_string(),
        u8::from(remember).to_string(),
    );
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::AskRememberPassword { remember },
    })
}

const MOBILE_LINK_MODE: u8 = 4;
const NULL_LINK_MODE: u8 = 0;
const MOBILE_LOGIN_PASSWORD_LENGTH: usize = 17;

fn battle_tower_leaderboard(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let records = battle_tower_mobile_records(state);
    let acknowledged = !records.is_empty();
    state.mobile_link.leaderboard = records.clone();
    if acknowledged {
        state.battle_tower.leaderboard_acknowledged = false;
    }
    state.script_runtime.script_value = Some(if acknowledged { "0" } else { "10" }.to_string());
    state.script_runtime.variables.insert(
        "_value".to_string(),
        state
            .script_runtime
            .script_value
            .clone()
            .unwrap_or_default(),
    );
    state.script_runtime.variables.insert(
        "battle_tower_leaderboard_count".to_string(),
        records.len().to_string(),
    );
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::BattleTowerLeaderboard {
            records,
            acknowledged,
        },
    })
}

fn mobile_handshake(
    state: &mut GameState,
    routine: &str,
    mode: &str,
    link_mode: u8,
    serial_status: LinkSerialConnectionStatus,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    record_mobile_handshake(state, routine, mode)?;
    state.link_session.link_mode = link_mode;
    state.link_session.serial_connection_status = serial_status;
    set_script_numeric_value(state, 0);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::MobileHandshake {
            routine: routine.to_string(),
            mode: mode.to_string(),
            link_mode,
            serial_status,
            handshakes: state.mobile_link.handshakes,
        },
    })
}

fn mobile_session_end(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    state.mobile_link.terminated = true;
    state.link_session.link_mode = NULL_LINK_MODE;
    state.link_session.serial_connection_status = LinkSerialConnectionStatus::NotEstablished;
    set_script_numeric_value(state, 0);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::MobileSessionEnded,
    })
}

fn battle_tower_mobile_flag(
    state: &mut GameState,
    routine: &str,
    flag: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    state.battle_tower.mobile_flags.insert(flag.to_string());
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::BattleTowerMobileFlag {
            flag: flag.to_string(),
        },
    })
}

fn mobile_select_three_mons(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let raw_indexes = required_string_script_variable(state, routine, "_selected_party_indexes")?;
    let indexes = parse_selected_party_indexes(routine, &raw_indexes)?;
    state.battle_tower.selected_party_indexes = indexes.clone();
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::MobileSelectThreeMons { indexes },
    })
}

fn record_mobile_handshake(
    state: &mut GameState,
    routine: &str,
    mode: &str,
) -> Result<(), SpecialRoutineError> {
    let password = required_string_script_variable(state, routine, "_mobile_login_password")?;
    if password.len() > MOBILE_LOGIN_PASSWORD_LENGTH {
        return Err(SpecialRoutineError::MobilePasswordTooLong {
            routine: routine.to_string(),
        });
    }
    let raw_timer = required_string_script_variable(state, routine, "_mobile_battle_timer")?;
    let timer = parse_mobile_battle_timer(routine, &raw_timer)?;
    let adapter_status = required_string_script_variable(state, routine, "_mobile_adapter_status")?;
    let adapter_secondary_status =
        required_string_script_variable(state, routine, "_mobile_adapter_secondary_status")?;

    state.mobile_link.mode = Some(mode.to_string());
    state.mobile_link.adapter_status = adapter_status;
    state.mobile_link.adapter_secondary_status = adapter_secondary_status;
    state.mobile_link.battle_timer = timer;
    state.mobile_link.login_password = password;
    state.mobile_link.handshakes = state.mobile_link.handshakes.saturating_add(1);
    state.mobile_link.terminated = false;
    state.mobile_link.leaderboard = battle_tower_mobile_records(state);
    state.script_runtime.variables.insert(
        "mobile_handshakes".to_string(),
        state.mobile_link.handshakes.to_string(),
    );
    state.script_runtime.variables.insert(
        "battle_tower_leaderboard_count".to_string(),
        state.mobile_link.leaderboard.len().to_string(),
    );
    Ok(())
}

fn battle_tower_mobile_records(state: &GameState) -> Vec<MobileBattleTowerRecord> {
    let count = state
        .battle_tower
        .record_streaks
        .len()
        .min(state.battle_tower.record_outcomes.len())
        .min(state.battle_tower.record_days.len());
    (0..count)
        .map(|index| MobileBattleTowerRecord {
            streak: state.battle_tower.record_streaks[index],
            outcome: if state.battle_tower.record_outcomes[index] {
                "win".to_string()
            } else {
                "loss".to_string()
            },
            day: state.battle_tower.record_days[index],
        })
        .collect()
}

fn parse_mobile_battle_timer(routine: &str, raw: &str) -> Result<[u8; 3], SpecialRoutineError> {
    let parts = raw.split(',').collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(SpecialRoutineError::InvalidMobileBattleTimer {
            routine: routine.to_string(),
            value: raw.to_string(),
        });
    }
    let first = parse_u8_token(routine, parts[0])?;
    let second = parse_u8_token(routine, parts[1])?;
    let third = parse_u8_token(routine, parts[2])?;
    Ok([first, second, third])
}

fn parse_selected_party_indexes(
    routine: &str,
    raw: &str,
) -> Result<Vec<usize>, SpecialRoutineError> {
    let indexes = raw
        .split(',')
        .map(|part| {
            part.parse::<usize>()
                .map_err(|_| SpecialRoutineError::InvalidNumericValue {
                    routine: routine.to_string(),
                    value: raw.to_string(),
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(indexes)
}

fn parse_u8_token(routine: &str, raw: &str) -> Result<u8, SpecialRoutineError> {
    raw.parse::<u8>()
        .map_err(|_| SpecialRoutineError::InvalidNumericValue {
            routine: routine.to_string(),
            value: raw.to_string(),
        })
}

fn give_odd_egg(
    state: &mut GameState,
    species_catalog: &BTreeMap<String, PokemonSpecies>,
    learnsets: &SpeciesLearnsets,
    growth_rates: &GrowthRateCatalog,
    move_catalog: &BTreeMap<String, Move>,
    odd_egg_definitions: &[OddEggDefinition],
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let table_index = draw_odd_egg_index(state, odd_egg_definitions, routine)?;
    let definition = &odd_egg_definitions[table_index];
    let species = required_species_metadata(species_catalog, routine, definition.species.as_str())?;
    let dvs = Dv::from_non_hp(
        definition.dvs[0],
        definition.dvs[1],
        definition.dvs[2],
        definition.dvs[3],
    );
    let mut egg = create_pokemon_from_known_dvs(
        species,
        definition.level,
        dvs,
        learnsets,
        move_catalog,
        growth_rates,
    )
    .map_err(|error| SpecialRoutineError::GiftPokemonBuild {
        routine: routine.to_string(),
        error: error.to_string(),
    })?;
    egg.nickname = definition.nickname.clone();
    egg.item = None;
    egg.moves = definition
        .moves
        .iter()
        .map(|move_id| {
            let move_data =
                move_catalog
                    .get(move_id)
                    .ok_or_else(|| SpecialRoutineError::UnknownMove {
                        routine: routine.to_string(),
                        party_slot: 0,
                        move_id: move_id.clone(),
                    })?;
            Ok(LearnedMove {
                name: move_id.clone(),
                current_pp: move_data.pp,
                pp_ups: 0,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    egg.hp = 0;
    egg.status = None;
    egg.sleep_turns = 0;
    egg.original_trainer_name = definition.original_trainer_name.clone();
    egg.original_trainer_id = definition.original_trainer_id;
    egg.experience = definition.experience;
    egg.happiness = definition.hatch_cycles;
    let location = state.storage.register_capture(egg.clone()).map_err(|_| {
        SpecialRoutineError::GiftStorageFull {
            routine: routine.to_string(),
            species: definition.species.clone(),
        }
    })?;
    let CaptureStorageLocation::Party { slot: party_slot } = location else {
        return Err(SpecialRoutineError::GiftStorageFull {
            routine: routine.to_string(),
            species: definition.species.clone(),
        });
    };
    state.sync_party_from_storage();
    state.script_runtime.variables.insert(
        "wCurPartySpecies".to_string(),
        definition.species.to_string(),
    );
    state
        .script_runtime
        .variables
        .insert("wCurPartyMon".to_string(), party_slot.to_string());
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::GiveOddEgg {
            table_index,
            species: definition.species.clone(),
            party_slot,
            shiny: definition.dvs == [2, 10, 10, 10],
            rng_seed_after: state.rng_seed,
        },
    })
}

fn draw_odd_egg_index(
    state: &mut GameState,
    odd_egg_definitions: &[OddEggDefinition],
    routine: &str,
) -> Result<usize, SpecialRoutineError> {
    if odd_egg_definitions.is_empty() {
        return Err(SpecialRoutineError::MissingOddEggDefinitions {
            routine: routine.to_string(),
        });
    }
    let total = odd_egg_definitions
        .iter()
        .map(|definition| u32::from(definition.probability))
        .sum::<u32>();
    if total != 100 {
        return Err(SpecialRoutineError::InvalidOddEggTable {
            routine: routine.to_string(),
        });
    }
    let mut rng = Random::new(state.rng_seed);
    let random_word = rng.randrange(0x1_0000) as u16;
    state.rng_seed = rng.seed();
    let mut cumulative = 0u32;
    for (index, definition) in odd_egg_definitions.iter().enumerate() {
        cumulative += u32::from(definition.probability);
        let threshold = (cumulative * 0xffff) / 100;
        if u32::from(random_word) <= threshold {
            return Ok(index);
        }
    }
    Err(SpecialRoutineError::InvalidOddEggTable {
        routine: routine.to_string(),
    })
}

fn validate_battle_tower_rules(
    rules: &BattleTowerRules,
    routine: &str,
) -> Result<(), SpecialRoutineError> {
    if rules.required_party_count == 0 {
        return Err(SpecialRoutineError::InvalidBattleTowerRules {
            routine: routine.to_string(),
            message: "requiredPartyCount must be nonzero".to_string(),
        });
    }
    if rules.challenge_streak_length == 0 {
        return Err(SpecialRoutineError::InvalidBattleTowerRules {
            routine: routine.to_string(),
            message: "challengeStreakLength must be nonzero".to_string(),
        });
    }
    if rules.level_group_size == 0 {
        return Err(SpecialRoutineError::InvalidBattleTowerRules {
            routine: routine.to_string(),
            message: "levelGroupSize must be nonzero".to_string(),
        });
    }
    if rules.minimum_level_group == 0 || rules.maximum_level_group < rules.minimum_level_group {
        return Err(SpecialRoutineError::InvalidBattleTowerRules {
            routine: routine.to_string(),
            message: "level group range must be nonzero and ordered".to_string(),
        });
    }
    for (field, value) in [
        (
            "partyCountFailureText",
            rules.party_count_failure_text.as_str(),
        ),
        (
            "duplicateSpeciesFailureText",
            rules.duplicate_species_failure_text.as_str(),
        ),
        (
            "duplicateHeldItemFailureText",
            rules.duplicate_held_item_failure_text.as_str(),
        ),
        ("eggFailureText", rules.egg_failure_text.as_str()),
    ] {
        if value.trim().is_empty() || value.trim() != value {
            return Err(SpecialRoutineError::InvalidBattleTowerRules {
                routine: routine.to_string(),
                message: format!("{field} must be an exact nonempty text id"),
            });
        }
    }
    Ok(())
}

fn battle_tower_rule_failure(state: &GameState, rules: &BattleTowerRules) -> Option<String> {
    let party = state
        .storage
        .party
        .pokemon
        .iter()
        .flatten()
        .collect::<Vec<_>>();
    if party.len() != rules.required_party_count {
        return Some(rules.party_count_failure_text.clone());
    }

    let mut species = BTreeSet::new();
    for pokemon in &party {
        if pokemon.species.id == "EGG" {
            continue;
        }
        if !species.insert(pokemon.species.id.as_str()) {
            return Some(rules.duplicate_species_failure_text.clone());
        }
    }

    let mut held_items = BTreeSet::new();
    for pokemon in &party {
        if pokemon.species.id == "EGG" {
            continue;
        }
        let Some(item) = pokemon.item.as_deref() else {
            continue;
        };
        if item == "NO_ITEM" {
            continue;
        }
        if !held_items.insert(item) {
            return Some(rules.duplicate_held_item_failure_text.clone());
        }
    }

    if party.iter().any(|pokemon| pokemon.species.id == "EGG") {
        return Some(rules.egg_failure_text.clone());
    }

    None
}

fn sync_battle_tower_beaten_count(state: &mut GameState) {
    state.battle_tower.beaten_trainers = state.battle_tower.beaten_trainers.min(99);
    state.script_runtime.variables.insert(
        "wNrOfBeatenBattleTowerTrainers".to_string(),
        state.battle_tower.beaten_trainers.to_string(),
    );
}

fn reset_battle_tower_trainer_records(state: &mut GameState, challenge_streak_length: u8) {
    state.battle_tower.beaten_trainers = 0;
    state.battle_tower.trainer_history = vec![0xff; challenge_streak_length as usize];
}

fn record_battle_tower_run(
    state: &mut GameState,
    beaten: u8,
    success: bool,
    challenge_streak_length: u8,
) {
    state
        .battle_tower
        .record_streaks
        .insert(0, beaten.min(challenge_streak_length));
    state.battle_tower.record_outcomes.insert(0, success);
    state
        .battle_tower
        .record_days
        .insert(0, state.time.current_day);
    state
        .battle_tower
        .record_streaks
        .truncate(challenge_streak_length as usize);
    state
        .battle_tower
        .record_outcomes
        .truncate(challenge_streak_length as usize);
    state
        .battle_tower
        .record_days
        .truncate(challenge_streak_length as usize);
    state.battle_tower.record_state = 1;
    state.battle_tower.record_last_day = Some(state.time.current_day);
    state.battle_tower.record_reset_counter = 0;
    state.battle_tower.leaderboard_acknowledged = false;
}

fn battle_tower_record_status(state: &mut GameState) -> u8 {
    let mut status = state.battle_tower.record_state;
    if status != 0 && battle_tower_timer_expired(state, 8) {
        status = 8;
        state.battle_tower.record_state = 0;
    }
    status
}

fn battle_tower_timer_expired(state: &GameState, max_days: u8) -> bool {
    if state.battle_tower.record_reset_counter >= 2 {
        return true;
    }
    let Some(last_day) = state.battle_tower.record_last_day else {
        return false;
    };
    state.time.current_day.wrapping_sub(last_day) >= max_days
}

fn required_party_pokemon<'a>(
    state: &'a GameState,
    routine: &str,
    party_slot: usize,
) -> Result<&'a Pokemon, SpecialRoutineError> {
    state
        .storage
        .party
        .pokemon
        .get(party_slot)
        .and_then(Option::as_ref)
        .ok_or_else(|| SpecialRoutineError::InvalidPartySlot {
            routine: routine.to_string(),
            party_slot,
        })
}

fn required_party_pokemon_mut<'a>(
    state: &'a mut GameState,
    routine: &str,
    party_slot: usize,
) -> Result<&'a mut Pokemon, SpecialRoutineError> {
    state
        .storage
        .party
        .pokemon
        .get_mut(party_slot)
        .and_then(Option::as_mut)
        .ok_or_else(|| SpecialRoutineError::InvalidPartySlot {
            routine: routine.to_string(),
            party_slot,
        })
}

fn remove_party_member(state: &mut GameState, party_slot: usize) {
    for index in party_slot..state.storage.party.pokemon.len() - 1 {
        state.storage.party.pokemon[index] = state.storage.party.pokemon[index + 1].take();
    }
    let last_index = state.storage.party.pokemon.len() - 1;
    state.storage.party.pokemon[last_index] = None;
    state.sync_party_from_storage();
}

fn sample_dvs(state: &mut GameState) -> Dv {
    let mut rng = Random::new(state.rng_seed);
    let attack = rng.randrange(16) as u8;
    let defense = rng.randrange(16) as u8;
    let speed = rng.randrange(16) as u8;
    let special = rng.randrange(16) as u8;
    state.rng_seed = rng.seed();
    Dv::from_non_hp(attack, defense, speed, special)
}

fn bag_quantity_by_id(
    state: &GameState,
    item_catalog: &BTreeMap<String, Item>,
    item_id: &str,
) -> u16 {
    let Some(item) = item_catalog.get(item_id) else {
        return 0;
    };
    state.bag.quantity(item)
}

fn ensure_buenas_password<'a>(
    state: &mut GameState,
    categories: &'a [BuenaPasswordCategoryDefinition],
    routine: &str,
) -> Result<(&'a BuenaPasswordCategoryDefinition, String), SpecialRoutineError> {
    if categories.is_empty() {
        return Err(SpecialRoutineError::MissingBuenaPasswordCategories {
            routine: routine.to_string(),
        });
    }
    let current_day = state.time.current_day;
    if !state.buenas_password.generated || state.buenas_password.generation_day != current_day {
        let mut rng = Random::new(state.rng_seed);
        let category_index = rng.randrange(categories.len() as u32) as usize;
        let category = &categories[category_index];
        if category.options.is_empty() {
            return Err(SpecialRoutineError::InvalidBuenaPasswordCategoryIndex {
                routine: routine.to_string(),
                index: category_index,
            });
        }
        let option_index = rng.randrange(category.options.len() as u32) as usize;
        state.rng_seed = rng.seed();
        state.buenas_password.category_index = category_index;
        state.buenas_password.option_index = option_index;
        state.buenas_password.generation_day = current_day;
        state.buenas_password.generated = true;
    }
    let Some(category) = categories.get(state.buenas_password.category_index) else {
        return Err(SpecialRoutineError::InvalidBuenaPasswordCategoryIndex {
            routine: routine.to_string(),
            index: state.buenas_password.category_index,
        });
    };
    let Some(correct) = category.options.get(state.buenas_password.option_index) else {
        return Err(SpecialRoutineError::InvalidBuenaPasswordOptionIndex {
            routine: routine.to_string(),
            index: state.buenas_password.option_index,
        });
    };
    Ok((category, correct.clone()))
}

fn calculate_magikarp_length(
    pokemon: &Pokemon,
    trainer_id: u16,
    magikarp_lengths: &[MagikarpLengthEntry],
    routine: &str,
) -> Result<(u8, u8), SpecialRoutineError> {
    let table = require_magikarp_lengths(magikarp_lengths, routine)?;
    let dv0 = ((pokemon.dvs.attack & 0x0f) << 4) | (pokemon.dvs.defense & 0x0f);
    let dv1 = ((pokemon.dvs.speed & 0x0f) << 4) | (pokemon.dvs.special & 0x0f);
    let id_high = rotate_right_u8((trainer_id >> 8) as u8, 1);
    let id_low = rotate_right_u8((trainer_id & 0xff) as u8, 1);
    let b = rotate_right_u8(dv0, 2) ^ id_high;
    let c = rotate_right_u8(dv1, 2) ^ id_low;
    let bc = u16::from_be_bytes([b, c]);
    let length_mm = if b == 0 && c < 10 {
        u16::from(c) + 190
    } else {
        let mut multiplier = 2u16;
        let mut resolved = None;
        for entry in table {
            let threshold = entry.threshold;
            if b < ((threshold >> 8) & 0xff) as u8 {
                let delta = bc.wrapping_sub(threshold);
                let quotient = (delta / entry.divisor) & 0xff;
                resolved = Some(quotient + 100 * (2 + multiplier));
                break;
            }
            multiplier += 1;
        }
        resolved.unwrap_or_else(|| {
            let threshold = table[table.len() - 1].threshold;
            1600 + bc.wrapping_sub(threshold)
        })
    };
    let total_inches = (u32::from(length_mm) * 10) / 254;
    Ok(((total_inches / 12) as u8, (total_inches % 12) as u8))
}

fn require_magikarp_lengths<'a>(
    magikarp_lengths: &'a [MagikarpLengthEntry],
    routine: &str,
) -> Result<&'a [MagikarpLengthEntry], SpecialRoutineError> {
    if magikarp_lengths.is_empty() {
        return Err(SpecialRoutineError::MissingMagikarpLengthTable {
            routine: routine.to_string(),
        });
    }
    let mut previous = None;
    for entry in magikarp_lengths {
        if entry.divisor == 0 {
            return Err(SpecialRoutineError::InvalidMagikarpLengthTable {
                routine: routine.to_string(),
                message: format!("threshold {} has zero divisor", entry.threshold),
            });
        }
        if previous.is_some_and(|previous| entry.threshold <= previous) {
            return Err(SpecialRoutineError::InvalidMagikarpLengthTable {
                routine: routine.to_string(),
                message: "thresholds must be strictly increasing".to_string(),
            });
        }
        previous = Some(entry.threshold);
    }
    Ok(magikarp_lengths)
}

fn rotate_right_u8(value: u8, count: u8) -> u8 {
    let mut rotated = value;
    for _ in 0..count {
        rotated = (rotated >> 1) | ((rotated & 1) << 7);
    }
    rotated
}

fn format_magikarp_length(feet: u8, inches: u8) -> String {
    format!("{feet}'{inches}\"")
}

fn day_care_resident<'a>(
    state: &'a GameState,
    routine: &str,
    caretaker: &str,
) -> Result<&'a crate::state::DayCareResidentState, SpecialRoutineError> {
    match caretaker {
        "man" => Ok(&state.day_care.man),
        "lady" => Ok(&state.day_care.lady),
        exact => Err(SpecialRoutineError::InvalidDayCareCaretaker {
            routine: routine.to_string(),
            caretaker: exact.to_string(),
        }),
    }
}

fn day_care_resident_mut<'a>(
    state: &'a mut GameState,
    routine: &str,
    caretaker: &str,
) -> Result<&'a mut crate::state::DayCareResidentState, SpecialRoutineError> {
    match caretaker {
        "man" => Ok(&mut state.day_care.man),
        "lady" => Ok(&mut state.day_care.lady),
        exact => Err(SpecialRoutineError::InvalidDayCareCaretaker {
            routine: routine.to_string(),
            caretaker: exact.to_string(),
        }),
    }
}

fn set_day_care_active(
    state: &mut GameState,
    routine: &str,
    caretaker: &str,
    active: bool,
) -> Result<(), SpecialRoutineError> {
    day_care_resident_mut(state, routine, caretaker)?.active = active;
    Ok(())
}

fn day_care_deposit(
    state: &mut GameState,
    routine: &str,
    caretaker: &str,
) -> Result<crate::state::DayCareInteractionState, SpecialRoutineError> {
    if day_care_resident(state, routine, caretaker)?
        .pokemon
        .is_some()
    {
        return Ok(crate::state::DayCareInteractionState {
            caretaker: caretaker.to_string(),
            action: "deposit".to_string(),
            success: false,
            pokemon: None,
            level: None,
            reason: Some("occupied".to_string()),
        });
    }
    let party_slot = required_usize_script_variable(state, routine, "_party_slot")?;
    let pokemon = required_party_pokemon(state, routine, party_slot)?.clone();
    remove_party_member(state, party_slot);
    let resident = day_care_resident_mut(state, routine, caretaker)?;
    resident.initial_experience = pokemon.experience;
    resident.initial_level = pokemon.level;
    resident.steps = 0;
    resident.pokemon = Some(pokemon.clone());
    update_day_care_compatibility(state);
    Ok(crate::state::DayCareInteractionState {
        caretaker: caretaker.to_string(),
        action: "deposit".to_string(),
        success: true,
        pokemon: Some(pokemon.species.id),
        level: Some(pokemon.level),
        reason: None,
    })
}

fn day_care_withdraw(
    state: &mut GameState,
    routine: &str,
    caretaker: &str,
) -> Result<crate::state::DayCareInteractionState, SpecialRoutineError> {
    let Some(pokemon) = day_care_resident_mut(state, routine, caretaker)?
        .pokemon
        .take()
    else {
        return Ok(crate::state::DayCareInteractionState {
            caretaker: caretaker.to_string(),
            action: "withdraw".to_string(),
            success: false,
            pokemon: None,
            level: None,
            reason: Some("empty".to_string()),
        });
    };
    let species = pokemon.species.id.clone();
    let level = pokemon.level;
    let stored = state.storage.party.add_pokemon(pokemon);
    if stored {
        state.sync_party_from_storage();
        let resident = day_care_resident_mut(state, routine, caretaker)?;
        resident.initial_experience = 0;
        resident.initial_level = 0;
        resident.steps = 0;
        update_day_care_compatibility(state);
    }
    Ok(crate::state::DayCareInteractionState {
        caretaker: caretaker.to_string(),
        action: "withdraw".to_string(),
        success: stored,
        pokemon: Some(species),
        level: Some(level),
        reason: (!stored).then(|| "party_full".to_string()),
    })
}

fn day_care_inspect_interaction(
    state: &GameState,
    routine: &str,
    caretaker: &str,
) -> Result<crate::state::DayCareInteractionState, SpecialRoutineError> {
    let resident = day_care_resident(state, routine, caretaker)?;
    Ok(crate::state::DayCareInteractionState {
        caretaker: caretaker.to_string(),
        action: "inspect".to_string(),
        success: resident.pokemon.is_some(),
        pokemon: resident
            .pokemon
            .as_ref()
            .map(|pokemon| pokemon.species.id.clone()),
        level: resident.pokemon.as_ref().map(|pokemon| pokemon.level),
        reason: resident.pokemon.is_none().then(|| "empty".to_string()),
    })
}

fn update_day_care_compatibility(state: &mut GameState) {
    state.day_care.compatibility_score =
        u8::from(state.day_care.man.pokemon.is_some() && state.day_care.lady.pokemon.is_some());
    if state.day_care.compatibility_score == 0 {
        state.day_care.steps_until_next_egg = 0;
    }
}

fn reset_link_state(state: &mut GameState) {
    state.link_session.link_mode = 0;
    state.link_session.player_link_action = 0;
    state.link_session.chosen_cable_club_room = 0;
    state.link_session.other_player_link_mode = 0;
    state.link_session.serial_connection_status = LinkSerialConnectionStatus::NotEstablished;
    state.link_session.friend_ready = false;
    state.link_session.last_result = false;
    state.link_session.active_room = None;
}

fn optional_bool_script_variable(
    state: &GameState,
    routine: &str,
    variable: &str,
) -> Result<Option<bool>, SpecialRoutineError> {
    let Some(raw_value) = state.script_runtime.variables.get(variable).cloned() else {
        return Ok(None);
    };
    match raw_value.as_str() {
        "1" | "true" | "TRUE" => Ok(Some(true)),
        "0" | "false" | "FALSE" => Ok(Some(false)),
        exact => Err(SpecialRoutineError::InvalidNumericValue {
            routine: routine.to_string(),
            value: exact.to_string(),
        }),
    }
}

fn required_bool_script_variable(
    state: &GameState,
    routine: &str,
    variable: &str,
) -> Result<bool, SpecialRoutineError> {
    optional_bool_script_variable(state, routine, variable)?.ok_or_else(|| {
        SpecialRoutineError::MissingScriptValue {
            routine: routine.to_string(),
            variable: variable.to_string(),
        }
    })
}

fn happiness_delta(
    happiness_data: &HappinessData,
    change_code: u8,
    happiness: u8,
    routine: &str,
) -> Result<i16, SpecialRoutineError> {
    let entry = happiness_data
        .changes
        .iter()
        .find(|entry| entry.change_code == change_code)
        .ok_or_else(|| SpecialRoutineError::InvalidHappinessData {
            routine: routine.to_string(),
            message: format!("missing change code {change_code}"),
        })?;
    if happiness < 100 {
        Ok(entry.low)
    } else if happiness < 200 {
        Ok(entry.mid)
    } else {
        Ok(entry.high)
    }
}

fn require_happiness_data<'a>(
    happiness_data: Option<&'a HappinessData>,
    routine: &str,
) -> Result<&'a HappinessData, SpecialRoutineError> {
    let data = happiness_data.ok_or_else(|| SpecialRoutineError::MissingHappinessData {
        routine: routine.to_string(),
    })?;
    if data.changes.is_empty() {
        return Err(SpecialRoutineError::InvalidHappinessData {
            routine: routine.to_string(),
            message: "changes must not be empty".to_string(),
        });
    }
    if data.services.is_empty() {
        return Err(SpecialRoutineError::InvalidHappinessData {
            routine: routine.to_string(),
            message: "services must not be empty".to_string(),
        });
    }
    Ok(data)
}

fn happiness_service_outcomes<'a>(
    happiness_data: &'a HappinessData,
    routine: &str,
) -> Result<&'a [HappinessServiceOutcome], SpecialRoutineError> {
    let table = happiness_data
        .services
        .iter()
        .find(|table| table.routine == routine)
        .ok_or_else(|| SpecialRoutineError::InvalidHappinessData {
            routine: routine.to_string(),
            message: format!("missing service table for {routine}"),
        })?;
    if table.outcomes.is_empty() {
        return Err(SpecialRoutineError::InvalidHappinessData {
            routine: routine.to_string(),
            message: format!("service table {routine} has no outcomes"),
        });
    }
    Ok(&table.outcomes)
}

fn select_happiness_service_outcome(
    outcomes: &[HappinessServiceOutcome],
    roll: u8,
) -> HappinessServiceOutcome {
    let mut remaining = roll;
    for outcome in outcomes {
        if remaining < outcome.roll_weight {
            return *outcome;
        }
        remaining = remaining.wrapping_sub(outcome.roll_weight);
    }
    outcomes[outcomes.len() - 1]
}

fn apply_signed_happiness_delta(value: u8, delta: i16) -> u8 {
    (i16::from(value) + delta).clamp(0, 255) as u8
}

fn oak_rating<'a>(
    oak_ratings: &'a [OakRatingEntry],
    caught_count: usize,
    routine: &str,
) -> Result<&'a OakRatingEntry, SpecialRoutineError> {
    if oak_ratings.is_empty() {
        return Err(SpecialRoutineError::MissingOakRatingTable {
            routine: routine.to_string(),
        });
    }
    let mut previous_limit = None;
    for entry in oak_ratings {
        if entry.fanfare.trim().is_empty()
            || entry.fanfare.trim() != entry.fanfare
            || entry.text_label.trim().is_empty()
            || entry.text_label.trim() != entry.text_label
        {
            return Err(SpecialRoutineError::InvalidOakRatingTable {
                routine: routine.to_string(),
                message: "entries must contain exact nonempty fanfare and text label ids"
                    .to_string(),
            });
        }
        if previous_limit.is_some_and(|limit| entry.caught_count_limit <= limit) {
            return Err(SpecialRoutineError::InvalidOakRatingTable {
                routine: routine.to_string(),
                message: "caughtCountLimit values must be strictly increasing".to_string(),
            });
        }
        previous_limit = Some(entry.caught_count_limit);
        if caught_count <= entry.caught_count_limit {
            return Ok(entry);
        }
    }
    Err(SpecialRoutineError::InvalidOakRatingTable {
        routine: routine.to_string(),
        message: format!("no rating covers caught count {caught_count}"),
    })
}

fn read_event_flag(
    state: &GameState,
    routine: &str,
    flag: &str,
) -> Result<bool, SpecialRoutineError> {
    state
        .flags
        .is_event_flag_set(flag)
        .map_err(|error| SpecialRoutineError::EventFlag {
            routine: routine.to_string(),
            error,
        })
}

fn format_money(value: u32) -> String {
    format!("{value:06}")
}

fn format_coins(value: u16) -> String {
    format!("{value:04}")
}

fn set_script_bool_value(state: &mut GameState, value: bool) {
    set_script_numeric_value(state, u8::from(value));
}

fn set_script_numeric_value(state: &mut GameState, value: u8) {
    set_script_u32_value(state, u32::from(value));
}

fn set_script_u32_value(state: &mut GameState, value: u32) {
    let value = value.to_string();
    state.script_runtime.script_value = Some(value.clone());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), value);
}

fn required_species_value(state: &GameState, routine: &str) -> Result<String, SpecialRoutineError> {
    state
        .script_runtime
        .variables
        .get("_value")
        .cloned()
        .or_else(|| state.script_runtime.script_value.clone())
        .ok_or_else(|| SpecialRoutineError::MissingSpeciesValue {
            routine: routine.to_string(),
        })
}

fn required_u8_script_value(state: &GameState, routine: &str) -> Result<u8, SpecialRoutineError> {
    let raw_value = state
        .script_runtime
        .variables
        .get("_value")
        .cloned()
        .or_else(|| state.script_runtime.script_value.clone())
        .ok_or_else(|| SpecialRoutineError::MissingScriptValue {
            routine: routine.to_string(),
            variable: "_value".to_string(),
        })?;
    raw_value
        .parse::<u8>()
        .map_err(|_| SpecialRoutineError::InvalidNumericValue {
            routine: routine.to_string(),
            value: raw_value,
        })
}

fn required_string_script_variable(
    state: &GameState,
    routine: &str,
    variable: &str,
) -> Result<String, SpecialRoutineError> {
    state
        .script_runtime
        .variables
        .get(variable)
        .cloned()
        .ok_or_else(|| SpecialRoutineError::MissingScriptValue {
            routine: routine.to_string(),
            variable: variable.to_string(),
        })
}

fn required_usize_script_variable(
    state: &GameState,
    routine: &str,
    variable: &str,
) -> Result<usize, SpecialRoutineError> {
    let raw_value = required_string_script_variable(state, routine, variable)?;
    raw_value
        .parse::<usize>()
        .map_err(|_| SpecialRoutineError::InvalidNumericValue {
            routine: routine.to_string(),
            value: raw_value,
        })
}

fn optional_usize_script_variable(
    state: &GameState,
    routine: &str,
    variable: &str,
) -> Result<Option<usize>, SpecialRoutineError> {
    let Some(raw_value) = state.script_runtime.variables.get(variable).cloned() else {
        return Ok(None);
    };
    raw_value
        .parse::<usize>()
        .map(Some)
        .map_err(|_| SpecialRoutineError::InvalidNumericValue {
            routine: routine.to_string(),
            value: raw_value,
        })
}

fn required_selected_party_slot(
    state: &GameState,
    routine: &str,
) -> Result<usize, SpecialRoutineError> {
    let Some(raw_value) = state
        .script_runtime
        .variables
        .get("_selected_party_index")
        .cloned()
        .or_else(|| state.script_runtime.variables.get("_party_slot").cloned())
    else {
        return Err(SpecialRoutineError::MissingScriptValue {
            routine: routine.to_string(),
            variable: "_selected_party_index".to_string(),
        });
    };
    raw_value
        .parse::<usize>()
        .map_err(|_| SpecialRoutineError::InvalidNumericValue {
            routine: routine.to_string(),
            value: raw_value,
        })
}

fn optional_u8_script_variable(
    state: &GameState,
    routine: &str,
    variable: &str,
) -> Result<Option<u8>, SpecialRoutineError> {
    let Some(raw_value) = state.script_runtime.variables.get(variable).cloned() else {
        return Ok(None);
    };
    raw_value
        .parse::<u8>()
        .map(Some)
        .map_err(|_| SpecialRoutineError::InvalidNumericValue {
            routine: routine.to_string(),
            value: raw_value,
        })
}

fn required_u8_script_variable(
    state: &GameState,
    routine: &str,
    variable: &str,
) -> Result<u8, SpecialRoutineError> {
    optional_u8_script_variable(state, routine, variable)?.ok_or_else(|| {
        SpecialRoutineError::MissingScriptValue {
            routine: routine.to_string(),
            variable: variable.to_string(),
        }
    })
}

fn optional_u16_script_variable(
    state: &GameState,
    routine: &str,
    variable: &str,
) -> Result<Option<u16>, SpecialRoutineError> {
    let Some(raw_value) = state.script_runtime.variables.get(variable).cloned() else {
        return Ok(None);
    };
    raw_value
        .parse::<u16>()
        .map(Some)
        .map_err(|_| SpecialRoutineError::InvalidNumericValue {
            routine: routine.to_string(),
            value: raw_value,
        })
}

fn required_u16_script_variable(
    state: &GameState,
    routine: &str,
    variable: &str,
) -> Result<u16, SpecialRoutineError> {
    optional_u16_script_variable(state, routine, variable)?.ok_or_else(|| {
        SpecialRoutineError::MissingScriptValue {
            routine: routine.to_string(),
            variable: variable.to_string(),
        }
    })
}

fn optional_i16_script_variable(
    state: &GameState,
    routine: &str,
    variable: &str,
) -> Result<Option<i16>, SpecialRoutineError> {
    let Some(raw_value) = state.script_runtime.variables.get(variable).cloned() else {
        return Ok(None);
    };
    raw_value
        .parse::<i16>()
        .map(Some)
        .map_err(|_| SpecialRoutineError::InvalidNumericValue {
            routine: routine.to_string(),
            value: raw_value,
        })
}

fn required_species_metadata<'a>(
    species_catalog: &'a BTreeMap<String, PokemonSpecies>,
    routine: &str,
    species: &str,
) -> Result<&'a PokemonSpecies, SpecialRoutineError> {
    species_catalog
        .get(species)
        .ok_or_else(|| SpecialRoutineError::UnknownSpecies {
            routine: routine.to_string(),
            species: species.to_string(),
        })
}

fn required_numeric_script_value(
    state: &GameState,
    routine: &str,
) -> Result<i64, SpecialRoutineError> {
    let raw_value = required_raw_script_value(state, routine)?;
    raw_value
        .parse::<i64>()
        .map_err(|_| SpecialRoutineError::InvalidNumericValue {
            routine: routine.to_string(),
            value: raw_value,
        })
}

fn required_raw_script_value(
    state: &GameState,
    routine: &str,
) -> Result<String, SpecialRoutineError> {
    state
        .script_runtime
        .variables
        .get("_value")
        .cloned()
        .or_else(|| state.script_runtime.script_value.clone())
        .ok_or_else(|| SpecialRoutineError::MissingScriptValue {
            routine: routine.to_string(),
            variable: "_value".to_string(),
        })
}

fn pokemon_nickname_or_species(pokemon: &Pokemon) -> String {
    if pokemon.nickname.trim().is_empty() {
        pokemon.species.id.clone()
    } else {
        pokemon.nickname.clone()
    }
}

fn pokemon_matches_species_and_ot(
    pokemon: &Pokemon,
    species: &str,
    player_name: &str,
    player_id: u16,
) -> bool {
    pokemon.species.id == species
        && pokemon.original_trainer_id == player_id
        && pokemon.original_trainer_name == player_name
}

fn storage_owns_species_with_ot(
    state: &GameState,
    species: &str,
    player_name: &str,
    player_id: u16,
    routine: &str,
) -> Result<bool, SpecialRoutineError> {
    if state
        .storage
        .party
        .pokemon
        .iter()
        .flatten()
        .any(|pokemon| pokemon_matches_species_and_ot(pokemon, species, player_name, player_id))
    {
        return Ok(true);
    }
    for (box_index, pc_box) in state.storage.pc_boxes.iter().enumerate() {
        if pc_box.count > MAX_BOX_MONS {
            return Err(SpecialRoutineError::InvalidPcBoxCount {
                routine: routine.to_string(),
                box_index,
                count: pc_box.count,
            });
        }
        if pc_box
            .pokemon
            .iter()
            .take(pc_box.count)
            .flatten()
            .any(|pokemon| pokemon_matches_species_and_ot(pokemon, species, player_name, player_id))
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn graphics_command(
    state: &mut GameState,
    routine: &str,
    kind: ScriptGraphicsRuntimeKind,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    state
        .script_runtime
        .graphics_events
        .push(ScriptGraphicsRuntimeEvent {
            command: "special".to_string(),
            kind,
            color: None,
            direction: None,
            frames: None,
            source_script: routine.to_string(),
            command_index: 0,
        });
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::GraphicsCommand { kind },
    })
}

fn screen_fade(
    state: &mut GameState,
    routine: &str,
    color: ScriptFadeColor,
    direction: ScriptFadeDirection,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    const FADE_FRAMES: u16 = 8;
    state.script_runtime.pending_screen_fade = Some(ScriptScreenFade {
        color,
        direction,
        frames: FADE_FRAMES,
        source_script: routine.to_string(),
        command_index: 0,
    });
    state
        .script_runtime
        .graphics_events
        .push(ScriptGraphicsRuntimeEvent {
            command: "special".to_string(),
            kind: ScriptGraphicsRuntimeKind::ScreenFade,
            color: Some(color),
            direction: Some(direction),
            frames: Some(FADE_FRAMES),
            source_script: routine.to_string(),
            command_index: 0,
        });
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::ScreenFade {
            color,
            direction,
            frames: FADE_FRAMES,
        },
    })
}

fn heal_pokemon(
    pokemon: &mut Pokemon,
    move_catalog: &BTreeMap<String, Move>,
    routine: &str,
    party_slot: usize,
) -> Result<(), SpecialRoutineError> {
    pokemon.hp = pokemon.max_hp;
    pokemon.status = None;
    pokemon.sleep_turns = 0;
    pokemon.flinching = false;
    pokemon.rampage_turns = 0;
    pokemon.confusion_turns = 0;
    pokemon.perish_song_turns = 0;
    pokemon.focus_energy = false;
    for learned in &mut pokemon.moves {
        let move_data =
            move_catalog
                .get(&learned.name)
                .ok_or_else(|| SpecialRoutineError::UnknownMove {
                    routine: routine.to_string(),
                    party_slot,
                    move_id: learned.name.clone(),
                })?;
        learned.current_pp = max_move_pp(move_data.pp, learned.pp_ups);
    }
    Ok(())
}

fn max_move_pp(base_pp: u8, pp_ups: u8) -> u8 {
    base_pp.saturating_add((base_pp / 5).saturating_mul(pp_ups.min(3)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        BaseStats, Dv, LearnedMove, PcBox, PokemonSpecies, Trainer, TrainerPartyPokemon,
        growth_rate, item_pocket, pokemon_type,
    };
    use crate::systems::experience::{GrowthRateCatalog, crystal_growth_rate_catalog_for_tests};
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
    static EMPTY_TEST_ROAMERS: LazyLock<Vec<RoamingPokemonDefinition>> = LazyLock::new(Vec::new);
    static EMPTY_TEST_BUENA_PASSWORD_CATEGORIES: LazyLock<Vec<BuenaPasswordCategoryDefinition>> =
        LazyLock::new(Vec::new);
    static EMPTY_TEST_BUENA_PRIZES: LazyLock<Vec<BuenaPrizeDefinition>> = LazyLock::new(Vec::new);
    static EMPTY_TEST_KURT_APRICORN_RECIPES: LazyLock<Vec<KurtApricornRecipe>> =
        LazyLock::new(Vec::new);
    static EMPTY_TEST_DRATINI_MOVE_SETS: LazyLock<Vec<DratiniMoveSetDefinition>> =
        LazyLock::new(Vec::new);
    static EMPTY_TEST_MAGIKARP_LENGTHS: LazyLock<Vec<MagikarpLengthEntry>> =
        LazyLock::new(Vec::new);
    static EMPTY_TEST_TRAINERS: LazyLock<TrainerCatalog> = LazyLock::new(TrainerCatalog::default);
    const MODPACK_SPECIAL_ROUTINES_JSON: &str = include_str!(
        "../../../../../apps/web/assets/data/content-packs/core-modular/special_routines/routines.json"
    );

    #[test]
    fn special_routine_registry_is_exact_and_covers_core_modpack_declarations() {
        assert!(is_known_special_routine("HealParty"));
        assert!(is_known_special_routine("UnusedDummySpecial"));
        assert!(!is_known_special_routine("healparty"));
        assert!(!is_known_special_routine("MODPACK_ONLY_ROUTINE"));

        let routines: Vec<String> = serde_json::from_str(MODPACK_SPECIAL_ROUTINES_JSON)
            .expect("core special routines json");
        let unknown: Vec<&str> = routines
            .iter()
            .map(String::as_str)
            .filter(|routine| !is_known_special_routine(routine))
            .collect();
        assert_eq!(unknown, Vec::<&str>::new());
    }

    fn move_data(name: &str, pp: u8) -> Move {
        Move {
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
            battle_focus_energy: None,
            battle_stat_drop_guard: None,
            battle_stat_drop_guard_turns: None,
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
            odd_egg_definitions: &[],
            oak_ratings: &[],
        }
    }

    fn full_context_with_dratini_move_sets<'a>(
        move_catalog: &'a BTreeMap<String, Move>,
        species_catalog: &'a BTreeMap<String, PokemonSpecies>,
        learnsets: &'a SpeciesLearnsets,
        item_catalog: &'a BTreeMap<String, Item>,
        dratini_move_sets: &'a [DratiniMoveSetDefinition],
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
            odd_egg_definitions: &[],
            oak_ratings: &[],
        }
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
            odd_egg_definitions: &[],
            oak_ratings: &[],
        }
    }

    fn battle_tower_rules_with_banned_species(banned_species: Vec<String>) -> BattleTowerRules {
        BattleTowerRules {
            banned_species,
            required_party_count: 3,
            challenge_streak_length: 7,
            minimum_level_group: 1,
            maximum_level_group: 10,
            level_group_size: 10,
            party_count_failure_text: "OnlyThreeMonMayBeEnteredText".to_string(),
            duplicate_species_failure_text: "TheMonMustAllBeDifferentKindsText".to_string(),
            duplicate_held_item_failure_text: "TheMonMustNotHoldTheSameItemsText".to_string(),
            egg_failure_text: "YouCantTakeAnEggText".to_string(),
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
            odd_egg_definitions: &[],
            oak_ratings: &[],
        }
    }

    fn full_context_with_kurt_apricorn_recipes<'a>(
        move_catalog: &'a BTreeMap<String, Move>,
        species_catalog: &'a BTreeMap<String, PokemonSpecies>,
        learnsets: &'a SpeciesLearnsets,
        item_catalog: &'a BTreeMap<String, Item>,
        kurt_apricorn_recipes: &'a [KurtApricornRecipe],
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
            odd_egg_definitions: &[],
            oak_ratings: &[],
        }
    }

    fn full_context_with_buena_password_categories<'a>(
        move_catalog: &'a BTreeMap<String, Move>,
        species_catalog: &'a BTreeMap<String, PokemonSpecies>,
        learnsets: &'a SpeciesLearnsets,
        item_catalog: &'a BTreeMap<String, Item>,
        buena_password_categories: &'a [BuenaPasswordCategoryDefinition],
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
            odd_egg_definitions: &[],
            oak_ratings: &[],
        }
    }

    fn full_context_with_buena_prizes<'a>(
        move_catalog: &'a BTreeMap<String, Move>,
        species_catalog: &'a BTreeMap<String, PokemonSpecies>,
        learnsets: &'a SpeciesLearnsets,
        item_catalog: &'a BTreeMap<String, Item>,
        buena_prizes: &'a [BuenaPrizeDefinition],
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
            odd_egg_definitions: &[],
            oak_ratings: &[],
        }
    }

    fn full_context_with_roamers<'a>(
        move_catalog: &'a BTreeMap<String, Move>,
        species_catalog: &'a BTreeMap<String, PokemonSpecies>,
        learnsets: &'a SpeciesLearnsets,
        item_catalog: &'a BTreeMap<String, Item>,
        roaming_pokemon: &'a [RoamingPokemonDefinition],
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
        RuntimeSpawnPointRef {
            identifier,
            map_constant: map_name.to_string(),
            map_name: map_name.to_string(),
            group_id,
            map_id,
            tile_x,
            tile_y,
            group_name: "GROUP".to_string(),
            metatile_x: tile_x / 2,
            metatile_y: tile_y / 2,
            subtile_x: tile_x % 2,
            subtile_y: tile_y % 2,
        }
    }

    fn spawn_context<'a>(
        move_catalog: &'a BTreeMap<String, Move>,
        runtime_spawn_points: &'a BTreeMap<String, RuntimeSpawnPointRef>,
    ) -> SpecialRoutineContext<'a> {
        SpecialRoutineContext {
            move_catalog,
            cry_by_species: &EMPTY_TEST_CRIES,
            species_catalog: &EMPTY_TEST_SPECIES,
            learnsets: &EMPTY_TEST_LEARNSETS,
            growth_rates: &TEST_GROWTH_RATES,
            item_catalog: &EMPTY_TEST_ITEMS,
            runtime_spawn_points,
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
            odd_egg_definitions: &[],
            oak_ratings: &[],
        }
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
            .register_capture(pokemon("CHIKORITA"))
            .expect("store");
        state.sync_party_from_storage();
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
            .register_capture(pokemon("CHIKORITA"))
            .expect("store first");
        state
            .storage
            .register_capture(pokemon("CYNDAQUIL"))
            .expect("store second");
        state.sync_party_from_storage();

        let outcome = apply_special_routine(&mut state, &moves(), "HealParty").expect("heal party");

        assert_eq!(outcome.routine, "HealParty");
        assert_eq!(
            outcome.effect,
            SpecialRoutineEffect::HealParty {
                healed_slots: vec![0, 1]
            }
        );
        assert_eq!(
            state.script_runtime.last_special_routine.as_deref(),
            Some("HealParty")
        );
        for slot in 0..2 {
            let pokemon = state.storage.party.pokemon[slot]
                .as_ref()
                .expect("party pokemon");
            assert_eq!(pokemon.hp, pokemon.max_hp);
            assert_eq!(pokemon.status, None);
            assert_eq!(pokemon.sleep_turns, 0);
            assert_eq!(pokemon.confusion_turns, 0);
            assert!(!pokemon.focus_energy);
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
            .register_capture(pokemon("CHIKORITA"))
            .expect("store");
        state.sync_party_from_storage();
        let before = state.clone();

        let error =
            apply_special_routine(&mut state, &moves(), "healparty").expect_err("case exact");

        assert!(matches!(
            error,
            SpecialRoutineError::UnsupportedRoutine { routine } if routine == "healparty"
        ));
        assert_eq!(state, before);
    }

    #[test]
    fn heal_party_rejects_unknown_move_without_mutation() {
        let mut state = GameState::default();
        let mut pokemon = pokemon("CHIKORITA");
        pokemon.moves[0].name = "tackle".to_string();
        state.storage.register_capture(pokemon).expect("store");
        state.sync_party_from_storage();
        let before = state.clone();

        let error =
            apply_special_routine(&mut state, &moves(), "HealParty").expect_err("unknown move");

        assert!(matches!(
            error,
            SpecialRoutineError::UnknownMove {
                routine,
                party_slot: 0,
                move_id
            } if routine == "HealParty" && move_id == "tackle"
        ));
        assert_eq!(state, before);
    }

    #[test]
    fn fade_out_music_records_exact_two_frame_music_none_fade() {
        let mut state = GameState::default();
        state.script_runtime.current_music = Some("MUSIC_ROUTE_30".to_string());

        let outcome =
            apply_special_routine(&mut state, &moves(), "FadeOutMusic").expect("fade music");

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
    fn restart_map_music_requests_exact_map_music_restart() {
        let mut state = GameState::default();
        state.script_runtime.map_music_restart_disabled = true;

        let outcome =
            apply_special_routine(&mut state, &moves(), "RestartMapMusic").expect("restart music");

        assert_eq!(outcome.effect, SpecialRoutineEffect::RestartMapMusic);
        assert!(state.script_runtime.map_music_requested);
        assert!(!state.script_runtime.map_music_restart_disabled);
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
    fn play_map_music_requests_exact_map_music_restart() {
        let mut state = GameState::default();
        state.script_runtime.map_music_restart_disabled = true;

        let outcome =
            apply_special_routine(&mut state, &moves(), "PlayMapMusic").expect("play map music");

        assert_eq!(outcome.effect, SpecialRoutineEffect::PlayMapMusic);
        assert!(state.script_runtime.map_music_requested);
        assert!(!state.script_runtime.map_music_restart_disabled);
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

        let outcome =
            apply_special_routine(&mut state, &moves(), "CheckMobileAdapterStatusSpecial")
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
        let mut egg = pokemon("EGG");
        egg.nickname = "EGG".to_string();
        egg.happiness = 1;
        let mut chikorita = pokemon("CHIKORITA");
        chikorita.nickname = "Leafy".to_string();
        chikorita.happiness = 218;
        state.storage.register_capture(egg).expect("store egg");
        state
            .storage
            .register_capture(chikorita)
            .expect("store mon");
        state.sync_party_from_storage();

        let outcome = apply_special_routine(&mut state, &moves(), "GetFirstPokemonHappiness")
            .expect("happiness");

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
        let mut egg = pokemon("EGG");
        egg.nickname = "EGG".to_string();
        state.storage.register_capture(egg).expect("store egg");
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
        let mut egg = pokemon("EGG");
        egg.nickname = "EGG".to_string();
        state.storage.register_capture(egg).expect("store egg");
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
            .register_capture(chikorita)
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
            .register_capture(pokemon("CHIKORITA"))
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
            .register_capture(pokemon("CHIKORITA"))
            .expect("store");
        case_state.sync_party_from_storage();
        case_state.script_runtime.script_value = Some("chikorita".to_string());

        let case_outcome =
            apply_special_routine(&mut case_state, &moves(), "FindPartyMonThatSpecies")
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
            .register_capture(pokemon("CHIKORITA"))
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
            .register_capture(chikorita)
            .expect("store matching mon");
        state.sync_party_from_storage();

        state
            .script_runtime
            .variables
            .insert("_value".to_string(), "30".to_string());
        let level = apply_special_routine(&mut state, &moves(), "FindPartyMonAboveLevel")
            .expect("find level");

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
                .register_capture(pokemon)
                .expect("store beast");
        }
        let mut box0 = PcBox::new(0);
        let mut suicune = pokemon("SUICUNE");
        suicune.original_trainer_name = "KRIS".to_string();
        suicune.original_trainer_id = 0x2345;
        assert!(box0.add_pokemon(suicune));
        state.storage.pc_boxes.push(box0);
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

        let beasts =
            apply_special_routine(&mut state, &moves(), "BeastsCheck").expect("beasts check");

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
        state.storage.pc_boxes.push(pc_box);
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

        let missing_error =
            apply_special_routine(&mut missing_state, &moves(), "ActivateFishingSwarm")
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
        let changed = apply_special_routine(&mut state, &moves(), "SetPlayerPalette")
            .expect("palette changes");

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
            tile: crate::world::map::TilePosition::new(34, 10),
            facing: crate::world::map::Direction::Down,
            mode: crate::world::movement::MovementMode::Normal,
        };

        let awake =
            apply_special_routine(&mut state, &moves(), "SnorlaxAwake").expect("snorlax awake");

        assert_eq!(
            awake.effect,
            SpecialRoutineEffect::SnorlaxAwake {
                music: Some("MUSIC_POKE_FLUTE_CHANNEL".to_string()),
                tile: Some((34, 10)),
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
                tile: Some((34, 10)),
                awake: false
            }
        );
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));
    }

    #[test]
    fn snorlax_awake_accepts_packed_coordinate_candidates() {
        let mut state = GameState::default();
        state.script_runtime.current_music = Some("MUSIC_POKE_FLUTE_CHANNEL".to_string());
        state.overworld = crate::state::OverworldMemory::Active {
            map_name: "Route11".to_string(),
            tile: crate::world::map::TilePosition::new(67, 17),
            facing: crate::world::map::Direction::Down,
            mode: crate::world::movement::MovementMode::Normal,
        };

        let outcome =
            apply_special_routine(&mut state, &moves(), "SnorlaxAwake").expect("packed snorlax");

        assert_eq!(
            outcome.effect,
            SpecialRoutineEffect::SnorlaxAwake {
                music: Some("MUSIC_POKE_FLUTE_CHANNEL".to_string()),
                tile: Some((67, 17)),
                awake: true
            }
        );
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

        let day =
            apply_special_routine(&mut state, &moves(), "SetDayOfWeek").expect("set day of week");

        assert_eq!(day.effect, SpecialRoutineEffect::SetDayOfWeek { day: 0 });
        assert_eq!(state.time.current_day, 0);
        assert_eq!(state.time.day_of_week, 0);
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));

        let set_dst =
            apply_special_routine(&mut state, &moves(), "InitialSetDSTFlag").expect("set dst flag");

        assert_eq!(set_dst.effect, SpecialRoutineEffect::InitialSetDstFlag);
        assert!(state.time.dst);

        let clear_dst = apply_special_routine(&mut state, &moves(), "InitialClearDSTFlag")
            .expect("clear dst flag");

        assert_eq!(clear_dst.effect, SpecialRoutineEffect::InitialClearDstFlag);
        assert!(!state.time.dst);
        assert_eq!(
            state.script_runtime.last_special_routine.as_deref(),
            Some("InitialClearDSTFlag")
        );
    }

    #[test]
    fn update_time_special_recomputes_registers_and_time_of_day() {
        let mut state = GameState::default();
        state.time.start_time = crate::systems::time::ClockTime::new(2, 9, 30, 15);
        state.time.registers.rtc_day_lo = 3;
        state.time.registers.rtc_hours = 8;
        state.time.registers.rtc_minutes = 45;
        state.time.registers.rtc_seconds = 50;

        let outcome =
            apply_special_routine(&mut state, &moves(), "UpdateTime").expect("update time");

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
        assert_eq!(state.time.game_time_hours, 18);
        assert_eq!(state.time.game_time_minutes, 16);
        assert_eq!(state.time.game_time_seconds, 5);
        assert_eq!(state.time.current_day, 5);
    }

    #[test]
    fn sample_kenji_break_countdown_uses_runtime_rng_seed() {
        let mut state = GameState::default();
        state.rng_seed = 1;

        let outcome = apply_special_routine(&mut state, &moves(), "SampleKenjiBreakCountdown")
            .expect("sample kenji countdown");

        assert_eq!(
            outcome.effect,
            SpecialRoutineEffect::SampleKenjiBreakCountdown {
                value: 4,
                rng_seed_after: 58_598
            }
        );
        assert_eq!(state.kenji_break_timer, 4);
        assert_eq!(state.rng_seed, 58_598);
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("4"));
        assert_eq!(
            state
                .script_runtime
                .variables
                .get("_value")
                .map(String::as_str),
            Some("4")
        );
    }

    #[test]
    fn lucky_number_show_flag_and_reset_use_daily_lucky_number() {
        let mut state = GameState::default();
        state.lucky_number_show_flag = true;
        state.time.current_day = 6;
        state.rng_seed = 1;

        let check = apply_special_routine(&mut state, &moves(), "CheckLuckyNumberShowFlag")
            .expect("check lucky flag");

        assert_eq!(
            check.effect,
            SpecialRoutineEffect::CheckLuckyNumberShowFlag { flag: true }
        );
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));

        let reset = apply_special_routine(&mut state, &moves(), "ResetLuckyNumberShowFlag")
            .expect("reset lucky flag");

        assert_eq!(
            reset.effect,
            SpecialRoutineEffect::ResetLuckyNumberShowFlag {
                lucky_number: 16_523,
                lucky_number_day: 6,
                rng_seed_after: 127_215
            }
        );
        assert!(!state.lucky_number_show_flag);
        assert_eq!(state.lucky_number_day, Some(6));
        assert_eq!(state.lucky_id_number, 16_523);
        assert_eq!(state.rng_seed, 127_215);
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));

        let before_seed = state.rng_seed;
        let reset_same_day =
            apply_special_routine(&mut state, &moves(), "ResetLuckyNumberShowFlag")
                .expect("same-day reset");

        assert_eq!(
            reset_same_day.effect,
            SpecialRoutineEffect::ResetLuckyNumberShowFlag {
                lucky_number: 16_523,
                lucky_number_day: 6,
                rng_seed_after: before_seed
            }
        );
        assert_eq!(state.rng_seed, before_seed);

        let printed = apply_special_routine(&mut state, &moves(), "PrintTodaysLuckyNumber")
            .expect("print lucky number");

        assert_eq!(
            printed.effect,
            SpecialRoutineEffect::PrintTodaysLuckyNumber {
                lucky_number: 16_523,
                formatted: "16523".to_string()
            }
        );
        assert_eq!(state.rng_seed, before_seed);
        assert_eq!(
            state
                .script_runtime
                .named_buffers
                .get("STRING_BUFFER_3")
                .map(String::as_str),
            Some("16523")
        );
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("16523"));
    }

    #[test]
    fn check_for_lucky_number_winners_scans_party_and_pc_with_pc_tie_priority() {
        let mut state = GameState::default();
        state.lucky_number_day = Some(2);
        state.time.current_day = 2;
        state.lucky_id_number = 45_123;
        let mut party_match = pokemon("CHIKORITA");
        party_match.original_trainer_id = 31_123;
        state
            .storage
            .register_capture(party_match)
            .expect("store party match");
        let mut pc_tie = pokemon("TOTODILE");
        pc_tie.original_trainer_id = 51_123;
        let mut box0 = PcBox::new(0);
        assert!(box0.add_pokemon(pc_tie));
        state.storage.pc_boxes.push(box0);
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
            .register_capture(pokemon("CHIKORITA"))
            .expect("store party");
        state.storage.pc_boxes.push(PcBox::new(0));
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
            .register_capture(pokemon("CHIKORITA"))
            .expect("store party");
        let mut pc_box = PcBox::new(0);
        pc_box.count = MAX_BOX_MONS + 1;
        state.storage.pc_boxes.push(pc_box);
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

        let outcome =
            apply_special_routine(&mut state, &moves(), "GSHealings").expect("gs healings");

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
            .register_capture(pokemon("HO_OH"))
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
        let open =
            apply_special_routine(&mut state, &moves(), "HoOhChamber").expect("open chamber");

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

            let outcome =
                apply_special_routine(&mut state, &moves(), routine).expect("screen fade");

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
            .register_capture(pokemon("CHIKORITA"))
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

        let pc = apply_special_routine(&mut state, &moves(), "PokemonCenterPC")
            .expect("pokemon center pc");

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
        assert_eq!(
            state
                .script_runtime
                .variables
                .get("_pc_context")
                .map(String::as_str),
            Some("PokemonCenterPC")
        );
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));

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
        assert_eq!(
            state
                .script_runtime
                .variables
                .get("_oak_rating_label")
                .map(String::as_str),
            Some("OakRating01")
        );

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
        assert_eq!(
            state
                .script_runtime
                .variables
                .get("_town_map_current_map")
                .map(String::as_str),
            Some("NewBarkTown")
        );

        let printer =
            apply_special_routine(&mut state, &moves(), "UnownPrinter").expect("unown printer");

        assert_eq!(
            printer.effect,
            SpecialRoutineEffect::UnownPrinter { unlocked: true }
        );
        assert_eq!(
            state
                .script_runtime
                .variables
                .get("_unown_printer_unlocked")
                .map(String::as_str),
            Some("1")
        );
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
        let radio = apply_special_routine(&mut state, &moves(), "MapRadio").expect("map radio");

        assert_eq!(
            radio.effect,
            SpecialRoutineEffect::MapRadio {
                station: "MAPRADIO_UNOWN".to_string()
            }
        );
        assert_eq!(
            state
                .script_runtime
                .variables
                .get("_map_radio_station")
                .map(String::as_str),
            Some("MAPRADIO_UNOWN")
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
            .register_capture(pokemon("CHIKORITA"))
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
        assert_eq!(
            state
                .script_runtime
                .variables
                .get("_deleted_move")
                .map(String::as_str),
            Some("GROWL")
        );
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));

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
            ("UnownPuzzle", ScriptGraphicsRuntimeKind::UnownPuzzle),
            ("OmanyteChamber", ScriptGraphicsRuntimeKind::OmanyteChamber),
            (
                "DisplayUnownWords",
                ScriptGraphicsRuntimeKind::DisplayUnownWords,
            ),
        ];

        for (routine, kind) in cases {
            let mut state = GameState::default();

            let outcome = apply_special_routine(&mut state, &moves(), routine)
                .expect("visual special routine");

            assert_eq!(
                outcome.effect,
                SpecialRoutineEffect::RuntimeVisualCommand { kind }
            );
            assert_eq!(state.script_runtime.active_menu.as_deref(), Some(routine));
            assert_eq!(
                state
                    .script_runtime
                    .variables
                    .get("_visual_special")
                    .map(String::as_str),
                Some(routine)
            );
            assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));
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
    fn check_pokerus_records_exact_status_engine_flag_and_phone_call() {
        let mut state = GameState::default();
        let mut infected = pokemon("CHIKORITA");
        infected.status = Some("POKERUS".to_string());
        state
            .storage
            .register_capture(infected)
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
            Ok(true)
        );
        assert_eq!(
            state.script_runtime.special_phone_calls,
            vec!["SPECIALCALL_POKERUS".to_string()]
        );
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));

        let second =
            apply_special_routine(&mut state, &moves(), "CheckPokerus").expect("check again");

        assert_eq!(
            second.effect,
            SpecialRoutineEffect::CheckPokerus {
                found: true,
                newly_discovered: false
            }
        );
        assert_eq!(state.script_runtime.special_phone_calls.len(), 1);
    }

    #[test]
    fn happiness_services_require_exact_party_slot_and_apply_exact_change_tables() {
        let mut state = GameState::default();
        let mut mon = pokemon("CHIKORITA");
        mon.happiness = 70;
        mon.nickname = "Leafy".to_string();
        state.storage.register_capture(mon).expect("store mon");
        state.sync_party_from_storage();
        let move_catalog = moves();
        let species_catalog = BTreeMap::new();
        let item_catalog = BTreeMap::new();
        let happiness_data = HappinessData {
            changes: vec![
                HappinessChangeEntry {
                    code: "HAPPINESS_OLDERCUT1".to_string(),
                    change_code: 9,
                    low: 1,
                    mid: 1,
                    high: 1,
                },
                HappinessChangeEntry {
                    code: "HAPPINESS_OLDERCUT2".to_string(),
                    change_code: 10,
                    low: 3,
                    mid: 3,
                    high: 1,
                },
                HappinessChangeEntry {
                    code: "HAPPINESS_OLDERCUT3".to_string(),
                    change_code: 11,
                    low: 5,
                    mid: 5,
                    high: 2,
                },
                HappinessChangeEntry {
                    code: "HAPPINESS_YOUNGCUT1".to_string(),
                    change_code: 12,
                    low: 1,
                    mid: 1,
                    high: 1,
                },
                HappinessChangeEntry {
                    code: "HAPPINESS_YOUNGCUT2".to_string(),
                    change_code: 13,
                    low: 3,
                    mid: 3,
                    high: 1,
                },
                HappinessChangeEntry {
                    code: "HAPPINESS_YOUNGCUT3".to_string(),
                    change_code: 14,
                    low: 10,
                    mid: 10,
                    high: 4,
                },
                HappinessChangeEntry {
                    code: "HAPPINESS_GROOMING".to_string(),
                    change_code: 18,
                    low: 3,
                    mid: 3,
                    high: 1,
                },
            ],
            services: vec![
                HappinessServiceTable {
                    routine: "OlderHaircutBrother".to_string(),
                    outcomes: vec![
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
                },
                HappinessServiceTable {
                    routine: "YoungerHaircutBrother".to_string(),
                    outcomes: vec![
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
                },
                HappinessServiceTable {
                    routine: "DaisysGrooming".to_string(),
                    outcomes: vec![HappinessServiceOutcome {
                        roll_weight: 255,
                        script_value: 2,
                        change_code: 18,
                    }],
                },
            ],
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
                change_code: 9,
                rng_seed_after: 1
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
                change_code: 13,
                rng_seed_after: 1
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
                change_code: 18,
                rng_seed_after: 1
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
            .register_capture(pokemon("CHIKORITA"))
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
        state.storage.register_capture(mon).expect("store mon");
        state.sync_party_from_storage();

        state
            .script_runtime
            .variables
            .insert("_party_slot".to_string(), "0".to_string());
        state
            .script_runtime
            .variables
            .insert("_selected_nickname".to_string(), "Chiko".to_string());
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
        assert_eq!(
            state
                .script_runtime
                .variables
                .get("_poke_seer_ot_id")
                .map(String::as_str),
            Some("8738")
        );

        let mut move_catalog = moves();
        move_catalog.insert("EMBER".to_string(), move_data("EMBER", 25));
        state
            .script_runtime
            .variables
            .insert("_move".to_string(), "ember".to_string());
        let lower = apply_special_routine(&mut state, &move_catalog, "MoveTutor")
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
            apply_special_routine(&mut state, &move_catalog, "MoveTutor").expect("move tutor");

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

        let repeat =
            apply_special_routine(&mut state, &move_catalog, "MoveTutor").expect("move known");
        assert_eq!(
            repeat.effect,
            SpecialRoutineEffect::MoveTutor {
                party_slot: 0,
                species: "CHIKORITA".to_string(),
                move_name: "EMBER".to_string(),
                learned: false
            }
        );
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
            .register_capture(pokemon("CHIKORITA"))
            .expect("store photo mon");
        state.sync_party_from_storage();

        let bank = apply_special_routine(&mut state, &moves(), "BankOfMom").expect("bank");
        assert_eq!(
            bank.effect,
            SpecialRoutineEffect::BankOfMom {
                money: 1200,
                moms_money: 345
            }
        );
        assert_eq!(
            state
                .script_runtime
                .variables
                .get("_mom_money")
                .map(String::as_str),
            Some("345")
        );

        let slot = apply_special_routine(&mut state, &moves(), "SlotMachine").expect("slot");
        assert_eq!(slot.effect, SpecialRoutineEffect::SlotMachine { coins: 99 });
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("99"));

        let card = apply_special_routine(&mut state, &moves(), "CardFlip").expect("card");
        assert_eq!(card.effect, SpecialRoutineEffect::CardFlip { coins: 99 });

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

        let trainer_house =
            apply_special_routine(&mut state, &moves(), "TrainerHouse").expect("trainer house");
        assert_eq!(
            trainer_house.effect,
            SpecialRoutineEffect::TrainerHouse {
                wins: 7,
                losses: 3,
                draws: 1
            }
        );
        assert_eq!(
            state.pending_special_battle_type.as_deref(),
            Some("BATTLETYPE_TRAINER_HOUSE")
        );

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

        let cancel = apply_special_routine(&mut state, &moves(), "Menu_ChallengeExplanationCancel")
            .expect("cancel challenge explanation");
        assert_eq!(
            cancel.effect,
            SpecialRoutineEffect::BattleTowerChallengeExplanationCancel
        );
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));
    }

    #[test]
    fn inactive_declared_specials_reject_without_runtime_mutation() {
        let cases = [
            "RandomUnseenWildMon",
            "RandomPhoneWildMon",
            "RandomPhoneMon",
            "Function11ac3e",
            "TradeCornerHoldMon",
            "Function11b5e8",
            "Function11b7e5",
            "Function11b879",
            "Function11b920",
            "Function11b93b",
            "Function170114",
            "Function1704e1",
            "UnusedBattleTowerDummySpecial1",
            "Function11ba38",
            "Function11c1ab",
            "Function17d2b6",
            "Function17d2ce",
            "Function102142",
            "UnusedBattleTowerDummySpecial2",
            "UnusedMemoryGame",
            "UnusedCheckUnusedTwoDayTimer",
            "UnusedFindItemInPCOrBag",
            "UnusedDummySpecial",
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
        let declared: Vec<String> =
            serde_json::from_str(MODPACK_SPECIAL_ROUTINES_JSON).expect("special routines json");
        let mut missing = Vec::new();

        for routine in declared {
            let mut state = GameState::default();
            let result = apply_special_routine(&mut state, &moves(), &routine);
            if matches!(
                result,
                Err(SpecialRoutineError::UnsupportedRoutine { routine: unsupported })
                    if unsupported == routine
            ) {
                missing.push(routine);
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

        let outcome = apply_special_routine_with_context(
            &mut state,
            full_context_with_shuckie_gift(
                &move_catalog,
                &species,
                &learnsets,
                &items,
                &shuckie_gift,
            ),
            "GiveShuckle",
        )
        .expect("give shuckle");

        assert_eq!(
            outcome.effect,
            SpecialRoutineEffect::GiveShuckle {
                stored: true,
                rng_seed_after: 222_509
            }
        );
        let shuckie = state.storage.party.pokemon[0].as_ref().expect("shuckie");
        assert_eq!(shuckie.species.id, "SHUCKLE");
        assert_eq!(shuckie.item.as_deref(), Some("BERRY"));
        assert_eq!(shuckie.nickname, "SHUCKIE");
        assert_eq!(shuckie.original_trainer_name, "MANIA");
        assert_eq!(shuckie.original_trainer_id, 518);
        assert_eq!(
            state.flags.is_engine_flag_set("ENGINE_GOT_SHUCKIE_TODAY"),
            Ok(true)
        );

        state
            .script_runtime
            .variables
            .insert("_selected_party_index".to_string(), "0".to_string());
        let returned = apply_special_routine_with_context(
            &mut state,
            full_context_with_shuckie_gift(
                &move_catalog,
                &species,
                &learnsets,
                &items,
                &shuckie_gift,
            ),
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

        let error = apply_special_routine(&mut state, &moves(), "GiveShuckle")
            .expect_err("missing Shuckie gift rejected");

        assert_eq!(
            error,
            SpecialRoutineError::MissingShuckieGift {
                routine: "GiveShuckle".to_string()
            }
        );
        assert_eq!(state, before);
    }

    #[test]
    fn give_dratini_replaces_last_dratini_moves_from_exact_move_catalog() {
        let mut state = GameState::default();
        state
            .storage
            .register_capture(pokemon("DRATINI"))
            .expect("store");
        state
            .storage
            .register_capture(pokemon("DRATINI"))
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
        let dratini_move_sets = vec![
            DratiniMoveSetDefinition {
                mode: 0,
                moves: vec![
                    "WRAP".to_string(),
                    "THUNDER_WAVE".to_string(),
                    "TWISTER".to_string(),
                    "EXTREMESPEED".to_string(),
                ],
            },
            DratiniMoveSetDefinition {
                mode: 1,
                moves: vec![
                    "WRAP".to_string(),
                    "LEER".to_string(),
                    "THUNDER_WAVE".to_string(),
                    "TWISTER".to_string(),
                ],
            },
        ];

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
            .register_capture(pokemon("DRATINI"))
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
        let recipes = vec![KurtApricornRecipe {
            apricorn: "RED_APRICORN".to_string(),
            ball: "LEVEL_BALL".to_string(),
        }];

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
        let recipes = vec![KurtApricornRecipe {
            apricorn: "RED_APRICORN".to_string(),
            ball: "LEVEL_BALL".to_string(),
        }];

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
            .register_capture(pokemon("PIKACHU"))
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
        let roaming_definitions = vec![
            RoamingPokemonDefinition {
                species: "RAIKOU".to_string(),
                level: 40,
                map_group: 2,
                map_number: 5,
            },
            RoamingPokemonDefinition {
                species: "ENTEI".to_string(),
                level: 40,
                map_group: 10,
                map_number: 4,
            },
        ];
        let roamers = apply_special_routine_with_context(
            &mut state,
            full_context_with_roamers(
                &move_catalog,
                &species,
                &learnsets,
                &items,
                &roaming_definitions,
            ),
            "InitRoamMons",
        )
        .expect("init roamers");

        assert_eq!(state.roaming_pokemon.len(), 2);
        assert_eq!(
            state.roaming_pokemon,
            vec![
                RoamingPokemonState {
                    species: "RAIKOU".to_string(),
                    level: 40,
                    map_group: 2,
                    map_number: 5,
                    hp: 0,
                    dvs: 0
                },
                RoamingPokemonState {
                    species: "ENTEI".to_string(),
                    level: 40,
                    map_group: 10,
                    map_number: 4,
                    hp: 0,
                    dvs: 0
                }
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
            .register_capture(pokemon("PIKACHU"))
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
        let check = apply_special_routine(&mut state, &moves, "CheckMysteryGift")
            .expect("check mystery gift");
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
        state.rng_seed = 1;
        let moves = moves();
        let species = BTreeMap::new();
        let learnsets = SpeciesLearnsets::new();
        let items = BTreeMap::new();
        let buena_password_categories = vec![
            BuenaPasswordCategoryDefinition {
                id: "JohtoStarters".to_string(),
                category_type: "BUENA_MON".to_string(),
                points: 10,
                options: vec![
                    "CYNDAQUIL".to_string(),
                    "TOTODILE".to_string(),
                    "CHIKORITA".to_string(),
                ],
            },
            BuenaPasswordCategoryDefinition {
                id: "Beverages".to_string(),
                category_type: "BUENA_ITEM".to_string(),
                points: 12,
                options: vec![
                    "FRESH_WATER".to_string(),
                    "SODA_POP".to_string(),
                    "LEMONADE".to_string(),
                ],
            },
            BuenaPasswordCategoryDefinition {
                id: "HealingItems".to_string(),
                category_type: "BUENA_ITEM".to_string(),
                points: 12,
                options: vec![
                    "POTION".to_string(),
                    "ANTIDOTE".to_string(),
                    "PARLYZ_HEAL".to_string(),
                ],
            },
        ];

        let first = apply_special_routine_with_context(
            &mut state,
            full_context_with_buena_password_categories(
                &moves,
                &species,
                &learnsets,
                &items,
                &buena_password_categories,
            ),
            "BuenasPassword",
        )
        .expect("generate password");

        assert_eq!(
            first.effect,
            SpecialRoutineEffect::BuenasPassword {
                category: "JohtoStarters".to_string(),
                category_type: "BUENA_MON".to_string(),
                correct: "TOTODILE".to_string(),
                guess: None,
                matched: false,
                rng_seed_after: 127_215
            }
        );
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));

        state
            .script_runtime
            .variables
            .insert("BUENA_PASSWORD".to_string(), "TOTODILE".to_string());
        let correct = apply_special_routine_with_context(
            &mut state,
            full_context_with_buena_password_categories(
                &moves,
                &species,
                &learnsets,
                &items,
                &buena_password_categories,
            ),
            "BuenasPassword",
        )
        .expect("match password");
        assert_eq!(
            correct.effect,
            SpecialRoutineEffect::BuenasPassword {
                category: "JohtoStarters".to_string(),
                category_type: "BUENA_MON".to_string(),
                correct: "TOTODILE".to_string(),
                guess: Some("TOTODILE".to_string()),
                matched: true,
                rng_seed_after: 127_215
            }
        );

        let item = item_data("RARE_CANDY");
        let items = BTreeMap::from([("RARE_CANDY".to_string(), item.clone())]);
        let buena_prizes = vec![BuenaPrizeDefinition {
            item_id: "RARE_CANDY".to_string(),
            cost: 3,
        }];
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
        assert_eq!(state.bag.quantity(&item), 2);
    }

    #[test]
    fn buena_password_requires_explicit_modpack_category_data_without_builtin_table_fallback() {
        let mut state = GameState::default();
        state.time.current_day = 3;
        state.rng_seed = 1;
        let before = state.clone();

        let error = apply_special_routine(&mut state, &moves(), "BuenasPassword")
            .expect_err("missing Buena password categories reject");

        assert_eq!(
            error,
            SpecialRoutineError::MissingBuenaPasswordCategories {
                routine: "BuenasPassword".to_string()
            }
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

        let outcome = apply_special_routine(&mut state, &moves(), "CelebiShrineEvent")
            .expect("celebi shrine");

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
        let mut magikarp = pokemon("MAGIKARP");
        magikarp.original_trainer_name = "KRIS".to_string();
        magikarp.dvs = Dv::from_non_hp(10, 10, 10, 10);
        state.storage.register_capture(magikarp).expect("store");
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
                feet: 4,
                inches: 0,
                result: 3
            }
        );
        assert_eq!(state.magikarp_record.best_feet, 4);
        assert_eq!(state.magikarp_record.best_inches, 0);
        assert_eq!(state.magikarp_record.best_owner_name, "KRIS");
        assert_eq!(
            state
                .script_runtime
                .named_buffers
                .get("STRING_BUFFER_1")
                .map(String::as_str),
            Some("4'0\"")
        );

        let sign = apply_special_routine(&mut state, &moves(), "MagikarpHouseSign").expect("sign");

        assert_eq!(
            sign.effect,
            SpecialRoutineEffect::MagikarpHouseSign {
                feet: 4,
                inches: 0,
                formatted: "4'0\"".to_string()
            }
        );
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("4'0\""));
    }

    #[test]
    fn selected_party_specials_require_exact_selection_without_slot_zero_fallback() {
        let mut shuckie_state = GameState::default();
        shuckie_state
            .storage
            .register_capture(pokemon("SHUCKLE"))
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
        let shuckie_error = apply_special_routine_with_context(
            &mut shuckie_state,
            full_context_with_shuckie_gift(
                &move_catalog,
                &species,
                &learnsets,
                &items,
                &shuckie_gift,
            ),
            "ReturnShuckie",
        )
        .expect_err("missing shuckie selection rejected");
        assert!(matches!(
            shuckie_error,
            SpecialRoutineError::MissingScriptValue { routine, variable }
                if routine == "ReturnShuckie" && variable == "_selected_party_index"
        ));
        assert_eq!(shuckie_state, before_shuckie);

        let mut magikarp_state = GameState::default();
        magikarp_state
            .storage
            .register_capture(pokemon("MAGIKARP"))
            .expect("store magikarp");
        magikarp_state.sync_party_from_storage();
        let before_magikarp = magikarp_state.clone();
        let magikarp_error =
            apply_special_routine(&mut magikarp_state, &moves(), "CheckMagikarpLength")
                .expect_err("missing magikarp selection rejected");
        assert!(matches!(
            magikarp_error,
            SpecialRoutineError::MissingScriptValue { routine, variable }
                if routine == "CheckMagikarpLength" && variable == "_selected_party_index"
        ));
        assert_eq!(magikarp_state, before_magikarp);
    }

    #[test]
    fn day_care_deposit_inspect_and_withdraw_are_saveable_actions() {
        let mut state = GameState::default();
        state
            .storage
            .register_capture(pokemon("CHIKORITA"))
            .expect("store");
        state.sync_party_from_storage();
        state
            .script_runtime
            .variables
            .insert("_day_care_action".to_string(), "deposit".to_string());
        state
            .script_runtime
            .variables
            .insert("_party_slot".to_string(), "0".to_string());

        let deposit = apply_special_routine(&mut state, &moves(), "DayCareMan").expect("deposit");

        assert_eq!(
            deposit.effect,
            SpecialRoutineEffect::DayCareInteraction {
                caretaker: "man".to_string(),
                action: "deposit".to_string(),
                success: true,
                pokemon: Some("CHIKORITA".to_string())
            }
        );
        assert_eq!(state.storage.party.filled_slots(), 0);
        assert_eq!(
            state
                .day_care
                .man
                .pokemon
                .as_ref()
                .map(|pokemon| pokemon.species.id.as_str()),
            Some("CHIKORITA")
        );

        let inspect = apply_special_routine(&mut state, &moves(), "DayCareMon1").expect("inspect");
        assert_eq!(
            inspect.effect,
            SpecialRoutineEffect::DayCareMon {
                caretaker: "man".to_string(),
                occupied: true,
                pokemon: Some("CHIKORITA".to_string()),
                level: Some(5)
            }
        );

        state
            .script_runtime
            .variables
            .insert("_day_care_action".to_string(), "withdraw".to_string());
        let withdraw = apply_special_routine(&mut state, &moves(), "DayCareMan").expect("withdraw");

        assert_eq!(
            withdraw.effect,
            SpecialRoutineEffect::DayCareInteraction {
                caretaker: "man".to_string(),
                action: "withdraw".to_string(),
                success: true,
                pokemon: Some("CHIKORITA".to_string())
            }
        );
        assert_eq!(state.storage.party.filled_slots(), 1);
        assert!(state.day_care.man.pokemon.is_none());
    }

    #[test]
    fn day_care_interaction_requires_explicit_action_without_inspect_fallback() {
        let mut state = GameState::default();
        state
            .storage
            .register_capture(pokemon("CHIKORITA"))
            .expect("store");
        state.sync_party_from_storage();
        let before = state.clone();

        let error = apply_special_routine(&mut state, &moves(), "DayCareMan")
            .expect_err("missing day care action rejected");

        assert!(matches!(
            error,
            SpecialRoutineError::MissingScriptValue { routine, variable }
                if routine == "DayCareMan" && variable == "_day_care_action"
        ));
        assert_eq!(state, before);
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
    fn bug_contest_setup_selects_contestants_and_drops_off_party_backup() {
        let mut state = GameState::default();
        state.rng_seed = 1;
        state
            .storage
            .register_capture(pokemon("CHIKORITA"))
            .expect("lead");
        state
            .storage
            .register_capture(pokemon("CYNDAQUIL"))
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

        let contestants = apply_special_routine_with_context(
            &mut state,
            full_context_with_bug_contest_config(
                &move_catalog,
                &species_catalog,
                &EMPTY_TEST_LEARNSETS,
                &item_catalog,
                &bug_contest_config,
            ),
            "SelectRandomBugContestContestants",
        )
        .expect("contestants");
        assert_eq!(
            contestants.effect,
            SpecialRoutineEffect::SelectRandomBugContestContestants {
                flags: vec![
                    "EVENT_BUG_CATCHING_CONTESTANT_3A".to_string(),
                    "EVENT_BUG_CATCHING_CONTESTANT_4A".to_string(),
                    "EVENT_BUG_CATCHING_CONTESTANT_6A".to_string(),
                    "EVENT_BUG_CATCHING_CONTESTANT_8A".to_string(),
                    "EVENT_BUG_CATCHING_CONTESTANT_10A".to_string(),
                ],
                rng_seed_after: 178_626
            }
        );
        assert_eq!(state.bug_contest.selected_contestant_flags.len(), 5);
        assert_eq!(
            state
                .flags
                .is_event_flag_set("EVENT_BUG_CATCHING_CONTESTANT_3A"),
            Ok(true)
        );

        let drop =
            apply_special_routine(&mut state, &moves(), "ContestDropOffMons").expect("drop off");
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
    fn bug_contest_caught_mon_is_registered_and_judging_records_rank() {
        let mut state = GameState::default();
        state
            .storage
            .register_capture(pokemon("CHIKORITA"))
            .expect("lead");
        state.sync_party_from_storage();
        state.bug_contest.caught_mon = Some(pokemon("SCYTHER"));
        state.bug_contest.caught_species = Some("SCYTHER".to_string());
        state.bug_contest.caught_level = Some(14);

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
        assert!(state.bug_contest.caught_mon.is_none());
        assert_eq!(state.bug_contest.last_result, Some(0));

        state
            .script_runtime
            .variables
            .insert("_bug_contest_rank".to_string(), "2".to_string());
        let judging =
            apply_special_routine(&mut state, &moves(), "BugContestJudging").expect("judging");

        assert_eq!(
            judging.effect,
            SpecialRoutineEffect::BugContestJudging { rank: 2 }
        );
        assert_eq!(state.bug_contest.last_rank, Some(2));
        assert_eq!(
            state
                .script_runtime
                .named_buffers
                .get("STRING_BUFFER_3")
                .map(String::as_str),
            Some("2")
        );
    }

    #[test]
    fn magikarp_length_requires_modpack_table_without_constant_fallback() {
        let mut state = GameState::default();
        let magikarp = pokemon("MAGIKARP");
        state.storage.register_capture(magikarp).expect("store");
        state.sync_party_from_storage();
        state
            .script_runtime
            .variables
            .insert("_selected_party_index".to_string(), "0".to_string());
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
    fn bug_contest_judging_requires_rank_without_caught_state_fallback() {
        let mut state = GameState::default();
        state.bug_contest.caught_mon = Some(pokemon("SCYTHER"));
        state.bug_contest.caught_species = Some("SCYTHER".to_string());
        state.bug_contest.caught_level = Some(14);
        let before = state.clone();

        let error = apply_special_routine(&mut state, &moves(), "BugContestJudging")
            .expect_err("missing judging rank rejected");

        assert!(matches!(
            error,
            SpecialRoutineError::MissingScriptValue { routine, variable }
                if routine == "BugContestJudging" && variable == "_bug_contest_rank"
        ));
        assert_eq!(state, before);
    }

    #[test]
    fn link_request_timeout_same_room_and_close_update_saveable_session() {
        let mut state = GameState::default();

        let trade = apply_special_routine(&mut state, &moves(), "SetBitsForLinkTradeRequest")
            .expect("trade request");
        assert_eq!(
            trade.effect,
            SpecialRoutineEffect::LinkAction { action: 1, room: 1 }
        );
        assert_eq!(state.link_session.chosen_cable_club_room, 1);

        state
            .script_runtime
            .variables
            .insert("_link_friend_ready".to_string(), "1".to_string());
        let friend = apply_special_routine(&mut state, &moves(), "WaitForLinkedFriend")
            .expect("friend ready");
        assert_eq!(
            friend.effect,
            SpecialRoutineEffect::LinkResult {
                success: true,
                link_mode: 0
            }
        );
        assert_eq!(
            state.link_session.serial_connection_status,
            LinkSerialConnectionStatus::UsingExternalClock
        );

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
                link_mode: 2
            }
        );
        assert_eq!(state.link_session.link_mode, 2);

        let close = apply_special_routine(&mut state, &moves(), "CloseLink").expect("close");
        assert_eq!(
            close.effect,
            SpecialRoutineEffect::LinkResult {
                success: false,
                link_mode: 0
            }
        );
        assert_eq!(state.link_session.link_mode, 0);
        assert_eq!(state.link_session.chosen_cable_club_room, 0);

        state
            .script_runtime
            .variables
            .insert("_link_timeout".to_string(), "1".to_string());
        state.link_session.chosen_cable_club_room = 2;
        let timeout = apply_special_routine(&mut state, &moves(), "CheckLinkTimeout_Receptionist")
            .expect("timeout");
        assert_eq!(
            timeout.effect,
            SpecialRoutineEffect::LinkResult {
                success: false,
                link_mode: 0
            }
        );

        state.link_session.chosen_cable_club_room = 1;
        state
            .script_runtime
            .variables
            .insert("_link_timeout".to_string(), "0".to_string());
        state
            .script_runtime
            .variables
            .insert("_other_player_link_mode".to_string(), "2".to_string());
        let connected =
            apply_special_routine(&mut state, &moves(), "CheckLinkTimeout_Receptionist")
                .expect("connected");
        assert_eq!(
            connected.effect,
            SpecialRoutineEffect::LinkResult {
                success: true,
                link_mode: 0
            }
        );
        assert_eq!(state.link_session.other_player_link_mode, 2);
    }

    #[test]
    fn link_peer_inputs_are_required_without_session_fallbacks() {
        let cases = [
            ("WaitForLinkedFriend", "_link_friend_ready"),
            ("CheckLinkTimeout_Receptionist", "_link_timeout"),
            ("CheckBothSelectedSameRoom", "_other_player_room"),
            ("CableClubCheckWhichChris", "_player_gender"),
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
        state
            .script_runtime
            .variables
            .insert("_link_timeout".to_string(), "0".to_string());
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
            .insert("_player_gender".to_string(), "male".to_string());
        let before = state.clone();
        let error = apply_special_routine(&mut state, &moves(), "CableClubCheckWhichChris")
            .expect_err("gender is exact and not case coerced");
        assert!(matches!(
            error,
            SpecialRoutineError::InvalidNumericValue { routine, value }
                if routine == "CableClubCheckWhichChris" && value == "male"
        ));
        assert_eq!(state, before);
    }

    #[test]
    fn link_rooms_time_capsule_quick_save_and_gender_branch_are_stateful() {
        let mut state = GameState::default();
        let trade_center =
            apply_special_routine(&mut state, &moves(), "TradeCenter").expect("trade center");
        assert_eq!(
            trade_center.effect,
            SpecialRoutineEffect::LinkRoom {
                room: "TradeCenter".to_string(),
                link_mode: 2
            }
        );
        let colosseum =
            apply_special_routine(&mut state, &moves(), "Colosseum").expect("colosseum");
        assert_eq!(
            colosseum.effect,
            SpecialRoutineEffect::LinkRoom {
                room: "Colosseum".to_string(),
                link_mode: 3
            }
        );
        let capsule =
            apply_special_routine(&mut state, &moves(), "EnterTimeCapsule").expect("capsule");
        assert_eq!(
            capsule.effect,
            SpecialRoutineEffect::LinkRoom {
                room: "TimeCapsule".to_string(),
                link_mode: 1
            }
        );

        let mut celebi = pokemon("CELEBI");
        celebi.species.int_id = 251;
        celebi.nickname = "ILEX".to_string();
        state
            .storage
            .register_capture(celebi)
            .expect("store celebi");
        state.sync_party_from_storage();
        let compat = apply_special_routine(&mut state, &moves(), "CheckTimeCapsuleCompatibility")
            .expect("compat");
        assert_eq!(
            compat.effect,
            SpecialRoutineEffect::TimeCapsuleCompatibility {
                result_code: 1,
                mon_name: Some("ILEX".to_string()),
                move_name: None
            }
        );

        let save = apply_special_routine(&mut state, &moves(), "TryQuickSave").expect("save");
        assert_eq!(
            save.effect,
            SpecialRoutineEffect::QuickSave { requested: true }
        );
        assert!(state.link_session.quick_save_requested);

        let ask =
            apply_special_routine(&mut state, &moves(), "AskMobileOrCable").expect("ask cable");
        assert_eq!(
            ask.effect,
            SpecialRoutineEffect::AskMobileOrCable {
                selection: ".Cable".to_string()
            }
        );

        state
            .script_runtime
            .variables
            .insert("_player_gender".to_string(), "FEMALE".to_string());
        let chris =
            apply_special_routine(&mut state, &moves(), "CableClubCheckWhichChris").expect("chris");
        assert_eq!(
            chris.effect,
            SpecialRoutineEffect::CableClubCheckWhichChris { male_player: false }
        );
    }

    #[test]
    fn battle_tower_rules_actions_and_records_are_saveable() {
        let mut state = GameState::default();
        state.time.current_day = 9;
        state
            .storage
            .register_capture(pokemon("CHIKORITA"))
            .expect("store chikorita");
        state
            .storage
            .register_capture(pokemon("CYNDAQUIL"))
            .expect("store cyndaquil");
        state
            .storage
            .register_capture(pokemon("TOTODILE"))
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
            SpecialRoutineEffect::CheckForBattleTowerRules { failure: None }
        );
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));

        state.script_runtime.variables.insert(
            "_value".to_string(),
            "BATTLETOWERACTION_SET_EXPLANATION_READ".to_string(),
        );
        let explanation = apply_special_routine(&mut state, &move_catalog, "BattleTowerAction")
            .expect("set read");
        assert_eq!(
            explanation.effect,
            SpecialRoutineEffect::BattleTowerAction {
                action: "BATTLETOWERACTION_SET_EXPLANATION_READ".to_string(),
                value: "1".to_string(),
                truthy: true
            }
        );
        assert!(state.battle_tower.explanation_read);

        state.script_runtime.variables.insert(
            "_value".to_string(),
            "BATTLETOWERACTION_SAVELEVELGROUP".to_string(),
        );
        state
            .script_runtime
            .variables
            .insert("_battle_tower_level_group".to_string(), "5".to_string());
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

        state.script_runtime.variables.insert(
            "_value".to_string(),
            "BATTLETOWERACTION_SAVEOPTIONS".to_string(),
        );
        state
            .script_runtime
            .variables
            .insert("_selected_reward".to_string(), "HP_UP".to_string());
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

        state
            .script_runtime
            .variables
            .insert("_value".to_string(), "BATTLETOWERACTION_1D".to_string());
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

        let menu =
            apply_special_routine(&mut state, &move_catalog, "BattleTowerRoomMenu").expect("menu");
        assert_eq!(
            menu.effect,
            SpecialRoutineEffect::BattleTowerRoomMenu {
                records: vec![BattleTowerRecentRecord {
                    day: 9,
                    wins: 1,
                    result: "win".to_string()
                }]
            }
        );
        assert_eq!(
            state.script_runtime.active_menu.as_deref(),
            Some("BattleTowerRoomMenu")
        );
    }

    #[test]
    fn battle_tower_option_actions_require_exact_script_inputs_without_state_fallbacks() {
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

        let cases = [
            (
                "BATTLETOWERACTION_SAVELEVELGROUP",
                "_battle_tower_level_group",
            ),
            ("BATTLETOWERACTION_SAVEOPTIONS", "_selected_reward"),
        ];

        for (action, missing_variable) in cases {
            let mut state = GameState::default();
            state.battle_tower.level_group = 3;
            state.battle_tower.reward_item = "CARBOS".to_string();
            state
                .script_runtime
                .variables
                .insert("_value".to_string(), action.to_string());
            let before = state.clone();

            let error = apply_special_routine(&mut state, &moves(), "BattleTowerAction")
                .expect_err("missing battle tower option input rejected");

            assert!(matches!(
                error,
                SpecialRoutineError::MissingScriptValue { routine, variable }
                    if routine == "BattleTowerAction" && variable == missing_variable
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
            .register_capture(pokemon("MEWTWO"))
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
            .register_capture(pokemon("CHIKORITA"))
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
                failure: Some("CUSTOM_BATTLE_TOWER_PARTY_COUNT".to_string())
            }
        );
    }

    #[test]
    fn battle_tower_rejects_invalid_parties_and_mobile_prompt_is_explicit() {
        let mut state = GameState::default();
        state
            .storage
            .register_capture(pokemon("CHIKORITA"))
            .expect("first");
        state
            .storage
            .register_capture(pokemon("CHIKORITA"))
            .expect("duplicate");
        state
            .storage
            .register_capture(pokemon("TOTODILE"))
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
                failure: Some("TheMonMustAllBeDifferentKindsText".to_string())
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

        let init =
            apply_special_routine(&mut state, &moves(), "Function1011f1").expect("mobile init");
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

        let end =
            apply_special_routine(&mut state, &moves(), "Function101220").expect("mobile end");
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

        let selected = apply_special_routine(&mut state, &moves(), "Mobile_SelectThreeMons")
            .expect("select mons");
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

        let mut selection = GameState::default();
        selection.battle_tower.selected_party_indexes = vec![9, 8, 7];
        let before_selection = selection.clone();
        let selection_error =
            apply_special_routine(&mut selection, &moves(), "Mobile_SelectThreeMons")
                .expect_err("missing mobile party selection rejected");
        assert!(matches!(
            selection_error,
            SpecialRoutineError::MissingScriptValue { routine, variable }
                if routine == "Mobile_SelectThreeMons" && variable == "_selected_party_indexes"
        ));
        assert_eq!(selection, before_selection);
    }

    #[test]
    fn give_odd_egg_uses_exact_table_entry_and_modpack_move_pp() {
        let mut state = GameState::default();
        state.rng_seed = 1;
        let mut species = species_catalog(&[("CLEFFA", 173)]);
        species
            .get_mut("CLEFFA")
            .expect("cleffa")
            .step_cycles_to_hatch = 99;
        let learnsets = [("CLEFFA".to_string(), Vec::new())].into_iter().collect();
        let moves = [
            ("POUND".to_string(), move_data("POUND", 35)),
            ("CHARM".to_string(), move_data("CHARM", 20)),
            ("DIZZY_PUNCH".to_string(), move_data("DIZZY_PUNCH", 10)),
        ]
        .into_iter()
        .collect::<BTreeMap<_, _>>();
        let odd_egg_definitions = test_odd_egg_definitions();

        let outcome = apply_special_routine_with_context(
            &mut state,
            full_context_with_odd_egg_definitions(
                &moves,
                &species,
                &learnsets,
                &odd_egg_definitions,
            ),
            "GiveOddEgg",
        )
        .expect("odd egg");

        assert_eq!(
            outcome.effect,
            SpecialRoutineEffect::GiveOddEgg {
                table_index: 3,
                species: "CLEFFA".to_string(),
                party_slot: 0,
                shiny: true,
                rng_seed_after: 58_598
            }
        );
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
                }
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
        state.rng_seed = 1;
        let species = species_catalog(&[("CLEFFA", 173)]);
        let learnsets = [("CLEFFA".to_string(), Vec::new())].into_iter().collect();
        let moves = [
            ("POUND".to_string(), move_data("POUND", 35)),
            ("CHARM".to_string(), move_data("CHARM", 20)),
            ("DIZZY_PUNCH".to_string(), move_data("DIZZY_PUNCH", 10)),
        ]
        .into_iter()
        .collect::<BTreeMap<_, _>>();
        let before = state.clone();

        let error = apply_special_routine_with_context(
            &mut state,
            full_context(&moves, &species, &learnsets, &BTreeMap::new()),
            "GiveOddEgg",
        )
        .expect_err("missing Odd Egg definitions reject");

        assert_eq!(
            error,
            SpecialRoutineError::MissingOddEggDefinitions {
                routine: "GiveOddEgg".to_string()
            }
        );
        assert_eq!(state, before);
    }

    #[test]
    fn give_odd_egg_rejects_full_party_without_pc_fallback() {
        let mut state = GameState::default();
        for _ in 0..6 {
            state
                .storage
                .register_capture(pokemon("CHIKORITA"))
                .expect("fill party");
        }
        state.sync_party_from_storage();
        let species = species_catalog(&[("CLEFFA", 173)]);
        let learnsets = [("CLEFFA".to_string(), Vec::new())].into_iter().collect();
        let moves = [
            ("POUND".to_string(), move_data("POUND", 35)),
            ("CHARM".to_string(), move_data("CHARM", 20)),
            ("DIZZY_PUNCH".to_string(), move_data("DIZZY_PUNCH", 10)),
        ]
        .into_iter()
        .collect::<BTreeMap<_, _>>();
        let odd_egg_definitions = test_odd_egg_definitions();

        let error = apply_special_routine_with_context(
            &mut state,
            full_context_with_odd_egg_definitions(
                &moves,
                &species,
                &learnsets,
                &odd_egg_definitions,
            ),
            "GiveOddEgg",
        )
        .expect_err("full party rejects odd egg");

        assert_eq!(
            error,
            SpecialRoutineError::GiftStorageFull {
                routine: "GiveOddEgg".to_string(),
                species: "CLEFFA".to_string()
            }
        );
    }

    #[test]
    fn warp_to_spawn_point_resolves_saved_group_map_from_modpack_spawns() {
        let mut state = GameState::default();
        state
            .script_runtime
            .variables
            .insert("wLastSpawnMapGroup".to_string(), "23".to_string());
        state
            .script_runtime
            .variables
            .insert("wLastSpawnMapNumber".to_string(), "9".to_string());
        let spawns = BTreeMap::from([
            (
                "0".to_string(),
                spawn_point(0, "PlayersHouse2F", 24, 7, 3, 3),
            ),
            (
                "14".to_string(),
                spawn_point(14, "GoldenrodPokecenter1F", 23, 9, 6, 4),
            ),
        ]);

        let outcome = apply_special_routine_with_context(
            &mut state,
            spawn_context(&moves(), &spawns),
            "WarpToSpawnPoint",
        )
        .expect("warp to spawn");

        assert_eq!(
            outcome.effect,
            SpecialRoutineEffect::WarpToSpawnPoint {
                spawn_identifier: 14,
                map_name: "GoldenrodPokecenter1F".to_string(),
                tile: TilePosition::new(6, 4)
            }
        );
        assert_eq!(state.last_spawn_identifier, Some(14));
        assert_eq!(
            state
                .script_runtime
                .pending_script_warp
                .as_ref()
                .map(|warp| (warp.target_map.as_str(), warp.tile)),
            Some(("GoldenrodPokecenter1F", TilePosition::new(6, 4)))
        );
        assert_eq!(
            state.overworld,
            OverworldMemory::Active {
                map_name: "GoldenrodPokecenter1F".to_string(),
                tile: TilePosition::new(6, 4),
                facing: Direction::Down,
                mode: MovementMode::Normal
            }
        );
        assert_eq!(
            state
                .script_runtime
                .variables
                .get("wDefaultSpawnpoint")
                .map(String::as_str),
            Some("14")
        );
    }

    #[test]
    fn warp_to_spawn_point_uses_saved_spawn_id_or_errors_without_pack_data() {
        let mut state = GameState::default();
        state.last_spawn_identifier = Some(21);
        let spawns = BTreeMap::from([(
            "21".to_string(),
            spawn_point(21, "IndigoPlateauPokecenter1F", 11, 4, 9, 7),
        )]);

        let outcome = apply_special_routine_with_context(
            &mut state,
            spawn_context(&moves(), &spawns),
            "WarpToSpawnPoint",
        )
        .expect("saved spawn id");
        assert_eq!(
            outcome.effect,
            SpecialRoutineEffect::WarpToSpawnPoint {
                spawn_identifier: 21,
                map_name: "IndigoPlateauPokecenter1F".to_string(),
                tile: TilePosition::new(9, 7)
            }
        );

        let mut missing = GameState::default();
        let error = apply_special_routine_with_context(
            &mut missing,
            spawn_context(&moves(), &BTreeMap::new()),
            "WarpToSpawnPoint",
        )
        .expect_err("missing spawn data");
        assert_eq!(
            error,
            SpecialRoutineError::MissingRuntimeSpawnPoints {
                routine: "WarpToSpawnPoint".to_string()
            }
        );
    }

    #[test]
    fn load_battle_tower_opponent_uses_exact_pack_trainer_and_sprite() {
        let mut state = GameState::default();
        state.script_runtime.variables.insert(
            "_battle_tower_trainer_id".to_string(),
            "BT_EDWARD".to_string(),
        );
        state.script_runtime.variables.insert(
            "_battle_tower_sprite_constant".to_string(),
            "SPRITE_GENTLEMAN".to_string(),
        );
        state.script_runtime.variables.insert(
            "_battle_tower_target_object".to_string(),
            "BATTLETOWERBATTLEROOM_GENTLEMAN".to_string(),
        );
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
        let mut catalog = TrainerCatalog::default();
        catalog
            .insert(Trainer {
                name: "EDWARD@".to_string(),
                trainer_id: "BT_EDWARD".to_string(),
                trainer_class: "GENTLEMAN".to_string(),
                party: vec![TrainerPartyPokemon {
                    species: "PERSIAN".to_string(),
                    level: 33,
                    item: None,
                    moves: Vec::new(),
                    dvs: Dv::default(),
                }],
                win_quote: "EDWARD@: I won!".to_string(),
                lose_quote: "EDWARD@: I lost!".to_string(),
                items: Vec::new(),
                base_reward: 18,
                ai_move_flags: 0,
                ai_item_switch_flags: 0,
                encounter_music: "MUSIC_BATTLE_TOWER_THEME".to_string(),
                ai_layers: vec!["basic".to_string()],
            })
            .expect("insert trainer");

        let outcome = apply_special_routine_with_context(
            &mut state,
            trainer_context(&moves, &species, &learnsets, &catalog),
            "LoadOpponentTrainerAndPokemonWithOTSprite",
        )
        .expect("load opponent");

        assert_eq!(
            outcome.effect,
            SpecialRoutineEffect::LoadOpponentTrainerAndPokemonWithOtSprite {
                trainer_id: "BT_EDWARD".to_string(),
                trainer_class: "GENTLEMAN".to_string(),
                trainer_name: "EDWARD@".to_string(),
                party_size: 1,
                sprite_constant: "SPRITE_GENTLEMAN".to_string(),
                target_object: "BATTLETOWERBATTLEROOM_GENTLEMAN".to_string()
            }
        );
        assert_eq!(
            state.battle_tower.loaded_trainer_id.as_deref(),
            Some("BT_EDWARD")
        );
        assert_eq!(
            state.battle_tower.last_sprite_constant.as_deref(),
            Some("SPRITE_GENTLEMAN")
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
                assert_eq!(trainer_id, "BT_EDWARD");
                assert_eq!(trainer_name, "EDWARD@");
                assert_eq!(enemy_party.len(), 1);
                assert_eq!(enemy_party[0].species.id, "PERSIAN");
                assert_eq!(enemy_party[0].level, 33);
                assert_eq!(encounter_music, "MUSIC_BATTLE_TOWER_THEME");
                assert_eq!(ai_layers, &vec!["basic".to_string()]);
            }
            other => panic!("expected battle tower trainer battle, got {other:?}"),
        }
        assert_eq!(
            state
                .script_runtime
                .variables
                .get("battle_tower_sprite_constant")
                .map(String::as_str),
            Some("SPRITE_GENTLEMAN")
        );
    }

    #[test]
    fn battle_tower_battle_and_opponent_load_require_exact_inputs_without_state_fallbacks() {
        let mut battle_state = GameState::default();
        battle_state.battle_result = 1;
        battle_state.battle_tower.quick_saved = true;
        battle_state.battle_tower.beaten_trainers = 3;
        let before_battle = battle_state.clone();
        let battle_error = apply_special_routine(&mut battle_state, &moves(), "BattleTowerBattle")
            .expect_err("missing battle tower result rejected");
        assert!(matches!(
            battle_error,
            SpecialRoutineError::MissingScriptValue { routine, variable }
                if routine == "BattleTowerBattle" && variable == "_battle_result"
        ));
        assert_eq!(battle_state, before_battle);

        let mut load_state = GameState::default();
        load_state.script_runtime.variables.insert(
            "_battle_tower_trainer_id".to_string(),
            "BT_EDWARD".to_string(),
        );
        load_state.script_runtime.variables.insert(
            "_battle_tower_sprite_constant".to_string(),
            "SPRITE_GENTLEMAN".to_string(),
        );
        let before_load = load_state.clone();
        let catalog = TrainerCatalog::default();
        let load_error = apply_special_routine_with_context(
            &mut load_state,
            trainer_context(
                &moves(),
                &BTreeMap::new(),
                &SpeciesLearnsets::new(),
                &catalog,
            ),
            "LoadOpponentTrainerAndPokemonWithOTSprite",
        )
        .expect_err("missing battle tower target object rejected");
        assert!(matches!(
            load_error,
            SpecialRoutineError::MissingScriptValue { routine, variable }
                if routine == "LoadOpponentTrainerAndPokemonWithOTSprite"
                    && variable == "_battle_tower_target_object"
        ));
        assert_eq!(load_state, before_load);
    }

    #[test]
    fn load_battle_tower_opponent_rejects_unknown_trainer() {
        let mut state = GameState::default();
        state.script_runtime.variables.insert(
            "_battle_tower_trainer_id".to_string(),
            "MISSING_TRAINER".to_string(),
        );
        state.script_runtime.variables.insert(
            "_battle_tower_sprite_constant".to_string(),
            "SPRITE_GENTLEMAN".to_string(),
        );
        state.script_runtime.variables.insert(
            "_battle_tower_target_object".to_string(),
            "BATTLETOWERBATTLEROOM_GENTLEMAN".to_string(),
        );
        let catalog = TrainerCatalog::default();

        let error = apply_special_routine_with_context(
            &mut state,
            trainer_context(
                &moves(),
                &BTreeMap::new(),
                &SpeciesLearnsets::new(),
                &catalog,
            ),
            "LoadOpponentTrainerAndPokemonWithOTSprite",
        )
        .expect_err("unknown trainer");

        assert_eq!(
            error,
            SpecialRoutineError::UnknownBattleTowerTrainer {
                routine: "LoadOpponentTrainerAndPokemonWithOTSprite".to_string(),
                trainer_id: "MISSING_TRAINER".to_string()
            }
        );
    }
}
