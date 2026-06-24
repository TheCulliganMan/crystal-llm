import fs from "fs";
import { exportCoreContentPack } from "./export-content-pack";

const mockWriteJsonToTargets = jest.fn();
const mockRemoveMatchingOutputs = jest.fn();
const mockEnsureDir = jest.fn();
const mockReadJsonAssetSync = jest.fn();

const TEST_CONTENT_PACK_CATEGORIES = [
  "pokemon",
  "moves",
  "learnsets",
  "level_up_moves",
  "egg_moves",
  "evolutions",
  "maps",
  "map_blocks",
  "map_attributes",
  "map_dimensions",
  "wild_encounters",
  "runtime_spawn_points",
  "runtime_map_metadata",
  "flee_mons",
  "fishing",
  "fruit_trees",
  "npcs",
  "pokegear_landmarks",
  "pc_strings",
  "menu_icons",
  "items",
  "marts",
  "currency_constants",
  "trainers",
  "pokedex",
  "pokedex_entries",
  "pokemon_frontpic_anim",
  "initialize_events",
  "story_event_script_constants",
  "story_events",
  "phone_scripts",
  "phone_contacts",
  "permanent_phone_numbers",
  "special_phone_calls",
  "npc_trades",
  "special_routines",
  "asm_text",
  "move_names",
  "battle_animations",
  "battle_animation_table",
  "battle_anim_bundle",
  "sprite_anim_bundle",
  "sprite_palette_defaults",
  "pokegear_town_map_palette_map",
  "pokemon_cries",
  "audio",
  "tilesets",
  "playability",
] as const;

const strictFiles = (
  overrides: Partial<Record<(typeof TEST_CONTENT_PACK_CATEGORIES)[number], string[]>>
): Record<(typeof TEST_CONTENT_PACK_CATEGORIES)[number], string[]> => ({
  ...Object.fromEntries(TEST_CONTENT_PACK_CATEGORIES.map((category) => [category, []])),
  ...overrides,
}) as Record<(typeof TEST_CONTENT_PACK_CATEGORIES)[number], string[]>;

const mockStrictIndexAndEmptyMapBlocks = (index = { version: 1, packs: [] as unknown[] }): void => {
  mockReadJsonAssetSync.mockImplementation((filePath: string) => {
    if (filePath.endsWith("content-packs/index.json")) {
      return index;
    }
    if (filePath.endsWith("map_blocks.json")) {
      return {};
    }
    throw new Error(`Unexpected read ${filePath}`);
  });
};

