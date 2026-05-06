import { readJsonAssetSync } from "./asset-reader";
import {
  listContentPackFilesSync,
  loadContentPackCategoryJsonSync,
  loadMergedItemsSync,
  loadMergedMapBlocksSync,
  loadMergedMapAttributesSync,
  loadMergedMapDimensionsSync,
  loadMergedNpcDataSync,
  loadMergedPokedexSync,
  loadMergedPokemonDataSync,
  loadMergedTrainersSync,
  loadMergedWildEncountersSync,
  mergePokegearLandmarksPayload,
  resetContentPackCache,
} from "./content-packs";

jest.mock("./asset-reader", () => ({
  readJsonAssetSync: jest.fn(),
}));

jest.mock("./asset-manifest", () => ({
  assetExists: jest.fn(() => false),
}));

jest.mock("./paths", () => ({
  getDataDir: () => "/mock/assets/data",
}));

type PathPayloadMap = Record<string, unknown>;

const mockReadJson = readJsonAssetSync as jest.MockedFunction<typeof readJsonAssetSync>;
const mockAssetExists = jest.requireMock("./asset-manifest").assetExists as jest.MockedFunction<
  (filePath: string) => boolean
>;

const installJsonMock = (pathPayloads: PathPayloadMap): void => {
  mockReadJson.mockImplementation((filePath: string) => {
    const value = pathPayloads[filePath];
    if (value === undefined) {
      throw new Error(`Missing test payload for ${filePath}`);
    }
    return value;
  });
};

