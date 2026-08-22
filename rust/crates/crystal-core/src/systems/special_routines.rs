use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::battle::start::first_available_battle_party_index;
use crate::models::pokemon::{CaughtData, StatExperience};
use crate::models::{
    Dv, Item, LearnedMove, MAX_BOX_MONS, MAX_PC_BOXES, Move, Party, Pokemon, PokemonSpecies,
    TrainerCatalog, calculate_stats, create_pokemon_from_known_dvs,
};
use crate::random::{CrystalRandom, CrystalRandomState, DividerSource};
use crate::state::{
    BattleMemory, BattleTowerState, BuenasPasswordState, EventFlagError, GameState,
    LinkSerialConnectionStatus, MagikarpRecordState, MobileBattleTowerRecord, RoamingPokemonState,
    ScriptAudioRuntimeEvent, ScriptAudioRuntimeKind, ScriptFadeColor, ScriptFadeDirection,
    ScriptGraphicsRuntimeEvent, ScriptGraphicsRuntimeKind, ScriptMoneyRuntimeEvent,
    ScriptMoneyRuntimeKind, ScriptMusicFade, ScriptScreenFade,
};
use crate::systems::experience::{ExperienceError, GrowthRateCatalog, calculate_experience};
use crate::systems::learnsets::SpeciesLearnsets;
use crate::systems::phone::PhoneContactCatalog;
use crate::systems::time::ClockTime;
use crate::world::encounters::{TimeOfDay, WildEncounter, WildEncounterData};
use crate::world::map::{METATILE_WIDTH, TilePosition};

