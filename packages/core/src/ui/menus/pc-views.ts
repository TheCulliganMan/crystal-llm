// ASM mapping: pokecrystal_disassembly/engine/menus/bills_pc.asm (BillsPC_RefreshTextboxes/OAM cursor).
import { Surface } from "@pokecrystal/core/ui/surface";
import { TilemapSurface, SPACE_TILE } from "@pokecrystal/core/ui/tilemap-surface";
import { buildDefaultCharMap } from "@pokecrystal/core/ui/text/glyph-map";
import { renderFontText } from "@pokecrystal/core/ui/text/render-font";
import { GameButton, buttonKeys } from "@pokecrystal/core/input/buttons";
import { formatDefaultBoxName } from "@pokecrystal/core/core/models/box";
import { NAME_LENGTH } from "@pokecrystal/core/core/constants";
import { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { PC_ARROW_TILE_IDS, PC_TEXT_PALETTE, getPcArrowTiles, getPcCursorTile } from "./pc-wallpaper";
import { PCLayout, TileRegion } from "./pc-layout";

const TILE_SIZE = 8;
const CHAR_MAP = buildDefaultCharMap();

export const PC_WINDOW_FILL: [number, number, number] = [255, 255, 255];

export const OAM_XFLIP = 1 << 5;
export const OAM_YFLIP = 1 << 6;
export const SPRITE_X_OFFSET = 8;
export const SPRITE_Y_OFFSET = 16;
export const CURSOR_ROW_HEIGHT = TILE_SIZE * 2;

const Z_INDEX_PC_WINDOW = 10;
const Z_INDEX_PC_BOX_WINDOW = 15;

export interface CursorOAMEntry {
  x: number;
  y: number;
  tileId: number;
  attributes: number;
}

export type PCCursorVariant = "selection" | "insert";
export type PCCursorSpriteSpec = readonly [
  xTile: number,
  yTile: number,
  xOffset: number,
  yOffset: number,
  tileId: number,
  attributes: number,
];

export const BILLS_PC_SELECTION_CURSOR: readonly PCCursorSpriteSpec[] = [
  [10, 4, 0, 6, 0x00, 0],
  [11, 4, 0, 6, 0x00, 0],
  [12, 4, 0, 6, 0x00, 0],
  [13, 4, 0, 6, 0x00, 0],
  [14, 4, 0, 6, 0x00, 0],
  [15, 4, 0, 6, 0x00, 0],
  [16, 4, 0, 6, 0x00, 0],
  [17, 4, 0, 6, 0x00, 0],
  [18, 4, 0, 6, 0x00, 0],
  [18, 4, 7, 6, 0x00, 0],
  [10, 7, 0, 1, 0x00, OAM_YFLIP],
  [11, 7, 0, 1, 0x00, OAM_YFLIP],
  [12, 7, 0, 1, 0x00, OAM_YFLIP],
  [13, 7, 0, 1, 0x00, OAM_YFLIP],
  [14, 7, 0, 1, 0x00, OAM_YFLIP],
  [15, 7, 0, 1, 0x00, OAM_YFLIP],
  [16, 7, 0, 1, 0x00, OAM_YFLIP],
  [17, 7, 0, 1, 0x00, OAM_YFLIP],
  [18, 7, 0, 1, 0x00, OAM_YFLIP],
  [18, 7, 7, 1, 0x00, OAM_YFLIP],
  [9, 5, 6, 6, 0x01, 0],
  [9, 6, 6, 1, 0x01, OAM_YFLIP],
  [19, 5, 1, 6, 0x01, OAM_XFLIP],
  [19, 6, 1, 1, 0x01, OAM_XFLIP | OAM_YFLIP],
];

export const BILLS_PC_INSERT_CURSOR: readonly PCCursorSpriteSpec[] = [
  [10, 4, 0, 7, 0x06, 0],
  [11, 5, 0, 3, 0x00, OAM_YFLIP],
  [12, 5, 0, 3, 0x00, OAM_YFLIP],
  [13, 5, 0, 3, 0x00, OAM_YFLIP],
  [14, 5, 0, 3, 0x00, OAM_YFLIP],
  [15, 5, 0, 3, 0x00, OAM_YFLIP],
  [16, 5, 0, 3, 0x00, OAM_YFLIP],
  [17, 5, 0, 3, 0x00, OAM_YFLIP],
  [18, 5, 0, 3, 0x00, OAM_YFLIP],
  [19, 4, 0, 7, 0x07, 0],
];

const cursorTableForVariant = (variant: PCCursorVariant): readonly PCCursorSpriteSpec[] =>
  variant === "insert" ? BILLS_PC_INSERT_CURSOR : BILLS_PC_SELECTION_CURSOR;

const flipSurface = (surface: Surface, flipX: boolean, flipY: boolean): Surface => {
  if (!flipX && !flipY) {
    return surface;
  }
  const [width, height] = surface.get_size();
  const flipped = new Surface(width, height);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const sourceX = flipX ? width - 1 - x : x;
      const sourceY = flipY ? height - 1 - y : y;
      flipped.set_at([x, y], surface.get_at([sourceX, sourceY]));
    }
  }
  return flipped;
};

