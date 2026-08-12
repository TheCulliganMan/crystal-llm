fn bevy_shell_source() -> &'static str {
    concat!(
        include_str!("../../bevy_shell.rs"),
        include_str!("../deterministic_session.rs"),
        include_str!("../trainer_card.rs"),
        include_str!("../title_menu.rs"),
        include_str!("../credits.rs"),
        include_str!("../script_callbacks.rs"),
        include_str!("../economy.rs"),
        include_str!("../battle_messages.rs"),
        include_str!("../battle_results.rs"),
        include_str!("../battle_entry.rs"),
        include_str!("../menu_rendering.rs"),
        include_str!("../render_mod.rs"),
        include_str!("../overworld_rendering.rs"),
        include_str!("../start_menu.rs"),
        include_str!("../bitmap_font.rs"),
        include_str!("../graphics_assets.rs"),
        include_str!("../field_pack.rs"),
    )
}

/// Match the desktop executable: the game data comes from the explicit
/// compiled pack, while artwork resolves from the workspace asset root.
/// `load_from_compiled_pack` intentionally addresses web runtime data and
/// therefore cannot exercise the native desktop pack in this test.
fn workspace_desktop_runtime(asset_root: &AssetRoot) -> CrystalRuntime {
    let pack_path = std::env::var_os("CRYSTAL_RENDER_TEST_PACK")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            asset_root
                .repository_root
                .join("content-packs/core-modular.crystalpack")
        });
    let loaded = crystal_assets::read_loaded_verified_compiled_game_pack(&pack_path)
        .expect("load desktop compiled pack");
    CrystalRuntime::from_loaded_compiled_pack(asset_root, loaded)
        .expect("construct desktop runtime from compiled pack")
}

#[test]
fn release_shell_contains_no_developer_shortcut_dispatcher() {
    let source = bevy_shell_source();
    let dispatcher = format!("{}_{}_{}", "runtime", "developer", "shortcuts_enabled");
    assert!(
        !source.contains(&dispatcher),
        "release shell must not compile a developer keyboard dispatcher"
    );
}

#[test]
fn release_hotkey_mapper_has_no_space_or_escape_aliases() {
    let source = bevy_shell_source();
    let start = source
        .find("fn apply_runtime_hotkeys")
        .expect("runtime hotkey mapper");
    let end = source[start..]
        .find("fn drain_unused_runtime_ticks")
        .map(|offset| start + offset)
        .expect("end of runtime hotkey mapper");
    let mapper = &source[start..end];

    assert!(
        !mapper.contains("KeyCode::Escape") && !mapper.contains("KeyCode::Space"),
        "only configured Game Boy controls may drive the runtime hotkey mapper"
    );
}

#[test]
fn release_name_input_accepts_only_configured_game_boy_controls() {
    let source = bevy_shell_source();
    let start = source
        .find("fn apply_visible_name_input_keys")
        .expect("name input mapper");
    let end = source[start..]
        .find("fn apply_visible_name_input_smoke_char")
        .map(|offset| start + offset)
        .expect("end of name input mapper");
    let mapper = &source[start..end];

    assert!(!mapper.contains("KeyCode::Backspace"));
    assert!(!mapper.contains("KeyCode::ShiftLeft"));
}

#[test]
fn release_runtime_contains_no_partial_idle_frame_path() {
    let fast_path = format!("{}_{}_{}", "advance", "idle_frame", "fast");
    assert!(
        !bevy_shell_source().contains(&fast_path)
            && !include_str!("../../lib.rs").contains(&fast_path),
        "release runtime must advance every gameplay frame through the authoritative path"
    );
}

#[test]
fn release_shell_cannot_invoke_a_special_outside_script_execution() {
    let direct_special = format!("{}_{}_{}", "apply", "noop", "special");
    assert!(
        !bevy_shell_source().contains(&direct_special)
            && !include_str!("../../lib.rs").contains(&direct_special),
        "a source special must be reached by the script interpreter, never a shell action"
    );
}

#[test]
fn release_shell_has_no_host_happiness_service_actions() {
    let direct_service = format!("{}_{}_{}", "apply", "visible", "happiness_service");
    assert!(
        !bevy_shell_source().contains(&direct_service),
        "happiness must execute only through its exported script special, not a host action"
    );
}

