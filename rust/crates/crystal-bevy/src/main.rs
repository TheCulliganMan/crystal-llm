use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use crystal_assets::{
    RuntimeBadgeRegion, RuntimeBattleTowerMobileFlag, RuntimeBugContestAction,
    RuntimeCableClubRequest, RuntimeCurrencyAccount, RuntimeCurrencyDeltaCommand,
    RuntimeDayCareAction, RuntimeDayCareCaretaker, RuntimeLinkBattleResult, RuntimeLinkRoomSpecial,
    RuntimeMobileHandshakeCommand, RuntimeMutationCommand, RuntimeMysteryGiftAction,
    RuntimePendingScriptRequest, RuntimePendingScriptRequestKind, RuntimeScriptEventDrainResult,
    RuntimeScriptEventQueue, RuntimeShuckieAction,
};
use crystal_bevy::{
    BevyShellConfig, BevyShellStart, CrystalRuntime, RuntimeGameShell, RuntimeGiftPokemonGrant,
    RuntimePartyRecovery, RuntimeShellSnapshot, VisibleShellBattleSmokeRef, VisibleShellSmokeItem,
    VisibleShellSmokePokemon,
    assets::{AssetRoot, modpack::COMPILED_GAME_PACK_EXTENSION},
    core::battle::turn::{BattleAction, BattleEvent, BattleSide},
    core::input::GameButton,
    core::models::{Dv, PARTY_SIZE},
    core::multiplayer::LinkMessage,
    core::save::SAVE_EXTENSION,
    core::systems::script_runtime::ScriptRuntimeInputs,
    core::systems::script_text::ScriptTextAction,
    core::systems::script_warps::ScriptMapAction,
    core::systems::special_routines::SpecialRoutineEffect,
    core::world::collision::{Terrain, TilesetCollision, describe_collision, sample_collision},
    core::world::encounters::EncounterSurface,
    core::world::map::{OverworldMapData, TilePosition},
};

const DEFAULT_PACK_PATH: &str = "content-packs/core-modular.crystalpack";

