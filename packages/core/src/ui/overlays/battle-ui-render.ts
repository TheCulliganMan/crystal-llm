import { AUTO_INPUT } from '../../engine/battle/auto-input';
import { PlayerGender } from '../../core/enums';
import { B_PAD_B } from '../../input/controls';
import { BagMenu } from '../menus/bag-menu';
import { PokemonMenu } from '../menus/pokemon-menu';
import { PokemonStatsScreen } from '../menus/pokemon-stats';
import * as battle_dialogue from './battle-dialogue';
import { apply_battle_inputs } from './battle-input';
import { render_battle_background } from './battle-scene';
import { BattleMenu } from './_battle-menu';
import {
  buildBagControlLines,
  buildBagMenuLines,
  buildBattleSnapshot,
  buildPokemonMenuControlLines,
  buildPokemonMenuLines,
  buildPokemonStatsControlLines,
  render_battle_text_overlay,
  renderTextSnapshot,
} from '../text-overlays';
import * as draw from './battle-ui-draw';
import { _SPACE_TILE } from '../tilemap-surface';
import {
  apply_palette_registers,
  apply_runtime_postprocessing,
  dispatch_animation_audio,
  draw_animation_sprites,
  draw_battle_sprites,
  draw_trainer_hud_icons,
  overlay_move_windows,
  update_battle_sprite_frames,
} from './battle-ui-sprites';
import {
  _copy_tilemap,
  _dialogue_wait_gate_active,
  _sync_waiting_flag,
  activate_pending_yes_no_prompt,
  release_force_party_menu,
  start_trainer_exit_animation,
} from './battle-ui-core';
import {
  apply_auto_joypad_inputs,
  flush_deferred_animation_events,
  forward_pack_menu_inputs,
  forward_pokemon_menu_inputs,
  forward_pokemon_stats_inputs,
} from './battle-ui-input';
import {
  complete_active_evolution,
  enqueue_exp_gain,
  maybe_cancel_active_evolution,
  maybe_start_pending_evolution,
  process_move_learning,
  update_exp_animation,
} from './battle-ui-moves';
import { BattleUIState, BattleTextDelayFlag, BATTLE_TEXT_ADVANCE_DELAY_FRAMES } from './battle-ui-state';
import { TrainerExitAnimationPair, resolve_player_backpic_id } from './battle-intro';
import { BattleContext } from '../../engine/battle/battle/battle-context';
import { createBagMenuUI, createPokemonMenuUI } from './battle-ui-menu-utils';
import { Surface } from '../surface';
import type { BattleAnimationRuntime } from './battle-bg-effects';
import { MonMenuItem } from '../../core/enums/mon-menu';
import { PartyMenuAction } from '../../core/enums/party-menu';
import { MonType } from '../../core/enums/pokemon';

const normalize_trainer_sprite_id = (trainer_class: string): string => {
  const normalized = trainer_class.trim().toLowerCase();
  if (normalized.endsWith('m') && !normalized.endsWith('_m')) {
    return `${normalized.slice(0, -1)}_m`;
  }
  if (normalized.endsWith('f') && !normalized.endsWith('_f')) {
    return `${normalized.slice(0, -1)}_f`;
  }
  return normalized;
};

const reset_presentation_flag = (state: BattleUIState): void => {
  state.presented_this_frame = false;
};

const isTextOnlyBattleUi = (ui: BattleUIState["ui"]): boolean => {
  const candidate = ui as { renderSnapshot?: unknown; getChildren?: () => unknown[] } | null;
  return Boolean(candidate) &&
    typeof candidate?.renderSnapshot === "function" &&
    typeof candidate?.getChildren !== "function";
};

const present_frame = (state: BattleUIState): void => {
  state.ui.update();
  state.presented_this_frame = true;
};

const is_send_out_animation = (animation_player: { current_animation_script?: { name: string } | null }): boolean => {
  const current = animation_player.current_animation_script;
  if (!current) {
    return false;
  }
  const name = current.name ?? '';
  const normalized = name.replace(/_/g, '').replace(/\s+/g, '').toLowerCase();
  return normalized.includes('sendoutmon');
};

// Tutorial-only fast-forward to let manual input advance the Pokeball throw animation.
const FAST_TUTORIAL_ANIMATION_TICKS = 12;

