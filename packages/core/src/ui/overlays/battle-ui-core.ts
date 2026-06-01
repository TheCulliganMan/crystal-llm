import * as battle_dialogue from './battle-dialogue';
import { DataLoader } from '../../core/data-loader';
import { GameState, createInitialGameState } from '../../core/state';
import { Surface } from '../surface';
import { BaseUI } from '../base-ui';
import type { BaseFontRenderer } from '../base-ui';
import { _CHAR_MAP, _CHAR_MAP_VALUES, _CLEAR_TILE } from '../tilemap-surface';
import { AnimationPlayer } from './_battle-animation';
import { pushDebugLog } from '@pokecrystal/core/core/debug-log';
import { build_exp_tiles, build_hp_tiles } from './battle-bars';
import { DialogueState } from './battle-dialogue';
import { DialogueWindow, YesNoPrompt, type DialogueUI } from '../text/dialogue';
import { BattleInputState, BattleWRAM, ensure_menu_cursor } from './battle-input';
import {
  build_trainer_entrance_animation,
  build_trainer_exit_animation,
  TrainerEntranceAnimation,
} from './battle-intro';
import { BACKPIC_TILE_HEIGHT, BACKPIC_TILE_WIDTH } from './trainer-entrance';
import { build_trainer_victory_slide } from './trainer-victory';
import { validateBattleLayout } from './_battle-layout-validation';
import { BattleVRAMAllocator, BattleHardwareRegisters } from './_battle-vram';
import {
  BattleBackgroundTilemap,
  BattleUILayoutFactory,
  PAL_TEXT_WINDOW,
  PAL_HP_GREEN,
  build_battle_tilemap,
  build_battle_tileset,
  build_palette_variants,
} from './_battle-background';
import type { BattleUILayout } from './_battle-layout';
import { render_battle_background } from './battle-scene';
import { BATTLE_MENU_HEADER, BattleMenu, layout_matches_header } from './_battle-menu';
import { BattleUIPhase, BattleUIState, TrainerSpriteMode, ensure_move_metadata, move_display_name_for_state, init_hp_animation_states } from './battle-ui-state';
import { AudioEngine } from '../../engine/systems/audio';
import { BattleSpriteOAMManager } from './battle-oam';
import { PlayerGender } from '../../core/enums';

type BattleFont = BaseFontRenderer & {
  font_tiles: Record<number, Surface>;
};

export type BattleUI = BaseUI & {
  tile_size: number;
  font: BattleFont;
  _apply_colorkey_transparency?: (surface: Surface) => Surface;
  get_sprite_surface?: (spriteId: string, spriteType: string, frame?: number) => Surface | null;
  _get_pokemon_frame_surface?: (speciesId: string, frame: number) => Surface | null;
};

type TrainerIntroUI = {
  get_sprite_surface: (spriteId: string, spriteType: string, frame?: number) => Surface | null;
  _apply_colorkey_transparency: (surface: Surface) => Surface;
  _get_pokemon_frame_surface: (speciesId: string, frame: number) => Surface | null;
};

const lastBattleWaitStatus = new WeakMap<BattleUIState, string | null>();

const require_trainer_intro_ui = (ui: BattleUI): TrainerIntroUI => {
  if (!ui.get_sprite_surface || !ui._apply_colorkey_transparency || !ui._get_pokemon_frame_surface) {
    throw new Error("Battle UI is missing trainer intro rendering hooks.");
  }
  return ui as TrainerIntroUI;
};

const require_palette_ui = (ui: BattleUI): { _apply_colorkey_transparency: (surface: Surface) => Surface } => {
  if (!ui._apply_colorkey_transparency) {
    throw new Error("Battle UI is missing palette post-processing.");
  }
  return ui as { _apply_colorkey_transparency: (surface: Surface) => Surface };
};

const createBattleInputState = (): BattleInputState => ({
  active_direction: null,
  repeat_timer: 0,
});

