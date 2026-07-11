use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::battle::start::{first_available_battle_party_index, materialize_trainer_party};
use crate::models::{
    CaptureStorageLocation, Dv, Item, LearnedMove, MAX_BOX_MONS, Move, Party, Pokemon,
    PokemonSpecies, TrainerCatalog, create_pokemon_from_known_dvs, max_move_pp,
};
use crate::random::Random;
use crate::state::{
    BattleMemory, BattleTowerState, BuenasPasswordState, EventFlagError, GameState,
    LinkSerialConnectionStatus, MagikarpRecordState, MobileBattleTowerRecord, OverworldMemory,
    RoamingPokemonState, ScriptAudioRuntimeEvent, ScriptAudioRuntimeKind, ScriptFadeColor,
    ScriptFadeDirection, ScriptGraphicsRuntimeEvent, ScriptGraphicsRuntimeKind,
    ScriptMapRuntimeEvent, ScriptMapRuntimeKind, ScriptMoneyRuntimeEvent, ScriptMoneyRuntimeKind,
    ScriptMusicFade, ScriptScreenFade, ScriptWarpRequest,
};
use crate::systems::experience::GrowthRateCatalog;
use crate::systems::learnsets::SpeciesLearnsets;
use crate::systems::phone::PhoneContactCatalog;
use crate::systems::time::ClockTime;
use crate::world::encounters::{TimeOfDay, WildEncounter, WildEncounterData};
use crate::world::map::{Direction, METATILE_WIDTH, TilePosition};
use crate::world::movement::MovementMode;