const is_throw_pokeball_animation = (
  animation_player: { current_animation_script?: { name: string } | null },
): boolean => {
  const current = animation_player.current_animation_script;
  if (!current) {
    return false;
  }
  const name = current.name ?? '';
  const normalized = name.replace(/_/g, '').replace(/\s+/g, '').toLowerCase();
  return normalized.includes('throwpokeball');
};

const clear_trainer_overrides = (runtime: BattleAnimationRuntime): void => {
  runtime.enemy_sprite_type_override = null;
  runtime.enemy_sprite_override = null;
  runtime.player_sprite_type_override = null;
  runtime.player_sprite_override = null;
};

const finish_trainer_exit_animation = (
  state: BattleUIState,
  runtime: BattleAnimationRuntime,
  targetSide: string,
): void => {
  if (targetSide === 'both') {
    runtime.player_offset_x = 0;
    runtime.enemy_offset_x = 0;
  } else if (targetSide === 'enemy') {
    runtime.enemy_offset_x = 0;
  } else {
    runtime.player_offset_x = 0;
  }
  restore_trainer_overlay_visibility(state, runtime);
  state.trainer_exit = null;
  state.trainer_hud_visible = false;
  state.trainer_sprites_visible = false;
  state.trainer_sprite_override_mode = null;
};

export const reset_battler_visibility = (runtime: BattleAnimationRuntime): void => {
  runtime.player_visible = true;
  runtime.enemy_visible = true;
};

export const advance_animation_player = (
  state: BattleUIState,
  options: { tutorialBattle: boolean },
): void => {
  const fastForward =
    options.tutorialBattle &&
    Boolean(state.fast_animation_request) &&
    is_throw_pokeball_animation(state.animation_player);
  const ticks = fastForward ? FAST_TUTORIAL_ANIMATION_TICKS : 1;
  state.fast_animation_request = false;
  for (let i = 0; i < ticks; i += 1) {
    state.animation_player.update();
    if (!state.animation_player.is_active()) {
      break;
    }
  }
};

const restore_trainer_overlay_visibility = (state: BattleUIState, runtime: BattleAnimationRuntime): void => {
  const playerVisible = state.trainer_overlay_player_visible;
  const enemyVisible = state.trainer_overlay_enemy_visible;
  if (playerVisible !== null && playerVisible !== undefined) {
    runtime.player_visible = playerVisible;
  }
  if (enemyVisible !== null && enemyVisible !== undefined) {
    runtime.enemy_visible = enemyVisible;
  }
  state.trainer_overlay_player_visible = null;
  state.trainer_overlay_enemy_visible = null;
};

const trainer_exit_blocked_by_dialogue = (state: BattleUIState): boolean => {
  if (state.trainer_intro) {
    return true;
  }
  if (state.dialogue.forced_visible || battle_dialogue.waiting_flag(state.dialogue)) {
    return true;
  }
  if (_dialogue_wait_gate_active(state)) {
    return true;
  }
  return false;
};

export const maybe_start_pending_trainer_exit = (state: BattleUIState): void => {
  if (!state.pending_trainer_exit || state.trainer_exit) {
    return;
  }
  if (trainer_exit_blocked_by_dialogue(state)) {
    return;
  }
  const side = state.pending_trainer_exit_side ?? undefined;
  if (side === 'player') {
    // ASM mapping: engine/battle/core.asm::SlideBattlePicOut (a=9) slides the player backpic out.
    // Keep the player trainer sprite visible so the slide-out is rendered.
    state.trainer_sprite_override_mode = 'player';
    state.trainer_sprites_visible = true;
  }
  start_trainer_exit_animation(state, side ? { side } : undefined);
  state.pending_trainer_exit = false;
  state.pending_trainer_exit_side = null;
};