const createBattleWRAM = (): BattleWRAM => ({
  current_menu: BattleMenu.MAIN,
  menu_header: BATTLE_MENU_HEADER,
  wBattleMenuCursorPosition: 0,
  wMoveMenuCursorPosition: 0,
  wPartyMenuCursorPosition: 0,
  wPackMenuCursorPosition: 0,
  wBattleHasJustStarted: 0,
  wBattleTextDelay: 0,
  wTextDelayFlags: 0,
  wInputType: 0,
  confirm_pressed: false,
  cancel_pressed: false,
  select_pressed: false,
  last_num_moves: 0,
  last_party_size: 0,
  wBattleMenuCursorPositionNext: 0,
  swapping_move_index: null,
  last_item_names: [],
});

export const _copy_tilemap = (
  source: BattleBackgroundTilemap,
  target: BattleBackgroundTilemap,
  options?: { copy_attributes?: boolean },
): void => {
  const copyAttributes = options?.copy_attributes ?? true;
  for (let row = 0; row < target.height; row += 1) {
    target.tiles[row] = [...source.tiles[row]];
    if (copyAttributes) {
      target.attributes[row] = [...source.attributes[row]];
    }
  }
  target.markDirty();
};

export const _describe_wram = (wram: BattleWRAM): string => {
  return (
    `menu=${wram.current_menu} cursor=${wram.wBattleMenuCursorPosition} ` +
    `move_cursor=${wram.wMoveMenuCursorPosition} party_cursor=${wram.wPartyMenuCursorPosition} ` +
    `pack_cursor=${wram.wPackMenuCursorPosition} confirm=${wram.confirm_pressed} ` +
    `cancel=${wram.cancel_pressed} select=${wram.select_pressed} ` +
    `swap=${wram.swapping_move_index ?? '-'} last_moves=${wram.last_num_moves} ` +
    `last_party=${wram.last_party_size}`
  );
};

const REQUIRED_BATTLE_GLYPHS = new Set<number>([..._CHAR_MAP_VALUES, _CLEAR_TILE]);

