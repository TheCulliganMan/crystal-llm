// ASM mapping: pokecrystal_disassembly/engine/menus/party_menu.asm (InitPartyMenuGFX icons).
import fs from "fs";
import path from "path";
import { Pokemon as PokemonId } from "@pokecrystal/core/core/constants";
import { Pokemon } from "@pokecrystal/core/core/models";
import { getAssetPath, getDataDir } from "@pokecrystal/core/core/paths";
import { TILE_SIZE } from "@pokecrystal/core/engine/world/tile";
import { Rect, Surface } from "@pokecrystal/core/ui/surface";
import { gameEngine } from "../game-engine";
import { PartyEntry } from "./party-menu-layout";
import { gbc5To8 } from "@pokecrystal/core/core/gbc-colors";

type Palette = ReadonlyArray<ReadonlyArray<number>>;

const ICON_WIDTH = 16;
const ICON_HEIGHT = 16;
const ICON_SPRITE_HEIGHT = 32;
const GB_SPRITE_X_OFFSET = 8;
const GB_SPRITE_Y_OFFSET = 16;

const gbToScreenX = (raw: number): number => raw - GB_SPRITE_X_OFFSET;
const gbToScreenY = (raw: number): number => raw - GB_SPRITE_Y_OFFSET;

const ICON_BASE_X = gbToScreenX(0x10);
const ICON_SELECTED_X = ICON_BASE_X;
// ASM: InitPartyMenuIcon sets y=$1c+slot*16, then OAMData_RedWalk uses dbsprite -1,-1.
const ICON_BASE_Y = gbToScreenY(0x1c - TILE_SIZE);
const ICON_ROW_STRIDE = 2 * TILE_SIZE;
const ICON_OVERLAY_OFFSET = 8;

const SPRITE_PALETTE_PATH = getAssetPath("gfx", "stats", "party_menu_ob.pal");
const ICON_TABLE_PATH = path.join(getDataDir(), "menu_icons.json");
const ICON_DIR = getAssetPath("gfx", "icons");
const OVERLAY_DIR = getAssetPath("gfx", "stats");
const MAIL_SUFFIX = "_MAIL";

const GREEN_THRESHOLD = 0.5;
const YELLOW_THRESHOLD = 0.21;
const FRAME_DURATIONS: Record<string, number> = { green: 8, yellow: 72, red: 136 };
const BOB_AMPLITUDES: Record<string, number> = { green: -2, yellow: -1, red: 0 };
const BOB_TOGGLE_INTERVAL = 16;

const normalizeIconSpeciesKey = (value: string): string =>
  String(value ?? "").toUpperCase().replace(/_+/g, "_");

const scaleGb = (value: number): number => gbc5To8(value);

const loadPalette = (): Palette => {
  const lines = fs.readFileSync(SPRITE_PALETTE_PATH, "utf-8").split(/\r?\n/);
  const palette: [number, number, number][] = [];
  for (const raw of lines) {
    const line = raw.trim();
    if (!line || !line.toUpperCase().startsWith("RGB")) {
      continue;
    }
    const parts = line.split(/\s+/);
    if (parts.length < 4) {
      continue;
    }
    const r = Number(parts[1].replace(",", ""));
    const g = Number(parts[2].replace(",", ""));
    const b = Number(parts[3]);
    palette.push([scaleGb(r), scaleGb(g), scaleGb(b)]);
    if (palette.length === 4) {
      break;
    }
  }
  if (palette.length !== 4) {
    throw new Error("Party menu OBJ palette must define four colours");
  }
  return palette;
};

const paletteRgba = (palette: Palette, paletteIndex: number): [number, number, number, number] => {
  if (paletteIndex >= palette.length) {
    throw new Error(`Palette index ${paletteIndex} missing from party menu OBJ palette`);
  }
  const [r, g, b] = palette[paletteIndex];
  const alpha = paletteIndex === 0 ? 0 : 255;
  return [r, g, b, alpha];
};

const grayscalePaletteIndex = (surface: Surface, x: number, y: number): number => {
  const [r, g, b, a] = surface.get_at([x, y]);
  if (a === 0) {
    return 0;
  }
  const value = Math.round((r + g + b) / 3);
  const pngSample = Math.max(0, Math.min(3, Math.round(value / 85)));
  return 3 - pngSample;
};

