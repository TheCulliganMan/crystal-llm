import { PokemonSpeciesSchema, TrainerSchema, type Pokemon } from "@pokecrystal/core/core/models";
import { ItemSchema } from "@pokecrystal/core/core/models/item";
import { getFilledSlots } from "@pokecrystal/core/core/models/party";
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { getPokedexFlag } from "@pokecrystal/core/core/pokedex";
import { createInitialGameState } from "@pokecrystal/core/core/state";
import { Event, EventManager } from "@pokecrystal/core/engine/events/events";
import type { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { TILE_SIZE } from "@pokecrystal/core/engine/world/tile";
import type { OverworldMap } from "@pokecrystal/core/engine/world/overworld";
import type { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import * as asyncLoop from "@pokecrystal/core/ui/async-loop";
import type { ScriptRunner } from "../runner";
import { ScriptRunnerImpl } from "../runner";
import { createTestPokemon } from "../test-utils";
import * as textHelpers from "../text-helpers";
import {
  GiveEggCommand,
  GiveNicknamePromptCommand,
  GivePokeCommand,
  LoadTrainerCommand,
  resolveItemName,
  resolveSpecies,
  StartBattleCommand,
  TrainerCommand,
} from "./battle";

const installNamingScreenAssets = (): (() => void) => {
  const fontSurface = new gameEngine.Surface(TILE_SIZE * 2, TILE_SIZE);
  fontSurface.fill([255, 255, 255, 255]);
  const cursorSurface = new gameEngine.Surface(TILE_SIZE, TILE_SIZE * 2);
  cursorSurface.fill([0, 0, 0, 0]);
  const borderSurface = new gameEngine.Surface(TILE_SIZE, TILE_SIZE);
  borderSurface.fill([10, 10, 10, 255]);
  const underlineSurface = new gameEngine.Surface(TILE_SIZE, TILE_SIZE);
  underlineSurface.fill([20, 20, 20, 255]);
  const middleLineSurface = new gameEngine.Surface(TILE_SIZE, TILE_SIZE);
  middleLineSurface.fill([30, 30, 30, 255]);

  const loadSyncMock = jest
    .spyOn(gameEngine.image, "loadSync")
    .mockImplementation((path: string) => {
      if (path.includes("gfx/font/font.png")) {
        return fontSurface;
      }
      if (path.includes("gfx/naming_screen/cursor.png")) {
        return cursorSurface;
      }
      if (path.includes("gfx/naming_screen/border.png")) {
        return borderSurface;
      }
      if (path.includes("gfx/naming_screen/underline.png")) {
        return underlineSurface;
      }
      if (path.includes("gfx/naming_screen/middle_line.png")) {
        return middleLineSurface;
      }
      return null;
    });

  return () => loadSyncMock.mockRestore();
};

const mockResolvedBattleText = (labels: Record<string, string>) =>
  jest.spyOn(textHelpers, "resolveText").mockImplementation(
    (_runner, _overworld, label) => labels[label] ?? label
  );

describe("battle command lookups", () => {
  it("binds species lookups to the loader instance", () => {
    const species = PokemonSpeciesSchema.parse({
      id: "TOTODILE",
      int_id: 158,
      base_stats: {
        hp: 50,
        attack: 65,
        defense: 64,
        speed: 43,
        special_attack: 44,
        special_defense: 48,
      },
      type1: "WATER",
      type2: "WATER",
      catch_rate: 45,
      base_exp: 66,
      gender_ratio: 31,
      unknown1: 0,
      step_cycles_to_hatch: 20,
      unknown2: 0,
      growth_rate: "GROWTH_MEDIUM_SLOW",
      egg_group1: "EGG_MONSTER",
      egg_group2: "EGG_WATER_1",
    });
    const pokemonData = new Map([[species.id, species]]);
    const loader = {
      pokemonData,
      get_pokemon_species: (name: string) => pokemonData.get(name) ?? null,
    } as unknown as DataLoader;

    expect(resolveSpecies(loader, "Totodile")).toBe(species);
  });

  it("binds item lookups to the loader instance", () => {
    const item = ItemSchema.parse({ name: "POTION" });
    const itemData = new Map([[item.name, item]]);
    const loader = {
      itemData,
      get_item: (name: string) => itemData.get(name) ?? null,
    } as unknown as DataLoader;

    expect(resolveItemName(loader, "potion")).toBe("POTION");
  });

  it("throws when a gift item token cannot be resolved", () => {
    const loader = {
      itemData: new Map(),
      get_item: () => null,
    } as unknown as DataLoader;

    expect(() => resolveItemName(loader, "mystery_foo")).toThrow(
      "Unknown gift item 'mystery_foo'."
    );
  });

  it("binds trainer lookups to the data loader instance", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const trainer = {
      name: "TRAINER_TEST",
      party: [],
    };
    const dataLoader = {
      trainerData: new Map([["TRAINER_TEST", trainer]]),
      get_trainer(name: string) {
        if (this !== dataLoader) {
          throw new Error("unbound trainer lookup");
        }
        return this.trainerData.get(name) ?? null;
      },
    };
    const overworld = { data_loader: dataLoader } as unknown as OverworldMap;

    const command = new LoadTrainerCommand("TRAINER_CLASS", "TRAINER_TEST");
    command.runner = {} as ScriptRunner;
    command.execute(gameState, eventManager, overworld);

    expect(gameState.wram.other_trainer?.name).toBe("TRAINER_TEST");
  });

  it("resolves the rival placeholder name to question marks before the rival is named", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const trainer = TrainerSchema.parse({
      name: "<RIVAL>",
      party: [],
    });
    const dataLoader = {
      trainerData: new Map([["RIVAL_TEST", trainer]]),
      get_trainer(name: string) {
        return this.trainerData.get(name) ?? null;
      },
    };
    const overworld = { data_loader: dataLoader } as unknown as OverworldMap;

    const command = new LoadTrainerCommand("TRAINER_CLASS", "RIVAL_TEST");
    command.runner = {} as ScriptRunner;
    command.execute(gameState, eventManager, overworld);

    expect(gameState.wram.other_trainer?.name).toBe("???");
  });

  it("resolves the rival placeholder name to the saved rival name once it exists", () => {
    const gameState = createInitialGameState();
    gameState.sram.rival_name = "SILVER";
    const eventManager = new EventManager(gameState);
    const trainer = TrainerSchema.parse({
      name: "<RIVAL>",
      party: [],
    });
    const dataLoader = {
      trainerData: new Map([["RIVAL_TEST", trainer]]),
      get_trainer(name: string) {
        return this.trainerData.get(name) ?? null;
      },
    };
    const overworld = { data_loader: dataLoader } as unknown as OverworldMap;

    const command = new LoadTrainerCommand("TRAINER_CLASS", "RIVAL_TEST");
    command.runner = {} as ScriptRunner;
    command.execute(gameState, eventManager, overworld);

    expect(gameState.wram.other_trainer?.name).toBe("SILVER");
  });
});