fn pokemon_is_egg(pokemon: &Pokemon) -> bool {
    pokemon.is_egg || pokemon.status.as_deref() == Some("EGG") || pokemon.species.id == "EGG"
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecialRoutineOutcome {
    pub routine: String,
    pub effect: SpecialRoutineEffect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BugContestPlacement {
    pub place: u8,
    pub winner_id: u8,
    pub trainer_name: String,
    pub species: String,
    pub score: u16,
    pub player: bool,
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
        random_state_after: CrystalRandomState,
    },
    RandomPhoneWildMon {
        contact_id: String,
        map_name: String,
        time_of_day: TimeOfDay,
        species: String,
        random_state_after: CrystalRandomState,
    },
    RandomPhoneMon {
        contact_id: String,
        trainer_id: String,
        species: String,
        party_index: usize,
        random_state_after: CrystalRandomState,
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
        random_state_after: CrystalRandomState,
    },
    CheckLuckyNumberShowFlag {
        flag: bool,
    },
    ResetLuckyNumberShowFlag {
        lucky_number: u16,
        lucky_number_day: u8,
        random_state_after: CrystalRandomState,
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
        letters: Vec<u8>,
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
        random_state_after: CrystalRandomState,
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
        roamers: [RoamingPokemonState; ROAMING_POKEMON_SLOT_COUNT],
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
        options: Vec<String>,
        correct: String,
        guess: Option<String>,
        matched: bool,
        random_state_after: CrystalRandomState,
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
        random_state_after: CrystalRandomState,
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
        placements: Vec<BugContestPlacement>,
        random_state_after: CrystalRandomState,
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
        failures: Vec<String>,
    },
    BattleTowerRoomMenu {
        level_groups: Vec<u8>,
        selection: Option<u8>,
        rejection: Option<BattleTowerRoomMenuRejection>,
        cancelled: bool,
    },
    BattleTowerBattle {
        result_code: u8,
        beaten_trainers: u8,
        challenge_state: u8,
    },
    BattleTowerBattleStarted,
    BattleTowerMobileError,
    LoadOpponentTrainerAndPokemonWithOtSprite {
        trainer_id: String,
        trainer_class: String,
        trainer_name: String,
        party_size: usize,
        sprite_constant: String,
        target_object: String,
        random_state_after: CrystalRandomState,
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
        safari_game_was_active: bool,
        bug_contest_timer_was_active: bool,
    },
    GiveOddEgg {
        table_index: usize,
        species: String,
        party_slot: usize,
        shiny: bool,
        random_state_after: CrystalRandomState,
    },
    BankOfMom {
        initialized: bool,
        money: u32,
        moms_money: u32,
    },
    SlotMachineStarted {
        coins_before: u16,
        bet: u8,
        bias: Option<String>,
        offsets: [usize; 3],
        windows: [[String; 3]; 3],
        coins: u16,
        random_state_after: CrystalRandomState,
    },
    SlotMachineReelStopped {
        reel: u8,
        mode: String,
        animation_start_offset: usize,
        animation_count: u8,
        offsets: [usize; 3],
        windows: [[String; 3]; 3],
        coins: u16,
        random_state_after: CrystalRandomState,
    },
    SlotMachineResult {
        payout: u16,
        matched_symbol: Option<String>,
        winning_lines: Vec<String>,
        coins: u16,
        random_state_after: CrystalRandomState,
    },
    SlotMachinePayout {
        coins_before: u16,
        payout_remaining: u16,
        coins: u16,
        random_state_after: CrystalRandomState,
    },
    CardFlipStarted {
        coins_before: u16,
        deck: Vec<String>,
        revealed: Vec<bool>,
        coins: u16,
        random_state_after: CrystalRandomState,
    },
    CardFlipShuffled {
        deck: Vec<String>,
        revealed: Vec<bool>,
        coins: u16,
        random_state_after: CrystalRandomState,
    },
    CardFlipRevealed {
        coins_before: u16,
        card_index: usize,
        card_name: String,
        card_level: u8,
        payout: u16,
        deck: Vec<String>,
        revealed: Vec<bool>,
        coins: u16,
        random_state_after: CrystalRandomState,
    },
    CardFlipPayout {
        coins_before: u16,
        coins: u16,
        random_state_after: CrystalRandomState,
    },
    UnownPuzzle {
        puzzle_id: String,
        solved: bool,
        moves: u16,
        layout: Vec<Vec<u8>>,
        holding_piece: Option<u8>,
        random_state_after: CrystalRandomState,
    },
    UnusedMemoryGame {
        matched: bool,
        symbol: Option<String>,
        first_index: usize,
        second_index: usize,
        coins: u16,
        random_state_after: CrystalRandomState,
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
        enabled: bool,
    },
    PhotoStudio {
        party_slot: Option<usize>,
        species: Option<String>,
    },
    BattleTowerChallengeExplanationCancel {
        english: bool,
        selection: Option<u8>,
    },
    DisplayLinkRecord {
        wins: u16,
        losses: u16,
        draws: u16,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum BattleTowerRoomMenuRejection {
    PartyMonTopsThisLevel,
    UberRestriction { species: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
#[serde(deny_unknown_fields)]
pub enum SpecialRoutineError {
    #[error("unsupported exact special routine {routine}")]
    UnsupportedRoutine { routine: String },
    #[error("special routine {routine} requires an authoritative divider source")]
    MissingDividerSource { routine: String },
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RandomSpecialRoutineError<E> {
    Routine(SpecialRoutineError),
    Divider(E),
}

impl<E> From<SpecialRoutineError> for RandomSpecialRoutineError<E> {
    fn from(error: SpecialRoutineError) -> Self {
        Self::Routine(error)
    }
}

impl<E: std::fmt::Display> std::fmt::Display for RandomSpecialRoutineError<E> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Routine(error) => error.fmt(formatter),
            Self::Divider(error) => write!(formatter, "special routine divider source: {error}"),
        }
    }
}

impl<E: std::fmt::Debug + std::fmt::Display> std::error::Error for RandomSpecialRoutineError<E> {}

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
    pub roaming_pokemon: &'a RoamingPokemonCatalog,
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
pub struct BugContestEncounterEntry {
    pub weight: u8,
    pub species: String,
    pub min_level: u8,
    pub max_level: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BugContestConfig {
    pub park_balls: u8,
    pub timer_minutes: u8,
    pub timer_seconds: u8,
    pub selected_contestant_count: usize,
    pub contestant_flags: Vec<String>,
    pub encounters: Vec<BugContestEncounterEntry>,
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
            encounters: Vec<BugContestEncounterEntry>,
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
        validate_bug_contest_encounters(&raw.encounters).map_err(serde::de::Error::custom)?;
        Ok(Self {
            park_balls: raw.park_balls,
            timer_minutes: raw.timer_minutes,
            timer_seconds: raw.timer_seconds,
            selected_contestant_count: raw.selected_contestant_count,
            contestant_flags: raw.contestant_flags,
            encounters: raw.encounters,
        })
    }
}

impl<'de> Deserialize<'de> for BugContestEncounterEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct RawEntry {
            weight: u8,
            species: String,
            min_level: u8,
            max_level: u8,
        }

        let raw = RawEntry::deserialize(deserializer)?;
        require_special_token("bug contest encounter species", &raw.species)
            .map_err(serde::de::Error::custom)?;
        if raw.min_level == 0 || raw.min_level > raw.max_level || raw.max_level > 100 {
            return Err(serde::de::Error::custom(format!(
                "bug contest encounter {} has invalid level range {}..={}",
                raw.species, raw.min_level, raw.max_level
            )));
        }
        Ok(Self {
            weight: raw.weight,
            species: raw.species,
            min_level: raw.min_level,
            max_level: raw.max_level,
        })
    }
}

fn validate_bug_contest_encounters(encounters: &[BugContestEncounterEntry]) -> Result<(), String> {
    if encounters.len() != 11 {
        return Err(format!(
            "bug contest encounters must contain 10 weighted rows plus one sentinel, found {} rows",
            encounters.len()
        ));
    }
    let Some((sentinel, weighted)) = encounters.split_last() else {
        return Err("bug contest encounters must include the final -1 sentinel row".to_string());
    };
    if sentinel.weight != u8::MAX {
        return Err(format!(
            "bug contest final encounter weight must be the normalized -1 sentinel 255, found {}",
            sentinel.weight
        ));
    }
    let mut total = 0u16;
    for (index, entry) in weighted.iter().enumerate() {
        if entry.weight == 0 || entry.weight == u8::MAX {
            return Err(format!(
                "bug contest encounter weight at index {index} must be 1..=254, found {}",
                entry.weight
            ));
        }
        total += u16::from(entry.weight);
    }
    if total != 100 {
        return Err(format!(
            "bug contest ordinary encounter weights must total 100, found {total}"
        ));
    }
    Ok(())
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
    InvalidEncounterTable {
        message: String,
    },
    InvalidEncounterSpecies {
        index: usize,
        species: String,
    },
    UnsupportedEncounterSpecies {
        index: usize,
        species: String,
    },
    InvalidEncounterLevelRange {
        index: usize,
        min_level: u8,
        max_level: u8,
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
    if let Err(message) = validate_bug_contest_encounters(&config.encounters) {
        issues.push(BugContestConfigIssue::InvalidEncounterTable { message });
    }
    for (index, encounter) in config.encounters.iter().enumerate() {
        if !is_exact_nonempty_special_token(&encounter.species) {
            issues.push(BugContestConfigIssue::InvalidEncounterSpecies {
                index,
                species: encounter.species.clone(),
            });
        }
        if encounter.species == "UNOWN" {
            issues.push(BugContestConfigIssue::UnsupportedEncounterSpecies {
                index,
                species: encounter.species.clone(),
            });
        }
        if encounter.min_level == 0
            || encounter.min_level > encounter.max_level
            || encounter.max_level > 100
        {
            issues.push(BugContestConfigIssue::InvalidEncounterLevelRange {
                index,
                min_level: encounter.min_level,
                max_level: encounter.max_level,
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
    pub reward_candidates: Vec<String>,
    pub excluded_reward_items: Vec<String>,
    pub reward_quantity: u16,
    pub reward_failure_sentinel: String,
    pub reward_item_values: BTreeMap<String, u8>,
    pub minimum_level_group: u8,
    pub maximum_level_group: u8,
    pub level_group_size: u8,
    pub party_count_failure_text: String,
    pub duplicate_species_failure_text: String,
    pub duplicate_held_item_failure_text: String,
    pub egg_failure_text: String,
    pub trainers: Vec<BattleTowerTrainerDefinition>,
    pub mon_groups: Vec<Vec<BattleTowerMonDefinition>>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BattleTowerTrainerDefinition {
    pub index: usize,
    pub trainer_class: String,
    pub name: String,
    pub sprite_constant: String,
    pub female: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BattleTowerMonDefinition {
    pub species: String,
    pub item: Option<String>,
    pub moves: Vec<String>,
    pub original_trainer_id: u16,
    pub experience: u32,
    pub stat_exp: Vec<u16>,
    pub dvs: Vec<u8>,
    pub pp: Vec<u8>,
    pub happiness: u8,
    pub pokerus: Vec<u8>,
    pub level: u8,
    pub status: Vec<u8>,
    pub stats: Vec<u16>,
    pub nickname: String,
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
            reward_candidates: Vec<String>,
            excluded_reward_items: Vec<String>,
            reward_quantity: u16,
            reward_failure_sentinel: String,
            reward_item_values: BTreeMap<String, u8>,
            minimum_level_group: u8,
            maximum_level_group: u8,
            level_group_size: u8,
            party_count_failure_text: String,
            duplicate_species_failure_text: String,
            duplicate_held_item_failure_text: String,
            egg_failure_text: String,
            trainers: Vec<BattleTowerTrainerDefinition>,
            mon_groups: Vec<Vec<BattleTowerMonDefinition>>,
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
        if raw.reward_candidates.is_empty() || raw.reward_quantity == 0 {
            return Err(serde::de::Error::custom(
                "battle tower reward candidates and quantity must be nonzero",
            ));
        }
        require_special_token(
            "battle tower rewardFailureSentinel",
            &raw.reward_failure_sentinel,
        )
        .map_err(serde::de::Error::custom)?;
        for item_id in raw
            .reward_candidates
            .iter()
            .chain(std::iter::once(&raw.reward_failure_sentinel))
        {
            if !raw.reward_item_values.contains_key(item_id) {
                return Err(serde::de::Error::custom(format!(
                    "battle tower rewardItemValues is missing {item_id}"
                )));
            }
        }
        let unique_reward_values = raw.reward_item_values.values().collect::<BTreeSet<_>>();
        if unique_reward_values.len() != raw.reward_item_values.len() {
            return Err(serde::de::Error::custom(
                "battle tower rewardItemValues must contain unique item bytes",
            ));
        }
        let mut reward_candidates = BTreeSet::new();
        for item_id in &raw.reward_candidates {
            require_special_token("battle tower rewardCandidates entry", item_id)
                .map_err(serde::de::Error::custom)?;
            if !reward_candidates.insert(item_id.as_str()) {
                return Err(serde::de::Error::custom(format!(
                    "battle tower reward candidate {item_id} is duplicated"
                )));
            }
        }
        for item_id in &raw.excluded_reward_items {
            require_special_token("battle tower excludedRewardItems entry", item_id)
                .map_err(serde::de::Error::custom)?;
            if !reward_candidates.contains(item_id.as_str()) {
                return Err(serde::de::Error::custom(format!(
                    "battle tower excluded reward {item_id} is not a candidate"
                )));
            }
        }
        if raw
            .reward_candidates
            .iter()
            .all(|item| raw.excluded_reward_items.contains(item))
        {
            return Err(serde::de::Error::custom(
                "battle tower reward table excludes every candidate",
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
        if raw.trainers.is_empty() {
            return Err(serde::de::Error::custom(
                "battle tower trainer roster must be nonempty",
            ));
        }
        if raw.mon_groups.is_empty() || raw.mon_groups.iter().any(Vec::is_empty) {
            return Err(serde::de::Error::custom(
                "battle tower Pokemon groups must be present and nonempty",
            ));
        }

        Ok(Self {
            banned_species: raw.banned_species,
            required_party_count: raw.required_party_count,
            challenge_streak_length: raw.challenge_streak_length,
            reward_candidates: raw.reward_candidates,
            excluded_reward_items: raw.excluded_reward_items,
            reward_quantity: raw.reward_quantity,
            reward_failure_sentinel: raw.reward_failure_sentinel,
            reward_item_values: raw.reward_item_values,
            minimum_level_group: raw.minimum_level_group,
            maximum_level_group: raw.maximum_level_group,
            level_group_size: raw.level_group_size,
            party_count_failure_text: raw.party_count_failure_text,
            duplicate_species_failure_text: raw.duplicate_species_failure_text,
            duplicate_held_item_failure_text: raw.duplicate_held_item_failure_text,
            egg_failure_text: raw.egg_failure_text,
            trainers: raw.trainers,
            mon_groups: raw.mon_groups,
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
    MissingTrainerRoster,
    MissingMonGroups,
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
    if rules.trainers.is_empty() {
        issues.push(BattleTowerRulesIssue::MissingTrainerRoster);
    }
    if rules.mon_groups.is_empty() || rules.mon_groups.iter().any(Vec::is_empty) {
        issues.push(BattleTowerRulesIssue::MissingMonGroups);
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
    pub divisor: u8,
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
        if raw.divisor > u16::from(u8::MAX) {
            return Err(serde::de::Error::custom(
                "magikarp length divisor must fit one source byte",
            ));
        }
        Ok(Self {
            threshold: raw.threshold,
            divisor: raw.divisor as u8,
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
        if entries.len() != 14 {
            return Err(serde::de::Error::custom(
                "Magikarp length table must contain exactly 14 source rows",
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
    InvalidEntryCount {
        actual: usize,
    },
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
    if entries.len() != 14 {
        issues.push(MagikarpLengthTableIssue::InvalidEntryCount {
            actual: entries.len(),
        });
    }
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

pub const ROAMING_POKEMON_SLOT_COUNT: usize = 3;
pub const ROAMING_POKEMON_ROUTE_COUNT: usize = 16;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoamingMapLocation {
    pub map_group: u8,
    pub map_number: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoamingPokemonInitWrite {
    pub slot: u8,
    pub species: String,
    pub level: u8,
    pub map_group: u8,
    pub map_number: u8,
    pub hp: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoamingPokemonRoute {
    pub map_group: u8,
    pub map_number: u8,
    pub connections: Vec<RoamingMapLocation>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoamingPokemonCatalog {
    pub slot_count: u8,
    pub inactive_map: RoamingMapLocation,
    pub init_writes: Vec<RoamingPokemonInitWrite>,
    pub routes: Vec<RoamingPokemonRoute>,
    pub jump_mask: u8,
}

impl<'de> Deserialize<'de> for RoamingPokemonCatalog {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct RawCatalog {
            slot_count: u8,
            inactive_map: RoamingMapLocation,
            init_writes: Vec<RoamingPokemonInitWrite>,
            routes: Vec<RoamingPokemonRoute>,
            jump_mask: u8,
        }

        let raw = RawCatalog::deserialize(deserializer)?;
        let catalog = Self {
            slot_count: raw.slot_count,
            inactive_map: raw.inactive_map,
            init_writes: raw.init_writes,
            routes: raw.routes,
            jump_mask: raw.jump_mask,
        };
        if let Some(issue) = roaming_pokemon_catalog_shape_issues(&catalog)
            .into_iter()
            .next()
        {
            return Err(serde::de::Error::custom(issue.to_string()));
        }
        Ok(catalog)
    }
}

impl RoamingPokemonCatalog {
    pub fn is_empty(&self) -> bool {
        self.init_writes.is_empty() && self.routes.is_empty()
    }

    pub fn init_write(&self, slot: usize) -> Option<&RoamingPokemonInitWrite> {
        self.init_writes
            .iter()
            .find(|write| usize::from(write.slot) == slot)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RoamingPokemonCatalogIssue {
    #[error("roaming Pokemon slotCount must be 3, found {slot_count}")]
    InvalidSlotCount { slot_count: u8 },
    #[error("roaming Pokemon inactiveMap must not be the pre-init 0/0 location")]
    InvalidInactiveMap,
    #[error("roaming Pokemon inactiveMap collides with a live init, route, or connection map")]
    InactiveMapCollision,
    #[error("roaming Pokemon initWrites must contain ordered source slots 0 then 1")]
    InvalidInitWriteOrder,
    #[error("roaming Pokemon init slot {slot} species is invalid: {species}")]
    InvalidInitSpecies { slot: u8, species: String },
    #[error("roaming Pokemon init slot {slot} species {species} is missing from Pokemon data")]
    UnknownInitSpecies { slot: u8, species: String },
    #[error("roaming Pokemon init slot {slot} level {level} is outside 1..=100")]
    InvalidInitLevel { slot: u8, level: u8 },
    #[error("roaming Pokemon init slot {slot} uses invalid map {map_group}/{map_number}")]
    InvalidInitMap {
        slot: u8,
        map_group: u8,
        map_number: u8,
    },
    #[error("roaming Pokemon init slot {slot} map {map_group}/{map_number} is not a route origin")]
    UnknownInitRoute {
        slot: u8,
        map_group: u8,
        map_number: u8,
    },
    #[error("roaming Pokemon init slot {slot} hp must be source value 0, found {hp}")]
    InvalidInitHp { slot: u8, hp: u8 },
    #[error("roaming Pokemon catalog must contain exactly 16 ordered route rows, found {count}")]
    InvalidRouteCount { count: usize },
    #[error("roaming Pokemon route {index} uses duplicate or invalid map {map_group}/{map_number}")]
    InvalidRouteMap {
        index: usize,
        map_group: u8,
        map_number: u8,
    },
    #[error("roaming Pokemon route {index} must contain 1..=4 connections, found {count}")]
    InvalidConnectionCount { index: usize, count: usize },
    #[error(
        "roaming Pokemon route {index} connection {connection} uses invalid map {map_group}/{map_number}"
    )]
    InvalidConnectionMap {
        index: usize,
        connection: usize,
        map_group: u8,
        map_number: u8,
    },
    #[error("roaming Pokemon route {index} repeats connection {map_group}/{map_number}")]
    DuplicateConnection {
        index: usize,
        map_group: u8,
        map_number: u8,
    },
    #[error(
        "roaming Pokemon route {index} connection {map_group}/{map_number} is not a route origin"
    )]
    UnknownConnectionTarget {
        index: usize,
        map_group: u8,
        map_number: u8,
    },
    #[error("roaming Pokemon jumpMask must be 15, found {jump_mask}")]
    InvalidJumpMask { jump_mask: u8 },
}

pub fn roaming_pokemon_catalog_shape_issues(
    catalog: &RoamingPokemonCatalog,
) -> Vec<RoamingPokemonCatalogIssue> {
    let mut issues = Vec::new();
    if usize::from(catalog.slot_count) != ROAMING_POKEMON_SLOT_COUNT {
        issues.push(RoamingPokemonCatalogIssue::InvalidSlotCount {
            slot_count: catalog.slot_count,
        });
    }
    if catalog.inactive_map.map_group == 0 || catalog.inactive_map.map_number == 0 {
        issues.push(RoamingPokemonCatalogIssue::InvalidInactiveMap);
    }
    let inactive_collision = catalog
        .init_writes
        .iter()
        .any(|write| write.map_group == catalog.inactive_map.map_group)
        || catalog.routes.iter().any(|route| {
            route.map_group == catalog.inactive_map.map_group
                || route
                    .connections
                    .iter()
                    .any(|connection| connection.map_group == catalog.inactive_map.map_group)
        });
    if inactive_collision {
        issues.push(RoamingPokemonCatalogIssue::InactiveMapCollision);
    }
    if catalog.init_writes.len() != 2
        || catalog.init_writes[0].slot != 0
        || catalog.init_writes[1].slot != 1
    {
        issues.push(RoamingPokemonCatalogIssue::InvalidInitWriteOrder);
    }
    if catalog.routes.len() != ROAMING_POKEMON_ROUTE_COUNT {
        issues.push(RoamingPokemonCatalogIssue::InvalidRouteCount {
            count: catalog.routes.len(),
        });
    }
    if catalog.jump_mask != 15 {
        issues.push(RoamingPokemonCatalogIssue::InvalidJumpMask {
            jump_mask: catalog.jump_mask,
        });
    }
    let mut route_maps = BTreeSet::new();
    for (index, route) in catalog.routes.iter().enumerate() {
        if route.map_group == 0
            || route.map_number == 0
            || !route_maps.insert((route.map_group, route.map_number))
        {
            issues.push(RoamingPokemonCatalogIssue::InvalidRouteMap {
                index,
                map_group: route.map_group,
                map_number: route.map_number,
            });
        }
        if !(1..=4).contains(&route.connections.len()) {
            issues.push(RoamingPokemonCatalogIssue::InvalidConnectionCount {
                index,
                count: route.connections.len(),
            });
        }
        let mut connections = BTreeSet::new();
        for (connection, target) in route.connections.iter().enumerate() {
            if target.map_group == 0 || target.map_number == 0 {
                issues.push(RoamingPokemonCatalogIssue::InvalidConnectionMap {
                    index,
                    connection,
                    map_group: target.map_group,
                    map_number: target.map_number,
                });
            }
            if !connections.insert((target.map_group, target.map_number)) {
                issues.push(RoamingPokemonCatalogIssue::DuplicateConnection {
                    index,
                    map_group: target.map_group,
                    map_number: target.map_number,
                });
            }
        }
    }
    for write in &catalog.init_writes {
        if !route_maps.contains(&(write.map_group, write.map_number)) {
            issues.push(RoamingPokemonCatalogIssue::UnknownInitRoute {
                slot: write.slot,
                map_group: write.map_group,
                map_number: write.map_number,
            });
        }
    }
    for (index, route) in catalog.routes.iter().enumerate() {
        for target in &route.connections {
            if !route_maps.contains(&(target.map_group, target.map_number)) {
                issues.push(RoamingPokemonCatalogIssue::UnknownConnectionTarget {
                    index,
                    map_group: target.map_group,
                    map_number: target.map_number,
                });
            }
        }
    }
    issues
}

pub fn roaming_pokemon_catalog_issues(
    catalog: &RoamingPokemonCatalog,
    species_ids: &BTreeSet<String>,
) -> Vec<RoamingPokemonCatalogIssue> {
    let mut issues = roaming_pokemon_catalog_shape_issues(catalog);
    for write in &catalog.init_writes {
        if !is_exact_nonempty_special_token(&write.species) {
            issues.push(RoamingPokemonCatalogIssue::InvalidInitSpecies {
                slot: write.slot,
                species: write.species.clone(),
            });
        } else if !species_ids.contains(&write.species) {
            issues.push(RoamingPokemonCatalogIssue::UnknownInitSpecies {
                slot: write.slot,
                species: write.species.clone(),
            });
        }
        if !(1..=100).contains(&write.level) {
            issues.push(RoamingPokemonCatalogIssue::InvalidInitLevel {
                slot: write.slot,
                level: write.level,
            });
        }
        if write.map_group == 0 || write.map_number == 0 {
            issues.push(RoamingPokemonCatalogIssue::InvalidInitMap {
                slot: write.slot,
                map_group: write.map_group,
                map_number: write.map_number,
            });
        }
        if write.hp != 0 {
            issues.push(RoamingPokemonCatalogIssue::InvalidInitHp {
                slot: write.slot,
                hp: write.hp,
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
    "Function170114",
    "Function1704e1",
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
    let empty_roaming_pokemon = RoamingPokemonCatalog::default();
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

pub fn apply_random_special_routine<S>(
    state: &mut GameState,
    move_catalog: &BTreeMap<String, Move>,
    routine: &str,
    divider: &mut S,
) -> Result<SpecialRoutineOutcome, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource + ?Sized,
{
    let empty_cries = BTreeMap::new();
    let empty_species = BTreeMap::new();
    let empty_learnsets = SpeciesLearnsets::new();
    let empty_growth_rates = GrowthRateCatalog::new();
    let empty_items = BTreeMap::new();
    let empty_spawn_points = BTreeMap::new();
    let empty_trainers = TrainerCatalog::default();
    let empty_roaming_pokemon = RoamingPokemonCatalog::default();
    let empty_buena_password_categories = BuenaPasswordCategories::default();
    let empty_buena_prizes = BTreeMap::new();
    let empty_kurt_apricorn_recipes = BTreeMap::new();
    let empty_dratini_move_sets = BTreeMap::new();
    let empty_phone_contacts = PhoneContactCatalog::default();
    let empty_wild_encounters = BTreeMap::new();
    apply_random_special_routine_with_context(
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
        divider,
    )
}

pub fn apply_random_special_routine_with_context<S>(
    state: &mut GameState,
    context: SpecialRoutineContext<'_>,
    routine: &str,
    divider: &mut S,
) -> Result<SpecialRoutineOutcome, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource + ?Sized,
{
    let mut next = state.clone();
    let outcome = match routine {
        "SampleKenjiBreakCountdown" => sample_kenji_break_countdown(&mut next, routine, divider),
        "ResetLuckyNumberShowFlag" => reset_lucky_number_show_flag(&mut next, routine, divider),
        "RandomUnseenWildMon" => random_unseen_wild_mon(
            &mut next,
            context.species_catalog,
            context.phone_contacts,
            context.wild_encounters,
            routine,
            divider,
        ),
        "RandomPhoneWildMon" => random_phone_wild_mon(
            &mut next,
            context.species_catalog,
            context.phone_contacts,
            context.wild_encounters,
            routine,
            divider,
        ),
        "RandomPhoneMon" => random_phone_mon(
            &mut next,
            context.species_catalog,
            context.phone_contacts,
            context.trainer_catalog,
            routine,
            divider,
        ),
        "UnownPuzzle" => unown_puzzle(&mut next, routine, divider),
        "CardFlip" => card_flip(&mut next, context.item_catalog, routine, divider),
        "SlotMachine" => slot_machine(&mut next, context.item_catalog, routine, divider),
        "UnusedMemoryGame" | "MemoryGame" => {
            unused_memory_game(&mut next, context.item_catalog, routine, divider)
        }
        "DayCareMan" => day_care_interaction(&mut next, routine, "man", divider),
        "DayCareLady" => day_care_interaction(&mut next, routine, "lady", divider),
        "DayCareManOutside" => day_care_man_outside(&mut next, routine, divider),
        "GiveShuckle" => give_shuckle(&mut next, context, routine, divider),
        "BuenasPassword" => buenas_password(
            &mut next,
            context.buena_password_categories,
            routine,
            divider,
        ),
        "SelectRandomBugContestContestants" => select_random_bug_contest_contestants(
            &mut next,
            context.bug_contest_config,
            routine,
            divider,
        ),
        "BugContestJudging" => {
            bug_contest_judging(&mut next, context.bug_contest_config, routine, divider)
        }
        "BattleTowerAction" => battle_tower_random_action(&mut next, context, routine, divider),
        "LoadOpponentTrainerAndPokemonWithOTSprite" => {
            load_opponent_trainer_and_pokemon_with_ot_sprite(&mut next, context, routine, divider)
        }
        "GiveOddEgg" => give_odd_egg(
            &mut next,
            context.species_catalog,
            context.learnsets,
            context.growth_rates,
            context.move_catalog,
            context.odd_egg_definitions,
            routine,
            divider,
        ),
        exact => Err(SpecialRoutineError::UnsupportedRoutine {
            routine: exact.to_string(),
        }
        .into()),
    }?;
    *state = next;
    Ok(outcome)
}

pub fn apply_special_routine_with_context(
    state: &mut GameState,
    context: SpecialRoutineContext<'_>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    match routine {
        "WarpToSpawnPoint" => warp_to_spawn_point(state, routine),
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
        "SampleKenjiBreakCountdown" | "ResetLuckyNumberShowFlag" => {
            Err(SpecialRoutineError::MissingDividerSource {
                routine: routine.to_string(),
            })
        }
        "CheckLuckyNumberShowFlag" => check_lucky_number_show_flag(state, routine),
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
        "ToggleMaptileDecorations" => toggle_maptile_decorations(state, routine),
        "ToggleDecorationsVisibility" => toggle_decorations_visibility(state, routine),
        "MagnetTrain" => visual_command(state, routine, ScriptGraphicsRuntimeKind::MagnetTrain),
        "Diploma" => visual_command(state, routine, ScriptGraphicsRuntimeKind::Diploma),
        "PrintDiploma" => visual_command(state, routine, ScriptGraphicsRuntimeKind::PrintDiploma),
        "UnownPuzzle" => Err(SpecialRoutineError::MissingDividerSource {
            routine: routine.to_string(),
        }),
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
        "MoveTutor" => move_tutor(state, context.move_catalog, context.happiness_data, routine),
        "BankOfMom" => bank_of_mom(state, routine),
        "SlotMachine" => Err(SpecialRoutineError::MissingDividerSource {
            routine: routine.to_string(),
        }),
        "CardFlip" => Err(SpecialRoutineError::MissingDividerSource {
            routine: routine.to_string(),
        }),
        "UnusedMemoryGame" | "MemoryGame" => Err(SpecialRoutineError::MissingDividerSource {
            routine: routine.to_string(),
        }),
        "DisplayLinkRecord" => display_link_record(state, routine),
        "TrainerHouse" => trainer_house(state, routine),
        "PhotoStudio" => photo_studio(state, routine),
        "GiveShuckle" => Err(SpecialRoutineError::MissingDividerSource {
            routine: routine.to_string(),
        }),
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
        "BuenasPassword" => Err(SpecialRoutineError::MissingDividerSource {
            routine: routine.to_string(),
        }),
        "BuenaPrize" => buena_prize(state, context.item_catalog, context.buena_prizes, routine),
        "CelebiShrineEvent" => celebi_shrine_event(state, routine),
        "CheckMagikarpLength" => check_magikarp_length(state, context.magikarp_lengths, routine),
        "MagikarpHouseSign" => magikarp_house_sign(state, routine),
        "DayCareMan" | "DayCareLady" | "DayCareManOutside" => {
            Err(SpecialRoutineError::MissingDividerSource {
                routine: routine.to_string(),
            })
        }
        "DayCareMon1" => day_care_mon(state, routine, "man"),
        "DayCareMon2" => day_care_mon(state, routine, "lady"),
        "GiveParkBalls" => give_park_balls(state, context.bug_contest_config, routine),
        "StartBugContestTimer" => start_bug_contest_timer(state, routine),
        "CheckBugContestTimer" => check_bug_contest_timer(state, routine),
        "SelectRandomBugContestContestants" => Err(SpecialRoutineError::MissingDividerSource {
            routine: routine.to_string(),
        }),
        "ContestDropOffMons" => contest_drop_off_mons(state, routine),
        "ContestReturnMons" => contest_return_mons(state, routine),
        "CheckPartyFullAfterContest" => check_party_full_after_contest(state, routine),
        "BugContestJudging" => Err(SpecialRoutineError::MissingDividerSource {
            routine: routine.to_string(),
        }),
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
        "BattleTowerAction" => battle_tower_action(
            state,
            context.battle_tower_rules,
            context.item_catalog,
            routine,
        ),
        "CheckForBattleTowerRules" => {
            check_for_battle_tower_rules(state, context.battle_tower_rules, routine)
        }
        "BattleTowerRoomMenu" => battle_tower_room_menu(state, context.battle_tower_rules, routine),
        "BattleTowerBattle" => battle_tower_battle(
            state,
            context.battle_tower_rules,
            context.move_catalog,
            routine,
        ),
        "BattleTowerMobileError" => battle_tower_mobile_error(state, routine),
        "LoadOpponentTrainerAndPokemonWithOTSprite" => {
            Err(SpecialRoutineError::MissingDividerSource {
                routine: routine.to_string(),
            })
        }
        "AskRememberPassword" => ask_remember_password(state, routine),
        "Function1700ba" => battle_tower_leaderboard(state, routine),
        "Function170114" => battle_tower_initialize_challenge_ram(state, routine),
        "Function1704e1" => battle_tower_room_menu(state, context.battle_tower_rules, routine),
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
        "GiveOddEgg" => Err(SpecialRoutineError::MissingDividerSource {
            routine: routine.to_string(),
        }),
        "Menu_ChallengeExplanationCancel" => {
            battle_tower_challenge_explanation_cancel(state, routine)
        }
        "UnusedDummySpecial"
        | "UnusedBattleTowerDummySpecial1"
        | "UnusedBattleTowerDummySpecial2" => noop_special(routine),
        "UnusedFindItemInPCOrBag" => {
            unused_find_item_in_pc_or_bag(state, context.item_catalog, routine)
        }
        "RandomUnseenWildMon" | "RandomPhoneWildMon" | "RandomPhoneMon" => {
            Err(SpecialRoutineError::MissingDividerSource {
                routine: routine.to_string(),
            })
        }
        "Function11ba38" => function11ba38(state, routine),
        "Function11ac3e" | "TradeCornerHoldMon" | "Function11b5e8" | "Function11b7e5"
        | "Function11b879" | "Function11b920" | "Function11b93b" | "Function11c1ab"
        | "Function17d2b6" | "Function17d2ce" | "Function102142" => {
            inactive_declared_routine(routine)
        }
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
        if pokemon.is_egg || pokemon.species.id == "EGG" {
            continue;
        }
        if heal_pokemon(pokemon, move_catalog) {
            healed_slots.push(party_slot);
        }
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
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let safari_game_was_active = state
        .flags
        .is_engine_flag_set("ENGINE_SAFARI_ZONE")
        .map_err(|error| SpecialRoutineError::EventFlag {
            routine: routine.to_string(),
            error,
        })?;
    let bug_contest_timer_was_active = state.bug_contest.timer_active;
    state.bug_contest.timer_active = false;
    state.bug_contest.timer_start_time = None;
    // This source bit is never exposed through a live Crystal script flag in
    // the compiled pack. Its absent sparse entry is the cleared WRAM value.
    state.flags.engine_flags.remove("ENGINE_SAFARI_ZONE");
    state
        .flags
        .set_engine_flag("ENGINE_BUG_CONTEST_TIMER", false)
        .map_err(|error| SpecialRoutineError::EventFlag {
            routine: routine.to_string(),
            error,
        })?;
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::WarpToSpawnPoint {
            safari_game_was_active,
            bug_contest_timer_was_active,
        },
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
            (!pokemon_is_egg(pokemon)).then_some((index, pokemon))
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
        .find(|pokemon| !pokemon_is_egg(pokemon) && pokemon.level > level)
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
        .find(|pokemon| !pokemon_is_egg(pokemon) && pokemon.happiness >= happiness)
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

fn random_unseen_wild_mon<S>(
    state: &mut GameState,
    species_catalog: &BTreeMap<String, PokemonSpecies>,
    phone_contacts: &PhoneContactCatalog,
    wild_encounters: &BTreeMap<String, WildEncounterData>,
    routine: &str,
    divider: &mut S,
) -> Result<SpecialRoutineOutcome, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource + ?Sized,
{
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
        }
        .into());
    }

    let mut rng = CrystalRandom::new(state.random_state, divider);
    let rare_index = loop {
        // AddNTimes cannot overflow the bank-local encounter table address,
        // so the first call enters with carry clear. Every retry follows AND
        // %11, which also clears carry.
        let masked = rng
            .random(false)
            .map_err(RandomSpecialRoutineError::Divider)?
            .value
            & 0b11;
        if masked != 0 {
            break 4 + (masked as usize - 1);
        }
    };
    state.random_state = rng.state();

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
            random_state_after: state.random_state,
        },
    })
}

fn random_phone_wild_mon<S>(
    state: &mut GameState,
    species_catalog: &BTreeMap<String, PokemonSpecies>,
    phone_contacts: &PhoneContactCatalog,
    wild_encounters: &BTreeMap<String, WildEncounterData>,
    routine: &str,
    divider: &mut S,
) -> Result<SpecialRoutineOutcome, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource + ?Sized,
{
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
        }
        .into());
    }
    let mut rng = CrystalRandom::new(state.random_state, divider);
    // The preceding bank-local AddNTimes and DEC loop leave carry clear.
    let sample = rng
        .random(false)
        .map_err(RandomSpecialRoutineError::Divider)?
        .value;
    let selected = &slots[(sample & 0b11) as usize];
    state.random_state = rng.state();
    write_phone_species_buffers(state, species_catalog, routine, selected)?;
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::RandomPhoneWildMon {
            contact_id,
            map_name: encounters.map_name.clone(),
            time_of_day,
            species: selected.species.clone(),
            random_state_after: state.random_state,
        },
    })
}

fn random_phone_mon<S>(
    state: &mut GameState,
    species_catalog: &BTreeMap<String, PokemonSpecies>,
    phone_contacts: &PhoneContactCatalog,
    trainer_catalog: &TrainerCatalog,
    routine: &str,
    divider: &mut S,
) -> Result<SpecialRoutineOutcome, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource + ?Sized,
{
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
        }
        .into());
    }

    let mut rng = CrystalRandom::new(state.random_state, divider);
    let party_index = loop {
        // The party-count loop exits after `cp -1`, which clears carry on
        // equality. Rejected candidates reach the retry after `cp e` with
        // carry clear because the candidate was greater than or equal to E.
        let masked = (rng
            .random(false)
            .map_err(RandomSpecialRoutineError::Divider)?
            .value
            & 0b111) as usize;
        if masked < trainer.party.len() {
            break masked;
        }
    };
    state.random_state = rng.state();

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
            random_state_after: state.random_state,
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
    // calling InitDayOfWeek. Invoking this routine without that register is
    // invalid; preserving or clamping another clock field invents a path that
    // does not exist in the source flow.
    let selected_value = state
        .script_runtime
        .variables
        .get("wTempDayOfWeek")
        .ok_or_else(|| SpecialRoutineError::MissingScriptValue {
            routine: routine.to_string(),
            variable: "wTempDayOfWeek".to_string(),
        })?;
    let selected_day = selected_value
        .parse::<u8>()
        .ok()
        .filter(|day| *day < 7)
        .ok_or_else(|| SpecialRoutineError::InvalidNumericValue {
            routine: routine.to_string(),
            value: selected_value.clone(),
        })?;
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
            hour: state.time.registers.hours,
            minute: state.time.registers.minutes,
            second: state.time.registers.seconds,
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

fn sample_kenji_break_countdown<S>(
    state: &mut GameState,
    routine: &str,
    divider: &mut S,
) -> Result<SpecialRoutineOutcome, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource + ?Sized,
{
    let mut rng = CrystalRandom::new(state.random_state, divider);
    // Script_special reaches Special through bank-local pointer additions;
    // the pointer table cannot overflow, so SampleKenji enters with carry
    // clear just like the CheckDailyResetTimer call site.
    let sample = rng
        .random(false)
        .map_err(RandomSpecialRoutineError::Divider)?
        .value;
    let value = 3 + (sample & 0x03);
    state.random_state = rng.state();
    state.kenji_break_timer = value;
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::SampleKenjiBreakCountdown {
            value,
            random_state_after: state.random_state,
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

fn reset_lucky_number_show_flag<S>(
    state: &mut GameState,
    routine: &str,
    divider: &mut S,
) -> Result<SpecialRoutineOutcome, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource + ?Sized,
{
    state.lucky_number_show_flag = false;
    let lucky_number = load_or_regenerate_lucky_number(state, divider)?;
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::ResetLuckyNumberShowFlag {
            lucky_number,
            lucky_number_day: state.time.current_day,
            random_state_after: state.random_state,
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

    let lucky_number = state.lucky_id_number;
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
        state
            .script_runtime
            .named_buffers
            .insert("STRING_BUFFER_1".to_string(), species.replace('_', " "));
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

fn load_or_regenerate_lucky_number<S>(
    state: &mut GameState,
    divider: &mut S,
) -> Result<u16, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource + ?Sized,
{
    let current_day = state.time.current_day;
    let current_marker = current_day.wrapping_add(1);
    let stored_marker = state
        .lucky_number_day
        .map(|day| day.wrapping_add(1))
        .unwrap_or(0);
    if stored_marker != current_marker {
        let mut rng = CrystalRandom::new(state.random_state, divider);
        // LoadOrRegenerateLuckyIDNumber compares the saved day marker against
        // current_day + 1. The CP carry feeds the first Random call, and the
        // first call's SBC carry is preserved by `ld c, a` into the second.
        let first = rng
            .random(stored_marker < current_marker)
            .map_err(RandomSpecialRoutineError::Divider)?;
        let second = rng
            .random(first.carry_out)
            .map_err(RandomSpecialRoutineError::Divider)?;
        let high = u16::from(first.value);
        let low = u16::from(second.value);
        state.random_state = rng.state();
        state.lucky_id_number = (high << 8) | low;
        state.lucky_number_day = Some(current_day);
    }
    Ok(state.lucky_id_number)
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
    let lucky_number = state.lucky_id_number;
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
    // PlayersHousePC clears wScriptVar before the asynchronous PC/menu flow.
    // Only leaving the decoration menu after a real change sets it back to
    // TRUE so PlayersHousePCScript takes its map-reload warp branch.
    set_script_bool_value(state, false);
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
    let letters = state.pokedex.unown_letters.clone();
    state.script_runtime.active_menu = (!letters.is_empty()).then(|| "UnownPrinter".to_string());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::UnownPrinter { letters },
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

fn shuffled_unown_puzzle<S>(rng: &mut CrystalRandom<S>) -> Result<UnownPuzzleState, S::Error>
where
    S: DividerSource,
{
    let mut layout = [[0_u8; 6]; 6];
    for piece_id in 1..=16 {
        loop {
            // InitUnownPuzzlePiecePositions enters every Random call with
            // carry clear: its table arithmetic cannot overflow and both the
            // accepted and occupied-slot paths execute AND before retrying.
            let slot_index = usize::from(rng.random(false)?.value & 0x0f);
            let (x, y) = UNOWN_START_POSITIONS[slot_index];
            if layout[y][x] == 0 {
                layout[y][x] = piece_id;
                break;
            }
        }
    }
    Ok(UnownPuzzleState {
        layout,
        holding_piece: None,
        moves: 0,
    })
}

fn load_unown_puzzle_state(
    state: &mut GameState,
    routine: &str,
    puzzle_id: &str,
) -> Result<UnownPuzzleState, SpecialRoutineError> {
    let Some(raw_layout) = migrate_unown_puzzle_variable(state, "unown_layout", puzzle_id) else {
        return Err(SpecialRoutineError::InvalidUnownPuzzleState {
            routine: routine.to_string(),
            message: format!("puzzle {puzzle_id} has no active layout"),
        });
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

fn unown_puzzle<S>(
    state: &mut GameState,
    routine: &str,
    divider: &mut S,
) -> Result<SpecialRoutineOutcome, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource + ?Sized,
{
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

    let mut puzzle = if matches!(action.as_deref(), None | Some("shuffle")) {
        let mut rng = CrystalRandom::new(state.random_state, &mut *divider);
        let puzzle = shuffled_unown_puzzle(&mut rng).map_err(RandomSpecialRoutineError::Divider)?;
        state.random_state = rng.state();
        puzzle
    } else {
        load_unown_puzzle_state(state, routine, &puzzle_id)?
    };

    match action.as_deref() {
        Some("shuffle") => {}
        Some("pickup") => {
            let (x, y) = unown_coords(state, routine)?;
            if puzzle.holding_piece.is_some() {
                return Err(SpecialRoutineError::InvalidUnownPuzzleState {
                    routine: routine.to_string(),
                    message: "cannot pick up a piece while already holding one".to_string(),
                }
                .into());
            }
            let piece = puzzle.layout[y][x];
            if piece == 0 {
                return Err(SpecialRoutineError::InvalidUnownPuzzleState {
                    routine: routine.to_string(),
                    message: "no piece present at that coordinate".to_string(),
                }
                .into());
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
                }
                .into());
            };
            if puzzle.layout[y][x] != 0 {
                return Err(SpecialRoutineError::InvalidUnownPuzzleState {
                    routine: routine.to_string(),
                    message: "target coordinate is already occupied".to_string(),
                }
                .into());
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
            }
            .into());
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
            random_state_after: state.random_state,
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
        effect: SpecialRoutineEffect::RuntimeVisualCommand { kind },
    })
}

const CONSOLE_DECORATION_SPRITES: &[(&str, &str)] = &[
    ("DECO_FAMICOM", "SPRITE_FAMICOM"),
    ("DECO_SNES", "SPRITE_SNES"),
    ("DECO_N64", "SPRITE_N64"),
    ("DECO_VIRTUAL_BOY", "SPRITE_VIRTUAL_BOY"),
];

const BIG_DOLL_DECORATION_SPRITES: &[(&str, &str)] = &[
    ("DECO_BIG_SNORLAX_DOLL", "SPRITE_BIG_SNORLAX"),
    ("DECO_BIG_ONIX_DOLL", "SPRITE_BIG_ONIX"),
    ("DECO_BIG_LAPRAS_DOLL", "SPRITE_BIG_LAPRAS"),
];

const ORNAMENT_DECORATION_SPRITES: &[(&str, &str)] = &[
    ("DECO_PIKACHU_DOLL", "SPRITE_PIKACHU"),
    ("DECO_SURF_PIKACHU_DOLL", "SPRITE_SURFING_PIKACHU"),
    ("DECO_CLEFAIRY_DOLL", "SPRITE_CLEFAIRY"),
    ("DECO_JIGGLYPUFF_DOLL", "SPRITE_JIGGLYPUFF"),
    ("DECO_BULBASAUR_DOLL", "SPRITE_BULBASAUR"),
    ("DECO_CHARMANDER_DOLL", "SPRITE_CHARMANDER"),
    ("DECO_SQUIRTLE_DOLL", "SPRITE_SQUIRTLE"),
    ("DECO_POLIWAG_DOLL", "SPRITE_POLIWAG"),
    ("DECO_DIGLETT_DOLL", "SPRITE_DIGLETT"),
    ("DECO_STARYU_DOLL", "SPRITE_STARMIE"),
    ("DECO_MAGIKARP_DOLL", "SPRITE_MAGIKARP"),
    ("DECO_ODDISH_DOLL", "SPRITE_ODDISH"),
    ("DECO_GENGAR_DOLL", "SPRITE_GENGAR"),
    ("DECO_SHELLDER_DOLL", "SPRITE_SHELLDER"),
    ("DECO_GRIMER_DOLL", "SPRITE_GRIMER"),
    ("DECO_VOLTORB_DOLL", "SPRITE_VOLTORB"),
    ("DECO_WEEDLE_DOLL", "SPRITE_WEEDLE"),
    ("DECO_UNOWN_DOLL", "SPRITE_UNOWN"),
    ("DECO_GEODUDE_DOLL", "SPRITE_GEODUDE"),
    ("DECO_MACHOP_DOLL", "SPRITE_MACHOP"),
    ("DECO_TENTACOOL_DOLL", "SPRITE_TENTACOOL"),
    ("DECO_GOLD_TROPHY_DOLL", "SPRITE_GOLD_TROPHY"),
    ("DECO_SILVER_TROPHY_DOLL", "SPRITE_SILVER_TROPHY"),
];

fn equipped_decoration_block(
    state: &GameState,
    routine: &str,
    memory: &str,
    blocks: &[(&str, u16)],
) -> Result<Option<u16>, SpecialRoutineError> {
    let Some(decoration) = state.script_runtime.memory.get(memory) else {
        return Ok(None);
    };
    if decoration == "0" {
        return Ok(None);
    }
    blocks
        .iter()
        .find_map(|(candidate, block)| (decoration == candidate).then_some(*block))
        .map(Some)
        .ok_or_else(|| SpecialRoutineError::InvalidState {
            routine: routine.to_string(),
            message: format!("{memory} has invalid equipped decoration {decoration}"),
        })
}

fn equipped_decoration_sprite<'a>(
    state: &GameState,
    routine: &str,
    memory: &str,
    sprites: &'a [(&'a str, &'a str)],
) -> Result<Option<&'a str>, SpecialRoutineError> {
    let Some(decoration) = state.script_runtime.memory.get(memory) else {
        return Ok(None);
    };
    if decoration == "0" {
        return Ok(None);
    }
    sprites
        .iter()
        .find_map(|(candidate, sprite)| (decoration == candidate).then_some(*sprite))
        .map(Some)
        .ok_or_else(|| SpecialRoutineError::InvalidState {
            routine: routine.to_string(),
            message: format!("{memory} has invalid equipped decoration {decoration}"),
        })
}

fn toggle_maptile_decorations(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let crate::state::OverworldMemory::Active { map_name, .. } = &state.overworld else {
        return Err(SpecialRoutineError::InvalidState {
            routine: routine.to_string(),
            message: "requires an active overworld map".to_string(),
        });
    };
    let map_name = map_name.clone();

    let bed = equipped_decoration_block(
        state,
        routine,
        "wDecoBed",
        &[
            ("DECO_FEATHERY_BED", 0x1b),
            ("DECO_PINK_BED", 0x1c),
            ("DECO_POLKADOT_BED", 0x1d),
            ("DECO_PIKACHU_BED", 0x1e),
        ],
    )?;
    let plant = equipped_decoration_block(
        state,
        routine,
        "wDecoPlant",
        &[
            ("DECO_MAGNAPLANT", 0x20),
            ("DECO_TROPICPLANT", 0x21),
            ("DECO_JUMBOPLANT", 0x22),
        ],
    )?;
    let poster = equipped_decoration_block(
        state,
        routine,
        "wDecoPoster",
        &[
            ("DECO_TOWN_MAP", 0x1f),
            ("DECO_PIKACHU_POSTER", 0x23),
            ("DECO_CLEFAIRY_POSTER", 0x24),
            ("DECO_JIGGLYPUFF_POSTER", 0x25),
        ],
    )?;
    let carpet = equipped_decoration_block(
        state,
        routine,
        "wDecoCarpet",
        &[
            ("DECO_RED_CARPET", 0x08),
            ("DECO_BLUE_CARPET", 0x0b),
            ("DECO_YELLOW_CARPET", 0x0e),
            ("DECO_GREEN_CARPET", 0x11),
        ],
    )?;
    let outcome = visual_command(
        state,
        routine,
        ScriptGraphicsRuntimeKind::ToggleMaptileDecorations,
    )?;
    {
        let overrides = state.map_block_overrides.entry(map_name).or_default();
        for position in [(0, 2), (3, 2), (3, 0), (0, 0), (0, 1), (1, 1), (2, 1)] {
            overrides.remove(&position);
        }
        if let Some(block) = bed {
            overrides.insert((0, 2), block);
        }
        if let Some(block) = plant {
            overrides.insert((3, 2), block);
        }
        if let Some(block) = poster {
            overrides.insert((3, 0), block);
        }
        if let Some(block) = carpet {
            overrides.insert((0, 0), block);
            overrides.insert((0, 1), block + 1);
            overrides.insert((1, 1), block + 2);
            overrides.insert((2, 1), block + 1);
        }
    }
    state
        .flags
        .set_event_flag("EVENT_PLAYERS_ROOM_POSTER", poster.is_none())
        .map_err(|error| SpecialRoutineError::EventFlag {
            routine: routine.to_string(),
            error,
        })?;
    Ok(outcome)
}

fn toggle_decorations_visibility(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let decorations = [
        (
            "wDecoConsole",
            "SPRITE_CONSOLE",
            "EVENT_PLAYERS_HOUSE_2F_CONSOLE",
            CONSOLE_DECORATION_SPRITES,
        ),
        (
            "wDecoLeftOrnament",
            "SPRITE_DOLL_1",
            "EVENT_PLAYERS_HOUSE_2F_DOLL_1",
            ORNAMENT_DECORATION_SPRITES,
        ),
        (
            "wDecoRightOrnament",
            "SPRITE_DOLL_2",
            "EVENT_PLAYERS_HOUSE_2F_DOLL_2",
            ORNAMENT_DECORATION_SPRITES,
        ),
        (
            "wDecoBigDoll",
            "SPRITE_BIG_DOLL",
            "EVENT_PLAYERS_HOUSE_2F_BIG_DOLL",
            BIG_DOLL_DECORATION_SPRITES,
        ),
    ];
    let resolved = decorations
        .into_iter()
        .map(|(memory, sprite_base, event_flag, sprites)| {
            Ok((
                sprite_base,
                event_flag,
                equipped_decoration_sprite(state, routine, memory, sprites)?,
            ))
        })
        .collect::<Result<Vec<_>, SpecialRoutineError>>()?;
    let outcome = visual_command(
        state,
        routine,
        ScriptGraphicsRuntimeKind::ToggleDecorationsVisibility,
    )?;
    for (sprite_base, event_flag, sprite) in resolved {
        if let Some(sprite) = sprite {
            state
                .script_runtime
                .variable_sprites
                .insert(sprite_base.to_string(), sprite.to_string());
        } else {
            state.script_runtime.variable_sprites.remove(sprite_base);
        }
        state
            .flags
            .set_event_flag(event_flag, sprite.is_none())
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
    let already_discovered = state
        .flags
        .is_engine_flag_set(ENGINE_FLAG)
        .map_err(|error| SpecialRoutineError::EventFlag {
            routine: routine.to_string(),
            error,
        })?;
    let found = state.storage.party.pokemon.iter().flatten().any(|pokemon| {
        !pokemon_is_egg(pokemon)
            && (pokemon.pokerus != 0 || pokemon.status.as_deref() == Some(POKERUS_STATUS))
    });
    let newly_discovered = found && !already_discovered;
    // `_CheckPokerus` is a carry-only party query. The Pokecenter standard
    // script owns `setflag ENGINE_CAUGHT_POKERUS` and the subsequent
    // `specialphonecall SPECIALCALL_POKERUS` after its warning text.
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
    let selected_nickname = required_string_script_variable(state, routine, "_selected_nickname")?;
    let (species, old_nickname, new_nickname) = {
        let pokemon = required_party_pokemon_mut(state, routine, party_slot)?;
        let species = pokemon.species.id.clone();
        let old_nickname = pokemon_nickname_or_species(pokemon);
        let new_nickname =
            if selected_nickname.trim().is_empty() || selected_nickname == old_nickname {
                old_nickname.clone()
            } else {
                pokemon.nickname = selected_nickname.clone();
                selected_nickname
            };
        (species, old_nickname, new_nickname)
    };
    state.sync_party_from_storage();
    state
        .script_runtime
        .named_buffers
        .insert("STRING_BUFFER_1".to_string(), new_nickname.clone());
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
    happiness_data: Option<&HappinessData>,
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
    let happiness_data = require_happiness_data(happiness_data, routine)?;
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
            let delta = happiness_delta(
                happiness_data,
                5, // HAPPINESS_LEARNMOVE
                pokemon.happiness,
                routine,
            )?;
            pokemon.happiness = apply_signed_happiness_delta(pokemon.happiness, delta);
            true
        };
        (species, learned)
    };
    state.sync_party_from_storage();
    // MoveTutor writes FALSE after a successful learn and $ff when its
    // selection loop is cancelled without learning a move.
    set_script_numeric_value(state, if learned { 0 } else { u8::MAX });
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
    let initialized = state.mom_saving_active;
    state.mom_saving_active = true;
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::BankOfMom {
            initialized,
            money: state.money,
            moms_money: state.moms_money,
        },
    })
}

#[derive(Debug, Clone, Copy)]
enum GameCornerGame {
    SlotMachine,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlotStopMode {
    Normal,
    SkipToSeven,
    Slow,
    Golem,
    Chansey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SlotStopResolution {
    mode: SlotStopMode,
    animation_start_offset: usize,
    animation_count: u8,
}

impl SlotStopResolution {
    fn normal(offset: usize) -> Self {
        Self {
            mode: SlotStopMode::Normal,
            animation_start_offset: offset,
            animation_count: 0,
        }
    }
}

impl SlotStopMode {
    fn name(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::SkipToSeven => "skip_to_seven",
            Self::Slow => "slow",
            Self::Golem => "golem",
            Self::Chansey => "chansey",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CardFlipOutcome {
    card_index: usize,
    card_name: String,
    card_level: u8,
    payout: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MemoryGameOutcome {
    matched: bool,
    symbol: Option<String>,
    first_index: usize,
    second_index: usize,
}

fn slot_machine<S>(
    state: &mut GameState,
    item_catalog: &BTreeMap<String, Item>,
    routine: &str,
    divider: &mut S,
) -> Result<SpecialRoutineOutcome, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource + ?Sized,
{
    coin_game_service(
        state,
        item_catalog,
        routine,
        GameCornerGame::SlotMachine,
        divider,
    )
}

fn card_flip<S>(
    state: &mut GameState,
    item_catalog: &BTreeMap<String, Item>,
    routine: &str,
    divider: &mut S,
) -> Result<SpecialRoutineOutcome, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource + ?Sized,
{
    const COIN_CASE: &str = "COIN_CASE";
    let coin_case =
        item_catalog
            .get(COIN_CASE)
            .ok_or_else(|| SpecialRoutineError::UnknownItem {
                routine: routine.to_string(),
                item_id: COIN_CASE.to_string(),
            })?;
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
    let action = state
        .script_runtime
        .variables
        .remove("card_flip_action")
        .ok_or_else(|| SpecialRoutineError::MissingScriptValue {
            routine: routine.to_string(),
            variable: "card_flip_action".to_string(),
        })?;
    let coins_before = state.coins;
    match action.as_str() {
        "start" => {
            let (deck, revealed) = start_card_flip_round(state, routine, divider)?;
            if state.coins < 3 {
                return Ok(SpecialRoutineOutcome {
                    routine: routine.to_string(),
                    effect: SpecialRoutineEffect::GameCornerGameUnavailable {
                        game: routine.to_string(),
                        reason: GameCornerUnavailableReason::NoCoins,
                    },
                });
            }
            state.coins -= 3;
            let coins = state.coins;
            set_script_u32_value(state, u32::from(coins));
            Ok(SpecialRoutineOutcome {
                routine: routine.to_string(),
                effect: SpecialRoutineEffect::CardFlipStarted {
                    coins_before,
                    deck,
                    revealed,
                    coins,
                    random_state_after: state.random_state,
                },
            })
        }
        "shuffle" => {
            let (deck, revealed) = start_card_flip_round(state, routine, divider)?;
            let coins = state.coins;
            set_script_u32_value(state, u32::from(coins));
            Ok(SpecialRoutineOutcome {
                routine: routine.to_string(),
                effect: SpecialRoutineEffect::CardFlipShuffled {
                    deck,
                    revealed,
                    coins,
                    random_state_after: state.random_state,
                },
            })
        }
        "reveal" => {
            let flip = flip_card(state, routine)?;
            let coins = state.coins;
            set_script_u32_value(state, u32::from(coins));
            Ok(SpecialRoutineOutcome {
                routine: routine.to_string(),
                effect: SpecialRoutineEffect::CardFlipRevealed {
                    coins_before,
                    card_index: flip.card_index,
                    card_name: flip.card_name,
                    card_level: flip.card_level,
                    payout: flip.payout,
                    deck: parse_required_comma_tokens(state, routine, "card_flip_deck")?,
                    revealed: parse_required_bool_tokens(state, routine, "card_flip_revealed", 24)?,
                    coins,
                    random_state_after: state.random_state,
                },
            })
        }
        "payout" => {
            state.coins = state.coins.saturating_add(1).min(GAME_CORNER_MAX_COINS);
            let coins = state.coins;
            set_script_u32_value(state, u32::from(coins));
            Ok(SpecialRoutineOutcome {
                routine: routine.to_string(),
                effect: SpecialRoutineEffect::CardFlipPayout {
                    coins_before,
                    coins,
                    random_state_after: state.random_state,
                },
            })
        }
        value => Err(SpecialRoutineError::InvalidState {
            routine: routine.to_string(),
            message: format!(
                "card_flip_action must be start, shuffle, reveal, or payout, found {value}"
            ),
        }
        .into()),
    }
}

fn unused_memory_game<S>(
    state: &mut GameState,
    item_catalog: &BTreeMap<String, Item>,
    routine: &str,
    divider: &mut S,
) -> Result<SpecialRoutineOutcome, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource + ?Sized,
{
    coin_game_service(
        state,
        item_catalog,
        routine,
        GameCornerGame::UnusedMemoryGame,
        divider,
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

fn parse_slot_symbol(value: &str) -> Option<SlotSymbol> {
    match value {
        "SEVEN" => Some(SlotSymbol::Seven),
        "POKEBALL" => Some(SlotSymbol::Pokeball),
        "CHERRY" => Some(SlotSymbol::Cherry),
        "PIKACHU" => Some(SlotSymbol::Pikachu),
        "SQUIRTLE" => Some(SlotSymbol::Squirtle),
        "STARYU" => Some(SlotSymbol::Staryu),
        "NONE" => None,
        _ => None,
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

fn slot_next_byte<S>(rng: &mut CrystalRandom<&mut S>) -> Result<u8, S::Error>
where
    S: DividerSource + ?Sized,
{
    rng.random(false).map(|output| output.value)
}

fn slot_window(reel: &[SlotSymbol; SLOT_REEL_LENGTH], offset: usize) -> [SlotSymbol; 3] {
    [
        reel[offset % SLOT_REEL_LENGTH],
        reel[(offset + 1) % SLOT_REEL_LENGTH],
        reel[(offset + 2) % SLOT_REEL_LENGTH],
    ]
}

fn slot_windows(offsets: [usize; 3]) -> [[SlotSymbol; 3]; 3] {
    [
        slot_window(&SLOT_REELS[0], offsets[0]),
        slot_window(&SLOT_REELS[1], offsets[1]),
        slot_window(&SLOT_REELS[2], offsets[2]),
    ]
}

fn slot_windows_named(offsets: [usize; 3]) -> [[String; 3]; 3] {
    slot_windows(offsets).map(|window| window.map(|symbol| slot_symbol_name(symbol).to_string()))
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

fn slot_bias<S>(
    rng: &mut CrystalRandom<&mut S>,
    lucky: bool,
) -> Result<Option<SlotSymbol>, S::Error>
where
    S: DividerSource + ?Sized,
{
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
    let roll = slot_next_byte(rng)?;
    Ok(table
        .iter()
        .find_map(|(threshold, symbol)| (roll <= *threshold).then_some(*symbol))
        .flatten())
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

fn slot_stop_reel2<S>(
    offsets: &mut [usize; 3],
    bias: Option<SlotSymbol>,
    bet: u8,
    rng: &mut CrystalRandom<&mut S>,
) -> Result<SlotStopResolution, S::Error>
where
    S: DividerSource + ?Sized,
{
    let animation_start_offset = offsets[1];
    if bet >= 2 && (bias.is_none() || bias == Some(SlotSymbol::Seven)) {
        // Slots_StopReel2 checks the visible seven before calling Random.  The
        // ordering is observable because a failed eligibility check consumes
        // no random byte in the original engine.
        if slot_window(&SLOT_REELS[0], offsets[0]).contains(&SlotSymbol::Seven)
            && slot_next_byte(rng)? < SLOT_PERCENT_31 + 1
            && let Some(aligned) = slot_attempt_skip_to_seven(*offsets, bet)
        {
            *offsets = aligned;
            return Ok(SlotStopResolution {
                mode: SlotStopMode::SkipToSeven,
                animation_start_offset,
                animation_count: 0,
            });
        }
    }

    let mut counter = 4;
    loop {
        let windows = [
            slot_window(&SLOT_REELS[0], offsets[0]),
            slot_window(&SLOT_REELS[1], offsets[1]),
        ];
        let (matched_symbol, _) = slot_check_first_two(&windows, bet);
        if matched_symbol.is_some() && matched_symbol == bias {
            return Ok(SlotStopResolution::normal(animation_start_offset));
        }
        if bias.is_none() || counter == 0 {
            return Ok(SlotStopResolution::normal(animation_start_offset));
        }
        offsets[1] = slot_advance(offsets[1], 1);
        counter -= 1;
    }
}

fn slot_reel3_match(offsets: [usize; 3], bet: u8) -> Option<SlotSymbol> {
    slot_check_all_three(&slot_windows(offsets), bet).0
}

fn slot_advance_reel3_until_no_match(offsets: &mut [usize; 3], bet: u8) {
    for _ in 0..SLOT_REEL_LENGTH {
        if slot_reel3_match(*offsets, bet).is_none() {
            return;
        }
        offsets[2] = slot_advance(offsets[2], 1);
    }
    unreachable!("every reel-three offset cannot match fixed first-two windows")
}

fn slot_apply_reel3_slow(offsets: &mut [usize; 3], bias: Option<SlotSymbol>, bet: u8) -> u8 {
    let target = (bias == Some(SlotSymbol::Seven)).then_some(SlotSymbol::Seven);
    let mut count = 0_u8;
    loop {
        if count >= 17 {
            let matched = slot_reel3_match(*offsets, bet);
            if matched == target {
                return count;
            }
        }
        offsets[2] = slot_advance(offsets[2], 1);
        count += 1;
    }
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

fn slot_apply_reel3_golem<S>(
    offsets: &mut [usize; 3],
    bias: Option<SlotSymbol>,
    bet: u8,
    rng: &mut CrystalRandom<&mut S>,
) -> Result<u8, S::Error>
where
    S: DividerSource + ?Sized,
{
    if bias == Some(SlotSymbol::Seven) {
        let mut count = 0_u8;
        loop {
            offsets[2] = slot_advance(offsets[2], 1);
            count += 1;
            if slot_reel3_match(*offsets, bet) == Some(SlotSymbol::Seven) {
                return Ok(count);
            }
        }
    }
    let mut stride = 0;
    while stride < 4 {
        stride = slot_next_byte(rng)? & 0x7;
    }
    let initial_offset = offsets[2];
    let mut simulated_offset = initial_offset;
    let mut count = stride;
    loop {
        let step = count;
        count += 1;
        simulated_offset = slot_advance(simulated_offset, usize::from(step));
        let mut simulated = *offsets;
        simulated[2] = simulated_offset;
        if slot_reel3_match(simulated, bet).is_none() {
            offsets[2] = slot_advance(initial_offset, usize::from(count));
            return Ok(count);
        }
    }
}

fn slot_apply_reel3_chansey(offsets: &mut [usize; 3], bet: u8) -> u8 {
    let mut count = 0_u8;
    loop {
        offsets[2] = slot_advance(offsets[2], 17);
        count += 1;
        if slot_reel3_match(*offsets, bet) == Some(SlotSymbol::Seven) {
            return count;
        }
    }
}

fn slot_stop_reel3<S>(
    offsets: &mut [usize; 3],
    bias: Option<SlotSymbol>,
    bet: u8,
    rng: &mut CrystalRandom<&mut S>,
) -> Result<SlotStopResolution, S::Error>
where
    S: DividerSource + ?Sized,
{
    let windows_first_two = [
        slot_window(&SLOT_REELS[0], offsets[0]),
        slot_window(&SLOT_REELS[1], offsets[1]),
    ];
    let (matched_symbol, saw_seven) = slot_check_first_two(&windows_first_two, bet);
    if matched_symbol.is_none() || !saw_seven {
        let animation_start_offset = offsets[2];
        slot_apply_reel3_stop(offsets, bias, bet);
        return Ok(SlotStopResolution::normal(animation_start_offset));
    }
    let action = if bias == Some(SlotSymbol::Seven) {
        let roll = slot_next_byte(rng)?;
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
        let roll = slot_next_byte(rng)?;
        if roll >= SLOT_PERCENT_63 {
            "stop"
        } else if roll >= SLOT_PERCENT_31 + 1 {
            "slow"
        } else {
            "golem"
        }
    };
    let resolution = match action {
        "stop" => {
            let animation_start_offset = offsets[2];
            slot_apply_reel3_stop(offsets, bias, bet);
            SlotStopResolution::normal(animation_start_offset)
        }
        "slow" => {
            slot_advance_reel3_until_no_match(offsets, bet);
            let animation_start_offset = offsets[2];
            let animation_count = slot_apply_reel3_slow(offsets, bias, bet);
            SlotStopResolution {
                mode: SlotStopMode::Slow,
                animation_start_offset,
                animation_count,
            }
        }
        "golem" => {
            slot_advance_reel3_until_no_match(offsets, bet);
            let animation_start_offset = offsets[2];
            let animation_count = slot_apply_reel3_golem(offsets, bias, bet, rng)?;
            SlotStopResolution {
                mode: SlotStopMode::Golem,
                animation_start_offset,
                animation_count,
            }
        }
        "chansey" => {
            slot_advance_reel3_until_no_match(offsets, bet);
            let animation_start_offset = offsets[2];
            let animation_count = slot_apply_reel3_chansey(offsets, bet);
            SlotStopResolution {
                mode: SlotStopMode::Chansey,
                animation_start_offset,
                animation_count,
            }
        }
        _ => unreachable!("slot action is selected from static branches"),
    };
    Ok(resolution)
}

fn parse_required_comma_tokens(
    state: &GameState,
    routine: &str,
    variable: &str,
) -> Result<Vec<String>, SpecialRoutineError> {
    let raw = state
        .script_runtime
        .variables
        .get(variable)
        .ok_or_else(|| SpecialRoutineError::MissingScriptValue {
            routine: routine.to_string(),
            variable: variable.to_string(),
        })?;
    let values = raw
        .split(',')
        .map(str::trim)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if values.is_empty() || values.iter().any(String::is_empty) {
        return Err(SpecialRoutineError::InvalidState {
            routine: routine.to_string(),
            message: format!("{variable} must be a nonempty comma-separated list"),
        });
    }
    Ok(values)
}

fn parse_slot_offsets(state: &GameState, routine: &str) -> Result<[usize; 3], SpecialRoutineError> {
    let values = parse_required_comma_tokens(state, routine, "slot_offsets")?;
    if values.len() != 3 {
        return Err(SpecialRoutineError::InvalidState {
            routine: routine.to_string(),
            message: "slot_offsets must contain exactly 3 entries".to_string(),
        });
    }
    let mut offsets = [0; 3];
    for (index, value) in values.into_iter().enumerate() {
        let offset =
            value
                .parse::<usize>()
                .map_err(|_| SpecialRoutineError::InvalidNumericValue {
                    routine: routine.to_string(),
                    value: value.clone(),
                })?;
        if offset >= SLOT_REEL_LENGTH {
            return Err(SpecialRoutineError::InvalidState {
                routine: routine.to_string(),
                message: format!("slot offset {offset} must be below {SLOT_REEL_LENGTH}"),
            });
        }
        offsets[index] = offset;
    }
    Ok(offsets)
}

fn write_slot_offsets(state: &mut GameState, offsets: [usize; 3]) {
    state.script_runtime.variables.insert(
        "slot_offsets".to_string(),
        offsets.map(|offset| offset.to_string()).join(","),
    );
}

fn parse_slot_bias_state(
    state: &GameState,
    routine: &str,
) -> Result<Option<SlotSymbol>, SpecialRoutineError> {
    let value = state
        .script_runtime
        .variables
        .get("slot_bias")
        .ok_or_else(|| SpecialRoutineError::MissingScriptValue {
            routine: routine.to_string(),
            variable: "slot_bias".to_string(),
        })?;
    if value == "NONE" {
        return Ok(None);
    }
    parse_slot_symbol(value)
        .ok_or_else(|| SpecialRoutineError::InvalidState {
            routine: routine.to_string(),
            message: format!("invalid slot_bias {value}"),
        })
        .map(Some)
}

fn parse_required_bool_tokens(
    state: &GameState,
    routine: &str,
    variable: &str,
    len: usize,
) -> Result<Vec<bool>, SpecialRoutineError> {
    let values = parse_required_comma_tokens(state, routine, variable)?;
    if values.len() != len {
        return Err(SpecialRoutineError::InvalidState {
            routine: routine.to_string(),
            message: format!("{variable} must contain exactly {len} entries"),
        });
    }
    values
        .into_iter()
        .map(|value| match value.as_str() {
            "0" => Ok(false),
            "1" => Ok(true),
            _ => Err(SpecialRoutineError::InvalidState {
                routine: routine.to_string(),
                message: format!("{variable} entries must be exactly 0 or 1"),
            }),
        })
        .collect()
}

fn required_game_index(
    state: &GameState,
    routine: &str,
    variable: &str,
    len: usize,
) -> Result<usize, SpecialRoutineError> {
    let raw = state
        .script_runtime
        .variables
        .get(variable)
        .ok_or_else(|| SpecialRoutineError::MissingScriptValue {
            routine: routine.to_string(),
            variable: variable.to_string(),
        })?;
    let index = raw
        .parse::<usize>()
        .map_err(|_| SpecialRoutineError::InvalidNumericValue {
            routine: routine.to_string(),
            value: raw.clone(),
        })?;
    if index >= len {
        return Err(SpecialRoutineError::InvalidState {
            routine: routine.to_string(),
            message: format!("{variable} index {index} is outside board length {len}"),
        });
    }
    Ok(index)
}

fn card_flip_identity(face: u8) -> (&'static str, u8) {
    let species = match face & 3 {
        0 => "PIKACHU",
        1 => "JIGGLYPUFF",
        2 => "POLIWAG",
        _ => "ODDISH",
    };
    (species, (face >> 2) + 1)
}

fn card_flip_payout(face: u8, cursor_x: usize, cursor_y: usize) -> u16 {
    let species = usize::from(face & 3);
    let level = usize::from((face >> 2) + 1);
    match (cursor_x, cursor_y) {
        (2 | 3, 0) if species < 2 => 6,
        (4 | 5, 0) if species >= 2 => 6,
        (2..=5, 1) if species == cursor_x - 2 => 12,
        (0, 2 | 3) if level <= 2 => 9,
        (0, 4 | 5) if (3..=4).contains(&level) => 9,
        (0, 6 | 7) if level >= 5 => 9,
        (1, 2..=7) if level == cursor_y - 1 => 18,
        (2..=5, 2..=7) if species == cursor_x - 2 && level == cursor_y - 1 => 72,
        _ => 0,
    }
}

fn start_card_flip_round<S>(
    state: &mut GameState,
    routine: &str,
    divider: &mut S,
) -> Result<(Vec<String>, Vec<bool>), RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource + ?Sized,
{
    let initialize = match state
        .script_runtime
        .variables
        .get("card_flip_initialize")
        .map(String::as_str)
    {
        Some("1") => true,
        Some(value) => {
            return Err(SpecialRoutineError::InvalidState {
                routine: routine.to_string(),
                message: format!("card_flip_initialize must be exactly 1, found {value}"),
            }
            .into());
        }
        None => false,
    };
    let deck = if initialize {
        let mut rng = CrystalRandom::new(state.random_state, &mut *divider);
        let mut deck = vec![0_u8; 24];
        for face in (1_u8..24).rev() {
            loop {
                // ByteFill enters the first call with carry clear. Every
                // retry follows either AND $1f or an occupancy AND, which
                // also clears carry.
                let index = usize::from(
                    rng.random(false)
                        .map_err(RandomSpecialRoutineError::Divider)?
                        .value
                        & 0x1f,
                );
                if index < deck.len() && deck[index] == 0 {
                    deck[index] = face;
                    break;
                }
            }
        }
        state.random_state = rng.state();
        deck.into_iter().map(|face| face.to_string()).collect()
    } else {
        parse_required_comma_tokens(state, routine, "card_flip_deck")?
    };
    validate_card_flip_deck(&deck, routine)?;
    let revealed = if initialize {
        vec![false; deck.len()]
    } else {
        parse_required_bool_tokens(state, routine, "card_flip_revealed", deck.len())?
    };
    state
        .script_runtime
        .variables
        .insert("card_flip_deck".to_string(), deck.join(","));
    state
        .script_runtime
        .variables
        .remove("card_flip_initialize");
    store_card_flip_revealed(state, &revealed);
    Ok((deck, revealed))
}

fn validate_card_flip_deck(deck: &[String], routine: &str) -> Result<Vec<u8>, SpecialRoutineError> {
    if deck.len() != 24 {
        return Err(SpecialRoutineError::InvalidState {
            routine: routine.to_string(),
            message: format!(
                "card_flip_deck must contain exactly 24 cards, found {}",
                deck.len()
            ),
        });
    }
    let encoded_deck = deck
        .iter()
        .map(|value| value.parse::<u8>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| SpecialRoutineError::InvalidState {
            routine: routine.to_string(),
            message: "card_flip_deck must contain encoded card identities 0 through 23".to_string(),
        })?;
    let mut identities = encoded_deck.clone();
    identities.sort_unstable();
    if identities != (0_u8..24).collect::<Vec<_>>() {
        return Err(SpecialRoutineError::InvalidState {
            routine: routine.to_string(),
            message: "card_flip_deck must contain each encoded card identity exactly once"
                .to_string(),
        });
    }
    Ok(encoded_deck)
}

fn store_card_flip_revealed(state: &mut GameState, revealed: &[bool]) {
    state.script_runtime.variables.insert(
        "card_flip_revealed".to_string(),
        revealed
            .iter()
            .map(|flag| if *flag { "1" } else { "0" })
            .collect::<Vec<_>>()
            .join(","),
    );
}

fn flip_card(state: &mut GameState, routine: &str) -> Result<CardFlipOutcome, SpecialRoutineError> {
    let deck = parse_required_comma_tokens(state, routine, "card_flip_deck")?;
    let encoded_deck = validate_card_flip_deck(&deck, routine)?;
    let mut revealed =
        parse_required_bool_tokens(state, routine, "card_flip_revealed", deck.len())?;
    let card_index = required_game_index(state, routine, "card_flip_index", deck.len())?;
    if revealed[card_index] {
        return Err(SpecialRoutineError::InvalidState {
            routine: routine.to_string(),
            message: format!("card_flip_index {card_index} has already been revealed"),
        });
    }
    revealed[card_index] = true;
    let face = encoded_deck[card_index];
    let (card_name, card_level) = card_flip_identity(face);
    let cursor_x = required_game_index(state, routine, "card_flip_bet_x", 6)?;
    let cursor_y = required_game_index(state, routine, "card_flip_bet_y", 8)?;
    let payout = card_flip_payout(face, cursor_x, cursor_y);
    store_card_flip_revealed(state, &revealed);
    Ok(CardFlipOutcome {
        card_index,
        card_name: card_name.to_string(),
        card_level,
        payout,
    })
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

fn play_memory_game<S>(
    state: &mut GameState,
    routine: &str,
    divider: &mut S,
) -> Result<MemoryGameOutcome, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource + ?Sized,
{
    const MEMORY_SYMBOLS: [&str; 8] = [
        "ODDISH",
        "POLIWAG",
        "PIKACHU",
        "JIGGLYPUFF",
        "RATTATA",
        "VOLTORB",
        "DITTO",
        "ELECTABUZZ",
    ];
    let mut rng = CrystalRandom::new(state.random_state, divider);
    let mut generated_board = MEMORY_SYMBOLS
        .into_iter()
        .flat_map(|symbol| [symbol.to_string(), symbol.to_string()])
        .collect::<Vec<_>>();
    for index in (1..generated_board.len()).rev() {
        let swap_index = usize::from(
            rng.random(false)
                .map_err(RandomSpecialRoutineError::Divider)?
                .value,
        ) % (index + 1);
        generated_board.swap(index, swap_index);
    }
    let board = if state.script_runtime.variables.contains_key("memory_board") {
        parse_required_comma_tokens(state, routine, "memory_board")?
    } else {
        // TypeScript constructs a shuffled MemoryGame and explicitly shuffles
        // it once more when no serialized board is available.
        for index in (1..generated_board.len()).rev() {
            let swap_index = usize::from(
                rng.random(false)
                    .map_err(RandomSpecialRoutineError::Divider)?
                    .value,
            ) % (index + 1);
            generated_board.swap(index, swap_index);
        }
        generated_board
    };
    state.random_state = rng.state();
    if board.len() != 16 {
        return Err(SpecialRoutineError::InvalidState {
            routine: routine.to_string(),
            message: format!(
                "memory_board must contain exactly 16 cards, found {}",
                board.len()
            ),
        }
        .into());
    }
    let mut revealed = if state
        .script_runtime
        .variables
        .contains_key("memory_revealed")
    {
        parse_required_bool_tokens(state, routine, "memory_revealed", board.len())?
    } else {
        vec![false; board.len()]
    };
    let requested_first = required_game_index(state, routine, "memory_first", board.len())?;
    let requested_second = required_game_index(state, routine, "memory_second", board.len())?;
    let outcome =
        memory_reveal(&board, &mut revealed, requested_first, requested_second).map_err(|_| {
            SpecialRoutineError::InvalidState {
                routine: routine.to_string(),
                message: "memory selections must be distinct unrevealed cards".to_string(),
            }
        })?;

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

fn coin_game_service<S>(
    state: &mut GameState,
    item_catalog: &BTreeMap<String, Item>,
    routine: &str,
    game: GameCornerGame,
    divider: &mut S,
) -> Result<SpecialRoutineOutcome, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource + ?Sized,
{
    let before = state.clone();
    match coin_game_service_inner(state, item_catalog, routine, game, divider) {
        Ok(outcome) => Ok(outcome),
        Err(error) => {
            *state = before;
            Err(error)
        }
    }
}

fn coin_game_service_inner<S>(
    state: &mut GameState,
    item_catalog: &BTreeMap<String, Item>,
    routine: &str,
    game: GameCornerGame,
    divider: &mut S,
) -> Result<SpecialRoutineOutcome, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource + ?Sized,
{
    const COIN_CASE: &str = "COIN_CASE";
    let coin_case =
        item_catalog
            .get(COIN_CASE)
            .ok_or_else(|| SpecialRoutineError::UnknownItem {
                routine: routine.to_string(),
                item_id: COIN_CASE.to_string(),
            })?;
    let slot_action = if matches!(game, GameCornerGame::SlotMachine) {
        state.script_runtime.variables.get("slot_action").cloned()
    } else {
        None
    };
    if state.coins == 0
        && (!matches!(game, GameCornerGame::SlotMachine)
            || slot_action
                .as_deref()
                .is_none_or(|action| action == "start"))
    {
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
    let slot_action = if matches!(game, GameCornerGame::SlotMachine) {
        Some(
            state
                .script_runtime
                .variables
                .remove("slot_action")
                .ok_or_else(|| SpecialRoutineError::MissingScriptValue {
                    routine: routine.to_string(),
                    variable: "slot_action".to_string(),
                })?,
        )
    } else {
        None
    };

    state
        .script_runtime
        .variables
        .insert("_coin_case_balance".to_string(), state.coins.to_string());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    state.script_runtime.active_menu = None;
    let effect = match game {
        GameCornerGame::SlotMachine => {
            let coins_before = state.coins;
            let action = slot_action
                .as_deref()
                .expect("SlotMachine action was required above");
            if action == "payout" {
                let raw_remaining = state
                    .script_runtime
                    .variables
                    .get("slot_payout_remaining")
                    .ok_or_else(|| SpecialRoutineError::MissingScriptValue {
                        routine: routine.to_string(),
                        variable: "slot_payout_remaining".to_string(),
                    })?;
                let remaining = raw_remaining.parse::<u16>().map_err(|_| {
                    SpecialRoutineError::InvalidNumericValue {
                        routine: routine.to_string(),
                        value: raw_remaining.clone(),
                    }
                })?;
                if remaining == 0 {
                    return Err(SpecialRoutineError::InvalidState {
                        routine: routine.to_string(),
                        message: "slot_payout_remaining must be positive for payout".to_string(),
                    }
                    .into());
                }
                let payout_remaining = remaining - 1;
                state.script_runtime.variables.insert(
                    "slot_payout_remaining".to_string(),
                    payout_remaining.to_string(),
                );
                state.coins = state.coins.saturating_add(1).min(GAME_CORNER_MAX_COINS);
                let coins = state.coins;
                set_script_u32_value(state, u32::from(coins));
                return Ok(SpecialRoutineOutcome {
                    routine: routine.to_string(),
                    effect: SpecialRoutineEffect::SlotMachinePayout {
                        coins_before,
                        payout_remaining,
                        coins,
                        random_state_after: state.random_state,
                    },
                });
            }
            let raw_bet = state
                .script_runtime
                .variables
                .get("slot_bet")
                .ok_or_else(|| SpecialRoutineError::MissingScriptValue {
                    routine: routine.to_string(),
                    variable: "slot_bet".to_string(),
                })?;
            let bet =
                raw_bet
                    .parse::<u8>()
                    .map_err(|_| SpecialRoutineError::InvalidNumericValue {
                        routine: routine.to_string(),
                        value: raw_bet.clone(),
                    })?;
            if !(1..=3).contains(&bet) {
                return Err(SpecialRoutineError::InvalidState {
                    routine: routine.to_string(),
                    message: format!("slot_bet must be between 1 and 3, found {bet}"),
                }
                .into());
            }
            if action == "start" && coins_before < u16::from(bet) {
                return Err(SpecialRoutineError::InvalidState {
                    routine: routine.to_string(),
                    message: format!(
                        "slot_bet {bet} exceeds the current coin balance {coins_before}"
                    ),
                }
                .into());
            }
            match action {
                "start" => {
                    let offsets = if state.script_runtime.variables.contains_key("slot_offsets") {
                        parse_slot_offsets(state, routine)?
                    } else {
                        // Slots_InitReelTiles renders position 14 before storing
                        // the next position.  This happens once on entry.
                        [SLOT_REEL_LENGTH - 1; 3]
                    };
                    let lucky = state
                        .script_runtime
                        .variables
                        .get("slot_mode")
                        .is_some_and(|mode| mode.eq_ignore_ascii_case("lucky"))
                        || matches!(
                            state.script_runtime.script_value.as_deref(),
                            Some("1") | Some("TRUE") | Some("true")
                        );
                    let retained_bias = state
                        .script_runtime
                        .variables
                        .get("slot_bias")
                        .filter(|value| value.as_str() == "SEVEN")
                        .map(|_| SlotSymbol::Seven);
                    let mut rng = CrystalRandom::new(state.random_state, divider);
                    if !state
                        .script_runtime
                        .variables
                        .contains_key("slot_keep_seven_bias_chance")
                    {
                        let keep_chance = slot_next_byte(&mut rng)
                            .map_err(RandomSpecialRoutineError::Divider)?
                            & 0x2a
                            == 0;
                        state.script_runtime.variables.insert(
                            "slot_keep_seven_bias_chance".to_string(),
                            u8::from(keep_chance).to_string(),
                        );
                    }
                    let bias = if retained_bias.is_some() {
                        retained_bias
                    } else {
                        slot_bias(&mut rng, lucky).map_err(RandomSpecialRoutineError::Divider)?
                    };
                    state.random_state = rng.state();
                    state.coins = coins_before - u16::from(bet);
                    state.script_runtime.variables.insert(
                        "slot_bias".to_string(),
                        bias.map(slot_symbol_name).unwrap_or("NONE").to_string(),
                    );
                    state
                        .script_runtime
                        .variables
                        .insert("slot_next_reel".to_string(), "1".to_string());
                    state
                        .script_runtime
                        .variables
                        .insert("slot_payout_remaining".to_string(), "0".to_string());
                    write_slot_offsets(state, offsets);
                    let coins = state.coins;
                    set_script_u32_value(state, u32::from(coins));
                    SpecialRoutineEffect::SlotMachineStarted {
                        coins_before,
                        bet,
                        bias: bias.map(slot_symbol_name).map(str::to_string),
                        offsets,
                        windows: slot_windows_named(offsets),
                        coins,
                        random_state_after: state.random_state,
                    }
                }
                "stop" => {
                    let raw_reel =
                        state
                            .script_runtime
                            .variables
                            .get("slot_reel")
                            .ok_or_else(|| SpecialRoutineError::MissingScriptValue {
                                routine: routine.to_string(),
                                variable: "slot_reel".to_string(),
                            })?;
                    let reel = raw_reel.parse::<u8>().map_err(|_| {
                        SpecialRoutineError::InvalidNumericValue {
                            routine: routine.to_string(),
                            value: raw_reel.clone(),
                        }
                    })?;
                    let raw_next = state
                        .script_runtime
                        .variables
                        .get("slot_next_reel")
                        .ok_or_else(|| SpecialRoutineError::MissingScriptValue {
                            routine: routine.to_string(),
                            variable: "slot_next_reel".to_string(),
                        })?;
                    let next_reel = raw_next.parse::<u8>().map_err(|_| {
                        SpecialRoutineError::InvalidNumericValue {
                            routine: routine.to_string(),
                            value: raw_next.clone(),
                        }
                    })?;
                    if reel != next_reel || !(1..=3).contains(&reel) {
                        return Err(SpecialRoutineError::InvalidState {
                            routine: routine.to_string(),
                            message: format!(
                                "slot_reel must be the next reel {next_reel}, found {reel}"
                            ),
                        }
                        .into());
                    }
                    let mut offsets = parse_slot_offsets(state, routine)?;
                    let bias = parse_slot_bias_state(state, routine)?;
                    let mut rng = CrystalRandom::new(state.random_state, divider);
                    let resolution = match reel {
                        1 => {
                            let animation_start_offset = offsets[0];
                            offsets[0] = slot_stop_reel1(offsets[0], bias);
                            SlotStopResolution::normal(animation_start_offset)
                        }
                        2 => slot_stop_reel2(&mut offsets, bias, bet, &mut rng)
                            .map_err(RandomSpecialRoutineError::Divider)?,
                        3 => slot_stop_reel3(&mut offsets, bias, bet, &mut rng)
                            .map_err(RandomSpecialRoutineError::Divider)?,
                        _ => unreachable!("validated slot reel"),
                    };
                    state.random_state = rng.state();
                    write_slot_offsets(state, offsets);
                    state
                        .script_runtime
                        .variables
                        .insert("slot_next_reel".to_string(), (reel + 1).to_string());
                    let windows = slot_windows(offsets);
                    let coins = state.coins;
                    set_script_u32_value(state, u32::from(coins));
                    SpecialRoutineEffect::SlotMachineReelStopped {
                        reel,
                        mode: resolution.mode.name().to_string(),
                        animation_start_offset: resolution.animation_start_offset,
                        animation_count: resolution.animation_count,
                        offsets,
                        windows: windows.map(|window| {
                            window.map(|symbol| slot_symbol_name(symbol).to_string())
                        }),
                        coins,
                        random_state_after: state.random_state,
                    }
                }
                "result" => {
                    let next_reel = state
                        .script_runtime
                        .variables
                        .get("slot_next_reel")
                        .and_then(|value| value.parse::<u8>().ok());
                    if next_reel != Some(4) {
                        return Err(SpecialRoutineError::InvalidState {
                            routine: routine.to_string(),
                            message: "slot result requires all three reels to be stopped"
                                .to_string(),
                        }
                        .into());
                    }
                    let offsets = parse_slot_offsets(state, routine)?;
                    let windows = slot_windows(offsets);
                    let (matched_symbol, winning_lines) = slot_check_all_three(&windows, bet);
                    let payout = matched_symbol.map(slot_symbol_payout).unwrap_or(0);
                    let mut rng = CrystalRandom::new(state.random_state, divider);
                    if matched_symbol == Some(SlotSymbol::Seven) {
                        let keep_chance = state
                            .script_runtime
                            .variables
                            .get("slot_keep_seven_bias_chance")
                            .ok_or_else(|| SpecialRoutineError::MissingScriptValue {
                                routine: routine.to_string(),
                                variable: "slot_keep_seven_bias_chance".to_string(),
                            })?;
                        let mask = match keep_chance.as_str() {
                            "0" => 0x14,
                            "1" => 0x1c,
                            value => {
                                return Err(SpecialRoutineError::InvalidState {
                                    routine: routine.to_string(),
                                    message: format!(
                                        "slot_keep_seven_bias_chance must be 0 or 1, found {value}"
                                    ),
                                }
                                .into());
                            }
                        };
                        if slot_next_byte(&mut rng).map_err(RandomSpecialRoutineError::Divider)?
                            & mask
                            != 0
                        {
                            state
                                .script_runtime
                                .variables
                                .insert("slot_bias".to_string(), "NONE".to_string());
                        }
                    }
                    state.random_state = rng.state();
                    state
                        .script_runtime
                        .variables
                        .insert("slot_payout_remaining".to_string(), payout.to_string());
                    let coins = state.coins;
                    set_script_u32_value(state, u32::from(coins));
                    SpecialRoutineEffect::SlotMachineResult {
                        payout,
                        matched_symbol: matched_symbol.map(slot_symbol_name).map(str::to_string),
                        winning_lines,
                        coins,
                        random_state_after: state.random_state,
                    }
                }
                value => {
                    return Err(SpecialRoutineError::InvalidState {
                        routine: routine.to_string(),
                        message: format!(
                            "slot_action must be start, stop, result, or payout, found {value}"
                        ),
                    }
                    .into());
                }
            }
        }
        GameCornerGame::UnusedMemoryGame => {
            let outcome = play_memory_game(state, routine, divider)?;
            set_script_bool_value(state, outcome.matched);
            SpecialRoutineEffect::UnusedMemoryGame {
                matched: outcome.matched,
                symbol: outcome.symbol,
                first_index: outcome.first_index,
                second_index: outcome.second_index,
                coins: state.coins,
                random_state_after: state.random_state,
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
    let enabled = state.mystery_gift.trainer_house_flag;
    set_script_bool_value(state, enabled);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::TrainerHouse { enabled },
    })
}

fn photo_studio(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let party_slot = required_usize_script_variable(state, routine, "_party_slot")?;
    let pokemon = required_party_pokemon(state, routine, party_slot)?;
    let species = pokemon.species.id.clone();
    if !pokemon_is_egg(pokemon) {
        state.script_runtime.active_pokemon_picture = Some(species.clone());
        state
            .script_runtime
            .named_buffers
            .insert("STRING_BUFFER_1".to_string(), species.clone());
    }
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::PhotoStudio {
            party_slot: Some(party_slot),
            species: Some(species),
        },
    })
}

fn battle_tower_challenge_explanation_cancel(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let english = required_numeric_script_value(state, routine)? != 0;
    let selection = optional_u8_script_variable(state, routine, "_battle_tower_challenge_choice")?;
    if let Some(selection) = selection {
        if !(1..=4).contains(&selection) {
            return Err(SpecialRoutineError::InvalidState {
                routine: routine.to_string(),
                message: format!(
                    "Battle Tower challenge menu selection must be 1..=4, found {selection}"
                ),
            });
        }
        state.script_runtime.active_menu = None;
        set_script_numeric_value(state, selection);
    } else {
        state.script_runtime.active_menu = Some(routine.to_string());
        // The source initializes wScriptVar to its cancel result before VerticalMenu.
        set_script_numeric_value(state, 4);
    }
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::BattleTowerChallengeExplanationCancel { english, selection },
    })
}

fn give_shuckle<S>(
    state: &mut GameState,
    context: SpecialRoutineContext<'_>,
    routine: &str,
    divider: &mut S,
) -> Result<SpecialRoutineOutcome, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource + ?Sized,
{
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
        }
        .into());
    }
    // TryAddMonToParty returns before GeneratePartyMonStats when the party is
    // full, so the capacity failure consumes no DIV samples.
    if state.storage.party.next_open_slot().is_none() {
        set_script_bool_value(state, false);
        state.script_runtime.last_special_routine = Some(routine.to_string());
        return Ok(SpecialRoutineOutcome {
            routine: routine.to_string(),
            effect: SpecialRoutineEffect::GiveShuckle {
                stored: false,
                random_state_after: state.random_state,
            },
        });
    }
    let dvs = sample_shuckie_dvs(state, divider)?;
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
                random_state_after: state.random_state,
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
            random_state_after: state.random_state,
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
    catalog: &RoamingPokemonCatalog,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    if catalog.is_empty() {
        return Err(SpecialRoutineError::MissingRoamingPokemonDefinitions {
            routine: routine.to_string(),
        });
    }
    for write in &catalog.init_writes {
        required_species_metadata(species_catalog, routine, &write.species)?;
        let slot = &mut state.roaming_pokemon[usize::from(write.slot)];
        slot.species = Some(write.species.clone());
        slot.level = write.level;
        slot.map_group = write.map_group;
        slot.map_number = write.map_number;
        slot.hp = write.hp;
    }
    let roamers = state.roaming_pokemon.clone();
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

fn buenas_password<S>(
    state: &mut GameState,
    categories: &BuenaPasswordCategories,
    routine: &str,
    divider: &mut S,
) -> Result<SpecialRoutineOutcome, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource + ?Sized,
{
    let (category_id, category, correct) =
        ensure_buenas_password(state, categories, routine, divider)?;
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
            }
            .into());
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
            options: category.options.clone(),
            correct,
            guess,
            matched,
            random_state_after: state.random_state,
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
        "VAR_BLUECARDBALANCE".to_string(),
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

fn day_care_interaction<S>(
    state: &mut GameState,
    routine: &str,
    caretaker: &str,
    divider: &mut S,
) -> Result<SpecialRoutineOutcome, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource + ?Sized,
{
    let action = required_string_script_variable(state, routine, "_day_care_action")?;
    let mut rng = CrystalRandom::new(state.random_state, divider);
    let outcome = match action.as_str() {
        "open" => crate::state::DayCareInteractionState {
            caretaker: caretaker.to_string(),
            action: "open".to_string(),
            success: true,
            pokemon: None,
            level: None,
            reason: None,
        },
        "deposit" => day_care_deposit(state, routine, caretaker, &mut rng)?,
        "withdraw" => day_care_withdraw(state, routine, caretaker, &mut rng)?,
        "collect_egg" => day_care_collect_egg(state, routine, &mut rng)?,
        "inspect" => day_care_inspect_interaction(state, routine, caretaker)?,
        exact => {
            return Err(SpecialRoutineError::MissingScriptValue {
                routine: routine.to_string(),
                variable: format!("unsupported _day_care_action {exact}"),
            }
            .into());
        }
    };
    state.random_state = rng.state();
    // The intro routines set their active bits before the player's YES/NO
    // choice; a successful deposit and an inspection retain the same flag.
    if action == "open" || (action == "deposit" && outcome.success) || action == "inspect" {
        set_day_care_active(state, routine, caretaker, true)?;
    }
    if action != "open" {
        set_script_bool_value(state, outcome.success);
        state.day_care.last_interaction = Some(outcome.clone());
    }
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

fn day_care_collect_egg<S>(
    state: &mut GameState,
    _routine: &str,
    rng: &mut CrystalRandom<S>,
) -> Result<crate::state::DayCareInteractionState, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource,
{
    let Some(egg) = state.day_care.egg.clone() else {
        return Ok(crate::state::DayCareInteractionState {
            caretaker: "man".to_string(),
            action: "collect_egg".to_string(),
            success: false,
            pokemon: None,
            level: None,
            reason: Some("no_egg".to_string()),
        });
    };
    if !state.storage.party.has_space() {
        return Ok(crate::state::DayCareInteractionState {
            caretaker: "man".to_string(),
            action: "collect_egg".to_string(),
            success: false,
            pokemon: Some(egg.species.id),
            level: Some(egg.level),
            reason: Some("party_full".to_string()),
        });
    }
    let species = egg.species.id.clone();
    let level = egg.level;
    if !state.storage.party.add_pokemon(egg) {
        return Ok(crate::state::DayCareInteractionState {
            caretaker: "man".to_string(),
            action: "collect_egg".to_string(),
            success: false,
            pokemon: Some(species),
            level: Some(level),
            reason: Some("party_full".to_string()),
        });
    }
    state.sync_party_from_storage();
    state.day_care.egg = None;
    state.day_care.egg_present = false;
    state.day_care.steps_since_last_egg = 0;
    // DayCare_GiveEgg immediately calls DayCare_InitBreeding, which always
    // chooses a fresh 150..=255 countdown rather than retaining the egg
    // check's previous random reset byte.
    state.day_care.steps_until_next_egg = 0;
    update_day_care_compatibility(state, rng).map_err(RandomSpecialRoutineError::Divider)?;
    Ok(crate::state::DayCareInteractionState {
        caretaker: "man".to_string(),
        action: "collect_egg".to_string(),
        success: true,
        pokemon: Some(species),
        level: Some(level),
        reason: None,
    })
}

fn day_care_man_outside<S>(
    state: &mut GameState,
    routine: &str,
    divider: &mut S,
) -> Result<SpecialRoutineOutcome, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource + ?Sized,
{
    set_day_care_active(state, routine, "man", true)?;
    let mut rng = CrystalRandom::new(state.random_state, divider);
    let outcome = day_care_collect_egg(state, routine, &mut rng)?;
    state.random_state = rng.state();
    let success = outcome.success;
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

fn select_random_bug_contest_contestants<S>(
    state: &mut GameState,
    bug_contest_config: Option<&BugContestConfig>,
    routine: &str,
    divider: &mut S,
) -> Result<SpecialRoutineOutcome, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource + ?Sized,
{
    let config = require_bug_contest_config(bug_contest_config, routine)?;
    let contestant_count = u8::try_from(config.contestant_flags.len()).map_err(|_| {
        SpecialRoutineError::InvalidBugContestConfig {
            routine: routine.to_string(),
            message: "contestant_flags must contain at most 255 entries".to_string(),
        }
    })?;
    let quotient_width = u8::MAX / contestant_count;
    let acceptance_limit = quotient_width * contestant_count;
    let mut rng = CrystalRandom::new(state.random_state, &mut *divider);
    let mut chosen = Vec::new();
    while chosen.len() < config.selected_contestant_count {
        let sample = rng
            .random(false)
            .map_err(RandomSpecialRoutineError::Divider)?
            .value;
        // SelectRandomBugContestContestants rejects $fa..=$ff, then
        // SimpleDivide returns the quotient for a divisor of 25.
        if sample >= acceptance_limit {
            continue;
        }
        let candidate = usize::from(sample / quotient_width);
        if !chosen.contains(&candidate) {
            chosen.push(candidate);
        }
    }
    state.random_state = rng.state();
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
            random_state_after: state.random_state,
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
    let result = if state.storage.party.has_space() {
        let mut contest_mon = contest_mon;
        set_bug_contest_caught_data(&mut contest_mon);
        if !state.storage.party.add_pokemon(contest_mon) {
            return Err(SpecialRoutineError::InvalidState {
                routine: routine.to_string(),
                message: "party reported space but rejected the contest Pokemon".to_string(),
            });
        }
        CAUGHT_MON
    } else {
        let current_pc_box = state.current_pc_box;
        if current_pc_box >= MAX_PC_BOXES {
            return Err(SpecialRoutineError::InvalidCurrentPcBox {
                routine: routine.to_string(),
                current_pc_box,
                box_count: MAX_PC_BOXES,
            });
        }
        let current_box_full = state
            .storage
            .pc_boxes
            .get(current_pc_box)
            .is_some_and(|pc_box| !pc_box.has_space());
        if current_box_full {
            // caught_data.asm falls through to .BoxFull, discarding the caught
            // Pokemon while overwriting sBoxMon1's caught provenance.
            if let Some(first_boxed_mon) =
                state.storage.pc_boxes[current_pc_box].pokemon[0].as_mut()
            {
                set_bug_contest_caught_data(first_boxed_mon);
            }
        } else {
            let mut contest_mon = contest_mon;
            set_bug_contest_caught_data(&mut contest_mon);
            state
                .storage
                .register_capture_in_box(current_pc_box, contest_mon)
                .map_err(|error| SpecialRoutineError::InvalidState {
                    routine: routine.to_string(),
                    message: format!("current contest box rejected Pokemon despite space: {error}"),
                })?;
        }
        BOXED_MON
    };
    state.bug_contest.caught_species = None;
    state.bug_contest.caught_level = None;
    state.sync_party_from_storage();
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

fn set_bug_contest_caught_data(pokemon: &mut Pokemon) {
    match pokemon.caught_data.as_mut() {
        Some(caught_data) => {
            caught_data.level = pokemon.level & 0x3f;
            caught_data.location = 0x13; // LANDMARK_NATIONAL_PARK
        }
        None => {
            pokemon.caught_data = Some(CaughtData {
                level: pokemon.level & 0x3f,
                time_of_day: None,
                original_trainer_gender: 0,
                location: 0x13,
            });
        }
    }
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
    state.script_runtime.active_text_label = None;
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

fn bug_contest_judging<S>(
    state: &mut GameState,
    bug_contest_config: Option<&BugContestConfig>,
    routine: &str,
    divider: &mut S,
) -> Result<SpecialRoutineOutcome, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource + ?Sized,
{
    let config = require_bug_contest_config(bug_contest_config, routine)?;
    let mut rng = CrystalRandom::new(state.random_state, &mut *divider);
    let mut winners = std::array::from_fn(|_| BugContestCandidate::default());
    for (index, contestant) in BUG_CONTESTANTS.iter().enumerate() {
        let Some(flag) = config.contestant_flags.get(index) else {
            break;
        };
        if state.bug_contest.selected_contestant_flags.contains(flag) {
            continue;
        }
        let placement = loop {
            // CheckBugContestContestantFlag ends in AND, so every active
            // contestant's first call enters with carry clear. Rejected 3s
            // loop after AND 3 / CP 3, which also leaves carry clear.
            let masked = rng
                .random(false)
                .map_err(RandomSpecialRoutineError::Divider)?
                .value
                & 0x03;
            if masked != 3 {
                break usize::from(masked);
            }
        };
        let (species, base_score) = contestant.placements[placement];
        // The placement-table AddHL operations cannot cross a bank boundary,
        // so the score perturbation call also enters with carry clear.
        let perturbation = rng
            .random(false)
            .map_err(RandomSpecialRoutineError::Divider)?
            .value
            & 0x07;
        let score = base_score.saturating_add(u16::from(perturbation));
        insert_bug_contest_winner(
            &mut winners,
            BugContestCandidate {
                winner_id: index as u8 + 2,
                species: species.to_string(),
                score,
            },
        );
    }
    let player_score = state
        .bug_contest
        .caught_mon
        .as_ref()
        .map(bug_contest_player_score)
        .unwrap_or(0);
    let player_species = state
        .bug_contest
        .caught_mon
        .as_ref()
        .map(|pokemon| pokemon.species.id.clone())
        .unwrap_or_else(|| "POKEMON".to_string());
    insert_bug_contest_winner(
        &mut winners,
        BugContestCandidate {
            winner_id: 1,
            species: player_species,
            score: player_score,
        },
    );
    state.random_state = rng.state();
    let rank = winners
        .iter()
        .position(|winner| winner.winner_id == 1)
        .map(|index| index as u8 + 1)
        .unwrap_or(4);
    state.bug_contest.last_rank = Some(rank);
    state
        .script_runtime
        .named_buffers
        .insert("STRING_BUFFER_3".to_string(), rank.to_string());
    set_script_numeric_value(state, rank);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    let placements = winners
        .iter()
        .enumerate()
        .map(|(index, winner)| BugContestPlacement {
            place: index as u8 + 1,
            winner_id: winner.winner_id,
            trainer_name: bug_contest_trainer_name(winner.winner_id, &state.player_name),
            species: winner.species.clone(),
            score: winner.score,
            player: winner.winner_id == 1,
        })
        .collect();
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::BugContestJudging {
            rank,
            placements,
            random_state_after: state.random_state,
        },
    })
}

#[derive(Debug, Clone, Copy)]
struct BugContestant {
    trainer_name: &'static str,
    placements: [(&'static str, u16); 3],
}

const BUG_CONTESTANTS: [BugContestant; 10] = [
    BugContestant {
        trainer_name: "BUG CATCHER DON",
        placements: [("KAKUNA", 300), ("METAPOD", 285), ("CATERPIE", 226)],
    },
    BugContestant {
        trainer_name: "BUG CATCHER ED",
        placements: [("BUTTERFREE", 286), ("BUTTERFREE", 251), ("CATERPIE", 237)],
    },
    BugContestant {
        trainer_name: "COOLTRAINER NICK",
        placements: [("SCYTHER", 357), ("BUTTERFREE", 349), ("PINSIR", 368)],
    },
    BugContestant {
        trainer_name: "POKEFAN WILLIAM",
        placements: [("PINSIR", 332), ("BUTTERFREE", 324), ("VENONAT", 321)],
    },
    BugContestant {
        trainer_name: "BUG CATCHER BENNY",
        placements: [("BUTTERFREE", 318), ("WEEDLE", 295), ("CATERPIE", 285)],
    },
    BugContestant {
        trainer_name: "CAMPER BARRY",
        placements: [("PINSIR", 366), ("VENONAT", 329), ("KAKUNA", 314)],
    },
    BugContestant {
        trainer_name: "PICNICKER CINDY",
        placements: [("BUTTERFREE", 341), ("METAPOD", 301), ("CATERPIE", 264)],
    },
    BugContestant {
        trainer_name: "BUG CATCHER JOSH",
        placements: [("SCYTHER", 326), ("BUTTERFREE", 292), ("METAPOD", 282)],
    },
    BugContestant {
        trainer_name: "YOUNGSTER SAMUEL",
        placements: [("WEEDLE", 270), ("PINSIR", 282), ("CATERPIE", 251)],
    },
    BugContestant {
        trainer_name: "SCHOOLBOY KIPP",
        placements: [("VENONAT", 267), ("PARAS", 254), ("KAKUNA", 259)],
    },
];

#[derive(Debug, Clone, Default)]
struct BugContestCandidate {
    winner_id: u8,
    species: String,
    score: u16,
}

fn insert_bug_contest_winner(
    winners: &mut [BugContestCandidate; 3],
    candidate: BugContestCandidate,
) {
    if candidate.score >= winners[0].score {
        winners[2] = winners[1].clone();
        winners[1] = winners[0].clone();
        winners[0] = candidate;
    } else if candidate.score >= winners[1].score {
        winners[2] = winners[1].clone();
        winners[1] = candidate;
    } else if candidate.score >= winners[2].score {
        winners[2] = candidate;
    }
}

fn bug_contest_trainer_name(winner_id: u8, player_name: &str) -> String {
    if winner_id == 1 {
        return player_name.to_string();
    }
    BUG_CONTESTANTS
        .get(winner_id.saturating_sub(2) as usize)
        .map(|contestant| contestant.trainer_name.to_string())
        .unwrap_or_else(|| "TRAINER".to_string())
}

fn bug_contest_player_score(pokemon: &Pokemon) -> u16 {
    let dv_byte_0 = ((pokemon.dvs.attack & 0xf) << 4) | (pokemon.dvs.defense & 0xf);
    let dv_byte_1 = ((pokemon.dvs.speed & 0xf) << 4) | (pokemon.dvs.special & 0xf);
    let dv_bonus = bug_contest_dv_bonus(dv_byte_0, dv_byte_1);
    pokemon
        .max_hp
        .saturating_mul(4)
        .saturating_add(pokemon.attack)
        .saturating_add(pokemon.defense)
        .saturating_add(pokemon.speed)
        .saturating_add(pokemon.special_attack)
        .saturating_add(pokemon.special_defense)
        .saturating_add(u16::from(dv_bonus))
        .saturating_add(pokemon.hp / 8)
        .saturating_add(u16::from(pokemon.item.is_some()))
}

fn bug_contest_dv_bonus(byte_0: u8, byte_1: u8) -> u8 {
    let c = (byte_0 & 0x02).wrapping_mul(4);
    let d = (byte_0.rotate_left(4) & 0x02)
        .wrapping_mul(2)
        .wrapping_add(c);
    let c = byte_1 & 0x02;
    ((byte_1.rotate_left(4) & 0x02) >> 1)
        .wrapping_add(c)
        .wrapping_add(c)
        .wrapping_add(d)
        .wrapping_add(d)
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
    let serial_established = state.link_session.serial_connection_status.is_established();
    if ready != serial_established {
        return Err(SpecialRoutineError::InvalidState {
            routine: routine.to_string(),
            message: format!(
                "ready={ready} requires matching serial connection status, found {:?}",
                state.link_session.serial_connection_status
            ),
        });
    }
    if ready {
        state.link_session.friend_ready = true;
    } else {
        state.link_session.friend_ready = false;
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
        if state.link_session.serial_connection_status.is_established() {
            return Err(SpecialRoutineError::InvalidState {
                routine: routine.to_string(),
                message: format!(
                    "timeout requires connection_not_established, found {:?}",
                    state.link_session.serial_connection_status
                ),
            });
        }
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
    if !state.link_session.serial_connection_status.is_established() {
        return Err(SpecialRoutineError::InvalidState {
            routine: routine.to_string(),
            message: "connected result requires an established serial clock owner".to_string(),
        });
    }
    state.link_session.player_link_action = state.link_session.chosen_cable_club_room;
    state.link_session.other_player_link_mode = other_mode;
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

fn battle_tower_random_action<S>(
    state: &mut GameState,
    context: SpecialRoutineContext<'_>,
    routine: &str,
    divider: &mut S,
) -> Result<SpecialRoutineOutcome, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource + ?Sized,
{
    let action = required_raw_script_value(state, routine)?;
    if action != "BATTLETOWERACTION_CHOOSEREWARD" {
        return battle_tower_action(
            state,
            context.battle_tower_rules,
            context.item_catalog,
            routine,
        )
        .map_err(Into::into);
    }
    let rules =
        context
            .battle_tower_rules
            .ok_or_else(|| SpecialRoutineError::MissingBattleTowerRules {
                routine: routine.to_string(),
            })?;
    validate_battle_tower_rules(rules, routine)?;
    let candidate_count = rules.reward_candidates.len();
    if candidate_count > usize::from(u8::MAX) + 1 {
        return Err(SpecialRoutineError::InvalidBattleTowerRules {
            routine: routine.to_string(),
            message: "rewardCandidates exceeds the source byte table".to_string(),
        }
        .into());
    }
    let mask = candidate_count.next_power_of_two() - 1;
    let mut rng = CrystalRandom::new(state.random_state, &mut *divider);
    let selected = loop {
        // The rejection branch reaches .loop after `cp excluded_item`, which
        // clears carry for the equal value; the initial jumptable path also
        // enters Random with carry clear.
        let mut index = usize::from(
            rng.random(false)
                .map_err(RandomSpecialRoutineError::Divider)?
                .value,
        ) & mask;
        if index >= candidate_count {
            index -= candidate_count;
        }
        let candidate = &rules.reward_candidates[index];
        if !rules.excluded_reward_items.contains(candidate) {
            break candidate.clone();
        }
    };
    state.random_state = rng.state();
    state.battle_tower.reward_item = selected.clone();
    state.battle_tower.reward_given = false;
    state
        .script_runtime
        .variables
        .insert("battle_tower_reward_item".to_string(), selected);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::BattleTowerAction {
            action: action.clone(),
            value: action,
            truthy: true,
        },
    })
}

fn battle_tower_action(
    state: &mut GameState,
    battle_tower_rules: Option<&BattleTowerRules>,
    item_catalog: &BTreeMap<String, Item>,
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
    if action_key == "BATTLETOWERACTION_CHOOSEREWARD" {
        return Err(SpecialRoutineError::MissingDividerSource {
            routine: routine.to_string(),
        });
    }
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
            state
                .script_runtime
                .variables
                .remove("battle_tower_mon_history");
            sync_battle_tower_beaten_count(state);
            ("0".to_string(), false)
        }
        "BATTLETOWERACTION_SAVELEVELGROUP" => {
            // SaveBattleTowerLevelGroup copies the already-selected
            // wBTChoiceOfLvlGroup into SRAM. BattleTowerState is the durable
            // representation of that value, so no invented input is needed.
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
            // This action calls SaveOptions. Reward selection is the distinct
            // BATTLETOWERACTION_CHOOSEREWARD action executed on entry.
            ("1".to_string(), true)
        }
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
            state.battle_tower.record_last_day = Some(state.time.current_day);
            state.battle_tower.record_state = state.battle_tower.record_state.max(1);
            sync_battle_tower_beaten_count(state);
            ("1".to_string(), true)
        }
        "BATTLETOWERACTION_GIVEREWARD" => {
            let rules =
                battle_tower_rules.ok_or_else(|| SpecialRoutineError::MissingBattleTowerRules {
                    routine: routine.to_string(),
                })?;
            validate_battle_tower_rules(rules, routine)?;
            let reward_item = state.battle_tower.reward_item.clone();
            if !rules.reward_candidates.contains(&reward_item)
                || rules.excluded_reward_items.contains(&reward_item)
            {
                return Err(SpecialRoutineError::InvalidState {
                    routine: routine.to_string(),
                    message: format!(
                        "saved Battle Tower reward {reward_item} is not a selectable source candidate"
                    ),
                });
            }
            let item =
                item_catalog
                    .get(&reward_item)
                    .ok_or_else(|| SpecialRoutineError::UnknownItem {
                        routine: routine.to_string(),
                        item_id: reward_item.clone(),
                    })?;
            let mut prospective_bag = state.bag.clone();
            let fits = prospective_bag
                .add_item(item, rules.reward_quantity)
                .map_err(|error| SpecialRoutineError::InvalidState {
                    routine: routine.to_string(),
                    message: format!("could not test Battle Tower reward capacity: {error}"),
                })?;
            if fits {
                (reward_item, true)
            } else {
                (rules.reward_failure_sentinel.clone(), false)
            }
        }
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
    state.script_runtime.named_buffers.insert(
        "STRING_BUFFER_2".to_string(),
        rules.required_party_count.to_string(),
    );
    let failures = battle_tower_rule_failures(state, rules);
    state.battle_tower.last_rule_failure = failures.first().cloned();
    state.script_runtime.variables.insert(
        "battle_tower_rule_failure".to_string(),
        failures.first().cloned().unwrap_or_default(),
    );
    set_script_bool_value(state, !failures.is_empty());
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::CheckForBattleTowerRules { failures },
    })
}

fn battle_tower_room_menu(
    state: &mut GameState,
    battle_tower_rules: Option<&BattleTowerRules>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let rules = battle_tower_rules.ok_or_else(|| SpecialRoutineError::MissingBattleTowerRules {
        routine: routine.to_string(),
    })?;
    validate_battle_tower_rules(rules, routine)?;
    let hall_of_fame = state
        .flags
        .engine_flags
        .get("STATUSFLAGS_HALL_OF_FAME_F")
        .copied()
        .unwrap_or(false);
    let maximum = if hall_of_fame {
        rules.maximum_level_group
    } else {
        rules.maximum_level_group.min(4)
    };
    let level_groups = (rules.minimum_level_group..=maximum).collect::<Vec<_>>();
    let selection = optional_u8_script_variable(state, routine, "_battle_tower_room_selection")?;
    let cancelled = optional_bool_script_variable(state, routine, "_battle_tower_room_cancelled")?
        .unwrap_or(false);
    if selection.is_some() && cancelled {
        return Err(SpecialRoutineError::InvalidState {
            routine: routine.to_string(),
            message: "Battle Tower room menu cannot select and cancel together".to_string(),
        });
    }

    let mut rejection = None;
    if selection.is_none() && !cancelled {
        state.battle_tower.beaten_trainers = 0;
        sync_battle_tower_beaten_count(state);
    }
    if cancelled {
        state.script_runtime.active_menu = None;
        set_script_numeric_value(state, 0x0a);
    } else if let Some(selection) = selection {
        if !level_groups.contains(&selection) {
            return Err(SpecialRoutineError::InvalidState {
                routine: routine.to_string(),
                message: format!(
                    "Battle Tower level group {selection} is outside unlocked source options {level_groups:?}"
                ),
            });
        }
        let level_cap = selection.saturating_mul(rules.level_group_size);
        if state
            .storage
            .party
            .pokemon
            .iter()
            .flatten()
            .any(|pokemon| pokemon.level > level_cap)
        {
            rejection = Some(BattleTowerRoomMenuRejection::PartyMonTopsThisLevel);
        } else if level_cap < 70
            && let Some(pokemon) = state
                .storage
                .party
                .pokemon
                .iter()
                .flatten()
                .find(|pokemon| {
                    pokemon.level < 70
                        && rules
                            .banned_species
                            .contains_key(pokemon.species.id.as_str())
                })
        {
            rejection = Some(BattleTowerRoomMenuRejection::UberRestriction {
                species: pokemon.species.id.clone(),
            });
        }
        if rejection.is_some() {
            state.script_runtime.active_menu = Some("BattleTowerRoomMenu".to_string());
        } else {
            state.battle_tower.level_group = selection;
            state.script_runtime.active_menu = None;
        }
        set_script_numeric_value(state, 0);
    } else {
        state.script_runtime.active_menu = Some("BattleTowerRoomMenu".to_string());
        set_script_numeric_value(state, 0);
    }
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::BattleTowerRoomMenu {
            level_groups,
            selection,
            rejection,
            cancelled,
        },
    })
}

fn battle_tower_battle(
    state: &mut GameState,
    battle_tower_rules: Option<&BattleTowerRules>,
    move_catalog: &BTreeMap<String, Move>,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    let Some(raw_result) = state
        .script_runtime
        .variables
        .get("_battle_result")
        .cloned()
    else {
        if !matches!(
            &state.battle,
            BattleMemory::Trainer { battle_type, .. } if battle_type == "BATTLETYPE_BATTLE_TOWER"
        ) {
            return Err(SpecialRoutineError::InvalidState {
                routine: routine.to_string(),
                message: "Battle Tower battle start requires a loaded Battle Tower opponent"
                    .to_string(),
            });
        }
        heal_battle_tower_party(state, move_catalog);
        state.battle_tower.quick_saved = false;
        state.script_runtime.last_special_routine = Some(routine.to_string());
        return Ok(SpecialRoutineOutcome {
            routine: routine.to_string(),
            effect: SpecialRoutineEffect::BattleTowerBattleStarted,
        });
    };
    let result_code = parse_exact_u8_token(routine, &raw_result, &raw_result)?;
    state.script_runtime.variables.remove("_battle_result");
    state.battle = BattleMemory::Inactive;
    state.battle_active_party_index = None;
    state.battle_active_enemy_party_index = None;
    state.battle_rewarded_enemy_party_indices.clear();
    heal_battle_tower_party(state, move_catalog);
    state.battle_tower.quick_saved = false;
    if result_code != 0 {
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
    state.battle_tower.record_state = state.battle_tower.record_state.max(1);
    if state.battle_tower.beaten_trainers >= rules.challenge_streak_length {
        state.battle_tower.challenge_state = BATTLETOWER_WON_CHALLENGE;
        state.battle_tower.record_last_day = Some(state.time.current_day);
    }
    sync_battle_tower_beaten_count(state);
    // RunBattleTowerTrainer copies wBattleResult into wScriptVar before the
    // battle-room script checks `ifnotequal $0`. Preserve that exact result;
    // using a generic truthy success value turns every win into the loss path.
    set_script_numeric_value(state, result_code);
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

fn heal_battle_tower_party(state: &mut GameState, move_catalog: &BTreeMap<String, Move>) {
    for pokemon in state.storage.party.pokemon.iter_mut().flatten() {
        if !pokemon.is_egg && pokemon.species.id != "EGG" {
            heal_pokemon(pokemon, move_catalog);
        }
    }
    state.sync_party_from_storage();
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

fn canonical_battle_tower_opponent<S>(
    state: &mut GameState,
    rules: &BattleTowerRules,
    context: SpecialRoutineContext<'_>,
    routine: &str,
    divider: &mut S,
) -> Result<(String, String, String, String, bool, Vec<Pokemon>), RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource + ?Sized,
{
    const UNIQUE_TRAINERS: usize = 70;
    const UNIQUE_MONS: usize = 21;
    let group_index = state
        .battle_tower
        .level_group
        .saturating_sub(rules.minimum_level_group) as usize;
    let group = rules.mon_groups.get(group_index).ok_or_else(|| {
        SpecialRoutineError::BattleTowerTrainerBuild {
            routine: routine.to_string(),
            trainer_id: format!("group_{group_index}"),
            error: "compiled Battle Tower level group is missing".to_string(),
        }
    })?;
    if rules.trainers.len() != UNIQUE_TRAINERS {
        return Err(SpecialRoutineError::InvalidBattleTowerRules {
            routine: routine.to_string(),
            message: format!(
                "compiled Battle Tower trainer roster must contain exactly {UNIQUE_TRAINERS} entries, found {}",
                rules.trainers.len()
            ),
        }
        .into());
    }
    if group.len() != UNIQUE_MONS {
        return Err(SpecialRoutineError::BattleTowerTrainerBuild {
            routine: routine.to_string(),
            trainer_id: format!("group_{group_index}"),
            error: format!(
                "compiled Battle Tower group must contain exactly {UNIQUE_MONS} Pokemon, found {}",
                group.len()
            ),
        }
        .into());
    }

    let mut rng = CrystalRandom::new(state.random_state, &mut *divider);
    let mut accumulator = rng.state().add;
    let trainer_index = loop {
        // Crystal 1.1 samples a trainer by adding the updated hRandomAdd to
        // the saved B accumulator. Range retries retain the unmasked sum;
        // history retries retain the accepted, masked index in B.
        rng.random(false)
            .map_err(RandomSpecialRoutineError::Divider)?;
        accumulator = rng.state().add.wrapping_add(accumulator);
        let candidate = accumulator & 0x7f;
        if candidate >= UNIQUE_TRAINERS as u8 {
            continue;
        }
        accumulator = candidate;
        if state.battle_tower.trainer_history.contains(&candidate) {
            continue;
        }
        break usize::from(candidate);
    };
    let trainer = rules.trainers.get(trainer_index).ok_or_else(|| {
        SpecialRoutineError::BattleTowerTrainerBuild {
            routine: routine.to_string(),
            trainer_id: format!("trainer_{trainer_index}"),
            error: "canonical trainer index is missing from compiled roster".to_string(),
        }
    })?;
    if trainer.index != trainer_index {
        return Err(SpecialRoutineError::BattleTowerTrainerBuild {
            routine: routine.to_string(),
            trainer_id: trainer.name.clone(),
            error: format!(
                "compiled trainer table entry {trainer_index} declares index {}",
                trainer.index
            ),
        }
        .into());
    }
    let history_slot = usize::from(state.battle_tower.beaten_trainers)
        .min(state.battle_tower.trainer_history.len().saturating_sub(1));
    if let Some(slot) = state.battle_tower.trainer_history.get_mut(history_slot) {
        *slot = trainer.index as u8;
    }

    let mut mon_history = state
        .script_runtime
        .variables
        .get("battle_tower_mon_history")
        .map(|value| {
            value
                .split(';')
                .filter_map(|entry| {
                    let (group, index) = entry.split_once(':')?;
                    Some((group.parse::<usize>().ok()?, index.parse::<usize>().ok()?))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let recent_species = mon_history
        .iter()
        .filter(|(group, _)| *group == group_index)
        .filter_map(|(_, index)| group.get(*index))
        .map(|mon| mon.species.as_str())
        .collect::<BTreeSet<_>>();
    let mut selected: Vec<(usize, &BattleTowerMonDefinition)> = Vec::new();
    while selected.len() < rules.required_party_count {
        // .FindARandomBattleTowerMon reloads B from the current hRandomAdd.
        // Only an out-of-range retry keeps the unmasked sum. Species/item or
        // previous-team rejection restarts here and reloads B.
        let mut mon_accumulator = rng.state().add;
        let candidate_index = loop {
            rng.random(false)
                .map_err(RandomSpecialRoutineError::Divider)?;
            mon_accumulator = rng.state().add.wrapping_add(mon_accumulator);
            let candidate = mon_accumulator & 0x1f;
            if candidate >= UNIQUE_MONS as u8 {
                continue;
            }
            let candidate_index = usize::from(candidate);
            let mon = &group[candidate_index];
            let rejected_current = selected
                .iter()
                .any(|(_, chosen)| chosen.species == mon.species || chosen.item == mon.item);
            if rejected_current || recent_species.contains(mon.species.as_str()) {
                mon_accumulator = rng.state().add;
                continue;
            }
            break candidate_index;
        };
        selected.push((candidate_index, &group[candidate_index]));
    }
    let mut enemy_party = Vec::with_capacity(selected.len());
    for (index, mon) in selected {
        let species = context.species_catalog.get(&mon.species).ok_or_else(|| {
            SpecialRoutineError::BattleTowerTrainerBuild {
                routine: routine.to_string(),
                trainer_id: trainer.name.clone(),
                error: format!("unknown Battle Tower species {}", mon.species),
            }
        })?;
        let dvs = Dv::from_non_hp(
            mon.dvs.first().copied().unwrap_or(0),
            mon.dvs.get(1).copied().unwrap_or(0),
            mon.dvs.get(2).copied().unwrap_or(0),
            mon.dvs.get(3).copied().unwrap_or(0),
        );
        let mut pokemon = create_pokemon_from_known_dvs(
            species,
            mon.level,
            dvs,
            context.learnsets,
            context.move_catalog,
            context.growth_rates,
        )
        .map_err(|error| SpecialRoutineError::BattleTowerTrainerBuild {
            routine: routine.to_string(),
            trainer_id: trainer.name.clone(),
            error: error.to_string(),
        })?;
        pokemon.nickname = mon.nickname.clone();
        pokemon.item = mon.item.clone();
        pokemon.original_trainer_name = trainer.name.clone();
        pokemon.original_trainer_id = mon.original_trainer_id;
        pokemon.experience = mon.experience as i32;
        pokemon.happiness = mon.happiness;
        pokemon.hp_exp = mon.stat_exp.first().copied().unwrap_or(0);
        pokemon.attack_exp = mon.stat_exp.get(1).copied().unwrap_or(0);
        pokemon.defense_exp = mon.stat_exp.get(2).copied().unwrap_or(0);
        pokemon.speed_exp = mon.stat_exp.get(3).copied().unwrap_or(0);
        pokemon.special_exp = mon.stat_exp.get(4).copied().unwrap_or(0);
        if mon.stats.len() >= 7 {
            pokemon.hp = mon.stats[0];
            pokemon.max_hp = mon.stats[1];
            pokemon.attack = mon.stats[2];
            pokemon.defense = mon.stats[3];
            pokemon.speed = mon.stats[4];
            pokemon.special_attack = mon.stats[5];
            pokemon.special_defense = mon.stats[6];
        }
        for (slot, move_name) in mon.moves.iter().enumerate() {
            if move_name == "NO_MOVE" {
                continue;
            }
            if let Some(move_data) = context.move_catalog.get(move_name) {
                if let Some(existing) = pokemon.moves.get_mut(slot) {
                    existing.name = move_name.clone();
                    existing.current_pp = mon.pp.get(slot).copied().unwrap_or(move_data.pp);
                }
            }
        }
        enemy_party.push(pokemon);
        mon_history.push((group_index, index));
    }
    if mon_history.len() > 6 {
        let keep_from = mon_history.len() - 6;
        mon_history.drain(..keep_from);
    }
    state.random_state = rng.state();
    state.script_runtime.variables.insert(
        "battle_tower_mon_history".to_string(),
        mon_history
            .iter()
            .map(|(group, index)| format!("{group}:{index}"))
            .collect::<Vec<_>>()
            .join(";"),
    );
    Ok((
        format!("BATTLE_TOWER_{}", trainer.index),
        trainer.trainer_class.clone(),
        trainer.name.clone(),
        trainer.sprite_constant.clone(),
        trainer.female,
        enemy_party,
    ))
}

fn load_opponent_trainer_and_pokemon_with_ot_sprite<S>(
    state: &mut GameState,
    context: SpecialRoutineContext<'_>,
    routine: &str,
    divider: &mut S,
) -> Result<SpecialRoutineOutcome, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource + ?Sized,
{
    if state.pending_static_wild_terminal.is_some() {
        return Err(SpecialRoutineError::InvalidState {
            routine: routine.to_string(),
            message: "pending static-wild terminal must resume before a Battle Tower battle starts"
                .to_string(),
        }
        .into());
    }
    if !matches!(state.battle, BattleMemory::Inactive) {
        return Err(SpecialRoutineError::InvalidState {
            routine: routine.to_string(),
            message: "Battle Tower opponent load requires inactive battle memory".to_string(),
        }
        .into());
    }
    // LoadOpponentTrainerAndPokemonWithOTSprite consumes the object constant
    // placed in wScriptVar by the immediately preceding `setval`.
    let target_object = required_raw_script_value(state, routine)?;
    let rules =
        context
            .battle_tower_rules
            .ok_or_else(|| SpecialRoutineError::MissingBattleTowerRules {
                routine: routine.to_string(),
            })?;
    validate_battle_tower_rules(rules, routine)?;
    if rules.trainers.is_empty() || rules.mon_groups.is_empty() {
        return Err(SpecialRoutineError::InvalidBattleTowerRules {
            routine: routine.to_string(),
            message: "compiled trainer roster and Pokemon groups are required".to_string(),
        }
        .into());
    }
    let (trainer_id, trainer_class, trainer_name, sprite_constant, trainer_female, enemy_party) =
        canonical_battle_tower_opponent(state, rules, context, routine, divider)?;
    let text_index = if trainer_female {
        let candidate = state.random_state.add & 0x0f;
        if candidate >= 15 {
            candidate - 15
        } else {
            candidate
        }
    } else {
        let candidate = state.random_state.add & 0x1f;
        if candidate >= 25 {
            candidate - 25
        } else {
            candidate
        }
    } + 1;
    let gender = if trainer_female { "F" } else { "M" };
    let intro_text = format!("_BTGreeting{gender}{text_index}Text");
    let win_text = format!("_BTLoss{gender}{text_index}Text");
    let loss_text = format!("_BTWin{gender}{text_index}Text");
    let reward = 0;
    let encounter_music = "MUSIC_BATTLE_TOWER_THEME".to_string();
    let ai_move_flags = 0;
    let ai_item_switch_flags = 0;
    let ai_layers = Vec::new();
    let enemy_pokemon = enemy_party.first().cloned().ok_or_else(|| {
        SpecialRoutineError::BattleTowerTrainerBuild {
            routine: routine.to_string(),
            trainer_id: trainer_id.clone(),
            error: "empty trainer party".to_string(),
        }
    })?;

    state.battle_tower.loaded_trainer_id = Some(trainer_id.clone());
    state.battle_tower.last_sprite_constant = Some(sprite_constant.clone());
    // `CopyBTTrainer_FromBT_OT_TowBT_OTTemp` commits these two bytes to SRAM
    // before `StartBattle`. They deliberately make a reset or power loss in
    // the room distinguishable from the resumable save-and-quit state.
    state.battle_tower.challenge_state = BATTLETOWER_CHALLENGE_IN_PROGRESS;
    state.battle_tower.beaten_trainers = state
        .battle_tower
        .beaten_trainers
        .saturating_add(1)
        .min(rules.challenge_streak_length);
    sync_battle_tower_beaten_count(state);
    state.battle = BattleMemory::Trainer {
        battle_type: "BATTLETYPE_BATTLE_TOWER".to_string(),
        trainer_class: trainer_class.clone(),
        trainer_id: trainer_id.clone(),
        trainer_name: trainer_name.clone(),
        event_flag: String::new(),
        seen_text: String::new(),
        win_text: win_text.clone(),
        loss_text: loss_text.clone(),
        callback: String::new(),
        source_script: routine.to_string(),
        enemy_pokemon,
        enemy_party: enemy_party.clone(),
        reward,
        encounter_music,
        ai_move_flags,
        ai_item_switch_flags,
        ai_layers,
    };
    let active_party_index = first_available_battle_party_index(state).ok_or_else(|| {
        SpecialRoutineError::BattleTowerTrainerBuild {
            routine: routine.to_string(),
            trainer_id: trainer_id.clone(),
            error: "no non-fainted player party Pokemon".to_string(),
        }
    })?;

    state.battle_result = 0;
    state.script_runtime.variables.remove("_battle_result");
    state.battle_active_party_index = Some(active_party_index);
    state.battle_active_enemy_party_index = Some(0);
    state.battle_rewarded_enemy_party_indices.clear();
    state
        .script_runtime
        .variables
        .insert("other_trainer_class".to_string(), trainer_class.clone());
    state
        .script_runtime
        .variables
        .insert("other_trainer_id".to_string(), trainer_id.clone());
    state
        .script_runtime
        .variables
        .insert("other_trainer_name".to_string(), trainer_name.clone());
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
    state
        .script_runtime
        .variables
        .insert("battle_tower_intro_text".to_string(), intro_text);
    state
        .script_runtime
        .variables
        .insert("battle_tower_win_text".to_string(), win_text);
    state
        .script_runtime
        .variables
        .insert("battle_tower_loss_text".to_string(), loss_text);
    set_script_bool_value(state, true);
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::LoadOpponentTrainerAndPokemonWithOtSprite {
            trainer_id,
            trainer_class,
            trainer_name,
            party_size: enemy_party.len(),
            sprite_constant,
            target_object,
            random_state_after: state.random_state,
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

fn battle_tower_initialize_challenge_ram(
    state: &mut GameState,
    routine: &str,
) -> Result<SpecialRoutineOutcome, SpecialRoutineError> {
    // ASM InitBattleTowerChallengeRAM clears the transient challenge RAM
    // before copying the persistent record block. Keep persistent streaks and
    // leaderboard records intact while clearing only the active challenge.
    state.battle_tower.challenge_state = BATTLETOWER_NO_CHALLENGE;
    state.battle_tower.beaten_trainers = 0;
    state.battle_tower.quick_saved = false;
    state.battle_tower.loaded_trainer_id = None;
    state.battle_tower.last_sprite_constant = None;
    state.battle_tower.selected_party_indexes.clear();
    state
        .script_runtime
        .variables
        .remove("battle_tower_mon_history");
    state.script_runtime.last_special_routine = Some(routine.to_string());
    Ok(SpecialRoutineOutcome {
        routine: routine.to_string(),
        effect: SpecialRoutineEffect::Noop,
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

fn give_odd_egg<S>(
    state: &mut GameState,
    species_catalog: &BTreeMap<String, PokemonSpecies>,
    learnsets: &SpeciesLearnsets,
    growth_rates: &GrowthRateCatalog,
    move_catalog: &BTreeMap<String, Move>,
    odd_egg_definitions: &[OddEggDefinition],
    routine: &str,
    divider: &mut S,
) -> Result<SpecialRoutineOutcome, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource + ?Sized,
{
    validate_odd_egg_table(odd_egg_definitions, routine)?;
    let Some(party_slot) = state.storage.party.next_open_slot() else {
        // The map script checks PartyCount before calling GiveOddEgg. Keep the
        // core boundary equally strict so a full party consumes zero DIV
        // reads even when invoked directly.
        return Err(SpecialRoutineError::GiftStorageFull {
            routine: routine.to_string(),
            species: odd_egg_definitions[0].species.clone(),
        }
        .into());
    };
    let table_index = draw_odd_egg_index(state, odd_egg_definitions, routine, divider)?;
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
            Ok::<LearnedMove, SpecialRoutineError>(LearnedMove {
                name: move_id.clone(),
                current_pp: move_data.pp,
                pp_ups: 0,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    egg.hp = 0;
    egg.is_egg = true;
    egg.status = None;
    egg.sleep_turns = 0;
    egg.original_trainer_name = definition.original_trainer_name.clone();
    egg.original_trainer_id = definition.original_trainer_id;
    egg.experience = definition.experience;
    egg.happiness = definition.hatch_cycles;
    if !state.storage.party.add_pokemon(egg.clone()) {
        return Err(SpecialRoutineError::InvalidState {
            routine: routine.to_string(),
            message: "party reported an open Odd Egg slot but rejected the egg".to_string(),
        }
        .into());
    }
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
            random_state_after: state.random_state,
        },
    })
}

fn validate_odd_egg_table(
    odd_egg_definitions: &[OddEggDefinition],
    routine: &str,
) -> Result<(), SpecialRoutineError> {
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
    Ok(())
}

fn draw_odd_egg_index<S>(
    state: &mut GameState,
    odd_egg_definitions: &[OddEggDefinition],
    routine: &str,
    divider: &mut S,
) -> Result<usize, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource + ?Sized,
{
    let mut rng = CrystalRandom::new(state.random_state, &mut *divider);
    // The special pointer-table lookup reaches Random after carry-clearing
    // bank-local ADD instructions. The returned A byte is hRandomSub; the
    // low byte is the newly updated hRandomAdd.
    let high = rng
        .random(false)
        .map_err(RandomSpecialRoutineError::Divider)?
        .value;
    let random_word = u16::from_be_bytes([high, rng.state().add]);
    state.random_state = rng.state();
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
    }
    .into())
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
    if rules.reward_candidates.is_empty() || rules.reward_quantity == 0 {
        return Err(SpecialRoutineError::InvalidBattleTowerRules {
            routine: routine.to_string(),
            message: "rewardCandidates and rewardQuantity must be nonzero".to_string(),
        });
    }
    if !is_exact_nonempty_special_token(&rules.reward_failure_sentinel) {
        return Err(SpecialRoutineError::InvalidBattleTowerRules {
            routine: routine.to_string(),
            message: "rewardFailureSentinel must be an exact item id".to_string(),
        });
    }
    if rules
        .reward_candidates
        .iter()
        .chain(std::iter::once(&rules.reward_failure_sentinel))
        .any(|item_id| !rules.reward_item_values.contains_key(item_id))
        || rules
            .reward_item_values
            .values()
            .collect::<BTreeSet<_>>()
            .len()
            != rules.reward_item_values.len()
    {
        return Err(SpecialRoutineError::InvalidBattleTowerRules {
            routine: routine.to_string(),
            message: "rewardItemValues must cover rewards and the sentinel with unique bytes"
                .to_string(),
        });
    }
    let mut reward_candidates = BTreeSet::new();
    for item_id in &rules.reward_candidates {
        if !is_exact_nonempty_special_token(item_id) || !reward_candidates.insert(item_id.as_str())
        {
            return Err(SpecialRoutineError::InvalidBattleTowerRules {
                routine: routine.to_string(),
                message: "rewardCandidates must contain unique exact item ids".to_string(),
            });
        }
    }
    if rules.excluded_reward_items.iter().any(|item_id| {
        !is_exact_nonempty_special_token(item_id) || !reward_candidates.contains(item_id.as_str())
    }) || rules
        .reward_candidates
        .iter()
        .all(|item| rules.excluded_reward_items.contains(item))
    {
        return Err(SpecialRoutineError::InvalidBattleTowerRules {
            routine: routine.to_string(),
            message: "excludedRewardItems must be exact candidate ids and leave a reward"
                .to_string(),
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

fn battle_tower_rule_failures(state: &GameState, rules: &BattleTowerRules) -> Vec<String> {
    let party = state
        .storage
        .party
        .pokemon
        .iter()
        .flatten()
        .collect::<Vec<_>>();
    let mut failures = Vec::new();
    if party.len() != rules.required_party_count {
        failures.push(rules.party_count_failure_text.clone());
    }

    let mut species = BTreeSet::new();
    for pokemon in &party {
        if pokemon_is_egg(pokemon) {
            continue;
        }
        if !species.insert(pokemon.species.id.as_str()) {
            failures.push(rules.duplicate_species_failure_text.clone());
            break;
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
            failures.push(rules.duplicate_held_item_failure_text.clone());
            break;
        }
    }

    if party.iter().any(|pokemon| pokemon_is_egg(pokemon)) {
        failures.push(rules.egg_failure_text.clone());
    }

    failures
}

fn sync_battle_tower_beaten_count(state: &mut GameState) {
    state.battle_tower.beaten_trainers = state.battle_tower.beaten_trainers.min(99);
    state.script_runtime.memory.insert(
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

fn sample_shuckie_dvs<S>(
    state: &mut GameState,
    divider: &mut S,
) -> Result<Dv, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource + ?Sized,
{
    let mut rng = CrystalRandom::new(state.random_state, &mut *divider);
    // The battle-mode `and a` before GeneratePartyMonStats clears carry.
    let attack_defense = rng
        .random(false)
        .map_err(RandomSpecialRoutineError::Divider)?;
    // No instruction changes carry between the two Random calls: the second
    // call receives the borrow from the first call's final SBC.
    let speed_special = rng
        .random(attack_defense.carry_out)
        .map_err(RandomSpecialRoutineError::Divider)?;
    state.random_state = rng.state();
    Ok(Dv::from_non_hp(
        attack_defense.value >> 4,
        attack_defense.value & 0x0f,
        speed_special.value >> 4,
        speed_special.value & 0x0f,
    ))
}

fn ensure_buenas_password<'a, S>(
    state: &mut GameState,
    categories: &'a BuenaPasswordCategories,
    routine: &str,
    divider: &mut S,
) -> Result<
    (&'a str, &'a BuenaPasswordCategoryDefinition, String),
    RandomSpecialRoutineError<S::Error>,
>
where
    S: DividerSource + ?Sized,
{
    if categories.order.is_empty() || categories.categories.is_empty() {
        return Err(SpecialRoutineError::MissingBuenaPasswordCategories {
            routine: routine.to_string(),
        }
        .into());
    }
    let current_day = state.time.current_day;
    if !state.buenas_password.generated || state.buenas_password.generation_day != current_day {
        if categories.order.len() != 11 {
            return Err(SpecialRoutineError::InvalidState {
                routine: routine.to_string(),
                message: format!(
                    "Buena password table must contain exactly 11 categories, found {}",
                    categories.order.len()
                ),
            }
            .into());
        }
        let mut rng = CrystalRandom::new(state.random_state, &mut *divider);
        let category_index = loop {
            // BuenasPassword4 masks hRandomSub with $f and rejects 11..15.
            // Both the initial path and each retry enter with carry clear.
            let candidate = rng
                .random(false)
                .map_err(RandomSpecialRoutineError::Divider)?
                .value
                & 0x0f;
            if candidate < 11 {
                break usize::from(candidate);
            }
        };
        let Some(category_id) = categories.order.get(category_index) else {
            return Err(SpecialRoutineError::InvalidBuenaPasswordCategoryIndex {
                routine: routine.to_string(),
                index: category_index,
            }
            .into());
        };
        let Some(category) = categories.categories.get(category_id) else {
            return Err(SpecialRoutineError::InvalidBuenaPasswordCategoryIndex {
                routine: routine.to_string(),
                index: category_index,
            }
            .into());
        };
        if category.options.len() != 3 {
            return Err(SpecialRoutineError::InvalidBuenaPasswordCategoryIndex {
                routine: routine.to_string(),
                index: category_index,
            }
            .into());
        }
        let option_index = loop {
            // The option loop masks with 3 and rejects 3; SWAP/AND/CP leave
            // carry clear for every attempt.
            let candidate = rng
                .random(false)
                .map_err(RandomSpecialRoutineError::Divider)?
                .value
                & 0x03;
            if candidate < 3 {
                break usize::from(candidate);
            }
        };
        state.random_state = rng.state();
        state.buenas_password.category_index = category_index;
        state.buenas_password.option_index = option_index;
        state.buenas_password.generation_day = current_day;
        state.buenas_password.generated = true;
    }
    let Some(category_id) = categories.order.get(state.buenas_password.category_index) else {
        return Err(SpecialRoutineError::InvalidBuenaPasswordCategoryIndex {
            routine: routine.to_string(),
            index: state.buenas_password.category_index,
        }
        .into());
    };
    let Some(category) = categories.categories.get(category_id) else {
        return Err(SpecialRoutineError::InvalidBuenaPasswordCategoryIndex {
            routine: routine.to_string(),
            index: state.buenas_password.category_index,
        }
        .into());
    };
    let Some(correct) = category.options.get(state.buenas_password.option_index) else {
        return Err(SpecialRoutineError::InvalidBuenaPasswordOptionIndex {
            routine: routine.to_string(),
            index: state.buenas_password.option_index,
        }
        .into());
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
    let dv0 = ((pokemon.dvs.attack & 0x0f) << 4) | (pokemon.dvs.defense & 0x0f);
    let dv1 = ((pokemon.dvs.speed & 0x0f) << 4) | (pokemon.dvs.special & 0x0f);
    calculate_magikarp_length_from_dv_bytes(dv0, dv1, trainer_id, magikarp_lengths, routine)
}

pub fn calculate_magikarp_length_from_dv_bytes(
    dv0: u8,
    dv1: u8,
    trainer_id: u16,
    magikarp_lengths: &[MagikarpLengthEntry],
    routine: &str,
) -> Result<(u8, u8), SpecialRoutineError> {
    let table = require_magikarp_lengths(magikarp_lengths, routine)?;
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
                let quotient = (delta / u16::from(entry.divisor)) & 0xff;
                resolved = Some(quotient + 100 * multiplier);
                break;
            }
            multiplier += 1;
        }
        resolved.unwrap_or_else(|| {
            let threshold = table[table.len() - 1].threshold;
            1600u16.wrapping_add(bc.wrapping_sub(threshold))
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
    if magikarp_lengths.len() != 14 {
        return Err(SpecialRoutineError::InvalidMagikarpLengthTable {
            routine: routine.to_string(),
            message: format!(
                "expected exactly 14 source rows, found {}",
                magikarp_lengths.len()
            ),
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

fn day_care_deposit<S>(
    state: &mut GameState,
    routine: &str,
    caretaker: &str,
    rng: &mut CrystalRandom<S>,
) -> Result<crate::state::DayCareInteractionState, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource,
{
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
    update_day_care_compatibility(state, rng).map_err(RandomSpecialRoutineError::Divider)?;
    Ok(crate::state::DayCareInteractionState {
        caretaker: caretaker.to_string(),
        action: "deposit".to_string(),
        success: true,
        pokemon: Some(pokemon.species.id),
        level: Some(pokemon.level),
        reason: None,
    })
}

fn day_care_withdraw<S>(
    state: &mut GameState,
    routine: &str,
    caretaker: &str,
    rng: &mut CrystalRandom<S>,
) -> Result<crate::state::DayCareInteractionState, RandomSpecialRoutineError<S::Error>>
where
    S: DividerSource,
{
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
    // Do not remove the resident until the party has accepted it.  Crystal
    // leaves the Day-Care slot intact when withdrawal cannot be completed;
    // taking it first makes a malformed/full party silently delete the mon.
    let Some(pokemon) = day_care_resident(state, routine, caretaker)?
        .pokemon
        .clone()
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
    let gained_levels =
        level.saturating_sub(day_care_resident(state, routine, caretaker)?.initial_level);
    let fee = 100u32.saturating_add(u32::from(gained_levels) * 100);
    if state.money < fee {
        return Ok(crate::state::DayCareInteractionState {
            caretaker: caretaker.to_string(),
            action: "withdraw".to_string(),
            success: false,
            pokemon: Some(species),
            level: Some(level),
            reason: Some("not_enough_money".to_string()),
        });
    }
    let stored = state.storage.party.add_pokemon(pokemon);
    if stored {
        state.money -= fee;
        day_care_resident_mut(state, routine, caretaker)?
            .pokemon
            .take();
        state.sync_party_from_storage();
        let resident = day_care_resident_mut(state, routine, caretaker)?;
        resident.initial_experience = 0;
        resident.initial_level = 0;
        resident.steps = 0;
        resident.active = false;
        update_day_care_compatibility(state, rng).map_err(RandomSpecialRoutineError::Divider)?;
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

fn update_day_care_compatibility<S>(
    state: &mut GameState,
    rng: &mut CrystalRandom<S>,
) -> Result<(), S::Error>
where
    S: DividerSource,
{
    state.day_care.compatibility_score = day_care_compatibility_score(
        state.day_care.man.pokemon.as_ref(),
        state.day_care.lady.pokemon.as_ref(),
    );
    if matches!(state.day_care.compatibility_score, 0 | 255) {
        state.day_care.steps_until_next_egg = 0;
    } else if state.day_care.steps_until_next_egg == 0 {
        // DayCare_InitBreeding calls Random until a byte in 150..=255 is
        // produced.  This is intentionally independent of the compatibility
        // score; the score controls whether breeding is possible, while the
        // random timer controls when the egg is offered.
        let mut carry_in = false;
        let steps = loop {
            let sample = rng.random(carry_in)?.value;
            if sample >= 150 {
                break u16::from(sample);
            }
            // The rejected `cp 150; jr c` feeds carry into the retry.
            carry_in = true;
        };
        state.day_care.steps_until_next_egg = steps;
    }
    Ok(())
}

/// Advance Day Care state at the same overworld-step boundary as Crystal.
/// Experience/egg inheritance is handled when the egg is collected; this
/// routine owns the persistent counters and compatibility lifecycle.
#[derive(Debug, PartialEq, Eq, Error)]
pub enum DayCareStepError<E> {
    #[error("day-care experience calculation failed: {0}")]
    Experience(#[from] ExperienceError),
    #[error("day-care divider source failed: {0}")]
    Divider(E),
}

pub fn advance_day_care_step<S>(
    state: &mut GameState,
    growth_rates: &GrowthRateCatalog,
    rng: &mut CrystalRandom<S>,
) -> Result<(), DayCareStepError<S::Error>>
where
    S: DividerSource,
{
    // Validate every curve that can be consumed on this step before changing
    // either resident. A malformed pack must not partially advance Day Care.
    for resident in [&state.day_care.man, &state.day_care.lady] {
        let Some(pokemon) = resident.pokemon.as_ref() else {
            continue;
        };
        if resident.steps >= 255 && pokemon.level < 100 {
            calculate_experience(
                growth_rates,
                &pokemon.species.growth_rate,
                pokemon.level + 1,
            )?;
        }
    }
    for resident in [&mut state.day_care.man, &mut state.day_care.lady] {
        let Some(pokemon) = resident.pokemon.as_mut() else {
            continue;
        };
        resident.steps = resident.steps.saturating_add(1);
        // The Day-Care overworld loop awards one level per 256 steps.  Keep
        // the counter as a remainder and refresh the resident's persistent
        // stats so withdrawing the Pokémon reflects the levels gained while
        // it was stored.
        while resident.steps >= 256 && pokemon.level < 100 {
            resident.steps -= 256;
            pokemon.level = pokemon.level.saturating_add(1);
            pokemon.experience = pokemon.experience.max(calculate_experience(
                growth_rates,
                &pokemon.species.growth_rate,
                pokemon.level,
            )?);
            let stats = calculate_stats(
                &pokemon.species,
                pokemon.level,
                pokemon.dvs,
                StatExperience {
                    hp: pokemon.hp_exp,
                    attack: pokemon.attack_exp,
                    defense: pokemon.defense_exp,
                    speed: pokemon.speed_exp,
                    special: pokemon.special_exp,
                },
            );
            pokemon.max_hp = stats.max_hp;
            pokemon.hp = stats.max_hp;
        }
    }
    if matches!(state.day_care.compatibility_score, 0 | 255) || state.day_care.egg_present {
        return Ok(());
    }
    state.day_care.steps_since_last_egg = state.day_care.steps_since_last_egg.wrapping_add(1);
    let countdown = (state.day_care.steps_until_next_egg as u8).wrapping_sub(1);
    state.day_care.steps_until_next_egg = u16::from(countdown);
    if countdown != 0 {
        return Ok(());
    }

    // With both breeding residents present, DayCareStep's lady level `cp`
    // is the last carry-writing instruction on the ordinary path into this
    // Random call. A level below MAX_LEVEL therefore enters carry-set.
    let countdown_carry = state
        .day_care
        .lady
        .pokemon
        .as_ref()
        .is_some_and(|pokemon| pokemon.level < 100);
    state.day_care.steps_until_next_egg = u16::from(
        rng.random(countdown_carry)
            .map_err(DayCareStepError::Divider)?
            .value,
    );
    let compatibility = state.day_care.compatibility_score;
    let egg_threshold = match compatibility {
        255 | 0 => 0,
        230..=254 => 80,
        170..=229 => 40,
        110..=169 => 30,
        _ => 10,
    };
    // The final compatibility `cp` before `.okay` carries only for the
    // lowest compatibility tier.
    let produces_egg = egg_threshold > 0
        && rng
            .random(compatibility < 110)
            .map_err(DayCareStepError::Divider)?
            .value
            < egg_threshold;
    if !produces_egg {
        return Ok(());
    }

    {
        // Crystal materializes the egg before setting the daycare-man flag.
        // Keep the generated record in SRAM so pickup does not depend on a
        // second, lossy reconstruction pass.  The female species is used
        // unless one parent is Ditto; inherited DVs/moves are refined by the
        // battle/data layer when the egg is built from a full catalog.
        let parents = (
            state.day_care.man.pokemon.as_ref(),
            state.day_care.lady.pokemon.as_ref(),
        );
        let parent = match parents {
            (Some(man), Some(lady)) if man.species.id == "DITTO" => Some(lady),
            (Some(man), Some(lady)) if lady.species.id == "DITTO" => Some(man),
            (Some(man), Some(_)) if pokemon_gender_code(man) == Some(true) => Some(man),
            (Some(_), Some(lady)) => Some(lady),
            _ => None,
        };
        if let Some(parent) = parent {
            let move_and_dv_donor = match parents {
                (Some(man), Some(lady)) if man.species.id == "DITTO" => {
                    if pokemon_gender_code(lady) != Some(true) {
                        lady
                    } else {
                        man
                    }
                }
                (Some(man), Some(lady)) if lady.species.id == "DITTO" => {
                    if pokemon_gender_code(man) != Some(true) {
                        man
                    } else {
                        lady
                    }
                }
                (Some(man), Some(_lady)) if pokemon_gender_code(man) == Some(false) => man,
                (Some(_man), Some(lady)) => lady,
                _ => parent,
            };
            // DayCare_InitBreeding chooses the Nidoran egg species before it
            // rolls the two fresh DV bytes.  Keep that byte in the exact RNG
            // position; the pack layer replaces the cloned species record
            // with the canonical Nidoran data after this core-only hook.
            let mut egg_species_id = parent.species.id.clone();
            let maternal_nidoran = matches!(parent.species.id.as_str(), "NIDORAN_F" | "NIDORINA");
            if maternal_nidoran {
                let species_roll = rng.random(false).map_err(DayCareStepError::Divider)?.value;
                if species_roll >= 129 {
                    egg_species_id = "NIDORAN_M".to_string();
                } else {
                    egg_species_id = "NIDORAN_F".to_string();
                }
            }
            let random_dv_1 = rng.random(false).map_err(DayCareStepError::Divider)?;
            let random_dv_2 = rng
                .random(random_dv_1.carry_out)
                .map_err(DayCareStepError::Divider)?
                .value;
            let random_dv_1 = random_dv_1.value;
            let mut dvs = Dv::from_non_hp(
                random_dv_1 >> 4,
                random_dv_1 & 0x0f,
                random_dv_2 >> 4,
                random_dv_2 & 0x0f,
            );
            dvs.defense = move_and_dv_donor.dvs.defense & 0x0f;
            dvs.special = move_and_dv_donor.dvs.special & 0x07;
            let mut egg = parent.clone();
            egg.species.id = egg_species_id;
            egg.dvs = dvs;
            egg.nickname = "EGG".to_string();
            egg.item = None;
            // Preserve the non-maternal parent's moves as candidates for
            // InitEggMoves.  The pack layer filters these against the
            // species' canonical egg-move table and combines them with the
            // level-five moves; keeping the candidates here prevents the
            // core-only step hook from losing them before that pass.
            egg.moves = move_and_dv_donor.moves.clone();
            egg.moves.truncate(4);
            egg.status = None;
            egg.is_egg = true;
            egg.level = 5;
            egg.original_trainer_name = state.player_name.clone();
            egg.original_trainer_id = state.player_id;
            egg.caught_data = None;
            egg.mail = None;
            egg.experience = 0;
            let stats =
                calculate_stats(&egg.species, egg.level, egg.dvs, StatExperience::default());
            egg.hp = stats.max_hp;
            egg.max_hp = stats.max_hp;
            egg.attack = stats.attack;
            egg.defense = stats.defense;
            egg.speed = stats.speed;
            egg.special_attack = stats.special_attack;
            egg.special_defense = stats.special_defense;
            egg.happiness = egg.species.step_cycles_to_hatch;
            egg.sleep_turns = 0;
            egg.flinching = false;
            egg.rampage_turns = 0;
            egg.confusion_turns = 0;
            egg.perish_song_turns = 0;
            egg.focus_energy = false;
            egg.turns_in_battle = 0;
            egg.stat_boosts.clear();
            state.day_care.egg = Some(egg);
        }
        state.day_care.egg_present = true;
        state.day_care.steps_since_last_egg = 0;
    }
    Ok(())
}

fn day_care_compatibility_score(first: Option<&Pokemon>, second: Option<&Pokemon>) -> u8 {
    let (Some(first), Some(second)) = (first, second) else {
        return 0;
    };
    let first_cannot_breed = matches!(
        first.species.egg_group1.as_str(),
        "EGG_NONE" | "EGG_NO_EGGS"
    ) && matches!(
        first.species.egg_group2.as_str(),
        "EGG_NONE" | "EGG_NO_EGGS"
    );
    let second_cannot_breed = matches!(
        second.species.egg_group1.as_str(),
        "EGG_NONE" | "EGG_NO_EGGS"
    ) && matches!(
        second.species.egg_group2.as_str(),
        "EGG_NONE" | "EGG_NO_EGGS"
    );
    if first_cannot_breed || second_cannot_breed {
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
        return 255;
    }
    let mut score: u8 = if first.species.id == second.species.id {
        254
    } else {
        128
    };
    if first.original_trainer_id == second.original_trainer_id {
        score = score.saturating_sub(77);
    }
    score
}

fn pokemon_gender_code(pokemon: &Pokemon) -> Option<bool> {
    match pokemon.species.gender_ratio {
        255 => None,
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
        .script_value
        .clone()
        .or_else(|| state.script_runtime.variables.get("_value").cloned())
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

fn heal_pokemon(pokemon: &mut Pokemon, move_catalog: &BTreeMap<String, Move>) -> bool {
    let mut changed = false;
    if pokemon.hp != pokemon.max_hp {
        pokemon.hp = pokemon.max_hp;
        changed = true;
    }
    if pokemon.status.is_some() {
        pokemon.status = None;
        changed = true;
    }
    for learned in &mut pokemon.moves {
        let Some(move_data) = move_catalog.get(&learned.name) else {
            continue;
        };
        let restored_pp = move_data.pp;
        if learned.current_pp != restored_pp {
            learned.current_pp = restored_pp;
            changed = true;
        }
    }
    changed
}

#[cfg(test)]
#[path = "special_routines_tests.rs"]
mod tests;