const CURSOR_TILE_CACHE = new Map<string, Surface>();

const dominantRgb = (surface: Surface): [number, number, number] => {
  const [width, height] = surface.get_size();
  const counts = new Map<string, number>();
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const [r, g, b] = surface.get_at([x, y]);
      const key = `${r},${g},${b}`;
      counts.set(key, (counts.get(key) ?? 0) + 1);
    }
  }
  let dominant = "0,0,0";
  let dominantCount = -1;
  for (const [key, count] of counts) {
    if (count > dominantCount) {
      dominant = key;
      dominantCount = count;
    }
  }
  return dominant.split(",").map((component) => Number(component)) as [number, number, number];
};

const pcObjectTileForOam = (tileId: number, attributes: number): Surface => {
  const base = getPcCursorTile(tileId);
  const flipped = flipSurface(base, Boolean(attributes & OAM_XFLIP), Boolean(attributes & OAM_YFLIP));
  const objectTile = flipped.copy();
  // The PC cursor is OBJ-backed; color 0 is transparent on hardware.
  // The PNG fallback has already been mapped to RGB, so key the dominant background color.
  objectTile.set_colorkey(dominantRgb(base));
  return objectTile;
};

const pcCursorTileForOam = (tileId: number, attributes: number): Surface => {
  const key = `${tileId}:${attributes & (OAM_XFLIP | OAM_YFLIP)}`;
  const cached = CURSOR_TILE_CACHE.get(key);
  if (cached) {
    return cached;
  }
  const tile = pcObjectTileForOam(tileId, attributes);
  CURSOR_TILE_CACHE.set(key, tile);
  return tile;
};

export class BillsPCCursorOAM {
  public entries: CursorOAMEntry[] = [];

  update(cursorRow: number, listingHeight: number, variant: PCCursorVariant = "selection"): void {
    if (listingHeight <= 0 || cursorRow < 0) {
      this.entries = [];
      return;
    }
    const row = Math.min(cursorRow, listingHeight - 1);
    const rowOffset = row * CURSOR_ROW_HEIGHT;
    const entries: CursorOAMEntry[] = [];
    for (const [xTile, yTile, xOffset, yOffset, tileId, attributes] of cursorTableForVariant(variant)) {
      const baseX = xTile * TILE_SIZE + xOffset - SPRITE_X_OFFSET;
      const baseY = yTile * TILE_SIZE + yOffset + rowOffset - SPRITE_Y_OFFSET;
      entries.push({
        x: baseX,
        y: baseY,
        tileId,
        attributes,
      });
    }
    this.entries = entries;
  }
}