describe("givepoke nickname handling", () => {
  it("dispatches the nickname prompt and records the result after the callback", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    dataLoader.get_text = (label: string) =>
      label === "CaughtAskNicknameText" ? "Give a nickname?" : label;
    const overworld = { dialogue: { active: false } } as unknown as OverworldEngine;
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    const command = new GiveNicknamePromptCommand();
    command.runner = runner;

    const dispatchSpy = jest.spyOn(eventManager, "dispatch");
    command.execute(gameState, eventManager, overworld);

    const names = dispatchSpy.mock.calls.map(([event]) => (event as Event).name);
    expect(names).toEqual(["open_text", "show_text", "wait_for_input", "prompt_yes_no"]);
    expect(Object.prototype.hasOwnProperty.call(runner.variables, "_givepoke_nickname_choice")).toBe(false);

    const promptEvent = dispatchSpy.mock.calls[3]?.[0] as Event;
    const callback = promptEvent?.data?.callback as ((value: boolean) => void) | undefined;
    expect(typeof callback).toBe("function");
    callback?.(true);

    expect(runner.last_yes_no_result).toBe(true);
    expect(runner.last_condition_result).toBe(true);
    expect(runner.variables._givepoke_nickname_choice).toEqual([0]);
  });

  it("applies a nickname override when the givepoke choice is accepted", () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = "CHRIS";
    gameState.sram.player_id = 4321;
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    const species = PokemonSpeciesSchema.parse({
      id: "TOTODILE",
      int_id: 158,
      base_stats: {
        hp: 50,
        attack: 65,
        defense: 64,
        speed: 43,
        special_attack: 44,
        special_defense: 48,
      },
      type1: "WATER",
      type2: "WATER",
      catch_rate: 45,
      base_exp: 66,
      gender_ratio: 31,
      unknown1: 0,
      step_cycles_to_hatch: 20,
      unknown2: 0,
      growth_rate: "GROWTH_MEDIUM_SLOW",
      egg_group1: "EGG_MONSTER",
      egg_group2: "EGG_WATER_1",
    });
    dataLoader.pokemonData = new Map([[species.id, species]]);
    const overworld = {} as unknown as OverworldEngine;
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    runner.variables._givepoke_nickname_choice = [0];
    runner.variables._givepoke_nickname_value = "CRONCLAW";

    const command = new GivePokeCommand("Totodile", 5, null);
    command.runner = runner;
    command.execute(gameState, eventManager, overworld);

    expect(getFilledSlots(gameState.sram.party)).toBe(1);
    expect(gameState.sram.party.pokemon[0]?.nickname).toBe("CRONCLAW");
    expect(getPokedexFlag(gameState, species.int_id, "seen")).toBe(true);
    expect(getPokedexFlag(gameState, species.int_id, "owned")).toBe(true);
    expect(runner.variables._givepoke_nickname_value).toBeUndefined();
    expect(runner.variables._givepoke_nickname_choice).toBeUndefined();
  });

  it("throws when a custom givepoke name label resolves to the label token itself", () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = "CHRIS";
    gameState.sram.player_id = 4321;
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    const species = PokemonSpeciesSchema.parse({
      id: "TOTODILE",
      int_id: 158,
      base_stats: {
        hp: 50,
        attack: 65,
        defense: 64,
        speed: 43,
        special_attack: 44,
        special_defense: 48,
      },
      type1: "WATER",
      type2: "WATER",
      catch_rate: 45,
      base_exp: 66,
      gender_ratio: 31,
      unknown1: 0,
      step_cycles_to_hatch: 20,
      unknown2: 0,
      growth_rate: "GROWTH_MEDIUM_SLOW",
      egg_group1: "EGG_MONSTER",
      egg_group2: "EGG_WATER_1",
    });
    dataLoader.pokemonData = new Map([[species.id, species]]);
    dataLoader.get_text = (label: string) => label;

    const overworld = { data_loader: dataLoader } as unknown as OverworldEngine;
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    const command = new GivePokeCommand("Totodile", 5, null, "TotallyMissingGiftName");
    command.runner = runner;

    expect(() => command.execute(gameState, eventManager, overworld)).toThrow(
      "Missing ASM custom name for label 'TotallyMissingGiftName'."
    );
  });

  it("throws when a custom givepoke name label contains a malformed ASM db token", () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = "CHRIS";
    gameState.sram.player_id = 4321;
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    const species = PokemonSpeciesSchema.parse({
      id: "TOTODILE",
      int_id: 158,
      base_stats: {
        hp: 50,
        attack: 65,
        defense: 64,
        speed: 43,
        special_attack: 44,
        special_defense: 48,
      },
      type1: "WATER",
      type2: "WATER",
      catch_rate: 45,
      base_exp: 66,
      gender_ratio: 31,
      unknown1: 0,
      step_cycles_to_hatch: 20,
      unknown2: 0,
      growth_rate: "GROWTH_MEDIUM_SLOW",
      egg_group1: "EGG_MONSTER",
      egg_group2: "EGG_WATER_1",
    });
    dataLoader.pokemonData = new Map([[species.id, species]]);
    dataLoader.get_script = jest.fn((label: string) => {
      if (label === "GiftSpearowName") {
        return [{ command: "db", args: ["$4G", "0"] }];
      }
      return null;
    });

    const overworld = { data_loader: dataLoader } as unknown as OverworldEngine;
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    const command = new GivePokeCommand("Totodile", 5, null, "GiftSpearowName");
    command.runner = runner;

    expect(() => command.execute(gameState, eventManager, overworld)).toThrow(
      "Invalid ASM custom name token '$4G' in label 'GiftSpearowName'."
    );
  });

  it("throws when givepoke would fabricate a fallback OT name", () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = "";
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    const species = PokemonSpeciesSchema.parse({
      id: "TOTODILE",
      int_id: 158,
      base_stats: {
        hp: 50,
        attack: 65,
        defense: 64,
        speed: 43,
        special_attack: 44,
        special_defense: 48,
      },
      type1: "WATER",
      type2: "WATER",
      catch_rate: 45,
      base_exp: 66,
      gender_ratio: 31,
      unknown1: 0,
      step_cycles_to_hatch: 20,
      unknown2: 0,
      growth_rate: "GROWTH_MEDIUM_SLOW",
      egg_group1: "EGG_MONSTER",
      egg_group2: "EGG_WATER_1",
    });
    dataLoader.pokemonData = new Map([[species.id, species]]);

    const overworld = { data_loader: dataLoader } as unknown as OverworldEngine;
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    const command = new GivePokeCommand("Totodile", 5, null);
    command.runner = runner;

    expect(() => command.execute(gameState, eventManager, overworld)).toThrow(
      "Gift Pokemon requires a non-empty ASM player name."
    );
  });

  it("throws when giveegg would fabricate a fallback OT name", () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = "";
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    const species = PokemonSpeciesSchema.parse({
      id: "TOGEPI",
      int_id: 175,
      base_stats: {
        hp: 35,
        attack: 20,
        defense: 65,
        speed: 20,
        special_attack: 40,
        special_defense: 65,
      },
      type1: "NORMAL",
      type2: "NORMAL",
      catch_rate: 190,
      base_exp: 74,
      gender_ratio: 31,
      unknown1: 0,
      step_cycles_to_hatch: 10,
      unknown2: 0,
      growth_rate: "GROWTH_FAST",
      egg_group1: "EGG_NONE",
      egg_group2: "EGG_NONE",
    });
    dataLoader.pokemonData = new Map([[species.id, species]]);

    const overworld = { data_loader: dataLoader } as unknown as OverworldEngine;
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    const command = new GiveEggCommand("Togepi", "5");
    command.runner = runner;

    expect(() => command.execute(gameState, eventManager, overworld)).toThrow(
      "Gift Pokemon requires a non-empty ASM player name."
    );
  });

  it("initializes a gifted egg with ASM egg state", () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = "CHRIS";
    gameState.sram.player_id = 4321;
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    const species = PokemonSpeciesSchema.parse({
      id: "TOGEPI",
      int_id: 175,
      base_stats: {
        hp: 35,
        attack: 20,
        defense: 65,
        speed: 20,
        special_attack: 40,
        special_defense: 65,
      },
      type1: "NORMAL",
      type2: "NORMAL",
      catch_rate: 190,
      base_exp: 74,
      gender_ratio: 31,
      unknown1: 0,
      step_cycles_to_hatch: 10,
      unknown2: 0,
      growth_rate: "GROWTH_FAST",
      egg_group1: "EGG_NONE",
      egg_group2: "EGG_NONE",
    });
    dataLoader.pokemonData = new Map([[species.id, species]]);

    const overworld = { data_loader: dataLoader } as unknown as OverworldEngine;
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    const command = new GiveEggCommand("Togepi", "5");
    command.runner = runner;
    command.execute(gameState, eventManager, overworld);

    const egg = gameState.sram.party.pokemon[0];
    expect(egg?.nickname).toBe("EGG");
    expect(egg?.happiness).toBe(10);
    expect(egg?.hp).toBe(0);
    expect(egg?.original_trainer_name).toBe("CHRIS");
    expect(egg?.original_trainer_id).toBe(4321);
  });

  it("throws when giveegg receives a malformed ASM level token", () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = "CHRIS";
    gameState.sram.player_id = 4321;
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    const species = PokemonSpeciesSchema.parse({
      id: "TOGEPI",
      int_id: 175,
      base_stats: {
        hp: 35,
        attack: 20,
        defense: 65,
        speed: 20,
        special_attack: 40,
        special_defense: 65,
      },
      type1: "NORMAL",
      type2: "NORMAL",
      catch_rate: 190,
      base_exp: 74,
      gender_ratio: 31,
      unknown1: 0,
      step_cycles_to_hatch: 10,
      unknown2: 0,
      growth_rate: "GROWTH_FAST",
      egg_group1: "EGG_NONE",
      egg_group2: "EGG_NONE",
    });
    dataLoader.pokemonData = new Map([[species.id, species]]);

    const overworld = { data_loader: dataLoader } as unknown as OverworldEngine;
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    const command = new GiveEggCommand("Togepi", "5foo");
    command.runner = runner;

    expect(() => command.execute(gameState, eventManager, overworld)).toThrow(
      "Invalid giveegg level '5foo'."
    );
  });

  it("uses the ASM two-step pre-evolution species when creating a gifted egg", () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = "CHRIS";
    gameState.sram.player_id = 4321;
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    const pichu = PokemonSpeciesSchema.parse({
      id: "PICHU",
      int_id: 172,
      base_stats: {
        hp: 20,
        attack: 40,
        defense: 15,
        speed: 60,
        special_attack: 35,
        special_defense: 35,
      },
      type1: "ELECTRIC",
      type2: "ELECTRIC",
      catch_rate: 190,
      base_exp: 42,
      gender_ratio: 31,
      unknown1: 0,
      step_cycles_to_hatch: 11,
      unknown2: 0,
      growth_rate: "GROWTH_MEDIUM_FAST",
      egg_group1: "EGG_NONE",
      egg_group2: "EGG_NONE",
    });
    const pikachu = PokemonSpeciesSchema.parse({
      id: "PIKACHU",
      int_id: 25,
      base_stats: {
        hp: 35,
        attack: 55,
        defense: 30,
        speed: 90,
        special_attack: 50,
        special_defense: 40,
      },
      type1: "ELECTRIC",
      type2: "ELECTRIC",
      catch_rate: 190,
      base_exp: 82,
      gender_ratio: 31,
      unknown1: 0,
      step_cycles_to_hatch: 12,
      unknown2: 0,
      growth_rate: "GROWTH_MEDIUM_FAST",
      egg_group1: "EGG_GROUND",
      egg_group2: "EGG_FAIRY",
    });
    const raichu = PokemonSpeciesSchema.parse({
      id: "RAICHU",
      int_id: 26,
      base_stats: {
        hp: 60,
        attack: 90,
        defense: 55,
        speed: 110,
        special_attack: 90,
        special_defense: 80,
      },
      type1: "ELECTRIC",
      type2: "ELECTRIC",
      catch_rate: 75,
      base_exp: 122,
      gender_ratio: 31,
      unknown1: 0,
      step_cycles_to_hatch: 13,
      unknown2: 0,
      growth_rate: "GROWTH_MEDIUM_FAST",
      egg_group1: "EGG_GROUND",
      egg_group2: "EGG_FAIRY",
    });
    dataLoader.pokemonData = new Map([
      [pichu.id, pichu],
      [pikachu.id, pikachu],
      [raichu.id, raichu],
    ]);

    const overworld = { data_loader: dataLoader } as unknown as OverworldEngine;
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    const command = new GiveEggCommand("Raichu", "5");
    command.runner = runner;
    command.execute(gameState, eventManager, overworld);

    const egg = gameState.sram.party.pokemon[0];
    expect(egg?.species.id).toBe("PICHU");
    expect(egg?.nickname).toBe("EGG");
    expect(egg?.happiness).toBe(11);
    expect(egg?.hp).toBe(0);
  });

  it("assigns the ASM gift OT id for a custom-OT party gift mon", () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = "CHRIS";
    gameState.sram.player_id = 4321;
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    const species = PokemonSpeciesSchema.parse({
      id: "TOTODILE",
      int_id: 158,
      base_stats: {
        hp: 50,
        attack: 65,
        defense: 64,
        speed: 43,
        special_attack: 44,
        special_defense: 48,
      },
      type1: "WATER",
      type2: "WATER",
      catch_rate: 45,
      base_exp: 66,
      gender_ratio: 31,
      unknown1: 0,
      step_cycles_to_hatch: 20,
      unknown2: 0,
      growth_rate: "GROWTH_MEDIUM_SLOW",
      egg_group1: "EGG_MONSTER",
      egg_group2: "EGG_WATER_1",
    });
    dataLoader.pokemonData = new Map([[species.id, species]]);
    dataLoader.get_text = (label: string) => {
      if (label === "GiftTrainerOt") {
        return "KIRK@";
      }
      return label;
    };

    const overworld = { data_loader: dataLoader } as unknown as OverworldEngine;
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    const command = new GivePokeCommand("Totodile", 5, null, null, "GiftTrainerOt");
    command.runner = runner;
    command.execute(gameState, eventManager, overworld);

    const mon = gameState.sram.party.pokemon[0];
    expect(mon?.original_trainer_name).toBe("KIRK");
    expect(mon?.original_trainer_id).toBe(1001);
  });

  it("skips the nickname prompt for custom gift metadata and keeps the scripted nickname", () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = "CHRIS";
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    const species = PokemonSpeciesSchema.parse({
      id: "TOTODILE",
      int_id: 158,
      base_stats: {
        hp: 50,
        attack: 65,
        defense: 64,
        speed: 43,
        special_attack: 44,
        special_defense: 48,
      },
      type1: "WATER",
      type2: "WATER",
      catch_rate: 45,
      base_exp: 66,
      gender_ratio: 31,
      unknown1: 0,
      step_cycles_to_hatch: 20,
      unknown2: 0,
      growth_rate: "GROWTH_MEDIUM_SLOW",
      egg_group1: "EGG_MONSTER",
      egg_group2: "EGG_WATER_1",
    });
    dataLoader.pokemonData = new Map([[species.id, species]]);
    dataLoader.get_text = (label: string) => {
      if (label === "GiftNickname") {
        return "ODDISH@";
      }
      if (label === "GiftTrainerOt") {
        return "KIRK@";
      }
      return label;
    };

    const overworld = { data_loader: dataLoader } as unknown as OverworldEngine;
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    runner.variables._givepoke_nickname_choice = [0];
    runner.variables._givepoke_nickname_value = "BUBBLES";

    const command = new GivePokeCommand("Totodile", 5, null, "GiftNickname", "GiftTrainerOt");
    command.runner = runner;
    command.execute(gameState, eventManager, overworld);

    const mon = gameState.sram.party.pokemon[0];
    expect(mon?.nickname).toBe("ODDISH");
    expect(mon?.original_trainer_name).toBe("KIRK");
    expect(runner.variables._givepoke_nickname_choice).toBeUndefined();
    expect(runner.variables._givepoke_nickname_value).toBeUndefined();
  });

  it("does not keep the player's OT id when a custom-OT gift mon is sent to BILL's PC", () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = "CHRIS";
    gameState.sram.player_id = 4321;
    gameState.sram.party.pokemon = [
      createTestPokemon("TOTODILE", 10),
      createTestPokemon("TOTODILE", 11),
      createTestPokemon("TOTODILE", 12),
      createTestPokemon("TOTODILE", 13),
      createTestPokemon("TOTODILE", 14),
      createTestPokemon("TOTODILE", 15),
    ];
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    const species = PokemonSpeciesSchema.parse({
      id: "TOTODILE",
      int_id: 158,
      base_stats: {
        hp: 50,
        attack: 65,
        defense: 64,
        speed: 43,
        special_attack: 44,
        special_defense: 48,
      },
      type1: "WATER",
      type2: "WATER",
      catch_rate: 45,
      base_exp: 66,
      gender_ratio: 31,
      unknown1: 0,
      step_cycles_to_hatch: 20,
      unknown2: 0,
      growth_rate: "GROWTH_MEDIUM_SLOW",
      egg_group1: "EGG_MONSTER",
      egg_group2: "EGG_WATER_1",
    });
    dataLoader.pokemonData = new Map([[species.id, species]]);
    dataLoader.get_text = (label: string) => {
      if (label === "GiftTrainerOt") {
        return "KIRK@";
      }
      return label;
    };

    const overworld = { data_loader: dataLoader } as unknown as OverworldEngine;
    const runner = {
      data_loader: dataLoader,
      overworld,
      variables: {},
    } as unknown as ScriptRunner;
    const command = new GivePokeCommand("Totodile", 5, null, null, "GiftTrainerOt");
    command.runner = runner;
    command.execute(gameState, eventManager, overworld);

    const boxMon = gameState.sram.pc_boxes[0]?.pokemon[0];
    expect(boxMon?.original_trainer_name).toBe("KIRK");
    expect(boxMon?.original_trainer_id).not.toBe(gameState.sram.player_id);
  });

  it("queues the naming screen and clears dialogue waits during givepoke", async () => {
    const restoreAssets = installNamingScreenAssets();
    const nextFrameMock = jest.spyOn(asyncLoop, "nextFrame").mockResolvedValue(undefined);
    try {
      const gameState = createInitialGameState();
      gameState.sram.player_name = "CHRIS";
      gameState.sram.player_id = 4321;
      const eventManager = new EventManager(gameState);
      const dataLoader = new DataLoader();
      const species = PokemonSpeciesSchema.parse({
        id: "TOTODILE",
        int_id: 158,
        base_stats: {
          hp: 50,
          attack: 65,
          defense: 64,
          speed: 43,
          special_attack: 44,
          special_defense: 48,
        },
        type1: "WATER",
        type2: "WATER",
        catch_rate: 45,
        base_exp: 66,
        gender_ratio: 31,
        unknown1: 0,
        step_cycles_to_hatch: 20,
        unknown2: 0,
        growth_rate: "GROWTH_MEDIUM_SLOW",
        egg_group1: "EGG_MONSTER",
        egg_group2: "EGG_WATER_1",
      });
      dataLoader.pokemonData = new Map([[species.id, species]]);

      const ui = {
        screen: new gameEngine.Surface(160, 144),
        clearScreen: jest.fn(),
        update: jest.fn(),
        eventQueue: gameEngine.event.createQueue(),
      };
      const dialogue = {
        clear_script_waits: jest.fn(),
        forceCloseText: jest.fn(),
      };
      const overworld = {
        ui,
        input_capture_active: false,
        dialogue,
        data_loader: dataLoader,
      } as unknown as OverworldEngine;
      const queueTask = jest.fn();
      const runner = {
        _queue_overworld_task: queueTask,
        variables: { _givepoke_nickname_choice: [0] },
        data_loader: dataLoader,
        overworld,
      } as unknown as ScriptRunner;
      const command = new GivePokeCommand("Totodile", 5, null);
      command.runner = runner;

      command.execute(gameState, eventManager, overworld);

      expect(queueTask).toHaveBeenCalledTimes(1);
      const scheduler = queueTask.mock.calls[0]?.[0] as (callback: () => void) => boolean;
      const done = jest.fn();
      gameEngine.event.post(
        new gameEngine.event.Event(gameEngine.KEYDOWN, {
          key: "Enter",
          code: "Enter",
          button: "start",
          is_press: true,
        }),
        ui.eventQueue
      );
      gameEngine.event.post(
        new gameEngine.event.Event(gameEngine.KEYDOWN, {
          key: "KeyZ",
          code: "KeyZ",
          button: "a",
          is_press: true,
        }),
        ui.eventQueue
      );
      scheduler(done);

      await new Promise((resolve) => setTimeout(resolve, 0));
      await new Promise((resolve) => setTimeout(resolve, 0));

      expect(dialogue.clear_script_waits).toHaveBeenCalledTimes(1);
      expect(dialogue.forceCloseText).toHaveBeenCalledTimes(1);
      expect(done).toHaveBeenCalledTimes(1);
      expect((runner.last_value as any)?.givepoke?.nickname).toBe("TOTODILE");
      expect(overworld.input_capture_active).toBe(false);
      expect(nextFrameMock).toHaveBeenCalled();
    } finally {
      nextFrameMock.mockRestore();
      restoreAssets();
    }
  });
});