#[test]
fn every_compiled_dialogue_resume_path_reaches_a_runtime_boundary_without_looping() {
    type Scripts = std::collections::BTreeMap<String, serde_json::Value>;
    type State = (String, usize);

    fn command_name(command: &serde_json::Value) -> &str {
        command
            .get("command")
            .and_then(serde_json::Value::as_str)
            .expect("compiled command name")
    }

    fn is_dialogue(command: &str) -> bool {
        matches!(
            command,
            "opentext"
                | "closetext"
                | "writetext"
                | "farwritetext"
                | "jumptext"
                | "jumptextfaceplayer"
                | "farjumptext"
                | "promptbutton"
                | "waitbutton"
                | "yesorno"
        )
    }

    fn is_modal_special(command: &serde_json::Value) -> bool {
        command_name(command) == "special"
            && command
                .get("args")
                .and_then(serde_json::Value::as_array)
                .and_then(|args| args.first())
                .and_then(serde_json::Value::as_str)
                .is_some_and(|routine| {
                    matches!(
                        routine,
                        "SetDayOfWeek"
                            | "NameRival"
                            | "MoveTutor"
                            | "BuenasPassword"
                            | "BuenaPrize"
                            | "UnownPrinter"
                            | "Menu_ChallengeExplanationCancel"
                            | "BattleTowerRoomMenu"
                            | "BattleTowerMobileError"
                            | "BattleTowerLeaderboard"
                    )
                })
    }

    fn is_runtime_boundary(command: &serde_json::Value) -> bool {
        let name = command_name(command);
        matches!(
            name,
            "jumptext"
                | "jumptextfaceplayer"
                | "farjumptext"
                | "promptbutton"
                | "waitbutton"
                | "yesorno"
        ) || matches!(
            name,
            "applymovement"
                | "earthquake"
                | "pause"
                | "showemote"
                | "startbattle"
                | "trainer"
                | "_2dmenu"
                | "verticalmenu"
                | "warp"
                | "warpfacing"
                | "newloadmap"
                | "reloadmap"
                | "reloadmappart"
                | "reloadmapafterbattle"
                | "refreshmap"
                | "reanchormap"
        ) || is_modal_special(command)
    }

    fn parent_label(label: &str) -> &str {
        label.rsplit_once('@').map_or(label, |(_, parent)| parent)
    }

    fn target_label(
        current: &str,
        raw: &str,
        scripts: &Scripts,
        globals: &Scripts,
    ) -> Option<String> {
        if scripts.contains_key(raw) || globals.contains_key(raw) {
            return Some(raw.to_string());
        }
        if raw.starts_with('.') {
            let scoped = format!("{raw}@{}", parent_label(current));
            if scripts.contains_key(&scoped) || globals.contains_key(&scoped) {
                return Some(scoped);
            }
        }
        None
    }

    fn target_arg(
        command: &serde_json::Value,
        current: &str,
        scripts: &Scripts,
        globals: &Scripts,
    ) -> Option<String> {
        command
            .get("args")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .rev()
            .find_map(|arg| target_label(current, arg, scripts, globals))
    }

    fn prove_reaches_boundary(
        state: State,
        scripts: &Scripts,
        globals: &Scripts,
        visiting: &mut Vec<State>,
        proven: &mut std::collections::BTreeSet<State>,
    ) -> std::result::Result<(), String> {
        if proven.contains(&state) {
            return Ok(());
        }
        if let Some(loop_start) = visiting.iter().position(|seen| seen == &state) {
            return Err(format!(
                "boundary-free compiled-script loop: {:?}",
                &visiting[loop_start..]
            ));
        }
        if visiting.len() >= 256 {
            return Err(format!(
                "dialogue auto-resume exceeds the runtime 256-command budget: {visiting:?}"
            ));
        }
        let body = scripts
            .get(&state.0)
            .or_else(|| globals.get(&state.0))
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| format!("missing compiled script body {}", state.0))?;
        let Some(command) = body.get(state.1) else {
            proven.insert(state);
            return Ok(());
        };
        let name = command_name(command);
        if is_runtime_boundary(command)
            || matches!(name, "end" | "endcallback" | "return" | "reloadandreturn")
        {
            proven.insert(state);
            return Ok(());
        }

        visiting.push(state.clone());
        let mut edges = vec![(state.0.clone(), state.1 + 1)];
        if matches!(name, "sjump" | "farsjump") {
            edges.clear();
        }
        if matches!(
            name,
            "sjump"
                | "farsjump"
                | "scall"
                | "farcall"
                | "farscall"
                | "ifequal"
                | "ifnotequal"
                | "iffalse"
                | "iftrue"
                | "ifless"
                | "ifgreater"
        ) && let Some(target) = target_arg(command, &state.0, scripts, globals)
        {
            edges.push((target, 0));
        }
        for edge in edges {
            prove_reaches_boundary(edge, scripts, globals, visiting, proven)?;
        }
        visiting.pop();
        proven.insert(state);
        Ok(())
    }

    let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let runtime = workspace_desktop_runtime(&AssetRoot::new(repo_root));
    let globals = runtime
        .data()
        .global_scripts
        .as_ref()
        .map(|module| module.scripts.clone())
        .unwrap_or_default();
    let mut audited_scripts = 0usize;
    let mut audited_boundaries = 0usize;
    let mut failures = Vec::new();

    for (module_name, scripts) in runtime
        .data()
        .maps
        .iter()
        .map(|(name, module)| (name.as_str(), &module.scripts))
        .chain(std::iter::once(("<global>", &globals)))
    {
        for (label, body) in scripts {
            let Some(commands) = body.as_array() else {
                continue;
            };
            let dialogue_indexes = commands
                .iter()
                .enumerate()
                .filter_map(|(index, command)| is_dialogue(command_name(command)).then_some(index))
                .collect::<Vec<_>>();
            if dialogue_indexes.is_empty() {
                continue;
            }
            audited_scripts += 1;
            let starts = std::iter::once(0)
                .chain(dialogue_indexes.iter().map(|index| index + 1))
                .collect::<std::collections::BTreeSet<_>>();
            audited_boundaries += starts.len();
            for start in starts {
                let mut visiting = Vec::new();
                let mut proven = std::collections::BTreeSet::new();
                if let Err(error) = prove_reaches_boundary(
                    (label.clone(), start),
                    scripts,
                    &globals,
                    &mut visiting,
                    &mut proven,
                ) {
                    failures.push(format!("{module_name}:{label}:{start}: {error}"));
                }
            }
        }
    }

    assert!(
        audited_scripts > 1_000,
        "dialogue audit covered only {audited_scripts} scripts"
    );
    assert!(
        audited_boundaries > 4_000,
        "dialogue audit covered only {audited_boundaries} resume points"
    );
    assert!(
        failures.is_empty(),
        "compiled dialogue contains runtime-blocking paths:\n{}",
        failures.join("\n")
    );
}

use super::*;
use crate::core::systems::script_text::{ScriptTextBody, ScriptTextBodyCommand};

#[test]
fn every_asm_writetext_continues_to_the_authored_input_boundary() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let runtime = workspace_desktop_runtime(&AssetRoot::new(repo_root));
    let globals = runtime
        .data()
        .global_scripts
        .as_ref()
        .map(|module| module.scripts.clone())
        .unwrap_or_default();
    let mut total = 0usize;
    let mut followed_by_waitbutton = 0usize;
    let mut followed_by_promptbutton = 0usize;
    let mut followed_by_yesorno = 0usize;
    let mut followed_by_other = 0usize;

    for scripts in runtime
        .data()
        .maps
        .values()
        .map(|module| &module.scripts)
        .chain(std::iter::once(&globals))
    {
        for body in scripts.values().filter_map(serde_json::Value::as_array) {
            for (index, command) in body.iter().enumerate() {
                let name = command
                    .get("command")
                    .and_then(serde_json::Value::as_str)
                    .expect("compiled command name");
                if !matches!(name, "writetext" | "farwritetext") {
                    continue;
                }
                total += 1;
                assert!(
                    !crate::compiled_script_boundary_stops_run(
                        name,
                        &Some(crate::RuntimeCompiledScriptBoundary::TextLabel(
                            "ASMText".to_string()
                        ))
                    ),
                    "{name} is a presentation command in ScriptEvents and must never suspend the interpreter"
                );
                match body
                    .get(index + 1)
                    .and_then(|next| next.get("command"))
                    .and_then(serde_json::Value::as_str)
                {
                    Some("waitbutton") => followed_by_waitbutton += 1,
                    Some("promptbutton") => followed_by_promptbutton += 1,
                    Some("yesorno") => followed_by_yesorno += 1,
                    _ => followed_by_other += 1,
                }
            }
        }
    }

    eprintln!(
        "asm_dialogue_census total={total} waitbutton={followed_by_waitbutton} promptbutton={followed_by_promptbutton} yesorno={followed_by_yesorno} other={followed_by_other}"
    );

    assert!(total > 1_900, "ASM dialogue census covered only {total} text commands");
    assert!(
        followed_by_waitbutton > 1_400,
        "ASM dialogue census covered only {followed_by_waitbutton} writetext -> waitbutton pairs"
    );
    assert!(
        followed_by_promptbutton > 270,
        "ASM dialogue census covered only {followed_by_promptbutton} writetext -> promptbutton pairs"
    );
    assert!(
        followed_by_yesorno > 110,
        "ASM dialogue census covered only {followed_by_yesorno} writetext -> yesorno pairs"
    );
    assert!(
        followed_by_other > 190,
        "ASM dialogue census covered only {followed_by_other} non-adjacent input boundaries"
    );
}

