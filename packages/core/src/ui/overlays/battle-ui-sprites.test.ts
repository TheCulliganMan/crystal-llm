import type { BattleUIState } from './battle-ui-state';
import {
  advance_battle_sprite_frame,
  apply_runtime_postprocessing,
  draw_battle_sprites,
  draw_battler_surface,
  draw_trainer_hud_icons,
  reset_battle_ui_sprite_caches,
  type SpriteFramePokemon,
} from './battle-ui-sprites';
import type { SpriteFrameState } from './battle-ui-sprites';
import { Rect, Surface } from '../surface';
import { BattleUILayoutFactory } from './_battle-layout';
import type { BattleAnimationRuntime } from './battle-bg-effects';
import type { Pokemon } from '../../core/models';
import { parse_frontpic_anim_script, register_frontpic_anim_scripts } from './pokemon-frontpic-animation';
import * as frontpicAnimation from './pokemon-frontpic-animation';
import { gameEngine } from '../game-engine';
import { BattleSpriteOAMManager } from './battle-oam';
import fs from 'fs';
import { reset_deferred_image_preloads_for_test } from '../deferred-assets';

jest.mock('../player-backpics', () => ({
  load_player_backpic_surface: jest.fn(),
}));

const { load_player_backpic_surface } = jest.requireMock('../player-backpics') as {
  load_player_backpic_surface: jest.Mock;
};

const buildSpriteFrameState = (
  getPokemonFrameCount?: SpriteFrameState['ui']['get_pokemon_frame_count']
): SpriteFrameState => {
  const ui: SpriteFrameState['ui'] = {};
  if (getPokemonFrameCount) {
    ui.get_pokemon_frame_count = getPokemonFrameCount;
  }
  return {
    _sprite_frame_counts: {},
    _sprite_frame_timers: {},
    _sprite_frame_indices: {},
    _frontpic_animators: {},
    ui,
  };
};

const buildBallIconSheet = (): Surface => {
  const sheet = new Surface(32, 8);
  for (let idx = 0; idx < 4; idx += 1) {
    const tile = new Surface(8, 8);
    tile.fill([170, 170, 170, 255]);
    sheet.blit(tile, [idx * 8, 0]);
  }
  return sheet;
};

