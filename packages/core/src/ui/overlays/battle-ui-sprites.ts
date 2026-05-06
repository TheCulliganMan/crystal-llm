// ASM mapping: engine/battle/* (battle sprite OAM layout + palette handling).
import fs from 'fs';
import { Pokemon } from '../../core/models';
import { PlayerGender, StatusCondition } from '../../core/enums';
import { BattleContext, BattleStateEnum } from '../../engine/battle/battle/battle-context';
import { BattleMenu } from './_battle-menu';
import { BattleUIRoot, BattleUIState } from './battle-ui-state';
import { isCrySound } from './battle-animation-state';
import * as battle_dialogue from './battle-dialogue';
import { gameEngine } from '../game-engine';
import { Rect, Surface } from '../surface';
import { getAssetPath } from '../../core/paths';
import { gbc5To8, gbcWordToRgb } from '../../core/gbc-colors';
import { isShinyDvs } from '../../core/pokemon-dvs';
import {
  _BATTLE_PALETTES,
  dmg_palette_from_register,
  retint_tileset_palette,
} from './_battle-background';
import { is_dmg_baseline_palette_state } from './_battle-palettes';
import { BattlerSide, BattleAnimationRuntime } from './battle-bg-effects';
import { invalidate_scene_cache } from './battle-scene';
import { load_player_backpic_surface } from '../player-backpics';
import {
  FrontpicAnimator,
  ensure_frontpic_anim_program,
  is_frontpic_anim_program_pending,
  resolve_frontpic_anim_program,
} from './pokemon-frontpic-animation';
import { ensure_image_preload } from '../deferred-assets';

const OAM_X_OFFSET = 8;
const OAM_Y_OFFSET = 16;
const BALL_ICON_TILE_SIZE = 8;
const MAX_PARTY_ICONS = 6;
// ASM mapping: engine/battle/trainer_huds.asm::LoadBallIconGFX + StageBallTilesData.
const BALL_ICON_PATH = getAssetPath('gfx', 'battle', 'balls.png');

const BALL_ICON_NORMAL = 0;
const BALL_ICON_STATUS = 1;
const BALL_ICON_FAINTED = 2;
const BALL_ICON_EMPTY = 3;

let BALL_ICON_CACHE: Surface[] | null = null;
let BATTLE_OBJECT_PALS: Array<Array<[number, number, number]>> | null = null;
const POKEMON_BATTLE_PALETTE_CACHE = new Map<string, Array<[number, number, number]>>();

type GameEngineSurface = InstanceType<typeof gameEngine.Surface>;

export type SpriteFramePokemon = {
  species: {
    id: string;
  };
};

const convertEngineSurfaceToUi = (source: GameEngineSurface): Surface => {
  return Surface.fromImageData(source.getImageData());
};

const should_draw_trainer_icons = (state: BattleUIState): boolean => {
  if (state.trainer_intro && !state.trainer_intro.is_finished) {
    return true;
  }
  if (state.trainer_exit && !state.trainer_exit.is_finished) {
    return true;
  }
  const wram = state.game_state?.wram as { wBattleHasJustStarted?: number } | undefined;
  return Boolean(
    state.trainer_hud_visible &&
      wram?.wBattleHasJustStarted &&
      !state.trainer_send_out_seen
  );
};

export const reset_battle_ui_sprite_caches = (): void => {
  BALL_ICON_CACHE = null;
  BATTLE_OBJECT_PALS = null;
  POKEMON_BATTLE_PALETTE_CACHE.clear();
};

const read_gbcpal_palette = (path: string): Array<[number, number, number]> => {
  if (!fs.existsSync(path)) {
    throw new Error(`Missing Pokemon battle palette: ${path}`);
  }
  const data = fs.readFileSync(path);
  if (data.length < 8) {
    throw new Error(`Pokemon battle palette ${path} must contain at least 4 colors.`);
  }
  const palette: Array<[number, number, number]> = [];
  for (let idx = 0; idx < 4; idx += 1) {
    palette.push(gbcWordToRgb(data.readUInt16LE(idx * 2)));
  }
  return palette;
};