#[test]
fn every_compiled_applymovement_is_a_blocking_asm_boundary_before_later_dialogue() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let runtime = workspace_desktop_runtime(&AssetRoot::new(repo_root));
    let globals = runtime
        .data()
        .global_scripts
        .as_ref()
        .map(|module| module.scripts.clone())
        .unwrap_or_default();
    let mut total = 0usize;
    let mut followed_by_dialogue = 0usize;
    for scripts in runtime
        .data()
        .maps
        .values()
        .map(|module| &module.scripts)
        .chain(std::iter::once(&globals))
    {
        for commands in scripts.values().filter_map(serde_json::Value::as_array) {
            for (index, command) in commands.iter().enumerate() {
                if command.get("command").and_then(serde_json::Value::as_str)
                    != Some("applymovement")
                {
                    continue;
                }
                total += 1;
                assert!(crate::compiled_script_boundary_stops_run(
                    "applymovement",
                    &Some(crate::RuntimeCompiledScriptBoundary::ScriptMovement)
                ));
                if commands[index + 1..].iter().take(4).any(|later| {
                    matches!(
                        later.get("command").and_then(serde_json::Value::as_str),
                        Some("opentext" | "writetext" | "farwritetext")
                    )
                }) {
                    followed_by_dialogue += 1;
                }
            }
        }
    }
    eprintln!(
        "asm_movement_census total={total} followed_by_nearby_dialogue={followed_by_dialogue}"
    );
    assert!(total >= 480, "movement audit covered only {total} commands");
    assert!(
        followed_by_dialogue > 25,
        "movement/dialogue ordering audit covered only {followed_by_dialogue} transitions"
    );
}

#[test]
fn all_elm_starters_complete_the_full_asm_rival_battle_branch() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let elm_scripts = &runtime
        .data()
        .maps
        .get("ElmsLab")
        .expect("Elm's Lab module")
        .scripts;

    for (starter, elm_script, starter_event, rival_trainer, rival_species) in [
        (
            "CYNDAQUIL",
            "CyndaquilPokeBallScript",
            "EVENT_GOT_CYNDAQUIL_FROM_ELM",
            "RIVAL1_1_TOTODILE",
            "TOTODILE",
        ),
        (
            "TOTODILE",
            "TotodilePokeBallScript",
            "EVENT_GOT_TOTODILE_FROM_ELM",
            "RIVAL1_1_CHIKORITA",
            "CHIKORITA",
        ),
        (
            "CHIKORITA",
            "ChikoritaPokeBallScript",
            "EVENT_GOT_CHIKORITA_FROM_ELM",
            "RIVAL1_1_CYNDAQUIL",
            "CYNDAQUIL",
        ),
    ] {
        let commands = elm_scripts
            .get(elm_script)
            .and_then(serde_json::Value::as_array)
            .unwrap_or_else(|| panic!("missing Elm starter script {elm_script}"));
        let has_command = |name: &str, arg: &str| {
            commands.iter().any(|command| {
                command.get("command").and_then(serde_json::Value::as_str) == Some(name)
                    && command
                        .get("args")
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(|args| args.iter().any(|value| value.as_str() == Some(arg)))
            })
        };
        assert!(has_command("pokepic", starter), "{elm_script} shows the wrong starter");
        assert!(has_command("setevent", starter_event), "{elm_script} sets the wrong event");
        assert!(has_command("givepoke", starter), "{elm_script} grants the wrong starter");

        let mut shell = RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime.clone(),
            1,
            "CherrygroveCity",
            39,
            7,
        )
        .expect("start Cherrygrove rival branch");
        shell
            .set_script_flag_for_smoke(starter_event)
            .expect("set selected Elm starter event");
        shell
            .add_party_pokemon(
                starter,
                100,
                None,
                None,
                "PLAYER",
                1,
                Dv::from_non_hp(10, 10, 10, 10),
            )
            .expect("add selected Elm starter");
        if shell.script_events_snapshot().script_ended.is_some() {
            shell
                .take_script_end_state()
                .expect("clear map initialization script end before rival scene");
        }
        let run = shell
            .run_compiled_script_until_boundary(
                RuntimeCompiledScriptCursor {
                    origin_map_name: "CherrygroveCity".to_string(),
                    source_script: "CherrygroveRivalSceneNorth".to_string(),
                    command_index: 12,
                },
                24,
                ScriptRuntimeInputs::default(),
                ScriptPhoneInputs::default(),
            )
            .expect("run starter-dependent rival branch");
        assert!(
            matches!(run.boundary, Some(crate::RuntimeCompiledScriptBoundary::ActiveBattle(_))),
            "{starter} branch did not reach the rival battle: {run:?}"
        );
        let battle_step = run
            .steps
            .iter()
            .find(|step| step.command == "startbattle")
            .unwrap_or_else(|| panic!("{starter} branch did not create a trainer battle"));
        let RuntimeMutationResult::ScriptedTrainerBattleStarted(
            crate::TrainerBattleStartStatus::Started(battle),
        ) = &battle_step.mutation.result
        else {
            panic!("{starter} branch did not start the expected rival battle");
        };
        assert_eq!(
            battle.trainer_id, rival_trainer,
            "wrong rival trainer for {starter}"
        );
        assert_eq!(
            battle
                .enemy_party
                .first()
                .map(|pokemon| pokemon.species.id.as_str()),
            Some(rival_species),
            "wrong rival counter-starter for {starter}"
        );

        // Exercise the actual battle engine to a win, including reward claim,
        // trainer-party advancement, scripted completion, and the post-battle
        // Rival dialogue boundary. The level advantage keeps this a branch
        // parity test rather than an RNG-sensitive combat-balance test.
        shell
            .snapshot()
            .unwrap_or_else(|error| panic!("{starter} rival battle snapshot failed: {error:#}"));
        let mut turns = 0usize;
        let trainer_defeated = loop {
            turns += 1;
            assert!(turns <= 64, "{starter} rival battle exceeded 64 turns");
            let player_action = BattleAction::Move { slot: 0 };
            let enemy_action = BattleAction::Move { slot: 0 };
            let rng_seed_after = shell
                .preview_active_battle_turn_rng_seed_after(
                    player_action.clone(),
                    enemy_action.clone(),
                )
                .expect("preview rival turn RNG boundary");
            let turn = shell
                .resolve_active_battle_turn(player_action, enemy_action, rng_seed_after)
                .expect("resolve complete rival battle turn");
            assert!(
                turn.outcome.state.player.hp > 0,
                "level-100 {starter} unexpectedly fainted in branch parity battle"
            );
            if turn.outcome.state.enemy.hp > 0 {
                continue;
            }
            shell
                .claim_active_trainer_battle_rewards()
                .expect("claim rival battle rewards");
            let advance = shell
                .advance_active_trainer_battle()
                .expect("advance defeated rival party");
            if advance.trainer_defeated {
                break true;
            }
        };
        assert!(trainer_defeated);
        let completion = shell
            .complete_scripted_trainer_battle(
                &battle_step.origin_map_name,
                &battle_step.source_script,
                battle_step.command_index,
                true,
                true,
            )
            .expect("complete full rival battle");
        assert!(completion.continued_after_battle);
        assert!(!shell.has_active_battle());
        let reload = shell
            .run_compiled_script_until_boundary(
                battle_step
                    .next_cursor
                    .clone()
                    .expect("rival post-battle cursor"),
                32,
                ScriptRuntimeInputs::default(),
                ScriptPhoneInputs::default(),
            )
            .expect("resume rival script through its blocking map reload");
        assert!(matches!(
            reload.boundary,
            Some(crate::RuntimeCompiledScriptBoundary::PendingMapLoad(ref load))
                if load.command == "reloadmap"
        ));
        shell
            .take_pending_script_request(crate::RuntimePendingScriptRequestKind::MapLoad)
            .expect("complete rival map reload before post-battle branch");
        let post_battle = shell
            .run_compiled_script_until_boundary(
                reload.next_cursor.expect("rival map-reload continuation"),
                32,
                ScriptRuntimeInputs::default(),
                ScriptPhoneInputs::default(),
            )
            .expect("resume rival post-battle branch after map reload");
        assert!(
            matches!(post_battle.boundary, Some(crate::RuntimeCompiledScriptBoundary::TextWait(_))),
            "{starter} rival post-battle script did not reach its authored win text: {post_battle:?}"
        );
        // ASM defines WIN=0, copies wBattleResult into wScriptVar at
        // startbattle, and branches at `iftrue .AfterVictorious` only for a
        // nonzero result. The local branch names are from the Rival's point of
        // view: a player win therefore falls through `.AfterYourDefeat` and
        // reaches the text label named `YouLost`.
        assert_eq!(shell.session().state().battle_result & 0x3f, 0);
        assert_eq!(
            shell.script_events_snapshot().pending_text_label.as_deref(),
            Some("CherrygroveRivalText_YouLost")
        );
    }
}

