import { GameState } from '../../core/state';
import { LearnedMove, Move, Pokemon } from '../../core/models';
import { MoveName, PokemonType } from '../../core/enums';
import { AudioEngine } from '../../engine/systems/audio';
import { BattleContext } from '../../engine/battle/battle/battle-context';
import { BattleBackgroundTilemap, PAL_HP_GREEN } from './_battle-background';
import { BattleMenu, BattleMenuHeader } from './_battle-menu';
import { BattleBarTiles, ExpBarTiles, HP_BAR_LENGTH_PX } from './battle-bars';
import type { BattleTextWindow, BattleUILayout } from './_battle-layout';
import type { BattleUI } from './ui-types';
import { BattleSpriteOAMManager } from './battle-oam';
import { BattleHardwareRegisters, BattleVRAMAllocator } from './_battle-vram';
import { DialogueState } from './battle-dialogue';
import { AnimationPlayer } from './_battle-animation';
import { _CHAR_MAP } from '../tilemap-surface';
import { Surface } from '../surface';
import { loadMoveMetadata } from './battle-experience';
import { selected_move_entry, type_display_name } from './_battle-hud-helpers';
import { Evolution } from '../../engine/systems/evolution';
import { DataLoader } from '../../core/data-loader';
import type { BagMenu } from '../menus/bag-menu';
import type { MenuUI } from '../menus/types';
import type { PokemonMenu, PokemonMenuUI } from '../menus/pokemon-menu';
import type { PokemonStatsScreen } from '../menus/pokemon-stats';
import type { TrainerEntranceAnimation, TrainerExitAnimation, TrainerExitAnimationPair } from './battle-intro';
import type { TrainerVictorySlide } from './trainer-victory';
import type { Event } from '../../engine/world/events';
import type { YesNoPrompt } from '../text/dialogue';
import type { FrontpicAnimator } from './pokemon-frontpic-animation';

export const STATICMENU_CURSOR_TILE = _CHAR_MAP['\u25b6'];
export const HP_ANIM_STEP_FRAMES = 2;
const HP_YELLOW_THRESHOLD = Math.floor((HP_BAR_LENGTH_PX * 50) / 100);
const HP_RED_THRESHOLD = Math.floor((HP_BAR_LENGTH_PX * 21) / 100);

export enum HPBarThreshold {
  RED = 0,
  YELLOW = 1,
  GREEN = 2,
}

export const _hp_zone_for_pixels = (pixels: number): HPBarThreshold => {
  if (pixels <= HP_RED_THRESHOLD) {
    return HPBarThreshold.RED;
  }
  if (pixels <= HP_YELLOW_THRESHOLD) {
    return HPBarThreshold.YELLOW;
  }
  return HPBarThreshold.GREEN;
};

export enum BattleTextDelayFlag {
  FAST_TEXT_DELAY = 1 << 0,
  TEXT_DELAY = 1 << 1,
}

export const BACKGROUND_COLOUR: [number, number, number] = [248, 248, 248];
export const BATTLE_TEXT_COLOUR: [number, number, number] = [56, 48, 48];
export const _BATTLE_SPRITE_FRAME_DELAY = 18;
// ASM mapping: home/print_text.asm::PrintLetterDelay handles text pacing; battle prompts
// should not impose an extra fixed multi-frame post-confirm lock.
export const BATTLE_TEXT_ADVANCE_DELAY_FRAMES = 0;

export type TrainerSpriteMode = 'player' | 'enemy' | 'both';

export type FrontpicAnimationRequest = {
  side: 'player' | 'enemy';
  speed: number;
};

export enum BattleUIPhase {
  INACTIVE = 'INACTIVE',
  TRANSITION = 'TRANSITION',
  MENU = 'MENU',
  DIALOGUE = 'DIALOGUE',
  ANIMATION = 'ANIMATION',
}

export type YesNoPromptState = {
  active: boolean;
  result: boolean | null;
  pending_activation: boolean;
  prompt: YesNoPrompt | null;
};

export type PendingEvolutionRequest = {
  pokemon: Pokemon;
};

export type ActiveEvolutionState = {
  pokemon: Pokemon;
  evolution: Evolution;
  previous_species_id: string;
  target_species_id: string;
};

export enum MoveLearningPhase {
  ANNOUNCE = 'ANNOUNCE',
  DECIDE = 'DECIDE',
  ASK_FORGET = 'ASK_FORGET',
  WAIT_FORGET_PROMPT = 'WAIT_FORGET_PROMPT',
  FORGET_PROMPT_RESULT = 'FORGET_PROMPT_RESULT',
  FORGET_MENU_TEXT = 'FORGET_MENU_TEXT',
  PREPARE_FORGET_MENU = 'PREPARE_FORGET_MENU',
  FORGET_MENU = 'FORGET_MENU',
  HANDLE_MENU_SELECTION = 'HANDLE_MENU_SELECTION',
  HM_WARNING = 'HM_WARNING',
  STOP_PROMPT = 'STOP_PROMPT',
  WAIT_STOP_PROMPT = 'WAIT_STOP_PROMPT',
  STOP_PROMPT_RESULT = 'STOP_PROMPT_RESULT',
  DID_NOT_LEARN = 'DID_NOT_LEARN',
  FORGET_ANIMATION = 'FORGET_ANIMATION',
  WAIT_FORGET_ANIMATION = 'WAIT_FORGET_ANIMATION',
  LEARN_NEW_MOVE = 'LEARN_NEW_MOVE',
  FINAL = 'FINAL',
}