export const create_battle_ui = (
  ui: BattleUI,
  options?: {
    layout?: BattleUILayout | null;
    game_state?: GameState | null;
    data_loader?: DataLoader | null;
    load_data?: boolean;
  },
): BattleUIState => {
  const layout = options?.layout ?? null;
  const gameState = options?.game_state ?? null;
  const dataLoader = options?.data_loader ?? null;
  const loadData = options?.load_data ?? true;

  if (loadData && dataLoader?.ensure_battle_data) {
    dataLoader.ensure_battle_data();
  }

  const resolvedLayout = layout ?? BattleUILayoutFactory.fromAsmDefaults();
  validateBattleLayout(resolvedLayout, ui.tile_size);
  const resolvedState = gameState ?? createInitialGameState();

  if (!layout_matches_header(resolvedLayout, BATTLE_MENU_HEADER)) {
    console.debug('Battle layout overrides the default menu header; cursor asserts disabled.');
  }

  const baseTilemap = build_battle_tilemap(resolvedLayout);
  const workingTilemap = new BattleBackgroundTilemap();
  _copy_tilemap(baseTilemap, workingTilemap);

  const vram = new BattleVRAMAllocator();
  const glyphIds = new Set<number>();
  for (const tileId of REQUIRED_BATTLE_GLYPHS) {
    if (tileId in ui.font.font_tiles) {
      glyphIds.add(tileId);
    }
  }

  const [tilesetRaw, baseTiles] = build_battle_tileset(ui.font.font_tiles, {
    allocator: vram,
    textbox_frame: resolvedState.sram.textbox_frame,
  });
  const tileset: Record<number, Record<number, Surface>> = { ...tilesetRaw };
  if (glyphIds.size) {
    const variants = build_palette_variants(ui.font.font_tiles, Array.from(glyphIds).sort((a, b) => a - b), {
      tiles: baseTiles,
      allocator: vram,
    });
    Object.assign(tileset, variants);
  }

  const hpTiles = build_hp_tiles(ui.font.font_tiles);
  const expTiles = build_exp_tiles(ui.font.font_tiles);
  const dialogueWindow = new DialogueWindow(ui, resolvedState, Math.max(1, resolvedLayout.text_box.height_tiles - 2));
  const dialogue: DialogueState = {
    window: resolvedLayout.text_box,
    dialogue: dialogueWindow,
    queue: [],
    pending_waits: 0,
    forced_visible: false,
    auto_close_after_display: false,
  };

  return {
    ui,
    layout: resolvedLayout,
    game_state: resolvedState,
    tilemap: workingTilemap,
    tilemap_base: baseTilemap,
    tileset,
    base_tiles: { ...baseTiles },
    hp_tiles: hpTiles,
    exp_tiles: expTiles,
    dialogue,
    animation_player: new AnimationPlayer(ui),
    data_loader: dataLoader ?? null,
    wram: createBattleWRAM(),
    vram,
    hardware: new BattleHardwareRegisters(),
    waiting_for_input: false,
    manual_wait_override: false,
    ui_phase: BattleUIPhase.INACTIVE,
    presented_this_frame: false,
    dialogue_wait_gate_active: false,
    pending_animation_events: [],
    fast_animation_request: false,
    fast_text_request: false,
    active: true,
    exp_animation: null,
    exp_animation_queue: [],
    sprites_enabled: true,
    trainer_sprites_visible: false,
    trainer_send_out_seen: false,
    trainer_hud_visible: false,
    pending_trainer_exit: false,
    pending_trainer_exit_side: null,
    trainer_sprite_override_mode: null,
    trainer_overlay_player_visible: null,
    trainer_overlay_enemy_visible: null,
    force_party_menu: false,
    block_on_pending_evolution: false,
    pending_evolutions: [],
    pending_move_learns: [],
    pending_nickname_request: null,
    block_on_move_learning: false,
    scx: 0,
    scy: 0,
    palette_registers: {},
    hp_palettes: { player: PAL_HP_GREEN, enemy: PAL_HP_GREEN },
    hp_animation_states: init_hp_animation_states(),
    oam_manager: new BattleSpriteOAMManager(),
    animation_clock: { frame: 0, tick: () => {} },
    input_state: createBattleInputState(),
    bag_repeat_state: createBattleInputState(),
    pokemon_repeat_state: createBattleInputState(),
    yes_no_prompt: {
      active: false,
      result: null,
      pending_activation: false,
      prompt: null,
    },
    player_sprite_frame: 0,
    enemy_sprite_frame: 0,
    frontpic_animation: null,
    _sprite_frame_counts: {},
    _sprite_frame_timers: {},
    _sprite_frame_indices: {},
    _frontpic_animators: {},
    _loaded_battle_sprites: new Set(),
    is_mock: false,
    trainer_intro: null,
    trainer_victory: null,
    trainer_exit: null,
    evolution_animation: null,
    active_evolution: null,
    context: null,
    audio_engine: null,
    pending_pack_action: null,
    pending_pokemon_selection: null,
    battle_item_target_selection: false,
    bag_menu: null,
    pokemon_menu: null,
    pokemon_stats: null,
    active_move_learn: null,
    move_forget_menu: null,
    _move_metadata: ensure_move_metadata(),
    _type_display_name: move_display_name_for_state,
  } as unknown as BattleUIState;
};

const _reset_wram = (wram: BattleWRAM): void => {
  wram.current_menu = BattleMenu.MAIN;
  wram.menu_header = BATTLE_MENU_HEADER;
  wram.wBattleMenuCursorPosition = 0;
  ensure_menu_cursor(wram);
  wram.wMoveMenuCursorPosition = 0;
  wram.wPartyMenuCursorPosition = 0;
  wram.wPackMenuCursorPosition = 0;
  wram.confirm_pressed = false;
  wram.cancel_pressed = false;
  wram.select_pressed = false;
  wram.swapping_move_index = null;
  wram.last_num_moves = 0;
  wram.last_party_size = 0;
  wram.last_item_names = [];
};

