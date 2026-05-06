// ASM mapping: engine/pokegear/pokegear.asm menu logic + layout.
import { GameState } from "../../core/state";
import { PlayerGender } from "../../core/enums";
import { canonicaliseTimeOfDay, syncGameClock } from "../../engine/systems/time";
import type { AudioEngine } from "../../engine/systems/audio";
import { getMapMetadataByGroup } from "../../engine/world/maps";
import { TILE_SIZE } from "../../engine/world/tile";
import { isCancelEvent, isConfirmEvent, isKeyDownEvent } from "../../input/buttons";
import type { KeyEvent } from "../../input/buttons";
import { TileRegion } from "../tile-layout";
import { TilemapSurface } from "../tilemap-surface";
import type { TilemapTileset } from "../tilemap-surface";
import { buildDefaultCharMap } from "../text/glyph-map";
import { PokegearBackground } from "./pokegear-bg";
import {
  PokegearCard,
  PokegearStateMachine,
  PHONE_LIST_LENGTH,
  PokegearScriptRunner,
} from "./pokegear-state";
import {
  PhoneContactDirectory,
  loadPhoneContactDirectory,
  type PhoneContactRecord,
} from "./pokegear-contacts";
import { resolveLandmarkText } from "./pokegear-labels";
import { LandmarkEntry } from "@pokecrystal/assets/content/pokegear";
import { Surface } from "../surface";
import { projectLandmarkToTownMapPixel } from "../overlays/town-map-coords";
import { drawTownMapCursorMarker } from "../overlays/town-map-marker";

const CARD_LABELS: Record<PokegearCard, string> = {
  [PokegearCard.CLOCK]: "CLOCK",
  [PokegearCard.MAP]: "MAP",
  [PokegearCard.PHONE]: "PHONE",
  [PokegearCard.RADIO]: "RADIO",
};

const legendLines = (firstLine: string, ...rest: string[]): string[] => {
  return [firstLine, ...rest];
};

export const buildPokegearControlLines = (card: PokegearCard): string[] => {
  const lines = legendLines("L/R=Card B=Exit");
  if (card === PokegearCard.MAP) {
    lines.push("Up/Down=Move");
  } else if (card === PokegearCard.PHONE) {
    lines.push("Up/Down=Move A=Call");
  } else if (card === PokegearCard.RADIO) {
    lines.push("Up/Down=Tune");
  }
  return lines;
};
const DAY_NAMES = ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"];
const CARD_TABS_REGION = new TileRegion(0, 0, 20, 3);
const CLOCK_PANEL_REGION = new TileRegion(3, 5, 14, 5);
const MAP_PANEL_REGION = new TileRegion(1, 2, 18, 10);
const PHONE_LIST_REGION = new TileRegion(1, 4, 16, PHONE_LIST_LENGTH * 2);
const RADIO_STATION_REGION = new TileRegion(1, 8, 18, 3);
const RADIO_STATION_NAME_TILE: [number, number] = [2, 9];
const TEXTBOX_REGION = new TileRegion(0, 12, 20, 6);
const SWITCH_HINT_TILE: [number, number] = [12, 1];
const SIGNAL_REGION = new TileRegion(17, 1, 2, 2);
const PHONE_NAME_WIDTH = PHONE_LIST_REGION.width - 1;
const PHONE_CLASS_WIDTH = PHONE_LIST_REGION.width - 4;
const PHONE_CONTACTS = loadPhoneContactDirectory();
const CHAR_MAP = buildDefaultCharMap();
const WINDOW_BORDER_CHARS = new Set(["\u250c", "\u2500", "\u2510", "\u2502", "\u2514", "\u2518", " "]);
const GLYPH_TILE_IDS = new Set(
  Object.entries(CHAR_MAP)
    .filter(([char]) => !WINDOW_BORDER_CHARS.has(char))
    .map(([, tileId]) => tileId),
);
const SPACE_TILE_ID = CHAR_MAP[" "] ?? 0x7f;
const MAP_LABEL_ICON_TILE = 0x34;
const MAP_LABEL_TEXT_X = 9;
const MAP_LABEL_TEXT_WIDTH = 11;

type PokegearUI = {
  screen: Surface | null;
  font: { fontTiles?: Record<number, Surface>; font_tiles?: Record<number, Surface> };
};

