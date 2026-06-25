import fs from "fs";
import path from "path";
import type { PokemonSpecies } from "@pokecrystal/core/core/models/pokemon";
import { exportTrainers } from "./export-trainers";

var mockDisassemblyRoot = "/mock/pokecrystal";
const mockWriteJsonToTargets = jest.fn();

jest.mock("@pokecrystal/core/core/paths", () => ({
  getDisassemblyRoot: () => mockDisassemblyRoot || "/mock/pokecrystal",
  getAssetsRoot: () => "/mock/assets",
  getDataDir: () => "/mock/assets/data",
}));

jest.mock("./asm-utils", () => {
  const actual = jest.requireActual("./asm-utils");
  return {
    ...actual,
    writeJsonToTargets: (...args: unknown[]) => mockWriteJsonToTargets(...args),
  };
});

jest.mock("./export-data", () => ({
  parseMoves: () => ({
    TACKLE: {
      name: "TACKLE",
      type: "NORMAL",
      power: 35,
      accuracy: 95,
      pp: 35,
      effect: "NORMAL_HIT",
      effect_chance: 0,
      stat: null,
      amount: null,
    },
  }),
}));

const species = (id: string): PokemonSpecies => ({
  id,
  int_id: 1,
  base_stats: { hp: 45, attack: 49, defense: 49, speed: 45, special_attack: 65, special_defense: 65 },
  type1: "GRASS",
  type2: "GRASS",
  catch_rate: 45,
  base_exp: 64,
  item1: null,
  item2: null,
  gender_ratio: 127,
  unknown1: 0,
  step_cycles_to_hatch: 20,
  unknown2: 0,
  growth_rate: "GROWTH_MEDIUM_SLOW",
  egg_group1: "EGG_MONSTER",
  egg_group2: "EGG_PLANT",
  tmhm_learnset: [],
  ability: "NONE",
  pic_size: 0,
  front_pic: 0,
  back_pic: 0,
  weight: 0,
  evolutions: null,
} as PokemonSpecies);

describe("exportTrainers", () => {
  beforeEach(() => {
    mockDisassemblyRoot = "/mock/pokecrystal";
    mockWriteJsonToTargets.mockReset();
    jest.restoreAllMocks();
    jest.spyOn(fs, "readFileSync").mockImplementation((filePath: fs.PathOrFileDescriptor) => {
      const file = String(filePath);
      if (file.endsWith(path.join("data", "trainers", "parties.asm"))) {
        return [
          '	db "Joey", TRAINERTYPE_NORMAL',
          "	db 4, CHIKORITA",
          "	db -1",
        ].join("\n") as never;
      }
      if (file.endsWith(path.join("constants", "trainer_constants.asm"))) {
        return [
          "trainerclass YOUNGSTER",
          "	const YOUNGSTER_JOEY",
        ].join("\n") as never;
      }
      if (file.endsWith(path.join("data", "trainers", "attributes.asm"))) {
        return "	db 4 ; base reward\n" as never;
      }
      throw new Error(`unexpected read ${file}`);
    });
  });

  it("requires explicit pokemonData instead of reading legacy asset fallbacks", () => {
    expect(() => exportTrainers([])).toThrow(
      "exportTrainers requires explicit pokemonData from the current core export."
    );
  });

  it("exports trainers from explicit pokemonData", () => {
    const trainers = exportTrainers([species("CHIKORITA")]);

    expect(trainers).toEqual([
      expect.objectContaining({
        name: "Joey",
        trainer_id: "YOUNGSTER_JOEY",
        trainer_class: "YOUNGSTER",
        base_reward: 4,
        encounter_music: "MUSIC_YOUNGSTER_ENCOUNTER",
        party: [expect.objectContaining({ species: expect.objectContaining({ id: "CHIKORITA" }) })],
      }),
    ]);
    expect(mockWriteJsonToTargets).toHaveBeenCalledWith("trainers.json", trainers, { indent: 2 });
  });
});