describe('battle-ui-sprites', () => {
  let originalLoadSync: typeof gameEngine.image.loadSync | undefined;
  let originalPreload: typeof gameEngine.image.preload | undefined;
  let originalExistsSync: typeof fs.existsSync;
  let originalReadFileSync: typeof fs.readFileSync;

  beforeEach(() => {
    originalLoadSync = gameEngine.image.loadSync;
    originalPreload = gameEngine.image.preload;
    originalExistsSync = fs.existsSync;
    originalReadFileSync = fs.readFileSync;
    reset_battle_ui_sprite_caches();
    reset_deferred_image_preloads_for_test();
  });

  afterEach(() => {
    gameEngine.image.loadSync = originalLoadSync;
    gameEngine.image.preload = originalPreload!;
    fs.existsSync = originalExistsSync;
    fs.readFileSync = originalReadFileSync;
    load_player_backpic_surface.mockReset();
    reset_battle_ui_sprite_caches();
  });

  it('advances battle sprite frames using asm frontpic scripts', () => {
    register_frontpic_anim_scripts({
      pikachu: parse_frontpic_anim_script(`
        frame 0, 01
        frame 1, 01
        endanim
      `),
    });
    const state = buildSpriteFrameState(() => 2);
    const pokemon: SpriteFramePokemon = { species: { id: 'pikachu' } };
    const frames: number[] = [];
    for (let i = 0; i < 4; i += 1) {
      const result = advance_battle_sprite_frame(state, pokemon, 'pokemon_front', {
        animate: true,
        speed: 0,
        keyPrefix: 'enemy',
      });
      frames.push(result.frame);
      expect(result.complete).toBe(false);
    }
    expect(frames).toEqual([0, 0, 1, 1]);
    const final = advance_battle_sprite_frame(state, pokemon, 'pokemon_front', {
      animate: true,
      speed: 0,
      keyPrefix: 'enemy',
    });
    expect(final.complete).toBe(true);
  });

  it('keeps a frontpic animation request alive while the browser anim script is still loading', () => {
    const pendingSpy = jest
      .spyOn(frontpicAnimation, 'is_frontpic_anim_program_pending')
      .mockReturnValue(true);
    const resolveSpy = jest
      .spyOn(frontpicAnimation, 'resolve_frontpic_anim_program')
      .mockReturnValue(null);
    const state = buildSpriteFrameState(() => 2);
    const pokemon: SpriteFramePokemon = { species: { id: 'pikachu' } };

    const result = advance_battle_sprite_frame(state, pokemon, 'pokemon_front', {
      animate: true,
      speed: 0,
      keyPrefix: 'enemy',
    });

    expect(result).toEqual({ frame: 0, complete: false });
    expect(resolveSpy).toHaveBeenCalledWith('pikachu');
    expect(pendingSpy).toHaveBeenCalledWith('pikachu');
  });

  it('clips battler sprites when row mode is active', () => {
    const screen = new Surface(32, 32);
    const sprite = new Surface(16, 16);
    const blitSpy = jest.spyOn(screen, 'blit');
    draw_battler_surface(screen, sprite, 0, 0, 1, 1);
    expect(blitSpy).toHaveBeenCalledTimes(1);
    const [, dest, rect] = blitSpy.mock.calls[0];
    expect(dest).toEqual([4, 4]);
    if (!(rect instanceof Rect)) {
      throw new Error("Expected sprite clip rect.");
    }
    expect(rect.x).toBe(4);
    expect(rect.y).toBe(4);
    expect(rect.width).toBe(8);
    expect(rect.height).toBe(8);
    blitSpy.mockRestore();
  });

  it('draws the full battler sprite unchanged when row mode is disabled', () => {
    const screen = new Surface(32, 32);
    const sprite = new Surface(16, 16);
    const blitSpy = jest.spyOn(screen, 'blit');

    draw_battler_surface(screen, sprite, 3, 4, 0, 0);

    expect(blitSpy).toHaveBeenCalledTimes(1);
    expect(blitSpy).toHaveBeenCalledWith(sprite, [3, 4]);
    blitSpy.mockRestore();
  });

  it('draws battle sprites and preloads assets once per species', () => {
    const screen = new Surface(160, 144);
    const enemySurface = new Surface(56, 56);
    const playerSurface = new Surface(56, 56);
    load_player_backpic_surface.mockReturnValue(playerSurface);
    const loadSprite = jest.fn();
    const ui = {
      tile_size: 8,
      screen,
      loadSprite,
      _get_pokemon_frame_surface: jest.fn(() => enemySurface),
      get_sprite_surface: jest.fn(() => playerSurface),
    };
    const state = {
      ui,
      layout: BattleUILayoutFactory.fromAsmDefaults(),
      sprites_enabled: true,
      enemy_sprite_frame: 0,
      player_sprite_frame: 0,
      _loaded_battle_sprites: new Set<string>(),
    } as unknown as BattleUIState;
    const runtime = {
      player_sprite_type_override: 'player_back',
    } as unknown as BattleAnimationRuntime;
    const player = { species: { id: 'pikachu' } } as Pokemon;
    const enemy = { species: { id: 'bulbasaur' } } as Pokemon;
    const blitSpy = jest.spyOn(screen, 'blit');

    draw_battle_sprites(state, player, enemy, runtime);
    draw_battle_sprites(state, player, enemy, runtime);

    expect(blitSpy).toHaveBeenCalledTimes(4);
    expect(loadSprite).toHaveBeenCalledTimes(2);
    expect(loadSprite).toHaveBeenCalledWith('bulbasaur', 'pokemon_front');
    expect(loadSprite).toHaveBeenCalledWith('pikachu', 'player_back');
    blitSpy.mockRestore();
  });

  it('recolors shiny Pokemon battle sprites with the ASM shiny palette', () => {
    const screen = new Surface(160, 144);
    const enemySurface = new Surface(56, 56);
    enemySurface.fill([0, 0, 0, 0]);
    enemySurface.set_at([0, 0], [222, 206, 57, 255]);
    const ui = {
      tile_size: 8,
      screen,
      loadSprite: jest.fn(),
      _get_pokemon_frame_surface: jest.fn(() => enemySurface),
      get_sprite_surface: jest.fn(() => null),
    };
    const state = {
      ui,
      layout: BattleUILayoutFactory.fromAsmDefaults(),
      sprites_enabled: true,
      enemy_sprite_frame: 0,
      player_sprite_frame: 0,
      _loaded_battle_sprites: new Set<string>(),
    } as unknown as BattleUIState;
    const runtime = {
      player_visible: false,
    } as unknown as BattleAnimationRuntime;
    const player = { species: { id: 'pikachu' } } as Pokemon;
    const enemy = {
      species: { id: 'gyarados' },
      dvs: { attack: 14, defense: 10, speed: 10, special: 10, hp: 0 },
    } as Pokemon;

    draw_battle_sprites(state, player, enemy, runtime);

    const [enemyX, enemyY] = state.layout.enemy_sprite.pixelPosition(8, 1);
    expect(screen.get_at([enemyX, enemyY])).toEqual([206, 165, 66, 255]);
  });

  it('registers battler animation gfx from the live battler surfaces before animation rendering', () => {
    const screen = new Surface(160, 144);
    const enemySurface = new Surface(56, 56);
    const playerSurface = new Surface(48, 48);
    load_player_backpic_surface.mockReturnValue(playerSurface);
    const registerBattlerSurfaces = jest.fn();
    const ui = {
      tile_size: 8,
      screen,
      loadSprite: jest.fn(),
      _get_pokemon_frame_surface: jest.fn(() => enemySurface),
      get_sprite_surface: jest.fn(() => playerSurface),
    };
    const state = {
      ui,
      layout: BattleUILayoutFactory.fromAsmDefaults(),
      sprites_enabled: true,
      enemy_sprite_frame: 0,
      player_sprite_frame: 0,
      _loaded_battle_sprites: new Set<string>(),
      animation_player: {
        anim_data: { register_battler_surfaces: registerBattlerSurfaces },
      },
    } as unknown as BattleUIState;

    draw_battle_sprites(
      state,
      { species: { id: 'pikachu' } } as Pokemon,
      { species: { id: 'hoothoot' } } as Pokemon,
      {} as BattleAnimationRuntime,
    );

    expect(registerBattlerSurfaces).toHaveBeenCalledWith({ enemySurface });
    expect(registerBattlerSurfaces).toHaveBeenCalledWith({ playerSurface });
  });

  it('masks battler-targeted overlays to sprite alpha instead of painting the transparent corners', () => {
    const screen = new Surface(160, 144);
    screen.fill([12, 34, 56, 255]);
    const enemySurface = new Surface(56, 56);
    enemySurface.fill([255, 255, 255, 0]);
    enemySurface.set_at([10, 10], [0, 0, 0, 255]);
    const layout = BattleUILayoutFactory.fromAsmDefaults();
    const ui = {
      tile_size: 8,
      screen,
      _get_pokemon_frame_surface: jest.fn(() => enemySurface),
    };
    const state = {
      ui,
      layout,
      enemy_sprite_frame: 0,
      player_sprite_frame: 0,
      animation_player: { palette_state: {}, runtime_state: {} },
      context: {
        enemyPokemon: { species: { id: 'bulbasaur' } },
        playerPokemon: { species: { id: 'chikorita' } },
      },
    } as unknown as BattleUIState;
    const runtime = {
      overlay_colour: [255, 255, 255],
      overlay_alpha: 255,
      overlay_target: 'enemy',
    } as unknown as BattleAnimationRuntime;

    apply_runtime_postprocessing(state, runtime);

    const [enemyX, enemyY] = layout.enemy_sprite.pixelPosition(8, 1);
    expect(screen.get_at([enemyX, enemyY])).toEqual([12, 34, 56, 255]);
    expect(screen.get_at([enemyX + 10, enemyY + 10])).toEqual([255, 255, 255, 255]);
  });

  it('retries battle sprite preload after a failed load instead of caching the failed attempt', () => {
    const screen = new Surface(160, 144);
    const enemySurface = new Surface(56, 56);
    const playerSurface = new Surface(56, 56);
    load_player_backpic_surface.mockReturnValue(playerSurface);
    const loadSprite = jest
      .fn()
      .mockImplementationOnce(() => {
        throw new Error('transient preload failure');
      });
    const ui = {
      tile_size: 8,
      screen,
      loadSprite,
      _get_pokemon_frame_surface: jest.fn(() => enemySurface),
      get_sprite_surface: jest.fn(() => playerSurface),
    };
    const state = {
      ui,
      layout: BattleUILayoutFactory.fromAsmDefaults(),
      sprites_enabled: true,
      enemy_sprite_frame: 0,
      player_sprite_frame: 0,
      _loaded_battle_sprites: new Set<string>(),
    } as unknown as BattleUIState;
    const runtime = {
      player_sprite_type_override: 'player_back',
    } as unknown as BattleAnimationRuntime;
    const player = { species: { id: 'pikachu' } } as Pokemon;
    const enemy = { species: { id: 'bulbasaur' } } as Pokemon;

    expect(() => draw_battle_sprites(state, player, enemy, runtime)).toThrow(
      'transient preload failure',
    );
    expect(state._loaded_battle_sprites.has('bulbasaur:pokemon_front')).toBe(false);

    draw_battle_sprites(state, player, enemy, runtime);

    expect(loadSprite).toHaveBeenNthCalledWith(1, 'bulbasaur', 'pokemon_front');
    expect(loadSprite).toHaveBeenNthCalledWith(2, 'bulbasaur', 'pokemon_front');
    expect(loadSprite).toHaveBeenNthCalledWith(3, 'pikachu', 'player_back');
  });

  it('throws from the ASM player backpic loader instead of falling back to generic sprite hooks', () => {
    const screen = new Surface(160, 144);
    const enemySurface = new Surface(56, 56);
    load_player_backpic_surface.mockImplementation(() => {
      throw new Error('missing player backpic');
    });
    const ui = {
      tile_size: 8,
      screen,
      loadSprite: jest.fn(),
      _get_pokemon_frame_surface: jest.fn(() => enemySurface),
      get_sprite_surface: jest.fn(() => new Surface(56, 56)),
    };
    const state = {
      ui,
      game_state: { wram: { player_gender: null } },
      layout: BattleUILayoutFactory.fromAsmDefaults(),
      sprites_enabled: true,
      enemy_sprite_frame: 0,
      player_sprite_frame: 0,
      _loaded_battle_sprites: new Set<string>(),
    } as unknown as BattleUIState;
    const runtime = {
      player_sprite_type_override: 'player_back',
    } as unknown as BattleAnimationRuntime;
    const player = { species: { id: 'pikachu' } } as Pokemon;
    const enemy = { species: { id: 'bulbasaur' } } as Pokemon;

    expect(() => draw_battle_sprites(state, player, enemy, runtime)).toThrow('missing player backpic');
    expect(ui.get_sprite_surface).not.toHaveBeenCalled();
  });

  it('defers drawing when an existing enemy front sprite is still preloading', () => {
    const screen = new Surface(160, 144);
    const playerSurface = new Surface(56, 56);
    load_player_backpic_surface.mockReturnValue(playerSurface);
    gameEngine.image.preload = jest.fn(async () => new Surface(56, 56));
    const ui = {
      tile_size: 8,
      screen,
      loadSprite: jest.fn(),
      _get_pokemon_frame_surface: jest.fn(() => null),
      get_sprite_surface: jest.fn(() => playerSurface),
    };
    const state = {
      ui,
      game_state: { wram: { player_gender: null } },
      layout: BattleUILayoutFactory.fromAsmDefaults(),
      sprites_enabled: true,
      enemy_sprite_frame: 0,
      player_sprite_frame: 0,
      _loaded_battle_sprites: new Set<string>(),
    } as unknown as BattleUIState;
    const runtime = {} as BattleAnimationRuntime;
    const player = { species: { id: 'pikachu' } } as Pokemon;
    const enemy = { species: { id: 'bulbasaur' } } as Pokemon;

    expect(() => draw_battle_sprites(state, player, enemy, runtime)).not.toThrow();
    expect(gameEngine.image.preload).toHaveBeenCalledWith(
      expect.stringContaining('/assets/gfx/pokemon/bulbasaur/front.png')
    );
  });

  it('defers drawing when an existing player front sprite is still preloading', () => {
    const screen = new Surface(160, 144);
    const enemySurface = new Surface(56, 56);
    load_player_backpic_surface.mockReturnValue(null);
    gameEngine.image.preload = jest.fn(async () => new Surface(56, 56));
    const ui = {
      tile_size: 8,
      screen,
      loadSprite: jest.fn(),
      _get_pokemon_frame_surface: jest.fn((speciesId: string) =>
        speciesId === 'bulbasaur' ? enemySurface : null
      ),
      get_sprite_surface: jest.fn(() => null),
    };
    const state = {
      ui,
      game_state: { wram: { player_gender: null } },
      layout: BattleUILayoutFactory.fromAsmDefaults(),
      sprites_enabled: true,
      enemy_sprite_frame: 0,
      player_sprite_frame: 0,
      _loaded_battle_sprites: new Set<string>(),
    } as unknown as BattleUIState;
    const runtime = {
      player_sprite_type_override: 'pokemon_front',
    } as unknown as BattleAnimationRuntime;
    const player = { species: { id: 'pikachu' } } as Pokemon;
    const enemy = { species: { id: 'bulbasaur' } } as Pokemon;

    expect(() => draw_battle_sprites(state, player, enemy, runtime)).not.toThrow();
    expect(gameEngine.image.preload).toHaveBeenCalledWith(
      expect.stringContaining('/assets/gfx/pokemon/pikachu/front.png')
    );
  });

  it('defers drawing when an existing player back sprite is still preloading', () => {
    const screen = new Surface(160, 144);
    const enemySurface = new Surface(56, 56);
    load_player_backpic_surface.mockReturnValue(new Surface(48, 48));
    gameEngine.image.preload = jest.fn(async () => new Surface(48, 48));
    const ui = {
      tile_size: 8,
      screen,
      loadSprite: jest.fn(),
      _get_pokemon_frame_surface: jest.fn(() => enemySurface),
      get_sprite_surface: jest.fn(() => null),
    };
    const state = {
      ui,
      game_state: { wram: { player_gender: null } },
      layout: BattleUILayoutFactory.fromAsmDefaults(),
      sprites_enabled: true,
      enemy_sprite_frame: 0,
      player_sprite_frame: 0,
      _loaded_battle_sprites: new Set<string>(),
    } as unknown as BattleUIState;
    const runtime = {
      player_sprite_type_override: 'pokemon_back',
    } as unknown as BattleAnimationRuntime;
    const player = { species: { id: 'cyndaquil' } } as Pokemon;
    const enemy = { species: { id: 'bulbasaur' } } as Pokemon;

    expect(() => draw_battle_sprites(state, player, enemy, runtime)).not.toThrow();
    expect(gameEngine.image.preload).toHaveBeenCalledWith(
      expect.stringContaining('/assets/gfx/pokemon/cyndaquil/back.png')
    );
  });

  it('defers drawing when an existing trainer sprite is still preloading', () => {
    const screen = new Surface(160, 144);
    load_player_backpic_surface.mockReturnValue(new Surface(48, 48));
    gameEngine.image.preload = jest.fn(async () => new Surface(56, 56));
    const ui = {
      tile_size: 8,
      screen,
      loadSprite: jest.fn(),
      _get_pokemon_frame_surface: jest.fn(() => null),
      get_sprite_surface: jest.fn(() => null),
    };
    const state = {
      ui,
      game_state: { wram: { player_gender: null } },
      layout: BattleUILayoutFactory.fromAsmDefaults(),
      sprites_enabled: true,
      enemy_sprite_frame: 0,
      player_sprite_frame: 0,
      _loaded_battle_sprites: new Set<string>(),
    } as unknown as BattleUIState;
    const runtime = {
      enemy_sprite_type_override: 'trainer',
      player_sprite_type_override: 'player_back',
    } as unknown as BattleAnimationRuntime;
    const player = { species: { id: 'pikachu' } } as Pokemon;
    const enemy = { species: { id: 'oak' } } as Pokemon;

    expect(() => draw_battle_sprites(state, player, enemy, runtime)).not.toThrow();
    expect(gameEngine.image.preload).toHaveBeenCalledWith(
      expect.stringContaining('/assets/gfx/trainers/oak.png')
    );
  });

  it('still throws when a pokemon front sprite asset is actually missing', () => {
    const screen = new Surface(160, 144);
    load_player_backpic_surface.mockReturnValue(new Surface(56, 56));
    gameEngine.image.preload = jest.fn();
    const ui = {
      tile_size: 8,
      screen,
      loadSprite: jest.fn(),
      _get_pokemon_frame_surface: jest.fn(() => null),
      get_sprite_surface: jest.fn(() => null),
    };
    const state = {
      ui,
      game_state: { wram: { player_gender: null } },
      layout: BattleUILayoutFactory.fromAsmDefaults(),
      sprites_enabled: true,
      enemy_sprite_frame: 0,
      player_sprite_frame: 0,
      _loaded_battle_sprites: new Set<string>(),
    } as unknown as BattleUIState;
    const runtime = {} as BattleAnimationRuntime;
    const player = { species: { id: 'pikachu' } } as Pokemon;
    const enemy = { species: { id: 'missingmon' } } as Pokemon;

    expect(() => draw_battle_sprites(state, player, enemy, runtime)).toThrow(
      'Missing battle sprite surface for enemy missingmon (pokemon_front) frame 0.',
    );
  });

  it('defers trainer hud icons when the existing ball sheet is still preloading', () => {
    gameEngine.image.loadSync = jest.fn(() => null);
    gameEngine.image.preload = jest.fn(async () => buildBallIconSheet());
    const state = {
      ui: {
        tile_size: 8,
        screen: new Surface(160, 144),
      },
      game_state: {
        wram: {
          wBattleHasJustStarted: 1,
        },
      },
      trainer_hud_visible: true,
      trainer_intro: null,
      trainer_sprites_visible: false,
      animation_player: null,
      oam_manager: new BattleSpriteOAMManager(),
    } as unknown as BattleUIState;
    const battleContext = {
      playerParty: [],
      enemyParty: [],
      trainerBattle: false,
    };

    expect(() => draw_trainer_hud_icons(state, battleContext as never)).not.toThrow();
    expect(gameEngine.image.preload).toHaveBeenCalledWith(
      expect.stringContaining('/assets/gfx/battle/balls.png')
    );
  });

  it('throws when the ball icon sheet is unavailable instead of synthesizing placeholders', () => {
    gameEngine.image.loadSync = jest.fn(() => null);
    gameEngine.image.preload = undefined as typeof gameEngine.image.preload;
    const existsSync = jest.spyOn(fs, 'existsSync').mockImplementation((target: fs.PathLike) => {
      const value = String(target);
      if (value.endsWith('gfx/battle/balls.png')) {
        return false;
      }
      return originalExistsSync(target);
    });
    const state = {
      ui: {
        tile_size: 8,
        screen: new Surface(160, 144),
      },
      game_state: {
        wram: {
          wBattleHasJustStarted: 1,
        },
      },
      trainer_hud_visible: true,
      trainer_intro: null,
      trainer_sprites_visible: false,
      animation_player: null,
      oam_manager: new BattleSpriteOAMManager(),
    } as unknown as BattleUIState;
    const battleContext = {
      playerParty: [],
      enemyParty: [],
      trainerBattle: false,
    };

    expect(() => draw_trainer_hud_icons(state, battleContext as never)).toThrow(
      'Missing battle ball icons:'
    );

    existsSync.mockRestore();
  });

  it('draws trainer hud ball icons from the loaded battle sheet', () => {
    const sheet = buildBallIconSheet();
    gameEngine.image.loadSync = jest.fn(() => sheet);
    const screen = new Surface(160, 144);
    const state = {
      ui: {
        tile_size: 8,
        screen,
      },
      game_state: {
        wram: {
          wBattleHasJustStarted: 1,
        },
      },
      trainer_hud_visible: true,
      trainer_intro: null,
      trainer_sprites_visible: false,
      animation_player: null,
      oam_manager: new BattleSpriteOAMManager(),
    } as unknown as BattleUIState;
    const battleContext = {
      playerParty: [
        {
          hp: 20,
          status: 0,
        },
      ],
      enemyParty: [],
      trainerBattle: false,
    };

    draw_trainer_hud_icons(state, battleContext as never);

    expect(screen.get_at([88, 80])[3]).toBeGreaterThan(0);
  });

  it('does not keep trainer hud ball icons visible after the intro window closes', () => {
    const sheet = buildBallIconSheet();
    gameEngine.image.loadSync = jest.fn(() => sheet);
    const screen = new Surface(160, 144);
    const flush = jest.spyOn(BattleSpriteOAMManager.prototype, 'flush');
    const state = {
      ui: {
        tile_size: 8,
        screen,
      },
      game_state: {
        wram: {
          wBattleHasJustStarted: 1,
        },
      },
      trainer_hud_visible: false,
      trainer_intro: null,
      trainer_sprites_visible: true,
      animation_player: null,
      oam_manager: new BattleSpriteOAMManager(),
    } as unknown as BattleUIState;
    const battleContext = {
      playerParty: [
        {
          hp: 20,
          status: 0,
        },
      ],
      enemyParty: [
        {
          hp: 20,
          status: 0,
        },
      ],
      trainerBattle: true,
    };

    draw_trainer_hud_icons(state, battleContext as never);

    expect(screen.get_at([88, 80])[3]).toBe(0);
    expect(screen.get_at([64, 16])[3]).toBe(0);
    expect(flush).not.toHaveBeenCalled();
    flush.mockRestore();
  });

  it('does not draw sticky party ball icons during the normal wild battle menu', () => {
    const sheet = buildBallIconSheet();
    gameEngine.image.loadSync = jest.fn(() => sheet);
    const screen = new Surface(160, 144);
    const flush = jest.spyOn(BattleSpriteOAMManager.prototype, 'flush');
    const state = {
      ui: {
        tile_size: 8,
        screen,
      },
      game_state: {
        wram: {
          wBattleHasJustStarted: 0,
        },
      },
      trainer_hud_visible: true,
      trainer_intro: null,
      trainer_exit: null,
      trainer_send_out_seen: true,
      animation_player: null,
      oam_manager: new BattleSpriteOAMManager(),
    } as unknown as BattleUIState;
    const battleContext = {
      playerParty: [
        {
          hp: 20,
          status: 0,
        },
      ],
      enemyParty: [],
      trainerBattle: false,
    };

    draw_trainer_hud_icons(state, battleContext as never);

    expect(screen.get_at([88, 80])[3]).toBe(0);
    expect(flush).not.toHaveBeenCalled();
    flush.mockRestore();
  });

  it('throws when the ball icon sheet is truncated instead of backfilling placeholders', () => {
    const sheet = new Surface(24, 8);
    sheet.fill([170, 170, 170, 255]);
    gameEngine.image.loadSync = jest.fn(() => sheet);
    const state = {
      ui: {
        tile_size: 8,
        screen: new Surface(160, 144),
      },
      game_state: {
        wram: {
          wBattleHasJustStarted: 1,
        },
      },
      trainer_hud_visible: true,
      trainer_intro: null,
      trainer_sprites_visible: false,
      animation_player: null,
      oam_manager: new BattleSpriteOAMManager(),
    } as unknown as BattleUIState;
    const battleContext = {
      playerParty: [],
      enemyParty: [],
      trainerBattle: false,
    };

    expect(() => draw_trainer_hud_icons(state, battleContext as never)).toThrow(
      'Battle ball icon sheet is missing tile 3'
    );
  });

  it('throws when battle object palettes are unavailable instead of fabricating grayscale defaults', () => {
    const sheet = buildBallIconSheet();
    gameEngine.image.loadSync = jest.fn(() => sheet);
    const existsSync = jest.spyOn(fs, 'existsSync').mockImplementation((target: fs.PathLike) => {
      const value = String(target);
      if (value.endsWith('gfx/battle_anims/battle_anims.pal')) {
        return false;
      }
      return originalExistsSync(target);
    });
    const state = {
      ui: {
        tile_size: 8,
        screen: new Surface(160, 144),
      },
      game_state: {
        wram: {
          wBattleHasJustStarted: 1,
        },
      },
      trainer_hud_visible: true,
      trainer_intro: null,
      trainer_sprites_visible: false,
      animation_player: null,
      oam_manager: new BattleSpriteOAMManager(),
    } as unknown as BattleUIState;
    const battleContext = {
      playerParty: [],
      enemyParty: [],
      trainerBattle: false,
    };

    expect(() => draw_trainer_hud_icons(state, battleContext as never)).toThrow(
      'Battle object palettes are required for the asset-only runtime:'
    );

    existsSync.mockRestore();
  });

  it('throws when battle object palette data is incomplete instead of fabricating grayscale defaults', () => {
    const sheet = buildBallIconSheet();
    gameEngine.image.loadSync = jest.fn(() => sheet);
    const readFileSync = jest.spyOn(fs, 'readFileSync').mockImplementation(((target: fs.PathLike, options?: unknown) => {
      const value = String(target);
      if (value.endsWith('gfx/battle_anims/battle_anims.pal')) {
        return [
          'RGB 31, 31, 31, 21, 21, 21, 10, 10, 10, 0, 0, 0',
          'RGB 31, 0, 0, 21, 0, 0, 10, 0, 0, 0, 0, 0',
        ].join('\n');
      }
      return originalReadFileSync(target, options as Parameters<typeof fs.readFileSync>[1]);
    }) as typeof fs.readFileSync);
    const state = {
      ui: {
        tile_size: 8,
        screen: new Surface(160, 144),
      },
      game_state: {
        wram: {
          wBattleHasJustStarted: 1,
        },
      },
      trainer_hud_visible: true,
      trainer_intro: null,
      trainer_sprites_visible: false,
      animation_player: null,
      oam_manager: new BattleSpriteOAMManager(),
    } as unknown as BattleUIState;
    const battleContext = {
      playerParty: [],
      enemyParty: [],
      trainerBattle: false,
    };

    expect(() => draw_trainer_hud_icons(state, battleContext as never)).toThrow(
      'Battle object palette source is incomplete:'
    );

    readFileSync.mockRestore();
  });
});
