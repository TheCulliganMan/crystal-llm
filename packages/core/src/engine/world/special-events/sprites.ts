import { GameState } from "@pokecrystal/core/core/state";
import { PlayerGender } from "@pokecrystal/core/core/enums";
import { PlayerState } from "@pokecrystal/core/core/enums/overworld";
import { ScriptRunner } from "./utils";
import { SpriteAnimation } from "@pokecrystal/core/engine/systems/animation";
import type { EventManager } from "@pokecrystal/core/engine/events/events";
import type { MapAttributes } from "@pokecrystal/core/core/models/map";

const PlayerState_VALUES = new Set(Object.values(PlayerState));

type Overworld = {
  player_state?: PlayerState | number | string;
  player_sprite_id?: string;
  player_palette_id?: number;
  player_animations?: Record<string, SpriteAnimation> | null;
  _create_player_animations?: () => Record<string, SpriteAnimation>;
  _preserve_animation_frames?: (
    existing: SpriteAnimation | null | undefined,
    next: SpriteAnimation,
    options: { reload_standing: boolean; reload_walking: boolean }
  ) => SpriteAnimation | null | undefined;
  refresh_map_sprites?: (options?: {
    reload_standing?: boolean;
    reload_walking?: boolean;
  }) => void;
  reload_sprites_without_palette_changes?: (options?: {
    reload_standing?: boolean;
    reload_walking?: boolean;
  }) => void;
  _load_misc_sprite_assets?: () => void;
  _refresh_tileset_for_current_map?: (attributes?: MapAttributes | null) => void;
};

const SPRITES_SKIP_WALKING_GFX_F = 6;
const SPRITES_SKIP_STANDING_GFX_F = 7;
const SPRITES_SKIP_WALKING = 1 << SPRITES_SKIP_WALKING_GFX_F;
const SPRITES_SKIP_STANDING = 1 << SPRITES_SKIP_STANDING_GFX_F;

const SURFING_STATES: readonly PlayerState[] = [
  PlayerState.SURF,
  PlayerState.SURF_PIKA,
];

const PLAYER_STATE_FLAGS: Record<PlayerState, number> = {
  [PlayerState.NORMAL]: 0,
  [PlayerState.BIKE]: 1,
  [PlayerState.SKATE]: 2,
  [PlayerState.SURF]: 4,
  [PlayerState.SURF_PIKA]: 8,
};

const FLAG_TO_PLAYER_STATE: Record<number, PlayerState> = Object.fromEntries(
  Object.entries(PLAYER_STATE_FLAGS).map(([state, flag]) => [
    flag,
    state as PlayerState,
  ])
) as Record<number, PlayerState>;

const PLAYER_STATE_SPRITES: Record<PlayerGender, Array<[number, string]>> = {
  [PlayerGender.MALE]: [
    [PLAYER_STATE_FLAGS[PlayerState.NORMAL], "chris"],
    [PLAYER_STATE_FLAGS[PlayerState.BIKE], "chris_bike"],
    [PLAYER_STATE_FLAGS[PlayerState.SURF], "surf"],
    [PLAYER_STATE_FLAGS[PlayerState.SURF_PIKA], "surfing_pikachu"],
  ],
  [PlayerGender.FEMALE]: [
    [PLAYER_STATE_FLAGS[PlayerState.NORMAL], "kris"],
    [PLAYER_STATE_FLAGS[PlayerState.BIKE], "kris_bike"],
    [PLAYER_STATE_FLAGS[PlayerState.SURF], "surf"],
    [PLAYER_STATE_FLAGS[PlayerState.SURF_PIKA], "surfing_pikachu"],
  ],
};

type PlayerGenderInput = PlayerGender | string | number | null | undefined;

const normalizeGender = (value: PlayerGenderInput): PlayerGender => {
  if (value === PlayerGender.MALE || value === PlayerGender.FEMALE) {
    return value;
  }
  if (typeof value === "string") {
    const normalized = value.trim().toLowerCase();
    if (normalized === "male" || normalized === "0") {
      return PlayerGender.MALE;
    }
    if (normalized === "female" || normalized === "1") {
      return PlayerGender.FEMALE;
    }
  }
  if (typeof value === "number") {
    return value === PlayerGender.FEMALE ? PlayerGender.FEMALE : PlayerGender.MALE;
  }
  return PlayerGender.MALE;
};

const playerStateFromFlag = (flag: number | null | undefined): PlayerState => {
  if (flag === null || flag === undefined) {
    return PlayerState.NORMAL;
  }
  return FLAG_TO_PLAYER_STATE[flag] ?? PlayerState.NORMAL;
};