#[test]
fn cherrygrove_can_lose_battle_resumes_the_opposite_asm_branch_without_whiteout() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let mut shell = RuntimeGameShell::new_game_at_runtime_tile(
        asset_root,
        runtime,
        1,
        "CherrygroveCity",
        39,
        7,
    )
    .expect("start Cherrygrove can-lose branch");
    shell
        .add_party_pokemon(
            "CYNDAQUIL",
            5,
            None,
            None,
            "PLAYER",
            1,
            Dv::from_non_hp(10, 10, 10, 10),
        )
        .expect("add starter");
    if shell.script_events_snapshot().script_ended.is_some() {
        shell
            .take_script_end_state()
            .expect("clear map initialization script end");
    }
    let run = shell
        .run_compiled_script_until_boundary(
            RuntimeCompiledScriptCursor {
                origin_map_name: "CherrygroveCity".to_string(),
                source_script: "CherrygroveRivalSceneNorth".to_string(),
                command_index: 12,
            },
            24,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("start can-lose rival battle");
    let battle_step = run
        .steps
        .iter()
        .find(|step| step.command == "startbattle")
        .expect("startbattle step");
    let completion = shell
        .complete_scripted_trainer_battle(
            &battle_step.origin_map_name,
            &battle_step.source_script,
            battle_step.command_index,
            false,
            true,
        )
        .expect("complete authored can-lose battle as a loss");
    assert!(completion.continued_after_battle);
    assert_eq!(shell.session().state().battle_result & 0x3f, 1);
    assert!(!shell.has_active_battle());
    assert_eq!(shell.snapshot().expect("loss snapshot").overworld.map_name, "CherrygroveCity");

    let reload = shell
        .run_compiled_script_until_boundary(
            battle_step.next_cursor.clone().expect("postbattle cursor"),
            16,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("run loss to map reload");
    assert!(matches!(
        reload.boundary,
        Some(crate::RuntimeCompiledScriptBoundary::PendingMapLoad(ref load))
            if load.command == "reloadmap"
    ));
    shell
        .take_pending_script_request(crate::RuntimePendingScriptRequestKind::MapLoad)
        .expect("complete loss map reload");
    let post_battle = shell
        .run_compiled_script_until_boundary(
            reload.next_cursor.expect("loss reload continuation"),
            24,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("resume loss branch");
    assert!(matches!(
        post_battle.boundary,
        Some(crate::RuntimeCompiledScriptBoundary::TextWait(_))
    ));
    // WIN=0 and LOSE=1. Therefore `iftrue .AfterVictorious` is the Rival's
    // victory, i.e. the player's authored loss branch.
    assert_eq!(
        shell.script_events_snapshot().pending_text_label.as_deref(),
        Some("CherrygroveRivalText_YouWon")
    );
}

#[test]
fn azalea_rival_full_multi_pokemon_battle_advances_every_party_slot_then_resumes() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let mut shell = RuntimeGameShell::new_game_at_runtime_tile(
        asset_root,
        runtime,
        1,
        "AzaleaTown",
        5,
        10,
    )
    .expect("start Azalea Rival branch");
    shell
        .set_script_flag_for_smoke("EVENT_GOT_CYNDAQUIL_FROM_ELM")
        .expect("set selected starter event");
    shell
        .add_party_pokemon(
            "TYPHLOSION",
            100,
            None,
            None,
            "PLAYER",
            1,
            Dv::from_non_hp(15, 15, 15, 15),
        )
        .expect("add battle lead");
    if shell.script_events_snapshot().script_ended.is_some() {
        shell
            .take_script_end_state()
            .expect("clear map initialization script end");
    }
    // Start at the starter-dependent branch checks after the introductory
    // text. With Cyndaquil selected, ASM loads RIVAL1_2_TOTODILE.
    let run = shell
        .run_compiled_script_until_boundary(
            RuntimeCompiledScriptCursor {
                origin_map_name: "AzaleaTown".to_string(),
                source_script: "AzaleaTownRivalBattleScript".to_string(),
                command_index: 6,
            },
            32,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("run Azalea starter branch to battle");
    let battle_step = run
        .steps
        .iter()
        .find(|step| step.command == "startbattle")
        .expect("Azalea startbattle step");
    let RuntimeMutationResult::ScriptedTrainerBattleStarted(
        crate::TrainerBattleStartStatus::Started(battle),
    ) = &battle_step.mutation.result
    else {
        panic!("Azalea Rival did not start the expected trainer battle");
    };
    assert_eq!(battle.trainer_id, "RIVAL1_2_TOTODILE");
    assert_eq!(
        battle
            .enemy_party
            .iter()
            .map(|pokemon| pokemon.species.id.as_str())
            .collect::<Vec<_>>(),
        vec!["GASTLY", "ZUBAT", "CROCONAW"],
        "ASM counter-starter branch must materialize the entire three-Pokemon party"
    );

    let mut defeated_species = Vec::new();
    let mut turns = 0usize;
    loop {
        turns += 1;
        assert!(turns <= 128, "Azalea Rival battle exceeded 128 turns");
        let before = shell.snapshot().expect("Azalea battle snapshot");
        let enemy_species = before
            .battle
            .as_ref()
            .expect("active Azalea battle")
            .enemy_pokemon
            .species
            .id
            .clone();
        let player_action = BattleAction::Move { slot: 0 };
        let enemy_action = BattleAction::Move { slot: 0 };
        let rng_seed_after = shell
            .preview_active_battle_turn_rng_seed_after(
                player_action.clone(),
                enemy_action.clone(),
            )
            .expect("preview Azalea turn RNG");
        let turn = shell
            .resolve_active_battle_turn(player_action, enemy_action, rng_seed_after)
            .expect("resolve Azalea battle turn");
        assert!(turn.outcome.state.player.hp > 0);
        if turn.outcome.state.enemy.hp > 0 {
            continue;
        }
        defeated_species.push(enemy_species);
        shell
            .claim_active_trainer_battle_rewards()
            .expect("claim per-enemy Azalea rewards");
        if shell
            .advance_active_trainer_battle()
            .expect("advance exact Azalea trainer party")
            .trainer_defeated
        {
            break;
        }
    }
    assert_eq!(defeated_species, vec!["GASTLY", "ZUBAT", "CROCONAW"]);
    let completion = shell
        .complete_scripted_trainer_battle(
            &battle_step.origin_map_name,
            &battle_step.source_script,
            battle_step.command_index,
            true,
            false,
        )
        .expect("complete Azalea multi-Pokemon battle");
    assert!(completion.continued_after_battle);
    let reload = shell
        .run_compiled_script_until_boundary(
            battle_step.next_cursor.clone().expect("Azalea postbattle cursor"),
            16,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("run Azalea postbattle to reloadmapafterbattle");
    assert!(matches!(
        reload.boundary,
        Some(crate::RuntimeCompiledScriptBoundary::PendingMapLoad(ref load))
            if load.command == "reloadmapafterbattle"
    ));
    shell
        .take_pending_script_request(crate::RuntimePendingScriptRequestKind::MapLoad)
        .expect("complete Azalea battle reload");
    let post_battle = shell
        .run_compiled_script_until_boundary(
            reload.next_cursor.expect("Azalea reload continuation"),
            32,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("resume Azalea script after full trainer party");
    assert!(matches!(
        post_battle.boundary,
        Some(crate::RuntimeCompiledScriptBoundary::TextWait(_))
    ));
    assert_eq!(
        shell.script_events_snapshot().pending_text_label.as_deref(),
        Some("AzaleaTownRivalAfterText")
    );
}

#[test]
fn real_pack_trainer_battle_starts_and_resolves_from_route_30() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root.clone());
    let runtime = CrystalRuntime::load_from_compiled_pack(
        &asset_root,
        "content-packs/core-modular.crystalpack",
    )
    .expect("load core pack for Red battle");
    let party = [VisibleShellSmokePokemon {
        species_id: "TYPHLOSION".to_string(),
        level: 100,
        held_item_id: None,
    }];
    let smoke = smoke_visible_shell_trainer_battle(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier: 26,
            map_name: "Route30".to_string(),
            tile_x: 8,
            tile_y: 8,
        },
        BevyShellConfig {
            smoke_player_name: Some("TEST".to_string()),
            ..Default::default()
        },
        VisibleShellBattleSmokeRef {
            map_name: "Route30".to_string(),
            source_script: "TrainerYoungsterJoey".to_string(),
            // ASM Route30 TrainerYoungsterJoey begins with the trainer
            // command that declares YOUNGSTER JOEY1.
            command_index: 0,
        },
        &party,
    )
    .expect("trainer battle should resolve through the public Bevy shell bridge");
    assert_eq!(smoke.trainer_class, "YOUNGSTER");
    assert_eq!(smoke.trainer_id, "JOEY1");
    assert!(smoke.trainer_defeated);
    assert!(!smoke.active_battle_after);
    assert!(smoke.turns > 0);
}

fn oak_intro_prompt_arrow_dark_pixels(image: &Image) -> usize {
    let width = image.texture_descriptor.size.width as usize;
    (OAK_INTRO_PROMPT_ARROW_Y..OAK_INTRO_PROMPT_ARROW_Y + SOURCE_TILE_SIZE)
        .flat_map(|y| {
            (OAK_INTRO_PROMPT_ARROW_X..OAK_INTRO_PROMPT_ARROW_X + SOURCE_TILE_SIZE)
                .map(move |x| (x, y))
        })
        .filter(|(x, y)| {
            let offset = (y * width + x) * 4;
            offset + 3 < image.data.len()
                && image.data[offset] < 32
                && image.data[offset + 1] < 32
                && image.data[offset + 2] < 32
                && image.data[offset + 3] == 255
        })
        .count()
}

#[test]
fn boot_textbox_wraps_and_normalizes_like_typescript_bitmap_font() {
    assert_eq!(
        normalize_boot_text("Welcome to the\nworld of #MON!"),
        "Welcome to the\nworld of POKéMON!"
    );
    assert_eq!(
        wrap_boot_text_for_box("Welcome to the world of #MON!", 18, 4),
        vec![
            "Welcome to the".to_string(),
            "world of POKéMON!".to_string()
        ]
    );
    assert_eq!(
        wrap_boot_text_for_box("one two three four five six seven", 7, 2),
        vec!["one two".to_string(), "three".to_string()],
        "boot textboxes must clamp to the inner textbox line count instead of overflowing"
    );
}

#[test]
fn special_boundary_entries_do_not_expose_rust_effect_details() {
    let generic_boundary = special_boundary_display(&SpecialRoutineEffect::Noop);
    assert_eq!(generic_boundary.label, "SpecialRoutine");
    assert!(
        generic_boundary.details.is_empty(),
        "generic special boundaries must not expose Rust enum dumps: {generic_boundary:?}"
    );

    let boundary = SpecialBoundaryDisplay {
        label: "FadeOutMusic".to_string(),
        details: vec![
            "effect=FadeOutMusic { audio_id: MUSIC_NONE }".to_string(),
            "audio=MUSIC_NONE".to_string(),
        ],
    };

    let entries = visible_special_boundary_display_entries(&boundary);
    assert_eq!(entries, vec![">FadeOutMusic".to_string()]);

    let mut context = Vec::new();
    append_special_boundary_display_context(&boundary, &mut context);
    assert_eq!(context, vec!["special_boundary=FadeOutMusic".to_string()]);
}

#[test]
fn runtime_tick_timer_preserves_all_vblanks_while_bounding_input_catch_up() {
    let mut timer = RuntimeTickTimer::new(1.0 / 60.0);
    timer.tick(1.0 / 30.0);
    assert_eq!(timer.take_vblanks(), 2);
    assert_eq!(timer.take_ticks(), MAX_RUNTIME_CATCH_UP_TICKS);
    assert!(!timer.has_tick());
}

#[test]
fn runtime_tick_timer_long_stall_does_not_drop_elapsed_vblanks() {
    let mut timer = RuntimeTickTimer::new(1.0 / 60.0);
    timer.tick(2.0);
    assert_eq!(timer.take_vblanks(), 120);
    assert_eq!(timer.take_ticks(), MAX_RUNTIME_CATCH_UP_TICKS);
}

#[test]
fn visible_sequence_clock_recovers_normal_low_refresh_cadence_without_unbounded_skip() {
    let mut clock = VisibleSequenceTickClock::realtime();
    assert_eq!(
        clock.consume_frames(3.0 * GAME_TICK_SECONDS),
        3,
        "a 20 Hz host must preserve the 60 Hz title/intro wall-clock cadence"
    );
    assert_eq!(
        clock.consume_frames(2.0),
        MAX_VISIBLE_SEQUENCE_CATCH_UP_FRAMES,
        "a stalled host frame must not skip an entire visible sequence"
    );
}

#[test]
fn transient_audio_queue_keeps_only_the_latest_sfx_or_cry() {
    let command = |audio_id: &str, kind: ModpackAudioKind| BevyAudioCommand {
        audio_id: audio_id.to_string(),
        kind,
        mode: ModpackAudioPlaybackMode::RawPcm,
        looped: false,
    };
    let queue = coalesce_pending_transient_audio(vec![
        command("MUSIC_OLD", ModpackAudioKind::Music),
        command("SFX_OLD", ModpackAudioKind::SoundEffect),
        command("CRY_OLD", ModpackAudioKind::Cry),
        command("MUSIC_ROUTE_29", ModpackAudioKind::Music),
        command("SFX_NEW", ModpackAudioKind::SoundEffect),
    ]);
    assert_eq!(
        queue
            .iter()
            .map(|command| command.audio_id.as_str())
            .collect::<Vec<_>>(),
        vec!["MUSIC_ROUTE_29", "SFX_NEW"]
    );
}

#[test]
fn pending_music_guard_rejects_duplicate_track_but_not_transition() {
    let command = |audio_id: &str, kind: ModpackAudioKind| BevyAudioCommand {
        audio_id: audio_id.to_string(),
        kind,
        mode: ModpackAudioPlaybackMode::RawPcm,
        looped: true,
    };
    let pending = vec![command("MUSIC_NEW_BARK_TOWN", ModpackAudioKind::Music)];
    assert!(pending_music_command_is(&pending, "MUSIC_NEW_BARK_TOWN"));
    assert!(!pending_music_command_is(&pending, "MUSIC_ROUTE_29"));
}

#[test]
fn bitmap_font_background_is_transparent_and_dark_glyph_is_opaque() {
    assert!(!bitmap_font_glyph_pixel(255, 255, 255, 255));
    assert!(!bitmap_font_glyph_pixel(255, 255, 255, 0));
    assert!(bitmap_font_glyph_pixel(0, 0, 0, 255));
    assert!(bitmap_font_glyph_pixel(170, 170, 170, 255));
}

#[test]
fn yes_no_prompt_owns_all_yes_no_text_variants() {
    assert_eq!(
        (
            FIELD_YES_NO_LEFT_TILE,
            FIELD_YES_NO_TOP_TILE,
            FIELD_YES_NO_WIDTH_TILES,
            FIELD_YES_NO_HEIGHT_TILES,
        ),
        (14.0, 7.0, 6.0, 4.0),
        "ASM/TypeScript YesNoBox uses the 6x4 outer window at tile (14, 7)"
    );
    assert!(is_visible_yes_no_prompt_entry("YES"));
    assert!(is_visible_yes_no_prompt_entry(">NO"));
    assert!(is_visible_yes_no_prompt_entry("YES / NO"));
    assert!(!is_visible_yes_no_prompt_entry("Would you like to save?"));
}

#[test]
fn phone_number_prompt_uses_the_standard_yes_no_window_cursor() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    {
        let state = runtime_shell.shell.session_mut().state_mut();
        state.script_runtime.text_window_open = false;
        state.script_runtime.pending_text_label = None;
    }
    runtime_shell.pending_phone_prompt = Some(PendingPhonePrompt {
        source_script: "PhonePromptFixture".to_string(),
        command_index: 0,
        contact_id: "PHONE_ELM".to_string(),
    });
    runtime_shell.yes_no_cursor = Some(MenuCursor {
        surface_id: "ui:phone-number".to_string(),
        option_index: 1,
    });
    mark_runtime_snapshot_dirty(&mut runtime_shell);
    let snapshot = runtime_shell
        .shell
        .snapshot()
        .expect("phone prompt snapshot");
    assert!(scene_dialog_yes_no_active(&snapshot, &runtime_shell));
    assert_eq!(
        scene_dialog_yes_no_cursor_index(&snapshot, &runtime_shell),
        1
    );
}