type PokegearAudioEngine = Pick<AudioEngine, "playSound"> & {
  startRadioChannel?: (station: string, durationFrames?: number) => void;
  stopRadioChannel?: () => void;
  playMusic?: (song: string, role?: string) => void;
  restartMapMusic?: () => void;
};

const fontTilesForUi = (ui: PokegearUI): Record<number, Surface> => {
  return ui.font.fontTiles ?? ui.font.font_tiles ?? {};
};

class CompositeTileset {
  private readonly tiles: Array<Surface | undefined>;

  constructor(
    baseTiles: Surface[],
    overrides: Record<number, Surface>,
    glyphIds: Set<number>,
    options: { spaceFillColor: [number, number, number]; glyphColor: [number, number, number] },
  ) {
    this.tiles = [...baseTiles];
    const baseLen = baseTiles.length;
    for (const [tileIdRaw, tile] of Object.entries(overrides)) {
      const tileId = Number(tileIdRaw);
      if (!glyphIds.has(tileId) || tileId < baseLen) {
        continue;
      }
      this.tiles[tileId] = this.prepareGlyph(tile, options.spaceFillColor, options.glyphColor);
    }
  }

  getTiles(): Array<Surface | undefined> {
    return this.tiles;
  }

  private prepareGlyph(
    tile: Surface,
    spaceFillColor: [number, number, number],
    glyphColor: [number, number, number],
  ): Surface {
    const glyphSurface = new Surface(TILE_SIZE, TILE_SIZE);
    glyphSurface.fill([spaceFillColor[0], spaceFillColor[1], spaceFillColor[2], 255]);
    for (let y = 0; y < TILE_SIZE; y += 1) {
      for (let x = 0; x < TILE_SIZE; x += 1) {
        const [, , , a] = tile.getAt(x, y);
        if (a === 0) {
          continue;
        }
        glyphSurface.setAt(x, y, [glyphColor[0], glyphColor[1], glyphColor[2], Math.max(1, a)]);
      }
    }
    return glyphSurface;
  }
}

const drawTriangle = (surface: Surface, points: Array<[number, number]>, color: [number, number, number]): void => {
  if (points.length !== 3) {
    return;
  }
  const [p0, p1, p2] = points;
  const minX = Math.max(0, Math.floor(Math.min(p0[0], p1[0], p2[0])));
  const maxX = Math.min(surface.width - 1, Math.ceil(Math.max(p0[0], p1[0], p2[0])));
  const minY = Math.max(0, Math.floor(Math.min(p0[1], p1[1], p2[1])));
  const maxY = Math.min(surface.height - 1, Math.ceil(Math.max(p0[1], p1[1], p2[1])));

  const area = (ax: number, ay: number, bx: number, by: number, cx: number, cy: number): number =>
    (bx - ax) * (cy - ay) - (by - ay) * (cx - ax);
  const totalArea = area(p0[0], p0[1], p1[0], p1[1], p2[0], p2[1]);
  if (totalArea === 0) {
    return;
  }
  for (let y = minY; y <= maxY; y += 1) {
    for (let x = minX; x <= maxX; x += 1) {
      const w0 = area(p1[0], p1[1], p2[0], p2[1], x, y);
      const w1 = area(p2[0], p2[1], p0[0], p0[1], x, y);
      const w2 = area(p0[0], p0[1], p1[0], p1[1], x, y);
      if ((w0 >= 0 && w1 >= 0 && w2 >= 0) || (w0 <= 0 && w1 <= 0 && w2 <= 0)) {
        surface.setAt(x, y, [color[0], color[1], color[2], 255]);
      }
    }
  }
};

const drawCircle = (surface: Surface, center: [number, number], radius: number, color: [number, number, number]): void => {
  const [cx, cy] = center;
  const r2 = radius * radius;
  for (let y = Math.max(0, cy - radius); y <= Math.min(surface.height - 1, cy + radius); y += 1) {
    for (let x = Math.max(0, cx - radius); x <= Math.min(surface.width - 1, cx + radius); x += 1) {
      const dx = x - cx;
      const dy = y - cy;
      if (dx * dx + dy * dy <= r2) {
        surface.setAt(x, y, [color[0], color[1], color[2], 255]);
      }
    }
  }
};