const flagForPlayerState = (state: PlayerState): number => {
  return PLAYER_STATE_FLAGS[state] ?? 0;
};

const spriteForStateFlag = (gender: PlayerGender, flag: number): string => {
  const table = PLAYER_STATE_SPRITES[gender] ?? PLAYER_STATE_SPRITES[PlayerGender.MALE];
  for (const [entryFlag, spriteId] of table) {
    if (entryFlag === flag) {
      return spriteId;
    }
  }
  return table[0]?.[1] ?? "chris";
};

type PlayerStateInput = PlayerState | string | number | null | undefined;

const normalizePlayerState = (value: PlayerStateInput): PlayerState => {
  if (PlayerState_VALUES.has(value as PlayerState)) {
    return value as PlayerState;
  }
  if (typeof value === "number") {
    return playerStateFromFlag(value);
  }
  if (typeof value === "string") {
    const token = value.trim().toLowerCase();
    for (const candidate of Object.values(PlayerState)) {
      if (token === candidate) {
        return candidate;
      }
    }
  }
  return PlayerState.NORMAL;
};

const resolvePlayerSpriteId = (
  game_state: GameState,
  overworld?: Overworld | null
): { spriteId: string; state: PlayerState; gender: PlayerGender } => {
  if (!overworld) {
    throw new Error("Updating the player sprite requires an overworld instance.");
  }
  let stateFlag = game_state.wram.wPlayerState ?? 0;
  let state = playerStateFromFlag(stateFlag);
  const gender = normalizeGender(game_state.wram.player_gender);
  const candidateState = normalizePlayerState(overworld.player_state ?? state);
  if ((!stateFlag || stateFlag === 0) && candidateState !== PlayerState.NORMAL) {
    state = candidateState;
    stateFlag = flagForPlayerState(candidateState);
  }
  const spriteId = spriteForStateFlag(gender, stateFlag ?? 0);
  return { spriteId, state, gender };
};

export function load_map_palettes(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld | null; event_manager?: EventManager } = {}
): boolean {
  // ASM: engine/overworld/warp_connection.asm::LoadMapPalettes
  void game_state;
  void runner;
  void event_manager;

  if (!overworld) {
    throw new Error("LoadMapPalettes requires an active overworld.");
  }
  const refresh = overworld._refresh_tileset_for_current_map;
  if (typeof refresh !== "function") {
    throw new Error(
      "LoadMapPalettes expects an overworld with '_refresh_tileset_for_current_map'."
    );
  }
  refresh();
  return true;
}

export function update_player_sprite(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
    reload_standing = true,
    reload_walking = true,
  }: {
    runner?: ScriptRunner;
    overworld?: Overworld | null;
    event_manager?: EventManager;
    reload_standing?: boolean;
    reload_walking?: boolean;
  } = {}
): string {
  // ASM: engine/overworld/player_object.asm::UpdatePlayerSprite
  void event_manager;
  const { spriteId, state } = resolvePlayerSpriteId(game_state, overworld);

  game_state.wram.surfing = SURFING_STATES.includes(state);
  if (!overworld) {
    throw new Error("UpdatePlayerSprite requires an active overworld.");
  }

  overworld.player_sprite_id = spriteId;
  const animationFactory = overworld._create_player_animations;
  if (typeof animationFactory !== "function") {
    throw new Error(
      "UpdatePlayerSprite expects an overworld with '_create_player_animations'."
    );
  }
  const previous = overworld.player_animations ?? null;
  let animations: Record<string, SpriteAnimation> = animationFactory.call(overworld);
  if (!reload_standing || !reload_walking) {
    const merge = overworld._preserve_animation_frames;
    if (typeof merge === "function" && animations && typeof animations === "object") {
      const merged: Record<string, SpriteAnimation> = {};
      for (const [direction, animation] of Object.entries(animations)) {
        const existing = previous?.[direction] ?? null;
        const mergedAnimation = merge(existing, animation, {
          reload_standing,
          reload_walking,
        });
        merged[direction] = mergedAnimation ?? animation;
      }
      animations = merged;
    }
  }
  overworld.player_animations = animations;

  if (runner) {
    runner.last_condition_result = true;
    runner.last_value = spriteId;
  }
  return spriteId;
}

