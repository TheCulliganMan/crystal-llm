import fs from "fs";
import { exportCoreContentPack } from "./export-content-pack";

const mockWriteJsonToTargets = jest.fn();
const mockRemoveMatchingOutputs = jest.fn();
const mockReadJsonAssetSync = jest.fn();

jest.mock("./asm-utils", () => ({
  writeJsonToTargets: (...args: unknown[]) => mockWriteJsonToTargets(...args),
  removeMatchingOutputs: (...args: unknown[]) => mockRemoveMatchingOutputs(...args),
}));

jest.mock("@pokecrystal/core/core/asset-reader", () => ({
  readJsonAssetSync: (...args: unknown[]) => mockReadJsonAssetSync(...args),
}));

jest.mock("@pokecrystal/core/core/paths", () => ({
  getDataDir: () => "/mock/assets/data",
}));

describe("export-core-content-pack", () => {
  beforeEach(() => {
    mockWriteJsonToTargets.mockReset();
    mockRemoveMatchingOutputs.mockReset();
    mockReadJsonAssetSync.mockReset();
    jest.restoreAllMocks();
  });

  it("writes an enabled core modular pack, disabled module packs, and keeps existing packs", () => {
    mockReadJsonAssetSync.mockReturnValue({
      version: 1,
      packs: [
        {
          id: "city-pack",
          enabled: true,
          files: {
            pokemon: [],
            moves: [],
            learnsets: [],
            level_up_moves: [],
            egg_moves: [],
            evolutions: [],
            map_attributes: ["content-packs/city-pack/map_attributes/goldenrod.json"],
            map_dimensions: [],
            wild_encounters: [],
            npcs: [],
            pokegear_landmarks: [],
          },
        },
        {
          id: "module-route-stale",
          enabled: true,
          files: {
            wild_encounters: ["content-packs/stale/wild_encounters/stale.json"],
          },
        },
      ],
    });

    exportCoreContentPack({
      pokemonData: [{ id: "TOTODILE" } as never],
      movesData: { SCRATCH: { name: "SCRATCH", type: "NORMAL" } as never },
      learnsetsData: { TOTODILE: [[1, "SCRATCH"]] },
      levelUpMovesData: { TOTODILE: [{ level: 1, move: "SCRATCH" }] },
      eggMovesData: { TOTODILE: ["CRUNCH"] },
      evolutions: [{ species: "TOTODILE" } as never],
      wildEncounters: [{ map_name: "Route1" } as never],
      mapDimensions: { Route1: { width: 10, height: 8 } },
      mapAttributes: { Route1: { environment: "TOWN" } },
      items: [{ name: "POTION", price: 300 } as never],
      trainers: [{ trainer_id: "YOUNGSTER_JOE", name: "Youngster Joe" } as never],
      pokedex: [{ species: "TOTODILE", text: "A tiny croc." } as never],
      pokegearLandmarks: {
        landmarks: [
          {
            id: 1,
            constant: "LANDMARK_NEW_BARK_TOWN",
            label: "NEW_BARK_TOWN",
            name: "New Bark Town",
            x: 12,
            y: 34,
            region: "JOHTO",
          },
        ],
        map_to_landmark: { NewBarkTown: "LANDMARK_NEW_BARK_TOWN" },
      },
      npcData: {
        Route1: [
          {
            x: 1,
            y: 2,
            sprite: "SPRITE_YOUNGSTER",
            spritemovedata: "SPRITEMOVEDATA_STANDING_DOWN",
            move_range_x: 0,
            move_range_y: 0,
            hram_x: -1,
            hram_y: -1,
            pal: 9,
            object_type: "OBJECTTYPE_SCRIPT",
            radius: 0,
            script: "Route1YoungsterScript",
            event_flag: "-1",
            object_identifier: "ROUTE1_YOUNGSTER",
          },
        ],
      } as never,
    });

    expect(mockRemoveMatchingOutputs).toHaveBeenCalledWith("content-packs/core-modular", ".json");
    expect(mockWriteJsonToTargets).toHaveBeenCalledWith(
      "content-packs/core-modular/npcs/route1.json",
      {
        Route1: [
          expect.objectContaining({
            script: "Route1YoungsterScript",
            object_identifier: "ROUTE1_YOUNGSTER",
          }),
        ],
      },
      { indent: 2 }
    );
    expect(mockWriteJsonToTargets).toHaveBeenCalledWith(
      "content-packs/core-modular.compiled.json",
      expect.objectContaining({
        version: 1,
        packId: "core-modular",
        categories: expect.objectContaining({
          pokemon: [{ id: "TOTODILE" }],
          moves: [{ name: "SCRATCH", type: "NORMAL" }],
          learnsets: [{ species: "TOTODILE", learnset: [[1, "SCRATCH"]] }],
          level_up_moves: [{ species: "TOTODILE", moves: [{ level: 1, move: "SCRATCH" }] }],
          egg_moves: [{ species: "TOTODILE", moves: ["CRUNCH"] }],
          evolutions: [{ species: "TOTODILE" }],
          map_attributes: [{ Route1: { environment: "TOWN" } }],
          map_dimensions: [{ Route1: { width: 10, height: 8 } }],
          wild_encounters: [{ map_name: "Route1" }],
          npcs: [
            {
              Route1: [
                expect.objectContaining({
                  script: "Route1YoungsterScript",
                  object_identifier: "ROUTE1_YOUNGSTER",
                }),
              ],
            },
          ],
          pokegear_landmarks: [
            expect.objectContaining({
              landmarks: expect.any(Array),
              map_to_landmark: { NewBarkTown: "LANDMARK_NEW_BARK_TOWN" },
            }),
          ],
          items: [{ name: "POTION", price: 300 }],
        }),
      }),
      { indent: 0 }
    );

    const indexCall = mockWriteJsonToTargets.mock.calls.at(-1);
    expect(indexCall?.[0]).toBe("content-packs/index.json");
    expect(indexCall?.[1]).toEqual({
      version: 1,
      packs: expect.arrayContaining([
        expect.objectContaining({ id: "city-pack" }),
        expect.objectContaining({
          id: "core-modular",
          enabled: true,
          priority: -100,
          compiled: "content-packs/core-modular.compiled.json",
          files: expect.objectContaining({
            pokemon: ["content-packs/core-modular/pokemon/totodile.json"],
            moves: ["content-packs/core-modular/moves/scratch.json"],
            learnsets: ["content-packs/core-modular/learnsets/totodile.json"],
            level_up_moves: ["content-packs/core-modular/level_up_moves/totodile.json"],
            egg_moves: ["content-packs/core-modular/egg_moves/totodile.json"],
            evolutions: ["content-packs/core-modular/evolutions/totodile.json"],
            map_attributes: ["content-packs/core-modular/map_attributes/route1.json"],
            map_dimensions: ["content-packs/core-modular/map_dimensions/route1.json"],
            wild_encounters: ["content-packs/core-modular/wild_encounters/route1.json"],
            npcs: ["content-packs/core-modular/npcs/route1.json"],
            pokegear_landmarks: ["content-packs/core-modular/pokegear_landmarks/landmarks.json"],
            items: ["content-packs/core-modular/items/potion.json"],
          }),
        }),
        expect.objectContaining({
          id: "module-pokemon-totodile",
          enabled: false,
          files: expect.objectContaining({
            pokemon: ["content-packs/core-modular/pokemon/totodile.json"],
          }),
        }),
        expect.objectContaining({
          id: "module-move-scratch",
          enabled: false,
          files: expect.objectContaining({
            moves: ["content-packs/core-modular/moves/scratch.json"],
          }),
        }),
        expect.objectContaining({
          id: "module-learnset-totodile",
          enabled: false,
          files: expect.objectContaining({
            learnsets: ["content-packs/core-modular/learnsets/totodile.json"],
          }),
        }),
        expect.objectContaining({
          id: "module-level-up-move-totodile",
          enabled: false,
          files: expect.objectContaining({
            level_up_moves: ["content-packs/core-modular/level_up_moves/totodile.json"],
          }),
        }),
        expect.objectContaining({
          id: "module-egg-move-totodile",
          enabled: false,
          files: expect.objectContaining({
            egg_moves: ["content-packs/core-modular/egg_moves/totodile.json"],
          }),
        }),
        expect.objectContaining({
          id: "module-pokegear-landmarks-landmarks",
          enabled: false,
          files: expect.objectContaining({
            pokegear_landmarks: ["content-packs/core-modular/pokegear_landmarks/landmarks.json"],
          }),
        }),
        expect.objectContaining({
          id: "module-route-route1",
          enabled: false,
          files: expect.objectContaining({
            wild_encounters: ["content-packs/core-modular/wild_encounters/route1.json"],
            npcs: ["content-packs/core-modular/npcs/route1.json"],
          }),
        }),
        expect.objectContaining({
          id: "module-npc-route1",
          enabled: false,
          files: expect.objectContaining({
            npcs: ["content-packs/core-modular/npcs/route1.json"],
          }),
        }),
        expect.objectContaining({
          id: "module-item-potion",
          enabled: false,
          files: expect.objectContaining({
            items: ["content-packs/core-modular/items/potion.json"],
          }),
        }),
        expect.objectContaining({
          id: "module-trainer-youngster_joe",
          files: expect.objectContaining({
            trainers: ["content-packs/core-modular/trainers/youngster_joe.json"],
          }),
        }),
        expect.objectContaining({
          id: "module-pokedex-totodile",
          files: expect.objectContaining({
            pokedex: ["content-packs/core-modular/pokedex/totodile.json"],
          }),
        }),
      ]),
    });
    const packs = indexCall?.[1]?.packs as Array<{ id: string; enabled?: boolean }>;
    expect(packs.find((pack) => pack.id === "module-route-stale")).toBeUndefined();
    expect(packs.filter((pack) => pack.id.startsWith("module-"))).toEqual(
      expect.arrayContaining([expect.objectContaining({ id: "module-route-route1", enabled: false })])
    );
    expect(packs.filter((pack) => pack.id.startsWith("module-")).every((pack) => pack.enabled === false)).toBe(true);
  });

  it("normalizes legacy pack entries that do not list newer categories", () => {
    mockReadJsonAssetSync.mockReturnValue({
      version: 1,
      packs: [
        {
          id: "legacy-pack",
          enabled: true,
          files: {
            pokemon: ["content-packs/legacy-pack/pokemon/chikorita.json"],
          },
        },
      ],
    });

    exportCoreContentPack({
      pokemonData: [],
      movesData: {},
      learnsetsData: {},
      levelUpMovesData: {},
      eggMovesData: {},
      evolutions: [],
      wildEncounters: [],
      mapDimensions: {},
      mapAttributes: {},
      items: [],
      trainers: [],
      pokedex: [],
      npcData: {},
      pokegearLandmarks: { landmarks: [], map_to_landmark: {} },
    });

    const indexCall = mockWriteJsonToTargets.mock.calls.at(-1);
    const packs = indexCall?.[1]?.packs as Array<{
      id: string;
      files: Record<string, string[]>;
    }>;
    const legacyPack = packs.find((pack) => pack.id === "legacy-pack");

    expect(legacyPack?.files).toEqual(
      expect.objectContaining({
        pokemon: ["content-packs/legacy-pack/pokemon/chikorita.json"],
        npcs: [],
        items: [],
        trainers: [],
        pokedex: [],
        story_events: [],
        phone_scripts: [],
      })
    );
  });

  it("matches route module map files by map identity, not wildcard filename", () => {
    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      return String(pathLike).endsWith("/maps");
    });
    jest.spyOn(fs, "readdirSync").mockImplementation((pathLike) => {
      if (String(pathLike).endsWith("/maps")) {
        return ["Route29.json", "Route46.json", "CherrygroveCity.json", "NewBarkTown.json"] as never;
      }
      return [] as never;
    });
    mockReadJsonAssetSync.mockImplementation((filePath: string) => {
      if (filePath.endsWith("content-packs/index.json")) {
        return { version: 1, packs: [] };
      }
      if (filePath.endsWith("maps/Route29.json")) {
        return {
          Route29_MapScripts: [{ command: "def_scene_scripts", args: [] }],
          Route29_MapEvents: [{ command: "def_warp_events", args: [] }],
        };
      }
      if (filePath.endsWith("maps/Route46.json")) {
        return {
          Route46_MapScripts: [{ command: "def_scene_scripts", args: [] }],
          Route46_MapEvents: [{ command: "def_warp_events", args: [] }],
        };
      }
      if (filePath.endsWith("maps/CherrygroveCity.json")) {
        return {
          CherrygroveCity_MapScripts: [{ command: "def_scene_scripts", args: [] }],
          CherrygroveCity_MapEvents: [{ command: "def_warp_events", args: [] }],
        };
      }
      if (filePath.endsWith("maps/NewBarkTown.json")) {
        return {
          NewBarkTown_MapScripts: [{ command: "def_scene_scripts", args: [] }],
          NewBarkTown_MapEvents: [{ command: "def_warp_events", args: [] }],
        };
      }
      if (filePath.endsWith("map_blocks.json")) {
        return {};
      }
      throw new Error(`Unexpected read ${filePath}`);
    });

    exportCoreContentPack({
      pokemonData: [],
      movesData: {},
      learnsetsData: {},
      levelUpMovesData: {},
      eggMovesData: {},
      evolutions: [],
      wildEncounters: [{ map_name: "Route 29" } as never],
      mapDimensions: {
        Route29: { width: 10, height: 9 },
        Route46: { width: 8, height: 8 },
        CherrygroveCity: { width: 12, height: 10 },
        NewBarkTown: { width: 10, height: 8 },
      },
      mapAttributes: {
        Route29: {
          environment: "ROUTE",
          connections: [
            { direction: "north", target_map: "Route46", offset: 10 },
            { direction: "west", target_map: "CherrygroveCity", offset: 0 },
            { direction: "east", target_map: "NewBarkTown", offset: 0 },
          ],
        },
        Route46: { environment: "ROUTE", connections: [] },
        CherrygroveCity: { environment: "TOWN", connections: [] },
        NewBarkTown: { environment: "TOWN", connections: [] },
      },
      items: [],
      trainers: [],
      pokedex: [],
      npcData: { Route29: [], Route46: [], CherrygroveCity: [], NewBarkTown: [] },
      pokegearLandmarks: { landmarks: [], map_to_landmark: {} },
    });

    const indexCall = mockWriteJsonToTargets.mock.calls.at(-1);
    const packs = indexCall?.[1]?.packs as Array<{
      id: string;
      enabled?: boolean;
      files: {
        maps: string[];
        map_attributes: string[];
        map_dimensions: string[];
        npcs: string[];
      };
    }>;
    const corePack = packs.find((pack) => pack.id === "core-modular");
    const routeModule = packs.find((pack) => pack.id === "module-route-route_29");

    expect(corePack).toEqual(
      expect.objectContaining({
        enabled: true,
        files: expect.objectContaining({
          maps: expect.arrayContaining([
            "content-packs/core-modular/maps/route29.json",
            "content-packs/core-modular/maps/route46.json",
            "content-packs/core-modular/maps/cherrygrovecity.json",
            "content-packs/core-modular/maps/newbarktown.json",
          ]),
          map_attributes: expect.arrayContaining([
            "content-packs/core-modular/map_attributes/route29.json",
            "content-packs/core-modular/map_attributes/route46.json",
            "content-packs/core-modular/map_attributes/cherrygrovecity.json",
            "content-packs/core-modular/map_attributes/newbarktown.json",
          ]),
        }),
      })
    );
    expect(routeModule).toBeDefined();
    expect(routeModule?.enabled).toBe(false);
    expect(routeModule?.files.maps).toEqual(["content-packs/core-modular/maps/route29.json"]);
    expect(routeModule?.files.map_attributes).toEqual([
      "content-packs/core-modular/map_attributes/route29.json",
    ]);
    expect(routeModule?.files.map_dimensions).toEqual([
      "content-packs/core-modular/map_dimensions/route29.json",
    ]);
    expect(routeModule?.files.npcs).toEqual([
      "content-packs/core-modular/npcs/route29.json",
    ]);
  });

  it("writes compact trainer party entries with species references", () => {
    mockReadJsonAssetSync.mockReturnValue({ version: 1, packs: [] });

    exportCoreContentPack({
      pokemonData: [],
      movesData: {},
      learnsetsData: {},
      levelUpMovesData: {},
      eggMovesData: {},
      evolutions: [],
      wildEncounters: [],
      mapDimensions: {},
      mapAttributes: {},
      items: [],
      trainers: [
        {
          trainer_id: "YOUNGSTER_JOE",
          name: "Youngster Joe",
          trainer_class: "YOUNGSTER",
          party: [
            {
              species: {
                id: "RATTATA",
                base_stats: { hp: 30 },
                tmhm_learnset: ["HEADBUTT"],
              },
              nickname: "RATTATA",
              level: 6,
              item: null,
              moves: [],
              hp: 30,
              max_hp: 30,
              original_trainer_name: "Trainer",
              original_trainer_id: 0,
              experience: 0,
              happiness: 0,
              dvs: { attack: 0, defense: 0, speed: 0, special: 0, hp: 0 },
              sleep_turns: 0,
              flinching: false,
              stat_boosts: {
                HP: 0,
                ATTACK: 0,
                DEFENSE: 0,
                SPEED: 0,
                SPECIAL_ATTACK: 0,
                SPECIAL_DEFENSE: 0,
                ACCURACY: 0,
                EVASION: 0,
              },
              last_damage_taken: 0,
            },
          ],
        } as never,
      ],
      pokedex: [],
      npcData: {},
      pokegearLandmarks: { landmarks: [], map_to_landmark: {} },
    });

    expect(mockWriteJsonToTargets).toHaveBeenCalledWith(
      "content-packs/core-modular/trainers/youngster_joe.json",
      expect.objectContaining({
        party: [
          {
            species: "RATTATA",
            level: 6,
          },
        ],
      }),
      { indent: 2 }
    );

    const trainerCall = mockWriteJsonToTargets.mock.calls.find(
      (call) => call[0] === "content-packs/core-modular/trainers/youngster_joe.json"
    );
    const trainerPayload = trainerCall?.[1] as { party?: Array<Record<string, unknown>> };

    expect(trainerPayload.party?.[0].species).toBe("RATTATA");
    expect(JSON.stringify(trainerPayload)).not.toContain("base_stats");
    expect(JSON.stringify(trainerPayload)).not.toContain("tmhm_learnset");
  });

  it("preserves explicit trainer party overrides while stripping generated defaults", () => {
    mockReadJsonAssetSync.mockReturnValue({ version: 1, packs: [] });

    exportCoreContentPack({
      pokemonData: [],
      movesData: {},
      learnsetsData: {},
      levelUpMovesData: {},
      eggMovesData: {},
      evolutions: [],
      wildEncounters: [],
      mapDimensions: {},
      mapAttributes: {},
      items: [],
      trainers: [
        {
          trainer_id: "BUG_CATCHER_AL",
          name: "Bug Catcher Al",
          trainer_class: "BUG_CATCHER",
          party: [
            {
              species: {
                id: "CATERPIE",
                base_stats: { hp: 35 },
              },
              nickname: "STRING",
              level: 7,
              item: "BERRY",
              moves: ["TACKLE"],
              hp: 20,
              max_hp: 35,
              original_trainer_name: "Trainer",
              original_trainer_id: 0,
              experience: 0,
              happiness: 0,
              dvs: { attack: 1, defense: 0, speed: 0, special: 0, hp: 0 },
              sleep_turns: 0,
              flinching: false,
              focus_energy: true,
              stat_boosts: {
                HP: 0,
                ATTACK: 1,
                DEFENSE: 0,
                SPEED: 0,
                SPECIAL_ATTACK: 0,
                SPECIAL_DEFENSE: 0,
                ACCURACY: 0,
                EVASION: 0,
              },
              last_damage_taken: 0,
            },
          ],
        } as never,
      ],
      pokedex: [],
      npcData: {},
      pokegearLandmarks: { landmarks: [], map_to_landmark: {} },
    });

    expect(mockWriteJsonToTargets).toHaveBeenCalledWith(
      "content-packs/core-modular/trainers/bug_catcher_al.json",
      expect.objectContaining({
        party: [
          {
            species: "CATERPIE",
            level: 7,
            nickname: "STRING",
            item: "BERRY",
            moves: ["TACKLE"],
            hp: 20,
            dvs: { attack: 1, defense: 0, speed: 0, special: 0, hp: 0 },
            focus_energy: true,
            stat_boosts: {
              HP: 0,
              ATTACK: 1,
              DEFENSE: 0,
              SPEED: 0,
              SPECIAL_ATTACK: 0,
              SPECIAL_DEFENSE: 0,
              ACCURACY: 0,
              EVASION: 0,
            },
          },
        ],
      }),
      { indent: 2 }
    );

    const trainerCall = mockWriteJsonToTargets.mock.calls.find(
      (call) => call[0] === "content-packs/core-modular/trainers/bug_catcher_al.json"
    );
    const partyMember = (trainerCall?.[1] as { party?: Array<Record<string, unknown>> }).party?.[0];

    expect(partyMember).not.toHaveProperty("max_hp");
    expect(partyMember).not.toHaveProperty("original_trainer_name");
    expect(partyMember).not.toHaveProperty("experience");
    expect(partyMember).not.toHaveProperty("last_damage_taken");
  });

  it("throws when a trainer party entry cannot be serialized to a species id", () => {
    mockReadJsonAssetSync.mockReturnValue({ version: 1, packs: [] });

    expect(() =>
      exportCoreContentPack({
        pokemonData: [],
        movesData: {},
        learnsetsData: {},
        levelUpMovesData: {},
        eggMovesData: {},
        evolutions: [],
        wildEncounters: [],
        mapDimensions: {},
        mapAttributes: {},
        items: [],
        trainers: [
          {
            trainer_id: "BROKEN",
            name: "Broken",
            party: [
              {
                species: { base_stats: { hp: 10 } },
                level: 1,
              },
            ],
          } as never,
        ],
        pokedex: [],
        npcData: {},
        pokegearLandmarks: { landmarks: [], map_to_landmark: {} },
      })
    ).toThrow(
      "Unable to export trainer BROKEN party[0] because species is not a species id string or Pokemon species record with an id."
    );
  });

  it("exports map files and map block entries into route modules", () => {
    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      return String(pathLike).endsWith("/maps");
    });
    jest.spyOn(fs, "readdirSync").mockImplementation((pathLike) => {
      if (String(pathLike).endsWith("/maps")) {
        return ["NewRoute.json"] as never;
      }
      return [] as never;
    });
    mockReadJsonAssetSync.mockImplementation((filePath: string) => {
      if (filePath.endsWith("content-packs/index.json")) {
        return { version: 1, packs: [] };
      }
      if (filePath.endsWith("maps/NewRoute.json")) {
        return {
          NewRoute_MapScripts: [{ command: "def_scene_scripts", args: [] }],
          NewRoute_MapEvents: [{ command: "def_warp_events", args: [] }],
        };
      }
      if (filePath.endsWith("map_blocks.json")) {
        return { NewRoute_Blocks: "AQID" };
      }
      throw new Error(`Unexpected read ${filePath}`);
    });

    exportCoreContentPack({
      pokemonData: [],
      movesData: {},
      learnsetsData: {},
      levelUpMovesData: {},
      eggMovesData: {},
      evolutions: [],
      wildEncounters: [{ map_name: "NewRoute" } as never],
      mapDimensions: { NewRoute: { width: 4, height: 4 } },
      mapAttributes: { NewRoute: { tileset_name: "johto", blocks_label: "NewRoute_Blocks" } },
      items: [],
      trainers: [],
      pokedex: [],
      npcData: { NewRoute: [] },
      pokegearLandmarks: { landmarks: [], map_to_landmark: {} },
    });

    expect(mockWriteJsonToTargets).toHaveBeenCalledWith(
      "content-packs/core-modular/maps/newroute.json",
      expect.objectContaining({
        NewRoute_MapScripts: expect.any(Array),
        NewRoute_MapEvents: expect.any(Array),
      }),
      { indent: 2 }
    );
    expect(mockWriteJsonToTargets).toHaveBeenCalledWith(
      "content-packs/core-modular/map_blocks/newroute_blocks.json",
      { NewRoute_Blocks: "AQID" },
      { indent: 2 }
    );

    const indexCall = mockWriteJsonToTargets.mock.calls.at(-1);
    const packs = indexCall?.[1]?.packs as Array<{
      id: string;
      files: { maps: string[]; map_blocks: string[] };
    }>;
    const routeModule = packs.find((pack) => pack.id === "module-route-newroute");

    expect(routeModule?.files.maps).toEqual(["content-packs/core-modular/maps/newroute.json"]);
    expect(routeModule?.files.map_blocks).toEqual([
      "content-packs/core-modular/map_blocks/newroute_blocks.json",
    ]);
  });

});