export const BOX_ROWS = 5;
export const BOX_COLUMNS = 4;
export const SLOTS_PER_BOX = BOX_ROWS * BOX_COLUMNS;
export const BILLS_PC_LIST_NAME_MAX_CHARS = NAME_LENGTH - 1;

interface FontRenderer {
  renderText: (
    text: string,
    x: number,
    y: number,
    surface: Surface,
    options?: { textWidth?: number; maxLines?: number; uppercase?: boolean }
  ) => void;
  getCharTile?: (char: string) => Surface | null | undefined;
}

interface PCWindowUI {
  font: FontRenderer;
  drawWindow: (
    surface: Surface,
    x: number,
    y: number,
    width: number,
    height: number,
    options?: { fill?: [number, number, number]; zIndex?: number }
  ) => void;
}

const swapPcTextboxCorners = (ui: PCWindowUI, screen: Surface, originX: number, originY: number, width: number): void => {
  const left = ui.font.getCharTile?.("└");
  if (left) {
    screen.blit(left, [originX, originY]);
  }
  const right = ui.font.getCharTile?.("┘");
  if (right) {
    screen.blit(right, [originX + (width - 1) * TILE_SIZE, originY]);
  }
};

export const drawPcTextbox = (
  ui: PCWindowUI,
  screen: Surface | TilemapSurface,
  region: TileRegion,
  {
    swapTopCorners = false,
    fill = PC_WINDOW_FILL,
    zIndex = Z_INDEX_PC_WINDOW,
  }: { swapTopCorners?: boolean; fill?: [number, number, number]; zIndex?: number } = {}
): [number, number] => {
  const [originX, originY] = region.toPixels();
  if (screen instanceof TilemapSurface) {
    screen.drawWindow(region.x, region.y, region.width, region.height, {
      attr: PC_TEXT_PALETTE,
      fillTile: SPACE_TILE,
    });
    if (swapTopCorners) {
      screen.setTile(region.x, region.y, CHAR_MAP["└"]);
      screen.setTile(region.x + region.width - 1, region.y, CHAR_MAP["┘"]);
    }
    return [originX, originY];
  }
  ui.drawWindow(screen, originX, originY, region.width, region.height, { fill, zIndex });
  if (swapTopCorners) {
    swapPcTextboxCorners(ui, screen, originX, originY, region.width);
  }
  return [originX, originY];
};

export const playPcBootSound = (audioEngine: AudioEngine | null): void => {
  audioEngine?.playSound("SFX_BOOT_PC");
};

export const playPcSwitchSound = (audioEngine: AudioEngine | null): void => {
  audioEngine?.playSound("SFX_SWITCH_POKEMON");
};

export const playPcShutdownSound = (audioEngine: AudioEngine | null): void => {
  audioEngine?.playSound("SFX_SHUT_DOWN_PC");
};

export class PCMessageWindowView {
  private readonly leftArrow: Surface;
  private readonly rightArrow: Surface;

  constructor(private readonly region: TileRegion) {
    [this.leftArrow, this.rightArrow] = getPcArrowTiles();
  }

  draw(
    ui: PCWindowUI,
    target: Surface | TilemapSurface,
    boxIndex: number,
    boxName: string,
    { showArrows = false }: { showArrows?: boolean } = {}
  ): void {
    const [originX, originY] = drawPcTextbox(ui, target, this.region, { zIndex: Z_INDEX_PC_BOX_WINDOW });
    const label = boxName || formatDefaultBoxName(boxIndex);
    if (target instanceof TilemapSurface) {
      target.writeText(this.region.x + 1, this.region.y + 1, label, {
        maxLength: this.region.width - 2,
        pad: true,
      });
      if (showArrows) {
        const arrowRow = this.region.y + 1;
        target.setTile(this.region.x, arrowRow, PC_ARROW_TILE_IDS.left);
        target.setTile(19, arrowRow, PC_ARROW_TILE_IDS.right);
      }
      return;
    }
    renderFontText(ui.font, label, originX + TILE_SIZE, originY + TILE_SIZE, target, {
      textWidth: Math.max(1, (this.region.width - 2) * TILE_SIZE),
      maxLines: 1,
    });
    if (showArrows) {
      const arrowY = originY + TILE_SIZE;
      target.blit(this.leftArrow, [this.region.x * TILE_SIZE, arrowY]);
      target.blit(this.rightArrow, [19 * TILE_SIZE, arrowY]);
    }
  }
}