export const apply_trainer_overlay = (
  state: BattleUIState,
  options: {
    trainerOverlayActive: boolean;
    battleStartActive: boolean;
    sendOutActive: boolean;
    tutorialBattle: boolean;
    throwPokeballActive?: boolean;
  }
): void => {
  const runtime = state.animation_player.runtime_state;
  if (options.tutorialBattle && !state.trainer_sprites_visible) {
    state.trainer_sprites_visible = true;
    state.trainer_sprite_override_mode = 'player';
  }
  if (!state.trainer_sprites_visible) {
    clear_trainer_overrides(runtime);
    restore_trainer_overlay_visibility(state, runtime);
    return;
  }

  // ASM: engine/battle/core.asm::ShowSetEnemyMonAndSendOutAnimation replaces trainer pics at send-out.
  if (options.sendOutActive && !options.tutorialBattle) {
    clear_trainer_overrides(runtime);
    // During the enemy send-out, the opponent's Pokemon is visible while the player side
    // stays hidden until BattleMonEntrance restores the player backpic for "Go! <MON>!".
    runtime.player_visible = false;
    runtime.enemy_visible = true;
    state.trainer_sprites_visible = false;
    state.trainer_sprite_override_mode = null;
    state.trainer_overlay_player_visible = null;
    state.trainer_overlay_enemy_visible = null;
    return;
  }

  if (state.trainer_overlay_player_visible === null && state.trainer_overlay_enemy_visible === null) {
    state.trainer_overlay_player_visible = Boolean(runtime.player_visible);
    state.trainer_overlay_enemy_visible = Boolean(runtime.enemy_visible);
  }
  const mode = state.trainer_sprite_override_mode ?? (options.trainerOverlayActive ? 'both' : 'player');
  const enemyModeActive =
    options.trainerOverlayActive &&
    !options.tutorialBattle &&
    (mode === 'enemy' || mode === 'both');
  const playerModeActive = mode === 'player' || mode === 'both';

  if (enemyModeActive) {
    runtime.enemy_sprite_type_override = 'trainer';
    const trainerClassOverride = String((state.game_state.wram as { other_trainer_class?: string }).other_trainer_class ?? '');
    runtime.enemy_sprite_override = normalize_trainer_sprite_id(trainerClassOverride);
  }
  runtime.enemy_visible = true;

  if (playerModeActive) {
    runtime.player_sprite_type_override = 'player_back';
    const wram = state.game_state.wram as { player_gender?: PlayerGender; battle_type?: string };
    const gender = wram.player_gender ?? PlayerGender.MALE;
    const spriteId = resolve_player_backpic_id(gender, String(wram.battle_type ?? ''));
    runtime.player_sprite_override = spriteId;
    runtime.player_visible = !(options.tutorialBattle && options.throwPokeballActive);
  } else if (options.trainerOverlayActive && !options.tutorialBattle) {
    // ASM: engine/battle/core.asm::BattleIntro -> ClearSprites hides the player backpic before enemy send-out.
    runtime.player_visible = false;
  }

  let hideOverlay = false;
  if (!options.tutorialBattle) {
    if (state.trainer_send_out_seen && !options.sendOutActive) {
      hideOverlay = true;
    }
    if (hideOverlay) {
      clear_trainer_overrides(runtime);
      restore_trainer_overlay_visibility(state, runtime);
      state.trainer_sprites_visible = false;
      state.trainer_sprite_override_mode = null;
    }
  }
};