export const reset_menu_selection = (state: BattleUIState): void => {
  _reset_wram(state.wram);
  state.input_state.active_direction = null;
  state.input_state.repeat_timer = 0;
  state.pending_pack_action = null;
  state.bag_menu = null;
  state.bag_repeat_state.active_direction = null;
  state.bag_repeat_state.repeat_timer = 0;
  state.pokemon_menu = null;
  state.pokemon_stats = null;
  state.pending_pokemon_selection = null;
  state.battle_item_target_selection = false;
  state.pokemon_repeat_state.active_direction = null;
  state.pokemon_repeat_state.repeat_timer = 0;
};

export const begin_battle = (state: BattleUIState): void => {
  state.active = true;
  state.waiting_for_input = false;
  state.manual_wait_override = false;
  state.dialogue_wait_gate_active = false;
  state.pending_animation_events = [];
  state.fast_animation_request = false;
  state.exp_animation = null;
  state.exp_animation_queue = [];
  state.trainer_intro = null;
  state.trainer_victory = null;
  state.trainer_exit = null;
  state.evolution_animation = null;
  state.pending_evolutions = [];
  state.active_evolution = null;
  state.block_on_pending_evolution = false;
  state.sprites_enabled = true;
  state.trainer_sprites_visible = false;
  state.trainer_send_out_seen = false;
  state.trainer_hud_visible = false;
  state.pending_trainer_exit = false;
  state.pending_trainer_exit_side = null;
  state.trainer_sprite_override_mode = null;
  state.trainer_overlay_player_visible = null;
  state.trainer_overlay_enemy_visible = null;
  battle_dialogue.reset_dialogue(state.dialogue);
  state.animation_player.reset();
  // ASM mapping: engine/battle/core.asm::DrawPlayerHUD/DrawEnemyHUD redraw HP bars from
  // current battler HP on battle entry; stale animation progress must not carry over.
  state.hp_animation_states = init_hp_animation_states();
  state.vram.toggle_oam(true);
  state.hardware.set_scroll(0, 0);
  state.vram.record_scroll(0, 0);
  _reset_wram(state.wram);
  state.game_state.wram.wBattleTextDelay = 0;
  state.game_state.wram.wTextDelayFlags = 0;
  state.player_sprite_frame = 0;
  state.enemy_sprite_frame = 0;
  state.frontpic_animation = null;
  state._sprite_frame_timers = {};
  state._sprite_frame_indices = {};
  state._frontpic_animators = {};
  state._loaded_battle_sprites = new Set();
  console.info(`Battle UI activated; ${_describe_wram(state.wram)}`);
  _initialize_text_window(state);
  state.ui_phase = BattleUIPhase.MENU;
};

export const end_battle = (state: BattleUIState): void => {
  state.active = false;
  state.waiting_for_input = false;
  state.manual_wait_override = false;
  state.fast_animation_request = false;
  state.pending_animation_events = [];
  state.exp_animation = null;
  state.exp_animation_queue = [];
  state.evolution_animation = null;
  state.trainer_victory = null;
  state.trainer_exit = null;
  state.pending_trainer_exit = false;
  state.pending_trainer_exit_side = null;
  state.pending_evolutions = [];
  state.active_evolution = null;
  state.block_on_pending_evolution = false;
  battle_dialogue.reset_dialogue(state.dialogue);
  state.animation_player.reset();
  state.player_sprite_frame = 0;
  state.enemy_sprite_frame = 0;
  state.frontpic_animation = null;
  state._sprite_frame_timers = {};
  state._sprite_frame_indices = {};
  state._frontpic_animators = {};
  state._loaded_battle_sprites = new Set();
  state.dialogue_wait_gate_active = false;
  state.trainer_sprites_visible = false;
  state.trainer_send_out_seen = false;
  state.trainer_hud_visible = false;
  state.trainer_sprite_override_mode = null;
  state.trainer_overlay_player_visible = null;
  state.trainer_overlay_enemy_visible = null;
  state.game_state.wram.wBattleTextDelay = 0;
  state.game_state.wram.wTextDelayFlags = 0;
  state.ui_phase = BattleUIPhase.INACTIVE;
  console.info('Battle UI deactivated');
};

const _initialize_text_window = (state: BattleUIState): void => {
  const window = state.layout.text_box;
  state.tilemap_base.drawWindow(window.tile_x, window.tile_y, window.width_tiles, window.height_tiles, {
    attr: PAL_TEXT_WINDOW,
  });
};