export class BillsPCCursorView {
  private readonly oam = new BillsPCCursorOAM();

  constructor(_region: TileRegion, _rowHeight: number, private readonly rows: number) {}

  draw(screen: Surface, row: number, variant: PCCursorVariant = "selection"): void {
    this.oam.update(row, this.rows, variant);
    for (const entry of this.oam.entries) {
      screen.blit(pcCursorTileForOam(entry.tileId, entry.attributes), [entry.x, entry.y]);
    }
  }

  updateOam(row: number, variant: PCCursorVariant = "selection"): void {
    this.oam.update(row, this.rows, variant);
  }

  get oamEntries(): CursorOAMEntry[] {
    return this.oam.entries;
  }
}

export class BillsPCListView {
  constructor(
    private readonly region: TileRegion,
    private readonly rows: number,
    private readonly placeholder: string,
    private readonly cancelLabel: string,
    private readonly cursorView: BillsPCCursorView
  ) {}

  draw(
    ui: PCWindowUI,
    target: Surface | TilemapSurface,
    entries: Array<{ nickname?: string }>,
    scroll: number,
    cursor: number,
    { cursorSurface }: { cursorSurface?: Surface } = {}
  ): void {
    this.cursorView.updateOam(cursor);
    const [originX, originY] = drawPcTextbox(ui, target, this.region, { swapTopCorners: true });
    if (target instanceof TilemapSurface) {
      if (cursorSurface && cursor >= 0 && cursor < this.rows) {
        this.cursorView.draw(cursorSurface, cursor);
      }
      const textX = this.region.x + 1;
      const textY = this.region.y + 2;
      for (let row = 0; row < this.rows; row++) {
        const label = this.labelForRow(entries, scroll + row);
        target.writeText(textX, textY + row * 2, label, {
          maxLength: this.region.width - 2,
          pad: true,
        });
      }
      return;
    }
    if (cursor >= 0 && cursor < this.rows) {
      this.cursorView.draw(target, cursor);
    }
    const textX = originX + TILE_SIZE;
    const textY = originY + TILE_SIZE * 2;
    for (let row = 0; row < this.rows; row++) {
      const label = this.labelForRow(entries, scroll + row);
      renderFontText(ui.font, label, textX, textY + row * TILE_SIZE * 2, target);
    }
  }

  private labelForRow(entries: Array<{ nickname?: string }>, index: number): string {
    if (index < entries.length) {
      const nickname = entries[index].nickname ?? "";
      return (nickname || this.placeholder).toUpperCase().slice(0, BILLS_PC_LIST_NAME_MAX_CHARS);
    }
    if (index === entries.length) {
      return this.cancelLabel;
    }
    return "";
  }
}

export class PCActionMenuView {
  constructor(private readonly layout: PCLayout) {}