const read_shiny_palette = (path: string): Array<[number, number, number]> => {
  if (!fs.existsSync(path)) {
    throw new Error(`Missing Pokemon shiny battle palette: ${path}`);
  }
  const entries: Array<[number, number, number]> = [];
  for (const raw of fs.readFileSync(path, 'utf-8').split(/\r?\n/)) {
    const line = raw.split(';')[0].trim();
    if (!line || !line.toUpperCase().startsWith('RGB')) {
      continue;
    }
    const parts = line.replace(/RGB/gi, '').replace(/,/g, ' ').split(/\s+/).filter(Boolean);
    if (parts.length < 3) {
      continue;
    }
    const values = parts.slice(0, 3).map((part) => Number(part));
    if (values.some((value) => Number.isNaN(value))) {
      continue;
    }
    entries.push([
      gbc5To8(values[0], 'shiny palette r'),
      gbc5To8(values[1], 'shiny palette g'),
      gbc5To8(values[2], 'shiny palette b'),
    ]);
  }
  if (entries.length !== 2) {
    throw new Error(`Pokemon shiny battle palette ${path} must contain exactly 2 colors.`);
  }
  return [[255, 255, 255], entries[0], entries[1], [0, 0, 0]];
};

const pokemon_battle_palette = (
  speciesId: string,
  paletteType: 'normal' | 'shiny',
): Array<[number, number, number]> => {
  const normalized = speciesId.trim().toLowerCase();
  const key = `${normalized}:${paletteType}`;
  const cached = POKEMON_BATTLE_PALETTE_CACHE.get(key);
  if (cached) {
    return cached;
  }
  const sourceDir = getAssetPath('gfx', 'pokemon', normalized);
  const palette = paletteType === 'normal'
    ? read_gbcpal_palette(`${sourceDir}/normal.gbcpal`)
    : read_shiny_palette(`${sourceDir}/shiny.pal`);
  POKEMON_BATTLE_PALETTE_CACHE.set(key, palette);
  return palette;
};

const recolor_pokemon_surface_for_battle = (
  surface: Surface,
  speciesId: string,
  pokemon: Pokemon,
): Surface => {
  if (!isShinyDvs(pokemon.dvs)) {
    return surface;
  }
  const normalPalette = pokemon_battle_palette(speciesId, 'normal');
  const shinyPalette = pokemon_battle_palette(speciesId, 'shiny');
  const colourMap = new Map<string, [number, number, number]>();
  normalPalette.forEach((colour, idx) => {
    colourMap.set(colour.join(','), shinyPalette[idx] ?? colour);
  });

  const [width, height] = surface.get_size();
  const recolored = new Surface(width, height);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const [r, g, b, a] = surface.get_at([x, y]);
      if (a === 0) {
        recolored.set_at([x, y], [0, 0, 0, 0]);
        continue;
      }
      const mapped = colourMap.get(`${r},${g},${b}`);
      if (mapped) {
        recolored.set_at([x, y], [mapped[0], mapped[1], mapped[2], a]);
      } else {
        recolored.set_at([x, y], [r, g, b, a]);
      }
    }
  }
  return recolored;
};

const load_ball_icons = (): Surface[] | null => {
  if (BALL_ICON_CACHE) {
    return BALL_ICON_CACHE;
  }
  const canCheckFs = typeof window === 'undefined';
  if (canCheckFs && !fs.existsSync(BALL_ICON_PATH)) {
    throw new Error(`Missing battle ball icons: ${BALL_ICON_PATH}`);
  }
  const loadSync = typeof gameEngine.image.loadSync === 'function' ? gameEngine.image.loadSync : null;
  const engineSheet = loadSync ? loadSync(BALL_ICON_PATH) : null;
  if (!engineSheet) {
    if (ensure_image_preload(BALL_ICON_PATH)) {
      return null;
    }
    throw new Error(
      `Battle ball icons must be preloaded before HUD rendering: ${BALL_ICON_PATH}`
    );
  }
  const sheet = convertEngineSurfaceToUi(engineSheet);
  const [sheetWidth, sheetHeight] = sheet.get_size();
  if (sheetHeight < BALL_ICON_TILE_SIZE || sheetWidth < BALL_ICON_TILE_SIZE * 4) {
    throw new Error(
      `Battle ball icon sheet is missing tile 3 at ${BALL_ICON_PATH}`
    );
  }
  const icons: Surface[] = [];
  for (let idx = 0; idx < 4; idx += 1) {
    const rect = new Rect(
      idx * BALL_ICON_TILE_SIZE,
      0,
      BALL_ICON_TILE_SIZE,
      BALL_ICON_TILE_SIZE
    );
    try {
      const icon = sheet
        .subsurface(rect)
        .copy();
      icons.push(icon);
    } catch {
      throw new Error(
        `Battle ball icon sheet is missing tile ${idx} at ${BALL_ICON_PATH}`
      );
    }
  }
  BALL_ICON_CACHE = icons;
  return BALL_ICON_CACHE;
};

