import { gameEngine, Surface, Rect } from "@pokecrystal/core/ui/game-engine";
import type { GameEngineEvent } from "@pokecrystal/core/ui/game-engine";
import { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { CompositeUI, ScreenUI, isTextUI } from "@pokecrystal/core/ui/screens/screen-types";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { gbc5To8 } from "@pokecrystal/core/core/gbc-colors";
import {
  GameButton,
  isButtonEvent,
  isCancelEvent,
  isConfirmEvent,
  isKeyDownEvent,
  isKeyUpEvent,
  mapKeyToButton,
  mapKeyToDirection,
} from "@pokecrystal/core/input/controls";
import { TILE_SIZE } from "@pokecrystal/core/engine/world/tile";

const INITIAL_REPEAT_DELAY_FRAMES = 12;
const REPEAT_INTERVAL_FRAMES = 4;
const DIPLOMA_BG_PALETTE = [
  [27, 31, 27],
  [21, 21, 21],
  [13, 13, 13],
  [0, 0, 0],
].map(([r, g, b]) => [gbc5To8(r), gbc5To8(g), gbc5To8(b)] as const);

const applyDiplomaBackgroundPalette = (source: Surface): Surface => {
  const tinted = new gameEngine.Surface(source.get_width(), source.get_height());
  for (let y = 0; y < source.get_height(); y += 1) {
    for (let x = 0; x < source.get_width(); x += 1) {
      const [r, g, b, a] = source.get_at([x, y]);
      const grayscale = a === 0 ? 255 : Math.round((r + g + b) / 3);
      const paletteIndex = Math.max(0, Math.min(3, Math.round((255 - grayscale) / 85)));
      const [pr, pg, pb] = DIPLOMA_BG_PALETTE[paletteIndex];
      tinted.set_at([x, y], [pr, pg, pb, 255]);
    }
  }
  return tinted;
};

type NameEntryEvent = GameEngineEvent;

type NameEntryEventSnapshot = {
  type: string;
  key?: string | null;
  direction?: string | null;
  button?: GameButton | null;
  is_press?: boolean | null;
};

const hasChildUis = (candidate: unknown): candidate is { getChildren: () => unknown[] } =>
  Boolean(candidate) && typeof (candidate as { getChildren?: unknown }).getChildren === "function";

const directionForEvent = (event: NameEntryEvent): string | null => {
  if (typeof event.direction === "string") {
    return event.direction;
  }
  // Browser keyboard events include both the literal key and its resolved Game
  // Boy button. KeyA is an A-button binding, so do not reinterpret its `a` key
  // value as the CLI movement shortcut for left.
  if (typeof event.button === "string") {
    return null;
  }
  const rawKey = event.key ?? event.code ?? null;
  if (typeof rawKey === "string") {
    const lowered = rawKey.toLowerCase();
    if (["up", "w"].includes(lowered)) return "up";
    if (["down", "s"].includes(lowered)) return "down";
    if (["left", "a"].includes(lowered)) return "left";
    if (["right", "d"].includes(lowered)) return "right";
  }
  return mapKeyToDirection(rawKey);
};

const isKeydown = (event: NameEntryEvent): boolean => {
  if (event.is_press !== undefined && event.is_press !== null) {
    return Boolean(event.is_press);
  }
  return isKeyDownEvent(event);
};

const isKeyup = (event: NameEntryEvent): boolean => {
  if (event.is_press !== undefined && event.is_press !== null) {
    return !event.is_press;
  }
  return isKeyUpEvent(event);
};

const buttonForEvent = (event: NameEntryEvent): GameButton | null => {
  if (typeof event.button === "string") {
    return event.button as GameButton;
  }
  if (event.key === gameEngine.K_BACKSPACE || event.code === gameEngine.K_BACKSPACE) {
    return GameButton.B;
  }
  if (isConfirmEvent(event)) {
    return GameButton.A;
  }
  if (isCancelEvent(event)) {
    return GameButton.B;
  }
  if (isButtonEvent(event, GameButton.Select)) {
    return GameButton.Select;
  }
  if (isButtonEvent(event, GameButton.Start)) {
    return GameButton.Start;
  }
  return mapKeyToButton(event.code ?? event.key ?? null);
};

class RepeatState {
  private held = new Set<string>();
  private timers = new Map<string, number>();

  isHeld(direction: string): boolean {
    return this.held.has(direction);
  }

  start(direction: string): void {
    this.held.add(direction);
    this.timers.set(direction, INITIAL_REPEAT_DELAY_FRAMES);
  }

  stop(direction: string): void {
    this.held.delete(direction);
    this.timers.delete(direction);
  }

  stopAll(): void {
    this.held.clear();
    this.timers.clear();
  }

  tick(): string[] {
    const repeated: string[] = [];
    for (const direction of Array.from(this.held)) {
      const timer = (this.timers.get(direction) ?? 0) - 1;
      if (timer <= 0) {
        repeated.push(direction);
        this.timers.set(direction, REPEAT_INTERVAL_FRAMES);
      } else {
        this.timers.set(direction, timer);
      }
    }
    return repeated;
  }
}

class NamingScreenTileset {
  static readonly SPACE_TILE_INDEX = 0x7f;
  tiles: Surface[] = [];
  fontTiles: Record<number, Surface> = {};
  specialTiles: Record<string, Surface> = {};

  constructor() {
    this.loadFontTiles();
    this.loadSpecialTiles();
    this.createTilesList();
  }

  getTileSurface(tileIndex: number): Surface {
    if (tileIndex >= 0 && tileIndex < this.tiles.length) {
      return this.tiles[tileIndex];
    }
    return new gameEngine.Surface(TILE_SIZE, TILE_SIZE);
  }

  private loadImage(path: string, label: string): Surface {
    const loader = gameEngine.image.loadSync;
    if (typeof loader !== "function") {
      throw new Error(`Naming screen ${label} requires a synchronous image loader.`);
    }
    const surface = loader(path);
    if (!surface || surface.get_width() <= 0 || surface.get_height() <= 0) {
      throw new Error(
        `Failed to load naming screen ${label} from ${path}. Ensure assets are preloaded.`
      );
    }
    return surface;
  }

  private loadFontTiles(): void {
    // ASM: engine/menus/naming_screen.asm::LoadNamingScreenGFX (LoadStandardFont/LoadFontsExtra).
    const fontPath = getAssetPath("gfx", "font", "font.png");
    const fontSurface = applyDiplomaBackgroundPalette(this.loadImage(fontPath, "font"));
    const tileWidth = TILE_SIZE;
    const tileHeight = TILE_SIZE;
    const tilesPerRow = Math.floor(fontSurface.get_width() / tileWidth);
    const tilesPerColumn = Math.floor(fontSurface.get_height() / tileHeight);
    const totalTiles = Math.min(256, tilesPerRow * tilesPerColumn);

    for (let tileIndex = 0; tileIndex < totalTiles; tileIndex += 1) {
      const row = Math.floor(tileIndex / tilesPerRow);
      const col = tileIndex % tilesPerRow;
      const x = col * tileWidth;
      const y = row * tileHeight;
      if (x + tileWidth > fontSurface.get_width() || y + tileHeight > fontSurface.get_height()) {
        break;
      }
      const rect = new Rect(x, y, tileWidth, tileHeight);
      this.fontTiles[tileIndex] = fontSurface.subsurface(rect);
    }
  }

  private loadSpecialTiles(): void {
    // ASM: engine/menus/naming_screen.asm::LoadNamingScreenGFX (cursor/border/underline/middle line).
    const cursorPath = getAssetPath("gfx", "naming_screen", "cursor.png");
    const cursorSurface = this.loadImage(cursorPath, "cursor");
    const cursorTiles: Surface[] = [];
    const cursorRows = Math.max(1, Math.floor(cursorSurface.get_height() / TILE_SIZE));
    for (let row = 0; row < cursorRows && cursorTiles.length < 2; row += 1) {
      const rect = new Rect(0, row * TILE_SIZE, TILE_SIZE, TILE_SIZE);
      if (rect.bottom > cursorSurface.get_height()) {
        continue;
      }
      cursorTiles.push(cursorSurface.subsurface(rect));
    }
    if (cursorTiles.length !== 2) {
      throw new Error("Naming screen cursor graphics are malformed; expected two tiles.");
    }
    this.specialTiles.cursor_0 = cursorTiles[0];
    this.specialTiles.cursor_1 = cursorTiles[1];

    this.specialTiles.underline = applyDiplomaBackgroundPalette(this.loadImage(
      getAssetPath("gfx", "naming_screen", "underline.png"),
      "underline"
    ));
    this.specialTiles.middle_line = applyDiplomaBackgroundPalette(this.loadImage(
      getAssetPath("gfx", "naming_screen", "middle_line.png"),
      "middle_line"
    ));
    this.specialTiles.border = applyDiplomaBackgroundPalette(this.loadImage(
      getAssetPath("gfx", "naming_screen", "border.png"),
      "border"
    ));
  }

  private createTilesList(): void {
    for (let i = 0; i < 256; i += 1) {
      const tile = new gameEngine.Surface(TILE_SIZE, TILE_SIZE);
      tile.fill([...DIPLOMA_BG_PALETTE[0], 255]);
      this.tiles.push(tile);
    }

    for (let i = 0; i < 128; i += 1) {
      const tile = this.fontTiles[i];
      if (tile) {
        this.tiles[0x80 + i] = tile;
      }
    }

    const cursor0 = this.specialTiles.cursor_0;
    const cursor1 = this.specialTiles.cursor_1;
    if (!cursor0 || !cursor1) {
      throw new Error("Naming screen cursor tiles are missing.");
    }
    const NAMINGSCREEN_CURSOR = 0x7e;
    this.tiles[NAMINGSCREEN_CURSOR] = cursor0;
    this.tiles[NAMINGSCREEN_CURSOR + 1] = cursor1;

    const border = this.specialTiles.border;
    if (border) {
      this.tiles[NameEntryScreen.NAMINGSCREEN_BORDER] = border;
    }
    const middleLine = this.specialTiles.middle_line;
    if (middleLine) {
      this.tiles[NameEntryScreen.NAMINGSCREEN_MIDDLELINE] = middleLine;
    }
    const underline = this.specialTiles.underline;
    if (underline) {
      this.tiles[NameEntryScreen.NAMINGSCREEN_UNDERLINE] = underline;
    }

    const spaceTile = new gameEngine.Surface(TILE_SIZE, TILE_SIZE);
    spaceTile.fill([...DIPLOMA_BG_PALETTE[0], 255]);
    this.tiles[NamingScreenTileset.SPACE_TILE_INDEX] = spaceTile;
  }
}

export class NameEntryScreen {
  static readonly SCREEN_TILE_WIDTH = 20;
  static readonly SCREEN_TILE_HEIGHT = 18;
  static readonly MAX_NAME_LENGTH = 7;

  private readonly tileset = new NamingScreenTileset();
  private readonly repeatState = new RepeatState();
  private readonly textUi: ScreenUI | null;
  private readonly isTextOnlyUi: boolean;

  private cursorSmallSurface: Surface;
  private cursorSmallOffset: [number, number];
  private cursorBigSurface: Surface;
  private cursorBigOffset: [number, number];

  private tilemap: number[][] = [];
  private cursorColumn = 0;
  private cursorRow = 0;
  private nameTiles: number[] = [];
  private nameChars: string[] = [];
  private lastInput: NameEntryEventSnapshot | null = null;

  name = "";
  finished = false;
  case: "upper" | "lower" = "upper";
  defaultMaxNameLength = NameEntryScreen.MAX_NAME_LENGTH;
  maxNameLength = NameEntryScreen.MAX_NAME_LENGTH;

  constructor(
    private readonly ui: ScreenUI,
    private prompt: string,
    private readonly audioEngine: AudioEngine | null = null
  ) {
    this.textUi = this.resolveTextUiTarget(ui);
    this.isTextOnlyUi = isTextUI(ui) && !hasChildUis(ui);
    const [smallSurface, smallOffset] = this.composeCursorSurface(NameEntryScreen.SMALL_CURSOR_OAM);
    this.cursorSmallSurface = smallSurface;
    this.cursorSmallOffset = smallOffset;
    const [bigSurface, bigOffset] = this.composeCursorSurface(NameEntryScreen.BIG_CURSOR_OAM);
    this.cursorBigSurface = bigSurface;
    this.cursorBigOffset = bigOffset;
    this.reset();
  }

  reset(options: { prompt?: string; maxNameLength?: number } = {}): void {
    this.repeatState.stopAll();
    if (options.prompt) {
      this.prompt = options.prompt;
    }
    if (options.maxNameLength !== undefined) {
      if (options.maxNameLength <= 0) {
        throw new Error("Naming screen length must be positive");
      }
      this.defaultMaxNameLength = options.maxNameLength;
    }
    this.maxNameLength = this.defaultMaxNameLength;
    this.case = "upper";
    this.finished = false;
    this.initializeState();
  }

  fillName(defaultName: string): void {
    if (!defaultName) {
      return;
    }
    for (const char of defaultName) {
      if (this.nameChars.length >= this.maxNameLength) {
        break;
      }
      const tileId = this._tileIdForCharacter(char);
      if (tileId === null) {
        continue;
      }
      this.appendCharacter(tileId, char);
    }
  }

  private _tileIdForCharacter(char: string): number | null {
    const id =
      NameEntryScreen.CHAR_TO_TILE[char] ??
      NameEntryScreen.CHAR_TO_TILE[char.toUpperCase()];
    return id ?? null;
  }

  get cursorPos(): [number, number] {
    return [this.cursorColumn, this.cursorRow];
  }

  set cursorPos(value: [number, number]) {
    const [column, row] = value;
    this.cursorColumn = Math.max(0, Math.min(NameEntryScreen.LETTER_X_OFFSETS.length - 1, column));
    this.cursorRow = Math.max(0, Math.min(NameEntryScreen.BOTTOM_ROW_INDEX, row));
  }

  handleInput(event: NameEntryEvent): string | null {
    const direction = directionForEvent(event);
    const button = buttonForEvent(event);
    this.lastInput = {
      type: String(event.type),
      key: typeof event.key === 'string' ? event.key : null,
      direction: direction ?? null,
      button,
      is_press: event.is_press ?? null,
    };

    if (direction && event.is_press === false) {
      this.repeatState.stop(direction);
      return null;
    }

    if (!isKeydown(event) && !(direction && event.is_press === undefined)) {
      return null;
    }

    if (direction) {
      if (!this.repeatState.isHeld(direction)) {
        this.repeatState.start(direction);
      }
      this.moveByDirection(direction);
      return null;
    }

    if (button === GameButton.A) {
      this.pressA();
    } else if (button === GameButton.B) {
      this.pressB();
    } else if (button === GameButton.Select) {
      this.toggleCase();
    } else if (button === GameButton.Start) {
      this.pressStart();
    } else if (!this.isTextOnlyUi && this.handleTextualCharacter(event)) {
      return null;
    }
    return null;
  }

  update(): void {
    for (const direction of this.repeatState.tick()) {
      this.moveByDirection(direction);
    }
  }

  draw(): void {
    if (this.isTextOnlyUi) {
      this.renderTextSnapshot();
      return;
    }
    const screen = this.ui.screen;
    if (!screen || !this.ui.clearScreen) {
      return;
    }
    this.ui.clearScreen([255, 255, 255]);
    const nativeWidth = NameEntryScreen.SCREEN_TILE_WIDTH * TILE_SIZE;
    const nativeHeight = NameEntryScreen.SCREEN_TILE_HEIGHT * TILE_SIZE;
    const nativeSurface = new gameEngine.Surface(nativeWidth, nativeHeight);
    nativeSurface.fill([255, 255, 255, 255]);
    this.renderTilemap(nativeSurface);

    const cursorSurface =
      this.cursorRow === NameEntryScreen.BOTTOM_ROW_INDEX
        ? this.cursorBigSurface
        : this.cursorSmallSurface;
    const [offsetX, offsetY] =
      this.cursorRow === NameEntryScreen.BOTTOM_ROW_INDEX
        ? this.cursorBigOffset
        : this.cursorSmallOffset;
    const [cursorX, cursorY] = this.computeCursorPixelPosition();
    nativeSurface.blit(cursorSurface, [cursorX + offsetX, cursorY + offsetY]);

    const scaleFactor = Math.max(
      1,
      Math.min(
        Math.floor(screen.get_width() / nativeWidth),
        Math.floor(screen.get_height() / nativeHeight)
      )
    );
    const scaledSurface = gameEngine.transform.scale(nativeSurface, [
      nativeWidth * scaleFactor,
      nativeHeight * scaleFactor,
    ]);
    const scaledX = Math.floor(screen.get_width() / 2 - scaledSurface.get_width() / 2);
    const scaledY = Math.floor(screen.get_height() / 2 - scaledSurface.get_height() / 2);
    screen.blit(scaledSurface, [scaledX, scaledY]);
    this.renderTextSnapshot();
  }

  isFinished(): boolean {
    return this.finished;
  }

  private initializeState(): void {
    this.tilemap = Array.from({ length: NameEntryScreen.SCREEN_TILE_HEIGHT }, () =>
      Array(NameEntryScreen.SCREEN_TILE_WIDTH).fill(NameEntryScreen.NAMINGSCREEN_BORDER)
    );
    this.clearBox(1, 1, 18, 6);
    this.clearBox(1, 8, 18, 7);
    this.clearBox(1, 16, 18, 1);

    this.cursorColumn = 0;
    this.cursorRow = 0;

    this.nameTiles = [
      NameEntryScreen.NAMINGSCREEN_UNDERLINE,
      ...Array(this.maxNameLength - 1).fill(NameEntryScreen.NAMINGSCREEN_MIDDLELINE),
    ];
    this.nameChars = [];
    this.name = "";

    this.placePrompt();
    this.applyKeyboardLayout();
    this.updateNameTilemap();
  }

  private tokenizeString(text: string): string[] {
    const tokens: string[] = [];
    let index = 0;
    while (index < text.length) {
      let matched = false;
      for (const token of NameEntryScreen.MULTI_TILE_TOKENS) {
        if (text.startsWith(token, index)) {
          tokens.push(token);
          index += token.length;
          matched = true;
          break;
        }
      }
      if (!matched) {
        tokens.push(text[index]);
        index += 1;
      }
    }
    return tokens;
  }

  private stringToTiles(text: string): number[] {
    const tiles: number[] = [];
    for (const token of this.tokenizeString(text)) {
      if (token === "@") {
        break;
      }
      const tile = NameEntryScreen.CHAR_TO_TILE[token];
      if (tile === undefined) {
        throw new Error(`Unsupported naming screen glyph: ${token}`);
      }
      tiles.push(tile);
    }
    if (tiles.length !== NameEntryScreen.KEYBOARD_COLUMNS) {
      throw new Error("Naming screen keyboard rows must be exactly 17 tiles wide");
    }
    return tiles;
  }

  private clearBox(tileX: number, tileY: number, width: number, height: number): void {
    for (let row = tileY; row < tileY + height; row += 1) {
      if (row < 0 || row >= NameEntryScreen.SCREEN_TILE_HEIGHT) {
        continue;
      }
      for (let col = tileX; col < tileX + width; col += 1) {
        if (col >= 0 && col < NameEntryScreen.SCREEN_TILE_WIDTH) {
          this.tilemap[row][col] = 0;
        }
      }
    }
  }

  private placePrompt(): void {
    this.writeString(this.prompt.toUpperCase(), NameEntryScreen.NAME_ENTRY_X, 2);
  }

  private writeString(text: string, tileX: number, tileY: number): void {
    let x = tileX;
    for (const token of this.tokenizeString(text)) {
      if (token === "@") {
        break;
      }
      if (tileY < 0 || tileY >= NameEntryScreen.SCREEN_TILE_HEIGHT || x < 0 || x >= NameEntryScreen.SCREEN_TILE_WIDTH) {
        x += 1;
        continue;
      }
      const tileIndex = NameEntryScreen.CHAR_TO_TILE[token];
      if (tileIndex !== undefined) {
        this.tilemap[tileY][x] = tileIndex;
      }
      x += 1;
    }
  }

  private applyKeyboardLayout(): void {
    const layout = this.case === "upper" ? NameEntryScreen.UPPER_LAYOUT : NameEntryScreen.LOWER_LAYOUT;
    layout.forEach((rowText, rowIndex) => {
      const tileRow = this.stringToTiles(rowText);
      const tileY = NameEntryScreen.KEYBOARD_START_Y + rowIndex * NameEntryScreen.KEYBOARD_ROW_SPACING;
      tileRow.forEach((tileIndex, column) => {
        const tileX = NameEntryScreen.KEYBOARD_START_X + column;
        if (tileX >= 0 && tileX < NameEntryScreen.SCREEN_TILE_WIDTH) {
          this.tilemap[tileY][tileX] = tileIndex;
        }
      });
    });
  }

  private updateNameTilemap(): void {
    this.nameTiles.forEach((tileIndex, index) => {
      const tileX = NameEntryScreen.NAME_ENTRY_X + index;
      if (tileX >= 0 && tileX < NameEntryScreen.SCREEN_TILE_WIDTH) {
        this.tilemap[NameEntryScreen.NAME_ENTRY_Y][tileX] = tileIndex;
      }
    });
  }

  private composeCursorSurface(
    entries: Array<[number, number, number, number, number, boolean, boolean]>
  ): [Surface, [number, number]] {
    const headTile = this.tileset.specialTiles.cursor_0;
    const bodyTile = this.tileset.specialTiles.cursor_1;
    if (!headTile || !bodyTile) {
      throw new Error("Naming screen cursor tiles are missing");
    }
    const tileLookup: Record<number, Surface> = {
      0: headTile,
      1: bodyTile,
    };

    const positions: Array<[number, number, Surface]> = [];
    let minX = Number.POSITIVE_INFINITY;
    let minY = Number.POSITIVE_INFINITY;
    let maxX = Number.NEGATIVE_INFINITY;
    let maxY = Number.NEGATIVE_INFINITY;

    for (const entry of entries) {
      const [xTile, yTile, xPx, yPx, tileIdx, xflip, yflip] = entry;
      const baseX = xTile * TILE_SIZE + xPx;
      const baseY = yTile * TILE_SIZE + yPx;
      let tileSurface = tileLookup[tileIdx];
      if (!tileSurface) {
        throw new Error(`Missing cursor tile index: ${tileIdx}`);
      }
      if (xflip || yflip) {
        tileSurface = gameEngine.transform.flip(tileSurface, xflip, yflip);
      }
      positions.push([baseX, baseY, tileSurface]);
      minX = Math.min(minX, baseX);
      minY = Math.min(minY, baseY);
      maxX = Math.max(maxX, baseX + TILE_SIZE);
      maxY = Math.max(maxY, baseY + TILE_SIZE);
    }

    const width = Math.max(1, Math.floor(maxX - minX));
    const height = Math.max(1, Math.floor(maxY - minY));
    const surface = new gameEngine.Surface(width, height);
    const offsetX = Math.floor(minX);
    const offsetY = Math.floor(minY);
    positions.forEach(([baseX, baseY, tileSurface]) => {
      surface.blit(tileSurface, [baseX - offsetX, baseY - offsetY]);
    });
    surface.set_colorkey([255, 255, 255]);
    return [surface, [offsetX, offsetY]];
  }

  private handleTextualCharacter(event: NameEntryEvent): boolean {
    let text = event.text ?? event.unicode ?? "";
    if (typeof text === "string") {
      text = text.slice(0, 1);
    }
    if (text) {
      const char = text[0];
      if (/[a-z]/i.test(char)) {
        const inputChar = NameEntryScreen.CHAR_TO_TILE[char] !== undefined ? char : char.toUpperCase();
        const tileId = NameEntryScreen.CHAR_TO_TILE[inputChar];
        if (tileId !== undefined && this.nameChars.length < this.maxNameLength) {
          this.appendCharacterFromInput(tileId, inputChar);
          this.playConfirmSound();
          return true;
        }
      }
      if (char === " ") {
        const tileId = NameEntryScreen.CHAR_TO_TILE[" "];
        if (tileId !== undefined && this.nameChars.length < this.maxNameLength) {
          this.appendCharacterFromInput(tileId, " ");
          this.playConfirmSound();
          return true;
        }
      }
    }
    const key = event.key ?? event.code ?? null;
    if (typeof key === "number") {
      if (key >= 97 && key <= 122) {
        const char = String.fromCharCode(key).toUpperCase();
        const tileId = NameEntryScreen.CHAR_TO_TILE[char];
        if (tileId !== undefined && this.nameChars.length < this.maxNameLength) {
          this.appendCharacterFromInput(tileId, char);
          this.playConfirmSound();
          return true;
        }
      }
    }
    if (key === gameEngine.K_BACKSPACE) {
      if (this.deleteCharacter()) {
        this.playCancelSound();
        return true;
      }
    }
    return false;
  }

  private moveVertical(direction: number): void {
    if (direction === -1) {
      this.cursorRow = this.cursorRow === 0 ? NameEntryScreen.BOTTOM_ROW_INDEX : this.cursorRow - 1;
    } else if (direction === 1) {
      this.cursorRow = this.cursorRow === NameEntryScreen.BOTTOM_ROW_INDEX ? 0 : this.cursorRow + 1;
    }
    this.playCursorSound();
  }

  private moveHorizontal(direction: number): void {
    if (this.cursorRow === NameEntryScreen.BOTTOM_ROW_INDEX) {
      this.moveHorizontalBottom(direction);
      return;
    }
    this.cursorColumn =
      (this.cursorColumn + direction + NameEntryScreen.LETTER_X_OFFSETS.length) %
      NameEntryScreen.LETTER_X_OFFSETS.length;
    this.playCursorSound();
  }

  private moveHorizontalBottom(direction: number): void {
    const group = this.getBottomGroup();
    if (direction === 1) {
      this.cursorColumn = group === 3 ? 0 : group === 1 ? 3 : 6;
    } else {
      this.cursorColumn = group === 1 ? 6 : group === 2 ? 0 : 3;
    }
    this.playCursorSound();
  }

  private getBottomGroup(): number {
    if (this.cursorColumn < 3) return 1;
    if (this.cursorColumn < 6) return 2;
    return 3;
  }

  private moveByDirection(direction: string): void {
    if (direction === "up") {
      this.moveVertical(-1);
    } else if (direction === "down") {
      this.moveVertical(1);
    } else if (direction === "left") {
      this.moveHorizontal(-1);
    } else if (direction === "right") {
      this.moveHorizontal(1);
    }
  }

  private pressA(): void {
    let success = false;
    const command = this.getCursorCommand();
    if (command === "char") {
      if (this.nameChars.length >= this.maxNameLength) {
        return;
      }
      const tileId = this.getSelectedTile();
      if (tileId === null) return;
      const char = NameEntryScreen.TILE_TO_CHAR[tileId];
      if (!char) return;
      success = this.appendCharacterFromInput(tileId, char);
    } else if (command === "case") {
      this.toggleCase();
      success = true;
    } else if (command === "delete") {
      success = this.deleteCharacter();
    } else if (command === "end") {
      success = this.confirmName();
    }
    if (success) {
      this.playConfirmSound();
    }
  }

  private pressStart(): void {
    this.moveCursorToEnd();
  }

  private confirmName(): boolean {
    this.finished = true;
    return true;
  }

  private moveCursorToEnd(): void {
    this.cursorColumn = 8;
    this.cursorRow = NameEntryScreen.BOTTOM_ROW_INDEX;
  }

  private pressB(): void {
    if (this.deleteCharacter()) {
      this.playCancelSound();
    }
  }

  private toggleCase(): void {
    this.case = this.case === "upper" ? "lower" : "upper";
    this.applyKeyboardLayout();
  }

  private appendCharacter(tileId: number, char: string): boolean {
    const position = this.nameChars.length;
    this.nameTiles[position] = tileId;
    this.nameChars.push(char);
    if (position + 1 < this.maxNameLength) {
      this.nameTiles[position + 1] = NameEntryScreen.NAMINGSCREEN_UNDERLINE;
      for (let idx = position + 2; idx < this.maxNameLength; idx += 1) {
        this.nameTiles[idx] = NameEntryScreen.NAMINGSCREEN_MIDDLELINE;
      }
    }
    this.name = this.nameChars.join("");
    this.updateNameTilemap();
    return true;
  }

  private appendCharacterFromInput(tileId: number, char: string): boolean {
    const success = this.appendCharacter(tileId, char);
    if (success && this.nameChars.length >= this.maxNameLength) {
      this.moveCursorToEnd();
    }
    return success;
  }

  private deleteCharacter(): boolean {
    if (!this.nameChars.length) {
      return false;
    }
    this.nameChars.pop();
    const newLength = this.nameChars.length;
    this.nameTiles[newLength] = NameEntryScreen.NAMINGSCREEN_UNDERLINE;
    if (newLength + 1 < this.maxNameLength) {
      if (this.nameTiles[newLength + 1] === NameEntryScreen.NAMINGSCREEN_UNDERLINE) {
        this.nameTiles[newLength + 1] = NameEntryScreen.NAMINGSCREEN_MIDDLELINE;
      }
    }
    this.name = this.nameChars.join("");
    this.finished = false;
    this.updateNameTilemap();
    return true;
  }

  private playCursorSound(): void {
    return;
  }

  private playConfirmSound(): void {
    return;
  }

  private playCancelSound(): void {
    return;
  }

  private getCursorCommand(): string {
    if (this.cursorRow === NameEntryScreen.BOTTOM_ROW_INDEX) {
      const group = this.getBottomGroup();
      if (group === 1) return "case";
      if (group === 2) return "delete";
      return "end";
    }
    return "char";
  }

  private getSelectedTile(): number | null {
    const [tileX, tileY] = this.computeCursorTileCoords();
    if (
      tileX >= 0 &&
      tileX < NameEntryScreen.SCREEN_TILE_WIDTH &&
      tileY >= 0 &&
      tileY < NameEntryScreen.SCREEN_TILE_HEIGHT
    ) {
      return this.tilemap[tileY][tileX];
    }
    return null;
  }

  private describeSelectedCell(): string {
    if (this.cursorRow === NameEntryScreen.BOTTOM_ROW_INDEX) {
      const group = this.getBottomGroup();
      if (group === 1) {
        return this.case === "upper" ? "lower" : "UPPER";
      }
      if (group === 2) {
        return "DEL";
      }
      return "END";
    }
    const tileId = this.getSelectedTile();
    if (tileId === null) {
      return "(empty)";
    }
    return NameEntryScreen.TILE_TO_CHAR[tileId] ?? "(empty)";
  }

  private computeCursorTileCoords(): [number, number] {
    const xOffsets =
      this.cursorRow === NameEntryScreen.BOTTOM_ROW_INDEX
        ? NameEntryScreen.CASE_ROW_X_OFFSETS
        : NameEntryScreen.LETTER_X_OFFSETS;
    const xOffset = xOffsets[this.cursorColumn];
    const anchorX = NameEntryScreen.CURSOR_BASE_X + xOffset;
    const anchorY = NameEntryScreen.CURSOR_BASE_Y + this.cursorRow * NameEntryScreen.ROW_PIXEL_STEP;
    const tileX = Math.floor((anchorX - 8) / TILE_SIZE);
    const tileY = Math.floor((anchorY - 16) / TILE_SIZE);
    return [tileX, tileY];
  }

  private computeCursorPixelPosition(): [number, number] {
    const [tileX, tileY] = this.computeCursorTileCoords();
    return [tileX * TILE_SIZE, tileY * TILE_SIZE];
  }

  private renderTilemap(surface: Surface): void {
    for (let row = 0; row < NameEntryScreen.SCREEN_TILE_HEIGHT; row += 1) {
      for (let col = 0; col < NameEntryScreen.SCREEN_TILE_WIDTH; col += 1) {
        const tileIndex = this.tilemap[row][col];
        const tileSurface = this.tileset.getTileSurface(tileIndex);
        surface.blit(tileSurface, [col * TILE_SIZE, row * TILE_SIZE]);
      }
    }
  }

  private resolveTextUiTarget(ui: ScreenUI | CompositeUI | null): ScreenUI | null {
    if (isTextUI(ui)) {
      return ui;
    }
    if (ui && "children" in ui) {
      for (const child of ui.children) {
        if (isTextUI(child)) {
          return child;
        }
      }
    }
    return null;
  }

  private renderTextSnapshot(): void {
    if (!this.textUi || !this.textUi.renderSnapshot) {
      return;
    }
    const layout = this.case === "upper" ? NameEntryScreen.UPPER_LAYOUT : NameEntryScreen.LOWER_LAYOUT;
    const menuLines: string[] = [];
    layout.forEach((rowText, rowIndex) => {
      const row = rowText.trimEnd();
      menuLines.push(row);
      if (rowIndex === this.cursorRow) {
        const pointerIndex = Math.min(NameEntryScreen.KEYBOARD_COLUMNS - 1, this.cursorColumn * 2);
        const marker = " ".repeat(Math.max(0, pointerIndex)) + "▲";
        menuLines.push(marker);
      }
    });

    const infoLines = [
      "STATE: name_entry",
      "Use move up/down/left/right to move the cursor; press a to select.",
      "Use press b to delete, press start to choose END, or type_text for letters.",
      `PROMPT: ${this.prompt.toUpperCase()}`,
      `CASE: ${this.case}`,
      `NAME: ${this.name || "(blank)"}`,
      `LENGTH: ${this.nameChars.length}/${this.maxNameLength}`,
      `CURSOR: row ${this.cursorRow} col ${this.cursorColumn}`,
      `SELECTED: ${this.describeSelectedCell()}`,
      `LAST INPUT: ${this.lastInput ? JSON.stringify(this.lastInput) : "none"}`,
    ];
    if (this.finished) {
      infoLines.push("STATUS: confirmed");
    } else if (this.nameChars.length >= this.maxNameLength) {
      infoLines.push("STATUS: full - delete before adding");
    }

    const viewportLines = [
      "NAME ENTRY",
      `LAST INPUT: ${this.lastInput ? JSON.stringify(this.lastInput) : "none"}`,
      this.prompt.toUpperCase(),
    ];
    this.textUi.renderSnapshot(viewportLines, infoLines, "Name Entry", "Name Entry", menuLines, null, null);
  }

  static readonly SMALL_CURSOR_OAM: Array<[number, number, number, number, number, boolean, boolean]> = [
    [-1, -1, 7, 7, 0, false, false],
    [0, -1, 0, 7, 0, true, false],
    [-1, 0, 7, 0, 0, false, true],
    [0, 0, 0, 0, 0, true, true],
  ];
  static readonly BIG_CURSOR_OAM: Array<[number, number, number, number, number, boolean, boolean]> = [
    [0, -1, 0, 7, 0, false, false],
    [1, -1, 0, 7, 1, false, false],
    [2, -1, 0, 7, 1, false, false],
    [3, -1, 0, 7, 1, false, false],
    [4, -1, 0, 7, 0, true, false],
    [0, 0, 0, 0, 0, false, true],
    [1, 0, 0, 0, 1, false, true],
    [2, 0, 0, 0, 1, false, true],
    [3, 0, 0, 0, 1, false, true],
    [4, 0, 0, 0, 0, true, true],
  ];

  static readonly KEYBOARD_COLUMNS = 17;
  static readonly KEYBOARD_START_X = 2;
  static readonly KEYBOARD_START_Y = 8;
  static readonly KEYBOARD_ROW_SPACING = 2;

  static readonly NAME_ENTRY_X = 5;
  static readonly NAME_ENTRY_Y = 6;

  static readonly CURSOR_BASE_X = 24;
  static readonly CURSOR_BASE_Y = 80;
  static readonly ROW_PIXEL_STEP = 0x10;

  static readonly LETTER_X_OFFSETS = [0x00, 0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80];
  static readonly CASE_ROW_X_OFFSETS = [0x00, 0x00, 0x00, 0x30, 0x30, 0x30, 0x60, 0x60, 0x60];

  static readonly NAMINGSCREEN_BORDER = 0x60;
  static readonly NAMINGSCREEN_UNDERLINE = 0xf2;
  static readonly NAMINGSCREEN_MIDDLELINE = 0xeb;
  static readonly SPACE_TILE = 0x7f;

  static readonly BOTTOM_ROW_INDEX = 4;

  static readonly UPPER_LAYOUT = [
    "A B C D E F G H I",
    "J K L M N O P Q R",
    "S T U V W X Y Z  ",
    "- ? ! / . ,      ",
    "lower  DEL   END ",
  ];
  static readonly LOWER_LAYOUT = [
    "a b c d e f g h i",
    "j k l m n o p q r",
    "s t u v w x y z  ",
    "× ( ) : ; [ ] <PK> <MN>",
    "UPPER  DEL   END ",
  ];

  static readonly MULTI_TILE_TOKENS = [
    "<PK>",
    "<MN>",
    "'d",
    "'l",
    "'m",
    "'r",
    "'s",
    "'t",
    "'v",
  ];

  static readonly CHAR_TO_TILE: Record<string, number> = {
    " ": 0x7f,
    A: 0x80,
    B: 0x81,
    C: 0x82,
    D: 0x83,
    E: 0x84,
    F: 0x85,
    G: 0x86,
    H: 0x87,
    I: 0x88,
    J: 0x89,
    K: 0x8a,
    L: 0x8b,
    M: 0x8c,
    N: 0x8d,
    O: 0x8e,
    P: 0x8f,
    Q: 0x90,
    R: 0x91,
    S: 0x92,
    T: 0x93,
    U: 0x94,
    V: 0x95,
    W: 0x96,
    X: 0x97,
    Y: 0x98,
    Z: 0x99,
    a: 0xa0,
    b: 0xa1,
    c: 0xa2,
    d: 0xa3,
    e: 0xa4,
    f: 0xa5,
    g: 0xa6,
    h: 0xa7,
    i: 0xa8,
    j: 0xa9,
    k: 0xaa,
    l: 0xab,
    m: 0xac,
    n: 0xad,
    o: 0xae,
    p: 0xaf,
    q: 0xb0,
    r: 0xb1,
    s: 0xb2,
    t: 0xb3,
    u: 0xb4,
    v: 0xb5,
    w: 0xb6,
    x: 0xb7,
    y: 0xb8,
    z: 0xb9,
    "0": 0xf6,
    "1": 0xf7,
    "2": 0xf8,
    "3": 0xf9,
    "4": 0xfa,
    "5": 0xfb,
    "6": 0xfc,
    "7": 0xfd,
    "8": 0xfe,
    "9": 0xff,
    "(": 0x9a,
    ")": 0x9b,
    ":": 0x9c,
    ";": 0x9d,
    "[": 0x9e,
    "]": 0x9f,
    Ä: 0xc0,
    Ö: 0xc1,
    Ü: 0xc2,
    ä: 0xc3,
    ö: 0xc4,
    ü: 0xc5,
    "'d": 0xd0,
    "'l": 0xd1,
    "'m": 0xd2,
    "'r": 0xd3,
    "'s": 0xd4,
    "'t": 0xd5,
    "'v": 0xd6,
    "←": 0xdf,
    "'": 0xe0,
    "<PK>": 0xe1,
    "<MN>": 0xe2,
    "-": 0xe3,
    "?": 0xe6,
    "!": 0xe7,
    ".": 0xe8,
    "&": 0xe9,
    é: 0xea,
    "→": 0xeb,
    "▷": 0xec,
    "▶": 0xed,
    "▼": 0xee,
    "♂": 0xef,
    "¥": 0xf0,
    "×": 0xf1,
    "<DOT>": 0xf2,
    "/": 0xf3,
    ",": 0xf4,
    "♀": 0xf5,
  };

  static readonly TILE_TO_CHAR: Record<number, string> = Object.fromEntries(
    Object.entries(NameEntryScreen.CHAR_TO_TILE).map(([key, value]) => [value, key])
  );
}