export const set_waiting_for_input = (state: BattleUIState, value: boolean): void => {
  state.manual_wait_override = Boolean(value);
  state.waiting_for_input = Boolean(value);
};

export const is_waiting_for_input = (state: BattleUIState): boolean =>
  state.waiting_for_input || state.manual_wait_override;

export const set_text_box_visible = (state: BattleUIState, value: boolean): void => {
  if (value) {
    battle_dialogue.force_text_box(state.dialogue, true);
  } else {
    battle_dialogue.close_text_box(state.dialogue);
  }
};

export const is_text_box_visible = (state: BattleUIState): boolean => state.dialogue.forced_visible;

export const set_game_state = (state: BattleUIState, gameState: GameState | null): void => {
  const resolved = gameState ?? createInitialGameState();
  state.game_state = resolved;
  state.dialogue.dialogue.game_state = resolved;
};

export const set_audio_engine = (state: BattleUIState, audioEngine: AudioEngine | null): void => {
  state.audio_engine = audioEngine;
};

const _build_intro_background_surface = (state: BattleUIState): Surface | null => {
  try {
    const width = Math.max(state.ui.screen.width, 256);
    const surface = new Surface(width, state.ui.screen.height);
    surface.fill([255, 255, 255, 255]);
    const tilemap = new BattleBackgroundTilemap();
    const window = state.layout.text_box;
    tilemap.drawWindow(window.tile_x, window.tile_y, window.width_tiles, window.height_tiles, {
      attr: PAL_TEXT_WINDOW,
    });
    const scene = new Surface(state.ui.screen.width, state.ui.screen.height);
    render_battle_background(scene, tilemap, state.tileset, {
      scx: state.hardware.scx,
      scy: state.hardware.scy,
    });
    let x = 0;
    while (x < surface.width) {
      surface.blit(scene, [x, 0]);
      x += scene.width;
    }
    return surface;
  } catch {
    return null;
  }
};

export const start_trainer_intro = (
  state: BattleUIState,
  options: {
    player_gender: PlayerGender;
    trainer_class: string;
    enemy_species: string;
    battle_type: string;
    enemy_party_size?: number;
  },
): void => {
  if (state.trainer_intro) {
    return;
  }
  const builder = build_trainer_entrance_animation;
  const builderOptions: {
    player_gender: PlayerGender;
    trainer_class: string;
    enemy_species: string;
    battle_type: string;
    hardware: BattleHardwareRegisters;
    enemy_party_size: number;
    palette_state?: Record<string, number | null> | null;
    background_surface?: Surface | null;
  } = {
    player_gender: options.player_gender,
    trainer_class: options.trainer_class,
    enemy_species: options.enemy_species,
    battle_type: options.battle_type,
    hardware: state.hardware,
    enemy_party_size: options.enemy_party_size ?? 0,
  };
  if (state.animation_player.palette_state) {
    builderOptions.palette_state = state.animation_player.palette_state;
  }
  const backgroundSurface = _build_intro_background_surface(state);
  if (backgroundSurface) {
    builderOptions.background_surface = backgroundSurface;
  }
  const trainerIntro = builder(require_trainer_intro_ui(state.ui as unknown as BattleUI), builderOptions);
  if (trainerIntro instanceof TrainerEntranceAnimation) {
    state.trainer_intro = trainerIntro;
    state.trainer_hud_visible = true;
  }
  const tileCount = BACKPIC_TILE_WIDTH * BACKPIC_TILE_HEIGHT;
  state.vram.record_tiles({
    start_tile: 0x31,
    tile_count: tileCount,
    source: 'InitBattleDisplay.PlaceGraphic',
  });
  state.sprites_enabled = false;
};

export const trainer_intro_active = (state: BattleUIState): boolean => state.trainer_intro !== null;