const tint_surface_to_palette = (
  surface: Surface,
  palette: Array<[number, number, number]>
): Surface => {
  const valueToLevel: Record<number, number> = { 255: 0, 170: 1, 85: 2, 0: 3 };
  const [width, height] = surface.get_size();
  const tinted = new Surface(width, height);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const [r, g, b, a] = surface.get_at([x, y]);
      if (a === 0) {
        tinted.set_at([x, y], [0, 0, 0, 0]);
        continue;
      }
      const value = Math.floor((r + g + b) / 3);
      const level = valueToLevel[value] ?? Math.min(3, Math.max(0, Math.floor(value / 64)));
      const paletteIndex = Math.min(Math.max(level, 0), palette.length - 1);
      const colour = palette[paletteIndex];
      const alpha = paletteIndex === 0 ? 0 : a;
      tinted.set_at([x, y], [colour[0], colour[1], colour[2], alpha]);
    }
  }
  return tinted;
};

const ball_icon_for_pokemon = (pokemon: Pokemon | null | undefined): Surface => {
  const icons = load_ball_icons();
  if (!icons) {
    return new Surface(BALL_ICON_TILE_SIZE, BALL_ICON_TILE_SIZE);
  }
  if (!pokemon) {
    return icons[BALL_ICON_EMPTY];
  }
  if (pokemon.hp <= 0) {
    return icons[BALL_ICON_FAINTED];
  }
  const status = pokemon.status;
  let hasStatus = false;
  if (status !== null && status !== undefined) {
    if (typeof status === 'number') {
      hasStatus = status !== 0;
    } else if (typeof status === 'string') {
      hasStatus = status.toUpperCase() !== 'OK';
    } else {
      hasStatus = true;
    }
  }
  if (hasStatus) {
    return icons[BALL_ICON_STATUS];
  }
  return icons[BALL_ICON_NORMAL];
};

const load_battle_object_palettes = (): Array<Array<[number, number, number]>> => {
  if (BATTLE_OBJECT_PALS) {
    return BATTLE_OBJECT_PALS;
  }
  const palettePath = getAssetPath('gfx', 'battle_anims', 'battle_anims.pal');
  if (!fs.existsSync(palettePath)) {
    throw new Error(`Battle object palettes are required for the asset-only runtime: ${palettePath}`);
  }
  const palettes: Array<Array<[number, number, number]>> = [];
  const entries: Array<[number, number, number]> = [];
  const lines = fs.readFileSync(palettePath, 'utf-8').split(/\r?\n/);
  for (const raw of lines) {
    const line = raw.split(';')[0].trim();
    if (!line || !line.toUpperCase().startsWith('RGB')) {
      continue;
    }
    const parts = line.replace(/RGB/gi, '').replace(/,/g, ' ').split(/\s+/).filter(Boolean);
    if (parts.length < 3) {
      continue;
    }
    const values = parts.slice(0, 3).map((part) => Number(part));
    if (values.some((value) => Number.isNaN(value))) {
      continue;
    }
    entries.push([
      gbc5To8(values[0]),
      gbc5To8(values[1]),
      gbc5To8(values[2]),
    ]);
  }
  for (let idx = 0; idx < entries.length; idx += 4) {
    const group = entries.slice(idx, idx + 4);
    if (group.length === 4) {
      palettes.push(group);
    }
  }
  if (palettes.length < 6) {
    throw new Error(
      `Battle object palette source is incomplete: ${palettePath} yielded ${palettes.length} palettes`
    );
  }
  BATTLE_OBJECT_PALS = palettes.slice(0, 6);
  return BATTLE_OBJECT_PALS;
};