fn main() -> Result<()> {
    let args = parse_args()?;
    if args.help {
        print_usage();
        return Ok(());
    }
    if args.smoke_visible_title_new_game && args.smoke_visible_title_continue.is_some() {
        bail!(
            "--smoke-visible-title-new-game and --smoke-visible-title-continue are mutually exclusive"
        );
    }
    if args.smoke_visible_title_name_input
        && (args.smoke_visible_title_new_game || args.smoke_visible_title_continue.is_some())
    {
        bail!(
            "--smoke-visible-title-name-input cannot be combined with other visible title smokes"
        );
    }

    let pack = args.pack.as_deref().unwrap_or(DEFAULT_PACK_PATH);
    let explicit_repo = args.repo.is_some();
    let mut repository_root = match args.repo {
        Some(repo) => repo,
        None => env::current_dir().context("resolve current directory for default --repo")?,
    }
    .canonicalize()
    .context("canonicalize repository root")?;
    // Cargo users commonly launch from `rust/`, while the pack is rooted one
    // directory above it. Keep explicit --repo behavior strict, but make the
    // default executable discover the repository in either location.
    if !explicit_repo
        && !repository_root.join(pack).is_file()
        && repository_root
            .parent()
            .is_some_and(|parent| parent.join(pack).is_file())
    {
        repository_root = repository_root
            .parent()
            .expect("checked repository parent")
            .to_path_buf();
    }
    let asset_root = AssetRoot::new(repository_root.clone());
    let runtime = CrystalRuntime::load_from_compiled_pack(&asset_root, pack)?;

    if args.list_spawns {
        print_boot_summary(&runtime);
        print_spawns(&runtime);
        return Ok(());
    }

    if args.list_script_map_commands {
        print_script_map_commands(&runtime);
        return Ok(());
    }

    if args.list_script_scene_commands {
        print_script_scene_commands(&runtime);
        return Ok(());
    }

    if args.list_script_battles {
        print_script_battles(&runtime);
        return Ok(());
    }

    if args.list_gift_pokemon {
        print_gift_pokemon(&runtime);
        return Ok(());
    }

    if let Some(script_label) = args.list_script.as_deref() {
        print_compiled_script(&runtime, script_label)?;
        return Ok(());
    }

    if let Some(map_name) = args.list_map_objects.as_deref() {
        print_map_objects(&runtime, map_name)?;
        return Ok(());
    }

    if let Some(map_name) = args.list_map_events.as_deref() {
        print_map_events(&runtime, map_name)?;
        return Ok(());
    }

    if let Some(map_name) = args.list_field_targets.as_deref() {
        print_field_targets(&runtime, map_name)?;
        return Ok(());
    }

    if let Some(shop_ref) = args.smoke_shop.as_ref() {
        let spawn_identifier = args
            .spawn
            .context("--smoke-shop requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-shop cannot be combined with --load-save or --save-path");
        }
        smoke_script_shop(
            asset_root,
            runtime,
            spawn_identifier,
            shop_ref,
            args.smoke_start_map.as_ref(),
            &args.smoke_buttons,
            &args.smoke_script,
            args.smoke_money,
            args.smoke_buy.as_ref(),
            args.smoke_sell.as_ref(),
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if let Some(field_item) = args.smoke_field_item.as_ref() {
        let spawn_identifier = args
            .spawn
            .context("--smoke-field-item requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-field-item cannot be combined with --load-save or --save-path");
        }
        smoke_field_item_use(
            asset_root,
            runtime,
            spawn_identifier,
            field_item,
            args.smoke_start_map.as_ref(),
            &args.smoke_buttons,
            &args.smoke_script,
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if let Some(recovery) = args.smoke_party_recovery.as_ref() {
        let spawn_identifier = args
            .spawn
            .context("--smoke-party-recovery requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-party-recovery cannot be combined with --load-save or --save-path");
        }
        if args.smoke_party.is_empty() {
            bail!("--smoke-party-recovery requires --smoke-party Species:Level");
        }
        smoke_party_recovery(
            asset_root,
            runtime,
            spawn_identifier,
            recovery,
            args.smoke_start_map.as_ref(),
            &args.smoke_party,
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if args.smoke_roamers {
        let spawn_identifier = args
            .spawn
            .context("--smoke-roamers requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-roamers cannot be combined with --load-save or --save-path");
        }
        smoke_roamers(
            asset_root,
            runtime,
            spawn_identifier,
            args.smoke_start_map.as_ref(),
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if args.smoke_magikarp_length {
        let spawn_identifier = args
            .spawn
            .context("--smoke-magikarp-length requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-magikarp-length cannot be combined with --load-save or --save-path");
        }
        smoke_magikarp_length(
            asset_root,
            runtime,
            spawn_identifier,
            args.smoke_start_map.as_ref(),
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if args.smoke_odd_egg {
        let spawn_identifier = args
            .spawn
            .context("--smoke-odd-egg requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-odd-egg cannot be combined with --load-save or --save-path");
        }
        smoke_odd_egg(
            asset_root,
            runtime,
            spawn_identifier,
            args.smoke_start_map.as_ref(),
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if args.smoke_mystery_gift {
        let spawn_identifier = args
            .spawn
            .context("--smoke-mystery-gift requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-mystery-gift cannot be combined with --load-save or --save-path");
        }
        smoke_mystery_gift(
            asset_root,
            runtime,
            spawn_identifier,
            args.smoke_start_map.as_ref(),
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if args.smoke_buena_password {
        let spawn_identifier = args
            .spawn
            .context("--smoke-buena-password requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-buena-password cannot be combined with --load-save or --save-path");
        }
        smoke_buena_password(
            asset_root,
            runtime,
            spawn_identifier,
            args.smoke_start_map.as_ref(),
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if args.smoke_shuckie {
        let spawn_identifier = args
            .spawn
            .context("--smoke-shuckie requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-shuckie cannot be combined with --load-save or --save-path");
        }
        smoke_shuckie(
            asset_root,
            runtime,
            spawn_identifier,
            args.smoke_start_map.as_ref(),
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if args.smoke_dratini {
        let spawn_identifier = args
            .spawn
            .context("--smoke-dratini requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-dratini cannot be combined with --load-save or --save-path");
        }
        smoke_dratini(
            asset_root,
            runtime,
            spawn_identifier,
            args.smoke_start_map.as_ref(),
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if args.smoke_kurt_apricorn {
        let spawn_identifier = args
            .spawn
            .context("--smoke-kurt-apricorn requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-kurt-apricorn cannot be combined with --load-save or --save-path");
        }
        smoke_kurt_apricorn(
            asset_root,
            runtime,
            spawn_identifier,
            args.smoke_start_map.as_ref(),
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if args.smoke_bills_grandfather {
        let spawn_identifier = args
            .spawn
            .context("--smoke-bills-grandfather requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-bills-grandfather cannot be combined with --load-save or --save-path");
        }
        smoke_bills_grandfather(
            asset_root,
            runtime,
            spawn_identifier,
            args.smoke_start_map.as_ref(),
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if args.smoke_unown_printer {
        let spawn_identifier = args
            .spawn
            .context("--smoke-unown-printer requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-unown-printer cannot be combined with --load-save or --save-path");
        }
        smoke_unown_printer(
            asset_root,
            runtime,
            spawn_identifier,
            args.smoke_start_map.as_ref(),
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if args.smoke_map_radio {
        let spawn_identifier = args
            .spawn
            .context("--smoke-map-radio requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-map-radio cannot be combined with --load-save or --save-path");
        }
        smoke_map_radio(
            asset_root,
            runtime,
            spawn_identifier,
            args.smoke_start_map.as_ref(),
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if args.smoke_name_rater {
        let spawn_identifier = args
            .spawn
            .context("--smoke-name-rater requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-name-rater cannot be combined with --load-save or --save-path");
        }
        smoke_name_rater(
            asset_root,
            runtime,
            spawn_identifier,
            args.smoke_start_map.as_ref(),
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if args.smoke_poke_seer {
        let spawn_identifier = args
            .spawn
            .context("--smoke-poke-seer requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-poke-seer cannot be combined with --load-save or --save-path");
        }
        smoke_poke_seer(
            asset_root,
            runtime,
            spawn_identifier,
            args.smoke_start_map.as_ref(),
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if args.smoke_bank_of_mom {
        let spawn_identifier = args
            .spawn
            .context("--smoke-bank-of-mom requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-bank-of-mom cannot be combined with --load-save or --save-path");
        }
        smoke_bank_of_mom(
            asset_root,
            runtime,
            spawn_identifier,
            args.smoke_start_map.as_ref(),
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if args.smoke_link_records {
        let spawn_identifier = args
            .spawn
            .context("--smoke-link-records requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-link-records cannot be combined with --load-save or --save-path");
        }
        smoke_link_records(
            asset_root,
            runtime,
            spawn_identifier,
            args.smoke_start_map.as_ref(),
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if args.smoke_link_rooms {
        let spawn_identifier = args
            .spawn
            .context("--smoke-link-rooms requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-link-rooms cannot be combined with --load-save or --save-path");
        }
        smoke_link_rooms(
            asset_root,
            runtime,
            spawn_identifier,
            args.smoke_start_map.as_ref(),
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if args.smoke_link_handshake {
        let spawn_identifier = args
            .spawn
            .context("--smoke-link-handshake requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-link-handshake cannot be combined with --load-save or --save-path");
        }
        smoke_link_handshake(
            asset_root,
            runtime,
            spawn_identifier,
            args.smoke_start_map.as_ref(),
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if args.smoke_battle_tower {
        let spawn_identifier = args
            .spawn
            .context("--smoke-battle-tower requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-battle-tower cannot be combined with --load-save or --save-path");
        }
        smoke_battle_tower(
            asset_root,
            runtime,
            spawn_identifier,
            args.smoke_start_map.as_ref(),
            &args.smoke_party,
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if args.smoke_bug_contest {
        let spawn_identifier = args
            .spawn
            .context("--smoke-bug-contest requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-bug-contest cannot be combined with --load-save or --save-path");
        }
        if args.smoke_party.len() < 2 {
            bail!("--smoke-bug-contest requires at least two --smoke-party Species:Level entries");
        }
        smoke_bug_contest(
            asset_root,
            runtime,
            spawn_identifier,
            args.smoke_start_map.as_ref(),
            &args.smoke_party,
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if args.smoke_day_care {
        let spawn_identifier = args
            .spawn
            .context("--smoke-day-care requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-day-care cannot be combined with --load-save or --save-path");
        }
        if args.smoke_party.len() < 2 {
            bail!("--smoke-day-care requires at least two --smoke-party Species:Level entries");
        }
        smoke_day_care(
            asset_root,
            runtime,
            spawn_identifier,
            args.smoke_start_map.as_ref(),
            &args.smoke_party,
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if let Some(field_move) = args.smoke_field_move.as_ref() {
        let spawn_identifier = args
            .spawn
            .context("--smoke-field-move requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-field-move cannot be combined with --load-save or --save-path");
        }
        smoke_field_move_use(
            asset_root,
            runtime,
            spawn_identifier,
            field_move,
            args.smoke_start_map.as_ref(),
            &args.smoke_set_flags,
            &args.smoke_buttons,
            &args.smoke_script,
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if let Some(fishing) = args.smoke_fishing.as_ref() {
        let spawn_identifier = args
            .spawn
            .context("--smoke-fishing requires --spawn <id> for a runtime shell")?;
        let party = args
            .smoke_party
            .first()
            .context("--smoke-fishing requires --smoke-party Species:Level")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-fishing cannot be combined with --load-save or --save-path");
        }
        smoke_fishing(
            asset_root,
            runtime,
            spawn_identifier,
            fishing,
            args.smoke_start_map.as_ref(),
            party,
            &args.smoke_buttons,
            &args.smoke_script,
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if let Some(menu) = args.smoke_menu.as_ref() {
        let spawn_identifier = args
            .spawn
            .context("--smoke-menu requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-menu cannot be combined with --load-save or --save-path");
        }
        smoke_vertical_menu(
            asset_root,
            runtime,
            spawn_identifier,
            menu,
            args.smoke_start_map.as_ref(),
            &args.smoke_buttons,
            &args.smoke_script,
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if args.smoke_interact {
        let spawn_identifier = args
            .spawn
            .context("--smoke-interact requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-interact cannot be combined with --load-save or --save-path");
        }
        smoke_interaction(
            asset_root,
            runtime,
            spawn_identifier,
            args.smoke_start_map.as_ref(),
            &args.smoke_buttons,
            &args.smoke_script,
            args.smoke_save.as_ref(),
            args.smoke_player_name.as_deref(),
        )?;
        return Ok(());
    }

    if args.smoke_pc_storage {
        let spawn_identifier = args
            .spawn
            .context("--smoke-pc-storage requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-pc-storage cannot be combined with --load-save or --save-path");
        }
        if args.smoke_party.len() < 2 {
            bail!("--smoke-pc-storage requires at least two --smoke-party Species:Level entries");
        }
        let pc_item = args
            .smoke_pc_item
            .as_ref()
            .context("--smoke-pc-storage requires --smoke-pc-item ItemId:Quantity")?;
        smoke_pc_storage(
            asset_root,
            runtime,
            spawn_identifier,
            &args.smoke_party,
            pc_item,
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if let Some(elevator) = args.smoke_elevator.as_ref() {
        let spawn_identifier = args
            .spawn
            .context("--smoke-elevator requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-elevator cannot be combined with --load-save or --save-path");
        }
        smoke_elevator(
            asset_root,
            runtime,
            spawn_identifier,
            elevator,
            &args.smoke_buttons,
            &args.smoke_script,
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if let Some(elevfloor) = args.smoke_elevfloor.as_ref() {
        let spawn_identifier = args
            .spawn
            .context("--smoke-elevfloor requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-elevfloor cannot be combined with --load-save or --save-path");
        }
        smoke_elevfloor_command(
            asset_root,
            runtime,
            spawn_identifier,
            elevfloor,
            args.smoke_start_map.as_ref(),
            &args.smoke_buttons,
            &args.smoke_script,
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if let Some(script_warp) = args.smoke_script_warp.as_ref() {
        let spawn_identifier = args
            .spawn
            .context("--smoke-script-warp requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-script-warp cannot be combined with --load-save or --save-path");
        }
        smoke_script_warp_command(
            asset_root,
            runtime,
            spawn_identifier,
            script_warp,
            args.smoke_start_map.as_ref(),
            args.smoke_script_warp_pending,
            &args.smoke_buttons,
            &args.smoke_script,
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if let Some(script_map) = args.smoke_script_map_pending.as_ref() {
        let spawn_identifier = args
            .spawn
            .context("--smoke-script-map-pending requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-script-map-pending cannot be combined with --load-save or --save-path");
        }
        smoke_script_map_pending_command(
            asset_root,
            runtime,
            spawn_identifier,
            script_map,
            args.smoke_start_map.as_ref(),
            &args.smoke_buttons,
            &args.smoke_script,
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if let Some(script_text) = args.smoke_script_text_pending.as_ref() {
        let spawn_identifier = args
            .spawn
            .context("--smoke-script-text-pending requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-script-text-pending cannot be combined with --load-save or --save-path");
        }
        smoke_script_text_pending_command(
            asset_root,
            runtime,
            spawn_identifier,
            script_text,
            args.smoke_start_map.as_ref(),
            &args.smoke_buttons,
            &args.smoke_script,
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if let Some(wild_battle) = args.smoke_wild_battle.as_ref() {
        let spawn_identifier = args
            .spawn
            .context("--smoke-wild-battle requires --spawn <id> for a runtime shell")?;
        args.smoke_party
            .first()
            .context("--smoke-wild-battle requires --smoke-party Species:Level")?;
        if args.smoke_capture_ball.is_some() && args.smoke_battle_item.is_some() {
            bail!("--smoke-capture-ball and --smoke-battle-item are mutually exclusive");
        }
        let (player_action, enemy_action) =
            if args.smoke_capture_ball.is_some() || args.smoke_battle_item.is_some() {
                (None, None)
            } else {
                (
                    Some(
                        args.smoke_player_action
                            .clone()
                            .context("--smoke-wild-battle requires --smoke-player-action")?,
                    ),
                    Some(
                        args.smoke_enemy_action
                            .clone()
                            .context("--smoke-wild-battle requires --smoke-enemy-action")?,
                    ),
                )
            };
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-wild-battle cannot be combined with --load-save or --save-path");
        }
        smoke_wild_battle(
            asset_root,
            runtime,
            spawn_identifier,
            wild_battle,
            args.smoke_start_map.as_ref(),
            &args.smoke_party,
            player_action,
            enemy_action,
            args.smoke_capture_ball.as_deref(),
            args.smoke_battle_item.as_deref(),
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if let Some(gift_ref) = args.smoke_gift_pokemon.as_ref() {
        let spawn_identifier = args
            .spawn
            .context("--smoke-gift-pokemon requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-gift-pokemon cannot be combined with --load-save or --save-path");
        }
        smoke_gift_pokemon(
            asset_root,
            runtime,
            spawn_identifier,
            gift_ref,
            args.smoke_start_map.as_ref(),
            &args.smoke_buttons,
            &args.smoke_script,
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if let Some(trainer_battle) = args.smoke_trainer_battle.as_ref() {
        let spawn_identifier = args
            .spawn
            .context("--smoke-trainer-battle requires --spawn <id> for a runtime shell")?;
        args.smoke_party
            .first()
            .context("--smoke-trainer-battle requires --smoke-party Species:Level")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-trainer-battle cannot be combined with --load-save or --save-path");
        }
        smoke_trainer_battle(
            asset_root,
            runtime,
            spawn_identifier,
            trainer_battle,
            args.smoke_start_map.as_ref(),
            &args.smoke_party,
            args.smoke_save.as_ref(),
        )?;
        return Ok(());
    }

    if args.smoke_link_journal {
        let spawn_identifier = args
            .spawn
            .context("--smoke-link-journal requires --spawn <id> for a runtime shell")?;
        if args.load_save.is_some() || args.save_path.is_some() || args.smoke_save.is_some() {
            bail!(
                "--smoke-link-journal cannot be combined with --load-save, --save-path, or --smoke-save"
            );
        }
        if args.smoke_buttons.is_empty() && args.smoke_script.is_empty() {
            bail!("--smoke-link-journal requires --smoke-buttons or --smoke-script");
        }
        smoke_link_journal(
            asset_root,
            runtime,
            spawn_identifier,
            args.smoke_start_map.as_ref(),
            &args.smoke_buttons,
            &args.smoke_script,
        )?;
        return Ok(());
    }

    if let Some(save_path) = args
        .smoke_save
        .as_ref()
        .filter(|_| !args.smoke_visible_overworld)
    {
        let spawn_identifier = args
            .spawn
            .context("--smoke-save requires --spawn <id> for a new-game smoke")?;
        if args.load_save.is_some() || args.save_path.is_some() || args.smoke_load_save.is_some() {
            bail!(
                "--smoke-save cannot be combined with --load-save, --save-path, or --smoke-load-save"
            );
        }
        smoke_save_resume(
            asset_root,
            runtime,
            spawn_identifier,
            save_path,
            &args.smoke_buttons,
            &args.smoke_script,
        )?;
        return Ok(());
    }

    if let Some(save_path) = args.smoke_title_new_game.as_ref() {
        if args.spawn.is_some()
            || args.load_save.is_some()
            || args.save_path.is_some()
            || args.smoke_load_save.is_some()
        {
            bail!(
                "--smoke-title-new-game cannot be combined with --spawn, --load-save, --save-path, or --smoke-load-save"
            );
        }
        let smoke_player_name = args
            .smoke_player_name
            .as_deref()
            .context("--smoke-title-new-game requires --smoke-player-name <name>")?;
        smoke_title_new_game(asset_root, runtime, save_path, smoke_player_name)?;
        return Ok(());
    }

    if let Some(load_path) = args.smoke_load_save.as_ref() {
        if args.spawn.is_some() || args.load_save.is_some() {
            bail!("--smoke-load-save cannot be combined with --spawn or --load-save");
        }
        smoke_load_save(asset_root, runtime, load_path, args.save_path.as_ref())?;
        return Ok(());
    }

    if args.smoke_visible_title_new_game {
        let spawn_identifier = runtime
            .title_new_game_spawn_identifier()
            .context("resolve visible title new-game spawn from compiled pack")?;
        if args.spawn.is_some() || args.load_save.is_some() {
            bail!("--smoke-visible-title-new-game cannot be combined with --spawn or --load-save");
        }
        let smoke = crystal_bevy::smoke_visible_shell_title(
            asset_root,
            runtime,
            spawn_identifier,
            args.save_path.clone(),
            false,
        )?;
        println!(
            "visible_title selected={} title=[{}] map={} tile=({}, {}) checksum={:?} saved_frame={} save_path={}",
            smoke.selected,
            smoke.title_entries.join("|"),
            smoke.map,
            smoke.tile_x,
            smoke.tile_y,
            smoke.state_hash,
            smoke
                .saved_frame
                .map(|frame| frame.to_string())
                .unwrap_or_else(|| "none".to_string()),
            smoke
                .save_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "none".to_string())
        );
        return Ok(());
    }

    if args.smoke_visible_title_name_input {
        let spawn_identifier = runtime
            .title_new_game_spawn_identifier()
            .context("resolve visible title name-input spawn from compiled pack")?;
        if args.spawn.is_some() || args.load_save.is_some() {
            bail!(
                "--smoke-visible-title-name-input cannot be combined with --spawn or --load-save"
            );
        }
        let smoke = crystal_bevy::smoke_visible_shell_title_name_input(
            asset_root,
            runtime,
            spawn_identifier,
            args.save_path.clone(),
            args.smoke_player_name
                .as_deref()
                .context("--smoke-visible-title-name-input requires --smoke-player-name <name>")?,
        )?;
        println!(
            "visible_title_name_input selected={} title=[{}] initial_name=[{}] typed_name=[{}] trainer={} map={} tile=({}, {}) checksum={:?} saved_frame={} save_path={}",
            smoke.selected,
            smoke.title_entries.join("|"),
            smoke.initial_name_entries.join("|"),
            smoke.typed_name_entries.join("|"),
            smoke.trainer_name,
            smoke.map,
            smoke.tile_x,
            smoke.tile_y,
            smoke.state_hash,
            smoke
                .saved_frame
                .map(|frame| frame.to_string())
                .unwrap_or_else(|| "none".to_string()),
            smoke
                .save_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "none".to_string())
        );
        return Ok(());
    }

    if let Some(save_path) = args.smoke_visible_title_continue.as_ref() {
        let spawn_identifier = runtime
            .title_new_game_spawn_identifier()
            .context("resolve visible title continue spawn from compiled pack")?;
        if args.spawn.is_some() || args.load_save.is_some() || args.save_path.is_some() {
            bail!(
                "--smoke-visible-title-continue cannot be combined with --spawn, --load-save, or --save-path"
            );
        }
        let smoke = crystal_bevy::smoke_visible_shell_title(
            asset_root,
            runtime,
            spawn_identifier,
            Some(save_path.clone()),
            true,
        )?;
        println!(
            "visible_title selected={} title=[{}] map={} tile=({}, {}) checksum={:?} saved_frame={} save_path={}",
            smoke.selected,
            smoke.title_entries.join("|"),
            smoke.map,
            smoke.tile_x,
            smoke.tile_y,
            smoke.state_hash,
            smoke
                .saved_frame
                .map(|frame| frame.to_string())
                .unwrap_or_else(|| "none".to_string()),
            smoke
                .save_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "none".to_string())
        );
        return Ok(());
    }

    if let Some(save_path) = args.smoke_visible_start_menu.as_ref() {
        let spawn_identifier = args
            .spawn
            .context("--smoke-visible-start-menu requires --spawn <id>")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-visible-start-menu cannot be combined with --load-save or --save-path");
        }
        args.smoke_party
            .first()
            .context("--smoke-visible-start-menu requires --smoke-party Species:Level")?;
        args.smoke_visible_bag_item.first().context(
            "--smoke-visible-start-menu requires --smoke-visible-bag-item ItemId:Quantity",
        )?;
        let party = args
            .smoke_party
            .iter()
            .map(|pokemon| VisibleShellSmokePokemon {
                species_id: pokemon.species_id.clone(),
                level: pokemon.level,
                held_item_id: pokemon.held_item_id.clone(),
            })
            .collect::<Vec<_>>();
        let bag_items = args
            .smoke_visible_bag_item
            .iter()
            .map(|item| VisibleShellSmokeItem {
                item_id: item.item_id.clone(),
                quantity: item.quantity,
            })
            .collect::<Vec<_>>();
        let smoke = crystal_bevy::smoke_visible_shell_start_menu(
            asset_root,
            runtime,
            BevyShellStart::NewGame { spawn_identifier },
            BevyShellConfig {
                quick_save_path: Some(save_path.clone()),
                smoke_player_name: args.smoke_player_name.clone(),
            },
            &party,
            &bag_items,
        )?;
        println!(
            "visible_start_menu map={} tile=({}, {}) start=[{}] party=[{}] pack=[{}] trainer=[{}] save=[{}] saved_frame={} save_path={}",
            smoke.initial_map,
            smoke.initial_tile_x,
            smoke.initial_tile_y,
            smoke.start_menu_entries.join("|"),
            smoke.party_entries.join("|"),
            smoke.pack_entries.join("|"),
            smoke.trainer_entries.join("|"),
            smoke.save_entries.join("|"),
            smoke.saved_frame,
            smoke.save_path.display()
        );
        return Ok(());
    }

    if args.smoke_visible_party {
        let spawn_identifier = args
            .spawn
            .context("--smoke-visible-party requires --spawn <id>")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-visible-party cannot be combined with --load-save or --save-path");
        }
        if args.smoke_party.len() < 2 {
            bail!("--smoke-visible-party requires at least two --smoke-party Species:Level values");
        }
        let party = args
            .smoke_party
            .iter()
            .map(|pokemon| VisibleShellSmokePokemon {
                species_id: pokemon.species_id.clone(),
                level: pokemon.level,
                held_item_id: pokemon.held_item_id.clone(),
            })
            .collect::<Vec<_>>();
        let smoke = crystal_bevy::smoke_visible_shell_party(
            asset_root,
            runtime,
            BevyShellStart::NewGame { spawn_identifier },
            BevyShellConfig {
                smoke_player_name: args.smoke_player_name.clone(),
                ..Default::default()
            },
            &party,
        )?;
        println!(
            "visible_party lead_before={} lead_after={} initial=[{}] actions=[{}] summary=[{}] switch=[{}] final=[{}] checksum={:?}",
            smoke.lead_before,
            smoke.lead_after,
            smoke.initial_entries.join("|"),
            smoke.action_entries.join("|"),
            smoke.summary_entries.join("|"),
            smoke.switch_entries.join("|"),
            smoke.final_entries.join("|"),
            smoke.state_hash
        );
        return Ok(());
    }

    if args.smoke_visible_overworld {
        if args.save_path.is_some() {
            bail!("--smoke-visible-overworld cannot be combined with --save-path");
        }
        let start = match (args.spawn, args.load_save.as_ref()) {
            (Some(spawn_identifier), None) => bevy_shell_start_from_smoke_start_map(
                spawn_identifier,
                args.smoke_start_map.as_ref(),
            ),
            (None, Some(save_path)) => BevyShellStart::LoadSave {
                save_path: save_path.clone(),
            },
            (Some(_), Some(_)) => {
                bail!("--smoke-visible-overworld cannot combine --spawn and --load-save");
            }
            (None, None) => bail!("--smoke-visible-overworld requires --spawn <id> or --load-save"),
        };
        if args.smoke_buttons.is_empty() && args.smoke_script.is_empty() {
            bail!("--smoke-visible-overworld requires --smoke-buttons or --smoke-script");
        }
        let input_frames = smoke_input_frames(&args.smoke_buttons, &args.smoke_script)
            .into_iter()
            .map(<[GameButton]>::to_vec)
            .collect::<Vec<_>>();
        let smoke = crystal_bevy::smoke_visible_shell_overworld(
            asset_root,
            runtime,
            start,
            BevyShellConfig {
                smoke_player_name: args.smoke_player_name.clone(),
                ..Default::default()
            },
            &input_frames,
            args.smoke_save.as_ref(),
        )?;
        println!(
            "visible_overworld start={}@({}, {}) start_scene={} final={}@({}, {}) final_scene={} frames={} interactions={} coord_events={} trainer_sight={} warps={} connections={} wild_battles={} last_movement={} active_music={} pending_audio={} frame_events=[{}] audio=[{}] checksum={:?}",
            smoke.start_map,
            smoke.start_tile_x,
            smoke.start_tile_y,
            smoke.start_scene.as_deref().unwrap_or("none"),
            smoke.final_map,
            smoke.final_tile_x,
            smoke.final_tile_y,
            smoke.final_scene.as_deref().unwrap_or("none"),
            smoke.frames,
            smoke.interactions,
            smoke.coord_events,
            smoke.trainer_sight_events,
            smoke.warps,
            smoke.connections,
            smoke.wild_battles,
            smoke.last_movement.as_deref().unwrap_or("none"),
            smoke.active_music.as_deref().unwrap_or("none"),
            smoke.pending_audio,
            smoke.frame_events.join("|"),
            smoke.audio_events.join("|"),
            smoke.state_hash
        );
        return Ok(());
    }

    if let Some(wild_battle) = args.smoke_visible_wild_battle.as_ref() {
        let spawn_identifier = args
            .spawn
            .context("--smoke-visible-wild-battle requires --spawn <id>")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!("--smoke-visible-wild-battle cannot be combined with --load-save or --save-path");
        }
        args.smoke_party
            .first()
            .context("--smoke-visible-wild-battle requires --smoke-party Species:Level")?;
        let party = args
            .smoke_party
            .iter()
            .map(|pokemon| VisibleShellSmokePokemon {
                species_id: pokemon.species_id.clone(),
                level: pokemon.level,
                held_item_id: pokemon.held_item_id.clone(),
            })
            .collect::<Vec<_>>();
        let bag_items = args
            .smoke_visible_bag_item
            .iter()
            .map(|item| VisibleShellSmokeItem {
                item_id: item.item_id.clone(),
                quantity: item.quantity,
            })
            .collect::<Vec<_>>();
        let smoke = crystal_bevy::smoke_visible_shell_wild_battle(
            asset_root,
            runtime,
            bevy_shell_start_from_smoke_start_map(spawn_identifier, args.smoke_start_map.as_ref()),
            BevyShellConfig {
                smoke_player_name: args.smoke_player_name.clone(),
                ..Default::default()
            },
            VisibleShellBattleSmokeRef {
                map_name: wild_battle.map_name.clone(),
                source_script: wild_battle.source_script.clone(),
                command_index: wild_battle.command_index,
            },
            &party,
            &bag_items,
        )?;
        println!(
            "visible_wild_battle species={} level={} actions=[{}] switch=[{}] pack=[{}] balls=[{}] moves=[{}] after=[{}] active_after={} checksum={:?}",
            smoke.wild_species,
            smoke.wild_level,
            smoke.action_entries.join("|"),
            smoke.switch_entries.join("|"),
            smoke.pack_entries.join("|"),
            smoke.ball_entries.join("|"),
            smoke.move_entries.join("|"),
            smoke.after_entries.join("|"),
            smoke.active_battle_after,
            smoke.state_hash
        );
        return Ok(());
    }

    if let Some(trainer_battle) = args.smoke_visible_trainer_battle.as_ref() {
        let spawn_identifier = args
            .spawn
            .context("--smoke-visible-trainer-battle requires --spawn <id>")?;
        if args.load_save.is_some() || args.save_path.is_some() {
            bail!(
                "--smoke-visible-trainer-battle cannot be combined with --load-save or --save-path"
            );
        }
        args.smoke_party
            .first()
            .context("--smoke-visible-trainer-battle requires --smoke-party Species:Level")?;
        let party = args
            .smoke_party
            .iter()
            .map(|pokemon| VisibleShellSmokePokemon {
                species_id: pokemon.species_id.clone(),
                level: pokemon.level,
                held_item_id: pokemon.held_item_id.clone(),
            })
            .collect::<Vec<_>>();
        let smoke = crystal_bevy::smoke_visible_shell_trainer_battle(
            asset_root,
            runtime,
            bevy_shell_start_from_smoke_start_map(spawn_identifier, args.smoke_start_map.as_ref()),
            BevyShellConfig {
                smoke_player_name: args.smoke_player_name.clone(),
                ..Default::default()
            },
            VisibleShellBattleSmokeRef {
                map_name: trainer_battle.map_name.clone(),
                source_script: trainer_battle.source_script.clone(),
                command_index: trainer_battle.command_index,
            },
            &party,
        )?;
        println!(
            "visible_trainer_battle trainer={}:{} name={} initial=[{}] first_moves=[{}] shift_prompt=[{}] shift_prompts={} kept_shift={} switched_shift={} turns={} defeated={} final=[{}] active_after={} checksum={:?}",
            smoke.trainer_class,
            smoke.trainer_id,
            smoke.trainer_name,
            smoke.initial_entries.join("|"),
            smoke.first_move_entries.join("|"),
            smoke.shift_prompt_entries.join("|"),
            smoke.shift_prompt_count,
            smoke.kept_current_after_shift_prompt,
            smoke.switched_after_shift_prompt,
            smoke.turns,
            smoke.trainer_defeated,
            smoke.final_entries.join("|"),
            smoke.active_battle_after,
            smoke.state_hash
        );
        return Ok(());
    }

    let default_save_path = default_save_path(&repository_root, &runtime);
    let start = match (args.spawn, args.load_save) {
        (Some(_), None) => bail!(
            "--spawn is a smoke/debug entry point and cannot launch the interactive game; omit --spawn to start through the ASM intro/title/Oak sequence, or use --load-save to resume"
        ),
        (None, Some(save_path)) => BevyShellStart::LoadSave { save_path },
        (Some(_), Some(_)) => bail!("--spawn and --load-save are mutually exclusive"),
        (None, None) => BevyShellStart::Title {
            spawn_identifier: runtime.title_new_game_spawn_identifier()?,
            save_path: default_save_path
                .exists()
                .then_some(default_save_path.clone()),
        },
    };

    crystal_bevy::run_bevy_shell(
        asset_root,
        runtime,
        start,
        BevyShellConfig {
            quick_save_path: Some(args.save_path.unwrap_or_else(|| default_save_path.clone())),
            ..Default::default()
        },
    )
}

#[derive(Debug, Default)]
struct Args {
    help: bool,
    list_spawns: bool,
    list_script_map_commands: bool,
    list_script_scene_commands: bool,
    list_script_battles: bool,
    list_gift_pokemon: bool,
    list_script: Option<String>,
    list_map_objects: Option<String>,
    list_map_events: Option<String>,
    list_field_targets: Option<String>,
    repo: Option<PathBuf>,
    pack: Option<String>,
    spawn: Option<u16>,
    load_save: Option<PathBuf>,
    save_path: Option<PathBuf>,
    smoke_save: Option<PathBuf>,
    smoke_title_new_game: Option<PathBuf>,
    smoke_load_save: Option<PathBuf>,
    smoke_buttons: Vec<GameButton>,
    smoke_script: Vec<Vec<GameButton>>,
    smoke_shop: Option<SmokeShopRef>,
    smoke_money: Option<u32>,
    smoke_buy: Option<SmokeShopTransactionRef>,
    smoke_sell: Option<SmokeShopTransactionRef>,
    smoke_field_item: Option<SmokeFieldItemRef>,
    smoke_party_recovery: Option<SmokePartyRecoveryKind>,
    smoke_roamers: bool,
    smoke_magikarp_length: bool,
    smoke_odd_egg: bool,
    smoke_mystery_gift: bool,
    smoke_buena_password: bool,
    smoke_shuckie: bool,
    smoke_dratini: bool,
    smoke_kurt_apricorn: bool,
    smoke_bills_grandfather: bool,
    smoke_unown_printer: bool,
    smoke_map_radio: bool,
    smoke_name_rater: bool,
    smoke_poke_seer: bool,
    smoke_bank_of_mom: bool,
    smoke_link_records: bool,
    smoke_link_rooms: bool,
    smoke_link_handshake: bool,
    smoke_battle_tower: bool,
    smoke_bug_contest: bool,
    smoke_day_care: bool,
    smoke_field_move: Option<SmokeFieldMoveRef>,
    smoke_fishing: Option<SmokeFishingRef>,
    smoke_menu: Option<SmokeMenuRef>,
    smoke_interact: bool,
    smoke_pc_storage: bool,
    smoke_pc_item: Option<SmokeShopTransactionRef>,
    smoke_link_journal: bool,
    smoke_elevator: Option<SmokeElevatorRef>,
    smoke_elevfloor: Option<SmokeElevfloorRef>,
    smoke_script_warp: Option<SmokeScriptWarpRef>,
    smoke_script_warp_pending: bool,
    smoke_script_map_pending: Option<SmokeScriptMapPendingRef>,
    smoke_script_text_pending: Option<SmokeScriptTextPendingRef>,
    smoke_start_map: Option<SmokeStartMapRef>,
    smoke_set_flags: Vec<String>,
    smoke_gift_pokemon: Option<SmokeScriptCommandRef>,
    smoke_wild_battle: Option<SmokeScriptCommandRef>,
    smoke_party: Vec<SmokePartyPokemonRef>,
    smoke_player_action: Option<BattleAction>,
    smoke_enemy_action: Option<BattleAction>,
    smoke_player_name: Option<String>,
    smoke_capture_ball: Option<String>,
    smoke_battle_item: Option<String>,
    smoke_trainer_battle: Option<SmokeScriptCommandRef>,
    smoke_visible_start_menu: Option<PathBuf>,
    smoke_visible_bag_item: Vec<SmokeShopTransactionRef>,
    smoke_visible_title_new_game: bool,
    smoke_visible_title_name_input: bool,
    smoke_visible_title_continue: Option<PathBuf>,
    smoke_visible_party: bool,
    smoke_visible_overworld: bool,
    smoke_visible_wild_battle: Option<SmokeScriptCommandRef>,
    smoke_visible_trainer_battle: Option<SmokeScriptCommandRef>,
}

#[derive(Debug, Clone)]
struct SmokeShopRef {
    map_name: String,
    source_script: String,
    command_index: usize,
}

#[derive(Debug, Clone)]
struct SmokeScriptCommandRef {
    map_name: String,
    source_script: String,
    command_index: usize,
}

#[derive(Debug, Clone)]
struct SmokeShopTransactionRef {
    item_id: String,
    quantity: u16,
}

#[derive(Debug, Clone)]
struct SmokePartyPokemonRef {
    species_id: String,
    level: u8,
    held_item_id: Option<String>,
}

#[derive(Debug, Clone)]
struct SmokeFieldItemRef {
    kind: SmokeFieldItemKind,
    item_id: String,
    quantity: u16,
}

#[derive(Debug, Clone)]
enum SmokeFieldItemKind {
    Repel,
    Bicycle,
    Itemfinder,
    TownMap,
    Register,
    Pokegear,
    Box,
    EscapeRope,
    TmHm {
        species_id: String,
        level: u8,
        replace_slot: Option<usize>,
    },
    EvolutionStone {
        species_id: String,
        level: u8,
        expected_species_id: String,
    },
}

#[derive(Debug, Clone)]
enum SmokePartyRecoveryKind {
    FullHeal,
    Blackout,
}

#[derive(Debug, Clone)]
struct SmokeFieldMoveRef {
    kind: SmokeFieldMoveKind,
    species_id: String,
    level: u8,
    move_id: String,
}

#[derive(Debug, Clone)]
enum SmokeFieldMoveKind {
    Cut {
        metatile_x: u16,
        metatile_y: u16,
    },
    Whirlpool {
        metatile_x: u16,
        metatile_y: u16,
    },
    Strength,
    Flash,
    Surf,
    Waterfall,
    Fly {
        destination_spawn_identifier: u16,
        flypoint_flag: String,
    },
    Dig,
    Teleport,
    Headbutt {
        player_id: u16,
    },
    RockSmash,
    SweetScent {
        surface: EncounterSurface,
    },
}

#[derive(Debug, Clone)]
struct SmokeFishingRef {
    kind: SmokeFishingKind,
    id: String,
}

#[derive(Debug, Clone, Copy)]
enum SmokeFishingKind {
    Rod,
    Item,
}

#[derive(Debug, Clone)]
struct SmokeMenuRef {
    map_name: String,
    source_script: String,
    loadmenu_command_index: usize,
    verticalmenu_command_index: usize,
    option_index: usize,
    option: String,
}

#[derive(Debug, Clone)]
struct SmokeElevatorRef {
    map_name: String,
    data_label: String,
    source_script: String,
    elevator_command_index: usize,
    floor_index: usize,
    floor: String,
    warp: u16,
    target_map: String,
}

#[derive(Debug, Clone)]
struct SmokeElevfloorRef {
    map_name: String,
    source_script: String,
    command_index: usize,
    target_map: String,
}

#[derive(Debug, Clone)]
struct SmokeScriptWarpRef {
    map_name: String,
    source_script: String,
    command_index: usize,
    target_map: String,
}

#[derive(Debug, Clone)]
struct SmokeScriptMapPendingRef {
    map_name: String,
    source_script: String,
    command_index: usize,
    command: String,
}

#[derive(Debug, Clone)]
struct SmokeScriptTextPendingRef {
    map_name: String,
    source_script: String,
    open_command_index: usize,
    command_index: usize,
    command: String,
    accepted: bool,
}

#[derive(Debug, Clone)]
struct SmokeStartMapRef {
    map_name: String,
    tile_x: i16,
    tile_y: i16,
}

fn parse_args() -> Result<Args> {
    let mut args = Args::default();
    let mut values = env::args().skip(1);
    while let Some(arg) = values.next() {
        match arg.as_str() {
            "-h" | "--help" => args.help = true,
            "--list-spawns" => args.list_spawns = true,
            "--list-script-map-commands" => args.list_script_map_commands = true,
            "--list-script-scene-commands" => args.list_script_scene_commands = true,
            "--list-script-battles" => args.list_script_battles = true,
            "--list-gift-pokemon" => args.list_gift_pokemon = true,
            "--list-script" => {
                args.list_script = Some(
                    values
                        .next()
                        .context("--list-script requires ScriptLabel")?,
                )
            }
            "--list-map-objects" => {
                let value = values
                    .next()
                    .context("--list-map-objects requires MapName")?;
                if value.is_empty() {
                    bail!("--list-map-objects map name cannot be empty");
                }
                args.list_map_objects = Some(value);
            }
            "--list-map-events" => {
                let value = values
                    .next()
                    .context("--list-map-events requires MapName")?;
                if value.is_empty() {
                    bail!("--list-map-events map name cannot be empty");
                }
                args.list_map_events = Some(value);
            }
            "--list-field-targets" => {
                let value = values
                    .next()
                    .context("--list-field-targets requires MapName")?;
                if value.is_empty() {
                    bail!("--list-field-targets map name cannot be empty");
                }
                args.list_field_targets = Some(value);
            }
            "--repo" => {
                let value = values.next().context("--repo requires a path")?;
                args.repo = Some(parse_cli_path_arg("--repo", value)?);
            }
            "--pack" => {
                let value = values
                    .next()
                    .context("--pack requires a .crystalpack path")?;
                args.pack = Some(parse_pack_arg("--pack", value)?);
            }
            "--spawn" => {
                let value = values.next().context("--spawn requires an id")?;
                args.spawn = Some(
                    value
                        .parse::<u16>()
                        .with_context(|| format!("spawn id '{value}' is not a u16"))?,
                );
            }
            "--load-save" => {
                let value = values.next().context("--load-save requires a path")?;
                args.load_save = Some(parse_save_path_arg("--load-save", value)?);
            }
            "--save-path" => {
                let value = values.next().context("--save-path requires a path")?;
                args.save_path = Some(parse_save_path_arg("--save-path", value)?);
            }
            "--smoke-save" => {
                let value = values.next().context("--smoke-save requires a path")?;
                args.smoke_save = Some(parse_save_path_arg("--smoke-save", value)?);
            }
            "--smoke-title-new-game" => {
                let value = values
                    .next()
                    .context("--smoke-title-new-game requires a save path")?;
                args.smoke_title_new_game =
                    Some(parse_save_path_arg("--smoke-title-new-game", value)?);
            }
            "--smoke-load-save" => {
                let value = values.next().context("--smoke-load-save requires a path")?;
                args.smoke_load_save = Some(parse_save_path_arg("--smoke-load-save", value)?);
            }
            "--smoke-visible-title-new-game" => {
                args.smoke_visible_title_new_game = true;
            }
            "--smoke-visible-title-name-input" => {
                args.smoke_visible_title_name_input = true;
            }
            "--smoke-visible-title-continue" => {
                let value = values
                    .next()
                    .context("--smoke-visible-title-continue requires a save path")?;
                args.smoke_visible_title_continue = Some(parse_save_path_arg(
                    "--smoke-visible-title-continue",
                    value,
                )?);
            }
            "--smoke-buttons" => {
                let value = values
                    .next()
                    .context("--smoke-buttons requires comma-separated buttons")?;
                args.smoke_buttons = parse_smoke_buttons(&value)?;
            }
            "--smoke-script" => {
                let value = values
                    .next()
                    .context("--smoke-script requires semicolon-separated input frames")?;
                args.smoke_script = parse_smoke_script(&value)?;
            }
            "--smoke-shop" => {
                let value = values
                    .next()
                    .context("--smoke-shop requires MapName:SourceScript:CommandIndex")?;
                args.smoke_shop = Some(parse_smoke_shop_ref(&value)?);
            }
            "--smoke-money" => {
                let value = values.next().context("--smoke-money requires an amount")?;
                args.smoke_money = Some(
                    value
                        .parse::<u32>()
                        .with_context(|| format!("smoke money '{value}' is not a u32"))?,
                );
            }
            "--smoke-buy" => {
                let value = values
                    .next()
                    .context("--smoke-buy requires ItemId:Quantity")?;
                args.smoke_buy = Some(parse_smoke_shop_transaction_ref("--smoke-buy", &value)?);
            }
            "--smoke-sell" => {
                let value = values
                    .next()
                    .context("--smoke-sell requires ItemId:Quantity")?;
                args.smoke_sell = Some(parse_smoke_shop_transaction_ref("--smoke-sell", &value)?);
            }
            "--smoke-field-item" => {
                let value = values
                    .next()
                    .context("--smoke-field-item requires Kind:ItemId:Quantity[:args]")?;
                args.smoke_field_item = Some(parse_smoke_field_item_ref(&value)?);
            }
            "--smoke-party-recovery" => {
                let value = values
                    .next()
                    .context("--smoke-party-recovery requires FullHeal or Blackout")?;
                args.smoke_party_recovery = Some(parse_smoke_party_recovery_kind(&value)?);
            }
            "--smoke-roamers" => {
                args.smoke_roamers = true;
            }
            "--smoke-magikarp-length" => {
                args.smoke_magikarp_length = true;
            }
            "--smoke-odd-egg" => {
                args.smoke_odd_egg = true;
            }
            "--smoke-mystery-gift" => {
                args.smoke_mystery_gift = true;
            }
            "--smoke-buena-password" => {
                args.smoke_buena_password = true;
            }
            "--smoke-shuckie" => {
                args.smoke_shuckie = true;
            }
            "--smoke-dratini" => {
                args.smoke_dratini = true;
            }
            "--smoke-kurt-apricorn" => {
                args.smoke_kurt_apricorn = true;
            }
            "--smoke-bills-grandfather" => {
                args.smoke_bills_grandfather = true;
            }
            "--smoke-unown-printer" => {
                args.smoke_unown_printer = true;
            }
            "--smoke-map-radio" => {
                args.smoke_map_radio = true;
            }
            "--smoke-name-rater" => {
                args.smoke_name_rater = true;
            }
            "--smoke-poke-seer" => {
                args.smoke_poke_seer = true;
            }
            "--smoke-bank-of-mom" => {
                args.smoke_bank_of_mom = true;
            }
            "--smoke-link-records" => {
                args.smoke_link_records = true;
            }
            "--smoke-link-rooms" => {
                args.smoke_link_rooms = true;
            }
            "--smoke-link-handshake" => {
                args.smoke_link_handshake = true;
            }
            "--smoke-battle-tower" => {
                args.smoke_battle_tower = true;
            }
            "--smoke-bug-contest" => {
                args.smoke_bug_contest = true;
            }
            "--smoke-day-care" => {
                args.smoke_day_care = true;
            }
            "--smoke-field-move" => {
                let value = values
                    .next()
                    .context("--smoke-field-move requires Kind:Species:Level:MoveId[:args]")?;
                args.smoke_field_move = Some(parse_smoke_field_move_ref(&value)?);
            }
            "--smoke-fishing" => {
                let value = values
                    .next()
                    .context("--smoke-fishing requires Rod:RodId or Item:ItemId")?;
                args.smoke_fishing = Some(parse_smoke_fishing_ref(&value)?);
            }
            "--smoke-menu" => {
                let value = values.next().context(
                    "--smoke-menu requires MapName:SourceScript:LoadmenuIndex:VerticalmenuIndex:OptionIndex:Option",
                )?;
                args.smoke_menu = Some(parse_smoke_menu_ref(&value)?);
            }
            "--smoke-interact" => {
                args.smoke_interact = true;
            }
            "--smoke-pc-storage" => {
                args.smoke_pc_storage = true;
            }
            "--smoke-pc-item" => {
                let value = values
                    .next()
                    .context("--smoke-pc-item requires ItemId:Quantity")?;
                args.smoke_pc_item =
                    Some(parse_smoke_shop_transaction_ref("--smoke-pc-item", &value)?);
            }
            "--smoke-link-journal" => {
                args.smoke_link_journal = true;
            }
            "--smoke-elevator" => {
                let value = values.next().context(
                    "--smoke-elevator requires MapName:DataLabel:SourceScript:CommandIndex:FloorIndex:Floor:Warp:TargetMap",
                )?;
                args.smoke_elevator = Some(parse_smoke_elevator_ref(&value)?);
            }
            "--smoke-elevfloor" => {
                let value = values.next().context(
                    "--smoke-elevfloor requires MapName:SourceScript:CommandIndex:TargetMap",
                )?;
                args.smoke_elevfloor =
                    Some(parse_smoke_elevfloor_ref("--smoke-elevfloor", &value)?);
            }
            "--smoke-script-warp" => {
                let value = values.next().context(
                    "--smoke-script-warp requires MapName:SourceScript:CommandIndex:TargetMap",
                )?;
                args.smoke_script_warp =
                    Some(parse_smoke_script_warp_ref("--smoke-script-warp", &value)?);
            }
            "--smoke-script-warp-pending" => {
                args.smoke_script_warp_pending = true;
            }
            "--smoke-script-map-pending" => {
                let value = values.next().context(
                    "--smoke-script-map-pending requires MapName:SourceScript:CommandIndex:Command",
                )?;
                args.smoke_script_map_pending = Some(parse_smoke_script_map_pending_ref(&value)?);
            }
            "--smoke-script-text-pending" => {
                let value = values.next().context(
                    "--smoke-script-text-pending requires MapName:SourceScript:OpenIndex:CommandIndex:Command[:accepted]",
                )?;
                args.smoke_script_text_pending = Some(parse_smoke_script_text_pending_ref(&value)?);
            }
            "--smoke-start-map" => {
                let value = values
                    .next()
                    .context("--smoke-start-map requires MapName:RuntimeTileX:RuntimeTileY")?;
                args.smoke_start_map = Some(parse_smoke_start_map_ref(&value)?);
            }
            "--smoke-set-flag" => {
                let value = values
                    .next()
                    .context("--smoke-set-flag requires a script or engine flag id")?;
                if value.is_empty() {
                    bail!("--smoke-set-flag flag id cannot be empty");
                }
                args.smoke_set_flags.push(value.to_string());
            }
            "--smoke-gift-pokemon" => {
                let value = values
                    .next()
                    .context("--smoke-gift-pokemon requires MapName:SourceScript:CommandIndex")?;
                args.smoke_gift_pokemon = Some(parse_smoke_script_command_ref(
                    "--smoke-gift-pokemon",
                    &value,
                )?);
            }
            "--smoke-wild-battle" => {
                let value = values
                    .next()
                    .context("--smoke-wild-battle requires MapName:SourceScript:CommandIndex")?;
                args.smoke_wild_battle = Some(parse_smoke_script_command_ref(
                    "--smoke-wild-battle",
                    &value,
                )?);
            }
            "--smoke-party" => {
                let value = values
                    .next()
                    .context("--smoke-party requires Species:Level[:HeldItem]")?;
                args.smoke_party
                    .push(parse_smoke_party_pokemon_ref(&value)?);
            }
            "--smoke-player-action" => {
                let value = values
                    .next()
                    .context("--smoke-player-action requires an action")?;
                args.smoke_player_action = Some(parse_smoke_battle_action(&value)?);
            }
            "--smoke-player-name" => {
                let value = values
                    .next()
                    .context("--smoke-player-name requires a non-empty name")?;
                if value.trim() != value || value.is_empty() {
                    bail!("--smoke-player-name must be an exact non-empty name");
                }
                args.smoke_player_name = Some(value);
            }
            "--smoke-enemy-action" => {
                let value = values
                    .next()
                    .context("--smoke-enemy-action requires an action")?;
                args.smoke_enemy_action = Some(parse_smoke_battle_action(&value)?);
            }
            "--smoke-capture-ball" => {
                let value = values
                    .next()
                    .context("--smoke-capture-ball requires an item id")?;
                if value.is_empty() {
                    bail!("--smoke-capture-ball item id cannot be empty");
                }
                args.smoke_capture_ball = Some(value);
            }
            "--smoke-battle-item" => {
                let value = values
                    .next()
                    .context("--smoke-battle-item requires an item id")?;
                if value.is_empty() {
                    bail!("--smoke-battle-item item id cannot be empty");
                }
                args.smoke_battle_item = Some(value);
            }
            "--smoke-trainer-battle" => {
                let value = values
                    .next()
                    .context("--smoke-trainer-battle requires MapName:SourceScript:CommandIndex")?;
                args.smoke_trainer_battle = Some(parse_smoke_script_command_ref(
                    "--smoke-trainer-battle",
                    &value,
                )?);
            }
            "--smoke-visible-start-menu" => {
                let value = values
                    .next()
                    .context("--smoke-visible-start-menu requires a save path")?;
                args.smoke_visible_start_menu =
                    Some(parse_save_path_arg("--smoke-visible-start-menu", value)?);
            }
            "--smoke-visible-bag-item" => {
                let value = values
                    .next()
                    .context("--smoke-visible-bag-item requires ItemId:Quantity")?;
                args.smoke_visible_bag_item
                    .push(parse_smoke_shop_transaction_ref(
                        "--smoke-visible-bag-item",
                        &value,
                    )?);
            }
            "--smoke-visible-party" => {
                args.smoke_visible_party = true;
            }
            "--smoke-visible-overworld" => {
                args.smoke_visible_overworld = true;
            }
            "--smoke-visible-wild-battle" => {
                let value = values.next().context(
                    "--smoke-visible-wild-battle requires MapName:SourceScript:CommandIndex",
                )?;
                args.smoke_visible_wild_battle = Some(parse_smoke_script_command_ref(
                    "--smoke-visible-wild-battle",
                    &value,
                )?);
            }
            "--smoke-visible-trainer-battle" => {
                let value = values.next().context(
                    "--smoke-visible-trainer-battle requires MapName:SourceScript:CommandIndex",
                )?;
                args.smoke_visible_trainer_battle = Some(parse_smoke_script_command_ref(
                    "--smoke-visible-trainer-battle",
                    &value,
                )?);
            }
            other => bail!("unknown argument '{other}'"),
        }
    }
    Ok(args)
}

fn parse_cli_path_arg(flag: &str, value: String) -> Result<PathBuf> {
    let path = PathBuf::from(&value);
    if path.as_os_str().is_empty() {
        bail!("{flag} cannot be empty");
    }
    Ok(path)
}

fn parse_pack_arg(flag: &str, value: String) -> Result<String> {
    let path = parse_cli_path_arg(flag, value)?;
    require_extension(flag, &path, COMPILED_GAME_PACK_EXTENSION)?;
    path.into_os_string()
        .into_string()
        .map_err(|_| anyhow::anyhow!("{flag} must be UTF-8"))
}

fn parse_save_path_arg(flag: &str, value: String) -> Result<PathBuf> {
    let path = parse_cli_path_arg(flag, value)?;
    require_extension(flag, &path, SAVE_EXTENSION)?;
    Ok(path)
}

fn parse_smoke_buttons(value: &str) -> Result<Vec<GameButton>> {
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }
    value
        .split(',')
        .map(|part| match part {
            "a" | "A" => Ok(GameButton::A),
            "b" | "B" => Ok(GameButton::B),
            "start" | "Start" => Ok(GameButton::Start),
            "select" | "Select" => Ok(GameButton::Select),
            "right" | "Right" => Ok(GameButton::Right),
            "left" | "Left" => Ok(GameButton::Left),
            "up" | "Up" => Ok(GameButton::Up),
            "down" | "Down" => Ok(GameButton::Down),
            other => bail!("unknown smoke button '{other}'"),
        })
        .collect()
}

fn parse_smoke_script(value: &str) -> Result<Vec<Vec<GameButton>>> {
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut frames = Vec::new();
    for (index, frame_spec) in value.split(';').enumerate() {
        let (frame, repeat_count) = parse_smoke_frame_spec(frame_spec)
            .with_context(|| format!("invalid --smoke-script frame {}", index + 1))?;
        frames.extend(std::iter::repeat_n(frame, repeat_count));
    }
    Ok(frames)
}

fn parse_smoke_frame_spec(value: &str) -> Result<(Vec<GameButton>, usize)> {
    if value.is_empty() {
        bail!("smoke script frames cannot be empty");
    }
    let Some((buttons, repeat_count)) = value.rsplit_once('*') else {
        return Ok((parse_smoke_buttons(value)?, 1));
    };
    if buttons.is_empty() {
        bail!("smoke frame repeat requires buttons before '*'");
    }
    let repeat_count = repeat_count
        .parse::<usize>()
        .with_context(|| format!("smoke frame repeat '{repeat_count}' is not a usize"))?;
    if repeat_count == 0 {
        bail!("smoke frame repeat count must be greater than zero");
    }
    Ok((parse_smoke_buttons(buttons)?, repeat_count))
}

fn parse_smoke_shop_ref(value: &str) -> Result<SmokeShopRef> {
    let command = parse_smoke_script_command_ref("--smoke-shop", value)?;
    Ok(SmokeShopRef {
        map_name: command.map_name,
        source_script: command.source_script,
        command_index: command.command_index,
    })
}

fn parse_smoke_script_command_ref(flag: &str, value: &str) -> Result<SmokeScriptCommandRef> {
    let parts = value.split(':').collect::<Vec<_>>();
    let [map_name, source_script, command_index] = parts.as_slice() else {
        bail!("{flag} must be MapName:SourceScript:CommandIndex");
    };
    if map_name.is_empty() || source_script.is_empty() {
        bail!("{flag} map and source script cannot be empty");
    }
    let command_index = command_index
        .parse::<usize>()
        .with_context(|| format!("{flag} command index '{command_index}' is not a usize"))?;
    Ok(SmokeScriptCommandRef {
        map_name: (*map_name).to_string(),
        source_script: (*source_script).to_string(),
        command_index,
    })
}

fn parse_smoke_shop_transaction_ref(flag: &str, value: &str) -> Result<SmokeShopTransactionRef> {
    let parts = value.split(':').collect::<Vec<_>>();
    let [item_id, quantity] = parts.as_slice() else {
        bail!("{flag} must be ItemId:Quantity");
    };
    if item_id.is_empty() {
        bail!("{flag} item id cannot be empty");
    }
    let quantity = quantity
        .parse::<u16>()
        .with_context(|| format!("{flag} quantity '{quantity}' is not a u16"))?;
    if quantity == 0 {
        bail!("{flag} quantity must be greater than zero");
    }
    Ok(SmokeShopTransactionRef {
        item_id: (*item_id).to_string(),
        quantity,
    })
}

fn parse_smoke_field_item_ref(value: &str) -> Result<SmokeFieldItemRef> {
    let parts = value.split(':').collect::<Vec<_>>();
    if parts.len() < 3 {
        bail!("--smoke-field-item must be Kind:ItemId:Quantity[:args]");
    }
    let kind = parts[0];
    let item_id = parts[1];
    let quantity = parts[2];
    let kind = match kind {
        "Repel" => {
            require_smoke_field_item_arg_count(&parts, 3)?;
            SmokeFieldItemKind::Repel
        }
        "Bicycle" => {
            require_smoke_field_item_arg_count(&parts, 3)?;
            SmokeFieldItemKind::Bicycle
        }
        "Itemfinder" => {
            require_smoke_field_item_arg_count(&parts, 3)?;
            SmokeFieldItemKind::Itemfinder
        }
        "TownMap" => {
            require_smoke_field_item_arg_count(&parts, 3)?;
            SmokeFieldItemKind::TownMap
        }
        "Register" => {
            require_smoke_field_item_arg_count(&parts, 3)?;
            SmokeFieldItemKind::Register
        }
        "Pokegear" => {
            require_smoke_field_item_arg_count(&parts, 3)?;
            SmokeFieldItemKind::Pokegear
        }
        "Box" => {
            require_smoke_field_item_arg_count(&parts, 3)?;
            SmokeFieldItemKind::Box
        }
        "EscapeRope" => {
            require_smoke_field_item_arg_count(&parts, 3)?;
            SmokeFieldItemKind::EscapeRope
        }
        "TmHm" => {
            if parts.len() != 5 && parts.len() != 6 {
                bail!(
                    "--smoke-field-item TmHm expects Kind:ItemId:Quantity:Species:Level[:ReplaceSlot]"
                );
            }
            let species_id = parts[3].to_string();
            if species_id.is_empty() {
                bail!("--smoke-field-item TmHm species id cannot be empty");
            }
            let level = parts[4].parse::<u8>().with_context(|| {
                format!("--smoke-field-item TmHm level '{}' is not a u8", parts[4])
            })?;
            if level == 0 {
                bail!("--smoke-field-item TmHm level must be greater than zero");
            }
            let replace_slot = if parts.len() == 6 {
                Some(parts[5].parse::<usize>().with_context(|| {
                    format!(
                        "--smoke-field-item TmHm replace slot '{}' is not a usize",
                        parts[5]
                    )
                })?)
            } else {
                None
            };
            SmokeFieldItemKind::TmHm {
                species_id,
                level,
                replace_slot,
            }
        }
        "EvolutionStone" => {
            require_smoke_field_item_arg_count(&parts, 6)?;
            let species_id = parts[3].to_string();
            let expected_species_id = parts[5].to_string();
            if species_id.is_empty() || expected_species_id.is_empty() {
                bail!(
                    "--smoke-field-item EvolutionStone species and expected species cannot be empty"
                );
            }
            let level = parts[4].parse::<u8>().with_context(|| {
                format!(
                    "--smoke-field-item EvolutionStone level '{}' is not a u8",
                    parts[4]
                )
            })?;
            if level == 0 {
                bail!("--smoke-field-item EvolutionStone level must be greater than zero");
            }
            SmokeFieldItemKind::EvolutionStone {
                species_id,
                level,
                expected_species_id,
            }
        }
        other => bail!("unknown --smoke-field-item kind '{other}'"),
    };
    if item_id.is_empty() {
        bail!("--smoke-field-item item id cannot be empty");
    }
    let quantity = quantity
        .parse::<u16>()
        .with_context(|| format!("--smoke-field-item quantity '{quantity}' is not a u16"))?;
    if quantity == 0 {
        bail!("--smoke-field-item quantity must be greater than zero");
    }
    Ok(SmokeFieldItemRef {
        kind,
        item_id: (*item_id).to_string(),
        quantity,
    })
}

fn parse_smoke_party_recovery_kind(value: &str) -> Result<SmokePartyRecoveryKind> {
    match value {
        "FullHeal" => Ok(SmokePartyRecoveryKind::FullHeal),
        "Blackout" => Ok(SmokePartyRecoveryKind::Blackout),
        other => bail!("unknown --smoke-party-recovery kind '{other}'"),
    }
}

fn require_smoke_field_item_arg_count(parts: &[&str], expected: usize) -> Result<()> {
    if parts.len() != expected {
        bail!(
            "--smoke-field-item {} expects {} colon-separated fields, found {}",
            parts.first().copied().unwrap_or(""),
            expected,
            parts.len()
        );
    }
    Ok(())
}

fn parse_smoke_field_move_ref(value: &str) -> Result<SmokeFieldMoveRef> {
    let parts = value.split(':').collect::<Vec<_>>();
    if parts.len() < 4 {
        bail!("--smoke-field-move must be Kind:Species:Level:MoveId[:args]");
    }
    let species_id = parts[1].to_string();
    if species_id.is_empty() {
        bail!("--smoke-field-move species id cannot be empty");
    }
    let level = parts[2]
        .parse::<u8>()
        .with_context(|| format!("--smoke-field-move level '{}' is not a u8", parts[2]))?;
    if level == 0 {
        bail!("--smoke-field-move level must be greater than zero");
    }
    let move_id = parts[3].to_string();
    if move_id.is_empty() {
        bail!("--smoke-field-move move id cannot be empty");
    }
    let kind = match parts[0] {
        "Cut" => {
            require_smoke_field_move_arg_count(&parts, 6)?;
            SmokeFieldMoveKind::Cut {
                metatile_x: parse_u16_arg("--smoke-field-move Cut metatile_x", parts[4])?,
                metatile_y: parse_u16_arg("--smoke-field-move Cut metatile_y", parts[5])?,
            }
        }
        "Whirlpool" => {
            require_smoke_field_move_arg_count(&parts, 6)?;
            SmokeFieldMoveKind::Whirlpool {
                metatile_x: parse_u16_arg("--smoke-field-move Whirlpool metatile_x", parts[4])?,
                metatile_y: parse_u16_arg("--smoke-field-move Whirlpool metatile_y", parts[5])?,
            }
        }
        "Strength" => {
            require_smoke_field_move_arg_count(&parts, 4)?;
            SmokeFieldMoveKind::Strength
        }
        "Flash" => {
            require_smoke_field_move_arg_count(&parts, 4)?;
            SmokeFieldMoveKind::Flash
        }
        "Surf" => {
            require_smoke_field_move_arg_count(&parts, 4)?;
            SmokeFieldMoveKind::Surf
        }
        "Waterfall" => {
            require_smoke_field_move_arg_count(&parts, 4)?;
            SmokeFieldMoveKind::Waterfall
        }
        "Fly" => {
            require_smoke_field_move_arg_count(&parts, 6)?;
            let flypoint_flag = parts[5].to_string();
            if flypoint_flag.is_empty() {
                bail!("--smoke-field-move Fly flypoint flag cannot be empty");
            }
            SmokeFieldMoveKind::Fly {
                destination_spawn_identifier: parse_u16_arg(
                    "--smoke-field-move Fly destination spawn",
                    parts[4],
                )?,
                flypoint_flag,
            }
        }
        "Dig" => {
            require_smoke_field_move_arg_count(&parts, 4)?;
            SmokeFieldMoveKind::Dig
        }
        "Teleport" => {
            require_smoke_field_move_arg_count(&parts, 4)?;
            SmokeFieldMoveKind::Teleport
        }
        "Headbutt" => {
            require_smoke_field_move_arg_count(&parts, 5)?;
            SmokeFieldMoveKind::Headbutt {
                player_id: parse_u16_arg("--smoke-field-move Headbutt player_id", parts[4])?,
            }
        }
        "RockSmash" => {
            require_smoke_field_move_arg_count(&parts, 4)?;
            SmokeFieldMoveKind::RockSmash
        }
        "SweetScent" => {
            require_smoke_field_move_arg_count(&parts, 5)?;
            SmokeFieldMoveKind::SweetScent {
                surface: parse_smoke_encounter_surface(parts[4])?,
            }
        }
        other => bail!("unknown --smoke-field-move kind '{other}'"),
    };
    Ok(SmokeFieldMoveRef {
        kind,
        species_id,
        level,
        move_id,
    })
}

fn require_smoke_field_move_arg_count(parts: &[&str], expected: usize) -> Result<()> {
    if parts.len() != expected {
        bail!(
            "--smoke-field-move {} expects {} colon-separated fields, found {}",
            parts[0],
            expected,
            parts.len()
        );
    }
    Ok(())
}

fn parse_u16_arg(label: &str, value: &str) -> Result<u16> {
    value
        .parse::<u16>()
        .with_context(|| format!("{label} '{value}' is not a u16"))
}

fn parse_smoke_encounter_surface(value: &str) -> Result<EncounterSurface> {
    match value {
        "Grass" => Ok(EncounterSurface::Grass),
        "Water" => Ok(EncounterSurface::Water),
        "Rock" => Ok(EncounterSurface::Rock),
        other => bail!("unknown encounter surface '{other}'"),
    }
}

fn parse_smoke_fishing_ref(value: &str) -> Result<SmokeFishingRef> {
    let parts = value.split(':').collect::<Vec<_>>();
    if parts.len() != 2 {
        bail!("--smoke-fishing must be Rod:RodId or Item:ItemId");
    }
    let kind = match parts[0] {
        "Rod" => SmokeFishingKind::Rod,
        "Item" => SmokeFishingKind::Item,
        other => bail!("unknown --smoke-fishing kind '{other}'"),
    };
    let id = parts[1].to_string();
    if id.is_empty() {
        bail!("--smoke-fishing id cannot be empty");
    }
    Ok(SmokeFishingRef { kind, id })
}

fn parse_smoke_menu_ref(value: &str) -> Result<SmokeMenuRef> {
    let parts = value.split(':').collect::<Vec<_>>();
    let [
        map_name,
        source_script,
        loadmenu_command_index,
        verticalmenu_command_index,
        option_index,
        option,
    ] = parts.as_slice()
    else {
        bail!(
            "--smoke-menu must be MapName:SourceScript:LoadmenuIndex:VerticalmenuIndex:OptionIndex:Option"
        );
    };
    if map_name.is_empty() || source_script.is_empty() || option.is_empty() {
        bail!("--smoke-menu map, source script, and option cannot be empty");
    }
    Ok(SmokeMenuRef {
        map_name: (*map_name).to_string(),
        source_script: (*source_script).to_string(),
        loadmenu_command_index: loadmenu_command_index.parse::<usize>().with_context(|| {
            format!("--smoke-menu loadmenu index '{loadmenu_command_index}' is not a usize")
        })?,
        verticalmenu_command_index: verticalmenu_command_index.parse::<usize>().with_context(
            || {
                format!(
                    "--smoke-menu verticalmenu index '{verticalmenu_command_index}' is not a usize"
                )
            },
        )?,
        option_index: option_index.parse::<usize>().with_context(|| {
            format!("--smoke-menu option index '{option_index}' is not a usize")
        })?,
        option: (*option).to_string(),
    })
}

fn parse_smoke_elevator_ref(value: &str) -> Result<SmokeElevatorRef> {
    let parts = value.split(':').collect::<Vec<_>>();
    let [
        map_name,
        data_label,
        source_script,
        elevator_command_index,
        floor_index,
        floor,
        warp,
        target_map,
    ] = parts.as_slice()
    else {
        bail!(
            "--smoke-elevator must be MapName:DataLabel:SourceScript:CommandIndex:FloorIndex:Floor:Warp:TargetMap"
        );
    };
    if map_name.is_empty()
        || data_label.is_empty()
        || source_script.is_empty()
        || floor.is_empty()
        || target_map.is_empty()
    {
        bail!(
            "--smoke-elevator map, data label, source script, floor, and target map cannot be empty"
        );
    }
    Ok(SmokeElevatorRef {
        map_name: (*map_name).to_string(),
        data_label: (*data_label).to_string(),
        source_script: (*source_script).to_string(),
        elevator_command_index: elevator_command_index.parse::<usize>().with_context(|| {
            format!("--smoke-elevator command index '{elevator_command_index}' is not a usize")
        })?,
        floor_index: floor_index.parse::<usize>().with_context(|| {
            format!("--smoke-elevator floor index '{floor_index}' is not a usize")
        })?,
        floor: (*floor).to_string(),
        warp: warp
            .parse::<u16>()
            .with_context(|| format!("--smoke-elevator warp '{warp}' is not a u16"))?,
        target_map: (*target_map).to_string(),
    })
}

fn parse_smoke_elevfloor_ref(flag: &str, value: &str) -> Result<SmokeElevfloorRef> {
    let command = parse_smoke_script_command_ref(
        flag,
        value.rsplit_once(':').map_or(value, |(prefix, _)| prefix),
    )?;
    let Some((_, target_map)) = value.rsplit_once(':') else {
        bail!("{flag} must be MapName:SourceScript:CommandIndex:TargetMap");
    };
    if target_map.is_empty() {
        bail!("{flag} target map cannot be empty");
    }
    Ok(SmokeElevfloorRef {
        map_name: command.map_name,
        source_script: command.source_script,
        command_index: command.command_index,
        target_map: target_map.to_string(),
    })
}

fn parse_smoke_script_warp_ref(flag: &str, value: &str) -> Result<SmokeScriptWarpRef> {
    let command = parse_smoke_script_command_ref(
        flag,
        value.rsplit_once(':').map_or(value, |(prefix, _)| prefix),
    )?;
    let Some((_, target_map)) = value.rsplit_once(':') else {
        bail!("{flag} must be MapName:SourceScript:CommandIndex:TargetMap");
    };
    if target_map.is_empty() {
        bail!("{flag} target map cannot be empty");
    }
    Ok(SmokeScriptWarpRef {
        map_name: command.map_name,
        source_script: command.source_script,
        command_index: command.command_index,
        target_map: target_map.to_string(),
    })
}

fn parse_smoke_script_map_pending_ref(value: &str) -> Result<SmokeScriptMapPendingRef> {
    let command_ref = parse_smoke_script_command_ref(
        "--smoke-script-map-pending",
        value.rsplit_once(':').map_or(value, |(prefix, _)| prefix),
    )?;
    let Some((_, command)) = value.rsplit_once(':') else {
        bail!("--smoke-script-map-pending must be MapName:SourceScript:CommandIndex:Command");
    };
    if command.is_empty() {
        bail!("--smoke-script-map-pending command cannot be empty");
    }
    Ok(SmokeScriptMapPendingRef {
        map_name: command_ref.map_name,
        source_script: command_ref.source_script,
        command_index: command_ref.command_index,
        command: command.to_string(),
    })
}

fn parse_smoke_script_text_pending_ref(value: &str) -> Result<SmokeScriptTextPendingRef> {
    let parts = value.split(':').collect::<Vec<_>>();
    if !(parts.len() == 5 || parts.len() == 6) {
        bail!(
            "--smoke-script-text-pending must be MapName:SourceScript:OpenIndex:CommandIndex:Command[:accepted]"
        );
    }
    let [
        map_name,
        source_script,
        open_command_index,
        command_index,
        command,
        rest @ ..,
    ] = parts.as_slice()
    else {
        unreachable!();
    };
    if map_name.is_empty() || source_script.is_empty() {
        bail!("--smoke-script-text-pending map and source script cannot be empty");
    }
    let open_command_index = open_command_index.parse::<usize>().with_context(|| {
        format!("--smoke-script-text-pending open index '{open_command_index}' is not a usize")
    })?;
    let command_index = command_index.parse::<usize>().with_context(|| {
        format!("--smoke-script-text-pending command index '{command_index}' is not a usize")
    })?;
    if command.is_empty() {
        bail!("--smoke-script-text-pending command cannot be empty");
    }
    let accepted = if let Some(value) = rest.first() {
        match *value {
            "true" | "True" => true,
            "false" | "False" => false,
            other => {
                bail!("--smoke-script-text-pending accepted must be true or false, got {other}")
            }
        }
    } else {
        true
    };
    Ok(SmokeScriptTextPendingRef {
        map_name: (*map_name).to_string(),
        source_script: (*source_script).to_string(),
        open_command_index,
        command_index,
        command: command.to_string(),
        accepted,
    })
}

fn parse_smoke_start_map_ref(value: &str) -> Result<SmokeStartMapRef> {
    let parts = value.split(':').collect::<Vec<_>>();
    let [map_name, tile_x, tile_y] = parts.as_slice() else {
        bail!("--smoke-start-map must be MapName:RuntimeTileX:RuntimeTileY");
    };
    if map_name.is_empty() {
        bail!("--smoke-start-map map name cannot be empty");
    }
    Ok(SmokeStartMapRef {
        map_name: (*map_name).to_string(),
        tile_x: tile_x.parse::<i16>().with_context(|| {
            format!("--smoke-start-map runtime tile x '{tile_x}' is not an i16")
        })?,
        tile_y: tile_y.parse::<i16>().with_context(|| {
            format!("--smoke-start-map runtime tile y '{tile_y}' is not an i16")
        })?,
    })
}

fn bevy_shell_start_from_smoke_start_map(
    spawn_identifier: u16,
    smoke_start_map: Option<&SmokeStartMapRef>,
) -> BevyShellStart {
    if let Some(start_map) = smoke_start_map {
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name: start_map.map_name.clone(),
            tile_x: start_map.tile_x,
            tile_y: start_map.tile_y,
        }
    } else {
        BevyShellStart::NewGame { spawn_identifier }
    }
}

fn parse_smoke_party_pokemon_ref(value: &str) -> Result<SmokePartyPokemonRef> {
    let parts = value.split(':').collect::<Vec<_>>();
    if !(parts.len() == 2 || parts.len() == 3) {
        bail!("--smoke-party must be Species:Level[:HeldItem]");
    }
    let species_id = parts[0];
    let level = parts[1];
    if species_id.is_empty() {
        bail!("--smoke-party species id cannot be empty");
    }
    let level = level
        .parse::<u8>()
        .with_context(|| format!("--smoke-party level '{level}' is not a u8"))?;
    if level == 0 {
        bail!("--smoke-party level must be greater than zero");
    }
    let held_item_id = match parts.as_slice() {
        [_, _] => None,
        [_, _, held_item_id] if held_item_id.is_empty() => {
            bail!("--smoke-party held item id cannot be empty")
        }
        [_, _, held_item_id] => Some((*held_item_id).to_string()),
        _ => unreachable!(),
    };
    Ok(SmokePartyPokemonRef {
        species_id: species_id.to_string(),
        level,
        held_item_id,
    })
}

fn parse_smoke_battle_action(value: &str) -> Result<BattleAction> {
    if value == "Run" {
        return Ok(BattleAction::Run);
    }
    if let Some(slot) = value.strip_prefix("Move:") {
        let slot = slot
            .parse::<usize>()
            .with_context(|| format!("battle move slot '{slot}' is not a usize"))?;
        if slot >= 4 {
            bail!("battle move slot {slot} is outside Crystal move range 0..3");
        }
        return Ok(BattleAction::Move { slot });
    }
    if let Some(payload) = value.strip_prefix("MoveSwitch:") {
        let (slot, party_index) = payload
            .split_once(':')
            .with_context(|| "battle move switch action must be MoveSwitch:SLOT:PARTY_INDEX")?;
        let slot = slot
            .parse::<usize>()
            .with_context(|| format!("battle move switch slot '{slot}' is not a usize"))?;
        if slot >= 4 {
            bail!("battle move switch slot {slot} is outside Crystal move range 0..3");
        }
        let party_index = party_index.parse::<usize>().with_context(|| {
            format!("battle move switch party index '{party_index}' is not a usize")
        })?;
        if party_index >= PARTY_SIZE {
            bail!("battle move switch party index {party_index} is outside party range");
        }
        return Ok(BattleAction::MoveSwitch { slot, party_index });
    }
    if let Some(party_index) = value.strip_prefix("Switch:") {
        let party_index = party_index
            .parse::<usize>()
            .with_context(|| format!("battle switch party index '{party_index}' is not a usize"))?;
        if party_index >= PARTY_SIZE {
            bail!("battle switch party index {party_index} is outside party range");
        }
        return Ok(BattleAction::Switch { party_index });
    }
    if let Some(item_id) = value.strip_prefix("Item:") {
        if item_id.is_empty() {
            bail!("battle item id cannot be empty");
        }
        return Ok(BattleAction::Item {
            item_id: item_id.to_string(),
        });
    }
    bail!("battle action must be Run, Move:N, MoveSwitch:SLOT:PARTY_INDEX, Switch:N, or Item:ID")
}

fn grant_smoke_party(
    shell: &mut RuntimeGameShell,
    party: &[SmokePartyPokemonRef],
    context: &str,
) -> Result<Vec<RuntimeGiftPokemonGrant>> {
    let mut grants = Vec::with_capacity(party.len());
    for (index, pokemon) in party.iter().enumerate() {
        let grant = shell
            .add_party_pokemon(
                &pokemon.species_id,
                pokemon.level,
                pokemon.held_item_id.clone(),
                None,
                "SMOKE",
                u16::try_from(index + 1).context("smoke party index overflow")?,
                Dv::from_non_hp(10, 10, 10, 10),
            )
            .with_context(|| {
                format!(
                    "add {context} party Pokemon {} level {} at smoke slot {}",
                    pokemon.species_id, pokemon.level, index
                )
            })?;
        grants.push(grant);
    }
    Ok(grants)
}

fn require_extension(flag: &str, path: &PathBuf, expected: &str) -> Result<()> {
    let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
        bail!("{flag} path {} must use .{expected}", path.display());
    };
    if extension != expected {
        bail!(
            "{flag} path {} must use .{expected}, got .{extension}",
            path.display()
        );
    }
    Ok(())
}

fn print_usage() {
    println!(
        "usage: crystal-bevy [--repo <repo-root>] [--pack <assets/data relative .crystalpack>] [--save-path <path>]"
    );
    println!("       defaults: --repo . --pack {DEFAULT_PACK_PATH}");
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--save-path <path>]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --load-save <path> [--save-path <path>]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --list-spawns"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --list-script-map-commands"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --list-script-scene-commands"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --list-script-battles"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --list-gift-pokemon"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --list-map-objects MapName"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --list-map-events MapName"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --list-field-targets MapName"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-save <path> [--smoke-buttons right,a] [--smoke-script 'right*8;down;down']"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --smoke-title-new-game <path> --smoke-player-name <name>"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --smoke-load-save <path> [--save-path <roundtrip-path>]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --smoke-visible-title-new-game [--save-path <path>]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --smoke-visible-title-name-input --smoke-player-name <name> [--save-path <path>]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --smoke-visible-title-continue <path>"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-visible-start-menu <path> --smoke-player-name <name> --smoke-party Species:Level[:HeldItem] --smoke-visible-bag-item ItemId:Quantity"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-visible-party --smoke-player-name <name> --smoke-party Species:Level[:HeldItem] --smoke-party Species:Level[:HeldItem]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-visible-overworld --smoke-player-name <name> --smoke-script 'right*8;down;a'"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-visible-wild-battle MapName:SourceScript:CommandIndex --smoke-player-name <name> --smoke-party Species:Level[:HeldItem] [--smoke-visible-bag-item ItemId:Quantity]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-visible-trainer-battle MapName:SourceScript:CommandIndex --smoke-player-name <name> --smoke-party Species:Level[:HeldItem]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-shop MapName:SourceScript:CommandIndex"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-shop MapName:SourceScript:CommandIndex [--smoke-money 1000] [--smoke-buy POTION:1] [--smoke-sell POTION:1] [--smoke-save /tmp/shop.crystalsave]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-field-item Repel:REPEL:1 [--smoke-save /tmp/field-item.crystalsave]"
    );
    eprintln!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-field-item TmHm:HM_CUT:1:CHIKORITA:5 [--smoke-save /tmp/tmhm.crystalsave]"
    );
    eprintln!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-field-item EvolutionStone:THUNDERSTONE:1:PIKACHU:20:RAICHU [--smoke-save /tmp/evolution.crystalsave]"
    );
    eprintln!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-field-item Register:BICYCLE:1 [--smoke-save /tmp/register-key-item.crystalsave]"
    );
    eprintln!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-party-recovery FullHeal|Blackout --smoke-party Species:Level[:HeldItem] [--smoke-save /tmp/party-recovery.crystalsave]"
    );
    eprintln!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-roamers [--smoke-save /tmp/roamers.crystalsave]"
    );
    eprintln!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-magikarp-length [--smoke-save /tmp/magikarp-length.crystalsave]"
    );
    eprintln!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-odd-egg [--smoke-save /tmp/odd-egg.crystalsave]"
    );
    eprintln!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-mystery-gift [--smoke-save /tmp/mystery-gift.crystalsave]"
    );
    eprintln!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-buena-password [--smoke-save /tmp/buena-password.crystalsave]"
    );
    eprintln!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-shuckie [--smoke-save /tmp/shuckie.crystalsave]"
    );
    eprintln!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-dratini [--smoke-save /tmp/dratini.crystalsave]"
    );
    eprintln!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-kurt-apricorn [--smoke-save /tmp/kurt-apricorn.crystalsave]"
    );
    eprintln!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-bills-grandfather [--smoke-save /tmp/bills-grandfather.crystalsave]"
    );
    eprintln!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-unown-printer [--smoke-save /tmp/unown-printer.crystalsave]"
    );
    eprintln!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-map-radio [--smoke-save /tmp/map-radio.crystalsave]"
    );
    eprintln!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-name-rater [--smoke-save /tmp/name-rater.crystalsave]"
    );
    eprintln!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-poke-seer [--smoke-save /tmp/poke-seer.crystalsave]"
    );
    eprintln!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-bank-of-mom [--smoke-save /tmp/bank-of-mom.crystalsave]"
    );
    eprintln!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-link-records [--smoke-save /tmp/link-records.crystalsave]"
    );
    eprintln!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-link-rooms [--smoke-save /tmp/link-rooms.crystalsave]"
    );
    eprintln!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-link-handshake [--smoke-save /tmp/link-handshake.crystalsave]"
    );
    eprintln!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-battle-tower --smoke-party Species:Level[:HeldItem] --smoke-party Species:Level[:HeldItem] --smoke-party Species:Level[:HeldItem] [--smoke-save /tmp/battle-tower.crystalsave]"
    );
    eprintln!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-bug-contest --smoke-party Species:Level[:HeldItem] --smoke-party Species:Level[:HeldItem] [--smoke-save /tmp/bug-contest.crystalsave]"
    );
    eprintln!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-day-care --smoke-party Species:Level[:HeldItem] --smoke-party Species:Level[:HeldItem] [--smoke-save /tmp/day-care.crystalsave]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] [--smoke-set-flag FLAG_ID] --smoke-field-move Kind:Species:Level:MoveId[:args] [--smoke-save /tmp/field-move.crystalsave]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-gift-pokemon MapName:SourceScript:CommandIndex [--smoke-save /tmp/gift.crystalsave]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-fishing Rod:GOOD_ROD|Item:GOOD_ROD [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-party Species:Level[:HeldItem] [--smoke-save /tmp/fishing.crystalsave]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-menu MapName:SourceScript:LoadmenuIndex:VerticalmenuIndex:OptionIndex:Option [--smoke-save /tmp/menu.crystalsave]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-interact --smoke-player-name <name> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] [--smoke-script 'right;up;a'] [--smoke-save /tmp/interact.crystalsave]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-pc-storage --smoke-party Species:Level[:HeldItem] --smoke-party Species:Level[:HeldItem] --smoke-pc-item ItemId:Quantity [--smoke-save /tmp/pc-storage.crystalsave]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-link-journal --smoke-script 'right;down;left;up'"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-elevator MapName:DataLabel:SourceScript:CommandIndex:FloorIndex:Floor:Warp:TargetMap [--smoke-save /tmp/elevator.crystalsave]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-elevfloor MapName:SourceScript:CommandIndex:TargetMap [--smoke-save /tmp/elevfloor.crystalsave]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-start-map MapName:RuntimeTileX:RuntimeTileY --smoke-elevfloor MapName:SourceScript:CommandIndex:TargetMap [--smoke-save /tmp/elevfloor.crystalsave]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-start-map MapName:RuntimeTileX:RuntimeTileY --smoke-script-warp MapName:SourceScript:CommandIndex:TargetMap [--smoke-script-warp-pending] [--smoke-save /tmp/script-warp.crystalsave]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-start-map MapName:RuntimeTileX:RuntimeTileY --smoke-script-map-pending MapName:SourceScript:CommandIndex:Command [--smoke-save /tmp/script-map.crystalsave]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-start-map MapName:RuntimeTileX:RuntimeTileY --smoke-script-text-pending MapName:SourceScript:OpenIndex:CommandIndex:Command[:accepted] [--smoke-save /tmp/script-text.crystalsave]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-wild-battle MapName:SourceScript:CommandIndex --smoke-party Species:Level[:HeldItem] [--smoke-player-action Move:0 --smoke-enemy-action Move:0 | --smoke-capture-ball MASTER_BALL | --smoke-battle-item X_ATTACK] [--smoke-save /tmp/battle.crystalsave]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-trainer-battle MapName:SourceScript:CommandIndex --smoke-party Species:Level[:HeldItem] [--smoke-save /tmp/trainer.crystalsave]"
    );
    println!(
        "example: cargo run -p crystal-bevy -- --repo /path/to/crystal-llm --pack <compiled-pack>.crystalpack --list-spawns"
    );
}