export class PokegearScreen {
  private readonly screen: Surface | null;
  private readonly logic: PokegearStateMachine;
  private readonly audioEngine: PokegearAudioEngine | null;
  private playerGender: PlayerGender;
  private bg: PokegearBackground;
  private tileset: TilemapTileset;
  private contactDirectory: PhoneContactDirectory;
  private pointerColor: [number, number, number];
  private phoneStatusMessage: string | null = null;
  private phoneStatusTimer = 0;
  private activeRadioSignature: string | null = null;

  constructor(
    private readonly ui: PokegearUI,
    private readonly gameState: GameState,
    options?: { scriptRunner?: PokegearScriptRunner | null; audioEngine?: PokegearAudioEngine | null },
  ) {
    this.screen = ui.screen;
    this.logic = new PokegearStateMachine(gameState, options?.scriptRunner ?? undefined);
    this.audioEngine = options?.audioEngine ?? null;
    this.playerGender = this.resolvePlayerGender();
    this.bg = new PokegearBackground(this.playerGender);
    this.tileset = this.buildTileset();
    this.contactDirectory = PHONE_CONTACTS;
    this.pointerColor = this.bg.pointerHighlightColor();
    this.logic.refresh();
    if (this.logic.availableCards.includes(PokegearCard.MAP)) {
      this.logic.forceCard(PokegearCard.MAP);
    }
  }

  static layoutRegions(): Record<string, TileRegion> {
    return {
      tabs: CARD_TABS_REGION,
      clock_panel: CLOCK_PANEL_REGION,
      map_panel: MAP_PANEL_REGION,
      phone_list: PHONE_LIST_REGION,
      radio_station: RADIO_STATION_REGION,
      textbox: TEXTBOX_REGION,
    };
  }

  get mode(): string {
    return this.logic.currentCard.toString();
  }

  getControlLines(): string[] {
    return buildPokegearControlLines(this.logic.currentCard);
  }

  private buildTileset(): TilemapTileset {
    const baseTiles = this.bg.tileSurfaces();
    const overrides = fontTilesForUi(this.ui);
    return new CompositeTileset(baseTiles, overrides, GLYPH_TILE_IDS, {
      spaceFillColor: this.bg.windowFillColor(),
      glyphColor: [0, 0, 0],
    }).getTiles();
  }

  private resolvePlayerGender(): PlayerGender {
    const gender = this.gameState.sram.player_gender;
    return typeof gender === "number" ? (gender as PlayerGender) : PlayerGender.MALE;
  }

  private ensureGenderAssets(): void {
    const current = this.resolvePlayerGender();
    if (current === this.playerGender) {
      return;
    }
    this.playerGender = current;
    this.bg.setPlayerGender(current);
    this.tileset = this.buildTileset();
    this.pointerColor = this.bg.pointerHighlightColor();
    this.syncHardwareState();
  }

  private syncHardwareState(): void {
    this.bg.syncHardware({ mapGroup: this.gameState.wram.wMapGroup, mapNumber: this.gameState.wram.wMapNumber });
  }

  handleInput(event: KeyEvent): string | null {
    if (!isKeyDownEvent(event)) {
      return null;
    }
    if (isCancelEvent(event)) {
      this.playMenuClick();
      this.exitPokegearRadio();
      return "exit";
    }
    this.ensureGenderAssets();
    this.logic.refresh();
    const key = event.key;
    if (key === "ArrowLeft") {
      this.logic.switchCard(-1);
      this.syncPokegearRadio();
      this.playMenuClick();
    } else if (key === "ArrowRight") {
      this.logic.switchCard(1);
      this.syncPokegearRadio();
      this.playMenuClick();
    } else if (this.logic.currentCard === PokegearCard.MAP) {
      if (key === "ArrowUp") {
        this.logic.moveMapCursor(-1);
      } else if (key === "ArrowDown") {
        this.logic.moveMapCursor(1);
      }
    } else if (this.logic.currentCard === PokegearCard.PHONE) {
      if (key === "ArrowUp") {
        this.logic.movePhoneCursor(-1);
      } else if (key === "ArrowDown") {
        this.logic.movePhoneCursor(1);
      } else if (isConfirmEvent(event)) {
        this.attemptPhoneCall();
      }
    } else if (this.logic.currentCard === PokegearCard.RADIO) {
      if (key === "ArrowUp") {
        this.logic.tuneRadio(1);
        this.syncPokegearRadio();
      } else if (key === "ArrowDown") {
        this.logic.tuneRadio(-1);
        this.syncPokegearRadio();
      }
    }
    return null;
  }