export const draw_trainer_hud_icons = (state: BattleUIState, battle_context: BattleContext): void => {
  if (!should_draw_trainer_icons(state)) {
    return;
  }
  if (!load_ball_icons()) {
    return;
  }
  const tileSize = state.ui.tile_size ?? BALL_ICON_TILE_SIZE;
  const palettes = load_battle_object_palettes();
  const basePalette: Array<[number, number, number]> =
    palettes.length >= 2
      ? palettes[1]
      : ([
          [255, 255, 255],
          [170, 170, 170],
          [85, 85, 85],
          [0, 0, 0],
        ] as Array<[number, number, number]>);
  const startPositions: Array<[number, number, number]> = [
    [12 * tileSize - OAM_X_OFFSET, 12 * tileSize - OAM_Y_OFFSET, tileSize],
  ];
  const playerParty = battle_context.playerParty ?? [];
  const enemyParty = battle_context.enemyParty ?? [];
  if (battle_context.trainerBattle) {
    startPositions.push([9 * tileSize - OAM_X_OFFSET, 4 * tileSize - OAM_Y_OFFSET, -tileSize]);
  }
  const parties = startPositions.length === 2 ? [playerParty, enemyParty] : [playerParty];
  let drewIcon = false;
  startPositions.forEach(([startX, startY, step], index) => {
    const party = parties[index] ?? [];
    for (let idx = 0; idx < MAX_PARTY_ICONS; idx += 1) {
      const pokemon = idx < party.length ? party[idx] : null;
      const icon = ball_icon_for_pokemon(pokemon);
      const tinted = tint_surface_to_palette(icon, basePalette);
      const x = startX + step * idx;
      state.oam_manager.blitSprite(state.ui.screen, tinted, x, startY);
      drewIcon = true;
    }
  });
  const animationPlayer = state.animation_player;
  if (drewIcon && (!animationPlayer || !animationPlayer.oam_enabled)) {
    state.oam_manager.flush(state.ui.screen);
  }
};

const resolve_sprite_surface = (
  state: BattleUIState,
  speciesId: string,
  spriteType: string,
  frame: number
): Surface | null => {
  const ui: BattleUIRoot = state.ui;
  if (spriteType === 'pokemon_front' && typeof ui._get_pokemon_frame_surface === 'function') {
    return ui._get_pokemon_frame_surface(speciesId, frame) as Surface | null;
  }
  if (spriteType === 'player_back') {
    const wram = (state.game_state?.wram ?? {}) as { player_gender?: PlayerGender };
    return load_player_backpic_surface(speciesId, { player_gender: wram.player_gender ?? null });
  }
  if (typeof ui.get_sprite_surface === 'function') {
    return ui.get_sprite_surface(speciesId, spriteType, frame);
  }
  if (typeof ui.getSpriteSurface === 'function') {
    return ui.getSpriteSurface(speciesId, { sprite_type: spriteType, frame }) as Surface | null;
  }
  return null;
};

const preload_battle_sprite = (
  state: BattleUIState,
  speciesId: string,
  spriteType: string
): void => {
  const loader = state.ui as {
    loadSprite?: (spriteId: string, spriteType?: string) => void;
    load_sprite?: (spriteId: string, spriteType?: string) => void;
  };
  const load = loader.loadSprite ?? loader.load_sprite;
  if (!load) {
    return;
  }
  const key = `${speciesId}:${spriteType}`;
  if (state._loaded_battle_sprites.has(key)) {
    if (spriteType === 'pokemon_front') {
      ensure_frontpic_anim_program(speciesId);
    }
    return;
  }
  load.call(loader, speciesId, spriteType);
  state._loaded_battle_sprites.add(key);
  if (spriteType === 'pokemon_front') {
    ensure_frontpic_anim_program(speciesId);
  }
};

const battle_sprite_paths = (speciesId: string, spriteType: string): string[] => {
  const normalized = speciesId.trim().toLowerCase();
  if (!normalized) {
    return [];
  }
  if (spriteType === 'pokemon_front') {
    return [getAssetPath('gfx', 'pokemon', normalized, 'front.png')];
  }
  if (spriteType === 'pokemon_back') {
    return [getAssetPath('gfx', 'pokemon', normalized, 'back.png')];
  }
  if (spriteType === 'trainer') {
    return [getAssetPath('gfx', 'trainers', `${normalized}.png`)];
  }
  if (spriteType === 'player_back') {
    return [
      getAssetPath('gfx', 'player', `${normalized}.png`),
      getAssetPath('gfx', 'battle', `${normalized}.png`),
    ];
  }
  if (spriteType === 'sprite' || spriteType === 'sprites') {
    return [getAssetPath('gfx', 'sprites', `${normalized}.png`)];
  }
  return [];
};

const should_defer_battle_sprite = (speciesId: string, spriteType: string): boolean => {
  const paths = battle_sprite_paths(speciesId, spriteType);
  if (!paths.length) {
    return false;
  }
  return paths.some((path) => ensure_image_preload(path));
};

