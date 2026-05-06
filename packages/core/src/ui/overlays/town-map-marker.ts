import type { Surface } from "@pokecrystal/core/ui/surface";

const CURSOR_SIZE = 16;
const CURSOR_ANCHOR = 8;

// ASM: SPRITE_ANIM_OBJ_POKEGEAR_ARROW uses SPRITE_ANIM_FRAMESET_STILL_CURSOR
// with SPRITE_ANIM_OAMSET_STILL_CURSOR and tile id $04 from gfx/pokegear/pokegear_sprites.png.
const POKEGEAR_CURSOR_MASK = [
  "#######..#######",
  "#######..#######",
  "##............##",
  "##............##",
  "##............##",
  "##............##",
  "##............##",
  "................",
  "................",
  "##............##",
  "##............##",
  "##............##",
  "##............##",
  "##............##",
  "#######..#######",
  "#######..#######",
] as const;

export const drawTownMapCursorMarker = (
  surface: Surface,
  center: [number, number],
  color: [number, number, number] = [0, 0, 0],
): void => {
  const [centerX, centerY] = center;
  const originX = centerX - CURSOR_ANCHOR;
  const originY = centerY - CURSOR_ANCHOR;
  for (let y = 0; y < CURSOR_SIZE; y += 1) {
    const row = POKEGEAR_CURSOR_MASK[y] ?? "";
    for (let x = 0; x < CURSOR_SIZE; x += 1) {
      if (row[x] !== "#") {
        continue;
      }
      const targetX = originX + x;
      const targetY = originY + y;
      if (targetX < 0 || targetY < 0 || targetX >= surface.width || targetY >= surface.height) {
        continue;
      }
      surface.setAt(targetX, targetY, [color[0], color[1], color[2], 255]);
    }
  }
};