fn smoke_script_shop(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    shop_ref: &SmokeShopRef,
    smoke_start_map: Option<&SmokeStartMapRef>,
    smoke_buttons: &[GameButton],
    smoke_script: &[Vec<GameButton>],
    smoke_money: Option<u32>,
    smoke_buy: Option<&SmokeShopTransactionRef>,
    smoke_sell: Option<&SmokeShopTransactionRef>,
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    let mut shell = if let Some(start) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            &start.map_name,
            start.tile_x,
            start.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    if let Some(amount) = smoke_money {
        shell.apply_runtime_mutation_command(RuntimeMutationCommand::AddCurrency(
            RuntimeCurrencyDeltaCommand {
                account: RuntimeCurrencyAccount::Money,
                amount,
            },
        ))?;
    }
    let input_frames = smoke_input_frames(smoke_buttons, smoke_script);
    let mut last_frame = None;
    for (index, buttons) in input_frames.iter().enumerate() {
        last_frame = Some(
            shell
                .tick(buttons.iter().copied())
                .with_context(|| format!("advance runtime shell input frame {}", index + 1))?
                .clone(),
        );
    }
    let shop = shell
        .open_script_shop(
            &shop_ref.map_name,
            &shop_ref.source_script,
            shop_ref.command_index,
        )
        .with_context(|| {
            let current = last_frame
                .as_ref()
                .map(|frame| frame.snapshot.map_name.as_str())
                .unwrap_or("<initial>");
            format!(
                "open script shop after {} input frames from current map {current}",
                input_frames.len()
            )
        })?;
    let snapshot = shell
        .snapshot()
        .context("snapshot runtime shell after shop")?;
    let buy_result = if let Some(buy) = smoke_buy {
        let transaction = shell
            .buy_shop_item(&buy.item_id, buy.quantity)
            .with_context(|| format!("buy {} x{}", buy.item_id, buy.quantity))?;
        Some(transaction)
    } else {
        None
    };
    let sell_result = if let Some(sell) = smoke_sell {
        let transaction = shell
            .sell_shop_item(&sell.item_id, sell.quantity)
            .with_context(|| format!("sell {} x{}", sell.item_id, sell.quantity))?;
        Some(transaction)
    } else {
        None
    };
    let final_snapshot = shell
        .snapshot()
        .context("snapshot runtime shell after shop transaction")?;
    let tracked_item_id = smoke_sell
        .map(|sell| sell.item_id.as_str())
        .or_else(|| smoke_buy.map(|buy| buy.item_id.as_str()));
    let tracked_item_quantity = tracked_item_id.map(|item_id| {
        final_snapshot
            .bag
            .items
            .iter()
            .chain(final_snapshot.bag.balls.iter())
            .chain(final_snapshot.bag.key_items.iter())
            .chain(final_snapshot.bag.pc_items.iter())
            .find(|item| item.item_id == item_id)
            .map(|item| item.quantity)
            .unwrap_or(0)
    });
    println!(
        "smoke-shop spawn={} frames={} map={} source_script={} command_index={} mart_type={} mart_id={} inventory={} final_money={} tracked_item={} tracked_quantity={} opened_checksum={:?} final_checksum={:?}",
        spawn_identifier,
        input_frames.len(),
        shop_ref.map_name,
        shop_ref.source_script,
        shop_ref.command_index,
        shop.outcome.mart_type,
        shop.outcome.mart_id,
        shop.outcome.inventory.join(","),
        final_snapshot.trainer.money,
        tracked_item_id.unwrap_or(""),
        tracked_item_quantity.unwrap_or(0),
        snapshot.state_checksum,
        final_snapshot.state_checksum,
    );
    if let Some(transaction) = buy_result {
        println!(
            "smoke-buy success={} message={} credited={} checksum={:?}",
            transaction.outcome.success,
            transaction.outcome.message,
            transaction.outcome.credited,
            transaction.state_checksum,
        );
    }
    if let Some(transaction) = sell_result {
        println!(
            "smoke-sell success={} message={} credited={} checksum={:?}",
            transaction.outcome.success,
            transaction.outcome.message,
            transaction.outcome.credited,
            transaction.state_checksum,
        );
    }
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save shop smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed shop smoke save")?;
        let resumed_tracked_quantity = tracked_item_id.map(|item_id| {
            resumed_snapshot
                .bag
                .items
                .iter()
                .chain(resumed_snapshot.bag.balls.iter())
                .chain(resumed_snapshot.bag.key_items.iter())
                .chain(resumed_snapshot.bag.pc_items.iter())
                .find(|item| item.item_id == item_id)
                .map(|item| item.quantity)
                .unwrap_or(0)
        });
        if resumed_snapshot.state_checksum != final_snapshot.state_checksum {
            bail!(
                "shop smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                final_snapshot.state_checksum
            );
        }
        if resumed_snapshot.trainer.money != final_snapshot.trainer.money {
            bail!(
                "shop smoke resumed money {} did not match final money {}",
                resumed_snapshot.trainer.money,
                final_snapshot.trainer.money
            );
        }
        if resumed_tracked_quantity != tracked_item_quantity {
            bail!(
                "shop smoke resumed tracked quantity {:?} did not match final tracked quantity {:?}",
                resumed_tracked_quantity,
                tracked_item_quantity
            );
        }
        println!(
            "smoke-shop-save path={} saved_frame={} resumed_money={} resumed_tracked_quantity={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot.trainer.money,
            resumed_tracked_quantity.unwrap_or(0),
            resumed_snapshot.state_checksum,
        );
    }
    let closed = shell.close_script_shop().context("close script shop")?;
    let closed_snapshot = shell
        .snapshot()
        .context("snapshot runtime shell after shop close")?;
    if closed.shop.mart_id != shop.outcome.mart_id {
        bail!(
            "closed shop mart {} did not match opened mart {}",
            closed.shop.mart_id,
            shop.outcome.mart_id
        );
    }
    if closed_snapshot.pending_shop.is_some() {
        bail!("pending_shop remained after closing script shop");
    }
    println!(
        "smoke-shop-close mart_id={} checksum={:?}",
        closed.shop.mart_id, closed.state_checksum
    );
    let drained = shell.drain_script_event_queue(RuntimeScriptEventQueue::Shop)?;
    let shop_events = match drained {
        RuntimeScriptEventDrainResult::Shop(events) => events,
        other => bail!("shop event drain returned non-shop events: {:?}", other),
    };
    if shop_events.is_empty() {
        bail!("shop smoke produced no script shop events to drain");
    }
    println!("smoke-shop-drain events={}", shop_events.len());
    Ok(())
}