export type PendingMoveLearn = {
  pokemon: Pokemon;
  move: LearnedMove;
};

export type ActiveMoveLearnState = {
  pokemon: Pokemon;
  move: LearnedMove;
  stage: MoveLearningPhase;
  replace_index: number | null;
  pending_selection: number | null;
  forget_move_name: MoveName | null;
};

export type PendingNicknamePrompt = {
  pokemon: Pokemon | null;
  species_name: string;
};

export type MoveForgetMenuState = {
  selection: number;
  option_count: number;
};

export class HPBarAnimationState {
  current_pixels = 0;
  target_pixels = 0;
  max_hp = 0;
  direction = 0;
  frames_until_step = 0;
  active = false;
  last_threshold = HPBarThreshold.GREEN;
  initialized = false;
  subject_token: unknown = null;

  sync(
    requested_pixels: number,
    maxHpValue: number,
    options?: { subject_token?: unknown; snap?: boolean }
  ): void {
    const bounded = Math.max(0, Math.min(HP_BAR_LENGTH_PX, requested_pixels));
    const subjectChanged =
      Object.prototype.hasOwnProperty.call(options ?? {}, 'subject_token') &&
      this.subject_token !== options?.subject_token;
    if (!this.initialized || this.max_hp !== maxHpValue || subjectChanged || options?.snap) {
      this.current_pixels = bounded;
      this.target_pixels = bounded;
      this.max_hp = maxHpValue;
      this.direction = 0;
      this.frames_until_step = 0;
      this.active = false;
      this.last_threshold = _hp_zone_for_pixels(bounded);
      this.initialized = true;
      if (Object.prototype.hasOwnProperty.call(options ?? {}, 'subject_token')) {
        this.subject_token = options?.subject_token;
      }
      return;
    }
    if (bounded === this.target_pixels) {
      return;
    }
    this.target_pixels = bounded;
    if (bounded > this.current_pixels) {
      this.direction = 1;
      this.active = true;
      this.frames_until_step = 0;
      return;
    }
    this.direction = -1;
    this.active = true;
    this.frames_until_step = 0;
  }

  step(audio_engine?: AudioEngine | null): number {
    if (!this.active) {
      return this.current_pixels;
    }
    if (this.frames_until_step > 0) {
      this.frames_until_step -= 1;
      return this.current_pixels;
    }
    this.frames_until_step = HP_ANIM_STEP_FRAMES;
    let nextPixels = Math.max(
      0,
      Math.min(HP_BAR_LENGTH_PX, this.current_pixels + this.direction)
    );
    if (
      (this.direction > 0 && nextPixels >= this.target_pixels) ||
      (this.direction < 0 && nextPixels <= this.target_pixels)
    ) {
      nextPixels = this.target_pixels;
      this.active = false;
    }
    const previousZone = this.last_threshold;
    const currentZone = _hp_zone_for_pixels(nextPixels);
    if (this.direction < 0 && currentZone < previousZone && audio_engine) {
      try {
        audio_engine.playSound('SFX_MENU');
      } catch {
        // Ignore missing SFX.
      }
    }
    this.last_threshold = currentZone;
    this.current_pixels = nextPixels;
    return this.current_pixels;
  }
}

export type BattleInputState = {
  active_direction: string | null;
  repeat_timer: number;
};

export type BattleUIRoot = BattleUI & MenuUI & PokemonMenuUI & {
  update: () => void;
  _apply_colorkey_transparency?: (surface: Surface) => Surface;
  get_sprite_surface?: (spriteId: string, spriteType: string, frame?: number) => Surface | null;
  getSpriteSurface?: (spriteId: string, options?: { sprite_type?: string; frame?: number }) => Surface | null;
  _get_pokemon_frame_surface?: (speciesId: string, frame: number) => Surface | null;
};

export type EvolutionCutsceneLike = {
  is_running?: boolean;
  is_finished: boolean;
  update: (surface: Surface, options?: { dialogue_waiting?: boolean; cancel_requested?: boolean }) => boolean;
  was_cancelled?: boolean;
};

