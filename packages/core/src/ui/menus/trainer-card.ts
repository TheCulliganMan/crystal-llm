// ASM mapping: pokecrystal_disassembly/engine/menus/trainer_card.asm (page transitions + badge anim).
import { GameState } from "@pokecrystal/core/core/state";
import { PlayerGender } from "@pokecrystal/core/core/enums";
import {
  BADGE_ENGINE_FLAG_ORDER,
  NUM_JOHTO_BADGES,
  assertAsmBadgeBanks,
  assertAsmJohtoBadgeBank,
} from "@pokecrystal/core/core/badges";
import { countPokedexEntries } from "@pokecrystal/core/core/pokedex";
import { syncGameClock } from "@pokecrystal/core/engine/systems/time";
import { Surface } from "@pokecrystal/core/ui/surface";
import { SPACE_TILE } from "@pokecrystal/core/ui/tilemap-surface";
import { BGMapWriter } from "@pokecrystal/core/ui/bg-map-sync";
import { GameButton, isButtonEvent, isKeyDownEvent, isStartEvent, type KeyEvent } from "@pokecrystal/core/input/buttons";
import { KEYS, keycodes } from "@pokecrystal/core/core/keycodes";
import {
  TrainerCardTilemap,
  clockColonCoords,
  idValueOrigin,
  moneyValueOrigin,
  nameValueOrigin,
  playTimeHoursOrigin,
  playTimeMinutesOrigin,
  pokedexValueOrigin,
  seedTrainerCardBadgePage,
  seedTrainerCardPageOne,
  smallColonTile,
  statusClearRegion,
  trainerCardBadgeTiles,
  trainerCardTileset,
} from "./trainer-card-layout";
import { FontRenderer } from "./types";

const JOHTO_BADGE_FLAG_NAMES: ReadonlyArray<string> = BADGE_ENGINE_FLAG_ORDER.slice(0, NUM_JOHTO_BADGES);
const KANTO_BADGE_FLAG_NAMES: ReadonlyArray<string> = BADGE_ENGINE_FLAG_ORDER.slice(NUM_JOHTO_BADGES);

enum TrainerCardPage {
  INFO = "info",
  JOHTO_BADGES = "johto_badges",
  KANTO_BADGES = "kanto_badges",
}

class BadgeSpec {
  constructor(
    public readonly name: string,
    public readonly x: number,
    public readonly y: number,
    public readonly palette: number,
    public readonly frames: number[]
  ) {}

  static fromRaw(name: string, rawX: number, rawY: number, palette: number, frames: number[]): BadgeSpec {
    return new BadgeSpec(name, rawX - 8, rawY - 16, palette, frames);
  }
}

const JOHTO_BADGES: BadgeSpec[] = [
  BadgeSpec.fromRaw("Zephyr", 0x18, 0x68, 0, [0x00, 0x20, 0x24, 0xa0, 0x00, 0x20, 0x24, 0xa0]),
  BadgeSpec.fromRaw("Hive", 0x38, 0x68, 0, [0x04, 0x20, 0x24, 0xa0, 0x04, 0x20, 0x24, 0xa0]),
  BadgeSpec.fromRaw("Plain", 0x58, 0x68, 0, [0x08, 0x20, 0x24, 0xa0, 0x08, 0x20, 0x24, 0xa0]),
  BadgeSpec.fromRaw("Fog", 0x78, 0x68, 0, [0x0c, 0x20, 0x24, 0xa0, 0x0c, 0x20, 0x24, 0xa0]),
  BadgeSpec.fromRaw("Mineral", 0x38, 0x80, 0, [0x10, 0x20, 0x24, 0xa0, 0x10, 0x20, 0x24, 0xa0]),
  BadgeSpec.fromRaw("Storm", 0x18, 0x80, 0, [0x14, 0x20, 0x24, 0xa0, 0x14, 0x20, 0x24, 0xa0]),
  BadgeSpec.fromRaw("Glacier", 0x58, 0x80, 0, [0x18, 0x20, 0x24, 0xa0, 0x18, 0x20, 0x24, 0xa0]),
  BadgeSpec.fromRaw("Rising", 0x78, 0x80, 0, [0x1c, 0x20, 0x24, 0xa0, 0x9c, 0x20, 0x24, 0xa0]),
];

const BADGE_TILE_OFFSETS = [0, 1, 2, 3];
const BADGE_TILE_OFFSETS_FLIPPED = [1, 0, 3, 2];

