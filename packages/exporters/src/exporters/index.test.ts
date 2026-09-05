const mockCalls: string[] = [];

const testBugContestEncounters = [
  { weight: 20, species: "CATERPIE", minLevel: 7, maxLevel: 18 },
  { weight: 20, species: "WEEDLE", minLevel: 7, maxLevel: 18 },
  { weight: 10, species: "METAPOD", minLevel: 9, maxLevel: 18 },
  { weight: 10, species: "KAKUNA", minLevel: 9, maxLevel: 18 },
  { weight: 5, species: "BUTTERFREE", minLevel: 12, maxLevel: 15 },
  { weight: 5, species: "BEEDRILL", minLevel: 12, maxLevel: 15 },
  { weight: 10, species: "VENONAT", minLevel: 10, maxLevel: 16 },
  { weight: 10, species: "PARAS", minLevel: 10, maxLevel: 17 },
  { weight: 5, species: "SCYTHER", minLevel: 13, maxLevel: 14 },
  { weight: 5, species: "PINSIR", minLevel: 13, maxLevel: 14 },
  { weight: 255, species: "VENOMOTH", minLevel: 30, maxLevel: 40 },
];

const mockExportData = jest.fn(() => {
  mockCalls.push("exportData");
  return {
    pokemonData: [{ id: "BULBASAUR", int_id: 1 }],
    movesData: { TACKLE: { name: "TACKLE", effect: "NORMAL_HIT" } },
    growthRatesData: [],
    learnsetsData: {},
    levelUpMovesData: {},
    eggMovesData: {},
  };
});
const mockExportItems = jest.fn(() => {
  mockCalls.push("exportItems");
  return [];
});
const mockExportEvolutions = jest.fn(() => {
  mockCalls.push("exportEvolutions");
  return [];
});
const mockExportWildEncounters = jest.fn(() => {
  mockCalls.push("exportWildEncounters");
  return [];
});
const mockExportFieldEncounters = jest.fn(() => {
  mockCalls.push("exportFieldEncounters");
  return [];
});
const mockExportFishing = jest.fn(() => {
  mockCalls.push("exportFishing");
  return { groups: {}, time_groups: [], swarm_rules: [] };
});
const mockExportFieldMoves = jest.fn(() => {
  mockCalls.push("exportFieldMoves");
  return {
    cut: { move_id: "CUT", badge: { region: "johto", index: 1 }, target_collisions: [], replacements: {} },
    whirlpool: { move_id: "WHIRLPOOL", badge: { region: "johto", index: 6 }, target_collisions: [], replacements: {} },
    strength: { move_id: "STRENGTH", badge: { region: "johto", index: 2 }, engine_flag: "ENGINE_STRENGTH_ACTIVE" },
    flash: { move_id: "FLASH", badge: { region: "johto", index: 0 }, engine_flag: "STATUSFLAGS_FLASH" },
    surf: { move_id: "SURF", badge: { region: "johto", index: 3 }, blocked_collisions: [], target_collisions: [] },
    waterfall: { move_id: "WATERFALL", badge: { region: "johto", index: 7 }, blocked_collisions: [], target_collisions: [] },
    fly: { move_id: "FLY", badge: { region: "johto", index: 5 } },
    dig: { move_id: "DIG", target_collisions: [] },
    teleport: { move_id: "TELEPORT", target_collisions: [] },
    headbutt: { move_id: "HEADBUTT", target_collisions: [0x15, 0x1d] },
    rock_smash: { move_id: "ROCK_SMASH", target_collisions: [] },
    sweet_scent: { move_id: "SWEET_SCENT", target_collisions: [] },
    escape_rope: { effect: "ESCAPE_ROPE", escape_rope_mode: "DIG_WARP" },
    repel: { effects: ["REPEL", "SUPER_REPEL", "MAX_REPEL"] },
    bicycle: { effect: "BICYCLE" },
    itemfinder: { effect: "ITEMFINDER" },
    squirtbottle: { effect: "SQUIRTBOTTLE" },
    coin_case: { effect: "COIN_CASE" },
    blue_card: { effect: "BLUE_CARD" },
    town_map: { effect: "TOWN_MAP" },
  };
});
const mockExportFieldBoxItems = jest.fn(() => {
  mockCalls.push("exportFieldBoxItems");
  return {
    NORMAL_BOX: {
      item_id: "NORMAL_BOX",
      effect: "NORMAL_BOX",
      decoration_flag: "EVENT_DECO_SILVER_TROPHY",
    },
  };
});
const mockExportFlyDestinations = jest.fn(() => {
  mockCalls.push("exportFlyDestinations");
  return { ENGINE_FLYPOINT_NEW_BARK: { label: "LANDMARK_NEW_BARK_TOWN" } };
});
const mockExportBattleRewardRules = jest.fn(() => {
  mockCalls.push("exportBattleRewardRules");
  return {
    max_level: 100,
    wild_exp_divisor: 7,
    trainer_exp_numerator: 3,
    trainer_exp_denominator: 2,
  };
});
const mockExportBattleEscapeRules = jest.fn(() => {
  mockCalls.push("exportBattleEscapeRules");
  return {
    player_speed_multiplier: 32,
    enemy_speed_divisor: 4,
    failed_attempt_bonus: 30,
    rng_roll_values: 256,
  };
});
const mockExportStepEventRules = jest.fn(() => {
  mockCalls.push("exportStepEventRules");
  return {
    poison_step_interval: 4,
    egg_step_trigger: 128,
    hatched_egg_happiness: 120,
    poison_status: "POISON",
    egg_nickname: "EGG",
    happiness_step_counter_mask: 1,
    happiness_step_counter_target: 0,
  };
});
const mockExportCaptureRules = jest.fn(() => {
  mockCalls.push("exportCaptureRules");
  return {
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
  };
});
const mockExportFruitTrees = jest.fn(() => {
  mockCalls.push("exportFruitTrees");
  return { FRUITTREE_ROUTE_29: "BERRY" };
});
const mockExportCurrencyConstants = jest.fn(() => {
  mockCalls.push("exportCurrencyConstants");
  return { MAX_MONEY: 999999, MAX_COINS: 9999, ROUTE43GATE_TOLL: 1000 };
});
const mockExportTrainers = jest.fn(() => {
  mockCalls.push("exportTrainers");
  return [];
});
const mockExportMapDimensions = jest.fn(() => {
  mockCalls.push("exportMapDimensions");
  return {};
});
const mockExportMapAttributes = jest.fn(() => {
  mockCalls.push("exportMapAttributes");
  return {};
});
const mockExportNpcData = jest.fn(() => {
  mockCalls.push("exportNpcData");
  return {};
});
const mockExportStoryEvents = jest.fn(() => {
  mockCalls.push("exportStoryEvents");
});
const mockExportPhoneScripts = jest.fn(() => {
  mockCalls.push("exportPhoneScripts");
});
const mockExportBattleAnimations = jest.fn(() => {
  mockCalls.push("exportBattleAnimations");
  return { BattleAnim_Pound: ["anim_ret"] };
});
const mockExportRuntimeAssets = jest.fn(() => {
  mockCalls.push("exportRuntimeAssets");
  return {
    fleeMons: { buckets: { always: ["RAIKOU"], often: [], sometimes: [] } },
    pcStrings: { PCString_ChooseaPKMN: "Choose a PKMN." },
    menuIcons: { CHIKORITA: "ICON_CHIKORITA" },
    pokedexEntries: {
      CHIKORITA: { species: "CHIKORITA", classification: "LEAF", heightDigits: 211, weightDigits: 140, pages: [] },
    },
    pokemonFrontpicAnimations: { CHIKORITA: { commands: [{ kind: "endanim" }] } },
    marts: { MART_CHERRYGROVE: ["POTION"] },
    phoneContacts: { PHONE_MOM: { contactId: "PHONE_MOM" } },
    permanentPhoneNumbers: { PHONE_MOM: {} },
    specialPhoneCalls: { SPECIALCALL_NONE: {} },
    npcTrades: { NPC_TRADE_MIKE: {} },
    specialRoutines: { FadeOutMusic: {} },
    encounterMusicModifiers: { trainerClasses: {}, species: {} },
  };
});
const mockExportRoamingPokemon = jest.fn(() => {
  mockCalls.push("exportRoamingPokemon");
  return { RAIKOU: { level: 40, mapGroup: 2, mapNumber: 5 } };
});
const mockExportBuenaPasswordCategories = jest.fn(() => {
  mockCalls.push("exportBuenaPasswordCategories");
  return {
    order: ["HealingItems"],
    categories: {
      HealingItems: { categoryType: "BUENA_ITEM", points: 12, options: ["POTION", "ANTIDOTE", "PARLYZ_HEAL"] },
    },
  };
});
const mockExportBuenaPrizes = jest.fn(() => {
  mockCalls.push("exportBuenaPrizes");
  return { RARE_CANDY: 3 };
});
const mockExportKurtApricornRecipes = jest.fn(() => {
  mockCalls.push("exportKurtApricornRecipes");
  return { RED_APRICORN: "LEVEL_BALL" };
});
const mockExportShuckieGift = jest.fn(() => {
  mockCalls.push("exportShuckieGift");
  return {
    species: "SHUCKLE",
    level: 15,
    heldItem: "BERRY",
    nickname: "SHUCKIE",
    originalTrainerName: "MANIA",
    originalTrainerId: 518,
    gotTodayEngineFlag: "ENGINE_GOT_SHUCKIE_TODAY",
  };
});
const mockExportDratiniMoveSets = jest.fn(() => {
  mockCalls.push("exportDratiniMoveSets");
  return { "0": ["WRAP", "THUNDER_WAVE", "TWISTER", "EXTREMESPEED"] };
});
const mockExportBugContestConfig = jest.fn(() => {
  mockCalls.push("exportBugContestConfig");
  return {
    parkBalls: 20,
    timerMinutes: 20,
    timerSeconds: 0,
    selectedContestantCount: 5,
    contestantFlags: ["EVENT_BUG_CATCHING_CONTESTANT_1A"],
    encounters: testBugContestEncounters,
  };
});
const mockExportBattleTowerRules = jest.fn(() => {
  mockCalls.push("exportBattleTowerRules");
  return {
    bannedSpecies: { MEWTWO: {}, MEW: {}, LUGIA: {}, HO_OH: {}, CELEBI: {} },
    requiredPartyCount: 3,
    challengeStreakLength: 7,
    minimumLevelGroup: 1,
    maximumLevelGroup: 10,
    levelGroupSize: 10,
    partyCountFailureText: "OnlyThreeMonMayBeEnteredText",
    duplicateSpeciesFailureText: "TheMonMustAllBeDifferentKindsText",
    duplicateHeldItemFailureText: "TheMonMustNotHoldTheSameItemsText",
    eggFailureText: "YouCantTakeAnEggText",
  };
});
const mockExportOakRatings = jest.fn(() => {
  mockCalls.push("exportOakRatings");
  return [
    {
      caughtCountLimit: 9,
      fanfare: "SFX_DEX_FANFARE_LESS_THAN_20",
      textLabel: "OakRating01",
    },
  ];
});
const mockExportOddEggDefinitions = jest.fn(() => {
  mockCalls.push("exportOddEggDefinitions");
  return [
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
  ];
});
const mockExportMagikarpLengths = jest.fn(() => {
  mockCalls.push("exportMagikarpLengths");
  return [{ threshold: 110, divisor: 1 }, { threshold: 310, divisor: 2 }];
});
const mockExportHappinessData = jest.fn(() => {
  mockCalls.push("exportHappinessData");
  return {
    changes: { "18": { code: "HAPPINESS_GROOMING", low: 3, mid: 3, high: 1 } },
    services: {
      DaisysGrooming: [{ rollWeight: 255, scriptValue: 2, changeCode: 18 }],
    },
  };
});
const mockExportEncounterSlotTables = jest.fn(() => {
  mockCalls.push("exportEncounterSlotTables");
  return {
    grass: [{ threshold: 100, slot: 0 }],
    water: [{ threshold: 100, slot: 0 }],
  };
});
const mockExportBattleStatMultipliers = jest.fn(() => {
  mockCalls.push("exportBattleStatMultipliers");
  return {
    stat: [{ numerator: 1, denominator: 1 }],
    accuracy: [{ numerator: 1, denominator: 1 }],
  };
});
const mockExportCaptureWobbleProbabilities = jest.fn(() => {
  mockCalls.push("exportCaptureWobbleProbabilities");
  return [{ catch_rate: 255, chance: 255 }];
});
const mockExportMovePriorityTable = jest.fn(() => {
  mockCalls.push("exportMovePriorityTable");
  return {
    base_priority: 1,
    effect_priorities: { PRIORITY_HIT: 2 },
    move_priorities: [{ move: "VITAL_THROW", priority: 0 }],
  };
});
const mockExportTypeCategories = jest.fn(() => {
  mockCalls.push("exportTypeCategories");
  return { physical: ["NORMAL"], special: ["FIRE"] };
});
const mockExportTypeEffectivenessTable = jest.fn(() => {
  mockCalls.push("exportTypeEffectivenessTable");
  return {
    matchups: [
      { attacker: "FIRE", defender: "GRASS", multiplier: { numerator: 2, denominator: 1 } },
    ],
    foresight_matchups: [
      { attacker: "NORMAL", defender: "GHOST", multiplier: { numerator: 0, denominator: 1 } },
    ],
  };
});
const mockExportWeatherModifiers = jest.fn(() => {
  mockCalls.push("exportWeatherModifiers");
  return {
    type_modifiers: [
      { weather: "WEATHER_RAIN", move_type: "WATER", multiplier: { numerator: 3, denominator: 2 } },
    ],
    move_effect_modifiers: [],
  };
});
const mockExportPokegearLandmarks = jest.fn(() => {
  mockCalls.push("exportPokegearLandmarks");
  return { landmarks: [], map_to_landmark: {} };
});
const mockExportPokegearPaletteMap = jest.fn(() => {
  mockCalls.push("exportPokegearPaletteMap");
  return { town_map: ["EARTH"], pokegear: ["BORDER"] };
});
const mockExportGraphicsAssets = jest.fn(() => {
  mockCalls.push("exportGraphicsAssets");
});
const mockExportPokedex = jest.fn(() => {
  mockCalls.push("exportPokedex");
  return [];
});
const mockExportPlayability = jest.fn(() => {
  mockCalls.push("exportPlayability");
  return { start_maps: ["PlayersHouse2F"] };
});
const mockExportAudioAssets = jest.fn(() => {
  mockCalls.push("exportAudioAssets");
  return {
    MUSIC_ROUTE_29: {
      id: "MUSIC_ROUTE_29",
      path: "content-packs/core-modular/music/MUSIC_ROUTE_29.pcm",
      kind: "music",
      source: "pcm",
      pcm_format: { sample_rate_hz: 22050, channels: 2, bits_per_sample: 16 },
    },
  };
});
const mockExportPokemonCryMetadataFromAsm = jest.fn(() => {
  mockCalls.push("exportPokemonCryMetadataFromAsm");
  return { BULBASAUR: { cry: "CRY_BULBASAUR", pitch: 128, length: 129 } };
});
const mockReadJsonAssetSync = jest.fn((filePath: string) => {
  if (filePath.endsWith("runtime_spawn_points.json")) {
    return { "0": { identifier: 0, mapConstant: "PLAYERS_HOUSE_2F", mapName: "PlayersHouse2F" } };
  }
  if (filePath.endsWith("runtime_map_metadata.json")) {
    return { PLAYERS_HOUSE_2F: { constant: "PLAYERS_HOUSE_2F", name: "PlayersHouse2F" } };
  }
  if (filePath.endsWith("initialize_events.json")) {
    return { eventFlags: ["EVENT_INITIAL"], engineFlags: [], variableSprites: {} };
  }
  if (filePath.endsWith("story_event_script_constants.json")) {
    return { global: { TRUE: 1 }, maps: {} };
  }
  if (filePath.endsWith("asm_text.json")) {
    return { WildPokemonAppearedText: "Wild appeared!" };
  }
  if (filePath.endsWith("move_names.json")) {
    return ["POUND"];
  }
  if (filePath.endsWith("battle_animation_table.json")) {
    return ["BattleAnim_Dummy", "BattleAnim_Pound"];
  }
  if (filePath.endsWith("battle_anim_bundle.json")) {
    return { objects: { BATTLE_ANIM_OBJ_HIT: {} }, framesets: { HIT: [] }, oam_sets: { HIT: {} }, gfx: { HIT: {} } };
  }
  if (filePath.endsWith("sprite_anim_bundle.json")) {
    return { oam_sets: { WALK: {} }, framesets: { WALK: [] }, animations: { WALK: {} } };
  }
  if (filePath.endsWith("sprite_palette_defaults.json")) {
    return { SPRITE_CHRIS: 0 };
  }
  throw new Error(`Unexpected read ${filePath}`);
});
const mockExportCoreContentPack = jest.fn(() => {
  mockCalls.push("exportCoreContentPack");
});

