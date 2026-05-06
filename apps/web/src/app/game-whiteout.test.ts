jest.mock("@pokecrystal/core/ui/overlays/battle-ui", () => ({
  create_battle_ui: jest.fn(() => ({ active: false })),
  begin_battle: jest.fn(),
  end_battle: jest.fn(),
  set_audio_engine: jest.fn(),
}));

jest.mock("@pokecrystal/core/engine/world/special-events", () => ({
  __esModule: true,
  fade_in_from_white: jest.fn(),
  fade_out_to_white: jest.fn(),
  heal_party: jest.fn(),
  warp_to_spawn_point: jest.fn(() => true),
}));

jest.mock("@pokecrystal/core/engine/world/story-events/specials/handlers", () => ({
  __esModule: true,
  STANDARD_SCRIPT_HANDLERS: {
    BugContestResultsWarpScript: jest.fn(),
  },
}));

jest.mock("@pokecrystal/core/core/logger", () => ({
  __esModule: true,
  default: {
    info: jest.fn(),
    warn: jest.fn(),
    error: jest.fn(),
  },
}));

jest.mock("@pokecrystal/core/core/save", () => {
  class SaveFileNotFoundError extends Error {}
  return {
    saveGame: jest.fn().mockResolvedValue(true),
    saveGameWithHistory: jest.fn().mockResolvedValue(true),
    loadGame: jest.fn(),
    SaveFileNotFoundError,
  };
});

