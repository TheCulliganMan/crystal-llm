const mockCalls: string[] = [];

const mockExportData = jest.fn(() => {
  mockCalls.push("exportData");
  return {
    pokemonData: [{ id: "BULBASAUR", int_id: 1 }],
    movesData: {},
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
    fleeMons: { always: ["RAIKOU"], often: [], sometimes: [] },
    pcStrings: { PCString_ChooseaPKMN: "Choose a PKMN." },
    menuIcons: { CHIKORITA: "ICON_CHIKORITA" },
    pokedexEntries: [{ species: "CHIKORITA", classification: "LEAF", heightDigits: 211, weightDigits: 140, pages: [] }],
    pokemonFrontpicAnimations: { CHIKORITA: { commands: [{ kind: "endanim" }] } },
    marts: { MART_CHERRYGROVE: ["POTION"] },
    phoneContacts: { PHONE_MOM: { contactId: "PHONE_MOM" } },
    permanentPhoneNumbers: ["PHONE_MOM"],
    specialPhoneCalls: ["SPECIALCALL_NONE"],
    npcTrades: ["NPC_TRADE_MIKE"],
    specialRoutines: ["FadeOutMusic"],
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
  return [{ id: "MUSIC_ROUTE_29", path: "content-packs/core-modular/music/MUSIC_ROUTE_29.mid", kind: "music" }];
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
jest.mock("./export-trainers", () => ({ exportTrainers: mockExportTrainers }));
jest.mock("./export-map-dimensions", () => ({ exportMapDimensions: mockExportMapDimensions }));
jest.mock("./export-map-attributes", () => ({ exportMapAttributes: mockExportMapAttributes }));
jest.mock("./export-npcs", () => ({ exportNpcData: mockExportNpcData }));
jest.mock("./export-story-events", () => ({ exportStoryEvents: mockExportStoryEvents }));
jest.mock("./export-phone-scripts", () => ({ exportPhoneScripts: mockExportPhoneScripts }));
jest.mock("./export-battle-animations", () => ({ exportBattleAnimations: mockExportBattleAnimations }));
jest.mock("./export-runtime-assets", () => ({ exportRuntimeAssets: mockExportRuntimeAssets }));
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
      "exportCoreContentPack",
    ]);
    expect(mockExportPlayability).toHaveBeenCalledWith({ itemIds: [] });
    expect(mockExportPokemonCryMetadataFromAsm).toHaveBeenCalledWith(["BULBASAUR"]);
    expect(mockExportAudioAssets).toHaveBeenCalledWith({
      BULBASAUR: { cry: "CRY_BULBASAUR", pitch: 128, length: 129 },
    });
    expect(mockExportCoreContentPack).toHaveBeenCalledWith({
      pokemonData: [{ id: "BULBASAUR", int_id: 1 }],
      movesData: {},
      learnsetsData: {},
      levelUpMovesData: {},
      eggMovesData: {},
      evolutions: [],
      wildEncounters: [],
      runtimeSpawnPoints: { "0": { identifier: 0, mapConstant: "PLAYERS_HOUSE_2F", mapName: "PlayersHouse2F" } },
      runtimeMapMetadata: { PLAYERS_HOUSE_2F: { constant: "PLAYERS_HOUSE_2F", name: "PlayersHouse2F" } },
      mapDimensions: {},
      mapAttributes: {},
      items: [],
      fleeMons: { always: ["RAIKOU"], often: [], sometimes: [] },
      marts: { MART_CHERRYGROVE: ["POTION"] },
      pcStrings: { PCString_ChooseaPKMN: "Choose a PKMN." },
      menuIcons: { CHIKORITA: "ICON_CHIKORITA" },
      pokedexEntries: [{ species: "CHIKORITA", classification: "LEAF", heightDigits: 211, weightDigits: 140, pages: [] }],
      pokemonFrontpicAnimations: { CHIKORITA: { commands: [{ kind: "endanim" }] } },
      initializeEvents: { eventFlags: ["EVENT_INITIAL"], engineFlags: [], variableSprites: {} },
      storyEventScriptConstants: { global: { TRUE: 1 }, maps: {} },
      phoneContacts: { PHONE_MOM: { contactId: "PHONE_MOM" } },
      permanentPhoneNumbers: ["PHONE_MOM"],
      specialPhoneCalls: ["SPECIALCALL_NONE"],
      npcTrades: ["NPC_TRADE_MIKE"],
      specialRoutines: ["FadeOutMusic"],
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
      pokedex: [],
      npcData: {},
      pokegearLandmarks: { landmarks: [], map_to_landmark: {} },
      playability: { start_maps: ["PlayersHouse2F"] },
      audioAssets: [{ id: "MUSIC_ROUTE_29", path: "content-packs/core-modular/music/MUSIC_ROUTE_29.mid", kind: "music" }],
    });
  });
});
