jest.mock("@pokecrystal/core/ui/overlays/battle-ui", () => ({
  create_battle_ui: jest.fn(() => ({ active: false })),
  begin_battle: jest.fn(),
  end_battle: jest.fn(),
  set_audio_engine: jest.fn(),
}));

import { Game } from "./game";
import { TextUI } from "@pokecrystal/core/ui/text-ui";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import type { BaseFontRenderer } from "@pokecrystal/core/ui/base-ui";
import type { Surface } from "@pokecrystal/core/ui/surface";
import { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { StartBattleEvent, type EventManager } from "@pokecrystal/core/engine/events/events";
import { PokemonSchema, type Pokemon } from "@pokecrystal/core/core/models";
import { TrainerSchema } from "@pokecrystal/core/core/models/trainer";
import {
  Ability,
  EggGroup,
  GenderRatio,
  GrowthRate,
  PokemonType,
} from "@pokecrystal/core/core/enums";
import { StatusCondition } from "@pokecrystal/core/core/enums/battle";
import { TrainerBattle } from "@pokecrystal/core/engine/battle/battle/trainer-battle";
import { finaliseBattle } from "@pokecrystal/core/engine/battle/battle/battle-finalization";
import { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { determineBattleMusic } from "@pokecrystal/core/engine/battle/battle/music";

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

const getEventManager = (target: Game): EventManager =>
  (target as unknown as GameInternals).eventManager;

const buildGame = async (): Promise<Game> => {
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
  gameEngine.image.load = async (path?: string) => {
    if (typeof path === "string" && /\/gfx\/frames\/\d+\.png$/.test(path)) {
      return new gameEngine.Surface(24, 16);
    }
    return new gameEngine.Surface(16, 16);
  };
  OverworldEngine.prototype.init_assets = async () => {};

  try {
    return await Game.create(ui);
  } finally {
    globalScope.fetch = originalFetch;
    globalScope.createImageBitmap = originalCreateImageBitmap;
    globalScope.Tileset = originalTileset;
    gameEngine.image.load = originalImageLoad;
    OverworldEngine.prototype.init_assets = originalInitAssets;
  }
};

const buildPokemonData = (id: string, level: number): Pokemon => {
  const species = {
    id,
    int_id: 1,
    base_stats: {
      hp: 35,
      attack: 55,
      defense: 40,
      speed: 90,
      special_attack: 50,
      special_defense: 50,
    },
    type1: PokemonType.NORMAL,
    type2: PokemonType.NORMAL,
    catch_rate: 45,
    base_exp: 60,
    gender_ratio: GenderRatio.GENDER_F12_5,
    unknown1: 0,
    step_cycles_to_hatch: 20,
    unknown2: 0,
    growth_rate: GrowthRate.GROWTH_MEDIUM_FAST,
    egg_group1: EggGroup.EGG_MONSTER,
    egg_group2: EggGroup.EGG_MONSTER,
    tmhm_learnset: [],
    ability: Ability.NONE,
    pic_size: 0,
    front_pic: 0,
    back_pic: 0,
    weight: 0,
  };
  return PokemonSchema.parse({
    species,
    nickname: id,
    level,
    hp: 10,
    max_hp: 10,
    original_trainer_name: "PLAYER",
    original_trainer_id: 0,
    experience: 0,
    happiness: 70,
  }) as unknown as Pokemon;
};

describe("Game battle start", () => {
  it("normalizes start battle payload pokemon for battle stats", async () => {
    const game = await buildGame();
    const eventManager = getEventManager(game);
    const playerData = buildPokemonData("PLAYERMON", 5);
    const enemyData = buildPokemonData("ENEMYMON", 4);

    eventManager.dispatch(
      new StartBattleEvent({
        player_pokemon: playerData,
        enemy_pokemon: enemyData,
        player_party: [playerData],
        enemy_party: [enemyData],
      })
    );

    const battle = game.getBattle();
    expect(battle).not.toBeNull();
    expect(typeof battle?.context.playerPokemon._calculateStat).toBe("function");
    expect(typeof battle?.context.enemyPokemon._calculateStat).toBe("function");
    for (const pokemon of battle?.context.playerParty ?? []) {
      expect(typeof pokemon._calculateStat).toBe("function");
    }
    expect(battle?.context.playerPokemon).toBe(playerData);
    expect(battle?.context.playerParty[0]).toBe(playerData);

    battle!.context.playerPokemon.hp = 3;
    battle!.context.playerPokemon.status = StatusCondition.POISON;
    battle!.context.playerPokemon.experience = 1234;
    expect(playerData.hp).toBe(3);
    expect(playerData.status).toBe(StatusCondition.POISON);
    expect(playerData.experience).toBe(1234);
  });

  it("uses TrainerBattle when a trainer payload is supplied", async () => {
    const game = await buildGame();
    const eventManager = getEventManager(game);
    const playerData = buildPokemonData("PLAYERMON", 5);
    const enemyData = buildPokemonData("ENEMYMON", 4);
    const trainer = TrainerSchema.parse({
      name: "TRAINER_TEST",
      trainer_class: "BUG_CATCHER",
      party: [enemyData],
      win_quote: "",
      lose_quote: "",
    });

    eventManager.dispatch(
      new StartBattleEvent({
        player_pokemon: playerData,
        enemy_pokemon: enemyData,
        player_party: [playerData],
        enemy_party: [enemyData],
        trainer,
      })
    );

    const battle = game.getBattle();
    expect(battle).not.toBeNull();
    expect(battle).toBeInstanceOf(TrainerBattle);
  });

  it("pays trainer battle reward through the game battle lifecycle", async () => {
    const game = await buildGame();
    const eventManager = getEventManager(game);
    const gameState = game.getGameState();
    const playerData = buildPokemonData("PLAYERMON", 12);
    const enemyData = buildPokemonData("ENEMYMON", 8);
    const trainer = TrainerSchema.parse({
      name: "TRAINER_TEST",
      trainer_class: "BUG_CATCHER",
      party: [enemyData],
      win_quote: "",
      lose_quote: "",
    });
    gameState.sram.money = 0;
    gameState.sram.moms_money = 0;
    gameState.sram.mom_saving_some_money = false;
    (game as unknown as { _runAutosave: jest.Mock })._runAutosave = jest.fn().mockResolvedValue(false);

    eventManager.dispatch(
      new StartBattleEvent({
        player_pokemon: playerData,
        enemy_pokemon: enemyData,
        player_party: [playerData],
        enemy_party: [enemyData],
        trainer,
        trainer_id: "TRAINER_TEST",
        trainer_reward: 200,
      })
    );

    const battle = game.getBattle();
    expect(battle).toBeInstanceOf(TrainerBattle);
    expect(battle?.context.trainerReward).toBe(200);
    Object.assign(battle!.battleUi, {
      pending_evolutions: [],
      pending_move_learns: [],
      block_on_move_learning: false,
      block_on_pending_evolution: false,
      active_evolution: null,
    });
    battle!.context.enemyParty.forEach((pokemon) => {
      pokemon.hp = 0;
    });
    battle!.context.enemyPokemon.hp = 0;

    finaliseBattle(battle!);
    await Promise.resolve();

    expect(gameState.sram.money).toBe(800);
  });

  it("starts battle music when a battle begins", async () => {
    const playMusicSpy = jest
      .spyOn(AudioEngine.prototype, "playMusic")
      .mockImplementation(() => {});
    const game = await buildGame();
    const eventManager = getEventManager(game);
    const playerData = buildPokemonData("PLAYERMON", 5);
    const enemyData = buildPokemonData("ENEMYMON", 4);

    eventManager.dispatch(
      new StartBattleEvent({
        player_pokemon: playerData,
        enemy_pokemon: enemyData,
        player_party: [playerData],
        enemy_party: [enemyData],
      })
    );

    expect(playMusicSpy).toHaveBeenCalledWith(
      determineBattleMusic(game.getGameState()),
      "battle"
    );
    playMusicSpy.mockRestore();
  });

  it("does not restart map music from the battle teardown tick", async () => {
    const restartMapMusicSpy = jest
      .spyOn(AudioEngine.prototype, "restartMapMusic")
      .mockImplementation(() => {});
    const game = await buildGame();
    const eventManager = getEventManager(game);
    const playerData = buildPokemonData("PLAYERMON", 5);
    const enemyData = buildPokemonData("ENEMYMON", 4);

    eventManager.dispatch(
      new StartBattleEvent({
        player_pokemon: playerData,
        enemy_pokemon: enemyData,
        player_party: [playerData],
        enemy_party: [enemyData],
      })
    );

    const battle = game.getBattle();
    expect(battle).not.toBeNull();
    battle!.update = jest.fn();
    battle!.isFinished = jest.fn(() => true);
    battle!.teardown = jest.fn();

    game.tick();

    expect(restartMapMusicSpy).not.toHaveBeenCalled();
    restartMapMusicSpy.mockRestore();
  });

});