  draw(
    ui: PCWindowUI,
    screen: Surface | TilemapSurface,
    actions: string[],
    actionIndex: number,
    partyHasPokemon: boolean,
    mode: string
  ): void {
    if (screen instanceof TilemapSurface) {
      const region = new TileRegion(
        this.layout.actions.x,
        this.layout.actions.y,
        this.layout.actions.width,
        this.layout.actions.height
      );
      drawPcTextbox(ui, screen, region, { zIndex: Z_INDEX_PC_BOX_WINDOW });
      const textX = region.x + 1;
      const textY = region.y + 1;
      const innerWidth = region.width - 2;
      actions.forEach((label, index) => {
        const cursor = index === actionIndex ? "▶" : " ";
        const text = label === "DEPOSIT" && !partyHasPokemon ? `${label}*` : label;
        screen.writeText(textX, textY + index, `${cursor}${text}`, {
          maxLength: innerWidth,
          pad: true,
        });
      });
      return;
    }
    const [originX, originY] = this.layout.actions.toPixels();
    ui.drawWindow(screen, originX, originY, this.layout.actions.width, this.layout.actions.height, {
      fill: PC_WINDOW_FILL,
      zIndex: Z_INDEX_PC_BOX_WINDOW,
    });
    actions.forEach((label, index) => {
      const entryY = originY + (index + 1) * TILE_SIZE;
      const cursor = index === actionIndex ? "▶" : " ";
      const text = label === "DEPOSIT" && !partyHasPokemon ? `${label}*` : label;
      renderFontText(ui.font, `${cursor}${text}`, originX + TILE_SIZE, entryY, screen);
    });
  }
}

export class PCBottomPromptView {
  constructor(private readonly region: TileRegion) {}

  draw(ui: PCWindowUI, screen: Surface | TilemapSurface, text: string): void {
    const [originX, originY] = drawPcTextbox(ui, screen, this.region, { zIndex: Z_INDEX_PC_BOX_WINDOW });
    if (screen instanceof TilemapSurface) {
      screen.writeText(this.region.x + 1, this.region.y + 1, text, {
        maxLength: this.region.width - 2,
        pad: true,
      });
      return;
    }
    renderFontText(ui.font, text, originX + TILE_SIZE, originY + TILE_SIZE, screen, {
      textWidth: Math.max(1, (this.region.width - 2) * TILE_SIZE),
      maxLines: 1,
      uppercase: false,
    });
  }
}

export class PCBrowseNavigator {
  private static readonly LEFT_KEYS = new Set(["ArrowLeft"]);
  private static readonly RIGHT_KEYS = new Set(["ArrowRight"]);
  private static readonly UP_KEYS = new Set(["ArrowUp"]);
  private static readonly DOWN_KEYS = new Set(["ArrowDown"]);
  private static readonly BOX_PREV_KEYS = new Set(["KeyQ"]);
  private static readonly BOX_NEXT_KEYS = new Set(["KeyE"]);
  private static readonly CONFIRM_KEYS = new Set(buttonKeys[GameButton.A]);
  private static readonly CANCEL_KEYS = new Set(buttonKeys[GameButton.B]);

  private readonly slots: number;

  constructor(rows: number, columns: number) {
    this.slots = rows * columns;
  }

  handleKey(
    keyName: string,
    keyCode: number | null,
    boxIndex: number,
    cursorSlot: number,
    boxCount: number
  ): [number, number, string | null] {
    const totalBoxes = Math.max(1, boxCount);
    if (PCBrowseNavigator.LEFT_KEYS.has(keyName)) {
      return [boxIndex, (cursorSlot - 1 + this.slots) % this.slots, "cursor_move"];
    }
    if (PCBrowseNavigator.RIGHT_KEYS.has(keyName)) {
      return [boxIndex, (cursorSlot + 1) % this.slots, "cursor_move"];
    }
    if (PCBrowseNavigator.UP_KEYS.has(keyName)) {
      return [boxIndex, (cursorSlot - BOX_COLUMNS + this.slots) % this.slots, "cursor_move"];
    }
    if (PCBrowseNavigator.DOWN_KEYS.has(keyName)) {
      return [boxIndex, (cursorSlot + BOX_COLUMNS) % this.slots, "cursor_move"];
    }
    if (PCBrowseNavigator.BOX_PREV_KEYS.has(keyName)) {
      return [(boxIndex - 1 + totalBoxes) % totalBoxes, cursorSlot, "box_change"];
    }
    if (PCBrowseNavigator.BOX_NEXT_KEYS.has(keyName)) {
      return [(boxIndex + 1) % totalBoxes, cursorSlot, "box_change"];
    }
    if (keyCode !== null && PCBrowseNavigator.CONFIRM_KEYS.has(keyCode)) {
      return [boxIndex, cursorSlot, "confirm"];
    }
    if (keyCode !== null && PCBrowseNavigator.CANCEL_KEYS.has(keyCode)) {
      return [boxIndex, cursorSlot, "cancel"];
    }
    return [boxIndex, cursorSlot, null];
  }
}

