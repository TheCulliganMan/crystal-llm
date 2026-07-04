import fs from "fs";
import { exportCoreContentPack } from "./export-content-pack";

const mockWriteJsonToTargets = jest.fn();
const mockRemoveMatchingOutputs = jest.fn();
const mockEnsureDir = jest.fn();
const mockReadJsonAssetSync = jest.fn();

const TEST_CONTENT_PACK_CATEGORIES = [
  "pokemon",
  "moves",
  "growth_rates",
  "learnsets",
  "level_up_moves",
  "egg_moves",
  "evolutions",
  "maps",
  "map_scripts",
  "map_blocks",
  "map_attributes",
  "map_dimensions",
  "wild_encounters",
  "field_encounters",
  "runtime_spawn_points",
  "fly_destinations",
  "runtime_map_metadata",
  "flee_mons",
  "roaming_pokemon",
  "buena_password_categories",
  "buena_prizes",
  "kurt_apricorn_recipes",
  "shuckie_gift",
  "dratini_move_sets",
  "bug_contest_config",
  "battle_tower_rules",
  "oak_ratings",
  "odd_egg_definitions",
  "magikarp_lengths",
  "happiness_data",
  "encounter_slot_tables",
  "encounter_music_modifiers",
  "battle_stat_multipliers",
  "capture_wobble_probabilities",
  "capture_rules",
  "move_priorities",
  "type_categories",
  "type_effectiveness",
  "weather_modifiers",
  "battle_reward_rules",
  "battle_escape_rules",
  "step_event_rules",
  "fishing",
  "field_moves",
  "field_box_items",
  "runtime_title_screen",
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
  overrides: Partial<
    Record<(typeof TEST_CONTENT_PACK_CATEGORIES)[number], string[]>
  >,
): Record<(typeof TEST_CONTENT_PACK_CATEGORIES)[number], string[]> =>
  ({
    ...Object.fromEntries(
      TEST_CONTENT_PACK_CATEGORIES.map((category) => [category, []]),
    ),
    ...overrides,
  }) as Record<(typeof TEST_CONTENT_PACK_CATEGORIES)[number], string[]>;

const titleMusicAudioAsset = {
  MUSIC_TITLE: {
    id: "MUSIC_TITLE",
    path: "content-packs/core-modular/music/MUSIC_TITLE.mid",
    kind: "music" as const,
    source: "midi" as const,
  },
};

const titleRuntimeSpawnPoints = {
  "0": {
    identifier: 0,
    mapConstant: "PLAYERS_HOUSE_2F",
    mapName: "PlayersHouse2F",
    groupId: 24,
    mapId: 7,
    tileX: 3,
    tileY: 3,
    groupName: "NEW_BARK",
    metatileX: 1,
    metatileY: 1,
    subtileX: 1,
    subtileY: 1,
  },
};