  draw(): void {
    syncGameClock(this.gameState);
    this.ensureGenderAssets();
    this.syncHardwareState();
    this.logic.refresh();
    this.syncPokegearRadio();
    if (!this.screen) {
      return;
    }
    const tilemap = this.buildTilemap();
    tilemap.blit(this.screen, this.tileset);
    this.drawIndicatorArrow(this.screen);
    if (this.logic.currentCard === PokegearCard.MAP) {
      this.drawMapOverlays(this.screen);
    }
  }

  getTextOverlay(): { viewportLines: string[]; infoLines: string[]; menuLines: string[] } {
    const card = this.logic.currentCard;
    const cardLabel = CARD_LABELS[card] ?? String(card);
    const viewportLines = [`POKEGEAR ${cardLabel}`];
    const infoLines = this.getControlLines();
    const menuLines: string[] = [];

    if (card === PokegearCard.CLOCK) {
      viewportLines.push(this.dayLabel());
      viewportLines.push(this.formatTime());
      return { viewportLines, infoLines, menuLines };
    }

    if (card === PokegearCard.MAP) {
      const cursorLabel = resolveLandmarkText(this.logic.mapCursorEntry).split("\n").join(" / ");
      const playerLabel = resolveLandmarkText(this.logic.mapPlayerEntry).split("\n").join(" / ");
      viewportLines.push(`MAP: ${cursorLabel}`);
      viewportLines.push(`PLAYER: ${playerLabel}`);
      viewportLines.push(`REGION: ${this.logic.mapRegion}`);
      return { viewportLines, infoLines, menuLines };
    }

    if (card === PokegearCard.PHONE) {
      const numbers = this.logic.phoneNumbers;
      if (!numbers.length) {
        menuLines.push("NO ENTRIES");
      } else {
        const total = numbers.length;
        const cursor = Math.min(this.logic.phoneCursor, total - 1);
        const scroll = this.logic.phoneScroll;
        const visible = numbers.slice(scroll, scroll + PHONE_LIST_LENGTH);
        if (scroll > 0) {
          menuLines.push("▲ more above");
        }
        for (let index = 0; index < visible.length; index += 1) {
          const contactId = visible[index];
          const absolute = scroll + index;
          const prefix = absolute === cursor ? "\u25b6" : " ";
          const lines = this.contactDirectory.displayLines(contactId);
          const topLine = lines[0];
          menuLines.push(`${prefix} ${topLine}`);
          if (lines.length > 1) {
            menuLines.push(`   ${lines[1]}`);
          }
        }
        if (scroll + visible.length < total) {
          menuLines.push("▼ more below");
        }
      }
      if (!this.logic.phoneServiceAvailable()) {
        viewportLines.push("PHONE: NO SERVICE");
      } else if (this.phoneStatusMessage) {
        viewportLines.push(`PHONE: ${this.phoneStatusMessage}`);
      }
      return { viewportLines, infoLines, menuLines };
    }

    if (card === PokegearCard.RADIO) {
      const frequency = this.logic.radioFrequency.frequency.toFixed(1);
      const station = this.logic.currentRadioStation();
      viewportLines.push(`FREQ: ${frequency}`);
      viewportLines.push(`STATION: ${station ? station.name : "NO SIGNAL"}`);
      return { viewportLines, infoLines, menuLines };
    }

    return { viewportLines, infoLines, menuLines };
  }

  private buildTilemap(): TilemapSurface {
    const tilemap = new TilemapSurface();
    const baseTiles = this.bg.tilemapForCard(this.logic.currentCard, this.logic.mapRegion);
    tilemap.loadTiles(baseTiles);
    this.applyCardTabs(tilemap);
    const card = this.logic.currentCard;
    if (card === PokegearCard.CLOCK) {
      const prompt = this.applyClockTiles(tilemap);
      this.writeBottomText(tilemap, prompt);
    } else if (card === PokegearCard.MAP) {
      this.applyMapTiles(tilemap);
    } else if (card === PokegearCard.PHONE) {
      const prompt = this.applyPhoneTiles(tilemap);
      this.writeBottomText(tilemap, prompt);
    } else {
      const prompt = this.applyRadioTiles(tilemap);
      this.writeBottomText(tilemap, prompt);
    }
    return tilemap;
  }

