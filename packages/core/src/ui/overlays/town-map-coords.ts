import type { LandmarkEntry } from "@pokecrystal/assets/content/pokegear";

const OAM_X_OFFSET = 8;
const OAM_Y_OFFSET = 16;
const MAP_WIDTH = 160;
const MAP_HEIGHT = 144;

// Landmark data stores Game Boy OAM coordinates, matching GetLandmarkCoords.
// Convert them back to surface pixels before drawing software overlays.
export const projectLandmarkToTownMapPixel = (entry: LandmarkEntry): [number, number] => {
  const rawX = Math.trunc(Number(entry.x ?? 0)) - OAM_X_OFFSET;
  const rawY = Math.trunc(Number(entry.y ?? 0)) - OAM_Y_OFFSET;
  const x = Math.max(0, Math.min(MAP_WIDTH - 1, rawX));
  const y = Math.max(0, Math.min(MAP_HEIGHT - 1, rawY));
  return [x, y];
};