#[test]
fn visible_save_policy_rejects_non_atomic_script_resume_boundaries() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    let baseline = runtime_shell
        .shell
        .snapshot()
        .expect("save-boundary baseline snapshot");

    let mut map_load = baseline.clone();
    map_load.script_events.pending_map_load = Some(crystal_core::state::ScriptMapLoadRequest {
        command: "reloadmap".to_string(),
        map_setup: None,
        source_script: "SaveBoundaryFixture".to_string(),
        command_index: 1,
    });
    assert!(
        visible_quick_save_blockers(&runtime_shell, &map_load, false, false)
            .contains(&"auto_script"),
        "a map reload must finish before the save can be committed"
    );

    let mut map_refresh = baseline.clone();
    map_refresh.script_events.pending_map_refresh =
        Some(crystal_core::state::ScriptMapRefreshRequest {
            command: "refreshmap".to_string(),
            map_setup: None,
            source_script: "SaveBoundaryFixture".to_string(),
            command_index: 2,
        });
    assert!(
        visible_quick_save_blockers(&runtime_shell, &map_refresh, false, false)
            .contains(&"auto_script"),
        "a map refresh must finish before the save can be committed"
    );

    runtime_shell.active_script_cursor = Some(ActiveScriptCursor {
        origin_map_name: "NewBarkTown".to_string(),
        source_script: "SaveBoundaryFixture".to_string(),
        next_command_index: 3,
    });
    assert!(
        visible_quick_save_blockers(&runtime_shell, &baseline, false, false).contains(&"script"),
        "dialogue, calls, movement, and post-battle continuations all retain an active script cursor"
    );
    runtime_shell.active_script_cursor = None;

    runtime_shell.pending_phone_prompt = Some(PendingPhonePrompt {
        source_script: "SaveBoundaryFixture".to_string(),
        command_index: 4,
        contact_id: "PHONE_ELM".to_string(),
    });
    assert!(
        visible_quick_save_blockers(&runtime_shell, &baseline, false, false)
            .contains(&"phone_prompt")
    );
    runtime_shell.pending_phone_prompt = None;

    runtime_shell.pending_day_of_week = Some(PendingDayOfWeekPrompt {
        origin_map_name: "PlayersHouse1F".to_string(),
        source_script: "SaveBoundaryFixture".to_string(),
        command_index: 5,
        selected_day: 0,
        confirming: false,
        yes_no_index: 0,
    });
    assert!(
        visible_quick_save_blockers(&runtime_shell, &baseline, false, false)
            .contains(&"day_of_week")
    );
}

