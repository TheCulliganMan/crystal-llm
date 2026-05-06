import { Surface } from '../surface';
import { PlayerGender } from '../../core/enums';
import { BattleHardwareRegisters } from './_battle-vram';
import { BattleSpriteOAMManager } from './battle-oam';
import { TrainerEntranceAnimation, SCREEN_HEIGHT, SCREEN_WIDTH, TOTAL_FRAMES } from './trainer-entrance';

// Mirrors pokecrystal_disassembly/engine/battle/core.asm (InitBattleDisplay/SlideBattlePicOut)
// and pokecrystal_disassembly/engine/battle/sliding_intro.asm (BattleIntroSlidingPics).
export { TrainerEntranceAnimation };
import { load_trainer_portrait_surface } from '../trainer-portraits';
import { load_player_backpic_surface } from '../player-backpics';
import { normalise_trainer_id } from './trainer-sprite-id';

export type BattleUI = {
  get_sprite_surface: (sprite_id: string, sprite_type: string) => Surface | null;
  _apply_colorkey_transparency: (surface: Surface) => Surface;
  _get_pokemon_frame_surface: (species_id: string, frame: number) => Surface | null;
};

const normalize_battle_type = (battle_type: string): string => battle_type.trim().toUpperCase();

export const resolve_player_backpic_id = (gender: PlayerGender, battle_type: string): string => {
  const normalized = normalize_battle_type(battle_type);
  // ASM: engine/events/catch_tutorial.asm::DudeTutorial uses the DUDE backpic during the tutorial battle.
  if (normalized === 'BATTLETYPE_TUTORIAL') {
    return 'dude';
  }
  return gender === PlayerGender.FEMALE ? 'kris_back' : 'chris_back';
};

const load_player_back_surface = (
  ui: BattleUI,
  gender: PlayerGender,
  battle_type: string
): Surface => {
  const spriteId = resolve_player_backpic_id(gender, battle_type);
  let surface: Surface | null = null;
  try {
    surface = load_player_backpic_surface(spriteId, { player_gender: gender });
  } catch {
    surface = ui.get_sprite_surface(spriteId, 'player_back');
  }
  if (!surface) {
    throw new Error(`Missing player back sprite surface for ${spriteId}.`);
  }
  return surface;
};

const load_enemy_surface = (
  ui: BattleUI,
  trainer_class: string,
  enemy_species: string
): Surface | null => {
  if (trainer_class) {
    const spriteId = normalise_trainer_id(trainer_class);
    const surface = load_trainer_portrait_surface(spriteId);
    return ui._apply_colorkey_transparency(surface);
  }
  return ui._get_pokemon_frame_surface(enemy_species, 0);
};

export class BattleIntroAnimation extends TrainerEntranceAnimation {
  constructor(
    ui: BattleUI,
    options: {
      player_gender: PlayerGender;
      trainer_class: string;
      enemy_species: string;
      battle_type: string;
      hardware: BattleHardwareRegisters;
      enemy_party_size: number;
      background_surface?: Surface | null;
      screen_size?: [number, number];
      palette_state?: Record<string, number | null> | null;
    }
  ) {
    const playerSurface = load_player_back_surface(ui, options.player_gender, options.battle_type);
    const enemySurface = load_enemy_surface(ui, options.trainer_class, options.enemy_species);
    super({
      player_surface: playerSurface,
      enemy_surface: enemySurface,
      background_surface: options.background_surface ?? null,
      hardware: options.hardware,
      palette_state: options.palette_state ?? null,
      screen_size: options.screen_size ?? [SCREEN_WIDTH, SCREEN_HEIGHT],
      strict_player_size: true,
      enemy_party_size: options.enemy_party_size,
    });
  }
}

export const build_battle_intro_animation = (
  ui: BattleUI,
  options: {
    player_gender: PlayerGender;
    trainer_class: string;
    enemy_species: string;
    battle_type: string;
    hardware: BattleHardwareRegisters;
    enemy_party_size: number;
    background_surface?: Surface | null;
    palette_state?: Record<string, number | null> | null;
  }
): BattleIntroAnimation => {
  return new BattleIntroAnimation(ui, {
    player_gender: options.player_gender,
    trainer_class: options.trainer_class,
    enemy_species: options.enemy_species,
    battle_type: options.battle_type,
    hardware: options.hardware,
    palette_state: options.palette_state ?? null,
    enemy_party_size: options.enemy_party_size,
    background_surface: options.background_surface ?? null,
  });
};

