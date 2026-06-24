import { exportData } from "./export-data";
import { exportItems } from "./export-items";
import { exportEvolutions } from "./export-evolutions";
import { exportWildEncounters } from "./export-wild-encounters";
import { exportTrainers } from "./export-trainers";
import { exportMapDimensions } from "./export-map-dimensions";
import { exportMapAttributes } from "./export-map-attributes";
import { exportStoryEvents } from "./export-story-events";
import { exportPhoneScripts } from "./export-phone-scripts";
import { exportPokedex } from "./export-pokedex";
import { exportCoreContentPack } from "./export-content-pack";
import { exportNpcData } from "./export-npcs";
import { exportPokegearLandmarks } from "./export-pokegear-landmarks";
import { exportGraphicsAssets } from "./export-graphics-assets";
import { exportPokegearPaletteMap } from "./export-pokegear-palette-map";
import { exportBattleAnimations } from "./export-battle-animations";
import { exportRuntimeAssets } from "./export-runtime-assets";
import { exportPlayability } from "./export-playability";
import { exportAudioAssets, exportPokemonCryMetadataFromAsm } from "./export-audio-assets";
import { readJsonAssetSync } from "@pokecrystal/core/core/asset-reader";
import { getDataDir } from "@pokecrystal/core/core/paths";
import { joinPath } from "@pokecrystal/core/core/path-utils";

export { exportNpcData } from "./export-npcs";

export function exportCoreData(): void {
  const { pokemonData, movesData, learnsetsData, levelUpMovesData, eggMovesData } = exportData();
  const items = exportItems();
  const evolutions = exportEvolutions();
  const wildEncounters = exportWildEncounters();
  const trainers = exportTrainers(pokemonData);
  const mapDimensions = exportMapDimensions();
  const mapAttributes = exportMapAttributes();
  const npcData = exportNpcData();
  exportStoryEvents();
  exportPhoneScripts();
  const battleAnimations = exportBattleAnimations();
  const runtimeAssets = exportRuntimeAssets();
  const pokegearLandmarks = exportPokegearLandmarks();
  const pokegearTownMapPaletteMap = exportPokegearPaletteMap();
  exportGraphicsAssets();
  const pokedex = exportPokedex();
  const playability = exportPlayability({ itemIds: items.map((item) => item.script_name) });
  const pokemonCries = exportPokemonCryMetadataFromAsm(pokemonData.map((pokemon) => pokemon.id));
  const audioAssets = exportAudioAssets(pokemonCries);
  const runtimeSpawnPoints = readJsonAssetSync(joinPath(getDataDir(), "runtime_spawn_points.json"));
  const runtimeMapMetadata = readJsonAssetSync(joinPath(getDataDir(), "runtime_map_metadata.json"));
  const initializeEvents = readJsonAssetSync(joinPath(getDataDir(), "initialize_events.json"));
  const storyEventScriptConstants = readJsonAssetSync(joinPath(getDataDir(), "story_event_script_constants.json"));
  const asmText = readJsonAssetSync(joinPath(getDataDir(), "asm_text.json"));
  const moveNames = readJsonAssetSync(joinPath(getDataDir(), "move_names.json"));
  const battleAnimationTable = readJsonAssetSync(joinPath(getDataDir(), "battle_animation_table.json"));
  const battleAnimBundle = readJsonAssetSync(joinPath(getDataDir(), "battle_anim_bundle.json"));
  const spriteAnimBundle = readJsonAssetSync(joinPath(getDataDir(), "sprite_anim_bundle.json"));
  const spritePaletteDefaults = readJsonAssetSync(joinPath(getDataDir(), "sprite_palette_defaults.json"));
  exportCoreContentPack({
    pokemonData,
    movesData,
    learnsetsData,
    levelUpMovesData,
    eggMovesData,
    evolutions,
    wildEncounters,
    runtimeSpawnPoints,
    runtimeMapMetadata,
    mapDimensions,
    mapAttributes,
    items,
    fleeMons: runtimeAssets.fleeMons,
    marts: runtimeAssets.marts,
    pcStrings: runtimeAssets.pcStrings,
    menuIcons: runtimeAssets.menuIcons,
    pokedexEntries: runtimeAssets.pokedexEntries,
    pokemonFrontpicAnimations: runtimeAssets.pokemonFrontpicAnimations,
    initializeEvents,
    storyEventScriptConstants,
    phoneContacts: runtimeAssets.phoneContacts,
    permanentPhoneNumbers: runtimeAssets.permanentPhoneNumbers,
    specialPhoneCalls: runtimeAssets.specialPhoneCalls,
    npcTrades: runtimeAssets.npcTrades,
    specialRoutines: runtimeAssets.specialRoutines,
    asmText: asmText as Record<string, string>,
    moveNames: moveNames as string[],
    battleAnimations,
    battleAnimationTable: battleAnimationTable as string[],
    battleAnimBundle,
    spriteAnimBundle,
    spritePaletteDefaults: spritePaletteDefaults as Record<string, number>,
    pokegearTownMapPaletteMap,
    pokemonCries,
    trainers,
    pokedex,
    npcData,
    pokegearLandmarks,
    playability,
    audioAssets,
  });
}

if (require.main === module) {
  exportCoreData();
}
