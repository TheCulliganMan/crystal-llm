use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::map::MapSceneTable;
use crate::models::{Bag, MAX_PC_BOXES, PARTY_SIZE, PokedexState, Pokemon, PokemonStorage};
use crate::systems::script_audio::{
    SCRIPT_AUDIO_CRY_COMMANDS, SCRIPT_AUDIO_MUSIC_COMMANDS, SCRIPT_AUDIO_MUSIC_FADE_COMMANDS,
    SCRIPT_AUDIO_NO_PAYLOAD_COMMANDS, SCRIPT_AUDIO_SOUND_EFFECT_COMMANDS,
};
use crate::systems::script_runtime::script_runtime_command_arg_counts;
use crate::systems::script_text::SCRIPT_TEXT_LABEL_COMMANDS;
use crate::systems::script_warps::{SCRIPT_MAP_LOAD_COMMANDS, SCRIPT_MAP_REFRESH_COMMANDS};
use crate::systems::shop::{SCRIPT_SHOP_ZERO_MART_TYPES, is_known_script_mart_type};
use crate::systems::step_events::StepEventCounters;
use crate::systems::time::TimeState;
use crate::timing::Frame;
use crate::world::map::{Direction, TilePosition};
use crate::world::movement::MovementMode;
use crate::world::session::OverworldSnapshot;