fn smoke_field_item_use(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    field_item: &SmokeFieldItemRef,
    smoke_start_map: Option<&SmokeStartMapRef>,
    smoke_buttons: &[GameButton],
    smoke_script: &[Vec<GameButton>],
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    let mut shell = if let Some(start_map) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            start_map.map_name.clone(),
            start_map.tile_x,
            start_map.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let input_frames = smoke_input_frames(smoke_buttons, smoke_script);
    for (index, buttons) in input_frames.iter().enumerate() {
        shell
            .tick(buttons.iter().copied())
            .with_context(|| format!("advance runtime shell input frame {}", index + 1))?;
    }
    let grant = shell
        .add_bag_item(&field_item.item_id, field_item.quantity)
        .with_context(|| {
            format!(
                "grant field item {} x{}",
                field_item.item_id, field_item.quantity
            )
        })?;
    let before_use = shell
        .snapshot()
        .context("snapshot runtime shell before field item use")?;
    let use_result = match &field_item.kind {
        SmokeFieldItemKind::Repel => {
            let result = shell.use_bag_repel_in_field(&field_item.item_id)?;
            format!(
                "repel steps_before={} steps_after={} active_before={} active_after={} consumed={} checksum={:?}",
                result.repel_steps_before,
                result.repel_steps_after,
                result.active_repel_item_before.as_deref().unwrap_or(""),
                result.active_repel_item_after.as_deref().unwrap_or(""),
                result.item_use.consumed,
                result.state_checksum,
            )
        }
        SmokeFieldItemKind::Bicycle => {
            let result = shell.use_bag_bicycle_in_field(&field_item.item_id)?;
            format!(
                "bicycle map={} permission={} mode_before={:?} mode_after={:?} consumed={} checksum={:?}",
                result.map_name,
                result.permission,
                result.mode_before,
                result.mode_after,
                result.item_use.consumed,
                result.state_checksum,
            )
        }
        SmokeFieldItemKind::Itemfinder => {
            let result = shell.use_bag_itemfinder_in_field(&field_item.item_id)?;
            format!(
                "itemfinder tile=({}, {}) found={} cues={} consumed={} checksum={:?}",
                result.player_tile.x,
                result.player_tile.y,
                result.found.is_some(),
                result.itemfinder_sound_cues,
                result.item_use.consumed,
                result.state_checksum,
            )
        }
        SmokeFieldItemKind::TownMap => {
            let result = shell.use_bag_town_map_in_field(&field_item.item_id)?;
            format!(
                "town-map map={} constant={} environment={} landmark={:?} consumed={} checksum={:?}",
                result.map_name,
                result.map_constant,
                result.environment,
                result.landmark,
                result.item_use.consumed,
                result.state_checksum,
            )
        }
        SmokeFieldItemKind::Register => {
            let result = shell.register_key_item(&field_item.item_id)?;
            let snapshot = shell
                .snapshot()
                .context("snapshot registered key item smoke")?;
            let registered = snapshot.progression.registered_key_item.as_deref();
            if registered != Some(field_item.item_id.as_str()) {
                bail!(
                    "register smoke snapshot registered item {:?} did not match {}",
                    registered,
                    field_item.item_id
                );
            }
            format!(
                "register item={} previous={} snapshot_registered={} checksum={:?}",
                result.outcome.item_id,
                result.outcome.previous_item_id.as_deref().unwrap_or(""),
                registered.unwrap_or(""),
                result.state_checksum,
            )
        }
        SmokeFieldItemKind::Pokegear => {
            let result = shell.use_bag_pokegear_in_field(&field_item.item_id)?;
            format!(
                "pokegear item={} consumed={} checksum={:?}",
                result.item_use.item_id, result.item_use.consumed, result.state_checksum,
            )
        }
        SmokeFieldItemKind::Box => {
            let result = shell.use_bag_box_in_field(&field_item.item_id)?;
            let snapshot = shell.snapshot().context("snapshot box item smoke")?;
            if !snapshot
                .progression
                .active_event_flags
                .contains(&result.decoration_flag)
            {
                bail!(
                    "box item smoke did not set decoration flag {}",
                    result.decoration_flag
                );
            }
            format!(
                "box item={} decoration_flag={} already_owned={} consumed={} checksum={:?}",
                result.item_use.item_id,
                result.decoration_flag,
                result.already_owned,
                result.item_use.consumed,
                result.state_checksum,
            )
        }
        SmokeFieldItemKind::EscapeRope => {
            let result = shell.use_bag_escape_rope_in_field(&field_item.item_id)?;
            format!(
                "escape-rope source={} destination={} consumed={} checksum={:?}",
                result.source_map,
                result.destination_map,
                result.item_use.consumed,
                result.state_checksum,
            )
        }
        SmokeFieldItemKind::TmHm {
            species_id,
            level,
            replace_slot,
        } => {
            let party_grant = shell
                .add_party_pokemon(
                    species_id,
                    *level,
                    None,
                    None,
                    "SMOKE",
                    1,
                    Dv::from_non_hp(10, 10, 10, 10),
                )
                .with_context(|| format!("grant TM/HM smoke party Pokemon {species_id}"))?;
            let result = shell
                .use_bag_tmhm_on_party_pokemon(&field_item.item_id, 0, *replace_slot)
                .with_context(|| format!("use TM/HM item {}", field_item.item_id))?;
            let party_after = shell.snapshot().context("snapshot TM/HM smoke party")?;
            let learned = party_after
                .party
                .slots
                .first()
                .and_then(|slot| {
                    slot.pokemon
                        .moves
                        .iter()
                        .find(|learned| learned.name == result.learned_move.learned_move)
                })
                .is_some();
            if !learned {
                bail!(
                    "TM/HM smoke did not find learned move {} in party slot 0",
                    result.learned_move.learned_move
                );
            }
            format!(
                "tmhm species={} level={} party_checksum={:?} move={} tmhm_index={} replaced_slot={:?} replaced_move={} consumed={} checksum={:?}",
                party_grant.outcome.species_id,
                party_grant.outcome.level,
                party_grant.state_checksum,
                result.learned_move.learned_move,
                result.learned_move.tmhm_index,
                result.learned_move.replaced_slot,
                result.learned_move.replaced_move.as_deref().unwrap_or(""),
                result.item_use.consumed,
                result.state_checksum,
            )
        }
        SmokeFieldItemKind::EvolutionStone {
            species_id,
            level,
            expected_species_id,
        } => {
            let party_grant = shell
                .add_party_pokemon(
                    species_id,
                    *level,
                    None,
                    None,
                    "SMOKE",
                    1,
                    Dv::from_non_hp(10, 10, 10, 10),
                )
                .with_context(|| {
                    format!("grant evolution stone smoke party Pokemon {species_id}")
                })?;
            let result = shell
                .use_bag_item_on_party_pokemon(&field_item.item_id, 0)
                .with_context(|| format!("use evolution stone item {}", field_item.item_id))?;
            let evolution_target = result
                .item_effect
                .evolution_target
                .as_deref()
                .context("evolution stone smoke produced no evolution target")?;
            if evolution_target != expected_species_id {
                bail!(
                    "evolution stone smoke target {} did not match expected {}",
                    evolution_target,
                    expected_species_id
                );
            }
            let party_after = shell
                .snapshot()
                .context("snapshot evolution stone smoke party")?;
            let species_after = party_after
                .party
                .slots
                .first()
                .map(|slot| slot.pokemon.species.id.as_str())
                .context("evolution stone smoke party slot 0 missing")?;
            if species_after != expected_species_id {
                bail!(
                    "evolution stone smoke party species {} did not match expected {}",
                    species_after,
                    expected_species_id
                );
            }
            format!(
                "evolution-stone species_before={} level_before={} species_after={} item_target={} consumed={} learned_moves=[{}] party_checksum={:?} checksum={:?}",
                party_grant.outcome.species_id,
                party_grant.outcome.level,
                species_after,
                evolution_target,
                result.item_use.consumed,
                result.item_effect.learned_moves.join("|"),
                party_grant.state_checksum,
                result.state_checksum,
            )
        }
    };
    let final_snapshot = shell
        .snapshot()
        .context("snapshot runtime shell after field item use")?;
    let final_quantity = bag_snapshot_quantity(&final_snapshot, &field_item.item_id);
    println!(
        "smoke-field-item spawn={} frames={} kind={:?} item={} grant_added={} grant_before={} grant_after={} before_checksum={:?} final_quantity={} final_checksum={:?}",
        spawn_identifier,
        input_frames.len(),
        field_item.kind,
        field_item.item_id,
        grant.added,
        grant.quantity_before,
        grant.quantity_after,
        before_use.state_checksum,
        final_quantity,
        final_snapshot.state_checksum,
    );
    println!("smoke-field-item-use {use_result}");
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save field item smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed field item smoke save")?;
        if resumed_snapshot.state_checksum != final_snapshot.state_checksum {
            bail!(
                "field item smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                final_snapshot.state_checksum
            );
        }
        let resumed_quantity = bag_snapshot_quantity(&resumed_snapshot, &field_item.item_id);
        if resumed_quantity != final_quantity {
            bail!(
                "field item smoke resumed quantity {} did not match final quantity {}",
                resumed_quantity,
                final_quantity
            );
        }
        println!(
            "smoke-field-item-save path={} saved_frame={} resumed_quantity={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_quantity,
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn smoke_party_recovery(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    recovery: &SmokePartyRecoveryKind,
    smoke_start_map: Option<&SmokeStartMapRef>,
    party: &[SmokePartyPokemonRef],
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    let mut shell = if let Some(start_map) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            start_map.map_name.clone(),
            start_map.tile_x,
            start_map.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let grants = grant_smoke_party(&mut shell, party, "party recovery")?;
    shell
        .runtime()
        .require_pokemon_status("POISON")
        .context("party recovery smoke requires compiled POISON status")?;
    let setup = shell
        .set_party_pokemon_recovery_state(0, 1, Some("POISON".to_string()), Some(0))
        .context("set party recovery smoke state")?;
    let before = shell
        .snapshot()
        .context("snapshot party recovery smoke before recovery")?;
    let before_slot = before
        .party
        .slots
        .first()
        .context("party recovery smoke missing party slot 0 before recovery")?;
    if before_slot.pokemon.hp != 1 || before_slot.pokemon.status.as_deref() != Some("POISON") {
        bail!(
            "party recovery setup did not persist damaged poisoned state: hp={} status={:?}",
            before_slot.pokemon.hp,
            before_slot.pokemon.status
        );
    }
    let first_move_before = before_slot
        .pokemon
        .moves
        .first()
        .context("party recovery smoke party slot 0 has no moves")?;
    if first_move_before.current_pp != 0 {
        bail!(
            "party recovery setup did not reduce first move PP to 0: {}",
            first_move_before.current_pp
        );
    }
    let recovery_result = match recovery {
        SmokePartyRecoveryKind::FullHeal => {
            let recovered = shell.full_heal_whole_party()?;
            describe_party_recovery("full-heal", &recovered)
        }
        SmokePartyRecoveryKind::Blackout => {
            let recovered = shell.resolve_blackout_to_last_spawn()?;
            format!(
                "blackout spawn={} map={} tile=({}, {}) {} checksum={:?}",
                recovered.spawn_identifier,
                recovered.map_name,
                recovered.tile.x,
                recovered.tile.y,
                describe_party_recovery("healed", &recovered.healed),
                recovered.state_checksum,
            )
        }
    };
    let final_snapshot = shell
        .snapshot()
        .context("snapshot party recovery smoke after recovery")?;
    assert_party_slot_recovered(&final_snapshot)?;
    println!(
        "smoke-party-recovery spawn={} kind={:?} party_grants={} setup_species={} setup_hp={}->{} setup_status={:?}->{:?} setup_first_move={} setup_pp={:?}->{:?} before_checksum={:?} final_checksum={:?}",
        spawn_identifier,
        recovery,
        grants.len(),
        setup.outcome.species_id,
        setup.outcome.hp_before,
        setup.outcome.hp_after,
        setup.outcome.status_before,
        setup.outcome.status_after,
        setup.outcome.first_move.as_deref().unwrap_or(""),
        setup.outcome.first_move_pp_before,
        setup.outcome.first_move_pp_after,
        before.state_checksum,
        final_snapshot.state_checksum,
    );
    println!("smoke-party-recovery-use {recovery_result}");
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save party recovery smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed party recovery smoke save")?;
        if resumed_snapshot.state_checksum != final_snapshot.state_checksum {
            bail!(
                "party recovery smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                final_snapshot.state_checksum
            );
        }
        assert_party_slot_recovered(&resumed_snapshot)?;
        println!(
            "smoke-party-recovery-save path={} saved_frame={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn describe_party_recovery(label: &str, recovered: &[RuntimePartyRecovery]) -> String {
    let entries = recovered
        .iter()
        .map(|entry| {
            format!(
                "{}:{} hp={}->{} status={:?}->{:?} pp=[{}]",
                entry.party_index,
                entry.species_id,
                entry.hp_before,
                entry.hp_after,
                entry.status_before,
                entry.status_after,
                entry
                    .pp_restored
                    .iter()
                    .map(|(move_id, before, after)| format!("{move_id}:{before}->{after}"))
                    .collect::<Vec<_>>()
                    .join("|")
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{label}={entries}")
}

fn assert_party_slot_recovered(snapshot: &RuntimeShellSnapshot) -> Result<()> {
    let slot = snapshot
        .party
        .slots
        .first()
        .context("party recovery smoke missing party slot 0 after recovery")?;
    if slot.pokemon.hp != slot.pokemon.max_hp {
        bail!(
            "party recovery smoke slot 0 hp {} did not match max {}",
            slot.pokemon.hp,
            slot.pokemon.max_hp
        );
    }
    if slot.pokemon.status.is_some() {
        bail!(
            "party recovery smoke slot 0 status was not cleared: {:?}",
            slot.pokemon.status
        );
    }
    let first_move = slot
        .pokemon
        .moves
        .first()
        .context("party recovery smoke slot 0 has no moves after recovery")?;
    if first_move.current_pp == 0 {
        bail!(
            "party recovery smoke first move {} PP was not restored",
            first_move.name
        );
    }
    Ok(())
}

fn smoke_field_move_use(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    field_move: &SmokeFieldMoveRef,
    smoke_start_map: Option<&SmokeStartMapRef>,
    smoke_set_flags: &[String],
    smoke_buttons: &[GameButton],
    smoke_script: &[Vec<GameButton>],
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    let mut shell = if let Some(start_map) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            start_map.map_name.clone(),
            start_map.tile_x,
            start_map.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let input_frames = smoke_input_frames(smoke_buttons, smoke_script);
    for (index, buttons) in input_frames.iter().enumerate() {
        shell
            .tick(buttons.iter().copied())
            .with_context(|| format!("advance runtime shell input frame {}", index + 1))?;
    }
    let party_grant = shell
        .add_party_pokemon(
            &field_move.species_id,
            field_move.level,
            None,
            None,
            "SMOKE",
            1,
            Dv::from_non_hp(10, 10, 10, 10),
        )
        .with_context(|| {
            format!(
                "add field move actor {} level {}",
                field_move.species_id, field_move.level
            )
        })?;
    let initial_moves = shell
        .snapshot()
        .context("snapshot field move actor before teaching move")?
        .party
        .slots
        .first()
        .map(|slot| slot.pokemon.moves.len())
        .unwrap_or(0);
    if initial_moves >= 4 {
        for _ in 0..=(initial_moves - 4) {
            shell
                .delete_party_move_special(0, 0)
                .context("make room for field move")?;
        }
    }
    let taught = shell
        .teach_party_move_special(0, field_move.move_id.clone())
        .with_context(|| format!("teach field move {}", field_move.move_id))?;
    for badge_index in 0..8 {
        shell
            .award_badge(RuntimeBadgeRegion::Johto, badge_index)
            .with_context(|| format!("award Johto badge {badge_index} for field move smoke"))?;
    }
    for flag_id in smoke_set_flags {
        let checksum = shell
            .set_script_flag_for_smoke(flag_id)
            .with_context(|| format!("set smoke flag {flag_id} before field move use"))?;
        println!("smoke-set-flag flag={flag_id} checksum={checksum:?}");
    }
    let before_use = shell
        .snapshot()
        .context("snapshot runtime shell before field move use")?;
    let use_result = match &field_move.kind {
        SmokeFieldMoveKind::Cut {
            metatile_x,
            metatile_y,
        } => {
            let result = shell.use_cut_field_move(0, *metatile_x, *metatile_y)?;
            format!(
                "cut map={} tileset={} tile=({}, {}) previous={} replacement={} variant={} checksum={:?}",
                result.outcome.map_name,
                result.outcome.tileset_name,
                result.outcome.metatile_x,
                result.outcome.metatile_y,
                result.outcome.previous_block_id,
                result.outcome.replacement_block_id,
                result.outcome.variant,
                result.state_checksum,
            )
        }
        SmokeFieldMoveKind::Whirlpool {
            metatile_x,
            metatile_y,
        } => {
            let result = shell.use_whirlpool_field_move(0, *metatile_x, *metatile_y)?;
            format!(
                "whirlpool map={} tileset={} tile=({}, {}) previous={} replacement={} variant={} checksum={:?}",
                result.outcome.map_name,
                result.outcome.tileset_name,
                result.outcome.metatile_x,
                result.outcome.metatile_y,
                result.outcome.previous_block_id,
                result.outcome.replacement_block_id,
                result.outcome.variant,
                result.state_checksum,
            )
        }
        SmokeFieldMoveKind::Strength => {
            let result = shell.use_strength_field_move(0)?;
            format!(
                "strength flag={} was_set={} is_set={} actor={} checksum={:?}",
                result.outcome.engine_flag,
                result.outcome.was_set,
                result.outcome.is_set,
                result.outcome.actor_species,
                result.state_checksum,
            )
        }
        SmokeFieldMoveKind::Flash => {
            let result = shell.use_flash_field_move(0)?;
            format!(
                "flash flag={} was_set={} is_set={} actor={} checksum={:?}",
                result.outcome.engine_flag,
                result.outcome.was_set,
                result.outcome.is_set,
                result.outcome.actor_species,
                result.state_checksum,
            )
        }
        SmokeFieldMoveKind::Surf => {
            let result = shell.use_surf_field_move(0)?;
            format!(
                "surf map={} from=({}, {}) to=({}, {}) steps={} mode={:?} checksum={:?}",
                result.outcome.map_name,
                result.outcome.from_tile.x,
                result.outcome.from_tile.y,
                result.outcome.to_tile.x,
                result.outcome.to_tile.y,
                result.outcome.steps,
                result.outcome.mode,
                result.state_checksum,
            )
        }
        SmokeFieldMoveKind::Waterfall => {
            let result = shell.use_waterfall_field_move(0)?;
            format!(
                "waterfall map={} from=({}, {}) to=({}, {}) steps={} mode={:?} checksum={:?}",
                result.outcome.map_name,
                result.outcome.from_tile.x,
                result.outcome.from_tile.y,
                result.outcome.to_tile.x,
                result.outcome.to_tile.y,
                result.outcome.steps,
                result.outcome.mode,
                result.state_checksum,
            )
        }
        SmokeFieldMoveKind::Fly {
            destination_spawn_identifier,
            flypoint_flag,
        } => {
            let result =
                shell.use_fly_field_move(0, *destination_spawn_identifier, flypoint_flag)?;
            format!(
                "fly source={} destination={} destination_spawn={} tile=({}, {}) flag={} actor={} checksum={:?}",
                result.source_map,
                result.destination_map,
                result.destination_spawn_identifier,
                result.destination_tile.x,
                result.destination_tile.y,
                result.flypoint_flag,
                result.actor_species,
                result.state_checksum,
            )
        }
        SmokeFieldMoveKind::Dig => {
            let result = shell.use_dig_field_move(0)?;
            format!(
                "dig source={} destination={} warp={} tile=({}, {}) actor={} checksum={:?}",
                result.source_map,
                result.destination_map,
                result.destination_warp_index,
                result.destination_tile.x,
                result.destination_tile.y,
                result.actor_species,
                result.state_checksum,
            )
        }
        SmokeFieldMoveKind::Teleport => {
            let result = shell.use_teleport_field_move(0)?;
            format!(
                "teleport source={} destination={} spawn={} tile=({}, {}) actor={} checksum={:?}",
                result.source_map,
                result.destination_map,
                result.destination_spawn_identifier,
                result.destination_tile.x,
                result.destination_tile.y,
                result.actor_species,
                result.state_checksum,
            )
        }
        SmokeFieldMoveKind::Headbutt { player_id } => {
            let result = shell.use_headbutt_field_move(0, *player_id)?;
            format!(
                "headbutt encounter={:?} wild_battle={} checksum={:?}",
                result.field_encounter,
                result.wild_battle.is_some(),
                result.state_checksum,
            )
        }
        SmokeFieldMoveKind::RockSmash => {
            let result = shell.use_rock_smash_field_move(0)?;
            format!(
                "rock-smash encounter={:?} wild_battle={} checksum={:?}",
                result.field_encounter,
                result.wild_battle.is_some(),
                result.state_checksum,
            )
        }
        SmokeFieldMoveKind::SweetScent { surface } => {
            let result = shell.use_sweet_scent_field_move(0, *surface)?;
            format!(
                "sweet-scent surface={:?} encounter={:?} wild_species={} wild_level={} checksum={:?}",
                surface,
                result.wild_encounter,
                result.wild_battle.enemy_pokemon.species.id,
                result.wild_battle.enemy_pokemon.level,
                result.state_checksum,
            )
        }
    };
    let final_snapshot = shell
        .snapshot()
        .context("snapshot runtime shell after field move use")?;
    println!(
        "smoke-field-move spawn={} frames={} kind={:?} species={} level={} move={} party_checksum={:?} taught_checksum={:?} before_checksum={:?} final_map={} final_tile=({}, {}) final_checksum={:?}",
        spawn_identifier,
        input_frames.len(),
        field_move.kind,
        party_grant.outcome.species_id,
        party_grant.outcome.level,
        field_move.move_id,
        party_grant.state_checksum,
        taught.state_checksum,
        before_use.state_checksum,
        final_snapshot.overworld.map_name,
        final_snapshot.overworld.tile.x,
        final_snapshot.overworld.tile.y,
        final_snapshot.state_checksum,
    );
    println!("smoke-field-move-use {use_result}");
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save field move smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed field move smoke save")?;
        if resumed_snapshot.state_checksum != final_snapshot.state_checksum {
            bail!(
                "field move smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                final_snapshot.state_checksum
            );
        }
        println!(
            "smoke-field-move-save path={} saved_frame={} resumed_map={} resumed_tile=({}, {}) resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot.overworld.map_name,
            resumed_snapshot.overworld.tile.x,
            resumed_snapshot.overworld.tile.y,
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn smoke_fishing(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    fishing: &SmokeFishingRef,
    smoke_start_map: Option<&SmokeStartMapRef>,
    party: &SmokePartyPokemonRef,
    smoke_buttons: &[GameButton],
    smoke_script: &[Vec<GameButton>],
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    let mut shell = if let Some(start) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            &start.map_name,
            start.tile_x,
            start.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let input_frames = smoke_input_frames(smoke_buttons, smoke_script);
    for (index, buttons) in input_frames.iter().enumerate() {
        shell
            .tick(buttons.iter().copied())
            .with_context(|| format!("advance runtime shell input frame {}", index + 1))?;
    }
    let party_grant = shell
        .add_party_pokemon(
            &party.species_id,
            party.level,
            party.held_item_id.clone(),
            None,
            "SMOKE",
            1,
            Dv::from_non_hp(10, 10, 10, 10),
        )
        .with_context(|| {
            format!(
                "add fishing party Pokemon {} level {}",
                party.species_id, party.level
            )
        })?;
    let before_cast = shell
        .snapshot()
        .context("snapshot runtime shell before fishing")?;
    let (rod, consumed, cast, item_checksum) = match fishing.kind {
        SmokeFishingKind::Rod => {
            let cast = shell
                .cast_fishing_rod(&fishing.id)
                .with_context(|| format!("cast fishing rod {}", fishing.id))?;
            (fishing.id.clone(), false, cast, None)
        }
        SmokeFishingKind::Item => {
            shell
                .add_bag_item(&fishing.id, 1)
                .with_context(|| format!("grant fishing rod item {}", fishing.id))?;
            let item_use = shell
                .use_bag_fishing_rod_in_field(&fishing.id)
                .with_context(|| format!("use fishing rod item {}", fishing.id))?;
            (
                item_use.rod,
                item_use.item_use.consumed,
                item_use.cast,
                Some(item_use.state_checksum),
            )
        }
    };
    let final_snapshot = shell
        .snapshot()
        .context("snapshot runtime shell after fishing")?;
    let battle = cast
        .wild_battle
        .as_ref()
        .map(|battle| {
            format!(
                "{}:{}",
                battle.enemy_pokemon.species.id, battle.enemy_pokemon.level
            )
        })
        .unwrap_or_else(|| "".to_string());
    println!(
        "smoke-fishing spawn={} frames={} kind={:?} id={} rod={} consumed={} party_species={} party_level={} party_checksum={:?} map={} tile=({}, {}) group={} outcome={:?} bite={:?} battle={} item_checksum={:?} cast_checksum={:?} final_checksum={:?}",
        spawn_identifier,
        input_frames.len(),
        fishing.kind,
        fishing.id,
        rod,
        consumed,
        party_grant.outcome.species_id,
        party_grant.outcome.level,
        party_grant.state_checksum,
        final_snapshot.overworld.map_name,
        final_snapshot.overworld.tile.x,
        final_snapshot.overworld.tile.y,
        cast.session.group.as_deref().unwrap_or(""),
        cast.session.outcome,
        cast.bite,
        battle,
        item_checksum,
        cast.state_checksum,
        final_snapshot.state_checksum,
    );
    if before_cast.state_checksum == final_snapshot.state_checksum {
        bail!("fishing smoke did not mutate runtime state");
    }
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save fishing smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed fishing smoke save")?;
        if resumed_snapshot.state_checksum != final_snapshot.state_checksum {
            bail!(
                "fishing smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                final_snapshot.state_checksum
            );
        }
        println!(
            "smoke-fishing-save path={} saved_frame={} resumed_map={} resumed_tile=({}, {}) resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot.overworld.map_name,
            resumed_snapshot.overworld.tile.x,
            resumed_snapshot.overworld.tile.y,
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn smoke_vertical_menu(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    menu: &SmokeMenuRef,
    smoke_start_map: Option<&SmokeStartMapRef>,
    smoke_buttons: &[GameButton],
    smoke_script: &[Vec<GameButton>],
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    let mut shell = if let Some(start) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            &start.map_name,
            start.tile_x,
            start.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let input_frames = smoke_input_frames(smoke_buttons, smoke_script);
    for (index, buttons) in input_frames.iter().enumerate() {
        shell
            .tick(buttons.iter().copied())
            .with_context(|| format!("advance runtime shell input frame {}", index + 1))?;
    }
    let before_open = shell
        .snapshot()
        .context("snapshot runtime shell before menu smoke")?;
    let opened = shell.open_vertical_menu(
        &menu.map_name,
        format!("{}:{}", menu.source_script, menu.verticalmenu_command_index),
        &menu.source_script,
        menu.loadmenu_command_index,
        menu.verticalmenu_command_index,
    )?;
    let opened_snapshot = shell
        .snapshot()
        .context("snapshot runtime shell after menu open")?;
    let selected = shell.select_vertical_menu_option(
        &opened.menu_id,
        &menu.source_script,
        menu.verticalmenu_command_index,
        menu.option_index,
        &menu.option,
    )?;
    let final_snapshot = shell
        .snapshot()
        .context("snapshot runtime shell after menu selection")?;
    println!(
        "smoke-menu spawn={} frames={} map={} menu_key={} menu_id={} source_script={} loadmenu_index={} verticalmenu_index={} option_index={} option={} options={} script_value={} before_checksum={:?} open_checksum={:?} selection_checksum={:?} final_checksum={:?}",
        spawn_identifier,
        input_frames.len(),
        opened.map_name,
        opened.menu_key,
        opened.menu_id,
        opened.source_script,
        opened.loadmenu_command_index,
        opened.verticalmenu_command_index,
        selected.option_index,
        selected.option,
        opened.options.join(","),
        selected.script_value,
        before_open.state_checksum,
        opened.state_checksum,
        selected.state_checksum,
        final_snapshot.state_checksum,
    );
    if opened_snapshot.menu.is_none() {
        bail!("menu smoke opened no active menu snapshot");
    }
    if before_open.state_checksum == final_snapshot.state_checksum {
        bail!("menu smoke did not mutate runtime state");
    }
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save menu smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed menu smoke save")?;
        if resumed_snapshot.state_checksum != final_snapshot.state_checksum {
            bail!(
                "menu smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                final_snapshot.state_checksum
            );
        }
        println!(
            "smoke-menu-save path={} saved_frame={} resumed_script_value={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed
                .session()
                .state()
                .script_runtime
                .script_value
                .as_deref()
                .unwrap_or(""),
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn smoke_interaction(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    smoke_start_map: Option<&SmokeStartMapRef>,
    smoke_buttons: &[GameButton],
    smoke_script: &[Vec<GameButton>],
    smoke_save: Option<&PathBuf>,
    smoke_player_name: Option<&str>,
) -> Result<()> {
    let mut input_frames: Vec<Vec<GameButton>> = smoke_input_frames(smoke_buttons, smoke_script)
        .into_iter()
        .map(<[GameButton]>::to_vec)
        .collect();
    if input_frames.is_empty()
        || !input_frames
            .last()
            .is_some_and(|buttons| buttons.contains(&GameButton::A))
    {
        input_frames.push(vec![GameButton::A]);
    }
    let smoke = crystal_bevy::smoke_visible_shell_overworld(
        asset_root,
        runtime,
        bevy_shell_start_from_smoke_start_map(spawn_identifier, smoke_start_map),
        BevyShellConfig {
            quick_save_path: smoke_save.cloned(),
            smoke_player_name: smoke_player_name.map(ToOwned::to_owned),
        },
        &input_frames,
        smoke_save,
    )
    .context("run visible interaction smoke through Bevy runtime shell")?;
    if smoke.interactions == 0 {
        bail!("--smoke-interact did not record an interaction");
    }
    println!(
        "smoke-interact spawn={} frames={} start={}@({}, {}) final={}@({}, {}) interactions={} coord_events={} trainer_sight={} warps={} connections={} wild_battles={} active_music={} pending_audio={} audio=[{}] checksum={:?}",
        spawn_identifier,
        input_frames.len(),
        smoke.start_map,
        smoke.start_tile_x,
        smoke.start_tile_y,
        smoke.final_map,
        smoke.final_tile_x,
        smoke.final_tile_y,
        smoke.interactions,
        smoke.coord_events,
        smoke.trainer_sight_events,
        smoke.warps,
        smoke.connections,
        smoke.wild_battles,
        smoke.active_music.as_deref().unwrap_or("none"),
        smoke.pending_audio,
        smoke.audio_events.join("|"),
        smoke.state_hash,
    );
    Ok(())
}

fn smoke_pc_storage(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    party: &[SmokePartyPokemonRef],
    pc_item: &SmokeShopTransactionRef,
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    let mut shell = RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?;
    let party_grants = grant_smoke_party(&mut shell, party, "PC storage")?;
    let item_grant = shell
        .add_bag_item(&pc_item.item_id, pc_item.quantity)
        .with_context(|| {
            format!(
                "grant PC storage item {} x{}",
                pc_item.item_id, pc_item.quantity
            )
        })?;
    let initial_snapshot = shell
        .snapshot()
        .context("snapshot runtime shell before PC storage smoke")?;
    let box_switch = shell
        .switch_current_pc_box(1)
        .context("switch PC storage smoke to box 1")?;
    let deposit = shell
        .deposit_party_pokemon_to_current_box(1)
        .context("deposit second smoke party Pokemon into current PC box")?;
    let after_deposit = shell
        .snapshot()
        .context("snapshot runtime shell after PC Pokemon deposit")?;
    let deposited_box = after_deposit
        .storage
        .boxes
        .iter()
        .find(|pc_box| pc_box.index == deposit.box_index)
        .with_context(|| format!("find PC box {} after deposit", deposit.box_index))?;
    if !deposited_box
        .slots
        .iter()
        .any(|slot| slot.index == deposit.box_slot && slot.pokemon == deposit.pokemon)
    {
        bail!(
            "PC storage smoke did not find deposited Pokemon {} in box {} slot {}",
            deposit.pokemon.species.id,
            deposit.box_index,
            deposit.box_slot
        );
    }
    let withdraw = shell
        .withdraw_current_box_pokemon_to_party(deposit.box_slot)
        .context("withdraw smoke Pokemon from current PC box")?;
    if withdraw.pokemon != deposit.pokemon {
        bail!(
            "PC storage smoke withdrew {:?}, expected deposited {:?}",
            withdraw.pokemon,
            deposit.pokemon
        );
    }
    let item_deposit = shell
        .deposit_bag_item_to_pc(&pc_item.item_id, pc_item.quantity)
        .with_context(|| {
            format!(
                "deposit {} x{} into PC item storage",
                pc_item.item_id, pc_item.quantity
            )
        })?;
    if item_deposit.pc_quantity_after < pc_item.quantity {
        bail!(
            "PC storage smoke deposited item but PC quantity {} is less than {}",
            item_deposit.pc_quantity_after,
            pc_item.quantity
        );
    }
    let item_withdraw = shell
        .withdraw_pc_item_to_bag(&pc_item.item_id, pc_item.quantity)
        .with_context(|| {
            format!(
                "withdraw {} x{} from PC item storage",
                pc_item.item_id, pc_item.quantity
            )
        })?;
    let final_snapshot = shell
        .snapshot()
        .context("snapshot runtime shell after PC storage smoke")?;
    if final_snapshot.storage.current_pc_box != box_switch.box_index_after {
        bail!(
            "PC storage smoke final box {} did not match switched box {}",
            final_snapshot.storage.current_pc_box,
            box_switch.box_index_after
        );
    }
    if final_snapshot.storage.party_count != initial_snapshot.storage.party_count {
        bail!(
            "PC storage smoke final party count {} did not match initial {}",
            final_snapshot.storage.party_count,
            initial_snapshot.storage.party_count
        );
    }
    let final_bag_quantity = bag_snapshot_quantity(&final_snapshot, &pc_item.item_id);
    let final_pc_quantity = pc_item_snapshot_quantity(&final_snapshot, &pc_item.item_id);
    if final_bag_quantity != item_grant.quantity_after {
        bail!(
            "PC storage smoke final bag quantity {} did not match granted quantity {}",
            final_bag_quantity,
            item_grant.quantity_after
        );
    }
    if final_pc_quantity + pc_item.quantity != item_deposit.pc_quantity_after {
        bail!(
            "PC storage smoke final PC quantity {} did not reflect withdrawing {} from {}",
            final_pc_quantity,
            pc_item.quantity,
            item_deposit.pc_quantity_after
        );
    }
    println!(
        "smoke-pc-storage spawn={} party_grants={} current_box_before={} current_box_after={} deposited_species={} deposited_box={} deposited_slot={} withdrawn_party_index={} item={} item_grant_before={} item_grant_after={} item_pc_after_deposit={} item_bag_after_withdraw={} item_pc_after_withdraw={} initial_checksum={:?} final_checksum={:?}",
        spawn_identifier,
        party_grants.len(),
        box_switch.box_index_before,
        box_switch.box_index_after,
        deposit.pokemon.species.id,
        deposit.box_index,
        deposit.box_slot,
        withdraw.party_index,
        pc_item.item_id,
        item_grant.quantity_before,
        item_grant.quantity_after,
        item_deposit.pc_quantity_after,
        item_withdraw.bag_quantity_after,
        item_withdraw.pc_quantity_after,
        initial_snapshot.state_checksum,
        final_snapshot.state_checksum,
    );
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save PC storage smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed PC storage smoke save")?;
        if resumed_snapshot.state_checksum != final_snapshot.state_checksum {
            bail!(
                "PC storage smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                final_snapshot.state_checksum
            );
        }
        if resumed_snapshot.storage.current_pc_box != final_snapshot.storage.current_pc_box {
            bail!(
                "PC storage smoke resumed current box {} did not match final {}",
                resumed_snapshot.storage.current_pc_box,
                final_snapshot.storage.current_pc_box
            );
        }
        if resumed_snapshot.storage.party_count != final_snapshot.storage.party_count {
            bail!(
                "PC storage smoke resumed party count {} did not match final {}",
                resumed_snapshot.storage.party_count,
                final_snapshot.storage.party_count
            );
        }
        let resumed_bag_quantity = bag_snapshot_quantity(&resumed_snapshot, &pc_item.item_id);
        let resumed_pc_quantity = pc_item_snapshot_quantity(&resumed_snapshot, &pc_item.item_id);
        if resumed_bag_quantity != final_bag_quantity || resumed_pc_quantity != final_pc_quantity {
            bail!(
                "PC storage smoke resumed item quantities bag={} pc={} did not match final bag={} pc={}",
                resumed_bag_quantity,
                resumed_pc_quantity,
                final_bag_quantity,
                final_pc_quantity
            );
        }
        println!(
            "smoke-pc-storage-save path={} saved_frame={} resumed_box={} resumed_party_count={} resumed_bag_quantity={} resumed_pc_quantity={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot.storage.current_pc_box,
            resumed_snapshot.storage.party_count,
            resumed_bag_quantity,
            resumed_pc_quantity,
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn smoke_roamers(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    smoke_start_map: Option<&SmokeStartMapRef>,
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    let declared_roamers = runtime.roaming_species_ids();
    if declared_roamers.is_empty() {
        bail!("--smoke-roamers requires compiled roaming Pokemon definitions");
    }
    let mut shell = if let Some(start_map) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            start_map.map_name.clone(),
            start_map.tile_x,
            start_map.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let before = shell
        .snapshot()
        .context("snapshot runtime shell before roamer init")?;
    if !before.roaming_pokemon.is_empty() {
        bail!(
            "new runtime shell already had {} active roamers before InitRoamMons",
            before.roaming_pokemon.len()
        );
    }
    let init = shell.init_roam_mons().context("initialize roamers")?;
    let after = shell
        .snapshot()
        .context("snapshot runtime shell after roamer init")?;
    let SpecialRoutineEffect::InitRoamMons { roamers } = &init.outcome.effect else {
        bail!(
            "InitRoamMons returned unexpected effect {:?}",
            init.outcome.effect
        );
    };
    if roamers.is_empty() {
        bail!("InitRoamMons returned no roamers");
    }
    if after.roaming_pokemon != *roamers {
        bail!(
            "snapshot roamers {:?} did not match InitRoamMons outcome {:?}",
            after.roaming_pokemon,
            roamers
        );
    }
    let active_species = after
        .roaming_pokemon
        .iter()
        .map(|roamer| roamer.species.clone())
        .collect::<std::collections::BTreeSet<_>>();
    if active_species != declared_roamers {
        bail!(
            "active roamer species {:?} did not match declared species {:?}",
            active_species,
            declared_roamers
        );
    }
    println!(
        "smoke-roamers spawn={} declared={} active={} routine={} before_checksum={:?} init_checksum={:?} final_checksum={:?}",
        spawn_identifier,
        declared_roamers.len(),
        after.roaming_pokemon.len(),
        init.outcome.routine,
        before.state_checksum,
        init.state_checksum,
        after.state_checksum,
    );
    for roamer in &after.roaming_pokemon {
        println!(
            "smoke-roamer species={} level={} map_group={} map_number={} hp={} dvs={}",
            roamer.species,
            roamer.level,
            roamer.map_group,
            roamer.map_number,
            roamer.hp,
            roamer.dvs
        );
    }
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save roamer smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed roamer smoke save")?;
        if resumed_snapshot.state_checksum != after.state_checksum {
            bail!(
                "roamer smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                after.state_checksum
            );
        }
        if resumed_snapshot.roaming_pokemon != after.roaming_pokemon {
            bail!(
                "roamer smoke resumed roamers {:?} did not match final {:?}",
                resumed_snapshot.roaming_pokemon,
                after.roaming_pokemon
            );
        }
        println!(
            "smoke-roamers-save path={} saved_frame={} resumed_roamers={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot.roaming_pokemon.len(),
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn smoke_magikarp_length(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    smoke_start_map: Option<&SmokeStartMapRef>,
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    if runtime.data().magikarp_lengths.is_empty() {
        bail!("--smoke-magikarp-length requires compiled Magikarp length definitions");
    }
    let mut shell = if let Some(start_map) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            start_map.map_name.clone(),
            start_map.tile_x,
            start_map.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let party = [SmokePartyPokemonRef {
        species_id: "MAGIKARP".to_string(),
        level: 20,
        held_item_id: None,
    }];
    let grants = grant_smoke_party(&mut shell, &party, "Magikarp length")?;
    let before = shell
        .snapshot()
        .context("snapshot runtime shell before Magikarp length")?;
    if before.magikarp_record.best_feet != 0
        || before.magikarp_record.best_inches != 0
        || before.magikarp_record.current_feet != 0
        || before.magikarp_record.current_inches != 0
    {
        bail!(
            "new runtime shell already had Magikarp record current={}'{} best={}'{}",
            before.magikarp_record.current_feet,
            before.magikarp_record.current_inches,
            before.magikarp_record.best_feet,
            before.magikarp_record.best_inches
        );
    }
    let measured = shell
        .check_magikarp_length(0)
        .context("check Magikarp length")?;
    let after = shell
        .snapshot()
        .context("snapshot runtime shell after Magikarp length")?;
    let SpecialRoutineEffect::CheckMagikarpLength {
        party_slot,
        species,
        feet,
        inches,
        result,
    } = &measured.outcome.effect
    else {
        bail!(
            "CheckMagikarpLength returned unexpected effect {:?}",
            measured.outcome.effect
        );
    };
    if *party_slot != 0 || species != "MAGIKARP" {
        bail!(
            "Magikarp length measured wrong party target slot={} species={}",
            party_slot,
            species
        );
    }
    if *feet == 0 && *inches == 0 {
        bail!("Magikarp length returned zero length");
    }
    if after.magikarp_record.current_feet != *feet
        || after.magikarp_record.current_inches != *inches
        || after.magikarp_record.best_feet != *feet
        || after.magikarp_record.best_inches != *inches
        || after.magikarp_record.best_owner_name != "SMOKE"
    {
        bail!(
            "Magikarp record {:?} did not match measured length {}'{} owner SMOKE",
            after.magikarp_record,
            feet,
            inches
        );
    }
    let formatted = format!("{feet}'{inches}\"");
    if after
        .script_events
        .named_buffers
        .get("STRING_BUFFER_1")
        .map(String::as_str)
        != Some(formatted.as_str())
    {
        bail!(
            "Magikarp length buffer {:?} did not match {formatted}",
            after.script_events.named_buffers.get("STRING_BUFFER_1")
        );
    }
    let result_value = result.to_string();
    if after.script_events.script_value.as_deref() != Some(result_value.as_str()) {
        bail!(
            "Magikarp length script value {:?} did not match result {}",
            after.script_events.script_value,
            result
        );
    }
    let measured_party_slot = after
        .party
        .slots
        .first()
        .context("Magikarp length smoke party was empty after grant")?;
    println!(
        "smoke-magikarp-length spawn={} grants={} species={} level={} length={}'{} result={} owner={} before_checksum={:?} measure_checksum={:?} final_checksum={:?}",
        spawn_identifier,
        grants.len(),
        species,
        measured_party_slot.pokemon.level,
        feet,
        inches,
        result,
        after.magikarp_record.best_owner_name,
        before.state_checksum,
        measured.state_checksum,
        after.state_checksum,
    );
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save Magikarp length smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed Magikarp length smoke save")?;
        if resumed_snapshot.state_checksum != after.state_checksum {
            bail!(
                "Magikarp length smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                after.state_checksum
            );
        }
        if resumed_snapshot.magikarp_record != after.magikarp_record {
            bail!(
                "Magikarp length smoke resumed record {:?} did not match final {:?}",
                resumed_snapshot.magikarp_record,
                after.magikarp_record
            );
        }
        println!(
            "smoke-magikarp-length-save path={} saved_frame={} resumed_record={}'{} owner={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot.magikarp_record.best_feet,
            resumed_snapshot.magikarp_record.best_inches,
            resumed_snapshot.magikarp_record.best_owner_name,
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn smoke_odd_egg(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    smoke_start_map: Option<&SmokeStartMapRef>,
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    if runtime.data().odd_egg_definitions.is_empty() {
        bail!("--smoke-odd-egg requires compiled Odd Egg definitions");
    }
    let mut shell = if let Some(start_map) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            start_map.map_name.clone(),
            start_map.tile_x,
            start_map.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let before = shell
        .snapshot()
        .context("snapshot runtime shell before Odd Egg")?;
    let given = shell.give_odd_egg().context("give Odd Egg")?;
    let after = shell
        .snapshot()
        .context("snapshot runtime shell after Odd Egg")?;
    if before.state_checksum == after.state_checksum {
        bail!("Odd Egg smoke did not mutate runtime state");
    }
    let SpecialRoutineEffect::GiveOddEgg {
        table_index,
        species,
        party_slot,
        shiny,
        rng_seed_after,
    } = &given.outcome.effect
    else {
        bail!(
            "GiveOddEgg returned unexpected effect {:?}",
            given.outcome.effect
        );
    };
    if *table_index >= shell.runtime().data().odd_egg_definitions.len() {
        bail!(
            "Odd Egg selected table index {} outside {} compiled definitions",
            table_index,
            shell.runtime().data().odd_egg_definitions.len()
        );
    }
    let odd_egg_slot = after
        .party
        .slots
        .iter()
        .find(|slot| slot.index == *party_slot)
        .with_context(|| format!("Odd Egg party slot {} was not populated", party_slot))?;
    if odd_egg_slot.pokemon.species.id != *species {
        bail!(
            "Odd Egg party slot species {} did not match effect species {}",
            odd_egg_slot.pokemon.species.id,
            species
        );
    }
    if odd_egg_slot.pokemon.hp != 0 {
        bail!(
            "Odd Egg party slot hp {} was not egg hp 0",
            odd_egg_slot.pokemon.hp
        );
    }
    if after.party.slots.len() != before.party.slots.len() + 1 {
        bail!(
            "Odd Egg party count {} did not increase from {}",
            after.party.slots.len(),
            before.party.slots.len()
        );
    }
    if after.storage.party_count != after.party.slots.len() {
        bail!(
            "Odd Egg storage party_count {} did not match party slots {}",
            after.storage.party_count,
            after.party.slots.len()
        );
    }
    if after
        .script_events
        .variables
        .get("wCurPartySpecies")
        .map(String::as_str)
        != Some(species.as_str())
    {
        bail!(
            "Odd Egg wCurPartySpecies {:?} did not match {}",
            after.script_events.variables.get("wCurPartySpecies"),
            species
        );
    }
    let party_slot_value = party_slot.to_string();
    if after
        .script_events
        .variables
        .get("wCurPartyMon")
        .map(String::as_str)
        != Some(party_slot_value.as_str())
    {
        bail!(
            "Odd Egg wCurPartyMon {:?} did not match {}",
            after.script_events.variables.get("wCurPartyMon"),
            party_slot
        );
    }
    if after.script_events.script_value.as_deref() != Some("1") {
        bail!(
            "Odd Egg script value {:?} did not report success",
            after.script_events.script_value
        );
    }
    println!(
        "smoke-odd-egg spawn={} table_index={} species={} party_slot={} level={} shiny={} rng_seed_after={} before_checksum={:?} give_checksum={:?} final_checksum={:?}",
        spawn_identifier,
        table_index,
        species,
        party_slot,
        odd_egg_slot.pokemon.level,
        shiny,
        rng_seed_after,
        before.state_checksum,
        given.state_checksum,
        after.state_checksum,
    );
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save Odd Egg smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed Odd Egg smoke save")?;
        if resumed_snapshot.state_checksum != after.state_checksum {
            bail!(
                "Odd Egg smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                after.state_checksum
            );
        }
        let resumed_slot = resumed_snapshot
            .party
            .slots
            .iter()
            .find(|slot| slot.index == *party_slot)
            .with_context(|| format!("resumed Odd Egg party slot {} was missing", party_slot))?;
        if resumed_slot.pokemon != odd_egg_slot.pokemon {
            bail!(
                "resumed Odd Egg {:?} did not match final {:?}",
                resumed_slot.pokemon,
                odd_egg_slot.pokemon
            );
        }
        println!(
            "smoke-odd-egg-save path={} saved_frame={} resumed_species={} resumed_party_slot={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_slot.pokemon.species.id,
            resumed_slot.index,
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn smoke_mystery_gift(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    smoke_start_map: Option<&SmokeStartMapRef>,
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    let mut shell = if let Some(start_map) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            start_map.map_name.clone(),
            start_map.tile_x,
            start_map.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let before = shell
        .snapshot()
        .context("snapshot runtime shell before Mystery Gift")?;
    if before.mystery_gift.unlocked
        || before.mystery_gift.stored_item.is_some()
        || before.mystery_gift.backup_item.is_some()
    {
        bail!(
            "new runtime shell already had Mystery Gift state {:?}",
            before.mystery_gift
        );
    }

    let unlocked = shell
        .use_mystery_gift(RuntimeMysteryGiftAction::Unlock)
        .context("unlock Mystery Gift")?;
    let after_unlock = shell
        .snapshot()
        .context("snapshot runtime shell after Mystery Gift unlock")?;
    let SpecialRoutineEffect::UnlockMysteryGift { newly_unlocked } = &unlocked.outcome.effect
    else {
        bail!(
            "UnlockMysteryGift returned unexpected effect {:?}",
            unlocked.outcome.effect
        );
    };
    if !newly_unlocked || !after_unlock.mystery_gift.unlocked {
        bail!(
            "Mystery Gift unlock effect {} left snapshot {:?}",
            newly_unlocked,
            after_unlock.mystery_gift
        );
    }
    if after_unlock.mystery_gift.stored_item.is_some()
        || after_unlock.mystery_gift.backup_item.is_some()
    {
        bail!(
            "Mystery Gift unlock left pending items {:?}",
            after_unlock.mystery_gift
        );
    }
    if after_unlock.script_events.script_value.as_deref() != Some("1") {
        bail!(
            "Mystery Gift unlock script value {:?} did not report newly unlocked",
            after_unlock.script_events.script_value
        );
    }

    let checked = shell
        .use_mystery_gift(RuntimeMysteryGiftAction::Check)
        .context("check Mystery Gift")?;
    let after_check = shell
        .snapshot()
        .context("snapshot runtime shell after Mystery Gift check")?;
    let SpecialRoutineEffect::CheckMysteryGift { has_pending_item } = &checked.outcome.effect
    else {
        bail!(
            "CheckMysteryGift returned unexpected effect {:?}",
            checked.outcome.effect
        );
    };
    if *has_pending_item {
        bail!("new Mystery Gift smoke unexpectedly had a pending item");
    }
    if after_check.mystery_gift != after_unlock.mystery_gift {
        bail!(
            "Mystery Gift check mutated state {:?} from {:?}",
            after_check.mystery_gift,
            after_unlock.mystery_gift
        );
    }
    if after_check.script_events.script_value.as_deref() != Some("0") {
        bail!(
            "Mystery Gift check script value {:?} did not report no pending item",
            after_check.script_events.script_value
        );
    }

    let claimed = shell
        .use_mystery_gift(RuntimeMysteryGiftAction::ClaimItem)
        .context("claim empty Mystery Gift item")?;
    let after_claim = shell
        .snapshot()
        .context("snapshot runtime shell after Mystery Gift claim")?;
    let SpecialRoutineEffect::GetMysteryGiftItem { item_id, received } = &claimed.outcome.effect
    else {
        bail!(
            "GetMysteryGiftItem returned unexpected effect {:?}",
            claimed.outcome.effect
        );
    };
    if item_id.is_some() || *received {
        bail!(
            "empty Mystery Gift claim returned item {:?} received={}",
            item_id,
            received
        );
    }
    if after_claim.mystery_gift != after_unlock.mystery_gift {
        bail!(
            "Mystery Gift empty claim mutated state {:?} from {:?}",
            after_claim.mystery_gift,
            after_unlock.mystery_gift
        );
    }
    if after_claim.script_events.script_value.as_deref() != Some("0") {
        bail!(
            "Mystery Gift empty claim script value {:?} did not report no item",
            after_claim.script_events.script_value
        );
    }
    println!(
        "smoke-mystery-gift spawn={} unlocked={} pending={} before_checksum={:?} unlock_checksum={:?} check_checksum={:?} final_checksum={:?}",
        spawn_identifier,
        after_claim.mystery_gift.unlocked,
        after_claim.mystery_gift.stored_item.is_some(),
        before.state_checksum,
        unlocked.state_checksum,
        checked.state_checksum,
        after_claim.state_checksum,
    );
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save Mystery Gift smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed Mystery Gift smoke save")?;
        if resumed_snapshot.state_checksum != after_claim.state_checksum {
            bail!(
                "Mystery Gift smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                after_claim.state_checksum
            );
        }
        if resumed_snapshot.mystery_gift != after_claim.mystery_gift {
            bail!(
                "Mystery Gift smoke resumed state {:?} did not match final {:?}",
                resumed_snapshot.mystery_gift,
                after_claim.mystery_gift
            );
        }
        println!(
            "smoke-mystery-gift-save path={} saved_frame={} resumed_unlocked={} resumed_pending={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot.mystery_gift.unlocked,
            resumed_snapshot.mystery_gift.stored_item.is_some(),
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn smoke_buena_password(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    smoke_start_map: Option<&SmokeStartMapRef>,
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    const SMOKE_BLUE_CARD_POINTS: u8 = 99;
    let mut shell = if let Some(start_map) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            start_map.map_name.clone(),
            start_map.tile_x,
            start_map.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let before = shell
        .snapshot()
        .context("snapshot runtime shell before Buena password")?;
    if before.buenas_password.generated || before.trainer.blue_card_balance != 0 {
        bail!(
            "new runtime shell already had Buena state {:?} and Blue Card balance {}",
            before.buenas_password,
            before.trainer.blue_card_balance
        );
    }

    let prompt = shell
        .use_buena_password(None)
        .context("generate Buena password")?;
    let after_prompt = shell
        .snapshot()
        .context("snapshot runtime shell after Buena password prompt")?;
    let SpecialRoutineEffect::BuenasPassword {
        category,
        category_type,
        correct,
        guess,
        matched,
        rng_seed_after,
    } = &prompt.outcome.effect
    else {
        bail!(
            "BuenasPassword prompt returned unexpected effect {:?}",
            prompt.outcome.effect
        );
    };
    if guess.is_some() || *matched {
        bail!(
            "Buena password prompt unexpectedly had guess {:?} matched={}",
            guess,
            matched
        );
    }
    if !after_prompt.buenas_password.generated
        || after_prompt.progression.rng_seed != *rng_seed_after
        || after_prompt.script_events.variables.get("_buena_category") != Some(category)
        || after_prompt.script_events.variables.get("_buena_password") != Some(correct)
    {
        bail!(
            "Buena password prompt state {:?} variables {:?} rng {} did not match effect category={} password={} rng={}",
            after_prompt.buenas_password,
            after_prompt.script_events.variables,
            after_prompt.progression.rng_seed,
            category,
            correct,
            rng_seed_after
        );
    }

    let answer = shell
        .use_buena_password(Some(correct.clone()))
        .context("answer Buena password")?;
    let after_answer = shell
        .snapshot()
        .context("snapshot runtime shell after Buena password answer")?;
    let SpecialRoutineEffect::BuenasPassword {
        guess: answered_guess,
        matched: answered,
        ..
    } = &answer.outcome.effect
    else {
        bail!(
            "BuenasPassword answer returned unexpected effect {:?}",
            answer.outcome.effect
        );
    };
    if answered_guess.as_deref() != Some(correct.as_str())
        || !answered
        || after_answer.script_events.script_value.as_deref() != Some("1")
        || after_answer.buenas_password != after_prompt.buenas_password
    {
        bail!(
            "Buena password answer guess={:?} matched={} script_value={:?} state {:?} did not match prompt {:?}",
            answered_guess,
            answered,
            after_answer.script_events.script_value,
            after_answer.buenas_password,
            after_prompt.buenas_password
        );
    }

    let prize_item = shell
        .runtime()
        .buena_prize_item_ids()
        .into_iter()
        .next()
        .context("--smoke-buena-password requires compiled Buena prize definitions")?;
    let prize_quantity_before = runtime_bag_quantity(&after_answer.bag, &prize_item);
    let points_checksum = shell
        .set_blue_card_balance_for_smoke(SMOKE_BLUE_CARD_POINTS)
        .context("seed Buena smoke Blue Card balance")?;
    let after_points = shell
        .snapshot()
        .context("snapshot runtime shell after Buena Blue Card point seed")?;
    if after_points.trainer.blue_card_balance != u16::from(SMOKE_BLUE_CARD_POINTS) {
        bail!(
            "Buena smoke Blue Card point seed produced balance {}",
            after_points.trainer.blue_card_balance
        );
    }

    let prize = shell
        .use_buena_prize(prize_item.clone(), 1)
        .with_context(|| format!("redeem Buena prize {prize_item}"))?;
    let after = shell
        .snapshot()
        .context("snapshot runtime shell after Buena prize")?;
    let SpecialRoutineEffect::BuenaPrize {
        item_id,
        quantity,
        points_spent,
        balance,
    } = &prize.outcome.effect
    else {
        bail!(
            "BuenaPrize returned unexpected effect {:?}",
            prize.outcome.effect
        );
    };
    if item_id != &prize_item || *quantity != 1 || *points_spent == 0 {
        bail!(
            "Buena prize effect item={} quantity={} points_spent={} did not match smoke request {} x1",
            item_id,
            quantity,
            points_spent,
            prize_item
        );
    }
    if *balance != SMOKE_BLUE_CARD_POINTS - *points_spent {
        bail!(
            "Buena prize balance {} did not match seeded {} minus spent {}",
            balance,
            SMOKE_BLUE_CARD_POINTS,
            points_spent
        );
    }
    if after.trainer.blue_card_balance != u16::from(*balance)
        || runtime_bag_quantity(&after.bag, &prize_item) != prize_quantity_before + 1
        || after.script_events.script_value.as_deref() != Some("1")
        || after.script_events.last_special_routine.as_deref() != Some("BuenaPrize")
    {
        bail!(
            "Buena prize state balance={} quantity_before={} quantity_after={} script_value={:?} last_special={:?} did not match successful redemption",
            after.trainer.blue_card_balance,
            prize_quantity_before,
            runtime_bag_quantity(&after.bag, &prize_item),
            after.script_events.script_value,
            after.script_events.last_special_routine
        );
    }

    println!(
        "smoke-buena-password spawn={} category={} category_type={} correct={} rng_seed_after={} prize={} points_spent={} balance_after={} before_checksum={:?} prompt_checksum={:?} answer_checksum={:?} points_checksum={:?} final_checksum={:?}",
        spawn_identifier,
        category,
        category_type,
        correct,
        rng_seed_after,
        prize_item,
        points_spent,
        after.trainer.blue_card_balance,
        before.state_checksum,
        prompt.state_checksum,
        answer.state_checksum,
        points_checksum,
        after.state_checksum,
    );
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save Buena password smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed Buena password smoke save")?;
        if resumed_snapshot.state_checksum != after.state_checksum {
            bail!(
                "Buena password smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                after.state_checksum
            );
        }
        if resumed_snapshot.buenas_password != after.buenas_password
            || resumed_snapshot.trainer.blue_card_balance != after.trainer.blue_card_balance
            || runtime_bag_quantity(&resumed_snapshot.bag, &prize_item)
                != runtime_bag_quantity(&after.bag, &prize_item)
        {
            bail!(
                "Buena password smoke resumed state did not match final: resumed={:?}/balance {}/quantity {} final={:?}/balance {}/quantity {}",
                resumed_snapshot.buenas_password,
                resumed_snapshot.trainer.blue_card_balance,
                runtime_bag_quantity(&resumed_snapshot.bag, &prize_item),
                after.buenas_password,
                after.trainer.blue_card_balance,
                runtime_bag_quantity(&after.bag, &prize_item)
            );
        }
        println!(
            "smoke-buena-password-save path={} saved_frame={} resumed_category_index={} resumed_option_index={} resumed_balance={} resumed_prize_quantity={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot.buenas_password.category_index,
            resumed_snapshot.buenas_password.option_index,
            resumed_snapshot.trainer.blue_card_balance,
            runtime_bag_quantity(&resumed_snapshot.bag, &prize_item),
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn smoke_shuckie(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    smoke_start_map: Option<&SmokeStartMapRef>,
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    let mut shell = if let Some(start_map) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            start_map.map_name.clone(),
            start_map.tile_x,
            start_map.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let before = shell
        .snapshot()
        .context("snapshot runtime shell before Shuckie")?;
    let gift = before
        .special
        .shuckie_gift
        .clone()
        .context("--smoke-shuckie requires compiled Shuckie gift data")?;
    if !before.party.slots.is_empty() || before.storage.party_count != 0 {
        bail!(
            "new runtime shell already had party slots={} storage party_count={}",
            before.party.slots.len(),
            before.storage.party_count
        );
    }
    if before
        .progression
        .active_engine_flags
        .contains(&gift.got_today_engine_flag)
    {
        bail!(
            "new runtime shell already had Shuckie engine flag {}",
            gift.got_today_engine_flag
        );
    }

    let give = shell
        .use_shuckie(RuntimeShuckieAction::Give, None)
        .context("give Shuckie")?;
    let after_give = shell
        .snapshot()
        .context("snapshot runtime shell after Shuckie gift")?;
    let SpecialRoutineEffect::GiveShuckle {
        stored,
        rng_seed_after,
    } = &give.outcome.effect
    else {
        bail!(
            "GiveShuckle returned unexpected effect {:?}",
            give.outcome.effect
        );
    };
    if !stored {
        bail!("Shuckie gift was not stored");
    }
    if after_give.progression.rng_seed != *rng_seed_after {
        bail!(
            "Shuckie gift rng seed {} did not match effect {}",
            after_give.progression.rng_seed,
            rng_seed_after
        );
    }
    if after_give.party.slots.len() != 1 || after_give.storage.party_count != 1 {
        bail!(
            "Shuckie gift party slots={} storage party_count={} did not become one",
            after_give.party.slots.len(),
            after_give.storage.party_count
        );
    }
    let shuckie_slot = after_give
        .party
        .slots
        .first()
        .context("Shuckie gift did not populate party slot 0")?;
    let shuckie = &shuckie_slot.pokemon;
    if shuckie_slot.index != 0
        || shuckie.species.id != gift.species
        || shuckie.level != gift.level
        || shuckie.item.as_deref() != Some(gift.held_item.as_str())
        || shuckie.nickname != gift.nickname
        || shuckie.original_trainer_name != gift.original_trainer_name
        || shuckie.original_trainer_id != gift.original_trainer_id
    {
        bail!(
            "Shuckie gift party slot {:?} did not match compiled gift {:?}",
            shuckie_slot,
            gift
        );
    }
    if !after_give
        .progression
        .active_engine_flags
        .contains(&gift.got_today_engine_flag)
    {
        bail!(
            "Shuckie gift did not set engine flag {}",
            gift.got_today_engine_flag
        );
    }
    if after_give.script_events.script_value.as_deref() != Some("1")
        || after_give.script_events.variables.get("wCurPartySpecies") != Some(&gift.species)
        || after_give.script_events.last_special_routine.as_deref() != Some("GiveShuckle")
    {
        bail!(
            "Shuckie gift script state value={:?} species={:?} last_special={:?} did not match",
            after_give.script_events.script_value,
            after_give.script_events.variables.get("wCurPartySpecies"),
            after_give.script_events.last_special_routine
        );
    }

    let returned = shell
        .use_shuckie(RuntimeShuckieAction::Return, Some(0))
        .context("return Shuckie")?;
    let after = shell
        .snapshot()
        .context("snapshot runtime shell after Shuckie return")?;
    let SpecialRoutineEffect::ReturnShuckie { party_slot, result } = &returned.outcome.effect
    else {
        bail!(
            "ReturnShuckie returned unexpected effect {:?}",
            returned.outcome.effect
        );
    };
    if *party_slot != Some(0) || *result != 2 {
        bail!(
            "ReturnShuckie effect slot={:?} result={} did not report returned slot 0",
            party_slot,
            result
        );
    }
    if !after.party.slots.is_empty()
        || after.storage.party_count != 0
        || after.script_events.script_value.as_deref() != Some("2")
        || after.script_events.last_special_routine.as_deref() != Some("ReturnShuckie")
    {
        bail!(
            "Shuckie return state slots={} party_count={} script_value={:?} last_special={:?} did not match returned state",
            after.party.slots.len(),
            after.storage.party_count,
            after.script_events.script_value,
            after.script_events.last_special_routine
        );
    }
    if !after
        .progression
        .active_engine_flags
        .contains(&gift.got_today_engine_flag)
    {
        bail!(
            "Shuckie return unexpectedly cleared engine flag {}",
            gift.got_today_engine_flag
        );
    }

    println!(
        "smoke-shuckie spawn={} species={} level={} held_item={} nickname={} ot={} ot_id={} rng_seed_after={} return_result={} before_checksum={:?} give_checksum={:?} final_checksum={:?}",
        spawn_identifier,
        gift.species,
        gift.level,
        gift.held_item,
        gift.nickname,
        gift.original_trainer_name,
        gift.original_trainer_id,
        rng_seed_after,
        result,
        before.state_checksum,
        give.state_checksum,
        after.state_checksum,
    );
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save Shuckie smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed Shuckie smoke save")?;
        if resumed_snapshot.state_checksum != after.state_checksum {
            bail!(
                "Shuckie smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                after.state_checksum
            );
        }
        if resumed_snapshot.party != after.party
            || resumed_snapshot.storage.party_count != after.storage.party_count
            || resumed_snapshot.progression.active_engine_flags
                != after.progression.active_engine_flags
            || resumed_snapshot.script_events.script_value != after.script_events.script_value
            || resumed_snapshot.script_events.last_special_routine
                != after.script_events.last_special_routine
        {
            bail!(
                "Shuckie smoke resumed state did not match final state: resumed party_count={} flags={:?} script={:?}/{:?} final party_count={} flags={:?} script={:?}/{:?}",
                resumed_snapshot.storage.party_count,
                resumed_snapshot.progression.active_engine_flags,
                resumed_snapshot.script_events.script_value,
                resumed_snapshot.script_events.last_special_routine,
                after.storage.party_count,
                after.progression.active_engine_flags,
                after.script_events.script_value,
                after.script_events.last_special_routine
            );
        }
        println!(
            "smoke-shuckie-save path={} saved_frame={} resumed_party_count={} resumed_flag_set={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot.storage.party_count,
            resumed_snapshot
                .progression
                .active_engine_flags
                .contains(&gift.got_today_engine_flag),
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn smoke_dratini(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    smoke_start_map: Option<&SmokeStartMapRef>,
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    let mut shell = if let Some(start_map) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            start_map.map_name.clone(),
            start_map.tile_x,
            start_map.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let before = shell
        .snapshot()
        .context("snapshot runtime shell before Dratini")?;
    let (mode, expected_moves) = before
        .special
        .dratini_move_sets
        .iter()
        .next()
        .map(|(mode, moves)| (*mode, moves.clone()))
        .context("--smoke-dratini requires compiled Dratini move set data")?;
    if !before.party.slots.is_empty() || before.storage.party_count != 0 {
        bail!(
            "new runtime shell already had party slots={} storage party_count={}",
            before.party.slots.len(),
            before.storage.party_count
        );
    }

    let party = [SmokePartyPokemonRef {
        species_id: "DRATINI".to_string(),
        level: 15,
        held_item_id: None,
    }];
    let grants = grant_smoke_party(&mut shell, &party, "Dratini")?;
    let grant = grants
        .first()
        .context("Dratini grant did not return a grant")?;
    let seeded = shell
        .snapshot()
        .context("snapshot runtime shell after Dratini party grant")?;
    if seeded.party.slots.len() != 1 || seeded.storage.party_count != 1 {
        bail!(
            "Dratini party seed slots={} storage party_count={} did not become one",
            seeded.party.slots.len(),
            seeded.storage.party_count
        );
    }
    let seeded_slot = seeded
        .party
        .slots
        .first()
        .context("Dratini party seed did not populate slot 0")?;
    if seeded_slot.index != 0 || seeded_slot.pokemon.species.id != "DRATINI" {
        bail!("Dratini party seed populated unexpected slot {seeded_slot:?}");
    }

    let used = shell
        .give_dratini(mode)
        .with_context(|| format!("run GiveDratini mode {mode}"))?;
    let after = shell
        .snapshot()
        .context("snapshot runtime shell after Dratini")?;
    let SpecialRoutineEffect::GiveDratini {
        party_slot,
        mode: effect_mode,
        move_names,
        learned,
    } = &used.outcome.effect
    else {
        bail!(
            "GiveDratini returned unexpected effect {:?}",
            used.outcome.effect
        );
    };
    if *party_slot != Some(0) || *effect_mode != mode || move_names != &expected_moves || !*learned
    {
        bail!(
            "GiveDratini effect slot={:?} mode={} moves={:?} learned={} did not match expected mode={} moves={:?}",
            party_slot,
            effect_mode,
            move_names,
            learned,
            mode,
            expected_moves
        );
    }
    let after_slot = after
        .party
        .slots
        .first()
        .context("Dratini disappeared from party after GiveDratini")?;
    let actual_moves = after_slot
        .pokemon
        .moves
        .iter()
        .map(|known| known.name.clone())
        .collect::<Vec<_>>();
    if after_slot.index != 0
        || after_slot.pokemon.species.id != "DRATINI"
        || actual_moves != expected_moves
    {
        bail!(
            "Dratini party slot {:?} moves {:?} did not match compiled moves {:?}",
            after_slot,
            actual_moves,
            expected_moves
        );
    }
    let mode_script_value = mode.to_string();
    if after.script_events.script_value.as_deref() != Some(mode_script_value.as_str())
        || after
            .script_events
            .variables
            .get("wCurPartySpecies")
            .map(String::as_str)
            != Some("DRATINI")
        || after.script_events.last_special_routine.as_deref() != Some("GiveDratini")
    {
        bail!(
            "Dratini script state value={:?} species={:?} last_special={:?} did not match",
            after.script_events.script_value,
            after.script_events.variables.get("wCurPartySpecies"),
            after.script_events.last_special_routine
        );
    }

    println!(
        "smoke-dratini spawn={} mode={} party_slot=0 level={} moves={} grant_checksum={:?} final_checksum={:?}",
        spawn_identifier,
        mode,
        seeded_slot.pokemon.level,
        actual_moves.join(","),
        grant.state_checksum,
        after.state_checksum,
    );
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save Dratini smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed Dratini smoke save")?;
        if resumed_snapshot.state_checksum != after.state_checksum {
            bail!(
                "Dratini smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                after.state_checksum
            );
        }
        if resumed_snapshot.party != after.party
            || resumed_snapshot.storage.party_count != after.storage.party_count
            || resumed_snapshot.script_events.script_value != after.script_events.script_value
            || resumed_snapshot.script_events.last_special_routine
                != after.script_events.last_special_routine
        {
            bail!(
                "Dratini smoke resumed state did not match final state: resumed party_count={} script={:?}/{:?} final party_count={} script={:?}/{:?}",
                resumed_snapshot.storage.party_count,
                resumed_snapshot.script_events.script_value,
                resumed_snapshot.script_events.last_special_routine,
                after.storage.party_count,
                after.script_events.script_value,
                after.script_events.last_special_routine
            );
        }
        println!(
            "smoke-dratini-save path={} saved_frame={} resumed_party_count={} resumed_moves={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot.storage.party_count,
            resumed_snapshot
                .party
                .slots
                .first()
                .map(|slot| slot
                    .pokemon
                    .moves
                    .iter()
                    .map(|known| known.name.as_str())
                    .collect::<Vec<_>>()
                    .join(","))
                .unwrap_or_default(),
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn smoke_kurt_apricorn(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    smoke_start_map: Option<&SmokeStartMapRef>,
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    const SMOKE_APRICORN_QUANTITY: u16 = 2;
    let mut shell = if let Some(start_map) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            start_map.map_name.clone(),
            start_map.tile_x,
            start_map.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let before = shell
        .snapshot()
        .context("snapshot runtime shell before Kurt apricorn")?;
    let (apricorn_id, ball_id) = before
        .special
        .kurt_apricorn_recipes
        .iter()
        .next()
        .map(|(apricorn, ball)| (apricorn.clone(), ball.clone()))
        .context("--smoke-kurt-apricorn requires compiled Kurt apricorn recipe data")?;
    if runtime_bag_quantity(&before.bag, &apricorn_id) != 0 {
        bail!("new runtime shell already had {apricorn_id} in the bag");
    }

    let grant = shell
        .add_bag_item(&apricorn_id, SMOKE_APRICORN_QUANTITY)
        .with_context(|| format!("grant Kurt apricorn smoke item {apricorn_id}"))?;
    let seeded = shell
        .snapshot()
        .context("snapshot runtime shell after Kurt apricorn item grant")?;
    let seeded_quantity = runtime_bag_quantity(&seeded.bag, &apricorn_id);
    if seeded_quantity != SMOKE_APRICORN_QUANTITY {
        bail!(
            "Kurt apricorn bag quantity {} did not become {} after grant",
            seeded_quantity,
            SMOKE_APRICORN_QUANTITY
        );
    }

    let used = shell
        .use_kurt_apricorn(apricorn_id.clone(), SMOKE_APRICORN_QUANTITY)
        .with_context(|| format!("select Kurt apricorn {apricorn_id}"))?;
    let after = shell
        .snapshot()
        .context("snapshot runtime shell after Kurt apricorn")?;
    let SpecialRoutineEffect::SelectApricornForKurt { apricorn, quantity } = &used.outcome.effect
    else {
        bail!(
            "SelectApricornForKurt returned unexpected effect {:?}",
            used.outcome.effect
        );
    };
    if apricorn.as_deref() != Some(apricorn_id.as_str()) || *quantity != SMOKE_APRICORN_QUANTITY {
        bail!(
            "SelectApricornForKurt effect apricorn={:?} quantity={} did not match {} x{}",
            apricorn,
            quantity,
            apricorn_id,
            SMOKE_APRICORN_QUANTITY
        );
    }
    let after_quantity = runtime_bag_quantity(&after.bag, &apricorn_id);
    if after_quantity != 0 {
        bail!(
            "Kurt apricorn bag quantity {} did not decrement to zero",
            after_quantity
        );
    }
    let quantity_script_value = SMOKE_APRICORN_QUANTITY.to_string();
    if after.script_events.script_value.as_deref() != Some("1")
        || after
            .script_events
            .variables
            .get("_kurt_apricorn_type")
            .map(String::as_str)
            != Some(apricorn_id.as_str())
        || after
            .script_events
            .variables
            .get("_kurt_apricorn_quantity")
            .map(String::as_str)
            != Some(quantity_script_value.as_str())
        || after
            .script_events
            .variables
            .get("VAR_KURT_APRICORNS")
            .map(String::as_str)
            != Some(quantity_script_value.as_str())
        || after.script_events.last_special_routine.as_deref() != Some("SelectApricornForKurt")
    {
        bail!(
            "Kurt apricorn script state value={:?} type={:?} quantity={:?} var={:?} last_special={:?} did not match",
            after.script_events.script_value,
            after.script_events.variables.get("_kurt_apricorn_type"),
            after.script_events.variables.get("_kurt_apricorn_quantity"),
            after.script_events.variables.get("VAR_KURT_APRICORNS"),
            after.script_events.last_special_routine
        );
    }

    println!(
        "smoke-kurt-apricorn spawn={} apricorn={} ball={} quantity={} grant_checksum={:?} final_checksum={:?}",
        spawn_identifier,
        apricorn_id,
        ball_id,
        SMOKE_APRICORN_QUANTITY,
        grant.state_checksum,
        after.state_checksum,
    );
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save Kurt apricorn smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed Kurt apricorn smoke save")?;
        if resumed_snapshot.state_checksum != after.state_checksum {
            bail!(
                "Kurt apricorn smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                after.state_checksum
            );
        }
        if resumed_snapshot.bag != after.bag
            || resumed_snapshot.script_events.script_value != after.script_events.script_value
            || resumed_snapshot.script_events.last_special_routine
                != after.script_events.last_special_routine
        {
            bail!(
                "Kurt apricorn smoke resumed state did not match final state: resumed quantity={} script={:?}/{:?} final quantity={} script={:?}/{:?}",
                runtime_bag_quantity(&resumed_snapshot.bag, &apricorn_id),
                resumed_snapshot.script_events.script_value,
                resumed_snapshot.script_events.last_special_routine,
                runtime_bag_quantity(&after.bag, &apricorn_id),
                after.script_events.script_value,
                after.script_events.last_special_routine
            );
        }
        println!(
            "smoke-kurt-apricorn-save path={} saved_frame={} resumed_quantity={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            runtime_bag_quantity(&resumed_snapshot.bag, &apricorn_id),
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn smoke_bills_grandfather(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    smoke_start_map: Option<&SmokeStartMapRef>,
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    let mut shell = if let Some(start_map) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            start_map.map_name.clone(),
            start_map.tile_x,
            start_map.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let before = shell
        .snapshot()
        .context("snapshot runtime shell before Bill's Grandfather")?;
    let species_id = before
        .pokemon
        .iter()
        .find(|pokemon| pokemon.species_id != "EGG")
        .map(|pokemon| pokemon.species_id.clone())
        .context("--smoke-bills-grandfather requires at least one compiled Pokemon species")?;
    if !before.party.slots.is_empty() || before.storage.party_count != 0 {
        bail!(
            "new runtime shell already had party slots={} storage party_count={}",
            before.party.slots.len(),
            before.storage.party_count
        );
    }

    let party = [SmokePartyPokemonRef {
        species_id: species_id.clone(),
        level: 15,
        held_item_id: None,
    }];
    let grants = grant_smoke_party(&mut shell, &party, "Bill's Grandfather")?;
    let grant = grants
        .first()
        .context("Bill's Grandfather party grant did not return a grant")?;
    let seeded = shell
        .snapshot()
        .context("snapshot runtime shell after Bill's Grandfather party grant")?;
    if seeded.party.slots.len() != 1 || seeded.storage.party_count != 1 {
        bail!(
            "Bill's Grandfather party seed slots={} storage party_count={} did not become one",
            seeded.party.slots.len(),
            seeded.storage.party_count
        );
    }

    let used = shell
        .use_bills_grandfather(Some(0), None)
        .context("run Bill's Grandfather party selection")?;
    let after = shell
        .snapshot()
        .context("snapshot runtime shell after Bill's Grandfather")?;
    let SpecialRoutineEffect::BillsGrandfather {
        party_slot,
        species,
    } = &used.outcome.effect
    else {
        bail!(
            "BillsGrandfather returned unexpected effect {:?}",
            used.outcome.effect
        );
    };
    if *party_slot != Some(0) || species.as_deref() != Some(species_id.as_str()) {
        bail!(
            "BillsGrandfather effect slot={:?} species={:?} did not match slot 0 species {}",
            party_slot,
            species,
            species_id
        );
    }
    let display_species = species_id.replace('_', " ");
    if after.script_events.script_value.as_deref() != Some(species_id.as_str())
        || after
            .script_events
            .variables
            .get("wCurPartySpecies")
            .map(String::as_str)
            != Some(species_id.as_str())
        || after
            .script_events
            .variables
            .get("wNamedObjectIndex")
            .map(String::as_str)
            != Some(species_id.as_str())
        || after
            .script_events
            .variables
            .get("_value")
            .map(String::as_str)
            != Some(species_id.as_str())
        || after
            .script_events
            .named_buffers
            .get("STRING_BUFFER_1")
            .map(String::as_str)
            != Some(display_species.as_str())
        || after
            .script_events
            .named_buffers
            .get("STRING_BUFFER_3")
            .map(String::as_str)
            != Some(display_species.as_str())
        || after.script_events.last_special_routine.as_deref() != Some("BillsGrandfather")
    {
        bail!(
            "Bill's Grandfather script state value={:?} species={:?} object={:?} buffer1={:?} buffer3={:?} last_special={:?} did not match {}",
            after.script_events.script_value,
            after.script_events.variables.get("wCurPartySpecies"),
            after.script_events.variables.get("wNamedObjectIndex"),
            after.script_events.named_buffers.get("STRING_BUFFER_1"),
            after.script_events.named_buffers.get("STRING_BUFFER_3"),
            after.script_events.last_special_routine,
            species_id
        );
    }

    println!(
        "smoke-bills-grandfather spawn={} species={} display_species={} party_slot=0 grant_checksum={:?} final_checksum={:?}",
        spawn_identifier, species_id, display_species, grant.state_checksum, after.state_checksum,
    );
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save Bill's Grandfather smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed Bill's Grandfather smoke save")?;
        if resumed_snapshot.state_checksum != after.state_checksum {
            bail!(
                "Bill's Grandfather smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                after.state_checksum
            );
        }
        if resumed_snapshot.party != after.party
            || resumed_snapshot.storage.party_count != after.storage.party_count
            || resumed_snapshot.script_events.script_value != after.script_events.script_value
            || resumed_snapshot.script_events.variables != after.script_events.variables
            || resumed_snapshot.script_events.named_buffers != after.script_events.named_buffers
            || resumed_snapshot.script_events.last_special_routine
                != after.script_events.last_special_routine
        {
            bail!(
                "Bill's Grandfather smoke resumed state did not match final state: resumed party_count={} script={:?}/{:?} final party_count={} script={:?}/{:?}",
                resumed_snapshot.storage.party_count,
                resumed_snapshot.script_events.script_value,
                resumed_snapshot.script_events.last_special_routine,
                after.storage.party_count,
                after.script_events.script_value,
                after.script_events.last_special_routine
            );
        }
        println!(
            "smoke-bills-grandfather-save path={} saved_frame={} resumed_party_count={} resumed_species={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot.storage.party_count,
            resumed_snapshot
                .script_events
                .variables
                .get("wCurPartySpecies")
                .map(String::as_str)
                .unwrap_or(""),
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn smoke_unown_printer(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    smoke_start_map: Option<&SmokeStartMapRef>,
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    let mut shell = if let Some(start_map) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            start_map.map_name.clone(),
            start_map.tile_x,
            start_map.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let before = shell
        .snapshot()
        .context("snapshot runtime shell before Unown Printer")?;
    if before.script_events.active_menu.is_some() {
        bail!(
            "new runtime shell already had active menu {:?}",
            before.script_events.active_menu
        );
    }

    let opened = shell
        .open_unown_printer_special()
        .context("open Unown Printer special")?;
    let after = shell
        .snapshot()
        .context("snapshot runtime shell after Unown Printer")?;
    let SpecialRoutineEffect::UnownPrinter { unlocked } = &opened.outcome.effect else {
        bail!(
            "UnownPrinter returned unexpected effect {:?}",
            opened.outcome.effect
        );
    };
    if !*unlocked {
        bail!("UnownPrinter effect did not report unlocked=true");
    }
    if after.script_events.active_menu.as_deref() != Some("UnownPrinter")
        || after.script_events.script_value.as_deref() != Some("1")
        || after
            .script_events
            .variables
            .get("_unown_printer_unlocked")
            .map(String::as_str)
            != Some("1")
        || after.script_events.last_special_routine.as_deref() != Some("UnownPrinter")
    {
        bail!(
            "Unown Printer script state menu={:?} value={:?} unlocked={:?} last_special={:?} did not match",
            after.script_events.active_menu,
            after.script_events.script_value,
            after.script_events.variables.get("_unown_printer_unlocked"),
            after.script_events.last_special_routine
        );
    }

    println!(
        "smoke-unown-printer spawn={} active_menu={} unlocked={} before_checksum={:?} final_checksum={:?}",
        spawn_identifier,
        after.script_events.active_menu.as_deref().unwrap_or("none"),
        unlocked,
        before.state_checksum,
        after.state_checksum,
    );
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save Unown Printer smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed Unown Printer smoke save")?;
        if resumed_snapshot.state_checksum != after.state_checksum {
            bail!(
                "Unown Printer smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                after.state_checksum
            );
        }
        if resumed_snapshot.script_events.active_menu != after.script_events.active_menu
            || resumed_snapshot.script_events.script_value != after.script_events.script_value
            || resumed_snapshot.script_events.variables != after.script_events.variables
            || resumed_snapshot.script_events.last_special_routine
                != after.script_events.last_special_routine
        {
            bail!(
                "Unown Printer smoke resumed state did not match final state: resumed menu={:?} script={:?}/{:?} final menu={:?} script={:?}/{:?}",
                resumed_snapshot.script_events.active_menu,
                resumed_snapshot.script_events.script_value,
                resumed_snapshot.script_events.last_special_routine,
                after.script_events.active_menu,
                after.script_events.script_value,
                after.script_events.last_special_routine
            );
        }
        println!(
            "smoke-unown-printer-save path={} saved_frame={} resumed_menu={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot
                .script_events
                .active_menu
                .as_deref()
                .unwrap_or("none"),
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn smoke_map_radio(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    smoke_start_map: Option<&SmokeStartMapRef>,
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    const SMOKE_RADIO_STATION: &str = "POKE_FLUTE_RADIO";
    let mut shell = if let Some(start_map) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            start_map.map_name.clone(),
            start_map.tile_x,
            start_map.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let before = shell
        .snapshot()
        .context("snapshot runtime shell before Map Radio")?;
    if before.script_events.active_menu.is_some() {
        bail!(
            "new runtime shell already had active menu {:?}",
            before.script_events.active_menu
        );
    }

    let opened = shell
        .open_map_radio_special(SMOKE_RADIO_STATION.to_string())
        .context("open Map Radio special")?;
    let after = shell
        .snapshot()
        .context("snapshot runtime shell after Map Radio")?;
    let SpecialRoutineEffect::MapRadio { station } = &opened.outcome.effect else {
        bail!(
            "MapRadio returned unexpected effect {:?}",
            opened.outcome.effect
        );
    };
    if station != SMOKE_RADIO_STATION {
        bail!(
            "MapRadio effect station {} did not match {}",
            station,
            SMOKE_RADIO_STATION
        );
    }
    if after.script_events.active_menu.as_deref() != Some("MapRadio")
        || after.script_events.script_value.as_deref() != Some(SMOKE_RADIO_STATION)
        || after
            .script_events
            .variables
            .get("_map_radio_station")
            .map(String::as_str)
            != Some(SMOKE_RADIO_STATION)
        || after.script_events.last_special_routine.as_deref() != Some("MapRadio")
    {
        bail!(
            "Map Radio script state menu={:?} value={:?} station={:?} last_special={:?} did not match {}",
            after.script_events.active_menu,
            after.script_events.script_value,
            after.script_events.variables.get("_map_radio_station"),
            after.script_events.last_special_routine,
            SMOKE_RADIO_STATION
        );
    }

    println!(
        "smoke-map-radio spawn={} active_menu={} station={} before_checksum={:?} final_checksum={:?}",
        spawn_identifier,
        after.script_events.active_menu.as_deref().unwrap_or("none"),
        station,
        before.state_checksum,
        after.state_checksum,
    );
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save Map Radio smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed Map Radio smoke save")?;
        if resumed_snapshot.state_checksum != after.state_checksum {
            bail!(
                "Map Radio smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                after.state_checksum
            );
        }
        if resumed_snapshot.script_events.active_menu != after.script_events.active_menu
            || resumed_snapshot.script_events.script_value != after.script_events.script_value
            || resumed_snapshot.script_events.variables != after.script_events.variables
            || resumed_snapshot.script_events.last_special_routine
                != after.script_events.last_special_routine
        {
            bail!(
                "Map Radio smoke resumed state did not match final state: resumed menu={:?} script={:?}/{:?} final menu={:?} script={:?}/{:?}",
                resumed_snapshot.script_events.active_menu,
                resumed_snapshot.script_events.script_value,
                resumed_snapshot.script_events.last_special_routine,
                after.script_events.active_menu,
                after.script_events.script_value,
                after.script_events.last_special_routine
            );
        }
        println!(
            "smoke-map-radio-save path={} saved_frame={} resumed_menu={} resumed_station={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot
                .script_events
                .active_menu
                .as_deref()
                .unwrap_or("none"),
            resumed_snapshot
                .script_events
                .variables
                .get("_map_radio_station")
                .map(String::as_str)
                .unwrap_or(""),
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn smoke_name_rater(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    smoke_start_map: Option<&SmokeStartMapRef>,
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    const SMOKE_NICKNAME: &str = "RUSTY";
    let mut shell = if let Some(start_map) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            start_map.map_name.clone(),
            start_map.tile_x,
            start_map.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let before = shell
        .snapshot()
        .context("snapshot runtime shell before Name Rater")?;
    let species_id = before
        .pokemon
        .iter()
        .find(|pokemon| pokemon.species_id != "EGG")
        .map(|pokemon| pokemon.species_id.clone())
        .context("--smoke-name-rater requires at least one compiled Pokemon species")?;
    if !before.party.slots.is_empty() || before.storage.party_count != 0 {
        bail!(
            "new runtime shell already had party slots={} storage party_count={}",
            before.party.slots.len(),
            before.storage.party_count
        );
    }

    let party = [SmokePartyPokemonRef {
        species_id: species_id.clone(),
        level: 15,
        held_item_id: None,
    }];
    let grants = grant_smoke_party(&mut shell, &party, "Name Rater")?;
    let grant = grants
        .first()
        .context("Name Rater party grant did not return a grant")?;
    let seeded = shell
        .snapshot()
        .context("snapshot runtime shell after Name Rater party grant")?;
    let seeded_slot = seeded
        .party
        .slots
        .first()
        .context("Name Rater party seed did not populate slot 0")?;
    let old_nickname = seeded_slot.pokemon.nickname.clone();

    let rated = shell
        .rate_party_nickname_special(0, SMOKE_NICKNAME.to_string())
        .context("run Name Rater special")?;
    let after = shell
        .snapshot()
        .context("snapshot runtime shell after Name Rater")?;
    let SpecialRoutineEffect::NameRater {
        party_slot,
        species,
        old_nickname: effect_old_nickname,
        new_nickname,
    } = &rated.outcome.effect
    else {
        bail!(
            "NameRater returned unexpected effect {:?}",
            rated.outcome.effect
        );
    };
    if *party_slot != 0
        || species != &species_id
        || effect_old_nickname != &old_nickname
        || new_nickname != SMOKE_NICKNAME
    {
        bail!(
            "NameRater effect slot={} species={} old={:?} new={:?} did not match slot 0 species={} old={:?} new={}",
            party_slot,
            species,
            effect_old_nickname,
            new_nickname,
            species_id,
            old_nickname,
            SMOKE_NICKNAME
        );
    }
    let after_slot = after
        .party
        .slots
        .first()
        .context("Name Rater party slot disappeared after rename")?;
    if after_slot.pokemon.nickname != SMOKE_NICKNAME
        || after.script_events.script_value.as_deref() != Some(SMOKE_NICKNAME)
        || after
            .script_events
            .named_buffers
            .get("STRING_BUFFER_1")
            .map(String::as_str)
            != Some(SMOKE_NICKNAME)
        || after.script_events.last_special_routine.as_deref() != Some("NameRater")
    {
        bail!(
            "Name Rater state nickname={:?} value={:?} buffer={:?} last_special={:?} did not match {}",
            after_slot.pokemon.nickname,
            after.script_events.script_value,
            after.script_events.named_buffers.get("STRING_BUFFER_1"),
            after.script_events.last_special_routine,
            SMOKE_NICKNAME
        );
    }

    println!(
        "smoke-name-rater spawn={} species={} old_nickname={} new_nickname={} grant_checksum={:?} final_checksum={:?}",
        spawn_identifier,
        species_id,
        old_nickname,
        SMOKE_NICKNAME,
        grant.state_checksum,
        after.state_checksum,
    );
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save Name Rater smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed Name Rater smoke save")?;
        if resumed_snapshot.state_checksum != after.state_checksum {
            bail!(
                "Name Rater smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                after.state_checksum
            );
        }
        if resumed_snapshot.party != after.party
            || resumed_snapshot.script_events.script_value != after.script_events.script_value
            || resumed_snapshot.script_events.named_buffers != after.script_events.named_buffers
            || resumed_snapshot.script_events.last_special_routine
                != after.script_events.last_special_routine
        {
            bail!(
                "Name Rater smoke resumed state did not match final state: resumed nickname={:?} script={:?}/{:?} final nickname={:?} script={:?}/{:?}",
                resumed_snapshot
                    .party
                    .slots
                    .first()
                    .map(|slot| slot.pokemon.nickname.as_str()),
                resumed_snapshot.script_events.script_value,
                resumed_snapshot.script_events.last_special_routine,
                after
                    .party
                    .slots
                    .first()
                    .map(|slot| slot.pokemon.nickname.as_str()),
                after.script_events.script_value,
                after.script_events.last_special_routine
            );
        }
        println!(
            "smoke-name-rater-save path={} saved_frame={} resumed_nickname={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot
                .party
                .slots
                .first()
                .map(|slot| slot.pokemon.nickname.as_str())
                .unwrap_or(""),
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn smoke_poke_seer(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    smoke_start_map: Option<&SmokeStartMapRef>,
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    let mut shell = if let Some(start_map) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            start_map.map_name.clone(),
            start_map.tile_x,
            start_map.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let before = shell
        .snapshot()
        .context("snapshot runtime shell before Poke Seer")?;
    let species_id = before
        .pokemon
        .iter()
        .find(|pokemon| pokemon.species_id != "EGG")
        .map(|pokemon| pokemon.species_id.clone())
        .context("--smoke-poke-seer requires at least one compiled Pokemon species")?;
    if !before.party.slots.is_empty() || before.storage.party_count != 0 {
        bail!(
            "new runtime shell already had party slots={} storage party_count={}",
            before.party.slots.len(),
            before.storage.party_count
        );
    }

    let party = [SmokePartyPokemonRef {
        species_id: species_id.clone(),
        level: 15,
        held_item_id: None,
    }];
    let grants = grant_smoke_party(&mut shell, &party, "Poke Seer")?;
    let grant = grants
        .first()
        .context("Poke Seer party grant did not return a grant")?;
    let seeded = shell
        .snapshot()
        .context("snapshot runtime shell after Poke Seer party grant")?;
    let seeded_slot = seeded
        .party
        .slots
        .first()
        .context("Poke Seer party seed did not populate slot 0")?;
    let expected_nickname = if seeded_slot.pokemon.nickname.is_empty() {
        seeded_slot.pokemon.species.id.clone()
    } else {
        seeded_slot.pokemon.nickname.clone()
    };
    let expected_ot_name = seeded_slot.pokemon.original_trainer_name.clone();
    let expected_ot_id = seeded_slot.pokemon.original_trainer_id;

    let seen = shell
        .see_party_pokemon_special(0)
        .context("run Poke Seer special")?;
    let after = shell
        .snapshot()
        .context("snapshot runtime shell after Poke Seer")?;
    let SpecialRoutineEffect::PokeSeer {
        party_slot,
        species,
        nickname,
        original_trainer_name,
        original_trainer_id,
    } = &seen.outcome.effect
    else {
        bail!(
            "PokeSeer returned unexpected effect {:?}",
            seen.outcome.effect
        );
    };
    if *party_slot != 0
        || species != &species_id
        || nickname != &expected_nickname
        || original_trainer_name != &expected_ot_name
        || *original_trainer_id != expected_ot_id
    {
        bail!(
            "PokeSeer effect slot={} species={} nickname={} ot={} ot_id={} did not match slot 0 species={} nickname={} ot={} ot_id={}",
            party_slot,
            species,
            nickname,
            original_trainer_name,
            original_trainer_id,
            species_id,
            expected_nickname,
            expected_ot_name,
            expected_ot_id
        );
    }
    let expected_ot_id_value = expected_ot_id.to_string();
    if after.script_events.script_value.as_deref() != Some("1")
        || after
            .script_events
            .named_buffers
            .get("STRING_BUFFER_1")
            .map(String::as_str)
            != Some(expected_nickname.as_str())
        || after
            .script_events
            .named_buffers
            .get("STRING_BUFFER_2")
            .map(String::as_str)
            != Some(expected_ot_name.as_str())
        || after
            .script_events
            .variables
            .get("_poke_seer_ot_id")
            .map(String::as_str)
            != Some(expected_ot_id_value.as_str())
        || after.script_events.last_special_routine.as_deref() != Some("PokeSeer")
    {
        bail!(
            "Poke Seer script state value={:?} nick_buffer={:?} ot_buffer={:?} ot_id={:?} last_special={:?} did not match",
            after.script_events.script_value,
            after.script_events.named_buffers.get("STRING_BUFFER_1"),
            after.script_events.named_buffers.get("STRING_BUFFER_2"),
            after.script_events.variables.get("_poke_seer_ot_id"),
            after.script_events.last_special_routine
        );
    }

    println!(
        "smoke-poke-seer spawn={} species={} nickname={} ot={} ot_id={} grant_checksum={:?} final_checksum={:?}",
        spawn_identifier,
        species_id,
        expected_nickname,
        expected_ot_name,
        expected_ot_id,
        grant.state_checksum,
        after.state_checksum,
    );
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save Poke Seer smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed Poke Seer smoke save")?;
        if resumed_snapshot.state_checksum != after.state_checksum {
            bail!(
                "Poke Seer smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                after.state_checksum
            );
        }
        if resumed_snapshot.party != after.party
            || resumed_snapshot.script_events.script_value != after.script_events.script_value
            || resumed_snapshot.script_events.variables != after.script_events.variables
            || resumed_snapshot.script_events.named_buffers != after.script_events.named_buffers
            || resumed_snapshot.script_events.last_special_routine
                != after.script_events.last_special_routine
        {
            bail!(
                "Poke Seer smoke resumed state did not match final state: resumed script={:?}/{:?} final script={:?}/{:?}",
                resumed_snapshot.script_events.script_value,
                resumed_snapshot.script_events.last_special_routine,
                after.script_events.script_value,
                after.script_events.last_special_routine
            );
        }
        println!(
            "smoke-poke-seer-save path={} saved_frame={} resumed_nickname={} resumed_ot={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot
                .script_events
                .named_buffers
                .get("STRING_BUFFER_1")
                .map(String::as_str)
                .unwrap_or(""),
            resumed_snapshot
                .script_events
                .named_buffers
                .get("STRING_BUFFER_2")
                .map(String::as_str)
                .unwrap_or(""),
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn runtime_bag_quantity(bag: &crystal_bevy::RuntimeBagSnapshot, item_id: &str) -> u16 {
    bag.items
        .iter()
        .chain(bag.balls.iter())
        .chain(bag.key_items.iter())
        .chain(bag.pc_items.iter())
        .chain(bag.custom_pockets.values().flat_map(|items| items.iter()))
        .find_map(|item| (item.item_id == item_id).then_some(item.quantity))
        .unwrap_or(0)
}

fn smoke_bank_of_mom(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    smoke_start_map: Option<&SmokeStartMapRef>,
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    const SMOKE_MONEY: u32 = 1200;
    let mut shell = if let Some(start_map) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            start_map.map_name.clone(),
            start_map.tile_x,
            start_map.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let before = shell
        .snapshot()
        .context("snapshot runtime shell before Bank of Mom")?;
    if before.trainer.money != 0 || before.trainer.moms_money != 0 {
        bail!(
            "new runtime shell already had money={} moms_money={}",
            before.trainer.money,
            before.trainer.moms_money
        );
    }
    let money = shell
        .add_currency(RuntimeCurrencyAccount::Money, SMOKE_MONEY)
        .context("seed Bank of Mom smoke money")?;
    let after_money = shell
        .snapshot()
        .context("snapshot runtime shell after Bank of Mom money seed")?;
    if after_money.trainer.money != SMOKE_MONEY || after_money.trainer.moms_money != 0 {
        bail!(
            "Bank of Mom money seed produced money={} moms_money={}",
            after_money.trainer.money,
            after_money.trainer.moms_money
        );
    }
    let opened = shell
        .open_bank_of_mom_special()
        .context("open Bank of Mom special")?;
    let after = shell
        .snapshot()
        .context("snapshot runtime shell after Bank of Mom")?;
    let SpecialRoutineEffect::BankOfMom {
        money: effect_money,
        moms_money: effect_moms_money,
    } = &opened.outcome.effect
    else {
        bail!(
            "BankOfMom returned unexpected effect {:?}",
            opened.outcome.effect
        );
    };
    if *effect_money != SMOKE_MONEY || *effect_moms_money != 0 {
        bail!(
            "Bank of Mom effect money={} moms_money={} did not match smoke state",
            effect_money,
            effect_moms_money
        );
    }
    if after.trainer.money != SMOKE_MONEY || after.trainer.moms_money != 0 {
        bail!(
            "Bank of Mom mutated balances money={} moms_money={}",
            after.trainer.money,
            after.trainer.moms_money
        );
    }
    if after.script_events.active_menu.as_deref() != Some("BankOfMom") {
        bail!(
            "Bank of Mom active menu {:?} did not match BankOfMom",
            after.script_events.active_menu
        );
    }
    if after.script_events.script_value.as_deref() != Some("1") {
        bail!(
            "Bank of Mom script value {:?} did not report success",
            after.script_events.script_value
        );
    }
    if after
        .script_events
        .variables
        .get("_bank_money")
        .map(String::as_str)
        != Some("1200")
    {
        bail!(
            "Bank of Mom _bank_money {:?} did not match 1200",
            after.script_events.variables.get("_bank_money")
        );
    }
    if after
        .script_events
        .variables
        .get("_mom_money")
        .map(String::as_str)
        != Some("0")
    {
        bail!(
            "Bank of Mom _mom_money {:?} did not match 0",
            after.script_events.variables.get("_mom_money")
        );
    }
    println!(
        "smoke-bank-of-mom spawn={} money={} moms_money={} before_checksum={:?} money_checksum={:?} final_checksum={:?}",
        spawn_identifier,
        effect_money,
        effect_moms_money,
        before.state_checksum,
        money.state_checksum,
        after.state_checksum,
    );
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save Bank of Mom smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed Bank of Mom smoke save")?;
        if resumed_snapshot.state_checksum != after.state_checksum {
            bail!(
                "Bank of Mom smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                after.state_checksum
            );
        }
        if resumed_snapshot.trainer.money != after.trainer.money
            || resumed_snapshot.trainer.moms_money != after.trainer.moms_money
            || resumed_snapshot.script_events.active_menu != after.script_events.active_menu
        {
            bail!(
                "Bank of Mom smoke resumed money={} moms_money={} menu={:?} did not match final money={} moms_money={} menu={:?}",
                resumed_snapshot.trainer.money,
                resumed_snapshot.trainer.moms_money,
                resumed_snapshot.script_events.active_menu,
                after.trainer.money,
                after.trainer.moms_money,
                after.script_events.active_menu,
            );
        }
        println!(
            "smoke-bank-of-mom-save path={} saved_frame={} resumed_money={} resumed_moms_money={} resumed_menu={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot.trainer.money,
            resumed_snapshot.trainer.moms_money,
            resumed_snapshot
                .script_events
                .active_menu
                .as_deref()
                .unwrap_or("none"),
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn smoke_link_records(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    smoke_start_map: Option<&SmokeStartMapRef>,
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    let mut shell = if let Some(start_map) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            start_map.map_name.clone(),
            start_map.tile_x,
            start_map.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let before = shell
        .snapshot()
        .context("snapshot runtime shell before link records")?;
    if before.progression.link_wins != 0
        || before.progression.link_losses != 0
        || before.progression.link_draws != 0
        || before.progression.pending_special_battle_type.is_some()
    {
        bail!(
            "new runtime shell already had link stats wins={} losses={} draws={} pending_battle={:?}",
            before.progression.link_wins,
            before.progression.link_losses,
            before.progression.link_draws,
            before.progression.pending_special_battle_type
        );
    }

    let win = shell
        .record_link_battle_result(RuntimeLinkBattleResult::Win)
        .context("record link battle win")?;
    let loss = shell
        .record_link_battle_result(RuntimeLinkBattleResult::Loss)
        .context("record link battle loss")?;
    let draw = shell
        .record_link_battle_result(RuntimeLinkBattleResult::Draw)
        .context("record link battle draw")?;
    if win.wins_after != 1 || loss.losses_after != 1 || draw.draws_after != 1 {
        bail!(
            "link battle result counters were win={} loss={} draw={}",
            win.wins_after,
            loss.losses_after,
            draw.draws_after
        );
    }

    let display = shell
        .open_display_link_record_special()
        .context("open Display Link Record special")?;
    let after_display = shell
        .snapshot()
        .context("snapshot runtime shell after Display Link Record")?;
    let SpecialRoutineEffect::DisplayLinkRecord {
        wins: display_wins,
        losses: display_losses,
        draws: display_draws,
    } = &display.outcome.effect
    else {
        bail!(
            "DisplayLinkRecord returned unexpected effect {:?}",
            display.outcome.effect
        );
    };
    if (*display_wins, *display_losses, *display_draws) != (1, 1, 1) {
        bail!(
            "Display Link Record effect wins={} losses={} draws={} did not match smoke stats",
            display_wins,
            display_losses,
            display_draws
        );
    }
    if after_display.script_events.active_menu.as_deref() != Some("DisplayLinkRecord") {
        bail!(
            "Display Link Record active menu {:?} did not match DisplayLinkRecord",
            after_display.script_events.active_menu
        );
    }
    if after_display.script_events.script_value.as_deref() != Some("1") {
        bail!(
            "Display Link Record script value {:?} did not report success",
            after_display.script_events.script_value
        );
    }
    if after_display
        .script_events
        .variables
        .get("_link_battle_wins")
        .map(String::as_str)
        != Some("1")
        || after_display
            .script_events
            .variables
            .get("_link_battle_losses")
            .map(String::as_str)
            != Some("1")
        || after_display
            .script_events
            .variables
            .get("_link_battle_draws")
            .map(String::as_str)
            != Some("1")
    {
        bail!(
            "Display Link Record variables did not match wins/losses/draws: {:?}",
            after_display.script_events.variables
        );
    }

    let trainer_house = shell
        .open_trainer_house_special()
        .context("open Trainer House special")?;
    let after = shell
        .snapshot()
        .context("snapshot runtime shell after Trainer House")?;
    let SpecialRoutineEffect::TrainerHouse {
        wins: trainer_wins,
        losses: trainer_losses,
        draws: trainer_draws,
    } = &trainer_house.outcome.effect
    else {
        bail!(
            "TrainerHouse returned unexpected effect {:?}",
            trainer_house.outcome.effect
        );
    };
    if (*trainer_wins, *trainer_losses, *trainer_draws) != (1, 1, 1) {
        bail!(
            "Trainer House effect wins={} losses={} draws={} did not match smoke stats",
            trainer_wins,
            trainer_losses,
            trainer_draws
        );
    }
    if after.progression.link_wins != 1
        || after.progression.link_losses != 1
        || after.progression.link_draws != 1
    {
        bail!(
            "link record stats after Trainer House were wins={} losses={} draws={}",
            after.progression.link_wins,
            after.progression.link_losses,
            after.progression.link_draws
        );
    }
    if after.progression.pending_special_battle_type.as_deref() != Some("BATTLETYPE_TRAINER_HOUSE")
    {
        bail!(
            "Trainer House pending battle {:?} did not match BATTLETYPE_TRAINER_HOUSE",
            after.progression.pending_special_battle_type
        );
    }
    if after.script_events.active_menu.as_deref() != Some("TrainerHouse") {
        bail!(
            "Trainer House active menu {:?} did not match TrainerHouse",
            after.script_events.active_menu
        );
    }
    if after.script_events.script_value.as_deref() != Some("1") {
        bail!(
            "Trainer House script value {:?} did not report success",
            after.script_events.script_value
        );
    }
    if after
        .script_events
        .variables
        .get("_trainer_house_wins")
        .map(String::as_str)
        != Some("1")
        || after
            .script_events
            .variables
            .get("_trainer_house_losses")
            .map(String::as_str)
            != Some("1")
        || after
            .script_events
            .variables
            .get("_trainer_house_draws")
            .map(String::as_str)
            != Some("1")
    {
        bail!(
            "Trainer House variables did not match wins/losses/draws: {:?}",
            after.script_events.variables
        );
    }

    println!(
        "smoke-link-records spawn={} wins={} losses={} draws={} display_menu={} trainer_menu={} pending_battle={} before_checksum={:?} record_checksum={:?} display_checksum={:?} final_checksum={:?}",
        spawn_identifier,
        after.progression.link_wins,
        after.progression.link_losses,
        after.progression.link_draws,
        after_display
            .script_events
            .active_menu
            .as_deref()
            .unwrap_or("none"),
        after.script_events.active_menu.as_deref().unwrap_or("none"),
        after
            .progression
            .pending_special_battle_type
            .as_deref()
            .unwrap_or("none"),
        before.state_checksum,
        draw.state_checksum,
        display.state_checksum,
        after.state_checksum,
    );
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save link records smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed link records smoke save")?;
        if resumed_snapshot.state_checksum != after.state_checksum {
            bail!(
                "link records smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                after.state_checksum
            );
        }
        if resumed_snapshot.progression.link_wins != after.progression.link_wins
            || resumed_snapshot.progression.link_losses != after.progression.link_losses
            || resumed_snapshot.progression.link_draws != after.progression.link_draws
            || resumed_snapshot.progression.pending_special_battle_type
                != after.progression.pending_special_battle_type
            || resumed_snapshot.script_events.active_menu != after.script_events.active_menu
        {
            bail!(
                "link records smoke resumed stats/menu did not match final stats/menu: resumed=({}, {}, {}, {:?}, {:?}) final=({}, {}, {}, {:?}, {:?})",
                resumed_snapshot.progression.link_wins,
                resumed_snapshot.progression.link_losses,
                resumed_snapshot.progression.link_draws,
                resumed_snapshot.progression.pending_special_battle_type,
                resumed_snapshot.script_events.active_menu,
                after.progression.link_wins,
                after.progression.link_losses,
                after.progression.link_draws,
                after.progression.pending_special_battle_type,
                after.script_events.active_menu,
            );
        }
        println!(
            "smoke-link-records-save path={} saved_frame={} resumed_wins={} resumed_losses={} resumed_draws={} resumed_pending_battle={} resumed_menu={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot.progression.link_wins,
            resumed_snapshot.progression.link_losses,
            resumed_snapshot.progression.link_draws,
            resumed_snapshot
                .progression
                .pending_special_battle_type
                .as_deref()
                .unwrap_or("none"),
            resumed_snapshot
                .script_events
                .active_menu
                .as_deref()
                .unwrap_or("none"),
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn smoke_link_rooms(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    smoke_start_map: Option<&SmokeStartMapRef>,
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    let mut shell = if let Some(start_map) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            start_map.map_name.clone(),
            start_map.tile_x,
            start_map.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let before = shell
        .snapshot()
        .context("snapshot runtime shell before link rooms")?;
    if before.link_session.link_mode != 0 || before.link_session.active_room.is_some() {
        bail!(
            "new runtime shell already had link_mode={} active_room={:?}",
            before.link_session.link_mode,
            before.link_session.active_room
        );
    }

    let trade = smoke_open_link_room(
        &mut shell,
        RuntimeLinkRoomSpecial::TradeCenter,
        "TradeCenter",
        2,
        "TradeCenter",
    )?;
    let colosseum = smoke_open_link_room(
        &mut shell,
        RuntimeLinkRoomSpecial::Colosseum,
        "Colosseum",
        3,
        "Colosseum",
    )?;
    let capsule = smoke_open_link_room(
        &mut shell,
        RuntimeLinkRoomSpecial::TimeCapsule,
        "TimeCapsule",
        1,
        "EnterTimeCapsule",
    )?;

    let after = shell
        .snapshot()
        .context("snapshot runtime shell after link rooms")?;
    if after.link_session.link_mode != 1
        || after.link_session.active_room.as_deref() != Some("TimeCapsule")
        || after.script_events.last_special_routine.as_deref() != Some("EnterTimeCapsule")
    {
        bail!(
            "link room final state mode={} active_room={:?} last_special={:?} did not match Time Capsule",
            after.link_session.link_mode,
            after.link_session.active_room,
            after.script_events.last_special_routine
        );
    }

    println!(
        "smoke-link-rooms spawn={} trade_mode={} colosseum_mode={} capsule_mode={} final_room={} before_checksum={:?} trade_checksum={:?} colosseum_checksum={:?} final_checksum={:?}",
        spawn_identifier,
        2,
        3,
        after.link_session.link_mode,
        after.link_session.active_room.as_deref().unwrap_or("none"),
        before.state_checksum,
        trade.state_checksum,
        colosseum.state_checksum,
        capsule.state_checksum,
    );
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save link rooms smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed link rooms smoke save")?;
        if resumed_snapshot.state_checksum != after.state_checksum {
            bail!(
                "link rooms smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                after.state_checksum
            );
        }
        if resumed_snapshot.link_session != after.link_session
            || resumed_snapshot.script_events.last_special_routine
                != after.script_events.last_special_routine
        {
            bail!(
                "link rooms smoke resumed link state {:?}/{:?} did not match final {:?}/{:?}",
                resumed_snapshot.link_session,
                resumed_snapshot.script_events.last_special_routine,
                after.link_session,
                after.script_events.last_special_routine
            );
        }
        println!(
            "smoke-link-rooms-save path={} saved_frame={} resumed_mode={} resumed_room={} resumed_last_special={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot.link_session.link_mode,
            resumed_snapshot
                .link_session
                .active_room
                .as_deref()
                .unwrap_or("none"),
            resumed_snapshot
                .script_events
                .last_special_routine
                .as_deref()
                .unwrap_or("none"),
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn smoke_open_link_room(
    shell: &mut RuntimeGameShell,
    room: RuntimeLinkRoomSpecial,
    expected_room: &str,
    expected_link_mode: u8,
    expected_routine: &str,
) -> Result<crystal_bevy::RuntimeSpecialRoutineUse> {
    let opened = shell
        .open_link_room_special(room)
        .with_context(|| format!("open link room special {expected_room}"))?;
    let snapshot = shell
        .snapshot()
        .with_context(|| format!("snapshot link room special {expected_room}"))?;
    let SpecialRoutineEffect::LinkRoom {
        room: effect_room,
        link_mode,
    } = &opened.outcome.effect
    else {
        bail!(
            "link room {expected_room} returned unexpected effect {:?}",
            opened.outcome.effect
        );
    };
    if effect_room != expected_room || *link_mode != expected_link_mode {
        bail!(
            "link room effect room={} mode={} did not match {} mode {}",
            effect_room,
            link_mode,
            expected_room,
            expected_link_mode
        );
    }
    if snapshot.link_session.active_room.as_deref() != Some(expected_room)
        || snapshot.link_session.link_mode != expected_link_mode
    {
        bail!(
            "link room state active_room={:?} mode={} did not match {} mode {}",
            snapshot.link_session.active_room,
            snapshot.link_session.link_mode,
            expected_room,
            expected_link_mode
        );
    }
    if snapshot.script_events.script_value.as_deref() != Some("1")
        || snapshot.script_events.last_special_routine.as_deref() != Some(expected_routine)
    {
        bail!(
            "link room script state value={:?} last_special={:?} did not match success routine {}",
            snapshot.script_events.script_value,
            snapshot.script_events.last_special_routine,
            expected_routine
        );
    }
    Ok(opened)
}

fn smoke_link_handshake(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    smoke_start_map: Option<&SmokeStartMapRef>,
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    let mut shell = if let Some(start_map) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            start_map.map_name.clone(),
            start_map.tile_x,
            start_map.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let before = shell
        .snapshot()
        .context("snapshot runtime shell before link handshake")?;
    if before.link_session.player_link_action != 0
        || before.link_session.chosen_cable_club_room != 0
        || before.link_session.link_mode != 0
    {
        bail!(
            "new runtime shell already had link action={} room={} mode={}",
            before.link_session.player_link_action,
            before.link_session.chosen_cable_club_room,
            before.link_session.link_mode
        );
    }

    let trade_request = shell
        .set_cable_club_request(RuntimeCableClubRequest::Trade)
        .context("set link trade request")?;
    assert_link_action_effect(&trade_request, 1, 1, "SetBitsForLinkTradeRequest")?;
    let after_trade_request = shell
        .snapshot()
        .context("snapshot runtime shell after trade request")?;
    if after_trade_request.link_session.player_link_action != 1
        || after_trade_request.link_session.chosen_cable_club_room != 1
        || after_trade_request.script_events.script_value.as_deref() != Some("1")
    {
        bail!(
            "trade request state action={} room={} script_value={:?} did not match request",
            after_trade_request.link_session.player_link_action,
            after_trade_request.link_session.chosen_cable_club_room,
            after_trade_request.script_events.script_value
        );
    }

    let friend = shell
        .wait_for_linked_friend_special(true)
        .context("wait for linked friend")?;
    assert_link_result_effect(&friend, true, 0, "WaitForLinkedFriend")?;
    let after_friend = shell
        .snapshot()
        .context("snapshot runtime shell after linked friend")?;
    if !after_friend.link_session.friend_ready
        || !after_friend.link_session.last_result
        || after_friend.script_events.script_value.as_deref() != Some("1")
    {
        bail!(
            "linked friend state ready={} result={} script_value={:?} did not report success",
            after_friend.link_session.friend_ready,
            after_friend.link_session.last_result,
            after_friend.script_events.script_value
        );
    }

    let same_room = shell
        .check_both_selected_same_room_special(1)
        .context("check both players selected same room")?;
    assert_link_result_effect(&same_room, true, 2, "CheckBothSelectedSameRoom")?;
    let after_same_room = shell
        .snapshot()
        .context("snapshot runtime shell after same room check")?;
    if after_same_room.link_session.link_mode != 2 || !after_same_room.link_session.last_result {
        bail!(
            "same-room state mode={} result={} did not report linked trade mode",
            after_same_room.link_session.link_mode,
            after_same_room.link_session.last_result
        );
    }

    let connected = shell
        .check_link_timeout_receptionist_special(false, 2)
        .context("check non-timeout link receptionist")?;
    assert_link_result_effect(&connected, true, 2, "CheckLinkTimeout_Receptionist")?;
    let after_connected = shell
        .snapshot()
        .context("snapshot runtime shell after non-timeout link receptionist")?;
    if after_connected.link_session.other_player_link_mode != 2
        || after_connected.link_session.player_link_action != 1
        || !after_connected.link_session.last_result
    {
        bail!(
            "connected link state other_mode={} action={} result={} did not match peer inputs",
            after_connected.link_session.other_player_link_mode,
            after_connected.link_session.player_link_action,
            after_connected.link_session.last_result
        );
    }

    let close = shell.close_link_special().context("close link")?;
    assert_link_result_effect(&close, false, 0, "CloseLink")?;
    let after_close = shell
        .snapshot()
        .context("snapshot runtime shell after close link")?;
    if after_close.link_session.link_mode != 0
        || after_close.link_session.chosen_cable_club_room != 0
        || after_close.link_session.friend_ready
    {
        bail!(
            "close link state mode={} room={} ready={} did not reset session",
            after_close.link_session.link_mode,
            after_close.link_session.chosen_cable_club_room,
            after_close.link_session.friend_ready
        );
    }

    let battle_request = shell
        .set_cable_club_request(RuntimeCableClubRequest::Battle)
        .context("set link battle request")?;
    assert_link_action_effect(&battle_request, 2, 2, "SetBitsForBattleRequest")?;
    let timeout = shell
        .check_link_timeout_receptionist_special(true, 0)
        .context("check timeout link receptionist")?;
    assert_link_result_effect(&timeout, false, 0, "CheckLinkTimeout_Receptionist")?;
    let after_timeout = shell
        .snapshot()
        .context("snapshot runtime shell after link timeout")?;
    if after_timeout.link_session.chosen_cable_club_room != 0
        || after_timeout.link_session.player_link_action != 0
        || after_timeout.script_events.script_value.as_deref() != Some("0")
    {
        bail!(
            "timeout link state action={} room={} script_value={:?} did not reset request",
            after_timeout.link_session.player_link_action,
            after_timeout.link_session.chosen_cable_club_room,
            after_timeout.script_events.script_value
        );
    }

    let other_exit = shell
        .wait_for_other_player_to_exit_special()
        .context("wait for other player to exit")?;
    assert_link_result_effect(&other_exit, true, 0, "WaitForOtherPlayerToExit")?;
    let after_exit = shell
        .snapshot()
        .context("snapshot runtime shell after other player exit")?;
    if after_exit.link_session.link_mode != 0
        || after_exit.link_session.failed_link_to_past
        || after_exit.script_events.script_value.as_deref() != Some("1")
    {
        bail!(
            "other-player-exit state mode={} failed={} script_value={:?} did not reset cleanly",
            after_exit.link_session.link_mode,
            after_exit.link_session.failed_link_to_past,
            after_exit.script_events.script_value
        );
    }

    let failed = shell
        .failed_link_to_past_special()
        .context("apply failed link to past")?;
    assert_link_result_effect(&failed, false, 1, "FailedLinkToPast")?;
    let after_failed = shell
        .snapshot()
        .context("snapshot runtime shell after failed link to past")?;
    if !after_failed.link_session.failed_link_to_past || after_failed.link_session.link_mode != 1 {
        bail!(
            "failed link to past state failed={} mode={} did not match expected",
            after_failed.link_session.failed_link_to_past,
            after_failed.link_session.link_mode
        );
    }

    let ask = shell
        .ask_mobile_or_cable_special()
        .context("ask mobile or cable")?;
    let SpecialRoutineEffect::AskMobileOrCable { selection } = &ask.outcome.effect else {
        bail!(
            "AskMobileOrCable returned unexpected effect {:?}",
            ask.outcome.effect
        );
    };
    if selection != ".Cable" {
        bail!("AskMobileOrCable selection {selection} did not match .Cable");
    }
    let chris = shell
        .cable_club_check_which_chris_special("FEMALE".to_string())
        .context("check Cable Club Chris gender branch")?;
    let SpecialRoutineEffect::CableClubCheckWhichChris { male_player } = &chris.outcome.effect
    else {
        bail!(
            "CableClubCheckWhichChris returned unexpected effect {:?}",
            chris.outcome.effect
        );
    };
    if *male_player || shell.snapshot()?.script_events.script_value.as_deref() != Some("0") {
        bail!("CableClubCheckWhichChris FEMALE did not produce false script result");
    }

    let after = shell
        .snapshot()
        .context("snapshot runtime shell after link handshake")?;
    println!(
        "smoke-link-handshake spawn={} trade_action={} same_room_mode={} connected_other_mode={} timeout_room={} failed_mode={} final_gender_value={} before_checksum={:?} connected_checksum={:?} final_checksum={:?}",
        spawn_identifier,
        after_trade_request.link_session.player_link_action,
        after_same_room.link_session.link_mode,
        after_connected.link_session.other_player_link_mode,
        after_timeout.link_session.chosen_cable_club_room,
        after_failed.link_session.link_mode,
        after
            .script_events
            .script_value
            .as_deref()
            .unwrap_or("none"),
        before.state_checksum,
        connected.state_checksum,
        after.state_checksum,
    );
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save link handshake smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed link handshake smoke save")?;
        if resumed_snapshot.state_checksum != after.state_checksum {
            bail!(
                "link handshake smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                after.state_checksum
            );
        }
        if resumed_snapshot.link_session != after.link_session
            || resumed_snapshot.script_events.script_value != after.script_events.script_value
            || resumed_snapshot.script_events.last_special_routine
                != after.script_events.last_special_routine
        {
            bail!(
                "link handshake smoke resumed state did not match final state: resumed={:?}/{:?}/{:?} final={:?}/{:?}/{:?}",
                resumed_snapshot.link_session,
                resumed_snapshot.script_events.script_value,
                resumed_snapshot.script_events.last_special_routine,
                after.link_session,
                after.script_events.script_value,
                after.script_events.last_special_routine
            );
        }
        println!(
            "smoke-link-handshake-save path={} saved_frame={} resumed_mode={} resumed_room={} resumed_ready={} resumed_result={} resumed_script_value={} resumed_last_special={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot.link_session.link_mode,
            resumed_snapshot.link_session.chosen_cable_club_room,
            resumed_snapshot.link_session.friend_ready,
            resumed_snapshot.link_session.last_result,
            resumed_snapshot
                .script_events
                .script_value
                .as_deref()
                .unwrap_or("none"),
            resumed_snapshot
                .script_events
                .last_special_routine
                .as_deref()
                .unwrap_or("none"),
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn smoke_battle_tower(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    smoke_start_map: Option<&SmokeStartMapRef>,
    smoke_party: &[SmokePartyPokemonRef],
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    if smoke_party.len() < 3 {
        bail!("--smoke-battle-tower requires at least three --smoke-party entries");
    }
    let mut shell = if let Some(start_map) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            start_map.map_name.clone(),
            start_map.tile_x,
            start_map.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let party_grants = grant_smoke_party(&mut shell, smoke_party, "Battle Tower")?;
    let before = shell
        .snapshot()
        .context("snapshot runtime shell before Battle Tower smoke")?;
    if before.battle_tower.challenge_state != 0
        || before.mobile_link.handshakes != 0
        || before.mobile_link.terminated
    {
        bail!(
            "new runtime shell already had Battle Tower/mobile state challenge={} handshakes={} terminated={}",
            before.battle_tower.challenge_state,
            before.mobile_link.handshakes,
            before.mobile_link.terminated
        );
    }
    let opponent_trainer = before
        .trainers
        .iter()
        .find(|trainer| !trainer.party.is_empty())
        .context("Battle Tower smoke requires a compiled trainer with party Pokemon")?;
    let opponent_trainer_id = opponent_trainer.trainer_id.clone();
    let opponent_trainer_class = opponent_trainer.trainer_class.clone();
    let opponent_party_size = opponent_trainer.party.len();

    let reset = shell
        .apply_battle_tower_action("BATTLETOWERACTION_RESETDATA".to_string(), None, None)
        .context("reset Battle Tower data")?;
    assert_battle_tower_action_effect(&reset, "BATTLETOWERACTION_RESETDATA", "0", false)?;
    let level_group = shell
        .apply_battle_tower_action(
            "BATTLETOWERACTION_SAVELEVELGROUP".to_string(),
            Some(5),
            None,
        )
        .context("save Battle Tower level group")?;
    assert_battle_tower_action_effect(&level_group, "BATTLETOWERACTION_SAVELEVELGROUP", "1", true)?;
    let options = shell
        .apply_battle_tower_action(
            "BATTLETOWERACTION_SAVEOPTIONS".to_string(),
            None,
            Some("HP_UP".to_string()),
        )
        .context("save Battle Tower reward options")?;
    assert_battle_tower_action_effect(&options, "BATTLETOWERACTION_SAVEOPTIONS", "1", true)?;
    let explanation = shell
        .apply_battle_tower_action(
            "BATTLETOWERACTION_SET_EXPLANATION_READ".to_string(),
            None,
            None,
        )
        .context("mark Battle Tower explanation read")?;
    assert_battle_tower_action_effect(
        &explanation,
        "BATTLETOWERACTION_SET_EXPLANATION_READ",
        "1",
        true,
    )?;
    let quick_save = shell
        .apply_battle_tower_action("BATTLETOWERACTION_SAVE_AND_QUIT".to_string(), None, None)
        .context("quick-save Battle Tower challenge")?;
    assert_battle_tower_action_effect(&quick_save, "BATTLETOWERACTION_SAVE_AND_QUIT", "1", true)?;

    let battle = shell
        .start_battle_tower_battle_special(0)
        .context("start Battle Tower battle")?;
    let SpecialRoutineEffect::BattleTowerBattle {
        result_code,
        beaten_trainers,
        challenge_state,
    } = &battle.outcome.effect
    else {
        bail!(
            "Battle Tower battle returned unexpected effect {:?}",
            battle.outcome.effect
        );
    };
    if (*result_code, *beaten_trainers, *challenge_state) != (0, 1, 2) {
        bail!(
            "Battle Tower battle result={} beaten={} challenge={} did not match first battle win",
            result_code,
            beaten_trainers,
            challenge_state
        );
    }
    if !shell.runtime().sprite_ids().contains("SPRITE_YOUNGSTER") {
        bail!("Battle Tower smoke requires compiled SPRITE_YOUNGSTER sprite");
    }
    let opponent = shell
        .load_battle_tower_opponent_special(
            opponent_trainer_id.clone(),
            "SPRITE_YOUNGSTER".to_string(),
            "BATTLETOWERBATTLEROOM_YOUNGSTER".to_string(),
        )
        .context("load Battle Tower opponent")?;
    let SpecialRoutineEffect::LoadOpponentTrainerAndPokemonWithOtSprite {
        trainer_id,
        trainer_class,
        party_size,
        sprite_constant,
        target_object,
        ..
    } = &opponent.outcome.effect
    else {
        bail!(
            "Battle Tower opponent load returned unexpected effect {:?}",
            opponent.outcome.effect
        );
    };
    if trainer_id != &opponent_trainer_id
        || trainer_class != &opponent_trainer_class
        || *party_size != opponent_party_size
        || sprite_constant != "SPRITE_YOUNGSTER"
        || target_object != "BATTLETOWERBATTLEROOM_YOUNGSTER"
    {
        bail!(
            "Battle Tower opponent trainer={} class={} party={} sprite={} target={} did not match smoke opponent",
            trainer_id,
            trainer_class,
            party_size,
            sprite_constant,
            target_object
        );
    }

    let room_menu = shell
        .open_battle_tower_room_menu_special()
        .context("open Battle Tower room menu")?;
    let SpecialRoutineEffect::BattleTowerRoomMenu { records } = &room_menu.outcome.effect else {
        bail!(
            "Battle Tower room menu returned unexpected effect {:?}",
            room_menu.outcome.effect
        );
    };
    if !records.is_empty() {
        bail!(
            "new Battle Tower smoke had {} room records before challenge records were written",
            records.len()
        );
    }
    let mobile_error = shell
        .show_battle_tower_mobile_error_special()
        .context("show Battle Tower mobile error")?;
    let SpecialRoutineEffect::BattleTowerMobileError = mobile_error.outcome.effect else {
        bail!(
            "Battle Tower mobile error returned unexpected effect {:?}",
            mobile_error.outcome.effect
        );
    };
    let remember = shell
        .ask_remember_password_special(true)
        .context("ask remember password")?;
    let SpecialRoutineEffect::AskRememberPassword { remember } = &remember.outcome.effect else {
        bail!(
            "remember-password prompt returned unexpected effect {:?}",
            remember.outcome.effect
        );
    };
    if !remember {
        bail!("remember-password prompt did not preserve the accepted choice");
    }
    let handshake = shell
        .apply_mobile_handshake_special(RuntimeMobileHandshakeCommand { accepted: true })
        .context("apply mobile handshake")?;
    let SpecialRoutineEffect::MobileHandshake {
        routine,
        mode,
        link_mode,
        handshakes,
        ..
    } = &handshake.outcome.effect
    else {
        bail!(
            "mobile handshake returned unexpected effect {:?}",
            handshake.outcome.effect
        );
    };
    if routine != "Function1011f1" || mode != "init" || *link_mode != 4 || *handshakes != 1 {
        bail!(
            "mobile handshake effect routine={} mode={} link_mode={} handshakes={} did not match init link",
            routine,
            mode,
            link_mode,
            handshakes
        );
    }
    let leaderboard = shell
        .open_battle_tower_leaderboard_special()
        .context("open Battle Tower leaderboard")?;
    let SpecialRoutineEffect::BattleTowerLeaderboard {
        records,
        acknowledged,
    } = &leaderboard.outcome.effect
    else {
        bail!(
            "Battle Tower leaderboard returned unexpected effect {:?}",
            leaderboard.outcome.effect
        );
    };
    if !records.is_empty() || *acknowledged {
        bail!(
            "Battle Tower leaderboard records={} acknowledged={} did not match empty unacknowledged board",
            records.len(),
            acknowledged
        );
    }
    let acknowledge_leaderboard = shell
        .apply_battle_tower_action("BATTLETOWERACTION_12".to_string(), None, None)
        .context("acknowledge Battle Tower leaderboard")?;
    assert_battle_tower_action_effect(&acknowledge_leaderboard, "BATTLETOWERACTION_12", "1", true)?;
    let mobile_flag = shell
        .set_battle_tower_mobile_flag_special(RuntimeBattleTowerMobileFlag::Enabled)
        .context("set Battle Tower mobile flag")?;
    let SpecialRoutineEffect::BattleTowerMobileFlag { flag } = &mobile_flag.outcome.effect else {
        bail!(
            "Battle Tower mobile flag returned unexpected effect {:?}",
            mobile_flag.outcome.effect
        );
    };
    if flag != "function103780" {
        bail!("Battle Tower mobile flag {flag} did not match enabled mobile flag");
    }
    let selected = shell
        .select_three_mobile_mons_special([0, 1, 2])
        .context("select three mobile mons")?;
    let SpecialRoutineEffect::MobileSelectThreeMons { indexes } = &selected.outcome.effect else {
        bail!(
            "mobile three-mon selection returned unexpected effect {:?}",
            selected.outcome.effect
        );
    };
    if indexes != &vec![0, 1, 2] {
        bail!(
            "mobile selected party indexes {:?} did not match [0, 1, 2]",
            indexes
        );
    }
    let ended = shell
        .end_mobile_session_special()
        .context("end mobile session")?;
    let SpecialRoutineEffect::MobileSessionEnded = ended.outcome.effect else {
        bail!(
            "mobile session end returned unexpected effect {:?}",
            ended.outcome.effect
        );
    };

    let after = shell
        .snapshot()
        .context("snapshot runtime shell after Battle Tower smoke")?;
    if after.battle_tower.level_group != 5
        || after.battle_tower.reward_item != "HP_UP"
        || !after.battle_tower.explanation_read
        || after.battle_tower.quick_saved
        || after.battle_tower.challenge_state != 2
        || after.battle_tower.beaten_trainers != 1
        || !after.battle_tower.leaderboard_acknowledged
        || after.battle_tower.loaded_trainer_id.as_deref() != Some(opponent_trainer_id.as_str())
        || after.battle_tower.last_sprite_constant.as_deref() != Some("SPRITE_YOUNGSTER")
        || after.battle_tower.selected_party_indexes != vec![0, 1, 2]
        || !after.battle_tower.mobile_flags.contains("function103780")
    {
        bail!(
            "Battle Tower state did not match smoke expectations: {:?}",
            after.battle_tower
        );
    }
    if after.mobile_link.mode.as_deref() != Some("init")
        || after.mobile_link.handshakes != 1
        || !after.mobile_link.terminated
        || after.link_session.link_mode != 0
    {
        bail!(
            "mobile/link state mode={:?} handshakes={} terminated={} link_mode={} did not match completed mobile session",
            after.mobile_link.mode,
            after.mobile_link.handshakes,
            after.mobile_link.terminated,
            after.link_session.link_mode
        );
    }
    let Some(active_battle) = &after.battle else {
        bail!("Battle Tower smoke did not leave an active trainer battle");
    };
    let crystal_bevy::RuntimeBattleKind::Trainer { trainer_id, .. } = &active_battle.kind else {
        bail!(
            "Battle Tower smoke active battle was not a trainer battle: {:?}",
            active_battle.kind
        );
    };
    if active_battle.battle_type != "BATTLETYPE_BATTLE_TOWER" || trainer_id != &opponent_trainer_id
    {
        bail!(
            "Battle Tower smoke active battle type={} trainer={} did not match loaded opponent",
            active_battle.battle_type,
            trainer_id
        );
    }

    println!(
        "smoke-battle-tower spawn={} party={} level_group={} reward={} challenge={} beaten={} opponent={} quick_saved={} selected={:?} mobile_mode={} handshakes={} terminated={} before_checksum={:?} opponent_checksum={:?} selected_checksum={:?} final_checksum={:?}",
        spawn_identifier,
        party_grants.len(),
        after.battle_tower.level_group,
        after.battle_tower.reward_item,
        after.battle_tower.challenge_state,
        after.battle_tower.beaten_trainers,
        trainer_id,
        after.battle_tower.quick_saved,
        after.battle_tower.selected_party_indexes,
        after.mobile_link.mode.as_deref().unwrap_or("none"),
        after.mobile_link.handshakes,
        after.mobile_link.terminated,
        before.state_checksum,
        opponent.state_checksum,
        selected.state_checksum,
        after.state_checksum,
    );
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save Battle Tower smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed Battle Tower smoke save")?;
        if resumed_snapshot.state_checksum != after.state_checksum {
            bail!(
                "Battle Tower smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                after.state_checksum
            );
        }
        if resumed_snapshot.battle_tower != after.battle_tower
            || resumed_snapshot.mobile_link != after.mobile_link
            || resumed_snapshot.link_session.link_mode != after.link_session.link_mode
        {
            bail!(
                "Battle Tower smoke resumed state did not match final state: resumed={:?}/{:?}/{} final={:?}/{:?}/{}",
                resumed_snapshot.battle_tower,
                resumed_snapshot.mobile_link,
                resumed_snapshot.link_session.link_mode,
                after.battle_tower,
                after.mobile_link,
                after.link_session.link_mode
            );
        }
        println!(
            "smoke-battle-tower-save path={} saved_frame={} resumed_level_group={} resumed_reward={} resumed_selected={:?} resumed_mobile_mode={} resumed_handshakes={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot.battle_tower.level_group,
            resumed_snapshot.battle_tower.reward_item,
            resumed_snapshot.battle_tower.selected_party_indexes,
            resumed_snapshot
                .mobile_link
                .mode
                .as_deref()
                .unwrap_or("none"),
            resumed_snapshot.mobile_link.handshakes,
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn smoke_bug_contest(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    smoke_start_map: Option<&SmokeStartMapRef>,
    party: &[SmokePartyPokemonRef],
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    let mut shell = if let Some(start) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            &start.map_name,
            start.tile_x,
            start.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let party_grants = grant_smoke_party(&mut shell, party, "Bug Contest")?;
    let before = shell
        .snapshot()
        .context("snapshot runtime shell before Bug Contest smoke")?;
    let park_balls = shell
        .use_bug_contest(RuntimeBugContestAction::GiveParkBalls, None)
        .context("give Bug Contest Park Balls")?;
    let SpecialRoutineEffect::GiveParkBalls { balls } = park_balls.outcome.effect else {
        bail!(
            "Bug Contest Park Balls returned unexpected effect {:?}",
            park_balls.outcome.effect
        );
    };
    let after_park_balls = shell
        .snapshot()
        .context("snapshot runtime shell after Bug Contest Park Balls")?;
    if balls == 0
        || after_park_balls.bug_contest.park_balls_remaining != balls
        || !after_park_balls.bug_contest.timer_active
    {
        bail!(
            "Bug Contest Park Balls state did not match effect balls={balls}: {:?}",
            after_park_balls.bug_contest
        );
    }

    let select_contestants = shell
        .use_bug_contest(RuntimeBugContestAction::SelectContestants, None)
        .context("select Bug Contest contestants")?;
    let SpecialRoutineEffect::SelectRandomBugContestContestants {
        flags,
        rng_seed_after,
    } = &select_contestants.outcome.effect
    else {
        bail!(
            "Bug Contest contestant selection returned unexpected effect {:?}",
            select_contestants.outcome.effect
        );
    };
    let after_select = shell
        .snapshot()
        .context("snapshot runtime shell after Bug Contest contestant selection")?;
    if flags.is_empty()
        || &after_select.bug_contest.selected_contestant_flags != flags
        || after_select.state_checksum == after_park_balls.state_checksum
    {
        bail!(
            "Bug Contest contestant selection did not persist flags {:?}: {:?}",
            flags,
            after_select.bug_contest
        );
    }

    let drop_off = shell
        .use_bug_contest(RuntimeBugContestAction::DropOffMons, None)
        .context("drop off Bug Contest party Pokemon")?;
    let SpecialRoutineEffect::ContestDropOffMons {
        result,
        backup_count,
        second_party_species,
    } = &drop_off.outcome.effect
    else {
        bail!(
            "Bug Contest party drop-off returned unexpected effect {:?}",
            drop_off.outcome.effect
        );
    };
    let after_drop_off = shell
        .snapshot()
        .context("snapshot runtime shell after Bug Contest party drop-off")?;
    let expected_backup_count = party_grants.len().saturating_sub(1);
    if *result != 0
        || *backup_count != expected_backup_count
        || after_drop_off.party.slots.len() != 1
        || after_drop_off.bug_contest.party_backup.len() != expected_backup_count
        || after_drop_off.bug_contest.second_party_species != *second_party_species
    {
        bail!(
            "Bug Contest party drop-off state result={} backup={} second={:?} did not match snapshot party={} state={:?}",
            result,
            backup_count,
            second_party_species,
            after_drop_off.party.slots.len(),
            after_drop_off.bug_contest
        );
    }

    let check_party = shell
        .use_bug_contest(RuntimeBugContestAction::CheckPartyFull, None)
        .context("check Bug Contest no-catch party result")?;
    let SpecialRoutineEffect::CheckPartyFullAfterContest { result, species } =
        &check_party.outcome.effect
    else {
        bail!(
            "Bug Contest party-full check returned unexpected effect {:?}",
            check_party.outcome.effect
        );
    };
    let after_check_party = shell
        .snapshot()
        .context("snapshot runtime shell after Bug Contest party-full check")?;
    if *result != 2 || species.is_some() || after_check_party.bug_contest.last_result != Some(2) {
        bail!(
            "Bug Contest no-catch party check result={} species={:?} state={:?}",
            result,
            species,
            after_check_party.bug_contest
        );
    }

    let judge = shell
        .use_bug_contest(RuntimeBugContestAction::Judge, Some(2))
        .context("judge Bug Contest rank")?;
    let SpecialRoutineEffect::BugContestJudging { rank } = judge.outcome.effect else {
        bail!(
            "Bug Contest judging returned unexpected effect {:?}",
            judge.outcome.effect
        );
    };
    if rank != 2 {
        bail!("Bug Contest judge rank {rank} did not match requested rank 2");
    }

    let return_mons = shell
        .use_bug_contest(RuntimeBugContestAction::ReturnMons, None)
        .context("return Bug Contest party Pokemon")?;
    let SpecialRoutineEffect::ContestReturnMons { restored_count } = return_mons.outcome.effect
    else {
        bail!(
            "Bug Contest party return returned unexpected effect {:?}",
            return_mons.outcome.effect
        );
    };
    let after = shell
        .snapshot()
        .context("snapshot runtime shell after Bug Contest smoke")?;
    if restored_count != party_grants.len()
        || after.party.slots.len() != party_grants.len()
        || !after.bug_contest.party_backup.is_empty()
        || after.bug_contest.second_party_species.is_some()
        || after.bug_contest.last_rank != Some(2)
        || after.bug_contest.last_result != Some(2)
        || after.bug_contest.selected_contestant_flags != *flags
        || after.bug_contest.park_balls_remaining != balls
    {
        bail!(
            "Bug Contest final state did not match smoke expectations restored={} party={} state={:?}",
            restored_count,
            after.party.slots.len(),
            after.bug_contest
        );
    }
    println!(
        "smoke-bug-contest spawn={} party={} balls={} timer={}:{:02} selected=[{}] rng_seed_after={} drop_backup={} no_catch_result={} rank={} before_checksum={:?} park_checksum={:?} select_checksum={:?} drop_checksum={:?} check_checksum={:?} judge_checksum={:?} return_checksum={:?} final_checksum={:?}",
        spawn_identifier,
        party_grants.len(),
        balls,
        after.bug_contest.timer_minutes_remaining,
        after.bug_contest.timer_seconds_remaining,
        flags.join(","),
        rng_seed_after,
        backup_count,
        after.bug_contest.last_result.unwrap_or_default(),
        after.bug_contest.last_rank.unwrap_or_default(),
        before.state_checksum,
        park_balls.state_checksum,
        select_contestants.state_checksum,
        drop_off.state_checksum,
        check_party.state_checksum,
        judge.state_checksum,
        return_mons.state_checksum,
        after.state_checksum,
    );
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("write Bug Contest smoke save {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed Bug Contest smoke save")?;
        if resumed_snapshot.state_checksum != after.state_checksum {
            bail!(
                "Bug Contest smoke resumed checksum {:?} did not match final {:?}",
                resumed_snapshot.state_checksum,
                after.state_checksum
            );
        }
        if resumed_snapshot.bug_contest != after.bug_contest {
            bail!(
                "Bug Contest smoke resumed state {:?} did not match final {:?}",
                resumed_snapshot.bug_contest,
                after.bug_contest
            );
        }
        println!(
            "smoke-bug-contest-save path={} saved_frame={} resumed_party={} resumed_rank={:?} resumed_result={:?} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot.party.slots.len(),
            resumed_snapshot.bug_contest.last_rank,
            resumed_snapshot.bug_contest.last_result,
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn smoke_day_care(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    smoke_start_map: Option<&SmokeStartMapRef>,
    party: &[SmokePartyPokemonRef],
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    let mut shell = if let Some(start) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            &start.map_name,
            start.tile_x,
            start.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let party_grants = grant_smoke_party(&mut shell, party, "Day Care")?;
    let before = shell
        .snapshot()
        .context("snapshot runtime shell before Day Care smoke")?;
    if before.day_care.man.pokemon.is_some() || before.day_care.lady.pokemon.is_some() {
        bail!(
            "new runtime shell already had Day Care residents: {:?}",
            before.day_care
        );
    }
    let deposited_species = party_grants
        .get(1)
        .map(|grant| grant.outcome.species_id.clone())
        .context("Day Care smoke requires a second granted party Pokemon")?;
    let deposited_level = party_grants
        .get(1)
        .map(|grant| grant.outcome.level)
        .context("Day Care smoke requires a second granted party Pokemon level")?;

    let deposit = shell
        .use_day_care(
            RuntimeDayCareCaretaker::Man,
            RuntimeDayCareAction::Deposit,
            Some(1),
        )
        .context("deposit Day Care Pokemon")?;
    let SpecialRoutineEffect::DayCareInteraction {
        caretaker,
        action,
        success,
        pokemon,
    } = &deposit.outcome.effect
    else {
        bail!(
            "Day Care deposit returned unexpected effect {:?}",
            deposit.outcome.effect
        );
    };
    if caretaker != "man"
        || action != "deposit"
        || !success
        || pokemon.as_deref() != Some(deposited_species.as_str())
    {
        bail!(
            "Day Care deposit effect caretaker={} action={} success={} pokemon={:?} did not match {}",
            caretaker,
            action,
            success,
            pokemon,
            deposited_species
        );
    }
    let after_deposit = shell
        .snapshot()
        .context("snapshot runtime shell after Day Care deposit")?;
    if after_deposit.party.slots.len() != party_grants.len() - 1
        || after_deposit
            .day_care
            .man
            .pokemon
            .as_ref()
            .map(|pokemon| pokemon.species.id.as_str())
            != Some(deposited_species.as_str())
        || after_deposit.day_care.man.initial_level != deposited_level
        || !after_deposit.day_care.man.active
    {
        bail!(
            "Day Care deposit state did not match expectations party={} state={:?}",
            after_deposit.party.slots.len(),
            after_deposit.day_care
        );
    }

    let inspect = shell
        .use_day_care(
            RuntimeDayCareCaretaker::Man,
            RuntimeDayCareAction::Inspect,
            None,
        )
        .context("inspect Day Care man")?;
    assert_day_care_interaction_effect(
        &inspect,
        "man",
        "inspect",
        true,
        Some(deposited_species.as_str()),
    )?;
    let resident = shell
        .check_day_care_resident_special(RuntimeDayCareCaretaker::Man)
        .context("check Day Care man resident")?;
    let SpecialRoutineEffect::DayCareMon {
        caretaker,
        occupied,
        pokemon,
        level,
    } = &resident.outcome.effect
    else {
        bail!(
            "Day Care resident check returned unexpected effect {:?}",
            resident.outcome.effect
        );
    };
    if caretaker != "man"
        || !occupied
        || pokemon.as_deref() != Some(deposited_species.as_str())
        || *level != Some(deposited_level)
    {
        bail!(
            "Day Care resident effect caretaker={} occupied={} pokemon={:?} level={:?} did not match {} level {}",
            caretaker,
            occupied,
            pokemon,
            level,
            deposited_species,
            deposited_level
        );
    }
    let outside = shell
        .check_day_care_man_outside_special()
        .context("check Day Care man outside")?;
    assert_day_care_interaction_effect(&outside, "man", "collect_egg", false, None)?;

    let withdraw = shell
        .use_day_care(
            RuntimeDayCareCaretaker::Man,
            RuntimeDayCareAction::Withdraw,
            None,
        )
        .context("withdraw Day Care Pokemon")?;
    assert_day_care_interaction_effect(
        &withdraw,
        "man",
        "withdraw",
        true,
        Some(deposited_species.as_str()),
    )?;
    let after = shell
        .snapshot()
        .context("snapshot runtime shell after Day Care smoke")?;
    if after.party.slots.len() != party_grants.len()
        || after.day_care.man.pokemon.is_some()
        || after.day_care.man.active
        || after.day_care.man.initial_level != 0
        || after.day_care.man.initial_experience != 0
        || after.day_care.compatibility_score != 0
        || after
            .day_care
            .last_interaction
            .as_ref()
            .map(|interaction| interaction.action.as_str())
            != Some("withdraw")
    {
        bail!(
            "Day Care final state did not match expectations party={} state={:?}",
            after.party.slots.len(),
            after.day_care
        );
    }
    println!(
        "smoke-day-care spawn={} party={} deposited={} level={} before_checksum={:?} deposit_checksum={:?} inspect_checksum={:?} resident_checksum={:?} outside_checksum={:?} withdraw_checksum={:?} final_checksum={:?}",
        spawn_identifier,
        party_grants.len(),
        deposited_species,
        deposited_level,
        before.state_checksum,
        deposit.state_checksum,
        inspect.state_checksum,
        resident.state_checksum,
        outside.state_checksum,
        withdraw.state_checksum,
        after.state_checksum,
    );
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("write Day Care smoke save {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed Day Care smoke save")?;
        if resumed_snapshot.state_checksum != after.state_checksum {
            bail!(
                "Day Care smoke resumed checksum {:?} did not match final {:?}",
                resumed_snapshot.state_checksum,
                after.state_checksum
            );
        }
        if resumed_snapshot.day_care != after.day_care {
            bail!(
                "Day Care smoke resumed state {:?} did not match final {:?}",
                resumed_snapshot.day_care,
                after.day_care
            );
        }
        println!(
            "smoke-day-care-save path={} saved_frame={} resumed_party={} resumed_last_action={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot.party.slots.len(),
            resumed_snapshot
                .day_care
                .last_interaction
                .as_ref()
                .map(|interaction| interaction.action.as_str())
                .unwrap_or("none"),
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn assert_day_care_interaction_effect(
    result: &crystal_bevy::RuntimeSpecialRoutineUse,
    expected_caretaker: &str,
    expected_action: &str,
    expected_success: bool,
    expected_pokemon: Option<&str>,
) -> Result<()> {
    let SpecialRoutineEffect::DayCareInteraction {
        caretaker,
        action,
        success,
        pokemon,
    } = &result.outcome.effect
    else {
        bail!(
            "Day Care {} returned unexpected effect {:?}",
            expected_action,
            result.outcome.effect
        );
    };
    if caretaker != expected_caretaker
        || action != expected_action
        || *success != expected_success
        || pokemon.as_deref() != expected_pokemon
    {
        bail!(
            "Day Care {} effect caretaker={} action={} success={} pokemon={:?} did not match caretaker={} success={} pokemon={:?}",
            expected_action,
            caretaker,
            action,
            success,
            pokemon,
            expected_caretaker,
            expected_success,
            expected_pokemon
        );
    }
    Ok(())
}

fn assert_battle_tower_action_effect(
    result: &crystal_bevy::RuntimeSpecialRoutineUse,
    expected_action: &str,
    expected_value: &str,
    expected_truthy: bool,
) -> Result<()> {
    let SpecialRoutineEffect::BattleTowerAction {
        action,
        value,
        truthy,
    } = &result.outcome.effect
    else {
        bail!(
            "{expected_action} returned unexpected effect {:?}",
            result.outcome.effect
        );
    };
    if action != expected_action || value != expected_value || *truthy != expected_truthy {
        bail!(
            "{expected_action} effect action={} value={} truthy={} did not match value={} truthy={}",
            action,
            value,
            truthy,
            expected_value,
            expected_truthy
        );
    }
    Ok(())
}

fn assert_link_action_effect(
    result: &crystal_bevy::RuntimeSpecialRoutineUse,
    expected_action: u8,
    expected_room: u8,
    expected_routine: &str,
) -> Result<()> {
    let SpecialRoutineEffect::LinkAction { action, room } = &result.outcome.effect else {
        bail!(
            "{expected_routine} returned unexpected effect {:?}",
            result.outcome.effect
        );
    };
    if *action != expected_action
        || *room != expected_room
        || result.outcome.routine != expected_routine
    {
        bail!(
            "{expected_routine} effect action={} room={} did not match action={} room={}",
            action,
            room,
            expected_action,
            expected_room
        );
    }
    Ok(())
}

fn assert_link_result_effect(
    result: &crystal_bevy::RuntimeSpecialRoutineUse,
    expected_success: bool,
    expected_link_mode: u8,
    expected_routine: &str,
) -> Result<()> {
    let SpecialRoutineEffect::LinkResult { success, link_mode } = &result.outcome.effect else {
        bail!(
            "{expected_routine} returned unexpected effect {:?}",
            result.outcome.effect
        );
    };
    if *success != expected_success
        || *link_mode != expected_link_mode
        || result.outcome.routine != expected_routine
    {
        bail!(
            "{expected_routine} effect success={} mode={} did not match success={} mode={}",
            success,
            link_mode,
            expected_success,
            expected_link_mode
        );
    }
    Ok(())
}

fn smoke_elevator(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    elevator: &SmokeElevatorRef,
    smoke_buttons: &[GameButton],
    smoke_script: &[Vec<GameButton>],
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    let mut shell = RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?;
    let input_frames = smoke_input_frames(smoke_buttons, smoke_script);
    for (index, buttons) in input_frames.iter().enumerate() {
        shell
            .tick(buttons.iter().copied())
            .with_context(|| format!("advance runtime shell input frame {}", index + 1))?;
    }
    let before_select = shell
        .snapshot()
        .context("snapshot runtime shell before elevator smoke")?;
    let selected = shell.select_elevator_floor(
        &elevator.map_name,
        &elevator.data_label,
        &elevator.source_script,
        elevator.elevator_command_index,
        elevator.floor_index,
        &elevator.floor,
        elevator.warp,
        &elevator.target_map,
    )?;
    let final_snapshot = shell
        .snapshot()
        .context("snapshot runtime shell after elevator selection")?;
    let pending = shell
        .session()
        .state()
        .script_runtime
        .pending_script_warp
        .as_ref()
        .context("elevator smoke did not queue a pending script warp")?;
    println!(
        "smoke-elevator spawn={} frames={} map={} data_label={} source_script={} command_index={} floor_index={} floor={} warp={} target_map={} destination=({}, {}) script_value={} pending_target={} pending_tile=({}, {}) before_checksum={:?} selection_checksum={:?} final_checksum={:?}",
        spawn_identifier,
        input_frames.len(),
        selected.map_name,
        selected.data_label,
        selected.source_script,
        selected.elevator_command_index,
        selected.floor_index,
        selected.floor,
        selected.warp,
        selected.target_map,
        selected.destination_tile.x,
        selected.destination_tile.y,
        selected.script_value,
        pending.target_map,
        pending.tile.x,
        pending.tile.y,
        before_select.state_checksum,
        selected.state_checksum,
        final_snapshot.state_checksum,
    );
    if before_select.state_checksum == final_snapshot.state_checksum {
        bail!("elevator smoke did not mutate runtime state");
    }
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save elevator smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed elevator smoke save")?;
        if resumed_snapshot.state_checksum != final_snapshot.state_checksum {
            bail!(
                "elevator smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                final_snapshot.state_checksum
            );
        }
        println!(
            "smoke-elevator-save path={} saved_frame={} resumed_map={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot.overworld.map_name,
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn smoke_elevfloor_command(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    elevfloor: &SmokeElevfloorRef,
    smoke_start_map: Option<&SmokeStartMapRef>,
    smoke_buttons: &[GameButton],
    smoke_script: &[Vec<GameButton>],
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    let mut shell = if let Some(start) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            &start.map_name,
            start.tile_x,
            start.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let input_frames = smoke_input_frames(smoke_buttons, smoke_script);
    for (index, buttons) in input_frames.iter().enumerate() {
        shell
            .tick(buttons.iter().copied())
            .with_context(|| format!("advance runtime shell input frame {}", index + 1))?;
    }
    let before_apply = shell
        .snapshot()
        .context("snapshot runtime shell before elevfloor command")?;
    let applied = shell.apply_script_runtime_command(
        &elevfloor.map_name,
        &elevfloor.source_script,
        elevfloor.command_index,
        ScriptRuntimeInputs::default(),
    )?;
    let final_snapshot = shell
        .snapshot()
        .context("snapshot runtime shell after elevfloor command")?;
    let floor = shell
        .session()
        .state()
        .script_runtime
        .elevator_floors
        .last()
        .context("elevfloor smoke did not queue an elevator floor")?;
    if floor.target_map != elevfloor.target_map {
        bail!(
            "elevfloor smoke target map was {}, not {}",
            floor.target_map,
            elevfloor.target_map
        );
    }
    println!(
        "smoke-elevfloor spawn={} frames={} map={} source_script={} command_index={} floor={} warp={} target_map={} outcome={:?} before_checksum={:?} applied_checksum={:?} final_checksum={:?}",
        spawn_identifier,
        input_frames.len(),
        elevfloor.map_name,
        floor.source_script,
        floor.command_index,
        floor.floor,
        floor.warp,
        floor.target_map,
        applied.outcome,
        before_apply.state_checksum,
        applied.state_checksum,
        final_snapshot.state_checksum,
    );
    if before_apply.state_checksum == final_snapshot.state_checksum {
        bail!("elevfloor smoke did not mutate runtime state");
    }
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save elevfloor smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed elevfloor smoke save")?;
        if resumed_snapshot.state_checksum != final_snapshot.state_checksum {
            bail!(
                "elevfloor smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                final_snapshot.state_checksum
            );
        }
        println!(
            "smoke-elevfloor-save path={} saved_frame={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn smoke_script_warp_command(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    script_warp: &SmokeScriptWarpRef,
    smoke_start_map: Option<&SmokeStartMapRef>,
    keep_pending: bool,
    smoke_buttons: &[GameButton],
    smoke_script: &[Vec<GameButton>],
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    let mut shell = if let Some(start) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            &start.map_name,
            start.tile_x,
            start.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let input_frames = smoke_input_frames(smoke_buttons, smoke_script);
    for (index, buttons) in input_frames.iter().enumerate() {
        shell
            .tick(buttons.iter().copied())
            .with_context(|| format!("advance runtime shell input frame {}", index + 1))?;
    }
    let before_apply = shell
        .snapshot()
        .context("snapshot runtime shell before script warp command")?;
    let applied = shell.apply_script_map_command(
        &script_warp.map_name,
        &script_warp.source_script,
        script_warp.command_index,
    )?;
    let queued = shell
        .session()
        .state()
        .script_runtime
        .pending_script_warp
        .clone()
        .context("script warp smoke did not queue a pending script warp")?;
    if queued.target_map != script_warp.target_map {
        bail!(
            "script warp smoke target map was {}, not {}",
            queued.target_map,
            script_warp.target_map
        );
    }
    if keep_pending {
        let pending_snapshot = shell
            .snapshot()
            .context("snapshot runtime shell after pending script warp")?;
        println!(
            "smoke-script-warp-pending spawn={} frames={} map={} source_script={} command_index={} action={:?} queued_target={} queued_tile=({}, {}) before_checksum={:?} applied_checksum={:?} pending_checksum={:?}",
            spawn_identifier,
            input_frames.len(),
            script_warp.map_name,
            script_warp.source_script,
            script_warp.command_index,
            applied.action,
            queued.target_map,
            queued.tile.x,
            queued.tile.y,
            before_apply.state_checksum,
            applied.state_checksum,
            pending_snapshot.state_checksum,
        );
        if before_apply.state_checksum == pending_snapshot.state_checksum {
            bail!("pending script warp smoke did not mutate runtime state");
        }
        if let Some(save_path) = smoke_save {
            shell.save(save_path).with_context(|| {
                format!("save pending script warp smoke to {}", save_path.display())
            })?;
            let summary = shell.runtime().load_save_summary(save_path)?;
            let resumed =
                RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
            let resumed_snapshot = resumed
                .snapshot()
                .context("snapshot resumed pending script warp smoke save")?;
            if resumed_snapshot.state_checksum != pending_snapshot.state_checksum {
                bail!(
                    "pending script warp smoke resumed checksum {:?} did not match pending checksum {:?}",
                    resumed_snapshot.state_checksum,
                    pending_snapshot.state_checksum
                );
            }
            println!(
                "smoke-script-warp-pending-save path={} saved_frame={} resumed_map={} resumed_checksum={:?}",
                save_path.display(),
                summary.saved_frame(),
                resumed_snapshot.overworld.map_name,
                resumed_snapshot.state_checksum,
            );
        }
        return Ok(());
    }
    let transitioned = shell
        .execute_pending_script_warp()
        .context("transition pending script warp")?;
    let final_snapshot = shell
        .snapshot()
        .context("snapshot runtime shell after script warp transition")?;
    if final_snapshot.overworld.map_name != script_warp.target_map {
        bail!(
            "script warp smoke ended on {}, not {}",
            final_snapshot.overworld.map_name,
            script_warp.target_map
        );
    }
    println!(
        "smoke-script-warp spawn={} frames={} map={} source_script={} command_index={} action={:?} queued_target={} queued_tile=({}, {}) transition_target={} transition_tile=({}, {}) before_checksum={:?} applied_checksum={:?} final_checksum={:?}",
        spawn_identifier,
        input_frames.len(),
        script_warp.map_name,
        script_warp.source_script,
        script_warp.command_index,
        applied.action,
        queued.target_map,
        queued.tile.x,
        queued.tile.y,
        transitioned.target_map,
        transitioned.tile.x,
        transitioned.tile.y,
        before_apply.state_checksum,
        applied.state_checksum,
        final_snapshot.state_checksum,
    );
    if before_apply.state_checksum == final_snapshot.state_checksum {
        bail!("script warp smoke did not mutate runtime state");
    }
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save script warp smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed script warp smoke save")?;
        if resumed_snapshot.state_checksum != final_snapshot.state_checksum {
            bail!(
                "script warp smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                final_snapshot.state_checksum
            );
        }
        println!(
            "smoke-script-warp-save path={} saved_frame={} resumed_map={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot.overworld.map_name,
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn smoke_script_map_pending_command(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    script_map: &SmokeScriptMapPendingRef,
    smoke_start_map: Option<&SmokeStartMapRef>,
    smoke_buttons: &[GameButton],
    smoke_script: &[Vec<GameButton>],
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    let mut shell = if let Some(start) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            &start.map_name,
            start.tile_x,
            start.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let input_frames = smoke_input_frames(smoke_buttons, smoke_script);
    for (index, buttons) in input_frames.iter().enumerate() {
        shell
            .tick(buttons.iter().copied())
            .with_context(|| format!("advance runtime shell input frame {}", index + 1))?;
    }
    let before_apply = shell
        .snapshot()
        .context("snapshot runtime shell before script map pending command")?;
    let applied = shell.apply_script_map_command(
        &script_map.map_name,
        &script_map.source_script,
        script_map.command_index,
    )?;
    let state = shell.session().state();
    let pending_kind = match &applied.action {
        ScriptMapAction::LoadMap {
            command, map_setup, ..
        } => {
            if command != &script_map.command {
                bail!(
                    "script map pending smoke applied command {}, not {}",
                    command,
                    script_map.command
                );
            }
            let pending = state
                .script_runtime
                .pending_map_load
                .as_ref()
                .context("script map pending smoke did not queue pending_map_load")?;
            if pending.command != *command || pending.map_setup != *map_setup {
                bail!(
                    "pending_map_load {:?}:{:?} did not match action {:?}:{:?}",
                    pending.command,
                    pending.map_setup,
                    command,
                    map_setup
                );
            }
            Some(RuntimePendingScriptRequestKind::MapLoad)
        }
        ScriptMapAction::RefreshMap {
            command, map_setup, ..
        } => {
            if command != &script_map.command {
                bail!(
                    "script map pending smoke applied command {}, not {}",
                    command,
                    script_map.command
                );
            }
            let pending = state
                .script_runtime
                .pending_map_refresh
                .as_ref()
                .context("script map pending smoke did not queue pending_map_refresh")?;
            if pending.command != *command || pending.map_setup != *map_setup {
                bail!(
                    "pending_map_refresh {:?}:{:?} did not match action {:?}:{:?}",
                    pending.command,
                    pending.map_setup,
                    command,
                    map_setup
                );
            }
            Some(RuntimePendingScriptRequestKind::MapRefresh)
        }
        ScriptMapAction::WarpCheck { .. } => {
            if script_map.command != "warpcheck" {
                bail!(
                    "script map pending smoke applied warpcheck, not {}",
                    script_map.command
                );
            }
            if !state.script_runtime.warp_check_requested {
                bail!("script map pending smoke did not set warp_check_requested");
            }
            None
        }
        other => {
            bail!(
                "--smoke-script-map-pending expected load/refresh/warpcheck action, got {:?}",
                other
            );
        }
    };
    let pending_snapshot = shell
        .snapshot()
        .context("snapshot runtime shell after script map pending command")?;
    println!(
        "smoke-script-map-pending spawn={} frames={} map={} source_script={} command_index={} command={} action={:?} before_checksum={:?} applied_checksum={:?} pending_checksum={:?}",
        spawn_identifier,
        input_frames.len(),
        script_map.map_name,
        script_map.source_script,
        script_map.command_index,
        script_map.command,
        applied.action,
        before_apply.state_checksum,
        applied.state_checksum,
        pending_snapshot.state_checksum,
    );
    if before_apply.state_checksum == pending_snapshot.state_checksum {
        bail!("script map pending smoke did not mutate runtime state");
    }
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save script map pending smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed script map pending smoke save")?;
        if resumed_snapshot.state_checksum != pending_snapshot.state_checksum {
            bail!(
                "script map pending smoke resumed checksum {:?} did not match pending checksum {:?}",
                resumed_snapshot.state_checksum,
                pending_snapshot.state_checksum
            );
        }
        println!(
            "smoke-script-map-pending-save path={} saved_frame={} resumed_map={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot.overworld.map_name,
            resumed_snapshot.state_checksum,
        );
    }
    if let Some(kind) = pending_kind {
        let taken = shell
            .take_pending_script_request(kind)
            .context("take pending script map request")?;
        match (&applied.action, taken) {
            (
                ScriptMapAction::LoadMap {
                    command, map_setup, ..
                },
                RuntimePendingScriptRequest::MapLoad(load),
            ) if load.command == *command && load.map_setup == *map_setup => {}
            (
                ScriptMapAction::RefreshMap {
                    command, map_setup, ..
                },
                RuntimePendingScriptRequest::MapRefresh(refresh),
            ) if refresh.command == *command && refresh.map_setup == *map_setup => {}
            (action, request) => {
                bail!(
                    "taken pending script map request {:?} did not match action {:?}",
                    request,
                    action
                );
            }
        }
        let taken_snapshot = shell
            .snapshot()
            .context("snapshot runtime shell after taking script map pending request")?;
        let runtime = &shell.session().state().script_runtime;
        match kind {
            RuntimePendingScriptRequestKind::MapLoad if runtime.pending_map_load.is_some() => {
                bail!("pending_map_load remained after taking map load request");
            }
            RuntimePendingScriptRequestKind::MapRefresh
                if runtime.pending_map_refresh.is_some() =>
            {
                bail!("pending_map_refresh remained after taking map refresh request");
            }
            _ => {}
        }
        println!(
            "smoke-script-map-pending-take kind={:?} checksum={:?}",
            kind, taken_snapshot.state_checksum
        );
    }
    let drained = shell.drain_script_event_queue(RuntimeScriptEventQueue::Map)?;
    let map_events = match drained {
        RuntimeScriptEventDrainResult::Map(events) => events,
        other => bail!("map event drain returned non-map events: {:?}", other),
    };
    if map_events.is_empty() {
        bail!("script map pending smoke produced no script map events to drain");
    }
    println!("smoke-script-map-pending-drain events={}", map_events.len());
    Ok(())
}

fn smoke_script_text_pending_command(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    script_text: &SmokeScriptTextPendingRef,
    smoke_start_map: Option<&SmokeStartMapRef>,
    smoke_buttons: &[GameButton],
    smoke_script: &[Vec<GameButton>],
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    let mut shell = if let Some(start) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            &start.map_name,
            start.tile_x,
            start.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let input_frames = smoke_input_frames(smoke_buttons, smoke_script);
    for (index, buttons) in input_frames.iter().enumerate() {
        shell
            .tick(buttons.iter().copied())
            .with_context(|| format!("advance runtime shell input frame {}", index + 1))?;
    }
    let before_open = shell
        .snapshot()
        .context("snapshot runtime shell before script text open")?;
    let opened = shell.apply_script_text_command(
        &script_text.map_name,
        &script_text.source_script,
        script_text.open_command_index,
    )?;
    if !matches!(opened.action, ScriptTextAction::Open { .. }) {
        bail!(
            "script text pending smoke open index {} produced {:?}",
            script_text.open_command_index,
            opened.action
        );
    }
    let applied = shell.apply_script_text_command(
        &script_text.map_name,
        &script_text.source_script,
        script_text.command_index,
    )?;
    match &applied.action {
        ScriptTextAction::WaitButton { command, .. } => {
            if command != &script_text.command {
                bail!(
                    "script text pending smoke applied command {}, not {}",
                    command,
                    script_text.command
                );
            }
            let wait = shell
                .session()
                .state()
                .script_runtime
                .pending_text_wait
                .as_ref()
                .context("script text pending smoke did not queue pending_text_wait")?;
            if wait.command != *command {
                bail!(
                    "pending_text_wait command {} did not match action {}",
                    wait.command,
                    command
                );
            }
        }
        ScriptTextAction::YesNo { .. } => {
            if script_text.command != "yesorno" {
                bail!(
                    "script text pending smoke applied yesorno, not {}",
                    script_text.command
                );
            }
            if shell
                .session()
                .state()
                .script_runtime
                .pending_yes_no
                .is_none()
            {
                bail!("script text pending smoke did not queue pending_yes_no");
            }
        }
        other => {
            bail!(
                "--smoke-script-text-pending expected waitbutton/promptbutton/yesorno action, got {:?}",
                other
            );
        }
    }
    let pending_snapshot = shell
        .snapshot()
        .context("snapshot runtime shell after script text pending command")?;
    println!(
        "smoke-script-text-pending spawn={} frames={} map={} source_script={} open_index={} command_index={} command={} action={:?} before_checksum={:?} open_checksum={:?} pending_checksum={:?}",
        spawn_identifier,
        input_frames.len(),
        script_text.map_name,
        script_text.source_script,
        script_text.open_command_index,
        script_text.command_index,
        script_text.command,
        applied.action,
        before_open.state_checksum,
        opened.state_checksum,
        pending_snapshot.state_checksum,
    );
    if before_open.state_checksum == pending_snapshot.state_checksum {
        bail!("script text pending smoke did not mutate runtime state");
    }
    if let Some(save_path) = smoke_save {
        shell.save(save_path).with_context(|| {
            format!("save script text pending smoke to {}", save_path.display())
        })?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed script text pending smoke save")?;
        if resumed_snapshot.state_checksum != pending_snapshot.state_checksum {
            bail!(
                "script text pending smoke resumed checksum {:?} did not match pending checksum {:?}",
                resumed_snapshot.state_checksum,
                pending_snapshot.state_checksum
            );
        }
        println!(
            "smoke-script-text-pending-save path={} saved_frame={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot.state_checksum,
        );
    }
    match &applied.action {
        ScriptTextAction::WaitButton { command, .. } => {
            let advanced = shell.advance_pending_text_wait()?;
            if advanced.wait.command != *command {
                bail!(
                    "advanced text wait command {} did not match action {}",
                    advanced.wait.command,
                    command
                );
            }
        }
        ScriptTextAction::YesNo { .. } => {
            let resolved = shell.resolve_pending_yes_no(script_text.accepted)?;
            if resolved.accepted != script_text.accepted {
                bail!(
                    "resolved yes/no accepted={} did not match expected {}",
                    resolved.accepted,
                    script_text.accepted
                );
            }
        }
        _ => unreachable!(),
    }
    let cleared = shell.session().state();
    if cleared.script_runtime.pending_text_wait.is_some() {
        bail!("pending_text_wait remained after advancing text wait");
    }
    if cleared.script_runtime.pending_yes_no.is_some() {
        bail!("pending_yes_no remained after resolving yes/no");
    }
    let cleared_snapshot = shell
        .snapshot()
        .context("snapshot runtime shell after consuming script text pending command")?;
    println!(
        "smoke-script-text-pending-consume checksum={:?}",
        cleared_snapshot.state_checksum
    );
    let drained = shell.drain_script_event_queue(RuntimeScriptEventQueue::Text)?;
    let text_events = match drained {
        RuntimeScriptEventDrainResult::Text(events) => events,
        other => bail!("text event drain returned non-text events: {:?}", other),
    };
    if text_events.len() < 2 {
        bail!(
            "script text pending smoke drained {} text events, expected at least 2",
            text_events.len()
        );
    }
    println!(
        "smoke-script-text-pending-drain events={}",
        text_events.len()
    );
    Ok(())
}

fn smoke_gift_pokemon(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    gift_ref: &SmokeScriptCommandRef,
    smoke_start_map: Option<&SmokeStartMapRef>,
    smoke_buttons: &[GameButton],
    smoke_script: &[Vec<GameButton>],
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    if !runtime.has_gift_pokemon_command_at(
        &gift_ref.map_name,
        &gift_ref.source_script,
        gift_ref.command_index,
    ) {
        bail!(
            "--smoke-gift-pokemon command {}:{}:{} is not a compiled gift Pokemon command",
            gift_ref.map_name,
            gift_ref.source_script,
            gift_ref.command_index
        );
    }
    let gift_key = runtime
        .gift_pokemon_keys()
        .into_iter()
        .find(|key| {
            key.map_name == gift_ref.map_name
                && key.source_script == gift_ref.source_script
                && key.command_index == gift_ref.command_index
        })
        .context("resolve compiled gift Pokemon key for smoke")?;
    let mut shell = if let Some(start) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            &start.map_name,
            start.tile_x,
            start.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let input_frames = if smoke_script.is_empty() && smoke_buttons.is_empty() {
        Vec::new()
    } else {
        smoke_input_frames(smoke_buttons, smoke_script)
    };
    for (index, frame) in input_frames.iter().enumerate() {
        shell
            .tick(frame.iter().copied())
            .with_context(|| format!("advance runtime shell input frame {}", index + 1))?;
    }
    if shell.session().state().player_name.is_empty() {
        shell
            .set_trainer_identity("SMOKE", 1)
            .context("seed deterministic trainer identity for gift Pokemon smoke")?;
    }
    let before = shell
        .snapshot()
        .context("snapshot runtime shell before gift Pokemon")?;
    let inputs = shell
        .compiled_script_runtime_inputs(&gift_ref.source_script, gift_ref.command_index)
        .context("generate deterministic gift Pokemon runtime inputs")?;
    let grant = shell
        .grant_compiled_gift_pokemon_command(
            &gift_ref.source_script,
            gift_ref.command_index,
            inputs
                .gift_original_trainer_name
                .clone()
                .context("gift Pokemon smoke missing original trainer name")?,
            inputs
                .gift_original_trainer_id
                .context("gift Pokemon smoke missing original trainer id")?,
            inputs
                .gift_dvs
                .context("gift Pokemon smoke missing deterministic DVs")?,
            inputs
                .gift_rng_seed_after
                .context("gift Pokemon smoke missing rng seed boundary")?,
            inputs
                .gift_nickname_accepted
                .context("gift Pokemon smoke missing nickname decision")?,
            inputs.gift_nickname.clone(),
        )
        .context("grant compiled gift Pokemon command")?;
    let after = shell
        .snapshot()
        .context("snapshot runtime shell after gift Pokemon")?;
    if before.state_checksum == after.state_checksum {
        bail!("gift Pokemon smoke did not mutate runtime state");
    }
    let found_in_party = after
        .party
        .slots
        .iter()
        .any(|slot| slot.pokemon.species.id == grant.outcome.species_id);
    let found_in_pc = after.storage.boxes.iter().any(|pc_box| {
        pc_box
            .slots
            .iter()
            .any(|slot| slot.pokemon.species.id == grant.outcome.species_id)
    });
    if !(found_in_party || found_in_pc) {
        bail!(
            "gift Pokemon {} was not found in party or PC storage after grant",
            grant.outcome.species_id
        );
    }
    if !grant
        .outcome
        .pokemon
        .species
        .id
        .eq(&grant.outcome.species_id)
    {
        bail!(
            "gift Pokemon outcome species {} did not match stored Pokemon {}",
            grant.outcome.species_id,
            grant.outcome.pokemon.species.id
        );
    }
    println!(
        "smoke-gift-pokemon spawn={} frames={} map={} source_script={} command_index={} species={} level={} egg={} location={:?} party_before={} party_after={} pokedex_owned_before={} pokedex_owned_after={} before_checksum={:?} grant_checksum={:?} final_checksum={:?}",
        spawn_identifier,
        input_frames.len(),
        gift_ref.map_name,
        gift_ref.source_script,
        gift_ref.command_index,
        grant.outcome.species_id,
        grant.outcome.level,
        gift_key.egg,
        grant.outcome.location,
        before.party.slots.len(),
        after.party.slots.len(),
        before.progression.pokedex_owned,
        after.progression.pokedex_owned,
        before.state_checksum,
        grant.state_checksum,
        after.state_checksum
    );
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("write smoke gift Pokemon save {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed gift Pokemon smoke save")?;
        if resumed_snapshot.state_checksum != after.state_checksum {
            bail!(
                "gift Pokemon smoke resumed checksum {:?} did not match final {:?}",
                resumed_snapshot.state_checksum,
                after.state_checksum
            );
        }
        println!(
            "smoke-gift-pokemon-save path={} saved_frame={} resumed_party={} resumed_pokedex_owned={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot.party.slots.len(),
            resumed_snapshot.progression.pokedex_owned,
            resumed_snapshot.state_checksum
        );
    }
    Ok(())
}

fn smoke_wild_battle(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    battle_ref: &SmokeScriptCommandRef,
    smoke_start_map: Option<&SmokeStartMapRef>,
    party: &[SmokePartyPokemonRef],
    player_action: Option<BattleAction>,
    enemy_action: Option<BattleAction>,
    capture_ball: Option<&str>,
    battle_item: Option<&str>,
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    let mut shell = if let Some(start) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            &start.map_name,
            start.tile_x,
            start.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let party_grants = grant_smoke_party(&mut shell, party, "battle")?;
    let party_grant = party_grants
        .first()
        .context("wild battle smoke requires at least one granted party Pokemon")?;
    let start = shell
        .start_scripted_wild_battle(
            &battle_ref.map_name,
            &battle_ref.source_script,
            battle_ref.command_index,
        )
        .with_context(|| {
            format!(
                "start scripted wild battle {}:{}:{}",
                battle_ref.map_name, battle_ref.source_script, battle_ref.command_index
            )
        })?;
    let before_action = shell
        .snapshot()
        .context("snapshot runtime shell before battle action")?;
    let action_outcome = if let Some(ball_id) = capture_ball {
        let ball_grant = shell
            .add_bag_item(ball_id, 1)
            .with_context(|| format!("grant capture ball {ball_id}"))?;
        let attempt = shell
            .throw_ball_at_active_battle(ball_id)
            .with_context(|| format!("throw capture ball {ball_id}"))?;
        let Some(capture) = attempt.outcome.as_ref() else {
            bail!("capture ball {ball_id} did not produce an outcome");
        };
        let completion = if capture.caught {
            Some(
                shell
                    .complete_active_wild_capture(capture)
                    .with_context(|| format!("complete capture with {ball_id}"))?,
            )
        } else {
            None
        };
        format!(
            "capture ball={} grant_before={} grant_after={} caught={} blocked={} wobbles={} completion_stored={} attempt_checksum={:?} completion_checksum={:?}",
            ball_id,
            ball_grant.quantity_before,
            ball_grant.quantity_after,
            capture.caught,
            capture.blocked,
            capture.wobble_count,
            completion
                .as_ref()
                .and_then(|value| value.stored.as_ref())
                .is_some(),
            attempt.state_checksum,
            completion
                .as_ref()
                .map(|value| value.state_checksum.clone()),
        )
    } else if let Some(item_id) = battle_item {
        let item_grant = shell
            .add_bag_item(item_id, 1)
            .with_context(|| format!("grant battle item {item_id}"))?;
        let item_use = shell
            .use_bag_item_on_active_battle_pokemon(item_id)
            .with_context(|| format!("use active battle item {item_id}"))?;
        format!(
            "battle_item item={} grant_before={} grant_after={} consumed={} use_checksum={:?}",
            item_id,
            item_grant.quantity_before,
            item_grant.quantity_after,
            item_use.item_use.consumed,
            item_use.state_checksum,
        )
    } else {
        let player_action = player_action.context("missing battle player action")?;
        let enemy_action = enemy_action.context("missing battle enemy action")?;
        let rng_seed_after = shell
            .preview_active_battle_command_rng_seed_after(
                player_action.clone(),
                enemy_action.clone(),
            )
            .context("preview fixed battle command rng boundary")?;
        let command = shell
            .resolve_active_battle_command(
                player_action.clone(),
                enemy_action.clone(),
                rng_seed_after,
            )
            .with_context(|| {
                format!(
                    "resolve battle command player={:?} enemy={:?}",
                    player_action, enemy_action
                )
            })?;
        format!("command {:?}", command.outcome)
    };
    let final_snapshot = shell
        .snapshot()
        .context("snapshot runtime shell after battle action")?;
    let active_battle = final_snapshot.battle.as_ref();
    println!(
        "smoke-wild-battle spawn={} map={} source_script={} command_index={} party_species={} party_level={} party_checksum={:?} wild_species={} wild_level={} before_checksum={:?} action_outcome={} final_active_battle={} final_checksum={:?}",
        spawn_identifier,
        battle_ref.map_name,
        battle_ref.source_script,
        battle_ref.command_index,
        party_grant.outcome.species_id,
        party_grant.outcome.level,
        party_grant.state_checksum,
        start.species,
        start.level,
        before_action.state_checksum,
        action_outcome,
        active_battle.is_some(),
        final_snapshot.state_checksum,
    );
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save wild battle smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed wild battle smoke save")?;
        if resumed_snapshot.state_checksum != final_snapshot.state_checksum {
            bail!(
                "wild battle smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                final_snapshot.state_checksum
            );
        }
        println!(
            "smoke-wild-battle-save path={} saved_frame={} resumed_active_battle={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot.battle.is_some(),
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn smoke_trainer_battle(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    battle_ref: &SmokeScriptCommandRef,
    smoke_start_map: Option<&SmokeStartMapRef>,
    party: &[SmokePartyPokemonRef],
    smoke_save: Option<&PathBuf>,
) -> Result<()> {
    let mut shell = if let Some(start) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime,
            spawn_identifier,
            &start.map_name,
            start.tile_x,
            start.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?
    };
    let party_grants = grant_smoke_party(&mut shell, party, "trainer battle")?;
    let party_grant = party_grants
        .first()
        .context("trainer battle smoke requires at least one granted party Pokemon")?;
    let start = shell
        .start_scripted_trainer_battle(
            &battle_ref.map_name,
            &battle_ref.source_script,
            battle_ref.command_index,
        )
        .with_context(|| {
            format!(
                "start scripted trainer battle {}:{}:{}",
                battle_ref.map_name, battle_ref.source_script, battle_ref.command_index
            )
        })?;
    let before_completion = shell
        .snapshot()
        .context("snapshot runtime shell before trainer battle completion")?;
    let mut battle_turns = 0usize;
    let mut reward_claims = 0usize;
    let mut trainer_defeated = false;
    while battle_turns < 24 && !trainer_defeated {
        let rng_seed_after = shell
            .preview_active_battle_turn_rng_seed_after(
                BattleAction::Move { slot: 0 },
                BattleAction::Move { slot: 0 },
            )
            .context("preview fixed trainer battle turn rng boundary")?;
        let turn = shell
            .resolve_active_battle_turn(
                BattleAction::Move { slot: 0 },
                BattleAction::Move { slot: 0 },
                rng_seed_after,
            )
            .with_context(|| {
                format!(
                    "resolve trainer battle turn {} for {}:{}:{}",
                    battle_turns + 1,
                    battle_ref.map_name,
                    battle_ref.source_script,
                    battle_ref.command_index
                )
            })?;
        battle_turns += 1;
        let enemy_fainted = turn.outcome.events.iter().any(|event| {
            matches!(
                event,
                BattleEvent::Fainted {
                    side: BattleSide::Enemy
                }
            )
        });
        if !enemy_fainted {
            continue;
        }
        shell
            .claim_active_trainer_battle_rewards()
            .context("claim active trainer battle rewards")?;
        reward_claims += 1;
        let advance = shell
            .advance_active_trainer_battle()
            .context("advance active trainer battle")?;
        trainer_defeated = advance.trainer_defeated;
    }
    if !trainer_defeated {
        bail!(
            "trainer battle smoke did not defeat trainer after {} turns",
            battle_turns
        );
    }
    let completion = shell
        .complete_scripted_trainer_battle(
            &battle_ref.map_name,
            &battle_ref.source_script,
            battle_ref.command_index,
            true,
            false,
        )
        .with_context(|| {
            format!(
                "complete scripted trainer battle {}:{}:{}",
                battle_ref.map_name, battle_ref.source_script, battle_ref.command_index
            )
        })?;
    let final_snapshot = shell
        .snapshot()
        .context("snapshot runtime shell after trainer battle completion")?;
    println!(
        "smoke-trainer-battle spawn={} map={} source_script={} command_index={} party_species={} party_level={} party_checksum={:?} trainer={:?} before_checksum={:?} turns={} reward_claims={} continued={} prize_money={:?} money_after={:?} effects={} final_active_battle={} final_money={} final_checksum={:?}",
        spawn_identifier,
        battle_ref.map_name,
        battle_ref.source_script,
        battle_ref.command_index,
        party_grant.outcome.species_id,
        party_grant.outcome.level,
        party_grant.state_checksum,
        start,
        before_completion.state_checksum,
        battle_turns,
        reward_claims,
        completion.continued_after_battle,
        completion.trainer_prize_money,
        completion.money_after,
        completion.effects.is_some(),
        final_snapshot.battle.is_some(),
        final_snapshot.trainer.money,
        final_snapshot.state_checksum,
    );
    if let Some(save_path) = smoke_save {
        shell
            .save(save_path)
            .with_context(|| format!("save trainer battle smoke to {}", save_path.display()))?;
        let summary = shell.runtime().load_save_summary(save_path)?;
        let resumed =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let resumed_snapshot = resumed
            .snapshot()
            .context("snapshot resumed trainer battle smoke save")?;
        if resumed_snapshot.state_checksum != final_snapshot.state_checksum {
            bail!(
                "trainer battle smoke resumed checksum {:?} did not match final checksum {:?}",
                resumed_snapshot.state_checksum,
                final_snapshot.state_checksum
            );
        }
        if resumed_snapshot.trainer.money != final_snapshot.trainer.money {
            bail!(
                "trainer battle smoke resumed money {} did not match final money {}",
                resumed_snapshot.trainer.money,
                final_snapshot.trainer.money
            );
        }
        println!(
            "smoke-trainer-battle-save path={} saved_frame={} resumed_active_battle={} resumed_money={} resumed_checksum={:?}",
            save_path.display(),
            summary.saved_frame(),
            resumed_snapshot.battle.is_some(),
            resumed_snapshot.trainer.money,
            resumed_snapshot.state_checksum,
        );
    }
    Ok(())
}

fn smoke_save_resume(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    save_path: &PathBuf,
    smoke_buttons: &[GameButton],
    smoke_script: &[Vec<GameButton>],
) -> Result<()> {
    let mut shell = RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)?;
    let before = shell.snapshot().context("snapshot new runtime shell")?;
    let input_frames = smoke_input_frames(smoke_buttons, smoke_script);
    let mut frame = None;
    for (index, buttons) in input_frames.iter().enumerate() {
        frame = Some(
            shell
                .tick(buttons.iter().copied())
                .with_context(|| format!("advance runtime shell input frame {}", index + 1))?
                .clone(),
        );
    }
    let frame = frame.context("smoke input sequence must contain at least one frame")?;
    shell.save(save_path)?;
    let summary = shell.runtime().load_save_summary(save_path)?;
    let resumed =
        RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
    let resumed_snapshot = resumed
        .snapshot()
        .context("snapshot resumed runtime shell")?;
    println!(
        "smoke-save spawn={} start_map={} final_map={} final_tile=({}, {}) facing={:?} mode={:?} before_checksum={:?} after_checksum={:?} resumed_checksum={:?} frames={} frame={} saved_frame={} path={}",
        spawn_identifier,
        before.overworld.map_name,
        frame.snapshot.map_name,
        frame.snapshot.tile.x,
        frame.snapshot.tile.y,
        frame.snapshot.facing,
        frame.snapshot.mode,
        before.state_checksum,
        frame.state_checksum,
        resumed_snapshot.state_checksum,
        input_frames.len(),
        frame.snapshot.frame,
        summary.saved_frame(),
        save_path.display()
    );
    Ok(())
}

fn smoke_link_journal(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    smoke_start_map: Option<&SmokeStartMapRef>,
    smoke_buttons: &[GameButton],
    smoke_script: &[Vec<GameButton>],
) -> Result<()> {
    let mut record_shell = if let Some(start) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime.clone(),
            spawn_identifier,
            &start.map_name,
            start.tile_x,
            start.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root.clone(), runtime.clone(), spawn_identifier)?
    };
    let descriptor = record_shell
        .link_session_descriptor("smoke-link-journal", 1, "P1")
        .context("build smoke link descriptor")?;
    let input_frames = smoke_input_frames(smoke_buttons, smoke_script);
    let recorded = record_shell
        .record_local_input_journal(
            &descriptor,
            input_frames.iter().map(|buttons| buttons.to_vec()),
        )
        .context("record smoke link input journal")?;
    let mut replay_shell = if let Some(start) = smoke_start_map {
        RuntimeGameShell::new_game_at_runtime_tile(
            asset_root,
            runtime,
            spawn_identifier,
            &start.map_name,
            start.tile_x,
            start.tile_y,
        )?
    } else {
        RuntimeGameShell::new_game(asset_root, runtime, spawn_identifier)?
    };
    replay_shell
        .validate_local_input_journal_start(&descriptor, &recorded.journal)
        .context("preflight smoke link input journal")?;
    let replayed = replay_shell
        .apply_local_input_journal(&descriptor, recorded.journal.clone())
        .context("replay smoke link input journal")?;
    if replayed.terminal_checksum != recorded.terminal_checksum {
        bail!(
            "smoke link replay checksum {:?} did not match recorded {:?}",
            replayed.terminal_checksum,
            recorded.terminal_checksum
        );
    }
    let input_message = record_shell
        .input_journal_message(recorded.clone())
        .context("build smoke input journal message")?;
    let replay_message = record_shell
        .save_resume_replay_message(
            &descriptor,
            recorded.clone(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .context("build smoke save-resume replay message")?;
    if !matches!(input_message, LinkMessage::InputJournal(_)) {
        bail!("smoke link journal built non-input-journal message");
    }
    if !matches!(replay_message, LinkMessage::SaveResumeReplay(_)) {
        bail!("smoke link journal built non-save-resume-replay message");
    }
    let recorded_snapshot = record_shell
        .snapshot()
        .context("snapshot recorded smoke link shell")?;
    let replayed_snapshot = replay_shell
        .snapshot()
        .context("snapshot replayed smoke link shell")?;
    if replayed_snapshot.state_checksum != recorded_snapshot.state_checksum {
        bail!(
            "smoke link replay snapshot checksum {:?} did not match recorded {:?}",
            replayed_snapshot.state_checksum,
            recorded_snapshot.state_checksum
        );
    }
    println!(
        "smoke-link-journal spawn={} frames={} start_frame={} terminal_frame={} start_hash={:#010x} terminal_hash={:#010x} fingerprint={} input_message={} replay_message={} final_map={} final_tile=({}, {}) replay_checksum={:?}",
        spawn_identifier,
        recorded.journal.frames().len(),
        recorded.journal.start_checksum().frame(),
        recorded.journal.terminal_checksum().frame(),
        recorded.journal.start_checksum().hash(),
        recorded.journal.terminal_checksum().hash(),
        recorded.fingerprint_hex()?,
        input_message.message_type(),
        replay_message.message_type(),
        replayed_snapshot.overworld.map_name,
        replayed_snapshot.overworld.tile.x,
        replayed_snapshot.overworld.tile.y,
        replayed_snapshot.state_checksum,
    );
    Ok(())
}

fn smoke_title_new_game(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    save_path: &PathBuf,
    smoke_player_name: &str,
) -> Result<()> {
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .context("resolve title new-game spawn from compiled pack")?;
    let mut shell = RuntimeGameShell::new_game(asset_root.clone(), runtime, spawn_identifier)
        .with_context(|| format!("start title new-game smoke at spawn {spawn_identifier}"))?;
    let initial = shell
        .snapshot()
        .context("snapshot title new-game initial runtime shell")?;
    shell
        .set_trainer_identity(smoke_player_name, initial.trainer.player_id)
        .context("set explicit title new-game smoke trainer identity")?;
    let snapshot = shell
        .snapshot()
        .context("snapshot title new-game runtime shell")?;
    shell
        .save(save_path)
        .with_context(|| format!("save title new-game smoke to {}", save_path.display()))?;
    let summary = shell.runtime().load_save_summary(save_path)?;
    let resumed =
        RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
    let resumed_snapshot = resumed
        .snapshot()
        .context("snapshot title new-game save roundtrip")?;
    if summary.modpack().id() != shell.runtime().modpack().id() {
        bail!(
            "title new-game save modpack id {} does not match runtime {}",
            summary.modpack().id(),
            shell.runtime().modpack().id()
        );
    }
    if summary.modpack().hash() != shell.runtime().modpack().hash() {
        bail!(
            "title new-game save modpack hash {} does not match runtime {}",
            summary.modpack().hash(),
            shell.runtime().modpack().hash()
        );
    }
    if summary.pack_content_hash() != shell.runtime().pack_identity().content_hash.as_str() {
        bail!(
            "title new-game save pack content hash {} does not match runtime {}",
            summary.pack_content_hash(),
            shell.runtime().pack_identity().content_hash
        );
    }
    println!(
        "smoke-title-new-game spawn={} map={} tile=({}, {}) facing={:?} checksum={:?} saved_frame={} pack_content_hash={} roundtrip_checksum={:?} path={}",
        spawn_identifier,
        snapshot.overworld.map_name,
        snapshot.overworld.tile.x,
        snapshot.overworld.tile.y,
        snapshot.overworld.facing,
        snapshot.state_checksum,
        summary.saved_frame(),
        summary.pack_content_hash(),
        resumed_snapshot.state_checksum,
        save_path.display()
    );
    Ok(())
}

fn smoke_load_save(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    load_path: &PathBuf,
    save_path: Option<&PathBuf>,
) -> Result<()> {
    let summary = runtime.load_save_summary(load_path)?;
    let shell = RuntimeGameShell::resume_from_save(asset_root.clone(), runtime, load_path)?;
    let snapshot = shell.snapshot().context("snapshot loaded runtime shell")?;
    if let Some(save_path) = save_path {
        shell.save(save_path)?;
        let roundtrip_summary = shell.runtime().load_save_summary(save_path)?;
        let roundtrip =
            RuntimeGameShell::resume_from_save(asset_root, shell.runtime().clone(), save_path)?;
        let roundtrip_snapshot = roundtrip
            .snapshot()
            .context("snapshot roundtrip runtime shell")?;
        println!(
            "smoke-load-save path={} map={} tile=({}, {}) facing={:?} mode={:?} checksum={:?} saved_frame={} roundtrip_path={} roundtrip_frame={} roundtrip_checksum={:?}",
            load_path.display(),
            snapshot.overworld.map_name,
            snapshot.overworld.tile.x,
            snapshot.overworld.tile.y,
            snapshot.overworld.facing,
            snapshot.overworld.mode,
            snapshot.state_checksum,
            summary.saved_frame(),
            save_path.display(),
            roundtrip_summary.saved_frame(),
            roundtrip_snapshot.state_checksum,
        );
        anyhow::ensure!(
            snapshot.state_checksum == roundtrip_snapshot.state_checksum,
            "roundtrip save checksum mismatch: loaded {:?}, roundtrip {:?}",
            snapshot.state_checksum,
            roundtrip_snapshot.state_checksum
        );
        return Ok(());
    }
    println!(
        "smoke-load-save path={} map={} tile=({}, {}) facing={:?} mode={:?} checksum={:?} saved_frame={}",
        load_path.display(),
        snapshot.overworld.map_name,
        snapshot.overworld.tile.x,
        snapshot.overworld.tile.y,
        snapshot.overworld.facing,
        snapshot.overworld.mode,
        snapshot.state_checksum,
        summary.saved_frame(),
    );
    Ok(())
}

fn smoke_input_frames<'a>(
    smoke_buttons: &'a [GameButton],
    smoke_script: &'a [Vec<GameButton>],
) -> Vec<&'a [GameButton]> {
    if smoke_script.is_empty() {
        vec![smoke_buttons]
    } else {
        smoke_script.iter().map(Vec::as_slice).collect()
    }
}

fn bag_snapshot_quantity(snapshot: &RuntimeShellSnapshot, item_id: &str) -> u16 {
    let pocket_quantity = snapshot
        .bag
        .items
        .iter()
        .chain(snapshot.bag.balls.iter())
        .chain(snapshot.bag.key_items.iter())
        .chain(snapshot.bag.pc_items.iter())
        .find(|item| item.item_id == item_id)
        .map(|item| item.quantity)
        .unwrap_or(0);
    if pocket_quantity > 0 {
        return pocket_quantity;
    }
    if snapshot
        .bag
        .tm_hm
        .iter()
        .any(|tmhm| tmhm.item_id == item_id)
    {
        1
    } else {
        0
    }
}

fn pc_item_snapshot_quantity(snapshot: &RuntimeShellSnapshot, item_id: &str) -> u16 {
    snapshot
        .bag
        .pc_items
        .iter()
        .find(|item| item.item_id == item_id)
        .map(|item| item.quantity)
        .unwrap_or(0)
}

fn print_boot_summary(runtime: &CrystalRuntime) {
    let summary = runtime.boot_summary();
    println!(
        "pack={} maps={} species={} moves={} items={} wild_encounters={} music={} sfx={} cries={}",
        summary.modpack_id,
        summary.maps,
        summary.pokemon_species,
        summary.moves,
        summary.items,
        summary.wild_encounter_tables,
        summary.music_tracks,
        summary.sound_effects,
        summary.cries
    );
}

fn print_spawns(runtime: &CrystalRuntime) {
    println!("spawns:");
    for spawn in runtime.data().runtime_spawn_points().values() {
        println!(
            "  {} map={} constant={} group={} map_id={} tile=({}, {}) metatile=({}, {}) subtile=({}, {})",
            spawn.identifier,
            spawn.map_name,
            spawn.map_constant,
            spawn.group_id,
            spawn.map_id,
            spawn.tile_x,
            spawn.tile_y,
            spawn.metatile_x,
            spawn.metatile_y,
            spawn.subtile_x,
            spawn.subtile_y
        );
    }
}

fn default_save_path(repository_root: &std::path::Path, runtime: &CrystalRuntime) -> PathBuf {
    repository_root
        .join("target")
        .join("crystal-bevy")
        .join("saves")
        .join(format!(
            "{}-{}-{}.{}",
            runtime.modpack().id(),
            runtime.modpack().hash(),
            runtime.pack_identity().content_hash,
            SAVE_EXTENSION
        ))
}

fn print_script_map_commands(runtime: &CrystalRuntime) {
    println!("script_map_commands:");
    for key in runtime.script_map_command_keys() {
        println!(
            "  map={} command={} target={} x={} y={} facing={} map_setup={} source_script={} command_index={}",
            key.map_name,
            key.command,
            key.target_map.as_deref().unwrap_or(""),
            key.x.map(|value| value.to_string()).unwrap_or_default(),
            key.y.map(|value| value.to_string()).unwrap_or_default(),
            key.facing.as_deref().unwrap_or(""),
            key.map_setup.as_deref().unwrap_or(""),
            key.source_script,
            key.command_index,
        );
    }
}

fn print_script_scene_commands(runtime: &CrystalRuntime) {
    println!("script_scene_commands:");
    for key in runtime.script_scene_command_keys() {
        println!(
            "  map={} command={} map_id={} scene_id={} source_script={} command_index={}",
            key.map_name,
            key.command,
            key.map_id.as_deref().unwrap_or(""),
            key.scene_id.as_deref().unwrap_or(""),
            key.source_script,
            key.command_index,
        );
    }
}

fn print_script_battles(runtime: &CrystalRuntime) {
    println!("scripted_wild_battles:");
    for battle in runtime.scripted_wild_battle_keys() {
        println!(
            "  map={} source_script={} loadwildmon_index={} startbattle_index={} battle_type={} species={} level={} reload_map_after_battle={}",
            battle.map_name,
            battle.source_script,
            battle.loadwildmon_command_index,
            battle.startbattle_command_index,
            battle.battle_type,
            battle.species,
            battle.level,
            battle.reload_map_after_battle,
        );
    }
    println!("scripted_trainer_battles:");
    for battle in runtime.scripted_trainer_battle_keys() {
        println!(
            "  map={} source_script={} loadtrainer_index={} startbattle_index={} battle_type={} trainer_class={} trainer_id={} reload_map_after_battle={}",
            battle.map_name,
            battle.source_script,
            battle.loadtrainer_command_index,
            battle.startbattle_command_index,
            battle.battle_type,
            battle.trainer_class,
            battle.trainer_id,
            battle.reload_map_after_battle,
        );
    }
}

fn print_gift_pokemon(runtime: &CrystalRuntime) {
    println!("gift_pokemon:");
    for gift in runtime.gift_pokemon_keys() {
        println!(
            "  map={} source_script={} command_index={} species={} level_token={} level={} held_item={} nickname_label={} ot_label={} egg={}",
            gift.map_name,
            gift.source_script,
            gift.command_index,
            gift.species_id,
            gift.level_token,
            gift.level,
            gift.held_item_id.as_deref().unwrap_or(""),
            gift.nickname_label.as_deref().unwrap_or(""),
            gift.ot_label.as_deref().unwrap_or(""),
            gift.egg,
        );
    }
}

fn print_compiled_script(runtime: &CrystalRuntime, script_label: &str) -> Result<()> {
    println!("compiled_script {script_label}:");
    for (index, command) in runtime
        .compiled_script_commands(script_label)?
        .iter()
        .enumerate()
    {
        println!("  {index}: {command}");
    }
    Ok(())
}

fn print_map_objects(runtime: &CrystalRuntime, map_name: &str) -> Result<()> {
    let module = runtime.data().map_module(map_name)?;
    println!("map_objects map={map_name}:");
    for object in &module.objects {
        println!(
            "  tile=({}, {}) sprite={} type={} script={} event_flag={} object_identifier={} label={} move={} range=({}, {}) sightline={}",
            object.x,
            object.y,
            object.sprite,
            object.object_type,
            object.script,
            object.event_flag,
            object.object_identifier.as_deref().unwrap_or(""),
            object.label.as_deref().unwrap_or(""),
            object.spritemovedata,
            object.move_range_x,
            object.move_range_y,
            object.sightline_direction_override.as_deref().unwrap_or(""),
        );
    }
    Ok(())
}

fn print_map_events(runtime: &CrystalRuntime, map_name: &str) -> Result<()> {
    let module = runtime.data().map_module(map_name)?;
    println!("map_events map={map_name}:");
    println!(
        "  attributes: size={}x{} constant={} group={} tileset={} environment={} music={} connections={}",
        module.attributes.width,
        module.attributes.height,
        module.attributes.map_constant.as_deref().unwrap_or(""),
        module
            .attributes
            .map_group_constant
            .as_deref()
            .unwrap_or(""),
        module.attributes.tileset_name,
        module.attributes.environment.as_deref().unwrap_or(""),
        module.attributes.music.as_deref().unwrap_or(""),
        module.attributes.connections.len()
    );
    println!("  connections:");
    for connection in &module.attributes.connections {
        println!(
            "    direction={} target={} offset={}",
            connection.direction, connection.target_map, connection.offset
        );
    }
    println!("  warps:");
    for warp in &module.events.warps {
        println!(
            "    tile=({}, {}) target={} warp_id={}",
            warp.x, warp.y, warp.target_map, warp.target_warp_id
        );
    }
    println!("  coord_events:");
    for event in &module.events.coord_events {
        println!(
            "    tile=({}, {}) scene={} script={}",
            event.x, event.y, event.scene_id, event.script_name
        );
    }
    println!("  bg_events:");
    for event in &module.events.bg_events {
        println!(
            "    tile=({}, {}) type={} script={}",
            event.x, event.y, event.event_type, event.script
        );
    }
    println!("  passability:");
    let map_data = OverworldMapData {
        name: map_name.to_string(),
        width: module.attributes.width,
        height: module.attributes.height,
        border_block: u16::from(module.attributes.border_block),
        connections: module.attributes.connections.clone(),
        metatile_ids: module.blocks.clone(),
    };
    let tileset = runtime
        .data()
        .tileset_collision(&module.attributes.tileset_name)?;
    let runtime_width = i16::try_from(module.attributes.width)
        .context("map width exceeds runtime coordinate range")?
        .saturating_mul(2);
    let runtime_height = i16::try_from(module.attributes.height)
        .context("map height exceeds runtime coordinate range")?
        .saturating_mul(2);
    for y in 0..runtime_height {
        let mut row = String::new();
        for x in 0..runtime_width {
            let tile = TilePosition::new(x, y);
            let marker = if module.objects.iter().any(|object| {
                i16::try_from(object.x).ok() == Some(x) && i16::try_from(object.y).ok() == Some(y)
            }) {
                'O'
            } else if module.events.warps.iter().any(|warp| {
                i16::try_from(warp.x).ok() == Some(x) && i16::try_from(warp.y).ok() == Some(y)
            }) {
                'W'
            } else if module.events.coord_events.iter().any(|event| {
                i16::try_from(event.x).ok() == Some(x) && i16::try_from(event.y).ok() == Some(y)
            }) {
                'C'
            } else {
                match sample_collision(&map_data, &tileset, tile)
                    .map(|sample| describe_collision(sample.permission).terrain)
                {
                    Some(Terrain::Wall) | None => '#',
                    Some(Terrain::Water) => '~',
                    Some(Terrain::Land) => '.',
                }
            };
            row.push(marker);
        }
        println!("    y={y:02} {row}");
    }
    Ok(())
}

fn print_field_targets(runtime: &CrystalRuntime, map_name: &str) -> Result<()> {
    let module = runtime.data().map_module(map_name)?;
    let map_data = OverworldMapData {
        name: map_name.to_string(),
        width: module.attributes.width,
        height: module.attributes.height,
        border_block: u16::from(module.attributes.border_block),
        connections: module.attributes.connections.clone(),
        metatile_ids: module.blocks.clone(),
    };
    let tileset = runtime
        .data()
        .tileset_collision(&module.attributes.tileset_name)?;
    let runtime_width = i16::try_from(module.attributes.width)
        .context("map width exceeds runtime coordinate range")?
        .saturating_mul(2);
    let runtime_height = i16::try_from(module.attributes.height)
        .context("map height exceeds runtime coordinate range")?
        .saturating_mul(2);
    println!(
        "field_targets map={} tileset={} size={}x{}:",
        map_name, module.attributes.tileset_name, runtime_width, runtime_height
    );
    println!("  headbutt:");
    let headbutt_collisions = &runtime.data().field_moves.headbutt.target_collisions;
    let mut headbutt_count = 0usize;
    for y in 0..runtime_height {
        for x in 0..runtime_width {
            let tile = TilePosition::new(x, y);
            let Some(sample) = sample_collision(&map_data, &tileset, tile) else {
                continue;
            };
            if headbutt_collisions.contains(&sample.permission) {
                headbutt_count += 1;
                println!(
                    "    target=({}, {}) collision=0x{:02x} terrain={:?} starts={}",
                    x,
                    y,
                    sample.permission,
                    describe_collision(sample.permission).terrain,
                    format_field_target_starts(map_name, &map_data, &tileset, x, y),
                );
            }
        }
    }
    if headbutt_count == 0 {
        println!("    none");
    }
    println!("  rock_smash:");
    let mut rock_count = 0usize;
    for object in &module.objects {
        if object.spritemovedata != "SPRITEMOVEDATA_SMASHABLE_ROCK" {
            continue;
        }
        rock_count += 1;
        println!(
            "    target=({}, {}) object_identifier={} event_flag={} starts={}",
            object.x,
            object.y,
            object.object_identifier.as_deref().unwrap_or(""),
            object.event_flag,
            format_field_target_starts(
                map_name,
                &map_data,
                &tileset,
                i16::try_from(object.x).context("object x exceeds runtime coordinate range")?,
                i16::try_from(object.y).context("object y exceeds runtime coordinate range")?,
            ),
        );
    }
    if rock_count == 0 {
        println!("    none");
    }
    Ok(())
}

fn format_field_target_starts(
    map_name: &str,
    map_data: &OverworldMapData,
    tileset: &TilesetCollision,
    target_x: i16,
    target_y: i16,
) -> String {
    let starts = [
        (target_x, target_y - 1, "down"),
        (target_x, target_y + 1, "up"),
        (target_x - 1, target_y, "right"),
        (target_x + 1, target_y, "left"),
    ]
    .into_iter()
    .filter_map(|(start_x, start_y, facing_script)| {
        let tile = TilePosition::new(start_x, start_y);
        let sample = sample_collision(map_data, tileset, tile)?;
        match describe_collision(sample.permission).terrain {
            Terrain::Land => Some(format!(
                "{map_name}:{start_x}:{start_y};face={facing_script}"
            )),
            Terrain::Water | Terrain::Wall => None,
        }
    })
    .collect::<Vec<_>>();
    if starts.is_empty() {
        "-".to_string()
    } else {
        starts.join("|")
    }
}

#[cfg(not(feature = "bevy-shell"))]
compile_error!("crystal-bevy binary requires building with --features bevy-shell");