const applyPaletteToGrayscalePng = (source: Surface, palette: Palette): Surface => {
  const tinted = new Surface(source.get_width(), source.get_height());
  for (let y = 0; y < source.get_height(); y += 1) {
    for (let x = 0; x < source.get_width(); x += 1) {
      tinted.setAt(x, y, paletteRgba(palette, grayscalePaletteIndex(source, x, y)));
    }
  }
  return tinted;
};

const decode2bppTiles = (data: Buffer): number[][] => {
  if (data.length % 16 !== 0) {
    throw new Error("2bpp payload must be aligned to 16-byte tiles");
  }
  const tiles: number[][] = [];
  const tileCount = data.length / 16;
  for (let tileIndex = 0; tileIndex < tileCount; tileIndex += 1) {
    const base = tileIndex * 16;
    const pixels: number[] = [];
    for (let row = 0; row < TILE_SIZE; row += 1) {
      const plane0 = data[base + row * 2];
      const plane1 = data[base + row * 2 + 1];
      for (let col = 0; col < TILE_SIZE; col += 1) {
        const bit = 7 - col;
        const idx = ((plane0 >> bit) & 1) | (((plane1 >> bit) & 1) << 1);
        pixels.push(idx);
      }
    }
    tiles.push(pixels);
  }
  return tiles;
};

const renderTile = (pixels: number[], palette: Palette): Surface => {
  const surface = new Surface(TILE_SIZE, TILE_SIZE);
  for (let row = 0; row < TILE_SIZE; row += 1) {
    for (let col = 0; col < TILE_SIZE; col += 1) {
      const idx = pixels[row * TILE_SIZE + col];
      surface.setAt(col, row, paletteRgba(palette, idx));
    }
  }
  return surface;
};

const iconAssetStem = (iconName: string): string =>
  String(iconName ?? "")
    .toLowerCase()
    .replace(/^icon_/, "");

const sliceIconFrames = (iconName: string, palette: Palette): Surface[] => {
  const iconPath = path.join(ICON_DIR, `${iconName}.2bpp`);
  if (fs.existsSync(iconPath)) {
    const data = fs.readFileSync(iconPath);
    const tiles = decode2bppTiles(data);
    if (tiles.length < 8) {
      throw new Error(`${iconPath} must contain at least 8 tiles for two frames`);
    }

    const frames: Surface[] = [];
    for (let frameIndex = 0; frameIndex < 2; frameIndex += 1) {
      const frame = new Surface(ICON_WIDTH, ICON_HEIGHT);
      for (let row = 0; row < 2; row += 1) {
        for (let col = 0; col < 2; col += 1) {
          const tileIndex = frameIndex * 4 + row * 2 + col;
          const tile = renderTile(tiles[tileIndex], palette);
          frame.blit(tile, [col * TILE_SIZE, row * TILE_SIZE]);
        }
      }
      frames.push(frame);
    }
    return frames;
  }

  const pngPath = path.join(ICON_DIR, `${iconAssetStem(iconName)}.png`);
  const source = gameEngine.image.loadSync?.(pngPath);
  if (!source) {
    throw new Error(`Missing icon tiles for ${iconName}`);
  }
  if (source.get_width() < ICON_WIDTH || source.get_height() < ICON_SPRITE_HEIGHT) {
    throw new Error(`${pngPath} must contain at least two ${ICON_WIDTH}x${ICON_HEIGHT} icon frames`);
  }
  const tinted = applyPaletteToGrayscalePng(source, palette);
  return [
    tinted.subsurface(new Rect(0, 0, ICON_WIDTH, ICON_HEIGHT)),
    tinted.subsurface(new Rect(0, ICON_HEIGHT, ICON_WIDTH, ICON_HEIGHT)),
  ];
};