export const update = (state: BattleUIState, battle_context: BattleContext): void => {
  reset_presentation_flag(state);
  if (!state.active) {
    return;
  }
  state.context = battle_context;
  const battleWram = state.game_state.wram;
  const battleType = String(battleWram.battle_type ?? '').toUpperCase();
  const tutorialBattle = battleType === 'BATTLETYPE_TUTORIAL';
  const trainerClass = String(battleWram.other_trainer_class ?? '').trim();
  const trainerOverlayActive = Boolean(battle_context.trainerBattle || trainerClass);
  state.animation_clock.tick();
  state.oam_manager.reset();
  state.wram.menu_header = draw.menu_header_for_battle(state);

  const intro = state.trainer_intro;
  if (intro) {
    intro.draw(state.ui.screen, state.oam_manager);
    if (intro.is_finished) {
      const runtime = state.animation_player.runtime_state;
      state.trainer_intro = null;
      state.trainer_hud_visible = false;
      state.sprites_enabled = true;
      state.trainer_send_out_seen = false;
      reset_battler_visibility(runtime);
      if (trainerOverlayActive) {
        state.trainer_sprites_visible = true;
        state.trainer_sprite_override_mode = 'both';
      } else {
        state.trainer_sprites_visible = true;
        state.trainer_sprite_override_mode = 'player';
      }
    }
    draw_trainer_hud_icons(state, battle_context);
    present_frame(state);
    _sync_waiting_flag(state);
    return;
  }

  if (state.pending_trainer_exit && !state.trainer_exit) {
    maybe_start_pending_trainer_exit(state);
  }

  const exitAnim = state.trainer_exit;
  if (exitAnim) {
    exitAnim.draw(state.ui.screen);
  }

  const victorySlide = state.trainer_victory;
  if (victorySlide && !victorySlide.is_finished) {
    victorySlide.draw(state.ui.screen);
    present_frame(state);
    _sync_waiting_flag(state);
    return;
  }

  const joypad = state.game_state.hram.joypad ?? null;
  const cancel_requested = Boolean(joypad && (joypad.hJoyPressed & B_PAD_B));

  process_move_learning(state);
  const dialogueWaiting = battle_dialogue.waiting_flag(state.dialogue);
  if (!state.evolution_animation) {
    maybe_start_pending_evolution(state, battle_context);
  }
  if (maybe_cancel_active_evolution(state, cancel_requested)) {
    state.dialogue.dialogue.update();
    battle_dialogue.auto_close_if_idle(state.dialogue, state.yes_no_prompt.active);
    _sync_waiting_flag(state);
    return;
  }

  const cutscene = state.evolution_animation;
  if (cutscene) {
    const rendered = cutscene.update(state.ui.screen, {
      dialogue_waiting: dialogueWaiting,
      cancel_requested,
    });
    const finished = cutscene.is_finished;
    const cancelled = Boolean(cutscene.was_cancelled);
    if (finished || cancelled) {
      state.evolution_animation = null;
      complete_active_evolution(state, cancelled);
      state.dialogue.dialogue.update();
      battle_dialogue.auto_close_if_idle(state.dialogue, state.yes_no_prompt.active);
      _sync_waiting_flag(state);
      return;
    }
    if (rendered) {
      state.dialogue.dialogue.update();
      battle_dialogue.auto_close_if_idle(state.dialogue, state.yes_no_prompt.active);
      _sync_waiting_flag(state);
      return;
    }
  }

  if (state.wram.current_menu === BattleMenu.POKEMON) {
    render_pokemon_menu(state);
    return;
  }

  if (state.wram.current_menu === BattleMenu.PACK) {
    render_pack_menu(state);
    return;
  }

  if (joypad) {
    const menuActive = !battle_dialogue.is_visible(state.dialogue);
    apply_battle_inputs(state.wram, joypad, state.input_state, { menu_active: menuActive });
  }
  const inputType = Number((state.game_state.wram as { wInputType?: number }).wInputType ?? 0);
  if (joypad && inputType === AUTO_INPUT) {
    apply_auto_joypad_inputs(state, joypad);
  }

  advance_animation_player(state, { tutorialBattle });
  flush_deferred_animation_events(state);
  const activeExit = state.trainer_exit;
  let exitFinishedThisFrame = false;
  let exitFinishedSide = 'player';
  if (activeExit) {
    const runtime = state.animation_player.runtime_state;
    if (activeExit instanceof TrainerExitAnimationPair) {
      runtime.player_offset_x = activeExit.player_offset_x;
      runtime.enemy_offset_x = activeExit.enemy_offset_x;
      if (activeExit.is_finished) {
        state.trainer_hud_visible = false;
        exitFinishedThisFrame = true;
        exitFinishedSide = 'both';
      }
    } else if (typeof activeExit === 'object' && activeExit !== null) {
      const targetSide = (activeExit as { target_side?: string }).target_side ?? 'player';
      if (targetSide === 'enemy') {
        runtime.enemy_offset_x = (activeExit as { x_offset?: number }).x_offset ?? 0;
      } else {
        runtime.player_offset_x = (activeExit as { x_offset?: number }).x_offset ?? 0;
      }
      if ((activeExit as { is_finished?: boolean }).is_finished) {
        state.trainer_hud_visible = false;
        exitFinishedThisFrame = true;
        exitFinishedSide = targetSide;
      }
    }
  }
  const wram = state.game_state.wram as { wBattleHasJustStarted?: number };
  const battleStartActive = Boolean(wram.wBattleHasJustStarted);

  const sendOutActive = is_send_out_animation(state.animation_player);
  const throwPokeballActive = is_throw_pokeball_animation(state.animation_player);
  if (sendOutActive) {
    state.trainer_send_out_seen = true;
  }
  const suppressHud =
    battleStartActive && !state.trainer_send_out_seen && !sendOutActive;
  apply_trainer_overlay(state, {
    trainerOverlayActive,
    battleStartActive,
    sendOutActive,
    tutorialBattle,
    throwPokeballActive,
  });

  state.vram.toggle_oam(state.animation_player.oam_enabled);
  apply_palette_registers(state);
  const runtime = state.animation_player.runtime_state;
  state.scx = Number(runtime.screen_offset_x ?? 0);
  state.scy = Number(runtime.screen_offset_y ?? 0);
  state.hardware.set_scroll(state.scx, state.scy);
  state.vram.record_scroll(state.scx, state.scy);
  _copy_tilemap(state.tilemap_base, state.tilemap);
  if (suppressHud) {
    state.tilemap.clear_box(1, 0, 11, 4, { tile: _SPACE_TILE, attr: 0 });
    state.tilemap.clear_box(9, 7, 11, 5, { tile: _SPACE_TILE, attr: 0 });
  } else if (tutorialBattle) {
    state.tilemap.clear_box(9, 7, 11, 5, { tile: _SPACE_TILE, attr: 0 });
  }
  draw.render_text_window_band(state, battle_context);
  state.dialogue.dialogue.update();
  activate_pending_yes_no_prompt(state);
  update_exp_animation(state);
  if (isTextOnlyBattleUi(state.ui)) {
    dispatch_animation_audio(state);
    render_battle_text_overlay(state);
    battle_dialogue.auto_close_if_idle(state.dialogue, state.yes_no_prompt.active);
    update_battle_text_delay(state);
    _sync_waiting_flag(state);
    return;
  }
  const player = battle_context.playerPokemon;
  const enemy = battle_context.enemyPokemon;
  update_battle_sprite_frames(state, player, enemy);
  if (!suppressHud) {
    draw.draw_enemy_hud(state, enemy, Boolean(battle_context.trainerBattle));
    if (!tutorialBattle) {
      draw.draw_player_hud(state, player);
    }
  }
  draw.draw_dialogue_or_menu(state, player, battle_context);
  draw.draw_move_forget_menu(state);
  const screen = state.ui.screen as Surface;
  render_battle_background(screen, state.tilemap, state.tileset, {
    scx: state.hardware.scx,
    scy: state.hardware.scy,
    line_offsets_y: runtime.lcd_pointer ? runtime.line_scroll_y : null,
  });
  draw.draw_yes_no_prompt(state);
  if (victorySlide) {
    victorySlide.draw(screen);
  }
  draw_battle_sprites(state, player, enemy, runtime);
  draw_trainer_hud_icons(state, battle_context);
  draw_animation_sprites(state);
  overlay_move_windows(state);
  apply_runtime_postprocessing(state, runtime);
  dispatch_animation_audio(state);
  render_battle_text_overlay(state);
  present_frame(state);
  if (exitFinishedThisFrame) {
    finish_trainer_exit_animation(
      state,
      state.animation_player.runtime_state,
      exitFinishedSide,
    );
  }
  battle_dialogue.auto_close_if_idle(state.dialogue, state.yes_no_prompt.active);
  update_battle_text_delay(state);
  _sync_waiting_flag(state);
};

