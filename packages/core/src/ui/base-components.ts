import fs from "fs";
import path from "path";
import { gameEngine } from "./game-engine";
import { getBasePath } from "@pokecrystal/core/core/paths";
import { GameState } from "@pokecrystal/core/core/state";
import { gbc5To8 } from "@pokecrystal/core/core/gbc-colors";

export type RGB = [number, number, number];

export const parsePaletteFile = (filePath: string): RGB[] => {
  if (!fs.existsSync(filePath)) {
    throw new Error(`Missing palette file: ${filePath}`);
  }
  const entries: RGB[] = [];
  const lines = fs.readFileSync(filePath, "utf-8").split(/\r?\n/);
  for (const rawLine of lines) {
    const stripped = rawLine.split(";")[0].trim();
    if (!stripped || !stripped.toUpperCase().startsWith("RGB")) {
      continue;
    }
    const components = stripped
      .replace(/RGB/gi, "")
      .replace(/,/g, " ")
      .split(/\s+/)
      .filter(Boolean);
    if (components.length !== 3) {
      throw new Error(`Malformed RGB entry '${stripped}' in palette ${path.basename(filePath)}`);
    }
    const values = components.map((component) => Number(component));
    if (values.some((value) => Number.isNaN(value))) {
      throw new Error(`Malformed RGB entry '${stripped}' in palette ${path.basename(filePath)}`);
    }
    const rgb = values.map((value, idx) => gbc5To8(value, `${path.basename(filePath)} component ${idx}`));
    entries.push([rgb[0], rgb[1], rgb[2]]);
  }
  if (entries.length !== 4) {
    throw new Error(
      `Palette file ${path.basename(filePath)} must contain exactly 4 colours, got ${entries.length}`
    );
  }
  return entries;
};

export class HardwareRegisters {
  public scx = 0;
  public scy = 0;
  public wx = 0;
  public wy = 0;
  public lcdc_pointer: number | null = null;

  setScroll(scx: number, scy: number): void {
    this.scx = scx & 0xff;
    this.scy = scy & 0xff;
  }

  setWindow(wx: number, wy: number): void {
    this.wx = wx & 0xff;
    this.wy = wy & 0xff;
  }

  setLcdcPointer(pointer: number | null): void {
    this.lcdc_pointer = pointer === null ? null : pointer & 0xffff;
  }
}

export class PaletteManager {
  private readonly contexts = new Map<string, RGB[]>();

  constructor(private readonly basePath: string = getBasePath()) {}

  setPalette(context: string, palette: readonly RGB[]): RGB[] {
    if (palette.length !== 4) {
      throw new Error("UI palettes must contain exactly 4 colours.");
    }
    const normalized = palette.map((entry) => [...entry]) as RGB[];
    this.contexts.set(context, normalized);
    return normalized;
  }

  getPalette(context: string): RGB[] {
    const palette = this.contexts.get(context);
    if (!palette) {
      throw new Error(`No palette registered for context '${context}'`);
    }
    return palette;
  }

  loadPaletteFromFile(context: string, relativePath: string): void {
    const resolved = path.isAbsolute(relativePath)
      ? relativePath
      : path.join(this.basePath, relativePath);
    const palette = parsePaletteFile(resolved);
    this.setPalette(context, palette);
  }
}

export type WindowRecord = {
  target: InstanceType<typeof gameEngine.Surface>;
  rect: InstanceType<typeof gameEngine.Rect>;
  snapshot: InstanceType<typeof gameEngine.Surface>;
  zIndex: number;
};

const surfaceBounds = (surface: InstanceType<typeof gameEngine.Surface>): InstanceType<typeof gameEngine.Rect> => {
  const [width, height] = surface.get_size();
  return new gameEngine.Rect(0, 0, width, height);
};

const clipRect = (
  rect: InstanceType<typeof gameEngine.Rect>,
  bounds: InstanceType<typeof gameEngine.Rect>
): InstanceType<typeof gameEngine.Rect> | null => {
  const left = Math.max(rect.x, bounds.x);
  const top = Math.max(rect.y, bounds.y);
  const right = Math.min(rect.x + rect.width, bounds.x + bounds.width);
  const bottom = Math.min(rect.y + rect.height, bounds.y + bounds.height);
  const width = right - left;
  const height = bottom - top;
  if (width <= 0 || height <= 0) {
    return null;
  }
  return new gameEngine.Rect(left, top, width, height);
};

export class WindowStack {
  private records: WindowRecord[] = [];

  register(
    surface: InstanceType<typeof gameEngine.Surface>,
    rect: InstanceType<typeof gameEngine.Rect>,
    zIndex: number,
    snapshot?: InstanceType<typeof gameEngine.Surface> | null
  ): void {
    if (rect.width <= 0 || rect.height <= 0) {
      return;
    }
    const clipped = clipRect(rect, surfaceBounds(surface));
    if (!clipped) {
      return;
    }
    let resolvedSnapshot = snapshot ?? null;
    if (!resolvedSnapshot) {
      resolvedSnapshot = new gameEngine.Surface(clipped.width, clipped.height);
      resolvedSnapshot.blit(surface, [0, 0], clipped);
    } else if (resolvedSnapshot.get_size()[0] !== clipped.width || resolvedSnapshot.get_size()[1] !== clipped.height) {
      throw new Error("Snapshot size must match the registered region.");
    }
    this.records = this.records.filter(
      (record) =>
        !(
          record.target === surface &&
          record.rect.x === clipped.x &&
          record.rect.y === clipped.y &&
          record.rect.width === clipped.width &&
          record.rect.height === clipped.height &&
          record.zIndex === zIndex
        )
    );
    this.records.push({
      target: surface,
      rect: new gameEngine.Rect(clipped.x, clipped.y, clipped.width, clipped.height),
      snapshot: resolvedSnapshot,
      zIndex,
    });
  }

  render(): void {
    const ordered = [...this.records].sort((a, b) => a.zIndex - b.zIndex);
    for (const record of ordered) {
      record.target.blit(record.snapshot, [record.rect.x, record.rect.y]);
    }
  }

  reset(): void {
    this.records = [];
  }
}

export const detectSuperGameBoyMode = (): boolean => {
  const raw =
    process.env.SUPER_GAME_BOY ??
    process.env.NEXT_PUBLIC_SUPER_GAME_BOY ??
    process.env.SGB_MODE ??
    "";
  if (!raw) {
    return false;
  }
  return ["1", "true", "yes", "on"].includes(raw.toLowerCase());
};

export const syncRegistersToHram = (registers: HardwareRegisters, gameState: GameState): void => {
  const hram = gameState.hram as GameState["hram"] & {
    hSCX?: number;
    hSCY?: number;
    hWX?: number;
    hWY?: number;
  };
  hram.hSCX = registers.scx;
  hram.hSCY = registers.scy;
  hram.hWX = registers.wx;
  hram.hWY = registers.wy;
};
