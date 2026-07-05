use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use crystal_assets::{
    RuntimeBadgeRegion, RuntimeCurrencyAccount, RuntimeCurrencyDeltaCommand,
    RuntimeMutationCommand, RuntimePendingScriptRequest, RuntimePendingScriptRequestKind,
    RuntimeScriptEventDrainResult, RuntimeScriptEventQueue,
};
use crystal_bevy::{
    BevyShellConfig, BevyShellStart, CrystalRuntime, RuntimeGameShell, RuntimeGiftPokemonGrant,
    RuntimeShellSnapshot, VisibleShellBattleSmokeRef, VisibleShellSmokeItem,
    VisibleShellSmokePokemon,
    assets::{AssetRoot, modpack::COMPILED_GAME_PACK_EXTENSION},
    core::battle::turn::{BattleAction, BattleEvent, BattleSide},
    core::input::GameButton,
    core::models::{Dv, PARTY_SIZE},
    core::save::SAVE_EXTENSION,
    core::systems::script_runtime::ScriptRuntimeInputs,
    core::systems::script_text::ScriptTextAction,
    core::systems::script_warps::ScriptMapAction,
    core::world::collision::{Terrain, describe_collision, sample_collision},
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

    let pack = args.pack.as_deref().unwrap_or(DEFAULT_PACK_PATH);
    let repository_root = match args.repo {
        Some(repo) => repo,
        None => env::current_dir().context("resolve current directory for default --repo")?,
    }
    .canonicalize()
    .context("canonicalize repository root")?;
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
            &args.smoke_buttons,
            &args.smoke_script,
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
        smoke_title_new_game(asset_root, runtime, save_path)?;
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
            BevyShellConfig::default(),
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
            BevyShellConfig::default(),
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
            BevyShellConfig::default(),
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
            BevyShellConfig::default(),
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
        (Some(spawn_identifier), None) => BevyShellStart::NewGame { spawn_identifier },
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
    list_script: Option<String>,
    list_map_objects: Option<String>,
    list_map_events: Option<String>,
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
    smoke_field_move: Option<SmokeFieldMoveRef>,
    smoke_fishing: Option<SmokeFishingRef>,
    smoke_menu: Option<SmokeMenuRef>,
    smoke_interact: bool,
    smoke_elevator: Option<SmokeElevatorRef>,
    smoke_elevfloor: Option<SmokeElevfloorRef>,
    smoke_script_warp: Option<SmokeScriptWarpRef>,
    smoke_script_warp_pending: bool,
    smoke_script_map_pending: Option<SmokeScriptMapPendingRef>,
    smoke_script_text_pending: Option<SmokeScriptTextPendingRef>,
    smoke_start_map: Option<SmokeStartMapRef>,
    smoke_wild_battle: Option<SmokeScriptCommandRef>,
    smoke_party: Vec<SmokePartyPokemonRef>,
    smoke_player_action: Option<BattleAction>,
    smoke_enemy_action: Option<BattleAction>,
    smoke_capture_ball: Option<String>,
    smoke_battle_item: Option<String>,
    smoke_trainer_battle: Option<SmokeScriptCommandRef>,
    smoke_visible_start_menu: Option<PathBuf>,
    smoke_visible_bag_item: Vec<SmokeShopTransactionRef>,
    smoke_visible_title_new_game: bool,
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

#[derive(Debug, Clone, Copy)]
enum SmokeFieldItemKind {
    Repel,
    Bicycle,
    Itemfinder,
    TownMap,
    EscapeRope,
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
                    .context("--smoke-field-item requires Kind:ItemId:Quantity")?;
                args.smoke_field_item = Some(parse_smoke_field_item_ref(&value)?);
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
    let [kind, item_id, quantity] = parts.as_slice() else {
        bail!("--smoke-field-item must be Kind:ItemId:Quantity");
    };
    let kind = match *kind {
        "Repel" => SmokeFieldItemKind::Repel,
        "Bicycle" => SmokeFieldItemKind::Bicycle,
        "Itemfinder" => SmokeFieldItemKind::Itemfinder,
        "TownMap" => SmokeFieldItemKind::TownMap,
        "EscapeRope" => SmokeFieldItemKind::EscapeRope,
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
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --list-map-objects MapName"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --list-map-events MapName"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-save <path> [--smoke-buttons right,a] [--smoke-script 'right*8;down;down']"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --smoke-title-new-game <path>"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --smoke-load-save <path> [--save-path <roundtrip-path>]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --smoke-visible-title-new-game [--save-path <path>]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --smoke-visible-title-continue <path>"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-visible-start-menu <path> --smoke-party Species:Level[:HeldItem] --smoke-visible-bag-item ItemId:Quantity"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-visible-party --smoke-party Species:Level[:HeldItem] --smoke-party Species:Level[:HeldItem]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-visible-overworld --smoke-script 'right*8;down;a'"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-visible-wild-battle MapName:SourceScript:CommandIndex --smoke-party Species:Level[:HeldItem] [--smoke-visible-bag-item ItemId:Quantity]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-visible-trainer-battle MapName:SourceScript:CommandIndex --smoke-party Species:Level[:HeldItem]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-shop MapName:SourceScript:CommandIndex"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-shop MapName:SourceScript:CommandIndex [--smoke-money 1000] [--smoke-buy POTION:1] [--smoke-sell POTION:1] [--smoke-save /tmp/shop.crystalsave]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-field-item Repel:REPEL:1 [--smoke-save /tmp/field-item.crystalsave]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-field-move Kind:Species:Level:MoveId[:args] [--smoke-save /tmp/field-move.crystalsave]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-fishing Rod:GOOD_ROD|Item:GOOD_ROD [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] --smoke-party Species:Level[:HeldItem] [--smoke-save /tmp/fishing.crystalsave]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-menu MapName:SourceScript:LoadmenuIndex:VerticalmenuIndex:OptionIndex:Option [--smoke-save /tmp/menu.crystalsave]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-interact [--smoke-start-map MapName:RuntimeTileX:RuntimeTileY] [--smoke-script 'right;up;a'] [--smoke-save /tmp/interact.crystalsave]"
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
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-wild-battle MapName:SourceScript:CommandIndex --smoke-party Species:Level[:HeldItem] [--smoke-player-action Move:0 --smoke-enemy-action Move:0 | --smoke-capture-ball MASTER_BALL | --smoke-battle-item X_ATTACK] [--smoke-save /tmp/battle.crystalsave]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> --smoke-trainer-battle MapName:SourceScript:CommandIndex --smoke-party Species:Level[:HeldItem] [--smoke-save /tmp/trainer.crystalsave]"
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
    let use_result = match field_item.kind {
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

fn smoke_field_move_use(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    field_move: &SmokeFieldMoveRef,
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

fn smoke_title_new_game(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    save_path: &PathBuf,
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
        .set_trainer_identity("CHRIS", initial.trainer.player_id)
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
    snapshot
        .bag
        .items
        .iter()
        .chain(snapshot.bag.balls.iter())
        .chain(snapshot.bag.key_items.iter())
        .chain(snapshot.bag.pc_items.iter())
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

#[cfg(not(feature = "bevy-shell"))]
compile_error!("crystal-bevy binary requires building with --features bevy-shell");
