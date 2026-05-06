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
  exportBattleAnimations();
  exportRuntimeAssets();
  const pokegearLandmarks = exportPokegearLandmarks();
  exportPokegearPaletteMap();
  exportGraphicsAssets();
  const pokedex = exportPokedex();
  exportCoreContentPack({
    pokemonData,
    movesData,
    learnsetsData,
    levelUpMovesData,
    eggMovesData,
    evolutions,
    wildEncounters,
    mapDimensions,
    mapAttributes,
    items,
    trainers,
    pokedex,
    npcData,
    pokegearLandmarks,
  });
}

if (require.main === module) {
  exportCoreData();
}
