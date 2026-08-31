use std::path::PathBuf;

use crystal_assets::{ModpackAudioLoopPolicy, ModpackAudioPlaybackMode, RuntimeTitleScreen};
use crystal_core::models::pokemon::{StatExperience, calculate_stats};
use crystal_core::state::{
    CardFlipInput, MemoryGameButton, MemoryGameInput, MemoryGamePhase, MemoryGameState,
    OverworldObjectMapMemory, OverworldObjectMemory, ScriptTextRuntimeKind, SlotMachineInput,
};
use crystal_core::systems::script_objects::ScriptMovementEffect;
use crystal_core::world::collision::permissions;
use crystal_core::world::encounters::FieldEncounterKind;
use crystal_core::world::fishing::ROD_OLD;
use crystal_core::world::session::OverworldFollowState;

fn repository_root_for_tests() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("workspace is nested under rust/crates/crystal-bevy")
        .to_path_buf()
}

include!("runtime_tests/runtime_basics.rs");
include!("runtime_tests/overworld.rs");
include!("runtime_tests/unown_puzzles.rs");
include!("runtime_tests/battle_turns.rs");
include!("runtime_tests/special_routines.rs");
include!("runtime_tests/battle_items.rs");
include!("runtime_tests/save_validation.rs");

fn static_wild_battle_start_for_tests(
    data: &GameDataSet,
    request: StaticWildBattleRequest,
) -> StaticWildBattleStart {
    let mut divider = crystal_core::random::ReplayDivider::new([0; 8]);
    data.static_wild_battle_start(
        request,
        crystal_core::random::CrystalRandomState::default(),
        &mut divider,
    )
    .expect("static wild battle test fixture")
}

fn static_wild_origin_from_state(state: &GameState) -> RuntimeStaticWildBattleOrigin {
    let BattleMemory::StaticWild {
        battle_type,
        origin_map_name,
        species,
        level,
        source_script,
        startbattle_command_index,
        resume_command_index,
        ..
    } = &state.battle
    else {
        panic!("test fixture requires an active static wild battle");
    };
    RuntimeStaticWildBattleOrigin {
        map_name: origin_map_name.clone(),
        source_script: source_script.clone(),
        startbattle_command_index: *startbattle_command_index,
        resume_command_index: *resume_command_index,
        battle_type: battle_type.clone(),
        species: species.clone(),
        level: *level,
    }
}