export type BattleWRAM = {
  current_menu: BattleMenu;
  menu_header: BattleMenuHeader | null;
  wBattleMenuCursorPosition: number;
  wMoveMenuCursorPosition: number;
  wPartyMenuCursorPosition: number;
  wPackMenuCursorPosition: number;
  wBattleHasJustStarted: number;
  wBattleTextDelay: number;
  wTextDelayFlags: number;
  wInputType: number;
  confirm_pressed: boolean;
  cancel_pressed: boolean;
  select_pressed: boolean;
  last_num_moves: number;
  last_party_size: number;
  wBattleMenuCursorPositionNext?: number;
  swapping_move_index?: number | null;
  last_item_names?: string[];
};

export type BattleRuntimeState = {
  screen_offset_x?: number;
  screen_offset_y?: number;
  player_visible?: boolean;
  enemy_visible?: boolean;
  player_offset_x?: number;
  enemy_offset_x?: number;
  player_sprite_type_override?: string | null;
  enemy_sprite_type_override?: string | null;
  player_sprite_override?: string | null;
  enemy_sprite_override?: string | null;
};

export type BattleUIState = {
  ui: BattleUIRoot;
  layout: BattleUILayout;
  game_state: GameState;
  tilemap: BattleBackgroundTilemap;
  tilemap_base: BattleBackgroundTilemap;
  tileset: Record<number, Record<number, Surface>>;
  base_tiles: Record<number, Surface>;
  hp_tiles: BattleBarTiles;
  exp_tiles: ExpBarTiles;
  dialogue: DialogueState;
  animation_player: AnimationPlayer;
  context: BattleContext | null;
  hardware: BattleHardwareRegisters;
  vram: BattleVRAMAllocator;
  trainer_intro?: TrainerEntranceAnimation | null;
  trainer_victory?: TrainerVictorySlide | null;
  trainer_exit?: TrainerExitAnimation | TrainerExitAnimationPair | null;
  evolution_animation?: EvolutionCutsceneLike | null;
  pending_evolutions: PendingEvolutionRequest[];
  active_evolution?: ActiveEvolutionState | null;
  player_sprite_frame: number;
  enemy_sprite_frame: number;
  frontpic_animation: FrontpicAnimationRequest | null;
  _sprite_frame_counts: Record<string, number>;
  _sprite_frame_timers: Record<string, number>;
  _sprite_frame_indices: Record<string, number>;
  _frontpic_animators: Record<string, FrontpicAnimator>;
  _loaded_battle_sprites: Set<string>;
  audio_engine?: AudioEngine | null;
  data_loader?: DataLoader | null;
  waiting_for_input: boolean;
  manual_wait_override: boolean;
  ui_phase: BattleUIPhase;
  presented_this_frame: boolean;
  dialogue_wait_gate_active: boolean;
  pending_animation_events: Event[];
  fast_animation_request: boolean;
  fast_text_request: boolean;
  active: boolean;
  exp_animation?: unknown | null;
  exp_animation_queue?: unknown[];
  sprites_enabled: boolean;
  trainer_sprites_visible: boolean;
  trainer_send_out_seen: boolean;
  trainer_hud_visible: boolean;
  pending_trainer_exit: boolean;
  pending_trainer_exit_side?: string | null;
  trainer_sprite_override_mode?: TrainerSpriteMode | null;
  trainer_overlay_player_visible?: boolean | null;
  trainer_overlay_enemy_visible?: boolean | null;
  is_mock: boolean;
  wram: BattleWRAM;
  input_state: BattleInputState;
  bag_menu: BagMenu | null;
  pokemon_menu: PokemonMenu | null;
  pokemon_stats: PokemonStatsScreen | null;
  pending_pack_action?: [string, string] | null;
  bag_repeat_state: BattleInputState;
  pokemon_repeat_state: BattleInputState;
  pending_pokemon_selection?: number | null;
  battle_item_target_selection?: boolean;
  yes_no_prompt: YesNoPromptState;
  force_party_menu: boolean;
  block_on_pending_evolution: boolean;
  pending_move_learns: PendingMoveLearn[];
  active_move_learn?: ActiveMoveLearnState | null;
  move_forget_menu?: MoveForgetMenuState | null;
  pending_nickname_request: PendingNicknamePrompt | null;
  block_on_move_learning: boolean;
  scx: number;
  scy: number;
  palette_registers: Record<string, number>;
  hp_palettes: Record<string, number>;
  hp_animation_states: Record<string, HPBarAnimationState>;
  oam_manager: BattleSpriteOAMManager;
  animation_clock: { frame: number; tick: () => void };
  _move_metadata: Map<MoveName, Move>;
  _type_display_name: (pokemonType: PokemonType) => string;
};

export const init_hp_animation_states = (): Record<string, HPBarAnimationState> => {
  return {
    player: new HPBarAnimationState(),
    enemy: new HPBarAnimationState(),
  };
};

export const ensure_move_metadata = (): Map<MoveName, Move> => {
  return loadMoveMetadata();
};

export const move_display_name_for_state = (
  pokemonType: PokemonType
): string => {
  return type_display_name(pokemonType);
};