jest.mock("./export-data", () => ({ exportData: mockExportData }));
jest.mock("./export-items", () => ({ exportItems: mockExportItems }));
jest.mock("./export-evolutions", () => ({ exportEvolutions: mockExportEvolutions }));
jest.mock("./export-wild-encounters", () => ({ exportWildEncounters: mockExportWildEncounters }));
jest.mock("./export-field-encounters", () => ({ exportFieldEncounters: mockExportFieldEncounters }));
jest.mock("./export-fishing", () => ({ exportFishing: mockExportFishing }));
jest.mock("./export-field-moves", () => ({ exportFieldMoves: mockExportFieldMoves }));
jest.mock("./export-field-box-items", () => ({ exportFieldBoxItems: mockExportFieldBoxItems }));
jest.mock("./export-fly-destinations", () => ({ exportFlyDestinations: mockExportFlyDestinations }));
jest.mock("./export-battle-reward-rules", () => ({ exportBattleRewardRules: mockExportBattleRewardRules }));
jest.mock("./export-battle-escape-rules", () => ({ exportBattleEscapeRules: mockExportBattleEscapeRules }));
jest.mock("./export-step-event-rules", () => ({ exportStepEventRules: mockExportStepEventRules }));
jest.mock("./export-capture-rules", () => ({ exportCaptureRules: mockExportCaptureRules }));
jest.mock("./export-fruit-trees", () => ({ exportFruitTrees: mockExportFruitTrees }));
jest.mock("./export-currency-constants", () => ({ exportCurrencyConstants: mockExportCurrencyConstants }));
jest.mock("./export-trainers", () => ({ exportTrainers: mockExportTrainers }));
jest.mock("./export-map-dimensions", () => ({ exportMapDimensions: mockExportMapDimensions }));
jest.mock("./export-map-attributes", () => ({ exportMapAttributes: mockExportMapAttributes }));
jest.mock("./export-npcs", () => ({ exportNpcData: mockExportNpcData }));
jest.mock("./export-story-events", () => ({ exportStoryEvents: mockExportStoryEvents }));
jest.mock("./export-phone-scripts", () => ({ exportPhoneScripts: mockExportPhoneScripts }));
jest.mock("./export-battle-animations", () => ({ exportBattleAnimations: mockExportBattleAnimations }));
jest.mock("./export-runtime-assets", () => ({
  exportRuntimeAssets: mockExportRuntimeAssets,
  exportRoamingPokemon: mockExportRoamingPokemon,
  exportBuenaPasswordCategories: mockExportBuenaPasswordCategories,
  exportBuenaPrizes: mockExportBuenaPrizes,
  exportKurtApricornRecipes: mockExportKurtApricornRecipes,
  exportShuckieGift: mockExportShuckieGift,
  exportDratiniMoveSets: mockExportDratiniMoveSets,
  exportBugContestConfig: mockExportBugContestConfig,
  exportBattleTowerRules: mockExportBattleTowerRules,
  exportOakRatings: mockExportOakRatings,
  exportOddEggDefinitions: mockExportOddEggDefinitions,
  exportMagikarpLengths: mockExportMagikarpLengths,
  exportHappinessData: mockExportHappinessData,
  exportEncounterSlotTables: mockExportEncounterSlotTables,
  exportBattleStatMultipliers: mockExportBattleStatMultipliers,
  exportCaptureWobbleProbabilities: mockExportCaptureWobbleProbabilities,
  exportMovePriorityTable: mockExportMovePriorityTable,
  exportTypeCategories: mockExportTypeCategories,
  exportTypeEffectivenessTable: mockExportTypeEffectivenessTable,
  exportWeatherModifiers: mockExportWeatherModifiers,
}));
jest.mock("./export-pokegear-landmarks", () => ({ exportPokegearLandmarks: mockExportPokegearLandmarks }));
jest.mock("./export-pokegear-palette-map", () => ({ exportPokegearPaletteMap: mockExportPokegearPaletteMap }));
jest.mock("./export-graphics-assets", () => ({ exportGraphicsAssets: mockExportGraphicsAssets }));
jest.mock("./export-pokedex", () => ({ exportPokedex: mockExportPokedex }));
jest.mock("./export-playability", () => ({ exportPlayability: mockExportPlayability }));
jest.mock("./export-audio-assets", () => ({
  exportAudioAssets: mockExportAudioAssets,
  exportPokemonCryMetadataFromAsm: mockExportPokemonCryMetadataFromAsm,
}));
jest.mock("@pokecrystal/core/core/asset-reader", () => ({ readJsonAssetSync: mockReadJsonAssetSync }));
jest.mock("@pokecrystal/core/core/paths", () => ({ getDataDir: () => "/mock/assets/data" }));
jest.mock("@pokecrystal/core/core/path-utils", () => ({ joinPath: (...parts: string[]) => parts.join("/") }));
jest.mock("./export-content-pack", () => ({ exportCoreContentPack: mockExportCoreContentPack }));