export const render_pack_menu = (state: BattleUIState): void => {
  if (!state.bag_menu) {
    state.bag_menu = new BagMenu(
      createBagMenuUI(state.ui),
      state.game_state,
      state.audio_engine ?? null,
      state.data_loader ?? undefined
    );
    state.pending_pack_action = null;
    state.bag_repeat_state.active_direction = null;
    state.bag_repeat_state.repeat_timer = 0;
  }
  forward_pack_menu_inputs(state);
  state.bag_menu?.draw?.();
  if (state.bag_menu) {
    const bagMenu = state.bag_menu;
    const snapshot = buildBattleSnapshot(state);
    const hasPrompt = Boolean(snapshot?.promptLines?.length);
    const hasDialogue = Boolean(snapshot?.dialogueLines?.length);
    const infoLines = hasPrompt || hasDialogue ? (snapshot?.infoLines ?? []) : buildBagControlLines(bagMenu);
    renderTextSnapshot(state.ui, {
      viewportLines: snapshot?.viewportLines ?? ["BAG"],
      infoLines,
      viewportTitle: snapshot?.viewportTitle ?? "Bag",
      infoTitle: snapshot?.infoTitle ?? "Legend",
      menuLines: buildBagMenuLines(bagMenu),
      promptLines: snapshot?.promptLines ?? null,
      dialogueLines: snapshot?.dialogueLines ?? null,
    });
  }
  present_frame(state);
  _sync_waiting_flag(state);
};