export class PCActionNavigator {
  private static readonly MOVE_UP = new Set(["ArrowUp", "ArrowLeft"]);
  private static readonly MOVE_DOWN = new Set(["ArrowDown", "ArrowRight"]);
  private static readonly CONFIRM_KEYS = new Set(buttonKeys[GameButton.A]);
  private static readonly CANCEL_KEYS = new Set(buttonKeys[GameButton.B]);

  constructor(private actionCount: number) {}

  handleKey(keyName: string, keyCode: number | null, currentIndex: number): [number, string | null] {
    if (PCActionNavigator.MOVE_UP.has(keyName)) {
      return [(currentIndex - 1 + this.actionCount) % this.actionCount, "cursor_move"];
    }
    if (PCActionNavigator.MOVE_DOWN.has(keyName)) {
      return [(currentIndex + 1) % this.actionCount, "cursor_move"];
    }
    if (keyCode !== null && PCActionNavigator.CONFIRM_KEYS.has(keyCode)) {
      return [currentIndex, "confirm"];
    }
    if (keyCode !== null && PCActionNavigator.CANCEL_KEYS.has(keyCode)) {
      return [currentIndex, "cancel"];
    }
    return [currentIndex, null];
  }

  updateActionCount(actionCount: number): void {
    this.actionCount = Math.max(1, actionCount);
  }
}

export class PCMoveNavigator {
  private static readonly CONFIRM_KEYS = new Set(buttonKeys[GameButton.A]);
  private static readonly CANCEL_KEYS = new Set(buttonKeys[GameButton.B]);

  handleKey(keyName: string, keyCode: number | null): string | null {
    if (keyCode !== null && PCMoveNavigator.CONFIRM_KEYS.has(keyCode)) {
      return "confirm";
    }
    if (keyCode !== null && PCMoveNavigator.CANCEL_KEYS.has(keyCode)) {
      return "cancel";
    }
    return null;
  }
}

export class PCDepositNavigator {
  private static readonly UP_KEYS = new Set(["ArrowUp"]);
  private static readonly DOWN_KEYS = new Set(["ArrowDown"]);
  private static readonly CONFIRM_KEYS = new Set(buttonKeys[GameButton.A]);
  private static readonly CANCEL_KEYS = new Set(buttonKeys[GameButton.B]);

  public partySize: number;
  public cursor: number;

  constructor(partySize: number) {
    this.partySize = Math.max(0, partySize);
    this.cursor = this.partySize === 0 ? -1 : 0;
  }

  updatePartySize(size: number): void {
    this.partySize = Math.max(0, size);
    if (this.partySize === 0) {
      this.cursor = -1;
    } else {
      this.cursor = Math.max(0, Math.min(this.cursor, this.partySize - 1));
    }
  }

  handleKey(keyName: string, keyCode: number | null): string | null {
    if (PCDepositNavigator.UP_KEYS.has(keyName)) {
      if (this.partySize) {
        this.cursor = Math.max(0, this.cursor - 1);
      }
      return null;
    }
    if (PCDepositNavigator.DOWN_KEYS.has(keyName)) {
      if (this.partySize) {
        this.cursor = Math.min(this.partySize - 1, this.cursor + 1);
      }
      return null;
    }
    if (keyCode !== null && PCDepositNavigator.CONFIRM_KEYS.has(keyCode)) {
      return this.partySize ? "confirm" : null;
    }
    if (keyCode !== null && PCDepositNavigator.CANCEL_KEYS.has(keyCode)) {
      return "cancel";
    }
    return null;
  }
}