export const build_trainer_entrance_animation = build_battle_intro_animation;

export class TrainerExitAnimation {
  private static readonly PLAYER_WIDTH_TILES = 9;
  private static readonly ENEMY_WIDTH_TILES = 8;
  private static readonly STEP_DELAY_FRAMES = 2;
  private static readonly TILE_SIZE = 8;

  private readonly side: 'player' | 'enemy';
  private readonly width_tiles: number;
  private readonly step_pixels: number;
  private steps_remaining: number;
  private frame_cooldown = 0;
  private offset_x = 0;
  private finished = false;

  constructor(
    _ui: BattleUI,
    _options: { hardware: BattleHardwareRegisters; side?: string; tile_width_tiles?: number | null }
  ) {
    const side = (_options.side ?? 'player').trim().toLowerCase();
    this.side = side === 'enemy' ? 'enemy' : 'player';
    let widthTiles = _options.tile_width_tiles ?? null;
    if (!widthTiles) {
      widthTiles =
        this.side === 'enemy'
          ? TrainerExitAnimation.ENEMY_WIDTH_TILES
          : TrainerExitAnimation.PLAYER_WIDTH_TILES;
    }
    if (widthTiles <= 0) {
      throw new Error('Trainer exit animation requires a positive tile width.');
    }
    this.width_tiles = widthTiles;
    this.step_pixels =
      this.side === 'enemy' ? TrainerExitAnimation.TILE_SIZE : -TrainerExitAnimation.TILE_SIZE;
    this.steps_remaining = widthTiles;
  }

  get is_finished(): boolean {
    return this.finished;
  }

  draw(_surface: Surface, _oam_manager?: BattleSpriteOAMManager | null): void {
    if (this.finished) {
      return;
    }

    // ASM mapping: engine/battle/core.asm::SlideBattlePicOut
    // Each shift step is followed by `DelayFrames 2`, including the final step.
    if (this.frame_cooldown > 0) {
      this.frame_cooldown -= 1;
      if (this.frame_cooldown === 0 && this.steps_remaining <= 0) {
        this.finished = true;
      }
      return;
    }

    if (this.steps_remaining <= 0) {
      this.finished = true;
      return;
    }

    this.offset_x += this.step_pixels;
    this.steps_remaining -= 1;
    this.frame_cooldown = TrainerExitAnimation.STEP_DELAY_FRAMES;
  }

  get x_offset(): number {
    return this.offset_x;
  }

  get target_side(): 'player' | 'enemy' {
    return this.side;
  }
}

export class TrainerExitAnimationPair {
  private finished = false;

  constructor(private readonly player: TrainerExitAnimation, private readonly enemy: TrainerExitAnimation) {}

  get is_finished(): boolean {
    return this.finished;
  }

  draw(surface: Surface, oam_manager?: BattleSpriteOAMManager | null): void {
    if (this.finished) {
      return;
    }
    this.player.draw(surface, oam_manager);
    this.enemy.draw(surface, oam_manager);
    if (this.player.is_finished && this.enemy.is_finished) {
      this.finished = true;
    }
  }

  get player_offset_x(): number {
    return this.player.x_offset;
  }

  get enemy_offset_x(): number {
    return this.enemy.x_offset;
  }

  get target_side(): 'both' {
    return 'both';
  }
}

export const build_trainer_exit_animation = (
  ui: BattleUI,
  hardware: BattleHardwareRegisters,
  options?: { side?: string }
): TrainerExitAnimation | TrainerExitAnimationPair => {
  const side = (options?.side ?? 'player').trim().toLowerCase();
  if (side === 'both') {
    return new TrainerExitAnimationPair(
      new TrainerExitAnimation(ui, { hardware, side: 'player' }),
      new TrainerExitAnimation(ui, { hardware, side: 'enemy' })
    );
  }
  return new TrainerExitAnimation(ui, { hardware, side });
};

export { TOTAL_FRAMES };