export function refresh_sprites(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld | null; event_manager?: EventManager } = {}
): boolean {
  // ASM: engine/overworld/map_objects.asm::RefreshSprites
  void event_manager;
  const flags = game_state.wram.wSpriteFlags ?? 0;
  const reloadStanding = (flags & SPRITES_SKIP_STANDING) === 0;
  const reloadWalking = (flags & SPRITES_SKIP_WALKING) === 0;

  update_player_sprite(game_state, {
    runner,
    overworld,
    reload_standing: reloadStanding,
    reload_walking: reloadWalking,
  });

  if (!overworld) {
    throw new Error("RefreshSprites requires an active overworld.");
  }
  const refreshMethod = overworld.refresh_map_sprites;
  if (typeof refreshMethod === "function") {
    try {
      refreshMethod.call(overworld, {
        reload_standing: reloadStanding,
        reload_walking: reloadWalking,
      });
    } catch (error) {
      const message = (error as { message?: unknown } | null)?.message;
      if (!String(message ?? "").includes("unexpected")) {
        throw error;
      }
      refreshMethod.call(overworld);
    }
  } else {
    throw new Error("RefreshSprites expects an overworld with 'refresh_map_sprites'.");
  }
  return true;
}

export function update_sprites(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld | null; event_manager?: EventManager } = {}
): boolean {
  // ASM: engine/overworld/map_objects.asm::UpdateSprites
  void event_manager;
  const flags = game_state.wram.wSpriteFlags ?? 0;
  const reloadStanding = (flags & SPRITES_SKIP_STANDING) === 0;
  const reloadWalking = (flags & SPRITES_SKIP_WALKING) === 0;

  update_player_sprite(game_state, {
    runner,
    overworld,
    reload_standing: reloadStanding,
    reload_walking: reloadWalking,
  });

  if (!overworld) {
    throw new Error("UpdateSprites requires an active overworld.");
  }
  const reloadMethod = overworld.reload_sprites_without_palette_changes;
  if (typeof reloadMethod !== "function") {
    throw new Error(
      "UpdateSprites expects an overworld with 'reload_sprites_without_palette_changes'."
    );
  }
  try {
    reloadMethod.call(overworld, {
      reload_standing: reloadStanding,
      reload_walking: reloadWalking,
    });
  } catch (error) {
    const message = (error as { message?: unknown } | null)?.message;
    if (!String(message ?? "").includes("unexpected")) {
      throw error;
    }
    reloadMethod.call(overworld);
  }
  if (runner) {
    runner.last_condition_result = true;
  }
  return true;
}

const withSpriteFlagOverride = (
  game_state: GameState,
  {
    skip_standing,
    skip_walking,
  }: { skip_standing?: boolean; skip_walking?: boolean },
  fn: () => boolean
): boolean => {
  const original = game_state.wram.wSpriteFlags ?? 0;
  let flags = original;
  if (skip_standing !== undefined) {
    flags = skip_standing ? flags | SPRITES_SKIP_STANDING : flags & ~SPRITES_SKIP_STANDING;
  }
  if (skip_walking !== undefined) {
    flags = skip_walking ? flags | SPRITES_SKIP_WALKING : flags & ~SPRITES_SKIP_WALKING;
  }
  game_state.wram.wSpriteFlags = flags;
  try {
    return fn();
  } finally {
    game_state.wram.wSpriteFlags = original;
  }
};

export function load_standing_sprites_gfx(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld | null; event_manager?: EventManager } = {}
): boolean {
  // ASM: engine/overworld/map_objects.asm::LoadStandingSpritesGFX
  return withSpriteFlagOverride(
    game_state,
    { skip_standing: false, skip_walking: true },
    () =>
      load_used_sprites_gfx(game_state, {
        runner,
        overworld,
        event_manager,
      })
  );
}

export function load_walking_sprites_gfx(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld | null; event_manager?: EventManager } = {}
): boolean {
  // ASM: engine/overworld/map_objects.asm::LoadWalkingSpritesGFX
  return withSpriteFlagOverride(
    game_state,
    { skip_standing: true, skip_walking: false },
    () =>
      load_used_sprites_gfx(game_state, {
        runner,
        overworld,
        event_manager,
      })
  );
}

export function load_used_sprites_gfx(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld | null; event_manager?: EventManager } = {}
): boolean {
  // ASM: engine/overworld/map_objects.asm::LoadUsedSpritesGFX
  const result = refresh_sprites(game_state, { runner, overworld, event_manager });
  if (overworld?._load_misc_sprite_assets) {
    overworld._load_misc_sprite_assets();
  }
  return result;
}

export function check_mobile_adapter_status_special(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld | null; event_manager?: EventManager } = {}
): number {
  // ASM: engine/events/mobile/mobile.asm::CheckMobileAdapterStatusSpecial
  void game_state;
  void overworld;
  void event_manager;

  if (runner) {
    runner.last_condition_result = true;
    runner.last_value = 0;
    if (!runner.variables) {
      runner.variables = {};
    }
    runner.variables._value = 0;
  }
  return 0;
}
