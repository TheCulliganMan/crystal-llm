use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use std::collections::{BTreeMap, BTreeSet};

use crate::battle::turn::BattleCombatState;
use crate::input::{B_PAD_DOWN, B_PAD_LEFT, B_PAD_RIGHT, B_PAD_UP};
use crate::map::MapSceneTable;
use crate::models::{
    Bag, LearnedMove, MAX_PC_BOXES, PARTY_SIZE, PokedexState, Pokemon, PokemonSpecies,
    PokemonStorage, Trainer,
};
use crate::random::Random;
use crate::systems::script_audio::{
    SCRIPT_AUDIO_CRY_COMMANDS, SCRIPT_AUDIO_MUSIC_COMMANDS, SCRIPT_AUDIO_MUSIC_FADE_COMMANDS,
    SCRIPT_AUDIO_NO_PAYLOAD_COMMANDS, SCRIPT_AUDIO_SOUND_EFFECT_COMMANDS,
};
use crate::systems::script_runtime::script_runtime_command_arg_counts;
use crate::systems::script_text::SCRIPT_TEXT_LABEL_COMMANDS;
use crate::systems::script_warps::{SCRIPT_MAP_LOAD_COMMANDS, SCRIPT_MAP_REFRESH_COMMANDS};
use crate::systems::shop::{SCRIPT_SHOP_ZERO_MART_TYPES, is_known_script_mart_type};
use crate::systems::step_events::StepEventCounters;
use crate::systems::time::{ClockTime, TimeState};
use crate::timing::Frame;
use crate::world::map::{Direction, TilePosition};
use crate::world::movement::MovementMode;
use crate::world::session::{
    OverworldSnapshot, raw_event_tile_to_runtime_tile_checked, runtime_tile_to_raw_event_tile,
};

pub const PLAYER_NAME_LENGTH: usize = 8;
pub const PLAYER_GENDER_MALE: u8 = 0;
pub const PLAYER_GENDER_FEMALE: u8 = 1;
pub const MAILBOX_CAPACITY: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MailboxMail {
    pub item_id: String,
    pub mail: crate::models::pokemon::MailData,
}
const MOBILE_LOGIN_PASSWORD_LENGTH: usize = 17;
const BATTLE_TOWER_SAVE_FILE_FLAG_YOURS: u8 = 0x1;
const BATTLE_TOWER_SAVE_FILE_FLAG_EXPLANATION: u8 = 0x2;
const BATTLE_TOWER_SAVE_FILE_FLAGS_MASK: u8 =
    BATTLE_TOWER_SAVE_FILE_FLAG_YOURS | BATTLE_TOWER_SAVE_FILE_FLAG_EXPLANATION;
const BATTLE_TOWER_RULE_FAILURES: &[&str] = &[
    "OnlyThreeMonMayBeEnteredText",
    "TheMonMustAllBeDifferentKindsText",
    "TheMonMustNotHoldTheSameItemsText",
    "YouCantTakeAnEggText",
];
const BATTLE_TOWER_MOBILE_FLAGS: &[&str] = &[
    "function103780",
    "function1037c2",
    "function1037eb",
    "function10383c",
    "function10387b",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GameState {
    pub options: Options,
    pub player_name: String,
    pub player_id: u16,
    pub player_gender: u8,
    pub party: PartyState,
    pub storage: PokemonStorage,
    pub mailbox: Vec<MailboxMail>,
    pub pending_move_learn: Option<PendingMoveLearn>,
    pub pending_move_learn_queue: Vec<PendingMoveLearn>,
    pub bag: Bag,
    pub money: u32,
    pub moms_money: u32,
    pub mom_saving_active: bool,
    pub mom_saving_some_money: bool,
    pub coins: u16,
    pub pokedex: PokedexState,
    pub link_battle_stats: LinkBattleStats,
    pub badges: Badges,
    pub overworld: OverworldMemory,
    pub battle: BattleMemory,
    pub battle_result: u8,
    pub battle_active_party_index: Option<usize>,
    pub battle_active_enemy_party_index: Option<usize>,
    pub battle_rewarded_enemy_party_indices: BTreeSet<usize>,
    pub battle_escape_attempts: u8,
    pub battle_player_stat_drop_guard_turns: u8,
    pub battle_pay_day_money: u32,
    pub battle_amulet_coin_active: bool,
    pub wild_encounter_cooldown: u8,
    pub repel_steps_remaining: u16,
    pub active_repel_item: Option<String>,
    pub registered_key_item: Option<String>,
    pub dig_warp_map_name: Option<String>,
    pub dig_warp_index: Option<u16>,
    pub previous_warp_map_name: Option<String>,
    pub previous_warp_index: Option<u16>,
    pub backup_warp_map_name: Option<String>,
    pub backup_warp_index: Option<u16>,
    pub last_spawn_identifier: Option<u16>,
    pub kenji_break_timer: u8,
    pub player_palette_id: u8,
    pub map_block_overrides: BTreeMap<String, BTreeMap<(u16, u16), u16>>,
    pub map_object_overrides: BTreeMap<String, OverworldObjectMapMemory>,
    pub joypad: JoypadMemory,
    pub fishing: FishingMemory,
    pub swarms: SwarmMemory,
    pub step_events: StepEventCounters,
    pub time: TimeState,
    pub unused_two_day_timer: UnusedTwoDayTimerState,
    pub lucky_number_show_flag: bool,
    pub lucky_number_day: Option<u8>,
    pub lucky_id_number: u16,
    pub current_pc_box: usize,
    pub roaming_pokemon: Vec<RoamingPokemonState>,
    pub mystery_gift_unlocked: bool,
    pub mystery_gift: MysteryGiftState,
    pub blue_card_balance: u8,
    pub buenas_password: BuenasPasswordState,
    pub pending_special_battle_type: Option<String>,
    pub magikarp_record: MagikarpRecordState,
    pub day_care: DayCareState,
    pub bug_contest: BugContestState,
    pub link_session: LinkSessionState,
    pub battle_tower: BattleTowerState,
    pub hall_of_fame: HallOfFameState,
    pub mobile_link: MobileLinkState,
    pub gs_healings: u16,
    pub trainer_rankings_healings: u16,
    pub scenes: SceneMemory,
    pub flags: EventFlagMemory,
    pub script_runtime: ScriptRuntimeMemory,
    pub frame_counter: u64,
    pub rng_seed: u32,
    pub has_seen_intro: bool,
}

impl<'de> Deserialize<'de> for GameState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawGameState {
            options: Options,
            player_name: String,
            player_id: u16,
            player_gender: u8,
            party: PartyState,
            storage: PokemonStorage,
            mailbox: Vec<MailboxMail>,
            pending_move_learn: Option<PendingMoveLearn>,
            pending_move_learn_queue: Vec<PendingMoveLearn>,
            bag: Bag,
            money: u32,
            moms_money: u32,
            mom_saving_active: bool,
            mom_saving_some_money: bool,
            coins: u16,
            pokedex: PokedexState,
            link_battle_stats: LinkBattleStats,
            badges: Badges,
            overworld: OverworldMemory,
            battle: BattleMemory,
            battle_result: u8,
            battle_active_party_index: Option<usize>,
            battle_active_enemy_party_index: Option<usize>,
            battle_rewarded_enemy_party_indices: BTreeSet<usize>,
            battle_escape_attempts: u8,
            battle_player_stat_drop_guard_turns: u8,
            battle_pay_day_money: u32,
            battle_amulet_coin_active: bool,
            wild_encounter_cooldown: u8,
            repel_steps_remaining: u16,
            active_repel_item: Option<String>,
            registered_key_item: Option<String>,
            dig_warp_map_name: Option<String>,
            dig_warp_index: Option<u16>,
            #[serde(default)]
            previous_warp_map_name: Option<String>,
            #[serde(default)]
            previous_warp_index: Option<u16>,
            #[serde(default)]
            backup_warp_map_name: Option<String>,
            #[serde(default)]
            backup_warp_index: Option<u16>,
            last_spawn_identifier: Option<u16>,
            kenji_break_timer: u8,
            player_palette_id: u8,
            map_block_overrides: BTreeMap<String, BTreeMap<(u16, u16), u16>>,
            map_object_overrides: BTreeMap<String, OverworldObjectMapMemory>,
            joypad: JoypadMemory,
            fishing: FishingMemory,
            swarms: SwarmMemory,
            step_events: StepEventCounters,
            time: TimeState,
            unused_two_day_timer: UnusedTwoDayTimerState,
            lucky_number_show_flag: bool,
            lucky_number_day: Option<u8>,
            lucky_id_number: u16,
            current_pc_box: usize,
            roaming_pokemon: Vec<RoamingPokemonState>,
            mystery_gift_unlocked: bool,
            mystery_gift: MysteryGiftState,
            blue_card_balance: u8,
            buenas_password: BuenasPasswordState,
            pending_special_battle_type: Option<String>,
            magikarp_record: MagikarpRecordState,
            day_care: DayCareState,
            bug_contest: BugContestState,
            link_session: LinkSessionState,
            battle_tower: BattleTowerState,
            hall_of_fame: HallOfFameState,
            mobile_link: MobileLinkState,
            gs_healings: u16,
            trainer_rankings_healings: u16,
            scenes: SceneMemory,
            flags: EventFlagMemory,
            script_runtime: ScriptRuntimeMemory,
            frame_counter: u64,
            rng_seed: u32,
            has_seen_intro: bool,
        }

        let raw = RawGameState::deserialize(deserializer)?;
        let state = Self {
            options: raw.options,
            player_name: raw.player_name,
            player_id: raw.player_id,
            player_gender: raw.player_gender,
            party: raw.party,
            storage: raw.storage,
            mailbox: raw.mailbox,
            pending_move_learn: raw.pending_move_learn,
            pending_move_learn_queue: raw.pending_move_learn_queue,
            bag: raw.bag,
            money: raw.money,
            moms_money: raw.moms_money,
            mom_saving_active: raw.mom_saving_active,
            mom_saving_some_money: raw.mom_saving_some_money,
            coins: raw.coins,
            pokedex: raw.pokedex,
            link_battle_stats: raw.link_battle_stats,
            badges: raw.badges,
            overworld: raw.overworld,
            battle: raw.battle,
            battle_result: raw.battle_result,
            battle_active_party_index: raw.battle_active_party_index,
            battle_active_enemy_party_index: raw.battle_active_enemy_party_index,
            battle_rewarded_enemy_party_indices: raw.battle_rewarded_enemy_party_indices,
            battle_escape_attempts: raw.battle_escape_attempts,
            battle_player_stat_drop_guard_turns: raw.battle_player_stat_drop_guard_turns,
            battle_pay_day_money: raw.battle_pay_day_money,
            battle_amulet_coin_active: raw.battle_amulet_coin_active,
            wild_encounter_cooldown: raw.wild_encounter_cooldown,
            repel_steps_remaining: raw.repel_steps_remaining,
            active_repel_item: raw.active_repel_item,
            registered_key_item: raw.registered_key_item,
            dig_warp_map_name: raw.dig_warp_map_name,
            dig_warp_index: raw.dig_warp_index,
            previous_warp_map_name: raw.previous_warp_map_name,
            previous_warp_index: raw.previous_warp_index,
            backup_warp_map_name: raw.backup_warp_map_name,
            backup_warp_index: raw.backup_warp_index,
            last_spawn_identifier: raw.last_spawn_identifier,
            kenji_break_timer: raw.kenji_break_timer,
            player_palette_id: raw.player_palette_id,
            map_block_overrides: raw.map_block_overrides,
            map_object_overrides: raw.map_object_overrides,
            joypad: raw.joypad,
            fishing: raw.fishing,
            swarms: raw.swarms,
            step_events: raw.step_events,
            time: raw.time,
            unused_two_day_timer: raw.unused_two_day_timer,
            lucky_number_show_flag: raw.lucky_number_show_flag,
            lucky_number_day: raw.lucky_number_day,
            lucky_id_number: raw.lucky_id_number,
            current_pc_box: raw.current_pc_box,
            roaming_pokemon: raw.roaming_pokemon,
            mystery_gift_unlocked: raw.mystery_gift_unlocked,
            mystery_gift: raw.mystery_gift,
            blue_card_balance: raw.blue_card_balance,
            buenas_password: raw.buenas_password,
            pending_special_battle_type: raw.pending_special_battle_type,
            magikarp_record: raw.magikarp_record,
            day_care: raw.day_care,
            bug_contest: raw.bug_contest,
            link_session: raw.link_session,
            battle_tower: raw.battle_tower,
            hall_of_fame: raw.hall_of_fame,
            mobile_link: raw.mobile_link,
            gs_healings: raw.gs_healings,
            trainer_rankings_healings: raw.trainer_rankings_healings,
            scenes: raw.scenes,
            flags: raw.flags,
            script_runtime: raw.script_runtime,
            frame_counter: raw.frame_counter,
            rng_seed: raw.rng_seed,
            has_seen_intro: raw.has_seen_intro,
        };
        state.validate_saved_state().map_err(D::Error::custom)?;
        Ok(state)
    }
}

impl GameState {
    /// Construct the cleared WRAM state used as the input to Crystal's
    /// new-game initialization program.
    ///
    /// This is deliberately not `Default`: production callers must identify
    /// the lifecycle transition that creates a fresh state, while save loads
    /// continue to enter through validated deserialization.
    pub fn reset_wram_for_new_game() -> Self {
        Self {
            options: Options::default(),
            player_name: String::new(),
            player_id: 0,
            player_gender: PLAYER_GENDER_MALE,
            party: PartyState::default(),
            storage: PokemonStorage::default(),
            mailbox: Vec::new(),
            pending_move_learn: None,
            pending_move_learn_queue: Vec::new(),
            bag: Bag::default(),
            money: 0,
            moms_money: 0,
            mom_saving_active: false,
            mom_saving_some_money: false,
            coins: 0,
            pokedex: PokedexState::default(),
            link_battle_stats: LinkBattleStats::default(),
            badges: Badges::default(),
            overworld: OverworldMemory::default(),
            battle: BattleMemory::default(),
            battle_result: 0,
            battle_active_party_index: None,
            battle_active_enemy_party_index: None,
            battle_rewarded_enemy_party_indices: BTreeSet::new(),
            battle_escape_attempts: 0,
            battle_player_stat_drop_guard_turns: 0,
            battle_pay_day_money: 0,
            battle_amulet_coin_active: false,
            wild_encounter_cooldown: 0,
            repel_steps_remaining: 0,
            active_repel_item: None,
            registered_key_item: None,
            dig_warp_map_name: None,
            dig_warp_index: None,
            previous_warp_map_name: None,
            previous_warp_index: None,
            backup_warp_map_name: None,
            backup_warp_index: None,
            last_spawn_identifier: None,
            kenji_break_timer: 0,
            player_palette_id: 0,
            map_block_overrides: BTreeMap::new(),
            map_object_overrides: BTreeMap::new(),
            joypad: JoypadMemory::default(),
            fishing: FishingMemory::default(),
            swarms: SwarmMemory::default(),
            step_events: StepEventCounters::default(),
            time: TimeState::default(),
            unused_two_day_timer: UnusedTwoDayTimerState::default(),
            lucky_number_show_flag: false,
            lucky_number_day: None,
            lucky_id_number: 0,
            current_pc_box: 0,
            roaming_pokemon: Vec::new(),
            mystery_gift_unlocked: false,
            mystery_gift: MysteryGiftState::default(),
            blue_card_balance: 0,
            buenas_password: BuenasPasswordState::default(),
            pending_special_battle_type: None,
            magikarp_record: MagikarpRecordState::default(),
            day_care: DayCareState::default(),
            bug_contest: BugContestState::default(),
            link_session: LinkSessionState::default(),
            battle_tower: BattleTowerState::default(),
            hall_of_fame: HallOfFameState::default(),
            mobile_link: MobileLinkState::default(),
            gs_healings: 0,
            trainer_rankings_healings: 0,
            scenes: SceneMemory::default(),
            flags: EventFlagMemory::default(),
            script_runtime: ScriptRuntimeMemory::default(),
            frame_counter: 0,
            rng_seed: 1,
            has_seen_intro: false,
        }
    }
}

#[cfg(any(test, feature = "test-fixtures"))]
impl Default for GameState {
    fn default() -> Self {
        Self::reset_wram_for_new_game()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UnusedTwoDayTimerState {
    pub active: bool,
    pub remaining_days: u8,
    pub start_day: u8,
}

impl<'de> Deserialize<'de> for UnusedTwoDayTimerState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawUnusedTwoDayTimerState {
            active: bool,
            remaining_days: u8,
            start_day: u8,
        }

        let raw = RawUnusedTwoDayTimerState::deserialize(deserializer)?;
        let state = Self {
            active: raw.active,
            remaining_days: raw.remaining_days,
            start_day: raw.start_day,
        };
        state.validate_saved_state().map_err(D::Error::custom)?;
        Ok(state)
    }
}

impl GameState {
    /// Run Crystal's post-battle spread step for an already-active infection.
    /// The original scans one side of the party after a one-third chance,
    /// copying the source strain to the first uninfected neighbor. The same
    /// boundary also performs Crystal's rare Shuckle Berry conversion.
    pub fn spread_pokerus_after_battle(&mut self) -> bool {
        let mut rng = Random::new_crystal(self.rng_seed);
        let reached_goldenrod = self
            .flags
            .is_engine_flag_set("ENGINE_REACHED_GOLDENROD")
            .unwrap_or(false);
        if reached_goldenrod && rng.battle_random_byte() < 16 {
            if let Some(shuckle) = self
                .storage
                .party
                .pokemon
                .iter_mut()
                .flatten()
                .find(|mon| mon.species.id == "SHUCKLE" && mon.item.as_deref() == Some("BERRY"))
            {
                shuckle.item = Some("BERRY_JUICE".to_string());
            }
        }
        let Some(source_index) = self
            .storage
            .party
            .pokemon
            .iter()
            .position(|pokemon| pokemon.as_ref().is_some_and(|mon| mon.pokerus & 0x0f != 0))
        else {
            if reached_goldenrod {
                let (add, sub) = rng.crystal_random_add_sub();
                if add == 0 && sub < 3 {
                    let party_count = self.storage.party.filled_slots() as u8;
                    if party_count > 0 {
                        let target_index = loop {
                            let (_, sample) = rng.crystal_random_add_sub();
                            let index = sample & 0x07;
                            if index < party_count {
                                break usize::from(index);
                            }
                        };
                        let eligible = self.storage.party.pokemon[target_index]
                            .as_ref()
                            .is_some_and(|mon| mon.pokerus & 0xf0 == 0);
                        if eligible {
                            let sample = loop {
                                let (_, sample) = rng.crystal_random_add_sub();
                                if sample != 0 {
                                    break sample;
                                }
                            };
                            let duration_seed = if sample & 0xf0 == 0 {
                                sample
                            } else {
                                (sample & 0x07).saturating_add(1)
                            };
                            let status = (duration_seed << 4) | ((duration_seed & 0x03) + 1);
                            let infected = if let Some(mon) =
                                self.storage.party.pokemon[target_index].as_mut()
                            {
                                mon.pokerus = status;
                                true
                            } else {
                                false
                            };
                            if infected {
                                self.rng_seed = rng.seed();
                                self.sync_party_from_storage();
                                return true;
                            }
                        }
                    }
                }
            }
            self.rng_seed = rng.seed();
            return false;
        };
        if rng.battle_random_byte() >= 85 || self.storage.party.filled_slots() <= 1 {
            self.rng_seed = rng.seed();
            return false;
        }
        let scan_forward = rng.battle_random_byte() >= 128;
        let source_status = self.storage.party.pokemon[source_index]
            .as_ref()
            .map(|mon| mon.pokerus)
            .unwrap_or(0);
        let mut target = if scan_forward {
            source_index.checked_add(1)
        } else {
            source_index.checked_sub(1)
        };
        let target_index = loop {
            let Some(index) = target else { break None };
            let Some(mon) = self.storage.party.pokemon[index].as_ref() else {
                target = if scan_forward {
                    index.checked_add(1)
                } else {
                    index.checked_sub(1)
                };
                continue;
            };
            if mon.pokerus == 0 {
                break Some(index);
            }
            if mon.pokerus & 0x03 == 0 {
                break None;
            }
            target = if scan_forward {
                index.checked_add(1)
            } else {
                index.checked_sub(1)
            };
        };
        let Some(target_index) = target_index else {
            self.rng_seed = rng.seed();
            return false;
        };
        let strain_duration = ((source_status >> 4) & 0x03).max(1);
        if let Some(mon) = self.storage.party.pokemon[target_index].as_mut() {
            mon.pokerus = (source_status & 0xf0) | strain_duration;
        }
        self.rng_seed = rng.seed();
        self.sync_party_from_storage();
        true
    }
}

impl UnusedTwoDayTimerState {
    pub fn validate_saved_state(&self) -> Result<(), String> {
        if self.remaining_days > 2 {
            return Err(format!(
                "remaining_days {} exceeds the two-day timer length",
                self.remaining_days
            ));
        }
        if !self.active && (self.remaining_days != 0 || self.start_day != 0) {
            return Err("inactive timer must have remaining_days 0 and start_day 0".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PendingMoveLearn {
    pub party_index: usize,
    pub species_id: String,
    pub level: u8,
    pub learned_move: LearnedMove,
    pub defer_level_evolution: bool,
}

impl<'de> Deserialize<'de> for PendingMoveLearn {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawPendingMoveLearn {
            party_index: usize,
            species_id: String,
            level: u8,
            learned_move: LearnedMove,
            defer_level_evolution: bool,
        }

        let raw = RawPendingMoveLearn::deserialize(deserializer)?;
        let pending = Self {
            party_index: raw.party_index,
            species_id: raw.species_id,
            level: raw.level,
            learned_move: raw.learned_move,
            defer_level_evolution: raw.defer_level_evolution,
        };
        pending
            .validate_saved_state()
            .map_err(serde::de::Error::custom)?;
        Ok(pending)
    }
}

impl PendingMoveLearn {
    pub fn validate_saved_state(&self) -> Result<(), String> {
        if self.party_index >= PARTY_SIZE {
            return Err(format!(
                "pending_move_learn.party_index {} is outside party range 0..{}",
                self.party_index, PARTY_SIZE
            ));
        }
        validate_script_runtime_token("pending_move_learn.species_id", &self.species_id)?;
        if self.level == 0 {
            return Err("pending_move_learn.level must be nonzero".to_string());
        }
        self.learned_move
            .validate_saved_state(0)
            .map_err(|error| format!("invalid pending_move_learn.learned_move: {error}"))?;
        Ok(())
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LinkSessionState {
    pub link_mode: u8,
    pub player_link_action: u8,
    pub chosen_cable_club_room: u8,
    pub other_player_link_mode: u8,
    pub serial_connection_status: LinkSerialConnectionStatus,
    pub friend_ready: bool,
    pub last_result: bool,
    pub failed_link_to_past: bool,
    pub quick_save_requested: bool,
    pub active_room: Option<String>,
}

impl<'de> Deserialize<'de> for LinkSessionState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawLinkSessionState {
            link_mode: u8,
            player_link_action: u8,
            chosen_cable_club_room: u8,
            other_player_link_mode: u8,
            serial_connection_status: LinkSerialConnectionStatus,
            friend_ready: bool,
            last_result: bool,
            failed_link_to_past: bool,
            quick_save_requested: bool,
            active_room: Option<String>,
        }

        let raw = RawLinkSessionState::deserialize(deserializer)?;
        let state = Self {
            link_mode: raw.link_mode,
            player_link_action: raw.player_link_action,
            chosen_cable_club_room: raw.chosen_cable_club_room,
            other_player_link_mode: raw.other_player_link_mode,
            serial_connection_status: raw.serial_connection_status,
            friend_ready: raw.friend_ready,
            last_result: raw.last_result,
            failed_link_to_past: raw.failed_link_to_past,
            quick_save_requested: raw.quick_save_requested,
            active_room: raw.active_room,
        };
        state.validate_saved_state().map_err(D::Error::custom)?;
        Ok(state)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum LinkSerialConnectionStatus {
    #[default]
    NotEstablished,
    UsingExternalClock,
}

impl LinkSessionState {
    fn validate_saved_state(&self) -> Result<(), String> {
        validate_optional_script_runtime_token(
            "link_session.active_room",
            self.active_room.as_deref(),
        )?;
        if let Some(room) = &self.active_room {
            if self.link_mode == 0 {
                return Err(format!(
                    "link_session.active_room {room} cannot be saved with link_mode 0"
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum LinkSessionSaveError {
    #[error("saved link_session.active_room {room} is missing from compiled pack special routines")]
    MissingActiveRoom { room: String },
}

pub fn validate_saved_link_session_references<F>(
    link_session: &LinkSessionState,
    special_routine_exists: F,
) -> Result<(), LinkSessionSaveError>
where
    F: Fn(&str) -> bool,
{
    if let Some(room) = &link_session.active_room {
        if !special_routine_exists(room) {
            return Err(LinkSessionSaveError::MissingActiveRoom { room: room.clone() });
        }
    }
    Ok(())
}

pub const HALL_OF_FAME_TEAM_SIZE: usize = 6;
pub const HALL_OF_FAME_ENTRY_LIMIT: usize = 50;
pub const HALL_OF_FAME_MASTER_COUNT: u8 = 200;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HallOfFamePokemon {
    pub species: String,
    pub trainer_id: u16,
    pub dvs: u16,
    pub level: u8,
    pub nickname: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HallOfFameEntry {
    pub win_count: u8,
    pub team: [Option<HallOfFamePokemon>; HALL_OF_FAME_TEAM_SIZE],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HallOfFameState {
    pub count: u8,
    pub entries: Vec<HallOfFameEntry>,
    pub spawn_after_champion: Option<u16>,
}

impl Default for HallOfFameState {
    fn default() -> Self {
        Self {
            count: 0,
            entries: Vec::new(),
            spawn_after_champion: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BattleTowerState {
    pub challenge_state: u8,
    pub beaten_trainers: u8,
    pub trainer_history: Vec<u8>,
    pub level_group: u8,
    pub reward_item: String,
    pub reward_given: bool,
    pub quick_saved: bool,
    pub explanation_read: bool,
    pub save_file_flags: u8,
    pub gs_ball_flag: bool,
    pub record_streaks: Vec<u8>,
    pub record_outcomes: Vec<bool>,
    pub record_days: Vec<u8>,
    pub record_state: u8,
    pub record_last_day: Option<u8>,
    pub record_reset_counter: u8,
    pub leaderboard_acknowledged: bool,
    pub last_rule_failure: Option<String>,
    pub loaded_trainer_id: Option<String>,
    pub last_sprite_constant: Option<String>,
    pub selected_party_indexes: Vec<usize>,
    pub mobile_flags: BTreeSet<String>,
}

impl<'de> Deserialize<'de> for BattleTowerState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawBattleTowerState {
            challenge_state: u8,
            beaten_trainers: u8,
            trainer_history: Vec<u8>,
            level_group: u8,
            reward_item: String,
            reward_given: bool,
            quick_saved: bool,
            explanation_read: bool,
            save_file_flags: u8,
            gs_ball_flag: bool,
            record_streaks: Vec<u8>,
            record_outcomes: Vec<bool>,
            record_days: Vec<u8>,
            record_state: u8,
            record_last_day: Option<u8>,
            record_reset_counter: u8,
            leaderboard_acknowledged: bool,
            last_rule_failure: Option<String>,
            loaded_trainer_id: Option<String>,
            last_sprite_constant: Option<String>,
            selected_party_indexes: Vec<usize>,
            mobile_flags: BTreeSet<String>,
        }

        let raw = RawBattleTowerState::deserialize(deserializer)?;
        let state = Self {
            challenge_state: raw.challenge_state,
            beaten_trainers: raw.beaten_trainers,
            trainer_history: raw.trainer_history,
            level_group: raw.level_group,
            reward_item: raw.reward_item,
            reward_given: raw.reward_given,
            quick_saved: raw.quick_saved,
            explanation_read: raw.explanation_read,
            save_file_flags: raw.save_file_flags,
            gs_ball_flag: raw.gs_ball_flag,
            record_streaks: raw.record_streaks,
            record_outcomes: raw.record_outcomes,
            record_days: raw.record_days,
            record_state: raw.record_state,
            record_last_day: raw.record_last_day,
            record_reset_counter: raw.record_reset_counter,
            leaderboard_acknowledged: raw.leaderboard_acknowledged,
            last_rule_failure: raw.last_rule_failure,
            loaded_trainer_id: raw.loaded_trainer_id,
            last_sprite_constant: raw.last_sprite_constant,
            selected_party_indexes: raw.selected_party_indexes,
            mobile_flags: raw.mobile_flags,
        };
        state.validate_saved_state().map_err(D::Error::custom)?;
        Ok(state)
    }
}

impl Default for BattleTowerState {
    fn default() -> Self {
        Self {
            challenge_state: 0,
            beaten_trainers: 0,
            trainer_history: vec![0xff; 7],
            level_group: 0,
            reward_item: "POTION".to_string(),
            reward_given: false,
            quick_saved: false,
            explanation_read: false,
            save_file_flags: 0,
            gs_ball_flag: false,
            record_streaks: Vec::new(),
            record_outcomes: Vec::new(),
            record_days: Vec::new(),
            record_state: 0,
            record_last_day: None,
            record_reset_counter: 0,
            leaderboard_acknowledged: false,
            last_rule_failure: None,
            loaded_trainer_id: None,
            last_sprite_constant: None,
            selected_party_indexes: Vec::new(),
            mobile_flags: BTreeSet::new(),
        }
    }
}

impl BattleTowerState {
    fn validate_saved_state(&self) -> Result<(), String> {
        if self.challenge_state > 4 {
            return Err(format!(
                "battle_tower.challenge_state {} is outside Crystal range 0..4",
                self.challenge_state
            ));
        }
        let unknown_save_file_flags = self.save_file_flags & !BATTLE_TOWER_SAVE_FILE_FLAGS_MASK;
        if unknown_save_file_flags != 0 {
            return Err(format!(
                "battle_tower.save_file_flags {:#04x} contains unknown bits {:#04x}",
                self.save_file_flags, unknown_save_file_flags
            ));
        }
        if self.record_reset_counter > 2 {
            return Err(format!(
                "battle_tower.record_reset_counter {} is outside Crystal range 0..2",
                self.record_reset_counter
            ));
        }
        validate_script_runtime_token("battle_tower.reward_item", &self.reward_item)?;
        validate_optional_script_runtime_token(
            "battle_tower.last_rule_failure",
            self.last_rule_failure.as_deref(),
        )?;
        if let Some(failure) = &self.last_rule_failure {
            if !BATTLE_TOWER_RULE_FAILURES.contains(&failure.as_str()) {
                return Err(format!(
                    "battle_tower.last_rule_failure {failure} is not a saved Battle Tower rule failure"
                ));
            }
        }
        validate_optional_script_runtime_token(
            "battle_tower.loaded_trainer_id",
            self.loaded_trainer_id.as_deref(),
        )?;
        validate_optional_script_runtime_token(
            "battle_tower.last_sprite_constant",
            self.last_sprite_constant.as_deref(),
        )?;
        if self.record_streaks.len() != self.record_outcomes.len()
            || self.record_streaks.len() != self.record_days.len()
        {
            return Err(format!(
                "battle_tower record vectors have inconsistent lengths: streaks {}, outcomes {}, days {}",
                self.record_streaks.len(),
                self.record_outcomes.len(),
                self.record_days.len()
            ));
        }
        let mut selected_party_indexes = BTreeSet::new();
        for index in &self.selected_party_indexes {
            if *index >= PARTY_SIZE {
                return Err(format!(
                    "battle_tower.selected_party_indexes contains {index}, outside party range 0..{PARTY_SIZE}"
                ));
            }
            if !selected_party_indexes.insert(index) {
                return Err(format!(
                    "battle_tower.selected_party_indexes contains duplicate party index {index}"
                ));
            }
        }
        for flag in &self.mobile_flags {
            validate_script_runtime_token("battle_tower.mobile_flags", flag)?;
            if !BATTLE_TOWER_MOBILE_FLAGS.contains(&flag.as_str()) {
                return Err(format!(
                    "battle_tower.mobile_flags {flag} is not a saved Battle Tower mobile flag"
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MobileLinkState {
    pub mode: Option<String>,
    pub adapter_status: String,
    pub adapter_secondary_status: String,
    pub battle_timer: [u8; 3],
    pub login_password: String,
    pub handshakes: u32,
    pub leaderboard: Vec<MobileBattleTowerRecord>,
    pub terminated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MobileBattleTowerRecord {
    pub streak: u8,
    pub outcome: String,
    pub day: u8,
}

impl<'de> Deserialize<'de> for MobileLinkState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawMobileLinkState {
            mode: Option<String>,
            adapter_status: String,
            adapter_secondary_status: String,
            battle_timer: [u8; 3],
            login_password: String,
            handshakes: u32,
            leaderboard: Vec<MobileBattleTowerRecord>,
            terminated: bool,
        }

        let raw = RawMobileLinkState::deserialize(deserializer)?;
        let state = Self {
            mode: raw.mode,
            adapter_status: raw.adapter_status,
            adapter_secondary_status: raw.adapter_secondary_status,
            battle_timer: raw.battle_timer,
            login_password: raw.login_password,
            handshakes: raw.handshakes,
            leaderboard: raw.leaderboard,
            terminated: raw.terminated,
        };
        state.validate_saved_state().map_err(D::Error::custom)?;
        Ok(state)
    }
}

impl<'de> Deserialize<'de> for MobileBattleTowerRecord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawMobileBattleTowerRecord {
            streak: u8,
            outcome: String,
            day: u8,
        }

        let raw = RawMobileBattleTowerRecord::deserialize(deserializer)?;
        let record = Self {
            streak: raw.streak,
            outcome: raw.outcome,
            day: raw.day,
        };
        record.validate_saved_state(0).map_err(D::Error::custom)?;
        Ok(record)
    }
}

impl MobileLinkState {
    fn validate_saved_state(&self) -> Result<(), String> {
        validate_optional_script_runtime_token("mobile_link.mode", self.mode.as_deref())?;
        validate_empty_or_script_runtime_token("mobile_link.adapter_status", &self.adapter_status)?;
        validate_empty_or_script_runtime_token(
            "mobile_link.adapter_secondary_status",
            &self.adapter_secondary_status,
        )?;
        if self.login_password.len() > MOBILE_LOGIN_PASSWORD_LENGTH {
            return Err(format!(
                "mobile_link.login_password length {} exceeds Crystal mobile password limit {}",
                self.login_password.len(),
                MOBILE_LOGIN_PASSWORD_LENGTH
            ));
        }
        if !self.login_password.is_empty()
            && (self.login_password.trim() != self.login_password
                || self.login_password.chars().any(char::is_control))
        {
            return Err("mobile_link.login_password must be exact text".to_string());
        }
        if self.terminated && self.handshakes == 0 {
            return Err(
                "mobile_link.terminated cannot be saved before a mobile handshake".to_string(),
            );
        }
        if let Some(mode) = &self.mode {
            if self.handshakes == 0 {
                return Err(format!(
                    "mobile_link.mode {mode} cannot be saved before a mobile handshake"
                ));
            }
        }
        if self.handshakes == 0 {
            if !self.adapter_status.is_empty() {
                return Err(format!(
                    "mobile_link.adapter_status {} cannot be saved before a mobile handshake",
                    self.adapter_status
                ));
            }
            if !self.adapter_secondary_status.is_empty() {
                return Err(format!(
                    "mobile_link.adapter_secondary_status {} cannot be saved before a mobile handshake",
                    self.adapter_secondary_status
                ));
            }
            if self.battle_timer != [0; 3] {
                return Err(format!(
                    "mobile_link.battle_timer {:?} cannot be saved before a mobile handshake",
                    self.battle_timer
                ));
            }
            if !self.login_password.is_empty() {
                return Err(
                    "mobile_link.login_password cannot be saved before a mobile handshake"
                        .to_string(),
                );
            }
        }
        for (index, record) in self.leaderboard.iter().enumerate() {
            record.validate_saved_state(index)?;
        }
        if self.handshakes == 0 && !self.leaderboard.is_empty() {
            return Err(format!(
                "mobile_link.leaderboard has {} records before a mobile handshake",
                self.leaderboard.len()
            ));
        }
        Ok(())
    }
}

impl MobileBattleTowerRecord {
    fn validate_saved_state(&self, index: usize) -> Result<(), String> {
        validate_script_runtime_token(
            &format!("mobile_link.leaderboard[{index}].outcome"),
            &self.outcome,
        )?;
        if !matches!(self.outcome.as_str(), "win" | "loss") {
            return Err(format!(
                "mobile_link.leaderboard[{index}].outcome {} is not a saved mobile Battle Tower outcome",
                self.outcome
            ));
        }
        if self.day >= 7 {
            return Err(format!(
                "mobile_link.leaderboard[{index}].day {} is outside weekday range 0..6",
                self.day
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BugContestState {
    pub park_balls_remaining: u8,
    pub timer_active: bool,
    pub timer_minutes_remaining: u8,
    pub timer_seconds_remaining: u8,
    pub timer_start_time: Option<ClockTime>,
    pub party_backup: Vec<Pokemon>,
    pub second_party_species: Option<String>,
    pub caught_mon: Option<Pokemon>,
    pub pending_caught_mon: Option<Pokemon>,
    pub caught_species: Option<String>,
    pub caught_level: Option<u8>,
    pub selected_contestant_flags: Vec<String>,
    pub last_rank: Option<u8>,
    pub last_result: Option<u8>,
}

impl<'de> Deserialize<'de> for BugContestState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawBugContestState {
            park_balls_remaining: u8,
            timer_active: bool,
            timer_minutes_remaining: u8,
            timer_seconds_remaining: u8,
            #[serde(default)]
            timer_start_time: Option<ClockTime>,
            party_backup: Vec<Pokemon>,
            second_party_species: Option<String>,
            caught_mon: Option<Pokemon>,
            #[serde(default)]
            pending_caught_mon: Option<Pokemon>,
            caught_species: Option<String>,
            caught_level: Option<u8>,
            selected_contestant_flags: Vec<String>,
            last_rank: Option<u8>,
            last_result: Option<u8>,
        }

        let raw = RawBugContestState::deserialize(deserializer)?;
        let state = Self {
            park_balls_remaining: raw.park_balls_remaining,
            timer_active: raw.timer_active,
            timer_minutes_remaining: raw.timer_minutes_remaining,
            timer_seconds_remaining: raw.timer_seconds_remaining,
            timer_start_time: raw.timer_start_time,
            party_backup: raw.party_backup,
            second_party_species: raw.second_party_species,
            caught_mon: raw.caught_mon,
            pending_caught_mon: raw.pending_caught_mon,
            caught_species: raw.caught_species,
            caught_level: raw.caught_level,
            selected_contestant_flags: raw.selected_contestant_flags,
            last_rank: raw.last_rank,
            last_result: raw.last_result,
        };
        state.validate_saved_state().map_err(D::Error::custom)?;
        Ok(state)
    }
}

impl BugContestState {
    fn validate_saved_state(&self) -> Result<(), String> {
        if self.timer_seconds_remaining >= 60 {
            return Err(format!(
                "bug_contest.timer_seconds_remaining {} is outside clock range 0..59",
                self.timer_seconds_remaining
            ));
        }
        validate_optional_script_runtime_token(
            "bug_contest.second_party_species",
            self.second_party_species.as_deref(),
        )?;
        validate_optional_script_runtime_token(
            "bug_contest.caught_species",
            self.caught_species.as_deref(),
        )?;
        match (&self.caught_mon, &self.caught_species, self.caught_level) {
            (Some(pokemon), Some(species), Some(level)) => {
                pokemon
                    .validate_saved_state()
                    .map_err(|error| format!("bug_contest.caught_mon: {error}"))?;
                if pokemon.species.id != *species {
                    return Err(format!(
                        "bug_contest.caught_species {species} does not match caught_mon species {}",
                        pokemon.species.id
                    ));
                }
                if pokemon.level != level {
                    return Err(format!(
                        "bug_contest.caught_level {level} does not match caught_mon level {}",
                        pokemon.level
                    ));
                }
            }
            (Some(_), _, _) => {
                return Err(
                    "bug_contest.caught_mon requires caught_species and caught_level".to_string(),
                );
            }
            (None, Some(species), _) => {
                return Err(format!(
                    "bug_contest.caught_species {species} cannot be saved without caught_mon"
                ));
            }
            (None, None, Some(level)) => {
                return Err(format!(
                    "bug_contest.caught_level {level} cannot be saved without caught_mon"
                ));
            }
            (None, None, None) => {}
        }
        if let Some(pokemon) = &self.pending_caught_mon {
            pokemon
                .validate_saved_state()
                .map_err(|error| format!("bug_contest.pending_caught_mon: {error}"))?;
        }
        for flag in &self.selected_contestant_flags {
            validate_flag_name(flag)
                .map_err(|error| format!("bug_contest.selected_contestant_flags {error}"))?;
        }
        Ok(())
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MagikarpRecordState {
    pub current_feet: u8,
    pub current_inches: u8,
    pub best_feet: u8,
    pub best_inches: u8,
    pub best_owner_name: String,
}

impl<'de> Deserialize<'de> for MagikarpRecordState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawMagikarpRecordState {
            current_feet: u8,
            current_inches: u8,
            best_feet: u8,
            best_inches: u8,
            best_owner_name: String,
        }

        let raw = RawMagikarpRecordState::deserialize(deserializer)?;
        let state = Self {
            current_feet: raw.current_feet,
            current_inches: raw.current_inches,
            best_feet: raw.best_feet,
            best_inches: raw.best_inches,
            best_owner_name: raw.best_owner_name,
        };
        state.validate_saved_state().map_err(D::Error::custom)?;
        Ok(state)
    }
}

impl MagikarpRecordState {
    fn validate_saved_state(&self) -> Result<(), String> {
        validate_inches_field("magikarp_record.current_inches", self.current_inches)?;
        validate_inches_field("magikarp_record.best_inches", self.best_inches)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DayCareState {
    pub man: DayCareResidentState,
    pub lady: DayCareResidentState,
    pub compatibility_score: u8,
    pub egg_present: bool,
    /// The concrete egg generated by breeding.  Keeping the record (rather
    /// than only a boolean flag) makes pickup deterministic and saveable.
    #[serde(default)]
    pub egg: Option<Pokemon>,
    pub steps_until_next_egg: u16,
    pub steps_since_last_egg: u8,
    pub last_interaction: Option<DayCareInteractionState>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DayCareResidentState {
    pub pokemon: Option<Pokemon>,
    pub initial_experience: i32,
    pub initial_level: u8,
    pub steps: u32,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DayCareInteractionState {
    pub caretaker: String,
    pub action: String,
    pub success: bool,
    pub pokemon: Option<String>,
    pub level: Option<u8>,
    pub reason: Option<String>,
}

impl<'de> Deserialize<'de> for DayCareState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawDayCareState {
            man: DayCareResidentState,
            lady: DayCareResidentState,
            compatibility_score: u8,
            egg_present: bool,
            #[serde(default)]
            egg: Option<Pokemon>,
            steps_until_next_egg: u16,
            steps_since_last_egg: u8,
            last_interaction: Option<DayCareInteractionState>,
        }

        let raw = RawDayCareState::deserialize(deserializer)?;
        let state = Self {
            man: raw.man,
            lady: raw.lady,
            compatibility_score: raw.compatibility_score,
            egg_present: raw.egg_present,
            egg: raw.egg,
            steps_until_next_egg: raw.steps_until_next_egg,
            steps_since_last_egg: raw.steps_since_last_egg,
            last_interaction: raw.last_interaction,
        };
        state.validate_saved_state().map_err(D::Error::custom)?;
        Ok(state)
    }
}

impl<'de> Deserialize<'de> for DayCareResidentState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawDayCareResidentState {
            pokemon: Option<Pokemon>,
            initial_experience: i32,
            initial_level: u8,
            steps: u32,
            active: bool,
        }

        let raw = RawDayCareResidentState::deserialize(deserializer)?;
        let state = Self {
            pokemon: raw.pokemon,
            initial_experience: raw.initial_experience,
            initial_level: raw.initial_level,
            steps: raw.steps,
            active: raw.active,
        };
        state
            .validate_saved_state("day_care.resident")
            .map_err(D::Error::custom)?;
        Ok(state)
    }
}

impl<'de> Deserialize<'de> for DayCareInteractionState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawDayCareInteractionState {
            caretaker: String,
            action: String,
            success: bool,
            pokemon: Option<String>,
            level: Option<u8>,
            reason: Option<String>,
        }

        let raw = RawDayCareInteractionState::deserialize(deserializer)?;
        let state = Self {
            caretaker: raw.caretaker,
            action: raw.action,
            success: raw.success,
            pokemon: raw.pokemon,
            level: raw.level,
            reason: raw.reason,
        };
        state.validate_saved_state().map_err(D::Error::custom)?;
        Ok(state)
    }
}

impl DayCareState {
    fn validate_saved_state(&self) -> Result<(), String> {
        self.man.validate_saved_state("day_care.man")?;
        self.lady.validate_saved_state("day_care.lady")?;
        let both_residents_active = self.man.active && self.lady.active;
        if self.egg.is_some() && !self.egg_present {
            return Err("day_care.egg cannot exist without egg_present".to_string());
        }
        if self.egg.as_ref().is_some_and(|egg| !egg.is_egg) {
            return Err("day_care.egg must retain egg identity".to_string());
        }
        if self.egg_present && !both_residents_active {
            return Err(
                "day_care.egg_present cannot be saved without both residents active".to_string(),
            );
        }
        if self.compatibility_score != 0 && !both_residents_active {
            return Err(format!(
                "day_care.compatibility_score {} cannot be saved without both residents active",
                self.compatibility_score
            ));
        }
        if self.steps_until_next_egg != 0 && !both_residents_active {
            return Err(format!(
                "day_care.steps_until_next_egg {} cannot be saved without both residents active",
                self.steps_until_next_egg
            ));
        }
        if self.steps_since_last_egg != 0 && !both_residents_active {
            return Err(format!(
                "day_care.steps_since_last_egg {} cannot be saved without both residents active",
                self.steps_since_last_egg
            ));
        }
        if let Some(interaction) = &self.last_interaction {
            interaction.validate_saved_state()?;
        }
        Ok(())
    }
}

impl DayCareResidentState {
    fn validate_saved_state(&self, field: &str) -> Result<(), String> {
        let Some(pokemon) = &self.pokemon else {
            if self.active {
                return Err(format!("{field}.active cannot be saved without a Pokemon"));
            }
            if self.initial_experience != 0 {
                return Err(format!(
                    "{field}.initial_experience {} cannot be saved without a Pokemon",
                    self.initial_experience
                ));
            }
            if self.initial_level != 0 {
                return Err(format!(
                    "{field}.initial_level {} cannot be saved without a Pokemon",
                    self.initial_level
                ));
            }
            if self.steps != 0 {
                return Err(format!(
                    "{field}.steps {} cannot be saved without a Pokemon",
                    self.steps
                ));
            }
            return Ok(());
        };
        if self.initial_level == 0 {
            return Err(format!("{field}.initial_level must be nonzero"));
        }
        if !self.active {
            return Err(format!(
                "{field}.active must be true when a Pokemon is deposited"
            ));
        }
        if self.initial_experience < 0 {
            return Err(format!(
                "{field}.initial_experience {} must be nonnegative",
                self.initial_experience
            ));
        }
        pokemon
            .validate_saved_state()
            .map_err(|error| format!("{field}.pokemon: {error}"))?;
        Ok(())
    }
}

impl DayCareInteractionState {
    fn validate_saved_state(&self) -> Result<(), String> {
        validate_script_runtime_token("day_care.last_interaction.caretaker", &self.caretaker)?;
        validate_script_runtime_token("day_care.last_interaction.action", &self.action)?;
        validate_optional_script_runtime_token(
            "day_care.last_interaction.pokemon",
            self.pokemon.as_deref(),
        )?;
        validate_optional_script_runtime_token(
            "day_care.last_interaction.reason",
            self.reason.as_deref(),
        )?;
        if !matches!(self.caretaker.as_str(), "man" | "lady") {
            return Err(format!(
                "day_care.last_interaction.caretaker {} is not a saved Day Care caretaker",
                self.caretaker
            ));
        }
        if self.action == "collect_egg" && self.caretaker != "man" {
            return Err(format!(
                "day_care.last_interaction.action collect_egg requires caretaker man, got {}",
                self.caretaker
            ));
        }
        if let Some(level) = self.level {
            if level == 0 {
                return Err("day_care.last_interaction.level must be nonzero".to_string());
            }
        }
        match self.action.as_str() {
            "deposit" => {
                if !self.success
                    && matches!(
                        self.reason.as_deref(),
                        Some("occupied" | "egg" | "fainted" | "mail" | "last_usable")
                    )
                {
                    validate_day_care_pokemon_and_reason(
                        "day_care.last_interaction",
                        self,
                        false,
                        self.reason.as_deref(),
                    )
                } else {
                    validate_day_care_interaction_payload(
                        "day_care.last_interaction",
                        self,
                        DayCareInteractionShape {
                            success_pokemon: true,
                            success_reason: None,
                            failure_reason: Some("occupied"),
                            failure_pokemon: false,
                        },
                    )
                }
            }
            "withdraw" => {
                if self.success {
                    validate_day_care_pokemon_and_reason(
                        "day_care.last_interaction",
                        self,
                        true,
                        None,
                    )
                } else {
                    match self.reason.as_deref() {
                        Some("empty") => validate_day_care_pokemon_and_reason(
                            "day_care.last_interaction",
                            self,
                            false,
                            Some("empty"),
                        ),
                        Some("party_full") => validate_day_care_pokemon_and_reason(
                            "day_care.last_interaction",
                            self,
                            true,
                            Some("party_full"),
                        ),
                        Some("not_enough_money") => validate_day_care_pokemon_and_reason(
                            "day_care.last_interaction",
                            self,
                            true,
                            Some("not_enough_money"),
                        ),
                        Some(reason) => Err(format!(
                            "day_care.last_interaction.reason {reason} is not valid for failed withdraw"
                        )),
                        None => Err(
                            "day_care.last_interaction.reason is required for failed withdraw"
                                .to_string(),
                        ),
                    }
                }
            }
            "inspect" => validate_day_care_interaction_payload(
                "day_care.last_interaction",
                self,
                DayCareInteractionShape {
                    success_pokemon: true,
                    success_reason: None,
                    failure_reason: Some("empty"),
                    failure_pokemon: false,
                },
            ),
            "collect_egg" => validate_day_care_interaction_payload(
                "day_care.last_interaction",
                self,
                DayCareInteractionShape {
                    success_pokemon: false,
                    success_reason: None,
                    failure_reason: Some("no_egg"),
                    failure_pokemon: false,
                },
            ),
            action => Err(format!(
                "day_care.last_interaction.action {action} is not a saved Day Care action"
            )),
        }
    }
}

struct DayCareInteractionShape {
    success_pokemon: bool,
    success_reason: Option<&'static str>,
    failure_reason: Option<&'static str>,
    failure_pokemon: bool,
}

fn validate_day_care_interaction_payload(
    field: &str,
    interaction: &DayCareInteractionState,
    shape: DayCareInteractionShape,
) -> Result<(), String> {
    if interaction.success {
        validate_day_care_pokemon_and_reason(
            field,
            interaction,
            shape.success_pokemon,
            shape.success_reason,
        )
    } else {
        validate_day_care_pokemon_and_reason(
            field,
            interaction,
            shape.failure_pokemon,
            shape.failure_reason,
        )
    }
}

fn validate_day_care_pokemon_and_reason(
    field: &str,
    interaction: &DayCareInteractionState,
    expect_pokemon: bool,
    expected_reason: Option<&str>,
) -> Result<(), String> {
    match (expect_pokemon, &interaction.pokemon, interaction.level) {
        (true, Some(_), Some(_)) | (false, None, None) => {}
        (true, _, _) => {
            return Err(format!(
                "{field}.action {} requires pokemon and level",
                interaction.action
            ));
        }
        (false, Some(pokemon), _) => {
            return Err(format!(
                "{field}.pokemon {pokemon} is not allowed for action {}",
                interaction.action
            ));
        }
        (false, None, Some(level)) => {
            return Err(format!(
                "{field}.level {level} is not allowed for action {}",
                interaction.action
            ));
        }
    }
    match (expected_reason, interaction.reason.as_deref()) {
        (Some(expected), Some(actual)) if expected == actual => Ok(()),
        (Some(expected), Some(actual)) => Err(format!(
            "{field}.reason {actual} does not match expected reason {expected}"
        )),
        (Some(expected), None) => Err(format!("{field}.reason {expected} is required")),
        (None, Some(actual)) => Err(format!("{field}.reason {actual} is not allowed")),
        (None, None) => Ok(()),
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MysteryGiftState {
    pub stored_item: Option<String>,
    pub backup_item: Option<String>,
    pub trainer_house_flag: bool,
}

impl<'de> Deserialize<'de> for MysteryGiftState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawMysteryGiftState {
            stored_item: Option<String>,
            backup_item: Option<String>,
            trainer_house_flag: bool,
        }

        let raw = RawMysteryGiftState::deserialize(deserializer)?;
        let state = Self {
            stored_item: raw.stored_item,
            backup_item: raw.backup_item,
            trainer_house_flag: raw.trainer_house_flag,
        };
        state.validate_saved_state().map_err(D::Error::custom)?;
        Ok(state)
    }
}

impl MysteryGiftState {
    fn validate_saved_state(&self) -> Result<(), String> {
        validate_optional_script_runtime_token(
            "mystery_gift.stored_item",
            self.stored_item.as_deref(),
        )?;
        validate_optional_script_runtime_token(
            "mystery_gift.backup_item",
            self.backup_item.as_deref(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum MysteryGiftSaveError {
    #[error("saved {path} {item_id} is missing from compiled pack items")]
    MissingItem { path: &'static str, item_id: String },
}

pub fn validate_saved_mystery_gift_references<F>(
    mystery_gift: &MysteryGiftState,
    item_exists: F,
) -> Result<(), MysteryGiftSaveError>
where
    F: Fn(&str) -> bool,
{
    if let Some(item_id) = &mystery_gift.stored_item {
        if !item_exists(item_id) {
            return Err(MysteryGiftSaveError::MissingItem {
                path: "mystery_gift.stored_item",
                item_id: item_id.clone(),
            });
        }
    }
    if let Some(item_id) = &mystery_gift.backup_item {
        if !item_exists(item_id) {
            return Err(MysteryGiftSaveError::MissingItem {
                path: "mystery_gift.backup_item",
                item_id: item_id.clone(),
            });
        }
    }
    Ok(())
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuenasPasswordState {
    pub category_index: usize,
    pub option_index: usize,
    pub generation_day: u8,
    pub generated: bool,
}

impl<'de> Deserialize<'de> for BuenasPasswordState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawBuenasPasswordState {
            category_index: usize,
            option_index: usize,
            generation_day: u8,
            generated: bool,
        }

        let raw = RawBuenasPasswordState::deserialize(deserializer)?;
        let state = Self {
            category_index: raw.category_index,
            option_index: raw.option_index,
            generation_day: raw.generation_day,
            generated: raw.generated,
        };
        state.validate_saved_state().map_err(D::Error::custom)?;
        Ok(state)
    }
}

impl BuenasPasswordState {
    fn validate_saved_state(&self) -> Result<(), String> {
        if self.generation_day >= 7 {
            return Err(format!(
                "buenas_password.generation_day {} is outside weekday range 0..6",
                self.generation_day
            ));
        }
        if !self.generated && self.option_index != 0 {
            return Err(format!(
                "buenas_password.option_index {} cannot be saved before a password is generated",
                self.option_index
            ));
        }
        if !self.generated && self.category_index != 0 {
            return Err(format!(
                "buenas_password.category_index {} cannot be saved before a password is generated",
                self.category_index
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RoamingPokemonState {
    pub species: String,
    pub level: u8,
    pub map_group: u16,
    pub map_number: u16,
    pub hp: u16,
    pub dvs: u16,
}

impl<'de> Deserialize<'de> for RoamingPokemonState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawRoamingPokemonState {
            species: String,
            level: u8,
            map_group: u16,
            map_number: u16,
            hp: u16,
            dvs: u16,
        }

        let raw = RawRoamingPokemonState::deserialize(deserializer)?;
        let state = Self {
            species: raw.species,
            level: raw.level,
            map_group: raw.map_group,
            map_number: raw.map_number,
            hp: raw.hp,
            dvs: raw.dvs,
        };
        state.validate_saved_state(0).map_err(D::Error::custom)?;
        Ok(state)
    }
}

impl RoamingPokemonState {
    fn validate_saved_state(&self, index: usize) -> Result<(), String> {
        validate_script_runtime_token(&format!("roaming_pokemon[{index}].species"), &self.species)?;
        if self.level == 0 {
            return Err(format!("roaming_pokemon[{index}].level must be nonzero"));
        }
        if self.map_group == 0 && self.map_number != 0 {
            return Err(format!(
                "roaming_pokemon[{index}].map_group must be nonzero"
            ));
        }
        if self.map_number == 0 && self.map_group != 0 {
            return Err(format!(
                "roaming_pokemon[{index}].map_number must be nonzero"
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum RoamingSaveError {
    #[error(
        "saved roaming_pokemon[{index}].species {species} is missing from compiled pack pokemon"
    )]
    MissingSpecies { index: usize, species: String },
    #[error(
        "saved roaming_pokemon[{index}] {species} level {level} is not declared by compiled roaming Pokemon definitions"
    )]
    MissingDefinition {
        index: usize,
        species: String,
        level: u8,
    },
    #[error(
        "saved roaming_pokemon[{index}] location group {map_group} map {map_number} is missing from compiled runtime map metadata"
    )]
    MissingMapLocation {
        index: usize,
        map_group: u16,
        map_number: u16,
    },
}

pub fn validate_saved_roaming_references<F, G, H>(
    roaming_pokemon: &[RoamingPokemonState],
    species_exists: F,
    roaming_definition_exists: G,
    map_location_exists: H,
) -> Result<(), RoamingSaveError>
where
    F: Fn(&str) -> bool,
    G: Fn(&str, u8) -> bool,
    H: Fn(u16, u16) -> bool,
{
    for (index, roaming) in roaming_pokemon.iter().enumerate() {
        if !species_exists(&roaming.species) {
            return Err(RoamingSaveError::MissingSpecies {
                index,
                species: roaming.species.clone(),
            });
        }
        if !roaming_definition_exists(&roaming.species, roaming.level) {
            return Err(RoamingSaveError::MissingDefinition {
                index,
                species: roaming.species.clone(),
                level: roaming.level,
            });
        }
        if roaming.map_group == 0 && roaming.map_number == 0 {
            continue;
        }
        if !map_location_exists(roaming.map_group, roaming.map_number) {
            return Err(RoamingSaveError::MissingMapLocation {
                index,
                map_group: roaming.map_group,
                map_number: roaming.map_number,
            });
        }
    }
    Ok(())
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OverworldObjectMapMemory {
    pub objects: BTreeMap<String, OverworldObjectMemory>,
    pub hidden_object_identifiers: BTreeSet<String>,
    pub following: Option<OverworldFollowMemory>,
    pub last_talked_object_identifier: Option<String>,
    pub player_hidden: bool,
}

impl<'de> Deserialize<'de> for OverworldObjectMapMemory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawOverworldObjectMapMemory {
            objects: BTreeMap<String, OverworldObjectMemory>,
            hidden_object_identifiers: BTreeSet<String>,
            following: Option<OverworldFollowMemory>,
            last_talked_object_identifier: Option<String>,
            player_hidden: bool,
        }

        let raw = RawOverworldObjectMapMemory::deserialize(deserializer)?;
        let memory = Self {
            objects: raw.objects,
            hidden_object_identifiers: raw.hidden_object_identifiers,
            following: raw.following,
            last_talked_object_identifier: raw.last_talked_object_identifier,
            player_hidden: raw.player_hidden,
        };
        memory
            .validate_saved_state("overworld_object_map")
            .map_err(D::Error::custom)?;
        Ok(memory)
    }
}

impl OverworldObjectMapMemory {
    fn validate_saved_state(&self, map_name: &str) -> Result<(), String> {
        for object_id in self.objects.keys() {
            validate_script_runtime_token(
                &format!("map_object_overrides[{map_name}].objects"),
                object_id,
            )?;
        }
        for object_id in &self.hidden_object_identifiers {
            validate_script_runtime_token(
                &format!("map_object_overrides[{map_name}].hidden_object_identifiers"),
                object_id,
            )?;
            if self.objects.contains_key(object_id) {
                return Err(format!(
                    "map_object_overrides[{map_name}] object {object_id} cannot be both overridden and hidden"
                ));
            }
        }
        validate_optional_script_runtime_token(
            &format!("map_object_overrides[{map_name}].last_talked_object_identifier"),
            self.last_talked_object_identifier.as_deref(),
        )?;
        if let Some(following) = &self.following {
            validate_script_runtime_token(
                &format!("map_object_overrides[{map_name}].following.leader_object_id"),
                &following.leader_object_id,
            )?;
            validate_script_runtime_token(
                &format!("map_object_overrides[{map_name}].following.follower_object_id"),
                &following.follower_object_id,
            )?;
            if following.leader_object_id == following.follower_object_id {
                return Err(format!(
                    "map_object_overrides[{map_name}].following leader and follower cannot both be {}",
                    following.leader_object_id
                ));
            }
            if self
                .hidden_object_identifiers
                .contains(&following.leader_object_id)
            {
                return Err(format!(
                    "map_object_overrides[{map_name}].following leader {} cannot be hidden",
                    following.leader_object_id
                ));
            }
            if self
                .hidden_object_identifiers
                .contains(&following.follower_object_id)
            {
                return Err(format!(
                    "map_object_overrides[{map_name}].following follower {} cannot be hidden",
                    following.follower_object_id
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OverworldObjectMemory {
    pub x: u16,
    pub y: u16,
    pub tile: Option<TilePosition>,
    pub facing: Option<Direction>,
}

impl<'de> Deserialize<'de> for OverworldObjectMemory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawOverworldObjectMemory {
            x: u16,
            y: u16,
            #[serde(deserialize_with = "required_nullable_tile_position")]
            tile: Option<TilePosition>,
            facing: Option<Direction>,
        }

        let raw = RawOverworldObjectMemory::deserialize(deserializer)?;
        Ok(Self {
            x: raw.x,
            y: raw.y,
            tile: raw.tile,
            facing: raw.facing,
        })
    }
}

fn required_nullable_tile_position<'de, D>(
    deserializer: D,
) -> Result<Option<TilePosition>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<TilePosition>::deserialize(deserializer)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverworldFollowMemory {
    pub leader_object_id: String,
    pub follower_object_id: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum BattleMemory {
    #[default]
    Inactive,
    Wild {
        battle_type: String,
        battle_music: String,
        map_name: String,
        enemy_pokemon: Pokemon,
        enemy_party: Vec<Pokemon>,
    },
    StaticWild {
        battle_type: String,
        battle_music: String,
        species: String,
        level: u8,
        source_script: String,
        enemy_pokemon: Pokemon,
        enemy_party: Vec<Pokemon>,
    },
    Trainer {
        battle_type: String,
        trainer_class: String,
        trainer_id: String,
        trainer_name: String,
        event_flag: String,
        seen_text: String,
        win_text: String,
        loss_text: String,
        callback: String,
        source_script: String,
        enemy_pokemon: Pokemon,
        enemy_party: Vec<Pokemon>,
        reward: u32,
        encounter_music: String,
        ai_move_flags: u32,
        ai_item_switch_flags: u32,
        ai_layers: Vec<String>,
    },
}

impl<'de> Deserialize<'de> for BattleMemory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "snake_case", deny_unknown_fields)]
        enum RawBattleMemory {
            Inactive,
            Wild {
                battle_type: String,
                battle_music: String,
                map_name: String,
                enemy_pokemon: Pokemon,
                enemy_party: Vec<Pokemon>,
            },
            StaticWild {
                battle_type: String,
                battle_music: String,
                species: String,
                level: u8,
                source_script: String,
                enemy_pokemon: Pokemon,
                enemy_party: Vec<Pokemon>,
            },
            Trainer {
                battle_type: String,
                trainer_class: String,
                trainer_id: String,
                trainer_name: String,
                event_flag: String,
                seen_text: String,
                win_text: String,
                loss_text: String,
                callback: String,
                source_script: String,
                enemy_pokemon: Pokemon,
                enemy_party: Vec<Pokemon>,
                reward: u32,
                encounter_music: String,
                ai_move_flags: u32,
                ai_item_switch_flags: u32,
                ai_layers: Vec<String>,
            },
        }

        let raw = RawBattleMemory::deserialize(deserializer)?;
        let memory = match raw {
            RawBattleMemory::Inactive => Self::Inactive,
            RawBattleMemory::Wild {
                battle_type,
                battle_music,
                map_name,
                enemy_pokemon,
                enemy_party,
            } => Self::Wild {
                battle_type,
                battle_music,
                map_name,
                enemy_pokemon,
                enemy_party,
            },
            RawBattleMemory::StaticWild {
                battle_type,
                battle_music,
                species,
                level,
                source_script,
                enemy_pokemon,
                enemy_party,
            } => Self::StaticWild {
                battle_type,
                battle_music,
                species,
                level,
                source_script,
                enemy_pokemon,
                enemy_party,
            },
            RawBattleMemory::Trainer {
                battle_type,
                trainer_class,
                trainer_id,
                trainer_name,
                event_flag,
                seen_text,
                win_text,
                loss_text,
                callback,
                source_script,
                enemy_pokemon,
                enemy_party,
                reward,
                encounter_music,
                ai_move_flags,
                ai_item_switch_flags,
                ai_layers,
            } => Self::Trainer {
                battle_type,
                trainer_class,
                trainer_id,
                trainer_name,
                event_flag,
                seen_text,
                win_text,
                loss_text,
                callback,
                source_script,
                enemy_pokemon,
                enemy_party,
                reward,
                encounter_music,
                ai_move_flags,
                ai_item_switch_flags,
                ai_layers,
            },
        };
        memory.validate_saved_state().map_err(D::Error::custom)?;
        Ok(memory)
    }
}

impl BattleMemory {
    fn validate_saved_state(&self) -> Result<(), String> {
        match self {
            Self::Inactive => Ok(()),
            Self::Wild {
                battle_type,
                battle_music,
                map_name,
                enemy_pokemon,
                enemy_party,
            } => {
                validate_script_runtime_token("battle.wild.battle_type", battle_type)?;
                validate_script_runtime_token("battle.wild.battle_music", battle_music)?;
                validate_script_runtime_token("battle.wild.map_name", map_name)?;
                validate_battle_enemy_party_state("battle.wild", enemy_pokemon, enemy_party)
            }
            Self::StaticWild {
                battle_type,
                battle_music,
                species,
                level,
                source_script,
                enemy_pokemon,
                enemy_party,
            } => {
                validate_script_runtime_token("battle.static_wild.battle_type", battle_type)?;
                validate_script_runtime_token("battle.static_wild.battle_music", battle_music)?;
                validate_script_runtime_token("battle.static_wild.species", species)?;
                if *level == 0 {
                    return Err("battle.static_wild.level must be nonzero".to_string());
                }
                if enemy_pokemon.species.id != *species {
                    return Err(format!(
                        "battle.static_wild.species {species} does not match enemy_pokemon species {}",
                        enemy_pokemon.species.id
                    ));
                }
                if enemy_pokemon.level != *level {
                    return Err(format!(
                        "battle.static_wild.level {level} does not match enemy_pokemon level {}",
                        enemy_pokemon.level
                    ));
                }
                validate_script_runtime_label("battle.static_wild.source_script", source_script)?;
                validate_battle_enemy_party_state("battle.static_wild", enemy_pokemon, enemy_party)
            }
            Self::Trainer {
                battle_type,
                trainer_class,
                trainer_id,
                event_flag,
                seen_text,
                win_text,
                loss_text,
                callback,
                source_script,
                encounter_music,
                ai_layers,
                enemy_pokemon,
                enemy_party,
                reward: _,
                ai_move_flags: _,
                ai_item_switch_flags: _,
                trainer_name: _,
            } => {
                validate_script_runtime_token("battle.trainer.battle_type", battle_type)?;
                validate_script_runtime_token("battle.trainer.trainer_class", trainer_class)?;
                validate_script_runtime_token("battle.trainer.trainer_id", trainer_id)?;
                if !event_flag.is_empty() {
                    validate_flag_name(event_flag)
                        .map_err(|error| format!("battle.trainer.event_flag {error}"))?;
                }
                validate_empty_or_script_runtime_token("battle.trainer.seen_text", seen_text)?;
                validate_empty_or_exact_text("battle.trainer.win_text", win_text)?;
                validate_empty_or_exact_text("battle.trainer.loss_text", loss_text)?;
                validate_empty_or_script_runtime_token("battle.trainer.callback", callback)?;
                validate_script_runtime_label("battle.trainer.source_script", source_script)?;
                validate_script_runtime_token("battle.trainer.encounter_music", encounter_music)?;
                for (index, ai_layer) in ai_layers.iter().enumerate() {
                    validate_script_runtime_token(
                        &format!("battle.trainer.ai_layers[{index}]"),
                        ai_layer,
                    )?;
                }
                validate_battle_enemy_party_state("battle.trainer", enemy_pokemon, enemy_party)
            }
        }
    }

    fn enemy_party_len(&self) -> Option<usize> {
        match self {
            Self::Inactive => None,
            Self::Wild { enemy_party, .. }
            | Self::StaticWild { enemy_party, .. }
            | Self::Trainer { enemy_party, .. } => Some(enemy_party.len()),
        }
    }

    fn enemy_party_and_current(&self) -> Option<(&[Pokemon], &Pokemon)> {
        match self {
            Self::Inactive => None,
            Self::Wild {
                enemy_party,
                enemy_pokemon,
                ..
            }
            | Self::StaticWild {
                enemy_party,
                enemy_pokemon,
                ..
            }
            | Self::Trainer {
                enemy_party,
                enemy_pokemon,
                ..
            } => Some((enemy_party, enemy_pokemon)),
        }
    }
}

fn validate_battle_enemy_party_state(
    path: &str,
    enemy_pokemon: &Pokemon,
    enemy_party: &[Pokemon],
) -> Result<(), String> {
    if enemy_party.is_empty() {
        return Err(format!("{path}.enemy_party must not be empty"));
    }
    enemy_pokemon
        .validate_saved_state()
        .map_err(|error| format!("{path}.enemy_pokemon: {error}"))?;
    for (index, pokemon) in enemy_party.iter().enumerate() {
        pokemon
            .validate_saved_state()
            .map_err(|error| format!("{path}.enemy_party[{index}]: {error}"))?;
    }
    Ok(())
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum OverworldMemory {
    #[default]
    Inactive,
    Active {
        map_name: String,
        tile: TilePosition,
        facing: Direction,
        mode: MovementMode,
    },
}

impl<'de> Deserialize<'de> for OverworldMemory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "snake_case", deny_unknown_fields)]
        enum RawOverworldMemory {
            Inactive,
            Active {
                map_name: String,
                tile: TilePosition,
                facing: Direction,
                mode: MovementMode,
            },
        }

        let raw = RawOverworldMemory::deserialize(deserializer)?;
        let memory = match raw {
            RawOverworldMemory::Inactive => Self::Inactive,
            RawOverworldMemory::Active {
                map_name,
                tile,
                facing,
                mode,
            } => Self::Active {
                map_name,
                tile,
                facing,
                mode,
            },
        };
        memory.validate_saved_state().map_err(D::Error::custom)?;
        Ok(memory)
    }
}

impl OverworldMemory {
    fn validate_saved_state(&self) -> Result<(), String> {
        match self {
            Self::Inactive => Ok(()),
            Self::Active { map_name, .. } => {
                validate_script_runtime_token("overworld.active.map_name", map_name)
            }
        }
    }

    pub fn from_snapshot(snapshot: &OverworldSnapshot) -> Self {
        Self::Active {
            map_name: snapshot.map_name.clone(),
            tile: snapshot.tile,
            facing: snapshot.facing,
            mode: snapshot.mode,
        }
    }

    pub fn snapshot_identity(&self) -> Option<(&str, TilePosition, Direction, MovementMode)> {
        match self {
            Self::Inactive => None,
            Self::Active {
                map_name,
                tile,
                facing,
                mode,
            } => Some((map_name, *tile, *facing, *mode)),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeMemory {
    /// Saveable combat state for the currently active battle.  Crystal keeps
    /// these effects in WRAM across turns; storing it with the runtime memory
    /// prevents a turn boundary from silently resetting screens, weather,
    /// traps, and multi-turn move counters.
    pub active_battle_combat: Option<BattleCombatState>,
    pub script_value: Option<String>,
    pub variables: BTreeMap<String, String>,
    pub variable_writes: Vec<ScriptRuntimeVariableWrite>,
    pub memory: BTreeMap<String, String>,
    pub effects: Vec<ScriptRuntimeEffect>,
    pub named_buffers: BTreeMap<String, String>,
    pub asm_directives: Vec<ScriptRuntimeAsmDirective>,
    pub numeric_buffer_writes: Vec<ScriptRuntimeNumericBufferWrite>,
    pub elevator_floors: Vec<ScriptRuntimeElevatorFloor>,
    pub stone_table_entries: Vec<ScriptRuntimeStoneTableEntry>,
    pub decoration_descriptions: Vec<ScriptRuntimeDecorationDescription>,
    pub variable_sprites: BTreeMap<String, String>,
    pub phone_numbers: BTreeSet<String>,
    pub special_phone_calls: Vec<String>,
    pub pending_delays: Vec<ScriptRuntimeDelay>,
    pub pending_earthquakes: Vec<ScriptRuntimeEarthquake>,
    pub pending_emotes: Vec<ScriptRuntimeEmote>,
    pub command_queue: Vec<ScriptRuntimeQueuedCommand>,
    pub stack: Vec<String>,
    pub last_special_routine: Option<String>,
    pub last_talked_object: Option<String>,
    pub active_menu: Option<String>,
    pub active_pokemon_picture: Option<String>,
    pub menu_coords: Option<[i16; 4]>,
    pub map_music_restart_disabled: bool,
    pub map_music_requested: bool,
    pub window_open: bool,
    pub player_input_locked: bool,
    pub all_input_locked: bool,
    pub script_stop_requested: bool,
    pub item_notify_queued: bool,
    pub warp_sound_queued: bool,
    pub teleport_from_queued: bool,
    pub hall_of_fame_requested: bool,
    pub credits_requested: bool,
    pub reset_requested: bool,
    pub menu_2d_requested: bool,
    pub version_check_requested: bool,
    pub blackout_mod: Option<String>,
    pub battle_tower_text: Option<String>,
    pub completed_trades: Vec<String>,
    pub catch_tutorials: Vec<String>,
    pub checked_mail_targets: Vec<String>,
    pub given_mail_targets: Vec<String>,
    pub audio_events: Vec<ScriptAudioRuntimeEvent>,
    pub current_music: Option<String>,
    pub pending_music_fade: Option<ScriptMusicFade>,
    pub waiting_for_sound_effect: bool,
    pub graphics_events: Vec<ScriptGraphicsRuntimeEvent>,
    pub pending_screen_fade: Option<ScriptScreenFade>,
    pub money_events: Vec<ScriptMoneyRuntimeEvent>,
    pub map_events: Vec<ScriptMapRuntimeEvent>,
    pub pending_script_warp: Option<ScriptWarpRequest>,
    pub pending_map_load: Option<ScriptMapLoadRequest>,
    pub pending_map_refresh: Option<ScriptMapRefreshRequest>,
    pub warp_check_requested: bool,
    pub text_events: Vec<ScriptTextRuntimeEvent>,
    pub text_window_open: bool,
    pub pending_text_label: Option<String>,
    pub pending_text_wait: Option<ScriptTextWait>,
    pub pending_yes_no: Option<ScriptYesNoPrompt>,
    pub control_events: Vec<ScriptControlRuntimeEvent>,
    pub next_script: Option<ScriptLocation>,
    pub call_stack: Vec<ScriptReturnFrame>,
    pub deferred_scripts: Vec<ScriptLocation>,
    pub script_ended: Option<ScriptEndState>,
    pub shop_events: Vec<ScriptShopRuntimeEvent>,
    pub pending_shop: Option<ScriptShopRequest>,
    pub item_use_events: Vec<ItemUseRuntimeEvent>,
}

impl<'de> Deserialize<'de> for ScriptRuntimeMemory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawScriptRuntimeMemory {
            #[serde(default)]
            active_battle_combat: Option<BattleCombatState>,
            script_value: Option<String>,
            variables: BTreeMap<String, String>,
            variable_writes: Vec<ScriptRuntimeVariableWrite>,
            memory: BTreeMap<String, String>,
            effects: Vec<ScriptRuntimeEffect>,
            named_buffers: BTreeMap<String, String>,
            asm_directives: Vec<ScriptRuntimeAsmDirective>,
            numeric_buffer_writes: Vec<ScriptRuntimeNumericBufferWrite>,
            elevator_floors: Vec<ScriptRuntimeElevatorFloor>,
            stone_table_entries: Vec<ScriptRuntimeStoneTableEntry>,
            decoration_descriptions: Vec<ScriptRuntimeDecorationDescription>,
            variable_sprites: BTreeMap<String, String>,
            phone_numbers: BTreeSet<String>,
            special_phone_calls: Vec<String>,
            pending_delays: Vec<ScriptRuntimeDelay>,
            pending_earthquakes: Vec<ScriptRuntimeEarthquake>,
            pending_emotes: Vec<ScriptRuntimeEmote>,
            command_queue: Vec<ScriptRuntimeQueuedCommand>,
            stack: Vec<String>,
            last_special_routine: Option<String>,
            last_talked_object: Option<String>,
            active_menu: Option<String>,
            active_pokemon_picture: Option<String>,
            menu_coords: Option<[i16; 4]>,
            map_music_restart_disabled: bool,
            map_music_requested: bool,
            window_open: bool,
            player_input_locked: bool,
            all_input_locked: bool,
            script_stop_requested: bool,
            item_notify_queued: bool,
            warp_sound_queued: bool,
            teleport_from_queued: bool,
            hall_of_fame_requested: bool,
            credits_requested: bool,
            reset_requested: bool,
            menu_2d_requested: bool,
            version_check_requested: bool,
            blackout_mod: Option<String>,
            battle_tower_text: Option<String>,
            completed_trades: Vec<String>,
            catch_tutorials: Vec<String>,
            checked_mail_targets: Vec<String>,
            given_mail_targets: Vec<String>,
            audio_events: Vec<ScriptAudioRuntimeEvent>,
            current_music: Option<String>,
            pending_music_fade: Option<ScriptMusicFade>,
            waiting_for_sound_effect: bool,
            graphics_events: Vec<ScriptGraphicsRuntimeEvent>,
            pending_screen_fade: Option<ScriptScreenFade>,
            money_events: Vec<ScriptMoneyRuntimeEvent>,
            map_events: Vec<ScriptMapRuntimeEvent>,
            pending_script_warp: Option<ScriptWarpRequest>,
            pending_map_load: Option<ScriptMapLoadRequest>,
            pending_map_refresh: Option<ScriptMapRefreshRequest>,
            warp_check_requested: bool,
            text_events: Vec<ScriptTextRuntimeEvent>,
            text_window_open: bool,
            pending_text_label: Option<String>,
            pending_text_wait: Option<ScriptTextWait>,
            pending_yes_no: Option<ScriptYesNoPrompt>,
            control_events: Vec<ScriptControlRuntimeEvent>,
            next_script: Option<ScriptLocation>,
            call_stack: Vec<ScriptReturnFrame>,
            deferred_scripts: Vec<ScriptLocation>,
            script_ended: Option<ScriptEndState>,
            shop_events: Vec<ScriptShopRuntimeEvent>,
            pending_shop: Option<ScriptShopRequest>,
            item_use_events: Vec<ItemUseRuntimeEvent>,
        }

        let raw = RawScriptRuntimeMemory::deserialize(deserializer)?;
        let memory = Self {
            active_battle_combat: raw.active_battle_combat,
            script_value: raw.script_value,
            variables: raw.variables,
            variable_writes: raw.variable_writes,
            memory: raw.memory,
            effects: raw.effects,
            named_buffers: raw.named_buffers,
            asm_directives: raw.asm_directives,
            numeric_buffer_writes: raw.numeric_buffer_writes,
            elevator_floors: raw.elevator_floors,
            stone_table_entries: raw.stone_table_entries,
            decoration_descriptions: raw.decoration_descriptions,
            variable_sprites: raw.variable_sprites,
            phone_numbers: raw.phone_numbers,
            special_phone_calls: raw.special_phone_calls,
            pending_delays: raw.pending_delays,
            pending_earthquakes: raw.pending_earthquakes,
            pending_emotes: raw.pending_emotes,
            command_queue: raw.command_queue,
            stack: raw.stack,
            last_special_routine: raw.last_special_routine,
            last_talked_object: raw.last_talked_object,
            active_menu: raw.active_menu,
            active_pokemon_picture: raw.active_pokemon_picture,
            menu_coords: raw.menu_coords,
            map_music_restart_disabled: raw.map_music_restart_disabled,
            map_music_requested: raw.map_music_requested,
            window_open: raw.window_open,
            player_input_locked: raw.player_input_locked,
            all_input_locked: raw.all_input_locked,
            script_stop_requested: raw.script_stop_requested,
            item_notify_queued: raw.item_notify_queued,
            warp_sound_queued: raw.warp_sound_queued,
            teleport_from_queued: raw.teleport_from_queued,
            hall_of_fame_requested: raw.hall_of_fame_requested,
            credits_requested: raw.credits_requested,
            reset_requested: raw.reset_requested,
            menu_2d_requested: raw.menu_2d_requested,
            version_check_requested: raw.version_check_requested,
            blackout_mod: raw.blackout_mod,
            battle_tower_text: raw.battle_tower_text,
            completed_trades: raw.completed_trades,
            catch_tutorials: raw.catch_tutorials,
            checked_mail_targets: raw.checked_mail_targets,
            given_mail_targets: raw.given_mail_targets,
            audio_events: raw.audio_events,
            current_music: raw.current_music,
            pending_music_fade: raw.pending_music_fade,
            waiting_for_sound_effect: raw.waiting_for_sound_effect,
            graphics_events: raw.graphics_events,
            pending_screen_fade: raw.pending_screen_fade,
            money_events: raw.money_events,
            map_events: raw.map_events,
            pending_script_warp: raw.pending_script_warp,
            pending_map_load: raw.pending_map_load,
            pending_map_refresh: raw.pending_map_refresh,
            warp_check_requested: raw.warp_check_requested,
            text_events: raw.text_events,
            text_window_open: raw.text_window_open,
            pending_text_label: raw.pending_text_label,
            pending_text_wait: raw.pending_text_wait,
            pending_yes_no: raw.pending_yes_no,
            control_events: raw.control_events,
            next_script: raw.next_script,
            call_stack: raw.call_stack,
            deferred_scripts: raw.deferred_scripts,
            script_ended: raw.script_ended,
            shop_events: raw.shop_events,
            pending_shop: raw.pending_shop,
            item_use_events: raw.item_use_events,
        };
        memory.validate().map_err(D::Error::custom)?;
        Ok(memory)
    }
}

impl ScriptRuntimeMemory {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(next_script) = &self.next_script {
            validate_script_runtime_token("next_script.origin_map_name", &next_script.origin_map_name)?;
            validate_script_runtime_label("next_script.script", &next_script.script)?;
        }
        validate_optional_script_runtime_label(
            "last_special_routine",
            self.last_special_routine.as_deref(),
        )?;
        validate_optional_script_runtime_label(
            "last_talked_object",
            self.last_talked_object.as_deref(),
        )?;
        validate_optional_script_runtime_label("active_menu", self.active_menu.as_deref())?;
        validate_optional_script_runtime_label(
            "active_pokemon_picture",
            self.active_pokemon_picture.as_deref(),
        )?;
        validate_optional_script_runtime_token("blackout_mod", self.blackout_mod.as_deref())?;
        validate_optional_script_runtime_token(
            "battle_tower_text",
            self.battle_tower_text.as_deref(),
        )?;
        if let Some(coords) = self.menu_coords {
            validate_menu_coords(coords)?;
        }
        for (index, script) in self.deferred_scripts.iter().enumerate() {
            validate_script_runtime_token(
                &format!("deferred_scripts[{index}].origin_map_name"),
                &script.origin_map_name,
            )?;
            validate_script_runtime_label(
                &format!("deferred_scripts[{index}].script"),
                &script.script,
            )?;
        }
        for (index, script) in self.stack.iter().enumerate() {
            validate_script_runtime_label(&format!("stack[{index}]"), script)?;
        }
        for key in self.variables.keys() {
            validate_script_runtime_token(&format!("variables[{key}]"), key)?;
        }
        for key in self.memory.keys() {
            validate_script_runtime_token(&format!("memory[{key}]"), key)?;
        }
        for key in self.named_buffers.keys() {
            validate_script_runtime_token(&format!("named_buffers[{key}]"), key)?;
        }
        for (sprite, replacement) in &self.variable_sprites {
            validate_script_runtime_token(&format!("variable_sprites[{sprite}]"), sprite)?;
            validate_script_runtime_token(
                &format!("variable_sprites[{sprite}].replacement"),
                replacement,
            )?;
        }
        for contact_id in &self.phone_numbers {
            validate_script_runtime_token(&format!("phone_numbers[{contact_id}]"), contact_id)?;
        }
        for (index, call_id) in self.special_phone_calls.iter().enumerate() {
            validate_script_runtime_token(&format!("special_phone_calls[{index}]"), call_id)?;
        }
        for (index, trade_id) in self.completed_trades.iter().enumerate() {
            validate_script_runtime_token(&format!("completed_trades[{index}]"), trade_id)?;
        }
        for (index, species_id) in self.catch_tutorials.iter().enumerate() {
            validate_script_runtime_token(&format!("catch_tutorials[{index}]"), species_id)?;
        }
        for (index, target) in self.checked_mail_targets.iter().enumerate() {
            validate_script_runtime_label(&format!("checked_mail_targets[{index}]"), target)?;
        }
        for (index, target) in self.given_mail_targets.iter().enumerate() {
            validate_script_runtime_label(&format!("given_mail_targets[{index}]"), target)?;
        }
        for (index, frame) in self.call_stack.iter().enumerate() {
            validate_script_runtime_token(
                &format!("call_stack[{index}].origin_map_name"),
                &frame.origin_map_name,
            )?;
            validate_script_runtime_label(
                &format!("call_stack[{index}].source_script"),
                &frame.source_script,
            )?;
            validate_return_frame_payload(index, frame)?;
        }
        for (index, event) in self.control_events.iter().enumerate() {
            validate_script_runtime_label(
                &format!("control_events[{index}].source_script"),
                &event.source_script,
            )?;
            validate_optional_script_runtime_label(
                &format!("control_events[{index}].target_script"),
                event.target_script.as_deref(),
            )?;
            validate_control_event_payload(index, event)?;
        }
        if let Some(end) = &self.script_ended {
            validate_script_runtime_label("script_ended.source_script", &end.source_script)?;
            validate_script_end_state(end)?;
        }
        for (index, effect) in self.effects.iter().enumerate() {
            validate_script_runtime_token(&format!("effects[{index}].command"), &effect.command)?;
            validate_script_runtime_label(
                &format!("effects[{index}].source_script"),
                &effect.source_script,
            )?;
            validate_runtime_effect_payload(index, effect)?;
        }
        for (index, write) in self.variable_writes.iter().enumerate() {
            validate_script_runtime_token(
                &format!("variable_writes[{index}].target"),
                &write.target,
            )?;
            validate_script_runtime_label(
                &format!("variable_writes[{index}].source_script"),
                &write.source_script,
            )?;
            validate_variable_write_payload(index, write, &self.variables)?;
        }
        for (index, directive) in self.asm_directives.iter().enumerate() {
            validate_script_runtime_token(
                &format!("asm_directives[{index}].command"),
                &directive.command,
            )?;
            validate_script_runtime_label(
                &format!("asm_directives[{index}].source_script"),
                &directive.source_script,
            )?;
            validate_asm_directive_payload(index, directive)?;
        }
        for (index, write) in self.numeric_buffer_writes.iter().enumerate() {
            validate_script_runtime_token(
                &format!("numeric_buffer_writes[{index}].target_buffer"),
                &write.target_buffer,
            )?;
            validate_script_runtime_label(
                &format!("numeric_buffer_writes[{index}].source_script"),
                &write.source_script,
            )?;
            validate_numeric_buffer_write_payload(index, write, &self.named_buffers)?;
        }
        for (index, floor) in self.elevator_floors.iter().enumerate() {
            validate_script_runtime_token(
                &format!("elevator_floors[{index}].floor"),
                &floor.floor,
            )?;
            validate_script_runtime_token(
                &format!("elevator_floors[{index}].target_map"),
                &floor.target_map,
            )?;
            validate_script_runtime_label(
                &format!("elevator_floors[{index}].source_script"),
                &floor.source_script,
            )?;
        }
        for (index, entry) in self.stone_table_entries.iter().enumerate() {
            validate_script_runtime_token(
                &format!("stone_table_entries[{index}].object_event"),
                &entry.object_event,
            )?;
            validate_script_runtime_label(
                &format!("stone_table_entries[{index}].script"),
                &entry.script,
            )?;
            validate_script_runtime_label(
                &format!("stone_table_entries[{index}].source_script"),
                &entry.source_script,
            )?;
        }
        for (index, description) in self.decoration_descriptions.iter().enumerate() {
            validate_script_runtime_token(
                &format!("decoration_descriptions[{index}].decoration"),
                &description.decoration,
            )?;
            validate_script_runtime_label(
                &format!("decoration_descriptions[{index}].source_script"),
                &description.source_script,
            )?;
        }
        for (index, delay) in self.pending_delays.iter().enumerate() {
            validate_script_runtime_token(
                &format!("pending_delays[{index}].command"),
                &delay.command,
            )?;
            validate_script_runtime_label(
                &format!("pending_delays[{index}].source_script"),
                &delay.source_script,
            )?;
            validate_delay_payload(index, delay)?;
        }
        for (index, earthquake) in self.pending_earthquakes.iter().enumerate() {
            validate_script_runtime_label(
                &format!("pending_earthquakes[{index}].source_script"),
                &earthquake.source_script,
            )?;
            validate_earthquake_payload(index, earthquake)?;
        }
        for (index, emote) in self.pending_emotes.iter().enumerate() {
            validate_script_runtime_token(&format!("pending_emotes[{index}].emote"), &emote.emote)?;
            validate_script_runtime_token(
                &format!("pending_emotes[{index}].object"),
                &emote.object,
            )?;
            validate_script_runtime_label(
                &format!("pending_emotes[{index}].source_script"),
                &emote.source_script,
            )?;
        }
        for (index, command) in self.command_queue.iter().enumerate() {
            validate_script_runtime_token(
                &format!("command_queue[{index}].origin_map_name"),
                &command.origin_map_name,
            )?;
            validate_script_runtime_token(
                &format!("command_queue[{index}].command"),
                &command.command,
            )?;
            validate_script_runtime_token(
                &format!("command_queue[{index}].target"),
                &command.target,
            )?;
            validate_optional_script_runtime_token(
                &format!("command_queue[{index}].bank"),
                command.bank.as_deref(),
            )?;
            validate_script_runtime_label(
                &format!("command_queue[{index}].source_script"),
                &command.source_script,
            )?;
            validate_queued_command_payload(index, command)?;
        }
        validate_optional_script_runtime_token("current_music", self.current_music.as_deref())?;
        validate_optional_script_runtime_token(
            "pending_text_label",
            self.pending_text_label.as_deref(),
        )?;
        if let Some(fade) = &self.pending_music_fade {
            validate_script_runtime_token("pending_music_fade.audio_id", &fade.audio_id)?;
            validate_script_runtime_label("pending_music_fade.source_script", &fade.source_script)?;
        }
        for (index, event) in self.audio_events.iter().enumerate() {
            validate_script_runtime_token(
                &format!("audio_events[{index}].command"),
                &event.command,
            )?;
            validate_optional_script_runtime_token(
                &format!("audio_events[{index}].audio_id"),
                event.audio_id.as_deref(),
            )?;
            validate_script_runtime_label(
                &format!("audio_events[{index}].source_script"),
                &event.source_script,
            )?;
            validate_audio_event_payload(index, event)?;
        }
        for (index, event) in self.graphics_events.iter().enumerate() {
            validate_script_runtime_token(
                &format!("graphics_events[{index}].command"),
                &event.command,
            )?;
            validate_script_runtime_label(
                &format!("graphics_events[{index}].source_script"),
                &event.source_script,
            )?;
            validate_graphics_event_payload(index, event)?;
        }
        if let Some(fade) = &self.pending_screen_fade {
            validate_script_runtime_label(
                "pending_screen_fade.source_script",
                &fade.source_script,
            )?;
            validate_pending_screen_fade_payload(fade)?;
        }
        for (index, event) in self.money_events.iter().enumerate() {
            validate_script_runtime_token(
                &format!("money_events[{index}].command"),
                &event.command,
            )?;
            validate_script_runtime_label(
                &format!("money_events[{index}].source_script"),
                &event.source_script,
            )?;
            validate_money_event_payload(index, event)?;
        }
        for (index, event) in self.map_events.iter().enumerate() {
            validate_script_runtime_token(&format!("map_events[{index}].command"), &event.command)?;
            validate_optional_script_runtime_token(
                &format!("map_events[{index}].target_map"),
                event.target_map.as_deref(),
            )?;
            validate_optional_script_runtime_token(
                &format!("map_events[{index}].map_setup"),
                event.map_setup.as_deref(),
            )?;
            validate_script_runtime_label(
                &format!("map_events[{index}].source_script"),
                &event.source_script,
            )?;
            validate_map_event_payload(index, event)?;
        }
        if let Some(warp) = &self.pending_script_warp {
            validate_script_runtime_token("pending_script_warp.target_map", &warp.target_map)?;
            validate_script_runtime_label(
                "pending_script_warp.source_script",
                &warp.source_script,
            )?;
            validate_pending_script_warp_payload(warp)?;
        }
        if let Some(load) = &self.pending_map_load {
            validate_script_runtime_token("pending_map_load.command", &load.command)?;
            validate_optional_script_runtime_token(
                "pending_map_load.map_setup",
                load.map_setup.as_deref(),
            )?;
            validate_script_runtime_label("pending_map_load.source_script", &load.source_script)?;
            validate_pending_map_load_payload(load)?;
        }
        if let Some(refresh) = &self.pending_map_refresh {
            validate_script_runtime_token("pending_map_refresh.command", &refresh.command)?;
            validate_optional_script_runtime_token(
                "pending_map_refresh.map_setup",
                refresh.map_setup.as_deref(),
            )?;
            validate_script_runtime_label(
                "pending_map_refresh.source_script",
                &refresh.source_script,
            )?;
            validate_pending_map_refresh_payload(refresh)?;
        }
        for (index, event) in self.text_events.iter().enumerate() {
            validate_script_runtime_token(
                &format!("text_events[{index}].command"),
                &event.command,
            )?;
            validate_optional_script_runtime_token(
                &format!("text_events[{index}].text_label"),
                event.text_label.as_deref(),
            )?;
            validate_script_runtime_label(
                &format!("text_events[{index}].source_script"),
                &event.source_script,
            )?;
            validate_text_event_payload(index, event)?;
        }
        if let Some(wait) = &self.pending_text_wait {
            validate_script_runtime_token("pending_text_wait.command", &wait.command)?;
            validate_script_runtime_label("pending_text_wait.source_script", &wait.source_script)?;
            validate_pending_text_wait_command(&wait.command)?;
        }
        if let Some(prompt) = &self.pending_yes_no {
            validate_script_runtime_label("pending_yes_no.source_script", &prompt.source_script)?;
        }
        self.validate_text_continuation_state()?;
        for (index, event) in self.shop_events.iter().enumerate() {
            validate_script_shop_runtime_event(&format!("shop_events[{index}]"), event)?;
        }
        if let Some(shop) = &self.pending_shop {
            validate_script_shop_request("pending_shop", shop)?;
        }
        self.validate_modal_state()?;
        for (index, event) in self.item_use_events.iter().enumerate() {
            validate_script_runtime_token(
                &format!("item_use_events[{index}].item_id"),
                &event.item_id,
            )?;
            validate_script_runtime_token(
                &format!("item_use_events[{index}].context"),
                &event.context,
            )?;
            validate_item_use_event_context(index, &event.context)?;
        }
        Ok(())
    }

    fn validate_modal_state(&self) -> Result<(), String> {
        if self.pending_shop.is_some() {
            if let Some(menu) = &self.active_menu {
                return Err(format!(
                    "pending_shop cannot be saved while active_menu {menu} is open"
                ));
            }
            if self.pending_yes_no.is_some() {
                return Err("pending_shop cannot be saved with pending_yes_no".to_string());
            }
            if self.pending_text_wait.is_some() {
                return Err("pending_shop cannot be saved with pending_text_wait".to_string());
            }
            if let Some(text_label) = &self.pending_text_label {
                return Err(format!(
                    "pending_shop cannot be saved with pending_text_label {text_label}"
                ));
            }
        }
        if let Some(menu) = &self.active_menu {
            if self.pending_yes_no.is_some() {
                return Err(format!(
                    "active_menu {menu} cannot be saved with pending_yes_no"
                ));
            }
            if self.pending_text_wait.is_some() {
                return Err(format!(
                    "active_menu {menu} cannot be saved with pending_text_wait"
                ));
            }
        }
        if let Some(text_label) = &self.pending_text_label
            && self.pending_yes_no.is_some()
        {
            return Err(format!(
                "pending_text_label {text_label} cannot be saved with pending_yes_no"
            ));
        }
        Ok(())
    }

    fn validate_text_continuation_state(&self) -> Result<(), String> {
        if !self.text_window_open {
            if let Some(text_label) = &self.pending_text_label {
                return Err(format!(
                    "pending_text_label {text_label} cannot be saved without an open text window"
                ));
            }
            if self.pending_text_wait.is_some() {
                return Err(
                    "pending_text_wait cannot be saved without an open text window".to_string(),
                );
            }
            if self.pending_yes_no.is_some() {
                return Err(
                    "pending_yes_no cannot be saved without an open text window".to_string()
                );
            }
        }
        if self.pending_text_wait.is_some() && self.pending_yes_no.is_some() {
            return Err("pending_text_wait and pending_yes_no cannot both be saved".to_string());
        }
        Ok(())
    }
}

fn validate_script_shop_runtime_event(
    field: &str,
    event: &ScriptShopRuntimeEvent,
) -> Result<(), String> {
    validate_script_shop_mart_type(field, &event.mart_type, &event.mart_id)?;
    validate_script_runtime_token(&format!("{field}.mart_id"), &event.mart_id)?;
    for (index, item_id) in event.inventory.iter().enumerate() {
        validate_script_runtime_token(&format!("{field}.inventory[{index}]"), item_id)?;
    }
    validate_script_runtime_label(&format!("{field}.source_script"), &event.source_script)
}

fn validate_script_shop_request(field: &str, request: &ScriptShopRequest) -> Result<(), String> {
    validate_script_shop_mart_type(field, &request.mart_type, &request.mart_id)?;
    validate_script_runtime_token(&format!("{field}.mart_id"), &request.mart_id)?;
    for (index, item_id) in request.inventory.iter().enumerate() {
        validate_script_runtime_token(&format!("{field}.inventory[{index}]"), item_id)?;
    }
    validate_script_runtime_label(&format!("{field}.source_script"), &request.source_script)
}

pub fn saved_shop_event_command_payload(
    event: &ScriptShopRuntimeEvent,
) -> (&'static str, Vec<String>) {
    (
        "pokemart",
        vec![event.mart_type.clone(), event.mart_id.clone()],
    )
}

pub fn saved_shop_request_command_payload(
    request: &ScriptShopRequest,
) -> (&'static str, Vec<String>) {
    (
        "pokemart",
        vec![request.mart_type.clone(), request.mart_id.clone()],
    )
}

fn validate_script_shop_mart_type(
    field: &str,
    mart_type: &str,
    mart_id: &str,
) -> Result<(), String> {
    validate_script_runtime_token(&format!("{field}.mart_type"), mart_type)?;
    if !is_known_script_mart_type(mart_type) {
        return Err(format!(
            "{field}.mart_type {mart_type} is not a saved mart type"
        ));
    }
    if mart_id == "0" && !SCRIPT_SHOP_ZERO_MART_TYPES.contains(&mart_type) {
        return Err(format!(
            "{field}.mart_id 0 requires a zero-inventory mart type, got {mart_type}"
        ));
    }
    Ok(())
}

fn validate_menu_coords(coords: [i16; 4]) -> Result<(), String> {
    let [left, top, right, bottom] = coords;
    if left < 0 || top < 0 || right < 0 || bottom < 0 {
        return Err(format!(
            "menu_coords {:?} cannot contain negative coordinates",
            coords
        ));
    }
    if right < left {
        return Err(format!(
            "menu_coords right {right} cannot be less than left {left}"
        ));
    }
    if bottom < top {
        return Err(format!(
            "menu_coords bottom {bottom} cannot be less than top {top}"
        ));
    }
    Ok(())
}

fn validate_item_use_event_context(index: usize, context: &str) -> Result<(), String> {
    if matches!(context, "field" | "battle") {
        Ok(())
    } else {
        Err(format!(
            "item_use_events[{index}].context {context} is not a saved item-use context"
        ))
    }
}

fn validate_asm_directive_payload(
    index: usize,
    directive: &ScriptRuntimeAsmDirective,
) -> Result<(), String> {
    if !matches!(
        directive.command.as_str(),
        "dw" | "ldh" | "ld" | "dn" | "dba" | "dbw"
    ) {
        return Err(format!(
            "asm_directives[{index}].command {} is not a saved asm directive",
            directive.command
        ));
    }
    let expected = script_runtime_command_arg_counts()
        .get(directive.command.as_str())
        .copied()
        .ok_or_else(|| {
            format!(
                "asm_directives[{index}].command {} is missing runtime arity",
                directive.command
            )
        })?;
    if directive.args.len() != expected {
        return Err(format!(
            "asm_directives[{index}].args has {} entries, expected {expected} for {}",
            directive.args.len(),
            directive.command
        ));
    }
    for (arg_index, arg) in directive.args.iter().enumerate() {
        if arg.is_empty() || arg.trim() != arg {
            return Err(format!(
                "asm_directives[{index}].args[{arg_index}] has invalid arg '{arg}'"
            ));
        }
    }
    Ok(())
}

fn validate_numeric_buffer_write_payload(
    index: usize,
    write: &ScriptRuntimeNumericBufferWrite,
    named_buffers: &BTreeMap<String, String>,
) -> Result<(), String> {
    let parsed = write.value.parse::<u16>().map_err(|_| {
        format!(
            "numeric_buffer_writes[{index}].value {} is not a saved u16 value",
            write.value
        )
    })?;
    if write.value != parsed.to_string() {
        return Err(format!(
            "numeric_buffer_writes[{index}].value {} is not canonical",
            write.value
        ));
    }
    if write.width != 3 {
        return Err(format!(
            "numeric_buffer_writes[{index}].width {} must be 3",
            write.width
        ));
    }
    match named_buffers.get(&write.target_buffer) {
        Some(value) if value == &write.value => Ok(()),
        Some(value) => Err(format!(
            "numeric_buffer_writes[{index}].value {} does not match named_buffers[{}] {}",
            write.value, write.target_buffer, value
        )),
        None => Err(format!(
            "numeric_buffer_writes[{index}].target_buffer {} is missing from named_buffers",
            write.target_buffer
        )),
    }
}

pub fn saved_variable_write_command_payload(
    write: &ScriptRuntimeVariableWrite,
) -> (&'static str, Vec<String>) {
    ("writevar", vec![write.target.clone()])
}

pub fn saved_numeric_buffer_write_command_payload(
    write: &ScriptRuntimeNumericBufferWrite,
) -> (&'static str, Vec<String>) {
    ("getnum", vec![write.target_buffer.clone()])
}

pub fn saved_elevator_floor_command_payload(
    floor: &ScriptRuntimeElevatorFloor,
) -> (&'static str, Vec<String>) {
    (
        "elevfloor",
        vec![
            floor.floor.clone(),
            floor.warp.to_string(),
            floor.target_map.clone(),
        ],
    )
}

pub fn saved_stone_table_entry_command_payload(
    entry: &ScriptRuntimeStoneTableEntry,
) -> (&'static str, Vec<String>) {
    (
        "stonetable",
        vec![
            entry.warp.to_string(),
            entry.object_event.clone(),
            entry.script.clone(),
        ],
    )
}

pub fn saved_decoration_description_command_payload(
    description: &ScriptRuntimeDecorationDescription,
) -> (&'static str, Vec<String>) {
    ("describedecoration", vec![description.decoration.clone()])
}

fn validate_delay_payload(index: usize, delay: &ScriptRuntimeDelay) -> Result<(), String> {
    if matches!(delay.command.as_str(), "pause" | "wait") {
        Ok(())
    } else {
        Err(format!(
            "pending_delays[{index}].command {} is not a saved delay command",
            delay.command
        ))
    }
}

fn validate_earthquake_payload(
    index: usize,
    earthquake: &ScriptRuntimeEarthquake,
) -> Result<(), String> {
    if earthquake.shake_frames != earthquake.parameter {
        return Err(format!(
            "pending_earthquakes[{index}].shake_frames {} must equal parameter {}",
            earthquake.shake_frames, earthquake.parameter
        ));
    }
    let expected_sleep_frames = earthquake.parameter & 0x3f;
    if earthquake.sleep_frames != expected_sleep_frames {
        return Err(format!(
            "pending_earthquakes[{index}].sleep_frames {} must equal parameter & 0x3f ({expected_sleep_frames})",
            earthquake.sleep_frames
        ));
    }
    Ok(())
}

pub fn saved_delay_command_payload(delay: &ScriptRuntimeDelay) -> (&str, Vec<String>) {
    (&delay.command, vec![delay.frames.to_string()])
}

pub fn saved_music_fade_command_payload(fade: &ScriptMusicFade) -> (&'static str, Vec<String>) {
    (
        "musicfadeout",
        vec![fade.audio_id.clone(), fade.fade_frames.to_string()],
    )
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ScriptAudioRuntimeCommandError {
    #[error(
        "saved {path} {source_script}:{command_index} command {command} has unexpected fade_frames"
    )]
    UnexpectedFadeFrames {
        path: String,
        source_script: String,
        command_index: usize,
        command: String,
    },
    #[error("saved {path} {source_script}:{command_index} is missing audio_id")]
    MissingAudioId {
        path: String,
        source_script: String,
        command_index: usize,
    },
    #[error("saved {path} {source_script}:{command_index} is missing fade_frames")]
    MissingFadeFrames {
        path: String,
        source_script: String,
        command_index: usize,
    },
    #[error("saved {path} {source_script}:{command_index} command waitsfx has unexpected audio_id")]
    UnexpectedWaitSfxAudioId {
        path: String,
        source_script: String,
        command_index: usize,
    },
    #[error(
        "saved {path} {source_script}:{command_index} command waitsfx has unexpected fade_frames"
    )]
    UnexpectedWaitSfxFadeFrames {
        path: String,
        source_script: String,
        command_index: usize,
    },
    #[error(
        "saved {path} {source_script}:{command_index} special audio event must use command_index 0"
    )]
    UnexpectedSpecialCommandIndex {
        path: String,
        source_script: String,
        command_index: usize,
    },
    #[error(
        "saved {path} {source_script} special audio event is {actual:?}, expected {expected:?}"
    )]
    SpecialKindMismatch {
        path: String,
        source_script: String,
        actual: ScriptAudioRuntimeKind,
        expected: ScriptAudioRuntimeKind,
    },
    #[error("saved {path} {source_script} special cry event is missing audio_id")]
    MissingSpecialCryAudioId { path: String, source_script: String },
    #[error("saved {path} {source_script} special audio event has unexpected audio_id {actual}")]
    UnexpectedSpecialAudioId {
        path: String,
        source_script: String,
        actual: String,
    },
    #[error("saved {path} {source_script} special audio event is missing audio_id {expected}")]
    MissingSpecialAudioId {
        path: String,
        source_script: String,
        expected: &'static str,
    },
    #[error(
        "saved {path} {source_script} special audio event audio_id {actual} does not match {expected}"
    )]
    SpecialAudioIdMismatch {
        path: String,
        source_script: String,
        actual: String,
        expected: &'static str,
    },
    #[error("saved {path} {source_script} special audio event has unexpected fade_frames {actual}")]
    UnexpectedSpecialFadeFrames {
        path: String,
        source_script: String,
        actual: u16,
    },
    #[error("saved {path} {source_script} special audio event is missing fade_frames {expected}")]
    MissingSpecialFadeFrames {
        path: String,
        source_script: String,
        expected: u16,
    },
    #[error(
        "saved {path} {source_script} special audio event fade_frames {actual} does not match {expected}"
    )]
    SpecialFadeFramesMismatch {
        path: String,
        source_script: String,
        actual: u16,
        expected: u16,
    },
    #[error("saved {path} special routine {routine} does not emit audio")]
    SpecialRoutineDoesNotEmitAudio { path: String, routine: String },
}

pub fn saved_audio_runtime_event_command_args(
    path: &str,
    event: &ScriptAudioRuntimeEvent,
) -> Result<Option<Vec<String>>, ScriptAudioRuntimeCommandError> {
    match event.command.as_str() {
        "special" => {
            validate_special_audio_runtime_event_shape(path, event)?;
            Ok(None)
        }
        "playmusic" | "playsound" => {
            if event.fade_frames.is_some() {
                return Err(audio_command_error(
                    path,
                    event,
                    ScriptAudioRuntimeCommandErrorKind::UnexpectedFadeFrames,
                ));
            }
            let audio_id = event.audio_id.clone().ok_or_else(|| {
                audio_command_error(
                    path,
                    event,
                    ScriptAudioRuntimeCommandErrorKind::MissingAudioId,
                )
            })?;
            Ok(Some(vec![audio_id]))
        }
        "musicfadeout" => {
            let audio_id = event.audio_id.clone().ok_or_else(|| {
                audio_command_error(
                    path,
                    event,
                    ScriptAudioRuntimeCommandErrorKind::MissingAudioId,
                )
            })?;
            let fade_frames = event.fade_frames.ok_or_else(|| {
                audio_command_error(
                    path,
                    event,
                    ScriptAudioRuntimeCommandErrorKind::MissingFadeFrames,
                )
            })?;
            Ok(Some(vec![audio_id, fade_frames.to_string()]))
        }
        "waitsfx" => {
            if event.audio_id.is_some() {
                return Err(audio_command_error(
                    path,
                    event,
                    ScriptAudioRuntimeCommandErrorKind::UnexpectedWaitSfxAudioId,
                ));
            }
            if event.fade_frames.is_some() {
                return Err(audio_command_error(
                    path,
                    event,
                    ScriptAudioRuntimeCommandErrorKind::UnexpectedWaitSfxFadeFrames,
                ));
            }
            Ok(Some(Vec::new()))
        }
        "cry" => Ok(None),
        _ => Ok(None),
    }
}

enum ScriptAudioRuntimeCommandErrorKind {
    UnexpectedFadeFrames,
    MissingAudioId,
    MissingFadeFrames,
    UnexpectedWaitSfxAudioId,
    UnexpectedWaitSfxFadeFrames,
}

fn audio_command_error(
    path: &str,
    event: &ScriptAudioRuntimeEvent,
    kind: ScriptAudioRuntimeCommandErrorKind,
) -> ScriptAudioRuntimeCommandError {
    match kind {
        ScriptAudioRuntimeCommandErrorKind::UnexpectedFadeFrames => {
            ScriptAudioRuntimeCommandError::UnexpectedFadeFrames {
                path: path.to_string(),
                source_script: event.source_script.clone(),
                command_index: event.command_index,
                command: event.command.clone(),
            }
        }
        ScriptAudioRuntimeCommandErrorKind::MissingAudioId => {
            ScriptAudioRuntimeCommandError::MissingAudioId {
                path: path.to_string(),
                source_script: event.source_script.clone(),
                command_index: event.command_index,
            }
        }
        ScriptAudioRuntimeCommandErrorKind::MissingFadeFrames => {
            ScriptAudioRuntimeCommandError::MissingFadeFrames {
                path: path.to_string(),
                source_script: event.source_script.clone(),
                command_index: event.command_index,
            }
        }
        ScriptAudioRuntimeCommandErrorKind::UnexpectedWaitSfxAudioId => {
            ScriptAudioRuntimeCommandError::UnexpectedWaitSfxAudioId {
                path: path.to_string(),
                source_script: event.source_script.clone(),
                command_index: event.command_index,
            }
        }
        ScriptAudioRuntimeCommandErrorKind::UnexpectedWaitSfxFadeFrames => {
            ScriptAudioRuntimeCommandError::UnexpectedWaitSfxFadeFrames {
                path: path.to_string(),
                source_script: event.source_script.clone(),
                command_index: event.command_index,
            }
        }
    }
}

fn validate_special_audio_runtime_event_shape(
    path: &str,
    event: &ScriptAudioRuntimeEvent,
) -> Result<(), ScriptAudioRuntimeCommandError> {
    if event.command_index != 0 {
        return Err(
            ScriptAudioRuntimeCommandError::UnexpectedSpecialCommandIndex {
                path: path.to_string(),
                source_script: event.source_script.clone(),
                command_index: event.command_index,
            },
        );
    }
    match event.source_script.as_str() {
        "FadeOutMusic" => {
            validate_special_audio_event_kind(path, event, ScriptAudioRuntimeKind::FadeMusic)?;
            validate_special_audio_event_id(path, event, Some("MUSIC_NONE"))?;
            validate_special_audio_event_fade(path, event, Some(2))
        }
        "WaitSFX" => {
            validate_special_audio_event_kind(
                path,
                event,
                ScriptAudioRuntimeKind::WaitForSoundEffect,
            )?;
            validate_special_audio_event_id(path, event, None)?;
            validate_special_audio_event_fade(path, event, None)
        }
        "PlayCurMonCry" | "PlaySlowCry" => {
            validate_special_audio_event_kind(path, event, ScriptAudioRuntimeKind::Cry)?;
            if event.audio_id.is_none() {
                return Err(ScriptAudioRuntimeCommandError::MissingSpecialCryAudioId {
                    path: path.to_string(),
                    source_script: event.source_script.clone(),
                });
            }
            validate_special_audio_event_fade(path, event, None)
        }
        "GetMysteryGiftItem" => {
            validate_special_audio_event_kind(path, event, ScriptAudioRuntimeKind::SoundEffect)?;
            validate_special_audio_event_id(path, event, Some("SFX_ITEM"))?;
            validate_special_audio_event_fade(path, event, None)
        }
        routine => Err(
            ScriptAudioRuntimeCommandError::SpecialRoutineDoesNotEmitAudio {
                path: path.to_string(),
                routine: routine.to_string(),
            },
        ),
    }
}

fn validate_special_audio_event_kind(
    path: &str,
    event: &ScriptAudioRuntimeEvent,
    expected: ScriptAudioRuntimeKind,
) -> Result<(), ScriptAudioRuntimeCommandError> {
    if event.kind != expected {
        return Err(ScriptAudioRuntimeCommandError::SpecialKindMismatch {
            path: path.to_string(),
            source_script: event.source_script.clone(),
            actual: event.kind,
            expected,
        });
    }
    Ok(())
}

fn validate_special_audio_event_id(
    path: &str,
    event: &ScriptAudioRuntimeEvent,
    expected: Option<&'static str>,
) -> Result<(), ScriptAudioRuntimeCommandError> {
    match (event.audio_id.as_deref(), expected) {
        (actual, expected) if actual == expected => Ok(()),
        (Some(actual), None) => Err(ScriptAudioRuntimeCommandError::UnexpectedSpecialAudioId {
            path: path.to_string(),
            source_script: event.source_script.clone(),
            actual: actual.to_string(),
        }),
        (None, Some(expected)) => Err(ScriptAudioRuntimeCommandError::MissingSpecialAudioId {
            path: path.to_string(),
            source_script: event.source_script.clone(),
            expected,
        }),
        (Some(actual), Some(expected)) => {
            Err(ScriptAudioRuntimeCommandError::SpecialAudioIdMismatch {
                path: path.to_string(),
                source_script: event.source_script.clone(),
                actual: actual.to_string(),
                expected,
            })
        }
        (None, None) => Ok(()),
    }
}

fn validate_special_audio_event_fade(
    path: &str,
    event: &ScriptAudioRuntimeEvent,
    expected: Option<u16>,
) -> Result<(), ScriptAudioRuntimeCommandError> {
    match (event.fade_frames, expected) {
        (actual, expected) if actual == expected => Ok(()),
        (Some(actual), None) => Err(
            ScriptAudioRuntimeCommandError::UnexpectedSpecialFadeFrames {
                path: path.to_string(),
                source_script: event.source_script.clone(),
                actual,
            },
        ),
        (None, Some(expected)) => Err(ScriptAudioRuntimeCommandError::MissingSpecialFadeFrames {
            path: path.to_string(),
            source_script: event.source_script.clone(),
            expected,
        }),
        (Some(actual), Some(expected)) => {
            Err(ScriptAudioRuntimeCommandError::SpecialFadeFramesMismatch {
                path: path.to_string(),
                source_script: event.source_script.clone(),
                actual,
                expected,
            })
        }
        (None, None) => Ok(()),
    }
}

pub fn saved_earthquake_command_payload(
    earthquake: &ScriptRuntimeEarthquake,
) -> (&'static str, Vec<String>) {
    ("earthquake", vec![earthquake.parameter.to_string()])
}

pub fn saved_emote_command_payload(emote: &ScriptRuntimeEmote) -> (&'static str, Vec<String>) {
    (
        "showemote",
        vec![
            emote.emote.clone(),
            emote.object.clone(),
            emote.duration.to_string(),
        ],
    )
}

fn validate_audio_event_payload(
    index: usize,
    event: &ScriptAudioRuntimeEvent,
) -> Result<(), String> {
    match event.kind {
        ScriptAudioRuntimeKind::Music => {
            if event.command != "special"
                && !SCRIPT_AUDIO_MUSIC_COMMANDS.contains(&event.command.as_str())
            {
                return Err(format!(
                    "audio_events[{index}].command {} is not valid for Music",
                    event.command
                ));
            }
            if event.audio_id.is_none() {
                return Err(format!(
                    "audio_events[{index}].audio_id is required for {:?}",
                    event.kind
                ));
            }
            if event.fade_frames.is_some() {
                return Err(format!(
                    "audio_events[{index}].fade_frames is only valid for FadeMusic"
                ));
            }
        }
        ScriptAudioRuntimeKind::SoundEffect => {
            if event.command != "special"
                && !(event.command == "standard_receive_item"
                    && event.source_script == "ReceiveItemScript"
                    && event.audio_id.as_deref() == Some("SFX_ITEM")
                    && event.fade_frames.is_none())
                && !SCRIPT_AUDIO_SOUND_EFFECT_COMMANDS.contains(&event.command.as_str())
            {
                return Err(format!(
                    "audio_events[{index}].command {} is not valid for SoundEffect",
                    event.command
                ));
            }
            if event.audio_id.is_none() {
                return Err(format!(
                    "audio_events[{index}].audio_id is required for {:?}",
                    event.kind
                ));
            }
            if event.fade_frames.is_some() {
                return Err(format!(
                    "audio_events[{index}].fade_frames is only valid for FadeMusic"
                ));
            }
        }
        ScriptAudioRuntimeKind::Cry => {
            if event.command != "special"
                && !SCRIPT_AUDIO_CRY_COMMANDS.contains(&event.command.as_str())
            {
                return Err(format!(
                    "audio_events[{index}].command {} is not valid for Cry",
                    event.command
                ));
            }
            if event.audio_id.is_none() {
                return Err(format!(
                    "audio_events[{index}].audio_id is required for {:?}",
                    event.kind
                ));
            }
            if event.fade_frames.is_some() {
                return Err(format!(
                    "audio_events[{index}].fade_frames is only valid for FadeMusic"
                ));
            }
        }
        ScriptAudioRuntimeKind::FadeMusic => {
            if event.command != "special"
                && !SCRIPT_AUDIO_MUSIC_FADE_COMMANDS.contains(&event.command.as_str())
            {
                return Err(format!(
                    "audio_events[{index}].command {} is not valid for FadeMusic",
                    event.command
                ));
            }
            if event.audio_id.is_none() {
                return Err(format!(
                    "audio_events[{index}].audio_id is required for FadeMusic"
                ));
            }
            if event.fade_frames.is_none() {
                return Err(format!(
                    "audio_events[{index}].fade_frames is required for FadeMusic"
                ));
            }
        }
        ScriptAudioRuntimeKind::WaitForSoundEffect => {
            if event.command != "special"
                && !SCRIPT_AUDIO_NO_PAYLOAD_COMMANDS.contains(&event.command.as_str())
            {
                return Err(format!(
                    "audio_events[{index}].command {} is not valid for WaitForSoundEffect",
                    event.command
                ));
            }
            if event.audio_id.is_some() {
                return Err(format!(
                    "audio_events[{index}].audio_id is not valid for WaitForSoundEffect"
                ));
            }
            if event.fade_frames.is_some() {
                return Err(format!(
                    "audio_events[{index}].fade_frames is not valid for WaitForSoundEffect"
                ));
            }
        }
    }
    Ok(())
}

fn validate_pending_screen_fade_payload(fade: &ScriptScreenFade) -> Result<(), String> {
    if fade.command_index != 0 {
        return Err(format!(
            "pending_screen_fade.command_index {} must be 0",
            fade.command_index
        ));
    }
    if fade.frames != 8 {
        return Err(format!(
            "pending_screen_fade.frames {} must be 8",
            fade.frames
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum ScriptScreenFadeRoutineError {
    #[error("saved {path} {routine} is not a screen fade routine")]
    UnknownRoutine { path: String, routine: String },
    #[error("saved {path} {routine} color {actual:?} does not match {expected:?}")]
    ColorMismatch {
        path: String,
        routine: String,
        actual: ScriptFadeColor,
        expected: ScriptFadeColor,
    },
    #[error("saved {path} {routine} direction {actual:?} does not match {expected:?}")]
    DirectionMismatch {
        path: String,
        routine: String,
        actual: ScriptFadeDirection,
        expected: ScriptFadeDirection,
    },
    #[error("saved {path} {routine} frames {actual} does not match 8")]
    FrameMismatch {
        path: String,
        routine: String,
        actual: u16,
    },
}

pub fn validate_saved_screen_fade_routine(
    path: &str,
    routine: &str,
    color: ScriptFadeColor,
    direction: ScriptFadeDirection,
    frames: u16,
) -> Result<(), ScriptScreenFadeRoutineError> {
    let Some((expected_color, expected_direction)) = screen_fade_routine_fields(routine) else {
        return Err(ScriptScreenFadeRoutineError::UnknownRoutine {
            path: path.to_string(),
            routine: routine.to_string(),
        });
    };
    if color != expected_color {
        return Err(ScriptScreenFadeRoutineError::ColorMismatch {
            path: path.to_string(),
            routine: routine.to_string(),
            actual: color,
            expected: expected_color,
        });
    }
    if direction != expected_direction {
        return Err(ScriptScreenFadeRoutineError::DirectionMismatch {
            path: path.to_string(),
            routine: routine.to_string(),
            actual: direction,
            expected: expected_direction,
        });
    }
    if frames != 8 {
        return Err(ScriptScreenFadeRoutineError::FrameMismatch {
            path: path.to_string(),
            routine: routine.to_string(),
            actual: frames,
        });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum ScriptScreenFadeSaveError {
    #[error("saved {path} {source_script}:{command_index} screen fade must use command index 0")]
    CommandIndexMismatch {
        path: String,
        source_script: String,
        command_index: usize,
    },
    #[error(transparent)]
    Routine(#[from] ScriptScreenFadeRoutineError),
}

pub fn validate_saved_pending_screen_fade_shape(
    path: &str,
    fade: &ScriptScreenFade,
) -> Result<(), ScriptScreenFadeSaveError> {
    if fade.command_index != 0 {
        return Err(ScriptScreenFadeSaveError::CommandIndexMismatch {
            path: path.to_string(),
            source_script: fade.source_script.clone(),
            command_index: fade.command_index,
        });
    }
    validate_saved_screen_fade_routine(
        path,
        &fade.source_script,
        fade.color,
        fade.direction,
        fade.frames,
    )?;
    Ok(())
}

fn screen_fade_routine_fields(routine: &str) -> Option<(ScriptFadeColor, ScriptFadeDirection)> {
    match routine {
        "FadeOutToWhite" => Some((ScriptFadeColor::White, ScriptFadeDirection::Out)),
        "FadeInFromWhite" => Some((ScriptFadeColor::White, ScriptFadeDirection::In)),
        "FadeOutToBlack" => Some((ScriptFadeColor::Black, ScriptFadeDirection::Out)),
        "FadeInFromBlack" => Some((ScriptFadeColor::Black, ScriptFadeDirection::In)),
        _ => None,
    }
}

fn validate_graphics_event_payload(
    index: usize,
    event: &ScriptGraphicsRuntimeEvent,
) -> Result<(), String> {
    if event.command != "special" {
        return Err(format!(
            "graphics_events[{index}].command {} must be special",
            event.command
        ));
    }
    if event.command_index != 0 {
        return Err(format!(
            "graphics_events[{index}].command_index {} must be 0",
            event.command_index
        ));
    }
    let has_fade_payload =
        event.color.is_some() || event.direction.is_some() || event.frames.is_some();
    if event.kind == ScriptGraphicsRuntimeKind::ScreenFade {
        if event.color.is_none() {
            return Err(format!(
                "graphics_events[{index}].color is required for ScreenFade"
            ));
        }
        if event.direction.is_none() {
            return Err(format!(
                "graphics_events[{index}].direction is required for ScreenFade"
            ));
        }
        if event.frames.is_none() {
            return Err(format!(
                "graphics_events[{index}].frames is required for ScreenFade"
            ));
        }
        if event.frames != Some(8) {
            return Err(format!(
                "graphics_events[{index}].frames {} must be 8 for ScreenFade",
                event
                    .frames
                    .expect("screen fade frames presence is validated before exact value check")
            ));
        }
    } else if has_fade_payload {
        return Err(format!(
            "graphics_events[{index}] fade payload is only valid for ScreenFade"
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum ScriptGraphicsRuntimeEventError {
    #[error(
        "saved {path} {source_script}:{command_index} graphics command {command} does not match special"
    )]
    CommandMismatch {
        path: String,
        source_script: String,
        command_index: usize,
        command: String,
    },
    #[error(
        "saved {path} {source_script}:{command_index} graphics special must use command index 0"
    )]
    CommandIndexMismatch {
        path: String,
        source_script: String,
        command_index: usize,
    },
    #[error("saved {path} {source_script}:{command_index} screen fade is missing color")]
    MissingScreenFadeColor {
        path: String,
        source_script: String,
        command_index: usize,
    },
    #[error("saved {path} {source_script}:{command_index} screen fade is missing direction")]
    MissingScreenFadeDirection {
        path: String,
        source_script: String,
        command_index: usize,
    },
    #[error("saved {path} {source_script}:{command_index} screen fade is missing frames")]
    MissingScreenFadeFrames {
        path: String,
        source_script: String,
        command_index: usize,
    },
    #[error(
        "saved {path} {source_script}:{command_index} graphics event has unexpected fade payload"
    )]
    UnexpectedFadePayload {
        path: String,
        source_script: String,
        command_index: usize,
    },
    #[error(transparent)]
    ScreenFadeRoutine(#[from] ScriptScreenFadeRoutineError),
}

pub fn validate_saved_graphics_runtime_event_shape(
    path: &str,
    event: &ScriptGraphicsRuntimeEvent,
) -> Result<(), ScriptGraphicsRuntimeEventError> {
    if event.command != "special" {
        return Err(ScriptGraphicsRuntimeEventError::CommandMismatch {
            path: path.to_string(),
            source_script: event.source_script.clone(),
            command_index: event.command_index,
            command: event.command.clone(),
        });
    }
    if event.command_index != 0 {
        return Err(ScriptGraphicsRuntimeEventError::CommandIndexMismatch {
            path: path.to_string(),
            source_script: event.source_script.clone(),
            command_index: event.command_index,
        });
    }
    if event.kind == ScriptGraphicsRuntimeKind::ScreenFade {
        let color =
            event
                .color
                .ok_or_else(|| ScriptGraphicsRuntimeEventError::MissingScreenFadeColor {
                    path: path.to_string(),
                    source_script: event.source_script.clone(),
                    command_index: event.command_index,
                })?;
        let direction = event.direction.ok_or_else(|| {
            ScriptGraphicsRuntimeEventError::MissingScreenFadeDirection {
                path: path.to_string(),
                source_script: event.source_script.clone(),
                command_index: event.command_index,
            }
        })?;
        let frames = event.frames.ok_or_else(|| {
            ScriptGraphicsRuntimeEventError::MissingScreenFadeFrames {
                path: path.to_string(),
                source_script: event.source_script.clone(),
                command_index: event.command_index,
            }
        })?;
        validate_saved_screen_fade_routine(path, &event.source_script, color, direction, frames)?;
        return Ok(());
    }
    if event.color.is_some() || event.direction.is_some() || event.frames.is_some() {
        return Err(ScriptGraphicsRuntimeEventError::UnexpectedFadePayload {
            path: path.to_string(),
            source_script: event.source_script.clone(),
            command_index: event.command_index,
        });
    }
    Ok(())
}

fn validate_money_event_payload(
    index: usize,
    event: &ScriptMoneyRuntimeEvent,
) -> Result<(), String> {
    if event.command != "special" {
        return Err(format!(
            "money_events[{index}].command {} must be special",
            event.command
        ));
    }
    match event.kind {
        ScriptMoneyRuntimeKind::PlaceMoneyTopRight => {
            if event.coins.is_some() {
                return Err(format!(
                    "money_events[{index}].coins is not valid for PlaceMoneyTopRight"
                ));
            }
        }
        ScriptMoneyRuntimeKind::DisplayMoneyAndCoinBalance => {
            if event.coins.is_none() {
                return Err(format!(
                    "money_events[{index}].coins is required for DisplayMoneyAndCoinBalance"
                ));
            }
        }
        ScriptMoneyRuntimeKind::DisplayCoinCaseBalance => {
            if event.money != 0 {
                return Err(format!(
                    "money_events[{index}].money must be 0 for DisplayCoinCaseBalance"
                ));
            }
            if event.coins.is_none() {
                return Err(format!(
                    "money_events[{index}].coins is required for DisplayCoinCaseBalance"
                ));
            }
        }
    }
    if event.command_index != 0 {
        return Err(format!(
            "money_events[{index}].command_index {} must be 0",
            event.command_index
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum ScriptMoneyRuntimeRoutineError {
    #[error("saved {path} {routine} is not a money display routine")]
    UnknownRoutine { path: String, routine: String },
    #[error("saved {path} {routine} kind {actual:?} does not match {expected:?}")]
    KindMismatch {
        path: String,
        routine: String,
        actual: ScriptMoneyRuntimeKind,
        expected: ScriptMoneyRuntimeKind,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum ScriptMoneyRuntimeEventError {
    #[error(
        "saved {path} {source_script}:{command_index} money command {command} does not match special"
    )]
    CommandMismatch {
        path: String,
        source_script: String,
        command_index: usize,
        command: String,
    },
    #[error("saved {path} {source_script}:{command_index} money special must use command index 0")]
    CommandIndexMismatch {
        path: String,
        source_script: String,
        command_index: usize,
    },
    #[error("saved {path} {source_script}:{command_index} money event has unexpected coins")]
    UnexpectedCoins {
        path: String,
        source_script: String,
        command_index: usize,
    },
    #[error("saved {path} {source_script}:{command_index} money event is missing coins")]
    MissingCoins {
        path: String,
        source_script: String,
        command_index: usize,
    },
    #[error("saved {path} {source_script}:{command_index} money {money} does not match 0")]
    MoneyMismatch {
        path: String,
        source_script: String,
        command_index: usize,
        money: u32,
    },
    #[error(transparent)]
    Routine(#[from] ScriptMoneyRuntimeRoutineError),
}

pub fn validate_saved_money_runtime_event_shape(
    path: &str,
    event: &ScriptMoneyRuntimeEvent,
) -> Result<(), ScriptMoneyRuntimeEventError> {
    if event.command != "special" {
        return Err(ScriptMoneyRuntimeEventError::CommandMismatch {
            path: path.to_string(),
            source_script: event.source_script.clone(),
            command_index: event.command_index,
            command: event.command.clone(),
        });
    }
    if event.command_index != 0 {
        return Err(ScriptMoneyRuntimeEventError::CommandIndexMismatch {
            path: path.to_string(),
            source_script: event.source_script.clone(),
            command_index: event.command_index,
        });
    }
    match event.kind {
        ScriptMoneyRuntimeKind::PlaceMoneyTopRight => {
            if event.coins.is_some() {
                return Err(ScriptMoneyRuntimeEventError::UnexpectedCoins {
                    path: path.to_string(),
                    source_script: event.source_script.clone(),
                    command_index: event.command_index,
                });
            }
        }
        ScriptMoneyRuntimeKind::DisplayMoneyAndCoinBalance => {
            if event.coins.is_none() {
                return Err(ScriptMoneyRuntimeEventError::MissingCoins {
                    path: path.to_string(),
                    source_script: event.source_script.clone(),
                    command_index: event.command_index,
                });
            }
        }
        ScriptMoneyRuntimeKind::DisplayCoinCaseBalance => {
            if event.money != 0 {
                return Err(ScriptMoneyRuntimeEventError::MoneyMismatch {
                    path: path.to_string(),
                    source_script: event.source_script.clone(),
                    command_index: event.command_index,
                    money: event.money,
                });
            }
            if event.coins.is_none() {
                return Err(ScriptMoneyRuntimeEventError::MissingCoins {
                    path: path.to_string(),
                    source_script: event.source_script.clone(),
                    command_index: event.command_index,
                });
            }
        }
    }
    validate_saved_money_runtime_routine(path, event)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum LastTalkedObjectSaveError {
    #[error("saved script_runtime.last_talked_object {object_id} requires an active overworld map")]
    InactiveOverworld { object_id: String },
    #[error(
        "saved script_runtime.last_talked_object.map {map_name} is missing from compiled pack maps"
    )]
    MissingMap { map_name: String },
    #[error(
        "saved script_runtime.last_talked_object {object_id} is missing from compiled map {map_name} objects"
    )]
    MissingObject { map_name: String, object_id: String },
}

pub fn validate_saved_last_talked_object_reference(
    state: &GameState,
    object_id: &str,
    mut map_exists: impl FnMut(&str) -> bool,
    mut object_exists: impl FnMut(&str, &str) -> bool,
) -> Result<(), LastTalkedObjectSaveError> {
    let OverworldMemory::Active { map_name, .. } = &state.overworld else {
        return Err(LastTalkedObjectSaveError::InactiveOverworld {
            object_id: object_id.to_string(),
        });
    };
    if !map_exists(map_name) {
        return Err(LastTalkedObjectSaveError::MissingMap {
            map_name: map_name.clone(),
        });
    }
    if !object_exists(map_name, object_id) {
        return Err(LastTalkedObjectSaveError::MissingObject {
            map_name: map_name.clone(),
            object_id: object_id.to_string(),
        });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum ScriptCommandReferenceError {
    #[error("compiled script {script_label} for saved {path} is not a command array")]
    NonArrayScript { path: String, script_label: String },
    #[error(
        "saved {path} {script_label}:{command_index} is outside compiled script command count {command_count}"
    )]
    CommandIndexOutOfBounds {
        path: String,
        script_label: String,
        command_index: usize,
        command_count: usize,
    },
    #[error(
        "compiled script {script_label}:{command_index} for saved {path} is missing command name"
    )]
    MissingCommandName {
        path: String,
        script_label: String,
        command_index: usize,
    },
    #[error(
        "saved {path} {script_label}:{command_index} command {saved_command} does not match compiled command {compiled_command}"
    )]
    CommandMismatch {
        path: String,
        script_label: String,
        command_index: usize,
        saved_command: String,
        compiled_command: String,
    },
    #[error("compiled script {script_label}:{command_index} for saved {path} has non-array args")]
    NonArrayArgs {
        path: String,
        script_label: String,
        command_index: usize,
    },
    #[error(
        "compiled script {script_label}:{command_index} for saved {path} has non-string arg {arg_index}"
    )]
    NonStringArg {
        path: String,
        script_label: String,
        command_index: usize,
        arg_index: usize,
    },
    #[error(
        "saved {path} {script_label}:{command_index} args {saved_args:?} do not match compiled args {compiled_args:?}"
    )]
    ArgsMismatch {
        path: String,
        script_label: String,
        command_index: usize,
        saved_args: Vec<String>,
        compiled_args: Vec<String>,
    },
}

pub fn validate_saved_compiled_script_command_reference(
    script_body: &serde_json::Value,
    path: &str,
    script_label: &str,
    command_index: usize,
) -> Result<(), ScriptCommandReferenceError> {
    let commands = compiled_script_commands(script_body, path, script_label)?;
    if command_index < commands.len() {
        Ok(())
    } else {
        Err(ScriptCommandReferenceError::CommandIndexOutOfBounds {
            path: path.to_string(),
            script_label: script_label.to_string(),
            command_index,
            command_count: commands.len(),
        })
    }
}

pub fn validate_saved_compiled_script_command_name_reference(
    script_body: &serde_json::Value,
    path: &str,
    script_label: &str,
    command_index: usize,
    saved_command: &str,
) -> Result<(), ScriptCommandReferenceError> {
    let command = compiled_script_command(script_body, path, script_label, command_index)?;
    let compiled_command =
        compiled_script_command_name(command, path, script_label, command_index)?;
    if compiled_command == saved_command {
        Ok(())
    } else {
        Err(ScriptCommandReferenceError::CommandMismatch {
            path: path.to_string(),
            script_label: script_label.to_string(),
            command_index,
            saved_command: saved_command.to_string(),
            compiled_command: compiled_command.to_string(),
        })
    }
}

pub fn validate_saved_compiled_script_command_payload_reference(
    script_body: &serde_json::Value,
    path: &str,
    script_label: &str,
    command_index: usize,
    saved_command: &str,
    saved_args: &[String],
) -> Result<(), ScriptCommandReferenceError> {
    let command = compiled_script_command(script_body, path, script_label, command_index)?;
    let compiled_command =
        compiled_script_command_name(command, path, script_label, command_index)?;
    if compiled_command != saved_command {
        return Err(ScriptCommandReferenceError::CommandMismatch {
            path: path.to_string(),
            script_label: script_label.to_string(),
            command_index,
            saved_command: saved_command.to_string(),
            compiled_command: compiled_command.to_string(),
        });
    }
    let compiled_args = compiled_script_command_args(command, path, script_label, command_index)?;
    if compiled_args == saved_args {
        Ok(())
    } else {
        Err(ScriptCommandReferenceError::ArgsMismatch {
            path: path.to_string(),
            script_label: script_label.to_string(),
            command_index,
            saved_args: saved_args.to_vec(),
            compiled_args,
        })
    }
}

pub fn validate_saved_compiled_script_return_reference(
    script_body: &serde_json::Value,
    path: &str,
    script_label: &str,
    next_command_index: usize,
) -> Result<(), ScriptCommandReferenceError> {
    let commands = compiled_script_commands(script_body, path, script_label)?;
    if next_command_index <= commands.len() {
        Ok(())
    } else {
        Err(ScriptCommandReferenceError::CommandIndexOutOfBounds {
            path: path.to_string(),
            script_label: script_label.to_string(),
            command_index: next_command_index,
            command_count: commands.len(),
        })
    }
}

fn compiled_script_commands<'a>(
    script_body: &'a serde_json::Value,
    path: &str,
    script_label: &str,
) -> Result<&'a Vec<serde_json::Value>, ScriptCommandReferenceError> {
    script_body
        .as_array()
        .ok_or_else(|| ScriptCommandReferenceError::NonArrayScript {
            path: path.to_string(),
            script_label: script_label.to_string(),
        })
}

fn compiled_script_command<'a>(
    script_body: &'a serde_json::Value,
    path: &str,
    script_label: &str,
    command_index: usize,
) -> Result<&'a serde_json::Value, ScriptCommandReferenceError> {
    let commands = compiled_script_commands(script_body, path, script_label)?;
    commands.get(command_index).ok_or_else(|| {
        ScriptCommandReferenceError::CommandIndexOutOfBounds {
            path: path.to_string(),
            script_label: script_label.to_string(),
            command_index,
            command_count: commands.len(),
        }
    })
}

fn compiled_script_command_name<'a>(
    command: &'a serde_json::Value,
    path: &str,
    script_label: &str,
    command_index: usize,
) -> Result<&'a str, ScriptCommandReferenceError> {
    command
        .get("command")
        .and_then(|value| value.as_str())
        .ok_or_else(|| ScriptCommandReferenceError::MissingCommandName {
            path: path.to_string(),
            script_label: script_label.to_string(),
            command_index,
        })
}

fn compiled_script_command_args(
    command: &serde_json::Value,
    path: &str,
    script_label: &str,
    command_index: usize,
) -> Result<Vec<String>, ScriptCommandReferenceError> {
    let Some(args) = command.get("args") else {
        return Ok(Vec::new());
    };
    let Some(args) = args.as_array() else {
        return Err(ScriptCommandReferenceError::NonArrayArgs {
            path: path.to_string(),
            script_label: script_label.to_string(),
            command_index,
        });
    };
    args.iter()
        .enumerate()
        .map(|(arg_index, value)| {
            value.as_str().map(str::to_string).ok_or_else(|| {
                ScriptCommandReferenceError::NonStringArg {
                    path: path.to_string(),
                    script_label: script_label.to_string(),
                    command_index,
                    arg_index,
                }
            })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum TrainerEnemyPartySaveError {
    #[error(
        "saved battle.trainer.enemy_party has {saved_len} Pokemon, compiled trainer {trainer_id} has {expected_len}"
    )]
    PartyLengthMismatch {
        trainer_id: String,
        saved_len: usize,
        expected_len: usize,
    },
    #[error(
        "saved battle.trainer.enemy_party[{index}] species {saved_species} does not match compiled trainer {trainer_id} species {expected_species}"
    )]
    SpeciesMismatch {
        trainer_id: String,
        index: usize,
        saved_species: String,
        expected_species: String,
    },
    #[error(
        "saved battle.trainer.enemy_party[{index}] level {saved_level} does not match compiled trainer {trainer_id} level {expected_level}"
    )]
    LevelMismatch {
        trainer_id: String,
        index: usize,
        saved_level: u8,
        expected_level: u8,
    },
    #[error(
        "saved battle.trainer.enemy_party[{index}] DVs do not match compiled trainer {trainer_id} DVs"
    )]
    DvMismatch { trainer_id: String, index: usize },
    #[error(
        "saved battle.trainer.enemy_party[{index}] original trainer identity does not match compiled trainer {trainer_id}"
    )]
    OriginalTrainerMismatch { trainer_id: String, index: usize },
    #[error(
        "saved battle.trainer.enemy_party[{index}] has {saved_len} moves, compiled trainer {trainer_id} has {expected_len}"
    )]
    MoveLengthMismatch {
        trainer_id: String,
        index: usize,
        saved_len: usize,
        expected_len: usize,
    },
    #[error(
        "saved battle.trainer.enemy_party[{index}].moves[{move_index}] {saved_move} pp_ups {saved_pp_ups} does not match compiled trainer {trainer_id} move {expected_move} pp_ups {expected_pp_ups}"
    )]
    MoveMismatch {
        trainer_id: String,
        index: usize,
        move_index: usize,
        saved_move: String,
        saved_pp_ups: u8,
        expected_move: String,
        expected_pp_ups: u8,
    },
    #[error(
        "saved battle.trainer.enemy_pokemon {species} level {level} does not match any compiled trainer {trainer_id} party slot"
    )]
    ActiveEnemyMissingFromParty {
        trainer_id: String,
        species: String,
        level: u8,
    },
}

pub fn validate_saved_trainer_enemy_party_identity(
    trainer_id: &str,
    enemy_party: &[Pokemon],
    enemy_pokemon: &Pokemon,
    expected_party: &[Pokemon],
) -> Result<(), TrainerEnemyPartySaveError> {
    if enemy_party.len() != expected_party.len() {
        return Err(TrainerEnemyPartySaveError::PartyLengthMismatch {
            trainer_id: trainer_id.to_string(),
            saved_len: enemy_party.len(),
            expected_len: expected_party.len(),
        });
    }
    for (index, (saved, expected)) in enemy_party.iter().zip(expected_party.iter()).enumerate() {
        validate_saved_trainer_enemy_pokemon_identity(trainer_id, index, saved, expected)?;
    }
    if !enemy_party.iter().any(|pokemon| {
        pokemon.species.id == enemy_pokemon.species.id
            && pokemon.level == enemy_pokemon.level
            && pokemon.dvs == enemy_pokemon.dvs
    }) {
        return Err(TrainerEnemyPartySaveError::ActiveEnemyMissingFromParty {
            trainer_id: trainer_id.to_string(),
            species: enemy_pokemon.species.id.clone(),
            level: enemy_pokemon.level,
        });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum WildBattleSaveError {
    #[error(
        "saved battle.wild.battle_type {battle_type} does not match runtime wild battle type BATTLETYPE_NORMAL"
    )]
    BattleTypeMismatch { battle_type: String },
    #[error(
        "saved battle.wild {map_name} encounter {species}:{level} is missing from compiled wild encounter sources"
    )]
    MissingEncounter {
        map_name: String,
        species: String,
        level: u8,
    },
}

pub fn validate_saved_wild_battle_origin_reference(
    battle_type: &str,
    map_name: &str,
    enemy_pokemon: &Pokemon,
    mut encounter_exists: impl FnMut(&str, &str, u8) -> bool,
) -> Result<(), WildBattleSaveError> {
    if battle_type != "BATTLETYPE_NORMAL" {
        return Err(WildBattleSaveError::BattleTypeMismatch {
            battle_type: battle_type.to_string(),
        });
    }
    let species = enemy_pokemon.species.id.as_str();
    let level = enemy_pokemon.level;
    if encounter_exists(map_name, species, level) {
        Ok(())
    } else {
        Err(WildBattleSaveError::MissingEncounter {
            map_name: map_name.to_string(),
            species: species.to_string(),
            level,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum StaticWildBattleSaveError {
    #[error(
        "saved battle.static_wild {source_script} request {battle_type}:{species}:{level} is missing from compiled scripted wild battles"
    )]
    MissingScriptedBattle {
        source_script: String,
        battle_type: String,
        species: String,
        level: u8,
    },
}

pub fn validate_saved_static_wild_battle_origin_reference(
    battle_type: &str,
    species: &str,
    level: u8,
    source_script: &str,
    mut scripted_battle_exists: impl FnMut(&str, &str, &str, u8) -> bool,
) -> Result<(), StaticWildBattleSaveError> {
    if scripted_battle_exists(source_script, battle_type, species, level) {
        Ok(())
    } else {
        Err(StaticWildBattleSaveError::MissingScriptedBattle {
            source_script: source_script.to_string(),
            battle_type: battle_type.to_string(),
            species: species.to_string(),
            level,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum TrainerBattleRequestSaveError {
    #[error(
        "saved battle.trainer.source_script {source_script} is missing from compiled pack trainer battle sources"
    )]
    MissingSource { source_script: String },
    #[error(
        "saved battle.trainer.{field} {saved} does not match compiled trainer battle {source_script} {field} {compiled}"
    )]
    FieldMismatch {
        field: String,
        saved: String,
        compiled: String,
        source_script: String,
    },
}

pub fn validate_saved_trainer_battle_source_reference(
    source_script: &str,
    source_exists: impl FnOnce(&str) -> bool,
) -> Result<(), TrainerBattleRequestSaveError> {
    if source_exists(source_script) {
        Ok(())
    } else {
        Err(TrainerBattleRequestSaveError::MissingSource {
            source_script: source_script.to_string(),
        })
    }
}

pub fn validate_saved_trainer_battle_request_field(
    field: &str,
    saved: &str,
    compiled: &str,
    source_script: &str,
) -> Result<(), TrainerBattleRequestSaveError> {
    if saved == compiled {
        Ok(())
    } else {
        Err(TrainerBattleRequestSaveError::FieldMismatch {
            field: field.to_string(),
            saved: saved.to_string(),
            compiled: compiled.to_string(),
            source_script: source_script.to_string(),
        })
    }
}

pub struct SavedTrainerBattleFields<'a> {
    pub battle_type: &'a str,
    pub trainer_class: &'a str,
    pub event_flag: &'a str,
    pub seen_text: &'a str,
    pub win_text: &'a str,
    pub loss_text: &'a str,
    pub callback: &'a str,
}

pub fn validate_saved_trainer_battle_request_fields(
    saved: SavedTrainerBattleFields<'_>,
    compiled: SavedTrainerBattleFields<'_>,
    source_script: &str,
) -> Result<(), TrainerBattleRequestSaveError> {
    validate_saved_trainer_battle_request_field(
        "battle_type",
        saved.battle_type,
        compiled.battle_type,
        source_script,
    )?;
    validate_saved_trainer_battle_request_field(
        "trainer_class",
        saved.trainer_class,
        compiled.trainer_class,
        source_script,
    )?;
    validate_saved_trainer_battle_request_field(
        "event_flag",
        saved.event_flag,
        compiled.event_flag,
        source_script,
    )?;
    validate_saved_trainer_battle_request_field(
        "seen_text",
        saved.seen_text,
        compiled.seen_text,
        source_script,
    )?;
    validate_saved_trainer_battle_request_field(
        "win_text",
        saved.win_text,
        compiled.win_text,
        source_script,
    )?;
    validate_saved_trainer_battle_request_field(
        "loss_text",
        saved.loss_text,
        compiled.loss_text,
        source_script,
    )?;
    validate_saved_trainer_battle_request_field(
        "callback",
        saved.callback,
        compiled.callback,
        source_script,
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum TrainerMetadataSaveError {
    #[error(
        "saved battle.trainer.trainer_class {saved} does not match compiled pack trainer {trainer_id} class {compiled}"
    )]
    ClassMismatch {
        trainer_id: String,
        saved: String,
        compiled: String,
    },
    #[error(
        "saved battle.trainer.trainer_name {saved} does not match compiled pack trainer {trainer_id} name {compiled}"
    )]
    NameMismatch {
        trainer_id: String,
        saved: String,
        compiled: String,
    },
    #[error(
        "saved battle.trainer.ai_move_flags {saved} does not match compiled pack trainer {trainer_id} ai_move_flags {compiled}"
    )]
    AiMoveFlagsMismatch {
        trainer_id: String,
        saved: u32,
        compiled: u32,
    },
    #[error(
        "saved battle.trainer.ai_item_switch_flags {saved} does not match compiled pack trainer {trainer_id} ai_item_switch_flags {compiled}"
    )]
    AiItemSwitchFlagsMismatch {
        trainer_id: String,
        saved: u32,
        compiled: u32,
    },
    #[error(
        "saved battle.trainer.ai_layers {saved:?} do not match compiled pack trainer {trainer_id} ai_layers {compiled:?}"
    )]
    AiLayersMismatch {
        trainer_id: String,
        saved: Vec<String>,
        compiled: Vec<String>,
    },
    #[error(
        "saved battle.trainer.reward {saved} does not match compiled pack trainer {trainer_id} base_reward {compiled}"
    )]
    RewardMismatch {
        trainer_id: String,
        saved: u32,
        compiled: u32,
    },
    #[error(
        "saved battle.trainer.encounter_music {saved} does not match compiled pack trainer {trainer_id} encounter music {compiled}"
    )]
    EncounterMusicMismatch {
        trainer_id: String,
        saved: String,
        compiled: String,
    },
}

pub struct SavedTrainerMetadata<'a> {
    pub trainer_class: &'a str,
    pub trainer_name: &'a str,
    pub ai_move_flags: u32,
    pub ai_item_switch_flags: u32,
    pub ai_layers: &'a [String],
    pub reward: u32,
    pub encounter_music: &'a str,
}

pub fn validate_saved_trainer_metadata(
    trainer: &Trainer,
    saved: SavedTrainerMetadata<'_>,
) -> Result<(), TrainerMetadataSaveError> {
    if trainer.trainer_class != saved.trainer_class {
        return Err(TrainerMetadataSaveError::ClassMismatch {
            trainer_id: trainer.trainer_id.clone(),
            saved: saved.trainer_class.to_string(),
            compiled: trainer.trainer_class.clone(),
        });
    }
    if trainer.name != saved.trainer_name {
        return Err(TrainerMetadataSaveError::NameMismatch {
            trainer_id: trainer.trainer_id.clone(),
            saved: saved.trainer_name.to_string(),
            compiled: trainer.name.clone(),
        });
    }
    if trainer.ai_move_flags != saved.ai_move_flags {
        return Err(TrainerMetadataSaveError::AiMoveFlagsMismatch {
            trainer_id: trainer.trainer_id.clone(),
            saved: saved.ai_move_flags,
            compiled: trainer.ai_move_flags,
        });
    }
    if trainer.ai_item_switch_flags != saved.ai_item_switch_flags {
        return Err(TrainerMetadataSaveError::AiItemSwitchFlagsMismatch {
            trainer_id: trainer.trainer_id.clone(),
            saved: saved.ai_item_switch_flags,
            compiled: trainer.ai_item_switch_flags,
        });
    }
    if trainer.ai_layers != saved.ai_layers {
        return Err(TrainerMetadataSaveError::AiLayersMismatch {
            trainer_id: trainer.trainer_id.clone(),
            saved: saved.ai_layers.to_vec(),
            compiled: trainer.ai_layers.clone(),
        });
    }
    if trainer.base_reward != saved.reward {
        return Err(TrainerMetadataSaveError::RewardMismatch {
            trainer_id: trainer.trainer_id.clone(),
            saved: saved.reward,
            compiled: trainer.base_reward,
        });
    }
    if trainer.encounter_music != saved.encounter_music {
        return Err(TrainerMetadataSaveError::EncounterMusicMismatch {
            trainer_id: trainer.trainer_id.clone(),
            saved: saved.encounter_music.to_string(),
            compiled: trainer.encounter_music.clone(),
        });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum PokemonReferenceSaveError {
    #[error("saved {path}.species {species} is missing from compiled pack pokemon")]
    MissingSpecies { path: String, species: String },
    #[error("saved {path}.species {species} does not match compiled pack species data")]
    SpeciesMismatch { path: String, species: String },
    #[error("saved {path}.item {item_id} is missing from compiled pack items")]
    MissingItem { path: String, item_id: String },
    #[error("saved {path}.status {status} is missing from compiled pack status declarations")]
    MissingStatus { path: String, status: String },
    #[error("saved {path}.moves[{index}] {move_name} is missing from compiled pack moves")]
    MissingMove {
        path: String,
        index: usize,
        move_name: String,
    },
    #[error(
        "saved {path}.moves[{index}] {move_name} does not match compiled move name {compiled_move}"
    )]
    MoveNameMismatch {
        path: String,
        index: usize,
        move_name: String,
        compiled_move: String,
    },
    #[error(
        "saved {path}.moves[{index}] {move_name} current_pp {current_pp} exceeds compiled max PP {max_pp}"
    )]
    MovePpOverflow {
        path: String,
        index: usize,
        move_name: String,
        current_pp: u8,
        max_pp: u8,
    },
}

pub fn validate_saved_pokemon_reference(
    path: &str,
    pokemon: &Pokemon,
    mut compiled_species: impl FnMut(&str) -> Option<PokemonSpecies>,
    mut item_exists: impl FnMut(&str) -> bool,
    mut status_exists: impl FnMut(&str) -> bool,
    mut compiled_move: impl FnMut(&str) -> Option<(String, u8)>,
) -> Result<(), PokemonReferenceSaveError> {
    let species_id = pokemon.species.id.as_str();
    let Some(compiled) = compiled_species(species_id) else {
        return Err(PokemonReferenceSaveError::MissingSpecies {
            path: path.to_string(),
            species: species_id.to_string(),
        });
    };
    if pokemon.species != compiled {
        return Err(PokemonReferenceSaveError::SpeciesMismatch {
            path: path.to_string(),
            species: species_id.to_string(),
        });
    }
    if let Some(item_id) = &pokemon.item {
        if !item_exists(item_id) {
            return Err(PokemonReferenceSaveError::MissingItem {
                path: path.to_string(),
                item_id: item_id.clone(),
            });
        }
    }
    if let Some(status) = &pokemon.status {
        if !status_exists(status) {
            return Err(PokemonReferenceSaveError::MissingStatus {
                path: path.to_string(),
                status: status.clone(),
            });
        }
    }
    for (index, learned_move) in pokemon.moves.iter().enumerate() {
        let Some((compiled_name, base_pp)) = compiled_move(&learned_move.name) else {
            return Err(PokemonReferenceSaveError::MissingMove {
                path: path.to_string(),
                index,
                move_name: learned_move.name.clone(),
            });
        };
        if compiled_name != learned_move.name {
            return Err(PokemonReferenceSaveError::MoveNameMismatch {
                path: path.to_string(),
                index,
                move_name: learned_move.name.clone(),
                compiled_move: compiled_name,
            });
        }
        let max_pp = crate::models::max_move_pp(base_pp, learned_move.pp_ups);
        if learned_move.current_pp > max_pp {
            return Err(PokemonReferenceSaveError::MovePpOverflow {
                path: path.to_string(),
                index,
                move_name: learned_move.name.clone(),
                current_pp: learned_move.current_pp,
                max_pp,
            });
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum PokemonPartyReferenceSaveError {
    #[error("{0}")]
    Pokemon(String),
}

pub fn validate_saved_pokemon_party_references(
    path: &str,
    party: &[Pokemon],
    mut validate_pokemon: impl FnMut(&str, &Pokemon) -> Result<(), String>,
) -> Result<(), PokemonPartyReferenceSaveError> {
    for (index, pokemon) in party.iter().enumerate() {
        validate_pokemon(&format!("{path}[{index}]"), pokemon)
            .map_err(PokemonPartyReferenceSaveError::Pokemon)?;
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum MapReferenceSaveError {
    #[error("saved {path} {map_name} is missing from compiled pack maps")]
    MissingMap { path: String, map_name: String },
    #[error("saved {path} {map_name} does not match compiled map id {compiled_id}")]
    MapIdMismatch {
        path: String,
        map_name: String,
        compiled_id: String,
    },
    #[error("saved {path} {warp_index} is missing from compiled map {map_name} warps")]
    MissingWarp {
        path: String,
        map_name: String,
        warp_index: u16,
    },
    #[error("saved {path} {object_id} is missing from compiled map {map_name} objects")]
    MissingObject {
        path: String,
        map_name: String,
        object_id: String,
    },
}

pub fn validate_saved_map_reference(
    path: &str,
    map_name: &str,
    compiled_map_id: Option<&str>,
) -> Result<(), MapReferenceSaveError> {
    let Some(compiled_id) = compiled_map_id else {
        return Err(MapReferenceSaveError::MissingMap {
            path: path.to_string(),
            map_name: map_name.to_string(),
        });
    };
    if compiled_id != map_name {
        return Err(MapReferenceSaveError::MapIdMismatch {
            path: path.to_string(),
            map_name: map_name.to_string(),
            compiled_id: compiled_id.to_string(),
        });
    }
    Ok(())
}

pub fn validate_saved_warp_reference(
    path: &str,
    map_name: &str,
    warp_index: u16,
    compiled_map_id: Option<&str>,
    warp_exists: impl FnOnce(u16) -> bool,
) -> Result<(), MapReferenceSaveError> {
    validate_saved_map_reference(path, map_name, compiled_map_id)?;
    if warp_exists(warp_index) {
        Ok(())
    } else {
        Err(MapReferenceSaveError::MissingWarp {
            path: path.to_string(),
            map_name: map_name.to_string(),
            warp_index,
        })
    }
}

pub fn validate_saved_map_object_reference(
    path: &str,
    map_name: &str,
    object_id: &str,
    object_exists: impl FnOnce(&str) -> bool,
) -> Result<(), MapReferenceSaveError> {
    if object_exists(object_id) {
        Ok(())
    } else {
        Err(MapReferenceSaveError::MissingObject {
            path: path.to_string(),
            map_name: map_name.to_string(),
            object_id: object_id.to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum OverworldReferenceSaveError {
    #[error("saved overworld.active.map_name {map_name} is missing from compiled pack maps")]
    MissingMap { map_name: String },
    #[error("saved overworld.active tile ({x}, {y}) is not aligned to a raw map event coordinate")]
    UnalignedTile { map_name: String, x: i16, y: i16 },
    #[error(
        "saved overworld.active tile ({x}, {y}) is outside compiled map {map_name} runtime tile bounds {width}x{height}"
    )]
    TileOutOfBounds {
        map_name: String,
        x: i16,
        y: i16,
        width: u16,
        height: u16,
    },
}

pub fn validate_saved_overworld_references(
    overworld: &OverworldMemory,
    runtime_tile_bounds: impl FnOnce(&str) -> Option<(u16, u16)>,
) -> Result<(), OverworldReferenceSaveError> {
    let OverworldMemory::Active { map_name, tile, .. } = overworld else {
        return Ok(());
    };
    let Some((width, height)) = runtime_tile_bounds(map_name) else {
        return Err(OverworldReferenceSaveError::MissingMap {
            map_name: map_name.clone(),
        });
    };
    if tile.x < 0
        || tile.y < 0
        || u16::try_from(tile.x).map_or(true, |x| x >= width)
        || u16::try_from(tile.y).map_or(true, |y| y >= height)
    {
        Err(OverworldReferenceSaveError::TileOutOfBounds {
            map_name: map_name.clone(),
            x: tile.x,
            y: tile.y,
            width,
            height,
        })
    } else if runtime_tile_to_raw_event_tile(*tile).is_none() {
        Err(OverworldReferenceSaveError::UnalignedTile {
            map_name: map_name.clone(),
            x: tile.x,
            y: tile.y,
        })
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum BlockOverrideSaveError {
    #[error("saved map_block_overrides {map_name} is missing from compiled pack maps")]
    MissingMap { map_name: String },
    #[error(
        "saved map_block_overrides {map_name} references missing compiled tileset {tileset_name}"
    )]
    MissingTileset {
        map_name: String,
        tileset_name: String,
    },
    #[error(
        "saved map_block_overrides {map_name} coordinate ({x}, {y}) is outside compiled map dimensions {width}x{height}"
    )]
    CoordinateOutOfBounds {
        map_name: String,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
    },
    #[error(
        "saved map_block_overrides {map_name} coordinate ({x}, {y}) block {block_id:#04x} is missing from compiled tileset {tileset_name} collision data"
    )]
    MissingMetatile {
        map_name: String,
        x: u16,
        y: u16,
        block_id: u16,
        tileset_name: String,
    },
}

pub fn validate_saved_block_overrides(
    map_name: &str,
    overrides: &BTreeMap<(u16, u16), u16>,
    map_metadata: impl FnOnce(&str) -> Option<(u16, u16, String)>,
    tileset_exists: impl FnOnce(&str) -> bool,
    mut metatile_exists: impl FnMut(&str, u16) -> bool,
) -> Result<(), BlockOverrideSaveError> {
    let Some((width, height, tileset_name)) = map_metadata(map_name) else {
        return Err(BlockOverrideSaveError::MissingMap {
            map_name: map_name.to_string(),
        });
    };
    if !tileset_exists(&tileset_name) {
        return Err(BlockOverrideSaveError::MissingTileset {
            map_name: map_name.to_string(),
            tileset_name,
        });
    }
    for ((x, y), block_id) in overrides {
        if *x >= width || *y >= height {
            return Err(BlockOverrideSaveError::CoordinateOutOfBounds {
                map_name: map_name.to_string(),
                x: *x,
                y: *y,
                width,
                height,
            });
        }
        if !metatile_exists(&tileset_name, *block_id) {
            return Err(BlockOverrideSaveError::MissingMetatile {
                map_name: map_name.to_string(),
                x: *x,
                y: *y,
                block_id: *block_id,
                tileset_name,
            });
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum ObjectOverrideSaveError {
    #[error("saved map_object_overrides {map_name} is missing from compiled pack maps")]
    MissingMap { map_name: String },
    #[error("saved {path} {object_id} is missing from compiled map {map_name} object events")]
    MissingObject {
        path: String,
        map_name: String,
        object_id: String,
    },
    #[error(
        "saved map_object_overrides.objects {map_name}:{object_id} raw coordinate ({x}, {y}) overflows runtime tile coordinates"
    )]
    CoordinateOutOfRange {
        map_name: String,
        object_id: String,
        x: u16,
        y: u16,
    },
    #[error(
        "saved map_object_overrides.objects {map_name}:{object_id} raw coordinate ({raw_x}, {raw_y}) resolves to runtime tile ({runtime_x}, {runtime_y}) outside compiled runtime tile bounds {width}x{height}"
    )]
    RuntimeTileOutOfBounds {
        map_name: String,
        object_id: String,
        raw_x: u16,
        raw_y: u16,
        runtime_x: i16,
        runtime_y: i16,
        width: u16,
        height: u16,
    },
}

pub fn validate_saved_object_overrides(
    map_name: &str,
    memory: &OverworldObjectMapMemory,
    runtime_tile_bounds: impl FnOnce(&str) -> Option<(u16, u16)>,
    mut object_exists: impl FnMut(&str) -> bool,
) -> Result<(), ObjectOverrideSaveError> {
    let Some((width, height)) = runtime_tile_bounds(map_name) else {
        return Err(ObjectOverrideSaveError::MissingMap {
            map_name: map_name.to_string(),
        });
    };
    for (object_id, object_memory) in &memory.objects {
        validate_object_override_object(
            map_name,
            "map_object_overrides.objects",
            object_id,
            &mut object_exists,
        )?;
        let Some(runtime_tile) =
            raw_event_tile_to_runtime_tile_checked(object_memory.x, object_memory.y)
        else {
            return Err(ObjectOverrideSaveError::CoordinateOutOfRange {
                map_name: map_name.to_string(),
                object_id: object_id.clone(),
                x: object_memory.x,
                y: object_memory.y,
            });
        };
        if runtime_tile.x < 0
            || runtime_tile.y < 0
            || i32::from(runtime_tile.x) >= i32::from(width)
            || i32::from(runtime_tile.y) >= i32::from(height)
        {
            return Err(ObjectOverrideSaveError::RuntimeTileOutOfBounds {
                map_name: map_name.to_string(),
                object_id: object_id.clone(),
                raw_x: object_memory.x,
                raw_y: object_memory.y,
                runtime_x: runtime_tile.x,
                runtime_y: runtime_tile.y,
                width,
                height,
            });
        }
    }
    for object_id in &memory.hidden_object_identifiers {
        validate_object_override_object(
            map_name,
            "map_object_overrides.hidden_object_identifiers",
            object_id,
            &mut object_exists,
        )?;
    }
    if let Some(object_id) = &memory.last_talked_object_identifier {
        validate_object_override_object(
            map_name,
            "map_object_overrides.last_talked_object_identifier",
            object_id,
            &mut object_exists,
        )?;
    }
    if let Some(following) = &memory.following {
        validate_object_override_actor(
            map_name,
            "map_object_overrides.following.leader_object_id",
            &following.leader_object_id,
            &mut object_exists,
        )?;
        validate_object_override_actor(
            map_name,
            "map_object_overrides.following.follower_object_id",
            &following.follower_object_id,
            &mut object_exists,
        )?;
    }
    Ok(())
}

fn validate_object_override_object(
    map_name: &str,
    path: &str,
    object_id: &str,
    object_exists: &mut impl FnMut(&str) -> bool,
) -> Result<(), ObjectOverrideSaveError> {
    if object_exists(object_id) {
        Ok(())
    } else {
        Err(ObjectOverrideSaveError::MissingObject {
            path: path.to_string(),
            map_name: map_name.to_string(),
            object_id: object_id.to_string(),
        })
    }
}

fn validate_object_override_actor(
    map_name: &str,
    path: &str,
    object_id: &str,
    object_exists: &mut impl FnMut(&str) -> bool,
) -> Result<(), ObjectOverrideSaveError> {
    if object_id == "PLAYER" {
        Ok(())
    } else {
        validate_object_override_object(map_name, path, object_id, object_exists)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum CatalogReferenceSaveError {
    #[error("saved {path} {value} is missing from compiled pack {catalog}")]
    Missing {
        path: String,
        value: String,
        catalog: String,
    },
}

pub fn validate_saved_catalog_reference(
    path: &str,
    value: &str,
    catalog: &str,
    exists: impl FnOnce(&str) -> bool,
) -> Result<(), CatalogReferenceSaveError> {
    if exists(value) {
        Ok(())
    } else {
        Err(CatalogReferenceSaveError::Missing {
            path: path.to_string(),
            value: value.to_string(),
            catalog: catalog.to_string(),
        })
    }
}

pub fn validate_saved_optional_catalog_reference(
    path: &str,
    value: &str,
    catalog: &str,
    exists: impl FnOnce(&str) -> bool,
) -> Result<(), CatalogReferenceSaveError> {
    if value.is_empty() {
        Ok(())
    } else {
        validate_saved_catalog_reference(path, value, catalog, exists)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum ExactCatalogReferenceSaveError {
    #[error("saved {path} {value} is missing from compiled pack {catalog}")]
    Missing {
        path: String,
        value: String,
        catalog: String,
    },
    #[error("saved {path} {value} does not match compiled {compiled_field} {compiled_value}")]
    Mismatch {
        path: String,
        value: String,
        compiled_field: String,
        compiled_value: String,
    },
}

pub fn validate_saved_exact_catalog_reference(
    path: &str,
    value: &str,
    catalog: &str,
    compiled_field: &str,
    compiled_value: Option<String>,
) -> Result<(), ExactCatalogReferenceSaveError> {
    let Some(compiled_value) = compiled_value else {
        return Err(ExactCatalogReferenceSaveError::Missing {
            path: path.to_string(),
            value: value.to_string(),
            catalog: catalog.to_string(),
        });
    };
    if compiled_value == value {
        Ok(())
    } else {
        Err(ExactCatalogReferenceSaveError::Mismatch {
            path: path.to_string(),
            value: value.to_string(),
            compiled_field: compiled_field.to_string(),
            compiled_value,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum AudioReferenceSaveError {
    #[error("saved {path} {audio_id} is missing from compiled pack audio")]
    MissingAudio { path: String, audio_id: String },
    #[error("saved {path} {audio_id} is compiled as {actual_kind}, expected {expected_kind}")]
    KindMismatch {
        path: String,
        audio_id: String,
        actual_kind: String,
        expected_kind: String,
    },
}

pub fn validate_saved_audio_reference(
    path: &str,
    audio_id: &str,
    expected_kind: &str,
    compiled_kind: Option<&str>,
) -> Result<(), AudioReferenceSaveError> {
    let Some(actual_kind) = compiled_kind else {
        return Err(AudioReferenceSaveError::MissingAudio {
            path: path.to_string(),
            audio_id: audio_id.to_string(),
        });
    };
    if actual_kind == expected_kind {
        Ok(())
    } else {
        Err(AudioReferenceSaveError::KindMismatch {
            path: path.to_string(),
            audio_id: audio_id.to_string(),
            actual_kind: actual_kind.to_string(),
            expected_kind: expected_kind.to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum StorageReferenceSaveError {
    #[error("{0}")]
    Pokemon(String),
}

pub fn validate_saved_storage_references(
    storage: &PokemonStorage,
    mut validate_pokemon: impl FnMut(&str, &Pokemon) -> Result<(), String>,
) -> Result<(), StorageReferenceSaveError> {
    for (index, pokemon) in storage.party.pokemon.iter().enumerate() {
        if let Some(pokemon) = pokemon {
            validate_pokemon(&format!("storage.party[{index}]"), pokemon)
                .map_err(StorageReferenceSaveError::Pokemon)?;
        }
    }
    for (box_index, pc_box) in storage.pc_boxes.iter().enumerate() {
        for (slot_index, pokemon) in pc_box.pokemon.iter().enumerate() {
            if let Some(pokemon) = pokemon {
                validate_pokemon(
                    &format!("storage.pc_boxes[{box_index}][{slot_index}]"),
                    pokemon,
                )
                .map_err(StorageReferenceSaveError::Pokemon)?;
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum BugContestReferenceSaveError {
    #[error("{0}")]
    Pokemon(String),
    #[error("saved {path} {species} is missing from compiled pack pokemon")]
    MissingSpecies { path: String, species: String },
    #[error(
        "saved bug_contest.selected_contestant_flags {flag} is missing from compiled pack event flags"
    )]
    MissingContestantFlag { flag: String },
}

pub fn validate_saved_bug_contest_references(
    bug_contest: &BugContestState,
    mut validate_pokemon: impl FnMut(&str, &Pokemon) -> Result<(), String>,
    mut species_exists: impl FnMut(&str) -> bool,
    mut event_flag_exists: impl FnMut(&str) -> bool,
) -> Result<(), BugContestReferenceSaveError> {
    for (index, pokemon) in bug_contest.party_backup.iter().enumerate() {
        validate_pokemon(&format!("bug_contest.party_backup[{index}]"), pokemon)
            .map_err(BugContestReferenceSaveError::Pokemon)?;
    }
    if let Some(species) = &bug_contest.second_party_species {
        if !species_exists(species) {
            return Err(BugContestReferenceSaveError::MissingSpecies {
                path: "bug_contest.second_party_species".to_string(),
                species: species.clone(),
            });
        }
    }
    if let Some(pokemon) = &bug_contest.caught_mon {
        validate_pokemon("bug_contest.caught_mon", pokemon)
            .map_err(BugContestReferenceSaveError::Pokemon)?;
    }
    if let Some(species) = &bug_contest.caught_species {
        if !species_exists(species) {
            return Err(BugContestReferenceSaveError::MissingSpecies {
                path: "bug_contest.caught_species".to_string(),
                species: species.clone(),
            });
        }
    }
    for flag in &bug_contest.selected_contestant_flags {
        if !event_flag_exists(flag) {
            return Err(BugContestReferenceSaveError::MissingContestantFlag { flag: flag.clone() });
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum DayCareReferenceSaveError {
    #[error("{0}")]
    Pokemon(String),
    #[error(
        "saved day_care.last_interaction.pokemon {species} is missing from compiled pack pokemon"
    )]
    MissingInteractionSpecies { species: String },
}

pub fn validate_saved_day_care_references(
    day_care: &DayCareState,
    mut validate_pokemon: impl FnMut(&str, &Pokemon) -> Result<(), String>,
    mut species_exists: impl FnMut(&str) -> bool,
) -> Result<(), DayCareReferenceSaveError> {
    if let Some(pokemon) = &day_care.man.pokemon {
        validate_pokemon("day_care.man.pokemon", pokemon)
            .map_err(DayCareReferenceSaveError::Pokemon)?;
    }
    if let Some(pokemon) = &day_care.lady.pokemon {
        validate_pokemon("day_care.lady.pokemon", pokemon)
            .map_err(DayCareReferenceSaveError::Pokemon)?;
    }
    if let Some(pokemon) = &day_care.egg {
        validate_pokemon("day_care.egg", pokemon).map_err(DayCareReferenceSaveError::Pokemon)?;
        if !pokemon.is_egg {
            return Err(DayCareReferenceSaveError::Pokemon(
                "day_care.egg must retain egg identity".to_string(),
            ));
        }
    }
    if let Some(interaction) = &day_care.last_interaction {
        if let Some(species) = &interaction.pokemon {
            if !species_exists(species) {
                return Err(DayCareReferenceSaveError::MissingInteractionSpecies {
                    species: species.clone(),
                });
            }
        }
    }
    Ok(())
}

fn validate_saved_trainer_enemy_pokemon_identity(
    trainer_id: &str,
    index: usize,
    saved: &Pokemon,
    expected: &Pokemon,
) -> Result<(), TrainerEnemyPartySaveError> {
    if saved.species.id != expected.species.id {
        return Err(TrainerEnemyPartySaveError::SpeciesMismatch {
            trainer_id: trainer_id.to_string(),
            index,
            saved_species: saved.species.id.clone(),
            expected_species: expected.species.id.clone(),
        });
    }
    if saved.level != expected.level {
        return Err(TrainerEnemyPartySaveError::LevelMismatch {
            trainer_id: trainer_id.to_string(),
            index,
            saved_level: saved.level,
            expected_level: expected.level,
        });
    }
    if saved.dvs != expected.dvs {
        return Err(TrainerEnemyPartySaveError::DvMismatch {
            trainer_id: trainer_id.to_string(),
            index,
        });
    }
    if saved.original_trainer_name != expected.original_trainer_name
        || saved.original_trainer_id != expected.original_trainer_id
    {
        return Err(TrainerEnemyPartySaveError::OriginalTrainerMismatch {
            trainer_id: trainer_id.to_string(),
            index,
        });
    }
    if saved.moves.len() != expected.moves.len() {
        return Err(TrainerEnemyPartySaveError::MoveLengthMismatch {
            trainer_id: trainer_id.to_string(),
            index,
            saved_len: saved.moves.len(),
            expected_len: expected.moves.len(),
        });
    }
    for (move_index, (saved_move, expected_move)) in
        saved.moves.iter().zip(expected.moves.iter()).enumerate()
    {
        if saved_move.name != expected_move.name || saved_move.pp_ups != expected_move.pp_ups {
            return Err(TrainerEnemyPartySaveError::MoveMismatch {
                trainer_id: trainer_id.to_string(),
                index,
                move_index,
                saved_move: saved_move.name.clone(),
                saved_pp_ups: saved_move.pp_ups,
                expected_move: expected_move.name.clone(),
                expected_pp_ups: expected_move.pp_ups,
            });
        }
    }
    Ok(())
}

pub fn validate_saved_money_runtime_routine(
    path: &str,
    event: &ScriptMoneyRuntimeEvent,
) -> Result<(), ScriptMoneyRuntimeRoutineError> {
    let Some(expected) = money_runtime_routine_kind(&event.source_script) else {
        return Err(ScriptMoneyRuntimeRoutineError::UnknownRoutine {
            path: path.to_string(),
            routine: event.source_script.clone(),
        });
    };
    if event.kind != expected {
        return Err(ScriptMoneyRuntimeRoutineError::KindMismatch {
            path: path.to_string(),
            routine: event.source_script.clone(),
            actual: event.kind,
            expected,
        });
    }
    Ok(())
}

fn money_runtime_routine_kind(routine: &str) -> Option<ScriptMoneyRuntimeKind> {
    match routine {
        "PlaceMoneyTopRight" => Some(ScriptMoneyRuntimeKind::PlaceMoneyTopRight),
        "DisplayMoneyAndCoinBalance" => Some(ScriptMoneyRuntimeKind::DisplayMoneyAndCoinBalance),
        "DisplayCoinCaseBalance" => Some(ScriptMoneyRuntimeKind::DisplayCoinCaseBalance),
        _ => None,
    }
}

fn validate_map_event_payload(index: usize, event: &ScriptMapRuntimeEvent) -> Result<(), String> {
    match event.kind {
        ScriptMapRuntimeKind::NoWarp => {
            if event.command != "warp" {
                return Err(format!(
                    "map_events[{index}].command {} is not valid for NoWarp",
                    event.command
                ));
            }
            if event.target_map.is_some()
                || event.tile.is_some()
                || event.facing.is_some()
                || event.map_setup.is_some()
            {
                return Err(format!(
                    "map_events[{index}] {:?} cannot carry map payload",
                    event.kind
                ));
            }
        }
        ScriptMapRuntimeKind::WarpCheck => {
            if event.command != "warpcheck" {
                return Err(format!(
                    "map_events[{index}].command {} is not valid for WarpCheck",
                    event.command
                ));
            }
            if event.target_map.is_some()
                || event.tile.is_some()
                || event.facing.is_some()
                || event.map_setup.is_some()
            {
                return Err(format!(
                    "map_events[{index}] {:?} cannot carry map payload",
                    event.kind
                ));
            }
        }
        ScriptMapRuntimeKind::Warp => {
            if event.target_map.is_none() {
                return Err(format!(
                    "map_events[{index}].target_map is required for Warp"
                ));
            }
            if event.tile.is_none() {
                return Err(format!("map_events[{index}].tile is required for Warp"));
            }
            if event.map_setup.is_some() {
                return Err(format!(
                    "map_events[{index}].map_setup is not valid for Warp"
                ));
            }
            let expected_command = if event.facing.is_some() {
                "warpfacing"
            } else {
                "warp"
            };
            if event.command != expected_command {
                return Err(format!(
                    "map_events[{index}].command {} must be {expected_command} for Warp",
                    event.command
                ));
            }
            let tile = event
                .tile
                .expect("Warp map runtime event tile was required above");
            validate_script_map_runtime_tile_alignment(
                &format!("map_events[{index}]"),
                &event.source_script,
                event.command_index,
                expected_command,
                tile,
            )?;
        }
        ScriptMapRuntimeKind::LoadMap => {
            validate_map_load_command_payload(
                &format!("map_events[{index}]"),
                &event.command,
                event.map_setup.as_deref(),
            )?;
            if event.target_map.is_some() || event.tile.is_some() || event.facing.is_some() {
                return Err(format!(
                    "map_events[{index}] {:?} cannot carry warp payload",
                    event.kind
                ));
            }
        }
        ScriptMapRuntimeKind::RefreshMap => {
            validate_map_refresh_command_payload(
                &format!("map_events[{index}]"),
                &event.command,
                event.map_setup.as_deref(),
            )?;
            if event.target_map.is_some() || event.tile.is_some() || event.facing.is_some() {
                return Err(format!(
                    "map_events[{index}] {:?} cannot carry warp payload",
                    event.kind
                ));
            }
        }
    }
    Ok(())
}

fn validate_pending_script_warp_payload(warp: &ScriptWarpRequest) -> Result<(), String> {
    let command = if warp.facing.is_some() {
        "warpfacing"
    } else {
        "warp"
    };
    validate_script_map_runtime_tile_alignment(
        "pending_script_warp",
        &warp.source_script,
        warp.command_index,
        command,
        warp.tile,
    )
}

fn validate_script_map_runtime_tile_alignment(
    path: &str,
    source_script: &str,
    command_index: usize,
    command: &str,
    tile: TilePosition,
) -> Result<(), String> {
    if runtime_tile_to_raw_event_tile(tile).is_none() {
        return Err(format!(
            "{path} {source_script}:{command_index} command {command} tile ({}, {}) is not aligned to a raw map event coordinate",
            tile.x, tile.y
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum ScriptMapRuntimeCommandError {
    #[error(
        "saved {path} {source_script}:{command_index} command {command} has incomplete destination"
    )]
    IncompleteDestination {
        path: String,
        source_script: String,
        command_index: usize,
        command: String,
    },
    #[error(
        "saved {path} {source_script}:{command_index} command {command} has unexpected map payload"
    )]
    UnexpectedMapPayload {
        path: String,
        source_script: String,
        command_index: usize,
        command: String,
    },
    #[error(
        "saved {path} {source_script}:{command_index} command {command} has unexpected facing or map_setup"
    )]
    UnexpectedWarpPayload {
        path: String,
        source_script: String,
        command_index: usize,
        command: String,
    },
    #[error("saved {path} {source_script}:{command_index} command {command} is missing target_map")]
    MissingTargetMap {
        path: String,
        source_script: String,
        command_index: usize,
        command: String,
    },
    #[error("saved {path} {source_script}:{command_index} command {command} is missing tile")]
    MissingTile {
        path: String,
        source_script: String,
        command_index: usize,
        command: String,
    },
    #[error("saved {path} {source_script}:{command_index} command {command} is missing facing")]
    MissingFacing {
        path: String,
        source_script: String,
        command_index: usize,
        command: String,
    },
    #[error(
        "saved {path} {source_script}:{command_index} command {command} tile ({x}, {y}) is not aligned to a raw map event coordinate"
    )]
    UnsavableTile {
        path: String,
        source_script: String,
        command_index: usize,
        command: String,
        x: i16,
        y: i16,
    },
}

pub fn saved_map_runtime_event_command_args(
    path: &str,
    event: &ScriptMapRuntimeEvent,
) -> Result<Option<Vec<String>>, ScriptMapRuntimeCommandError> {
    match event.command.as_str() {
        "warp" => {
            if event.facing.is_some() || event.map_setup.is_some() {
                return Err(map_command_error(
                    path,
                    event,
                    ScriptMapRuntimeCommandErrorKind::UnexpectedWarpPayload,
                ));
            }
            match (&event.target_map, event.tile) {
                (Some(target_map), Some(tile)) => {
                    let raw_tile = saved_raw_event_tile(path, event, tile)?;
                    Ok(Some(vec![
                        target_map.clone(),
                        raw_tile.x.to_string(),
                        raw_tile.y.to_string(),
                    ]))
                }
                (None, None) => Ok(Some(vec![
                    "NONE".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                ])),
                _ => Err(map_command_error(
                    path,
                    event,
                    ScriptMapRuntimeCommandErrorKind::IncompleteDestination,
                )),
            }
        }
        "warpfacing" => {
            if event.map_setup.is_some() {
                return Err(map_command_error(
                    path,
                    event,
                    ScriptMapRuntimeCommandErrorKind::UnexpectedWarpPayload,
                ));
            }
            let target_map = event.target_map.clone().ok_or_else(|| {
                map_command_error(
                    path,
                    event,
                    ScriptMapRuntimeCommandErrorKind::MissingTargetMap,
                )
            })?;
            let tile = event.tile.ok_or_else(|| {
                map_command_error(path, event, ScriptMapRuntimeCommandErrorKind::MissingTile)
            })?;
            let raw_tile = saved_raw_event_tile(path, event, tile)?;
            let facing = event.facing.ok_or_else(|| {
                map_command_error(path, event, ScriptMapRuntimeCommandErrorKind::MissingFacing)
            })?;
            Ok(Some(vec![
                target_map,
                raw_tile.x.to_string(),
                raw_tile.y.to_string(),
                direction_script_token(facing).to_string(),
            ]))
        }
        "newloadmap" | "reanchormap" => Ok(Some(saved_optional_map_setup_arg(
            event.map_setup.as_deref(),
        ))),
        "warpcheck" | "reloadmap" | "reloadmappart" | "reloadmapafterbattle" | "refreshmap" => {
            if event.target_map.is_some()
                || event.tile.is_some()
                || event.facing.is_some()
                || event.map_setup.is_some()
            {
                Err(map_command_error(
                    path,
                    event,
                    ScriptMapRuntimeCommandErrorKind::UnexpectedMapPayload,
                ))
            } else {
                Ok(Some(Vec::new()))
            }
        }
        _ => Ok(None),
    }
}

enum ScriptMapRuntimeCommandErrorKind {
    IncompleteDestination,
    UnexpectedMapPayload,
    UnexpectedWarpPayload,
    MissingTargetMap,
    MissingTile,
    MissingFacing,
    UnsavableTile,
}

fn map_command_error(
    path: &str,
    event: &ScriptMapRuntimeEvent,
    kind: ScriptMapRuntimeCommandErrorKind,
) -> ScriptMapRuntimeCommandError {
    match kind {
        ScriptMapRuntimeCommandErrorKind::IncompleteDestination => {
            ScriptMapRuntimeCommandError::IncompleteDestination {
                path: path.to_string(),
                source_script: event.source_script.clone(),
                command_index: event.command_index,
                command: event.command.clone(),
            }
        }
        ScriptMapRuntimeCommandErrorKind::UnexpectedMapPayload => {
            ScriptMapRuntimeCommandError::UnexpectedMapPayload {
                path: path.to_string(),
                source_script: event.source_script.clone(),
                command_index: event.command_index,
                command: event.command.clone(),
            }
        }
        ScriptMapRuntimeCommandErrorKind::UnexpectedWarpPayload => {
            ScriptMapRuntimeCommandError::UnexpectedWarpPayload {
                path: path.to_string(),
                source_script: event.source_script.clone(),
                command_index: event.command_index,
                command: event.command.clone(),
            }
        }
        ScriptMapRuntimeCommandErrorKind::MissingTargetMap => {
            ScriptMapRuntimeCommandError::MissingTargetMap {
                path: path.to_string(),
                source_script: event.source_script.clone(),
                command_index: event.command_index,
                command: event.command.clone(),
            }
        }
        ScriptMapRuntimeCommandErrorKind::MissingTile => {
            ScriptMapRuntimeCommandError::MissingTile {
                path: path.to_string(),
                source_script: event.source_script.clone(),
                command_index: event.command_index,
                command: event.command.clone(),
            }
        }
        ScriptMapRuntimeCommandErrorKind::MissingFacing => {
            ScriptMapRuntimeCommandError::MissingFacing {
                path: path.to_string(),
                source_script: event.source_script.clone(),
                command_index: event.command_index,
                command: event.command.clone(),
            }
        }
        ScriptMapRuntimeCommandErrorKind::UnsavableTile => {
            let tile = event
                .tile
                .expect("unsaveable map event tile error requires a tile");
            ScriptMapRuntimeCommandError::UnsavableTile {
                path: path.to_string(),
                source_script: event.source_script.clone(),
                command_index: event.command_index,
                command: event.command.clone(),
                x: tile.x,
                y: tile.y,
            }
        }
    }
}

fn saved_raw_event_tile(
    path: &str,
    event: &ScriptMapRuntimeEvent,
    tile: TilePosition,
) -> Result<TilePosition, ScriptMapRuntimeCommandError> {
    runtime_tile_to_raw_event_tile(tile).ok_or_else(|| {
        map_command_error(path, event, ScriptMapRuntimeCommandErrorKind::UnsavableTile)
    })
}

fn saved_optional_map_setup_arg(map_setup: Option<&str>) -> Vec<String> {
    map_setup.iter().map(|value| (*value).to_string()).collect()
}

pub fn saved_map_load_command_payload(load: &ScriptMapLoadRequest) -> (&str, Vec<String>) {
    (
        &load.command,
        saved_optional_map_setup_arg(load.map_setup.as_deref()),
    )
}

pub fn saved_map_refresh_command_payload(refresh: &ScriptMapRefreshRequest) -> (&str, Vec<String>) {
    (
        &refresh.command,
        saved_optional_map_setup_arg(refresh.map_setup.as_deref()),
    )
}

pub fn saved_script_warp_command_payload(
    path: &str,
    warp: &ScriptWarpRequest,
) -> Result<(&'static str, Vec<String>), ScriptMapRuntimeCommandError> {
    let raw_tile = runtime_tile_to_raw_event_tile(warp.tile).ok_or_else(|| {
        ScriptMapRuntimeCommandError::UnsavableTile {
            path: path.to_string(),
            source_script: warp.source_script.clone(),
            command_index: warp.command_index,
            command: if warp.facing.is_some() {
                "warpfacing".to_string()
            } else {
                "warp".to_string()
            },
            x: warp.tile.x,
            y: warp.tile.y,
        }
    })?;
    let mut args = vec![
        warp.target_map.clone(),
        raw_tile.x.to_string(),
        raw_tile.y.to_string(),
    ];
    if let Some(facing) = warp.facing {
        args.push(direction_script_token(facing).to_string());
        Ok(("warpfacing", args))
    } else {
        Ok(("warp", args))
    }
}

fn direction_script_token(direction: Direction) -> &'static str {
    match direction {
        Direction::Down => "DOWN",
        Direction::Up => "UP",
        Direction::Left => "LEFT",
        Direction::Right => "RIGHT",
    }
}

fn validate_pending_map_load_payload(load: &ScriptMapLoadRequest) -> Result<(), String> {
    validate_map_load_command_payload("pending_map_load", &load.command, load.map_setup.as_deref())
}

fn validate_pending_map_refresh_payload(refresh: &ScriptMapRefreshRequest) -> Result<(), String> {
    validate_map_refresh_command_payload(
        "pending_map_refresh",
        &refresh.command,
        refresh.map_setup.as_deref(),
    )
}

fn validate_map_load_command_payload(
    field: &str,
    command: &str,
    map_setup: Option<&str>,
) -> Result<(), String> {
    if !SCRIPT_MAP_LOAD_COMMANDS.contains(&command) {
        return Err(format!(
            "{field}.command {command} is not a saved map load command"
        ));
    }
    if command == "newloadmap" {
        if map_setup.is_none() {
            return Err(format!("{field}.map_setup is required for newloadmap"));
        }
    } else if map_setup.is_some() {
        return Err(format!("{field}.map_setup is not valid for {command}"));
    }
    Ok(())
}

fn validate_map_refresh_command_payload(
    field: &str,
    command: &str,
    map_setup: Option<&str>,
) -> Result<(), String> {
    if !SCRIPT_MAP_REFRESH_COMMANDS.contains(&command) {
        return Err(format!(
            "{field}.command {command} is not a saved map refresh command"
        ));
    }
    if command != "reanchormap" && map_setup.is_some() {
        return Err(format!("{field}.map_setup is not valid for {command}"));
    }
    Ok(())
}

fn validate_control_event_payload(
    index: usize,
    event: &ScriptControlRuntimeEvent,
) -> Result<(), String> {
    match event.kind {
        ScriptControlRuntimeKind::Continue | ScriptControlRuntimeKind::End => {
            if event.target_script.is_some() {
                return Err(format!(
                    "control_events[{index}].target_script is not valid for {:?}",
                    event.kind
                ));
            }
        }
        ScriptControlRuntimeKind::Jump
        | ScriptControlRuntimeKind::Call
        | ScriptControlRuntimeKind::Defer
        | ScriptControlRuntimeKind::StandardJump => {
            if event.target_script.is_none() {
                return Err(format!(
                    "control_events[{index}].target_script is required for {:?}",
                    event.kind
                ));
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum ScriptControlRuntimeCommandError {
    #[error(
        "saved {path} {source_script}:{command_index} continued control event has unexpected target_script"
    )]
    UnexpectedContinueTarget {
        path: String,
        source_script: String,
        command_index: usize,
    },
    #[error(
        "saved {path} {source_script}:{command_index} end control event has unexpected target_script"
    )]
    UnexpectedEndTarget {
        path: String,
        source_script: String,
        command_index: usize,
    },
    #[error("saved {path} {source_script}:{command_index} control event is missing target_script")]
    MissingTarget {
        path: String,
        source_script: String,
        command_index: usize,
    },
}

pub fn validate_saved_control_runtime_event_shape(
    path: &str,
    event: &ScriptControlRuntimeEvent,
) -> Result<(), ScriptControlRuntimeCommandError> {
    match event.kind {
        ScriptControlRuntimeKind::Continue => {
            if event.target_script.is_some() {
                return Err(ScriptControlRuntimeCommandError::UnexpectedContinueTarget {
                    path: path.to_string(),
                    source_script: event.source_script.clone(),
                    command_index: event.command_index,
                });
            }
        }
        ScriptControlRuntimeKind::Jump
        | ScriptControlRuntimeKind::Call
        | ScriptControlRuntimeKind::Defer
        | ScriptControlRuntimeKind::StandardJump => {
            if event.target_script.is_none() {
                return Err(ScriptControlRuntimeCommandError::MissingTarget {
                    path: path.to_string(),
                    source_script: event.source_script.clone(),
                    command_index: event.command_index,
                });
            }
        }
        ScriptControlRuntimeKind::End => {
            if event.target_script.is_some() {
                return Err(ScriptControlRuntimeCommandError::UnexpectedEndTarget {
                    path: path.to_string(),
                    source_script: event.source_script.clone(),
                    command_index: event.command_index,
                });
            }
        }
    }
    Ok(())
}

fn validate_return_frame_payload(index: usize, frame: &ScriptReturnFrame) -> Result<(), String> {
    if frame.next_command_index == 0 {
        return Err(format!(
            "call_stack[{index}].next_command_index cannot be 0"
        ));
    }
    Ok(())
}

fn validate_script_end_state(end: &ScriptEndState) -> Result<(), String> {
    if end.callback && end.just_battled_guard {
        return Err("script_ended cannot be both callback and just_battled_guard".to_string());
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum ScriptEndCommandError {
    #[error(
        "saved script_runtime.script_ended.source_script {source_script}:{command_index} cannot be both callback and just_battled_guard"
    )]
    CallbackAndJustBattledGuard {
        source_script: String,
        command_index: usize,
    },
}

pub fn saved_script_end_command(
    end: &ScriptEndState,
) -> Result<&'static str, ScriptEndCommandError> {
    match (end.callback, end.just_battled_guard) {
        (false, false) => Ok("end"),
        (true, false) => Ok("endcallback"),
        (false, true) => Ok("endifjustbattled"),
        (true, true) => Err(ScriptEndCommandError::CallbackAndJustBattledGuard {
            source_script: end.source_script.clone(),
            command_index: end.command_index,
        }),
    }
}

fn validate_variable_write_payload(
    index: usize,
    write: &ScriptRuntimeVariableWrite,
    variables: &BTreeMap<String, String>,
) -> Result<(), String> {
    match variables.get(&write.target) {
        Some(value) if value == &write.value => Ok(()),
        Some(value) => Err(format!(
            "variable_writes[{index}].value {} does not match variables[{}] {}",
            write.value, write.target, value
        )),
        None => Err(format!(
            "variable_writes[{index}].target {} is missing from variables",
            write.target
        )),
    }
}

fn validate_runtime_effect_payload(
    index: usize,
    effect: &ScriptRuntimeEffect,
) -> Result<(), String> {
    let expected = script_runtime_command_arg_counts()
        .get(effect.command.as_str())
        .copied()
        .ok_or_else(|| {
            format!(
                "effects[{index}].command {} is not a saved runtime command",
                effect.command
            )
        })?;
    if effect.args.len() != expected {
        return Err(format!(
            "effects[{index}].args has {} entries, expected {expected} for {}",
            effect.args.len(),
            effect.command
        ));
    }
    Ok(())
}

fn validate_queued_command_payload(
    index: usize,
    command: &ScriptRuntimeQueuedCommand,
) -> Result<(), String> {
    match command.command.as_str() {
        "cmdqueue" | "conditional_event" => {
            if command.bank.is_none() {
                return Err(format!(
                    "command_queue[{index}].bank is required for {}",
                    command.command
                ));
            }
        }
        "writecmdqueue" | "elevator" | "callasm" | "checkpokemail" | "givepokemail" => {
            if command.bank.is_some() {
                return Err(format!(
                    "command_queue[{index}].bank is not valid for {}",
                    command.command
                ));
            }
        }
        _ => {
            return Err(format!(
                "command_queue[{index}].command {} is not a saved queued command",
                command.command
            ));
        }
    }
    Ok(())
}

pub fn saved_queued_command_args(command: &ScriptRuntimeQueuedCommand) -> Vec<String> {
    match &command.bank {
        Some(bank) => vec![bank.clone(), command.target.clone()],
        None => vec![command.target.clone()],
    }
}

fn validate_text_event_payload(index: usize, event: &ScriptTextRuntimeEvent) -> Result<(), String> {
    match event.kind {
        ScriptTextRuntimeKind::Write => {
            if event.source_script == "ReceiveItemScript"
                && event.command == "standard_receive_item"
                && event.command_index == 0
                && event.text_label.is_none()
                && !event.face_player
                && event.closes_text
            {
                return Ok(());
            }
            if event.text_label.is_none() {
                return Err(format!(
                    "text_events[{index}].text_label is required for Write"
                ));
            }
            if !SCRIPT_TEXT_LABEL_COMMANDS.contains(&event.command.as_str()) {
                return Err(format!(
                    "text_events[{index}].command {} is not valid for Write",
                    event.command
                ));
            }
            match event.command.as_str() {
                "writetext" | "farwritetext" => {
                    if event.face_player {
                        return Err(format!(
                            "text_events[{index}].face_player must be false for {}",
                            event.command
                        ));
                    }
                    if event.closes_text {
                        return Err(format!(
                            "text_events[{index}].closes_text must be false for {}",
                            event.command
                        ));
                    }
                }
                "jumptext" | "farjumptext" => {
                    if event.face_player {
                        return Err(format!(
                            "text_events[{index}].face_player must be false for {}",
                            event.command
                        ));
                    }
                    if !event.closes_text {
                        return Err(format!(
                            "text_events[{index}].closes_text must be true for {}",
                            event.command
                        ));
                    }
                }
                "jumptextfaceplayer" => {
                    if !event.face_player {
                        return Err(format!(
                            "text_events[{index}].face_player must be true for jumptextfaceplayer"
                        ));
                    }
                    if !event.closes_text {
                        return Err(format!(
                            "text_events[{index}].closes_text must be true for jumptextfaceplayer"
                        ));
                    }
                }
                _ => {}
            }
        }
        ScriptTextRuntimeKind::Open => {
            if event.text_label.is_some() {
                return Err(format!(
                    "text_events[{index}].text_label is not valid for {:?}",
                    event.kind
                ));
            }
            if event.face_player {
                return Err(format!(
                    "text_events[{index}].face_player is not valid for {:?}",
                    event.kind
                ));
            }
            if event.closes_text {
                return Err(format!(
                    "text_events[{index}].closes_text is not valid for {:?}",
                    event.kind
                ));
            }
            if event.command != "opentext" {
                return Err(format!(
                    "text_events[{index}].command {} is not valid for Open",
                    event.command
                ));
            }
        }
        ScriptTextRuntimeKind::Close => {
            if event.text_label.is_some() {
                return Err(format!(
                    "text_events[{index}].text_label is not valid for {:?}",
                    event.kind
                ));
            }
            if event.face_player {
                return Err(format!(
                    "text_events[{index}].face_player is not valid for {:?}",
                    event.kind
                ));
            }
            if event.closes_text {
                return Err(format!(
                    "text_events[{index}].closes_text is not valid for {:?}",
                    event.kind
                ));
            }
            if event.command != "closetext" {
                return Err(format!(
                    "text_events[{index}].command {} is not valid for Close",
                    event.command
                ));
            }
        }
        ScriptTextRuntimeKind::WaitButton => {
            if event.text_label.is_some() {
                return Err(format!(
                    "text_events[{index}].text_label is not valid for {:?}",
                    event.kind
                ));
            }
            if event.face_player {
                return Err(format!(
                    "text_events[{index}].face_player is not valid for {:?}",
                    event.kind
                ));
            }
            if event.closes_text {
                return Err(format!(
                    "text_events[{index}].closes_text is not valid for {:?}",
                    event.kind
                ));
            }
            if !matches!(event.command.as_str(), "promptbutton" | "waitbutton") {
                return Err(format!(
                    "text_events[{index}].command {} is not valid for WaitButton",
                    event.command
                ));
            }
        }
        ScriptTextRuntimeKind::YesNo => {
            if event.text_label.is_some() {
                return Err(format!(
                    "text_events[{index}].text_label is not valid for {:?}",
                    event.kind
                ));
            }
            if event.face_player {
                return Err(format!(
                    "text_events[{index}].face_player is not valid for {:?}",
                    event.kind
                ));
            }
            if event.closes_text {
                return Err(format!(
                    "text_events[{index}].closes_text is not valid for {:?}",
                    event.kind
                ));
            }
            if event.command != "yesorno" {
                return Err(format!(
                    "text_events[{index}].command {} is not valid for YesNo",
                    event.command
                ));
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum ScriptTextRuntimeCommandError {
    #[error(
        "saved {path} {source_script}:{command_index} command {command} has kind {actual:?}, expected {expected:?}"
    )]
    KindMismatch {
        path: String,
        source_script: String,
        command_index: usize,
        command: String,
        actual: ScriptTextRuntimeKind,
        expected: ScriptTextRuntimeKind,
    },
    #[error(
        "saved {path} {source_script}:{command_index} command {command} has face_player {actual}, expected {expected}"
    )]
    FacePlayerMismatch {
        path: String,
        source_script: String,
        command_index: usize,
        command: String,
        actual: bool,
        expected: bool,
    },
    #[error(
        "saved {path} {source_script}:{command_index} command {command} has closes_text {actual}, expected {expected}"
    )]
    ClosesTextMismatch {
        path: String,
        source_script: String,
        command_index: usize,
        command: String,
        actual: bool,
        expected: bool,
    },
    #[error("saved {path} {source_script}:{command_index} command {command} is missing text_label")]
    MissingTextLabel {
        path: String,
        source_script: String,
        command_index: usize,
        command: String,
    },
    #[error(
        "saved {path} {source_script}:{command_index} command {command} has unexpected text_label"
    )]
    UnexpectedTextLabel {
        path: String,
        source_script: String,
        command_index: usize,
        command: String,
    },
}

pub fn saved_text_runtime_event_command_args(
    path: &str,
    event: &ScriptTextRuntimeEvent,
) -> Result<Option<Vec<String>>, ScriptTextRuntimeCommandError> {
    let Some((expected_kind, expected_face_player, expected_closes_text, needs_label)) =
        text_runtime_command_shape(&event.command)
    else {
        return Ok(None);
    };
    if event.kind != expected_kind {
        return Err(ScriptTextRuntimeCommandError::KindMismatch {
            path: path.to_string(),
            source_script: event.source_script.clone(),
            command_index: event.command_index,
            command: event.command.clone(),
            actual: event.kind,
            expected: expected_kind,
        });
    }
    if event.face_player != expected_face_player {
        return Err(ScriptTextRuntimeCommandError::FacePlayerMismatch {
            path: path.to_string(),
            source_script: event.source_script.clone(),
            command_index: event.command_index,
            command: event.command.clone(),
            actual: event.face_player,
            expected: expected_face_player,
        });
    }
    if event.closes_text != expected_closes_text {
        return Err(ScriptTextRuntimeCommandError::ClosesTextMismatch {
            path: path.to_string(),
            source_script: event.source_script.clone(),
            command_index: event.command_index,
            command: event.command.clone(),
            actual: event.closes_text,
            expected: expected_closes_text,
        });
    }
    if needs_label {
        let Some(text_label) = &event.text_label else {
            return Err(ScriptTextRuntimeCommandError::MissingTextLabel {
                path: path.to_string(),
                source_script: event.source_script.clone(),
                command_index: event.command_index,
                command: event.command.clone(),
            });
        };
        Ok(Some(vec![text_label.clone()]))
    } else if event.text_label.is_some() {
        Err(ScriptTextRuntimeCommandError::UnexpectedTextLabel {
            path: path.to_string(),
            source_script: event.source_script.clone(),
            command_index: event.command_index,
            command: event.command.clone(),
        })
    } else {
        Ok(Some(Vec::new()))
    }
}

fn text_runtime_command_shape(command: &str) -> Option<(ScriptTextRuntimeKind, bool, bool, bool)> {
    match command {
        "opentext" => Some((ScriptTextRuntimeKind::Open, false, false, false)),
        "closetext" => Some((ScriptTextRuntimeKind::Close, false, false, false)),
        "promptbutton" | "waitbutton" => {
            Some((ScriptTextRuntimeKind::WaitButton, false, false, false))
        }
        "yesorno" => Some((ScriptTextRuntimeKind::YesNo, false, false, false)),
        "writetext" | "farwritetext" => Some((ScriptTextRuntimeKind::Write, false, false, true)),
        "jumptext" => Some((ScriptTextRuntimeKind::Write, false, true, true)),
        "farjumptext" => Some((ScriptTextRuntimeKind::Write, false, true, true)),
        "jumptextfaceplayer" => Some((ScriptTextRuntimeKind::Write, true, true, true)),
        _ => None,
    }
}

fn validate_pending_text_wait_command(command: &str) -> Result<(), String> {
    if matches!(
        command,
        "promptbutton"
            | "waitbutton"
            | "jumptext"
            | "jumptextfaceplayer"
            | "farjumptext"
            | "standard_receive_item"
    ) {
        Ok(())
    } else {
        Err(format!(
            "pending_text_wait.command {command} is not a saved text wait command"
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum ScriptTextWaitCommandError {
    #[error(
        "saved {path} {source_script}:{command_index} command {command} requires pending_text_label"
    )]
    MissingPendingTextLabel {
        path: String,
        source_script: String,
        command_index: usize,
        command: String,
    },
}

pub fn saved_pending_text_wait_command_args(
    path: &str,
    wait: &ScriptTextWait,
    pending_text_label: Option<&str>,
) -> Result<Option<Vec<String>>, ScriptTextWaitCommandError> {
    match wait.command.as_str() {
        "promptbutton" | "waitbutton" => Ok(Some(Vec::new())),
        "jumptext" | "jumptextfaceplayer" | "farjumptext" => {
            let Some(text_label) = pending_text_label else {
                return Err(ScriptTextWaitCommandError::MissingPendingTextLabel {
                    path: path.to_string(),
                    source_script: wait.source_script.clone(),
                    command_index: wait.command_index,
                    command: wait.command.clone(),
                });
            };
            Ok(Some(vec![text_label.to_string()]))
        }
        _ => Ok(None),
    }
}

fn validate_optional_script_runtime_label(field: &str, value: Option<&str>) -> Result<(), String> {
    if let Some(value) = value {
        validate_script_runtime_label(field, value)?;
    }
    Ok(())
}

fn validate_script_runtime_label(field: &str, value: &str) -> Result<(), String> {
    if is_exact_script_runtime_label(value) {
        Ok(())
    } else {
        Err(format!("{field} has invalid script label '{value}'"))
    }
}

fn validate_optional_script_runtime_token(field: &str, value: Option<&str>) -> Result<(), String> {
    if let Some(value) = value {
        validate_script_runtime_token(field, value)?;
    }
    Ok(())
}

fn validate_empty_or_script_runtime_token(field: &str, value: &str) -> Result<(), String> {
    if value.is_empty() {
        Ok(())
    } else {
        validate_script_runtime_token(field, value)
    }
}

fn validate_empty_or_exact_text(field: &str, value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Ok(());
    }
    if value.trim() != value || value.chars().any(char::is_control) {
        return Err(format!(
            "{field} must be exact, untrimmed, and contain no control characters"
        ));
    }
    Ok(())
}

fn validate_inches_field(field: &str, value: u8) -> Result<(), String> {
    if value >= 12 {
        return Err(format!("{field} {value} is outside inches range 0..11"));
    }
    Ok(())
}

fn validate_script_runtime_token(field: &str, value: &str) -> Result<(), String> {
    if !is_exact_script_runtime_token(value) {
        return Err(format!("{field} has invalid token '{value}'"));
    }
    if has_reserved_runtime_token_prefix(value) {
        return Err(format!(
            "{field} token '{value}' uses reserved runtime payload prefix"
        ));
    }
    Ok(())
}

fn is_exact_script_runtime_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn has_reserved_runtime_token_prefix(value: &str) -> bool {
    let lowered = value.to_ascii_lowercase();
    lowered.starts_with("fallback") || lowered.starts_with("legacy")
}

fn is_exact_script_runtime_label(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !has_reserved_runtime_token_prefix(value)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'@'))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ItemUseRuntimeEvent {
    pub item_id: String,
    pub context: String,
    pub consumed: bool,
}

impl<'de> Deserialize<'de> for ItemUseRuntimeEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawItemUseRuntimeEvent {
            item_id: String,
            context: String,
            consumed: bool,
        }

        let raw = RawItemUseRuntimeEvent::deserialize(deserializer)?;
        let event = Self {
            item_id: raw.item_id,
            context: raw.context,
            consumed: raw.consumed,
        };
        validate_script_runtime_token("item_use_event.item_id", &event.item_id)
            .map_err(D::Error::custom)?;
        validate_script_runtime_token("item_use_event.context", &event.context)
            .map_err(D::Error::custom)?;
        validate_item_use_event_context(0, &event.context).map_err(D::Error::custom)?;
        Ok(event)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeEffect {
    pub command: String,
    pub args: Vec<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeVariableWrite {
    pub target: String,
    pub value: String,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeAsmDirective {
    pub command: String,
    pub args: Vec<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeNumericBufferWrite {
    pub target_buffer: String,
    pub value: String,
    pub width: u8,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeElevatorFloor {
    pub floor: String,
    pub warp: u16,
    pub target_map: String,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeStoneTableEntry {
    pub warp: u16,
    pub object_event: String,
    pub script: String,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeDecorationDescription {
    pub decoration: String,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptAudioRuntimeEvent {
    pub command: String,
    pub kind: ScriptAudioRuntimeKind,
    pub audio_id: Option<String>,
    pub fade_frames: Option<u16>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ScriptAudioRuntimeKind {
    Music,
    SoundEffect,
    Cry,
    FadeMusic,
    WaitForSoundEffect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptMusicFade {
    pub audio_id: String,
    pub fade_frames: u16,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptGraphicsRuntimeEvent {
    pub command: String,
    pub kind: ScriptGraphicsRuntimeKind,
    pub color: Option<ScriptFadeColor>,
    pub direction: Option<ScriptFadeDirection>,
    pub frames: Option<u16>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ScriptGraphicsRuntimeKind {
    ScreenFade,
    ClearBgPalettesBufferScreen,
    ClearBgPalettes,
    UpdateTimePals,
    ClearTilemap,
    LoadMapPalettes,
    RefreshSprites,
    UpdateSprites,
    ReloadSpritesNoPalettes,
    BattleTowerFade,
    UpdatePlayerSprite,
    HealMachineAnim,
    SurfStartStep,
    LoadUsedSpritesGfx,
    ToggleMaptileDecorations,
    ToggleDecorationsVisibility,
    MagnetTrain,
    Diploma,
    PrintDiploma,
    UnownPuzzle,
    OmanyteChamber,
    DisplayUnownWords,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptScreenFade {
    pub color: ScriptFadeColor,
    pub direction: ScriptFadeDirection,
    pub frames: u16,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ScriptFadeColor {
    White,
    Black,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ScriptFadeDirection {
    Out,
    In,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptMoneyRuntimeEvent {
    pub command: String,
    pub kind: ScriptMoneyRuntimeKind,
    pub money: u32,
    pub coins: Option<u16>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ScriptMoneyRuntimeKind {
    PlaceMoneyTopRight,
    DisplayMoneyAndCoinBalance,
    DisplayCoinCaseBalance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptMapRuntimeEvent {
    pub command: String,
    pub kind: ScriptMapRuntimeKind,
    pub target_map: Option<String>,
    pub tile: Option<TilePosition>,
    pub facing: Option<Direction>,
    pub map_setup: Option<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ScriptMapRuntimeKind {
    NoWarp,
    Warp,
    WarpCheck,
    LoadMap,
    RefreshMap,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptWarpRequest {
    pub target_map: String,
    pub tile: TilePosition,
    pub facing: Option<Direction>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptMapLoadRequest {
    pub command: String,
    pub map_setup: Option<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptMapRefreshRequest {
    pub command: String,
    pub map_setup: Option<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptTextRuntimeEvent {
    pub command: String,
    pub kind: ScriptTextRuntimeKind,
    pub text_label: Option<String>,
    pub face_player: bool,
    pub closes_text: bool,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ScriptTextRuntimeKind {
    Open,
    Close,
    WaitButton,
    YesNo,
    Write,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptTextWait {
    pub command: String,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptYesNoPrompt {
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptControlRuntimeEvent {
    pub kind: ScriptControlRuntimeKind,
    pub target_script: Option<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ScriptControlRuntimeKind {
    Continue,
    Jump,
    Call,
    Defer,
    StandardJump,
    End,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptReturnFrame {
    pub origin_map_name: String,
    pub source_script: String,
    pub next_command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptLocation {
    pub origin_map_name: String,
    pub script: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptEndState {
    pub callback: bool,
    pub just_battled_guard: bool,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptShopRuntimeEvent {
    pub mart_type: String,
    pub mart_id: String,
    pub inventory: Vec<String>,
    pub source_script: String,
    pub command_index: usize,
}

impl<'de> Deserialize<'de> for ScriptShopRuntimeEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawScriptShopRuntimeEvent {
            mart_type: String,
            mart_id: String,
            inventory: Vec<String>,
            source_script: String,
            command_index: usize,
        }

        let raw = RawScriptShopRuntimeEvent::deserialize(deserializer)?;
        let event = Self {
            mart_type: raw.mart_type,
            mart_id: raw.mart_id,
            inventory: raw.inventory,
            source_script: raw.source_script,
            command_index: raw.command_index,
        };
        validate_script_shop_runtime_event("shop_event", &event).map_err(D::Error::custom)?;
        Ok(event)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptShopRequest {
    pub mart_type: String,
    pub mart_id: String,
    pub inventory: Vec<String>,
    pub source_script: String,
    pub command_index: usize,
}

impl<'de> Deserialize<'de> for ScriptShopRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawScriptShopRequest {
            mart_type: String,
            mart_id: String,
            inventory: Vec<String>,
            source_script: String,
            command_index: usize,
        }

        let raw = RawScriptShopRequest::deserialize(deserializer)?;
        let request = Self {
            mart_type: raw.mart_type,
            mart_id: raw.mart_id,
            inventory: raw.inventory,
            source_script: raw.source_script,
            command_index: raw.command_index,
        };
        validate_script_shop_request("shop_request", &request).map_err(D::Error::custom)?;
        Ok(request)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeDelay {
    pub command: String,
    pub frames: u16,
    pub source_script: String,
    pub command_index: usize,
}

impl<'de> Deserialize<'de> for ScriptRuntimeDelay {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawScriptRuntimeDelay {
            command: String,
            frames: u16,
            source_script: String,
            command_index: usize,
        }

        let raw = RawScriptRuntimeDelay::deserialize(deserializer)?;
        let delay = Self {
            command: raw.command,
            frames: raw.frames,
            source_script: raw.source_script,
            command_index: raw.command_index,
        };
        validate_script_runtime_token("pending_delay.command", &delay.command)
            .map_err(D::Error::custom)?;
        validate_script_runtime_label("pending_delay.source_script", &delay.source_script)
            .map_err(D::Error::custom)?;
        validate_delay_payload(0, &delay).map_err(D::Error::custom)?;
        Ok(delay)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeEarthquake {
    pub parameter: u16,
    pub shake_frames: u16,
    pub sleep_frames: u16,
    pub source_script: String,
    pub command_index: usize,
}

impl<'de> Deserialize<'de> for ScriptRuntimeEarthquake {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawScriptRuntimeEarthquake {
            parameter: u16,
            shake_frames: u16,
            sleep_frames: u16,
            source_script: String,
            command_index: usize,
        }

        let raw = RawScriptRuntimeEarthquake::deserialize(deserializer)?;
        let earthquake = Self {
            parameter: raw.parameter,
            shake_frames: raw.shake_frames,
            sleep_frames: raw.sleep_frames,
            source_script: raw.source_script,
            command_index: raw.command_index,
        };
        validate_script_runtime_label(
            "pending_earthquake.source_script",
            &earthquake.source_script,
        )
        .map_err(D::Error::custom)?;
        validate_earthquake_payload(0, &earthquake).map_err(D::Error::custom)?;
        Ok(earthquake)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeEmote {
    pub emote: String,
    pub object: String,
    pub duration: u16,
    pub source_script: String,
    pub command_index: usize,
}

impl<'de> Deserialize<'de> for ScriptRuntimeEmote {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawScriptRuntimeEmote {
            emote: String,
            object: String,
            duration: u16,
            source_script: String,
            command_index: usize,
        }

        let raw = RawScriptRuntimeEmote::deserialize(deserializer)?;
        let emote = Self {
            emote: raw.emote,
            object: raw.object,
            duration: raw.duration,
            source_script: raw.source_script,
            command_index: raw.command_index,
        };
        validate_script_runtime_token("pending_emote.emote", &emote.emote)
            .map_err(D::Error::custom)?;
        validate_script_runtime_token("pending_emote.object", &emote.object)
            .map_err(D::Error::custom)?;
        validate_script_runtime_label("pending_emote.source_script", &emote.source_script)
            .map_err(D::Error::custom)?;
        Ok(emote)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeQueuedCommand {
    pub origin_map_name: String,
    pub command: String,
    pub target: String,
    pub bank: Option<String>,
    pub source_script: String,
    pub command_index: usize,
}

impl GameState {
    /// Apply the per-day state invalidation performed by Crystal's
    /// `CheckDailyResetTimer`.  The RTC and the daily timer are separate in
    /// the original, but both are driven by the same completed day boundary
    /// in the frame runtime.
    pub fn apply_daily_reset(&mut self) {
        self.fishing.daily_flags1 = 0;
        self.fishing.swarm_flag = 0;
        self.swarms.active.clear();
        self.apply_pokerus_tick(1);
        if self.kenji_break_timer > 0 {
            self.kenji_break_timer -= 1;
        }
        if self.kenji_break_timer == 0 {
            let mut rng = Random::new_crystal(self.rng_seed);
            self.kenji_break_timer = 3 + (rng.battle_random_byte() & 0x03);
            self.rng_seed = rng.seed();
        }
    }

    /// Apply Crystal's daily Pokérus tick to party and PC records. The
    /// strain (high nibble) is retained after the low-nibble day counter
    /// reaches zero, preventing a cured Pokémon from re-contracting.
    pub fn apply_pokerus_tick(&mut self, days: u8) {
        fn tick(pokemon: &mut Pokemon, days: u8) {
            let remaining = pokemon.pokerus & 0x0f;
            if remaining == 0 {
                return;
            }
            pokemon.pokerus = (pokemon.pokerus & 0xf0) | remaining.saturating_sub(days);
        }

        for pokemon in self.storage.party.pokemon.iter_mut().flatten() {
            tick(pokemon, days);
        }
        for pc_box in &mut self.storage.pc_boxes {
            for pokemon in pc_box.pokemon.iter_mut().flatten() {
                tick(pokemon, days);
            }
        }
        self.sync_party_from_storage();
    }

    pub fn validate_saved_state(&self) -> Result<(), String> {
        validate_saved_player_name(&self.player_name)?;
        validate_saved_player_gender(self.player_gender)?;
        if self.wild_encounter_cooldown > 5 {
            return Err(format!(
                "wild_encounter_cooldown {} is outside Crystal range 0..=5",
                self.wild_encounter_cooldown
            ));
        }
        self.bag
            .validate()
            .map_err(|error| format!("invalid saved bag: {error}"))?;
        self.storage
            .validate_metadata()
            .map_err(|error| format!("invalid saved storage: {error}"))?;
        for (location, pokemon) in self
            .storage
            .party
            .pokemon
            .iter()
            .enumerate()
            .filter_map(|(index, pokemon)| pokemon.as_ref().map(|pokemon| (format!("party[{index}]"), pokemon)))
            .chain(self.storage.pc_boxes.iter().enumerate().flat_map(|(box_index, pc_box)| {
                pc_box.pokemon.iter().enumerate().filter_map(move |(slot_index, pokemon)| {
                    pokemon.as_ref().map(|pokemon| (format!("pc_boxes[{box_index}][{slot_index}]"), pokemon))
                })
            }))
        {
            let holds_mail_item = pokemon.item.as_deref().is_some_and(crate::models::item::is_mail_item_id);
            if holds_mail_item != pokemon.mail.is_some() {
                return Err(format!(
                    "saved {location} must have both a Mail item and Mail data or neither"
                ));
            }
        }
        if self.mailbox.len() > MAILBOX_CAPACITY {
            return Err(format!(
                "saved mailbox contains {} messages, exceeding capacity {MAILBOX_CAPACITY}",
                self.mailbox.len()
            ));
        }
        for (index, entry) in self.mailbox.iter().enumerate() {
            validate_script_runtime_token(
                &format!("mailbox[{index}].item_id"),
                &entry.item_id,
            )?;
            if !crate::models::item::is_mail_item_id(&entry.item_id) {
                return Err(format!(
                    "mailbox[{index}].item_id '{}' is not an ASM Mail item",
                    entry.item_id
                ));
            }
            if entry.mail.message.is_empty() {
                return Err(format!("mailbox[{index}] has an empty message"));
            }
            if entry.mail.author.is_empty() {
                return Err(format!("mailbox[{index}] has an empty author"));
            }
            validate_script_runtime_token(
                &format!("mailbox[{index}].mail.species"),
                &entry.mail.species,
            )?;
        }
        self.time
            .validate_saved_state()
            .map_err(|error| format!("invalid saved time: {error}"))?;
        self.unused_two_day_timer
            .validate_saved_state()
            .map_err(|error| format!("invalid saved unused two-day timer: {error}"))?;
        let projected_party = PartyState::from_storage(&self.storage);
        if self.party != projected_party {
            return Err("saved party projection does not match authoritative storage".to_string());
        }
        for pending in self
            .pending_move_learn
            .iter()
            .chain(self.pending_move_learn_queue.iter())
        {
            pending
                .validate_saved_state()
                .map_err(|error| format!("invalid pending move learn: {error}"))?;
            let Some(Some(pokemon)) = self.storage.party.pokemon.get(pending.party_index) else {
                return Err(format!(
                    "pending move learn party index {} is not occupied",
                    pending.party_index
                ));
            };
            if pokemon.species.id != pending.species_id {
                return Err(format!(
                    "pending move learn species {} does not match party slot {} species {}",
                    pending.species_id, pending.party_index, pokemon.species.id
                ));
            }
            if pokemon.level != pending.level {
                return Err(format!(
                    "pending move learn level {} does not match party slot {} level {}",
                    pending.level, pending.party_index, pokemon.level
                ));
            }
            if pokemon
                .moves
                .iter()
                .any(|known| known.name == pending.learned_move.name)
            {
                return Err(format!(
                    "pending move learn {} is already known by party slot {}",
                    pending.learned_move.name, pending.party_index
                ));
            }
            if pokemon.moves.len() < 4 {
                return Err(format!(
                    "pending move learn for party slot {} requires a full move list",
                    pending.party_index
                ));
            }
        }
        if self.current_pc_box >= MAX_PC_BOXES {
            return Err(format!(
                "current_pc_box {} is outside PC box range 0..{}",
                self.current_pc_box, MAX_PC_BOXES
            ));
        }
        for (index, roamer) in self.roaming_pokemon.iter().enumerate() {
            roamer.validate_saved_state(index)?;
        }
        self.validate_saved_battle_cursors()?;
        self.validate_saved_identity_fields()?;
        self.validate_saved_battle_runtime_consistency()?;
        self.scenes
            .validate()
            .map_err(|error| format!("invalid saved scene memory: {error}"))?;
        self.flags
            .validate()
            .map_err(|error| format!("invalid saved event flags: {error}"))?;
        self.swarms
            .validate_saved_state()
            .map_err(|error| format!("invalid saved swarm memory: {error}"))?;
        self.fishing
            .validate_saved_state()
            .map_err(|error| format!("invalid saved fishing memory: {error}"))?;
        self.joypad
            .validate_saved_state()
            .map_err(|error| format!("invalid saved joypad memory: {error}"))?;
        self.script_runtime
            .validate()
            .map_err(|error| format!("invalid saved script runtime: {error}"))?;
        Ok(())
    }

    fn validate_saved_battle_cursors(&self) -> Result<(), String> {
        let Some(enemy_party_len) = self.battle.enemy_party_len() else {
            if self.script_runtime.active_battle_combat.is_some() {
                return Err(
                    "active_battle_combat cannot be saved without an active battle".to_string(),
                );
            }
            if self.battle_active_party_index.is_some() {
                return Err(
                    "battle_active_party_index cannot be saved without an active battle"
                        .to_string(),
                );
            }
            if self.battle_active_enemy_party_index.is_some() {
                return Err(
                    "battle_active_enemy_party_index cannot be saved without an active battle"
                        .to_string(),
                );
            }
            if !self.battle_rewarded_enemy_party_indices.is_empty() {
                return Err(
                    "battle_rewarded_enemy_party_indices cannot be saved without an active battle"
                        .to_string(),
                );
            }
            return Ok(());
        };

        if let Some(index) = self.battle_active_party_index {
            if index >= PARTY_SIZE {
                return Err(format!(
                    "battle_active_party_index {index} is outside party range 0..{PARTY_SIZE}"
                ));
            }
            if self.storage.party.pokemon[index].is_none() {
                return Err(format!(
                    "battle_active_party_index {index} points to empty party slot"
                ));
            }
        }

        if let Some(index) = self.battle_active_enemy_party_index {
            if index >= enemy_party_len {
                return Err(format!(
                    "battle_active_enemy_party_index {index} is outside enemy party range 0..{enemy_party_len}"
                ));
            }
            if let Some((enemy_party, enemy_pokemon)) = self.battle.enemy_party_and_current() {
                if enemy_party[index] != *enemy_pokemon {
                    return Err(format!(
                        "battle_active_enemy_party_index {index} does not match battle enemy_pokemon"
                    ));
                }
            }
        }
        for index in &self.battle_rewarded_enemy_party_indices {
            if *index >= enemy_party_len {
                return Err(format!(
                    "battle_rewarded_enemy_party_indices contains {index}, outside enemy party range 0..{enemy_party_len}"
                ));
            }
        }
        Ok(())
    }

    fn validate_saved_battle_runtime_consistency(&self) -> Result<(), String> {
        let Some(enemy_party_len) = self.battle.enemy_party_len() else {
            if self.battle_escape_attempts != 0 {
                return Err(
                    "battle_escape_attempts cannot be saved without an active battle".to_string(),
                );
            }
            if self.battle_player_stat_drop_guard_turns != 0 {
                return Err(
                    "battle_player_stat_drop_guard_turns cannot be saved without an active battle"
                        .to_string(),
                );
            }
            if self.battle_pay_day_money != 0 {
                return Err(
                    "battle_pay_day_money cannot be saved without an active battle".to_string(),
                );
            }
            if self.battle_amulet_coin_active {
                return Err(
                    "battle_amulet_coin_active cannot be saved without an active battle"
                        .to_string(),
                );
            }
            return Ok(());
        };
        if enemy_party_len == 0 {
            return Ok(());
        }
        if self.battle_active_party_index.is_none() {
            return Err("battle_active_party_index must be saved for an active battle".to_string());
        }
        if self.battle_active_enemy_party_index.is_none() {
            return Err(
                "battle_active_enemy_party_index must be saved for an active battle".to_string(),
            );
        }
        if let Some(combat) = &self.script_runtime.active_battle_combat {
            if combat.enemy_party_index >= enemy_party_len {
                return Err(format!(
                    "active_battle_combat enemy index {} is outside enemy party range 0..{}",
                    combat.enemy_party_index, enemy_party_len
                ));
            }
        }
        if !self.battle_rewarded_enemy_party_indices.is_empty()
            && !matches!(self.battle, BattleMemory::Trainer { .. })
        {
            return Err(
                "battle_rewarded_enemy_party_indices can only be saved for an active trainer battle"
                    .to_string(),
            );
        }
        Ok(())
    }

    fn validate_saved_identity_fields(&self) -> Result<(), String> {
        self.overworld.validate_saved_state()?;
        self.battle.validate_saved_state()?;
        if self.player_palette_id > 7 {
            return Err(format!(
                "player_palette_id {} is outside Crystal palette range 0..7",
                self.player_palette_id
            ));
        }
        validate_optional_script_runtime_token(
            "active_repel_item",
            self.active_repel_item.as_deref(),
        )?;
        validate_optional_script_runtime_token(
            "registered_key_item",
            self.registered_key_item.as_deref(),
        )?;
        if let Some(item_id) = &self.registered_key_item {
            if !matches!(self.bag.key_items.get(item_id), Some(quantity) if *quantity > 0) {
                return Err(format!(
                    "registered_key_item {item_id} is not carried in saved key_items"
                ));
            }
        }
        match (self.repel_steps_remaining, &self.active_repel_item) {
            (0, Some(item_id)) => {
                return Err(format!(
                    "active_repel_item {item_id} cannot be saved with zero repel steps"
                ));
            }
            (steps, None) if steps > 0 => {
                return Err(format!(
                    "repel_steps_remaining {steps} cannot be saved without active_repel_item"
                ));
            }
            _ => {}
        }
        validate_optional_script_runtime_token(
            "dig_warp_map_name",
            self.dig_warp_map_name.as_deref(),
        )?;
        match (&self.dig_warp_map_name, self.dig_warp_index) {
            (Some(map_name), None) => {
                return Err(format!(
                    "dig_warp_map_name {map_name} cannot be saved without dig_warp_index"
                ));
            }
            (None, Some(index)) => {
                return Err(format!(
                    "dig_warp_index {index} cannot be saved without dig_warp_map_name"
                ));
            }
            _ => {}
        }
        validate_optional_script_runtime_token(
            "previous_warp_map_name",
            self.previous_warp_map_name.as_deref(),
        )?;
        match (&self.previous_warp_map_name, self.previous_warp_index) {
            (Some(map_name), None) => {
                return Err(format!(
                    "previous_warp_map_name {map_name} cannot be saved without previous_warp_index"
                ));
            }
            (None, Some(index)) => {
                return Err(format!(
                    "previous_warp_index {index} cannot be saved without previous_warp_map_name"
                ));
            }
            (_, Some(0)) => return Err("previous_warp_index cannot be zero".to_string()),
            _ => {}
        }
        validate_optional_script_runtime_token(
            "backup_warp_map_name",
            self.backup_warp_map_name.as_deref(),
        )?;
        match (&self.backup_warp_map_name, self.backup_warp_index) {
            // ASM updates wBackupMapGroup/wBackupMapNumber on ordinary map
            // transitions, but leaves wBackupWarpNumber at zero until a
            // Pokecenter/dynamic-warp path records one. `None` is the exact
            // Rust representation of that valid zero byte.
            (Some(_), None) | (None, None) => {}
            (None, Some(index)) => {
                return Err(format!(
                    "backup_warp_index {index} cannot be saved without backup_warp_map_name"
                ));
            }
            (_, Some(0)) => return Err("backup_warp_index cannot be zero".to_string()),
            (Some(_), Some(_)) => {}
        }
        validate_optional_script_runtime_token(
            "pending_special_battle_type",
            self.pending_special_battle_type.as_deref(),
        )?;
        for map_name in self.map_block_overrides.keys() {
            validate_script_runtime_token("map_block_overrides map", map_name)?;
        }
        for (map_name, memory) in &self.map_object_overrides {
            validate_script_runtime_token("map_object_overrides map", map_name)?;
            memory.validate_saved_state(map_name)?;
        }
        self.link_session.validate_saved_state()?;
        self.battle_tower.validate_saved_state()?;
        self.bug_contest.validate_saved_state()?;
        self.mystery_gift.validate_saved_state()?;
        self.day_care.validate_saved_state()?;
        self.mobile_link.validate_saved_state()?;
        self.magikarp_record.validate_saved_state()?;
        self.buenas_password.validate_saved_state()?;
        Ok(())
    }

    pub fn frame(&self) -> Frame {
        Frame(self.frame_counter)
    }

    pub fn try_advance_frame(&mut self) -> Result<Frame, GameStateFrameError> {
        let next = self
            .frame()
            .checked_next()
            .ok_or(GameStateFrameError::FrameCursorOverflow {
                frame: self.frame_counter,
            })?;
        self.frame_counter = next.0;
        Ok(next)
    }

    pub fn advance_frame(&mut self) -> Frame {
        self.try_advance_frame()
            .expect("game state frame cursor overflow")
    }

    pub fn sync_party_from_storage(&mut self) {
        self.party = PartyState::from_storage(&self.storage);
    }

    pub fn tick_repel_step_after_movement(&mut self) -> Option<String> {
        if self.repel_steps_remaining == 0 {
            return None;
        }
        self.repel_steps_remaining -= 1;
        if self.repel_steps_remaining == 0 {
            return self.active_repel_item.take();
        }
        None
    }

    pub fn commit_rng_seed(&mut self, rng_seed_after: u32) -> RngSeedCommit {
        let commit = RngSeedCommit {
            rng_seed_before: self.rng_seed,
            rng_seed_after,
        };
        self.rng_seed = rng_seed_after;
        commit
    }

    pub fn require_no_active_battle(&self) -> Result<(), GameStateBattleError> {
        match &self.battle {
            BattleMemory::Inactive => Ok(()),
            BattleMemory::Wild { .. } => Err(GameStateBattleError::ActiveBattle {
                battle_kind: "wild".to_string(),
            }),
            BattleMemory::StaticWild { .. } => Err(GameStateBattleError::ActiveBattle {
                battle_kind: "static_wild".to_string(),
            }),
            BattleMemory::Trainer { trainer_id, .. } => {
                Err(GameStateBattleError::ActiveTrainerBattle {
                    trainer_id: trainer_id.clone(),
                })
            }
        }
    }

    pub fn apply_joypad_mask(&mut self, mask: u8) -> Result<GameEvent, GameStateFrameError> {
        validate_joypad_mask(mask)?;
        Ok(self.joypad.apply_mask(mask))
    }

    pub fn apply_command(
        &mut self,
        command: GameCommand,
    ) -> Result<Vec<GameEvent>, GameStateFrameError> {
        match command {
            GameCommand::Joypad { mask } => Ok(vec![self.apply_joypad_mask(mask)?]),
            GameCommand::AdvanceFrame => {
                let frame = self.try_advance_frame()?;
                Ok(vec![GameEvent::FrameAdvanced { frame: frame.0 }])
            }
        }
    }
}

fn validate_saved_player_name(player_name: &str) -> Result<(), String> {
    if player_name.len() > PLAYER_NAME_LENGTH {
        return Err(format!(
            "player_name length {} exceeds Crystal limit {}",
            player_name.len(),
            PLAYER_NAME_LENGTH
        ));
    }
    if player_name.trim() != player_name || player_name.chars().any(char::is_control) {
        return Err(
            "player_name must be exact, untrimmed, and contain no control characters".to_string(),
        );
    }
    Ok(())
}

pub fn validate_saved_player_gender(player_gender: u8) -> Result<(), String> {
    match player_gender {
        PLAYER_GENDER_MALE | PLAYER_GENDER_FEMALE => Ok(()),
        other => Err(format!(
            "player_gender {other} is outside Crystal gender range 0..1"
        )),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum GameStateFrameError {
    #[error("game state frame cursor overflowed at frame {frame}")]
    FrameCursorOverflow { frame: u64 },
    #[error("joypad mask {mask:#010b} has conflicting direction buttons")]
    ConflictingJoypadDirections { mask: u8 },
}

fn validate_joypad_mask(mask: u8) -> Result<(), GameStateFrameError> {
    let directions = [B_PAD_RIGHT, B_PAD_LEFT, B_PAD_UP, B_PAD_DOWN]
        .into_iter()
        .filter(|direction| mask & *direction != 0)
        .count();
    if directions > 1 {
        return Err(GameStateFrameError::ConflictingJoypadDirections { mask });
    }
    Ok(())
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EventFlagMemory {
    pub event_flags: BTreeMap<String, bool>,
    pub engine_flags: BTreeMap<String, bool>,
}

impl<'de> Deserialize<'de> for EventFlagMemory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawEventFlagMemory {
            event_flags: BTreeMap<String, bool>,
            engine_flags: BTreeMap<String, bool>,
        }

        let raw = RawEventFlagMemory::deserialize(deserializer)?;
        let memory = Self {
            event_flags: raw.event_flags,
            engine_flags: raw.engine_flags,
        };
        memory.validate().map_err(D::Error::custom)?;
        Ok(memory)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum EventFlagError {
    #[error("empty flag name")]
    EmptyFlagName,
    #[error("invalid flag name {flag_name}")]
    InvalidFlagName { flag_name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum EventFlagSaveError {
    #[error("saved {path} {flag_name} is an engine flag, not an event flag")]
    EngineFlagSavedAsEventFlag { path: String, flag_name: String },
    #[error("saved {path} {flag_name} is not an engine flag")]
    EventFlagSavedAsEngineFlag { path: String, flag_name: String },
    #[error("saved {path} {flag_name} is missing from compiled pack event flags")]
    MissingEventFlag { path: String, flag_name: String },
    #[error("saved {path} {flag_name} is missing from compiled pack engine flags")]
    MissingEngineFlag { path: String, flag_name: String },
}

impl EventFlagMemory {
    pub fn validate(&self) -> Result<(), EventFlagError> {
        for flag_name in self.event_flags.keys() {
            validate_flag_name(flag_name)?;
        }
        for flag_name in self.engine_flags.keys() {
            validate_flag_name(flag_name)?;
        }
        Ok(())
    }

    pub fn set_event_flag(&mut self, flag_name: &str, value: bool) -> Result<(), EventFlagError> {
        let flag_name = validate_flag_name(flag_name)?;
        self.event_flags.insert(flag_name.to_string(), value);
        Ok(())
    }

    pub fn clear_event_flag(&mut self, flag_name: &str) -> Result<(), EventFlagError> {
        self.set_event_flag(flag_name, false)
    }

    pub fn is_event_flag_set(&self, flag_name: &str) -> Result<bool, EventFlagError> {
        let flag_name = validate_flag_name(flag_name)?;
        Ok(self.event_flags.get(flag_name).copied().unwrap_or(false))
    }

    pub fn set_engine_flag(&mut self, flag_name: &str, value: bool) -> Result<(), EventFlagError> {
        let flag_name = validate_flag_name(flag_name)?;
        self.engine_flags.insert(flag_name.to_string(), value);
        Ok(())
    }

    pub fn clear_engine_flag(&mut self, flag_name: &str) -> Result<(), EventFlagError> {
        self.set_engine_flag(flag_name, false)
    }

    pub fn is_engine_flag_set(&self, flag_name: &str) -> Result<bool, EventFlagError> {
        let flag_name = validate_flag_name(flag_name)?;
        Ok(self.engine_flags.get(flag_name).copied().unwrap_or(false))
    }

    pub fn set_script_flag(&mut self, flag_name: &str, value: bool) -> Result<(), EventFlagError> {
        if is_engine_flag_name(flag_name) {
            self.set_engine_flag(flag_name, value)
        } else {
            self.set_event_flag(flag_name, value)
        }
    }

    pub fn clear_script_flag(&mut self, flag_name: &str) -> Result<(), EventFlagError> {
        self.set_script_flag(flag_name, false)
    }

    pub fn is_script_flag_set(&self, flag_name: &str) -> Result<bool, EventFlagError> {
        if is_engine_flag_name(flag_name) {
            self.is_engine_flag_set(flag_name)
        } else {
            self.is_event_flag_set(flag_name)
        }
    }

    pub fn active_event_flags(&self) -> impl Iterator<Item = &String> {
        self.event_flags
            .iter()
            .filter_map(|(flag, value)| value.then_some(flag))
    }
}

pub fn validate_saved_flag_references<F, G>(
    flags: &EventFlagMemory,
    event_flag_exists: F,
    engine_flag_exists: G,
) -> Result<(), EventFlagSaveError>
where
    F: Fn(&str) -> bool,
    G: Fn(&str) -> bool,
{
    for flag_name in flags.event_flags.keys() {
        validate_saved_event_flag_reference("flags.event_flags", flag_name, &event_flag_exists)?;
    }
    for flag_name in flags.engine_flags.keys() {
        validate_saved_engine_flag_reference("flags.engine_flags", flag_name, &engine_flag_exists)?;
    }
    Ok(())
}

pub fn validate_saved_event_flag_reference(
    path: &str,
    flag_name: &str,
    event_flag_exists: impl FnOnce(&str) -> bool,
) -> Result<(), EventFlagSaveError> {
    if is_engine_flag_name(flag_name) {
        return Err(EventFlagSaveError::EngineFlagSavedAsEventFlag {
            path: path.to_string(),
            flag_name: flag_name.to_string(),
        });
    }
    if event_flag_exists(flag_name) {
        Ok(())
    } else {
        Err(EventFlagSaveError::MissingEventFlag {
            path: path.to_string(),
            flag_name: flag_name.to_string(),
        })
    }
}

pub fn validate_saved_engine_flag_reference(
    path: &str,
    flag_name: &str,
    engine_flag_exists: impl FnOnce(&str) -> bool,
) -> Result<(), EventFlagSaveError> {
    if !is_engine_flag_name(flag_name) {
        return Err(EventFlagSaveError::EventFlagSavedAsEngineFlag {
            path: path.to_string(),
            flag_name: flag_name.to_string(),
        });
    }
    if engine_flag_exists(flag_name) {
        Ok(())
    } else {
        Err(EventFlagSaveError::MissingEngineFlag {
            path: path.to_string(),
            flag_name: flag_name.to_string(),
        })
    }
}

pub fn is_engine_flag_name(flag_name: &str) -> bool {
    flag_name.starts_with("ENGINE_") || flag_name.starts_with("STATUSFLAGS_")
}

fn validate_flag_name(flag_name: &str) -> Result<&str, EventFlagError> {
    if flag_name.is_empty() {
        return Err(EventFlagError::EmptyFlagName);
    }
    if !is_exact_flag_name(flag_name) {
        return Err(EventFlagError::InvalidFlagName {
            flag_name: flag_name.to_string(),
        });
    }
    Ok(flag_name)
}

fn is_exact_flag_name(flag_name: &str) -> bool {
    !has_reserved_runtime_token_prefix(flag_name)
        && flag_name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SceneMemory {
    pub current_map_name: String,
    pub scene_name: String,
    pub map_scenes: BTreeMap<String, String>,
    pub map_scene_indices: BTreeMap<String, usize>,
}

impl<'de> Deserialize<'de> for SceneMemory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSceneMemory {
            current_map_name: String,
            scene_name: String,
            map_scenes: BTreeMap<String, String>,
            map_scene_indices: BTreeMap<String, usize>,
        }

        let raw = RawSceneMemory::deserialize(deserializer)?;
        let memory = Self {
            current_map_name: raw.current_map_name,
            scene_name: raw.scene_name,
            map_scenes: raw.map_scenes,
            map_scene_indices: raw.map_scene_indices,
        };
        memory.validate().map_err(D::Error::custom)?;
        Ok(memory)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneStatus {
    pub map_name: String,
    pub scene_name: String,
    pub scene_index: usize,
    pub script_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum SceneError {
    #[error("missing scene table for map {map_name}")]
    MissingSceneTable { map_name: String },
    #[error("invalid scene map name {map_name}")]
    InvalidMapName { map_name: String },
    #[error("invalid scene name {scene_name}")]
    InvalidSceneName { scene_name: String },
    #[error("saved scene map {map_name} has no saved scene index")]
    MissingSceneIndex { map_name: String },
    #[error("saved scene index for map {map_name} has no saved scene name")]
    UnexpectedSceneIndex { map_name: String },
    #[error("unknown scene {scene_name} for map {map_name}")]
    UnknownScene {
        map_name: String,
        scene_name: String,
    },
    #[error("empty scene table for map {map_name}")]
    EmptySceneTable { map_name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum SceneSaveError {
    #[error("saved scenes.scene_name is empty while current map {map_name} has a saved scene")]
    EmptyCurrentSceneWithSavedMapScene { map_name: String },
    #[error("saved scenes.scene_name requires scenes.current_map_name")]
    CurrentSceneMissingMap,
    #[error(
        "saved scenes.current {map_name}:{scene_name} is missing from scenes.map_scenes definitive entries"
    )]
    CurrentSceneMissingMapSceneEntry {
        map_name: String,
        scene_name: String,
    },
    #[error(
        "saved scenes.current {map_name}:{scene_name} does not match scenes.map_scenes value {saved_scene_name}"
    )]
    CurrentSceneMapSceneMismatch {
        map_name: String,
        scene_name: String,
        saved_scene_name: String,
    },
    #[error("saved {path} {map_name} is missing from compiled pack maps")]
    MissingMap {
        path: &'static str,
        map_name: String,
    },
    #[error("saved {path} {map_name}:{scene_name} is missing from compiled map scenes")]
    MissingScene {
        path: &'static str,
        map_name: String,
        scene_name: String,
    },
    #[error("saved {path} {map_name}:{scene_name} is missing saved scene index")]
    MissingSceneIndex {
        path: &'static str,
        map_name: String,
        scene_name: String,
    },
    #[error(
        "saved {path} {map_name}:{scene_name} index {saved_index} does not match compiled scene index {compiled_index}"
    )]
    SceneIndexMismatch {
        path: &'static str,
        map_name: String,
        scene_name: String,
        saved_index: usize,
        compiled_index: usize,
    },
}

impl SceneMemory {
    pub fn validate(&self) -> Result<(), SceneError> {
        if !self.current_map_name.is_empty() && !is_exact_scene_token(&self.current_map_name) {
            return Err(SceneError::InvalidMapName {
                map_name: self.current_map_name.clone(),
            });
        }
        if !self.scene_name.is_empty() && !is_exact_scene_token(&self.scene_name) {
            return Err(SceneError::InvalidSceneName {
                scene_name: self.scene_name.clone(),
            });
        }
        for (map_name, scene_name) in &self.map_scenes {
            validate_scene_token(map_name)
                .map_err(|map_name| SceneError::InvalidMapName { map_name })?;
            validate_scene_token(scene_name)
                .map_err(|scene_name| SceneError::InvalidSceneName { scene_name })?;
            if !self.map_scene_indices.contains_key(map_name) {
                return Err(SceneError::MissingSceneIndex {
                    map_name: map_name.clone(),
                });
            }
        }
        for map_name in self.map_scene_indices.keys() {
            validate_scene_token(map_name)
                .map_err(|map_name| SceneError::InvalidMapName { map_name })?;
            if !self.map_scenes.contains_key(map_name) {
                return Err(SceneError::UnexpectedSceneIndex {
                    map_name: map_name.clone(),
                });
            }
        }
        Ok(())
    }

    pub fn enter_map(
        &mut self,
        map_name: impl Into<String>,
        table: &MapSceneTable,
    ) -> Result<SceneStatus, SceneError> {
        let map_name = map_name.into();
        self.current_map_name = map_name.clone();
        let status = self.ensure_map_scene_initialized(&map_name, table)?;
        self.scene_name = status.scene_name.clone();
        Ok(status)
    }

    pub fn ensure_map_scene_initialized(
        &mut self,
        map_name: &str,
        table: &MapSceneTable,
    ) -> Result<SceneStatus, SceneError> {
        if table.scenes.is_empty() {
            return Err(SceneError::EmptySceneTable {
                map_name: map_name.to_string(),
            });
        }
        if let Some(scene_name) = self.map_scenes.get(map_name).cloned() {
            return self.scene_status(map_name, &scene_name, table);
        }
        let scene = table
            .scenes
            .first()
            .ok_or_else(|| SceneError::EmptySceneTable {
                map_name: map_name.to_string(),
            })?;
        self.set_map_scene(map_name, &scene.scene_id, table)
    }

    pub fn set_current_scene(
        &mut self,
        scene_name: &str,
        table: &MapSceneTable,
    ) -> Result<SceneStatus, SceneError> {
        let map_name = self.current_map_name.clone();
        if map_name.is_empty() {
            return Err(SceneError::MissingSceneTable {
                map_name: String::new(),
            });
        }
        self.set_map_scene(&map_name, scene_name, table)
    }

    pub fn set_map_scene(
        &mut self,
        map_name: &str,
        scene_name: &str,
        table: &MapSceneTable,
    ) -> Result<SceneStatus, SceneError> {
        let status = self.scene_status(map_name, scene_name, table)?;
        self.map_scenes
            .insert(map_name.to_string(), scene_name.to_string());
        self.map_scene_indices
            .insert(map_name.to_string(), status.scene_index);
        if self.current_map_name == map_name {
            self.scene_name = scene_name.to_string();
        }
        Ok(status)
    }

    pub fn check_scene(
        &mut self,
        map_name: &str,
        table: &MapSceneTable,
    ) -> Result<SceneStatus, SceneError> {
        let status = self.ensure_map_scene_initialized(map_name, table)?;
        if self.current_map_name == map_name {
            self.scene_name = status.scene_name.clone();
        }
        Ok(status)
    }

    fn scene_status(
        &self,
        map_name: &str,
        scene_name: &str,
        table: &MapSceneTable,
    ) -> Result<SceneStatus, SceneError> {
        if table.scenes.is_empty() {
            let scene_index =
                scene_name
                    .parse::<usize>()
                    .map_err(|_| SceneError::UnknownScene {
                        map_name: map_name.to_string(),
                        scene_name: scene_name.to_string(),
                    })?;
            return Ok(SceneStatus {
                map_name: map_name.to_string(),
                scene_name: scene_name.to_string(),
                scene_index,
                script_name: None,
            });
        }
        let (scene_index, scene) = table
            .scenes
            .iter()
            .enumerate()
            .find(|(_, scene)| scene.scene_id == scene_name)
            .ok_or_else(|| SceneError::UnknownScene {
                map_name: map_name.to_string(),
                scene_name: scene_name.to_string(),
            })?;
        Ok(SceneStatus {
            map_name: map_name.to_string(),
            scene_name: scene.scene_id.clone(),
            scene_index,
            script_name: scene.script_name.clone(),
        })
    }
}

pub fn validate_saved_scene_references<F, G>(
    scenes: &SceneMemory,
    map_exists: F,
    scene_index: G,
) -> Result<(), SceneSaveError>
where
    F: Fn(&str) -> bool,
    G: Fn(&str, &str) -> Option<usize>,
{
    if !scenes.current_map_name.is_empty() {
        if !map_exists(&scenes.current_map_name) {
            return Err(SceneSaveError::MissingMap {
                path: "scenes.current_map_name",
                map_name: scenes.current_map_name.clone(),
            });
        }
        if scenes.scene_name.is_empty() {
            if scenes.map_scenes.contains_key(&scenes.current_map_name) {
                return Err(SceneSaveError::EmptyCurrentSceneWithSavedMapScene {
                    map_name: scenes.current_map_name.clone(),
                });
            }
        } else {
            let Some(saved_scene_name) = scenes.map_scenes.get(&scenes.current_map_name) else {
                return Err(SceneSaveError::CurrentSceneMissingMapSceneEntry {
                    map_name: scenes.current_map_name.clone(),
                    scene_name: scenes.scene_name.clone(),
                });
            };
            if saved_scene_name != &scenes.scene_name {
                return Err(SceneSaveError::CurrentSceneMapSceneMismatch {
                    map_name: scenes.current_map_name.clone(),
                    scene_name: scenes.scene_name.clone(),
                    saved_scene_name: saved_scene_name.clone(),
                });
            }
            validate_saved_scene_entry(
                "scenes.current",
                &scenes.current_map_name,
                &scenes.scene_name,
                scenes
                    .map_scene_indices
                    .get(&scenes.current_map_name)
                    .copied(),
                &scene_index,
            )?;
        }
    } else if !scenes.scene_name.is_empty() {
        return Err(SceneSaveError::CurrentSceneMissingMap);
    }

    for (map_name, scene_name) in &scenes.map_scenes {
        if !map_exists(map_name) {
            return Err(SceneSaveError::MissingMap {
                path: "scenes.map_scenes",
                map_name: map_name.clone(),
            });
        }
        validate_saved_scene_entry(
            "scenes.map_scenes",
            map_name,
            scene_name,
            scenes.map_scene_indices.get(map_name).copied(),
            &scene_index,
        )?;
    }
    Ok(())
}

fn validate_saved_scene_entry<F>(
    path: &'static str,
    map_name: &str,
    scene_name: &str,
    saved_index: Option<usize>,
    scene_index: &F,
) -> Result<(), SceneSaveError>
where
    F: Fn(&str, &str) -> Option<usize>,
{
    let Some(compiled_index) = scene_index(map_name, scene_name) else {
        return Err(SceneSaveError::MissingScene {
            path,
            map_name: map_name.to_string(),
            scene_name: scene_name.to_string(),
        });
    };
    let Some(saved_index) = saved_index else {
        return Err(SceneSaveError::MissingSceneIndex {
            path,
            map_name: map_name.to_string(),
            scene_name: scene_name.to_string(),
        });
    };
    if saved_index != compiled_index {
        return Err(SceneSaveError::SceneIndexMismatch {
            path,
            map_name: map_name.to_string(),
            scene_name: scene_name.to_string(),
            saved_index,
            compiled_index,
        });
    }
    Ok(())
}

fn validate_scene_token(value: &str) -> Result<(), String> {
    if is_exact_scene_token(value) {
        Ok(())
    } else {
        Err(value.to_string())
    }
}

fn is_exact_scene_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !has_reserved_runtime_token_prefix(value)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Options {
    pub text_speed: TextSpeed,
    pub battle_scene: BattleScene,
    pub battle_style: BattleStyle,
    pub sound: Sound,
    pub menu_account: MenuAccount,
    pub print_option: PrintOption,
    pub frame: FrameType,
    pub no_text_scroll: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            text_speed: TextSpeed::Fast,
            battle_scene: BattleScene::On,
            battle_style: BattleStyle::Shift,
            sound: Sound::Stereo,
            menu_account: MenuAccount::On,
            print_option: PrintOption::Normal,
            frame: FrameType::Frame1,
            no_text_scroll: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum TextSpeed {
    Fast,
    Mid,
    Slow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum BattleScene {
    On,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum BattleStyle {
    Shift,
    Set,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Sound {
    Mono,
    Stereo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MenuAccount {
    On,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum PrintOption {
    Normal,
    Lightest,
    Lighter,
    Darker,
    Darkest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FrameType {
    Frame1,
    Frame2,
    Frame3,
    Frame4,
    Frame5,
    Frame6,
    Frame7,
    Frame8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartyState {
    pub pokemon: [Option<PartyPokemonRef>; 6],
}

impl Default for PartyState {
    fn default() -> Self {
        Self {
            pokemon: [const { None }; 6],
        }
    }
}

impl PartyState {
    pub fn from_storage(storage: &PokemonStorage) -> Self {
        let mut state = Self::default();
        for (index, pokemon) in storage.party.pokemon.iter().enumerate() {
            if let Some(pokemon) = pokemon {
                state.pokemon[index] = Some(PartyPokemonRef {
                    species: pokemon.species.id.clone(),
                    level: pokemon.level,
                });
            }
        }
        state
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartyPokemonRef {
    pub species: String,
    pub level: u8,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkBattleStats {
    pub wins: u16,
    pub losses: u16,
    pub draws: u16,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FishingRodState {
    #[default]
    Idle,
    Waiting,
    Bite,
    Battle,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FishingMemory {
    pub rod_state: FishingRodState,
    pub rod_index: Option<u8>,
    pub bites_remaining: u8,
    pub result: u8,
    pub daily_flags1: u8,
    pub swarm_flag: u8,
}

impl<'de> Deserialize<'de> for FishingMemory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawFishingMemory {
            rod_state: FishingRodState,
            rod_index: Option<u8>,
            bites_remaining: u8,
            result: u8,
            daily_flags1: u8,
            swarm_flag: u8,
        }

        let raw = RawFishingMemory::deserialize(deserializer)?;
        let memory = Self {
            rod_state: raw.rod_state,
            rod_index: raw.rod_index,
            bites_remaining: raw.bites_remaining,
            result: raw.result,
            daily_flags1: raw.daily_flags1,
            swarm_flag: raw.swarm_flag,
        };
        memory.validate_saved_state().map_err(D::Error::custom)?;
        Ok(memory)
    }
}

impl FishingMemory {
    fn validate_saved_state(&self) -> Result<(), String> {
        match self.rod_state {
            FishingRodState::Idle => {
                if self.rod_index.is_some() {
                    return Err("fishing.rod_index cannot be saved while idle".to_string());
                }
                if self.bites_remaining != 0 {
                    return Err(format!(
                        "fishing.bites_remaining {} cannot be saved while idle",
                        self.bites_remaining
                    ));
                }
                if self.result != 0 {
                    return Err(format!(
                        "fishing.result {} cannot be saved while idle",
                        self.result
                    ));
                }
            }
            FishingRodState::Waiting | FishingRodState::Bite | FishingRodState::Battle => {
                let Some(rod_index) = self.rod_index else {
                    return Err(format!(
                        "fishing.rod_index is required for {:?}",
                        self.rod_state
                    ));
                };
                if rod_index > 2 {
                    return Err(format!(
                        "fishing.rod_index {rod_index} is outside Crystal rod range 0..2"
                    ));
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SwarmMemory {
    pub active: BTreeMap<String, SwarmMapTarget>,
}

impl<'de> Deserialize<'de> for SwarmMemory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSwarmMemory {
            active: BTreeMap<String, SwarmMapTarget>,
        }

        let raw = RawSwarmMemory::deserialize(deserializer)?;
        let memory = Self { active: raw.active };
        memory.validate_saved_state().map_err(D::Error::custom)?;
        Ok(memory)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SwarmMapTarget {
    pub map_id: String,
    pub map_group: Option<u16>,
    pub map_number: Option<u16>,
}

impl<'de> Deserialize<'de> for SwarmMapTarget {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawSwarmMapTarget {
            map_id: String,
            map_group: Option<u16>,
            map_number: Option<u16>,
        }

        let raw = RawSwarmMapTarget::deserialize(deserializer)?;
        let target = Self {
            map_id: raw.map_id,
            map_group: raw.map_group,
            map_number: raw.map_number,
        };
        validate_script_runtime_token("swarms.active map_id", &target.map_id)
            .map_err(D::Error::custom)?;
        match (target.map_group, target.map_number) {
            (Some(_), Some(_)) | (None, None) => Ok(target),
            (Some(_), None) => Err(D::Error::custom(
                "swarms target has map_group without map_number",
            )),
            (None, Some(_)) => Err(D::Error::custom(
                "swarms target has map_number without map_group",
            )),
        }
    }
}

impl SwarmMemory {
    pub fn validate_saved_state(&self) -> Result<(), String> {
        for (swarm_token, target) in &self.active {
            validate_script_runtime_token("swarms.active token", swarm_token)?;
            validate_script_runtime_token("swarms.active map_id", &target.map_id)?;
            match (target.map_group, target.map_number) {
                (Some(_), Some(_)) => {}
                (None, None) => {
                    return Err(format!(
                        "swarms.active {swarm_token} is missing map_group and map_number"
                    ));
                }
                (Some(_), None) => {
                    return Err(format!(
                        "swarms.active {swarm_token} has map_group without map_number"
                    ));
                }
                (None, Some(_)) => {
                    return Err(format!(
                        "swarms.active {swarm_token} has map_number without map_group"
                    ));
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Badges {
    pub johto: [bool; 8],
    pub kanto: [bool; 8],
}

impl Default for Badges {
    fn default() -> Self {
        Self {
            johto: [false; 8],
            kanto: [false; 8],
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JoypadMemory {
    pub h_joypad_released: u8,
    pub h_joypad_pressed: u8,
    pub h_joypad_down: u8,
    pub h_joypad_sum: u8,
    pub h_joy_released: u8,
    pub h_joy_pressed: u8,
    pub h_joy_down: u8,
    pub h_joy_last: u8,
}

impl<'de> Deserialize<'de> for JoypadMemory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawJoypadMemory {
            h_joypad_released: u8,
            h_joypad_pressed: u8,
            h_joypad_down: u8,
            h_joypad_sum: u8,
            h_joy_released: u8,
            h_joy_pressed: u8,
            h_joy_down: u8,
            h_joy_last: u8,
        }

        let raw = RawJoypadMemory::deserialize(deserializer)?;
        let memory = Self {
            h_joypad_released: raw.h_joypad_released,
            h_joypad_pressed: raw.h_joypad_pressed,
            h_joypad_down: raw.h_joypad_down,
            h_joypad_sum: raw.h_joypad_sum,
            h_joy_released: raw.h_joy_released,
            h_joy_pressed: raw.h_joy_pressed,
            h_joy_down: raw.h_joy_down,
            h_joy_last: raw.h_joy_last,
        };
        memory.validate_saved_state().map_err(D::Error::custom)?;
        Ok(memory)
    }
}

impl JoypadMemory {
    fn validate_saved_state(&self) -> Result<(), String> {
        validate_joypad_mask(self.h_joypad_released).map_err(|error| error.to_string())?;
        validate_joypad_mask(self.h_joypad_pressed).map_err(|error| error.to_string())?;
        validate_joypad_mask(self.h_joypad_down).map_err(|error| error.to_string())?;
        validate_joypad_mask(self.h_joy_released).map_err(|error| error.to_string())?;
        validate_joypad_mask(self.h_joy_pressed).map_err(|error| error.to_string())?;
        validate_joypad_mask(self.h_joy_down).map_err(|error| error.to_string())?;
        validate_joypad_mask(self.h_joy_last).map_err(|error| error.to_string())?;
        if self.h_joypad_down != self.h_joy_down {
            return Err(format!(
                "joypad h_joypad_down {:#010b} does not match h_joy_down {:#010b}",
                self.h_joypad_down, self.h_joy_down
            ));
        }
        if self.h_joypad_pressed != self.h_joy_pressed {
            return Err(format!(
                "joypad h_joypad_pressed {:#010b} does not match h_joy_pressed {:#010b}",
                self.h_joypad_pressed, self.h_joy_pressed
            ));
        }
        if self.h_joypad_released != self.h_joy_released {
            return Err(format!(
                "joypad h_joypad_released {:#010b} does not match h_joy_released {:#010b}",
                self.h_joypad_released, self.h_joy_released
            ));
        }
        Ok(())
    }

    pub fn apply_mask(&mut self, mask: u8) -> GameEvent {
        let previous = self.h_joypad_down;
        let pressed = (mask ^ previous) & mask;
        let released = (mask ^ previous) & previous;
        self.h_joypad_released = released;
        self.h_joypad_pressed = pressed;
        self.h_joypad_down = mask;
        self.h_joypad_sum |= pressed;
        self.h_joy_released = released;
        self.h_joy_pressed = pressed;
        self.h_joy_down = mask;
        self.h_joy_last = previous;
        GameEvent::JoypadChanged {
            pressed,
            down: mask,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum GameCommand {
    Joypad { mask: u8 },
    AdvanceFrame,
}

impl<'de> Deserialize<'de> for GameCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
        enum RawGameCommand {
            Joypad { mask: u8 },
            AdvanceFrame,
        }

        match RawGameCommand::deserialize(deserializer)? {
            RawGameCommand::Joypad { mask } => {
                validate_joypad_mask(mask).map_err(D::Error::custom)?;
                Ok(Self::Joypad { mask })
            }
            RawGameCommand::AdvanceFrame => Ok(Self::AdvanceFrame),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum GameEvent {
    FrameAdvanced { frame: u64 },
    JoypadChanged { pressed: u8, down: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RngSeedCommit {
    pub rng_seed_before: u32,
    pub rng_seed_after: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum GameStateBattleError {
    #[error("active {battle_kind} battle")]
    ActiveBattle { battle_kind: String },
    #[error("active trainer battle {trainer_id}")]
    ActiveTrainerBattle { trainer_id: String },
}

#[cfg(test)]
mod tests {
    #[test]
    fn release_state_command_surface_has_no_event_only_menu_commands() {
        let source = include_str!("state.rs");
        let open = format!("{}{}", "Open", "Menu");
        let close = format!("{}{}", "Close", "Menu");
        assert!(
            !source.contains(&open) && !source.contains(&close),
            "menus must be entered through source-derived frame/script execution"
        );
    }

    #[test]
    fn default_state_is_available_only_as_a_test_fixture() {
        let source = include_str!("state.rs");
        assert!(source.contains(
            "#[cfg(any(test, feature = \"test-fixtures\"))]\nimpl Default for GameState"
        ));
        assert_eq!(GameState::default(), GameState::reset_wram_for_new_game());
    }

    use super::*;
    use crate::models::{BaseStats, Dv, PcBox};

    #[test]
    fn reset_wram_state_has_required_new_game_inputs() {
        let state = GameState::reset_wram_for_new_game();
        assert_eq!(state.options.text_speed, TextSpeed::Fast);
        assert_eq!(state.player_name, "");
        assert_eq!(state.player_id, 0);
        assert_eq!(state.player_gender, PLAYER_GENDER_MALE);
        assert_eq!(state.options.battle_scene, BattleScene::On);
        assert_eq!(state.options.battle_style, BattleStyle::Shift);
        assert_eq!(state.options.sound, Sound::Stereo);
        assert_eq!(state.options.menu_account, MenuAccount::On);
        assert_eq!(state.party.pokemon, [const { None }; 6]);
        assert_eq!(state.storage, PokemonStorage::default());
        assert_eq!(state.bag, Bag::default());
        assert_eq!(state.pokedex, PokedexState::default());
        assert_eq!(state.link_battle_stats, LinkBattleStats::default());
        assert_eq!(state.badges.johto, [false; 8]);
        assert_eq!(state.badges.kanto, [false; 8]);
        assert_eq!(state.overworld, OverworldMemory::Inactive);
        assert_eq!(state.battle_active_party_index, None);
        assert_eq!(state.battle_active_enemy_party_index, None);
        assert!(state.battle_rewarded_enemy_party_indices.is_empty());
        assert_eq!(state.battle_escape_attempts, 0);
        assert_eq!(state.battle_player_stat_drop_guard_turns, 0);
        assert_eq!(state.wild_encounter_cooldown, 0);
        assert_eq!(state.repel_steps_remaining, 0);
        assert_eq!(state.active_repel_item, None);
        assert_eq!(state.registered_key_item, None);
        assert_eq!(state.dig_warp_map_name, None);
        assert_eq!(state.dig_warp_index, None);
        assert_eq!(state.kenji_break_timer, 0);
        assert_eq!(state.player_palette_id, 0);
        assert_eq!(state.step_events, StepEventCounters::default());
        assert_eq!(state.time, TimeState::default());
        assert_eq!(
            state.unused_two_day_timer,
            UnusedTwoDayTimerState::default()
        );
        assert!(!state.lucky_number_show_flag);
        assert_eq!(state.lucky_number_day, None);
        assert_eq!(state.lucky_id_number, 0);
        assert_eq!(state.current_pc_box, 0);
        assert_eq!(state.gs_healings, 0);
        assert_eq!(state.trainer_rankings_healings, 0);
        assert!(!state.script_runtime.reset_requested);
        assert_eq!(state.scenes, SceneMemory::default());
        assert_eq!(state.flags, EventFlagMemory::default());
        assert_eq!(state.frame_counter, 0);
        assert_eq!(state.rng_seed, 1);
        assert!(!state.has_seen_intro);
    }

    #[test]
    fn daily_reset_clears_fishing_and_swarm_state() {
        let mut state = GameState::default();
        state.fishing.daily_flags1 = 0b1010;
        state.fishing.swarm_flag = 1;
        state.swarms.active.insert(
            "route-30".to_string(),
            SwarmMapTarget {
                map_id: "route-30".to_string(),
                map_group: None,
                map_number: None,
            },
        );

        state.apply_daily_reset();

        assert_eq!(state.fishing.daily_flags1, 0);
        assert_eq!(state.fishing.swarm_flag, 0);
        assert!(state.swarms.active.is_empty());
        assert!((3..=6).contains(&state.kenji_break_timer));
    }

    #[test]
    fn daily_reset_decrements_kenji_break_countdown_without_resampling() {
        let mut state = GameState::default();
        state.kenji_break_timer = 4;
        let seed = state.rng_seed;

        state.apply_daily_reset();

        assert_eq!(state.kenji_break_timer, 3);
        assert_eq!(state.rng_seed, seed);
    }

    #[test]
    fn pokerus_tick_preserves_strain_and_cures_low_nibble_in_party_and_pc() {
        let species =
            PokemonSpecies::new_for_tests("TOGEPI", BaseStats::new(35, 20, 65, 20, 40, 65));
        let mut party_mon = Pokemon::new_for_tests(species.clone(), 5, Dv::default());
        party_mon.pokerus = 0xa2;
        let mut boxed_mon = Pokemon::new_for_tests(species, 5, Dv::default());
        boxed_mon.pokerus = 0x53;
        let mut state = GameState::default();
        state.storage.party.pokemon[0] = Some(party_mon);
        let mut pc_box = PcBox::new(0);
        pc_box.set_slot(0, Some(boxed_mon));
        state.storage.pc_boxes.push(pc_box);
        state.sync_party_from_storage();

        state.apply_pokerus_tick(2);
        assert_eq!(
            state.storage.party.pokemon[0].as_ref().unwrap().pokerus,
            0xa0
        );
        assert_eq!(
            state.storage.pc_boxes[0].pokemon[0]
                .as_ref()
                .unwrap()
                .pokerus,
            0x51
        );

        state.apply_pokerus_tick(1);
        assert_eq!(
            state.storage.party.pokemon[0].as_ref().unwrap().pokerus,
            0xa0
        );
        assert_eq!(
            state.storage.pc_boxes[0].pokemon[0]
                .as_ref()
                .unwrap()
                .pokerus,
            0x50
        );
    }

    #[test]
    fn pokerus_spread_copies_strain_to_a_neighbor_and_advances_rng() {
        let species =
            PokemonSpecies::new_for_tests("TOGEPI", BaseStats::new(35, 20, 65, 20, 40, 65));
        let mut state = GameState::default();
        let mut infected = Pokemon::new_for_tests(species.clone(), 5, Dv::default());
        infected.pokerus = 0xa2;
        state.storage.party.pokemon[0] = Some(infected);
        state.storage.party.pokemon[1] = Some(Pokemon::new_for_tests(species, 5, Dv::default()));
        state.sync_party_from_storage();

        let seed = 0x0000_7001;
        state.rng_seed = seed;
        assert!(state.spread_pokerus_after_battle());
        assert_eq!(
            state.storage.party.pokemon[1].as_ref().unwrap().pokerus,
            0xa2
        );
        assert_ne!(state.rng_seed, seed);
    }

    #[test]
    fn state_serializes_for_saves_and_multiplayer_hash_inputs() {
        let state = GameState::default();
        let json = serde_json::to_string(&state).expect("serialize game state");
        assert!(json.contains(r#""text_speed":"fast""#));
        assert_eq!(
            serde_json::from_str::<GameState>(&json).expect("deserialize game state"),
            state
        );
    }

    #[test]
    fn saved_game_state_validates_non_script_pack_identifiers() {
        let mut state = GameState {
            active_repel_item: Some("SUPER REPEL".to_string()),
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err("active_repel_item has invalid token 'SUPER REPEL'".to_string())
        );

        state = GameState {
            registered_key_item: Some("COIN CASE".to_string()),
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err("registered_key_item has invalid token 'COIN CASE'".to_string())
        );

        state = GameState {
            active_repel_item: Some("fallback_repel".to_string()),
            repel_steps_remaining: 10,
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "active_repel_item token 'fallback_repel' uses reserved runtime payload prefix"
                    .to_string()
            )
        );

        state = GameState {
            player_name: " GOLD".to_string(),
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "player_name must be exact, untrimmed, and contain no control characters"
                    .to_string()
            )
        );

        state = GameState {
            player_name: "GOLD\n".to_string(),
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "player_name must be exact, untrimmed, and contain no control characters"
                    .to_string()
            )
        );

        state = GameState {
            player_name: "GOLDENROD".to_string(),
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err(format!(
                "player_name length 9 exceeds Crystal limit {PLAYER_NAME_LENGTH}"
            ))
        );

        state = GameState {
            player_gender: 2,
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err("player_gender 2 is outside Crystal gender range 0..1".to_string())
        );

        state = GameState::default();
        state.time.registers.minutes = 60;
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "invalid saved time: time.registers.minutes 60 is outside clock range 0..59"
                    .to_string()
            )
        );

        state = GameState::default();
        state.time.current_day = 8;
        state.time.day_of_week = 0;
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "invalid saved time: time.day_of_week 0 does not match current_day modulo 7 1"
                    .to_string()
            )
        );

        state = GameState::default();
        state.time.registers.hours = 10;
        state.time.game_time_hours = 9;
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "invalid saved time: time.game_time_hours 9 does not match registers.hours 10"
                    .to_string()
            )
        );

        state = GameState::default();
        state.unused_two_day_timer.active = true;
        state.unused_two_day_timer.remaining_days = 3;
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "invalid saved unused two-day timer: remaining_days 3 exceeds the two-day timer length"
                    .to_string()
            )
        );

        state = GameState::default();
        state.unused_two_day_timer.remaining_days = 1;
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "invalid saved unused two-day timer: inactive timer must have remaining_days 0 and start_day 0"
                    .to_string()
            )
        );

        state = GameState::default();
        state.player_palette_id = 8;
        assert_eq!(
            state.validate_saved_state(),
            Err("player_palette_id 8 is outside Crystal palette range 0..7".to_string())
        );

        state = GameState::default();
        state.repel_steps_remaining = 10;
        assert_eq!(
            state.validate_saved_state(),
            Err("repel_steps_remaining 10 cannot be saved without active_repel_item".to_string())
        );

        state = GameState::default();
        state.active_repel_item = Some("REPEL".to_string());
        assert_eq!(
            state.validate_saved_state(),
            Err("active_repel_item REPEL cannot be saved with zero repel steps".to_string())
        );

        state = GameState::default();
        state.dig_warp_map_name = Some("AzaleaTown".to_string());
        assert_eq!(
            state.validate_saved_state(),
            Err("dig_warp_map_name AzaleaTown cannot be saved without dig_warp_index".to_string())
        );

        state = GameState::default();
        state.dig_warp_index = Some(2);
        assert_eq!(
            state.validate_saved_state(),
            Err("dig_warp_index 2 cannot be saved without dig_warp_map_name".to_string())
        );

        state = GameState::default();
        state.dig_warp_map_name = Some("AzaleaTown".to_string());
        state.dig_warp_index = Some(2);
        assert_eq!(state.validate_saved_state(), Ok(()));

        state = GameState::default();
        state.link_session.active_room = Some("Trade Center".to_string());
        assert_eq!(
            state.validate_saved_state(),
            Err("link_session.active_room has invalid token 'Trade Center'".to_string())
        );

        state = GameState::default();
        state.link_session.active_room = Some("TradeCenter".to_string());
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "link_session.active_room TradeCenter cannot be saved with link_mode 0".to_string()
            )
        );

        state = GameState::default();
        state
            .map_block_overrides
            .insert("Route 29".to_string(), BTreeMap::new());
        assert_eq!(
            state.validate_saved_state(),
            Err("map_block_overrides map has invalid token 'Route 29'".to_string())
        );

        state = GameState::default();
        state
            .map_block_overrides
            .insert("fallbackRoute29".to_string(), BTreeMap::new());
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "map_block_overrides map token 'fallbackRoute29' uses reserved runtime payload prefix"
                    .to_string()
            )
        );

        state = GameState::default();
        state.map_object_overrides.insert(
            "Route29".to_string(),
            OverworldObjectMapMemory {
                hidden_object_identifiers: BTreeSet::from(["YOUNGSTER JOEY".to_string()]),
                ..OverworldObjectMapMemory::default()
            },
        );
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "map_object_overrides[Route29].hidden_object_identifiers has invalid token 'YOUNGSTER JOEY'"
                    .to_string()
            )
        );

        state = GameState::default();
        state.map_object_overrides.insert(
            "Route29".to_string(),
            OverworldObjectMapMemory {
                objects: BTreeMap::from([(
                    "LYRA".to_string(),
                    OverworldObjectMemory {
                        x: 1,
                        y: 1,
                        tile: Some(TilePosition::new(1, 1)),
                        facing: None,
                    },
                )]),
                hidden_object_identifiers: BTreeSet::from(["LYRA".to_string()]),
                ..OverworldObjectMapMemory::default()
            },
        );
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "map_object_overrides[Route29] object LYRA cannot be both overridden and hidden"
                    .to_string()
            )
        );

        state = GameState::default();
        state.map_object_overrides.insert(
            "Route29".to_string(),
            OverworldObjectMapMemory {
                following: Some(OverworldFollowMemory {
                    leader_object_id: "PLAYER".to_string(),
                    follower_object_id: "PLAYER".to_string(),
                }),
                ..OverworldObjectMapMemory::default()
            },
        );
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "map_object_overrides[Route29].following leader and follower cannot both be PLAYER"
                    .to_string()
            )
        );

        state = GameState::default();
        state.map_object_overrides.insert(
            "Route29".to_string(),
            OverworldObjectMapMemory {
                hidden_object_identifiers: BTreeSet::from(["PLAYER".to_string()]),
                following: Some(OverworldFollowMemory {
                    leader_object_id: "PLAYER".to_string(),
                    follower_object_id: "LYRA".to_string(),
                }),
                ..OverworldObjectMapMemory::default()
            },
        );
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "map_object_overrides[Route29].following leader PLAYER cannot be hidden"
                    .to_string()
            )
        );

        state = GameState::default();
        state.map_object_overrides.insert(
            "Route29".to_string(),
            OverworldObjectMapMemory {
                hidden_object_identifiers: BTreeSet::from(["LYRA".to_string()]),
                following: Some(OverworldFollowMemory {
                    leader_object_id: "PLAYER".to_string(),
                    follower_object_id: "LYRA".to_string(),
                }),
                ..OverworldObjectMapMemory::default()
            },
        );
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "map_object_overrides[Route29].following follower LYRA cannot be hidden"
                    .to_string()
            )
        );

        state = GameState::default();
        state.battle_tower.challenge_state = 5;
        assert_eq!(
            state.validate_saved_state(),
            Err("battle_tower.challenge_state 5 is outside Crystal range 0..4".to_string())
        );

        state = GameState::default();
        state.battle_tower.save_file_flags = 0x04;
        assert_eq!(
            state.validate_saved_state(),
            Err("battle_tower.save_file_flags 0x04 contains unknown bits 0x04".to_string())
        );

        state = GameState::default();
        state.battle_tower.record_reset_counter = 3;
        assert_eq!(
            state.validate_saved_state(),
            Err("battle_tower.record_reset_counter 3 is outside Crystal range 0..2".to_string())
        );

        state = GameState::default();
        state.battle_tower.reward_item = "RARE CANDY".to_string();
        assert_eq!(
            state.validate_saved_state(),
            Err("battle_tower.reward_item has invalid token 'RARE CANDY'".to_string())
        );

        state = GameState::default();
        state.battle_tower.last_rule_failure = Some("StaleFailureText".to_string());
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "battle_tower.last_rule_failure StaleFailureText is not a saved Battle Tower rule failure"
                    .to_string()
            )
        );

        state = GameState::default();
        state
            .battle_tower
            .mobile_flags
            .insert("function999999".to_string());
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "battle_tower.mobile_flags function999999 is not a saved Battle Tower mobile flag"
                    .to_string()
            )
        );

        state = GameState::default();
        state.battle_tower.last_rule_failure = Some("YouCantTakeAnEggText".to_string());
        state
            .battle_tower
            .mobile_flags
            .insert("function103780".to_string());
        assert_eq!(state.validate_saved_state(), Ok(()));

        state = GameState::default();
        state.battle_tower.selected_party_indexes = vec![0, PARTY_SIZE];
        assert_eq!(
            state.validate_saved_state(),
            Err(format!(
                "battle_tower.selected_party_indexes contains {PARTY_SIZE}, outside party range 0..{PARTY_SIZE}"
            ))
        );

        state = GameState::default();
        state.battle_tower.selected_party_indexes = vec![1, 1];
        assert_eq!(
            state.validate_saved_state(),
            Err("battle_tower.selected_party_indexes contains duplicate party index 1".to_string())
        );

        state = GameState::default();
        state.battle_tower.record_streaks = vec![7];
        state.battle_tower.record_outcomes = vec![true, false];
        state.battle_tower.record_days = vec![4];
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "battle_tower record vectors have inconsistent lengths: streaks 1, outcomes 2, days 1"
                    .to_string()
            )
        );

        state = GameState::default();
        state
            .bug_contest
            .selected_contestant_flags
            .push("EVENT BUG_CONTESTANT".to_string());
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "bug_contest.selected_contestant_flags invalid flag name EVENT BUG_CONTESTANT"
                    .to_string()
            )
        );

        state = GameState::default();
        state.bug_contest.timer_seconds_remaining = 60;
        assert_eq!(
            state.validate_saved_state(),
            Err("bug_contest.timer_seconds_remaining 60 is outside clock range 0..59".to_string())
        );

        state = GameState::default();
        state.bug_contest.caught_species = Some("SCYTHER".to_string());
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "bug_contest.caught_species SCYTHER cannot be saved without caught_mon".to_string()
            )
        );

        state = GameState::default();
        let mut species = crate::models::PokemonSpecies::new_for_tests(
            "SCYTHER",
            crate::models::BaseStats::new(70, 110, 80, 55, 80, 105),
        );
        species.int_id = 123;
        state.bug_contest.caught_mon = Some(Pokemon::new_for_tests(
            species,
            14,
            crate::models::Dv::default(),
        ));
        state.bug_contest.caught_species = Some("PINSIR".to_string());
        state.bug_contest.caught_level = Some(14);
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "bug_contest.caught_species PINSIR does not match caught_mon species SCYTHER"
                    .to_string()
            )
        );

        state = GameState::default();
        state.day_care.man.initial_level = 5;
        assert_eq!(
            state.validate_saved_state(),
            Err("day_care.man.initial_level 5 cannot be saved without a Pokemon".to_string())
        );

        state = GameState::default();
        state.day_care.lady.steps = 1;
        assert_eq!(
            state.validate_saved_state(),
            Err("day_care.lady.steps 1 cannot be saved without a Pokemon".to_string())
        );

        state = GameState::default();
        state.day_care.man.active = true;
        assert_eq!(
            state.validate_saved_state(),
            Err("day_care.man.active cannot be saved without a Pokemon".to_string())
        );

        state = GameState::default();
        let mut day_care_species = crate::models::PokemonSpecies::new_for_tests(
            "DITTO",
            crate::models::BaseStats::new(48, 48, 48, 48, 48, 48),
        );
        day_care_species.int_id = 132;
        state.day_care.man.pokemon = Some(Pokemon::new_for_tests(
            day_care_species,
            5,
            crate::models::Dv::default(),
        ));
        state.day_care.man.initial_level = 5;
        assert_eq!(
            state.validate_saved_state(),
            Err("day_care.man.active must be true when a Pokemon is deposited".to_string())
        );

        state = GameState::default();
        state.day_care.egg_present = true;
        assert_eq!(
            state.validate_saved_state(),
            Err("day_care.egg_present cannot be saved without both residents active".to_string())
        );

        state = GameState::default();
        state.day_care.compatibility_score = 70;
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "day_care.compatibility_score 70 cannot be saved without both residents active"
                    .to_string()
            )
        );

        state = GameState::default();
        state.day_care.steps_until_next_egg = 255;
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "day_care.steps_until_next_egg 255 cannot be saved without both residents active"
                    .to_string()
            )
        );

        state = GameState::default();
        state.day_care.steps_since_last_egg = 1;
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "day_care.steps_since_last_egg 1 cannot be saved without both residents active"
                    .to_string()
            )
        );

        state = GameState::default();
        state.day_care.last_interaction = Some(DayCareInteractionState {
            caretaker: "elder".to_string(),
            action: "inspect".to_string(),
            success: false,
            pokemon: None,
            level: None,
            reason: Some("empty".to_string()),
        });
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "day_care.last_interaction.caretaker elder is not a saved Day Care caretaker"
                    .to_string()
            )
        );

        state = GameState::default();
        state.day_care.last_interaction = Some(DayCareInteractionState {
            caretaker: "man".to_string(),
            action: "deposit".to_string(),
            success: true,
            pokemon: Some("DITTO".to_string()),
            level: None,
            reason: None,
        });
        assert_eq!(
            state.validate_saved_state(),
            Err("day_care.last_interaction.action deposit requires pokemon and level".to_string())
        );

        state = GameState::default();
        state.day_care.last_interaction = Some(DayCareInteractionState {
            caretaker: "lady".to_string(),
            action: "collect_egg".to_string(),
            success: true,
            pokemon: None,
            level: None,
            reason: None,
        });
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "day_care.last_interaction.action collect_egg requires caretaker man, got lady"
                    .to_string()
            )
        );

        state = GameState::default();
        state.day_care.last_interaction = Some(DayCareInteractionState {
            caretaker: "man".to_string(),
            action: "withdraw".to_string(),
            success: false,
            pokemon: Some("DITTO".to_string()),
            level: Some(5),
            reason: Some("party_full".to_string()),
        });
        assert_eq!(state.validate_saved_state(), Ok(()));

        state = GameState::default();
        state.mystery_gift.stored_item = Some("GOLD BERRY".to_string());
        assert_eq!(
            state.validate_saved_state(),
            Err("mystery_gift.stored_item has invalid token 'GOLD BERRY'".to_string())
        );

        state = GameState::default();
        state.mobile_link.mode = Some("MOBILE MODE".to_string());
        assert_eq!(
            state.validate_saved_state(),
            Err("mobile_link.mode has invalid token 'MOBILE MODE'".to_string())
        );

        state = GameState::default();
        state.mobile_link.adapter_status = "LOGGED IN".to_string();
        assert_eq!(
            state.validate_saved_state(),
            Err("mobile_link.adapter_status has invalid token 'LOGGED IN'".to_string())
        );

        state = GameState::default();
        state.mobile_link.mode = Some("legacy_mobile".to_string());
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "mobile_link.mode token 'legacy_mobile' uses reserved runtime payload prefix"
                    .to_string()
            )
        );

        state = GameState::default();
        state.mobile_link.adapter_status = "fallback_ready".to_string();
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "mobile_link.adapter_status token 'fallback_ready' uses reserved runtime payload prefix"
                    .to_string()
            )
        );

        state = GameState::default();
        state.mobile_link.login_password = "EIGHTEEN-CHARS!!!!".to_string();
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "mobile_link.login_password length 18 exceeds Crystal mobile password limit 17"
                    .to_string()
            )
        );

        state = GameState::default();
        state.mobile_link.login_password = " PASSWORD".to_string();
        assert_eq!(
            state.validate_saved_state(),
            Err("mobile_link.login_password must be exact text".to_string())
        );

        state = GameState::default();
        state.mobile_link.terminated = true;
        assert_eq!(
            state.validate_saved_state(),
            Err("mobile_link.terminated cannot be saved before a mobile handshake".to_string())
        );

        state = GameState::default();
        state.mobile_link.mode = Some("init".to_string());
        assert_eq!(
            state.validate_saved_state(),
            Err("mobile_link.mode init cannot be saved before a mobile handshake".to_string())
        );

        state = GameState::default();
        state.mobile_link.adapter_status = "ready".to_string();
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "mobile_link.adapter_status ready cannot be saved before a mobile handshake"
                    .to_string()
            )
        );

        state = GameState::default();
        state.mobile_link.leaderboard.push(MobileBattleTowerRecord {
            streak: 7,
            outcome: "WIN STREAK".to_string(),
            day: 1,
        });
        assert_eq!(
            state.validate_saved_state(),
            Err("mobile_link.leaderboard[0].outcome has invalid token 'WIN STREAK'".to_string())
        );

        state = GameState::default();
        state.mobile_link.leaderboard.push(MobileBattleTowerRecord {
            streak: 7,
            outcome: "draw".to_string(),
            day: 1,
        });
        assert_eq!(
	            state.validate_saved_state(),
	            Err(
	                "mobile_link.leaderboard[0].outcome draw is not a saved mobile Battle Tower outcome"
	                    .to_string()
	            )
	        );

        state = GameState::default();
        state.mobile_link.leaderboard.push(MobileBattleTowerRecord {
            streak: 7,
            outcome: "win".to_string(),
            day: 7,
        });
        assert_eq!(
            state.validate_saved_state(),
            Err("mobile_link.leaderboard[0].day 7 is outside weekday range 0..6".to_string())
        );

        state = GameState::default();
        state.mobile_link.leaderboard.push(MobileBattleTowerRecord {
            streak: 7,
            outcome: "loss".to_string(),
            day: 1,
        });
        assert_eq!(
            state.validate_saved_state(),
            Err("mobile_link.leaderboard has 1 records before a mobile handshake".to_string())
        );

        state = GameState::default();
        state.magikarp_record.current_inches = 12;
        assert_eq!(
            state.validate_saved_state(),
            Err("magikarp_record.current_inches 12 is outside inches range 0..11".to_string())
        );

        state = GameState::default();
        state.magikarp_record.best_inches = 12;
        assert_eq!(
            state.validate_saved_state(),
            Err("magikarp_record.best_inches 12 is outside inches range 0..11".to_string())
        );

        state = GameState::default();
        state.buenas_password.generation_day = 7;
        assert_eq!(
            state.validate_saved_state(),
            Err("buenas_password.generation_day 7 is outside weekday range 0..6".to_string())
        );

        state = GameState::default();
        state.buenas_password.option_index = 1;
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "buenas_password.option_index 1 cannot be saved before a password is generated"
                    .to_string()
            )
        );

        state = GameState::default();
        state.buenas_password.category_index = 1;
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "buenas_password.category_index 1 cannot be saved before a password is generated"
                    .to_string()
            )
        );

        state = GameState::default();
        state.bag.items.insert("POTION".to_string(), 100);
        assert_eq!(
            state.validate_saved_state(),
            Err("invalid saved bag: items.POTION quantity 100 exceeds stack limit 99".to_string())
        );

        state = GameState::default();
        let mut stored_species = crate::models::PokemonSpecies::new_for_tests(
            "CHIKORITA",
            crate::models::BaseStats::new(45, 49, 65, 45, 49, 65),
        );
        stored_species.int_id = 152;
        let mut stored_pokemon = Pokemon::new_for_tests(
            stored_species,
            5,
            crate::models::Dv::from_non_hp(1, 2, 3, 4),
        );
        stored_pokemon.level = 0;
        state.storage.party.pokemon[0] = Some(stored_pokemon);
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "invalid saved storage: party slot 0: pokemon.level 0 is outside range 1..100"
                    .to_string()
            )
        );

        state = GameState::default();
        let party_gap_pokemon = Pokemon::new_for_tests(
            crate::models::PokemonSpecies::new_for_tests(
                "CHIKORITA",
                crate::models::BaseStats::new(45, 49, 65, 45, 49, 65),
            ),
            5,
            crate::models::Dv::from_non_hp(1, 2, 3, 4),
        );
        state.storage.party.pokemon[1] = Some(party_gap_pokemon);
        assert_eq!(
            state.validate_saved_state(),
            Err("invalid saved storage: party slot 1 is filled after empty slot 0".to_string())
        );

        state = GameState::default();
        state.party.pokemon[0] = Some(PartyPokemonRef {
            species: "CHIKORITA".to_string(),
            level: 6,
        });
        assert_eq!(
            state.validate_saved_state(),
            Err("saved party projection does not match authoritative storage".to_string())
        );

        state = GameState {
            current_pc_box: MAX_PC_BOXES,
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err(format!(
                "current_pc_box {MAX_PC_BOXES} is outside PC box range 0..{MAX_PC_BOXES}"
            ))
        );

        state = GameState::default();
        state.roaming_pokemon.push(RoamingPokemonState {
            species: "RAIK OU".to_string(),
            level: 40,
            map_group: 1,
            map_number: 1,
            hp: 1,
            dvs: 0,
        });
        assert_eq!(
            state.validate_saved_state(),
            Err("roaming_pokemon[0].species has invalid token 'RAIK OU'".to_string())
        );

        state = GameState::default();
        state.roaming_pokemon.push(RoamingPokemonState {
            species: "RAIKOU".to_string(),
            level: 0,
            map_group: 1,
            map_number: 1,
            hp: 1,
            dvs: 0,
        });
        assert_eq!(
            state.validate_saved_state(),
            Err("roaming_pokemon[0].level must be nonzero".to_string())
        );

        state = GameState::default();
        state.roaming_pokemon.push(RoamingPokemonState {
            species: "RAIKOU".to_string(),
            level: 40,
            map_group: 1,
            map_number: 0,
            hp: 1,
            dvs: 0,
        });
        assert_eq!(
            state.validate_saved_state(),
            Err("roaming_pokemon[0].map_number must be nonzero".to_string())
        );

        state = GameState {
            overworld: OverworldMemory::Active {
                map_name: "Route 29".to_string(),
                tile: TilePosition::new(2, 2),
                facing: Direction::Down,
                mode: MovementMode::Normal,
            },
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err("overworld.active.map_name has invalid token 'Route 29'".to_string())
        );

        let inactive_state = GameState::default();
        assert_eq!(
            validate_saved_last_talked_object_reference(
                &inactive_state,
                "ROUTE29_YOUNGSTER1",
                |_| true,
                |_, _| true,
            ),
            Err(LastTalkedObjectSaveError::InactiveOverworld {
                object_id: "ROUTE29_YOUNGSTER1".to_string(),
            })
        );

        let active_state = GameState {
            overworld: OverworldMemory::Active {
                map_name: "Route29".to_string(),
                tile: TilePosition::new(2, 2),
                facing: Direction::Down,
                mode: MovementMode::Normal,
            },
            ..GameState::default()
        };
        assert_eq!(
            validate_saved_last_talked_object_reference(
                &active_state,
                "ROUTE29_YOUNGSTER1",
                |_| false,
                |_, _| true,
            ),
            Err(LastTalkedObjectSaveError::MissingMap {
                map_name: "Route29".to_string(),
            })
        );
        assert_eq!(
            validate_saved_last_talked_object_reference(
                &active_state,
                "ROUTE29_YOUNGSTER1",
                |_| true,
                |_, _| false,
            ),
            Err(LastTalkedObjectSaveError::MissingObject {
                map_name: "Route29".to_string(),
                object_id: "ROUTE29_YOUNGSTER1".to_string(),
            })
        );
        assert_eq!(
            validate_saved_last_talked_object_reference(
                &active_state,
                "ROUTE29_YOUNGSTER1",
                |map_name| map_name == "Route29",
                |map_name, object_id| {
                    map_name == "Route29" && object_id == "ROUTE29_YOUNGSTER1"
                },
            ),
            Ok(())
        );

        let script_body = serde_json::json!([
            { "command": "jumptext", "args": ["Route29Text"] },
            { "command": "end" }
        ]);
        assert_eq!(
            validate_saved_compiled_script_command_reference(
                &script_body,
                "script_runtime.effects[0].source_script",
                "Route29Script",
                2,
            ),
            Err(ScriptCommandReferenceError::CommandIndexOutOfBounds {
                path: "script_runtime.effects[0].source_script".to_string(),
                script_label: "Route29Script".to_string(),
                command_index: 2,
                command_count: 2,
            })
        );
        assert_eq!(
            validate_saved_compiled_script_command_name_reference(
                &script_body,
                "script_runtime.effects[0].source_script",
                "Route29Script",
                0,
                "writetext",
            ),
            Err(ScriptCommandReferenceError::CommandMismatch {
                path: "script_runtime.effects[0].source_script".to_string(),
                script_label: "Route29Script".to_string(),
                command_index: 0,
                saved_command: "writetext".to_string(),
                compiled_command: "jumptext".to_string(),
            })
        );
        assert_eq!(
            validate_saved_compiled_script_command_payload_reference(
                &script_body,
                "script_runtime.effects[0].source_script",
                "Route29Script",
                0,
                "jumptext",
                &["OtherText".to_string()],
            ),
            Err(ScriptCommandReferenceError::ArgsMismatch {
                path: "script_runtime.effects[0].source_script".to_string(),
                script_label: "Route29Script".to_string(),
                command_index: 0,
                saved_args: vec!["OtherText".to_string()],
                compiled_args: vec!["Route29Text".to_string()],
            })
        );
        assert_eq!(
            validate_saved_compiled_script_command_payload_reference(
                &script_body,
                "script_runtime.effects[0].source_script",
                "Route29Script",
                0,
                "jumptext",
                &["Route29Text".to_string()],
            ),
            Ok(())
        );
        assert_eq!(
            validate_saved_compiled_script_return_reference(
                &script_body,
                "script_runtime.call_stack[0].source_script",
                "Route29Script",
                2,
            ),
            Ok(())
        );
        assert_eq!(
            validate_saved_compiled_script_return_reference(
                &script_body,
                "script_runtime.call_stack[0].source_script",
                "Route29Script",
                3,
            ),
            Err(ScriptCommandReferenceError::CommandIndexOutOfBounds {
                path: "script_runtime.call_stack[0].source_script".to_string(),
                script_label: "Route29Script".to_string(),
                command_index: 3,
                command_count: 2,
            })
        );

        let mut trainer_species = crate::models::PokemonSpecies::new_for_tests(
            "PIDGEY",
            crate::models::BaseStats::new(40, 45, 40, 56, 35, 35),
        );
        trainer_species.int_id = 16;
        let mut expected_enemy =
            Pokemon::new_for_tests(trainer_species.clone(), 9, crate::models::Dv::default());
        expected_enemy.original_trainer_name = "FALKNER".to_string();
        expected_enemy.original_trainer_id = 1001;
        expected_enemy.moves = vec![crate::models::pokemon::LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 35,
            pp_ups: 0,
        }];
        let saved_enemy = expected_enemy.clone();
        assert_eq!(
            validate_saved_trainer_enemy_party_identity(
                "FALKNER1",
                &[],
                &saved_enemy,
                &[expected_enemy.clone()],
            ),
            Err(TrainerEnemyPartySaveError::PartyLengthMismatch {
                trainer_id: "FALKNER1".to_string(),
                saved_len: 0,
                expected_len: 1,
            })
        );
        let mut wrong_species = saved_enemy.clone();
        wrong_species.species.id = "SPEAROW".to_string();
        assert_eq!(
            validate_saved_trainer_enemy_party_identity(
                "FALKNER1",
                &[wrong_species],
                &saved_enemy,
                &[expected_enemy.clone()],
            ),
            Err(TrainerEnemyPartySaveError::SpeciesMismatch {
                trainer_id: "FALKNER1".to_string(),
                index: 0,
                saved_species: "SPEAROW".to_string(),
                expected_species: "PIDGEY".to_string(),
            })
        );
        let mut active_enemy = saved_enemy.clone();
        active_enemy.level = 10;
        assert_eq!(
            validate_saved_trainer_enemy_party_identity(
                "FALKNER1",
                &[saved_enemy.clone()],
                &active_enemy,
                &[expected_enemy.clone()],
            ),
            Err(TrainerEnemyPartySaveError::ActiveEnemyMissingFromParty {
                trainer_id: "FALKNER1".to_string(),
                species: "PIDGEY".to_string(),
                level: 10,
            })
        );
        assert_eq!(
            validate_saved_trainer_enemy_party_identity(
                "FALKNER1",
                &[saved_enemy.clone()],
                &saved_enemy,
                &[expected_enemy],
            ),
            Ok(())
        );
        assert_eq!(
            validate_saved_pokemon_reference(
                "battle.trainer.enemy_pokemon",
                &saved_enemy,
                |species| {
                    if species == "PIDGEY" {
                        Some(trainer_species.clone())
                    } else {
                        None
                    }
                },
                |_| true,
                |_| true,
                |move_name| {
                    if move_name == "TACKLE" {
                        Some(("TACKLE".to_string(), 35))
                    } else {
                        None
                    }
                },
            ),
            Ok(())
        );
        let mut stale_species = saved_enemy.clone();
        stale_species.species.base_stats.hp += 1;
        assert_eq!(
            validate_saved_pokemon_reference(
                "battle.trainer.enemy_pokemon",
                &stale_species,
                |_| Some(trainer_species.clone()),
                |_| true,
                |_| true,
                |_| Some(("TACKLE".to_string(), 35)),
            ),
            Err(PokemonReferenceSaveError::SpeciesMismatch {
                path: "battle.trainer.enemy_pokemon".to_string(),
                species: "PIDGEY".to_string(),
            })
        );
        let mut held_item = saved_enemy.clone();
        held_item.item = Some("BERRY".to_string());
        assert_eq!(
            validate_saved_pokemon_reference(
                "battle.trainer.enemy_pokemon",
                &held_item,
                |_| Some(trainer_species.clone()),
                |_| false,
                |_| true,
                |_| Some(("TACKLE".to_string(), 35)),
            ),
            Err(PokemonReferenceSaveError::MissingItem {
                path: "battle.trainer.enemy_pokemon".to_string(),
                item_id: "BERRY".to_string(),
            })
        );
        let mut bad_status = saved_enemy.clone();
        bad_status.status = Some("BRN".to_string());
        assert_eq!(
            validate_saved_pokemon_reference(
                "battle.trainer.enemy_pokemon",
                &bad_status,
                |_| Some(trainer_species.clone()),
                |_| true,
                |_| false,
                |_| Some(("TACKLE".to_string(), 35)),
            ),
            Err(PokemonReferenceSaveError::MissingStatus {
                path: "battle.trainer.enemy_pokemon".to_string(),
                status: "BRN".to_string(),
            })
        );
        let mut pp_overflow = saved_enemy.clone();
        pp_overflow.moves[0].current_pp = 40;
        assert_eq!(
            validate_saved_pokemon_reference(
                "battle.trainer.enemy_pokemon",
                &pp_overflow,
                |_| Some(trainer_species.clone()),
                |_| true,
                |_| true,
                |_| Some(("TACKLE".to_string(), 35)),
            ),
            Err(PokemonReferenceSaveError::MovePpOverflow {
                path: "battle.trainer.enemy_pokemon".to_string(),
                index: 0,
                move_name: "TACKLE".to_string(),
                current_pp: 40,
                max_pp: 35,
            })
        );
        assert_eq!(
            validate_saved_pokemon_party_references(
                "battle.trainer.enemy_party",
                &[saved_enemy.clone()],
                |path, _| Err(format!("bad pokemon at {path}")),
            ),
            Err(PokemonPartyReferenceSaveError::Pokemon(
                "bad pokemon at battle.trainer.enemy_party[0]".to_string()
            ))
        );
        assert_eq!(
            validate_saved_map_reference("overworld.active.map_name", "Route29", None),
            Err(MapReferenceSaveError::MissingMap {
                path: "overworld.active.map_name".to_string(),
                map_name: "Route29".to_string(),
            })
        );
        assert_eq!(
            validate_saved_map_reference("overworld.active.map_name", "Route29", Some("Route30"),),
            Err(MapReferenceSaveError::MapIdMismatch {
                path: "overworld.active.map_name".to_string(),
                map_name: "Route29".to_string(),
                compiled_id: "Route30".to_string(),
            })
        );
        assert_eq!(
            validate_saved_warp_reference("dig_warp_index", "Route29", 2, Some("Route29"), |_| {
                false
            }),
            Err(MapReferenceSaveError::MissingWarp {
                path: "dig_warp_index".to_string(),
                map_name: "Route29".to_string(),
                warp_index: 2,
            })
        );
        assert_eq!(
            validate_saved_map_object_reference(
                "map_object_overrides.hidden",
                "Route29",
                "ROUTE29_YOUNGSTER1",
                |_| false,
            ),
            Err(MapReferenceSaveError::MissingObject {
                path: "map_object_overrides.hidden".to_string(),
                map_name: "Route29".to_string(),
                object_id: "ROUTE29_YOUNGSTER1".to_string(),
            })
        );
        assert_eq!(
            validate_saved_catalog_reference(
                "script_runtime.last_special_routine",
                "UnknownRoutine",
                "special routines",
                |_| false,
            ),
            Err(CatalogReferenceSaveError::Missing {
                path: "script_runtime.last_special_routine".to_string(),
                value: "UnknownRoutine".to_string(),
                catalog: "special routines".to_string(),
            })
        );
        assert_eq!(
            validate_saved_optional_catalog_reference(
                "battle.trainer.seen_text",
                "",
                "text",
                |_| { false }
            ),
            Ok(())
        );
        assert_eq!(
            validate_saved_optional_catalog_reference(
                "battle.trainer.seen_text",
                "UnknownText",
                "text",
                |_| false,
            ),
            Err(CatalogReferenceSaveError::Missing {
                path: "battle.trainer.seen_text".to_string(),
                value: "UnknownText".to_string(),
                catalog: "text".to_string(),
            })
        );
        assert_eq!(
            validate_saved_exact_catalog_reference(
                "bag.items",
                "POTION",
                "items",
                "item script_name",
                None,
            ),
            Err(ExactCatalogReferenceSaveError::Missing {
                path: "bag.items".to_string(),
                value: "POTION".to_string(),
                catalog: "items".to_string(),
            })
        );
        assert_eq!(
            validate_saved_exact_catalog_reference(
                "bag.items",
                "POTION",
                "items",
                "item script_name",
                Some("DIFFERENT_SCRIPT_NAME".to_string()),
            ),
            Err(ExactCatalogReferenceSaveError::Mismatch {
                path: "bag.items".to_string(),
                value: "POTION".to_string(),
                compiled_field: "item script_name".to_string(),
                compiled_value: "DIFFERENT_SCRIPT_NAME".to_string(),
            })
        );
        assert_eq!(
            validate_saved_audio_reference(
                "script_runtime.current_music",
                "MUSIC_ROUTE_29",
                "Music",
                None
            ),
            Err(AudioReferenceSaveError::MissingAudio {
                path: "script_runtime.current_music".to_string(),
                audio_id: "MUSIC_ROUTE_29".to_string(),
            })
        );
        assert_eq!(
            validate_saved_audio_reference(
                "script_runtime.current_music",
                "SFX_TACKLE",
                "Music",
                Some("SoundEffect"),
            ),
            Err(AudioReferenceSaveError::KindMismatch {
                path: "script_runtime.current_music".to_string(),
                audio_id: "SFX_TACKLE".to_string(),
                actual_kind: "SoundEffect".to_string(),
                expected_kind: "Music".to_string(),
            })
        );
        assert_eq!(
            validate_saved_overworld_references(
                &OverworldMemory::Active {
                    map_name: "Route29".to_string(),
                    tile: TilePosition { x: 18, y: 8 },
                    facing: Direction::Down,
                    mode: MovementMode::Normal,
                },
                |_| Some((20, 10)),
            ),
            Ok(())
        );
        assert_eq!(
            validate_saved_overworld_references(
                &OverworldMemory::Active {
                    map_name: "Route29".to_string(),
                    tile: TilePosition { x: 20, y: 4 },
                    facing: Direction::Down,
                    mode: MovementMode::Normal,
                },
                |_| Some((20, 10)),
            ),
            Err(OverworldReferenceSaveError::TileOutOfBounds {
                map_name: "Route29".to_string(),
                x: 20,
                y: 4,
                width: 20,
                height: 10,
            })
        );
        assert_eq!(
            validate_saved_overworld_references(
                &OverworldMemory::Active {
                    map_name: "Route29".to_string(),
                    tile: TilePosition { x: -1, y: 4 },
                    facing: Direction::Down,
                    mode: MovementMode::Normal,
                },
                |_| Some((20, 10)),
            ),
            Err(OverworldReferenceSaveError::TileOutOfBounds {
                map_name: "Route29".to_string(),
                x: -1,
                y: 4,
                width: 20,
                height: 10,
            })
        );
        assert_eq!(
            validate_saved_overworld_references(
                &OverworldMemory::Active {
                    map_name: "Route29".to_string(),
                    tile: TilePosition { x: 3, y: 4 },
                    facing: Direction::Down,
                    mode: MovementMode::Normal,
                },
                |_| Some((20, 10)),
            ),
            Ok(())
        );
        assert_eq!(
            validate_saved_block_overrides(
                "Route29",
                &BTreeMap::from([((1, 1), 0x12)]),
                |_| Some((10, 10, "Overworld".to_string())),
                |_| true,
                |_, _| false,
            ),
            Err(BlockOverrideSaveError::MissingMetatile {
                map_name: "Route29".to_string(),
                x: 1,
                y: 1,
                block_id: 0x12,
                tileset_name: "Overworld".to_string(),
            })
        );
        assert_eq!(
            validate_saved_object_overrides(
                "Route29",
                &OverworldObjectMapMemory {
                    objects: BTreeMap::from([(
                        "YOUNGSTER".to_string(),
                        OverworldObjectMemory {
                            x: 10,
                            y: 1,
                            tile: Some(TilePosition::new(10, 1)),
                            facing: None,
                        },
                    )]),
                    ..OverworldObjectMapMemory::default()
                },
                |_| Some((10, 10)),
                |_| true,
            ),
            Err(ObjectOverrideSaveError::RuntimeTileOutOfBounds {
                map_name: "Route29".to_string(),
                object_id: "YOUNGSTER".to_string(),
                raw_x: 10,
                raw_y: 1,
                runtime_x: 10,
                runtime_y: 1,
                width: 10,
                height: 10,
            })
        );
        assert_eq!(
            validate_saved_object_overrides(
                "Route29",
                &OverworldObjectMapMemory {
                    objects: BTreeMap::from([(
                        "YOUNGSTER".to_string(),
                        OverworldObjectMemory {
                            x: 1,
                            y: 0,
                            tile: Some(TilePosition::new(1, 0)),
                            facing: None,
                        },
                    )]),
                    ..OverworldObjectMapMemory::default()
                },
                |_| Some((2, 2)),
                |_| true,
            ),
            Ok(())
        );
        assert_eq!(
            validate_saved_object_overrides(
                "CherrygroveCity",
                &OverworldObjectMapMemory {
                    following: Some(OverworldFollowMemory {
                        leader_object_id: "CHERRYGROVECITY_GRAMPS".to_string(),
                        follower_object_id: "PLAYER".to_string(),
                    }),
                    ..OverworldObjectMapMemory::default()
                },
                |_| Some((40, 18)),
                |object_id| object_id == "CHERRYGROVECITY_GRAMPS",
            ),
            Ok(())
        );
        assert_eq!(
            validate_saved_object_overrides(
                "CherrygroveCity",
                &OverworldObjectMapMemory {
                    following: Some(OverworldFollowMemory {
                        leader_object_id: "CHERRYGROVECITY_GRAMPS".to_string(),
                        follower_object_id: "MISSING_FOLLOWER".to_string(),
                    }),
                    ..OverworldObjectMapMemory::default()
                },
                |_| Some((40, 18)),
                |object_id| object_id == "CHERRYGROVECITY_GRAMPS",
            ),
            Err(ObjectOverrideSaveError::MissingObject {
                path: "map_object_overrides.following.follower_object_id".to_string(),
                map_name: "CherrygroveCity".to_string(),
                object_id: "MISSING_FOLLOWER".to_string(),
            })
        );
        assert_eq!(
            validate_saved_object_overrides(
                "Route29",
                &OverworldObjectMapMemory {
                    objects: BTreeMap::from([(
                        "YOUNGSTER".to_string(),
                        OverworldObjectMemory {
                            x: 1,
                            y: 0,
                            tile: Some(TilePosition::new(-1, 9)),
                            facing: None,
                        },
                    )]),
                    ..OverworldObjectMapMemory::default()
                },
                |_| Some((2, 2)),
                |_| true,
            ),
            Ok(())
        );
        assert_eq!(
            validate_saved_object_overrides(
                "Route29",
                &OverworldObjectMapMemory {
                    objects: BTreeMap::from([(
                        "YOUNGSTER".to_string(),
                        OverworldObjectMemory {
                            x: 40_000,
                            y: 1,
                            tile: Some(TilePosition::new(0, 1)),
                            facing: None,
                        },
                    )]),
                    ..OverworldObjectMapMemory::default()
                },
                |_| Some((30_000, 10)),
                |_| true,
            ),
            Err(ObjectOverrideSaveError::CoordinateOutOfRange {
                map_name: "Route29".to_string(),
                object_id: "YOUNGSTER".to_string(),
                x: 40_000,
                y: 1,
            })
        );
        assert_eq!(
            validate_saved_warp_reference(
                "dig_warp_index",
                "Route29",
                2,
                Some("Route29"),
                |warp| { warp == 2 }
            ),
            Ok(())
        );
        let mut storage = PokemonStorage::default();
        storage.party.pokemon[1] = Some(saved_enemy.clone());
        assert_eq!(
            validate_saved_storage_references(&storage, |path, _| Err(format!(
                "bad pokemon at {path}"
            ))),
            Err(StorageReferenceSaveError::Pokemon(
                "bad pokemon at storage.party[1]".to_string()
            ))
        );

        let mut contest = BugContestState::default();
        contest.party_backup.push(saved_enemy.clone());
        contest.second_party_species = Some("SCYTHER".to_string());
        contest
            .selected_contestant_flags
            .push("EVENT_CONTESTANT".to_string());
        assert_eq!(
            validate_saved_bug_contest_references(
                &contest,
                |_, _| Ok(()),
                |species| species == "SCYTHER",
                |flag| flag == "EVENT_CONTESTANT",
            ),
            Ok(())
        );
        contest.caught_species = Some("PINSIR".to_string());
        assert_eq!(
            validate_saved_bug_contest_references(
                &contest,
                |_, _| Ok(()),
                |species| species == "SCYTHER",
                |_| true,
            ),
            Err(BugContestReferenceSaveError::MissingSpecies {
                path: "bug_contest.caught_species".to_string(),
                species: "PINSIR".to_string(),
            })
        );

        let mut day_care = DayCareState::default();
        day_care.man.pokemon = Some(saved_enemy.clone());
        day_care.last_interaction = Some(DayCareInteractionState {
            caretaker: "man".to_string(),
            action: "inspect".to_string(),
            success: true,
            pokemon: Some("PIDGEY".to_string()),
            level: Some(9),
            reason: None,
        });
        assert_eq!(
            validate_saved_day_care_references(
                &day_care,
                |path, _| Err(format!("bad pokemon at {path}")),
                |_| true,
            ),
            Err(DayCareReferenceSaveError::Pokemon(
                "bad pokemon at day_care.man.pokemon".to_string()
            ))
        );
        assert_eq!(
            validate_saved_day_care_references(&day_care, |_, _| Ok(()), |_| false),
            Err(DayCareReferenceSaveError::MissingInteractionSpecies {
                species: "PIDGEY".to_string(),
            })
        );
        assert_eq!(
            validate_saved_wild_battle_origin_reference(
                "BATTLETYPE_FORCEITEM",
                "Route29",
                &saved_enemy,
                |_, _, _| true,
            ),
            Err(WildBattleSaveError::BattleTypeMismatch {
                battle_type: "BATTLETYPE_FORCEITEM".to_string(),
            })
        );
        assert_eq!(
            validate_saved_wild_battle_origin_reference(
                "BATTLETYPE_NORMAL",
                "Route29",
                &saved_enemy,
                |_, _, _| false,
            ),
            Err(WildBattleSaveError::MissingEncounter {
                map_name: "Route29".to_string(),
                species: "PIDGEY".to_string(),
                level: 9,
            })
        );
        assert_eq!(
            validate_saved_wild_battle_origin_reference(
                "BATTLETYPE_NORMAL",
                "Route29",
                &saved_enemy,
                |map_name, species, level| {
                    map_name == "Route29" && species == "PIDGEY" && level == 9
                },
            ),
            Ok(())
        );
        assert_eq!(
            validate_saved_static_wild_battle_origin_reference(
                "BATTLETYPE_NORMAL",
                "PIDGEY",
                9,
                "Route29StaticBattle",
                |_, _, _, _| false,
            ),
            Err(StaticWildBattleSaveError::MissingScriptedBattle {
                source_script: "Route29StaticBattle".to_string(),
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                species: "PIDGEY".to_string(),
                level: 9,
            })
        );
        assert_eq!(
            validate_saved_static_wild_battle_origin_reference(
                "BATTLETYPE_NORMAL",
                "PIDGEY",
                9,
                "Route29StaticBattle",
                |source_script, battle_type, species, level| {
                    source_script == "Route29StaticBattle"
                        && battle_type == "BATTLETYPE_NORMAL"
                        && species == "PIDGEY"
                        && level == 9
                },
            ),
            Ok(())
        );
        let saved_trainer_fields = SavedTrainerBattleFields {
            battle_type: "BATTLETYPE_TRAINER",
            trainer_class: "FALKNER",
            event_flag: "EVENT_BEAT_FALKNER",
            seen_text: "FalknerSeenText",
            win_text: "FalknerWinText",
            loss_text: "FalknerLossText",
            callback: "FalknerCallback",
        };
        let compiled_trainer_fields = SavedTrainerBattleFields {
            battle_type: "BATTLETYPE_TRAINER",
            trainer_class: "FALKNER",
            event_flag: "EVENT_BEAT_FALKNER",
            seen_text: "FalknerSeenText",
            win_text: "FalknerWinText",
            loss_text: "FalknerLossText",
            callback: "FalknerCallback",
        };
        assert_eq!(
            validate_saved_trainer_battle_request_fields(
                saved_trainer_fields,
                compiled_trainer_fields,
                "FalknerBattleScript",
            ),
            Ok(())
        );
        assert_eq!(
            validate_saved_trainer_battle_source_reference("MissingBattleScript", |_| false),
            Err(TrainerBattleRequestSaveError::MissingSource {
                source_script: "MissingBattleScript".to_string(),
            })
        );
        assert_eq!(
            validate_saved_trainer_battle_request_field(
                "win_text",
                "WrongWinText",
                "FalknerWinText",
                "FalknerBattleScript",
            ),
            Err(TrainerBattleRequestSaveError::FieldMismatch {
                field: "win_text".to_string(),
                saved: "WrongWinText".to_string(),
                compiled: "FalknerWinText".to_string(),
                source_script: "FalknerBattleScript".to_string(),
            })
        );
        let compiled_trainer = Trainer {
            name: "FALKNER".to_string(),
            trainer_id: "FALKNER1".to_string(),
            trainer_class: "FALKNER".to_string(),
            party: Vec::new(),
            win_quote: "FalknerWinText".to_string(),
            lose_quote: "FalknerLossText".to_string(),
            items: Vec::new(),
            base_reward: 900,
            ai_move_flags: 1,
            ai_item_switch_flags: 2,
            encounter_music: "MUSIC_HIKER_ENCOUNTER".to_string(),
            ai_layers: vec!["AI_BASIC".to_string()],
        };
        assert_eq!(
            validate_saved_trainer_metadata(
                &compiled_trainer,
                SavedTrainerMetadata {
                    trainer_class: "FALKNER",
                    trainer_name: "FALKNER",
                    ai_move_flags: 1,
                    ai_item_switch_flags: 2,
                    ai_layers: &["AI_BASIC".to_string()],
                    reward: 900,
                    encounter_music: "MUSIC_HIKER_ENCOUNTER",
                },
            ),
            Ok(())
        );
        assert_eq!(
            validate_saved_trainer_metadata(
                &compiled_trainer,
                SavedTrainerMetadata {
                    trainer_class: "BIRD_KEEPER",
                    trainer_name: "FALKNER",
                    ai_move_flags: 1,
                    ai_item_switch_flags: 2,
                    ai_layers: &["AI_BASIC".to_string()],
                    reward: 900,
                    encounter_music: "MUSIC_HIKER_ENCOUNTER",
                },
            ),
            Err(TrainerMetadataSaveError::ClassMismatch {
                trainer_id: "FALKNER1".to_string(),
                saved: "BIRD_KEEPER".to_string(),
                compiled: "FALKNER".to_string(),
            })
        );
        assert_eq!(
            validate_saved_trainer_metadata(
                &compiled_trainer,
                SavedTrainerMetadata {
                    trainer_class: "FALKNER",
                    trainer_name: "FALKNER",
                    ai_move_flags: 1,
                    ai_item_switch_flags: 2,
                    ai_layers: &["AI_SMART".to_string()],
                    reward: 900,
                    encounter_music: "MUSIC_HIKER_ENCOUNTER",
                },
            ),
            Err(TrainerMetadataSaveError::AiLayersMismatch {
                trainer_id: "FALKNER1".to_string(),
                saved: vec!["AI_SMART".to_string()],
                compiled: vec!["AI_BASIC".to_string()],
            })
        );

        let mut species = crate::models::PokemonSpecies::new_for_tests(
            "CHIKORITA",
            crate::models::BaseStats::new(45, 49, 49, 45, 65, 65),
        );
        species.int_id = 152;
        let pokemon = Pokemon::new_for_tests(species, 6, crate::models::Dv::default());

        state = GameState {
            battle: BattleMemory::Wild {
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                map_name: "Route29".to_string(),
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon.clone()],
            },
            battle_active_party_index: Some(PARTY_SIZE),
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err(format!(
                "battle_active_party_index {PARTY_SIZE} is outside party range 0..{PARTY_SIZE}"
            ))
        );

        state = GameState {
            battle: BattleMemory::Wild {
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                map_name: "Route29".to_string(),
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon.clone()],
            },
            battle_active_party_index: Some(0),
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err("battle_active_party_index 0 points to empty party slot".to_string())
        );

        state = GameState {
            battle_active_party_index: Some(0),
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err("battle_active_party_index cannot be saved without an active battle".to_string())
        );

        state = GameState {
            battle_active_enemy_party_index: Some(0),
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "battle_active_enemy_party_index cannot be saved without an active battle"
                    .to_string()
            )
        );

        state = GameState {
            battle_rewarded_enemy_party_indices: BTreeSet::from([0]),
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "battle_rewarded_enemy_party_indices cannot be saved without an active battle"
                    .to_string()
            )
        );

        state = GameState {
            battle_escape_attempts: 1,
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err("battle_escape_attempts cannot be saved without an active battle".to_string())
        );

        state = GameState {
            battle_player_stat_drop_guard_turns: 1,
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "battle_player_stat_drop_guard_turns cannot be saved without an active battle"
                    .to_string()
            )
        );

        state = GameState {
            battle_pay_day_money: 1,
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err("battle_pay_day_money cannot be saved without an active battle".to_string())
        );

        state = GameState {
            battle: BattleMemory::Wild {
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                map_name: "Route29".to_string(),
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon.clone()],
            },
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err("battle_active_party_index must be saved for an active battle".to_string())
        );

        state.storage.party.pokemon[0] = Some(pokemon.clone());
        state.sync_party_from_storage();
        state.battle_active_party_index = Some(0);
        assert_eq!(
            state.validate_saved_state(),
            Err("battle_active_enemy_party_index must be saved for an active battle".to_string())
        );

        state = GameState {
            battle: BattleMemory::Wild {
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                map_name: "Route29".to_string(),
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon.clone()],
            },
            battle_active_party_index: Some(0),
            battle_active_enemy_party_index: Some(1),
            ..GameState::default()
        };
        state.storage.party.pokemon[0] = Some(pokemon.clone());
        state.sync_party_from_storage();
        assert_eq!(
            state.validate_saved_state(),
            Err("battle_active_enemy_party_index 1 is outside enemy party range 0..1".to_string())
        );

        state = GameState {
            battle: BattleMemory::Wild {
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                map_name: "Route29".to_string(),
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon.clone()],
            },
            battle_active_party_index: Some(0),
            battle_rewarded_enemy_party_indices: BTreeSet::from([1]),
            ..GameState::default()
        };
        state.storage.party.pokemon[0] = Some(pokemon.clone());
        state.sync_party_from_storage();
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "battle_rewarded_enemy_party_indices contains 1, outside enemy party range 0..1"
                    .to_string()
            )
        );

        state = GameState {
            battle: BattleMemory::Wild {
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                map_name: "Route29".to_string(),
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon.clone()],
            },
            battle_active_party_index: Some(0),
            battle_active_enemy_party_index: Some(0),
            battle_rewarded_enemy_party_indices: BTreeSet::from([0]),
            ..GameState::default()
        };
        state.storage.party.pokemon[0] = Some(pokemon.clone());
        state.sync_party_from_storage();
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "battle_rewarded_enemy_party_indices can only be saved for an active trainer battle"
                    .to_string()
            )
        );

        state = GameState {
            battle: BattleMemory::Wild {
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                map_name: "Route29".to_string(),
                enemy_pokemon: pokemon.clone(),
                enemy_party: Vec::new(),
            },
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err("battle.wild.enemy_party must not be empty".to_string())
        );

        let mut damaged_enemy = pokemon.clone();
        damaged_enemy.hp -= 1;
        state = GameState {
            battle: BattleMemory::Wild {
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                map_name: "Route29".to_string(),
                enemy_pokemon: damaged_enemy,
                enemy_party: vec![pokemon.clone()],
            },
            battle_active_party_index: Some(0),
            battle_active_enemy_party_index: Some(0),
            ..GameState::default()
        };
        state.storage.party.pokemon[0] = Some(pokemon.clone());
        state.sync_party_from_storage();
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "battle_active_enemy_party_index 0 does not match battle enemy_pokemon".to_string()
            )
        );

        state = GameState {
            battle: BattleMemory::Wild {
                battle_type: "BATTLETYPE NORMAL".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                map_name: "Route29".to_string(),
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon.clone()],
            },
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err("battle.wild.battle_type has invalid token 'BATTLETYPE NORMAL'".to_string())
        );

        state = GameState {
            battle: BattleMemory::StaticWild {
                battle_type: "BATTLETYPE_FORCESHINY".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                species: "RED GYARADOS".to_string(),
                level: 30,
                source_script: "LakeOfRageRedGyarados".to_string(),
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon.clone()],
            },
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err("battle.static_wild.species has invalid token 'RED GYARADOS'".to_string())
        );

        state = GameState {
            battle: BattleMemory::StaticWild {
                battle_type: "BATTLETYPE_FORCESHINY".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                species: "CHIKORITA".to_string(),
                level: 0,
                source_script: "LakeOfRageRedGyarados".to_string(),
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon.clone()],
            },
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err("battle.static_wild.level must be nonzero".to_string())
        );

        state = GameState {
            battle: BattleMemory::StaticWild {
                battle_type: "BATTLETYPE_FORCESHINY".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                species: "CYNDAQUIL".to_string(),
                level: 6,
                source_script: "LakeOfRageRedGyarados".to_string(),
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon.clone()],
            },
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "battle.static_wild.species CYNDAQUIL does not match enemy_pokemon species CHIKORITA"
                    .to_string()
            )
        );

        state = GameState {
            battle: BattleMemory::StaticWild {
                battle_type: "BATTLETYPE_FORCESHINY".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                species: "CHIKORITA".to_string(),
                level: 7,
                source_script: "LakeOfRageRedGyarados".to_string(),
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon.clone()],
            },
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err("battle.static_wild.level 7 does not match enemy_pokemon level 6".to_string())
        );

        state = GameState {
            battle: BattleMemory::Trainer {
                battle_type: "BATTLETYPE_TRAINER".to_string(),
                trainer_class: "FALKNER".to_string(),
                trainer_id: "FALKNER1".to_string(),
                trainer_name: "Falkner".to_string(),
                event_flag: "EVENT BEAT_FALKNER".to_string(),
                seen_text: "FalknerSeenText".to_string(),
                win_text: "FalknerWinText".to_string(),
                loss_text: "FalknerLossText".to_string(),
                callback: "FalknerCallback".to_string(),
                source_script: "VioletGymFalkner".to_string(),
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon],
                reward: 900,
                encounter_music: "MUSIC_HIKER_ENCOUNTER".to_string(),
                ai_move_flags: 0,
                ai_item_switch_flags: 0,
                ai_layers: vec!["AI_BASIC".to_string()],
            },
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err("battle.trainer.event_flag invalid flag name EVENT BEAT_FALKNER".to_string())
        );
    }

    #[test]
    fn saved_registered_key_item_must_be_carried_key_item() {
        let mut state = GameState {
            registered_key_item: Some("BICYCLE".to_string()),
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err("registered_key_item BICYCLE is not carried in saved key_items".to_string())
        );

        state.bag.key_items.insert("BICYCLE".to_string(), 1);
        assert_eq!(state.validate_saved_state(), Ok(()));
    }

    #[test]
    fn saved_script_runtime_validates_control_continuation_labels() {
        let mut runtime = ScriptRuntimeMemory {
            next_script: Some(ScriptLocation {
                origin_map_name: "TestMap".to_string(),
                script: " .Done@Script".to_string(),
            }),
            ..ScriptRuntimeMemory::default()
        };
        assert_eq!(
            runtime.validate(),
            Err("next_script.script has invalid script label ' .Done@Script'".to_string())
        );

        runtime = ScriptRuntimeMemory {
            blackout_mod: Some("BLACKOUT MOD".to_string()),
            ..ScriptRuntimeMemory::default()
        };
        assert_eq!(
            runtime.validate(),
            Err("blackout_mod has invalid token 'BLACKOUT MOD'".to_string())
        );

        runtime = ScriptRuntimeMemory {
            battle_tower_text: Some("BATTLE TOWER INTRO".to_string()),
            ..ScriptRuntimeMemory::default()
        };
        assert_eq!(
            runtime.validate(),
            Err("battle_tower_text has invalid token 'BATTLE TOWER INTRO'".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.next_script = Some(ScriptLocation {
            origin_map_name: "TestMap".to_string(),
            script: ".Done@Script".to_string(),
        });
        runtime.deferred_scripts.push(ScriptLocation {
            origin_map_name: "TestMap".to_string(),
            script: ".Deferred @Script".to_string(),
        });
        assert_eq!(
            runtime.validate(),
            Err(
                "deferred_scripts[0].script has invalid script label '.Deferred @Script'"
                    .to_string()
            )
        );

        let mut state = GameState::default();
        state
            .script_runtime
            .control_events
            .push(ScriptControlRuntimeEvent {
                kind: ScriptControlRuntimeKind::Jump,
                target_script: Some(".Done@Script".to_string()),
                source_script: "Source Script".to_string(),
                command_index: 7,
            });
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "invalid saved script runtime: control_events[0].source_script has invalid script label 'Source Script'"
                    .to_string()
            )
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.control_events.push(ScriptControlRuntimeEvent {
            kind: ScriptControlRuntimeKind::Continue,
            target_script: Some(".Done@Script".to_string()),
            source_script: "SourceScript".to_string(),
            command_index: 7,
        });
        assert_eq!(
            runtime.validate(),
            Err("control_events[0].target_script is not valid for Continue".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.control_events.push(ScriptControlRuntimeEvent {
            kind: ScriptControlRuntimeKind::Jump,
            target_script: None,
            source_script: "SourceScript".to_string(),
            command_index: 7,
        });
        assert_eq!(
            runtime.validate(),
            Err("control_events[0].target_script is required for Jump".to_string())
        );

        let control_event = ScriptControlRuntimeEvent {
            kind: ScriptControlRuntimeKind::Continue,
            target_script: Some("TargetScript".to_string()),
            source_script: "SourceScript".to_string(),
            command_index: 7,
        };
        assert_eq!(
            validate_saved_control_runtime_event_shape(
                "script_runtime.control_events[0].source_script",
                &control_event,
            ),
            Err(ScriptControlRuntimeCommandError::UnexpectedContinueTarget {
                path: "script_runtime.control_events[0].source_script".to_string(),
                source_script: "SourceScript".to_string(),
                command_index: 7,
            })
        );

        let control_event = ScriptControlRuntimeEvent {
            kind: ScriptControlRuntimeKind::Call,
            target_script: Some("TargetScript".to_string()),
            source_script: "SourceScript".to_string(),
            command_index: 7,
        };
        assert_eq!(
            validate_saved_control_runtime_event_shape(
                "script_runtime.control_events[0].source_script",
                &control_event,
            ),
            Ok(())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.call_stack.push(ScriptReturnFrame {
            origin_map_name: "TestMap".to_string(),
            source_script: "SourceScript".to_string(),
            next_command_index: 0,
        });
        assert_eq!(
            runtime.validate(),
            Err("call_stack[0].next_command_index cannot be 0".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.script_ended = Some(ScriptEndState {
            callback: true,
            just_battled_guard: true,
            source_script: "SourceScript".to_string(),
            command_index: 7,
        });
        assert_eq!(
            runtime.validate(),
            Err("script_ended cannot be both callback and just_battled_guard".to_string())
        );
        assert_eq!(
            saved_script_end_command(runtime.script_ended.as_ref().expect("script end")),
            Err(ScriptEndCommandError::CallbackAndJustBattledGuard {
                source_script: "SourceScript".to_string(),
                command_index: 7,
            })
        );

        assert_eq!(
            saved_script_end_command(&ScriptEndState {
                callback: true,
                just_battled_guard: false,
                source_script: "SourceScript".to_string(),
                command_index: 7,
            }),
            Ok("endcallback")
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.script_ended = Some(ScriptEndState {
            callback: false,
            just_battled_guard: false,
            source_script: "Source Script".to_string(),
            command_index: 7,
        });
        assert_eq!(
            runtime.validate(),
            Err("script_ended.source_script has invalid script label 'Source Script'".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.effects.push(ScriptRuntimeEffect {
            command: "macroeffect".to_string(),
            args: Vec::new(),
            source_script: "EffectScript".to_string(),
            command_index: 3,
        });
        assert_eq!(
            runtime.validate(),
            Err("effects[0].command macroeffect is not a saved runtime command".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.effects.push(ScriptRuntimeEffect {
            command: "special".to_string(),
            args: Vec::new(),
            source_script: "EffectScript".to_string(),
            command_index: 3,
        });
        assert_eq!(
            runtime.validate(),
            Err("effects[0].args has 0 entries, expected 1 for special".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime
            .variables
            .insert("VAR BAD".to_string(), "TRUE".to_string());
        assert_eq!(
            runtime.validate(),
            Err("variables[VAR BAD] has invalid token 'VAR BAD'".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime
            .memory
            .insert("w MooMooBerries".to_string(), "TRUE".to_string());
        assert_eq!(
            runtime.validate(),
            Err("memory[w MooMooBerries] has invalid token 'w MooMooBerries'".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime
            .named_buffers
            .insert("STRING BUFFER 1".to_string(), "Runtime text.".to_string());
        assert_eq!(
            runtime.validate(),
            Err("named_buffers[STRING BUFFER 1] has invalid token 'STRING BUFFER 1'".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.variable_sprites.insert(
            "SPRITE WEIRD TREE".to_string(),
            "SPRITE_SUDOWOODO".to_string(),
        );
        assert_eq!(
            runtime.validate(),
            Err(
                "variable_sprites[SPRITE WEIRD TREE] has invalid token 'SPRITE WEIRD TREE'"
                    .to_string()
            )
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.variable_sprites.insert(
            "SPRITE_WEIRD_TREE".to_string(),
            "SPRITE SUDOWOODO".to_string(),
        );
        assert_eq!(
            runtime.validate(),
            Err(
                "variable_sprites[SPRITE_WEIRD_TREE].replacement has invalid token 'SPRITE SUDOWOODO'"
                    .to_string()
            )
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.phone_numbers.insert("PHONE MOM".to_string());
        assert_eq!(
            runtime.validate(),
            Err("phone_numbers[PHONE MOM] has invalid token 'PHONE MOM'".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime
            .special_phone_calls
            .push("SPECIALCALL MASTERBALL".to_string());
        assert_eq!(
            runtime.validate(),
            Err("special_phone_calls[0] has invalid token 'SPECIALCALL MASTERBALL'".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.completed_trades.push("NPC TRADE KYLE".to_string());
        assert_eq!(
            runtime.validate(),
            Err("completed_trades[0] has invalid token 'NPC TRADE KYLE'".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.catch_tutorials.push("RED GYARADOS".to_string());
        assert_eq!(
            runtime.validate(),
            Err("catch_tutorials[0] has invalid token 'RED GYARADOS'".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime
            .checked_mail_targets
            .push("Checked Mail Script".to_string());
        assert_eq!(
            runtime.validate(),
            Err(
                "checked_mail_targets[0] has invalid script label 'Checked Mail Script'"
                    .to_string()
            )
        );

        runtime = ScriptRuntimeMemory::default();
        runtime
            .given_mail_targets
            .push("Given Mail Script".to_string());
        assert_eq!(
            runtime.validate(),
            Err("given_mail_targets[0] has invalid script label 'Given Mail Script'".to_string())
        );

        let mut runtime = ScriptRuntimeMemory {
            current_music: Some("MUSIC ROUTE 29".to_string()),
            ..ScriptRuntimeMemory::default()
        };
        assert_eq!(
            runtime.validate(),
            Err("current_music has invalid token 'MUSIC ROUTE 29'".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.map_events.push(ScriptMapRuntimeEvent {
            command: "warp".to_string(),
            kind: ScriptMapRuntimeKind::Warp,
            target_map: Some("Route 29".to_string()),
            tile: Some(TilePosition::new(2, 2)),
            facing: Some(Direction::Down),
            map_setup: None,
            source_script: "Route29Script".to_string(),
            command_index: 1,
        });
        assert_eq!(
            runtime.validate(),
            Err("map_events[0].target_map has invalid token 'Route 29'".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.map_events.push(ScriptMapRuntimeEvent {
            command: "warp".to_string(),
            kind: ScriptMapRuntimeKind::Warp,
            target_map: None,
            tile: Some(TilePosition::new(2, 2)),
            facing: Some(Direction::Down),
            map_setup: None,
            source_script: "Route29Script".to_string(),
            command_index: 1,
        });
        assert_eq!(
            runtime.validate(),
            Err("map_events[0].target_map is required for Warp".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.map_events.push(ScriptMapRuntimeEvent {
            command: "warp".to_string(),
            kind: ScriptMapRuntimeKind::Warp,
            target_map: Some("Route29".to_string()),
            tile: Some(TilePosition { x: 2, y: 2 }),
            facing: Some(Direction::Right),
            map_setup: None,
            source_script: "Route29Script".to_string(),
            command_index: 1,
        });
        assert_eq!(
            runtime.validate(),
            Err("map_events[0].command warp must be warpfacing for Warp".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.map_events.push(ScriptMapRuntimeEvent {
            command: "warp".to_string(),
            kind: ScriptMapRuntimeKind::Warp,
            target_map: Some("Route29".to_string()),
            tile: Some(TilePosition::new(2, 2)),
            facing: Some(Direction::Down),
            map_setup: Some("MAPSETUP_WARP".to_string()),
            source_script: "Route29Script".to_string(),
            command_index: 1,
        });
        assert_eq!(
            runtime.validate(),
            Err("map_events[0].map_setup is not valid for Warp".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.map_events.push(ScriptMapRuntimeEvent {
            command: "refreshmap".to_string(),
            kind: ScriptMapRuntimeKind::LoadMap,
            target_map: None,
            tile: None,
            facing: None,
            map_setup: None,
            source_script: "Route29Script".to_string(),
            command_index: 1,
        });
        assert_eq!(
            runtime.validate(),
            Err("map_events[0].command refreshmap is not a saved map load command".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.map_events.push(ScriptMapRuntimeEvent {
            command: "newloadmap".to_string(),
            kind: ScriptMapRuntimeKind::LoadMap,
            target_map: None,
            tile: None,
            facing: None,
            map_setup: None,
            source_script: "Route29Script".to_string(),
            command_index: 1,
        });
        assert_eq!(
            runtime.validate(),
            Err("map_events[0].map_setup is required for newloadmap".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.map_events.push(ScriptMapRuntimeEvent {
            command: "refreshmap".to_string(),
            kind: ScriptMapRuntimeKind::RefreshMap,
            target_map: Some("Route29".to_string()),
            tile: None,
            facing: None,
            map_setup: None,
            source_script: "Route29Script".to_string(),
            command_index: 1,
        });
        assert_eq!(
            runtime.validate(),
            Err("map_events[0] RefreshMap cannot carry warp payload".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.pending_map_load = Some(ScriptMapLoadRequest {
            command: "refreshmap".to_string(),
            map_setup: None,
            source_script: "Route29Script".to_string(),
            command_index: 1,
        });
        assert_eq!(
            runtime.validate(),
            Err("pending_map_load.command refreshmap is not a saved map load command".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.pending_map_refresh = Some(ScriptMapRefreshRequest {
            command: "reloadmap".to_string(),
            map_setup: None,
            source_script: "Route29Script".to_string(),
            command_index: 1,
        });
        assert_eq!(
            runtime.validate(),
            Err(
                "pending_map_refresh.command reloadmap is not a saved map refresh command"
                    .to_string()
            )
        );

        let map_load = ScriptMapLoadRequest {
            command: "newloadmap".to_string(),
            map_setup: Some("MAPSETUP_WARP".to_string()),
            source_script: "Route29Script".to_string(),
            command_index: 1,
        };
        assert_eq!(
            saved_map_load_command_payload(&map_load),
            ("newloadmap", vec!["MAPSETUP_WARP".to_string()])
        );

        let map_refresh = ScriptMapRefreshRequest {
            command: "reanchormap".to_string(),
            map_setup: Some("MAPSETUP_CONNECTION".to_string()),
            source_script: "Route29Script".to_string(),
            command_index: 1,
        };
        assert_eq!(
            saved_map_refresh_command_payload(&map_refresh),
            ("reanchormap", vec!["MAPSETUP_CONNECTION".to_string()])
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.shop_events.push(ScriptShopRuntimeEvent {
            mart_type: "MARTTYPE_STANDARD".to_string(),
            mart_id: "CHERRYGROVE_MART".to_string(),
            inventory: vec!["PO TION".to_string()],
            source_script: "ShopScript".to_string(),
            command_index: 2,
        });
        assert_eq!(
            runtime.validate(),
            Err("shop_events[0].inventory[0] has invalid token 'PO TION'".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.shop_events.push(ScriptShopRuntimeEvent {
            mart_type: "MARTTYPE_CUSTOM".to_string(),
            mart_id: "CHERRYGROVE_MART".to_string(),
            inventory: Vec::new(),
            source_script: "ShopScript".to_string(),
            command_index: 2,
        });
        assert_eq!(
            runtime.validate(),
            Err("shop_events[0].mart_type MARTTYPE_CUSTOM is not a saved mart type".to_string())
        );
        let shop_event = ScriptShopRuntimeEvent {
            mart_type: "MARTTYPE_STANDARD".to_string(),
            mart_id: "CHERRYGROVE_MART".to_string(),
            inventory: vec!["POTION".to_string()],
            source_script: "ShopScript".to_string(),
            command_index: 2,
        };
        assert_eq!(
            saved_shop_event_command_payload(&shop_event),
            (
                "pokemart",
                vec![
                    "MARTTYPE_STANDARD".to_string(),
                    "CHERRYGROVE_MART".to_string(),
                ],
            )
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.pending_shop = Some(ScriptShopRequest {
            mart_type: "MARTTYPE_STANDARD".to_string(),
            mart_id: "0".to_string(),
            inventory: Vec::new(),
            source_script: "ShopScript".to_string(),
            command_index: 2,
        });
        assert_eq!(
            runtime.validate(),
            Err(
                "pending_shop.mart_id 0 requires a zero-inventory mart type, got MARTTYPE_STANDARD"
                    .to_string()
            )
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.active_menu = Some("RuntimeMenu".to_string());
        runtime.pending_shop = Some(ScriptShopRequest {
            mart_type: "MARTTYPE_STANDARD".to_string(),
            mart_id: "CHERRYGROVE_MART".to_string(),
            inventory: vec!["POTION".to_string()],
            source_script: "ShopScript".to_string(),
            command_index: 2,
        });
        assert_eq!(
            runtime.validate(),
            Err("pending_shop cannot be saved while active_menu RuntimeMenu is open".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.text_window_open = true;
        runtime.pending_yes_no = Some(ScriptYesNoPrompt {
            source_script: "PromptScript".to_string(),
            command_index: 4,
        });
        runtime.pending_shop = Some(ScriptShopRequest {
            mart_type: "MARTTYPE_STANDARD".to_string(),
            mart_id: "CHERRYGROVE_MART".to_string(),
            inventory: vec!["POTION".to_string()],
            source_script: "ShopScript".to_string(),
            command_index: 2,
        });
        assert_eq!(
            runtime.validate(),
            Err("pending_shop cannot be saved with pending_yes_no".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.text_window_open = true;
        runtime.active_menu = Some("RuntimeMenu".to_string());
        runtime.pending_text_wait = Some(ScriptTextWait {
            command: "waitbutton".to_string(),
            source_script: "TextScript".to_string(),
            command_index: 5,
        });
        assert_eq!(
            runtime.validate(),
            Err("active_menu RuntimeMenu cannot be saved with pending_text_wait".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.text_window_open = true;
        runtime.pending_text_label = Some("GreetingText".to_string());
        runtime.pending_yes_no = Some(ScriptYesNoPrompt {
            source_script: "PromptScript".to_string(),
            command_index: 4,
        });
        assert_eq!(
            runtime.validate(),
            Err("pending_text_label GreetingText cannot be saved with pending_yes_no".to_string())
        );
        let shop_request = ScriptShopRequest {
            mart_type: "MARTTYPE_STANDARD".to_string(),
            mart_id: "CHERRYGROVE_MART".to_string(),
            inventory: vec!["POTION".to_string()],
            source_script: "ShopScript".to_string(),
            command_index: 2,
        };
        assert_eq!(
            saved_shop_request_command_payload(&shop_request),
            (
                "pokemart",
                vec![
                    "MARTTYPE_STANDARD".to_string(),
                    "CHERRYGROVE_MART".to_string(),
                ],
            )
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.item_use_events.push(ItemUseRuntimeEvent {
            item_id: "POTION\n".to_string(),
            context: "field".to_string(),
            consumed: true,
        });
        assert_eq!(
            runtime.validate(),
            Err("item_use_events[0].item_id has invalid token 'POTION\n'".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.item_use_events.push(ItemUseRuntimeEvent {
            item_id: "POTION".to_string(),
            context: "menu".to_string(),
            consumed: true,
        });
        assert_eq!(
            runtime.validate(),
            Err("item_use_events[0].context menu is not a saved item-use context".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.variable_writes.push(ScriptRuntimeVariableWrite {
            target: "w ScriptVar".to_string(),
            value: "7".to_string(),
            source_script: "VarScript".to_string(),
            command_index: 3,
        });
        assert_eq!(
            runtime.validate(),
            Err("variable_writes[0].target has invalid token 'w ScriptVar'".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.variable_writes.push(ScriptRuntimeVariableWrite {
            target: "VAR_BLUECARDBALANCE".to_string(),
            value: "7".to_string(),
            source_script: "VarScript".to_string(),
            command_index: 3,
        });
        assert_eq!(
            runtime.validate(),
            Err(
                "variable_writes[0].target VAR_BLUECARDBALANCE is missing from variables"
                    .to_string()
            )
        );

        runtime = ScriptRuntimeMemory::default();
        runtime
            .variables
            .insert("VAR_BLUECARDBALANCE".to_string(), "8".to_string());
        runtime.variable_writes.push(ScriptRuntimeVariableWrite {
            target: "VAR_BLUECARDBALANCE".to_string(),
            value: "7".to_string(),
            source_script: "VarScript".to_string(),
            command_index: 3,
        });
        assert_eq!(
            runtime.validate(),
            Err(
                "variable_writes[0].value 7 does not match variables[VAR_BLUECARDBALANCE] 8"
                    .to_string()
            )
        );
        assert_eq!(
            saved_variable_write_command_payload(&runtime.variable_writes[0]),
            ("writevar", vec!["VAR_BLUECARDBALANCE".to_string()])
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.asm_directives.push(ScriptRuntimeAsmDirective {
            command: "db".to_string(),
            args: vec!["$00".to_string()],
            source_script: "AsmScript".to_string(),
            command_index: 4,
        });
        assert_eq!(
            runtime.validate(),
            Err("asm_directives[0].command db is not a saved asm directive".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.asm_directives.push(ScriptRuntimeAsmDirective {
            command: "dw".to_string(),
            args: vec![".MenuData".to_string(), ".OtherMenuData".to_string()],
            source_script: "AsmScript".to_string(),
            command_index: 4,
        });
        assert_eq!(
            runtime.validate(),
            Err("asm_directives[0].args has 2 entries, expected 1 for dw".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.asm_directives.push(ScriptRuntimeAsmDirective {
            command: "ld".to_string(),
            args: vec!["a".to_string(), " [rWBK]".to_string()],
            source_script: "AsmScript".to_string(),
            command_index: 4,
        });
        assert_eq!(
            runtime.validate(),
            Err("asm_directives[0].args[1] has invalid arg ' [rWBK]'".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime
            .numeric_buffer_writes
            .push(ScriptRuntimeNumericBufferWrite {
                target_buffer: "STRING BUFFER 3".to_string(),
                value: "12".to_string(),
                width: 3,
                source_script: "BufferScript".to_string(),
                command_index: 4,
            });
        assert_eq!(
            runtime.validate(),
            Err(
                "numeric_buffer_writes[0].target_buffer has invalid token 'STRING BUFFER 3'"
                    .to_string()
            )
        );

        runtime = ScriptRuntimeMemory::default();
        runtime
            .numeric_buffer_writes
            .push(ScriptRuntimeNumericBufferWrite {
                target_buffer: "STRING_BUFFER_3".to_string(),
                value: "00012".to_string(),
                width: 3,
                source_script: "BufferScript".to_string(),
                command_index: 4,
            });
        assert_eq!(
            runtime.validate(),
            Err("numeric_buffer_writes[0].value 00012 is not canonical".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime
            .numeric_buffer_writes
            .push(ScriptRuntimeNumericBufferWrite {
                target_buffer: "STRING_BUFFER_3".to_string(),
                value: "12".to_string(),
                width: 4,
                source_script: "BufferScript".to_string(),
                command_index: 4,
            });
        assert_eq!(
            runtime.validate(),
            Err("numeric_buffer_writes[0].width 4 must be 3".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime
            .numeric_buffer_writes
            .push(ScriptRuntimeNumericBufferWrite {
                target_buffer: "STRING_BUFFER_3".to_string(),
                value: "12".to_string(),
                width: 3,
                source_script: "BufferScript".to_string(),
                command_index: 4,
            });
        assert_eq!(
            runtime.validate(),
            Err(
                "numeric_buffer_writes[0].target_buffer STRING_BUFFER_3 is missing from named_buffers"
                    .to_string()
            )
        );

        runtime = ScriptRuntimeMemory::default();
        runtime
            .named_buffers
            .insert("STRING_BUFFER_3".to_string(), "13".to_string());
        runtime
            .numeric_buffer_writes
            .push(ScriptRuntimeNumericBufferWrite {
                target_buffer: "STRING_BUFFER_3".to_string(),
                value: "12".to_string(),
                width: 3,
                source_script: "BufferScript".to_string(),
                command_index: 4,
            });
        assert_eq!(
            runtime.validate(),
            Err(
                "numeric_buffer_writes[0].value 12 does not match named_buffers[STRING_BUFFER_3] 13"
                    .to_string()
            )
        );
        assert_eq!(
            saved_numeric_buffer_write_command_payload(&runtime.numeric_buffer_writes[0]),
            ("getnum", vec!["STRING_BUFFER_3".to_string()])
        );

        runtime = ScriptRuntimeMemory::default();
        runtime
            .stone_table_entries
            .push(ScriptRuntimeStoneTableEntry {
                warp: 1,
                object_event: "BLACKTHORNGYM2F_BOULDER1".to_string(),
                script: ".Stone Script".to_string(),
                source_script: "StoneScript".to_string(),
                command_index: 5,
            });
        assert_eq!(
            runtime.validate(),
            Err(
                "stone_table_entries[0].script has invalid script label '.Stone Script'"
                    .to_string()
            )
        );
        let stone = ScriptRuntimeStoneTableEntry {
            warp: 5,
            object_event: "BLACKTHORNGYM2F_BOULDER1".to_string(),
            script: "StoneScript".to_string(),
            source_script: "StoneTableScript".to_string(),
            command_index: 5,
        };
        assert_eq!(
            saved_stone_table_entry_command_payload(&stone),
            (
                "stonetable",
                vec![
                    "5".to_string(),
                    "BLACKTHORNGYM2F_BOULDER1".to_string(),
                    "StoneScript".to_string(),
                ],
            )
        );
        let floor = ScriptRuntimeElevatorFloor {
            floor: "B1F".to_string(),
            warp: 2,
            target_map: "GoldenrodDeptStoreB1F".to_string(),
            source_script: "ElevatorScript".to_string(),
            command_index: 5,
        };
        assert_eq!(
            saved_elevator_floor_command_payload(&floor),
            (
                "elevfloor",
                vec![
                    "B1F".to_string(),
                    "2".to_string(),
                    "GoldenrodDeptStoreB1F".to_string(),
                ],
            )
        );
        let description = ScriptRuntimeDecorationDescription {
            decoration: "DECODESC_LEFT_DOLL".to_string(),
            source_script: "DecorationScript".to_string(),
            command_index: 5,
        };
        assert_eq!(
            saved_decoration_description_command_payload(&description),
            ("describedecoration", vec!["DECODESC_LEFT_DOLL".to_string()],)
        );
        let warp = ScriptWarpRequest {
            target_map: "Route29".to_string(),
            tile: TilePosition { x: 18, y: 30 },
            facing: Some(Direction::Right),
            source_script: "WarpScript".to_string(),
            command_index: 5,
        };
        assert_eq!(
            saved_script_warp_command_payload(
                "script_runtime.pending_script_warp.source_script",
                &warp
            ),
            Ok((
                "warpfacing",
                vec![
                    "Route29".to_string(),
                    "18".to_string(),
                    "30".to_string(),
                    "RIGHT".to_string(),
                ],
            ))
        );
        let mut unaligned_warp = warp.clone();
        unaligned_warp.tile = TilePosition { x: 1, y: 0 };
        assert_eq!(
            saved_script_warp_command_payload(
                "script_runtime.pending_script_warp.source_script",
                &unaligned_warp
            ),
            Ok((
                "warpfacing",
                vec![
                    "Route29".to_string(),
                    "1".to_string(),
                    "0".to_string(),
                    "RIGHT".to_string(),
                ],
            ))
        );
        let mut unsavable_warp = warp;
        unsavable_warp.tile = TilePosition { x: -1, y: 15 };
        assert_eq!(
            saved_script_warp_command_payload(
                "script_runtime.pending_script_warp.source_script",
                &unsavable_warp
            ),
            Err(ScriptMapRuntimeCommandError::UnsavableTile {
                path: "script_runtime.pending_script_warp.source_script".to_string(),
                source_script: "WarpScript".to_string(),
                command_index: 5,
                command: "warpfacing".to_string(),
                x: -1,
                y: 15,
            })
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.command_queue.push(ScriptRuntimeQueuedCommand {
            origin_map_name: "TestMap".to_string(),
            command: "callasm".to_string(),
            target: "Queued Target".to_string(),
            bank: Some("BANK1".to_string()),
            source_script: "QueueScript".to_string(),
            command_index: 6,
        });
        assert_eq!(
            runtime.validate(),
            Err("command_queue[0].target has invalid token 'Queued Target'".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.command_queue.push(ScriptRuntimeQueuedCommand {
            origin_map_name: "TestMap".to_string(),
            command: "cmdqueue".to_string(),
            target: "QueuedTarget".to_string(),
            bank: None,
            source_script: "QueueScript".to_string(),
            command_index: 6,
        });
        assert_eq!(
            runtime.validate(),
            Err("command_queue[0].bank is required for cmdqueue".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.command_queue.push(ScriptRuntimeQueuedCommand {
            origin_map_name: "TestMap".to_string(),
            command: "writecmdqueue".to_string(),
            target: "QueuedTarget".to_string(),
            bank: Some("BANK1".to_string()),
            source_script: "QueueScript".to_string(),
            command_index: 6,
        });
        assert_eq!(
            runtime.validate(),
            Err("command_queue[0].bank is not valid for writecmdqueue".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.command_queue.push(ScriptRuntimeQueuedCommand {
            origin_map_name: "TestMap".to_string(),
            command: "macroqueue".to_string(),
            target: "QueuedTarget".to_string(),
            bank: None,
            source_script: "QueueScript".to_string(),
            command_index: 6,
        });
        assert_eq!(
            runtime.validate(),
            Err("command_queue[0].command macroqueue is not a saved queued command".to_string())
        );

        let queued = ScriptRuntimeQueuedCommand {
            origin_map_name: "TestMap".to_string(),
            command: "cmdqueue".to_string(),
            target: "QueuedTarget".to_string(),
            bank: Some("BANK1".to_string()),
            source_script: "QueueScript".to_string(),
            command_index: 6,
        };
        assert_eq!(
            saved_queued_command_args(&queued),
            vec!["BANK1".to_string(), "QueuedTarget".to_string()]
        );
        let queued = ScriptRuntimeQueuedCommand {
            bank: None,
            ..queued
        };
        assert_eq!(
            saved_queued_command_args(&queued),
            vec!["QueuedTarget".to_string()]
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.pending_delays.push(ScriptRuntimeDelay {
            command: "delay".to_string(),
            frames: 16,
            source_script: "DelayScript".to_string(),
            command_index: 7,
        });
        assert_eq!(
            runtime.validate(),
            Err("pending_delays[0].command delay is not a saved delay command".to_string())
        );

        let delay = ScriptRuntimeDelay {
            command: "pause".to_string(),
            frames: 15,
            source_script: "DelayScript".to_string(),
            command_index: 7,
        };
        assert_eq!(
            saved_delay_command_payload(&delay),
            ("pause", vec!["15".to_string()])
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.pending_earthquakes.push(ScriptRuntimeEarthquake {
            parameter: 32,
            shake_frames: 32,
            sleep_frames: 32,
            source_script: "Earthquake Script".to_string(),
            command_index: 7,
        });
        assert_eq!(
            runtime.validate(),
            Err(
                "pending_earthquakes[0].source_script has invalid script label 'Earthquake Script'"
                    .to_string()
            )
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.pending_earthquakes.push(ScriptRuntimeEarthquake {
            parameter: 84,
            shake_frames: 83,
            sleep_frames: 84 & 0x3f,
            source_script: "EarthquakeScript".to_string(),
            command_index: 7,
        });
        assert_eq!(
            runtime.validate(),
            Err("pending_earthquakes[0].shake_frames 83 must equal parameter 84".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.pending_earthquakes.push(ScriptRuntimeEarthquake {
            parameter: 84,
            shake_frames: 84,
            sleep_frames: 84,
            source_script: "EarthquakeScript".to_string(),
            command_index: 7,
        });
        assert_eq!(
            runtime.validate(),
            Err(
                "pending_earthquakes[0].sleep_frames 84 must equal parameter & 0x3f (20)"
                    .to_string()
            )
        );

        let earthquake = ScriptRuntimeEarthquake {
            parameter: 72,
            shake_frames: 72,
            sleep_frames: 8,
            source_script: "EarthquakeScript".to_string(),
            command_index: 8,
        };
        assert_eq!(
            saved_earthquake_command_payload(&earthquake),
            ("earthquake", vec!["72".to_string()])
        );

        let emote = ScriptRuntimeEmote {
            emote: "EMOTE_SHOCK".to_string(),
            object: "RuntimeObject".to_string(),
            duration: 16,
            source_script: "EmoteScript".to_string(),
            command_index: 9,
        };
        assert_eq!(
            saved_emote_command_payload(&emote),
            (
                "showemote",
                vec![
                    "EMOTE_SHOCK".to_string(),
                    "RuntimeObject".to_string(),
                    "16".to_string(),
                ],
            )
        );
        let music_fade = ScriptMusicFade {
            audio_id: "MUSIC_ROUTE_29".to_string(),
            fade_frames: 16,
            source_script: "MusicFadeScript".to_string(),
            command_index: 8,
        };
        assert_eq!(
            saved_music_fade_command_payload(&music_fade),
            (
                "musicfadeout",
                vec!["MUSIC_ROUTE_29".to_string(), "16".to_string()],
            )
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.audio_events.push(ScriptAudioRuntimeEvent {
            command: "play music".to_string(),
            kind: ScriptAudioRuntimeKind::Music,
            audio_id: Some("MUSIC_ROUTE_29".to_string()),
            fade_frames: None,
            source_script: "AudioScript".to_string(),
            command_index: 8,
        });
        assert_eq!(
            runtime.validate(),
            Err("audio_events[0].command has invalid token 'play music'".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.audio_events.push(ScriptAudioRuntimeEvent {
            command: "playmusic".to_string(),
            kind: ScriptAudioRuntimeKind::Music,
            audio_id: None,
            fade_frames: None,
            source_script: "AudioScript".to_string(),
            command_index: 8,
        });
        assert_eq!(
            runtime.validate(),
            Err("audio_events[0].audio_id is required for Music".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.audio_events.push(ScriptAudioRuntimeEvent {
            command: "playsound".to_string(),
            kind: ScriptAudioRuntimeKind::Music,
            audio_id: Some("SFX_TACKLE".to_string()),
            fade_frames: None,
            source_script: "AudioScript".to_string(),
            command_index: 8,
        });
        assert_eq!(
            runtime.validate(),
            Err("audio_events[0].command playsound is not valid for Music".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.audio_events.push(ScriptAudioRuntimeEvent {
            command: "playsound".to_string(),
            kind: ScriptAudioRuntimeKind::SoundEffect,
            audio_id: Some("SFX_TACKLE".to_string()),
            fade_frames: Some(8),
            source_script: "AudioScript".to_string(),
            command_index: 8,
        });
        assert_eq!(
            runtime.validate(),
            Err("audio_events[0].fade_frames is only valid for FadeMusic".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.audio_events.push(ScriptAudioRuntimeEvent {
            command: "musicfadeout".to_string(),
            kind: ScriptAudioRuntimeKind::FadeMusic,
            audio_id: Some("MUSIC_ROUTE_29".to_string()),
            fade_frames: None,
            source_script: "AudioScript".to_string(),
            command_index: 8,
        });
        assert_eq!(
            runtime.validate(),
            Err("audio_events[0].fade_frames is required for FadeMusic".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.audio_events.push(ScriptAudioRuntimeEvent {
            command: "playmusic".to_string(),
            kind: ScriptAudioRuntimeKind::FadeMusic,
            audio_id: Some("MUSIC_ROUTE_29".to_string()),
            fade_frames: Some(8),
            source_script: "AudioScript".to_string(),
            command_index: 8,
        });
        assert_eq!(
            runtime.validate(),
            Err("audio_events[0].command playmusic is not valid for FadeMusic".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.audio_events.push(ScriptAudioRuntimeEvent {
            command: "waitsfx".to_string(),
            kind: ScriptAudioRuntimeKind::WaitForSoundEffect,
            audio_id: Some("SFX_TACKLE".to_string()),
            fade_frames: None,
            source_script: "AudioScript".to_string(),
            command_index: 8,
        });
        assert_eq!(
            runtime.validate(),
            Err("audio_events[0].audio_id is not valid for WaitForSoundEffect".to_string())
        );
        let play_music = ScriptAudioRuntimeEvent {
            command: "playmusic".to_string(),
            kind: ScriptAudioRuntimeKind::Music,
            audio_id: Some("MUSIC_ROUTE_29".to_string()),
            fade_frames: None,
            source_script: "AudioScript".to_string(),
            command_index: 8,
        };
        assert_eq!(
            saved_audio_runtime_event_command_args("audio", &play_music),
            Ok(Some(vec!["MUSIC_ROUTE_29".to_string()]))
        );
        let fade_music = ScriptAudioRuntimeEvent {
            command: "musicfadeout".to_string(),
            kind: ScriptAudioRuntimeKind::FadeMusic,
            audio_id: Some("MUSIC_NONE".to_string()),
            fade_frames: Some(2),
            source_script: "AudioScript".to_string(),
            command_index: 9,
        };
        assert_eq!(
            saved_audio_runtime_event_command_args("audio", &fade_music),
            Ok(Some(vec!["MUSIC_NONE".to_string(), "2".to_string()]))
        );
        let wait_sfx = ScriptAudioRuntimeEvent {
            command: "waitsfx".to_string(),
            kind: ScriptAudioRuntimeKind::WaitForSoundEffect,
            audio_id: None,
            fade_frames: None,
            source_script: "AudioScript".to_string(),
            command_index: 10,
        };
        assert_eq!(
            saved_audio_runtime_event_command_args("audio", &wait_sfx),
            Ok(Some(Vec::new()))
        );
        let special_fade = ScriptAudioRuntimeEvent {
            command: "special".to_string(),
            kind: ScriptAudioRuntimeKind::FadeMusic,
            audio_id: Some("MUSIC_NONE".to_string()),
            fade_frames: Some(2),
            source_script: "FadeOutMusic".to_string(),
            command_index: 0,
        };
        assert_eq!(
            saved_audio_runtime_event_command_args("audio", &special_fade),
            Ok(None)
        );
        let invalid_special = ScriptAudioRuntimeEvent {
            command: "special".to_string(),
            kind: ScriptAudioRuntimeKind::Music,
            audio_id: None,
            fade_frames: None,
            source_script: "HealParty".to_string(),
            command_index: 0,
        };
        assert_eq!(
            saved_audio_runtime_event_command_args("audio", &invalid_special)
                .map_err(|error| error.to_string()),
            Err("saved audio special routine HealParty does not emit audio".to_string())
        );

        runtime = ScriptRuntimeMemory {
            pending_screen_fade: Some(ScriptScreenFade {
                color: ScriptFadeColor::White,
                direction: ScriptFadeDirection::Out,
                frames: 8,
                source_script: "FadeOutToWhite".to_string(),
                command_index: 1,
            }),
            ..ScriptRuntimeMemory::default()
        };
        assert_eq!(
            runtime.validate(),
            Err("pending_screen_fade.command_index 1 must be 0".to_string())
        );
        let pending_fade = runtime.pending_screen_fade.as_ref().unwrap();
        assert_eq!(
            validate_saved_pending_screen_fade_shape(
                "script_runtime.pending_screen_fade.source_script",
                pending_fade,
            )
            .map_err(|error| error.to_string()),
            Err(
                "saved script_runtime.pending_screen_fade.source_script FadeOutToWhite:1 screen fade must use command index 0"
                    .to_string()
            )
        );

        runtime = ScriptRuntimeMemory {
            pending_screen_fade: Some(ScriptScreenFade {
                color: ScriptFadeColor::White,
                direction: ScriptFadeDirection::Out,
                frames: 4,
                source_script: "FadeOutToWhite".to_string(),
                command_index: 0,
            }),
            ..ScriptRuntimeMemory::default()
        };
        assert_eq!(
            runtime.validate(),
            Err("pending_screen_fade.frames 4 must be 8".to_string())
        );

        assert_eq!(
            validate_saved_screen_fade_routine(
                "script_runtime.pending_screen_fade.source_script",
                "FadeOutToWhite",
                ScriptFadeColor::Black,
                ScriptFadeDirection::Out,
                8,
            ),
            Err(ScriptScreenFadeRoutineError::ColorMismatch {
                path: "script_runtime.pending_screen_fade.source_script".to_string(),
                routine: "FadeOutToWhite".to_string(),
                actual: ScriptFadeColor::Black,
                expected: ScriptFadeColor::White,
            })
        );
        assert_eq!(
            validate_saved_screen_fade_routine(
                "script_runtime.graphics_events[0].source_script",
                "UnknownFadeRoutine",
                ScriptFadeColor::White,
                ScriptFadeDirection::Out,
                8,
            ),
            Err(ScriptScreenFadeRoutineError::UnknownRoutine {
                path: "script_runtime.graphics_events[0].source_script".to_string(),
                routine: "UnknownFadeRoutine".to_string(),
            })
        );
        assert_eq!(
            validate_saved_screen_fade_routine(
                "script_runtime.graphics_events[0].source_script",
                "FadeInFromBlack",
                ScriptFadeColor::Black,
                ScriptFadeDirection::In,
                8,
            ),
            Ok(())
        );
        let graphics_fade = ScriptGraphicsRuntimeEvent {
            command: "special".to_string(),
            kind: ScriptGraphicsRuntimeKind::ScreenFade,
            color: Some(ScriptFadeColor::White),
            direction: Some(ScriptFadeDirection::Out),
            frames: Some(8),
            source_script: "FadeOutToWhite".to_string(),
            command_index: 0,
        };
        assert_eq!(
            validate_saved_graphics_runtime_event_shape(
                "script_runtime.graphics_events[0].source_script",
                &graphics_fade,
            ),
            Ok(())
        );
        let mut invalid_graphics_fade = graphics_fade.clone();
        invalid_graphics_fade.frames = Some(4);
        assert_eq!(
            validate_saved_graphics_runtime_event_shape(
                "script_runtime.graphics_events[0].source_script",
                &invalid_graphics_fade,
            )
            .map_err(|error| error.to_string()),
            Err(
                "saved script_runtime.graphics_events[0].source_script FadeOutToWhite frames 4 does not match 8"
                    .to_string()
            )
        );
        let clear_tilemap = ScriptGraphicsRuntimeEvent {
            command: "special".to_string(),
            kind: ScriptGraphicsRuntimeKind::ClearTilemap,
            color: Some(ScriptFadeColor::White),
            direction: None,
            frames: None,
            source_script: "ClearTilemap".to_string(),
            command_index: 0,
        };
        assert_eq!(
            validate_saved_graphics_runtime_event_shape(
                "script_runtime.graphics_events[0].source_script",
                &clear_tilemap,
            )
            .map_err(|error| error.to_string()),
            Err(
                "saved script_runtime.graphics_events[0].source_script ClearTilemap:0 graphics event has unexpected fade payload"
                    .to_string()
            )
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.graphics_events.push(ScriptGraphicsRuntimeEvent {
            command: "special_fade".to_string(),
            kind: ScriptGraphicsRuntimeKind::BattleTowerFade,
            color: None,
            direction: None,
            frames: None,
            source_script: "Graphics Script".to_string(),
            command_index: 9,
        });
        assert_eq!(
            runtime.validate(),
            Err(
                "graphics_events[0].source_script has invalid script label 'Graphics Script'"
                    .to_string()
            )
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.graphics_events.push(ScriptGraphicsRuntimeEvent {
            command: "special_fade".to_string(),
            kind: ScriptGraphicsRuntimeKind::BattleTowerFade,
            color: None,
            direction: None,
            frames: None,
            source_script: "BattleTowerFade".to_string(),
            command_index: 0,
        });
        assert_eq!(
            runtime.validate(),
            Err("graphics_events[0].command special_fade must be special".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.graphics_events.push(ScriptGraphicsRuntimeEvent {
            command: "special".to_string(),
            kind: ScriptGraphicsRuntimeKind::BattleTowerFade,
            color: None,
            direction: None,
            frames: None,
            source_script: "BattleTowerFade".to_string(),
            command_index: 9,
        });
        assert_eq!(
            runtime.validate(),
            Err("graphics_events[0].command_index 9 must be 0".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.graphics_events.push(ScriptGraphicsRuntimeEvent {
            command: "special".to_string(),
            kind: ScriptGraphicsRuntimeKind::ScreenFade,
            color: None,
            direction: Some(ScriptFadeDirection::Out),
            frames: Some(8),
            source_script: "FadeOutToWhite".to_string(),
            command_index: 0,
        });
        assert_eq!(
            runtime.validate(),
            Err("graphics_events[0].color is required for ScreenFade".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.graphics_events.push(ScriptGraphicsRuntimeEvent {
            command: "special".to_string(),
            kind: ScriptGraphicsRuntimeKind::ScreenFade,
            color: Some(ScriptFadeColor::White),
            direction: Some(ScriptFadeDirection::Out),
            frames: Some(4),
            source_script: "FadeOutToWhite".to_string(),
            command_index: 0,
        });
        assert_eq!(
            runtime.validate(),
            Err("graphics_events[0].frames 4 must be 8 for ScreenFade".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.graphics_events.push(ScriptGraphicsRuntimeEvent {
            command: "special".to_string(),
            kind: ScriptGraphicsRuntimeKind::ClearTilemap,
            color: Some(ScriptFadeColor::White),
            direction: None,
            frames: None,
            source_script: "ClearTilemap".to_string(),
            command_index: 0,
        });
        assert_eq!(
            runtime.validate(),
            Err("graphics_events[0] fade payload is only valid for ScreenFade".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.money_events.push(ScriptMoneyRuntimeEvent {
            command: "display money".to_string(),
            kind: ScriptMoneyRuntimeKind::PlaceMoneyTopRight,
            money: 3000,
            coins: None,
            source_script: "MoneyScript".to_string(),
            command_index: 10,
        });
        assert_eq!(
            runtime.validate(),
            Err("money_events[0].command has invalid token 'display money'".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.money_events.push(ScriptMoneyRuntimeEvent {
            command: "displaymoney".to_string(),
            kind: ScriptMoneyRuntimeKind::PlaceMoneyTopRight,
            money: 3000,
            coins: None,
            source_script: "PlaceMoneyTopRight".to_string(),
            command_index: 0,
        });
        assert_eq!(
            runtime.validate(),
            Err("money_events[0].command displaymoney must be special".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.money_events.push(ScriptMoneyRuntimeEvent {
            command: "special".to_string(),
            kind: ScriptMoneyRuntimeKind::PlaceMoneyTopRight,
            money: 3000,
            coins: None,
            source_script: "PlaceMoneyTopRight".to_string(),
            command_index: 2,
        });
        assert_eq!(
            runtime.validate(),
            Err("money_events[0].command_index 2 must be 0".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.money_events.push(ScriptMoneyRuntimeEvent {
            command: "special".to_string(),
            kind: ScriptMoneyRuntimeKind::PlaceMoneyTopRight,
            money: 3000,
            coins: Some(7),
            source_script: "MoneyScript".to_string(),
            command_index: 10,
        });
        assert_eq!(
            runtime.validate(),
            Err("money_events[0].coins is not valid for PlaceMoneyTopRight".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.money_events.push(ScriptMoneyRuntimeEvent {
            command: "special".to_string(),
            kind: ScriptMoneyRuntimeKind::DisplayMoneyAndCoinBalance,
            money: 3000,
            coins: None,
            source_script: "MoneyScript".to_string(),
            command_index: 10,
        });
        assert_eq!(
            runtime.validate(),
            Err("money_events[0].coins is required for DisplayMoneyAndCoinBalance".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.money_events.push(ScriptMoneyRuntimeEvent {
            command: "special".to_string(),
            kind: ScriptMoneyRuntimeKind::DisplayCoinCaseBalance,
            money: 3000,
            coins: Some(7),
            source_script: "MoneyScript".to_string(),
            command_index: 10,
        });
        assert_eq!(
            runtime.validate(),
            Err("money_events[0].money must be 0 for DisplayCoinCaseBalance".to_string())
        );

        let money_event = ScriptMoneyRuntimeEvent {
            command: "special".to_string(),
            kind: ScriptMoneyRuntimeKind::DisplayCoinCaseBalance,
            money: 0,
            coins: Some(7),
            source_script: "DisplayCoinCaseBalance".to_string(),
            command_index: 0,
        };
        assert_eq!(
            validate_saved_money_runtime_routine(
                "script_runtime.money_events[0].source_script",
                &money_event,
            ),
            Ok(())
        );
        assert_eq!(
            validate_saved_money_runtime_event_shape(
                "script_runtime.money_events[0].source_script",
                &money_event,
            ),
            Ok(())
        );

        let mut money_event = money_event;
        money_event.coins = None;
        assert_eq!(
            validate_saved_money_runtime_event_shape(
                "script_runtime.money_events[0].source_script",
                &money_event,
            )
            .map_err(|error| error.to_string()),
            Err(
                "saved script_runtime.money_events[0].source_script DisplayCoinCaseBalance:0 money event is missing coins"
                    .to_string()
            )
        );
        money_event.coins = Some(7);
        money_event.source_script = "UnknownMoneyRoutine".to_string();
        assert_eq!(
            validate_saved_money_runtime_routine(
                "script_runtime.money_events[0].source_script",
                &money_event,
            ),
            Err(ScriptMoneyRuntimeRoutineError::UnknownRoutine {
                path: "script_runtime.money_events[0].source_script".to_string(),
                routine: "UnknownMoneyRoutine".to_string(),
            })
        );

        money_event.source_script = "PlaceMoneyTopRight".to_string();
        assert_eq!(
            validate_saved_money_runtime_routine(
                "script_runtime.money_events[0].source_script",
                &money_event,
            ),
            Err(ScriptMoneyRuntimeRoutineError::KindMismatch {
                path: "script_runtime.money_events[0].source_script".to_string(),
                routine: "PlaceMoneyTopRight".to_string(),
                actual: ScriptMoneyRuntimeKind::DisplayCoinCaseBalance,
                expected: ScriptMoneyRuntimeKind::PlaceMoneyTopRight,
            })
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.pending_map_load = Some(ScriptMapLoadRequest {
            command: "reload map".to_string(),
            map_setup: Some("MAPSETUP_WARP".to_string()),
            source_script: "MapLoadScript".to_string(),
            command_index: 11,
        });
        assert_eq!(
            runtime.validate(),
            Err("pending_map_load.command has invalid token 'reload map'".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.pending_script_warp = Some(ScriptWarpRequest {
            target_map: "Route29".to_string(),
            tile: TilePosition { x: -1, y: 15 },
            facing: Some(Direction::Right),
            source_script: "WarpScript".to_string(),
            command_index: 5,
        });
        assert_eq!(
            runtime.validate(),
            Err(
                "pending_script_warp WarpScript:5 command warpfacing tile (-1, 15) is not aligned to a raw map event coordinate"
                    .to_string()
            )
        );

        let mut map_event = ScriptMapRuntimeEvent {
            command: "warpfacing".to_string(),
            kind: ScriptMapRuntimeKind::Warp,
            target_map: Some("Route29".to_string()),
            tile: Some(TilePosition { x: 18, y: 30 }),
            facing: Some(Direction::Left),
            map_setup: None,
            source_script: "MapScript".to_string(),
            command_index: 11,
        };
        assert_eq!(
            saved_map_runtime_event_command_args(
                "script_runtime.map_events[0].source_script",
                &map_event
            ),
            Ok(Some(vec![
                "Route29".to_string(),
                "18".to_string(),
                "30".to_string(),
                "LEFT".to_string(),
            ]))
        );

        map_event.tile = Some(TilePosition { x: -1, y: 15 });
        assert_eq!(
            saved_map_runtime_event_command_args(
                "script_runtime.map_events[0].source_script",
                &map_event
            ),
            Err(ScriptMapRuntimeCommandError::UnsavableTile {
                path: "script_runtime.map_events[0].source_script".to_string(),
                source_script: "MapScript".to_string(),
                command_index: 11,
                command: "warpfacing".to_string(),
                x: -1,
                y: 15,
            })
        );
        runtime = ScriptRuntimeMemory::default();
        runtime.map_events.push(map_event.clone());
        assert_eq!(
            runtime.validate(),
            Err(
                "map_events[0] MapScript:11 command warpfacing tile (-1, 15) is not aligned to a raw map event coordinate"
                    .to_string()
            )
        );
        map_event.tile = Some(TilePosition { x: 18, y: 30 });

        let mut map_event = map_event;
        map_event.command = "warp".to_string();
        map_event.target_map = None;
        map_event.tile = None;
        map_event.facing = None;
        assert_eq!(
            saved_map_runtime_event_command_args(
                "script_runtime.map_events[0].source_script",
                &map_event
            ),
            Ok(Some(vec![
                "NONE".to_string(),
                "0".to_string(),
                "0".to_string()
            ]))
        );

        map_event.command = "custommapcommand".to_string();
        assert_eq!(
            saved_map_runtime_event_command_args(
                "script_runtime.map_events[0].source_script",
                &map_event
            ),
            Ok(None)
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.text_events.push(ScriptTextRuntimeEvent {
            command: "writetext".to_string(),
            kind: ScriptTextRuntimeKind::Write,
            text_label: None,
            face_player: false,
            closes_text: false,
            source_script: "TextScript".to_string(),
            command_index: 12,
        });
        assert_eq!(
            runtime.validate(),
            Err("text_events[0].text_label is required for Write".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.text_events.push(ScriptTextRuntimeEvent {
            command: "opentext".to_string(),
            kind: ScriptTextRuntimeKind::Write,
            text_label: Some("GreetingText".to_string()),
            face_player: false,
            closes_text: false,
            source_script: "TextScript".to_string(),
            command_index: 12,
        });
        assert_eq!(
            runtime.validate(),
            Err("text_events[0].command opentext is not valid for Write".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.text_events.push(ScriptTextRuntimeEvent {
            command: "jumptext".to_string(),
            kind: ScriptTextRuntimeKind::Write,
            text_label: Some("GreetingText".to_string()),
            face_player: false,
            closes_text: false,
            source_script: "TextScript".to_string(),
            command_index: 12,
        });
        assert_eq!(
            runtime.validate(),
            Err("text_events[0].closes_text must be true for jumptext".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.text_events.push(ScriptTextRuntimeEvent {
            command: "jumptextfaceplayer".to_string(),
            kind: ScriptTextRuntimeKind::Write,
            text_label: Some("GreetingText".to_string()),
            face_player: false,
            closes_text: true,
            source_script: "TextScript".to_string(),
            command_index: 12,
        });
        assert_eq!(
            runtime.validate(),
            Err("text_events[0].face_player must be true for jumptextfaceplayer".to_string())
        );

        let text_event = ScriptTextRuntimeEvent {
            command: "jumptextfaceplayer".to_string(),
            kind: ScriptTextRuntimeKind::Write,
            text_label: Some("GreetingText".to_string()),
            face_player: true,
            closes_text: true,
            source_script: "TextScript".to_string(),
            command_index: 12,
        };
        assert_eq!(
            saved_text_runtime_event_command_args(
                "script_runtime.text_events[0].source_script",
                &text_event
            ),
            Ok(Some(vec!["GreetingText".to_string()]))
        );

        let mut text_event = text_event;
        text_event.command = "farwritetext".to_string();
        text_event.face_player = false;
        text_event.closes_text = false;
        runtime = ScriptRuntimeMemory::default();
        runtime.text_events.push(text_event.clone());
        assert_eq!(runtime.validate(), Ok(()));
        assert_eq!(
            saved_text_runtime_event_command_args(
                "script_runtime.text_events[0].source_script",
                &text_event
            ),
            Ok(Some(vec!["GreetingText".to_string()]))
        );

        text_event.command = "farjumptext".to_string();
        text_event.face_player = false;
        text_event.closes_text = true;
        runtime = ScriptRuntimeMemory::default();
        runtime.text_events.push(text_event.clone());
        assert_eq!(runtime.validate(), Ok(()));
        assert_eq!(
            saved_text_runtime_event_command_args(
                "script_runtime.text_events[0].source_script",
                &text_event
            ),
            Ok(Some(vec!["GreetingText".to_string()]))
        );

        text_event.command = "customtextcommand".to_string();
        assert_eq!(
            saved_text_runtime_event_command_args(
                "script_runtime.text_events[0].source_script",
                &text_event
            ),
            Ok(None)
        );

        text_event.command = "opentext".to_string();
        assert_eq!(
            saved_text_runtime_event_command_args(
                "script_runtime.text_events[0].source_script",
                &text_event
            ),
            Err(ScriptTextRuntimeCommandError::KindMismatch {
                path: "script_runtime.text_events[0].source_script".to_string(),
                source_script: "TextScript".to_string(),
                command_index: 12,
                command: "opentext".to_string(),
                actual: ScriptTextRuntimeKind::Write,
                expected: ScriptTextRuntimeKind::Open,
            })
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.text_events.push(ScriptTextRuntimeEvent {
            command: "opentext".to_string(),
            kind: ScriptTextRuntimeKind::Open,
            text_label: Some("GreetingText".to_string()),
            face_player: false,
            closes_text: false,
            source_script: "TextScript".to_string(),
            command_index: 12,
        });
        assert_eq!(
            runtime.validate(),
            Err("text_events[0].text_label is not valid for Open".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.text_events.push(ScriptTextRuntimeEvent {
            command: "waitbutton".to_string(),
            kind: ScriptTextRuntimeKind::Open,
            text_label: None,
            face_player: false,
            closes_text: false,
            source_script: "TextScript".to_string(),
            command_index: 12,
        });
        assert_eq!(
            runtime.validate(),
            Err("text_events[0].command waitbutton is not valid for Open".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.text_events.push(ScriptTextRuntimeEvent {
            command: "yesorno".to_string(),
            kind: ScriptTextRuntimeKind::YesNo,
            text_label: None,
            face_player: true,
            closes_text: false,
            source_script: "TextScript".to_string(),
            command_index: 12,
        });
        assert_eq!(
            runtime.validate(),
            Err("text_events[0].face_player is not valid for YesNo".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.text_events.push(ScriptTextRuntimeEvent {
            command: "waitbutton".to_string(),
            kind: ScriptTextRuntimeKind::WaitButton,
            text_label: None,
            face_player: false,
            closes_text: true,
            source_script: "TextScript".to_string(),
            command_index: 12,
        });
        assert_eq!(
            runtime.validate(),
            Err("text_events[0].closes_text is not valid for WaitButton".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.text_events.push(ScriptTextRuntimeEvent {
            command: "yesorno".to_string(),
            kind: ScriptTextRuntimeKind::WaitButton,
            text_label: None,
            face_player: false,
            closes_text: false,
            source_script: "TextScript".to_string(),
            command_index: 12,
        });
        assert_eq!(
            runtime.validate(),
            Err("text_events[0].command yesorno is not valid for WaitButton".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.pending_text_wait = Some(ScriptTextWait {
            command: "wait button".to_string(),
            source_script: "TextWaitScript".to_string(),
            command_index: 12,
        });
        assert_eq!(
            runtime.validate(),
            Err("pending_text_wait.command has invalid token 'wait button'".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.text_window_open = true;
        runtime.pending_text_wait = Some(ScriptTextWait {
            command: "opentext".to_string(),
            source_script: "TextWaitScript".to_string(),
            command_index: 12,
        });
        assert_eq!(
            runtime.validate(),
            Err("pending_text_wait.command opentext is not a saved text wait command".to_string())
        );

        let wait = ScriptTextWait {
            command: "jumptextfaceplayer".to_string(),
            source_script: "TextWaitScript".to_string(),
            command_index: 12,
        };
        assert_eq!(
            saved_pending_text_wait_command_args(
                "script_runtime.pending_text_wait.source_script",
                &wait,
                Some("GreetingText"),
            ),
            Ok(Some(vec!["GreetingText".to_string()]))
        );
        assert_eq!(
            saved_pending_text_wait_command_args(
                "script_runtime.pending_text_wait.source_script",
                &wait,
                None,
            ),
            Err(ScriptTextWaitCommandError::MissingPendingTextLabel {
                path: "script_runtime.pending_text_wait.source_script".to_string(),
                source_script: "TextWaitScript".to_string(),
                command_index: 12,
                command: "jumptextfaceplayer".to_string(),
            })
        );
        let far_wait = ScriptTextWait {
            command: "farjumptext".to_string(),
            ..wait
        };
        assert_eq!(
            saved_pending_text_wait_command_args(
                "script_runtime.pending_text_wait.source_script",
                &far_wait,
                Some("GreetingText"),
            ),
            Ok(Some(vec!["GreetingText".to_string()]))
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.pending_yes_no = Some(ScriptYesNoPrompt {
            source_script: "Yes No Script".to_string(),
            command_index: 13,
        });
        assert_eq!(
            runtime.validate(),
            Err(
                "pending_yes_no.source_script has invalid script label 'Yes No Script'".to_string()
            )
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.pending_text_label = Some("GreetingText".to_string());
        assert_eq!(
            runtime.validate(),
            Err(
                "pending_text_label GreetingText cannot be saved without an open text window"
                    .to_string()
            )
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.pending_text_wait = Some(ScriptTextWait {
            command: "waitbutton".to_string(),
            source_script: "TextWaitScript".to_string(),
            command_index: 12,
        });
        assert_eq!(
            runtime.validate(),
            Err("pending_text_wait cannot be saved without an open text window".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.pending_yes_no = Some(ScriptYesNoPrompt {
            source_script: "YesNoScript".to_string(),
            command_index: 13,
        });
        assert_eq!(
            runtime.validate(),
            Err("pending_yes_no cannot be saved without an open text window".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.text_window_open = true;
        runtime.pending_text_wait = Some(ScriptTextWait {
            command: "waitbutton".to_string(),
            source_script: "TextWaitScript".to_string(),
            command_index: 12,
        });
        runtime.pending_yes_no = Some(ScriptYesNoPrompt {
            source_script: "YesNoScript".to_string(),
            command_index: 13,
        });
        assert_eq!(
            runtime.validate(),
            Err("pending_text_wait and pending_yes_no cannot both be saved".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.menu_coords = Some([-1, 0, 5, 5]);
        assert_eq!(
            runtime.validate(),
            Err("menu_coords [-1, 0, 5, 5] cannot contain negative coordinates".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.menu_coords = Some([5, 0, 4, 5]);
        assert_eq!(
            runtime.validate(),
            Err("menu_coords right 4 cannot be less than left 5".to_string())
        );

        runtime = ScriptRuntimeMemory::default();
        runtime.menu_coords = Some([0, 6, 4, 5]);
        assert_eq!(
            runtime.validate(),
            Err("menu_coords bottom 5 cannot be less than top 6".to_string())
        );
    }

    #[test]
    fn party_state_projects_full_authoritative_storage() {
        let mut state = GameState::default();
        let mut species = crate::models::PokemonSpecies::new_for_tests(
            "CHIKORITA",
            crate::models::BaseStats::new(45, 49, 49, 45, 65, 65),
        );
        species.int_id = 152;
        let pokemon = Pokemon::new_for_tests(species, 6, crate::models::Dv::default());
        state
            .storage
            .register_capture(pokemon)
            .expect("capture registers");

        state.sync_party_from_storage();

        assert_eq!(
            state.party.pokemon[0],
            Some(PartyPokemonRef {
                species: "CHIKORITA".to_string(),
                level: 6,
            })
        );
    }

    #[test]
    fn state_json_rejects_unknown_saved_runtime_fields_without_legacy_fallbacks() {
        let mut state_json = serde_json::to_value(GameState::default()).expect("state json");
        state_json
            .as_object_mut()
            .expect("state object")
            .insert("legacy_sram".to_string(), serde_json::json!({}));
        let state_error = serde_json::from_value::<GameState>(state_json)
            .expect_err("game state must not accept legacy save fields")
            .to_string();
        assert!(
            state_error.contains("unknown field `legacy_sram`"),
            "{state_error}"
        );

        let mut runtime_json =
            serde_json::to_value(ScriptRuntimeMemory::default()).expect("runtime json");
        runtime_json
            .as_object_mut()
            .expect("runtime object")
            .insert(
                "fallback_script".to_string(),
                serde_json::json!("MomScript"),
            );
        let runtime_error = serde_json::from_value::<ScriptRuntimeMemory>(runtime_json)
            .expect_err("script runtime memory must not accept fallback script fields")
            .to_string();
        assert!(
            runtime_error.contains("unknown field `fallback_script`"),
            "{runtime_error}"
        );

        let link_status_error =
            serde_json::from_value::<LinkSerialConnectionStatus>(serde_json::json!({
                "using_external_clock": {
                    "legacy_clock": "serial"
                }
            }))
            .expect_err("link serial status must not accept legacy payloads")
            .to_string();
        assert!(
            link_status_error.contains("invalid type")
                || link_status_error.contains("unknown variant"),
            "{link_status_error}"
        );

        let battle_memory_error = serde_json::from_value::<BattleMemory>(serde_json::json!({
            "wild": {
                "battle_type": "BATTLETYPE_NORMAL",
                "map_name": "Route29",
                "enemy_pokemon": null,
                "enemy_party": [],
                "fallback_species": "RATTATA"
            }
        }))
        .expect_err("battle memory must not accept fallback species fields")
        .to_string();
        assert!(
            battle_memory_error.contains("unknown field `fallback_species`")
                || battle_memory_error.contains("invalid type"),
            "{battle_memory_error}"
        );

        let overworld_memory_error = serde_json::from_value::<OverworldMemory>(serde_json::json!({
            "active": {
                "map_name": "Route29",
                "tile": { "x": 1, "y": 2 },
                "facing": "down",
                "mode": "walking",
                "fallback_map_name": "NewBarkTown"
            }
        }))
        .expect_err("overworld memory must not accept fallback map fields")
        .to_string();
        assert!(
            overworld_memory_error.contains("unknown field `fallback_map_name`")
                || overworld_memory_error.contains("invalid type"),
            "{overworld_memory_error}"
        );

        let object_without_tile_error =
            serde_json::from_value::<OverworldObjectMemory>(serde_json::json!({
                "x": 1,
                "y": 2,
                "facing": null
            }))
            .expect_err("saved object memory must explicitly declare runtime tile")
            .to_string();
        assert!(
            object_without_tile_error.contains("missing field `tile`"),
            "{object_without_tile_error}"
        );

        assert_eq!(
            serde_json::from_value::<OverworldObjectMemory>(serde_json::json!({
                "x": 1,
                "y": 2,
                "tile": null,
                "facing": null
            }))
            .expect("explicit null tile is authoritative raw-coordinate memory"),
            OverworldObjectMemory {
                x: 1,
                y: 2,
                tile: None,
                facing: None,
            }
        );

        assert_eq!(
            serde_json::from_value::<OverworldObjectMemory>(serde_json::json!({
                "x": 1,
                "y": 2,
                "tile": { "x": -1, "y": 9 },
                "facing": "left"
            }))
            .expect("signed object runtime tile is saveable"),
            OverworldObjectMemory {
                x: 1,
                y: 2,
                tile: Some(TilePosition::new(-1, 9)),
                facing: Some(Direction::Left),
            }
        );

        let text_speed_error =
            serde_json::from_str::<TextSpeed>(r#"{"fast":{"legacy_speed":"FAST"}}"#)
                .expect_err("saved text speed must not accept legacy aliases")
                .to_string();
        assert!(
            text_speed_error.contains("invalid type")
                || text_speed_error.contains("unknown field `legacy_speed`"),
            "{text_speed_error}"
        );

        let runtime_kind_error =
            serde_json::from_str::<ScriptAudioRuntimeKind>(r#"{"cry":{"fallback_kind":"sfx"}}"#)
                .expect_err("runtime audio kinds must not accept fallback aliases")
                .to_string();
        assert!(
            runtime_kind_error.contains("invalid type")
                || runtime_kind_error.contains("unknown field `fallback_kind`"),
            "{runtime_kind_error}"
        );

        let frame_error = serde_json::from_value::<GameStateFrameError>(serde_json::json!({
            "FrameCursorOverflow": {
                "frame": 99,
                "legacy_frame": 98
            }
        }))
        .expect_err("frame errors must not accept legacy frame values")
        .to_string();
        assert!(
            frame_error.contains("unknown field `legacy_frame`"),
            "{frame_error}"
        );

        let scene_error = serde_json::from_value::<SceneError>(serde_json::json!({
            "UnknownScene": {
                "map_name": "NEW_BARK_TOWN",
                "scene_name": "SCENE_DEFAULT",
                "fallback_scene": "SCENE_NEVER"
            }
        }))
        .expect_err("scene errors must not accept fallback scenes")
        .to_string();
        assert!(
            scene_error.contains("unknown field `fallback_scene`"),
            "{scene_error}"
        );

        let fishing_error =
            serde_json::from_str::<FishingRodState>(r#"{"waiting":{"fallback_result":0}}"#)
                .expect_err("fishing rod state must not accept fallback result payloads")
                .to_string();
        assert!(
            fishing_error.contains("invalid type")
                || fishing_error.contains("unknown field `fallback_result`"),
            "{fishing_error}"
        );

        let command_error = serde_json::from_value::<GameCommand>(serde_json::json!({
            "type": "joypad",
            "mask": 1,
            "fallback_mask": 0
        }))
        .expect_err("game commands must not accept fallback input masks")
        .to_string();
        assert!(
            command_error.contains("unknown field `fallback_mask`"),
            "{command_error}"
        );

        let event_error = serde_json::from_value::<GameEvent>(serde_json::json!({
            "type": "joypad_changed",
            "pressed": 1,
            "down": 1,
            "legacy_down": 0
        }))
        .expect_err("game events must not accept legacy input masks")
        .to_string();
        assert!(
            event_error.contains("unknown field `legacy_down`"),
            "{event_error}"
        );
    }

    #[test]
    fn frame_advancement_is_explicit() {
        let mut state = GameState::default();
        assert_eq!(state.frame(), Frame(0));
        assert_eq!(state.try_advance_frame().expect("advance frame"), Frame(1));
        assert_eq!(state.frame_counter, 1);
        assert_eq!(state.advance_frame(), Frame(2));
        assert_eq!(state.frame_counter, 2);
    }

    #[test]
    fn frame_advancement_rejects_counter_overflow_without_wrapping() {
        let mut state = GameState {
            frame_counter: u64::MAX,
            ..GameState::default()
        };

        assert_eq!(
            state.try_advance_frame(),
            Err(GameStateFrameError::FrameCursorOverflow { frame: u64::MAX })
        );
        assert_eq!(state.frame_counter, u64::MAX);
    }

    #[test]
    fn repel_step_counter_expires_without_item_fallback() {
        let mut state = GameState {
            repel_steps_remaining: 2,
            active_repel_item: Some("REPEL".to_string()),
            ..GameState::default()
        };

        assert_eq!(state.tick_repel_step_after_movement(), None);
        assert_eq!(state.repel_steps_remaining, 1);
        assert_eq!(state.active_repel_item.as_deref(), Some("REPEL"));

        assert_eq!(
            state.tick_repel_step_after_movement(),
            Some("REPEL".to_string())
        );
        assert_eq!(state.repel_steps_remaining, 0);
        assert_eq!(state.active_repel_item, None);
    }

    #[test]
    fn joypad_memory_tracks_pressed_down_released_without_inference() {
        let mut state = GameState::default();

        assert_eq!(
            state.apply_joypad_mask(0b0001_0001),
            Ok(GameEvent::JoypadChanged {
                pressed: 0b0001_0001,
                down: 0b0001_0001,
            })
        );
        assert_eq!(state.joypad.h_joypad_pressed, 0b0001_0001);
        assert_eq!(state.joypad.h_joypad_down, 0b0001_0001);
        assert_eq!(state.joypad.h_joypad_released, 0);
        assert_eq!(state.joypad.h_joypad_sum, 0b0001_0001);

        assert_eq!(
            state.apply_joypad_mask(0b0010_0001),
            Ok(GameEvent::JoypadChanged {
                pressed: 0b0010_0000,
                down: 0b0010_0001,
            })
        );
        assert_eq!(state.joypad.h_joypad_pressed, 0b0010_0000);
        assert_eq!(state.joypad.h_joypad_released, 0b0001_0000);
        assert_eq!(state.joypad.h_joy_last, 0b0001_0001);
        assert_eq!(state.joypad.h_joypad_sum, 0b0011_0001);

        state
            .apply_joypad_mask(B_PAD_DOWN)
            .expect("direct direction change is a valid single-frame input");
        assert_eq!(state.joypad.h_joypad_pressed, B_PAD_DOWN);
        assert_eq!(state.joypad.h_joypad_down, B_PAD_DOWN);
        assert_eq!(state.joypad.h_joypad_sum & B_PAD_RIGHT, B_PAD_RIGHT);
        assert_eq!(state.joypad.h_joypad_sum & B_PAD_DOWN, B_PAD_DOWN);
        state
            .joypad
            .validate_saved_state()
            .expect("joypad accumulator may contain directions pressed across different frames");
    }

    #[test]
    fn rng_seed_commit_records_authoritative_before_after_seed() {
        let mut state = GameState {
            rng_seed: 0x1234_5678,
            ..GameState::default()
        };

        assert_eq!(
            state.commit_rng_seed(0xfeed_beef),
            RngSeedCommit {
                rng_seed_before: 0x1234_5678,
                rng_seed_after: 0xfeed_beef,
            }
        );
        assert_eq!(state.rng_seed, 0xfeed_beef);
    }

    #[test]
    fn no_active_battle_guard_reports_exact_active_battle_kind() {
        let species = crate::models::PokemonSpecies::new_for_tests(
            "CHIKORITA",
            crate::models::BaseStats::new(45, 49, 49, 45, 65, 65),
        );
        let pokemon = Pokemon::new_for_tests(species, 6, crate::models::Dv::default());
        assert_eq!(GameState::default().require_no_active_battle(), Ok(()));

        let mut wild = GameState::default();
        wild.battle = BattleMemory::Wild {
            battle_type: "BATTLETYPE_NORMAL".to_string(),
            battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
            map_name: "ROUTE_29".to_string(),
            enemy_pokemon: pokemon.clone(),
            enemy_party: vec![pokemon.clone()],
        };
        assert_eq!(
            wild.require_no_active_battle(),
            Err(GameStateBattleError::ActiveBattle {
                battle_kind: "wild".to_string(),
            })
        );

        let mut static_wild = GameState::default();
        static_wild.battle = BattleMemory::StaticWild {
            battle_type: "BATTLETYPE_NORMAL".to_string(),
            battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
            species: "SUDOWOODO".to_string(),
            level: 30,
            source_script: "Route36SudowoodoScript".to_string(),
            enemy_pokemon: pokemon.clone(),
            enemy_party: vec![pokemon.clone()],
        };
        assert_eq!(
            static_wild.require_no_active_battle(),
            Err(GameStateBattleError::ActiveBattle {
                battle_kind: "static_wild".to_string(),
            })
        );

        let mut trainer = GameState::default();
        trainer.battle = BattleMemory::Trainer {
            battle_type: "BATTLETYPE_TRAINER".to_string(),
            trainer_class: "YOUNGSTER".to_string(),
            trainer_id: "YOUNGSTER_JOEY".to_string(),
            trainer_name: "JOEY".to_string(),
            event_flag: "EVENT_BEAT_YOUNGSTER_JOEY".to_string(),
            seen_text: "YoungsterJoeySeenText".to_string(),
            win_text: "YoungsterJoeyWinText".to_string(),
            loss_text: "YoungsterJoeyLossText".to_string(),
            callback: "TrainerCallback".to_string(),
            source_script: "Route30YoungsterJoeyScript".to_string(),
            enemy_pokemon: pokemon.clone(),
            enemy_party: vec![pokemon],
            reward: 4,
            encounter_music: "MUSIC_YOUNGSTER_ENCOUNTER".to_string(),
            ai_move_flags: 0,
            ai_item_switch_flags: 0,
            ai_layers: Vec::new(),
        };
        assert_eq!(
            trainer.require_no_active_battle(),
            Err(GameStateBattleError::ActiveTrainerBattle {
                trainer_id: "YOUNGSTER_JOEY".to_string(),
            })
        );
    }

    #[test]
    fn joypad_commands_reject_conflicting_directions_without_mutation() {
        let mut state = GameState::default();

        assert_eq!(
            state.apply_joypad_mask(B_PAD_LEFT | B_PAD_RIGHT),
            Err(GameStateFrameError::ConflictingJoypadDirections {
                mask: B_PAD_LEFT | B_PAD_RIGHT,
            })
        );
        assert_eq!(state.joypad, JoypadMemory::default());

        assert_eq!(
            state.apply_command(GameCommand::Joypad {
                mask: B_PAD_UP | B_PAD_DOWN,
            }),
            Err(GameStateFrameError::ConflictingJoypadDirections {
                mask: B_PAD_UP | B_PAD_DOWN,
            })
        );
        assert_eq!(state.joypad, JoypadMemory::default());
    }

    #[test]
    fn game_commands_mutate_state_and_return_explicit_events() {
        let mut state = GameState::default();

        assert_eq!(
            state
                .apply_command(GameCommand::Joypad { mask: 0b0001_0000 })
                .expect("joypad command"),
            vec![GameEvent::JoypadChanged {
                pressed: 0b0001_0000,
                down: 0b0001_0000,
            }]
        );
        assert_eq!(
            state
                .apply_command(GameCommand::AdvanceFrame)
                .expect("frame command"),
            vec![GameEvent::FrameAdvanced { frame: 1 }]
        );
        assert_eq!(state.frame_counter, 1);
    }

    #[test]
    fn advance_frame_command_rejects_overflow_without_mutation() {
        let mut state = GameState {
            frame_counter: u64::MAX,
            ..GameState::default()
        };

        assert_eq!(
            state.apply_command(GameCommand::AdvanceFrame),
            Err(GameStateFrameError::FrameCursorOverflow { frame: u64::MAX })
        );
        assert_eq!(state.frame_counter, u64::MAX);
    }

    #[test]
    fn overworld_memory_serializes_aligned_active_position_for_saves_and_sync() {
        let memory = OverworldMemory::Active {
            map_name: "PlayersHouse2F".to_string(),
            tile: TilePosition::new(4, 4),
            facing: Direction::Down,
            mode: MovementMode::Normal,
        };
        let json = serde_json::to_string(&memory).expect("serialize overworld memory");

        assert_eq!(
            json,
            r#"{"active":{"map_name":"PlayersHouse2F","tile":{"x":4,"y":4},"facing":"down","mode":"normal"}}"#
        );
        assert_eq!(
            serde_json::from_str::<OverworldMemory>(&json).expect("deserialize overworld memory"),
            memory
        );
    }

    fn scene_table() -> MapSceneTable {
        MapSceneTable {
            scenes: vec![
                crate::map::MapScene {
                    scene_id: "SCENE_ELMSLAB_MEET_ELM".to_string(),
                    script_name: Some("ElmsLabMeetElmScene".to_string()),
                },
                crate::map::MapScene {
                    scene_id: "SCENE_ELMSLAB_NOOP".to_string(),
                    script_name: None,
                },
            ],
        }
    }

    #[test]
    fn scene_memory_initializes_from_explicit_pack_order() {
        let mut memory = SceneMemory::default();
        let status = memory
            .enter_map("ElmsLab", &scene_table())
            .expect("initialize scene");

        assert_eq!(status.scene_name, "SCENE_ELMSLAB_MEET_ELM");
        assert_eq!(status.scene_index, 0);
        assert_eq!(status.script_name, Some("ElmsLabMeetElmScene".to_string()));
        assert_eq!(
            memory.map_scenes["ElmsLab"],
            "SCENE_ELMSLAB_MEET_ELM".to_string()
        );
        assert_eq!(memory.map_scene_indices["ElmsLab"], 0);
        assert_eq!(memory.scene_name, "SCENE_ELMSLAB_MEET_ELM");
    }

    #[test]
    fn scene_memory_sets_exact_scene_without_case_coercion() {
        let mut memory = SceneMemory::default();
        memory
            .enter_map("ElmsLab", &scene_table())
            .expect("initialize scene");
        let status = memory
            .set_current_scene("SCENE_ELMSLAB_NOOP", &scene_table())
            .expect("set scene");

        assert_eq!(status.scene_index, 1);
        assert_eq!(status.script_name, None);
        assert_eq!(memory.scene_name, "SCENE_ELMSLAB_NOOP");
        assert_eq!(memory.map_scene_indices["ElmsLab"], 1);
        assert_eq!(
            memory.set_current_scene("scene_elmslab_noop", &scene_table()),
            Err(SceneError::UnknownScene {
                map_name: "ElmsLab".to_string(),
                scene_name: "scene_elmslab_noop".to_string(),
            })
        );
    }

    #[test]
    fn scene_memory_rejects_empty_scene_tables() {
        let mut memory = SceneMemory::default();
        assert_eq!(
            memory.enter_map("Route29", &MapSceneTable::default()),
            Err(SceneError::EmptySceneTable {
                map_name: "Route29".to_string(),
            })
        );
    }

    #[test]
    fn saved_scene_memory_validates_exact_tokens_and_index_pairs() {
        let mut memory = SceneMemory::default();
        memory.current_map_name = "Elms Lab".to_string();
        assert_eq!(
            memory.validate(),
            Err(SceneError::InvalidMapName {
                map_name: "Elms Lab".to_string(),
            })
        );

        let mut memory = SceneMemory::default();
        memory
            .map_scenes
            .insert("ElmsLab".to_string(), "SCENE_ELMSLAB_NOOP".to_string());
        assert_eq!(
            memory.validate(),
            Err(SceneError::MissingSceneIndex {
                map_name: "ElmsLab".to_string(),
            })
        );

        let mut memory = SceneMemory::default();
        memory.map_scene_indices.insert("ElmsLab".to_string(), 1);
        assert_eq!(
            memory.validate(),
            Err(SceneError::UnexpectedSceneIndex {
                map_name: "ElmsLab".to_string(),
            })
        );

        let mut state = GameState::default();
        state
            .scenes
            .map_scenes
            .insert("ElmsLab".to_string(), "SCENE ELMSLAB NOOP".to_string());
        state
            .scenes
            .map_scene_indices
            .insert("ElmsLab".to_string(), 1);
        assert_eq!(
            state.validate_saved_state(),
            Err("invalid saved scene memory: invalid scene name SCENE ELMSLAB NOOP".to_string())
        );
    }

    #[test]
    fn validate_saved_scene_references_rejects_missing_maps_and_scenes() {
        let mut memory = SceneMemory {
            current_map_name: "ElmsLab".to_string(),
            scene_name: "SCENE_ELMSLAB_MEET_ELM".to_string(),
            ..SceneMemory::default()
        };

        let error = validate_saved_scene_references(&memory, |_| false, |_, _| None)
            .expect_err("current scene map must exist in compiled maps");
        assert_eq!(
            error,
            SceneSaveError::MissingMap {
                path: "scenes.current_map_name",
                map_name: "ElmsLab".to_string(),
            }
        );

        let error = validate_saved_scene_references(&memory, |_| true, |_, _| Some(0))
            .expect_err("current scene must have a definitive per-map entry");
        assert_eq!(
            error,
            SceneSaveError::CurrentSceneMissingMapSceneEntry {
                map_name: "ElmsLab".to_string(),
                scene_name: "SCENE_ELMSLAB_MEET_ELM".to_string(),
            }
        );

        memory
            .map_scenes
            .insert("ElmsLab".to_string(), "SCENE_ELMSLAB_NOOP".to_string());
        memory.map_scene_indices.insert("ElmsLab".to_string(), 1);
        let error = validate_saved_scene_references(&memory, |_| true, |_, _| Some(0))
            .expect_err("current scene must match definitive per-map entry");
        assert_eq!(
            error,
            SceneSaveError::CurrentSceneMapSceneMismatch {
                map_name: "ElmsLab".to_string(),
                scene_name: "SCENE_ELMSLAB_MEET_ELM".to_string(),
                saved_scene_name: "SCENE_ELMSLAB_NOOP".to_string(),
            }
        );

        memory
            .map_scenes
            .insert("ElmsLab".to_string(), "SCENE_ELMSLAB_MEET_ELM".to_string());
        memory.map_scene_indices.insert("ElmsLab".to_string(), 0);
        let error = validate_saved_scene_references(&memory, |_| true, |_, _| None)
            .expect_err("current scene must exist in compiled scene table");
        assert_eq!(
            error,
            SceneSaveError::MissingScene {
                path: "scenes.current",
                map_name: "ElmsLab".to_string(),
                scene_name: "SCENE_ELMSLAB_MEET_ELM".to_string(),
            }
        );

        memory = SceneMemory::default();
        memory
            .map_scenes
            .insert("Route29".to_string(), "SCENE_ROUTE29_NOOP".to_string());
        memory.map_scene_indices.insert("Route29".to_string(), 0);
        let error = validate_saved_scene_references(&memory, |_| false, |_, _| None)
            .expect_err("saved map scene map must exist in compiled maps");
        assert_eq!(
            error,
            SceneSaveError::MissingMap {
                path: "scenes.map_scenes",
                map_name: "Route29".to_string(),
            }
        );
    }

    #[test]
    fn validate_saved_scene_references_checks_saved_index_against_compiled_index() {
        let mut memory = SceneMemory::default();
        memory
            .map_scenes
            .insert("ElmsLab".to_string(), "SCENE_ELMSLAB_NOOP".to_string());

        let error = validate_saved_scene_references(&memory, |_| true, |_, _| Some(1))
            .expect_err("saved map scene must carry saved scene index");
        assert_eq!(
            error,
            SceneSaveError::MissingSceneIndex {
                path: "scenes.map_scenes",
                map_name: "ElmsLab".to_string(),
                scene_name: "SCENE_ELMSLAB_NOOP".to_string(),
            }
        );

        memory.map_scene_indices.insert("ElmsLab".to_string(), 0);
        let error = validate_saved_scene_references(&memory, |_| true, |_, _| Some(1))
            .expect_err("saved scene index must match compiled scene order");
        assert_eq!(
            error,
            SceneSaveError::SceneIndexMismatch {
                path: "scenes.map_scenes",
                map_name: "ElmsLab".to_string(),
                scene_name: "SCENE_ELMSLAB_NOOP".to_string(),
                saved_index: 0,
                compiled_index: 1,
            }
        );
    }

    #[test]
    fn event_flags_are_exact_strings_without_case_coercion() {
        let mut flags = EventFlagMemory::default();
        flags
            .set_event_flag("EVENT_ROUTE_29_POTION", true)
            .expect("set flag");

        assert_eq!(flags.is_event_flag_set("EVENT_ROUTE_29_POTION"), Ok(true));
        assert_eq!(flags.is_event_flag_set("event_route_29_potion"), Ok(false));
        assert_eq!(
            flags.active_event_flags().cloned().collect::<Vec<_>>(),
            vec!["EVENT_ROUTE_29_POTION".to_string()]
        );
    }

    #[test]
    fn script_flags_route_engine_prefixes_to_engine_store() {
        let mut flags = EventFlagMemory::default();
        flags
            .set_script_flag("ENGINE_ZEPHYRBADGE", true)
            .expect("set engine flag");
        flags
            .set_script_flag("EVENT_BEAT_YOUNGSTER_JOEY", true)
            .expect("set event flag");

        assert_eq!(flags.is_engine_flag_set("ENGINE_ZEPHYRBADGE"), Ok(true));
        assert_eq!(flags.is_event_flag_set("ENGINE_ZEPHYRBADGE"), Ok(false));
        assert_eq!(
            flags.is_event_flag_set("EVENT_BEAT_YOUNGSTER_JOEY"),
            Ok(true)
        );
        flags
            .clear_script_flag("ENGINE_ZEPHYRBADGE")
            .expect("clear engine flag");
        assert_eq!(flags.is_engine_flag_set("ENGINE_ZEPHYRBADGE"), Ok(false));
    }

    #[test]
    fn validate_saved_mystery_gift_references_requires_compiled_items() {
        let mystery_gift = MysteryGiftState {
            stored_item: Some("GOLD_LEAF".to_string()),
            backup_item: Some("SILVER_LEAF".to_string()),
            trainer_house_flag: false,
        };

        let error =
            validate_saved_mystery_gift_references(&mystery_gift, |item| item == "SILVER_LEAF")
                .expect_err("stored mystery gift item must exist in compiled items");
        assert_eq!(
            error,
            MysteryGiftSaveError::MissingItem {
                path: "mystery_gift.stored_item",
                item_id: "GOLD_LEAF".to_string(),
            }
        );

        let error =
            validate_saved_mystery_gift_references(&mystery_gift, |item| item == "GOLD_LEAF")
                .expect_err("backup mystery gift item must exist in compiled items");
        assert_eq!(
            error,
            MysteryGiftSaveError::MissingItem {
                path: "mystery_gift.backup_item",
                item_id: "SILVER_LEAF".to_string(),
            }
        );
    }

    #[test]
    fn validate_saved_link_session_references_requires_compiled_active_room() {
        let link_session = LinkSessionState {
            active_room: Some("CableClubTradeCenter".to_string()),
            ..LinkSessionState::default()
        };

        let error = validate_saved_link_session_references(&link_session, |_| false)
            .expect_err("active link room must exist in compiled special routines");
        assert_eq!(
            error,
            LinkSessionSaveError::MissingActiveRoom {
                room: "CableClubTradeCenter".to_string(),
            }
        );

        assert_eq!(
            validate_saved_link_session_references(&link_session, |room| {
                room == "CableClubTradeCenter"
            }),
            Ok(())
        );
    }

    #[test]
    fn validate_saved_flag_references_rejects_cross_stored_flags() {
        let mut flags = EventFlagMemory::default();
        flags
            .event_flags
            .insert("ENGINE_ZEPHYRBADGE".to_string(), true);

        let error = validate_saved_flag_references(&flags, |_| true, |_| true)
            .expect_err("engine flag cannot be saved in event flag memory");
        assert_eq!(
            error,
            EventFlagSaveError::EngineFlagSavedAsEventFlag {
                path: "flags.event_flags".to_string(),
                flag_name: "ENGINE_ZEPHYRBADGE".to_string(),
            }
        );
        assert_eq!(
            validate_saved_event_flag_reference(
                "script_runtime.pending_events[0]",
                "ENGINE_ZEPHYRBADGE",
                |_| true,
            ),
            Err(EventFlagSaveError::EngineFlagSavedAsEventFlag {
                path: "script_runtime.pending_events[0]".to_string(),
                flag_name: "ENGINE_ZEPHYRBADGE".to_string(),
            })
        );

        flags = EventFlagMemory::default();
        flags
            .engine_flags
            .insert("EVENT_BEAT_YOUNGSTER_JOEY".to_string(), true);

        let error = validate_saved_flag_references(&flags, |_| true, |_| true)
            .expect_err("event flag cannot be saved in engine flag memory");
        assert_eq!(
            error,
            EventFlagSaveError::EventFlagSavedAsEngineFlag {
                path: "flags.engine_flags".to_string(),
                flag_name: "EVENT_BEAT_YOUNGSTER_JOEY".to_string(),
            }
        );
        assert_eq!(
            validate_saved_engine_flag_reference(
                "script_runtime.pending_events[0]",
                "EVENT_BEAT_YOUNGSTER_JOEY",
                |_| true,
            ),
            Err(EventFlagSaveError::EventFlagSavedAsEngineFlag {
                path: "script_runtime.pending_events[0]".to_string(),
                flag_name: "EVENT_BEAT_YOUNGSTER_JOEY".to_string(),
            })
        );
    }

    #[test]
    fn validate_saved_flag_references_requires_compiled_declarations() {
        let mut flags = EventFlagMemory::default();
        flags
            .event_flags
            .insert("EVENT_BEAT_YOUNGSTER_JOEY".to_string(), true);

        let error = validate_saved_flag_references(&flags, |_| false, |_| true)
            .expect_err("saved event flag must be declared by compiled pack");
        assert_eq!(
            error,
            EventFlagSaveError::MissingEventFlag {
                path: "flags.event_flags".to_string(),
                flag_name: "EVENT_BEAT_YOUNGSTER_JOEY".to_string(),
            }
        );

        flags = EventFlagMemory::default();
        flags
            .engine_flags
            .insert("ENGINE_ZEPHYRBADGE".to_string(), true);

        let error = validate_saved_flag_references(&flags, |_| true, |_| false)
            .expect_err("saved engine flag must be declared by compiled pack");
        assert_eq!(
            error,
            EventFlagSaveError::MissingEngineFlag {
                path: "flags.engine_flags".to_string(),
                flag_name: "ENGINE_ZEPHYRBADGE".to_string(),
            }
        );
    }

    #[test]
    fn empty_flag_names_are_errors() {
        let mut flags = EventFlagMemory::default();
        assert_eq!(
            flags.set_event_flag("", true),
            Err(EventFlagError::EmptyFlagName)
        );
        assert_eq!(
            flags.is_script_flag_set(""),
            Err(EventFlagError::EmptyFlagName)
        );
    }

    #[test]
    fn flag_names_reject_non_token_content_without_trimming() {
        let mut flags = EventFlagMemory::default();
        assert_eq!(
            flags.set_event_flag(" EVENT_ROUTE_29_POTION", true),
            Err(EventFlagError::InvalidFlagName {
                flag_name: " EVENT_ROUTE_29_POTION".to_string(),
            })
        );
        assert_eq!(
            flags.set_script_flag("ENGINE_ZEPHYR BADGE", true),
            Err(EventFlagError::InvalidFlagName {
                flag_name: "ENGINE_ZEPHYR BADGE".to_string(),
            })
        );
        assert_eq!(
            flags.is_script_flag_set("EVENT_ROUTE_29_POTION\n"),
            Err(EventFlagError::InvalidFlagName {
                flag_name: "EVENT_ROUTE_29_POTION\n".to_string(),
            })
        );
        assert!(flags.event_flags.is_empty());
        assert!(flags.engine_flags.is_empty());
    }

    #[test]
    fn saved_flag_maps_validate_exact_keys_after_deserialize() {
        let mut flags = EventFlagMemory::default();
        flags.event_flags.insert("EVENT_BAD FLAG".to_string(), true);
        assert_eq!(
            flags.validate(),
            Err(EventFlagError::InvalidFlagName {
                flag_name: "EVENT_BAD FLAG".to_string(),
            })
        );

        let mut state = GameState::default();
        state
            .flags
            .engine_flags
            .insert("ENGINE_ZEPHYRBADGE\n".to_string(), true);
        assert_eq!(
            state.validate_saved_state(),
            Err("invalid saved event flags: invalid flag name ENGINE_ZEPHYRBADGE\n".to_string())
        );
    }

    #[test]
    fn saved_flags_and_scenes_reject_reserved_runtime_prefix_tokens() {
        let mut flags = EventFlagMemory::default();
        assert_eq!(
            flags.set_event_flag("fallback_EVENT_ROUTE_29_POTION", true),
            Err(EventFlagError::InvalidFlagName {
                flag_name: "fallback_EVENT_ROUTE_29_POTION".to_string(),
            })
        );

        let scenes = SceneMemory {
            current_map_name: "legacyRoute29".to_string(),
            ..SceneMemory::default()
        };
        assert_eq!(
            scenes.validate(),
            Err(SceneError::InvalidMapName {
                map_name: "legacyRoute29".to_string(),
            })
        );
    }
}