  private applyCardTabs(tilemap: TilemapSurface): void {
    for (const row of [0, 1]) {
      tilemap.fillRect(0, row, 8, 1, SPACE_TILE_ID);
    }
    const cards = new Set(this.logic.availableCards);
    if (cards.has(PokegearCard.MAP)) {
      this.placeCardIcon(tilemap, 2, 0, 0x40);
    }
    if (cards.has(PokegearCard.PHONE)) {
      this.placeCardIcon(tilemap, 4, 0, 0x44);
    }
    if (cards.has(PokegearCard.RADIO)) {
      this.placeCardIcon(tilemap, 6, 0, 0x42);
    }
    this.placeCardIcon(tilemap, 0, 0, 0x46);
  }

  private placeCardIcon(tilemap: TilemapSurface, x: number, y: number, baseTile: number): void {
    tilemap.setTile(x, y, baseTile);
    tilemap.setTile(x + 1, y, (baseTile + 1) & 0xff);
    const bottom = (baseTile + 0x10) & 0xff;
    tilemap.setTile(x, y + 1, bottom);
    tilemap.setTile(x + 1, y + 1, (bottom + 1) & 0xff);
  }

  private writeBottomText(tilemap: TilemapSurface, text: string): void {
    const region = TEXTBOX_REGION;
    const content = text || CARD_LABELS[this.logic.currentCard] || "POKEGEAR";
    tilemap.fillRect(region.left + 1, region.top + 1, region.width - 2, region.height - 2, SPACE_TILE_ID);
    tilemap.writeText(region.left + 1, region.top + 1, content, {
      maxLength: region.width - 2,
      uppercase: false,
    });
  }

  private applyClockTiles(tilemap: TilemapSurface): string {
    tilemap.writeText(SWITCH_HINT_TILE[0], SWITCH_HINT_TILE[1], " SWITCH\u25b6", { maxLength: 8 });
    tilemap.clearBox(CLOCK_PANEL_REGION.left, CLOCK_PANEL_REGION.top, 14, 5, SPACE_TILE_ID);
    const dayLabel = this.dayLabel();
    const timeString = this.formatTime();
    tilemap.writeText(6, 6, dayLabel, { maxLength: 14 });
    tilemap.writeText(6, 8, timeString, { maxLength: 14, uppercase: false });
    return `${dayLabel} ${timeString}`;
  }

  private formatTime(): string {
    const hours = Math.max(0, this.gameState.sram.game_time_hours) % 24;
    const minutes = Math.max(0, this.gameState.sram.game_time_minutes) % 60;
    const period = hours < 12 ? "AM" : "PM";
    let displayHour = hours % 12;
    if (displayHour === 0) {
      displayHour = 12;
    }
    return `${String(displayHour).padStart(2, " ")}:${String(minutes).padStart(2, "0")}${period}`;
  }

  private dayLabel(): string {
    const dayIndex = this.gameState.sram.day_of_week % DAY_NAMES.length;
    const timeOfDay = canonicaliseTimeOfDay(String(this.gameState.wram.time_of_day ?? "day"));
    return `${DAY_NAMES[dayIndex]} ${timeOfDay}`;
  }

  private applyMapTiles(tilemap: TilemapSurface): void {
    tilemap.fillRect(1, 2, 18, 1, 0x07);
    tilemap.setTile(0, 2, 0x06);
    tilemap.setTile(19, 2, 0x17);
    const label = resolveLandmarkText(this.logic.mapCursorEntry);
    this.writeMapLabel(tilemap, label);
  }

  private writeMapLabel(tilemap: TilemapSurface, label: string): void {
    tilemap.clearBox(8, 0, 12, 2, SPACE_TILE_ID);
    tilemap.setTile(8, 0, MAP_LABEL_ICON_TILE);
    const [first, second] = label.split("\n", 2);
    tilemap.writeText(MAP_LABEL_TEXT_X, 0, first ?? "", {
      maxLength: MAP_LABEL_TEXT_WIDTH,
      pad: true,
      uppercase: false,
    });
    if (second) {
      tilemap.writeText(MAP_LABEL_TEXT_X, 1, second, {
        maxLength: MAP_LABEL_TEXT_WIDTH,
        pad: true,
        uppercase: false,
      });
    }
  }