const loadOverlay = (name: string, palette: Palette): Surface => {
  const overlayPath = path.join(OVERLAY_DIR, name.replace(/\.png$/, ".2bpp"));
  if (fs.existsSync(overlayPath)) {
    const tiles = decode2bppTiles(fs.readFileSync(overlayPath));
    if (!tiles.length) {
      throw new Error(`${overlayPath} did not contain any overlay tiles`);
    }
    return renderTile(tiles[0], palette);
  }

  const pngPath = path.join(OVERLAY_DIR, name);
  const surface = gameEngine.image.loadSync?.(pngPath);
  if (surface) {
    return applyPaletteToGrayscalePng(surface, palette);
  }
  throw new Error(`Missing overlay tiles for ${name}`);
};

const loadSpeciesIconMap = (): Record<string, string> => {
  const mapping = JSON.parse(fs.readFileSync(ICON_TABLE_PATH, "utf-8")) as Record<string, string>;
  const speciesOrder = Object.entries(PokemonId)
    .filter(([_name, value]) => typeof value === "number")
    .map(([name, value]) => ({ name, value: value as number }))
    .filter((entry) => entry.name !== "EGG")
    .sort((a, b) => a.value - b.value);

  if (Object.keys(mapping).length !== speciesOrder.length + 1) {
    throw new Error(
      `Menu icon table length ${Object.keys(mapping).length} did not match species list ${speciesOrder.length + 1}`
    );
  }
  for (const species of speciesOrder) {
    if (!mapping[normalizeIconSpeciesKey(species.name)]) {
      throw new Error(`Menu icon mapping missing species ${species.name}`);
    }
  }
  if (!mapping.EGG) {
    throw new Error("Menu icon mapping missing species EGG");
  }
  return mapping;
};

const iconSpeciesKey = (pokemon: Pokemon): string => {
  const nickname = String(pokemon.nickname ?? "");
  if (nickname.toUpperCase() === "EGG") {
    return "EGG";
  }
  return normalizeIconSpeciesKey(String(pokemon.species?.id ?? ""));
};

class IconState {
  public bobCounter = 0;
  public bobOffset = 0;

  constructor(
    public species: string,
    public frameIndex: number,
    public timer: number,
    public duration: number
  ) {}

  reset(species: string, duration: number): void {
    this.species = species;
    this.frameIndex = 0;
    this.duration = duration;
    this.timer = duration;
    this.bobCounter = 0;
    this.bobOffset = 0;
  }

  setDuration(duration: number): void {
    this.duration = duration;
    if (this.timer > duration) {
      this.timer = duration;
    }
  }

  advanceFrame(frozen: boolean): void {
    if (frozen || this.duration <= 0) {
      return;
    }
    if (this.timer > 0) {
      this.timer -= 1;
    }
    if (this.timer <= 0) {
      this.frameIndex ^= 1;
      this.timer = this.duration;
    }
  }

  updateBobOffset(amplitude: number, active: boolean, frozen: boolean): number {
    if (!active) {
      this.bobOffset = 0;
      return this.bobOffset;
    }
    if (frozen) {
      return this.bobOffset;
    }
    const previous = this.bobCounter;
    this.bobCounter = (this.bobCounter + 1) % (BOB_TOGGLE_INTERVAL * 2);
    if (previous & 0x0f) {
      return this.bobOffset;
    }
    if (previous & 0x10) {
      this.bobOffset = amplitude;
    } else {
      this.bobOffset = 0;
    }
    return this.bobOffset;
  }
}

export class PartyMenuIconRenderer {
  private readonly palette = loadPalette();
  private readonly speciesIcon = loadSpeciesIconMap();
  private readonly iconFrames: Record<string, Surface[]> = {};
  private readonly states: Record<number, IconState> = {};
  private readonly itemOverlay = loadOverlay("item.png", this.palette);
  private readonly mailOverlay = loadOverlay("mail.png", this.palette);