describe("startbattle canlose handling", () => {
  it("falls back to trainer base reward when trainer id is missing", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const dataLoader = {
      get_trainer_base_reward: jest.fn().mockReturnValue(0),
    } as unknown as DataLoader;
    const overworld = { data_loader: dataLoader } as OverworldEngine;
    const playerMon = createTestPokemon("TOTODILE", 10, { level: 10, hp: 10, max_hp: 10 });
    const enemyMon = createTestPokemon("PIDGEY", 11, { level: 9, hp: 10, max_hp: 10 });
    gameState.sram.party.pokemon = [playerMon, null, null, null, null, null];

    const trainer = TrainerSchema.parse({
      name: "FALKNER",
      trainer_id: "FALKNER1",
      trainer_class: "FALKNER",
      party: [enemyMon],
      win_quote: "",
      lose_quote: "",
      base_reward: 25,
    });

    const runner = {
      loaded_trainer: trainer,
      loaded_trainer_id: null,
      just_battled: false,
      pause: jest.fn(),
      resume: jest.fn(),
      last_condition_result: false,
      last_value: null,
      pending_reload_map: null,
      variables: {},
      data_loader: dataLoader,
    } as unknown as ScriptRunner;
    const command = new StartBattleCommand();
    command.runner = runner;

    let payload: Record<string, unknown> | null = null;
    eventManager.on("start_battle", (event) => {
      payload = event.data;
    });

    command.execute(gameState, eventManager, overworld);

    expect(payload?.trainer_reward).toBe(225);
  });

  it("hydrates trainer party moves and PP before starting the battle", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const dataLoader = {} as DataLoader;
    const overworld = { data_loader: dataLoader } as OverworldEngine;
    const playerMon = createTestPokemon("TOTODILE", 10, { hp: 10, max_hp: 10 });
    const enemyMon = createTestPokemon("CYNDAQUIL", 11, {
      level: 5,
      hp: 10,
      max_hp: 10,
      moves: [],
    });
    gameState.sram.party.pokemon = [playerMon, null, null, null, null, null];

    const trainer = TrainerSchema.parse({
      name: "RIVAL",
      trainer_id: "RIVAL1_1_CYNDAQUIL",
      trainer_class: "RIVAL1",
      party: [enemyMon],
      win_quote: "",
      lose_quote: "",
      base_reward: 25,
    });

    const runner = {
      loaded_trainer: trainer,
      loaded_trainer_id: trainer.trainer_id,
      just_battled: false,
      pause: jest.fn(),
      resume: jest.fn(),
      last_condition_result: false,
      last_value: null,
      pending_reload_map: null,
      variables: {},
      data_loader: dataLoader,
    } as unknown as ScriptRunner;
    const command = new StartBattleCommand();
    command.runner = runner;

    let payload: Record<string, unknown> | null = null;
    eventManager.on("start_battle", (event) => {
      payload = event.data;
    });

    command.execute(gameState, eventManager, overworld);

    const enemyParty = payload?.enemy_party as Pokemon[] | undefined;
    expect(enemyParty?.[0]?.moves.length).toBeGreaterThan(0);
    expect(enemyParty?.[0]?.moves.every((move) => move.current_pp > 0)).toBe(true);
    expect((runner.loaded_trainer as Trainer).party[0]?.moves.length).toBeGreaterThan(0);
    expect((runner.loaded_trainer as Trainer).party[0]?.moves.every((move) => move.current_pp > 0)).toBe(true);
  });

  it("resumes scripts after a loss when battle type allows losing", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const overworld = { data_loader: {} } as OverworldEngine;
    const playerMon = createTestPokemon("TOTODILE", 10, { hp: 10, max_hp: 10 });
    const enemyMon = createTestPokemon("CHIKORITA", 11, { hp: 10, max_hp: 10 });
    gameState.sram.party.pokemon = [playerMon, null, null, null, null, null];
    gameState.wram.battle_type = "BATTLETYPE_CANLOSE";

    const pause = jest.fn();
    const resume = jest.fn();
    const stopAllScripts = jest.fn();
    const runner = {
      loaded_trainer: { name: "RIVAL", party: [enemyMon] },
      loaded_trainer_id: null,
      just_battled: false,
      pause,
      resume,
      stop_all_scripts: stopAllScripts,
      last_condition_result: false,
      last_value: null,
      pending_reload_map: null,
      variables: {},
      data_loader: {},
    } as unknown as ScriptRunner;

    const command = new StartBattleCommand();
    command.runner = runner;
    command.execute(gameState, eventManager, overworld);

    eventManager.dispatch(new Event("battle_complete", { result: 1 }));

    expect(stopAllScripts).not.toHaveBeenCalled();
    expect(resume).toHaveBeenCalledTimes(1);
    expect(runner.last_condition_result).toBe(true);
    expect(runner.last_value).toBe(1);
    expect(pause).toHaveBeenCalledTimes(1);
  });

  it("throws when battle_complete reports a malformed result instead of defaulting to win", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const overworld = { data_loader: {} } as OverworldEngine;
    const playerMon = createTestPokemon("TOTODILE", 10, { hp: 10, max_hp: 10 });
    const enemyMon = createTestPokemon("CHIKORITA", 11, { hp: 10, max_hp: 10 });
    gameState.sram.party.pokemon = [playerMon, null, null, null, null, null];

    const runner = {
      loaded_trainer: { name: "RIVAL", party: [enemyMon] },
      loaded_trainer_id: null,
      just_battled: false,
      pause: jest.fn(),
      resume: jest.fn(),
      stop_all_scripts: jest.fn(),
      last_condition_result: false,
      last_value: null,
      pending_reload_map: null,
      variables: {},
      data_loader: {},
    } as unknown as ScriptRunner;

    const command = new StartBattleCommand();
    command.runner = runner;
    command.execute(gameState, eventManager, overworld);

    const consoleSpy = jest.spyOn(console, "error").mockImplementation(() => {});
    try {
      expect(() => eventManager.dispatch(new Event("battle_complete", { result: "not_a_number" }))).not.toThrow();
      expect(consoleSpy).toHaveBeenCalled();
    } finally {
      consoleSpy.mockRestore();
    }
  });

  it("does not retain pending reload-map state on an ordinary loss that should white out", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const overworld = { data_loader: {}, current_map_name: "ROUTE_29" } as OverworldEngine;
    const playerMon = createTestPokemon("TOTODILE", 10, { hp: 10, max_hp: 10 });
    const enemyMon = createTestPokemon("CHIKORITA", 11, { hp: 10, max_hp: 10 });
    gameState.sram.party.pokemon = [playerMon, null, null, null, null, null];
    gameState.wram.reload_map_after_battle = true;

    const stopAllScripts = jest.fn();
    const runner = {
      loaded_trainer: { name: "RIVAL", party: [enemyMon] },
      loaded_trainer_id: null,
      just_battled: false,
      pause: jest.fn(),
      resume: jest.fn(),
      stop_all_scripts: stopAllScripts,
      last_condition_result: false,
      last_value: null,
      pending_reload_map: null,
      variables: {},
      data_loader: {},
    } as unknown as ScriptRunner;

    const command = new StartBattleCommand();
    command.runner = runner;
    command.execute(gameState, eventManager, overworld);

    eventManager.dispatch(new Event("battle_complete", { result: 1 }));

    expect(stopAllScripts).toHaveBeenCalledTimes(1);
    expect(runner.pending_reload_map).toBeNull();
    expect(gameState.wram.reload_map_after_battle).toBe(false);
    expect(runner.just_battled).toBe(false);
  });

  it("throws when a wild battle is started with a malformed level instead of coercing it", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const species = PokemonSpeciesSchema.parse({
      id: "RATTATA",
      int_id: 19,
      base_stats: {
        hp: 30,
        attack: 56,
        defense: 35,
        speed: 72,
        special_attack: 25,
        special_defense: 35,
      },
      type1: "NORMAL",
      type2: "NORMAL",
      catch_rate: 255,
      base_exp: 57,
      gender_ratio: 127,
      unknown1: 0,
      step_cycles_to_hatch: 15,
      unknown2: 0,
      growth_rate: "GROWTH_MEDIUM_FAST",
      egg_group1: "EGG_GROUND",
      egg_group2: "EGG_GROUND",
    });
    const dataLoader = new DataLoader();
    dataLoader.pokemonData = new Map([[species.id, species]]);
    gameState.sram.party.pokemon = [createTestPokemon("TOTODILE", 10, { hp: 10, max_hp: 10 }), null, null, null, null, null];
    gameState.wram.wild_pokemon = { species: "RATTATA", level: "not_a_number" } as never;
    const overworld = { data_loader: dataLoader } as OverworldEngine;
    const runner = {
      loaded_trainer: null,
      loaded_trainer_id: null,
      just_battled: false,
      pause: jest.fn(),
      resume: jest.fn(),
      stop_all_scripts: jest.fn(),
      last_condition_result: false,
      last_value: null,
      pending_reload_map: null,
      variables: {},
      data_loader: dataLoader,
    } as unknown as ScriptRunner;

    const command = new StartBattleCommand();
    command.runner = runner;

    expect(() => command.execute(gameState, eventManager, overworld)).toThrow(
      "Invalid wild level 'not_a_number' supplied to StartBattleCommand"
    );
  });

  it("loads forced-shiny wild battles with ASM shiny DVs", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const species = PokemonSpeciesSchema.parse({
      id: "GYARADOS",
      int_id: 130,
      base_stats: {
        hp: 95,
        attack: 125,
        defense: 79,
        speed: 81,
        special_attack: 60,
        special_defense: 100,
      },
      type1: "WATER",
      type2: "FLYING",
      catch_rate: 45,
      base_exp: 214,
      gender_ratio: 127,
      unknown1: 0,
      step_cycles_to_hatch: 5,
      unknown2: 0,
      growth_rate: "GROWTH_SLOW",
      egg_group1: "EGG_WATER_2",
      egg_group2: "EGG_DRAGON",
    });
    const dataLoader = new DataLoader();
    dataLoader.pokemonData = new Map([[species.id, species]]);
    gameState.sram.party.pokemon = [
      createTestPokemon("TOTODILE", 10, { hp: 10, max_hp: 10 }),
      null,
      null,
      null,
      null,
      null,
    ];
    gameState.wram.wild_pokemon = { species: "GYARADOS", level: 30 } as never;
    gameState.wram.battle_type = "BATTLETYPE_FORCESHINY";
    const overworld = { data_loader: dataLoader } as OverworldEngine;
    const runner = {
      loaded_trainer: null,
      loaded_trainer_id: null,
      just_battled: false,
      pause: jest.fn(),
      resume: jest.fn(),
      stop_all_scripts: jest.fn(),
      last_condition_result: false,
      last_value: null,
      pending_reload_map: null,
      variables: {},
      data_loader: dataLoader,
    } as unknown as ScriptRunner;

    const command = new StartBattleCommand();
    command.runner = runner;
    let enemyPokemon: Pokemon | null = null;
    eventManager.on("start_battle", (event) => {
      enemyPokemon = event.data.enemy_pokemon as Pokemon;
    });

    command.execute(gameState, eventManager, overworld);

    expect(enemyPokemon?.dvs).toEqual({
      attack: 14,
      defense: 10,
      speed: 10,
      special: 10,
      hp: 0,
    });
  });

  it("loads Tin Tower Ho-Oh with its forced Sacred Ash held item and level-up moves", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const species = PokemonSpeciesSchema.parse({
      id: "HO_OH",
      int_id: 250,
      base_stats: {
        hp: 106,
        attack: 130,
        defense: 90,
        speed: 90,
        special_attack: 110,
        special_defense: 154,
      },
      type1: "FIRE",
      type2: "FLYING",
      catch_rate: 3,
      base_exp: 220,
      item1: "SACRED_ASH",
      item2: "SACRED_ASH",
      gender_ratio: 255,
      unknown1: 0,
      step_cycles_to_hatch: 120,
      unknown2: 0,
      growth_rate: "GROWTH_SLOW",
      egg_group1: "EGG_NONE",
      egg_group2: "EGG_NONE",
      tmhm_learnset: [],
      ability: "NONE",
      pic_size: 0,
      front_pic: 0,
      back_pic: 0,
      weight: 1990,
    });
    const dataLoader = new DataLoader();
    dataLoader.pokemonData = new Map([[species.id, species]]);
    gameState.sram.party.pokemon = [
      createTestPokemon("TOTODILE", 50, { hp: 100, max_hp: 100 }),
      null,
      null,
      null,
      null,
      null,
    ];
    gameState.wram.wild_pokemon = { species: "HO_OH", level: 60 } as never;
    gameState.wram.battle_type = "BATTLETYPE_FORCEITEM";
    const overworld = { data_loader: dataLoader } as OverworldEngine;
    const runner = {
      loaded_trainer: null,
      loaded_trainer_id: null,
      just_battled: false,
      pause: jest.fn(),
      resume: jest.fn(),
      stop_all_scripts: jest.fn(),
      last_condition_result: false,
      last_value: null,
      pending_reload_map: null,
      variables: {},
      data_loader: dataLoader,
    } as unknown as ScriptRunner;

    const command = new StartBattleCommand();
    command.runner = runner;
    let enemyPokemon: Pokemon | null = null;
    eventManager.on("start_battle", (event) => {
      enemyPokemon = event.data.enemy_pokemon as Pokemon;
    });

    command.execute(gameState, eventManager, overworld);

    expect(enemyPokemon?.species.id).toBe("HO_OH");
    expect(enemyPokemon?.level).toBe(60);
    expect(enemyPokemon?.item).toBe("SACRED_ASH");
    expect(enemyPokemon?.moves.map((move) => move.name)).toEqual([
      "GUST",
      "RECOVER",
      "FIRE_BLAST",
      "SUNNY_DAY",
    ]);
  });

  it("throws when loaded trainer party data is missing instead of falling back to stale wild data", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const species = PokemonSpeciesSchema.parse({
      id: "RATTATA",
      int_id: 19,
      base_stats: {
        hp: 30,
        attack: 56,
        defense: 35,
        speed: 72,
        special_attack: 25,
        special_defense: 35,
      },
      type1: "NORMAL",
      type2: "NORMAL",
      catch_rate: 255,
      base_exp: 57,
      gender_ratio: 127,
      unknown1: 0,
      step_cycles_to_hatch: 15,
      unknown2: 0,
      growth_rate: "GROWTH_MEDIUM_FAST",
      egg_group1: "EGG_GROUND",
      egg_group2: "EGG_GROUND",
    });
    const dataLoader = new DataLoader();
    dataLoader.pokemonData = new Map([[species.id, species]]);
    gameState.sram.party.pokemon = [
      createTestPokemon("TOTODILE", 10, { hp: 10, max_hp: 10 }),
      null,
      null,
      null,
      null,
      null,
    ];
    gameState.wram.wild_pokemon = { species: "RATTATA", level: 3 } as never;
    const overworld = { data_loader: dataLoader } as OverworldEngine;
    const runner = {
      loaded_trainer: { name: "BROKEN_TRAINER", party: [] },
      loaded_trainer_id: "BROKEN_TRAINER",
      just_battled: false,
      pause: jest.fn(),
      resume: jest.fn(),
      stop_all_scripts: jest.fn(),
      last_condition_result: false,
      last_value: null,
      pending_reload_map: null,
      variables: {},
      data_loader: dataLoader,
    } as unknown as ScriptRunner;

    const command = new StartBattleCommand();
    command.runner = runner;

    expect(() => command.execute(gameState, eventManager, overworld)).toThrow(
      "Loaded trainer is missing party data."
    );
  });
});