export const start_trainer_victory_slide = (
  state: BattleUIState,
  options: { trainer_class: string },
): void => {
  if (state.trainer_victory) {
    return;
  }
  const slide = build_trainer_victory_slide(require_palette_ui(state.ui as unknown as BattleUI), {
    trainer_class: options.trainer_class,
    palette_state: state.animation_player.palette_state ?? null,
  });
  state.trainer_victory = slide;
  state.sprites_enabled = false;
};

export const show_trainer_sprites = (
  state: BattleUIState,
  options?: { mode?: TrainerSpriteMode },
): void => {
  const mode = options?.mode ?? 'both';
  state.trainer_sprite_override_mode = mode;
  state.trainer_sprites_visible = true;
  state.trainer_send_out_seen = false;
  state.sprites_enabled = true;
};

export const dialogue_ready_for_yes_no = (state: BattleUIState): boolean => {
  const dlgState = state.dialogue;
  const window = dlgState.dialogue;
  if (dlgState.queue.length) {
    return false;
  }
  if (window.has_more_pages()) {
    return false;
  }
  return window.is_complete();
};

const ensure_yes_no_prompt = (state: BattleUIState): YesNoPrompt | null => {
  const existing = state.yes_no_prompt.prompt;
  if (existing) {
    return existing;
  }
  const ui = state.ui as unknown as DialogueUI | null | undefined;
  if (!ui || !ui.screen || typeof (ui.screen as { blit?: unknown }).blit !== "function") {
    return null;
  }
  const prompt = new YesNoPrompt(ui, state.audio_engine ?? null);
  state.yes_no_prompt.prompt = prompt;
  return prompt;
};

const reset_yes_no_prompt_state = (state: BattleUIState): void => {
  const prompt = ensure_yes_no_prompt(state);
  if (!prompt) {
    return;
  }
  prompt.selection = 0;
  prompt.finished = false;
};

export const activate_pending_yes_no_prompt = (state: BattleUIState): void => {
  const prompt = state.yes_no_prompt;
  if (!prompt.pending_activation) {
    return;
  }
  if (!dialogue_ready_for_yes_no(state)) {
    return;
  }
  prompt.pending_activation = false;
  prompt.active = true;
  prompt.result = null;
  reset_yes_no_prompt_state(state);
  battle_dialogue.force_text_box(state.dialogue, true);
};

export const show_yes_no_prompt = (state: BattleUIState): void => {
  const prompt = state.yes_no_prompt;
  prompt.active = true;
  prompt.result = null;
  prompt.pending_activation = false;
  reset_yes_no_prompt_state(state);
  battle_dialogue.force_text_box(state.dialogue, true);
};

export const get_yes_no_prompt_result = (state: BattleUIState): boolean | null =>
  state.yes_no_prompt.result;

export const clear_yes_no_prompt = (state: BattleUIState): void => {
  const prompt = state.yes_no_prompt;
  prompt.result = null;
  prompt.pending_activation = false;
};

export const force_party_menu_selection = (
  state: BattleUIState,
  party_size: number,
  options?: { preferred_index?: number | null },
): void => {
  state.force_party_menu = true;
  state.battle_item_target_selection = false;
  state.wram.current_menu = BattleMenu.POKEMON;
  const size = Math.max(0, party_size);
  state.wram.last_party_size = size;
  state.pending_pokemon_selection = null;
  if (size <= 0) {
    state.wram.wPartyMenuCursorPosition = 0;
    return;
  }
  const preferredIndex = options?.preferred_index;
  const storedCursor = state.wram.wPartyMenuCursorPosition;
  const rawCursor =
    preferredIndex === null || preferredIndex === undefined ? storedCursor : preferredIndex;
  const cursor = rawCursor >= 0 && rawCursor < size ? rawCursor : 0;
  state.wram.wPartyMenuCursorPosition = cursor;
};

export const release_force_party_menu = (state: BattleUIState): void => {
  state.force_party_menu = false;
};