export const render_pokemon_menu = (state: BattleUIState): void => {
  if (!state.pokemon_menu) {
    open_pokemon_menu(state);
  }
  if (state.pokemon_stats) {
    const statsResult = forward_pokemon_stats_inputs(state);
    if (statsResult === "exit") {
      state.pokemon_stats = null;
      _sync_waiting_flag(state);
      return;
    }
    state.pokemon_stats.draw();
    const snapshot = buildBattleSnapshot(state);
    const hasPrompt = Boolean(snapshot?.promptLines?.length);
    const hasDialogue = Boolean(snapshot?.dialogueLines?.length);
    const infoLines = hasPrompt || hasDialogue
      ? (snapshot?.infoLines ?? [])
      : buildPokemonStatsControlLines(state.pokemon_stats.getActivePokemon());
    const statsOverlay = state.pokemon_stats.getTextOverlay();
    renderTextSnapshot(state.ui, {
      viewportLines: statsOverlay.viewportLines,
      infoLines,
      viewportTitle: snapshot?.viewportTitle ?? "Pokemon Stats",
      infoTitle: snapshot?.infoTitle ?? "Legend",
      menuLines: statsOverlay.menuLines ?? null,
      promptLines: snapshot?.promptLines ?? null,
      dialogueLines: snapshot?.dialogueLines ?? null,
    });
    present_frame(state);
    _sync_waiting_flag(state);
    return;
  }
  const result = forward_pokemon_menu_inputs(state);
  handle_pokemon_menu_input_result(state, result);
  if (!state.pokemon_menu) {
    _sync_waiting_flag(state);
    return;
  }
  state.pokemon_menu?.draw?.();
  if (state.pokemon_menu) {
    const snapshot = buildBattleSnapshot(state);
    const hasPrompt = Boolean(snapshot?.promptLines?.length);
    const hasDialogue = Boolean(snapshot?.dialogueLines?.length);
    const infoLines = hasPrompt || hasDialogue
      ? (snapshot?.infoLines ?? [])
      : buildPokemonMenuControlLines(state.pokemon_menu);
    renderTextSnapshot(state.ui, {
      viewportLines: snapshot?.viewportLines ?? ["POKEMON MENU"],
      infoLines,
      viewportTitle: snapshot?.viewportTitle ?? "Pokemon Menu",
      infoTitle: snapshot?.infoTitle ?? "Legend",
      menuLines: buildPokemonMenuLines(state.pokemon_menu),
      promptLines: snapshot?.promptLines ?? null,
      dialogueLines: snapshot?.dialogueLines ?? null,
    });
  }
  present_frame(state);
  if (state.pending_pokemon_selection !== null && state.pending_pokemon_selection !== undefined) {
    close_pokemon_menu(state, true);
  }
  _sync_waiting_flag(state);
};

export const handle_pokemon_menu_selection = (state: BattleUIState, pokemon: unknown): boolean => {
  const party = state.game_state.sram.party.pokemon ?? [];
  const resolvedIndex = party.findIndex((member) => member === pokemon);
  if (resolvedIndex < 0) {
    throw new Error("Battle Pokemon menu selected a party member that is not in the current party.");
  }
  const index = resolvedIndex;
  state.pending_pokemon_selection = index;
  state.wram.wPartyMenuCursorPosition = index;
  return true;
};