export const draw_battle_sprites = (
  state: BattleUIState,
  player: Pokemon,
  enemy: Pokemon,
  runtime: BattleAnimationRuntime
): void => {
  if (!state.sprites_enabled) {
    return;
  }
  const tileSize = state.ui.tile_size ?? 8;
  const scale = state.layout.sprite_scale;

  let enemySpeciesId = enemy?.species?.id;
  const enemyOverride = runtime.enemy_sprite_override;
  if (enemyOverride === 'transform') {
    enemySpeciesId = player?.species?.id;
  } else if (enemyOverride) {
    enemySpeciesId = enemyOverride;
  }

  let playerSpeciesId = player?.species?.id;
  const playerOverride = runtime.player_sprite_override;
  if (playerOverride === 'transform') {
    playerSpeciesId = enemy?.species?.id;
  } else if (playerOverride) {
    playerSpeciesId = playerOverride;
  }

  if (enemySpeciesId) {
    let [enemyX, enemyY] = state.layout.enemy_sprite.pixelPosition(tileSize, scale);
    enemyX += Number(runtime.enemy_offset_x ?? 0);
    enemyY += Number(runtime.enemy_offset_y ?? 0);
    if (runtime.enemy_visible !== false) {
      const spriteType =
        runtime.enemy_sprite_type_override ?? state.layout.enemy_sprite.sprite_type;
      preload_battle_sprite(state, String(enemySpeciesId), spriteType);
      let surface = resolve_sprite_surface(state, String(enemySpeciesId), spriteType, state.enemy_sprite_frame);
      if (!surface) {
        if (should_defer_battle_sprite(String(enemySpeciesId), spriteType)) {
          return;
        }
        throw new Error(
          `Missing battle sprite surface for enemy ${String(enemySpeciesId)} (${spriteType}) frame ${state.enemy_sprite_frame}.`
        );
      }
      if (spriteType === 'pokemon_front' || spriteType === 'pokemon_back') {
        surface = recolor_pokemon_surface_for_battle(surface, String(enemySpeciesId), enemy);
      }
      state.animation_player?.anim_data?.register_battler_surfaces?.({ enemySurface: surface });
      const rowMode = Number(runtime.enemy_row_mode ?? 0);
      const rowState = Number(runtime.enemy_row_state ?? 0);
      draw_battler_surface(state.ui.screen, surface, enemyX, enemyY, rowMode, rowState);
    }
  }

  if (playerSpeciesId) {
    let [playerX, playerY] = state.layout.player_sprite.pixelPosition(tileSize, scale);
    playerX += Number(runtime.player_offset_x ?? 0);
    playerY += Number(runtime.player_offset_y ?? 0);
    if (runtime.player_visible !== false) {
      const spriteType =
        runtime.player_sprite_type_override ?? state.layout.player_sprite.sprite_type;
      preload_battle_sprite(state, String(playerSpeciesId), spriteType);
      let surface = resolve_sprite_surface(state, String(playerSpeciesId), spriteType, state.player_sprite_frame);
      if (!surface) {
        if (should_defer_battle_sprite(String(playerSpeciesId), spriteType)) {
          return;
        }
        throw new Error(
          `Missing battle sprite surface for player ${String(playerSpeciesId)} (${spriteType}) frame ${state.player_sprite_frame}.`
        );
      }
      if (spriteType === 'pokemon_front' || spriteType === 'pokemon_back') {
        surface = recolor_pokemon_surface_for_battle(surface, String(playerSpeciesId), player);
      }
      state.animation_player?.anim_data?.register_battler_surfaces?.({ playerSurface: surface });
      const rowMode = Number(runtime.player_row_mode ?? 0);
      const rowState = Number(runtime.player_row_state ?? 0);
      draw_battler_surface(state.ui.screen, surface, playerX, playerY, rowMode, rowState);
    }
  }
};

export const update_battle_sprite_frames = (
  state: BattleUIState,
  player: SpriteFramePokemon,
  enemy: SpriteFramePokemon
): void => {
  const request = state.frontpic_animation ?? null;
  const enemyActive = request?.side === "enemy";
  const playerActive = request?.side === "player";
  const enemyResult = advance_battle_sprite_frame(state, enemy, state.layout.enemy_sprite.sprite_type, {
    animate: enemyActive,
    speed: request?.speed ?? 0,
    keyPrefix: "enemy",
  });
  state.enemy_sprite_frame = enemyResult.frame;
  if (enemyActive && enemyResult.complete) {
    state.frontpic_animation = null;
  }

  const playerResult = advance_battle_sprite_frame(state, player, state.layout.player_sprite.sprite_type, {
    animate: playerActive,
    speed: request?.speed ?? 0,
    keyPrefix: "player",
  });
  state.player_sprite_frame = playerResult.frame;
  if (playerActive && playerResult.complete) {
    state.frontpic_animation = null;
  }
};

export type SpriteFrameUI = {
  get_pokemon_frame_count?: BattleUIRoot['get_pokemon_frame_count'];
};