pub const PLAYER_NAME_LENGTH: usize = 8;
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameState {
    pub options: Options,
    pub player_name: String,
    pub player_id: u16,
    pub party: PartyState,
    pub storage: PokemonStorage,
    pub bag: Bag,
    pub money: u32,
    pub moms_money: u32,
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
    pub repel_steps_remaining: u16,
    pub active_repel_item: Option<String>,
    pub dig_warp_map_name: Option<String>,
    pub dig_warp_index: Option<u16>,
    pub last_spawn_identifier: Option<u16>,
    pub kenji_break_timer: u8,
    pub player_palette_id: u8,
    pub map_block_overrides: BTreeMap<String, BTreeMap<(u16, u16), u16>>,
    pub map_object_overrides: BTreeMap<String, OverworldObjectMapMemory>,
    pub joypad: JoypadMemory,
    pub fishing: FishingMemory,
    pub step_events: StepEventCounters,
    pub time: TimeState,
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

impl Default for GameState {
    fn default() -> Self {
        Self {
            options: Options::default(),
            player_name: String::new(),
            player_id: 0,
            party: PartyState::default(),
            storage: PokemonStorage::default(),
            bag: Bag::default(),
            money: 0,
            moms_money: 0,
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
            repel_steps_remaining: 0,
            active_repel_item: None,
            dig_warp_map_name: None,
            dig_warp_index: None,
            last_spawn_identifier: None,
            kenji_break_timer: 0,
            player_palette_id: 0,
            map_block_overrides: BTreeMap::new(),
            map_object_overrides: BTreeMap::new(),
            joypad: JoypadMemory::default(),
            fishing: FishingMemory::default(),
            step_events: StepEventCounters::default(),
            time: TimeState::default(),
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

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MobileBattleTowerRecord {
    pub streak: u8,
    pub outcome: String,
    pub day: u8,
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
        if self.day >= 7 {
            return Err(format!(
                "mobile_link.leaderboard[{index}].day {} is outside weekday range 0..6",
                self.day
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BugContestState {
    pub park_balls_remaining: u8,
    pub timer_active: bool,
    pub timer_minutes_remaining: u8,
    pub timer_seconds_remaining: u8,
    pub party_backup: Vec<Pokemon>,
    pub second_party_species: Option<String>,
    pub caught_mon: Option<Pokemon>,
    pub caught_species: Option<String>,
    pub caught_level: Option<u8>,
    pub selected_contestant_flags: Vec<String>,
    pub last_rank: Option<u8>,
    pub last_result: Option<u8>,
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
        for flag in &self.selected_contestant_flags {
            validate_flag_name(flag)
                .map_err(|error| format!("bug_contest.selected_contestant_flags {error}"))?;
        }
        Ok(())
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MagikarpRecordState {
    pub current_feet: u8,
    pub current_inches: u8,
    pub best_feet: u8,
    pub best_inches: u8,
    pub best_owner_name: String,
}

impl MagikarpRecordState {
    fn validate_saved_state(&self) -> Result<(), String> {
        validate_inches_field("magikarp_record.current_inches", self.current_inches)?;
        validate_inches_field("magikarp_record.best_inches", self.best_inches)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DayCareState {
    pub man: DayCareResidentState,
    pub lady: DayCareResidentState,
    pub compatibility_score: u8,
    pub egg_present: bool,
    pub steps_until_next_egg: u16,
    pub steps_since_last_egg: u8,
    pub last_interaction: Option<DayCareInteractionState>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DayCareResidentState {
    pub pokemon: Option<Pokemon>,
    pub initial_experience: i32,
    pub initial_level: u8,
    pub steps: u32,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DayCareInteractionState {
    pub caretaker: String,
    pub action: String,
    pub success: bool,
    pub pokemon: Option<String>,
    pub level: Option<u8>,
    pub reason: Option<String>,
}

impl DayCareState {
    fn validate_saved_state(&self) -> Result<(), String> {
        self.man.validate_saved_state("day_care.man")?;
        self.lady.validate_saved_state("day_care.lady")?;
        let both_residents_active = self.man.active && self.lady.active;
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
            "deposit" => validate_day_care_interaction_payload(
                "day_care.last_interaction",
                self,
                DayCareInteractionShape {
                    success_pokemon: true,
                    success_reason: None,
                    failure_reason: Some("occupied"),
                    failure_pokemon: false,
                },
            ),
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

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MysteryGiftState {
    pub stored_item: Option<String>,
    pub backup_item: Option<String>,
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

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuenasPasswordState {
    pub category_index: usize,
    pub option_index: usize,
    pub generation_day: u8,
    pub generated: bool,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoamingPokemonState {
    pub species: String,
    pub level: u8,
    pub map_group: u16,
    pub map_number: u16,
    pub hp: u16,
    pub dvs: u16,
}

impl RoamingPokemonState {
    fn validate_saved_state(&self, index: usize) -> Result<(), String> {
        validate_script_runtime_token(&format!("roaming_pokemon[{index}].species"), &self.species)?;
        if self.level == 0 {
            return Err(format!("roaming_pokemon[{index}].level must be nonzero"));
        }
        if self.map_group == 0 {
            return Err(format!(
                "roaming_pokemon[{index}].map_group must be nonzero"
            ));
        }
        if self.map_number == 0 {
            return Err(format!(
                "roaming_pokemon[{index}].map_number must be nonzero"
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverworldObjectMapMemory {
    pub objects: BTreeMap<String, OverworldObjectMemory>,
    pub hidden_object_identifiers: BTreeSet<String>,
    pub following: Option<OverworldFollowMemory>,
    pub last_talked_object_identifier: Option<String>,
    pub player_hidden: bool,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverworldObjectMemory {
    pub x: u16,
    pub y: u16,
    pub facing: Option<Direction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverworldFollowMemory {
    pub leader_object_id: String,
    pub follower_object_id: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BattleMemory {
    #[default]
    Inactive,
    Wild {
        battle_type: String,
        map_name: String,
        enemy_pokemon: Pokemon,
        enemy_party: Vec<Pokemon>,
    },
    StaticWild {
        battle_type: String,
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

impl BattleMemory {
    fn validate_saved_state(&self) -> Result<(), String> {
        match self {
            Self::Inactive => Ok(()),
            Self::Wild {
                battle_type,
                map_name,
                enemy_pokemon,
                enemy_party,
            } => {
                validate_script_runtime_token("battle.wild.battle_type", battle_type)?;
                validate_script_runtime_token("battle.wild.map_name", map_name)?;
                validate_battle_enemy_party_state("battle.wild", enemy_pokemon, enemy_party)
            }
            Self::StaticWild {
                battle_type,
                species,
                level,
                source_script,
                enemy_pokemon,
                enemy_party,
            } => {
                validate_script_runtime_token("battle.static_wild.battle_type", battle_type)?;
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
                validate_script_runtime_token("battle.trainer.seen_text", seen_text)?;
                validate_script_runtime_token("battle.trainer.win_text", win_text)?;
                validate_script_runtime_token("battle.trainer.loss_text", loss_text)?;
                validate_script_runtime_token("battle.trainer.callback", callback)?;
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

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeMemory {
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
    pub next_script: Option<String>,
    pub call_stack: Vec<ScriptReturnFrame>,
    pub deferred_scripts: Vec<String>,
    pub script_ended: Option<ScriptEndState>,
    pub shop_events: Vec<ScriptShopRuntimeEvent>,
    pub pending_shop: Option<ScriptShopRequest>,
    pub item_use_events: Vec<ItemUseRuntimeEvent>,
}

impl ScriptRuntimeMemory {
    pub fn validate(&self) -> Result<(), String> {
        validate_optional_script_runtime_label("next_script", self.next_script.as_deref())?;
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
            validate_script_runtime_label(&format!("deferred_scripts[{index}]"), script)?;
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
            validate_numeric_buffer_write_payload(index, write)?;
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
    Ok(())
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

fn validate_audio_event_payload(
    index: usize,
    event: &ScriptAudioRuntimeEvent,
) -> Result<(), String> {
    match event.kind {
        ScriptAudioRuntimeKind::Music => {
            if !SCRIPT_AUDIO_MUSIC_COMMANDS.contains(&event.command.as_str()) {
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
            if !SCRIPT_AUDIO_SOUND_EFFECT_COMMANDS.contains(&event.command.as_str()) {
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
            if !SCRIPT_AUDIO_CRY_COMMANDS.contains(&event.command.as_str()) {
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
            if !SCRIPT_AUDIO_MUSIC_FADE_COMMANDS.contains(&event.command.as_str()) {
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
            if !SCRIPT_AUDIO_NO_PAYLOAD_COMMANDS.contains(&event.command.as_str()) {
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
                event.frames.unwrap_or_default()
            ));
        }
    } else if has_fade_payload {
        return Err(format!(
            "graphics_events[{index}] fade payload is only valid for ScreenFade"
        ));
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

fn validate_text_event_payload(index: usize, event: &ScriptTextRuntimeEvent) -> Result<(), String> {
    match event.kind {
        ScriptTextRuntimeKind::Write => {
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
                "writetext" => {
                    if event.face_player {
                        return Err(format!(
                            "text_events[{index}].face_player must be false for writetext"
                        ));
                    }
                    if event.closes_text {
                        return Err(format!(
                            "text_events[{index}].closes_text must be false for writetext"
                        ));
                    }
                }
                "jumptext" => {
                    if event.face_player {
                        return Err(format!(
                            "text_events[{index}].face_player must be false for jumptext"
                        ));
                    }
                    if !event.closes_text {
                        return Err(format!(
                            "text_events[{index}].closes_text must be true for jumptext"
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

fn validate_pending_text_wait_command(command: &str) -> Result<(), String> {
    if matches!(
        command,
        "promptbutton" | "waitbutton" | "jumptext" | "jumptextfaceplayer"
    ) {
        Ok(())
    } else {
        Err(format!(
            "pending_text_wait.command {command} is not a saved text wait command"
        ))
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

fn validate_inches_field(field: &str, value: u8) -> Result<(), String> {
    if value >= 12 {
        return Err(format!("{field} {value} is outside inches range 0..11"));
    }
    Ok(())
}

fn validate_script_runtime_token(field: &str, value: &str) -> Result<(), String> {
    if is_exact_script_runtime_token(value) {
        Ok(())
    } else {
        Err(format!("{field} has invalid token '{value}'"))
    }
}

fn is_exact_script_runtime_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn is_exact_script_runtime_label(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'@'))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemUseRuntimeEvent {
    pub item_id: String,
    pub context: String,
    pub consumed: bool,
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
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "snake_case")]
pub enum ScriptFadeColor {
    White,
    Black,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "snake_case")]
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
    pub source_script: String,
    pub next_command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptEndState {
    pub callback: bool,
    pub just_battled_guard: bool,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptShopRuntimeEvent {
    pub mart_type: String,
    pub mart_id: String,
    pub inventory: Vec<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptShopRequest {
    pub mart_type: String,
    pub mart_id: String,
    pub inventory: Vec<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeDelay {
    pub command: String,
    pub frames: u16,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeEarthquake {
    pub parameter: u16,
    pub shake_frames: u16,
    pub sleep_frames: u16,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeEmote {
    pub emote: String,
    pub object: String,
    pub duration: u16,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeQueuedCommand {
    pub command: String,
    pub target: String,
    pub bank: Option<String>,
    pub source_script: String,
    pub command_index: usize,
}

impl GameState {
    pub fn validate_saved_state(&self) -> Result<(), String> {
        validate_saved_player_name(&self.player_name)?;
        self.bag
            .validate()
            .map_err(|error| format!("invalid saved bag: {error}"))?;
        self.storage
            .validate_metadata()
            .map_err(|error| format!("invalid saved storage: {error}"))?;
        self.time
            .validate_saved_state()
            .map_err(|error| format!("invalid saved time: {error}"))?;
        let projected_party = PartyState::from_storage(&self.storage);
        if self.party != projected_party {
            return Err("saved party projection does not match authoritative storage".to_string());
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
        self.script_runtime
            .validate()
            .map_err(|error| format!("invalid saved script runtime: {error}"))?;
        Ok(())
    }

    fn validate_saved_battle_cursors(&self) -> Result<(), String> {
        let Some(enemy_party_len) = self.battle.enemy_party_len() else {
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
            return Ok(());
        };
        if enemy_party_len == 0 {
            return Ok(());
        }
        if self.battle_active_enemy_party_index.is_none() {
            return Err(
                "battle_active_enemy_party_index must be saved for an active battle".to_string(),
            );
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum GameStateFrameError {
    #[error("game state frame cursor overflowed at frame {frame}")]
    FrameCursorOverflow { frame: u64 },
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventFlagMemory {
    pub event_flags: BTreeMap<String, bool>,
    pub engine_flags: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum EventFlagError {
    #[error("empty flag name")]
    EmptyFlagName,
    #[error("invalid flag name {flag_name}")]
    InvalidFlagName { flag_name: String },
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
    flag_name
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneMemory {
    pub current_map_name: String,
    pub scene_name: String,
    pub map_scenes: BTreeMap<String, String>,
    pub map_scene_indices: BTreeMap<String, usize>,
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
#[serde(rename_all = "snake_case")]
pub enum TextSpeed {
    Fast,
    Mid,
    Slow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BattleScene {
    On,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BattleStyle {
    Shift,
    Set,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sound {
    Mono,
    Stereo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MenuAccount {
    On,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrintOption {
    Normal,
    Lightest,
    Lighter,
    Darker,
    Darkest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "snake_case")]
pub enum FishingRodState {
    #[default]
    Idle,
    Waiting,
    Bite,
    Battle,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FishingMemory {
    pub rod_state: FishingRodState,
    pub rod_index: Option<u8>,
    pub bites_remaining: u8,
    pub result: u8,
    pub daily_flags1: u8,
    pub swarm_flag: u8,
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GameCommand {
    Joypad { mask: u8 },
    OpenMenu,
    CloseMenu,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GameEvent {
    FrameAdvanced { frame: u64 },
    JoypadChanged { pressed: u8, down: u8 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state_matches_typescript_defaults_that_affect_gameplay() {
        let state = GameState::default();
        assert_eq!(state.options.text_speed, TextSpeed::Fast);
        assert_eq!(state.player_name, "");
        assert_eq!(state.player_id, 0);
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
        assert_eq!(state.repel_steps_remaining, 0);
        assert_eq!(state.active_repel_item, None);
        assert_eq!(state.dig_warp_map_name, None);
        assert_eq!(state.dig_warp_index, None);
        assert_eq!(state.kenji_break_timer, 0);
        assert_eq!(state.player_palette_id, 0);
        assert_eq!(state.step_events, StepEventCounters::default());
        assert_eq!(state.time, TimeState::default());
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
        state.mobile_link.login_password = "EIGHTEEN-CHARS!!!!!".to_string();
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "mobile_link.login_password length 18 exceeds Crystal mobile password limit 17"
                    .to_string()
            )
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
            outcome: "WIN".to_string(),
            day: 7,
        });
        assert_eq!(
            state.validate_saved_state(),
            Err("mobile_link.leaderboard[0].day 7 is outside weekday range 0..6".to_string())
        );

        state = GameState::default();
        state.mobile_link.leaderboard.push(MobileBattleTowerRecord {
            streak: 7,
            outcome: "WIN".to_string(),
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
        state.storage.party.pokemon[1] = Some(pokemon.clone());
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
                tile: TilePosition::new(1, 2),
                facing: Direction::Down,
                mode: MovementMode::Normal,
            },
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err("overworld.active.map_name has invalid token 'Route 29'".to_string())
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
            battle: BattleMemory::Wild {
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                map_name: "Route29".to_string(),
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon.clone()],
            },
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err("battle_active_enemy_party_index must be saved for an active battle".to_string())
        );

        state = GameState {
            battle: BattleMemory::Wild {
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                map_name: "Route29".to_string(),
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon.clone()],
            },
            battle_active_enemy_party_index: Some(1),
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err("battle_active_enemy_party_index 1 is outside enemy party range 0..1".to_string())
        );

        state = GameState {
            battle: BattleMemory::Wild {
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                map_name: "Route29".to_string(),
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon.clone()],
            },
            battle_rewarded_enemy_party_indices: BTreeSet::from([1]),
            ..GameState::default()
        };
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
                map_name: "Route29".to_string(),
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon.clone()],
            },
            battle_active_enemy_party_index: Some(0),
            battle_rewarded_enemy_party_indices: BTreeSet::from([0]),
            ..GameState::default()
        };
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
                map_name: "Route29".to_string(),
                enemy_pokemon: damaged_enemy,
                enemy_party: vec![pokemon.clone()],
            },
            battle_active_enemy_party_index: Some(0),
            ..GameState::default()
        };
        assert_eq!(
            state.validate_saved_state(),
            Err(
                "battle_active_enemy_party_index 0 does not match battle enemy_pokemon".to_string()
            )
        );

        state = GameState {
            battle: BattleMemory::Wild {
                battle_type: "BATTLETYPE NORMAL".to_string(),
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
    fn saved_script_runtime_validates_control_continuation_labels() {
        let mut runtime = ScriptRuntimeMemory {
            next_script: Some(" .Done@Script".to_string()),
            ..ScriptRuntimeMemory::default()
        };
        assert_eq!(
            runtime.validate(),
            Err("next_script has invalid script label ' .Done@Script'".to_string())
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
        runtime.next_script = Some(".Done@Script".to_string());
        runtime
            .deferred_scripts
            .push(".Deferred @Script".to_string());
        assert_eq!(
            runtime.validate(),
            Err("deferred_scripts[0] has invalid script label '.Deferred @Script'".to_string())
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

        runtime = ScriptRuntimeMemory::default();
        runtime.call_stack.push(ScriptReturnFrame {
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
            tile: Some(TilePosition::new(1, 2)),
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
            tile: Some(TilePosition::new(1, 2)),
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
            tile: Some(TilePosition { x: 1, y: 2 }),
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
            tile: Some(TilePosition::new(1, 2)),
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

        runtime = ScriptRuntimeMemory::default();
        runtime.command_queue.push(ScriptRuntimeQueuedCommand {
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
    fn overworld_memory_serializes_exact_active_position_for_saves_and_sync() {
        let memory = OverworldMemory::Active {
            map_name: "PlayersHouse2F".to_string(),
            tile: TilePosition::new(3, 3),
            facing: Direction::Down,
            mode: MovementMode::Normal,
        };
        let json = serde_json::to_string(&memory).expect("serialize overworld memory");

        assert_eq!(
            json,
            r#"{"active":{"map_name":"PlayersHouse2F","tile":{"x":3,"y":3},"facing":"down","mode":"normal"}}"#
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
}