#[test]
#[ignore = "performance probe; run explicitly with --ignored --nocapture"]
fn runtime_snapshot_performance_benchmark() {
    use std::hint::black_box;
    use std::time::Instant;

    const SAMPLES: usize = 120;
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let mut shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name: "Route36".to_string(),
            tile_x: 2,
            tile_y: 2,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize benchmark shell");
    let benchmark_text_label = shell
        .shell
        .runtime()
        .script_text_body_keys()
        .into_iter()
        .filter(|key| key.map_name == "Route36")
        .map(|key| key.body_key)
        .find(|key| !key.starts_with('.') && !key.contains('@'))
        .expect("benchmark pack contains at least one valid text body");

    let mut timed = |label: &str, step: &mut dyn FnMut() -> Result<()>| {
        let start = Instant::now();
        for _ in 0..SAMPLES {
            step().expect("benchmark step");
        }
        let elapsed = start.elapsed();
        eprintln!(
            "runtime_perf {label} samples={SAMPLES} total_us={} per_sample_us={:.2}",
            elapsed.as_micros(),
            elapsed.as_secs_f64() * 1_000_000.0 / SAMPLES as f64
        );
    };

    timed("idle_cached", &mut || {
        black_box(cached_runtime_snapshot(&mut shell)?);
        Ok(())
    });
    timed("idle_full_snapshot", &mut || {
        black_box(shell.shell.snapshot()?);
        Ok(())
    });
    timed("overworld_movement", &mut || {
        shell.shell.tick([GameButton::Right])?;
        mark_runtime_snapshot_dirty(&mut shell);
        black_box(cached_runtime_snapshot(&mut shell)?);
        Ok(())
    });
    timed("dialog_dirty", &mut || {
        let state = shell.shell.session_mut().state_mut();
        state.script_runtime.text_window_open = true;
        state.script_runtime.pending_text_label = Some(benchmark_text_label.clone());
        mark_runtime_snapshot_dirty(&mut shell);
        black_box(cached_runtime_snapshot(&mut shell)?);
        Ok(())
    });
    {
        let state = shell.shell.session_mut().state_mut();
        state.script_runtime.text_window_open = false;
        state.script_runtime.pending_text_label = None;
    }
    mark_runtime_snapshot_dirty(&mut shell);

    shell
        .shell
        .add_party_pokemon(
            "CYNDAQUIL",
            10,
            None,
            None,
            "BEVY_PERF_BATTLE",
            1,
            Dv::from_non_hp(10, 10, 10, 10),
        )
        .expect("add benchmark party Pokemon");
    shell
        .shell
        .start_scripted_wild_battle("Route36", "WateredWeirdTreeScript", 12)
        .expect("start benchmark battle");
    mark_runtime_snapshot_dirty(&mut shell);
    timed("battle_cached", &mut || {
        black_box(cached_runtime_snapshot(&mut shell)?);
        Ok(())
    });
}