describe("trainer battle text parity", () => {
  it("waits on trainer seen text before continuing into battle", () => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon = [createTestPokemon("TOTODILE", 10, { hp: 10, max_hp: 10 }), null, null, null, null, null];
    const eventManager = new EventManager(gameState);
    const dataLoader = {
      get_text: jest.fn().mockImplementation((label: string) =>
        label === "BattleSeenText" ? "I saw you!" : ""
      ),
    } as unknown as DataLoader;
    const overworld = { data_loader: dataLoader, dataLoader } as unknown as OverworldEngine;
    const runner = {
      _script_stack: [{ name: "TrainerBattleScript", commands: [], index: 0, allowFallthrough: false }],
      data_loader: dataLoader,
      dataLoader,
      last_value: null,
      last_condition_result: false,
      variables: {},
    } as unknown as ScriptRunner;

    const command = new TrainerCommand("LASS", "DANA1", "", "BattleSeenText", "0", "0", "");
    command.runner = runner;
    command.execute(gameState, eventManager, overworld);

    const textFrame = (runner as unknown as { _script_stack: Array<{ name: string; commands: unknown[] }> })
      ._script_stack.find((frame) => frame.name.endsWith("#trainer_text"));

    expect(textFrame?.commands.map((entry) => entry?.constructor?.name)).toEqual([
      "OpenTextCommand",
      "ResolvedBattleTextCommand",
      "WaitButtonCommand",
      "CloseTextCommand",
    ]);
  });

  it("plays the ASM trainer encounter music before showing seen text", () => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon = [createTestPokemon("TOTODILE", 10, { hp: 10, max_hp: 10 }), null, null, null, null, null];
    const eventManager = new EventManager(gameState);
    const audioEngine = {
      playMusic: jest.fn(),
    } as unknown as AudioEngine;
    const dataLoader = {
      get_text: jest.fn().mockImplementation((label: string) =>
        label === "BattleSeenText" ? "I saw you!" : ""
      ),
    } as unknown as DataLoader;
    const overworld = {
      data_loader: dataLoader,
      dataLoader,
      audio_engine: audioEngine,
    } as unknown as OverworldEngine;
    const runner = {
      _script_stack: [],
      data_loader: dataLoader,
      dataLoader,
      last_value: null,
      last_condition_result: false,
      variables: {},
    } as unknown as ScriptRunner;

    const command = new TrainerCommand("LASS", "DANA1", "", "BattleSeenText", "0", "0", "");
    command.runner = runner;
    command.execute(gameState, eventManager, overworld);

    expect(audioEngine.playMusic).toHaveBeenCalledWith("MUSIC_LASS_ENCOUNTER", "encounter");
  });

  it("throws when seen text resolves to the raw label token", () => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon = [createTestPokemon("TOTODILE", 10, { hp: 10, max_hp: 10 }), null, null, null, null, null];
    const eventManager = new EventManager(gameState);
    const dataLoader = {
      get_text: (label: string) => label,
    } as unknown as DataLoader;
    const overworld = { data_loader: dataLoader, dataLoader } as unknown as OverworldEngine;
    const runner = {
      _script_stack: [],
      data_loader: dataLoader,
      dataLoader,
      last_value: null,
      last_condition_result: false,
      variables: {},
    } as unknown as ScriptRunner;

    const command = new TrainerCommand("LASS", "DANA1", "", "BattleSeenText", "0", "0", "");
    command.runner = runner;

    const resolveTextMock = mockResolvedBattleText({
      BattleSeenText: "BattleSeenText",
    });
    try {
      expect(() => command.execute(gameState, eventManager, overworld)).toThrow(
        "Missing ASM battle text for label 'BattleSeenText'."
      );
    } finally {
      resolveTextMock.mockRestore();
    }
  });

  it("throws when win text resolves to the raw label token after battle", () => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon = [createTestPokemon("TOTODILE", 10, { hp: 10, max_hp: 10 }), null, null, null, null, null];
    const eventManager = new EventManager(gameState);
    const enemyMon = createTestPokemon("PIDGEY", 11, { level: 7, hp: 10, max_hp: 10 });
    const trainer = TrainerSchema.parse({
      name: "DANA",
      trainer_id: "DANA1",
      trainer_class: "LASS",
      party: [enemyMon],
      win_quote: "",
      lose_quote: "",
      base_reward: 16,
    });
    const dataLoader = {
      get_text: (label: string) => label,
      get_trainer: jest.fn().mockReturnValue(trainer),
    } as unknown as DataLoader;
    const overworld = { data_loader: dataLoader, dataLoader } as unknown as OverworldEngine;
    const runner = {
      _script_stack: [{ name: "TrainerBattleScript", commands: [], index: 0, allowFallthrough: false }],
      data_loader: dataLoader,
      dataLoader,
      loaded_trainer: null,
      loaded_trainer_id: null,
      last_value: null,
      last_condition_result: false,
      variables: {},
      set_event_flag: jest.fn(),
      defer: jest.fn(),
      jump: jest.fn(),
    } as unknown as ScriptRunner;

    const command = new TrainerCommand("LASS", "DANA1", "EVENT_BEAT_DANA", "0", "BattleWinText", "0", "");
    command.runner = runner;
    const resolveTextMock = mockResolvedBattleText({
      BattleWinText: "BattleWinText",
    });
    command.execute(gameState, eventManager, overworld);

    const consoleSpy = jest.spyOn(console, "error").mockImplementation(() => {});
    try {
      expect(() => eventManager.dispatch(new Event("battle_complete", { result: 0 }))).not.toThrow();
      expect(consoleSpy).toHaveBeenCalledTimes(1);
      const [message, error] = consoleSpy.mock.calls[0] ?? [];
      expect(message).toBe("Error in event listener for battle_complete:");
      expect(error).toBeInstanceOf(Error);
      expect((error as Error).message).toBe("Missing ASM battle text for label 'BattleWinText'.");
    } finally {
      consoleSpy.mockRestore();
      resolveTextMock.mockRestore();
    }
  });

  it("throws when trainer battle completion reports a malformed result instead of treating it as a win", () => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon = [createTestPokemon("TOTODILE", 10, { hp: 10, max_hp: 10 }), null, null, null, null, null];
    const eventManager = new EventManager(gameState);
    const enemyMon = createTestPokemon("PIDGEY", 11, { level: 7, hp: 10, max_hp: 10 });
    const trainer = TrainerSchema.parse({
      name: "DANA",
      trainer_id: "DANA1",
      trainer_class: "LASS",
      party: [enemyMon],
      win_quote: "",
      lose_quote: "",
      base_reward: 16,
    });
    const dataLoader = {
      get_text: jest.fn().mockReturnValue("Battle text"),
      get_trainer: jest.fn().mockReturnValue(trainer),
    } as unknown as DataLoader;
    const overworld = { data_loader: dataLoader } as unknown as OverworldEngine;
    const runner = {
      _script_stack: [],
      data_loader: dataLoader,
      loaded_trainer: null,
      loaded_trainer_id: null,
      last_value: null,
      last_condition_result: false,
      variables: {},
      set_event_flag: jest.fn(),
      defer: jest.fn(),
      jump: jest.fn(),
    } as unknown as ScriptRunner;

    const command = new TrainerCommand("LASS", "DANA1", "EVENT_BEAT_DANA", "0", "BattleWinText", "0", "");
    command.runner = runner;
    command.execute(gameState, eventManager, overworld);

    const consoleSpy = jest.spyOn(console, "error").mockImplementation(() => {});
    try {
      expect(() => eventManager.dispatch(new Event("battle_complete", { result: "oops" }))).not.toThrow();
      expect(consoleSpy).toHaveBeenCalled();
    } finally {
      consoleSpy.mockRestore();
    }
  });

  it("treats can-lose trainer defeats as completed trainer battles for flag and just-battled bookkeeping", () => {
    const gameState = createInitialGameState();
    gameState.wram.battle_type = "BATTLETYPE_CANLOSE";
    gameState.sram.party.pokemon = [createTestPokemon("TOTODILE", 10, { hp: 10, max_hp: 10 }), null, null, null, null, null];
    const eventManager = new EventManager(gameState);
    const enemyMon = createTestPokemon("PIDGEY", 11, { level: 7, hp: 10, max_hp: 10 });
    const trainer = TrainerSchema.parse({
      name: "DANA",
      trainer_id: "DANA1",
      trainer_class: "LASS",
      party: [enemyMon],
      win_quote: "",
      lose_quote: "",
      base_reward: 16,
    });
    const dataLoader = {
      get_text: jest.fn().mockReturnValue("Battle text"),
      get_trainer: jest.fn().mockReturnValue(trainer),
    } as unknown as DataLoader;
    const overworld = { data_loader: dataLoader, dataLoader } as unknown as OverworldEngine;
    const setEventFlag = jest.fn();
    const runner = {
      _script_stack: [],
      data_loader: dataLoader,
      dataLoader,
      loaded_trainer: null,
      loaded_trainer_id: null,
      just_battled: false,
      last_value: null,
      last_condition_result: false,
      variables: {},
      set_event_flag: setEventFlag,
      defer: jest.fn(),
      jump: jest.fn(),
    } as unknown as ScriptRunner;

    const command = new TrainerCommand("LASS", "DANA1", "EVENT_BEAT_DANA", "0", "BattleWinText", "BattleLossText", "");
    command.runner = runner;
    command.execute(gameState, eventManager, overworld);

    eventManager.dispatch(new Event("battle_complete", { result: 1 }));

    expect(setEventFlag).toHaveBeenCalledWith("EVENT_BEAT_DANA", true);
    expect(gameState.wram.wRunningTrainerBattleScript).toBe(-1);
    expect(runner.just_battled).toBe(true);
  });

  it("does not mark the trainer battle script as continued on an ordinary trainer loss", () => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon = [createTestPokemon("TOTODILE", 10, { hp: 10, max_hp: 10 }), null, null, null, null, null];
    const eventManager = new EventManager(gameState);
    const eventNames: string[] = [];
    for (const name of ["open_text", "show_text", "wait_for_input"]) {
      eventManager.on(name, (event) => eventNames.push(event.name));
    }
    const enemyMon = createTestPokemon("PIDGEY", 11, { level: 7, hp: 10, max_hp: 10 });
    const trainer = TrainerSchema.parse({
      name: "DANA",
      trainer_id: "DANA1",
      trainer_class: "LASS",
      party: [enemyMon],
      win_quote: "",
      lose_quote: "",
      base_reward: 16,
    });
    const dataLoader = {
      get_text: jest.fn().mockReturnValue("Battle text"),
      get_trainer: jest.fn().mockReturnValue(trainer),
    } as unknown as DataLoader;
    const overworld = { data_loader: dataLoader, dataLoader } as unknown as OverworldEngine;
    const setEventFlag = jest.fn();
    const runner = {
      _script_stack: [],
      data_loader: dataLoader,
      dataLoader,
      loaded_trainer: null,
      loaded_trainer_id: null,
      just_battled: false,
      last_value: null,
      last_condition_result: false,
      variables: {},
      set_event_flag: setEventFlag,
      defer: jest.fn(),
      jump: jest.fn(),
    } as unknown as ScriptRunner;

    const command = new TrainerCommand("LASS", "DANA1", "EVENT_BEAT_DANA", "0", "BattleWinText", "BattleLossText", "");
    command.runner = runner;
    command.execute(gameState, eventManager, overworld);

    eventManager.dispatch(new Event("battle_complete", { result: 1 }));

    expect(setEventFlag).not.toHaveBeenCalled();
    expect(gameState.wram.wRunningTrainerBattleScript).not.toBe(-1);
    expect(runner.just_battled).toBe(false);
    expect(eventNames).toEqual([]);
    expect(runner._script_stack ?? []).toHaveLength(0);
  });

  it("queues trainer after-battle text onto the resumed script stack instead of firing dialogue events during battle_complete", () => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon = [createTestPokemon("TOTODILE", 10, { hp: 10, max_hp: 10 }), null, null, null, null, null];
    const eventManager = new EventManager(gameState);
    const enemyMon = createTestPokemon("PIDGEY", 11, { level: 7, hp: 10, max_hp: 10 });
    const trainer = TrainerSchema.parse({
      name: "DANA",
      trainer_id: "DANA1",
      trainer_class: "LASS",
      party: [enemyMon],
      win_quote: "",
      lose_quote: "",
      base_reward: 16,
    });
    const eventNames: string[] = [];
    for (const name of ["open_text", "show_text", "wait_for_input"]) {
      eventManager.on(name, (event) => eventNames.push(event.name));
    }
    const dataLoader = {
      get_text: jest.fn().mockReturnValue("I lost..."),
      get_trainer: jest.fn().mockReturnValue(trainer),
    } as unknown as DataLoader;
    const runner = {
      _script_stack: [{ name: "TrainerBattleScript", commands: [], index: 0, allowFallthrough: false }],
      data_loader: dataLoader,
      dataLoader,
      loaded_trainer: null,
      loaded_trainer_id: null,
      just_battled: false,
      last_value: null,
      last_condition_result: false,
      variables: {},
      set_event_flag: jest.fn(),
      defer: jest.fn(),
      jump: jest.fn(),
      pause: jest.fn(),
      resume: jest.fn(),
    } as unknown as ScriptRunner;
    const overworld = { data_loader: dataLoader, dataLoader } as unknown as OverworldEngine;

    const command = new TrainerCommand("LASS", "DANA1", "EVENT_BEAT_DANA", "0", "BattleWinText", "0", ".AfterTrainer");
    command.runner = runner;
    const resolveTextMock = mockResolvedBattleText({
      BattleWinText: "I lost...",
    });
    try {
      command.execute(gameState, eventManager, overworld);

      eventManager.dispatch(new Event("battle_complete", { result: 0 }));

      expect(eventNames).toEqual([]);
      const stack = (runner._script_stack ?? []) as Array<{
        name: string;
        commands: Array<{ constructor: { name: string } }>;
      }>;
      const queued = stack[stack.length - 1];
      expect(queued?.name).toContain("#trainer_post_battle");
      expect(queued?.commands.map((commandEntry) => commandEntry.constructor.name)).toEqual([
        "OpenTextCommand",
        "ResolvedBattleTextCommand",
        "TrainerBattleCallbackCommand",
      ]);
      expect(runner.jump).not.toHaveBeenCalled();
      expect(runner.defer).not.toHaveBeenCalled();
    } finally {
      resolveTextMock.mockRestore();
    }
  });

  it("removes trainer battle_complete continuation listeners after the battle resolves", () => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon = [createTestPokemon("TOTODILE", 10, { hp: 10, max_hp: 10 }), null, null, null, null, null];
    const eventManager = new EventManager(gameState);
    const enemyMon = createTestPokemon("PIDGEY", 11, { level: 7, hp: 10, max_hp: 10 });
    const trainer = TrainerSchema.parse({
      name: "DANA",
      trainer_id: "DANA1",
      trainer_class: "LASS",
      party: [enemyMon],
      win_quote: "",
      lose_quote: "",
      base_reward: 16,
    });
    const dataLoader = {
      get_text: jest.fn().mockReturnValue("I lost..."),
      get_trainer: jest.fn().mockReturnValue(trainer),
    } as unknown as DataLoader;
    const runner = {
      _script_stack: [{ name: "TrainerBattleScript", commands: [], index: 0, allowFallthrough: false }],
      data_loader: dataLoader,
      dataLoader,
      loaded_trainer: null,
      loaded_trainer_id: null,
      just_battled: false,
      last_value: null,
      last_condition_result: false,
      variables: {},
      set_event_flag: jest.fn(),
      defer: jest.fn(),
      jump: jest.fn(),
      pause: jest.fn(),
      resume: jest.fn(),
    } as unknown as ScriptRunner;
    const overworld = { data_loader: dataLoader, dataLoader } as unknown as OverworldEngine;
    const command = new TrainerCommand("LASS", "DANA1", "EVENT_BEAT_DANA", "0", "BattleWinText", "0", "");
    command.runner = runner;

    command.execute(gameState, eventManager, overworld);
    expect(eventManager._listeners.battle_complete?.length).toBeGreaterThan(0);

    eventManager.dispatch(new Event("battle_complete", { result: 0 }));

    expect(eventManager._listeners.battle_complete ?? []).toHaveLength(0);
  });

  it("does not queue duplicate trainer post-battle work from repeated battle_complete events", () => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon = [createTestPokemon("TOTODILE", 10, { hp: 10, max_hp: 10 }), null, null, null, null, null];
    const eventManager = new EventManager(gameState);
    const enemyMon = createTestPokemon("PIDGEY", 11, { level: 7, hp: 10, max_hp: 10 });
    const trainer = TrainerSchema.parse({
      name: "DANA",
      trainer_id: "DANA1",
      trainer_class: "LASS",
      party: [enemyMon],
      win_quote: "",
      lose_quote: "",
      base_reward: 16,
    });
    const dataLoader = {
      get_text: jest.fn().mockReturnValue("I lost..."),
      get_trainer: jest.fn().mockReturnValue(trainer),
    } as unknown as DataLoader;
    const runner = {
      _script_stack: [{ name: "TrainerBattleScript", commands: [], index: 0, allowFallthrough: false }],
      data_loader: dataLoader,
      dataLoader,
      loaded_trainer: null,
      loaded_trainer_id: null,
      just_battled: false,
      last_value: null,
      last_condition_result: false,
      variables: {},
      set_event_flag: jest.fn(),
      defer: jest.fn(),
      jump: jest.fn(),
      pause: jest.fn(),
      resume: jest.fn(),
    } as unknown as ScriptRunner;
    const overworld = { data_loader: dataLoader, dataLoader } as unknown as OverworldEngine;
    const command = new TrainerCommand("LASS", "DANA1", "EVENT_BEAT_DANA", "0", "BattleWinText", "0", ".AfterTrainer");
    command.runner = runner;
    const resolveTextMock = mockResolvedBattleText({
      BattleWinText: "I lost...",
    });
    try {
      command.execute(gameState, eventManager, overworld);

      eventManager.dispatch(new Event("battle_complete", { result: 0 }));
      const stackAfterFirst = runner._script_stack?.length ?? 0;
      eventManager.dispatch(new Event("battle_complete", { result: 0 }));

      expect(runner._script_stack?.length ?? 0).toBe(stackAfterFirst);
      expect(
        (runner._script_stack ?? []).filter((frame) => frame.name.includes("#trainer_post_battle"))
      ).toHaveLength(1);
    } finally {
      resolveTextMock.mockRestore();
    }
  });

  it("recreates first Route 30 trainer without stacking post-battle inputs", () => {
    const gameState = createInitialGameState();
    gameState.wram.instant_mode = true;
    gameState.sram.player_name = "CHRIS";
    gameState.sram.party.pokemon = [
      createTestPokemon("TOTODILE", 10, { hp: 10, max_hp: 10 }),
      null,
      null,
      null,
      null,
      null,
    ];
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    const events: Array<{ name: string; text?: string }> = [];
    let startBattleCount = 0;

    for (const name of ["open_text", "show_text", "wait_for_input"]) {
      eventManager.on(name, (event) => {
        events.push({ name: event.name, text: String(event.data?.text ?? "") });
      });
    }
    eventManager.on("start_battle", () => {
      startBattleCount += 1;
    });

    const overworld = {
      current_map_name: "Route30",
      data_loader: dataLoader,
      dataLoader,
      requestEncounterMusic: jest.fn(),
    } as unknown as OverworldEngine;
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);

    runner.run("TrainerYoungsterJoey");
    for (let i = 0; i < 10 && startBattleCount === 0 && runner._script_stack.length > 0; i += 1) {
      runner.resume();
    }

    expect(startBattleCount).toBe(1);
    expect(eventManager._listeners.battle_complete ?? []).toHaveLength(2);

    const eventCountBeforeBattleComplete = events.length;
    const startedAt = performance.now();
    eventManager.dispatch(new Event("battle_complete", { result: 0 }));
    const elapsedMs = performance.now() - startedAt;
    const postBattleEvents = events.slice(eventCountBeforeBattleComplete);

    expect(postBattleEvents.map((event) => event.name)).toEqual([
      "open_text",
      "show_text",
      "wait_for_input",
    ]);
    expect(postBattleEvents[1]?.text).toContain("Ack! I lost again!");
    expect(postBattleEvents[1]?.text).toContain("Doggone it!");
    expect(elapsedMs).toBeLessThan(25);
    expect(eventManager._listeners.battle_complete ?? []).toHaveLength(0);
    expect(gameState.wram.event_flags.EVENT_BEAT_YOUNGSTER_JOEY).toBe(true);
    expect(gameState.wram.wRunningTrainerBattleScript).toBe(0);
    expect(runner.just_battled).toBe(false);

    const eventCountAfterFirstCompletion = events.length;
    eventManager.dispatch(new Event("battle_complete", { result: 0 }));

    expect(events).toHaveLength(eventCountAfterFirstCompletion);
    expect(
      events.filter((event) => event.name === "show_text" && event.text?.includes("Ack! I lost again!")),
    ).toHaveLength(1);
  });

  it("queues cleanup after one-shot trainer win text when there is no talk callback", () => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon = [createTestPokemon("TOTODILE", 10, { hp: 10, max_hp: 10 }), null, null, null, null, null];
    const eventManager = new EventManager(gameState);
    const enemyMon = createTestPokemon("PIDGEY", 11, { level: 7, hp: 10, max_hp: 10 });
    const trainer = TrainerSchema.parse({
      name: "DANA",
      trainer_id: "DANA1",
      trainer_class: "LASS",
      party: [enemyMon],
      win_quote: "",
      lose_quote: "",
      base_reward: 16,
    });
    const dataLoader = {
      get_text: jest.fn().mockReturnValue("I lost..."),
      get_trainer: jest.fn().mockReturnValue(trainer),
    } as unknown as DataLoader;
    const runner = {
      _script_stack: [{ name: "TrainerBattleScript", commands: [], index: 0, allowFallthrough: false }],
      data_loader: dataLoader,
      dataLoader,
      loaded_trainer: null,
      loaded_trainer_id: null,
      just_battled: false,
      last_value: null,
      last_condition_result: false,
      variables: {},
      set_event_flag: jest.fn(),
      defer: jest.fn(),
      jump: jest.fn(),
      pause: jest.fn(),
      resume: jest.fn(),
    } as unknown as ScriptRunner;
    const overworld = { data_loader: dataLoader, dataLoader } as unknown as OverworldEngine;
    const command = new TrainerCommand("LASS", "DANA1", "EVENT_BEAT_DANA", "0", "BattleWinText", "0", "");
    command.runner = runner;
    const resolveTextMock = mockResolvedBattleText({
      BattleWinText: "I lost...",
    });
    try {
      command.execute(gameState, eventManager, overworld);

      eventManager.dispatch(new Event("battle_complete", { result: 0 }));

      const stack = (runner._script_stack ?? []) as Array<{
        name: string;
        commands: Array<{ constructor: { name: string } }>;
      }>;
      const queued = stack[stack.length - 1];
      expect(queued?.name).toContain("#trainer_post_battle");
      expect(queued?.commands.map((commandEntry) => commandEntry.constructor.name)).toEqual([
        "OpenTextCommand",
        "ResolvedBattleTextCommand",
        "ClearTrainerBattleScriptCommand",
      ]);
    } finally {
      resolveTextMock.mockRestore();
    }
  });
});