import { Game } from "./game";
import { TextUI } from "@pokecrystal/core/ui/text-ui";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import type { BaseFontRenderer } from "@pokecrystal/core/ui/base-ui";
import type { Surface } from "@pokecrystal/core/ui/surface";
import { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { Event, type EventManager } from "@pokecrystal/core/engine/events/events";
import {
  fade_in_from_white,
  fade_out_to_white,
  heal_party,
  warp_to_spawn_point,
} from "@pokecrystal/core/engine/world/special-events";
import { createInitialGameState } from "@pokecrystal/core/core/state";
import { loadGame, saveGame } from "@pokecrystal/core/core/save";
import { Ability, EggGroup, GenderRatio, GrowthRate, PokemonType } from "@pokecrystal/core/core/enums";
import { StatusCondition } from "@pokecrystal/core/core/enums/battle";
import type { PokemonSpecies } from "@pokecrystal/core/core/models";
import { createPokemon } from "@pokecrystal/core/engine/systems/pokemon";

type MockFontRenderer = BaseFontRenderer & {
  font_tiles?: Record<number, InstanceType<typeof gameEngine.Surface>>;
};

type MockTextUI = TextUI & {
  tile_size?: number;
  font: MockFontRenderer;
};

interface TilesetInstance {
  tilesetName: string;
  metatiles: Array<{ collision: number[] }>;
  renderMetatile(): void;
  renderPriorityMetatile(): void;
}

type TilesetConstructor = new (tilesetName: string) => TilesetInstance;

type GlobalOverrides = {
  fetch?: typeof globalThis.fetch | undefined;
  createImageBitmap?: typeof globalThis.createImageBitmap | undefined;
  Tileset?: TilesetConstructor;
};

type GameInternals = {
  eventManager: EventManager;
};

const DEFAULT_BASE_STATS = {
  hp: 20,
  attack: 10,
  defense: 10,
  speed: 10,
  special_attack: 10,
  special_defense: 10,
};

const speciesCache = new Map<string, PokemonSpecies>();

const ensureSpecies = (id: string): PokemonSpecies => {
  const upperId = id.toUpperCase();
  const cached = speciesCache.get(upperId);
  if (cached) {
    return cached;
  }
  const species: PokemonSpecies = {
    id: upperId,
    int_id: 0,
    base_stats: DEFAULT_BASE_STATS,
    type1: PokemonType.NORMAL,
    type2: PokemonType.NONE,
    catch_rate: 45,
    base_exp: 64,
    item1: undefined,
    item2: undefined,
    gender_ratio: GenderRatio.GENDER_F50,
    unknown1: 0,
    step_cycles_to_hatch: 5120,
    unknown2: 0,
    growth_rate: GrowthRate.GROWTH_MEDIUM_FAST,
    egg_group1: EggGroup.EGG_MONSTER,
    egg_group2: EggGroup.EGG_MONSTER,
    tmhm_learnset: [],
    ability: Ability.NONE,
    pic_size: 0,
    front_pic: 0,
    back_pic: 0,
    evolutions: null,
    weight: 0,
  };
  speciesCache.set(upperId, species);
  return species;
};

const getEventManager = (target: Game): EventManager =>
  (target as unknown as GameInternals).eventManager;

const buildGame = async (options: Parameters<typeof Game.create>[1] = {}): Promise<Game> => {
  const ui = new TextUI(160, 144, 1, null, false, 0) as MockTextUI;
  const fontTiles: Record<number, InstanceType<typeof gameEngine.Surface>> = {};
  for (let i = 0; i < 256; i += 1) {
    fontTiles[i] = new gameEngine.Surface(8, 8);
  }
  ui.tile_size = 8;
  const fontRenderer = ui.font;
  fontRenderer.font_tiles = fontTiles as unknown as Record<number, Surface>;
  const noopRender: (..._args: Parameters<NonNullable<BaseFontRenderer["renderText"]>>) => void = () => {};
  fontRenderer.render_text = noopRender;
  fontRenderer.renderText = noopRender;

  const globalScope = globalThis as GlobalOverrides;
  const originalFetch = globalScope.fetch;
  const originalCreateImageBitmap = globalScope.createImageBitmap;
  const originalTileset = globalScope.Tileset;
  const originalImageLoad = gameEngine.image.load;
  const originalInitAssets = OverworldEngine.prototype.init_assets;

  class TilesetStub implements TilesetInstance {
    public tilesetName: string;
    public metatiles: Array<{ collision: number[] }>;

    constructor(tilesetName: string) {
      this.tilesetName = tilesetName || "placeholder";
      this.metatiles = Array.from({ length: 256 }, () => ({ collision: [0, 0, 0, 0] }));
    }

    renderMetatile(): void {
      // No-op for headless test coverage.
    }

    renderPriorityMetatile(): void {
      // No-op for headless test coverage.
    }
  }

  globalScope.fetch = undefined;
  globalScope.createImageBitmap = undefined;
  globalScope.Tileset = TilesetStub;
  gameEngine.image.load = async () => new gameEngine.Surface(24, 16);
  OverworldEngine.prototype.init_assets = async () => {};

  try {
    return await Game.create(ui, options);
  } finally {
    globalScope.fetch = originalFetch;
    globalScope.createImageBitmap = originalCreateImageBitmap;
    globalScope.Tileset = originalTileset;
    gameEngine.image.load = originalImageLoad;
    OverworldEngine.prototype.init_assets = originalInitAssets;
  }
};

describe("Game whiteout integration", () => {
  beforeEach(() => {
    jest.clearAllMocks();
    (loadGame as jest.Mock).mockReset();
    (warp_to_spawn_point as jest.Mock).mockReturnValue(true);
    (heal_party as jest.Mock).mockImplementation((context?: { game_state?: { sram?: { party?: { pokemon?: Array<{ hp?: number; max_hp?: number } | null> } } } }) => {
      const party = context?.game_state?.sram?.party?.pokemon ?? [];
      for (const mon of party) {
        if (mon) {
          mon.hp = Number(mon.max_hp ?? mon.hp ?? 0);
        }
      }
    });
  });

  it("whiteouts after a battle loss with no usable party members", async () => {
    const game = await buildGame();
    const eventManager = getEventManager(game);
    const textUi = (game as unknown as { ui: TextUI }).ui;
    const renderSnapshotSpy = jest.spyOn(textUi, "renderSnapshot");
    const dispatchSpy = jest.spyOn(eventManager, "dispatch");
    const overworld = game.getOverworld() as OverworldEngine & {
      dialogue: {
        waiting_for_input?: boolean;
        pending_waits?: number;
        active?: boolean;
        visible?: boolean;
        is_script_paused?: boolean;
        update?: () => void;
      } | null;
      input_capture_active?: boolean;
      lock_player_movement?: jest.Mock;
      unlock_player_movement?: jest.Mock;
    };
    if (overworld.dialogue) {
      overworld.dialogue.waiting_for_input = false;
    }
    overworld.input_capture_active = true;
    overworld.lock_player_movement = jest.fn();
    overworld.unlock_player_movement = jest.fn();
    renderSnapshotSpy.mockClear();

    const rattata = createPokemon(game.getGameState(), ensureSpecies("RATTATA"), 5);
    rattata.nickname = "DOWN";
    rattata.hp = 0;
    const pidgey = createPokemon(game.getGameState(), ensureSpecies("PIDGEY"), 5);
    pidgey.nickname = "OUT";
    pidgey.hp = 0;
    game.getGameState().sram.party.pokemon = [rattata, pidgey, null, null, null, null];
    game.getGameState().sram.money = 2000;

    eventManager.dispatch(new Event("battle_complete", { result: 1 }));

    game.tick();

    expect(overworld.input_capture_active).toBe(false);
    expect(renderSnapshotSpy).toHaveBeenCalled();

    game.tick();
    expect(dispatchSpy.mock.calls.map(([event]) => event.name)).toContain("show_text");
    expect(overworld.dialogue?.visible ?? false).toBe(true);
    expect(overworld.dialogue?.waiting_for_input ?? false).toBe(true);
    expect(overworld.lock_player_movement).toHaveBeenCalled();
  });

  it("whiteouts from overworld ticks when the party is already fully fainted", async () => {
    const game = await buildGame();
    const overworld = game.getOverworld() as OverworldEngine & {
      dialogue: {
        waiting_for_input?: boolean;
        pending_waits?: number;
        active?: boolean;
        visible?: boolean;
        is_script_paused?: boolean;
        update?: () => void;
      };
      lock_player_movement?: jest.Mock;
      unlock_player_movement?: jest.Mock;
    };
    overworld.dialogue = {
      waiting_for_input: false,
      pending_waits: 0,
      active: false,
      visible: false,
      is_script_paused: false,
      update: jest.fn(),
    };
    overworld.lock_player_movement = jest.fn();
    overworld.unlock_player_movement = jest.fn();

    const gameState = game.getGameState();
    const faintedPlayer = createPokemon(gameState, ensureSpecies("CYNDAQUIL"), 10);
    faintedPlayer.hp = 0;

    gameState.sram.party.pokemon = [faintedPlayer, null, null, null, null, null];
    gameState.sram.money = 2000;

    for (let i = 0; i < 60; i += 1) {
      game.tick();
    }

    expect(fade_out_to_white).toHaveBeenCalledTimes(1);
    expect(heal_party).toHaveBeenCalledTimes(1);
    expect(warp_to_spawn_point).toHaveBeenCalledTimes(1);
    expect(fade_in_from_white).toHaveBeenCalledTimes(1);
    expect(overworld.lock_player_movement).toHaveBeenCalled();
    expect(overworld.unlock_player_movement).toHaveBeenCalled();
    expect(gameState.sram.money).toBe(1000);
  });

  it("does not autosave the invalid all-fainted state when battle_complete starts whiteout", async () => {
    const game = await buildGame();
    const eventManager = getEventManager(game);
    const saveGameMock = saveGame as jest.Mock;
    const rattata = createPokemon(game.getGameState(), ensureSpecies("RATTATA"), 5);
    rattata.hp = 0;

    game.getGameState().sram.party.pokemon = [rattata, null, null, null, null, null];
    saveGameMock.mockClear();

    eventManager.dispatch(new Event("battle_complete", { result: 1 }));

    expect(saveGameMock).not.toHaveBeenCalled();
  });

  it("starts whiteout on the first tick after loading a save with a fully fainted party", async () => {
    const loadedState = createInitialGameState();
    const faintedPlayer = createPokemon(loadedState, ensureSpecies("CYNDAQUIL"), 10);
    faintedPlayer.hp = 0;
    loadedState.sram.party.pokemon = [faintedPlayer, null, null, null, null, null];
    (loadGame as jest.Mock).mockResolvedValue(loadedState);

    const game = await buildGame({ loadSlot: "startup-whiteout.sav", strictLoadSlot: true });
    const overworld = game.getOverworld() as OverworldEngine & {
      dialogue: {
        waiting_for_input?: boolean;
        pending_waits?: number;
        active?: boolean;
        visible?: boolean;
        is_script_paused?: boolean;
        update?: () => void;
      };
      lock_player_movement?: jest.Mock;
      unlock_player_movement?: jest.Mock;
    };
    overworld.dialogue = {
      waiting_for_input: false,
      pending_waits: 0,
      active: false,
      visible: false,
      is_script_paused: false,
      update: jest.fn(),
    };
    overworld.lock_player_movement = jest.fn();
    overworld.unlock_player_movement = jest.fn();

    game.tick();

    expect(overworld.lock_player_movement).toHaveBeenCalledTimes(1);
    expect(saveGame).not.toHaveBeenCalled();

    for (let i = 0; i < 60; i += 1) {
      game.tick();
    }

    expect(fade_out_to_white).toHaveBeenCalledTimes(1);
    expect(heal_party).toHaveBeenCalledTimes(1);
    expect(warp_to_spawn_point).toHaveBeenCalledTimes(1);
  });

  it("whiteouts and warps on startup even when the whole fainted party is poisoned", async () => {
    const loadedState = createInitialGameState();
    const faintedPoisonedPlayer = createPokemon(loadedState, ensureSpecies("CYNDAQUIL"), 10);
    faintedPoisonedPlayer.hp = 0;
    faintedPoisonedPlayer.status = StatusCondition.POISON;
    loadedState.sram.party.pokemon = [faintedPoisonedPlayer, null, null, null, null, null];
    (loadGame as jest.Mock).mockResolvedValue(loadedState);

    const game = await buildGame({ loadSlot: "startup-poison-whiteout.sav", strictLoadSlot: true });
    const overworld = game.getOverworld() as OverworldEngine & {
      dialogue: {
        waiting_for_input?: boolean;
        pending_waits?: number;
        active?: boolean;
        visible?: boolean;
        is_script_paused?: boolean;
        update?: () => void;
      };
      lock_player_movement?: jest.Mock;
      unlock_player_movement?: jest.Mock;
    };
    overworld.dialogue = {
      waiting_for_input: false,
      pending_waits: 0,
      active: false,
      visible: false,
      is_script_paused: false,
      update: jest.fn(),
    };
    overworld.lock_player_movement = jest.fn();
    overworld.unlock_player_movement = jest.fn();

    game.tick();

    expect(overworld.lock_player_movement).toHaveBeenCalledTimes(1);

    for (let i = 0; i < 60; i += 1) {
      game.tick();
    }

    expect(fade_out_to_white).toHaveBeenCalledTimes(1);
    expect(heal_party).toHaveBeenCalledTimes(1);
    expect(warp_to_spawn_point).toHaveBeenCalledTimes(1);
    expect(faintedPoisonedPlayer.hp).toBeGreaterThan(0);
  });
});