const resolveIndexedPartyPokemon = (
  state: BattleUIState,
  partyIndex: number | null | undefined,
  errorMessage: string,
): { index: number; pokemon: import('../../core/models').Pokemon } => {
  const party = state.game_state.sram.party.pokemon ?? [];
  const index = typeof partyIndex === "number" ? partyIndex : -1;
  const pokemon = index >= 0 ? party[index] : undefined;
  if (!pokemon) {
    throw new Error(errorMessage);
  }
  return { index, pokemon: pokemon as import("../../core/models").Pokemon };
};

const open_pokemon_stats_screen = (state: BattleUIState, partyIndex: number | null | undefined): void => {
  if (!state.pokemon_stats) {
    state.pokemon_stats = new PokemonStatsScreen(state.ui, state.game_state);
  }
  const { index, pokemon } = resolveIndexedPartyPokemon(
    state,
    partyIndex,
    "Battle Pokemon stats selected a party member that is not in the current party.",
  );
  state.pokemon_stats.showPokemon(pokemon, { monType: MonType.PARTYMON, partyIndex: index });
};

const open_pokemon_menu = (state: BattleUIState): void => {
  state.pending_pokemon_selection = null;
  state.pokemon_repeat_state.active_direction = null;
  state.pokemon_repeat_state.repeat_timer = 0;
  state.pokemon_menu = new PokemonMenu(
    createPokemonMenuUI(state.ui),
    state.game_state,
    state.audio_engine ?? null,
    { action: PartyMenuAction.SWITCH, battle_menu: true, switch_behavior: "select" }
  );
  state.pokemon_menu?.registerMonMenuHandler?.(MonMenuItem.SWITCH, (_menu, _pokemon, partyIndex) => {
    const { pokemon } = resolveIndexedPartyPokemon(
      state,
      partyIndex,
      "Battle Pokemon menu selected a party member that is not in the current party.",
    );
    handle_pokemon_menu_selection(state, pokemon);
  });
  state.pokemon_menu?.registerMonMenuHandler?.(MonMenuItem.STATS, (_menu, _pokemon, partyIndex) => {
    open_pokemon_stats_screen(state, partyIndex);
  });
  if (state.force_party_menu || state.battle_item_target_selection) {
    state.pokemon_menu?.requestSelection({
      handler: (pokemon) => handle_pokemon_menu_selection(state, pokemon),
    });
  }
};

const handle_pokemon_menu_input_result = (
  state: BattleUIState,
  result: [string, number] | null
): void => {
  if (!result) {
    return;
  }
  const [action] = result;
  if (action !== 'cancel') {
    return;
  }
  if (state.force_party_menu) {
    return;
  }
  close_pokemon_menu(state, false);
};

const close_pokemon_menu = (state: BattleUIState, keep_pending_selection: boolean): void => {
  if (!state.pokemon_menu) {
    return;
  }
  state.pokemon_menu = null;
  state.pokemon_stats = null;
  state.pokemon_repeat_state.active_direction = null;
  state.pokemon_repeat_state.repeat_timer = 0;
  state.wram.current_menu = BattleMenu.MAIN;
  state.battle_item_target_selection = false;
  release_force_party_menu(state);
  if (!keep_pending_selection) {
    state.pending_pokemon_selection = null;
  }
};

const update_battle_text_delay = (state: BattleUIState): void => {
  const gating = _dialogue_wait_gate_active(state);
  const visible = battle_dialogue.is_visible(state.dialogue);
  if (visible) {
    let flags = BattleTextDelayFlag.TEXT_DELAY;
    if (state.fast_text_request) {
      flags |= BattleTextDelayFlag.FAST_TEXT_DELAY;
    }
    state.game_state.wram.wTextDelayFlags = flags;
  } else {
    state.game_state.wram.wTextDelayFlags = 0;
  }
  if (gating && !state.dialogue_wait_gate_active) {
    state.game_state.wram.wBattleTextDelay = BATTLE_TEXT_ADVANCE_DELAY_FRAMES;
  }
  state.dialogue_wait_gate_active = gating;
  if (state.game_state.wram.wBattleTextDelay > 0) {
    state.game_state.wram.wBattleTextDelay -= 1;
  }
  state.fast_text_request = false;
};

export { enqueue_exp_gain };