#[test]
#[ignore = "interactive Bevy schedule probe; run explicitly with --ignored --nocapture"]
fn interactive_bevy_schedule_performance_benchmark() {
    use std::time::{Duration, Instant};

    const SAMPLES: usize = 120;
    const FRAME_BUDGET: Duration = Duration::from_nanos(16_742_706);

    #[derive(Resource, Default)]
    struct ScheduleProfileProbe {
        frame_started: Option<Instant>,
        input_finished: Option<Instant>,
        render_started: Option<Instant>,
        render_finished: Option<Instant>,
        phases: Vec<[Duration; 4]>,
    }

    fn profile_frame_start(mut probe: ResMut<ScheduleProfileProbe>) {
        probe.frame_started = Some(Instant::now());
    }

    fn profile_input_finished(mut probe: ResMut<ScheduleProfileProbe>) {
        probe.input_finished = Some(Instant::now());
    }

    fn profile_render_started(mut probe: ResMut<ScheduleProfileProbe>) {
        probe.render_started = Some(Instant::now());
    }

    fn profile_render_finished(mut probe: ResMut<ScheduleProfileProbe>) {
        probe.render_finished = Some(Instant::now());
    }

    fn profile_frame_finished(mut probe: ResMut<ScheduleProfileProbe>) {
        let (Some(frame), Some(input), Some(render_start), Some(render_end)) = (
            probe.frame_started,
            probe.input_finished,
            probe.render_started,
            probe.render_finished,
        ) else {
            return;
        };
        let finished = Instant::now();
        probe.phases.push([
            input.duration_since(frame),
            render_start.duration_since(input),
            render_end.duration_since(render_start),
            finished.duration_since(render_end),
        ]);
    }

    fn measure(app: &mut App, samples: usize) -> Vec<Duration> {
        (0..samples)
            .map(|_| {
                let started = Instant::now();
                app.update();
                started.elapsed()
            })
            .collect()
    }

    fn report(label: &str, mut samples: Vec<Duration>, phases: Vec<[Duration; 4]>) {
        let (slowest_frame, slowest_duration) = samples
            .iter()
            .copied()
            .enumerate()
            .max_by_key(|(_, duration)| *duration)
            .expect("profile contains samples");
        samples.sort_unstable();
        let percentile = |numerator: usize, denominator: usize| {
            let index = ((samples.len() - 1) * numerator) / denominator;
            samples[index].as_secs_f64() * 1_000_000.0
        };
        let total = samples.iter().copied().sum::<Duration>();
        let average_us = total.as_secs_f64() * 1_000_000.0 / samples.len() as f64;
        let missed = samples
            .iter()
            .filter(|duration| **duration > FRAME_BUDGET)
            .count();
        eprintln!(
            "interactive_bevy_profile phase={label} frames={} avg_us={average_us:.2} p50_us={:.2} p95_us={:.2} p99_us={:.2} max_us={:.2} max_frame={slowest_frame} max_frame_us={:.2} effective_fps={:.2} missed_59_7275hz={missed}",
            samples.len(),
            percentile(50, 100),
            percentile(95, 100),
            percentile(99, 100),
            samples.last().unwrap().as_secs_f64() * 1_000_000.0,
            slowest_duration.as_secs_f64() * 1_000_000.0,
            samples.len() as f64 / total.as_secs_f64(),
        );
        let phase_names = ["input", "pre_render", "render_playfield", "post_render_ui"];
        for (index, phase_name) in phase_names.into_iter().enumerate() {
            let mut values = phases.iter().map(|sample| sample[index]).collect::<Vec<_>>();
            values.sort_unstable();
            let total = values.iter().copied().sum::<Duration>();
            eprintln!(
                "interactive_bevy_profile_detail phase={label} system={phase_name} avg_us={:.2} p95_us={:.2} max_us={:.2}",
                total.as_secs_f64() * 1_000_000.0 / values.len() as f64,
                values[(values.len() - 1) * 95 / 100].as_secs_f64() * 1_000_000.0,
                values.last().unwrap().as_secs_f64() * 1_000_000.0,
            );
        }
    }

    fn take_phases(app: &mut App) -> Vec<[Duration; 4]> {
        std::mem::take(&mut app.world_mut().resource_mut::<ScheduleProfileProbe>().phases)
    }
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier: 14,
            map_name: "NewBarkTown".to_string(),
            tile_x: 13,
            tile_y: 6,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize interactive benchmark shell");
    runtime_shell.shell.set_runtime_journal_enabled(false);
    let mut app = integrated_shell_test_app(runtime_shell);
    app.init_resource::<ScheduleProfileProbe>()
        .add_systems(First, profile_frame_start)
        .add_systems(
            Update,
            profile_input_finished.after(drain_unused_runtime_ticks),
        )
        .add_systems(
            Update,
            profile_render_started
                .after(play_pending_audio)
                .before(render_playfield),
        )
        .add_systems(
            Update,
            profile_render_finished
                .after(render_playfield)
                .before(refresh_status_text),
        )
        .add_systems(Update, profile_frame_finished.after(refresh_shell_panels));
    app.update();
    take_phases(&mut app);

    let idle = measure(&mut app, SAMPLES);
    report("idle", idle, take_phases(&mut app));

    let mut keys = ButtonInput::<KeyCode>::default();
    keys.press(KeyCode::ArrowRight);
    app.insert_resource(keys);
    let held_right = measure(&mut app, SAMPLES);
    report("held_right", held_right, take_phases(&mut app));

    let mut traversal = Vec::with_capacity(SAMPLES);
    let directions = [
        KeyCode::ArrowDown,
        KeyCode::ArrowLeft,
        KeyCode::ArrowUp,
        KeyCode::ArrowRight,
    ];
    for frame in 0..SAMPLES {
        let mut keys = ButtonInput::<KeyCode>::default();
        if frame % 2 == 0 {
            keys.press(directions[(frame / 30) % directions.len()]);
        }
        app.insert_resource(keys);
        let elapsed = measure(&mut app, 1).pop().expect("one traversal sample");
        traversal.push(elapsed);
    }
    report("walk_and_turn", traversal, take_phases(&mut app));
}