const badgeTilesForFrame = (spec: BadgeSpec, frameCounter: number): number[] => {
  if (!spec.frames.length) {
    throw new Error(`Badge ${spec.name} is missing animation frames.`);
  }
  const base = spec.frames[frameCounter % spec.frames.length];
  const flip = Boolean(base & 0x80);
  const baseIndex = base & 0x7f;
  const offsets = flip ? BADGE_TILE_OFFSETS_FLIPPED : BADGE_TILE_OFFSETS;
  const tiles = offsets.map((offset) => baseIndex + offset);
  if (flip) {
    return tiles.map((tile) => tile | 0x80);
  }
  return tiles;
};

interface TrainerCardUI {
  screen: Surface | null;
  font: FontRenderer;
}

export class TrainerCardScreen {
  private colonVisible = true;
  private page: TrainerCardPage = TrainerCardPage.INFO;
  private badgeFrameCounter = 0;
  private lastColonFrame = -1;
  private lastBadgeAnimationFrame = -1;
  private badgeSurfaces: Surface[] = trainerCardBadgeTiles();
  private badgeSurfaceCache = new Map<string, Surface>();
  private bgMapWriter: BGMapWriter;
  private badgeFrameCount: number;

  constructor(private readonly ui: TrainerCardUI, private readonly gameState: GameState) {
    this.bgMapWriter = new BGMapWriter(gameState, "vBGMap0");
    const badgeFrameLen = JOHTO_BADGES[0]?.frames.length ?? 0;
    if (!badgeFrameLen) {
      throw new Error("Badge frames must not be empty.");
    }
    for (const badge of JOHTO_BADGES) {
      if (badge.frames.length !== badgeFrameLen) {
        throw new Error("All badges must share the same frame count for ASM-compatible animation.");
      }
    }
    this.badgeFrameCount = badgeFrameLen;
  }

  reset(): void {
    this.colonVisible = true;
    this.page = TrainerCardPage.INFO;
    this.badgeFrameCounter = 0;
    this.lastColonFrame = -1;
    this.lastBadgeAnimationFrame = -1;
    this.badgeSurfaceCache.clear();
  }

  getActivePage(): string {
    return this.page;
  }

  handleInput(event: KeyEvent): string | null {
    if (!isKeyDownEvent(event)) {
      return null;
    }
    if (isButtonEvent(event, GameButton.B) || isStartEvent(event)) {
      return "exit";
    }
    const keyCode = normalizeKeyCode(event.code ?? event.key);
    if (keyCode === KEYS.LEFT || String(event.code ?? event.key ?? "") === "ArrowLeft") {
      if (this.page === TrainerCardPage.JOHTO_BADGES) {
        this.page = TrainerCardPage.INFO;
      } else if (this.page === TrainerCardPage.KANTO_BADGES) {
        this.enterBadgePage(TrainerCardPage.JOHTO_BADGES);
      }
      return null;
    }
    if (keyCode === KEYS.RIGHT || String(event.code ?? event.key ?? "") === "ArrowRight") {
      if (this.page === TrainerCardPage.INFO) {
        this.enterBadgePage(TrainerCardPage.JOHTO_BADGES);
      } else if (this.page === TrainerCardPage.JOHTO_BADGES && this.hasKantoBadges()) {
        this.enterBadgePage(TrainerCardPage.KANTO_BADGES);
      } else if (this.page === TrainerCardPage.KANTO_BADGES) {
        this.page = TrainerCardPage.INFO;
      }
      return null;
    }
    if (isButtonEvent(event, GameButton.A)) {
      if (this.page === TrainerCardPage.INFO) {
        this.enterBadgePage(TrainerCardPage.JOHTO_BADGES);
        return null;
      }
      return "exit";
    }
    return null;
  }

  draw(): void {
    syncGameClock(this.gameState);
    if (!this.ui.screen) {
      throw new Error("UI screen surface is not initialized.");
    }
    const gender = this.playerGender();
    if (this.page === TrainerCardPage.INFO) {
      const tilemap = this.buildTilemap(gender);
      this.tickColonAnimation();
      const colonTile = this.colonVisible ? smallColonTile() : SPACE_TILE;
      const [colonX, colonY] = clockColonCoords();
      tilemap.setTile(colonX, colonY, colonTile);
      const tileset = trainerCardTileset(this.ui.font, gender, { includeLeaderTiles: false, includeCornerTile: true });
      tilemap.blit(this.ui.screen, tileset);
      this.bgMapWriter.request(tilemap);
      return;
    }
    this.drawBadgePage(gender);
  }