fn pokemon_is_egg(pokemon: &Pokemon) -> bool {
    pokemon.status.as_deref() == Some("EGG") || pokemon.species.id == "EGG"
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecialRoutineOutcome {
    pub routine: String,
    pub effect: SpecialRoutineEffect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SpecialRoutineEffect {
    Noop,
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
    RandomUnseenWildMon {
        contact_id: String,
        map_name: String,
        species: Option<String>,
        already_seen: bool,
        script_value: u8,
        rng_seed_after: u32,
    },
    RandomPhoneWildMon {
        contact_id: String,
        map_name: String,
        time_of_day: TimeOfDay,
        species: String,
        rng_seed_after: u32,
    },
    RandomPhoneMon {
        contact_id: String,
        trainer_id: String,
        species: String,
        party_index: usize,
        rng_seed_after: u32,
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
    UnusedCheckUnusedTwoDayTimer {
        start_day: u8,
        current_day: u8,
        elapsed_days: u8,
        remaining_days: u8,
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
    UnownChamber {
        chamber: String,
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
    BugContestTimer {
        active: bool,
        minutes_remaining: u8,
        seconds_remaining: u8,
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
    BugContestCaughtMonResolved {
        kept: bool,
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
        coins_before: u16,
        bet: u8,
        payout: u16,
        matched_symbol: Option<String>,
        winning_lines: Vec<String>,
        coins: u16,
        rng_seed_after: u32,
    },
    CardFlip {
        coins_before: u16,
        card_index: usize,
        card_name: String,
        payout: u16,
        coins: u16,
        rng_seed_after: u32,
    },
    UnownPuzzle {
        puzzle_id: String,
        solved: bool,
        moves: u16,
        layout: Vec<Vec<u8>>,
        holding_piece: Option<u8>,
        rng_seed_after: u32,
    },
    UnusedMemoryGame {
        matched: bool,
        symbol: Option<String>,
        first_index: usize,
        second_index: usize,
        coins: u16,
        rng_seed_after: u32,
    },
    UnusedFindItemInPcOrBag {
        item_id: String,
        found_in_pc: bool,
        found_in_bag: bool,
        script_value: u8,
    },
    Function11ba38 {
        selected_party_slot: usize,
        other_usable_party_mon: bool,
        script_value: u8,
    },
    GameCornerGameUnavailable {
        game: String,
        reason: GameCornerUnavailableReason,
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
#[serde(deny_unknown_fields)]
pub enum SpecialRoutineError {
    #[error("unsupported exact special routine {routine}")]
    UnsupportedRoutine { routine: String },
    #[error("declared special routine {routine} is inactive in the definitive modpack scripts")]
    InactiveDeclaredRoutine { routine: String },
    #[error("special routine {routine} has invalid state: {message}")]
    InvalidState { routine: String, message: String },
    #[error(
        "saved pending_special_battle_type {battle_type} is not declared by compiled scripted battles or special routines"
    )]
    SavedPendingSpecialBattleTypeMissing { battle_type: String },
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
    #[error("special routine {routine} has invalid Unown puzzle state: {message}")]
    InvalidUnownPuzzleState { routine: String, message: String },
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
    #[error("special routine {routine} mobile password must be exact text")]
    InvalidMobilePassword { routine: String },
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
    #[error("special routine {routine} has invalid Buena password guess {guess}")]
    InvalidBuenaPasswordGuess { routine: String, guess: String },
    #[error(
        "saved buenas_password.category_index {index} is outside compiled Buena password categories"
    )]
    SavedBuenaPasswordCategoryIndexOutOfRange { index: usize },
    #[error(
        "saved buenas_password.category_index {index} references missing compiled Buena password category {category_id}"
    )]
    SavedBuenaPasswordMissingCategory { index: usize, category_id: String },
    #[error(
        "saved buenas_password.option_index {index} is outside compiled Buena password category {category_id} options"
    )]
    SavedBuenaPasswordOptionIndexOutOfRange { index: usize, category_id: String },
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
    #[error("special routine {routine} requires a started Bug-Catching Contest timer")]
    BugContestTimerNotStarted { routine: String },
    #[error("special routine {routine} has invalid Bug-Catching Contest config: {message}")]
    InvalidBugContestConfig { routine: String, message: String },
    #[error("special routine {routine} requires Battle Tower rules from the modpack")]
    MissingBattleTowerRules { routine: String },
    #[error("special routine {routine} has invalid Battle Tower rules: {message}")]
    InvalidBattleTowerRules { routine: String, message: String },
    #[error("saved active Battle Tower state requires compiled Battle Tower rules")]
    SavedBattleTowerMissingRules,
    #[error(
        "saved battle_tower.level_group {level_group} is outside compiled Battle Tower range {minimum}..={maximum}"
    )]
    SavedBattleTowerLevelGroupOutOfRange {
        level_group: u8,
        minimum: u8,
        maximum: u8,
    },
    #[error(
        "saved {field} has {len} entries, compiled Battle Tower challenge_streak_length is {max_len}"
    )]
    SavedBattleTowerRecordTooLong {
        field: String,
        len: usize,
        max_len: usize,
    },
    #[error("saved battle_tower.selected_party_indexes slot {party_index} has no party Pokemon")]
    SavedBattleTowerEmptySelectedPartySlot { party_index: usize },
    #[error("saved battle_tower.selected_party_indexes slot {party_index} is outside saved party")]
    SavedBattleTowerSelectedPartySlotOutOfRange { party_index: usize },
    #[error("saved magikarp_record requires compiled Magikarp length definitions")]
    SavedMagikarpRecordRequiresLengthDefinitions,
    #[error(
        "special routine {routine} has invalid Battle Tower level group {level_group}; expected {minimum}..={maximum}"
    )]
    InvalidBattleTowerLevelGroup {
        routine: String,
        level_group: u8,
        minimum: u8,
        maximum: u8,
    },
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
    #[error("special routine {routine} requires VAR_CALLERID from script runtime variables")]
    MissingCallerId { routine: String },
    #[error("special routine {routine} references unknown phone contact {contact_id}")]
    UnknownPhoneContact { routine: String, contact_id: String },
    #[error("special routine {routine} phone contact {contact_id} has no map constant")]
    MissingPhoneContactMap { routine: String, contact_id: String },
    #[error("special routine {routine} phone contact {contact_id} has no trainer label")]
    MissingPhoneContactTrainer { routine: String, contact_id: String },
    #[error("special routine {routine} requires wild encounter table for caller map {map_name}")]
    MissingCallerWildEncounter { routine: String, map_name: String },
    #[error("special routine {routine} requires grass encounters for caller map {map_name}")]
    MissingCallerGrassEncounter { routine: String, map_name: String },
    #[error(
        "special routine {routine} caller map {map_name} has too few grass encounter slots: expected at least {expected}, found {found}"
    )]
    TooFewCallerGrassSlots {
        routine: String,
        map_name: String,
        expected: usize,
        found: usize,
    },
    #[error("special routine {routine} references unknown trainer {trainer_id}")]
    UnknownTrainer { routine: String, trainer_id: String },
    #[error("special routine {routine} trainer {trainer_id} has no party Pokemon")]
    EmptyTrainerParty { routine: String, trainer_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum LuckyNumberWinnerSource {
    Party,
    Pc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum GameCornerUnavailableReason {
    NoCoins,
    MissingCoinCase,
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
    pub roaming_pokemon: &'a RoamingPokemonDefinitions,
    pub buena_password_categories: &'a BuenaPasswordCategories,
    pub buena_prizes: &'a BuenaPrizeDefinitions,
    pub kurt_apricorn_recipes: &'a KurtApricornRecipes,
    pub shuckie_gift: Option<&'a ShuckieGiftDefinition>,
    pub dratini_move_sets: &'a DratiniMoveSets,
    pub bug_contest_config: Option<&'a BugContestConfig>,
    pub battle_tower_rules: Option<&'a BattleTowerRules>,
    pub magikarp_lengths: &'a [MagikarpLengthEntry],
    pub happiness_data: Option<&'a HappinessData>,
    pub trainer_catalog: &'a TrainerCatalog,
    pub phone_contacts: &'a PhoneContactCatalog,
    pub wild_encounters: &'a BTreeMap<String, WildEncounterData>,
    pub odd_egg_definitions: &'a [OddEggDefinition],
    pub oak_ratings: &'a [OakRatingEntry],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BugContestConfig {
    pub park_balls: u8,
    pub timer_minutes: u8,
    pub timer_seconds: u8,
    pub selected_contestant_count: usize,
    pub contestant_flags: Vec<String>,
}

impl<'de> Deserialize<'de> for BugContestConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct RawConfig {
            park_balls: u8,
            timer_minutes: u8,
            timer_seconds: u8,
            selected_contestant_count: usize,
            contestant_flags: Vec<String>,
        }

        let raw = RawConfig::deserialize(deserializer)?;
        if raw.park_balls == 0 {
            return Err(serde::de::Error::custom(
                "bug contest parkBalls must be nonzero",
            ));
        }
        if raw.timer_minutes == 0 && raw.timer_seconds == 0 {
            return Err(serde::de::Error::custom(
                "bug contest timer must be nonzero",
            ));
        }
        if raw.timer_seconds > 59 {
            return Err(serde::de::Error::custom(format!(
                "bug contest timerSeconds must be 0..59, found {}",
                raw.timer_seconds
            )));
        }
        if raw.selected_contestant_count == 0 {
            return Err(serde::de::Error::custom(
                "bug contest selectedContestantCount must be nonzero",
            ));
        }
        if raw.contestant_flags.len() < raw.selected_contestant_count {
            return Err(serde::de::Error::custom(format!(
                "bug contest selectedContestantCount {} exceeds contestant flag count {}",
                raw.selected_contestant_count,
                raw.contestant_flags.len()
            )));
        }
        let mut seen = BTreeSet::new();
        for (index, flag) in raw.contestant_flags.iter().enumerate() {
            require_special_token(&format!("bug contest contestantFlags[{index}]"), flag)
                .map_err(serde::de::Error::custom)?;
            if !seen.insert(flag.as_str()) {
                return Err(serde::de::Error::custom(format!(
                    "bug contest contestantFlags[{index}] duplicates {flag:?}"
                )));
            }
        }
        Ok(Self {
            park_balls: raw.park_balls,
            timer_minutes: raw.timer_minutes,
            timer_seconds: raw.timer_seconds,
            selected_contestant_count: raw.selected_contestant_count,
            contestant_flags: raw.contestant_flags,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum BugContestConfigIssue {
    MissingParkBalls,
    InvalidTimerSeconds {
        timer_seconds: u8,
    },
    MissingSelectedContestantCount,
    SelectedContestantCountExceedsFlags {
        selected_contestant_count: usize,
        contestant_flag_count: usize,
    },
    InvalidContestantFlag {
        index: usize,
        flag: String,
    },
    DuplicateContestantFlag {
        index: usize,
        flag: String,
    },
    UnknownContestantFlag {
        index: usize,
        flag: String,
    },
}

pub fn bug_contest_config_issues(
    config: &BugContestConfig,
    event_flags: &BTreeSet<String>,
) -> Vec<BugContestConfigIssue> {
    let mut issues = Vec::new();
    if config.park_balls == 0 {
        issues.push(BugContestConfigIssue::MissingParkBalls);
    }
    if config.timer_seconds > 59 {
        issues.push(BugContestConfigIssue::InvalidTimerSeconds {
            timer_seconds: config.timer_seconds,
        });
    }
    if config.selected_contestant_count == 0 {
        issues.push(BugContestConfigIssue::MissingSelectedContestantCount);
    }
    if config.contestant_flags.len() < config.selected_contestant_count {
        issues.push(BugContestConfigIssue::SelectedContestantCountExceedsFlags {
            selected_contestant_count: config.selected_contestant_count,
            contestant_flag_count: config.contestant_flags.len(),
        });
    }
    let mut seen = BTreeSet::new();
    for (index, flag) in config.contestant_flags.iter().enumerate() {
        if !is_exact_nonempty_special_token(flag) {
            issues.push(BugContestConfigIssue::InvalidContestantFlag {
                index,
                flag: flag.clone(),
            });
            continue;
        }
        if !seen.insert(flag.as_str()) {
            issues.push(BugContestConfigIssue::DuplicateContestantFlag {
                index,
                flag: flag.clone(),
            });
        }
        if !event_flags.contains(flag.as_str()) {
            issues.push(BugContestConfigIssue::UnknownContestantFlag {
                index,
                flag: flag.clone(),
            });
        }
    }
    issues
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BattleTowerRules {
    pub banned_species: BTreeMap<String, BattleTowerBannedSpeciesRule>,
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

impl<'de> Deserialize<'de> for BattleTowerRules {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct RawRules {
            banned_species: BTreeMap<String, BattleTowerBannedSpeciesRule>,
            required_party_count: usize,
            challenge_streak_length: u8,
            minimum_level_group: u8,
            maximum_level_group: u8,
            level_group_size: u8,
            party_count_failure_text: String,
            duplicate_species_failure_text: String,
            duplicate_held_item_failure_text: String,
            egg_failure_text: String,
        }

        let raw = RawRules::deserialize(deserializer)?;
        if raw.required_party_count == 0 {
            return Err(serde::de::Error::custom(
                "battle tower requiredPartyCount must be nonzero",
            ));
        }
        if raw.challenge_streak_length == 0 {
            return Err(serde::de::Error::custom(
                "battle tower challengeStreakLength must be nonzero",
            ));
        }
        if raw.level_group_size == 0 {
            return Err(serde::de::Error::custom(
                "battle tower levelGroupSize must be nonzero",
            ));
        }
        if raw.minimum_level_group == 0 || raw.maximum_level_group < raw.minimum_level_group {
            return Err(serde::de::Error::custom(
                "battle tower level group range must be nonzero and ordered",
            ));
        }
        for (field, value) in [
            (
                "battle tower partyCountFailureText",
                raw.party_count_failure_text.as_str(),
            ),
            (
                "battle tower duplicateSpeciesFailureText",
                raw.duplicate_species_failure_text.as_str(),
            ),
            (
                "battle tower duplicateHeldItemFailureText",
                raw.duplicate_held_item_failure_text.as_str(),
            ),
            ("battle tower eggFailureText", raw.egg_failure_text.as_str()),
        ] {
            require_special_token(field, value).map_err(serde::de::Error::custom)?;
        }
        for species_id in raw.banned_species.keys() {
            require_special_token("battle tower bannedSpecies key", species_id)
                .map_err(serde::de::Error::custom)?;
        }

        Ok(Self {
            banned_species: raw.banned_species,
            required_party_count: raw.required_party_count,
            challenge_streak_length: raw.challenge_streak_length,
            minimum_level_group: raw.minimum_level_group,
            maximum_level_group: raw.maximum_level_group,
            level_group_size: raw.level_group_size,
            party_count_failure_text: raw.party_count_failure_text,
            duplicate_species_failure_text: raw.duplicate_species_failure_text,
            duplicate_held_item_failure_text: raw.duplicate_held_item_failure_text,
            egg_failure_text: raw.egg_failure_text,
        })
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BattleTowerBannedSpeciesRule {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum BattleTowerFailureTextField {
    PartyCount,
    DuplicateSpecies,
    DuplicateHeldItem,
    Egg,
}

impl BattleTowerFailureTextField {
    pub const fn subject(self) -> &'static str {
        match self {
            Self::PartyCount => "battle_tower_rules:partyCountFailureText",
            Self::DuplicateSpecies => "battle_tower_rules:duplicateSpeciesFailureText",
            Self::DuplicateHeldItem => "battle_tower_rules:duplicateHeldItemFailureText",
            Self::Egg => "battle_tower_rules:eggFailureText",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum BattleTowerRulesIssue {
    MissingRequiredPartyCount,
    MissingChallengeStreakLength,
    MissingLevelGroupSize,
    InvalidLevelGroupRange,
    InvalidFailureText {
        field: BattleTowerFailureTextField,
        text_id: String,
    },
    InvalidBannedSpecies {
        species_id: String,
    },
    UnknownBannedSpecies {
        species_id: String,
    },
}

pub fn battle_tower_rules_issues(
    rules: &BattleTowerRules,
    species_ids: &BTreeSet<String>,
) -> Vec<BattleTowerRulesIssue> {
    let mut issues = Vec::new();
    if rules.required_party_count == 0 {
        issues.push(BattleTowerRulesIssue::MissingRequiredPartyCount);
    }
    if rules.challenge_streak_length == 0 {
        issues.push(BattleTowerRulesIssue::MissingChallengeStreakLength);
    }
    if rules.level_group_size == 0 {
        issues.push(BattleTowerRulesIssue::MissingLevelGroupSize);
    }
    if rules.minimum_level_group == 0 || rules.maximum_level_group < rules.minimum_level_group {
        issues.push(BattleTowerRulesIssue::InvalidLevelGroupRange);
    }
    for (field, value) in [
        (
            BattleTowerFailureTextField::PartyCount,
            rules.party_count_failure_text.as_str(),
        ),
        (
            BattleTowerFailureTextField::DuplicateSpecies,
            rules.duplicate_species_failure_text.as_str(),
        ),
        (
            BattleTowerFailureTextField::DuplicateHeldItem,
            rules.duplicate_held_item_failure_text.as_str(),
        ),
        (
            BattleTowerFailureTextField::Egg,
            rules.egg_failure_text.as_str(),
        ),
    ] {
        if !is_exact_nonempty_special_token(value) {
            issues.push(BattleTowerRulesIssue::InvalidFailureText {
                field,
                text_id: value.to_string(),
            });
        }
    }
    for species_id in rules.banned_species.keys() {
        if !is_exact_nonempty_special_token(species_id) {
            issues.push(BattleTowerRulesIssue::InvalidBannedSpecies {
                species_id: species_id.clone(),
            });
            continue;
        }
        if !species_ids.contains(species_id.as_str()) {
            issues.push(BattleTowerRulesIssue::UnknownBannedSpecies {
                species_id: species_id.clone(),
            });
        }
    }
    issues
}

pub fn is_default_battle_tower_trainer_history(trainer_history: &[u8]) -> bool {
    trainer_history.len() == 7 && trainer_history.iter().all(|trainer_id| *trainer_id == 0xff)
}

pub fn saved_battle_tower_state_is_active(tower: &BattleTowerState) -> bool {
    tower.challenge_state != 0
        || tower.beaten_trainers != 0
        || tower.level_group != 0
        || tower.reward_given
        || tower.quick_saved
        || tower.explanation_read
        || tower.save_file_flags != 0
        || tower.gs_ball_flag
        || tower.record_state != 0
        || tower.record_last_day.is_some()
        || tower.record_reset_counter != 0
        || tower.leaderboard_acknowledged
        || tower.last_rule_failure.is_some()
        || tower.loaded_trainer_id.is_some()
        || tower.last_sprite_constant.is_some()
        || !tower.selected_party_indexes.is_empty()
        || !tower.mobile_flags.is_empty()
        || !is_default_battle_tower_trainer_history(&tower.trainer_history)
        || !tower.record_streaks.is_empty()
        || !tower.record_outcomes.is_empty()
        || !tower.record_days.is_empty()
}

pub fn validate_saved_battle_tower_state(
    tower: &BattleTowerState,
    party: &Party,
    rules: Option<&BattleTowerRules>,
) -> Result<(), SpecialRoutineError> {
    if saved_battle_tower_state_is_active(tower) {
        let rules = rules.ok_or(SpecialRoutineError::SavedBattleTowerMissingRules)?;
        if tower.level_group != 0
            && (tower.level_group < rules.minimum_level_group
                || tower.level_group > rules.maximum_level_group)
        {
            return Err(SpecialRoutineError::SavedBattleTowerLevelGroupOutOfRange {
                level_group: tower.level_group,
                minimum: rules.minimum_level_group,
                maximum: rules.maximum_level_group,
            });
        }
        validate_saved_battle_tower_record_len(
            "battle_tower.trainer_history",
            tower.trainer_history.len(),
            rules.challenge_streak_length,
        )?;
        validate_saved_battle_tower_record_len(
            "battle_tower.record_streaks",
            tower.record_streaks.len(),
            rules.challenge_streak_length,
        )?;
        validate_saved_battle_tower_record_len(
            "battle_tower.record_outcomes",
            tower.record_outcomes.len(),
            rules.challenge_streak_length,
        )?;
        validate_saved_battle_tower_record_len(
            "battle_tower.record_days",
            tower.record_days.len(),
            rules.challenge_streak_length,
        )?;
    }
    for party_index in &tower.selected_party_indexes {
        match party.pokemon.get(*party_index) {
            Some(Some(_)) => {}
            Some(None) => {
                return Err(
                    SpecialRoutineError::SavedBattleTowerEmptySelectedPartySlot {
                        party_index: *party_index,
                    },
                );
            }
            None => {
                return Err(
                    SpecialRoutineError::SavedBattleTowerSelectedPartySlotOutOfRange {
                        party_index: *party_index,
                    },
                );
            }
        }
    }
    Ok(())
}

fn validate_saved_battle_tower_record_len(
    field: &str,
    len: usize,
    challenge_streak_length: u8,
) -> Result<(), SpecialRoutineError> {
    let max_len = usize::from(challenge_streak_length);
    if len > max_len {
        return Err(SpecialRoutineError::SavedBattleTowerRecordTooLong {
            field: field.to_string(),
            len,
            max_len,
        });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OakRatingEntry {
    pub caught_count_limit: usize,
    pub fanfare: String,
    pub text_label: String,
}

impl<'de> Deserialize<'de> for OakRatingEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct RawEntry {
            caught_count_limit: usize,
            fanfare: String,
            text_label: String,
        }

        let raw = RawEntry::deserialize(deserializer)?;
        require_special_token("oak rating fanfare", &raw.fanfare)
            .map_err(serde::de::Error::custom)?;
        require_special_token("oak rating textLabel", &raw.text_label)
            .map_err(serde::de::Error::custom)?;
        Ok(Self {
            caught_count_limit: raw.caught_count_limit,
            fanfare: raw.fanfare,
            text_label: raw.text_label,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct OakRatingTable(pub Vec<OakRatingEntry>);

impl<'de> Deserialize<'de> for OakRatingTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let entries = Vec::<OakRatingEntry>::deserialize(deserializer)?;
        if entries.is_empty() {
            return Err(serde::de::Error::custom(
                "Oak rating table must not be empty",
            ));
        }
        let mut previous_limit = None;
        for (index, entry) in entries.iter().enumerate() {
            if let Some(previous) = previous_limit
                && entry.caught_count_limit <= previous
            {
                return Err(serde::de::Error::custom(format!(
                    "Oak rating entry {index} caughtCountLimit must increase"
                )));
            }
            previous_limit = Some(entry.caught_count_limit);
        }
        Ok(Self(entries))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum OakRatingTableIssue {
    InvalidFanfare {
        index: usize,
        fanfare: String,
    },
    InvalidTextLabel {
        index: usize,
        text_label: String,
    },
    InvalidOrder {
        index: usize,
        caught_count_limit: usize,
        previous_limit: usize,
    },
    IncompleteCoverage {
        pokemon_count: usize,
        last_caught_count_limit: usize,
    },
}

pub fn oak_rating_table_issues(
    entries: &[OakRatingEntry],
    pokemon_count: usize,
) -> Vec<OakRatingTableIssue> {
    let mut issues = Vec::new();
    let mut previous_limit = None;
    for (index, entry) in entries.iter().enumerate() {
        if !is_exact_nonempty_special_token(&entry.fanfare) {
            issues.push(OakRatingTableIssue::InvalidFanfare {
                index,
                fanfare: entry.fanfare.clone(),
            });
        }
        if !is_exact_nonempty_special_token(&entry.text_label) {
            issues.push(OakRatingTableIssue::InvalidTextLabel {
                index,
                text_label: entry.text_label.clone(),
            });
        }
        if let Some(previous) = previous_limit
            && entry.caught_count_limit <= previous
        {
            issues.push(OakRatingTableIssue::InvalidOrder {
                index,
                caught_count_limit: entry.caught_count_limit,
                previous_limit: previous,
            });
        }
        previous_limit = Some(entry.caught_count_limit);
    }
    if let Some(last) = entries.last()
        && pokemon_count > 0
        && last.caught_count_limit < pokemon_count
    {
        issues.push(OakRatingTableIssue::IncompleteCoverage {
            pokemon_count,
            last_caught_count_limit: last.caught_count_limit,
        });
    }
    issues
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

impl<'de> Deserialize<'de> for OddEggDefinition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct RawDefinition {
            species: String,
            moves: Vec<String>,
            original_trainer_id: u16,
            dvs: [u8; 4],
            probability: u16,
            level: u8,
            experience: i32,
            hatch_cycles: u8,
            nickname: String,
            original_trainer_name: String,
        }

        let raw = RawDefinition::deserialize(deserializer)?;
        require_special_token("odd egg species", &raw.species).map_err(serde::de::Error::custom)?;
        if raw.moves.is_empty() || raw.moves.len() > 4 {
            return Err(serde::de::Error::custom(format!(
                "odd egg move list must contain 1..4 moves, found {}",
                raw.moves.len()
            )));
        }
        for (index, move_id) in raw.moves.iter().enumerate() {
            require_special_token(&format!("odd egg moves[{index}]"), move_id)
                .map_err(serde::de::Error::custom)?;
        }
        for (index, dv) in raw.dvs.iter().enumerate() {
            if *dv > 15 {
                return Err(serde::de::Error::custom(format!(
                    "odd egg dvs[{index}] must be 0..15, found {dv}"
                )));
            }
        }
        if raw.probability == 0 {
            return Err(serde::de::Error::custom(
                "odd egg probability must be nonzero",
            ));
        }
        if raw.level == 0 || raw.level > 100 {
            return Err(serde::de::Error::custom(format!(
                "odd egg level must be 1..100, found {}",
                raw.level
            )));
        }
        if raw.experience < 0 {
            return Err(serde::de::Error::custom(format!(
                "odd egg experience must be nonnegative, found {}",
                raw.experience
            )));
        }
        if raw.hatch_cycles == 0 {
            return Err(serde::de::Error::custom(
                "odd egg hatchCycles must be nonzero",
            ));
        }
        require_special_text("odd egg nickname", &raw.nickname)
            .map_err(serde::de::Error::custom)?;
        require_special_text("odd egg originalTrainerName", &raw.original_trainer_name)
            .map_err(serde::de::Error::custom)?;

        Ok(Self {
            species: raw.species,
            moves: raw.moves,
            original_trainer_id: raw.original_trainer_id,
            dvs: raw.dvs,
            probability: raw.probability,
            level: raw.level,
            experience: raw.experience,
            hatch_cycles: raw.hatch_cycles,
            nickname: raw.nickname,
            original_trainer_name: raw.original_trainer_name,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct OddEggDefinitionTable(pub Vec<OddEggDefinition>);

impl<'de> Deserialize<'de> for OddEggDefinitionTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let definitions = Vec::<OddEggDefinition>::deserialize(deserializer)?;
        if definitions.is_empty() {
            return Err(serde::de::Error::custom(
                "Odd Egg definitions must not be empty",
            ));
        }
        let total_probability = definitions
            .iter()
            .map(|definition| u32::from(definition.probability))
            .sum::<u32>();
        if total_probability != 100 {
            return Err(serde::de::Error::custom(format!(
                "Odd Egg definition probabilities must total 100, got {total_probability}"
            )));
        }
        Ok(Self(definitions))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum OddEggDefinitionIssue {
    InvalidProbabilityTotal {
        total_probability: u32,
    },
    InvalidSpecies {
        index: usize,
        species_id: String,
    },
    UnknownSpecies {
        index: usize,
        species_id: String,
    },
    InvalidMoveCount {
        index: usize,
        move_count: usize,
    },
    InvalidMove {
        index: usize,
        move_index: usize,
        move_id: String,
    },
    UnknownMove {
        index: usize,
        move_index: usize,
        move_id: String,
    },
    InvalidProbability {
        index: usize,
    },
    InvalidLevel {
        index: usize,
        level: u8,
    },
    InvalidNickname {
        index: usize,
        nickname: String,
    },
    InvalidOriginalTrainerName {
        index: usize,
        original_trainer_name: String,
    },
}

pub fn odd_egg_definition_issues(
    definitions: &[OddEggDefinition],
    species_ids: &BTreeSet<String>,
    move_ids: &BTreeSet<String>,
) -> Vec<OddEggDefinitionIssue> {
    let mut issues = Vec::new();
    if !definitions.is_empty() {
        let total_probability = definitions
            .iter()
            .map(|definition| u32::from(definition.probability))
            .sum::<u32>();
        if total_probability != 100 {
            issues.push(OddEggDefinitionIssue::InvalidProbabilityTotal { total_probability });
        }
    }

    for (index, definition) in definitions.iter().enumerate() {
        if !is_exact_nonempty_special_token(&definition.species) {
            issues.push(OddEggDefinitionIssue::InvalidSpecies {
                index,
                species_id: definition.species.clone(),
            });
        } else if !species_ids.contains(definition.species.as_str()) {
            issues.push(OddEggDefinitionIssue::UnknownSpecies {
                index,
                species_id: definition.species.clone(),
            });
        }
        if definition.moves.is_empty() || definition.moves.len() > 4 {
            issues.push(OddEggDefinitionIssue::InvalidMoveCount {
                index,
                move_count: definition.moves.len(),
            });
        }
        for (move_index, move_id) in definition.moves.iter().enumerate() {
            if !is_exact_nonempty_special_token(move_id) {
                issues.push(OddEggDefinitionIssue::InvalidMove {
                    index,
                    move_index,
                    move_id: move_id.clone(),
                });
            } else if !move_ids.contains(move_id.as_str()) {
                issues.push(OddEggDefinitionIssue::UnknownMove {
                    index,
                    move_index,
                    move_id: move_id.clone(),
                });
            }
        }
        if definition.probability == 0 {
            issues.push(OddEggDefinitionIssue::InvalidProbability { index });
        }
        if definition.level == 0 || definition.level > 100 {
            issues.push(OddEggDefinitionIssue::InvalidLevel {
                index,
                level: definition.level,
            });
        }
        if definition.nickname.trim().is_empty()
            || definition.nickname.trim() != definition.nickname
        {
            issues.push(OddEggDefinitionIssue::InvalidNickname {
                index,
                nickname: definition.nickname.clone(),
            });
        }
        if definition.original_trainer_name.trim().is_empty()
            || definition.original_trainer_name.trim() != definition.original_trainer_name
        {
            issues.push(OddEggDefinitionIssue::InvalidOriginalTrainerName {
                index,
                original_trainer_name: definition.original_trainer_name.clone(),
            });
        }
    }

    issues
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MagikarpLengthEntry {
    pub threshold: u16,
    pub divisor: u16,
}

impl<'de> Deserialize<'de> for MagikarpLengthEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct RawEntry {
            threshold: u16,
            divisor: u16,
        }

        let raw = RawEntry::deserialize(deserializer)?;
        if raw.divisor == 0 {
            return Err(serde::de::Error::custom(
                "magikarp length divisor must be nonzero",
            ));
        }
        Ok(Self {
            threshold: raw.threshold,
            divisor: raw.divisor,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct MagikarpLengthTable(pub Vec<MagikarpLengthEntry>);

impl<'de> Deserialize<'de> for MagikarpLengthTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let entries = Vec::<MagikarpLengthEntry>::deserialize(deserializer)?;
        if entries.is_empty() {
            return Err(serde::de::Error::custom(
                "Magikarp length table must not be empty",
            ));
        }
        let mut previous_threshold = None;
        for (index, entry) in entries.iter().enumerate() {
            if let Some(previous) = previous_threshold
                && entry.threshold <= previous
            {
                return Err(serde::de::Error::custom(format!(
                    "Magikarp length entry {index} threshold must increase"
                )));
            }
            previous_threshold = Some(entry.threshold);
        }
        Ok(Self(entries))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum MagikarpLengthTableIssue {
    InvalidDivisor {
        index: usize,
        threshold: u16,
    },
    InvalidThresholdOrder {
        index: usize,
        threshold: u16,
        previous_threshold: u16,
    },
}

pub fn magikarp_length_table_issues(
    entries: &[MagikarpLengthEntry],
) -> Vec<MagikarpLengthTableIssue> {
    let mut issues = Vec::new();
    let mut previous_threshold = None;
    for (index, entry) in entries.iter().enumerate() {
        if entry.divisor == 0 {
            issues.push(MagikarpLengthTableIssue::InvalidDivisor {
                index,
                threshold: entry.threshold,
            });
        }
        if let Some(previous) = previous_threshold
            && entry.threshold <= previous
        {
            issues.push(MagikarpLengthTableIssue::InvalidThresholdOrder {
                index,
                threshold: entry.threshold,
                previous_threshold: previous,
            });
        }
        previous_threshold = Some(entry.threshold);
    }
    issues
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HappinessData {
    pub changes: BTreeMap<u8, HappinessChangeEntry>,
    pub services: BTreeMap<String, Vec<HappinessServiceOutcome>>,
}

impl<'de> Deserialize<'de> for HappinessData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct RawData {
            changes: BTreeMap<u8, HappinessChangeEntry>,
            services: BTreeMap<String, Vec<HappinessServiceOutcome>>,
        }

        let raw = RawData::deserialize(deserializer)?;
        if raw.changes.is_empty() {
            return Err(serde::de::Error::custom(
                "happiness changes must not be empty",
            ));
        }
        if raw.services.is_empty() {
            return Err(serde::de::Error::custom(
                "happiness services must not be empty",
            ));
        }
        for (routine, outcomes) in &raw.services {
            require_special_token("happiness service routine", routine)
                .map_err(serde::de::Error::custom)?;
            if outcomes.is_empty() {
                return Err(serde::de::Error::custom(format!(
                    "happiness service {routine} must declare outcomes"
                )));
            }
            for outcome in outcomes {
                if !raw.changes.contains_key(&outcome.change_code) {
                    return Err(serde::de::Error::custom(format!(
                        "happiness service {routine} references unknown change code {}",
                        outcome.change_code
                    )));
                }
            }
        }
        let mut code_names = BTreeSet::new();
        for (change_code, entry) in &raw.changes {
            if !code_names.insert(entry.code.as_str()) {
                return Err(serde::de::Error::custom(format!(
                    "happiness change code {} duplicates code name {}",
                    change_code, entry.code
                )));
            }
        }
        Ok(Self {
            changes: raw.changes,
            services: raw.services,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HappinessChangeEntry {
    pub code: String,
    pub low: i16,
    pub mid: i16,
    pub high: i16,
}

impl<'de> Deserialize<'de> for HappinessChangeEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct RawEntry {
            code: String,
            low: i16,
            mid: i16,
            high: i16,
        }

        let raw = RawEntry::deserialize(deserializer)?;
        require_special_token("happiness change code", &raw.code)
            .map_err(serde::de::Error::custom)?;
        Ok(Self {
            code: raw.code,
            low: raw.low,
            mid: raw.mid,
            high: raw.high,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HappinessServiceOutcome {
    pub roll_weight: u8,
    pub script_value: u8,
    pub change_code: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum HappinessDataIssue {
    EmptyChanges,
    EmptyChangeCode { change_code: u8 },
    InvalidChangeCode { code: String, change_code: u8 },
    DuplicateChangeCode { code: String, change_code: u8 },
    EmptyServices,
    EmptyServiceRoutine { routine: String },
    InvalidServiceRoutine { routine: String },
    EmptyServiceOutcomes { routine: String },
    UnknownServiceChange { routine: String, change_code: u8 },
}

pub fn happiness_data_issues(data: &HappinessData) -> Vec<HappinessDataIssue> {
    let mut issues = Vec::new();
    if data.changes.is_empty() {
        issues.push(HappinessDataIssue::EmptyChanges);
    }

    let mut code_names = BTreeSet::new();
    for (change_code, entry) in &data.changes {
        if entry.code.trim().is_empty() {
            issues.push(HappinessDataIssue::EmptyChangeCode {
                change_code: *change_code,
            });
        } else if !is_exact_nonempty_special_token(&entry.code) {
            issues.push(HappinessDataIssue::InvalidChangeCode {
                code: entry.code.clone(),
                change_code: *change_code,
            });
        }
        if !code_names.insert(entry.code.clone()) {
            issues.push(HappinessDataIssue::DuplicateChangeCode {
                code: entry.code.clone(),
                change_code: *change_code,
            });
        }
    }

    if data.services.is_empty() {
        issues.push(HappinessDataIssue::EmptyServices);
    }
    for (routine, outcomes) in &data.services {
        if routine.trim().is_empty() {
            issues.push(HappinessDataIssue::EmptyServiceRoutine {
                routine: routine.clone(),
            });
        } else if !is_exact_nonempty_special_token(routine) {
            issues.push(HappinessDataIssue::InvalidServiceRoutine {
                routine: routine.clone(),
            });
        }
        if outcomes.is_empty() {
            issues.push(HappinessDataIssue::EmptyServiceOutcomes {
                routine: routine.clone(),
            });
        }
        for outcome in outcomes {
            if !data.changes.contains_key(&outcome.change_code) {
                issues.push(HappinessDataIssue::UnknownServiceChange {
                    routine: routine.clone(),
                    change_code: outcome.change_code,
                });
            }
        }
    }

    issues
}

pub type DratiniMoveSets = BTreeMap<u8, Vec<String>>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct DratiniMoveSetTable(pub DratiniMoveSets);

impl<'de> Deserialize<'de> for DratiniMoveSetTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let move_sets = DratiniMoveSets::deserialize(deserializer)?;
        if move_sets.is_empty() {
            return Err(serde::de::Error::custom(
                "Dratini move sets must not be empty",
            ));
        }
        for (mode, moves) in &move_sets {
            if moves.is_empty() {
                return Err(serde::de::Error::custom(format!(
                    "Dratini move set {mode} must not be empty"
                )));
            }
            if moves.len() > 4 {
                return Err(serde::de::Error::custom(format!(
                    "Dratini move set {mode} must contain at most 4 moves"
                )));
            }
            for (move_index, move_id) in moves.iter().enumerate() {
                require_special_token(
                    &format!("Dratini move set {mode} move {move_index}"),
                    move_id,
                )
                .map_err(serde::de::Error::custom)?;
            }
        }
        Ok(Self(move_sets))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum DratiniMoveSetIssue {
    EmptyMoveSet {
        mode: u8,
    },
    InvalidMove {
        mode: u8,
        move_index: usize,
        move_id: String,
    },
    UnknownMove {
        mode: u8,
        move_index: usize,
        move_id: String,
    },
}

pub fn dratini_move_set_issues(
    move_sets: &DratiniMoveSets,
    move_ids: &BTreeSet<String>,
) -> Vec<DratiniMoveSetIssue> {
    let mut issues = Vec::new();
    for (mode, moves) in move_sets {
        if moves.is_empty() {
            issues.push(DratiniMoveSetIssue::EmptyMoveSet { mode: *mode });
        }
        for (move_index, move_id) in moves.iter().enumerate() {
            if !is_exact_nonempty_special_token(move_id) {
                issues.push(DratiniMoveSetIssue::InvalidMove {
                    mode: *mode,
                    move_index,
                    move_id: move_id.clone(),
                });
            } else if !move_ids.contains(move_id.as_str()) {
                issues.push(DratiniMoveSetIssue::UnknownMove {
                    mode: *mode,
                    move_index,
                    move_id: move_id.clone(),
                });
            }
        }
    }
    issues
}

fn is_exact_nonempty_special_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !has_reserved_pack_prefix(value)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn is_exact_nonempty_special_value(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !has_reserved_pack_prefix(value)
        && !value.chars().any(char::is_control)
}

fn require_special_token(field: &str, value: &str) -> Result<(), String> {
    if is_exact_nonempty_special_token(value) {
        Ok(())
    } else {
        Err(format!(
            "{field} must be a nonempty exact pack token, found {value:?}"
        ))
    }
}

fn require_special_text(field: &str, value: &str) -> Result<(), String> {
    if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
        Err(format!(
            "{field} must be nonempty exact text, found {value:?}"
        ))
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

impl<'de> Deserialize<'de> for ShuckieGiftDefinition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct RawGift {
            species: String,
            level: u8,
            held_item: String,
            nickname: String,
            original_trainer_name: String,
            original_trainer_id: u16,
            got_today_engine_flag: String,
        }

        let raw = RawGift::deserialize(deserializer)?;
        require_special_token("shuckie species", &raw.species).map_err(serde::de::Error::custom)?;
        if raw.level == 0 || raw.level > 100 {
            return Err(serde::de::Error::custom(format!(
                "shuckie level must be 1..100, found {}",
                raw.level
            )));
        }
        require_special_token("shuckie heldItem", &raw.held_item)
            .map_err(serde::de::Error::custom)?;
        require_special_text("shuckie nickname", &raw.nickname)
            .map_err(serde::de::Error::custom)?;
        require_special_text("shuckie originalTrainerName", &raw.original_trainer_name)
            .map_err(serde::de::Error::custom)?;
        require_special_token("shuckie gotTodayEngineFlag", &raw.got_today_engine_flag)
            .map_err(serde::de::Error::custom)?;
        Ok(Self {
            species: raw.species,
            level: raw.level,
            held_item: raw.held_item,
            nickname: raw.nickname,
            original_trainer_name: raw.original_trainer_name,
            original_trainer_id: raw.original_trainer_id,
            got_today_engine_flag: raw.got_today_engine_flag,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShuckieGiftIssue {
    EmptySpecies,
    InvalidSpecies { species: String },
    UnknownSpecies { species: String },
    InvalidLevel,
    EmptyHeldItem,
    InvalidHeldItem { held_item: String },
    UnknownHeldItem { held_item: String },
    EmptyName,
    EmptyEngineFlag,
    InvalidEngineFlag { engine_flag: String },
    UnknownEngineFlag { engine_flag: String },
}

pub fn shuckie_gift_issues(
    gift: &ShuckieGiftDefinition,
    species_ids: &BTreeSet<String>,
    item_ids: &BTreeSet<String>,
    engine_flags: &BTreeSet<String>,
) -> Vec<ShuckieGiftIssue> {
    let mut issues = Vec::new();
    if gift.species.trim().is_empty() {
        issues.push(ShuckieGiftIssue::EmptySpecies);
    } else if !is_exact_nonempty_special_token(&gift.species) {
        issues.push(ShuckieGiftIssue::InvalidSpecies {
            species: gift.species.clone(),
        });
    } else if !species_ids.contains(&gift.species) {
        issues.push(ShuckieGiftIssue::UnknownSpecies {
            species: gift.species.clone(),
        });
    }
    if gift.level == 0 {
        issues.push(ShuckieGiftIssue::InvalidLevel);
    }
    if gift.held_item.trim().is_empty() {
        issues.push(ShuckieGiftIssue::EmptyHeldItem);
    } else if !is_exact_nonempty_special_token(&gift.held_item) {
        issues.push(ShuckieGiftIssue::InvalidHeldItem {
            held_item: gift.held_item.clone(),
        });
    } else if !item_ids.contains(&gift.held_item) {
        issues.push(ShuckieGiftIssue::UnknownHeldItem {
            held_item: gift.held_item.clone(),
        });
    }
    if gift.nickname.trim().is_empty() || gift.original_trainer_name.trim().is_empty() {
        issues.push(ShuckieGiftIssue::EmptyName);
    }
    if gift.got_today_engine_flag.trim().is_empty() {
        issues.push(ShuckieGiftIssue::EmptyEngineFlag);
    } else if !is_exact_nonempty_special_token(&gift.got_today_engine_flag) {
        issues.push(ShuckieGiftIssue::InvalidEngineFlag {
            engine_flag: gift.got_today_engine_flag.clone(),
        });
    } else if !engine_flags.contains(&gift.got_today_engine_flag) {
        issues.push(ShuckieGiftIssue::UnknownEngineFlag {
            engine_flag: gift.got_today_engine_flag.clone(),
        });
    }
    issues
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuenaPasswordCategoryDefinition {
    pub category_type: String,
    pub points: u8,
    pub options: Vec<String>,
}

impl<'de> Deserialize<'de> for BuenaPasswordCategoryDefinition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct RawCategory {
            category_type: String,
            points: u8,
            options: Vec<String>,
        }

        let raw = RawCategory::deserialize(deserializer)?;
        require_special_token("buena password categoryType", &raw.category_type)
            .map_err(serde::de::Error::custom)?;
        if !is_known_buena_password_category_type(&raw.category_type) {
            return Err(serde::de::Error::custom(format!(
                "unknown buena password categoryType {:?}",
                raw.category_type
            )));
        }
        if raw.points == 0 {
            return Err(serde::de::Error::custom(
                "buena password category points must be nonzero",
            ));
        }
        if raw.options.is_empty() {
            return Err(serde::de::Error::custom(
                "buena password category options must not be empty",
            ));
        }
        for (index, option) in raw.options.iter().enumerate() {
            if !is_exact_nonempty_special_value(option) {
                return Err(serde::de::Error::custom(format!(
                    "buena password options[{index}] must be exact and nonempty"
                )));
            }
        }
        Ok(Self {
            category_type: raw.category_type,
            points: raw.points,
            options: raw.options,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct BuenaPasswordCategories {
    pub order: Vec<String>,
    pub categories: BTreeMap<String, BuenaPasswordCategoryDefinition>,
}

impl<'de> Deserialize<'de> for BuenaPasswordCategories {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawCategories {
            order: Vec<String>,
            categories: BTreeMap<String, BuenaPasswordCategoryDefinition>,
        }

        let raw = RawCategories::deserialize(deserializer)?;
        if raw.order.is_empty() {
            return Err(serde::de::Error::custom(
                "buena password order must not be empty",
            ));
        }
        if raw.categories.is_empty() {
            return Err(serde::de::Error::custom(
                "buena password categories must not be empty",
            ));
        }
        let mut seen = BTreeSet::new();
        for id in &raw.order {
            require_special_token("buena password order id", id)
                .map_err(serde::de::Error::custom)?;
            if !seen.insert(id.as_str()) {
                return Err(serde::de::Error::custom(format!(
                    "duplicate buena password order id {id:?}"
                )));
            }
            if !raw.categories.contains_key(id) {
                return Err(serde::de::Error::custom(format!(
                    "buena password order id {id:?} has no category"
                )));
            }
        }
        for id in raw.categories.keys() {
            require_special_token("buena password category id", id)
                .map_err(serde::de::Error::custom)?;
            if !seen.contains(id.as_str()) {
                return Err(serde::de::Error::custom(format!(
                    "buena password category id {id:?} missing from order"
                )));
            }
        }
        Ok(Self {
            order: raw.order,
            categories: raw.categories,
        })
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuenaPasswordCategoryIssue {
    EmptyId {
        id: String,
    },
    InvalidId {
        id: String,
    },
    UnknownOrderedId {
        id: String,
    },
    DuplicateOrderedId {
        id: String,
    },
    InvalidCategoryType {
        id: String,
        category_type: String,
    },
    UnknownCategoryType {
        id: String,
        category_type: String,
    },
    InvalidPoints {
        id: String,
    },
    EmptyOptions {
        id: String,
    },
    EmptyOption {
        id: String,
        option_index: usize,
    },
    InvalidOption {
        id: String,
        option_index: usize,
        option: String,
    },
    UnknownSpecies {
        id: String,
        option_index: usize,
        species: String,
    },
    UnknownItem {
        id: String,
        option_index: usize,
        item_id: String,
    },
    UnknownMove {
        id: String,
        option_index: usize,
        move_id: String,
    },
}

pub fn buena_password_category_issues(
    catalog: &BuenaPasswordCategories,
    species_ids: &BTreeSet<String>,
    item_ids: &BTreeSet<String>,
    move_ids: &BTreeSet<String>,
) -> Vec<BuenaPasswordCategoryIssue> {
    let mut issues = Vec::new();
    let mut seen_order = BTreeSet::new();
    for id in &catalog.order {
        if id.is_empty() {
            issues.push(BuenaPasswordCategoryIssue::EmptyId { id: id.clone() });
        } else if !is_exact_nonempty_special_token(id) {
            issues.push(BuenaPasswordCategoryIssue::InvalidId { id: id.clone() });
        } else if !seen_order.insert(id.clone()) {
            issues.push(BuenaPasswordCategoryIssue::DuplicateOrderedId { id: id.clone() });
        } else if !catalog.categories.contains_key(id) {
            issues.push(BuenaPasswordCategoryIssue::UnknownOrderedId { id: id.clone() });
        }
    }
    for (id, category) in &catalog.categories {
        if id.is_empty() {
            issues.push(BuenaPasswordCategoryIssue::EmptyId { id: id.clone() });
        } else if !is_exact_nonempty_special_token(id) {
            issues.push(BuenaPasswordCategoryIssue::InvalidId { id: id.clone() });
        }
        if !seen_order.contains(id) {
            issues.push(BuenaPasswordCategoryIssue::UnknownOrderedId { id: id.clone() });
        }
        if !is_exact_nonempty_special_token(&category.category_type) {
            issues.push(BuenaPasswordCategoryIssue::InvalidCategoryType {
                id: id.clone(),
                category_type: category.category_type.clone(),
            });
        } else if !is_known_buena_password_category_type(&category.category_type) {
            issues.push(BuenaPasswordCategoryIssue::UnknownCategoryType {
                id: id.clone(),
                category_type: category.category_type.clone(),
            });
        }
        if category.points == 0 {
            issues.push(BuenaPasswordCategoryIssue::InvalidPoints { id: id.clone() });
        }
        if category.options.is_empty() {
            issues.push(BuenaPasswordCategoryIssue::EmptyOptions { id: id.clone() });
        }
        for (option_index, option) in category.options.iter().enumerate() {
            if option.is_empty() {
                issues.push(BuenaPasswordCategoryIssue::EmptyOption {
                    id: id.clone(),
                    option_index,
                });
                continue;
            }
            let option_is_valid = if category.category_type == "BUENA_STRING" {
                is_exact_nonempty_special_value(option)
            } else {
                is_exact_nonempty_special_token(option)
            };
            if !option_is_valid {
                issues.push(BuenaPasswordCategoryIssue::InvalidOption {
                    id: id.clone(),
                    option_index,
                    option: option.clone(),
                });
                continue;
            }
            match category.category_type.as_str() {
                BUENA_PASSWORD_CATEGORY_MON if !species_ids.contains(option) => {
                    issues.push(BuenaPasswordCategoryIssue::UnknownSpecies {
                        id: id.clone(),
                        option_index,
                        species: option.clone(),
                    });
                }
                BUENA_PASSWORD_CATEGORY_ITEM if !item_ids.contains(option) => {
                    issues.push(BuenaPasswordCategoryIssue::UnknownItem {
                        id: id.clone(),
                        option_index,
                        item_id: option.clone(),
                    });
                }
                BUENA_PASSWORD_CATEGORY_MOVE if !move_ids.contains(option) => {
                    issues.push(BuenaPasswordCategoryIssue::UnknownMove {
                        id: id.clone(),
                        option_index,
                        move_id: option.clone(),
                    });
                }
                _ => {}
            }
        }
    }
    issues
}

pub type KurtApricornRecipes = BTreeMap<String, String>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct KurtApricornRecipeTable(pub KurtApricornRecipes);

impl<'de> Deserialize<'de> for KurtApricornRecipeTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let recipes = KurtApricornRecipes::deserialize(deserializer)?;
        if recipes.is_empty() {
            return Err(serde::de::Error::custom(
                "kurt apricorn recipes must not be empty",
            ));
        }
        for (apricorn, ball) in &recipes {
            require_special_token("Kurt apricorn recipe apricorn id", apricorn)
                .map_err(serde::de::Error::custom)?;
            require_special_token("Kurt apricorn recipe ball id", ball)
                .map_err(serde::de::Error::custom)?;
        }
        Ok(Self(recipes))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KurtApricornRecipeIssue {
    EmptyApricorn { apricorn: String },
    InvalidApricorn { apricorn: String },
    UnknownApricorn { apricorn: String },
    EmptyBall { apricorn: String },
    InvalidBall { apricorn: String, ball: String },
    UnknownBall { apricorn: String, ball: String },
}

pub fn kurt_apricorn_recipe_issues(
    recipes: &KurtApricornRecipes,
    item_ids: &BTreeSet<String>,
) -> Vec<KurtApricornRecipeIssue> {
    let mut issues = Vec::new();
    for (apricorn, ball) in recipes {
        if apricorn.is_empty() {
            issues.push(KurtApricornRecipeIssue::EmptyApricorn {
                apricorn: apricorn.clone(),
            });
        } else if !is_exact_nonempty_special_token(apricorn) {
            issues.push(KurtApricornRecipeIssue::InvalidApricorn {
                apricorn: apricorn.clone(),
            });
        } else if !item_ids.contains(apricorn) {
            issues.push(KurtApricornRecipeIssue::UnknownApricorn {
                apricorn: apricorn.clone(),
            });
        }
        if ball.is_empty() {
            issues.push(KurtApricornRecipeIssue::EmptyBall {
                apricorn: apricorn.clone(),
            });
        } else if !is_exact_nonempty_special_token(ball) {
            issues.push(KurtApricornRecipeIssue::InvalidBall {
                apricorn: apricorn.clone(),
                ball: ball.clone(),
            });
        } else if !item_ids.contains(ball) {
            issues.push(KurtApricornRecipeIssue::UnknownBall {
                apricorn: apricorn.clone(),
                ball: ball.clone(),
            });
        }
    }
    issues
}

pub type BuenaPrizeDefinitions = BTreeMap<String, u8>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct BuenaPrizeDefinitionTable(pub BuenaPrizeDefinitions);

impl<'de> Deserialize<'de> for BuenaPrizeDefinitionTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let prizes = BuenaPrizeDefinitions::deserialize(deserializer)?;
        if prizes.is_empty() {
            return Err(serde::de::Error::custom(
                "buena prize definitions must not be empty",
            ));
        }
        for (item_id, cost) in &prizes {
            require_special_token("Buena prize item id", item_id)
                .map_err(serde::de::Error::custom)?;
            if *cost == 0 {
                return Err(serde::de::Error::custom(format!(
                    "Buena prize item '{item_id}' cost must be nonzero"
                )));
            }
        }
        Ok(Self(prizes))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuenaPrizeDefinitionIssue {
    EmptyItem { item_id: String },
    InvalidItem { item_id: String },
    UnknownItem { item_id: String },
    InvalidCost { item_id: String },
}

pub fn buena_prize_definition_issues(
    prizes: &BuenaPrizeDefinitions,
    item_ids: &BTreeSet<String>,
) -> Vec<BuenaPrizeDefinitionIssue> {
    let mut issues = Vec::new();
    for (item_id, cost) in prizes {
        if item_id.is_empty() {
            issues.push(BuenaPrizeDefinitionIssue::EmptyItem {
                item_id: item_id.clone(),
            });
        } else if !is_exact_nonempty_special_token(item_id) {
            issues.push(BuenaPrizeDefinitionIssue::InvalidItem {
                item_id: item_id.clone(),
            });
        } else if !item_ids.contains(item_id) {
            issues.push(BuenaPrizeDefinitionIssue::UnknownItem {
                item_id: item_id.clone(),
            });
        }
        if *cost == 0 {
            issues.push(BuenaPrizeDefinitionIssue::InvalidCost {
                item_id: item_id.clone(),
            });
        }
    }
    issues
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoamingPokemonDefinition {
    pub level: u8,
    pub map_group: u16,
    pub map_number: u16,
}

impl<'de> Deserialize<'de> for RoamingPokemonDefinition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct RawDefinition {
            level: u8,
            map_group: u16,
            map_number: u16,
        }

        let raw = RawDefinition::deserialize(deserializer)?;
        if raw.level == 0 || raw.level > 100 {
            return Err(serde::de::Error::custom(format!(
                "roaming Pokemon level must be 1..100, found {}",
                raw.level
            )));
        }
        if raw.map_group == 0 || raw.map_number == 0 {
            return Err(serde::de::Error::custom(
                "roaming Pokemon map group and number must be nonzero",
            ));
        }
        Ok(Self {
            level: raw.level,
            map_group: raw.map_group,
            map_number: raw.map_number,
        })
    }
}
pub type RoamingPokemonDefinitions = BTreeMap<String, RoamingPokemonDefinition>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoamingPokemonDefinitionIssue {
    EmptySpecies { species: String },
    InvalidSpecies { species: String },
    UnknownSpecies { species: String },
    InvalidLevel { species: String },
}

pub fn roaming_pokemon_definition_issues(
    definitions: &RoamingPokemonDefinitions,
    species_ids: &BTreeSet<String>,
) -> Vec<RoamingPokemonDefinitionIssue> {
    let mut issues = Vec::new();
    for (species, definition) in definitions {
        if species.trim().is_empty() {
            issues.push(RoamingPokemonDefinitionIssue::EmptySpecies {
                species: species.clone(),
            });
        } else if !is_exact_nonempty_special_token(species) {
            issues.push(RoamingPokemonDefinitionIssue::InvalidSpecies {
                species: species.clone(),
            });
        } else if !species_ids.contains(species) {
            issues.push(RoamingPokemonDefinitionIssue::UnknownSpecies {
                species: species.clone(),
            });
        }
        if definition.level == 0 {
            issues.push(RoamingPokemonDefinitionIssue::InvalidLevel {
                species: species.clone(),
            });
        }
    }
    issues
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSpawnPointCatalogIssue {
    IdentifierMismatch {
        key: String,
        identifier: u16,
    },
    MapMismatch {
        key: String,
        map_name: String,
        metadata_name: String,
    },
    UnknownMap {
        key: String,
        map_constant: String,
    },
    InvalidSpawnPoint {
        key: String,
    },
    CoordinateMismatch {
        key: String,
        tile_x: i16,
        tile_y: i16,
        expected_tile_x: i16,
        expected_tile_y: i16,
    },
    CoordinateOverflow {
        key: String,
        metatile_x: i16,
        metatile_y: i16,
        subtile_x: i16,
        subtile_y: i16,
    },
    InvalidSubtile {
        key: String,
        subtile_x: i16,
        subtile_y: i16,
        metatile_width: i16,
    },
    DuplicateMapBinding {
        key: String,
        existing_key: String,
        group_id: i16,
        map_id: i16,
    },
}

pub fn runtime_spawn_point_catalog_issues(
    spawn_points: &BTreeMap<String, RuntimeSpawnPointRef>,
    runtime_map_names: &BTreeMap<String, String>,
) -> Vec<RuntimeSpawnPointCatalogIssue> {
    let mut issues = Vec::new();
    let mut map_bindings = BTreeMap::new();

    for (key, spawn) in spawn_points {
        let invalid_spawn_point = !is_exact_nonempty_spawn_token(key)
            || !is_exact_nonempty_spawn_token(&spawn.map_constant)
            || !is_exact_nonempty_spawn_token(&spawn.map_name)
            || !is_exact_nonempty_spawn_token(&spawn.group_name);
        if invalid_spawn_point {
            issues.push(RuntimeSpawnPointCatalogIssue::InvalidSpawnPoint { key: key.clone() });
        }
        if !runtime_spawn_subtiles_are_valid(spawn) {
            issues.push(RuntimeSpawnPointCatalogIssue::InvalidSubtile {
                key: key.clone(),
                subtile_x: spawn.subtile_x,
                subtile_y: spawn.subtile_y,
                metatile_width: METATILE_WIDTH,
            });
        } else {
            match checked_runtime_spawn_expected_tile(spawn) {
                Some(expected_tile) => {
                    if spawn.tile_x != expected_tile.x || spawn.tile_y != expected_tile.y {
                        issues.push(RuntimeSpawnPointCatalogIssue::CoordinateMismatch {
                            key: key.clone(),
                            tile_x: spawn.tile_x,
                            tile_y: spawn.tile_y,
                            expected_tile_x: expected_tile.x,
                            expected_tile_y: expected_tile.y,
                        });
                    }
                }
                None => issues.push(RuntimeSpawnPointCatalogIssue::CoordinateOverflow {
                    key: key.clone(),
                    metatile_x: spawn.metatile_x,
                    metatile_y: spawn.metatile_y,
                    subtile_x: spawn.subtile_x,
                    subtile_y: spawn.subtile_y,
                }),
            }
        }
        if key.parse::<u16>().ok() != Some(spawn.identifier) {
            issues.push(RuntimeSpawnPointCatalogIssue::IdentifierMismatch {
                key: key.clone(),
                identifier: spawn.identifier,
            });
        }
        if is_exact_nonempty_spawn_token(&spawn.map_constant) && spawn.map_constant != "N_A" {
            match runtime_map_names.get(&spawn.map_constant) {
                Some(metadata_name) if metadata_name == &spawn.map_name => {}
                Some(metadata_name) => issues.push(RuntimeSpawnPointCatalogIssue::MapMismatch {
                    key: key.clone(),
                    map_name: spawn.map_name.clone(),
                    metadata_name: metadata_name.clone(),
                }),
                None => issues.push(RuntimeSpawnPointCatalogIssue::UnknownMap {
                    key: key.clone(),
                    map_constant: spawn.map_constant.clone(),
                }),
            }
        }
        if !invalid_spawn_point {
            if let Some(existing_key) =
                map_bindings.insert((spawn.group_id, spawn.map_id), key.clone())
            {
                issues.push(RuntimeSpawnPointCatalogIssue::DuplicateMapBinding {
                    key: key.clone(),
                    existing_key,
                    group_id: spawn.group_id,
                    map_id: spawn.map_id,
                });
            }
        }
    }

    issues
}

pub fn runtime_spawn_expected_tile(spawn: &RuntimeSpawnPointRef) -> TilePosition {
    checked_runtime_spawn_expected_tile(spawn)
        .expect("verified runtime spawn point coordinate must fit runtime tile arithmetic")
}

pub fn runtime_spawn_point_from_runtime_tile(
    identifier: u16,
    map_constant: String,
    map_name: String,
    group_id: i16,
    map_id: i16,
    group_name: String,
    tile: TilePosition,
) -> Option<RuntimeSpawnPointRef> {
    if tile.x < 0 || tile.y < 0 {
        return None;
    }
    let spawn = RuntimeSpawnPointRef {
        identifier,
        map_constant,
        map_name,
        group_id,
        map_id,
        tile_x: tile.x,
        tile_y: tile.y,
        group_name,
        metatile_x: tile.x.div_euclid(METATILE_WIDTH),
        metatile_y: tile.y.div_euclid(METATILE_WIDTH),
        subtile_x: tile.x.rem_euclid(METATILE_WIDTH),
        subtile_y: tile.y.rem_euclid(METATILE_WIDTH),
    };
    (checked_runtime_spawn_expected_tile(&spawn) == Some(tile)).then_some(spawn)
}

pub fn checked_runtime_spawn_expected_tile(spawn: &RuntimeSpawnPointRef) -> Option<TilePosition> {
    if !runtime_spawn_subtiles_are_valid(spawn) {
        return None;
    }
    let x = i32::from(spawn.metatile_x)
        .checked_mul(i32::from(METATILE_WIDTH))?
        .checked_add(i32::from(spawn.subtile_x))?;
    let y = i32::from(spawn.metatile_y)
        .checked_mul(i32::from(METATILE_WIDTH))?
        .checked_add(i32::from(spawn.subtile_y))?;
    Some(TilePosition::new(
        i16::try_from(x).ok()?,
        i16::try_from(y).ok()?,
    ))
}

pub fn runtime_spawn_subtiles_are_valid(spawn: &RuntimeSpawnPointRef) -> bool {
    (0..METATILE_WIDTH).contains(&spawn.subtile_x) && (0..METATILE_WIDTH).contains(&spawn.subtile_y)
}

fn is_exact_nonempty_spawn_token(value: &str) -> bool {
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
    "UnusedCheckUnusedTwoDayTimer",
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
    "AerodactylChamber",
    "KabutoChamber",
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
    "UnusedMemoryGame",
    "MemoryGame",
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
    "StartBugContestTimer",
    "CheckBugContestTimer",
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
    "CheckLinkTimeoutReceptionist",
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
    "RandomUnseenWildMon",
    "RandomPhoneWildMon",
    "RandomPhoneMon",
    "UnusedDummySpecial",
    "UnusedBattleTowerDummySpecial1",
    "UnusedBattleTowerDummySpecial2",
    "UnusedFindItemInPCOrBag",
    "Function11ba38",
];

pub const INACTIVE_DECLARED_SPECIAL_ROUTINES: &[&str] = &[
    "Function11ac3e",
    "TradeCornerHoldMon",
    "Function11b5e8",
    "Function11b7e5",
    "Function11b879",
    "Function11b920",
    "Function11b93b",
    "Function170114",
    "Function1704e1",
    "Function11c1ab",
    "Function17d2b6",
    "Function17d2ce",
    "Function102142",
];

pub fn is_known_special_routine(routine: &str) -> bool {
    EXECUTABLE_SPECIAL_ROUTINES.contains(&routine)
        || INACTIVE_DECLARED_SPECIAL_ROUTINES.contains(&routine)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecialRoutineCatalogIssue {
    EmptyRoutine { routine: String },
    InvalidRoutine { routine: String },
    UnknownRoutine { routine: String },
}

pub fn special_routine_catalog_issues(
    routines: &BTreeSet<String>,
) -> Vec<SpecialRoutineCatalogIssue> {
    let mut issues = Vec::new();
    for routine in routines {
        if routine.trim().is_empty() {
            issues.push(SpecialRoutineCatalogIssue::EmptyRoutine {
                routine: routine.clone(),
            });
            continue;
        } else if !is_exact_nonempty_special_token(routine) {
            issues.push(SpecialRoutineCatalogIssue::InvalidRoutine {
                routine: routine.clone(),
            });
            continue;
        }
        if !is_known_special_routine(routine) {
            issues.push(SpecialRoutineCatalogIssue::UnknownRoutine {
                routine: routine.clone(),
            });
        }
    }
    issues
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
    let empty_roaming_pokemon = BTreeMap::new();
    let empty_buena_password_categories = BuenaPasswordCategories::default();
    let empty_buena_prizes = BTreeMap::new();
    let empty_kurt_apricorn_recipes = BTreeMap::new();
    let empty_dratini_move_sets = BTreeMap::new();
    let empty_phone_contacts = PhoneContactCatalog::default();
    let empty_wild_encounters = BTreeMap::new();
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
            roaming_pokemon: &empty_roaming_pokemon,
            buena_password_categories: &empty_buena_password_categories,
            buena_prizes: &empty_buena_prizes,
            kurt_apricorn_recipes: &empty_kurt_apricorn_recipes,
            shuckie_gift: None,
            dratini_move_sets: &empty_dratini_move_sets,
            bug_contest_config: None,
            battle_tower_rules: None,
            magikarp_lengths: &[],
            happiness_data: None,
            trainer_catalog: &empty_trainers,
            phone_contacts: &empty_phone_contacts,
            wild_encounters: &empty_wild_encounters,
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
        "UnusedCheckUnusedTwoDayTimer" => unused_check_unused_two_day_timer(state, routine),
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
        "ToggleDecorationsVisibility" => toggle_decorations_visibility(state, routine),
        "MagnetTrain" => visual_command(state, routine, ScriptGraphicsRuntimeKind::MagnetTrain),
        "Diploma" => visual_command(state, routine, ScriptGraphicsRuntimeKind::Diploma),
        "PrintDiploma" => visual_command(state, routine, ScriptGraphicsRuntimeKind::PrintDiploma),
        "UnownPuzzle" => unown_puzzle(state, routine),
        "OmanyteChamber" => unown_chamber(state, context.item_catalog, routine, "OMANYTE"),
        "AerodactylChamber" => unown_chamber(state, context.item_catalog, routine, "AERODACTYL"),
        "KabutoChamber" => unown_chamber(state, context.item_catalog, routine, "KABUTO"),
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
        "SlotMachine" => slot_machine(state, context.item_catalog, routine),
        "CardFlip" => card_flip(state, context.item_catalog, routine),
        "UnusedMemoryGame" | "MemoryGame" => {
            unused_memory_game(state, context.item_catalog, routine)
        }
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
        "StartBugContestTimer" => start_bug_contest_timer(state, routine),
        "CheckBugContestTimer" => check_bug_contest_timer(state, routine),
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
        "CheckLinkTimeout_Receptionist" | "CheckLinkTimeoutReceptionist" => {
            check_link_timeout_receptionist(state, routine)
        }
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
        "UnusedDummySpecial"
        | "UnusedBattleTowerDummySpecial1"
        | "UnusedBattleTowerDummySpecial2" => noop_special(routine),
        "UnusedFindItemInPCOrBag" => {
            unused_find_item_in_pc_or_bag(state, context.item_catalog, routine)
        }
        "RandomUnseenWildMon" => random_unseen_wild_mon(
            state,
            context.species_catalog,
            context.phone_contacts,
            context.wild_encounters,
            routine,
        ),
        "RandomPhoneWildMon" => random_phone_wild_mon(
            state,
            context.species_catalog,
            context.phone_contacts,
            context.wild_encounters,
            routine,
        ),
        "RandomPhoneMon" => random_phone_mon(
            state,
            context.species_catalog,
            context.phone_contacts,
            context.trainer_catalog,
            routine,
        ),
        "Function11ba38" => function11ba38(state, routine),
        "Function11ac3e" | "TradeCornerHoldMon" | "Function11b5e8" | "Function11b7e5"
        | "Function11b879" | "Function11b920" | "Function11b93b" | "Function170114"
        | "Function1704e1" | "Function11c1ab" | "Function17d2b6" | "Function17d2ce"
        | "Function102142" => inactive_declared_routine(routine),
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

fn noop_special(routine: &str) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::Noop,
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
    let tile = runtime_spawn_expected_tile(spawn);
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
        command: "warp".to_string(),
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
        .insert("wXCoord".to_string(), tile.x.to_string());
    state
        .script_runtime
        .variables
        .insert("wYCoord".to_string(), tile.y.to_string());
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
    let group = optional_i16_script_variable(state, routine, "wLastSpawnMapGroup")?;
    let map_id = optional_i16_script_variable(state, routine, "wLastSpawnMapNumber")?;
    match (group, map_id) {
        (Some(group_id), Some(map_id)) => spawn_points
            .values()
            .find(|spawn| spawn.group_id == group_id && spawn.map_id == map_id)
            .ok_or_else(|| SpecialRoutineError::UnknownSpawnMap {
                routine: routine.to_string(),
                group_id,
                map_id,
            }),
        (Some(_), None) => Err(SpecialRoutineError::MissingScriptValue {
            routine: routine.to_string(),
            variable: "wLastSpawnMapNumber".to_string(),
        }),
        (None, Some(_)) => Err(SpecialRoutineError::MissingScriptValue {
            routine: routine.to_string(),
            variable: "wLastSpawnMapGroup".to_string(),
        }),
        (None, None) => {
            if let Some(spawn_identifier) = state.last_spawn_identifier {
                return spawn_points
                    .get(&spawn_identifier.to_string())
                    .ok_or_else(|| SpecialRoutineError::UnknownSpawnPoint {
                        routine: routine.to_string(),
                        spawn_identifier,
                    });
            }
            Err(SpecialRoutineError::MissingScriptValue {
                routine: routine.to_string(),
                variable: "wLastSpawnMapGroup".to_string(),
            })
        }
    }
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
    let is_egg = pokemon_is_egg(pokemon);
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

fn random_unseen_wild_mon(
    state: &mut GameState,
    species_catalog: &BTreeMap<String, PokemonSpecies>,
    phone_contacts: &PhoneContactCatalog,
    wild_encounters: &BTreeMap<String, WildEncounterData>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let (contact_id, map_name, encounters) =
        caller_grass_encounters(state, phone_contacts, wild_encounters, routine)?;
    let grass = encounters.grass.as_ref().ok_or_else(|| {
        SpecialRoutineError::MissingCallerGrassEncounter {
            routine: routine.to_string(),
            map_name: map_name.clone(),
        }
    })?;
    let morning = &grass.morning;
    if morning.len() < 7 {
        return Err(SpecialRoutineError::TooFewCallerGrassSlots {
            routine: routine.to_string(),
            map_name,
            expected: 7,
            found: morning.len(),
        });
    }

    let mut rng = Random::new(state.rng_seed);
    let rare_index = loop {
        let masked = rng.randrange(256) & 0b11;
        if masked != 0 {
            break 4 + (masked as usize - 1);
        }
    };
    state.rng_seed = rng.seed();

    let selected = &morning[rare_index];
    let common = &morning[..4];
    let is_common = common
        .iter()
        .any(|encounter| encounter.species == selected.species);
    let already_seen = state.pokedex.has_seen(&selected.species);
    let script_value = if is_common || already_seen { 1 } else { 0 };
    if script_value == 0 {
        write_phone_species_buffers(state, species_catalog, routine, selected)?;
    }
    state.script_runtime.script_value = Some(script_value.to_string());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::RandomUnseenWildMon {
            contact_id,
            map_name: encounters.map_name.clone(),
            species: (script_value == 0).then(|| selected.species.clone()),
            already_seen,
            script_value,
            rng_seed_after: state.rng_seed,
        },
    })
}

fn random_phone_wild_mon(
    state: &mut GameState,
    species_catalog: &BTreeMap<String, PokemonSpecies>,
    phone_contacts: &PhoneContactCatalog,
    wild_encounters: &BTreeMap<String, WildEncounterData>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let (contact_id, map_name, encounters) =
        caller_grass_encounters(state, phone_contacts, wild_encounters, routine)?;
    let time_of_day = state.time.time_of_day;
    let grass = encounters.grass.as_ref().ok_or_else(|| {
        SpecialRoutineError::MissingCallerGrassEncounter {
            routine: routine.to_string(),
            map_name: map_name.clone(),
        }
    })?;
    let slots = grass.slots(time_of_day);
    if slots.len() < 4 {
        return Err(SpecialRoutineError::TooFewCallerGrassSlots {
            routine: routine.to_string(),
            map_name,
            expected: 4,
            found: slots.len(),
        });
    }
    let mut rng = Random::new(state.rng_seed);
    let selected = &slots[(rng.randrange(256) & 0b11) as usize];
    state.rng_seed = rng.seed();
    write_phone_species_buffers(state, species_catalog, routine, selected)?;
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::RandomPhoneWildMon {
            contact_id,
            map_name: encounters.map_name.clone(),
            time_of_day,
            species: selected.species.clone(),
            rng_seed_after: state.rng_seed,
        },
    })
}

fn random_phone_mon(
    state: &mut GameState,
    species_catalog: &BTreeMap<String, PokemonSpecies>,
    phone_contacts: &PhoneContactCatalog,
    trainer_catalog: &TrainerCatalog,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let (contact_id, contact) = caller_phone_contact(state, phone_contacts, routine)?;
    let trainer_id = contact.trainer_label.clone().ok_or_else(|| {
        SpecialRoutineError::MissingPhoneContactTrainer {
            routine: routine.to_string(),
            contact_id: contact_id.clone(),
        }
    })?;
    let trainer =
        trainer_catalog
            .get(&trainer_id)
            .ok_or_else(|| SpecialRoutineError::UnknownTrainer {
                routine: routine.to_string(),
                trainer_id: trainer_id.clone(),
            })?;
    if trainer.party.is_empty() {
        return Err(SpecialRoutineError::EmptyTrainerParty {
            routine: routine.to_string(),
            trainer_id,
        });
    }

    let mut rng = Random::new(state.rng_seed);
    let party_index = loop {
        let masked = (rng.randrange(256) & 0b111) as usize;
        if masked < trainer.party.len() {
            break masked;
        }
    };
    state.rng_seed = rng.seed();

    let selected = &trainer.party[party_index];
    write_phone_species_id_buffers(state, species_catalog, routine, &selected.species)?;
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::RandomPhoneMon {
            contact_id,
            trainer_id: trainer.trainer_id.clone(),
            species: selected.species.clone(),
            party_index,
            rng_seed_after: state.rng_seed,
        },
    })
}

fn caller_grass_encounters<'a>(
    state: &GameState,
    phone_contacts: &'a PhoneContactCatalog,
    wild_encounters: &'a BTreeMap<String, WildEncounterData>,
    routine: &str,
) -> Result<(String, String, &'a WildEncounterData), SpecialRoutineError> {
    let (contact_id, contact) = caller_phone_contact(state, phone_contacts, routine)?;
    let map_name = contact.map_constant.clone().ok_or_else(|| {
        SpecialRoutineError::MissingPhoneContactMap {
            routine: routine.to_string(),
            contact_id: contact_id.clone(),
        }
    })?;
    let encounters = wild_encounters.get(&map_name).ok_or_else(|| {
        SpecialRoutineError::MissingCallerWildEncounter {
            routine: routine.to_string(),
            map_name: map_name.clone(),
        }
    })?;
    Ok((contact_id, map_name, encounters))
}

fn caller_phone_contact<'a>(
    state: &GameState,
    phone_contacts: &'a PhoneContactCatalog,
    routine: &str,
) -> Result<(String, &'a crate::systems::phone::PhoneContactRecord), SpecialRoutineError> {
    let contact_id = state
        .script_runtime
        .variables
        .get("VAR_CALLERID")
        .cloned()
        .ok_or_else(|| SpecialRoutineError::MissingCallerId {
            routine: routine.to_string(),
        })?;
    let contact = phone_contacts.0.get(&contact_id).ok_or_else(|| {
        SpecialRoutineError::UnknownPhoneContact {
            routine: routine.to_string(),
            contact_id: contact_id.clone(),
        }
    })?;
    Ok((contact_id, contact))
}

fn write_phone_species_buffers(
    state: &mut GameState,
    species_catalog: &BTreeMap<String, PokemonSpecies>,
    routine: &str,
    encounter: &WildEncounter,
) -> Result<(), SpecialRoutineError> {
    let species = required_species_metadata(species_catalog, routine, &encounter.species)?;
    state
        .script_runtime
        .variables
        .insert("wNamedObjectIndex".to_string(), species.int_id.to_string());
    state
        .script_runtime
        .named_buffers
        .insert("STRING_BUFFER_1".to_string(), encounter.species.clone());
    state
        .script_runtime
        .named_buffers
        .insert("STRING_BUFFER_4".to_string(), encounter.species.clone());
    Ok(())
}

fn write_phone_species_id_buffers(
    state: &mut GameState,
    species_catalog: &BTreeMap<String, PokemonSpecies>,
    routine: &str,
    species_id: &str,
) -> Result<(), SpecialRoutineError> {
    let species = required_species_metadata(species_catalog, routine, species_id)?;
    state
        .script_runtime
        .variables
        .insert("wNamedObjectIndex".to_string(), species.int_id.to_string());
    state
        .script_runtime
        .named_buffers
        .insert("STRING_BUFFER_1".to_string(), species_id.to_string());
    state
        .script_runtime
        .named_buffers
        .insert("STRING_BUFFER_4".to_string(), species_id.to_string());
    Ok(())
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
    const PROXIMITY_COORDS: &[(i32, i32)] = &[
        (33 * METATILE_WIDTH as i32, 8 * METATILE_WIDTH as i32),
        (34 * METATILE_WIDTH as i32, 10 * METATILE_WIDTH as i32),
        (35 * METATILE_WIDTH as i32, 10 * METATILE_WIDTH as i32),
        (36 * METATILE_WIDTH as i32, 8 * METATILE_WIDTH as i32),
        (36 * METATILE_WIDTH as i32, 9 * METATILE_WIDTH as i32),
    ];
    let x = i32::from(x);
    let y = i32::from(y);
    PROXIMITY_COORDS.iter().any(|&(px, py)| px == x && py == y)
}

fn set_day_of_week(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    // The ASM menu writes the selected weekday to wTempDayOfWeek before
    // calling InitDayOfWeek.  Headless callers may provide that same value in
    // the runtime variable bank; otherwise preserve the current weekday
    // rather than silently forcing Sunday.
    let selected_day = state
        .script_runtime
        .variables
        .get("wTempDayOfWeek")
        .and_then(|value| value.parse::<u8>().ok())
        .filter(|day| *day < 7)
        .unwrap_or_else(|| state.time.day_of_week.min(6));
    state.time.day_of_week = selected_day;
    state.time.current_day = selected_day;
    state.script_runtime.script_value = Some("1".to_string());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), "1".to_string());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::SetDayOfWeek { day: selected_day },
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

fn unused_check_unused_two_day_timer(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    const TIMER_DAYS: u8 = 2;
    let start_day = state.unused_two_day_timer.start_day;
    let current_day = state.time.current_day;
    let elapsed_days = current_day.wrapping_sub(start_day);
    let remaining_days = TIMER_DAYS.saturating_sub(elapsed_days);
    state.unused_two_day_timer.remaining_days = remaining_days;
    state.script_runtime.script_value = Some(remaining_days.to_string());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), remaining_days.to_string());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::UnusedCheckUnusedTwoDayTimer {
            start_day,
            current_day,
            elapsed_days,
            remaining_days,
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
    if pokemon.species.id.is_empty() || pokemon_is_egg(pokemon) {
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

fn unown_chamber(
    state: &mut GameState,
    item_catalog: &BTreeMap<String, Item>,
    routine: &str,
    chamber: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    const OMANYTE_FLAG: &str = "EVENT_WALL_OPENED_IN_OMANYTE_CHAMBER";
    const AERODACTYL_FLAG: &str = "EVENT_WALL_OPENED_IN_AERODACTYL_CHAMBER";
    const KABUTO_FLAG: &str = "EVENT_WALL_OPENED_IN_KABUTO_CHAMBER";
    const AERODACTYL_MAP: &str = "RuinsOfAlphAerodactylChamber";
    const KABUTO_MAP: &str = "RuinsOfAlphKabutoChamber";
    const WATER_STONE: &str = "WATER_STONE";

    let (flag, open) = match chamber {
        "OMANYTE" => {
            let flag_open = read_event_flag(state, routine, OMANYTE_FLAG)?;
            if flag_open {
                (OMANYTE_FLAG, true)
            } else {
                let water_stone = item_catalog.get(WATER_STONE).ok_or_else(|| {
                    SpecialRoutineError::UnknownItem {
                        routine: routine.to_string(),
                        item_id: WATER_STONE.to_string(),
                    }
                })?;
                let held_water_stone = state
                    .storage
                    .party
                    .pokemon
                    .iter()
                    .flatten()
                    .any(|pokemon| pokemon.item.as_deref() == Some(WATER_STONE));
                (
                    OMANYTE_FLAG,
                    state.bag.has_item(water_stone) || held_water_stone,
                )
            }
        }
        "AERODACTYL" => (
            AERODACTYL_FLAG,
            matches!(
                &state.overworld,
                crate::state::OverworldMemory::Active { map_name, .. }
                    if map_name == AERODACTYL_MAP
            ),
        ),
        "KABUTO" => (
            KABUTO_FLAG,
            matches!(
                &state.overworld,
                crate::state::OverworldMemory::Active { map_name, .. }
                    if map_name == KABUTO_MAP
            ),
        ),
        _ => {
            return Err(SpecialRoutineError::InvalidNumericValue {
                routine: routine.to_string(),
                value: chamber.to_string(),
            });
        }
    };
    if open {
        state
            .flags
            .set_event_flag(flag, true)
            .map_err(|error| SpecialRoutineError::EventFlag {
                routine: routine.to_string(),
                error,
            })?;
    }
    set_script_bool_value(state, open);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::UnownChamber {
            chamber: chamber.to_string(),
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
    let rival_name = required_string_script_variable(state, routine, "_rival_name")?;
    if rival_name.is_empty() || rival_name.chars().all(|value| value == ' ') {
        return Err(SpecialRoutineError::MissingScriptValue {
            routine: routine.to_string(),
            variable: "_rival_name".to_string(),
        });
    }
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
        if pokemon_is_egg(pokemon) {
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
    if species == "EGG"
        || state
            .storage
            .party
            .pokemon
            .get(party_slot)
            .and_then(|pokemon| pokemon.as_ref())
            .is_some_and(pokemon_is_egg)
    {
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

const UNOWN_PUZZLE_IDS: [&str; 4] = ["KABUTO", "OMANYTE", "AERODACTYL", "HOOH"];
const UNOWN_TARGET_LAYOUT: [[u8; 6]; 6] = [
    [0, 0, 0, 0, 0, 0],
    [0, 1, 2, 3, 4, 0],
    [0, 5, 6, 7, 8, 0],
    [0, 9, 10, 11, 12, 0],
    [0, 13, 14, 15, 16, 0],
    [0, 0, 0, 0, 0, 0],
];
const UNOWN_START_POSITIONS: [(usize, usize); 16] = [
    (0, 0),
    (1, 0),
    (2, 0),
    (3, 0),
    (4, 0),
    (5, 0),
    (0, 1),
    (5, 1),
    (0, 2),
    (5, 2),
    (0, 3),
    (5, 3),
    (0, 4),
    (5, 4),
    (0, 5),
    (5, 5),
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct UnownPuzzleState {
    layout: [[u8; 6]; 6],
    holding_piece: Option<u8>,
    moves: u16,
}

fn unown_puzzle_layout_vec(layout: &[[u8; 6]; 6]) -> Vec<Vec<u8>> {
    layout.iter().map(|row| row.to_vec()).collect()
}

fn normalize_unown_puzzle_id(
    state: &GameState,
    routine: &str,
) -> Result<String, SpecialRoutineError> {
    let raw_value = required_raw_script_value(state, routine)?;
    if let Ok(index) = parse_exact_usize_token(routine, &raw_value, &raw_value) {
        if let Some(puzzle_id) = UNOWN_PUZZLE_IDS.get(index) {
            return Ok((*puzzle_id).to_string());
        }
    }
    let token = raw_value.trim().to_ascii_uppercase();
    let resolved = match token.as_str() {
        "UNOWNPUZZLE_KABUTO" => "KABUTO",
        "UNOWNPUZZLE_OMANYTE" => "OMANYTE",
        "UNOWNPUZZLE_AERODACTYL" => "AERODACTYL",
        "UNOWNPUZZLE_HO_OH" => "HOOH",
        other => other,
    };
    if UNOWN_PUZZLE_IDS.contains(&resolved) {
        Ok(resolved.to_string())
    } else {
        Err(SpecialRoutineError::InvalidUnownPuzzleState {
            routine: routine.to_string(),
            message: format!("unknown puzzle id '{raw_value}'"),
        })
    }
}

fn unown_puzzle_variable_key(base_name: &str, puzzle_id: &str) -> String {
    format!("{base_name}_{puzzle_id}")
}

fn migrate_unown_puzzle_variable(
    state: &mut GameState,
    base_name: &str,
    puzzle_id: &str,
) -> Option<String> {
    let full_key = unown_puzzle_variable_key(base_name, puzzle_id);
    if let Some(value) = state.script_runtime.variables.get(&full_key).cloned() {
        return Some(value);
    }
    let value = state.script_runtime.variables.remove(base_name)?;
    state
        .script_runtime
        .variables
        .insert(full_key, value.clone());
    Some(value)
}

fn encode_unown_layout(layout: &[[u8; 6]; 6]) -> String {
    layout
        .iter()
        .map(|row| row.iter().map(u8::to_string).collect::<Vec<_>>().join(","))
        .collect::<Vec<_>>()
        .join(";")
}

fn parse_unown_layout(routine: &str, raw: &str) -> Result<[[u8; 6]; 6], SpecialRoutineError> {
    let mut layout = [[0_u8; 6]; 6];
    let rows = raw.split(';').collect::<Vec<_>>();
    if rows.len() != 6 {
        return Err(SpecialRoutineError::InvalidUnownPuzzleState {
            routine: routine.to_string(),
            message: "layout must contain six rows".to_string(),
        });
    }
    for (y, row) in rows.iter().enumerate() {
        let cells = row.split(',').collect::<Vec<_>>();
        if cells.len() != 6 {
            return Err(SpecialRoutineError::InvalidUnownPuzzleState {
                routine: routine.to_string(),
                message: "layout rows must contain six columns".to_string(),
            });
        }
        for (x, cell) in cells.iter().enumerate() {
            let value = parse_exact_u8_token(routine, cell, raw)?;
            if value > 16 {
                return Err(SpecialRoutineError::InvalidUnownPuzzleState {
                    routine: routine.to_string(),
                    message: "layout entries must be 0 or between 1 and 16".to_string(),
                });
            }
            layout[y][x] = value;
        }
    }
    Ok(layout)
}

fn validate_unown_puzzle_state(
    routine: &str,
    layout: &[[u8; 6]; 6],
    holding_piece: Option<u8>,
) -> Result<(), SpecialRoutineError> {
    let mut seen = [false; 17];
    for row in layout {
        for value in row {
            if *value == 0 {
                continue;
            }
            if seen[usize::from(*value)] {
                return Err(SpecialRoutineError::InvalidUnownPuzzleState {
                    routine: routine.to_string(),
                    message: format!("piece {value} appears more than once in the puzzle state"),
                });
            }
            seen[usize::from(*value)] = true;
        }
    }
    if let Some(piece) = holding_piece {
        if !(1..=16).contains(&piece) {
            return Err(SpecialRoutineError::InvalidUnownPuzzleState {
                routine: routine.to_string(),
                message: "holding_piece must be between 1 and 16".to_string(),
            });
        }
        if seen[usize::from(piece)] {
            return Err(SpecialRoutineError::InvalidUnownPuzzleState {
                routine: routine.to_string(),
                message: format!("held piece {piece} also appears in the puzzle layout"),
            });
        }
    }
    Ok(())
}

fn shuffled_unown_puzzle(rng: &mut Random) -> UnownPuzzleState {
    let mut layout = [[0_u8; 6]; 6];
    for piece_id in 1..=16 {
        loop {
            let slot_index = rng.randrange(256) as usize & 0x0f;
            let (x, y) = UNOWN_START_POSITIONS[slot_index];
            if layout[y][x] == 0 {
                layout[y][x] = piece_id;
                break;
            }
        }
    }
    UnownPuzzleState {
        layout,
        holding_piece: None,
        moves: 0,
    }
}

fn load_unown_puzzle_state(
    state: &mut GameState,
    routine: &str,
    puzzle_id: &str,
) -> Result<UnownPuzzleState, SpecialRoutineError> {
    let Some(raw_layout) = migrate_unown_puzzle_variable(state, "unown_layout", puzzle_id) else {
        let mut rng = Random::new(state.rng_seed);
        let puzzle = shuffled_unown_puzzle(&mut rng);
        state.rng_seed = rng.seed();
        return Ok(puzzle);
    };
    let layout = parse_unown_layout(routine, &raw_layout)?;
    let holding_piece = migrate_unown_puzzle_variable(state, "unown_holding_piece", puzzle_id)
        .and_then(|raw| {
            let trimmed = raw.trim();
            (!trimmed.is_empty() && trimmed != "null").then(|| trimmed.to_string())
        })
        .map(|raw| parse_exact_u8_token(routine, &raw, &raw))
        .transpose()?;
    let moves = migrate_unown_puzzle_variable(state, "unown_moves", puzzle_id)
        .map(|raw| parse_exact_u16_token(routine, &raw, &raw))
        .transpose()?
        .unwrap_or(0);
    validate_unown_puzzle_state(routine, &layout, holding_piece)?;
    Ok(UnownPuzzleState {
        layout,
        holding_piece,
        moves,
    })
}

fn unown_coords(state: &GameState, routine: &str) -> Result<(usize, usize), SpecialRoutineError> {
    let x = required_usize_script_variable(state, routine, "unown_x")?;
    let y = required_usize_script_variable(state, routine, "unown_y")?;
    if x >= 6 || y >= 6 {
        return Err(SpecialRoutineError::InvalidUnownPuzzleState {
            routine: routine.to_string(),
            message: "coordinates must be inside the 6x6 puzzle grid".to_string(),
        });
    }
    Ok((x, y))
}

fn unown_puzzle_is_solved(puzzle: &UnownPuzzleState) -> bool {
    puzzle.holding_piece.is_none() && puzzle.layout == UNOWN_TARGET_LAYOUT
}

fn store_unown_puzzle_state(state: &mut GameState, puzzle_id: &str, puzzle: &UnownPuzzleState) {
    state.script_runtime.variables.insert(
        unown_puzzle_variable_key("unown_layout", puzzle_id),
        encode_unown_layout(&puzzle.layout),
    );
    state.script_runtime.variables.insert(
        unown_puzzle_variable_key("unown_holding_piece", puzzle_id),
        puzzle
            .holding_piece
            .map(|piece| piece.to_string())
            .unwrap_or_else(|| "null".to_string()),
    );
    state.script_runtime.variables.insert(
        unown_puzzle_variable_key("unown_moves", puzzle_id),
        puzzle.moves.to_string(),
    );
    for key in [
        "unown_layout",
        "unown_holding_piece",
        "unown_moves",
        "unown_action",
        "unown_x",
        "unown_y",
    ] {
        state.script_runtime.variables.remove(key);
    }
}

fn unown_puzzle(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let puzzle_id = normalize_unown_puzzle_id(state, routine)?;
    state
        .script_runtime
        .variables
        .insert("wSolvedUnownPuzzle".to_string(), "0".to_string());
    let action = state
        .script_runtime
        .variables
        .get("unown_action")
        .cloned()
        .map(|value| value.to_ascii_lowercase());

    let mut puzzle = if action.is_some() {
        load_unown_puzzle_state(state, routine, &puzzle_id)?
    } else {
        let mut rng = Random::new(state.rng_seed);
        let puzzle = shuffled_unown_puzzle(&mut rng);
        state.rng_seed = rng.seed();
        puzzle
    };

    match action.as_deref() {
        Some("shuffle") => {
            let mut rng = Random::new(state.rng_seed);
            puzzle = shuffled_unown_puzzle(&mut rng);
            state.rng_seed = rng.seed();
        }
        Some("pickup") => {
            let (x, y) = unown_coords(state, routine)?;
            if puzzle.holding_piece.is_some() {
                return Err(SpecialRoutineError::InvalidUnownPuzzleState {
                    routine: routine.to_string(),
                    message: "cannot pick up a piece while already holding one".to_string(),
                });
            }
            let piece = puzzle.layout[y][x];
            if piece == 0 {
                return Err(SpecialRoutineError::InvalidUnownPuzzleState {
                    routine: routine.to_string(),
                    message: "no piece present at that coordinate".to_string(),
                });
            }
            puzzle.layout[y][x] = 0;
            puzzle.holding_piece = Some(piece);
        }
        Some("place") => {
            let (x, y) = unown_coords(state, routine)?;
            let Some(piece) = puzzle.holding_piece else {
                return Err(SpecialRoutineError::InvalidUnownPuzzleState {
                    routine: routine.to_string(),
                    message: "no piece is currently held".to_string(),
                });
            };
            if puzzle.layout[y][x] != 0 {
                return Err(SpecialRoutineError::InvalidUnownPuzzleState {
                    routine: routine.to_string(),
                    message: "target coordinate is already occupied".to_string(),
                });
            }
            puzzle.layout[y][x] = piece;
            puzzle.holding_piece = None;
            puzzle.moves = puzzle.moves.saturating_add(1);
        }
        Some("noop") | None => {}
        Some(unknown) => {
            return Err(SpecialRoutineError::InvalidUnownPuzzleState {
                routine: routine.to_string(),
                message: format!("unknown Unown puzzle action '{unknown}'"),
            });
        }
    }

    let solved = unown_puzzle_is_solved(&puzzle);
    store_unown_puzzle_state(state, &puzzle_id, &puzzle);
    state.script_runtime.variables.insert(
        "wSolvedUnownPuzzle".to_string(),
        u8::from(solved).to_string(),
    );
    state.script_runtime.last_special_routine = Some(routine.to_string());
    state.script_runtime.active_menu = action.is_none().then(|| routine.to_string());
    set_script_bool_value(state, solved);
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::UnownPuzzle {
            puzzle_id,
            solved,
            moves: puzzle.moves,
            layout: unown_puzzle_layout_vec(&puzzle.layout),
            holding_piece: puzzle.holding_piece,
            rng_seed_after: state.rng_seed,
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

fn toggle_decorations_visibility(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let outcome = visual_command(
        state,
        routine,
        ScriptGraphicsRuntimeKind::ToggleDecorationsVisibility,
    )?;
    for (sprite_base, event_flag) in [
        ("SPRITE_CONSOLE", "EVENT_PLAYERS_HOUSE_2F_CONSOLE"),
        ("SPRITE_DOLL_1", "EVENT_PLAYERS_HOUSE_2F_DOLL_1"),
        ("SPRITE_DOLL_2", "EVENT_PLAYERS_HOUSE_2F_DOLL_2"),
        ("SPRITE_BIG_DOLL", "EVENT_PLAYERS_HOUSE_2F_BIG_DOLL"),
    ] {
        state.script_runtime.variable_sprites.remove(sprite_base);
        state
            .flags
            .set_event_flag(event_flag, true)
            .map_err(|error| SpecialRoutineError::EventFlag {
                routine: routine.to_string(),
                error,
            })?;
    }
    Ok(outcome)
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
        pokemon.species.id != "EGG"
            && (pokemon.pokerus != 0 || pokemon.status.as_deref() == Some(POKERUS_STATUS))
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
    let roll = required_u8_script_variable(state, routine, "_rng_roll")?;
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

#[derive(Debug, Clone, Copy)]
enum GameCornerGame {
    SlotMachine,
    CardFlip,
    UnusedMemoryGame,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlotSymbol {
    Seven,
    Pokeball,
    Cherry,
    Pikachu,
    Squirtle,
    Staryu,
}

const GAME_CORNER_MAX_COINS: u16 = 9999;
const SLOT_REEL_LENGTH: usize = 15;
const SLOT_PERCENT_1: u8 = 0x02;
const SLOT_PERCENT_3: u8 = 0x07;
const SLOT_PERCENT_4: u8 = 0x0a;
const SLOT_PERCENT_6: u8 = 0x0f;
const SLOT_PERCENT_8: u8 = 0x14;
const SLOT_PERCENT_12: u8 = 0x1e;
const SLOT_PERCENT_16: u8 = 0x28;
const SLOT_PERCENT_19: u8 = 0x30;
const SLOT_PERCENT_24: u8 = 0x3c;
const SLOT_PERCENT_31: u8 = 0x4f;
const SLOT_PERCENT_47: u8 = 0x78;
const SLOT_PERCENT_63: u8 = 0xa0;
const SLOT_PERCENT_71: u8 = 0xb4;
const SLOT_PERCENT_100: u8 = 0xff;
const SLOT_REELS: [[SlotSymbol; SLOT_REEL_LENGTH]; 3] = [
    [
        SlotSymbol::Seven,
        SlotSymbol::Cherry,
        SlotSymbol::Staryu,
        SlotSymbol::Pikachu,
        SlotSymbol::Squirtle,
        SlotSymbol::Seven,
        SlotSymbol::Cherry,
        SlotSymbol::Staryu,
        SlotSymbol::Pikachu,
        SlotSymbol::Squirtle,
        SlotSymbol::Pokeball,
        SlotSymbol::Cherry,
        SlotSymbol::Staryu,
        SlotSymbol::Pikachu,
        SlotSymbol::Squirtle,
    ],
    [
        SlotSymbol::Seven,
        SlotSymbol::Pikachu,
        SlotSymbol::Cherry,
        SlotSymbol::Squirtle,
        SlotSymbol::Staryu,
        SlotSymbol::Pokeball,
        SlotSymbol::Pikachu,
        SlotSymbol::Cherry,
        SlotSymbol::Squirtle,
        SlotSymbol::Staryu,
        SlotSymbol::Pokeball,
        SlotSymbol::Pikachu,
        SlotSymbol::Cherry,
        SlotSymbol::Squirtle,
        SlotSymbol::Staryu,
    ],
    [
        SlotSymbol::Seven,
        SlotSymbol::Pikachu,
        SlotSymbol::Cherry,
        SlotSymbol::Squirtle,
        SlotSymbol::Staryu,
        SlotSymbol::Pikachu,
        SlotSymbol::Cherry,
        SlotSymbol::Squirtle,
        SlotSymbol::Staryu,
        SlotSymbol::Pikachu,
        SlotSymbol::Pokeball,
        SlotSymbol::Cherry,
        SlotSymbol::Squirtle,
        SlotSymbol::Staryu,
        SlotSymbol::Pikachu,
    ],
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct SlotSpinOutcome {
    matched_symbol: Option<SlotSymbol>,
    winning_lines: Vec<String>,
    payout: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CardFlipOutcome {
    card_index: usize,
    card_name: String,
    payout: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MemoryGameOutcome {
    matched: bool,
    symbol: Option<String>,
    first_index: usize,
    second_index: usize,
}

fn slot_machine(
    state: &mut GameState,
    item_catalog: &BTreeMap<String, Item>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    coin_game_service(state, item_catalog, routine, GameCornerGame::SlotMachine)
}

fn card_flip(
    state: &mut GameState,
    item_catalog: &BTreeMap<String, Item>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    coin_game_service(state, item_catalog, routine, GameCornerGame::CardFlip)
}

fn unused_memory_game(
    state: &mut GameState,
    item_catalog: &BTreeMap<String, Item>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    coin_game_service(
        state,
        item_catalog,
        routine,
        GameCornerGame::UnusedMemoryGame,
    )
}

fn slot_symbol_name(symbol: SlotSymbol) -> &'static str {
    match symbol {
        SlotSymbol::Seven => "SEVEN",
        SlotSymbol::Pokeball => "POKEBALL",
        SlotSymbol::Cherry => "CHERRY",
        SlotSymbol::Pikachu => "PIKACHU",
        SlotSymbol::Squirtle => "SQUIRTLE",
        SlotSymbol::Staryu => "STARYU",
    }
}

fn slot_symbol_payout(symbol: SlotSymbol) -> u16 {
    match symbol {
        SlotSymbol::Seven => 300,
        SlotSymbol::Pokeball => 50,
        SlotSymbol::Cherry => 6,
        SlotSymbol::Pikachu => 8,
        SlotSymbol::Squirtle => 10,
        SlotSymbol::Staryu => 15,
    }
}

fn slot_next_byte(rng: &mut Random) -> u8 {
    rng.randrange(256) as u8
}

fn slot_window(reel: &[SlotSymbol; SLOT_REEL_LENGTH], offset: usize) -> [SlotSymbol; 3] {
    [
        reel[offset % SLOT_REEL_LENGTH],
        reel[(offset + 1) % SLOT_REEL_LENGTH],
        reel[(offset + 2) % SLOT_REEL_LENGTH],
    ]
}

fn slot_advance(offset: usize, step: usize) -> usize {
    (offset + step) % SLOT_REEL_LENGTH
}

fn slot_line_order(bet: u8) -> &'static [&'static str] {
    match bet {
        1 => &["middle"],
        2 => &["bottom", "top", "middle"],
        _ => &["diagonal_up", "diagonal_down", "bottom", "top", "middle"],
    }
}

fn slot_line_symbols(windows: &[[SlotSymbol; 3]; 3], line: &str) -> [SlotSymbol; 3] {
    match line {
        "middle" => [windows[0][1], windows[1][1], windows[2][1]],
        "top" => [windows[0][0], windows[1][0], windows[2][0]],
        "bottom" => [windows[0][2], windows[1][2], windows[2][2]],
        "diagonal_up" => [windows[0][2], windows[1][1], windows[2][0]],
        "diagonal_down" => [windows[0][0], windows[1][1], windows[2][2]],
        _ => unreachable!("slot line comes from static line table"),
    }
}

fn slot_check_first_two(windows: &[[SlotSymbol; 3]; 2], bet: u8) -> (Option<SlotSymbol>, bool) {
    let mut matched_symbol = None;
    let mut saw_seven = false;
    for line in slot_line_order(bet) {
        let [first, second] = match *line {
            "middle" => [windows[0][1], windows[1][1]],
            "top" => [windows[0][0], windows[1][0]],
            "bottom" => [windows[0][2], windows[1][2]],
            "diagonal_up" => [windows[0][2], windows[1][1]],
            "diagonal_down" => [windows[0][0], windows[1][1]],
            _ => unreachable!("slot line comes from static line table"),
        };
        if first == second {
            matched_symbol = Some(first);
            saw_seven |= first == SlotSymbol::Seven;
        }
    }
    (matched_symbol, saw_seven)
}

fn slot_check_all_three(
    windows: &[[SlotSymbol; 3]; 3],
    bet: u8,
) -> (Option<SlotSymbol>, Vec<String>) {
    let mut matched_symbol = None;
    let mut winning_lines = Vec::new();
    for line in slot_line_order(bet) {
        let [first, second, third] = slot_line_symbols(windows, line);
        if first == second && second == third {
            matched_symbol = Some(first);
            winning_lines.push((*line).to_string());
        }
    }
    (matched_symbol, winning_lines)
}

fn slot_bias(rng: &mut Random, lucky: bool) -> Option<SlotSymbol> {
    let table: &[(u8, Option<SlotSymbol>)] = if lucky {
        &[
            (SLOT_PERCENT_1, Some(SlotSymbol::Seven)),
            (SLOT_PERCENT_1 + 1, Some(SlotSymbol::Pokeball)),
            (SLOT_PERCENT_3 + 1, Some(SlotSymbol::Staryu)),
            (SLOT_PERCENT_6 + 1, Some(SlotSymbol::Squirtle)),
            (SLOT_PERCENT_12, Some(SlotSymbol::Pikachu)),
            (SLOT_PERCENT_31 + 1, Some(SlotSymbol::Cherry)),
            (SLOT_PERCENT_100, None),
        ]
    } else {
        &[
            (SLOT_PERCENT_1 - 1, Some(SlotSymbol::Seven)),
            (SLOT_PERCENT_1 + 1, Some(SlotSymbol::Pokeball)),
            (SLOT_PERCENT_4, Some(SlotSymbol::Staryu)),
            (SLOT_PERCENT_8, Some(SlotSymbol::Squirtle)),
            (SLOT_PERCENT_16, Some(SlotSymbol::Pikachu)),
            (SLOT_PERCENT_19, Some(SlotSymbol::Cherry)),
            (SLOT_PERCENT_100, None),
        ]
    };
    let roll = slot_next_byte(rng);
    table
        .iter()
        .find_map(|(threshold, symbol)| (roll <= *threshold).then_some(*symbol))
        .flatten()
}

fn slot_stop_reel1(mut offset: usize, bias: Option<SlotSymbol>) -> usize {
    let Some(bias) = bias else {
        return offset;
    };
    let mut counter = 4;
    while counter > 0 {
        if slot_window(&SLOT_REELS[0], offset).contains(&bias) {
            break;
        }
        offset = slot_advance(offset, 1);
        counter -= 1;
    }
    offset
}

fn slot_attempt_skip_to_seven(offsets: [usize; 3], bet: u8) -> Option<[usize; 3]> {
    let first_window = slot_window(&SLOT_REELS[0], offsets[0]);
    if !first_window.contains(&SlotSymbol::Seven) {
        return None;
    }
    let mut offset_two = offsets[1];
    for _ in 0..(SLOT_REEL_LENGTH * 2) {
        let windows = [first_window, slot_window(&SLOT_REELS[1], offset_two)];
        let (_, saw_seven) = slot_check_first_two(&windows, bet);
        if saw_seven {
            return Some([offsets[0], offset_two, offsets[2]]);
        }
        offset_two = slot_advance(offset_two, 1);
    }
    None
}

fn slot_stop_reel2(offsets: &mut [usize; 3], bias: Option<SlotSymbol>, bet: u8, rng: &mut Random) {
    if bet >= 2
        && (bias.is_none() || bias == Some(SlotSymbol::Seven))
        && slot_next_byte(rng) < SLOT_PERCENT_31 + 1
        && let Some(aligned) = slot_attempt_skip_to_seven(*offsets, bet)
    {
        *offsets = aligned;
        return;
    }

    let mut counter = 4;
    loop {
        let windows = [
            slot_window(&SLOT_REELS[0], offsets[0]),
            slot_window(&SLOT_REELS[1], offsets[1]),
        ];
        let (matched_symbol, _) = slot_check_first_two(&windows, bet);
        if matched_symbol.is_some() && matched_symbol == bias {
            return;
        }
        if bias.is_none() || counter == 0 {
            return;
        }
        offsets[1] = slot_advance(offsets[1], 1);
        counter -= 1;
    }
}

fn slot_find_reel3_offset(
    offsets: &mut [usize; 3],
    bet: u8,
    target_symbol: Option<SlotSymbol>,
    step: usize,
) -> usize {
    for _ in 0..(SLOT_REEL_LENGTH * 2) {
        let windows = [
            slot_window(&SLOT_REELS[0], offsets[0]),
            slot_window(&SLOT_REELS[1], offsets[1]),
            slot_window(&SLOT_REELS[2], offsets[2]),
        ];
        let (matched_symbol, _) = slot_check_all_three(&windows, bet);
        if target_symbol.is_none() {
            if matched_symbol.is_none() {
                return offsets[2];
            }
        } else if matched_symbol == target_symbol {
            return offsets[2];
        }
        offsets[2] = slot_advance(offsets[2], step);
    }
    offsets[2]
}

fn slot_apply_reel3_stop(offsets: &mut [usize; 3], bias: Option<SlotSymbol>, bet: u8) {
    let mut counter = 4;
    loop {
        let windows = [
            slot_window(&SLOT_REELS[0], offsets[0]),
            slot_window(&SLOT_REELS[1], offsets[1]),
            slot_window(&SLOT_REELS[2], offsets[2]),
        ];
        let (matched_symbol, _) = slot_check_all_three(&windows, bet);
        if let Some(matched) = matched_symbol {
            if Some(matched) == bias {
                return;
            }
            offsets[2] = slot_advance(offsets[2], 1);
            if counter > 0 {
                counter -= 1;
            }
            continue;
        }
        if bias.is_none() || counter == 0 {
            return;
        }
        offsets[2] = slot_advance(offsets[2], 1);
        counter -= 1;
    }
}

fn slot_apply_reel3_golem(
    offsets: &mut [usize; 3],
    bias: Option<SlotSymbol>,
    bet: u8,
    rng: &mut Random,
) {
    if bias == Some(SlotSymbol::Seven) {
        offsets[2] = slot_find_reel3_offset(offsets, bet, Some(SlotSymbol::Seven), 1);
        return;
    }
    let mut stride = 0;
    while stride < 4 {
        stride = slot_next_byte(rng) & 0x7;
    }
    let mut step = usize::from(stride);
    for _ in 0..(SLOT_REEL_LENGTH * 2) {
        let windows = [
            slot_window(&SLOT_REELS[0], offsets[0]),
            slot_window(&SLOT_REELS[1], offsets[1]),
            slot_window(&SLOT_REELS[2], offsets[2]),
        ];
        let (matched_symbol, _) = slot_check_all_three(&windows, bet);
        if matched_symbol.is_none() {
            return;
        }
        offsets[2] = slot_advance(offsets[2], step);
        step += 1;
    }
}

fn slot_stop_reel3(offsets: &mut [usize; 3], bias: Option<SlotSymbol>, bet: u8, rng: &mut Random) {
    let windows_first_two = [
        slot_window(&SLOT_REELS[0], offsets[0]),
        slot_window(&SLOT_REELS[1], offsets[1]),
    ];
    let (matched_symbol, saw_seven) = slot_check_first_two(&windows_first_two, bet);
    if matched_symbol.is_none() || !saw_seven {
        slot_apply_reel3_stop(offsets, bias, bet);
        return;
    }
    let action = if bias == Some(SlotSymbol::Seven) {
        let roll = slot_next_byte(rng);
        if roll >= SLOT_PERCENT_71 {
            "stop"
        } else if roll >= SLOT_PERCENT_47 {
            "slow"
        } else if roll >= SLOT_PERCENT_24 {
            "golem"
        } else {
            "chansey"
        }
    } else {
        let roll = slot_next_byte(rng);
        if roll >= SLOT_PERCENT_63 {
            "stop"
        } else if roll >= SLOT_PERCENT_31 + 1 {
            "slow"
        } else {
            "golem"
        }
    };
    match action {
        "stop" => slot_apply_reel3_stop(offsets, bias, bet),
        "slow" => {
            let target = if bias == Some(SlotSymbol::Seven) {
                Some(SlotSymbol::Seven)
            } else {
                None
            };
            offsets[2] = slot_find_reel3_offset(offsets, bet, target, 1);
        }
        "golem" => slot_apply_reel3_golem(offsets, bias, bet, rng),
        "chansey" => offsets[2] = slot_find_reel3_offset(offsets, bet, Some(SlotSymbol::Seven), 17),
        _ => unreachable!("slot action is selected from static branches"),
    }
}

fn spin_slot_machine(rng: &mut Random, bet: u8, lucky: bool) -> SlotSpinOutcome {
    let bias = slot_bias(rng, lucky);
    let mut offsets = [
        usize::from(slot_next_byte(rng)) % SLOT_REEL_LENGTH,
        usize::from(slot_next_byte(rng)) % SLOT_REEL_LENGTH,
        usize::from(slot_next_byte(rng)) % SLOT_REEL_LENGTH,
    ];
    offsets[0] = slot_stop_reel1(offsets[0], bias);
    slot_stop_reel2(&mut offsets, bias, bet, rng);
    slot_stop_reel3(&mut offsets, bias, bet, rng);
    let windows = [
        slot_window(&SLOT_REELS[0], offsets[0]),
        slot_window(&SLOT_REELS[1], offsets[1]),
        slot_window(&SLOT_REELS[2], offsets[2]),
    ];
    let (matched_symbol, winning_lines) = slot_check_all_three(&windows, bet);
    let payout = matched_symbol.map(slot_symbol_payout).unwrap_or(0);
    SlotSpinOutcome {
        matched_symbol,
        winning_lines,
        payout,
    }
}

fn parse_card_flip_deck(value: Option<&String>) -> Vec<String> {
    value
        .map(|raw| {
            raw.split(',')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn parse_card_flip_revealed(value: Option<&String>, len: usize) -> Vec<bool> {
    let mut revealed = value
        .map(|raw| {
            raw.split(',')
                .map(|part| matches!(part.trim(), "1" | "true" | "TRUE"))
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec![false; len]);
    revealed.truncate(len);
    while revealed.len() < len {
        revealed.push(false);
    }
    revealed
}

fn parse_comma_tokens(value: Option<&String>) -> Vec<String> {
    value
        .map(|raw| {
            raw.split(',')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn parse_bool_tokens(value: Option<&String>, len: usize) -> Vec<bool> {
    let mut values = value
        .map(|raw| {
            raw.split(',')
                .map(|part| matches!(part.trim(), "1" | "true" | "TRUE"))
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec![false; len]);
    values.truncate(len);
    while values.len() < len {
        values.push(false);
    }
    values
}

fn default_card_flip_deck() -> Vec<String> {
    [
        "ODDISH",
        "POLIWAG",
        "PIKACHU",
        "JIGGLYPUFF",
        "RATTATA",
        "VOLTORB",
    ]
    .iter()
    .flat_map(|name| std::iter::repeat_n((*name).to_string(), 4))
    .collect()
}

fn shuffle_card_flip_deck(deck: &mut [String], rng: &mut Random) {
    for index in (1..deck.len()).rev() {
        let swap_index = rng.randrange((index + 1) as u32) as usize;
        deck.swap(index, swap_index);
    }
}

fn card_flip_payout(card_name: &str, deck: &[String], revealed: &[bool]) -> u16 {
    let remaining = deck
        .iter()
        .enumerate()
        .filter(|(index, card)| !revealed[*index] && card.as_str() == card_name)
        .count();
    match card_name {
        "PIKACHU" => match remaining {
            6 => 6,
            5 => 12,
            4 => 24,
            3 => 36,
            2 => 48,
            1 => 72,
            _ => 6,
        },
        _ => match remaining {
            4 => 6,
            3 => 12,
            2 => 18,
            1 => 36,
            _ => 6,
        },
    }
}

fn flip_card(state: &mut GameState, rng: &mut Random) -> CardFlipOutcome {
    let mut deck = parse_card_flip_deck(state.script_runtime.variables.get("card_flip_deck"));
    if deck.is_empty() {
        deck = default_card_flip_deck();
        shuffle_card_flip_deck(&mut deck, rng);
    }
    let mut revealed = parse_card_flip_revealed(
        state.script_runtime.variables.get("card_flip_revealed"),
        deck.len(),
    );
    let mut card_index = state
        .script_runtime
        .variables
        .get("card_flip_index")
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|index| *index < deck.len())
        .unwrap_or(0);
    if revealed.get(card_index).copied().unwrap_or(false)
        && let Some(fallback) = revealed.iter().position(|flag| !*flag)
    {
        card_index = fallback;
    }
    revealed[card_index] = true;
    let card_name = deck[card_index].clone();
    let payout = card_flip_payout(&card_name, &deck, &revealed);
    state
        .script_runtime
        .variables
        .insert("card_flip_deck".to_string(), deck.join(","));
    state.script_runtime.variables.insert(
        "card_flip_revealed".to_string(),
        revealed
            .iter()
            .map(|flag| if *flag { "1" } else { "0" })
            .collect::<Vec<_>>()
            .join(","),
    );
    CardFlipOutcome {
        card_index,
        card_name,
        payout,
    }
}

fn default_memory_board() -> Vec<String> {
    [
        "ODDISH",
        "POLIWAG",
        "PIKACHU",
        "JIGGLYPUFF",
        "RATTATA",
        "VOLTORB",
        "DITTO",
        "ELECTABUZZ",
    ]
    .iter()
    .flat_map(|name| [(*name).to_string(), (*name).to_string()])
    .collect()
}

fn shuffle_memory_board(board: &mut [String], rng: &mut Random) {
    for index in (1..board.len()).rev() {
        let swap_index = rng.randrange((index + 1) as u32) as usize;
        board.swap(index, swap_index);
    }
}

fn memory_reveal(
    board: &[String],
    revealed: &mut [bool],
    first_index: usize,
    second_index: usize,
) -> Result<MemoryGameOutcome, ()> {
    if first_index == second_index
        || first_index >= board.len()
        || second_index >= board.len()
        || revealed.get(first_index).copied().unwrap_or(false)
        || revealed.get(second_index).copied().unwrap_or(false)
    {
        return Err(());
    }
    let matched = board[first_index] == board[second_index];
    let symbol = matched.then(|| board[first_index].clone());
    if matched {
        revealed[first_index] = true;
        revealed[second_index] = true;
    }
    Ok(MemoryGameOutcome {
        matched,
        symbol,
        first_index,
        second_index,
    })
}

fn play_memory_game(state: &mut GameState, rng: &mut Random) -> Result<MemoryGameOutcome, ()> {
    let mut board = default_memory_board();
    shuffle_memory_board(&mut board, rng);

    let board_state = parse_comma_tokens(state.script_runtime.variables.get("memory_board"));
    let mut revealed = if board_state.is_empty() {
        shuffle_memory_board(&mut board, rng);
        vec![false; board.len()]
    } else {
        board = board_state;
        parse_bool_tokens(
            state.script_runtime.variables.get("memory_revealed"),
            board.len(),
        )
    };

    let first_index = state
        .script_runtime
        .variables
        .get("memory_first")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let second_index = state
        .script_runtime
        .variables
        .get("memory_second")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1);
    let outcome = memory_reveal(&board, &mut revealed, first_index, second_index)
        .or_else(|_| memory_reveal(&board, &mut revealed, 0, 1))?;

    state
        .script_runtime
        .variables
        .insert("memory_board".to_string(), board.join(","));
    state.script_runtime.variables.insert(
        "memory_revealed".to_string(),
        revealed
            .iter()
            .map(|flag| if *flag { "1" } else { "0" })
            .collect::<Vec<_>>()
            .join(","),
    );
    Ok(outcome)
}

fn coin_game_service(
    state: &mut GameState,
    item_catalog: &BTreeMap<String, Item>,
    routine: &str,
    game: GameCornerGame,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    const COIN_CASE: &str = "COIN_CASE";
    let coin_case =
        item_catalog
            .get(COIN_CASE)
            .ok_or_else(|| SpecialRoutineError::UnknownItem {
                routine: routine.to_string(),
                item_id: COIN_CASE.to_string(),
            })?;
    if state.coins == 0 {
        state.script_runtime.last_special_routine = Some(routine.to_string());
        return Ok(SpecialRoutineOutcome {
            routine: routine.to_string(),
            effect: SpecialRoutineEffect::GameCornerGameUnavailable {
                game: routine.to_string(),
                reason: GameCornerUnavailableReason::NoCoins,
            },
        });
    }
    if !state.bag.has_item(coin_case) {
        state.script_runtime.last_special_routine = Some(routine.to_string());
        return Ok(SpecialRoutineOutcome {
            routine: routine.to_string(),
            effect: SpecialRoutineEffect::GameCornerGameUnavailable {
                game: routine.to_string(),
                reason: GameCornerUnavailableReason::MissingCoinCase,
            },
        });
    }

    state
        .script_runtime
        .variables
        .insert("_coin_case_balance".to_string(), state.coins.to_string());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    state.script_runtime.active_menu = None;
    let effect = match game {
        GameCornerGame::SlotMachine => {
            let coins_before = state.coins;
            let mut bet = state
                .script_runtime
                .variables
                .get("slot_bet")
                .and_then(|value| value.parse::<u8>().ok())
                .unwrap_or(3)
                .clamp(1, 3);
            if coins_before < u16::from(bet) {
                bet = coins_before.max(1) as u8;
            }
            let lucky = state
                .script_runtime
                .variables
                .get("slot_mode")
                .is_some_and(|mode| mode.eq_ignore_ascii_case("lucky"))
                || matches!(
                    state.script_runtime.script_value.as_deref(),
                    Some("1") | Some("TRUE") | Some("true")
                );
            let mut rng = Random::new(state.rng_seed);
            let spin = spin_slot_machine(&mut rng, bet, lucky);
            state.rng_seed = rng.seed();
            let coins = coins_before
                .saturating_sub(u16::from(bet))
                .saturating_add(spin.payout)
                .min(GAME_CORNER_MAX_COINS);
            state.coins = coins;
            set_script_u32_value(state, u32::from(coins));
            SpecialRoutineEffect::SlotMachine {
                coins_before,
                bet,
                payout: spin.payout,
                matched_symbol: spin
                    .matched_symbol
                    .map(slot_symbol_name)
                    .map(str::to_string),
                winning_lines: spin.winning_lines,
                coins,
                rng_seed_after: state.rng_seed,
            }
        }
        GameCornerGame::CardFlip => {
            if state.coins < 3 {
                return Ok(SpecialRoutineOutcome {
                    routine: routine.to_string(),
                    effect: SpecialRoutineEffect::GameCornerGameUnavailable {
                        game: routine.to_string(),
                        reason: GameCornerUnavailableReason::NoCoins,
                    },
                });
            }
            let coins_before = state.coins;
            state.coins = state.coins.saturating_sub(3);
            let mut rng = Random::new(state.rng_seed);
            let flip = flip_card(state, &mut rng);
            state.rng_seed = rng.seed();
            let coins = state
                .coins
                .saturating_add(flip.payout)
                .min(GAME_CORNER_MAX_COINS);
            state.coins = coins;
            set_script_u32_value(state, u32::from(coins));
            SpecialRoutineEffect::CardFlip {
                coins_before,
                card_index: flip.card_index,
                card_name: flip.card_name,
                payout: flip.payout,
                coins,
                rng_seed_after: state.rng_seed,
            }
        }
        GameCornerGame::UnusedMemoryGame => {
            let mut rng = Random::new(state.rng_seed);
            let outcome = play_memory_game(state, &mut rng).map_err(|_| {
                SpecialRoutineError::InvalidNumericValue {
                    routine: routine.to_string(),
                    value: "memory_first,memory_second".to_string(),
                }
            })?;
            state.rng_seed = rng.seed();
            set_script_bool_value(state, outcome.matched);
            SpecialRoutineEffect::UnusedMemoryGame {
                matched: outcome.matched,
                symbol: outcome.symbol,
                first_index: outcome.first_index,
                second_index: outcome.second_index,
                coins: state.coins,
                rng_seed_after: state.rng_seed,
            }
        }
    };
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect,
    })
}

fn unused_find_item_in_pc_or_bag(
    state: &mut GameState,
    item_catalog: &BTreeMap<String, Item>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let item_id = required_raw_script_value(state, routine)?;
    let item = item_catalog
        .get(&item_id)
        .ok_or_else(|| SpecialRoutineError::UnknownItem {
            routine: routine.to_string(),
            item_id: item_id.clone(),
        })?;
    let found_in_pc = state.bag.has_pc_item(item);
    let found_in_bag = if found_in_pc {
        false
    } else {
        state.bag.has_item(item)
    };
    let script_value = u8::from(found_in_pc || found_in_bag);
    state.script_runtime.script_value = Some(script_value.to_string());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), script_value.to_string());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::UnusedFindItemInPcOrBag {
            item_id,
            found_in_pc,
            found_in_bag,
            script_value,
        },
    })
}

fn function11ba38(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let selected_party_slot = required_selected_party_slot(state, routine)?;
    required_party_pokemon(state, routine, selected_party_slot)?;
    let other_usable_party_mon =
        state
            .storage
            .party
            .pokemon
            .iter()
            .enumerate()
            .any(|(index, pokemon)| {
                index != selected_party_slot
                    && pokemon.as_ref().is_some_and(|pokemon| pokemon.hp > 0)
            });
    let script_value = u8::from(!other_usable_party_mon);
    state.script_runtime.script_value = Some(script_value.to_string());
    state
        .script_runtime
        .variables
        .insert("_value".to_string(), script_value.to_string());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::Function11ba38 {
            selected_party_slot,
            other_usable_party_mon,
            script_value,
        },
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
    let party_slot = required_usize_script_variable(state, routine, "_party_slot")?;
    let species = state
        .storage
        .party
        .pokemon
        .get(party_slot)
        .and_then(Option::as_ref)
        .map(|pokemon| pokemon.species.id.clone());
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
            party_slot: Some(party_slot),
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

    let selection_cancelled =
        required_bool_script_variable(state, routine, "_selection_cancelled")?;
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
    move_sets: &DratiniMoveSets,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    if move_sets.is_empty() {
        return Err(SpecialRoutineError::MissingDratiniMoveSets {
            routine: routine.to_string(),
        });
    }
    let mode = required_u8_script_value(state, routine)?;
    let Some(move_names) = move_sets.get(&mode) else {
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

    let mut learned = Vec::with_capacity(move_names.len());
    for move_name in move_names {
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
            move_names: move_names.clone(),
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
    recipes: &KurtApricornRecipes,
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
        .cloned();
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
    if !recipes.contains_key(&apricorn) {
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
    definitions: &RoamingPokemonDefinitions,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    if definitions.is_empty() {
        return Err(SpecialRoutineError::MissingRoamingPokemonDefinitions {
            routine: routine.to_string(),
        });
    }
    let mut roamers = Vec::with_capacity(definitions.len());
    for (species, definition) in definitions {
        required_species_metadata(species_catalog, routine, species)?;
        roamers.push(RoamingPokemonState {
            species: species.clone(),
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
    categories: &BuenaPasswordCategories,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let (category_id, category, correct) = ensure_buenas_password(state, categories, routine)?;
    let guess = state
        .script_runtime
        .variables
        .get("BUENA_PASSWORD")
        .cloned();
    if let Some(guess) = guess.as_deref() {
        let exact_guess = if category.category_type == BUENA_PASSWORD_CATEGORY_STRING {
            is_exact_nonempty_special_value(guess)
        } else {
            is_exact_nonempty_special_token(guess)
        };
        if !exact_guess {
            return Err(SpecialRoutineError::InvalidBuenaPasswordGuess {
                routine: routine.to_string(),
                guess: guess.to_string(),
            });
        }
    }
    let matched = guess.as_deref() == Some(correct.as_str());
    set_script_bool_value(state, matched);
    state
        .script_runtime
        .variables
        .insert("_buena_category".to_string(), category_id.to_string());
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
            category: category_id.to_string(),
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
    buena_prizes: &BuenaPrizeDefinitions,
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
    let Some(cost) = buena_prizes.get(&selected) else {
        return Err(SpecialRoutineError::UnknownItem {
            routine: routine.to_string(),
            item_id: selected,
        });
    };
    let points_spent = cost.checked_mul(quantity as u8).ok_or_else(|| {
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
                item_id: selected.clone(),
                quantity,
                points_spent,
                balance: state.blue_card_balance,
            },
        });
    }
    let item = item_catalog
        .get(&selected)
        .ok_or_else(|| SpecialRoutineError::UnknownItem {
            routine: routine.to_string(),
            item_id: selected.clone(),
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
                item_id: selected.clone(),
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
            item_id: selected,
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

    if required_bool_script_variable(state, routine, "_selection_cancelled")? {
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

pub fn validate_saved_magikarp_record_references(
    record: &MagikarpRecordState,
    has_magikarp_lengths: bool,
) -> Result<(), SpecialRoutineError> {
    let has_record = record.current_feet != 0
        || record.current_inches != 0
        || record.best_feet != 0
        || record.best_inches != 0
        || !record.best_owner_name.is_empty();
    if has_record && !has_magikarp_lengths {
        return Err(SpecialRoutineError::SavedMagikarpRecordRequiresLengthDefinitions);
    }
    Ok(())
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
        "deposit" => day_care_deposit(state, routine, caretaker)?,
        "withdraw" => day_care_withdraw(state, routine, caretaker)?,
        "inspect" => day_care_inspect_interaction(state, routine, caretaker)?,
        exact => {
            return Err(SpecialRoutineError::MissingScriptValue {
                routine: routine.to_string(),
                variable: format!("unsupported _day_care_action {exact}"),
            });
        }
    };
    // ASM keeps the resident active only after the requested interaction has
    // been validated.  In particular, a failed deposit/withdraw must not
    // leave a save with an active but unchanged resident.
    if (action == "deposit" && outcome.success) || action == "inspect" {
        set_day_care_active(state, routine, caretaker, true)?;
    }
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
    state.bug_contest.pending_caught_mon = None;
    state.bug_contest.caught_species = None;
    state.bug_contest.caught_level = None;
    state.bug_contest.party_backup.clear();
    state.bug_contest.timer_active = true;
    state.bug_contest.timer_minutes_remaining = config.timer_minutes;
    state.bug_contest.timer_seconds_remaining = config.timer_seconds;
    state.bug_contest.timer_start_time = Some(current_bug_contest_time(state));
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

fn current_bug_contest_time(state: &GameState) -> ClockTime {
    ClockTime {
        day: state.time.current_day % 140,
        hour: state.time.registers.hours,
        minute: state.time.registers.minutes,
        second: state.time.registers.seconds,
    }
}

fn elapsed_bug_contest_time(start: ClockTime, current: ClockTime) -> (u8, u8, u8, u8) {
    let mut seconds = i16::from(current.second) - i16::from(start.second);
    let mut borrow = 0;
    if seconds < 0 {
        seconds += 60;
        borrow = 1;
    }
    let mut minutes = i16::from(current.minute) - i16::from(start.minute) - borrow;
    borrow = 0;
    if minutes < 0 {
        minutes += 60;
        borrow = 1;
    }
    let mut hours = i16::from(current.hour) - i16::from(start.hour) - borrow;
    borrow = 0;
    if hours < 0 {
        hours += 24;
        borrow = 1;
    }
    let days = (i16::from(current.day) - i16::from(start.day) - borrow).rem_euclid(140);
    (days as u8, hours as u8, minutes as u8, seconds as u8)
}

fn start_bug_contest_timer(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    state.bug_contest.timer_active = true;
    state.bug_contest.timer_minutes_remaining = 20;
    state.bug_contest.timer_seconds_remaining = 0;
    state.bug_contest.timer_start_time = Some(current_bug_contest_time(state));
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::BugContestTimer {
            active: true,
            minutes_remaining: 20,
            seconds_remaining: 0,
        },
    })
}

fn check_bug_contest_timer(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let start = state.bug_contest.timer_start_time.ok_or_else(|| {
        SpecialRoutineError::BugContestTimerNotStarted {
            routine: routine.to_string(),
        }
    })?;
    let current = current_bug_contest_time(state);
    let (days, hours, elapsed_minutes, elapsed_seconds) = elapsed_bug_contest_time(start, current);
    state.bug_contest.timer_start_time = Some(current);

    let timed_out = if days > 0 || hours > 0 {
        true
    } else {
        let mut seconds_remaining =
            i16::from(state.bug_contest.timer_seconds_remaining) - i16::from(elapsed_seconds);
        let mut borrow = 0;
        if seconds_remaining < 0 {
            seconds_remaining += 60;
            borrow = 1;
        }
        let minutes_remaining = i16::from(state.bug_contest.timer_minutes_remaining)
            - i16::from(elapsed_minutes)
            - borrow;
        if minutes_remaining < 0 {
            true
        } else {
            state.bug_contest.timer_minutes_remaining = minutes_remaining as u8;
            state.bug_contest.timer_seconds_remaining = seconds_remaining as u8;
            false
        }
    };
    if timed_out {
        state.bug_contest.timer_minutes_remaining = 0;
        state.bug_contest.timer_seconds_remaining = 0;
    }
    let active = !timed_out;
    state.bug_contest.timer_active = active;
    set_script_bool_value(state, active);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::BugContestTimer {
            active,
            minutes_remaining: state.bug_contest.timer_minutes_remaining,
            seconds_remaining: state.bug_contest.timer_seconds_remaining,
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
    state.bug_contest.pending_caught_mon = None;
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
    state.bug_contest.pending_caught_mon = None;
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
        state.bug_contest.pending_caught_mon = None;
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

pub fn resolve_bug_contest_caught_mon(
    state: &mut GameState,
    keep_new: bool,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let pending = state.bug_contest.pending_caught_mon.take().ok_or_else(|| {
        SpecialRoutineError::InvalidState {
            routine: "BugContestSetCaughtContestMon".to_string(),
            message: "no pending caught Pokemon decision".to_string(),
        }
    })?;
    let species = pending.species.id.clone();
    if keep_new {
        state.bug_contest.caught_species = Some(species.clone());
        state.bug_contest.caught_level = Some(pending.level);
        state.bug_contest.caught_mon = Some(pending);
    }
    state.script_runtime.pending_yes_no = None;
    state.script_runtime.text_window_open = false;
    set_script_numeric_value(state, u8::from(!keep_new));
    state.script_runtime.last_special_routine = Some("BugContestSetCaughtContestMon".to_string());
    Ok(SpecialRoutineOutcome {
        routine: "BugContestSetCaughtContestMon".to_string(),
        effect: SpecialRoutineEffect::BugContestCaughtMonResolved {
            kept: keep_new,
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
    if raw_action.trim().is_empty() || raw_action.trim_start().starts_with(';') {
        return Err(SpecialRoutineError::MissingScriptValue {
            routine: routine.to_string(),
            variable: "_value action token".to_string(),
        });
    }
    if !is_exact_nonempty_special_token(&raw_action) {
        return Err(SpecialRoutineError::UnhandledBattleTowerAction {
            routine: routine.to_string(),
            action: raw_action.to_string(),
        });
    }
    let action = raw_action.to_string();
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
            let level_group = exact_battle_tower_level_group(state, rules, routine)?;
            let level_cap = level_group * rules.level_group_size;
            let highest = state
                .storage
                .party
                .pokemon
                .iter()
                .flatten()
                .map(|pokemon| pokemon.level)
                .max()
                .ok_or_else(|| SpecialRoutineError::EmptyParty {
                    routine: routine.to_string(),
                })?;
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
                .keys()
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
                    .contains_key(pokemon.species.id.as_str())
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

fn exact_battle_tower_level_group(
    state: &GameState,
    rules: &BattleTowerRules,
    routine: &str,
) -> Result<u8, SpecialRoutineError> {
    let level_group = state.battle_tower.level_group;
    if level_group < rules.minimum_level_group || level_group > rules.maximum_level_group {
        return Err(SpecialRoutineError::InvalidBattleTowerLevelGroup {
            routine: routine.to_string(),
            level_group,
            minimum: rules.minimum_level_group,
            maximum: rules.maximum_level_group,
        });
    }
    Ok(level_group)
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
    state.script_runtime.variables.insert(
        "_battle_tower_room_menu_cancelled".to_string(),
        "FALSE".to_string(),
    );
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
    let active_party_index = first_available_battle_party_index(state).ok_or_else(|| {
        SpecialRoutineError::BattleTowerTrainerBuild {
            routine: routine.to_string(),
            trainer_id: trainer_id.clone(),
            error: "no non-fainted player party Pokemon".to_string(),
        }
    })?;

    state.battle_result = 0;
    state.battle_active_party_index = Some(active_party_index);
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
    let remember = required_bool_script_variable(state, routine, "_yes_no_result")?;
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
            .expect("battle tower leaderboard sets script value before mirroring it"),
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
    if password.trim() != password || password.chars().any(char::is_control) {
        return Err(SpecialRoutineError::InvalidMobilePassword {
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
        .map(|part| parse_exact_usize_token(routine, part, raw))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(indexes)
}

fn parse_u8_token(routine: &str, raw: &str) -> Result<u8, SpecialRoutineError> {
    parse_exact_u8_token(routine, raw, raw)
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
        if pokemon_is_egg(pokemon) {
            continue;
        }
        if !species.insert(pokemon.species.id.as_str()) {
            return Some(rules.duplicate_species_failure_text.clone());
        }
    }

    let mut held_items = BTreeSet::new();
    for pokemon in &party {
        if pokemon_is_egg(pokemon) {
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

    if party.iter().any(|pokemon| pokemon_is_egg(pokemon)) {
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
    categories: &'a BuenaPasswordCategories,
    routine: &str,
) -> Result<(&'a str, &'a BuenaPasswordCategoryDefinition, String), SpecialRoutineError> {
    if categories.order.is_empty() || categories.categories.is_empty() {
        return Err(SpecialRoutineError::MissingBuenaPasswordCategories {
            routine: routine.to_string(),
        });
    }
    let current_day = state.time.current_day;
    if !state.buenas_password.generated || state.buenas_password.generation_day != current_day {
        let mut rng = Random::new(state.rng_seed);
        let category_index = rng.randrange(categories.order.len() as u32) as usize;
        let Some(category_id) = categories.order.get(category_index) else {
            return Err(SpecialRoutineError::InvalidBuenaPasswordCategoryIndex {
                routine: routine.to_string(),
                index: category_index,
            });
        };
        let Some(category) = categories.categories.get(category_id) else {
            return Err(SpecialRoutineError::InvalidBuenaPasswordCategoryIndex {
                routine: routine.to_string(),
                index: category_index,
            });
        };
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
    let Some(category_id) = categories.order.get(state.buenas_password.category_index) else {
        return Err(SpecialRoutineError::InvalidBuenaPasswordCategoryIndex {
            routine: routine.to_string(),
            index: state.buenas_password.category_index,
        });
    };
    let Some(category) = categories.categories.get(category_id) else {
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
    Ok((category_id.as_str(), category, correct.clone()))
}

pub fn validate_saved_buena_password_references(
    password: &BuenasPasswordState,
    categories: &BuenaPasswordCategories,
) -> Result<(), SpecialRoutineError> {
    if !password.generated {
        return Ok(());
    }
    let Some(category_id) = categories.order.get(password.category_index) else {
        return Err(
            SpecialRoutineError::SavedBuenaPasswordCategoryIndexOutOfRange {
                index: password.category_index,
            },
        );
    };
    let Some(category) = categories.categories.get(category_id) else {
        return Err(SpecialRoutineError::SavedBuenaPasswordMissingCategory {
            index: password.category_index,
            category_id: category_id.clone(),
        });
    };
    if category.options.get(password.option_index).is_none() {
        return Err(
            SpecialRoutineError::SavedBuenaPasswordOptionIndexOutOfRange {
                index: password.option_index,
                category_id: category_id.clone(),
            },
        );
    }
    Ok(())
}

const SAVED_SPECIAL_BATTLE_TYPE_BUILTIN_ROUTINES: &[(&str, &str)] = &[
    ("BATTLETYPE_TRAINER_HOUSE", "TrainerHouse"),
    ("BATTLETYPE_CELEBI", "CelebiShrineEvent"),
];

pub fn saved_special_battle_type_builtin_routines() -> &'static [(&'static str, &'static str)] {
    SAVED_SPECIAL_BATTLE_TYPE_BUILTIN_ROUTINES
}

pub fn saved_special_battle_type_builtin_routine(battle_type: &str) -> Option<&'static str> {
    saved_special_battle_type_builtin_routines()
        .iter()
        .find_map(|(candidate, routine)| (*candidate == battle_type).then_some(*routine))
}

pub fn validate_saved_pending_special_battle_type<F, G>(
    battle_type: Option<&str>,
    scripted_battle_type_exists: F,
    special_routine_exists: G,
) -> Result<(), SpecialRoutineError>
where
    F: Fn(&str) -> bool,
    G: Fn(&str) -> bool,
{
    let Some(battle_type) = battle_type else {
        return Ok(());
    };
    if scripted_battle_type_exists(battle_type) {
        return Ok(());
    }
    if saved_special_battle_type_builtin_routine(battle_type)
        .is_some_and(|routine| special_routine_exists(routine))
    {
        return Ok(());
    }
    Err(SpecialRoutineError::SavedPendingSpecialBattleTypeMissing {
        battle_type: battle_type.to_string(),
    })
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
    if !state.storage.party.has_space() {
        let resident = day_care_resident(state, routine, caretaker)?;
        return Ok(crate::state::DayCareInteractionState {
            caretaker: caretaker.to_string(),
            action: "withdraw".to_string(),
            success: false,
            pokemon: resident
                .pokemon
                .as_ref()
                .map(|pokemon| pokemon.species.id.clone()),
            level: resident.pokemon.as_ref().map(|pokemon| pokemon.level),
            reason: Some("party_full".to_string()),
        });
    }
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
        resident.active = false;
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
    state.day_care.compatibility_score = day_care_compatibility_score(
        state.day_care.man.pokemon.as_ref(),
        state.day_care.lady.pokemon.as_ref(),
    );
    if state.day_care.compatibility_score == 0 {
        state.day_care.steps_until_next_egg = 0;
    } else if state.day_care.steps_until_next_egg == 0 {
        state.day_care.steps_until_next_egg = 256u16
            .saturating_sub(u16::from(state.day_care.compatibility_score))
            .max(1);
    }
}

/// Advance Day Care state at the same overworld-step boundary as Crystal.
/// Experience/egg inheritance is handled when the egg is collected; this
/// routine owns the persistent counters and compatibility lifecycle.
pub fn advance_day_care_step(state: &mut GameState) {
    for resident in [&mut state.day_care.man, &mut state.day_care.lady] {
        if resident.pokemon.is_some() {
            resident.steps = resident.steps.saturating_add(1);
        }
    }
    update_day_care_compatibility(state);
    if state.day_care.compatibility_score == 0 || state.day_care.egg_present {
        return;
    }
    state.day_care.steps_since_last_egg = state.day_care.steps_since_last_egg.saturating_add(1);
    if u16::from(state.day_care.steps_since_last_egg) >= state.day_care.steps_until_next_egg {
        state.day_care.egg_present = true;
        state.day_care.steps_since_last_egg = 0;
    }
}

fn day_care_compatibility_score(first: Option<&Pokemon>, second: Option<&Pokemon>) -> u8 {
    let (Some(first), Some(second)) = (first, second) else {
        return 0;
    };
    if first.species.egg_group1 == "EGG_NONE"
        || second.species.egg_group1 == "EGG_NONE"
        || first.species.egg_group1 == "EGG_NO_EGGS"
        || second.species.egg_group1 == "EGG_NO_EGGS"
    {
        return 0;
    }
    let first_ditto = first.species.id == "DITTO";
    let second_ditto = second.species.id == "DITTO";
    if !first_ditto && !second_ditto {
        let groups_match = first.species.egg_group1 == second.species.egg_group1
            || first.species.egg_group1 == second.species.egg_group2
            || first.species.egg_group2 == second.species.egg_group1
            || first.species.egg_group2 == second.species.egg_group2;
        if !groups_match || pokemon_gender_code(first) == pokemon_gender_code(second) {
            return 0;
        }
    } else if first_ditto && second_ditto {
        return 0;
    }
    if first.dvs.defense & 0x0f == second.dvs.defense & 0x0f
        && first.dvs.special & 0x07 == second.dvs.special & 0x07
    {
        return 0;
    }
    let mut score: u8 = if first.species.id == second.species.id {
        254
    } else {
        128
    };
    if first.original_trainer_id == second.original_trainer_id
        && first.original_trainer_name == second.original_trainer_name
    {
        score = score.saturating_sub(77);
    }
    score
}

fn pokemon_gender_code(pokemon: &Pokemon) -> Option<bool> {
    match pokemon.species.gender_ratio {
        254 => Some(true),
        0 => Some(false),
        ratio => Some(pokemon.dvs.attack.saturating_mul(17) < ratio),
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
        "1" => Ok(Some(true)),
        "0" => Ok(Some(false)),
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
    let entry = happiness_data.changes.get(&change_code).ok_or_else(|| {
        SpecialRoutineError::InvalidHappinessData {
            routine: routine.to_string(),
            message: format!("missing change code {change_code}"),
        }
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
    let table = happiness_data.services.get(routine).ok_or_else(|| {
        SpecialRoutineError::InvalidHappinessData {
            routine: routine.to_string(),
            message: format!("missing service table for {routine}"),
        }
    })?;
    if table.is_empty() {
        return Err(SpecialRoutineError::InvalidHappinessData {
            routine: routine.to_string(),
            message: format!("service table {routine} has no outcomes"),
        });
    }
    Ok(table)
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
    parse_exact_u8_token(routine, &raw_value, &raw_value)
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

fn parse_exact_usize_token(
    routine: &str,
    token: &str,
    error_value: &str,
) -> Result<usize, SpecialRoutineError> {
    if !is_exact_unsigned_decimal_token(token) {
        return Err(invalid_numeric_value(routine, error_value));
    }
    token
        .parse::<usize>()
        .map_err(|_| invalid_numeric_value(routine, error_value))
}

fn parse_exact_u8_token(
    routine: &str,
    token: &str,
    error_value: &str,
) -> Result<u8, SpecialRoutineError> {
    if !is_exact_unsigned_decimal_token(token) {
        return Err(invalid_numeric_value(routine, error_value));
    }
    token
        .parse::<u8>()
        .map_err(|_| invalid_numeric_value(routine, error_value))
}

fn parse_exact_u16_token(
    routine: &str,
    token: &str,
    error_value: &str,
) -> Result<u16, SpecialRoutineError> {
    if !is_exact_unsigned_decimal_token(token) {
        return Err(invalid_numeric_value(routine, error_value));
    }
    token
        .parse::<u16>()
        .map_err(|_| invalid_numeric_value(routine, error_value))
}

fn parse_exact_i16_token(
    routine: &str,
    token: &str,
    error_value: &str,
) -> Result<i16, SpecialRoutineError> {
    if !is_exact_signed_decimal_token(token) {
        return Err(invalid_numeric_value(routine, error_value));
    }
    token
        .parse::<i16>()
        .map_err(|_| invalid_numeric_value(routine, error_value))
}

fn parse_exact_i64_token(
    routine: &str,
    token: &str,
    error_value: &str,
) -> Result<i64, SpecialRoutineError> {
    if !is_exact_signed_decimal_token(token) {
        return Err(invalid_numeric_value(routine, error_value));
    }
    token
        .parse::<i64>()
        .map_err(|_| invalid_numeric_value(routine, error_value))
}

fn is_exact_unsigned_decimal_token(token: &str) -> bool {
    !token.is_empty() && token.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_exact_signed_decimal_token(token: &str) -> bool {
    if let Some(digits) = token.strip_prefix('-') {
        return is_exact_unsigned_decimal_token(digits);
    }
    is_exact_unsigned_decimal_token(token)
}

fn invalid_numeric_value(routine: &str, value: &str) -> SpecialRoutineError {
    SpecialRoutineError::InvalidNumericValue {
        routine: routine.to_string(),
        value: value.to_string(),
    }
}

fn required_usize_script_variable(
    state: &GameState,
    routine: &str,
    variable: &str,
) -> Result<usize, SpecialRoutineError> {
    let raw_value = required_string_script_variable(state, routine, variable)?;
    parse_exact_usize_token(routine, &raw_value, &raw_value)
}

fn optional_usize_script_variable(
    state: &GameState,
    routine: &str,
    variable: &str,
) -> Result<Option<usize>, SpecialRoutineError> {
    let Some(raw_value) = state.script_runtime.variables.get(variable).cloned() else {
        return Ok(None);
    };
    parse_exact_usize_token(routine, &raw_value, &raw_value).map(Some)
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
    else {
        return Err(SpecialRoutineError::MissingScriptValue {
            routine: routine.to_string(),
            variable: "_selected_party_index".to_string(),
        });
    };
    parse_exact_usize_token(routine, &raw_value, &raw_value)
}

fn optional_u8_script_variable(
    state: &GameState,
    routine: &str,
    variable: &str,
) -> Result<Option<u8>, SpecialRoutineError> {
    let Some(raw_value) = state.script_runtime.variables.get(variable).cloned() else {
        return Ok(None);
    };
    parse_exact_u8_token(routine, &raw_value, &raw_value).map(Some)
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
    parse_exact_u16_token(routine, &raw_value, &raw_value).map(Some)
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
    parse_exact_i16_token(routine, &raw_value, &raw_value).map(Some)
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
    parse_exact_i64_token(routine, &raw_value, &raw_value)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        BaseStats, Dv, LearnedMove, PcBox, PokemonSpecies, Trainer, TrainerPartyPokemon,
        growth_rate, item_pocket, pokemon_type,
    };
    use crate::systems::experience::{GrowthRateCatalog, crystal_growth_rate_catalog_for_tests};
    use crate::systems::phone::PhoneContactRecord;
    use crate::world::encounters::WildEncounterTable;
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
    static EMPTY_TEST_ROAMERS: LazyLock<RoamingPokemonDefinitions> = LazyLock::new(BTreeMap::new);
    static EMPTY_TEST_BUENA_PASSWORD_CATEGORIES: LazyLock<BuenaPasswordCategories> =
        LazyLock::new(BuenaPasswordCategories::default);
    static EMPTY_TEST_BUENA_PRIZES: LazyLock<BuenaPrizeDefinitions> = LazyLock::new(BTreeMap::new);
    static EMPTY_TEST_KURT_APRICORN_RECIPES: LazyLock<KurtApricornRecipes> =
        LazyLock::new(BTreeMap::new);
    static EMPTY_TEST_DRATINI_MOVE_SETS: LazyLock<DratiniMoveSets> = LazyLock::new(BTreeMap::new);
    static EMPTY_TEST_MAGIKARP_LENGTHS: LazyLock<Vec<MagikarpLengthEntry>> =
        LazyLock::new(Vec::new);
    static EMPTY_TEST_TRAINERS: LazyLock<TrainerCatalog> = LazyLock::new(TrainerCatalog::default);
    static EMPTY_TEST_PHONE_CONTACTS: LazyLock<PhoneContactCatalog> =
        LazyLock::new(PhoneContactCatalog::default);
    static EMPTY_TEST_WILD_ENCOUNTERS: LazyLock<BTreeMap<String, WildEncounterData>> =
        LazyLock::new(BTreeMap::new);
    const MODPACK_SPECIAL_ROUTINES_JSON: &str = include_str!(
        "../../../../../apps/web/assets/data/content-packs/core-modular/special_routines/routines.json"
    );

    #[test]
    fn special_routine_registry_is_exact_and_covers_core_modpack_declarations() {
        assert!(is_known_special_routine("HealParty"));
        assert!(is_known_special_routine("UnusedDummySpecial"));
        assert!(!is_known_special_routine("healparty"));
        assert!(!is_known_special_routine("MODPACK_ONLY_ROUTINE"));

        let routines: BTreeMap<String, serde_json::Value> =
            serde_json::from_str(MODPACK_SPECIAL_ROUTINES_JSON)
                .expect("core special routines json");
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
    fn roaming_pokemon_definition_issues_validate_exact_species_and_level() {
        let species = BTreeSet::from(["RAIKOU".to_string()]);
        let definitions = BTreeMap::from([
            (
                String::new(),
                RoamingPokemonDefinition {
                    level: 0,
                    map_group: 1,
                    map_number: 1,
                },
            ),
            (
                "RAI KOU".to_string(),
                RoamingPokemonDefinition {
                    level: 40,
                    map_group: 1,
                    map_number: 2,
                },
            ),
            (
                "raikou".to_string(),
                RoamingPokemonDefinition {
                    level: 40,
                    map_group: 1,
                    map_number: 3,
                },
            ),
            (
                "RAIKOU".to_string(),
                RoamingPokemonDefinition {
                    level: 40,
                    map_group: 1,
                    map_number: 4,
                },
            ),
        ]);

        assert_eq!(
            roaming_pokemon_definition_issues(&definitions, &species),
            vec![
                RoamingPokemonDefinitionIssue::EmptySpecies {
                    species: String::new(),
                },
                RoamingPokemonDefinitionIssue::InvalidLevel {
                    species: String::new(),
                },
                RoamingPokemonDefinitionIssue::InvalidSpecies {
                    species: "RAI KOU".to_string(),
                },
                RoamingPokemonDefinitionIssue::UnknownSpecies {
                    species: "raikou".to_string(),
                },
            ]
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
        roaming_pokemon: &'a RoamingPokemonDefinitions,
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
            phone_contacts: &EMPTY_TEST_PHONE_CONTACTS,
            wild_encounters: &EMPTY_TEST_WILD_ENCOUNTERS,
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
            .register_capture(pokemon("CHIKORITA"))
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
            .register_capture(pokemon("CHIKORITA"))
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
            tile: crate::world::map::TilePosition::new(68, 20),
            facing: crate::world::map::Direction::Down,
            mode: crate::world::movement::MovementMode::Normal,
        };

        let awake =
            apply_special_routine(&mut state, &moves(), "SnorlaxAwake").expect("snorlax awake");

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

        let day =
            apply_special_routine(&mut state, &moves(), "SetDayOfWeek").expect("set day of week");

        assert_eq!(day.effect, SpecialRoutineEffect::SetDayOfWeek { day: 5 });
        assert_eq!(state.time.current_day, 5);
        assert_eq!(state.time.day_of_week, 5);
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
        let aerodactyl =
            apply_special_routine_with_context(&mut state, context, "AerodactylChamber")
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
    fn unown_puzzle_runs_exact_state_machine_instead_of_visual_command() {
        let mut state = GameState::default();
        state.rng_seed = 1;
        state
            .script_runtime
            .variables
            .insert("_value".to_string(), "2".to_string());

        let opened =
            apply_special_routine(&mut state, &moves(), "UnownPuzzle").expect("open puzzle");

        let SpecialRoutineEffect::UnownPuzzle {
            puzzle_id,
            solved,
            moves: puzzle_moves,
            layout,
            holding_piece,
            rng_seed_after,
        } = opened.effect
        else {
            panic!("expected UnownPuzzle effect");
        };
        assert_eq!(puzzle_id, "AERODACTYL");
        assert!(!solved);
        assert_eq!(puzzle_moves, 0);
        assert_eq!(holding_piece, None);
        assert_eq!(rng_seed_after, state.rng_seed);
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

        let pickup =
            apply_special_routine(&mut state, &moves(), "UnownPuzzle").expect("pickup piece");

        assert!(matches!(
            pickup.effect,
            SpecialRoutineEffect::UnownPuzzle {
                puzzle_id,
                solved: false,
                moves: 0,
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

        let place =
            apply_special_routine(&mut state, &moves(), "UnownPuzzle").expect("place piece");

        assert!(matches!(
            place.effect,
            SpecialRoutineEffect::UnownPuzzle {
                puzzle_id,
                solved: true,
                moves: 1,
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
        assert_eq!(
            state
                .script_runtime
                .variables
                .get("unown_moves_KABUTO")
                .map(String::as_str),
            Some("1")
        );
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

        let error = apply_special_routine(&mut state, &moves(), "UnownPuzzle")
            .expect_err("duplicate piece must fail");

        assert!(matches!(
            error,
            SpecialRoutineError::InvalidUnownPuzzleState { routine, message }
                if routine == "UnownPuzzle"
                    && message == "piece 1 appears more than once in the puzzle state"
        ));
    }

    #[test]
    fn toggle_decorations_visibility_hides_unset_room_decoration_objects() {
        let mut state = GameState::default();
        state
            .script_runtime
            .variable_sprites
            .insert("SPRITE_CONSOLE".to_string(), "SPRITE_FAMICOM".to_string());

        let outcome = apply_special_routine(&mut state, &moves(), "ToggleDecorationsVisibility")
            .expect("toggle decorations visibility");

        assert_eq!(
            outcome.effect,
            SpecialRoutineEffect::RuntimeVisualCommand {
                kind: ScriptGraphicsRuntimeKind::ToggleDecorationsVisibility
            }
        );
        for sprite in [
            "SPRITE_CONSOLE",
            "SPRITE_DOLL_1",
            "SPRITE_DOLL_2",
            "SPRITE_BIG_DOLL",
        ] {
            assert!(!state.script_runtime.variable_sprites.contains_key(sprite));
        }
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
            minimum_level_group: 0,
            maximum_level_group: 10,
            level_group_size: 0,
            party_count_failure_text: String::new(),
            duplicate_species_failure_text: "DuplicateSpeciesText".to_string(),
            duplicate_held_item_failure_text: "Duplicate HeldItemText".to_string(),
            egg_failure_text: "EggText".to_string(),
        };
        let species_ids = BTreeSet::from(["MEW".to_string()]);

        assert_eq!(
            battle_tower_rules_issues(&rules, &species_ids),
            vec![
                BattleTowerRulesIssue::MissingRequiredPartyCount,
                BattleTowerRulesIssue::MissingChallengeStreakLength,
                BattleTowerRulesIssue::MissingLevelGroupSize,
                BattleTowerRulesIssue::InvalidLevelGroupRange,
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
        state.storage.register_capture(mon).expect("store mon");
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
        let rng_seed_before = state.rng_seed;
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
        assert_eq!(state.rng_seed, rng_seed_before);
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

        let move_catalog = moves();
        let coin_case = item_data("COIN_CASE");
        let item_catalog = BTreeMap::from([("COIN_CASE".to_string(), coin_case.clone())]);
        state.bag.add_item(&coin_case, 1).expect("add coin case");
        let context = full_context(
            &move_catalog,
            &EMPTY_TEST_SPECIES,
            &EMPTY_TEST_LEARNSETS,
            &item_catalog,
        );

        state.rng_seed = 1;
        state
            .script_runtime
            .variables
            .insert("slot_bet".to_string(), "3".to_string());
        let slot =
            apply_special_routine_with_context(&mut state, context, "SlotMachine").expect("slot");
        let SpecialRoutineEffect::SlotMachine {
            coins_before,
            bet,
            payout,
            coins,
            rng_seed_after,
            ..
        } = slot.effect
        else {
            panic!("slot special returned non-slot effect");
        };
        assert_eq!(coins_before, 99);
        assert_eq!(bet, 3);
        assert_eq!(coins, 99 - 3 + payout);
        assert_eq!(state.coins, coins);
        assert_eq!(state.rng_seed, rng_seed_after);
        assert_eq!(
            state.script_runtime.script_value.as_deref(),
            Some(coins.to_string().as_str())
        );

        let card =
            apply_special_routine_with_context(&mut state, context, "CardFlip").expect("card");
        let SpecialRoutineEffect::CardFlip {
            coins_before,
            card_index,
            card_name,
            payout,
            coins,
            rng_seed_after,
        } = card.effect
        else {
            panic!("card flip special returned non-card effect");
        };
        assert_eq!(coins_before, state.coins + 3 - payout);
        assert!(card_index < 24);
        assert!(!card_name.is_empty());
        assert_eq!(coins, coins_before - 3 + payout);
        assert_eq!(state.coins, coins);
        assert_eq!(state.rng_seed, rng_seed_after);

        state.script_runtime.variables.insert(
            "memory_board".to_string(),
            "ODDISH,ODDISH,POLIWAG,POLIWAG,PIKACHU,PIKACHU,JIGGLYPUFF,JIGGLYPUFF,RATTATA,RATTATA,VOLTORB,VOLTORB,DITTO,DITTO,ELECTABUZZ,ELECTABUZZ".to_string(),
        );
        state
            .script_runtime
            .variables
            .insert("memory_first".to_string(), "0".to_string());
        state
            .script_runtime
            .variables
            .insert("memory_second".to_string(), "1".to_string());
        let memory = apply_special_routine_with_context(&mut state, context, "UnusedMemoryGame")
            .expect("memory game");
        let SpecialRoutineEffect::UnusedMemoryGame {
            matched,
            symbol,
            first_index,
            second_index,
            coins,
            rng_seed_after,
        } = memory.effect
        else {
            panic!("memory game returned non-memory effect");
        };
        assert!(matched);
        assert_eq!(symbol.as_deref(), Some("ODDISH"));
        assert_eq!((first_index, second_index), (0, 1));
        assert_eq!(coins, state.coins);
        assert_eq!(state.rng_seed, rng_seed_after);
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));
        assert_eq!(
            state
                .script_runtime
                .variables
                .get("memory_revealed")
                .map(String::as_str),
            Some("1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0")
        );
        assert_eq!(state.script_runtime.active_menu, None);

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

        let cancel = apply_special_routine(&mut state, &moves(), "Menu_ChallengeExplanationCancel")
            .expect("cancel challenge explanation");
        assert_eq!(
            cancel.effect,
            SpecialRoutineEffect::BattleTowerChallengeExplanationCancel
        );
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));
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
        let no_coins_outcome =
            apply_special_routine_with_context(&mut no_coins, context, "SlotMachine")
                .expect("no coins is handled by script text");
        assert_eq!(
            no_coins_outcome.effect,
            SpecialRoutineEffect::GameCornerGameUnavailable {
                game: "SlotMachine".to_string(),
                reason: GameCornerUnavailableReason::NoCoins,
            }
        );
        assert_eq!(no_coins.script_runtime.active_menu, None);

        let mut no_coin_case = GameState::default();
        no_coin_case.coins = 10;
        let missing_case =
            apply_special_routine_with_context(&mut no_coin_case, context, "CardFlip")
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
        state.rng_seed = 1;
        state.script_runtime.variables.insert(
            "VAR_CALLERID".to_string(),
            "PHONE_BIRDKEEPER_VANCE".to_string(),
        );
        let outcome = apply_special_routine_with_context(&mut state, context, "RandomPhoneWildMon")
            .expect("random phone wild mon");

        assert_eq!(
            outcome.effect,
            SpecialRoutineEffect::RandomPhoneWildMon {
                contact_id: "PHONE_BIRDKEEPER_VANCE".to_string(),
                map_name: "ROUTE_44".to_string(),
                time_of_day: TimeOfDay::Day,
                species: "PIDGEY".to_string(),
                rng_seed_after: 58_598,
            }
        );
        assert_eq!(
            state
                .script_runtime
                .named_buffers
                .get("STRING_BUFFER_4")
                .map(String::as_str),
            Some("PIDGEY")
        );
        assert_eq!(
            state
                .script_runtime
                .variables
                .get("wNamedObjectIndex")
                .map(String::as_str),
            Some("16")
        );
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
        state.rng_seed = 1;
        state
            .script_runtime
            .variables
            .insert("VAR_CALLERID".to_string(), "PHONE_HIKER_PARRY".to_string());
        let outcome =
            apply_special_routine_with_context(&mut state, context, "RandomUnseenWildMon")
                .expect("random unseen wild mon");

        assert_eq!(
            outcome.effect,
            SpecialRoutineEffect::RandomUnseenWildMon {
                contact_id: "PHONE_HIKER_PARRY".to_string(),
                map_name: "ROUTE_45".to_string(),
                species: Some("SKARMORY".to_string()),
                already_seen: false,
                script_value: 0,
                rng_seed_after: 127_215,
            }
        );
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));
        assert_eq!(
            state
                .script_runtime
                .named_buffers
                .get("STRING_BUFFER_1")
                .map(String::as_str),
            Some("SKARMORY")
        );
    }

    #[test]
    fn random_phone_wild_mon_requires_exact_caller_context() {
        let move_catalog = moves();
        let species_catalog = species_catalog(&[("PIDGEY", 16)]);
        let state = &mut GameState::default();
        let error = apply_special_routine_with_context(
            state,
            full_context(
                &move_catalog,
                &species_catalog,
                &EMPTY_TEST_LEARNSETS,
                &EMPTY_TEST_ITEMS,
            ),
            "RandomPhoneWildMon",
        )
        .expect_err("caller id is required");

        assert_eq!(
            error,
            SpecialRoutineError::MissingCallerId {
                routine: "RandomPhoneWildMon".to_string()
            }
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
        state.rng_seed = 1;
        state.script_runtime.variables.insert(
            "VAR_CALLERID".to_string(),
            "PHONE_BIRDKEEPER_VANCE".to_string(),
        );
        let outcome = apply_special_routine_with_context(&mut state, context, "RandomPhoneMon")
            .expect("random phone trainer mon");

        assert_eq!(
            outcome.effect,
            SpecialRoutineEffect::RandomPhoneMon {
                contact_id: "PHONE_BIRDKEEPER_VANCE".to_string(),
                trainer_id: "VANCE1".to_string(),
                species: "PIDGEY".to_string(),
                party_index: 0,
                rng_seed_after: 58_598,
            }
        );
        assert_eq!(
            state
                .script_runtime
                .named_buffers
                .get("STRING_BUFFER_4")
                .map(String::as_str),
            Some("PIDGEY")
        );
        assert_eq!(
            state
                .script_runtime
                .variables
                .get("wNamedObjectIndex")
                .map(String::as_str),
            Some("16")
        );
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

            let outcome =
                apply_special_routine(&mut state, &moves(), routine).expect("noop special");

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
            .register_capture(selected)
            .expect("store selected");
        state.storage.register_capture(other).expect("store other");
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

        let usable = apply_special_routine(&mut state, &moves(), "Function11ba38")
            .expect("another usable mon");

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
            "Function170114",
            "Function1704e1",
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
        state
            .script_runtime
            .variables
            .insert("_selection_cancelled".to_string(), "0".to_string());
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
        let roaming_definitions = BTreeMap::from([
            (
                "RAIKOU".to_string(),
                RoamingPokemonDefinition {
                    level: 40,
                    map_group: 2,
                    map_number: 5,
                },
            ),
            (
                "ENTEI".to_string(),
                RoamingPokemonDefinition {
                    level: 40,
                    map_group: 10,
                    map_number: 4,
                },
            ),
        ]);
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
                    species: "ENTEI".to_string(),
                    level: 40,
                    map_group: 10,
                    map_number: 4,
                    hp: 0,
                    dvs: 0
                },
                RoamingPokemonState {
                    species: "RAIKOU".to_string(),
                    level: 40,
                    map_group: 2,
                    map_number: 5,
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
        let buena_password_categories = BuenaPasswordCategories {
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

        let mut padded_guess = state.clone();
        padded_guess
            .script_runtime
            .variables
            .insert("BUENA_PASSWORD".to_string(), " TOTODILE".to_string());
        let before_padded_guess = padded_guess.clone();
        let padded_guess_error = apply_special_routine_with_context(
            &mut padded_guess,
            full_context_with_buena_password_categories(
                &moves,
                &species,
                &learnsets,
                &items,
                &buena_password_categories,
            ),
            "BuenasPassword",
        )
        .expect_err("padded Buena password guess rejected");
        assert_eq!(
            padded_guess_error,
            SpecialRoutineError::InvalidBuenaPasswordGuess {
                routine: "BuenasPassword".to_string(),
                guess: " TOTODILE".to_string(),
            }
        );
        assert_eq!(padded_guess, before_padded_guess);

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
        let missing_shuckie_cancel = apply_special_routine_with_context(
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
        assert_eq!(shuckie_state, before_shuckie_selection);

        let mut magikarp_state = GameState::default();
        magikarp_state
            .storage
            .register_capture(pokemon("MAGIKARP"))
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
        assert!(!state.day_care.man.active);
        assert_eq!(state.day_care.man.initial_experience, 0);
        assert_eq!(state.day_care.man.initial_level, 0);
        state
            .validate_saved_state()
            .expect("Day Care withdraw leaves save-valid state");
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
    fn day_care_withdraw_from_full_party_preserves_resident() {
        let mut state = GameState::default();
        for index in 0..crate::models::PARTY_SIZE {
            state
                .storage
                .register_capture(pokemon(if index % 2 == 0 {
                    "CHIKORITA"
                } else {
                    "CYNDAQUIL"
                }))
                .expect("store full-party Pokemon");
        }
        state.sync_party_from_storage();
        state.day_care.man.pokemon = Some(pokemon("TOTODILE"));
        state.day_care.man.active = true;
        state
            .script_runtime
            .variables
            .insert("_day_care_action".to_string(), "withdraw".to_string());

        let before = state.day_care.man.clone();
        let outcome = apply_special_routine(&mut state, &moves(), "DayCareMan")
            .expect("full-party withdrawal should be a handled refusal");

        assert_eq!(
            outcome.effect,
            SpecialRoutineEffect::DayCareInteraction {
                caretaker: "man".to_string(),
                action: "withdraw".to_string(),
                success: false,
                pokemon: Some("TOTODILE".to_string()),
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
        update_day_care_compatibility(&mut state);
        assert!(state.day_care.compatibility_score > 0);

        state.day_care.lady.pokemon.as_mut().unwrap().dvs.attack = 0;
        update_day_care_compatibility(&mut state);
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
        update_day_care_compatibility(&mut state);
        assert_eq!(state.day_care.compatibility_score, 0);
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

        for _ in 0..80 {
            advance_day_care_step(&mut state);
        }
        assert_eq!(state.day_care.man.steps, 80);
        assert_eq!(state.day_care.lady.steps, 80);
        assert!(state.day_care.egg_present);
        assert_eq!(state.day_care.steps_since_last_egg, 0);
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
        state.time.day_of_week = state.time.current_day % 7;
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
        assert_eq!(
            state
                .script_runtime
                .variables
                .get("_battle_tower_room_menu_cancelled")
                .map(String::as_str),
            Some("FALSE")
        );
        state
            .validate_saved_state()
            .expect("Battle Tower room menu leaves save-valid state");
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
                .register_capture(pokemon("CHIKORITA"))
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
        let password_error =
            apply_special_routine(&mut padded_password, &moves(), "Function1011f1")
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
        let selection_error =
            apply_special_routine(&mut selection, &moves(), "Mobile_SelectThreeMons")
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
                spawn_point(0, "PlayersHouse2F", 24, 7, 4, 4),
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

        let mut aliased = GameState::default();
        aliased
            .script_runtime
            .variables
            .insert("_last_spawn_map_group".to_string(), "23".to_string());
        aliased
            .script_runtime
            .variables
            .insert("_last_spawn_map_number".to_string(), "9".to_string());
        let before_aliased = aliased.clone();
        let error = apply_special_routine_with_context(
            &mut aliased,
            spawn_context(&moves(), &spawns),
            "WarpToSpawnPoint",
        )
        .expect_err("underscored spawn aliases are not runtime inputs");
        assert!(matches!(
            error,
            SpecialRoutineError::MissingScriptValue { routine, variable }
                if routine == "WarpToSpawnPoint" && variable == "wLastSpawnMapGroup"
        ));
        assert_eq!(aliased, before_aliased);

        let mut missing_group = GameState::default();
        missing_group
            .script_runtime
            .variables
            .insert("wLastSpawnMapNumber".to_string(), "9".to_string());
        let before_missing_group = missing_group.clone();
        let error = apply_special_routine_with_context(
            &mut missing_group,
            spawn_context(&moves(), &spawns),
            "WarpToSpawnPoint",
        )
        .expect_err("spawn group is required without saved spawn id");
        assert!(matches!(
            error,
            SpecialRoutineError::MissingScriptValue { routine, variable }
                if routine == "WarpToSpawnPoint" && variable == "wLastSpawnMapGroup"
        ));
        assert_eq!(missing_group, before_missing_group);

        let mut partial_with_spawn_id = GameState::default();
        partial_with_spawn_id.last_spawn_identifier = Some(14);
        partial_with_spawn_id
            .script_runtime
            .variables
            .insert("wLastSpawnMapNumber".to_string(), "9".to_string());
        let before_partial_with_spawn_id = partial_with_spawn_id.clone();
        let error = apply_special_routine_with_context(
            &mut partial_with_spawn_id,
            spawn_context(&moves(), &spawns),
            "WarpToSpawnPoint",
        )
        .expect_err("partial spawn pair is not completed from saved spawn id");
        assert!(matches!(
            error,
            SpecialRoutineError::MissingScriptValue { routine, variable }
                if routine == "WarpToSpawnPoint" && variable == "wLastSpawnMapGroup"
        ));
        assert_eq!(partial_with_spawn_id, before_partial_with_spawn_id);

        let mut missing_map = GameState::default();
        missing_map
            .script_runtime
            .variables
            .insert("wLastSpawnMapGroup".to_string(), "23".to_string());
        let before_missing_map = missing_map.clone();
        let error = apply_special_routine_with_context(
            &mut missing_map,
            spawn_context(&moves(), &spawns),
            "WarpToSpawnPoint",
        )
        .expect_err("spawn map is required with spawn group");
        assert!(matches!(
            error,
            SpecialRoutineError::MissingScriptValue { routine, variable }
                if routine == "WarpToSpawnPoint" && variable == "wLastSpawnMapNumber"
        ));
        assert_eq!(missing_map, before_missing_map);

        let mut partial_map_with_spawn_id = GameState::default();
        partial_map_with_spawn_id.last_spawn_identifier = Some(14);
        partial_map_with_spawn_id
            .script_runtime
            .variables
            .insert("wLastSpawnMapGroup".to_string(), "23".to_string());
        let before_partial_map_with_spawn_id = partial_map_with_spawn_id.clone();
        let error = apply_special_routine_with_context(
            &mut partial_map_with_spawn_id,
            spawn_context(&moves(), &spawns),
            "WarpToSpawnPoint",
        )
        .expect_err("partial spawn pair is not completed from saved spawn id");
        assert!(matches!(
            error,
            SpecialRoutineError::MissingScriptValue { routine, variable }
                if routine == "WarpToSpawnPoint" && variable == "wLastSpawnMapNumber"
        ));
        assert_eq!(partial_map_with_spawn_id, before_partial_map_with_spawn_id);
    }

    #[test]
    fn warp_to_spawn_point_uses_saved_spawn_id_or_errors_without_pack_data() {
        let mut state = GameState::default();
        state.last_spawn_identifier = Some(21);
        let spawns = BTreeMap::from([(
            "21".to_string(),
            spawn_point(21, "IndigoPlateauPokecenter1F", 11, 4, 10, 8),
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
                tile: TilePosition::new(10, 8)
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
        state
            .storage
            .register_capture(pokemon("PERSIAN"))
            .expect("store player party mon");
        state.sync_party_from_storage();
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
}