#[test]
fn screen_fade_reaches_black_in_exactly_eight_gameboy_frames() {
    let mut fade = VisibleScreenFade::new(ScriptFadeColor::Black, ScriptFadeDirection::Out, 8);
    for _ in 0..8 {
        fade.advance(GAME_TICK_SECONDS);
    }
    assert_eq!(fade.elapsed_frames, 8);
    assert_eq!(fade.alpha, 255);
}

#[test]
fn intro_renderer_preserves_every_semantic_frame() {
    let mut intro = VisibleIntroScreen::new();
    intro.jumptable_index = 13;
    intro.scene_frame_counter = 7;
    intro.scene_timer = 7;
    intro.scroll_x = 11;
    intro.scroll_y = 15;
    intro.global_anim_x_offset = 13;
    let render = intro_renderer::exact_presentation_state(&intro);
    assert_eq!(render.jumptable_index, intro.jumptable_index);
    assert_eq!(render.scene_frame_counter, 7);
    assert_eq!(render.scene_timer, 7);
    assert_eq!(render.scroll_x, 11);
    assert_eq!(render.scroll_y, 15);
    assert_eq!(render.global_anim_x_offset, 13);
    assert_eq!(
        intro.scene_frame_counter, 7,
        "simulation state is untouched"
    );
}

#[test]
fn title_art_cache_tracks_native_eight_frame_animation_cadence() {
    let mut title = TitleMenu {
        spawn_identifier: 0,
        save_path: None,
        cursor: MenuCursor {
            surface_id: "title".to_string(),
            option_index: 0,
        },
        phase: VisibleTitlePhase::PressStart,
        frame: 15,
        main_menu_frame: 0,
        scx: 0,
        title_timer: 0,
        clock_reset_trigger: false,
    };
    assert_eq!(title_screen_art_key(&title).frame, 8);
    title.frame = 16;
    assert_eq!(title_screen_art_key(&title).frame, 16);
}

#[test]
fn completed_fade_out_releases_after_one_terminal_frame() {
    let mut fade = VisibleScreenFade::new(ScriptFadeColor::Black, ScriptFadeDirection::Out, 1);
    fade.advance(GAME_TICK_SECONDS);
    assert!(
        !completed_screen_fade_should_clear(&fade),
        "the terminal black palette must be rendered once"
    );
    fade.terminal_frame_presented = true;
    assert!(
        completed_screen_fade_should_clear(&fade),
        "a completed FadeOut must not black out later field frames"
    );
}

#[test]
fn field_dialogue_reveal_uses_the_selected_text_speed() {
    assert_eq!(visible_text_frames_per_char(TextSpeed::Fast), 1);
    assert_eq!(visible_text_frames_per_char(TextSpeed::Mid), 3);
    assert_eq!(visible_text_frames_per_char(TextSpeed::Slow), 5);

    let reveal = VisibleFieldTextReveal {
        text: "PROF. ELM".to_string(),
        page_index: 0,
        visible_chars: 5,
        frames_until_next_char: 0,
    };
    assert_eq!(
        reveal
            .text
            .chars()
            .take(reveal.visible_chars)
            .collect::<String>(),
        "PROF."
    );
}

#[test]
fn runtime_tile_to_metatile_u16_uses_runtime_metatile_width() {
    assert_eq!(
        runtime_tile_to_metatile_u16(2, 0, "test").expect("runtime metatile coordinate"),
        (1, 0)
    );
    assert_eq!(
        runtime_tile_to_metatile_u16(4, 6, "test").expect("runtime metatile coordinate"),
        (2, 3)
    );
    assert!(
        runtime_tile_to_metatile_u16(3, 1, "test")
            .expect_err("unaligned runtime tile must reject")
            .to_string()
            .contains("not aligned to metatile width")
    );
}

#[test]
fn visible_shell_uses_the_game_boy_frame_cadence() {
    assert!(
        (f64::from(GAME_TICK_SECONDS) - GB_FRAME_DURATION_SECONDS).abs() < 0.000_001,
        "visible title, intro, and input timing must use the core Game Boy frame duration"
    );
}

#[test]
fn facing_runtime_tile_uses_field_interaction_stride_before_metatile_conversion() {
    let front = facing_runtime_tile_from(
        TilePosition::new(2, 2),
        crate::core::world::map::Direction::Right,
    )
    .expect("facing runtime tile stays inside runtime coordinate bounds");

    assert_eq!(front, TilePosition::new(3, 2));
    let error = runtime_tile_to_metatile_u16(front.x, front.y, "test")
        .expect_err("odd runtime tile does not convert to metatile");
    assert!(error.to_string().contains("not aligned to metatile width"));
}

#[test]
fn facing_metatile_coordinates_skip_unaligned_runtime_tiles() {
    assert_eq!(
        facing_metatile_coordinates(4, 6).expect("aligned facing tile"),
        Some((2, 3))
    );
    assert_eq!(
        facing_metatile_coordinates(39, 6).expect("odd object tile is not a metatile block"),
        None
    );
    assert_eq!(
        facing_metatile_coordinates(-1, 6).expect("negative front tile is outside map"),
        None
    );
}
#[cfg(feature = "voxel-view")]
#[test]
fn renderer_readiness_cannot_switch_the_manually_selected_world_view_to_2d() {
    let mut app = App::new();
    app.insert_resource(crystal_voxel_view::VoxelViewSettings {
        enabled: true,
        allow_f3_toggle: true,
    })
    .add_systems(Update, sync_manual_world_view_layers);
    let classic_world = app.world_mut().spawn(PlayerMarker).id();

    app.update();
    let hidden_layers = app
        .world()
        .entity(classic_world)
        .get::<bevy::render::view::RenderLayers>()
        .expect("manual 2.5D selection must park the classic overworld");
    assert!(hidden_layers.intersects(
        &bevy::render::view::RenderLayers::layer(
            crystal_voxel_view::HIDDEN_CLASSIC_WORLD_RENDER_LAYER,
        )
    ));

    // No VoxelViewStatus resource exists in this app. That is deliberate:
    // readiness, errors, and build state are structurally unable to affect
    // classic-world visibility. Only the manual setting can restore 2D.
    app.world_mut()
        .resource_mut::<crystal_voxel_view::VoxelViewSettings>()
        .enabled = false;
    app.update();
    assert!(
        app.world()
            .entity(classic_world)
            .get::<bevy::render::view::RenderLayers>()
            .is_none(),
        "manual 2D selection must restore the classic overworld to layer 0"
    );
}