  private applyPhoneTiles(tilemap: TilemapSurface): string {
    const pointerColumn = PHONE_LIST_REGION.left;
    tilemap.clearBox(pointerColumn, PHONE_LIST_REGION.top, PHONE_LIST_REGION.width, PHONE_LIST_REGION.height, SPACE_TILE_ID);
    const numbers = this.logic.phoneNumbers;
    if (!numbers.length) {
      tilemap.writeText(pointerColumn + 1, 4, "NO ENTRIES", { maxLength: PHONE_NAME_WIDTH });
      this.placePhoneBars(tilemap);
      return "NO PHONE NUMBERS";
    }
    const total = numbers.length;
    const cursor = Math.min(this.logic.phoneCursor, total - 1);
    const scroll = this.logic.phoneScroll;
    const visible = numbers.slice(scroll, scroll + PHONE_LIST_LENGTH);
    for (let index = 0; index < PHONE_LIST_LENGTH; index += 1) {
      const row = PHONE_LIST_REGION.top + index * 2;
      const absolute = scroll + index;
      const pointer = absolute === cursor ? "\u25b6" : " ";
      tilemap.writeText(pointerColumn, row, pointer, { maxLength: 1 });
      if (index >= visible.length) {
        continue;
      }
      const contactId = visible[index];
      const lines = this.contactDirectory.displayLines(contactId);
      const topLine = lines[0];
      tilemap.writeText(pointerColumn + 1, row, topLine, {
        maxLength: PHONE_NAME_WIDTH,
        uppercase: false,
      });
      if (lines.length > 1) {
        tilemap.writeText(pointerColumn + 4, row + 1, lines[1], {
          maxLength: PHONE_CLASS_WIDTH,
          uppercase: false,
        });
      }
    }
    this.placePhoneBars(tilemap);
    const status = this.consumePhoneStatusMessage();
    if (status) {
      return status;
    }
    if (!this.logic.phoneServiceAvailable()) {
      return "NO SERVICE";
    }
    return this.contactDirectory.primaryLabel(numbers[cursor]);
  }

  private placePhoneBars(tilemap: TilemapSurface): void {
    tilemap.setTile(17, 1, 0x3c);
    tilemap.setTile(18, 1, 0x3d);
    tilemap.setTile(17, 2, 0x3e);
    tilemap.setTile(18, 2, this.logic.phoneServiceAvailable() ? 0x3f : 0x4f);
  }

  private applyRadioTiles(tilemap: TilemapSurface): string {
    const station = this.logic.currentRadioStation();
    const name = station ? station.name : "NO SIGNAL";
    tilemap.clearBox(
      RADIO_STATION_REGION.left,
      RADIO_STATION_REGION.top,
      RADIO_STATION_REGION.width,
      RADIO_STATION_REGION.height,
      SPACE_TILE_ID,
    );
    tilemap.writeText(RADIO_STATION_NAME_TILE[0], RADIO_STATION_NAME_TILE[1], name, {
      maxLength: RADIO_STATION_REGION.width - 1,
      uppercase: false,
    });
    return name;
  }

  private drawIndicatorArrow(surface: Surface): void {
    const points = this.indicatorPoints();
    if (!points.length) {
      return;
    }
    drawTriangle(surface, points, this.pointerColor);
  }

  private indicatorPoints(): Array<[number, number]> {
    if (!this.logic.availableCards.length) {
      return [];
    }
    const activeCard = Number(this.logic.currentCard);
    const iconCenterTile = activeCard * 2 + 1;
    const arrowX = iconCenterTile * TILE_SIZE;
    const arrowY = TILE_SIZE * 2;
    return [
      [arrowX, arrowY],
      [arrowX - 4, arrowY + 7],
      [arrowX + 4, arrowY + 7],
    ];
  }

  private drawMapOverlays(surface: Surface): void {
    const project = (entry: LandmarkEntry): [number, number] => {
      return projectLandmarkToTownMapPixel(entry);
    };

    const [playerX, playerY] = project(this.logic.mapPlayerEntry);
    drawCircle(surface, [playerX, playerY], 2, this.pointerColor);
    const [cursorX, cursorY] = project(this.logic.mapCursorEntry);
    drawTownMapCursorMarker(surface, [cursorX, cursorY]);
  }