  private buildTilemap(gender: PlayerGender): TrainerCardTilemap {
    const tilemap = new TrainerCardTilemap();
    seedTrainerCardPageOne(tilemap, gender);
    this.writeName(tilemap);
    this.writeId(tilemap);
    this.writeMoney(tilemap);
    this.writePokedex(tilemap);
    this.writePlayTime(tilemap);
    return tilemap;
  }

  private writeName(tilemap: TrainerCardTilemap): void {
    const name = this.gameState.sram.player_name.slice(0, 10);
    if (!name.length) {
      throw new Error("Trainer Card requires SRAM player_name; ASM reads wPlayerName directly.");
    }
    const [x, y] = nameValueOrigin();
    tilemap.writeText(x, y, name, { maxLength: 10, uppercase: false });
  }

  private writeId(tilemap: TrainerCardTilemap): void {
    const trainerId = String(this.gameState.sram.player_id).padStart(5, "0");
    const [x, y] = idValueOrigin();
    tilemap.writeText(x, y, trainerId, { maxLength: 5, pad: true });
  }

  private writeMoney(tilemap: TrainerCardTilemap): void {
    const amount = Math.max(0, Math.min(Math.trunc(this.gameState.sram.money), 999999));
    const moneyText = formatTrainerCardMoney(amount);
    const [x, y] = moneyValueOrigin();
    tilemap.writeText(x, y, moneyText, { maxLength: 7, pad: true });
  }

  private writePokedex(tilemap: TrainerCardTilemap): void {
    if (!this.hasPokedex()) {
      const region = statusClearRegion();
      tilemap.clearBox(region.left, region.top, region.width, region.height, { tile: SPACE_TILE });
      return;
    }
    const count = countPokedexEntries(this.gameState.sram.pokedex_owned);
    const [x, y] = pokedexValueOrigin();
    tilemap.writeText(x, y, String(count).padStart(3, " "), { maxLength: 3, pad: true });
  }

  private writePlayTime(tilemap: TrainerCardTilemap): void {
    const hours = Math.max(0, Math.min(this.gameState.sram.game_time_hours, 9999));
    const minutes = Math.max(0, Math.min(this.gameState.sram.game_time_minutes, 59));
    const [xHours, yHours] = playTimeHoursOrigin();
    tilemap.writeText(xHours, yHours, String(hours).padStart(4, " "), { maxLength: 4, pad: true });
    const [xMinutes, yMinutes] = playTimeMinutesOrigin();
    tilemap.writeText(xMinutes, yMinutes, String(minutes).padStart(2, "0"), { maxLength: 2, pad: true });
  }

  private enterBadgePage(page: TrainerCardPage.JOHTO_BADGES | TrainerCardPage.KANTO_BADGES): void {
    if (this.page === page) {
      return;
    }
    this.page = page;
    this.resetBadgeAnimation();
  }

  private drawBadgePage(gender: PlayerGender): void {
    const tilemap = this.buildTilemap(gender);
    seedTrainerCardBadgePage(tilemap, gender, { preserveTop: true });
    const tileset = trainerCardTileset(this.ui.font, gender, { includeLeaderTiles: true, includeCornerTile: true });
    tilemap.blit(this.ui.screen!, tileset);
    this.bgMapWriter.request(tilemap);
    this.tickBadgeAnimation();
    if (this.page === TrainerCardPage.JOHTO_BADGES || this.page === TrainerCardPage.KANTO_BADGES) {
      this.drawBadges();
    }
  }

  private resetBadgeAnimation(): void {
    this.badgeFrameCounter = 0;
    this.lastBadgeAnimationFrame = -1;
    this.badgeSurfaceCache.clear();
  }

  private tickBadgeAnimation(): void {
    const frame = this.currentVBlankFrame();
    if (frame === this.lastBadgeAnimationFrame) {
      return;
    }
    this.lastBadgeAnimationFrame = frame;
    if ((frame & 0x07) !== 0) {
      return;
    }
    this.badgeFrameCounter = (this.badgeFrameCounter + 1) % this.badgeFrameCount;
  }

  private drawBadges(): void {
    const flags = this.activeBadgeFlags();
    JOHTO_BADGES.forEach((spec, index) => {
      if (!flags[index]) {
        return;
      }
      const frameTiles = badgeTilesForFrame(spec, this.badgeFrameCounter);
      this.blitBadge(spec.x, spec.y, frameTiles);
    });
  }