describe("exportCoreData", () => {
  beforeEach(() => {
    mockCalls.length = 0;
    jest.clearAllMocks();
  });

  it("runs newly added exporters before content-pack indexing", () => {
    const { exportCoreData } = require("./index") as typeof import("./index");

    exportCoreData();

    expect(mockCalls).toEqual([
      "exportData",
      "exportItems",
      "exportEvolutions",
      "exportWildEncounters",
      "exportFieldEncounters",
      "exportFishing",
      "exportFieldMoves",
      "exportFieldBoxItems",
      "exportFlyDestinations",
      "exportBattleRewardRules",
      "exportBattleEscapeRules",
      "exportStepEventRules",
      "exportCaptureRules",
      "exportFruitTrees",
      "exportTrainers",
      "exportMapDimensions",
      "exportMapAttributes",
      "exportNpcData",
      "exportStoryEvents",
      "exportPhoneScripts",
      "exportBattleAnimations",
      "exportRuntimeAssets",
      "exportPokegearLandmarks",
      "exportPokegearPaletteMap",
      "exportGraphicsAssets",
      "exportPokedex",
      "exportPlayability",
      "exportPokemonCryMetadataFromAsm",
      "exportAudioAssets",
      "exportRoamingPokemon",
      "exportBuenaPasswordCategories",
      "exportBuenaPrizes",
      "exportKurtApricornRecipes",
      "exportShuckieGift",
      "exportDratiniMoveSets",
      "exportBugContestConfig",
      "exportBattleTowerRules",
      "exportOakRatings",
      "exportOddEggDefinitions",
      "exportMagikarpLengths",
      "exportHappinessData",
      "exportEncounterSlotTables",
      "exportBattleStatMultipliers",
      "exportCaptureWobbleProbabilities",
      "exportMovePriorityTable",
      "exportTypeCategories",
      "exportTypeEffectivenessTable",
      "exportWeatherModifiers",
      "exportCurrencyConstants",
      "exportCoreContentPack",
    ]);
    expect(mockExportPlayability).toHaveBeenCalledWith({ itemIds: [] });
    expect(mockExportPokemonCryMetadataFromAsm).toHaveBeenCalledWith(["BULBASAUR"]);
    expect(mockExportAudioAssets).toHaveBeenCalledWith({
      BULBASAUR: { cry: "CRY_BULBASAUR", pitch: 128, length: 129 },
    });
    expect(mockExportCurrencyConstants).toHaveBeenCalledWith({ global: { TRUE: 1 }, maps: {} });
    expect(mockExportRoamingPokemon).toHaveBeenCalledWith({
      PLAYERS_HOUSE_2F: { constant: "PLAYERS_HOUSE_2F", name: "PlayersHouse2F" },
    });
    expect(mockExportBuenaPasswordCategories).toHaveBeenCalled();
    expect(mockExportBuenaPrizes).toHaveBeenCalled();
    expect(mockExportKurtApricornRecipes).toHaveBeenCalled();
    expect(mockExportShuckieGift).toHaveBeenCalled();
    expect(mockExportDratiniMoveSets).toHaveBeenCalled();
    expect(mockExportBugContestConfig).toHaveBeenCalled();
    expect(mockExportBattleTowerRules).toHaveBeenCalled();
    expect(mockExportOakRatings).toHaveBeenCalled();
    expect(mockExportOddEggDefinitions).toHaveBeenCalled();
    expect(mockExportMagikarpLengths).toHaveBeenCalled();
    expect(mockExportHappinessData).toHaveBeenCalled();
    expect(mockExportEncounterSlotTables).toHaveBeenCalled();
    expect(mockExportBattleStatMultipliers).toHaveBeenCalled();
    expect(mockExportCaptureWobbleProbabilities).toHaveBeenCalled();
    expect(mockExportMovePriorityTable).toHaveBeenCalledWith({
      TACKLE: { name: "TACKLE", effect: "NORMAL_HIT" },
    });
    expect(mockExportTypeCategories).toHaveBeenCalled();
    expect(mockExportTypeEffectivenessTable).toHaveBeenCalled();
    expect(mockExportWeatherModifiers).toHaveBeenCalled();
    expect(mockExportBattleRewardRules).toHaveBeenCalled();
    expect(mockExportBattleEscapeRules).toHaveBeenCalled();
    expect(mockExportStepEventRules).toHaveBeenCalled();
    expect(mockExportCaptureRules).toHaveBeenCalledWith([{ id: "BULBASAUR", int_id: 1 }]);
    expect(mockExportCoreContentPack).toHaveBeenCalledWith({
      pokemonData: [{ id: "BULBASAUR", int_id: 1 }],
      movesData: { TACKLE: { name: "TACKLE", effect: "NORMAL_HIT" } },
      growthRatesData: [],
      learnsetsData: {},
      levelUpMovesData: {},
      eggMovesData: {},
      evolutions: [],
      wildEncounters: [],
      fieldEncounters: [],
      decorations: expect.any(Object),
      fishing: { groups: {}, time_groups: [], swarm_rules: [] },
      fieldMoves: {
        cut: { move_id: "CUT", badge: { region: "johto", index: 1 }, target_collisions: [], replacements: {} },
        whirlpool: { move_id: "WHIRLPOOL", badge: { region: "johto", index: 6 }, target_collisions: [], replacements: {} },
        strength: { move_id: "STRENGTH", badge: { region: "johto", index: 2 }, engine_flag: "ENGINE_STRENGTH_ACTIVE" },
        flash: { move_id: "FLASH", badge: { region: "johto", index: 0 }, engine_flag: "STATUSFLAGS_FLASH" },
        surf: { move_id: "SURF", badge: { region: "johto", index: 3 }, blocked_collisions: [], target_collisions: [] },
        waterfall: { move_id: "WATERFALL", badge: { region: "johto", index: 7 }, blocked_collisions: [], target_collisions: [] },
        fly: { move_id: "FLY", badge: { region: "johto", index: 5 } },
        dig: { move_id: "DIG", target_collisions: [] },
        teleport: { move_id: "TELEPORT", target_collisions: [] },
        headbutt: { move_id: "HEADBUTT", target_collisions: [0x15, 0x1d] },
        rock_smash: { move_id: "ROCK_SMASH", target_collisions: [] },
        sweet_scent: { move_id: "SWEET_SCENT", target_collisions: [] },
        escape_rope: { effect: "ESCAPE_ROPE", escape_rope_mode: "DIG_WARP" },
        repel: { effects: ["REPEL", "SUPER_REPEL", "MAX_REPEL"] },
        bicycle: { effect: "BICYCLE" },
        itemfinder: { effect: "ITEMFINDER" },
        squirtbottle: { effect: "SQUIRTBOTTLE" },
        coin_case: { effect: "COIN_CASE" },
        blue_card: { effect: "BLUE_CARD" },
        town_map: { effect: "TOWN_MAP" },
      },
      fieldBoxItems: {
        NORMAL_BOX: {
          item_id: "NORMAL_BOX",
          effect: "NORMAL_BOX",
          decoration_flag: "EVENT_DECO_SILVER_TROPHY",
        },
      },
      flyDestinations: { ENGINE_FLYPOINT_NEW_BARK: { label: "LANDMARK_NEW_BARK_TOWN" } },
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
      fruitTrees: { FRUITTREE_ROUTE_29: "BERRY" },
      runtimeSpawnPoints: { "0": { identifier: 0, mapConstant: "PLAYERS_HOUSE_2F", mapName: "PlayersHouse2F" } },
      runtimeMapMetadata: { PLAYERS_HOUSE_2F: { constant: "PLAYERS_HOUSE_2F", name: "PlayersHouse2F" } },
      roamingPokemon: { RAIKOU: { level: 40, mapGroup: 2, mapNumber: 5 } },
      buenaPasswordCategories: {
        order: ["HealingItems"],
        categories: {
          HealingItems: { categoryType: "BUENA_ITEM", points: 12, options: ["POTION", "ANTIDOTE", "PARLYZ_HEAL"] },
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
      dratiniMoveSets: { "0": ["WRAP", "THUNDER_WAVE", "TWISTER", "EXTREMESPEED"] },
      bugContestConfig: {
        parkBalls: 20,
        timerMinutes: 20,
        timerSeconds: 0,
        selectedContestantCount: 5,
        contestantFlags: ["EVENT_BUG_CATCHING_CONTESTANT_1A"],
        encounters: testBugContestEncounters,
      },
      battleTowerRules: {
        bannedSpecies: { MEWTWO: {}, MEW: {}, LUGIA: {}, HO_OH: {}, CELEBI: {} },
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
      magikarpLengths: [{ threshold: 110, divisor: 1 }, { threshold: 310, divisor: 2 }],
      happinessData: {
        changes: { "18": { code: "HAPPINESS_GROOMING", low: 3, mid: 3, high: 1 } },
        services: {
          DaisysGrooming: [{ rollWeight: 255, scriptValue: 2, changeCode: 18 }],
        },
      },
      encounterSlotTables: {
        grass: [{ threshold: 100, slot: 0 }],
        water: [{ threshold: 100, slot: 0 }],
      },
      encounterMusicModifiers: { trainerClasses: {}, species: {} },
      battleStatMultipliers: {
        stat: [{ numerator: 1, denominator: 1 }],
        accuracy: [{ numerator: 1, denominator: 1 }],
      },
      captureWobbleProbabilities: [{ catch_rate: 255, chance: 255 }],
      movePriorities: {
        base_priority: 1,
        effect_priorities: { PRIORITY_HIT: 2 },
        move_priorities: [{ move: "VITAL_THROW", priority: 0 }],
      },
      typeCategories: { physical: ["NORMAL"], special: ["FIRE"] },
      typeEffectiveness: {
        matchups: [
          { attacker: "FIRE", defender: "GRASS", multiplier: { numerator: 2, denominator: 1 } },
        ],
        foresight_matchups: [
          { attacker: "NORMAL", defender: "GHOST", multiplier: { numerator: 0, denominator: 1 } },
        ],
      },
      weatherModifiers: {
        type_modifiers: [
          { weather: "WEATHER_RAIN", move_type: "WATER", multiplier: { numerator: 3, denominator: 2 } },
        ],
        move_effect_modifiers: [],
      },
      mapDimensions: {},
      mapAttributes: {},
      items: [],
      fleeMons: { buckets: { always: ["RAIKOU"], often: [], sometimes: [] } },
      marts: { MART_CHERRYGROVE: ["POTION"] },
      currencyConstants: { MAX_MONEY: 999999, MAX_COINS: 9999, ROUTE43GATE_TOLL: 1000 },
      pcStrings: { PCString_ChooseaPKMN: "Choose a PKMN." },
      menuIcons: { CHIKORITA: "ICON_CHIKORITA" },
      pokedexEntries: {
        CHIKORITA: { species: "CHIKORITA", classification: "LEAF", heightDigits: 211, weightDigits: 140, pages: [] },
      },
      pokemonFrontpicAnimations: { CHIKORITA: { commands: [{ kind: "endanim" }] } },
      initializeEvents: { eventFlags: ["EVENT_INITIAL"], engineFlags: [], variableSprites: {} },
      storyEventScriptConstants: { global: { TRUE: 1 }, maps: {} },
      phoneContacts: { PHONE_MOM: { contactId: "PHONE_MOM" } },
      permanentPhoneNumbers: { PHONE_MOM: {} },
      specialPhoneCalls: { SPECIALCALL_NONE: {} },
      npcTrades: { NPC_TRADE_MIKE: {} },
      specialRoutines: { FadeOutMusic: {} },
      asmText: { WildPokemonAppearedText: "Wild appeared!" },
      moveNames: ["POUND"],
      battleAnimations: { BattleAnim_Pound: ["anim_ret"] },
      battleAnimationTable: ["BattleAnim_Dummy", "BattleAnim_Pound"],
      battleAnimBundle: { objects: { BATTLE_ANIM_OBJ_HIT: {} }, framesets: { HIT: [] }, oam_sets: { HIT: {} }, gfx: { HIT: {} } },
      spriteAnimBundle: { oam_sets: { WALK: {} }, framesets: { WALK: [] }, animations: { WALK: {} } },
      spritePaletteDefaults: { SPRITE_CHRIS: 0 },
      pokegearTownMapPaletteMap: { town_map: ["EARTH"], pokegear: ["BORDER"] },
      pokemonCries: { BULBASAUR: { cry: "CRY_BULBASAUR", pitch: 128, length: 129 } },
      trainers: [],
      trainerClassNames: undefined,
      pokedex: [],
      npcData: {},
      pokegearLandmarks: { landmarks: [], map_to_landmark: {} },
      playability: { start_maps: ["PlayersHouse2F"] },
      audioAssets: {
        MUSIC_ROUTE_29: {
          id: "MUSIC_ROUTE_29",
          path: "content-packs/core-modular/music/MUSIC_ROUTE_29.pcm",
          kind: "music",
          source: "pcm",
          pcm_format: { sample_rate_hz: 22050, channels: 2, bits_per_sample: 16 },
        },
      },
    });
  });
});