export const should_block_state_advance = (state: BattleUIState): boolean => {
  const instantMode = Boolean(state.game_state?.wram?.instant_mode);
  const dialogueWait = battle_dialogue.waiting_flag(state.dialogue);
  const pendingLevels = state.exp_animation !== null;
  const pendingExpQueue = (state.exp_animation_queue?.length ?? 0) > 0;
  const animations = state.animation_player.is_active();
  const pendingAnimationEvents = (state.pending_animation_events?.length ?? 0) > 0;
  const waits = state.manual_wait_override || state.waiting_for_input;
  const delayGateActive = !instantMode && state.dialogue_wait_gate_active && state.game_state.wram.wBattleTextDelay > 0;
  const evolutionActive = state.evolution_animation && state.evolution_animation.is_running;
  const pendingQueue = state.block_on_pending_evolution || state.block_on_move_learning;
  const victoryAnimating = state.trainer_victory && !state.trainer_victory.is_finished;
  return Boolean(
    state.trainer_intro ||
      state.trainer_exit ||
      victoryAnimating ||
      dialogueWait ||
      pendingLevels ||
      pendingExpQueue ||
      animations ||
      pendingAnimationEvents ||
      waits ||
      delayGateActive ||
      evolutionActive ||
      pendingQueue,
  );
};

export const _sync_waiting_flag = (state: BattleUIState): void => {
  const instantMode = Boolean(state.game_state?.wram?.instant_mode);
  const dialogueWait = battle_dialogue.waiting_flag(state.dialogue);
  const promptActive = state.yes_no_prompt.active;
  const delayGateActive = !instantMode && state.dialogue_wait_gate_active && state.game_state.wram.wBattleTextDelay > 0;
  const evolutionActive = state.evolution_animation && state.evolution_animation.is_running;
  const victoryAnimating = state.trainer_victory && !state.trainer_victory.is_finished;
  state.waiting_for_input = Boolean(
    state.manual_wait_override ||
      dialogueWait ||
      promptActive ||
      evolutionActive ||
      state.block_on_pending_evolution ||
      state.block_on_move_learning ||
      delayGateActive,
  );
  if (state.trainer_intro || victoryAnimating || state.trainer_exit || state.animation_player.is_active()) {
    state.ui_phase = BattleUIPhase.ANIMATION;
  } else if (dialogueWait || promptActive || state.waiting_for_input) {
    state.ui_phase = BattleUIPhase.DIALOGUE;
  } else {
    state.ui_phase = BattleUIPhase.MENU;
  }
  const waitSummary =
    `dlg=${Number(dialogueWait)} pending=${state.dialogue.pending_waits} queue=${state.dialogue.queue.length} ` +
    `complete=${Number(state.dialogue.dialogue.is_complete())} prompt=${Number(promptActive)} ` +
    `manual=${Number(state.manual_wait_override)} phase=${state.ui_phase}`;
  const previousSummary = lastBattleWaitStatus.get(state) ?? null;
  if (waitSummary !== previousSummary) {
    pushDebugLog(`[battle] wait ${waitSummary}`);
    lastBattleWaitStatus.set(state, waitSummary);
  }
  if (
    promptActive ||
    state.manual_wait_override ||
    state.dialogue.queue.length ||
    state.dialogue.pending_waits > 0 ||
    !state.dialogue.dialogue.is_complete() ||
    evolutionActive ||
    state.block_on_pending_evolution ||
    state.block_on_move_learning ||
    delayGateActive
  ) {
    return;
  }
  state.waiting_for_input = false;
};

export const _dialogue_wait_gate_active = (state: BattleUIState): boolean => {
  const dlg = state.dialogue.dialogue;
  if (!dlg.is_complete()) {
    return false;
  }
  if (state.dialogue.pending_waits > 0) {
    return true;
  }
  if (dlg.has_more_pages()) {
    return true;
  }
  if (state.dialogue.queue.length) {
    return true;
  }
  return false;
};

export const start_trainer_exit_animation = (state: BattleUIState, options?: { side?: string }): void => {
  if (state.trainer_exit) {
    return;
  }
  const normalizedSide = options?.side?.trim().toLowerCase() ?? 'player';
  state.trainer_exit = build_trainer_exit_animation(
    require_trainer_intro_ui(state.ui as unknown as BattleUI),
    state.hardware,
    { side: normalizedSide },
  );
  state.sprites_enabled = true;
  state.trainer_sprites_visible = true;
};