  private attemptPhoneCall(): void {
    const numbers = this.logic.phoneNumbers;
    if (!numbers.length) {
      this.setPhoneStatus("NO PHONE NUMBERS");
      return;
    }
    const contactId = numbers[this.logic.phoneCursor];
    if (!contactId) {
      this.setPhoneStatus("CAN'T CALL");
      return;
    }
    const record = this.contactDirectory.record(contactId);
    if (!record) {
      this.setPhoneStatus("CAN'T CALL");
      return;
    }
    if (!this.logic.hasScriptRunner()) {
      this.setPhoneStatus("NO CALL HANDLER");
      throw new Error("Pok\u00e9gear phone calls require a script runner.");
    }
    if (!this.logic.phoneServiceAvailable()) {
      this.setPhoneStatus("NO SERVICE");
      return;
    }
    this.audioEngine?.playSound("SFX_CALL");
    if (this.gameState.wram.wLinkMode) {
      this.setPhoneStatus("NO SERVICE");
      this.logic.runPhoneScript("PhoneOutOfAreaScript");
      return;
    }
    if (!this.contactAvailable(record)) {
      this.setPhoneStatus("OUT OF AREA");
      this.logic.runPhoneScript("PhoneOutOfAreaScript");
      return;
    }
    const queued = this.logic.beginPhoneCall();
    if (!queued) {
      this.setPhoneStatus("OUT OF AREA");
      return;
    }
    this.logic.consumePhoneCall();
    const scriptName = this.resolvePhoneScript(record);
    if (scriptName) {
      this.logic.runPhoneScript(scriptName);
    }
    this.setPhoneStatus(this.contactDirectory.primaryLabel(contactId));
  }

  private playMenuClick(): void {
    this.audioEngine?.playSound("SFX_READ_TEXT_2");
  }

  private syncPokegearRadio(): void {
    if (this.logic.currentCard !== PokegearCard.RADIO) {
      this.exitPokegearRadio();
      return;
    }
    const station = this.logic.currentRadioStation();
    if (!station) {
      this.stopPokegearRadio();
      return;
    }
    const signature = `${station.constant}:${station.song}`;
    if (this.activeRadioSignature === signature) {
      return;
    }
    this.activeRadioSignature = signature;
    if (typeof this.audioEngine?.startRadioChannel === "function") {
      this.audioEngine.startRadioChannel(station.constant, 0);
      return;
    }
    this.audioEngine?.playMusic?.(station.song, "radio");
  }

  private exitPokegearRadio(): void {
    this.activeRadioSignature = null;
  }

  private stopPokegearRadio(): void {
    if (!this.activeRadioSignature) {
      return;
    }
    this.activeRadioSignature = null;
    if (typeof this.audioEngine?.stopRadioChannel === "function") {
      this.audioEngine.stopRadioChannel();
      return;
    }
    this.audioEngine?.restartMapMusic?.();
  }

  private resolvePhoneScript(record: PhoneContactRecord): string {
    const mapMetadata = getMapMetadataByGroup(this.gameState.wram.wMapGroup, this.gameState.wram.wMapNumber);
    if (record.mapConstant && mapMetadata?.constant === record.mapConstant) {
      return "PhoneScript_JustTalkToThem";
    }
    if (record.calleeScript) {
      return record.calleeScript;
    }
    return "PhoneOutOfAreaScript";
  }

  private contactAvailable(record: PhoneContactRecord): boolean {
    const mask = record.calleeTimeMask;
    if (mask === 0) {
      return false;
    }
    return Boolean(mask & this.currentTimeMask());
  }

  private currentTimeMask(): number {
    const timeOfDay = canonicaliseTimeOfDay(String(this.gameState.wram.time_of_day ?? "day"));
    if (timeOfDay === "MORN") {
      return 0x1;
    }
    if (timeOfDay === "DAY") {
      return 0x2;
    }
    return 0x4;
  }

  private setPhoneStatus(message: string, durationFrames: number = 120): void {
    this.phoneStatusMessage = message;
    this.phoneStatusTimer = Math.max(1, durationFrames);
  }

  private consumePhoneStatusMessage(): string | null {
    if (!this.phoneStatusMessage || this.phoneStatusTimer <= 0) {
      this.phoneStatusMessage = null;
      this.phoneStatusTimer = 0;
      return null;
    }
    this.phoneStatusTimer -= 1;
    if (this.phoneStatusTimer <= 0) {
      const msg = this.phoneStatusMessage;
      this.phoneStatusMessage = null;
      return msg;
    }
    return this.phoneStatusMessage;
  }
}