const mockStrictIndexAndEmptyMapBlocks = (
  index = { version: 1, packs: [] as unknown[] },
): void => {
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
  removeMatchingOutputs: (...args: unknown[]) =>
    mockRemoveMatchingOutputs(...args),
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
    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) =>
      String(pathLike).endsWith(".mid"),
    );
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike) => {
      if (String(pathLike).endsWith(".mid")) {
        return Buffer.from("MThd0000");
      }
      throw new Error(`Unexpected readFileSync ${String(pathLike)}`);
    });
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
      }),
    ).toThrow("Content pack file stem must be a single exact path segment");
  });

  it("rejects audio asset keys that do not match record ids", () => {
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
        trainers: [],
        pokedex: [],
        npcData: {},
        pokegearLandmarks: { landmarks: [], map_to_landmark: {} },
        audioAssets: {
          MUSIC_ROUTE_29: {
            id: "MUSIC_ROUTE_30",
            path: "content-packs/core-modular/music/MUSIC_ROUTE_30.mid",
            kind: "music",
            source: "midi",
          },
        },
      }),
    ).toThrow("Audio asset key MUSIC_ROUTE_29 does not match record id MUSIC_ROUTE_30");
  });

  it("rejects reserved audio asset ids before writing generated files", () => {
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
        trainers: [],
        pokedex: [],
        npcData: {},
        pokegearLandmarks: { landmarks: [], map_to_landmark: {} },
        audioAssets: {
          MUSIC_LEGACY_ROUTE_29: {
            id: "MUSIC_LEGACY_ROUTE_29",
            path: "content-packs/core-modular/music/MUSIC_LEGACY_ROUTE_29.mid",
            kind: "music",
            source: "midi",
          },
        },
      }),
    ).toThrow("Audio asset id MUSIC_LEGACY_ROUTE_29 must be an exact pack audio id");
  });

  it("rejects audio assets in a directory that does not match their kind", () => {
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
        trainers: [],
        pokedex: [],
        npcData: {},
        pokegearLandmarks: { landmarks: [], map_to_landmark: {} },
        audioAssets: {
          CRY_NIDORAN_M: {
            id: "CRY_NIDORAN_M",
            path: "content-packs/core-modular/sfx/CRY_NIDORAN_M.mid",
            kind: "cry",
            source: "midi",
          },
        },
      }),
    ).toThrow(
      "Audio asset CRY_NIDORAN_M must live under cries: content-packs/core-modular/sfx/CRY_NIDORAN_M.mid",
    );
  });

  it("rejects audio asset paths whose file stem does not exactly match the id", () => {
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
        trainers: [],
        pokedex: [],
        npcData: {},
        pokegearLandmarks: { landmarks: [], map_to_landmark: {} },
        audioAssets: {
          SFX_TACKLE: {
            id: "SFX_TACKLE",
            path: "content-packs/core-modular/sfx/SFX_POUND.mid",
            kind: "sound_effect",
            source: "midi",
          },
        },
      }),
    ).toThrow(
      "Audio asset SFX_TACKLE path must end with SFX_TACKLE.mid: content-packs/core-modular/sfx/SFX_POUND.mid",
    );
  });

  it("rejects audio metadata when the generated MIDI payload is missing", () => {
    mockStrictIndexAndEmptyMapBlocks();
    jest.spyOn(fs, "existsSync").mockImplementation(() => false);

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
        audioAssets: {
          MUSIC_ROUTE_29: {
            id: "MUSIC_ROUTE_29",
            path: "content-packs/core-modular/music/MUSIC_ROUTE_29.mid",
            kind: "music",
            source: "midi",
          },
        },
      }),
    ).toThrow(
      "Audio asset MUSIC_ROUTE_29 is missing generated MIDI file: content-packs/core-modular/music/MUSIC_ROUTE_29.mid",
    );
  });

  it("rejects generated audio payloads that are not MIDI files", () => {
    mockStrictIndexAndEmptyMapBlocks();
    jest.spyOn(fs, "readFileSync").mockImplementation((pathLike) => {
      if (String(pathLike).endsWith(".mid")) {
        return Buffer.from("NOPE");
      }
      throw new Error(`Unexpected readFileSync ${String(pathLike)}`);
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
        audioAssets: {
          MUSIC_ROUTE_29: {
            id: "MUSIC_ROUTE_29",
            path: "content-packs/core-modular/music/MUSIC_ROUTE_29.mid",
            kind: "music",
            source: "midi",
          },
        },
      }),
    ).toThrow(
      "Audio asset MUSIC_ROUTE_29 generated MIDI file must start with MThd: content-packs/core-modular/music/MUSIC_ROUTE_29.mid",
    );
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
            map_attributes: [
              "content-packs/city-pack/map_attributes/goldenrod.json",
            ],
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
      fleeMons: { buckets: { always: ["RAIKOU"], often: [], sometimes: [] } },
      roamingPokemon: { RAIKOU: { level: 40, mapGroup: 2, mapNumber: 5 } },
      buenaPasswordCategories: {
        order: ["HealingItems"],
        categories: {
          HealingItems: {
            categoryType: "BUENA_ITEM",
            points: 12,
            options: ["POTION", "ANTIDOTE", "PARLYZ_HEAL"],
          },
        },
      },
      buenaPrizes: { RARE_CANDY: 3 },
      kurtApricornRecipes: { RED_APRICORN: "LEVEL_BALL" },
      shuckieGift: {
        species: "SHUCKLE",
        level: 15,
        heldItem: "BERRY",
        nickname: "SHUCKIE",
        originalTrainerName: "MANIA",
        originalTrainerId: 518,
        gotTodayEngineFlag: "ENGINE_GOT_SHUCKIE_TODAY",
      },
      dratiniMoveSets: {
        "0": ["WRAP", "THUNDER_WAVE", "TWISTER", "EXTREMESPEED"],
      },
      bugContestConfig: {
        parkBalls: 20,
        timerMinutes: 20,
        timerSeconds: 0,
        selectedContestantCount: 5,
        contestantFlags: [
          "EVENT_BUG_CATCHING_CONTESTANT_1A",
          "EVENT_BUG_CATCHING_CONTESTANT_2A",
        ],
      },
      battleTowerRules: {
        bannedSpecies: {
          MEWTWO: {},
          MEW: {},
          LUGIA: {},
          HO_OH: {},
          CELEBI: {},
        },
        requiredPartyCount: 3,
        challengeStreakLength: 7,
        minimumLevelGroup: 1,
        maximumLevelGroup: 10,
        levelGroupSize: 10,
        partyCountFailureText: "OnlyThreeMonMayBeEnteredText",
        duplicateSpeciesFailureText: "TheMonMustAllBeDifferentKindsText",
        duplicateHeldItemFailureText: "TheMonMustNotHoldTheSameItemsText",
        eggFailureText: "YouCantTakeAnEggText",
      },
      oakRatings: [
        {
          caughtCountLimit: 9,
          fanfare: "SFX_DEX_FANFARE_LESS_THAN_20",
          textLabel: "OakRating01",
        },
      ],
      oddEggDefinitions: [
        {
          species: "CLEFFA",
          moves: ["POUND", "CHARM", "DIZZY_PUNCH"],
          originalTrainerId: 768,
          dvs: [2, 10, 10, 10],
          probability: 100,
          level: 5,
          experience: 125,
          hatchCycles: 20,
          nickname: "EGG",
          originalTrainerName: "ODD",
        },
      ],
      magikarpLengths: [
        { threshold: 110, divisor: 1 },
        { threshold: 310, divisor: 2 },
      ],
      happinessData: {
        changes: {
          "18": { code: "HAPPINESS_GROOMING", low: 3, mid: 3, high: 1 },
        },
        services: {
          DaisysGrooming: [{ rollWeight: 255, scriptValue: 2, changeCode: 18 }],
        },
      },
      encounterSlotTables: {
        grass: [{ threshold: 100, slot: 0 }],
        water: [{ threshold: 100, slot: 0 }],
      },
      encounterMusicModifiers: {
        modifiers: { MUSIC_POKEMON_MARCH: { numerator: 2, denominator: 1 } },
      },
      battleStatMultipliers: {
        stat: [{ numerator: 1, denominator: 1 }],
        accuracy: [{ numerator: 1, denominator: 1 }],
      },
      captureWobbleProbabilities: [{ catch_rate: 255, chance: 255 }],
      captureRules: {
        fast_ball_species: ["MAGNEMITE"],
        heavy_ball_modifiers: { MAGNEMITE: 0 },
        ball_rules: {
          POKE_BALL: {
            multiplier_numerator: 1,
            multiplier_denominator: 1,
            battle_type: "",
            skip_hp_calc: false,
            use_heavy_ball_weight_modifier: false,
            use_level_ball_multiplier: false,
            require_same_species: false,
            require_same_gender: false,
            require_fast_species: false,
          },
        },
        guaranteed_capture_balls: [],
        status_bonus: { SLEEP: 10, FREEZE: 10 },
      },
      movePriorities: {
        base_priority: 1,
        effect_priorities: { PRIORITY_HIT: 2 },
        move_priorities: [{ move: "VITAL_THROW", priority: 0 }],
      },
      typeCategories: {
        physical: ["NORMAL", "FIGHTING"],
        special: ["FIRE", "WATER"],
      },
      typeEffectiveness: {
        matchups: { FIRE: { GRASS: { numerator: 2, denominator: 1 } } },
        foresight_matchups: {
          NORMAL: { GHOST: { numerator: 0, denominator: 1 } },
        },
      },
      weatherModifiers: {
        type_modifiers: [
          {
            weather: "WEATHER_RAIN",
            move_type: "WATER",
            multiplier: { numerator: 3, denominator: 2 },
          },
        ],
        move_effect_modifiers: [],
      },
      battleRewardRules: {
        max_level: 100,
        wild_exp_divisor: 7,
        trainer_exp_numerator: 3,
        trainer_exp_denominator: 2,
      },
      battleEscapeRules: {
        player_speed_multiplier: 32,
        enemy_speed_divisor: 4,
        failed_attempt_bonus: 30,
        rng_roll_values: 256,
      },
      stepEventRules: {
        poison_step_interval: 4,
        egg_step_trigger: 128,
        hatched_egg_happiness: 120,
        poison_status: "POISON",
        egg_nickname: "EGG",
        happiness_step_counter_mask: 1,
        happiness_step_counter_target: 0,
      },
      fruitTrees: { FRUITTREE_ROUTE_29: "BERRY" },
      currencyConstants: { MAX_MONEY: 999999, MAX_COINS: 9999 },
      mapDimensions: { ROUTE_1: { width: 10, height: 8 } },
      mapAttributes: {
        Route1: { environment: "TOWN", map_constant: "ROUTE_1" },
      },
      items: [
        { name: "$00", script_name: "$00", price: 9999 } as never,
        { name: "POTION", script_name: "POTION", price: 300 } as never,
      ],
      pcStrings: { PCString_ChooseaPKMN: "Choose a PKMN." },
      menuIcons: { TOTODILE: "ICON_TOTODILE" },
      trainers: [
        { trainer_id: "YOUNGSTER_JOE", name: "Youngster Joe" } as never,
      ],
      pokedex: [{ species: "TOTODILE", text: "A tiny croc." } as never],
      pokedexEntries: {
        TOTODILE: {
          species: "TOTODILE",
          classification: "BIG_JAW",
          heightDigits: 200,
          weightDigits: 210,
          pages: ["It has sharp fangs."],
        },
      },
      pokemonFrontpicAnimations: {
        TOTODILE: { commands: [{ kind: "endanim" }] },
      },
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
      runtimeSpawnPoints: titleRuntimeSpawnPoints,
      audioAssets: {
        ...titleMusicAudioAsset,
        MUSIC_ROUTE_29: {
          id: "MUSIC_ROUTE_29",
          path: "content-packs/core-modular/music/MUSIC_ROUTE_29.mid",
          kind: "music",
          source: "midi",
        },
      },
    });

    expect(mockRemoveMatchingOutputs).toHaveBeenCalledWith(
      "content-packs/core-modular",
      ".json",
    );
    expect(mockRemoveMatchingOutputs).toHaveBeenCalledWith(
      "content-packs/core-modular/music",
      ".json",
    );
    expect(mockRemoveMatchingOutputs).toHaveBeenCalledWith(
      "content-packs/core-modular/sfx",
      ".json",
    );
    expect(mockRemoveMatchingOutputs).toHaveBeenCalledWith(
      "content-packs/core-modular/cries",
      ".json",
    );
    expect(fs.rmSync).not.toHaveBeenCalled();
    expect(fs.writeFileSync).not.toHaveBeenCalledWith(
      "/mock/assets/data/content-packs/core-modular/music/MUSIC_ROUTE_29.mid",
      expect.any(Buffer),
    );
    expect(mockWriteJsonToTargets).toHaveBeenCalledWith(
      "content-packs/core-modular/music/MUSIC_ROUTE_29.json",
      {
        MUSIC_ROUTE_29: {
          id: "MUSIC_ROUTE_29",
          path: "content-packs/core-modular/music/MUSIC_ROUTE_29.mid",
          kind: "music",
          source: "midi",
        },
      },
      { indent: 2 },
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
      { indent: 2 },
    );
    const compiledCall = mockWriteJsonToTargets.mock.calls.find(
      (call) => call[0] === "content-packs/core-modular.compiled.json",
    );
    expect(compiledCall?.[1]).toEqual(
      expect.objectContaining({
        version: 1,
        packId: "core-modular",
        categories: expect.objectContaining({
          pokemon: [{ TOTODILE: { id: "TOTODILE" } }],
          moves: [{ SCRATCH: { name: "SCRATCH", type: "NORMAL" } }],
          learnsets: [
            { TOTODILE: { species: "TOTODILE", learnset: [[1, "SCRATCH"]] } },
          ],
          level_up_moves: [
            {
              TOTODILE: {
                species: "TOTODILE",
                moves: [{ level: 1, move: "SCRATCH" }],
              },
            },
          ],
          egg_moves: [{ TOTODILE: { species: "TOTODILE", moves: ["CRUNCH"] } }],
          evolutions: [{ TOTODILE: { species: "TOTODILE" } }],
          map_attributes: [
            { Route1: { environment: "TOWN", map_constant: "ROUTE_1" } },
          ],
          map_dimensions: [{ ROUTE_1: { width: 10, height: 8 } }],
          wild_encounters: [{ Route1: { map_name: "Route1" } }],
          flee_mons: [
            { buckets: { always: ["RAIKOU"], often: [], sometimes: [] } },
          ],
          roaming_pokemon: [
            { RAIKOU: { level: 40, mapGroup: 2, mapNumber: 5 } },
          ],
          buena_password_categories: [
            {
              order: ["HealingItems"],
              categories: {
                HealingItems: {
                  categoryType: "BUENA_ITEM",
                  points: 12,
                  options: ["POTION", "ANTIDOTE", "PARLYZ_HEAL"],
                },
              },
            },
          ],
          buena_prizes: [{ RARE_CANDY: 3 }],
          kurt_apricorn_recipes: [{ RED_APRICORN: "LEVEL_BALL" }],
          shuckie_gift: [
            {
              species: "SHUCKLE",
              level: 15,
              heldItem: "BERRY",
              nickname: "SHUCKIE",
              originalTrainerName: "MANIA",
              originalTrainerId: 518,
              gotTodayEngineFlag: "ENGINE_GOT_SHUCKIE_TODAY",
            },
          ],
          dratini_move_sets: [
            { "0": ["WRAP", "THUNDER_WAVE", "TWISTER", "EXTREMESPEED"] },
          ],
          bug_contest_config: [
            {
              parkBalls: 20,
              timerMinutes: 20,
              timerSeconds: 0,
              selectedContestantCount: 5,
              contestantFlags: [
                "EVENT_BUG_CATCHING_CONTESTANT_1A",
                "EVENT_BUG_CATCHING_CONTESTANT_2A",
              ],
            },
          ],
          battle_tower_rules: [
            {
              bannedSpecies: {
                MEWTWO: {},
                MEW: {},
                LUGIA: {},
                HO_OH: {},
                CELEBI: {},
              },
              requiredPartyCount: 3,
              challengeStreakLength: 7,
              minimumLevelGroup: 1,
              maximumLevelGroup: 10,
              levelGroupSize: 10,
              partyCountFailureText: "OnlyThreeMonMayBeEnteredText",
              duplicateSpeciesFailureText: "TheMonMustAllBeDifferentKindsText",
              duplicateHeldItemFailureText: "TheMonMustNotHoldTheSameItemsText",
              eggFailureText: "YouCantTakeAnEggText",
            },
          ],
          oak_ratings: [
            [
              {
                caughtCountLimit: 9,
                fanfare: "SFX_DEX_FANFARE_LESS_THAN_20",
                textLabel: "OakRating01",
              },
            ],
          ],
          odd_egg_definitions: [
            [
              {
                species: "CLEFFA",
                moves: ["POUND", "CHARM", "DIZZY_PUNCH"],
                originalTrainerId: 768,
                dvs: [2, 10, 10, 10],
                probability: 100,
                level: 5,
                experience: 125,
                hatchCycles: 20,
                nickname: "EGG",
                originalTrainerName: "ODD",
              },
            ],
          ],
          magikarp_lengths: [
            [
              { threshold: 110, divisor: 1 },
              { threshold: 310, divisor: 2 },
            ],
          ],
          happiness_data: [
            {
              changes: {
                "18": { code: "HAPPINESS_GROOMING", low: 3, mid: 3, high: 1 },
              },
              services: {
                DaisysGrooming: [
                  { rollWeight: 255, scriptValue: 2, changeCode: 18 },
                ],
              },
            },
          ],
          encounter_slot_tables: [
            {
              grass: [{ threshold: 100, slot: 0 }],
              water: [{ threshold: 100, slot: 0 }],
            },
          ],
          encounter_music_modifiers: [
            {
              modifiers: {
                MUSIC_POKEMON_MARCH: { numerator: 2, denominator: 1 },
              },
            },
          ],
          battle_stat_multipliers: [
            {
              stat: [{ numerator: 1, denominator: 1 }],
              accuracy: [{ numerator: 1, denominator: 1 }],
            },
          ],
          capture_wobble_probabilities: [[{ catch_rate: 255, chance: 255 }]],
          capture_rules: [
            {
              fast_ball_species: ["MAGNEMITE"],
              heavy_ball_modifiers: { MAGNEMITE: 0 },
              ball_rules: {
                POKE_BALL: {
                  multiplier_numerator: 1,
                  multiplier_denominator: 1,
                  battle_type: "",
                  skip_hp_calc: false,
                  use_heavy_ball_weight_modifier: false,
                  use_level_ball_multiplier: false,
                  require_same_species: false,
                  require_same_gender: false,
                  require_fast_species: false,
                },
              },
              guaranteed_capture_balls: [],
              status_bonus: { SLEEP: 10, FREEZE: 10 },
            },
          ],
          move_priorities: [
            {
              base_priority: 1,
              effect_priorities: { PRIORITY_HIT: 2 },
              move_priorities: [{ move: "VITAL_THROW", priority: 0 }],
            },
          ],
          type_categories: [
            {
              physical: ["NORMAL", "FIGHTING"],
              special: ["FIRE", "WATER"],
            },
          ],
          type_effectiveness: [
            {
              matchups: { FIRE: { GRASS: { numerator: 2, denominator: 1 } } },
              foresight_matchups: {
                NORMAL: { GHOST: { numerator: 0, denominator: 1 } },
              },
            },
          ],
          weather_modifiers: [
            {
              type_modifiers: [
                {
                  weather: "WEATHER_RAIN",
                  move_type: "WATER",
                  multiplier: { numerator: 3, denominator: 2 },
                },
              ],
              move_effect_modifiers: [],
            },
          ],
          battle_reward_rules: [
            {
              max_level: 100,
              wild_exp_divisor: 7,
              trainer_exp_numerator: 3,
              trainer_exp_denominator: 2,
            },
          ],
          battle_escape_rules: [
            {
              player_speed_multiplier: 32,
              enemy_speed_divisor: 4,
              failed_attempt_bonus: 30,
              rng_roll_values: 256,
            },
          ],
          step_event_rules: [
            {
              poison_step_interval: 4,
              egg_step_trigger: 128,
              hatched_egg_happiness: 120,
              poison_status: "POISON",
              egg_nickname: "EGG",
              happiness_step_counter_mask: 1,
              happiness_step_counter_target: 0,
            },
          ],
          fishing: [],
          fruit_trees: [{ FRUITTREE_ROUTE_29: "BERRY" }],
          currency_constants: [{ MAX_MONEY: 999999, MAX_COINS: 9999 }],
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
          items: [
            { POTION: { name: "POTION", script_name: "POTION", price: 300 } },
          ],
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
          pokedex_entries: [
            {
              TOTODILE: {
                species: "TOTODILE",
                classification: "BIG_JAW",
                heightDigits: 200,
                weightDigits: 210,
                pages: ["It has sharp fangs."],
              },
            },
          ],
          pokemon_frontpic_anim: [
            { TOTODILE: { commands: [{ kind: "endanim" }] } },
          ],
          runtime_title_screen: [
            {
              new_game_spawn_identifier: 0,
              title_music: "MUSIC_TITLE",
            },
          ],
          audio: expect.arrayContaining([
            {
              MUSIC_TITLE: {
                id: "MUSIC_TITLE",
                path: "content-packs/core-modular/music/MUSIC_TITLE.mid",
                kind: "music",
                source: "midi",
              },
            },
            {
              MUSIC_ROUTE_29: {
                id: "MUSIC_ROUTE_29",
                path: "content-packs/core-modular/music/MUSIC_ROUTE_29.mid",
                kind: "music",
                source: "midi",
              },
            },
          ]),
        }),
      }),
    );
    expect(compiledCall?.[2]).toEqual({ indent: 0 });
    expect(mockWriteJsonToTargets).toHaveBeenCalledWith(
      "content-packs/core-modular.generated.json",
      expect.objectContaining({
        id: "core-modular",
        enabled: true,
        priority: -100,
        path: "content-packs/core-modular",
        compiled: null,
        files: expect.objectContaining({
          pokemon: ["content-packs/core-modular/pokemon/TOTODILE.json"],
          moves: ["content-packs/core-modular/moves/SCRATCH.json"],
          map_attributes: [
            "content-packs/core-modular/map_attributes/Route1.json",
          ],
          map_dimensions: [
            "content-packs/core-modular/map_dimensions/ROUTE_1.json",
          ],
        }),
      }),
      { indent: 2 },
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
          compiled: "content-packs/core-modular.crystalpack",
          files: strictFiles({}),
        }),
        expect.objectContaining({
          id: "module-audio-MUSIC_TITLE",
          enabled: false,
          files: expect.objectContaining({
            audio: ["content-packs/core-modular/music/MUSIC_TITLE.json"],
          }),
        }),
        expect.objectContaining({
          id: "module-audio-MUSIC_ROUTE_29",
          enabled: false,
          files: expect.objectContaining({
            audio: ["content-packs/core-modular/music/MUSIC_ROUTE_29.json"],
          }),
        }),
        expect.objectContaining({
          id: "module-fruit-trees-fruit_trees",
          enabled: false,
          files: expect.objectContaining({
            fruit_trees: [
              "content-packs/core-modular/fruit_trees/fruit_trees.json",
            ],
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
            level_up_moves: [
              "content-packs/core-modular/level_up_moves/TOTODILE.json",
            ],
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
            pokegear_landmarks: [
              "content-packs/core-modular/pokegear_landmarks/landmarks.json",
            ],
          }),
        }),
        expect.objectContaining({
          id: "module-route-Route1",
          enabled: false,
          files: expect.objectContaining({
            wild_encounters: [
              "content-packs/core-modular/wild_encounters/Route1.json",
            ],
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
            trainers: [
              "content-packs/core-modular/trainers/YOUNGSTER_JOE.json",
            ],
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
    const packs = indexCall?.[1]?.packs as Array<{
      id: string;
      enabled?: boolean;
    }>;
    expect(
      packs.find((pack) => pack.id === "module-route-stale"),
    ).toBeUndefined();
    expect(packs.filter((pack) => pack.id.startsWith("module-"))).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ id: "module-route-Route1", enabled: false }),
      ]),
    );
    expect(
      packs
        .filter((pack) => pack.id.startsWith("module-"))
        .every((pack) => pack.enabled === false),
    ).toBe(true);
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
      }),
    ).toThrow("Content pack 'legacy-pack' must declare priority.");
  });

  it("does not coerce route module map names when linking map files", () => {
    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      return (
        String(pathLike).endsWith("/maps") || String(pathLike).endsWith(".mid")
      );
    });
    jest.spyOn(fs, "readdirSync").mockImplementation((pathLike) => {
      if (String(pathLike).endsWith("/maps")) {
        return [
          "Route29.json",
          "Route46.json",
          "CherrygroveCity.json",
          "NewBarkTown.json",
        ] as never;
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
          CherrygroveCity_MapScripts: [
            { command: "def_scene_scripts", args: [] },
          ],
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
      npcData: {
        Route29: [],
        Route46: [],
        CherrygroveCity: [],
        NewBarkTown: [],
      },
      pokegearLandmarks: { landmarks: [], map_to_landmark: {} },
      runtimeSpawnPoints: titleRuntimeSpawnPoints,
      audioAssets: titleMusicAudioAsset,
    });

    const indexCall = mockWriteJsonToTargets.mock.calls.at(-1);
    const packs = indexCall?.[1]?.packs as Array<{
      id: string;
      enabled?: boolean;
      files: {
        maps: string[];
        map_scripts: string[];
        map_attributes: string[];
        map_dimensions: string[];
        npcs: string[];
      };
    }>;
    const generatedCorePackCall = mockWriteJsonToTargets.mock.calls.find(
      (call) => call[0] === "content-packs/core-modular.generated.json",
    );
    const generatedCorePack = generatedCorePackCall?.[1] as
      | {
          enabled?: boolean;
          files: {
            maps: string[];
            map_scripts: string[];
            map_attributes: string[];
          };
        }
      | undefined;
    const routeModule = packs.find(
      (pack) => pack.id === "module-route-Route 29",
    );

    expect(generatedCorePack).toEqual(
      expect.objectContaining({
        enabled: true,
        files: expect.objectContaining({
          maps: [],
          map_scripts: expect.arrayContaining([
            "content-packs/core-modular/map_scripts/Route29.json",
            "content-packs/core-modular/map_scripts/Route46.json",
            "content-packs/core-modular/map_scripts/CherrygroveCity.json",
            "content-packs/core-modular/map_scripts/NewBarkTown.json",
          ]),
          map_attributes: expect.arrayContaining([
            "content-packs/core-modular/map_attributes/Route29.json",
            "content-packs/core-modular/map_attributes/Route46.json",
            "content-packs/core-modular/map_attributes/CherrygroveCity.json",
            "content-packs/core-modular/map_attributes/NewBarkTown.json",
          ]),
        }),
      }),
    );
    expect(routeModule).toBeDefined();
    expect(routeModule?.enabled).toBe(false);
    expect(routeModule?.files.maps).toEqual([]);
    expect(routeModule?.files.map_scripts).toEqual([]);
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
      runtimeSpawnPoints: titleRuntimeSpawnPoints,
      audioAssets: titleMusicAudioAsset,
    });

    expect(mockWriteJsonToTargets).toHaveBeenCalledWith(
      "content-packs/core-modular/trainers/YOUNGSTER_JOE.json",
      expect.objectContaining({
        YOUNGSTER_JOE: expect.objectContaining({
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
      }),
      { indent: 2 },
    );

    const trainerCall = mockWriteJsonToTargets.mock.calls.find(
      (call) =>
        call[0] === "content-packs/core-modular/trainers/YOUNGSTER_JOE.json",
    );
    const trainerPayload = trainerCall?.[1] as {
      YOUNGSTER_JOE?: { party?: Array<Record<string, unknown>> };
    };

    expect(trainerPayload.YOUNGSTER_JOE?.party?.[0].species).toBe("RATTATA");
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
              moves: [{ name: "TACKLE", current_pp: 35, pp_ups: 0 }],
              hp: 20,
              dvs: { attack: 1, defense: 0, speed: 0, special: 0, hp: 0 },
            },
          ],
        } as never,
      ],
      pokedex: [],
      npcData: {},
      pokegearLandmarks: { landmarks: [], map_to_landmark: {} },
      runtimeSpawnPoints: titleRuntimeSpawnPoints,
      audioAssets: titleMusicAudioAsset,
    };

    exportCoreContentPack(payload);

    const trainerCall = mockWriteJsonToTargets.mock.calls.find(
      (call) =>
        call[0] === "content-packs/core-modular/trainers/BUG_CATCHER_AL.json",
    );
    const trainerPayload = trainerCall?.[1] as {
      BUG_CATCHER_AL?: { party?: Array<Record<string, unknown>> };
    };
    const partyMember = trainerPayload.BUG_CATCHER_AL?.party?.[0];
    expect(partyMember).toEqual({
      species: "CATERPIE",
      level: 7,
      item: "BERRY",
      moves: [{ name: "TACKLE", current_pp: 35, pp_ups: 0 }],
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
                ...(
                  payload.trainers[0] as {
                    party: Array<Record<string, unknown>>;
                  }
                ).party[0],
                custom_field: true,
              },
            ],
          } as never,
        ],
      }),
    ).toThrow(
      "because 'custom_field' is not part of the definitive trainer party schema",
    );
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
      }),
    ).toThrow(
      "Unable to export trainer BROKEN party[0] because species is not a species id string or Pokemon species record with an id.",
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
      }),
    ).toThrow(
      "Unable to export trainer BROKEN_HP party[0] because 'item' must be explicit modpack data.",
    );
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
            party: [
              {
                species: { id: "CATERPIE", base_stats: {} },
                level: 7,
              } as never,
            ],
          } as never,
        ],
        pokedex: [],
        npcData: {},
        pokegearLandmarks: { landmarks: [], map_to_landmark: {} },
      }),
    ).toThrow(
      "Unable to export trainer BROKEN_BASE_HP party[0] because 'item' must be explicit modpack data.",
    );
  });

  it("exports map files and map block entries into route modules", () => {
    jest.spyOn(fs, "existsSync").mockImplementation((pathLike) => {
      return (
        String(pathLike).endsWith("/maps") ||
        String(pathLike).endsWith("/tilesets") ||
        String(pathLike).endsWith(".mid")
      );
    });
    jest.spyOn(fs, "readdirSync").mockImplementation((pathLike) => {
      if (String(pathLike).endsWith("/maps")) {
        return ["NewRoute.json"] as never;
      }
      if (String(pathLike).endsWith("/tilesets")) {
        return ["johto.json", "johto_palette_map.json"] as never;
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
      if (filePath.endsWith("tilesets/johto.json")) {
        return { "10": ["FLOOR", "FLOOR", "FLOOR", "FLOOR"] };
      }
      if (filePath.endsWith("tilesets/johto_palette_map.json")) {
        return [0, 1, 2, 3];
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
      mapAttributes: {
        NewRoute: { tileset_name: "johto", blocks_label: "NewRoute_Blocks" },
      },
      items: [],
      trainers: [],
      pokedex: [],
      npcData: { NewRoute: [] },
      pokegearLandmarks: { landmarks: [], map_to_landmark: {} },
      runtimeSpawnPoints: titleRuntimeSpawnPoints,
      audioAssets: titleMusicAudioAsset,
    });

    expect(mockWriteJsonToTargets).toHaveBeenCalledWith(
      "content-packs/core-modular/map_scripts/NewRoute.json",
      expect.objectContaining({
        NewRoute_MapScripts: expect.any(Array),
        NewRoute_MapEvents: expect.any(Array),
      }),
      { indent: 2 },
    );
    expect(mockWriteJsonToTargets).toHaveBeenCalledWith(
      "content-packs/core-modular/map_blocks/NewRoute_Blocks.json",
      { NewRoute_Blocks: "AQID" },
      { indent: 2 },
    );

    const indexCall = mockWriteJsonToTargets.mock.calls.at(-1);
    const packs = indexCall?.[1]?.packs as Array<{
      id: string;
      files: {
        maps: string[];
        map_scripts: string[];
        map_blocks: string[];
        tilesets: string[];
      };
    }>;
    const routeModule = packs.find(
      (pack) => pack.id === "module-route-NewRoute",
    );

    expect(routeModule?.files.maps).toEqual([]);
    expect(routeModule?.files.map_scripts).toEqual([
      "content-packs/core-modular/map_scripts/NewRoute.json",
    ]);
    expect(routeModule?.files.map_blocks).toEqual([
      "content-packs/core-modular/map_blocks/NewRoute_Blocks.json",
    ]);
    expect(routeModule?.files.tilesets).toEqual([]);
    expect(mockWriteJsonToTargets).toHaveBeenCalledWith(
      "content-packs/core-modular/tilesets/johto.json",
      {
        johto: {
          collision: { "10": ["FLOOR", "FLOOR", "FLOOR", "FLOOR"] },
          palette_map: [0, 1, 2, 3],
        },
      },
      { indent: 2 },
    );
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
        mapAttributes: {
          NewRoute: { tileset_name: "johto", blocks_label: "NewRoute_Blocks" },
        },
        items: [],
        trainers: [],
        pokedex: [],
        npcData: { NewRoute: [] },
        pokegearLandmarks: { landmarks: [], map_to_landmark: {} },
      }),
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
      }),
    ).toThrow(
      "map_blocks.json entry 'NewRoute_Blocks' must be encoded block data.",
    );
  });
});