  private blitBadge(x: number, y: number, frameTiles: number[]): void {
    if (!this.ui.screen) {
      return;
    }
    for (let row = 0; row < 2; row++) {
      for (let col = 0; col < 2; col++) {
        const frameTile = frameTiles[row * 2 + col];
        const xflip = Boolean(frameTile & 0x80);
        const tileIndex = frameTile & 0x7f;
        const surface = this.badgeSurfaceForTile(tileIndex, xflip);
        this.ui.screen.blit(surface, [x + col * 8, y + row * 8]);
      }
    }
  }

  private badgeSurfaceForTile(tileIndex: number, flip: boolean): Surface {
    if (tileIndex < 0 || tileIndex >= this.badgeSurfaces.length) {
      throw new Error(`Badge tile ${tileIndex} is out of bounds`);
    }
    if (!flip) {
      return this.badgeSurfaces[tileIndex];
    }
    const key = `${tileIndex}-flip`;
    const cached = this.badgeSurfaceCache.get(key);
    if (cached) {
      return cached;
    }
    const surface = this.flipSurface(this.badgeSurfaces[tileIndex]);
    this.badgeSurfaceCache.set(key, surface);
    return surface;
  }

  private flipSurface(source: Surface): Surface {
    const flipped = new Surface(source.width, source.height);
    for (let y = 0; y < source.height; y++) {
      for (let x = 0; x < source.width; x++) {
        flipped.setAt(source.width - 1 - x, y, source.getAt(x, y));
      }
    }
    return flipped;
  }

  get currentPage(): TrainerCardPage {
    return this.page;
  }

  private playerGender(): PlayerGender {
    const gender = this.gameState.sram.player_gender;
    if (gender === PlayerGender.FEMALE) {
      return PlayerGender.FEMALE;
    }
    return PlayerGender.MALE;
  }

  private hasPokedex(): boolean {
    const sram = this.gameState.sram;
    const hasDexFlag = Boolean(this.gameState.wram.engine_flags?.ENGINE_POKEDEX);
    return Boolean(sram.johto_pokedex || hasDexFlag);
  }

  private tickColonAnimation(): void {
    const frame = this.currentVBlankFrame();
    if (frame === this.lastColonFrame) {
      return;
    }
    this.lastColonFrame = frame;
    if ((frame & 0x1f) === 0) {
      this.colonVisible = !this.colonVisible;
    }
  }

  private currentVBlankFrame(): number {
    const frame = Number(this.gameState.frame_counter ?? 0);
    if (!Number.isFinite(frame)) {
      throw new Error("Trainer Card requires a finite frame_counter for ASM timing parity.");
    }
    return Math.max(0, Math.trunc(frame)) >>> 0;
  }

  private johtoBadgeFlags(): boolean[] {
    const badges = this.gameState.sram.badges;
    const johto = assertAsmJohtoBadgeBank(badges.johto, "Trainer Card Johto badges");
    return this.badgeFlags(johto, JOHTO_BADGE_FLAG_NAMES);
  }

  private kantoBadgeFlags(): boolean[] {
    const { kanto } = assertAsmBadgeBanks(this.gameState.sram.badges, "Trainer Card Kanto badges");
    return this.badgeFlags(kanto, KANTO_BADGE_FLAG_NAMES);
  }

  private activeBadgeFlags(): boolean[] {
    if (this.page === TrainerCardPage.KANTO_BADGES) {
      return this.kantoBadgeFlags();
    }
    return this.johtoBadgeFlags();
  }

  private hasKantoBadges(): boolean {
    return this.kantoBadgeFlags().some(Boolean);
  }

  private badgeFlags(badgeBank: readonly boolean[], flagNames: ReadonlyArray<string>): boolean[] {
    const flags: boolean[] = [];
    for (let index = 0; index < JOHTO_BADGES.length; index++) {
      let owned = Boolean(badgeBank[index]);
      if (!owned) {
        const flagName = flagNames[index];
        if (flagName) {
          owned = Boolean(this.gameState.wram.engine_flags?.[flagName]);
        }
      }
      flags.push(owned);
    }
    return flags;
  }
}

const normalizeKeyCode = (value: string | number | null | undefined): number | null => {
  if (value === null || value === undefined) {
    return null;
  }
  if (typeof value === "number") {
    return value;
  }
  const mapped = keycodes[value];
  return typeof mapped === "number" ? mapped : null;
};

const formatTrainerCardMoney = (amount: number): string => {
  const digits = String(amount).padStart(6, " ");
  const firstDigit = digits.search(/[0-9]/);
  if (firstDigit < 0) {
    throw new Error("Trainer Card money formatting requires at least one numeric digit.");
  }
  return `${digits.slice(0, firstDigit)}¥${digits.slice(firstDigit)}`;
};
