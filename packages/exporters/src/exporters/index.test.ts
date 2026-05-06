const mockCalls: string[] = [];

const mockExportData = jest.fn(() => {
  mockCalls.push("exportData");
  return { pokemonData: [], movesData: {}, learnsetsData: {}, levelUpMovesData: {}, eggMovesData: {} };
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
});
const mockExportRuntimeAssets = jest.fn(() => {
  mockCalls.push("exportRuntimeAssets");
});
const mockExportPokegearLandmarks = jest.fn(() => {
  mockCalls.push("exportPokegearLandmarks");
  return { landmarks: [], map_to_landmark: {} };
});
const mockExportPokegearPaletteMap = jest.fn(() => {
  mockCalls.push("exportPokegearPaletteMap");
});
const mockExportGraphicsAssets = jest.fn(() => {
  mockCalls.push("exportGraphicsAssets");
});
const mockExportPokedex = jest.fn(() => {
  mockCalls.push("exportPokedex");
  return [];
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
      "exportCoreContentPack",
    ]);
    expect(mockExportCoreContentPack).toHaveBeenCalledWith({
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
  });
});