export type SpriteFrameState = Pick<
  BattleUIState,
  '_sprite_frame_counts' | '_sprite_frame_timers' | '_sprite_frame_indices' | '_frontpic_animators'
> & {
  ui: SpriteFrameUI;
};

export const advance_battle_sprite_frame = (
  state: SpriteFrameState,
  pokemon: SpriteFramePokemon,
  spriteType: string,
  options?: { animate?: boolean; speed?: number; keyPrefix?: string }
): { frame: number; complete: boolean } => {
  const animate = Boolean(options?.animate);
  const spriteTypeId = String(spriteType || "").toLowerCase();
  if (!animate || spriteTypeId !== "pokemon_front") {
    if (options?.keyPrefix) {
      const resetKey = `${options.keyPrefix}:${pokemon.species.id}:${spriteTypeId}:${options?.speed ?? 0}`;
      if (state._frontpic_animators) {
        delete state._frontpic_animators[resetKey];
      }
    }
    return { frame: 0, complete: false };
  }
  const speed = Math.max(0, Math.trunc(options?.speed ?? 0));
  const keyPrefix = options?.keyPrefix ?? "";
  const animKey = `${keyPrefix}:${pokemon.species.id}:${spriteTypeId}:${speed}`;
  const program = resolve_frontpic_anim_program(pokemon.species.id);
  if (!program) {
    if (is_frontpic_anim_program_pending(pokemon.species.id)) {
      return { frame: 0, complete: false };
    }
    if (state._frontpic_animators) {
      delete state._frontpic_animators[animKey];
    }
    return { frame: 0, complete: true };
  }
  if (!state._frontpic_animators) {
    state._frontpic_animators = {};
  }
  let animator = state._frontpic_animators[animKey];
  if (!animator) {
    animator = new FrontpicAnimator(program, speed);
    state._frontpic_animators[animKey] = animator;
  }

  const key = `${pokemon.species.id}:${spriteType}`;
  let frameCount = state._sprite_frame_counts[key];
  if (frameCount === undefined) {
    frameCount = resolve_sprite_frame_count(state.ui, pokemon.species.id, spriteType);
    state._sprite_frame_counts[key] = frameCount;
  }
  if (frameCount <= 0) {
    return { frame: 0, complete: true };
  }
  const result = animator.step();
  if (result.frame >= frameCount) {
    throw new Error(
      `Frontpic animation frame ${result.frame} exceeds ${frameCount} frames for ${pokemon.species.id}.`
    );
  }
  state._sprite_frame_indices[key] = result.frame;
  state._sprite_frame_timers[key] = animator.complete ? 0 : (state._sprite_frame_timers[key] ?? 0);
  return { frame: result.frame, complete: animator.complete };
};

export const resolve_sprite_frame_count = (
  ui: SpriteFrameUI,
  speciesId: string,
  spriteType: string
): number => {
  if (typeof ui.get_pokemon_frame_count !== 'function') {
    return 1;
  }
  return ui.get_pokemon_frame_count(speciesId, spriteType);
};

export const draw_animation_sprites = (state: BattleUIState): void => {
  if (!state.animation_player.oam_enabled) {
    return;
  }
  for (const sprite of state.animation_player.active_sprites) {
    const rendered = state.animation_player.render_sprite(sprite);
    if (!rendered) {
      continue;
    }
    const [spriteX, spriteY] = state.animation_player.resolve_sprite_position(sprite);
    const destX = spriteX + rendered.offset_x - OAM_X_OFFSET;
    const destY = spriteY + rendered.offset_y - OAM_Y_OFFSET;
    const engineSurface = rendered.surface;
    const surface = convertEngineSurfaceToUi(engineSurface);
    state.oam_manager.blitSprite(state.ui.screen, surface, destX, destY);
  }
  state.oam_manager.flush(state.ui.screen);
};

export const overlay_move_windows = (state: BattleUIState): void => {
  if (state.wram.current_menu !== BattleMenu.FIGHT) {
    return;
  }
  const context = state.context;
  if (!context || context.currentState !== BattleStateEnum.PLAYER_ACTION_SELECT) {
    return;
  }
  if (battle_dialogue.is_visible(state.dialogue)) {
    return;
  }
  for (const window of [state.layout.move_selection_window, state.layout.move_info_window]) {
    if (!window) {
      continue;
    }
    blit_window_tiles(state, window);
  }
};