  draw(
    surface: Surface,
    entries: PartyEntry[],
    {
      frozen,
      highlightSlot,
      switchOriginSlot,
      switchMode,
    }: {
      frozen: boolean;
      highlightSlot: number | null;
      switchOriginSlot: number | null;
      switchMode: boolean;
    }
  ): void {
    if (!surface || !entries.length) {
      Object.keys(this.states).forEach((key) => delete this.states[Number(key)]);
      return;
    }

    const slotLookup: Record<number, number> = {};
    entries.forEach((entry, idx) => {
      slotLookup[entry.index] = idx;
    });
    const effectiveHighlight = highlightSlot !== null && highlightSlot in slotLookup ? highlightSlot : null;
    const effectiveSwitch = switchOriginSlot !== null && switchOriginSlot in slotLookup ? switchOriginSlot : null;

    const activeSlots = new Set<number>();
    entries.forEach((entry, rowIndex) => {
      const pokemon = entry.pokemon as Pokemon & { max_hp?: number; item?: string | null };
      const speciesKey = iconSpeciesKey(pokemon);
      const iconName = this.speciesIcon[speciesKey] ?? "monster";
      const frames = this.iconFrames[iconName] ?? (this.iconFrames[iconName] = sliceIconFrames(iconName, this.palette));
      const zone = this.hpZone(pokemon);
      const duration = FRAME_DURATIONS[zone];
      const amplitude = BOB_AMPLITUDES[zone];

      let state = this.states[entry.index];
      if (!state) {
        state = new IconState(speciesKey, 0, duration, duration);
        this.states[entry.index] = state;
      } else if (state.species !== speciesKey) {
        state.reset(speciesKey, duration);
      } else {
        state.setDuration(duration);
      }

      const frame = frames[state.frameIndex % frames.length];
      const bobbing = this.shouldBob(entry.index, effectiveHighlight, effectiveSwitch, switchMode);
      const offset = state.updateBobOffset(amplitude, bobbing, frozen);
      const posX = this.iconX(entry.index, effectiveHighlight, effectiveSwitch, switchMode);
      const posY = this.iconY(rowIndex) + offset;
      surface.blit(frame, [posX, posY]);
      const overlay = this.overlayFor(pokemon);
      if (overlay) {
        surface.blit(overlay, [posX, posY + ICON_OVERLAY_OFFSET]);
      }
      state.advanceFrame(frozen);
      activeSlots.add(entry.index);
    });

    Object.keys(this.states).forEach((slot) => {
      const key = Number(slot);
      if (!activeSlots.has(key)) {
        delete this.states[key];
      }
    });
  }

  private overlayFor(pokemon: Pokemon & { item?: string | null }): Surface | null {
    const item = pokemon.item;
    if (!item) {
      return null;
    }
    if (item.toUpperCase().endsWith(MAIL_SUFFIX)) {
      return this.mailOverlay;
    }
    return this.itemOverlay;
  }

  private iconY(rowIndex: number): number {
    return ICON_BASE_Y + rowIndex * ICON_ROW_STRIDE;
  }

  private iconX(
    slotIndex: number,
    highlightSlot: number | null,
    switchOriginSlot: number | null,
    switchMode: boolean
  ): number {
    if (switchMode) {
      if (switchOriginSlot !== null && slotIndex === switchOriginSlot) {
        return ICON_SELECTED_X;
      }
      if (highlightSlot !== null && slotIndex === highlightSlot) {
        return ICON_SELECTED_X;
      }
      return ICON_BASE_X;
    }
    if (highlightSlot !== null && slotIndex === highlightSlot) {
      return ICON_SELECTED_X;
    }
    return ICON_BASE_X;
  }

  private shouldBob(
    slotIndex: number,
    highlightSlot: number | null,
    switchOriginSlot: number | null,
    switchMode: boolean
  ): boolean {
    if (switchMode) {
      return switchOriginSlot !== null && slotIndex === switchOriginSlot;
    }
    return highlightSlot !== null && slotIndex === highlightSlot;
  }

  private hpZone(pokemon: Pokemon & { max_hp?: number }): "green" | "yellow" | "red" {
    const maxHp = Math.max(1, Number(pokemon.max_hp ?? pokemon.hp ?? 1));
    const hp = Math.max(0, Math.min(pokemon.hp ?? 0, maxHp));
    const ratio = hp / maxHp;
    if (ratio >= GREEN_THRESHOLD) {
      return "green";
    }
    if (ratio >= YELLOW_THRESHOLD) {
      return "yellow";
    }
    return "red";
  }
}