jest.mock("./asm-utils", () => ({
  writeJsonToTargets: (...args: unknown[]) => mockWriteJsonToTargets(...args),
  removeMatchingOutputs: (...args: unknown[]) => mockRemoveMatchingOutputs(...args),
  ensureDir: (...args: unknown[]) => mockEnsureDir(...args),
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
    mockEnsureDir.mockReset();
    mockReadJsonAssetSync.mockReset();
    jest.restoreAllMocks();
    jest.spyOn(fs, "writeFileSync").mockImplementation(() => undefined);
    jest.spyOn(fs, "rmSync").mockImplementation(() => undefined);
  });

  it("rejects generated content pack file stems instead of normalizing them", () => {
    mockStrictIndexAndEmptyMapBlocks();

    expect(() =>
      exportCoreContentPack({
        pokemonData: [{ id: "BAD/SPECIES" } as never],
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
      })
    ).toThrow("Content pack file stem must be a single exact path segment");
  });

  it("writes an enabled core modular pack, disabled module packs, and keeps existing packs", () => {
    mockStrictIndexAndEmptyMapBlocks({
      version: 1,
      packs: [
        {
          id: "city-pack",
          enabled: true,
          priority: 10,
          path: "content-packs/city-pack",
          compiled: null,
          files: strictFiles({
            map_attributes: ["content-packs/city-pack/map_attributes/goldenrod.json"],
          }),
        },
        {
          id: "module-route-stale",
          enabled: true,
          priority: 0,
          path: "content-packs/stale",
          compiled: null,
          files: strictFiles({
            wild_encounters: ["content-packs/stale/wild_encounters/stale.json"],
          }),
        },
      ],
    });

    exportCoreContentPack({
      pokemonData: [{ id: "TOTODILE", evolutions: null } as never],
      movesData: { SCRATCH: { name: "SCRATCH", type: "NORMAL" } as never },
      learnsetsData: { TOTODILE: [[1, "SCRATCH"]] },
      levelUpMovesData: { TOTODILE: [{ level: 1, move: "SCRATCH" }] },
      eggMovesData: { TOTODILE: ["CRUNCH"] },
      evolutions: [{ species: "TOTODILE" } as never],
      wildEncounters: [{ map_name: "Route1" } as never],
      fleeMons: { always: ["RAIKOU"], often: [], sometimes: [] },
      mapDimensions: { ROUTE_1: { width: 10, height: 8 } },
      mapAttributes: { Route1: { environment: "TOWN", map_constant: "ROUTE_1" } },
      items: [{ name: "POTION", price: 300 } as never],
      pcStrings: { PCString_ChooseaPKMN: "Choose a PKMN." },
      menuIcons: { TOTODILE: "ICON_TOTODILE" },
      trainers: [{ trainer_id: "YOUNGSTER_JOE", name: "Youngster Joe" } as never],
      pokedex: [{ species: "TOTODILE", text: "A tiny croc." } as never],
      pokedexEntries: [
        {
          species: "TOTODILE",
          classification: "BIG_JAW",
          heightDigits: 200,
          weightDigits: 210,
          pages: ["It has sharp fangs."],
        },
      ],
      pokemonFrontpicAnimations: { TOTODILE: { commands: [{ kind: "endanim" }] } },
      pokegearLandmarks: {
        landmarks: [
          {
            id: 1,
            constant: "LANDMARK_NEW_BARK_TOWN",
            label: "NEW_BARK_TOWN",
            name: "NEW BARK TOWN",
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
      playability: {
        start_maps: ["Route1"],
        start_tiles: [{ map: "Route1", tile: { x: 1, y: 2 } }],
        initial_events: [],
        initial_items: [],
        goal_maps: [],
        goal_events: ["EVENT_HALL_OF_FAME"],
        goal_items: [],
        progression_rules: [],
        map_access: [],
        require_all_maps_reachable: false,
        require_walkable_maps: true,
      },
      audioAssets: [{ id: "MUSIC_ROUTE_29", path: "content-packs/core-modular/music/MUSIC_ROUTE_29.mid", kind: "music" }],
    });

    expect(mockRemoveMatchingOutputs).toHaveBeenCalledWith("content-packs/core-modular", ".json");
    expect(fs.rmSync).toHaveBeenCalledWith("/mock/assets/data/content-packs/core-modular/music", {
      recursive: true,
      force: true,
    });
    expect(fs.rmSync).toHaveBeenCalledWith("/mock/assets/data/content-packs/core-modular/sfx", {
      recursive: true,
      force: true,
    });
    expect(fs.rmSync).toHaveBeenCalledWith("/mock/assets/data/content-packs/core-modular/cries", {
      recursive: true,
      force: true,
    });
    expect(mockEnsureDir).toHaveBeenCalledWith("/mock/assets/data/content-packs/core-modular/music");
    expect(fs.writeFileSync).toHaveBeenCalledWith(
      "/mock/assets/data/content-packs/core-modular/music/MUSIC_ROUTE_29.mid",
      expect.any(Buffer)
    );
    expect(mockWriteJsonToTargets).toHaveBeenCalledWith(
      "content-packs/core-modular/npcs/Route1.json",
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
          map_attributes: [{ Route1: { environment: "TOWN", map_constant: "ROUTE_1" } }],
          map_dimensions: [{ ROUTE_1: { width: 10, height: 8 } }],
          wild_encounters: [{ map_name: "Route1" }],
          flee_mons: [{ always: ["RAIKOU"], often: [], sometimes: [] }],
          fishing: [],
          fruit_trees: [],
          currency_constants: [],
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
          pc_strings: [{ PCString_ChooseaPKMN: "Choose a PKMN." }],
          menu_icons: [{ TOTODILE: "ICON_TOTODILE" }],
          playability: [
            {
              start_maps: ["Route1"],
              start_tiles: [{ map: "Route1", tile: { x: 1, y: 2 } }],
              initial_events: [],
              initial_items: [],
              goal_maps: [],
              goal_events: ["EVENT_HALL_OF_FAME"],
              goal_items: [],
              progression_rules: [],
              map_access: [],
              require_all_maps_reachable: false,
              require_walkable_maps: true,
            },
          ],
          pokedex_entries: [[
            {
              species: "TOTODILE",
              classification: "BIG_JAW",
              heightDigits: 200,
              weightDigits: 210,
              pages: ["It has sharp fangs."],
            },
          ]],
          pokemon_frontpic_anim: [{ TOTODILE: { commands: [{ kind: "endanim" }] } }],
          audio: [{ id: "MUSIC_ROUTE_29", path: "content-packs/core-modular/music/MUSIC_ROUTE_29.mid", kind: "music" }],
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
            pokemon: ["content-packs/core-modular/pokemon/TOTODILE.json"],
            moves: ["content-packs/core-modular/moves/SCRATCH.json"],
            learnsets: ["content-packs/core-modular/learnsets/TOTODILE.json"],
            level_up_moves: ["content-packs/core-modular/level_up_moves/TOTODILE.json"],
            egg_moves: ["content-packs/core-modular/egg_moves/TOTODILE.json"],
            evolutions: ["content-packs/core-modular/evolutions/TOTODILE.json"],
            map_attributes: ["content-packs/core-modular/map_attributes/Route1.json"],
            map_dimensions: ["content-packs/core-modular/map_dimensions/ROUTE_1.json"],
            wild_encounters: ["content-packs/core-modular/wild_encounters/Route1.json"],
            npcs: ["content-packs/core-modular/npcs/Route1.json"],
            pokegear_landmarks: ["content-packs/core-modular/pokegear_landmarks/landmarks.json"],
            items: ["content-packs/core-modular/items/POTION.json"],
            playability: ["content-packs/core-modular/playability/core.json"],
            audio: ["content-packs/core-modular/music/MUSIC_ROUTE_29.mid"],
          }),
        }),
        expect.objectContaining({
          id: "module-audio-MUSIC_ROUTE_29",
          enabled: false,
          files: expect.objectContaining({
            audio: ["content-packs/core-modular/music/MUSIC_ROUTE_29.mid"],
          }),
        }),
        expect.objectContaining({
          id: "module-pokemon-TOTODILE",
          enabled: false,
          files: expect.objectContaining({
            pokemon: ["content-packs/core-modular/pokemon/TOTODILE.json"],
          }),
        }),
        expect.objectContaining({
          id: "module-move-SCRATCH",
          enabled: false,
          files: expect.objectContaining({
            moves: ["content-packs/core-modular/moves/SCRATCH.json"],
          }),
        }),
        expect.objectContaining({
          id: "module-learnset-TOTODILE",
          enabled: false,
          files: expect.objectContaining({
            learnsets: ["content-packs/core-modular/learnsets/TOTODILE.json"],
          }),
        }),
        expect.objectContaining({
          id: "module-level-up-move-TOTODILE",
          enabled: false,
          files: expect.objectContaining({
            level_up_moves: ["content-packs/core-modular/level_up_moves/TOTODILE.json"],
          }),
        }),
        expect.objectContaining({
          id: "module-egg-move-TOTODILE",
          enabled: false,
          files: expect.objectContaining({
            egg_moves: ["content-packs/core-modular/egg_moves/TOTODILE.json"],
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
          id: "module-route-Route1",
          enabled: false,
          files: expect.objectContaining({
            wild_encounters: ["content-packs/core-modular/wild_encounters/Route1.json"],
            npcs: ["content-packs/core-modular/npcs/Route1.json"],
          }),
        }),
        expect.objectContaining({
          id: "module-npc-Route1",
          enabled: false,
          files: expect.objectContaining({
            npcs: ["content-packs/core-modular/npcs/Route1.json"],
          }),
        }),
        expect.objectContaining({
          id: "module-item-POTION",
          enabled: false,
          files: expect.objectContaining({
            items: ["content-packs/core-modular/items/POTION.json"],
          }),
        }),
        expect.objectContaining({
          id: "module-trainer-YOUNGSTER_JOE",
          files: expect.objectContaining({
            trainers: ["content-packs/core-modular/trainers/YOUNGSTER_JOE.json"],
          }),
        }),
        expect.objectContaining({
          id: "module-pokedex-TOTODILE",
          files: expect.objectContaining({
            pokedex: ["content-packs/core-modular/pokedex/TOTODILE.json"],
          }),
        }),
        expect.objectContaining({
          id: "module-playability-core",
          files: expect.objectContaining({
            playability: ["content-packs/core-modular/playability/core.json"],
          }),
        }),
      ]),
    });
    const packs = indexCall?.[1]?.packs as Array<{ id: string; enabled?: boolean }>;
    expect(packs.find((pack) => pack.id === "module-route-stale")).toBeUndefined();
    expect(packs.filter((pack) => pack.id.startsWith("module-"))).toEqual(
      expect.arrayContaining([expect.objectContaining({ id: "module-route-Route1", enabled: false })])
    );
    expect(packs.filter((pack) => pack.id.startsWith("module-")).every((pack) => pack.enabled === false)).toBe(true);
  });

  it("rejects existing pack entries that omit definitive metadata or file categories", () => {
    mockStrictIndexAndEmptyMapBlocks({
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
        trainers: [],
        pokedex: [],
        npcData: {},
        pokegearLandmarks: { landmarks: [], map_to_landmark: {} },
      })
    ).toThrow("Content pack 'legacy-pack' must declare priority.");
  });

  it("does not coerce route module map names when linking map files", () => {
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
    const routeModule = packs.find((pack) => pack.id === "module-route-Route 29");

    expect(corePack).toEqual(
      expect.objectContaining({
        enabled: true,
        files: expect.objectContaining({
          maps: expect.arrayContaining([
            "content-packs/core-modular/maps/Route29.json",
            "content-packs/core-modular/maps/Route46.json",
            "content-packs/core-modular/maps/CherrygroveCity.json",
            "content-packs/core-modular/maps/NewBarkTown.json",
          ]),
          map_attributes: expect.arrayContaining([
            "content-packs/core-modular/map_attributes/Route29.json",
            "content-packs/core-modular/map_attributes/Route46.json",
            "content-packs/core-modular/map_attributes/CherrygroveCity.json",
            "content-packs/core-modular/map_attributes/NewBarkTown.json",
          ]),
        }),
      })
    );
    expect(routeModule).toBeDefined();
    expect(routeModule?.enabled).toBe(false);
    expect(routeModule?.files.maps).toEqual([]);
    expect(routeModule?.files.map_attributes).toEqual([]);
    expect(routeModule?.files.map_dimensions).toEqual([]);
    expect(routeModule?.files.npcs).toEqual([]);
  });

  it("writes compact trainer party entries with species references", () => {
    mockStrictIndexAndEmptyMapBlocks();

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
      "content-packs/core-modular/trainers/YOUNGSTER_JOE.json",
      expect.objectContaining({
        party: [
          {
            species: "RATTATA",
            level: 6,
            item: null,
            moves: [],
            dvs: { attack: 0, defense: 0, speed: 0, special: 0, hp: 0 },
          },
        ],
      }),
      { indent: 2 }
    );

    const trainerCall = mockWriteJsonToTargets.mock.calls.find(
      (call) => call[0] === "content-packs/core-modular/trainers/YOUNGSTER_JOE.json"
    );
    const trainerPayload = trainerCall?.[1] as { party?: Array<Record<string, unknown>> };

    expect(trainerPayload.party?.[0].species).toBe("RATTATA");
    expect(JSON.stringify(trainerPayload)).not.toContain("base_stats");
    expect(JSON.stringify(trainerPayload)).not.toContain("tmhm_learnset");
  });

  it("strips generated trainer party runtime fields and rejects unknown extras", () => {
    mockStrictIndexAndEmptyMapBlocks();

    const payload = {
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
              moves: [{ name: "TACKLE", current_pp: 35 }],
              hp: 20,
              dvs: { attack: 1, defense: 0, speed: 0, special: 0, hp: 0 },
            },
          ],
        } as never,
      ],
      pokedex: [],
      npcData: {},
      pokegearLandmarks: { landmarks: [], map_to_landmark: {} },
    };

    exportCoreContentPack(payload);

    const trainerCall = mockWriteJsonToTargets.mock.calls.find(
      (call) => call[0] === "content-packs/core-modular/trainers/BUG_CATCHER_AL.json"
    );
    const partyMember = (trainerCall?.[1] as { party?: Array<Record<string, unknown>> }).party?.[0];
    expect(partyMember).toEqual({
      species: "CATERPIE",
      level: 7,
      item: "BERRY",
      moves: [{ name: "TACKLE", current_pp: 35 }],
      dvs: { attack: 1, defense: 0, speed: 0, special: 0, hp: 0 },
    });

    mockWriteJsonToTargets.mockClear();
    expect(() =>
      exportCoreContentPack({
        ...payload,
        trainers: [
          {
            ...(payload.trainers[0] as Record<string, unknown>),
            party: [
              {
                ...(payload.trainers[0] as { party: Array<Record<string, unknown>> }).party[0],
                custom_field: true,
              },
            ],
          } as never,
        ],
      })
    ).toThrow("because 'custom_field' is not part of the definitive trainer party schema");
  });

  it("throws when a trainer party entry cannot be serialized to a species id", () => {
    mockStrictIndexAndEmptyMapBlocks();

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

  it("throws when compact trainer party fields are missing", () => {
    mockStrictIndexAndEmptyMapBlocks();

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
            trainer_id: "BROKEN_HP",
            name: "Broken HP",
            trainer_class: "BUG_CATCHER",
            party: [{ species: "CATERPIE", level: 7 } as never],
          } as never,
        ],
        pokedex: [],
        npcData: {},
        pokegearLandmarks: { landmarks: [], map_to_landmark: {} },
      })
    ).toThrow("Unable to export trainer BROKEN_HP party[0] because 'item' must be explicit modpack data.");
  });

  it("throws before accepting embedded trainer species without compact fields", () => {
    mockStrictIndexAndEmptyMapBlocks();

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
            trainer_id: "BROKEN_BASE_HP",
            name: "Broken Base HP",
            trainer_class: "BUG_CATCHER",
            party: [{ species: { id: "CATERPIE", base_stats: {} }, level: 7 } as never],
          } as never,
        ],
        pokedex: [],
        npcData: {},
        pokegearLandmarks: { landmarks: [], map_to_landmark: {} },
      })
    ).toThrow("Unable to export trainer BROKEN_BASE_HP party[0] because 'item' must be explicit modpack data.");
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
      "content-packs/core-modular/maps/NewRoute.json",
      expect.objectContaining({
        NewRoute_MapScripts: expect.any(Array),
        NewRoute_MapEvents: expect.any(Array),
      }),
      { indent: 2 }
    );
    expect(mockWriteJsonToTargets).toHaveBeenCalledWith(
      "content-packs/core-modular/map_blocks/NewRoute_Blocks.json",
      { NewRoute_Blocks: "AQID" },
      { indent: 2 }
    );

    const indexCall = mockWriteJsonToTargets.mock.calls.at(-1);
    const packs = indexCall?.[1]?.packs as Array<{
      id: string;
      files: { maps: string[]; map_blocks: string[] };
    }>;
    const routeModule = packs.find((pack) => pack.id === "module-route-NewRoute");

    expect(routeModule?.files.maps).toEqual(["content-packs/core-modular/maps/NewRoute.json"]);
    expect(routeModule?.files.map_blocks).toEqual([
      "content-packs/core-modular/map_blocks/NewRoute_Blocks.json",
    ]);
  });

  it("requires exported map block data instead of generating packs without blocks", () => {
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
        throw new Error("missing map blocks");
      }
      throw new Error(`Unexpected read ${filePath}`);
    });

    expect(() =>
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
      })
    ).toThrow("missing map blocks");
  });

  it("rejects malformed map block entries instead of dropping them", () => {
    mockReadJsonAssetSync.mockImplementation((filePath: string) => {
      if (filePath.endsWith("content-packs/index.json")) {
        return { version: 1, packs: [] };
      }
      if (filePath.endsWith("map_blocks.json")) {
        return { NewRoute_Blocks: 123 };
      }
      throw new Error(`Unexpected read ${filePath}`);
    });

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
        trainers: [],
        pokedex: [],
        npcData: {},
        pokegearLandmarks: { landmarks: [], map_to_landmark: {} },
      })
    ).toThrow("map_blocks.json entry 'NewRoute_Blocks' must be encoded block data.");
  });

});