export const blit_window_tiles = (state: BattleUIState, window: { tile_x: number; tile_y: number; width_tiles: number; height_tiles: number }): void => {
  const tileSize = state.ui.tile_size ?? 8;
  const tiles = state.tilemap.tiles;
  const attrs = state.tilemap.attributes;
  for (let row = 0; row < window.height_tiles; row += 1) {
    const tileY = window.tile_y + row;
    for (let col = 0; col < window.width_tiles; col += 1) {
      const tileX = window.tile_x + col;
      const tileId = tiles[tileY][tileX];
      const attr = attrs[tileY][tileX];
      const surface = resolve_tile_surface(state.tileset, tileId, attr);
      if (!surface) {
        continue;
      }
      const dest: [number, number] = [tileX * tileSize, tileY * tileSize];
      state.ui.screen.blit(surface, dest);
    }
  }
};

export const resolve_tile_surface = (
  tileset: Record<number, Surface | Record<number, Surface>>,
  tileId: number,
  attr: number
): Surface | null => {
  const entry = tileset[tileId];
  if (!entry) {
    return null;
  }
  if (entry instanceof Surface) {
    return entry;
  }
  if (typeof entry === 'object') {
    const paletteIndex = attr & 0x07;
    return entry[paletteIndex] ?? entry[0] ?? null;
  }
  return null;
};

export const dispatch_animation_audio = (state: BattleUIState): void => {
  const cues = state.animation_player.pending_audio();
  if (!cues || !state.audio_engine) {
    return;
  }
  for (const cue of cues) {
    let soundId = cue.sound_id;
    if (isCrySound(cue)) {
      const context = state.context;
      if (!context) {
        continue;
      }
      const battler = cue.panning === 'player' || cue.panning === 'left'
        ? context.playerPokemon
        : context.enemyPokemon;
      const speciesId = battler?.species?.id;
      if (!speciesId) {
        throw new Error('Cannot play cry without a species id.');
      }
      soundId = `CRY_${String(speciesId).toUpperCase()}`;
    }
    try {
      state.audio_engine.playSound(soundId, {
        duration: cue.duration ?? null,
        tracks: cue.tracks ?? null,
        panning: cue.panning ?? null,
        pitch: cue.pitch ?? null,
      });
    } catch {
      // Ignore missing sounds in headless environments.
    }
  }
};

const restore_default_palettes = (state: BattleUIState): void => {
  for (let idx = 0; idx < _BATTLE_PALETTES.length; idx += 1) {
    retint_tileset_palette(state.tileset, state.base_tiles, idx, _BATTLE_PALETTES[idx]);
  }
  state.palette_registers = {};
  invalidate_scene_cache();
};

export const apply_palette_registers = (state: BattleUIState): void => {
  const paletteState = state.animation_player.palette_state ?? {};
  if (!state.base_tiles) {
    return;
  }
  if (is_dmg_baseline_palette_state(paletteState)) {
    if (Object.keys(state.palette_registers).length) {
      restore_default_palettes(state);
    }
    return;
  }
  let changed = false;
  const requestedBgp = paletteState.bgp;
  if (requestedBgp !== null && requestedBgp !== undefined) {
    if (state.palette_registers.bgp !== requestedBgp) {
      const palette = dmg_palette_from_register(requestedBgp);
      if (palette) {
        for (let paletteIndex = 0; paletteIndex < _BATTLE_PALETTES.length; paletteIndex += 1) {
          retint_tileset_palette(state.tileset, state.base_tiles, paletteIndex, palette);
        }
        state.palette_registers.bgp = requestedBgp;
        changed = true;
      }
    }
  } else if ('bgp' in state.palette_registers) {
    for (let idx = 0; idx < _BATTLE_PALETTES.length; idx += 1) {
      retint_tileset_palette(state.tileset, state.base_tiles, idx, _BATTLE_PALETTES[idx]);
    }
    delete state.palette_registers.bgp;
    changed = true;
  }
  for (const register of ['obp0', 'obp1'] as const) {
    const requested = paletteState[register];
    if (requested === null || requested === undefined) {
      delete state.palette_registers[register];
    } else {
      state.palette_registers[register] = requested;
    }
  }
  if (changed) {
    invalidate_scene_cache();
  }
};

const apply_overlay = (
  surface: Surface,
  colour: [number, number, number],
  alpha: number
): void => {
  const clamped = Math.max(0, Math.min(255, Math.trunc(alpha)));
  const overlay = new Surface(surface.width, surface.height);
  overlay.fill([colour[0], colour[1], colour[2], clamped]);
  surface.blit(overlay, [0, 0]);
};