describe("content-packs merge behavior", () => {
  beforeEach(() => {
    mockReadJson.mockReset();
    mockAssetExists.mockReset();
    mockAssetExists.mockReturnValue(false);
    resetContentPackCache();
  });

  it("merges base pokemon data with per-pack additions and overrides", () => {
    installJsonMock({
      "/mock/assets/data/content-packs/index.json": {
        version: 1,
        packs: [
          {
            id: "johto-plus",
            enabled: true,
            priority: 50,
            path: "content-packs/johto-plus",
            files: {
              pokemon: [
                "content-packs/johto-plus/pokemon/totodile.json",
                "content-packs/johto-plus/pokemon/krabbyclaw.json",
              ],
            },
          },
        ],
      },
      "/mock/assets/data/pokemon_data.json": [
        { id: "TOTODILE", int_id: 158, base_exp: 66 },
        { id: "CYNDAQUIL", int_id: 155, base_exp: 65 },
      ],
      "/mock/assets/data/content-packs/johto-plus/pokemon/totodile.json": {
        id: "TOTODILE",
        int_id: 158,
        base_exp: 99,
      },
      "/mock/assets/data/content-packs/johto-plus/pokemon/krabbyclaw.json": {
        id: "KRABBYCLAW",
        int_id: 252,
        base_exp: 180,
      },
    });

    const merged = loadMergedPokemonDataSync();

    expect((merged.TOTODILE as { base_exp?: number }).base_exp).toBe(99);
    expect((merged.CYNDAQUIL as { base_exp?: number }).base_exp).toBe(65);
    expect((merged.KRABBYCLAW as { int_id?: number }).int_id).toBe(252);
  });

  it("merges map attributes and wild encounters from route/city packs", () => {
    installJsonMock({
      "/mock/assets/data/content-packs/index.json": {
        version: 1,
        packs: [
          {
            id: "map-pack",
            enabled: true,
            priority: 10,
            path: "content-packs/map-pack",
            files: {
              map_attributes: ["content-packs/map-pack/map_attributes/route_1.json"],
              wild_encounters: ["content-packs/map-pack/wild_encounters/new_route.json"],
            },
          },
        ],
      },
      "/mock/assets/data/map_attributes.json": {
        Route1: {
          tileset_name: "johto",
          border_block: 1,
          width: 10,
          height: 9,
          connections: [],
        },
      },
      "/mock/assets/data/content-packs/map-pack/map_attributes/route_1.json": {
        map_name: "Route1",
        tileset_name: "johto",
        border_block: 2,
        width: 12,
        height: 9,
        connections: [],
      },
      "/mock/assets/data/wild_encounters.json": [
        { map_name: "Route1", grass_rates: { day: 2 }, grass: null, water: null },
      ],
      "/mock/assets/data/content-packs/map-pack/wild_encounters/new_route.json": {
        map_name: "NewRoute",
        grass_rates: { day: 4 },
        grass: null,
        water: null,
      },
    });

    const attributes = loadMergedMapAttributesSync();
    const wild = loadMergedWildEncountersSync();

    expect((attributes.Route1 as { border_block?: number }).border_block).toBe(2);
    expect(
      wild.find((entry) => (entry as { map_name?: string }).map_name === "NewRoute")
    ).toBeTruthy();
  });

  it("loads enabled core-modular route data and lets higher-priority custom packs override it", () => {
    installJsonMock({
      "/mock/assets/data/content-packs/index.json": {
        version: 1,
        packs: [
          {
            id: "core-modular",
            enabled: true,
            priority: -100,
            files: {
              map_attributes: ["content-packs/core-modular/map_attributes/route29.json"],
              map_dimensions: ["content-packs/core-modular/map_dimensions/route29.json"],
              wild_encounters: ["content-packs/core-modular/wild_encounters/route_29.json"],
            },
          },
          {
            id: "module-route-route_29",
            enabled: false,
            priority: 0,
            files: {
              map_attributes: ["content-packs/core-modular/map_attributes/route29_disabled.json"],
              wild_encounters: ["content-packs/core-modular/wild_encounters/route_29_disabled.json"],
            },
          },
          {
            id: "route29-custom",
            enabled: true,
            priority: 10,
            files: {
              map_attributes: ["content-packs/route29-custom/map_attributes/route29.json"],
              wild_encounters: ["content-packs/route29-custom/wild_encounters/route_29.json"],
            },
          },
        ],
      },
      "/mock/assets/data/map_attributes.json": {},
      "/mock/assets/data/map_dimensions.json": {},
      "/mock/assets/data/wild_encounters.json": [],
      "/mock/assets/data/content-packs/core-modular/map_attributes/route29.json": {
        Route29: {
          tileset_name: "johto",
          border_block: 5,
          width: 30,
          height: 9,
          connections: [{ direction: "east", target_map: "NewBarkTown", offset: 0 }],
        },
      },
      "/mock/assets/data/content-packs/core-modular/map_dimensions/route29.json": {
        Route29: { width: 30, height: 9 },
      },
      "/mock/assets/data/content-packs/core-modular/wild_encounters/route_29.json": {
        map_name: "Route29",
        grass_rates: { day: 4 },
        grass: null,
        water: null,
      },
      "/mock/assets/data/content-packs/route29-custom/map_attributes/route29.json": {
        Route29: {
          tileset_name: "johto",
          border_block: 9,
          width: 30,
          height: 9,
          connections: [{ direction: "west", target_map: "CherrygroveCity", offset: 0 }],
        },
      },
      "/mock/assets/data/content-packs/route29-custom/wild_encounters/route_29.json": {
        map_name: "Route29",
        grass_rates: { day: 9 },
        grass: null,
        water: null,
      },
    });

    const attributes = loadMergedMapAttributesSync();
    const dimensions = loadMergedMapDimensionsSync();
    const wild = loadMergedWildEncountersSync();

    expect((attributes.Route29 as { border_block?: number }).border_block).toBe(9);
    expect((attributes.Route29 as { connections?: Array<{ target_map?: string }> }).connections?.[0].target_map).toBe(
      "CherrygroveCity"
    );
    expect(dimensions.Route29).toEqual({ width: 30, height: 9 });
    expect((wild[0] as { grass_rates?: { day?: number } }).grass_rates?.day).toBe(9);
    expect(mockReadJson).not.toHaveBeenCalledWith(
      "/mock/assets/data/content-packs/core-modular/map_attributes/route29_disabled.json"
    );
  });

  it("loads enabled compiled packs once and skips their individual files", () => {
    installJsonMock({
      "/mock/assets/data/content-packs/index.json": {
        version: 1,
        packs: [
          {
            id: "core-modular",
            enabled: true,
            priority: -100,
            compiled: "content-packs/core-modular.compiled.json",
            files: {
              pokemon: ["content-packs/core-modular/pokemon/totodile.json"],
              moves: ["content-packs/core-modular/moves/scratch.json"],
            },
          },
        ],
      },
      "/mock/assets/data/content-packs/core-modular.compiled.json": {
        version: 1,
        packId: "core-modular",
        categories: {
          pokemon: [{ id: "TOTODILE", base_exp: 66 }],
          moves: [{ name: "SCRATCH", type: "NORMAL" }],
        },
      },
    });

    expect(loadContentPackCategoryJsonSync("pokemon")).toEqual([
      { id: "TOTODILE", base_exp: 66 },
    ]);
    expect(loadContentPackCategoryJsonSync("moves")).toEqual([
      { name: "SCRATCH", type: "NORMAL" },
    ]);

    expect(mockReadJson).toHaveBeenCalledTimes(2);
    expect(mockReadJson).toHaveBeenCalledWith("/mock/assets/data/content-packs/index.json");
    expect(mockReadJson).toHaveBeenCalledWith("/mock/assets/data/content-packs/core-modular.compiled.json");
    expect(mockReadJson).not.toHaveBeenCalledWith(
      "/mock/assets/data/content-packs/core-modular/pokemon/totodile.json"
    );
    expect(mockReadJson).not.toHaveBeenCalledWith(
      "/mock/assets/data/content-packs/core-modular/moves/scratch.json"
    );
  });

  it("uses the bundled core compiled pack when the index omits compiled metadata", () => {
    mockAssetExists.mockImplementation(
      (filePath: string) => filePath === "/mock/assets/data/content-packs/core-modular.compiled.json"
    );
    installJsonMock({
      "/mock/assets/data/content-packs/index.json": {
        version: 1,
        packs: [
          {
            id: "core-modular",
            enabled: true,
            priority: -100,
            files: {
              maps: ["content-packs/core-modular/maps/route37.json"],
            },
          },
        ],
      },
      "/mock/assets/data/content-packs/core-modular.compiled.json": {
        version: 1,
        packId: "core-modular",
        categories: {
          maps: [
            {
              Route37_MapScripts: [{ command: "def_scene_scripts", args: [] }],
            },
          ],
        },
      },
    });

    expect(loadContentPackCategoryJsonSync("maps")).toEqual([
      {
        Route37_MapScripts: [{ command: "def_scene_scripts", args: [] }],
      },
    ]);
    expect(mockReadJson).not.toHaveBeenCalledWith(
      "/mock/assets/data/content-packs/core-modular/maps/route37.json"
    );
  });

  it("uses the bundled core compiled pack when the content-pack index is unavailable", () => {
    mockAssetExists.mockImplementation(
      (filePath: string) => filePath === "/mock/assets/data/content-packs/core-modular.compiled.json"
    );
    installJsonMock({
      "/mock/assets/data/content-packs/core-modular.compiled.json": {
        version: 1,
        packId: "core-modular",
        categories: {
          story_events: [
            {
              AzaleaTown: {
                AzaleaTownScript: [{ command: "end", args: [] }],
              },
            },
          ],
        },
      },
    });

    expect(loadContentPackCategoryJsonSync("story_events")).toEqual([
      {
        AzaleaTown: {
          AzaleaTownScript: [{ command: "end", args: [] }],
        },
      },
    ]);
    expect(mockReadJson).toHaveBeenCalledWith(
      "/mock/assets/data/content-packs/core-modular.compiled.json"
    );
  });

  it("merges NPC data from compiled content packs without reading per-map NPC files", () => {
    installJsonMock({
      "/mock/assets/data/content-packs/index.json": {
        version: 1,
        packs: [
          {
            id: "core-modular",
            enabled: true,
            priority: -100,
            compiled: "content-packs/core-modular.compiled.json",
            files: {
              npcs: ["content-packs/core-modular/npcs/cianwoodcity.json"],
            },
          },
        ],
      },
      "/mock/assets/data/npcs.json": {},
      "/mock/assets/data/content-packs/core-modular.compiled.json": {
        version: 1,
        packId: "core-modular",
        categories: {
          npcs: [
            {
              CianwoodCity: [
                {
                  script: "CianwoodCityChucksWife",
                  event_flag: "-1",
                  object_identifier: "CIANWOODCITY_POKEFAN_F",
                },
              ],
            },
          ],
        },
      },
    });

    expect(loadMergedNpcDataSync().CianwoodCity).toEqual([
      {
        script: "CianwoodCityChucksWife",
        event_flag: "-1",
        object_identifier: "CIANWOODCITY_POKEFAN_F",
      },
    ]);
    expect(mockReadJson).not.toHaveBeenCalledWith(
      "/mock/assets/data/content-packs/core-modular/npcs/cianwoodcity.json"
    );
  });

  it("applies non-compiled custom packs after compiled core packs by priority", () => {
    installJsonMock({
      "/mock/assets/data/content-packs/index.json": {
        version: 1,
        packs: [
          {
            id: "core-modular",
            enabled: true,
            priority: -100,
            compiled: "content-packs/core-modular.compiled.json",
            files: {
              pokemon: ["content-packs/core-modular/pokemon/totodile.json"],
            },
          },
          {
            id: "johto-plus",
            enabled: true,
            priority: 10,
            files: {
              pokemon: ["content-packs/johto-plus/pokemon/totodile.json"],
            },
          },
        ],
      },
      "/mock/assets/data/pokemon_data.json": [],
      "/mock/assets/data/content-packs/core-modular.compiled.json": {
        version: 1,
        packId: "core-modular",
        categories: {
          pokemon: [{ id: "TOTODILE", base_exp: 66 }],
        },
      },
      "/mock/assets/data/content-packs/johto-plus/pokemon/totodile.json": {
        id: "TOTODILE",
        base_exp: 99,
      },
    });

    expect((loadMergedPokemonDataSync().TOTODILE as { base_exp?: number }).base_exp).toBe(99);
    expect(mockReadJson).not.toHaveBeenCalledWith(
      "/mock/assets/data/content-packs/core-modular/pokemon/totodile.json"
    );
    expect(mockReadJson).toHaveBeenCalledWith(
      "/mock/assets/data/content-packs/johto-plus/pokemon/totodile.json"
    );
  });

  it("does not read disabled compiled packs", () => {
    installJsonMock({
      "/mock/assets/data/content-packs/index.json": {
        version: 1,
        packs: [
          {
            id: "module-pokemon-totodile",
            enabled: false,
            compiled: "content-packs/module-pokemon-totodile.compiled.json",
            files: {
              pokemon: ["content-packs/core-modular/pokemon/totodile.json"],
            },
          },
        ],
      },
    });

    expect(loadContentPackCategoryJsonSync("pokemon")).toEqual([]);
    expect(mockReadJson).toHaveBeenCalledTimes(1);
    expect(mockReadJson).not.toHaveBeenCalledWith(
      "/mock/assets/data/content-packs/module-pokemon-totodile.compiled.json"
    );
    expect(mockReadJson).not.toHaveBeenCalledWith(
      "/mock/assets/data/content-packs/core-modular/pokemon/totodile.json"
    );
  });

  it("throws a clear error when a declared compiled pack is missing or malformed", () => {
    installJsonMock({
      "/mock/assets/data/content-packs/index.json": {
        version: 1,
        packs: [
          {
            id: "core-modular",
            enabled: true,
            compiled: "content-packs/core-modular.compiled.json",
            files: {
              pokemon: ["content-packs/core-modular/pokemon/totodile.json"],
            },
          },
        ],
      },
      "/mock/assets/data/content-packs/core-modular.compiled.json": {
        version: 1,
        packId: "wrong-pack",
        categories: {
          pokemon: [],
        },
      },
    });

    expect(() => loadContentPackCategoryJsonSync("pokemon")).toThrow(
      "Unable to load compiled content pack core-modular from /mock/assets/data/content-packs/core-modular.compiled.json"
    );
  });

  it("uses a disabled route module only when it is explicitly enabled", () => {
    installJsonMock({
      "/mock/assets/data/content-packs/index.json": {
        version: 1,
        packs: [
          {
            id: "module-route-route_29",
            enabled: true,
            files: {
              map_attributes: ["content-packs/core-modular/map_attributes/route29.json"],
            },
          },
        ],
      },
      "/mock/assets/data/map_attributes.json": {},
      "/mock/assets/data/content-packs/core-modular/map_attributes/route29.json": {
        Route29: {
          tileset_name: "johto",
          border_block: 5,
          width: 30,
          height: 9,
          connections: [],
        },
      },
    });

    expect((loadMergedMapAttributesSync().Route29 as { border_block?: number }).border_block).toBe(5);
  });

  it("merges map block payloads from packs", () => {
    installJsonMock({
      "/mock/assets/data/content-packs/index.json": {
        version: 1,
        packs: [
          {
            id: "map-pack",
            enabled: true,
            files: {
              map_blocks: ["content-packs/map-pack/map_blocks/new_route_blocks.json"],
            },
          },
        ],
      },
      "/mock/assets/data/map_blocks.json": {
        Route29_Blocks: "base",
      },
      "/mock/assets/data/content-packs/map-pack/map_blocks/new_route_blocks.json": {
        NewRoute_Blocks: "custom",
        Route29_Blocks: "override",
      },
    });

    expect(loadMergedMapBlocksSync()).toEqual({
      Route29_Blocks: "override",
      NewRoute_Blocks: "custom",
    });
  });

  it("merges NPC data from per-map content-pack payloads", () => {
    installJsonMock({
      "/mock/assets/data/content-packs/index.json": {
        version: 1,
        packs: [
          {
            id: "npc-pack",
            enabled: true,
            priority: 10,
            path: "content-packs/npc-pack",
            files: {
              npcs: [
                "content-packs/npc-pack/npcs/route1.json",
                "content-packs/npc-pack/npcs/new_route.json",
              ],
            },
          },
        ],
      },
      "/mock/assets/data/npcs.json": {
        Route1: [{ script: "OldRoute1Npc" }],
        Route2: [{ script: "Route2Npc" }],
      },
      "/mock/assets/data/content-packs/npc-pack/npcs/route1.json": {
        Route1: [{ script: "NewRoute1Npc" }],
      },
      "/mock/assets/data/content-packs/npc-pack/npcs/new_route.json": {
        NewRoute: [{ script: "NewRouteNpc" }],
      },
    });

    const merged = loadMergedNpcDataSync();

    expect(merged.Route1).toEqual([{ script: "NewRoute1Npc" }]);
    expect(merged.Route2).toEqual([{ script: "Route2Npc" }]);
    expect(merged.NewRoute).toEqual([{ script: "NewRouteNpc" }]);
  });

  it("defaults missing NPC file lists to an empty category for legacy indexes", () => {
    installJsonMock({
      "/mock/assets/data/content-packs/index.json": {
        version: 1,
        packs: [
          {
            id: "legacy-pack",
            enabled: true,
            priority: 10,
            files: {
              pokemon: ["content-packs/legacy-pack/pokemon/chikorita.json"],
            },
          },
        ],
      },
    });

    expect(listContentPackFilesSync("npcs")).toEqual([]);
  });

  it("merges pokegear landmarks and map-to-landmark mappings", () => {
    installJsonMock({
      "/mock/assets/data/content-packs/index.json": {
        version: 1,
        packs: [
          {
            id: "city-pack",
            enabled: true,
            priority: 20,
            path: "content-packs/city-pack",
            files: {
              pokegear_landmarks: ["content-packs/city-pack/pokegear_landmarks/cities.json"],
            },
          },
        ],
      },
      "/mock/assets/data/content-packs/city-pack/pokegear_landmarks/cities.json": {
        landmarks: [
          { id: 200, constant: "NEW_CITY", name: "New City", label: "NEW CITY", x: 1, y: 2, region: "JOHTO" },
          { id: 14, constant: "VIOLET_CITY", name: "Violet City+", label: "VIOLET CITY", x: 9, y: 9, region: "JOHTO" },
        ],
        map_to_landmark: {
          NEW_CITY: "NEW_CITY",
          VIOLET_CITY: "VIOLET_CITY",
        },
      },
    });

    const merged = mergePokegearLandmarksPayload({
      landmarks: [
        { id: 14, constant: "VIOLET_CITY", name: "Violet City", label: "VIOLET CITY", x: 3, y: 4, region: "JOHTO" },
      ],
      map_to_landmark: {
        VIOLET_CITY: "VIOLET_CITY_OLD",
      },
    });

    const violet = merged.landmarks.find((entry) => entry.constant === "VIOLET_CITY");
    const newCity = merged.landmarks.find((entry) => entry.constant === "NEW_CITY");

    expect(violet?.name).toBe("Violet City+");
    expect(newCity?.name).toBe("New City");
    expect(merged.map_to_landmark.VIOLET_CITY).toBe("VIOLET_CITY");
    expect(merged.map_to_landmark.NEW_CITY).toBe("NEW_CITY");
  });

  it("merges items so route packs can ship custom item drops/rewards", () => {
    installJsonMock({
      "/mock/assets/data/content-packs/index.json": {
        version: 1,
        packs: [
          {
            id: "route-item-pack",
            enabled: true,
            files: {
              items: ["content-packs/route-item-pack/items/route_reward.json"],
            },
          },
        ],
      },
      "/mock/assets/data/items.json": [
        { name: "POTION", price: 300 },
      ],
      "/mock/assets/data/content-packs/route-item-pack/items/route_reward.json": {
        name: "POTION",
        price: 999,
      },
    });

    const merged = loadMergedItemsSync();
    const potion = merged.find((entry) => (entry as { name?: string }).name === "POTION") as {
      price?: number;
    };

    expect(potion.price).toBe(999);
  });

  it("merges trainer and pokedex payloads from packs", () => {
    installJsonMock({
      "/mock/assets/data/content-packs/index.json": {
        version: 1,
        packs: [
          {
            id: "trainer-pack",
            enabled: true,
            files: {
              trainers: ["content-packs/trainer-pack/trainers/youngster_joe.json"],
              pokedex: ["content-packs/trainer-pack/pokedex/totodile.json"],
            },
          },
        ],
      },
      "/mock/assets/data/trainers.json": [{ trainer_id: "YOUNGSTER_JOE", name: "Youngster Joe", team: [] }],
      "/mock/assets/data/pokedex.json": [{ species: "TOTODILE", text: "Old text." }],
      "/mock/assets/data/content-packs/trainer-pack/trainers/youngster_joe.json": {
        trainer_id: "YOUNGSTER_JOE",
        name: "Youngster Joseph",
        team: [],
      },
      "/mock/assets/data/content-packs/trainer-pack/pokedex/totodile.json": {
        species: "TOTODILE",
        text: "New text.",
      },
    });

    const trainers = loadMergedTrainersSync();
    const pokedex = loadMergedPokedexSync();

    expect((trainers[0] as { name?: string }).name).toBe("Youngster Joseph");
    expect((pokedex[0] as { text?: string }).text).toBe("New text.");
  });

  it("hydrates compact trainer party species references from merged pokemon data", () => {
    installJsonMock({
      "/mock/assets/data/content-packs/index.json": {
        version: 1,
        packs: [
          {
            id: "trainer-pack",
            enabled: true,
            files: {
              trainers: ["content-packs/trainer-pack/trainers/youngster_joe.json"],
            },
          },
        ],
      },
      "/mock/assets/data/trainers.json": [],
      "/mock/assets/data/pokemon_data.json": [
        {
          id: "TOTODILE",
          base_stats: { hp: 50 },
        },
      ],
      "/mock/assets/data/content-packs/trainer-pack/trainers/youngster_joe.json": {
        trainer_id: "YOUNGSTER_JOE",
        name: "Youngster Joe",
        party: [{ species: "TOTODILE", level: 8 }],
      },
    });

    const trainers = loadMergedTrainersSync();
    const trainer = trainers[0] as { party?: Array<Record<string, unknown>> };
    const partyMember = trainer.party?.[0] as {
      species?: { id?: string; base_stats?: { hp?: number } };
      nickname?: string;
      hp?: number;
      max_hp?: number;
      moves?: unknown[];
    };

    expect(partyMember.species?.id).toBe("TOTODILE");
    expect(partyMember.species?.base_stats?.hp).toBe(50);
    expect(partyMember.nickname).toBe("TOTODILE");
    expect(partyMember.hp).toBe(50);
    expect(partyMember.max_hp).toBe(50);
    expect(partyMember.moves).toEqual([]);
  });

  it("resolves compact trainer species from pokemon content-pack overrides", () => {
    installJsonMock({
      "/mock/assets/data/content-packs/index.json": {
        version: 1,
        packs: [
          {
            id: "custom-pack",
            enabled: true,
            files: {
              pokemon: ["content-packs/custom-pack/pokemon/krabbyclaw.json"],
              trainers: ["content-packs/custom-pack/trainers/cooltrainer.json"],
            },
          },
        ],
      },
      "/mock/assets/data/trainers.json": [],
      "/mock/assets/data/pokemon_data.json": [],
      "/mock/assets/data/content-packs/custom-pack/pokemon/krabbyclaw.json": {
        id: "KRABBYCLAW",
        base_stats: { hp: 80 },
      },
      "/mock/assets/data/content-packs/custom-pack/trainers/cooltrainer.json": {
        trainer_id: "COOLTRAINER",
        name: "Cooltrainer",
        party: [{ species: "KRABBYCLAW", level: 20, nickname: "CLAW" }],
      },
    });

    const trainers = loadMergedTrainersSync();
    const partyMember = (trainers[0] as { party?: Array<Record<string, unknown>> }).party?.[0] as {
      species?: { id?: string };
      nickname?: string;
      hp?: number;
    };

    expect(partyMember.species?.id).toBe("KRABBYCLAW");
    expect(partyMember.nickname).toBe("CLAW");
    expect(partyMember.hp).toBe(80);
  });

  it("keeps expanded trainer party species payloads compatible", () => {
    installJsonMock({
      "/mock/assets/data/content-packs/index.json": {
        version: 1,
        packs: [
          {
            id: "trainer-pack",
            enabled: true,
            files: {
              trainers: ["content-packs/trainer-pack/trainers/expanded.json"],
            },
          },
        ],
      },
      "/mock/assets/data/trainers.json": [],
      "/mock/assets/data/content-packs/trainer-pack/trainers/expanded.json": {
        trainer_id: "EXPANDED",
        name: "Expanded",
        party: [
          {
            species: { id: "CHIKORITA", base_stats: { hp: 45 } },
            nickname: "CHIKORITA",
            level: 5,
            hp: 45,
            max_hp: 45,
          },
        ],
      },
    });

    const trainers = loadMergedTrainersSync();
    const partyMember = (trainers[0] as { party?: Array<Record<string, unknown>> }).party?.[0] as {
      species?: { id?: string };
    };

    expect(partyMember.species?.id).toBe("CHIKORITA");
  });

  it("throws a clear error for compact trainer references to missing species", () => {
    installJsonMock({
      "/mock/assets/data/content-packs/index.json": {
        version: 1,
        packs: [
          {
            id: "trainer-pack",
            enabled: true,
            files: {
              trainers: ["content-packs/trainer-pack/trainers/missing.json"],
            },
          },
        ],
      },
      "/mock/assets/data/trainers.json": [],
      "/mock/assets/data/pokemon_data.json": [],
      "/mock/assets/data/content-packs/trainer-pack/trainers/missing.json": {
        trainer_id: "MISSING",
        name: "Missing",
        party: [{ species: "MISSINGNO", level: 1 }],
      },
    });

    expect(() => loadMergedTrainersSync()).toThrow(
      "Unable to resolve species MISSINGNO for trainer MISSING content-pack party entry."
    );
  });
});