const build_masked_overlay = (
  source: Surface,
  colour: [number, number, number],
  alpha: number
): Surface => {
  const overlay = new Surface(source.width, source.height);
  const clamped = Math.max(0, Math.min(255, Math.trunc(alpha)));
  if (clamped <= 0) {
    return overlay;
  }
  for (let y = 0; y < source.height; y += 1) {
    for (let x = 0; x < source.width; x += 1) {
      const [, , , srcAlpha] = source.get_at([x, y]);
      if (srcAlpha <= 0) {
        continue;
      }
      const maskedAlpha = Math.max(0, Math.min(255, Math.round((clamped * srcAlpha) / 255)));
      overlay.set_at([x, y], [colour[0], colour[1], colour[2], maskedAlpha]);
    }
  }
  return overlay;
};

const overlay_battler = (
  state: BattleUIState,
  side: BattlerSide,
  colour: [number, number, number],
  alpha: number
): void => {
  const tileSize = state.ui.tile_size ?? 8;
  const scale = state.layout.sprite_scale;
  const runtime = state.animation_player.runtime_state;
  const context = state.context;
  if (!context) {
    return;
  }
  const data = side === BattlerSide.PLAYER
    ? {
        override: runtime.player_sprite_override,
        speciesId: context.playerPokemon?.species?.id ?? null,
        frame: state.player_sprite_frame,
        spriteDef: state.layout.player_sprite,
        offsetX: runtime.player_offset_x ?? 0,
        offsetY: runtime.player_offset_y ?? 0,
        spriteType: state.layout.player_sprite.sprite_type,
      }
    : {
        override: runtime.enemy_sprite_override,
        speciesId: context.enemyPokemon?.species?.id ?? null,
        frame: state.enemy_sprite_frame,
        spriteDef: state.layout.enemy_sprite,
        offsetX: runtime.enemy_offset_x ?? 0,
        offsetY: runtime.enemy_offset_y ?? 0,
        spriteType: state.layout.enemy_sprite.sprite_type,
      };
  let speciesId = data.speciesId;
  if (data.override === 'transform') {
    speciesId = side === BattlerSide.PLAYER ? context.enemyPokemon?.species?.id ?? null : context.playerPokemon?.species?.id ?? null;
  } else if (data.override) {
    speciesId = data.override;
  }
  if (!speciesId) {
    return;
  }
  const surface = resolve_sprite_surface(state, String(speciesId), data.spriteType, data.frame);
  if (!surface) {
    return;
  }
  let [x, y] = data.spriteDef.pixelPosition(tileSize, scale);
  x += data.offsetX;
  y += data.offsetY;
  const overlay = build_masked_overlay(surface, colour, alpha);
  state.ui.screen.blit(overlay, [x, y]);
};

export const apply_runtime_postprocessing = (
  state: BattleUIState,
  runtime: BattleAnimationRuntime
): void => {
  const paletteState = state.animation_player.palette_state ?? {};
  if (!state.tileset && paletteState.bgp !== null && paletteState.bgp !== undefined) {
    const palette = dmg_palette_from_register(paletteState.bgp);
    if (palette) {
      apply_overlay(state.ui.screen, palette[1], 255);
    }
  }
  const overlayColour = runtime.overlay_colour;
  const overlayAlpha = runtime.overlay_alpha ?? 0;
  const overlayTarget = runtime.overlay_target;
  if (overlayColour && overlayAlpha) {
    if (overlayTarget === null || overlayTarget === undefined) {
      apply_overlay(state.ui.screen, overlayColour, overlayAlpha);
    } else {
      overlay_battler(state, overlayTarget, overlayColour, overlayAlpha);
    }
  }
};

export const draw_battler_surface = (
  screen: Surface,
  surface: Surface,
  x: number,
  y: number,
  rowMode: number,
  rowState: number
): void => {
  if (rowMode === 0) {
    screen.blit(surface, [x, y]);
    return;
  }
  const width = surface.width;
  const height = surface.height;
  const visibleSize = Math.max(0, rowState * 8);
  const visibleWidth = Math.min(width, visibleSize);
  const visibleHeight = Math.min(height, visibleSize);
  if (visibleWidth <= 0 || visibleHeight <= 0) {
    return;
  }
  // ASM mapping: BattleBGEffect_ReturnMon / BattleBGEffect_EnterMon place
  // progressively smaller square tile blocks at shifted BG origins so that
  // the sprite collapses/expands toward its center, not the bottom-right.
  const clipX = Math.floor((width - visibleWidth) / 2);
  const clipY = Math.floor((height - visibleHeight) / 2);
  const rect = new Rect(clipX, clipY, visibleWidth, visibleHeight);
  screen.blit(surface, [x + clipX, y + clipY], rect);
};
