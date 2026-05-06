// ASM mapping: pokecrystal_disassembly/engine/pokemon/stats_screen.asm (stats pages + joypad latching).
import fs from "fs";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { GameState } from "@pokecrystal/core/core/state";
import { Pokemon } from "@pokecrystal/core/core/models";
import { PlayerGender, PokemonType } from "@pokecrystal/core/core/enums";
import { GenderRatio, MonType } from "@pokecrystal/core/core/enums/pokemon";
import { calculateExperience } from "@pokecrystal/core/engine/experience";
import { BGMapWriter } from "@pokecrystal/core/ui/bg-map-sync";
import { TilemapSurface } from "@pokecrystal/core/ui/tilemap-surface";
import type { TilemapTileset } from "@pokecrystal/core/ui/tilemap-surface";
import { Surface } from "@pokecrystal/core/ui/surface";
import { buildDefaultCharMap } from "@pokecrystal/core/ui/text/glyph-map";
import { ID_GLYPH, LV_GLYPH } from "@pokecrystal/core/ui/text/constants";
import {
  B_PAD_A,
  B_PAD_B,
  B_PAD_DOWN,
  B_PAD_LEFT,
  B_PAD_RIGHT,
  B_PAD_UP,
  KeyEvent,
  GameButton,
  isButtonEvent,
  isKeyDownEvent,
  isKeyUpEvent,
} from "@pokecrystal/core/input/controls";
import { updateJoypadStateFromKeys } from "@pokecrystal/core/core/joypad";
import { gbc5To8, gbcWordToRgb } from "@pokecrystal/core/core/gbc-colors";
import { buttonKeys } from "@pokecrystal/core/input/controls";
import { normalizeKeycode } from "@pokecrystal/core/core/keycodes";
import { loadMoveMetadata } from "@pokecrystal/core/ui/overlays/battle-experience";

const TILE_SIZE = 8;

const PINK_PAGE = 1;
const GREEN_PAGE = 2;
const BLUE_PAGE = 3;
const NUM_STAT_PAGES = 3;
const STAT_PAGE_MASK = 0x03;
const PAGE_LABELS = ["STATUS", "MOVES", "STATS"] as const;

const CHAR_MAP = buildDefaultCharMap();
const NUMBER_TILE = 0x74;
const SHINY_TILE = 0x3f;
const HORIZONTAL_DIVIDER_TILE = 0x62;
const VERTICAL_DIVIDER_TILE = 0x31;
const PAGE_INDICATOR_SMALL = 0x36;
const PAGE_INDICATOR_LARGE = 0x3a;
const EXP_LEFT_CAP = 0x40;
const EXP_RIGHT_CAP = 0x41;
const STATS_TILE_BASE = 0x31;
const STATS_TILE_COUNT = 17;

const HP_TILE_START = 0x60;
const HP_EMPTY_TILE = HP_TILE_START + 0x02;
const HP_FULL_TILE = HP_TILE_START + 0x0a;
const HP_END_TILE = HP_TILE_START + 0x0b;
const EXP_BAR_ATTR = 0x02;
const PINK_ATTR = 0x03;
const GREEN_ATTR = 0x04;
const BLUE_ATTR = 0x05;
const TOP_HALF_ATTR = 0x01;
const CLEAR_TILE = 0x4f;
const MAX_STAT_VALUE = 999;
const MAX_STAT_EXP_SQRT = 255;

const PAGES_PALETTE_PATH = getAssetPath("gfx", "stats", "pages.pal");
const STATS_PALETTE_PATH = getAssetPath("gfx", "stats", "stats.pal");
const STATS_TILES_PATH = getAssetPath("gfx", "stats", "stats_tiles.2bpp");
const EXP_PALETTE_PATH = getAssetPath("gfx", "battle", "exp_bar.pal");

const HP_BAR_LENGTH_PX = 6 * TILE_SIZE;
const HP_GREEN_PIXEL_THRESHOLD = Math.floor((HP_BAR_LENGTH_PX * 50) / 100);
const HP_YELLOW_PIXEL_THRESHOLD = Math.floor((HP_BAR_LENGTH_PX * 21) / 100);

const STATUS_LABELS: Record<string, string> = {
  NONE: "OK",
  POISON: "PSN",
  SLEEP: "SLP",
  PARALYSIS: "PAR",
  BURN: "BRN",
  FREEZE: "FRZ",
  CONFUSION: "CNF",
};

const HP_GB_PALETTES: Record<number, [number, number, number][]> = {
  0: [
    [31, 31, 31],
    [30, 26, 15],
    [0, 23, 0],
    [0, 0, 0],
  ],
  1: [
    [31, 31, 31],
    [30, 26, 15],
    [31, 21, 0],
    [0, 0, 0],
  ],
  2: [
    [31, 31, 31],
    [30, 26, 15],
    [31, 0, 0],
    [0, 0, 0],
  ],
};

type Palette = [number, number, number][];
type PaletteOrder = Palette[];

interface ComputedStats {
  maxHp: number;
  attack: number;
  defense: number;
  specialAttack: number;
  specialDefense: number;
  speed: number;
}

type LegacyPokemonAdditionalFields = {
  pokerus_status?: number;
  pokerus?: number | boolean;
  original_trainer_gender?: PlayerGender | number | null;
  caught_gender?: PlayerGender | number | null;
};

type PokemonWithLegacyFields = Pokemon & LegacyPokemonAdditionalFields;

export interface StatsUI {
  screen: Surface | null;
  font: {
    paletteVariants?: (paletteOrder: PaletteOrder) => Record<number, Record<number, Surface>>;
  };
  playCry?: (speciesId: string) => void;
  getPokemonFrontSurface?: (speciesId: string, frame?: number) => Surface | null;
}

export class PokemonStatsScreen {
  private pokemon: Pokemon | null = null;
  private bgMapWriter: BGMapWriter;
  private page = PINK_PAGE;
  private monType: MonType = MonType.PARTYMON;
  private partyIndex = 0;
  private lastTilemap: TilemapSurface | null = null;
  private tilesetCache = new Map<string, TilemapTileset>();
  private monPaletteCache = new Map<string, Palette>();
  private statTileLevels: Record<number, number[]> | null = null;
  private statTilesetCache = new Map<string, TilemapTileset>();
  private heldKeys = new Set<string>();

  constructor(private readonly ui: StatsUI, private readonly gameState: GameState) {
    this.bgMapWriter = new BGMapWriter(gameState, "vBGMap0");
  }

  reset(): void {
    this.pokemon = null;
    this.page = PINK_PAGE;
    this.lastTilemap = null;
    this.heldKeys.clear();
    this.gameState.wram.wStatsScreenFlags &= ~STAT_PAGE_MASK;
    this.gameState.wram.wMenuJoypadFilter = 0;
  }

  getActivePokemon(): Pokemon | null {
    return this.pokemon;
  }

  getActivePage(): number {
    return this.page;
  }

  getTextOverlay(): { viewportLines: string[]; menuLines: string[] | null } {
    const viewportLines: string[] = ["POKEMON STATS"];
    if (!this.pokemon) {
      viewportLines.push("NO POKEMON SELECTED");
      return { viewportLines, menuLines: null };
    }
    const pokemon = this.pokemon;
    const computed = this.computedStats(pokemon);
    const displayedHp = this.displayedHp(pokemon, computed.maxHp);
    const [, hpPixels, hpPaletteIndex] = this.hpBarState(pokemon, computed.maxHp, displayedHp);

    const displayName = this.pokemonDisplayName(pokemon);
    viewportLines.push(`${displayName} L${pokemon.level}`);
    viewportLines.push(`SPECIES: ${pokemon.species.id}`);
    viewportLines.push(`DEX: ${String(pokemon.species.int_id).padStart(3, "0")}`);
    viewportLines.push(`GENDER: ${this.genderSymbol(pokemon) ?? "-"}`);
    viewportLines.push(`SHINY: ${this.isShiny(pokemon) ? "YES" : "NO"}`);
    viewportLines.push(`PAGE: ${this.page}/${NUM_STAT_PAGES}`);
    viewportLines.push("");
    viewportLines.push(`PAGE ${this.page}: ${PAGE_LABELS[this.page - 1] ?? "STATS"}`);

    if (pokemon.species.id === "EGG") {
      viewportLines.push("EGG STATUS");
      viewportLines.push(this.eggHatchLine(pokemon, 0));
      viewportLines.push(this.eggHatchLine(pokemon, 1));
      viewportLines.push(this.eggHatchLine(pokemon, 2));
      viewportLines.push(this.eggHatchLine(pokemon, 3));
      return { viewportLines, menuLines: null };
    }

    if (this.page === PINK_PAGE) {
      const [type1, type2] = this.typeLabels(pokemon);
      viewportLines.push(`HP: ${displayedHp}/${computed.maxHp}`);
      viewportLines.push(`STATUS: ${this.statusLabel(pokemon)}`);
      viewportLines.push(`TYPE: ${type2 ? `${type1}/${type2}` : type1}`);
      viewportLines.push(`EXP: ${pokemon.experience}`);
      viewportLines.push(`TO NEXT: ${this.expToNextLevel(pokemon)}`);
      viewportLines.push(`EXP FILL: ${this.expFillUnits(pokemon)}/64`);
      viewportLines.push(`HP BAR: ${hpPixels}/${HP_BAR_LENGTH_PX} ${this.hpPaletteLabel(hpPaletteIndex)}`);
      return { viewportLines, menuLines: null };
    }

    if (this.page === GREEN_PAGE) {
      const moveRows = this.moveRows(pokemon);
      viewportLines.push(`ITEM: ${this.itemName(pokemon)}`);
      moveRows.names.forEach((moveName, idx) => {
        viewportLines.push(`MOVE ${idx + 1}: ${moveName}`);
        const pp = moveRows.pp[idx]?.replace(/^PP\s*/, "") ?? "--/--";
        viewportLines.push(`PP${idx + 1}: ${pp}`);
      });
      return { viewportLines, menuLines: null };
    }

    viewportLines.push(`OT: ${pokemon.original_trainer_name}`);
    viewportLines.push(`ID: ${String(pokemon.original_trainer_id).padStart(5, "0")}`);
    viewportLines.push(`OT GENDER: ${this.otGenderSymbol(pokemon) ?? "-"}`);
    viewportLines.push(`ATTACK: ${computed.attack}`);
    viewportLines.push(`DEFENSE: ${computed.defense}`);
    viewportLines.push(`SPCL ATK: ${computed.specialAttack}`);
    viewportLines.push(`SPCL DEF: ${computed.specialDefense}`);
    viewportLines.push(`SPEED: ${computed.speed}`);
    viewportLines.push(
      `DVS: ATK ${pokemon.dvs.attack} DEF ${pokemon.dvs.defense} SPD ${pokemon.dvs.speed} SPC ${pokemon.dvs.special}`
    );
    viewportLines.push(
      `STAT EXP: HP ${pokemon.hp_exp ?? 0} ATK ${pokemon.attack_exp ?? 0} DEF ${pokemon.defense_exp ?? 0}`
    );
    viewportLines.push(`STAT EXP: SPC ${pokemon.special_exp ?? 0} SPD ${pokemon.speed_exp ?? 0}`);
    return { viewportLines, menuLines: null };
  }

  showPokemon(pokemon: Pokemon, { monType, partyIndex }: { monType?: MonType; partyIndex?: number } = {}): void {
    this.pokemon = pokemon;
    let resolvedType = monType ?? null;
    let resolvedIndex = partyIndex ?? null;
    if (!resolvedType) {
      const partyList = this.gameState.sram.party.pokemon;
      if (partyList.includes(pokemon)) {
        resolvedType = MonType.PARTYMON;
        resolvedIndex = partyList.indexOf(pokemon);
      } else {
        resolvedType = MonType.BOXMON;
      }
    }
    if (resolvedIndex === null) {
      resolvedIndex = resolvedType !== MonType.PARTYMON ? 0 : this.gameState.wram.wCurPartyMon;
    }
    this.monType = resolvedType;
    this.partyIndex = Math.max(0, resolvedIndex);
    const wram = this.gameState.wram;
    wram.wMonType = this.monType;
    wram.wCurPartyMon = this.partyIndex;
    wram.wCurPartySpecies = pokemon.species.id.toUpperCase();
    if (this.monType === MonType.PARTYMON) {
      const party = this.gameState.sram.party.pokemon.filter((mon) => mon !== null);
      wram.wPartyCount = party.length;
      wram.wPartyMenuCursor = this.partyIndex + 1;
    }
    wram.wMenuJoypadFilter = B_PAD_A | B_PAD_B | B_PAD_LEFT | B_PAD_RIGHT | B_PAD_UP | B_PAD_DOWN;
    this.setPage(PINK_PAGE);
    this.heldKeys.clear();
    if (pokemon.species.id !== "EGG") {
      this.playCry(pokemon);
    }
  }

  handleInput(event: KeyEvent): string | null {
    if (!this.pokemon) {
      return null;
    }
    if (!isKeyDownEvent(event) && !isKeyUpEvent(event)) {
      return null;
    }
    if (isKeyDownEvent(event) && isButtonEvent(event, GameButton.B)) {
      this.reset();
      return "exit";
    }
    this.updateHeldKeys(event);
    const hram = this.gameState.hram.joypad;
    hram.hJoyDown = 0;
    hram.hJoypadDown = 0;
    const pressed = this.latchMenuJoypad();
    if (!isKeyDownEvent(event)) {
      return null;
    }
    if (pressed & B_PAD_B) {
      this.reset();
      return "exit";
    }
    const isEgg = this.pokemon.species.id === "EGG";
    if (isEgg && pressed & B_PAD_A) {
      this.reset();
      return "exit";
    }
    if (!isEgg) {
      if (pressed & B_PAD_LEFT) {
        this.advancePage(-1);
        return null;
      }
      if (pressed & B_PAD_RIGHT) {
        this.advancePage(1);
        return null;
      }
      if (pressed & B_PAD_A) {
        if (this.page === BLUE_PAGE) {
          this.reset();
          return "exit";
        }
        this.advancePage(1);
        return null;
      }
    }
    if (pressed & B_PAD_UP) {
      if (this.shiftParty(-1) && this.pokemon && this.pokemon.species.id !== "EGG") {
        this.playCry(this.pokemon);
      }
      return null;
    }
    if (pressed & B_PAD_DOWN) {
      if (this.shiftParty(1) && this.pokemon && this.pokemon.species.id !== "EGG") {
        this.playCry(this.pokemon);
      }
      return null;
    }
    return null;
  }

  draw(): void {
    if (!this.ui.screen) {
      throw new Error("UI screen surface is not initialized.");
    }
    if (!this.pokemon) {
      return;
    }
    const computed = this.computedStats(this.pokemon);
    const displayedHp = this.displayedHp(this.pokemon, computed.maxHp);
    const [hpTiles, , hpPaletteIndex] = this.hpBarState(this.pokemon, computed.maxHp, displayedHp);
    this.gameState.wram.wCurHPPal = hpPaletteIndex;
    const monPalette = this.monPalette(this.pokemon);

    const tilemap = new TilemapSurface({ fillTile: CLEAR_TILE });
    tilemap.clearTilemap({ tile: CLEAR_TILE, attr: 0 });
    if (this.pokemon.species.id === "EGG") {
      this.drawEggPage(tilemap, this.pokemon);
    } else {
      this.drawUpper(tilemap, this.pokemon);
      this.drawPageIndicators(tilemap);
      if (this.page === PINK_PAGE) {
        this.drawPinkPage(tilemap, this.pokemon, hpTiles, displayedHp, computed.maxHp);
      } else if (this.page === GREEN_PAGE) {
        this.drawGreenPage(tilemap, this.pokemon);
      } else {
        this.drawBluePage(tilemap, this.pokemon, computed);
      }
      this.applyAttrmap(tilemap);
    }

    this.lastTilemap = tilemap;
    const paletteOrder = this.paletteOrder(monPalette, hpPaletteIndex);
    const cacheKey = `${this.pokemon.species.id.toLowerCase()}-${this.isShiny(this.pokemon)}-${hpPaletteIndex}-${this.page}`;
    this.writeTilemap(tilemap, this.tileset(paletteOrder, cacheKey));
    this.drawFrontSprite(this.pokemon);
  }

  get debugTilemap(): TilemapSurface | null {
    return this.lastTilemap;
  }

  private shiftParty(delta: number): boolean {
    const party = this.gameState.sram.party.pokemon.filter((mon) => mon !== null) as Pokemon[];
    if (!party.length) {
      return false;
    }
    const current = Math.max(0, Math.min(this.partyIndex, party.length - 1));
    const newIndex = current + delta;
    if (newIndex < 0 || newIndex >= party.length) {
      return false;
    }
    this.partyIndex = newIndex;
    this.pokemon = party[newIndex];
    const wram = this.gameState.wram;
    wram.wPartyCount = party.length;
    wram.wCurPartyMon = newIndex;
    wram.wCurPartySpecies = this.pokemon.species.id.toUpperCase();
    wram.wPartyMenuCursor = newIndex + 1;
    this.setPage(PINK_PAGE);
    return true;
  }

  private advancePage(delta: number): void {
    const newPage = ((this.page - 1 + delta + NUM_STAT_PAGES) % NUM_STAT_PAGES) + 1;
    if (newPage !== this.page) {
      this.setPage(newPage);
    }
  }

  private setPage(page: number): void {
    this.page = Math.max(1, Math.min(page, NUM_STAT_PAGES));
    const masked = this.gameState.wram.wStatsScreenFlags & ~STAT_PAGE_MASK;
    this.gameState.wram.wStatsScreenFlags = masked | (this.page & STAT_PAGE_MASK);
  }

  private drawUpper(tilemap: TilemapSurface, pokemon: Pokemon): void {
    this.placeDexHeader(tilemap, pokemon);
    this.placeLevel(tilemap, pokemon);
    this.placeNicknameAndSpecies(tilemap, pokemon);
    this.placeGender(tilemap, pokemon);
    this.placeShiny(tilemap, pokemon);
    this.placeHorizontalDivider(tilemap);
    this.placeArrows(tilemap);
  }

  private drawPageIndicators(tilemap: TilemapSurface): void {
    this.writeIndicator(tilemap, 13, 5, PAGE_INDICATOR_SMALL);
    this.writeIndicator(tilemap, 15, 5, PAGE_INDICATOR_SMALL);
    this.writeIndicator(tilemap, 17, 5, PAGE_INDICATOR_SMALL);
    const pageX = this.page === PINK_PAGE ? 13 : this.page === GREEN_PAGE ? 15 : 17;
    this.writeIndicator(tilemap, pageX, 5, PAGE_INDICATOR_LARGE);
  }

  private writeIndicator(tilemap: TilemapSurface, x: number, y: number, baseTile: number): void {
    tilemap.setTile(x, y, baseTile);
    tilemap.setTile(x + 1, y, baseTile + 1);
    tilemap.setTile(x, y + 1, baseTile + 2);
    tilemap.setTile(x + 1, y + 1, baseTile + 3);
  }

  private drawEggPage(tilemap: TilemapSurface, pokemon: Pokemon): void {
    this.placeHorizontalDivider(tilemap);
    tilemap.writeText(8, 1, "EGG", { maxLength: 3, pad: false, uppercase: true });
    tilemap.writeText(8, 3, ID_GLYPH, { maxLength: 1, pad: false, uppercase: false });
    tilemap.setTile(9, 3, NUMBER_TILE);
    tilemap.setTile(10, 3, CHAR_MAP["."]); 
    tilemap.writeText(8, 5, "OT/", { maxLength: 3, pad: false, uppercase: true });
    tilemap.writeText(11, 3, "?????", { maxLength: 5, pad: false });
    tilemap.writeText(11, 5, "?????", { maxLength: 5, pad: false });

    const lines = this.eggHatchLines(pokemon);
    let row = 9;
    for (const line of lines) {
      tilemap.writeText(1, row, line, { maxLength: 19, pad: false, uppercase: false });
      row += 2;
    }
  }

  private drawPinkPage(
    tilemap: TilemapSurface,
    pokemon: Pokemon,
    hpTiles: number[],
    displayedHp: number,
    maxHp: number
  ): void {
    tilemap.clearBox(0, 8, 20, 10);
    this.placeHpBar(tilemap, hpTiles, displayedHp, maxHp);
    this.drawVerticalDivider(tilemap, 9);
    tilemap.writeText(0, 12, "STATUS/", { maxLength: 7, pad: false, uppercase: true });
    tilemap.writeText(0, 14, "TYPE/", { maxLength: 5, pad: false, uppercase: true });
    const pokerusStatus = this.pokerusStatus(pokemon);
    if (pokerusStatus & 0x0f) {
      tilemap.writeText(1, 13, "#RUS", { maxLength: 4, pad: false, uppercase: false });
    } else {
      if (pokerusStatus & 0xf0) {
        tilemap.setTile(8, 8, CHAR_MAP["."]); 
      }
      const status = this.statusLabel(pokemon);
      if (status === "OK") {
        tilemap.writeText(6, 13, "OK", { maxLength: 2, pad: true });
      } else {
        tilemap.writeText(6, 13, status, { maxLength: 3, pad: true });
      }
    }
    const [type1, type2] = this.typeLabels(pokemon);
    tilemap.writeText(1, 15, type1, { maxLength: 8, pad: true, uppercase: true });
    tilemap.writeText(1, 16, type2 ?? "", { maxLength: 8, pad: true, uppercase: true });
    tilemap.writeText(10, 9, "EXP POINTS", { maxLength: 10, pad: false, uppercase: true });
    tilemap.writeText(10, 12, "LEVEL UP", { maxLength: 8, pad: false, uppercase: true });
    tilemap.writeText(14, 14, "TO", { maxLength: 2, pad: false, uppercase: true });
    tilemap.writeText(17, 14, this.formatLevelText(this.nextLevel(pokemon)), {
      maxLength: 3,
      pad: true,
    });
    tilemap.writeText(13, 10, `${pokemon.experience}`.padStart(7, " "), { maxLength: 7, pad: true });
    const expToNext = this.expToNextLevel(pokemon);
    tilemap.writeText(13, 13, `${expToNext}`.padStart(7, " "), { maxLength: 7, pad: true });
    this.placeExpBar(tilemap, pokemon);
  }

  private drawGreenPage(tilemap: TilemapSurface, pokemon: Pokemon): void {
    tilemap.clearBox(0, 8, 20, 10);
    tilemap.writeText(0, 8, "ITEM", { maxLength: 4, pad: false, uppercase: true });
    tilemap.writeText(8, 8, this.itemName(pokemon), { maxLength: 12, pad: true, uppercase: true });
    tilemap.writeText(0, 10, "MOVE", { maxLength: 4, pad: false, uppercase: true });
    const moveRows = this.moveRows(pokemon);
    moveRows.names.forEach((moveName, idx) => {
      tilemap.writeText(8, 10 + idx * 2, moveName, { maxLength: 12, pad: true, uppercase: true });
    });
    moveRows.pp.forEach((ppText, idx) => {
      tilemap.writeText(12, 11 + idx * 2, ppText, { maxLength: 8, pad: true, uppercase: false });
    });
  }

  private drawBluePage(tilemap: TilemapSurface, pokemon: Pokemon, stats: ComputedStats): void {
    tilemap.clearBox(0, 8, 20, 10);
    this.drawVerticalDivider(tilemap, 10);
    tilemap.writeText(0, 9, ID_GLYPH, { maxLength: 1, pad: false, uppercase: false });
    tilemap.setTile(1, 9, NUMBER_TILE);
    tilemap.setTile(2, 9, CHAR_MAP["."]); 
    tilemap.writeText(0, 12, "OT/", { maxLength: 3, pad: false, uppercase: true });
    tilemap.writeText(2, 10, `${pokemon.original_trainer_id}`.padStart(5, "0"), { maxLength: 5, pad: true });
    tilemap.writeText(2, 13, pokemon.original_trainer_name, { maxLength: 10, pad: true, uppercase: true });
    const otGender = this.otGenderSymbol(pokemon);
    if (otGender) {
      tilemap.writeText(9, 13, otGender, { maxLength: 1, pad: false, uppercase: false });
    }
    const labels = ["ATTACK", "DEFENSE", "SPCL.ATK", "SPCL.DEF", "SPEED"];
    const values = [stats.attack, stats.defense, stats.specialAttack, stats.specialDefense, stats.speed];
    let labelRow = 8;
    let valueRow = 9;
    const xLabel = 11;
    const xValue = xLabel + 6;
    labels.forEach((label, idx) => {
      tilemap.writeText(xLabel, labelRow, label, { maxLength: label.length, pad: false, uppercase: true });
      tilemap.writeText(xValue, valueRow, `${values[idx]}`.padStart(3, " "), { maxLength: 3, pad: true });
      labelRow += 2;
      valueRow += 2;
    });
  }

  private placeDexHeader(tilemap: TilemapSurface, pokemon: Pokemon): void {
    tilemap.setTile(8, 0, NUMBER_TILE);
    tilemap.setTile(9, 0, CHAR_MAP["."]);
    const dexNumber = Math.max(0, pokemon.species.int_id);
    const digits = dexNumber.toString().padStart(3, "0");
    for (let offset = 0; offset < digits.length; offset++) {
      tilemap.setTile(10 + offset, 0, CHAR_MAP[digits[offset]]);
    }
  }

  private placeLevel(tilemap: TilemapSurface, pokemon: Pokemon): void {
    const levelText = this.formatLevelText(Math.max(1, pokemon.level));
    tilemap.writeText(14, 0, levelText, { maxLength: 3, pad: true, uppercase: false });
  }

  private placeNicknameAndSpecies(tilemap: TilemapSurface, pokemon: Pokemon): void {
    const nickname = (pokemon.nickname ?? "").slice(0, 10);
    tilemap.writeText(8, 2, nickname, { maxLength: 10, pad: true, uppercase: false });
    tilemap.writeText(9, 4, "/", { maxLength: 1, pad: false });
    tilemap.writeText(10, 4, pokemon.species.id, { maxLength: 10, pad: false, uppercase: true });
  }

  private placeGender(tilemap: TilemapSurface, pokemon: Pokemon): void {
    const genderChar = this.genderSymbol(pokemon);
    if (genderChar) {
      tilemap.writeText(18, 0, genderChar, { maxLength: 1, pad: false, uppercase: false });
    }
  }

  private placeShiny(tilemap: TilemapSurface, pokemon: Pokemon): void {
    if (this.isShiny(pokemon)) {
      tilemap.setTile(19, 0, SHINY_TILE);
    }
  }

  private placeHorizontalDivider(tilemap: TilemapSurface): void {
    for (let x = 0; x < tilemap.width; x++) {
      tilemap.setTile(x, 7, HORIZONTAL_DIVIDER_TILE);
    }
  }

  private drawVerticalDivider(tilemap: TilemapSurface, x: number): void {
    for (let row = 8; row < 18; row++) {
      tilemap.setTile(x, row, VERTICAL_DIVIDER_TILE);
    }
  }

  private placeArrows(tilemap: TilemapSurface): void {
    tilemap.writeText(12, 6, "◀", { maxLength: 1, pad: false });
    tilemap.writeText(19, 6, "▶", { maxLength: 1, pad: false });
  }

  private placeHpBar(tilemap: TilemapSurface, tiles: number[], displayedHp: number, maxHp: number): void {
    const safeHp = Math.max(0, displayedHp);
    const safeMaxHp = Math.max(0, maxHp);
    tilemap.writeTiles(0, 9, tiles);
    tilemap.setTile(8, 9, EXP_RIGHT_CAP);
    tilemap.writeText(1, 10, `${safeHp}/${safeMaxHp}`.padStart(7, " "), {
      maxLength: 7,
      pad: true,
    });
  }

  private placeExpBar(tilemap: TilemapSurface, pokemon: Pokemon): void {
    const expTiles = this.expBarTiles(pokemon);
    tilemap.setTile(10, 16, EXP_LEFT_CAP);
    expTiles.forEach((tile, idx) => tilemap.setTile(11 + idx, 16, tile));
    tilemap.setTile(19, 16, EXP_RIGHT_CAP);
  }

  private displayedHp(pokemon: Pokemon, maxHp: number): number {
    if (this.monType === MonType.BOXMON) {
      return Math.max(0, maxHp);
    }
    return Math.max(0, Math.min(pokemon.hp, maxHp));
  }

  private statExpTerm(statExp: number): number {
    const value = Math.max(0, Math.trunc(statExp));
    let root = Math.floor(Math.sqrt(value));
    if (root * root < value) {
      root += 1;
    }
    root = Math.min(root, MAX_STAT_EXP_SQRT);
    return Math.floor(root / 4);
  }

  private calcStatValue({ base, dv, statExp, level, isHp }: { base: number; dv: number; statExp: number; level: number; isHp: boolean }): number {
    const baseTerm = Math.max(0, base) + Math.max(0, dv);
    const term = baseTerm * 2 + this.statExpTerm(statExp);
    const scaled = Math.floor((term * Math.max(1, level)) / 100);
    const value = isHp ? scaled + Math.max(1, level) + 10 : scaled + 5;
    return Math.min(value, MAX_STAT_VALUE);
  }

  private computedStats(pokemon: Pokemon): ComputedStats {
    const level = Math.max(1, pokemon.level);
    const base = pokemon.species.base_stats;
    const dvs = pokemon.dvs;
    const hpDv = ((dvs.attack & 0x01) << 3) | ((dvs.defense & 0x01) << 2) | ((dvs.speed & 0x01) << 1) | (dvs.special & 0x01);
    const maxHp = this.calcStatValue({ base: base.hp, dv: hpDv, statExp: pokemon.hp_exp ?? 0, level, isHp: true });
    const attack = this.calcStatValue({ base: base.attack, dv: dvs.attack, statExp: pokemon.attack_exp ?? 0, level, isHp: false });
    const defense = this.calcStatValue({ base: base.defense, dv: dvs.defense, statExp: pokemon.defense_exp ?? 0, level, isHp: false });
    const speed = this.calcStatValue({ base: base.speed, dv: dvs.speed, statExp: pokemon.speed_exp ?? 0, level, isHp: false });
    const specialAttack = this.calcStatValue({ base: base.special_attack, dv: dvs.special, statExp: pokemon.special_exp ?? 0, level, isHp: false });
    const specialDefense = this.calcStatValue({ base: base.special_defense, dv: dvs.special, statExp: pokemon.special_exp ?? 0, level, isHp: false });
    return { maxHp, attack, defense, specialAttack, specialDefense, speed };
  }

  private hpBarTiles(pokemon: Pokemon, maxHp?: number, currentHp?: number): [number[], number] {
    const maxHpValue = Math.max(0, maxHp ?? pokemon.max_hp);
    const currentHpValue = this.monType === MonType.BOXMON ? maxHpValue : Math.max(0, Math.min(currentHp ?? pokemon.hp, maxHpValue));
    let pixels = 0;
    if (maxHpValue > 0 && currentHpValue > 0) {
      pixels = Math.floor((currentHpValue * HP_BAR_LENGTH_PX) / maxHpValue);
      if (pixels === 0) {
        pixels = 1;
      }
    }
    pixels = Math.min(HP_BAR_LENGTH_PX, pixels);
    const tiles = [HP_TILE_START, HP_TILE_START + 1];
    let remaining = pixels;
    for (let i = 0; i < 6; i++) {
      if (remaining >= TILE_SIZE) {
        tiles.push(HP_FULL_TILE);
        remaining -= TILE_SIZE;
      } else if (remaining > 0) {
        tiles.push(HP_EMPTY_TILE + remaining);
        remaining = 0;
      } else {
        tiles.push(HP_EMPTY_TILE);
      }
    }
    tiles.push(HP_END_TILE);
    return [tiles, pixels];
  }

  private expBarTiles(pokemon: Pokemon): number[] {
    const units = this.expFillUnits(pokemon);
    const tiles = Array(8).fill(HORIZONTAL_DIVIDER_TILE);
    const fullTiles = Math.min(tiles.length, Math.floor(units / TILE_SIZE));
    const remainder = units % TILE_SIZE;
    for (let index = 0; index < tiles.length; index++) {
      const column = tiles.length - 1 - index;
      if (index < fullTiles) {
        tiles[column] = HP_FULL_TILE;
        continue;
      }
      if (index === fullTiles && remainder > 0) {
        tiles[column] = 0x54 + remainder;
        continue;
      }
      tiles[column] = HORIZONTAL_DIVIDER_TILE;
    }
    return tiles;
  }

  private expFillUnits(pokemon: Pokemon): number {
    const growthRate = pokemon.species.growth_rate;
    if (!growthRate) {
      return 0;
    }
    const level = Math.max(1, Math.min(99, pokemon.level));
    if (pokemon.level >= 100) {
      return 0;
    }
    const currentExp = Math.max(0, calculateExperience(growthRate, level));
    const nextExp = Math.max(0, calculateExperience(growthRate, Math.min(100, level + 1)));
    const span = Math.max(1, nextExp - currentExp);
    const cappedExp = Math.max(currentExp, Math.min(pokemon.experience, nextExp));
    const remaining = Math.max(0, nextExp - cappedExp);
    let unitsRemaining = Math.floor((remaining * 64) / span);
    unitsRemaining = Math.max(0, Math.min(64, unitsRemaining));
    return 64 - unitsRemaining;
  }

  private expToNextLevel(pokemon: Pokemon): number {
    if (pokemon.level >= 100) {
      return 0;
    }
    const nextLevelExp = calculateExperience(pokemon.species.growth_rate, pokemon.level + 1);
    return Math.max(0, nextLevelExp - pokemon.experience);
  }

  private nextLevel(pokemon: Pokemon): number {
    return pokemon.level >= 100 ? pokemon.level : pokemon.level + 1;
  }

  private itemName(pokemon: Pokemon): string {
    if (!pokemon.item) {
      return "---";
    }
    return pokemon.item.replace(/_/g, " ").toUpperCase();
  }

  private moveRows(pokemon: Pokemon): { names: string[]; pp: string[] } {
    const names: string[] = [];
    const ppText: string[] = [];
    const moves = pokemon.moves ?? [];
    const moveMetadata = loadMoveMetadata();
    for (let idx = 0; idx < 4; idx++) {
      const move = moves[idx];
      if (!move) {
        names.push("---");
        ppText.push("PP --/--");
        continue;
      }
      names.push(String(move.name).replace(/_/g, " "));
      const maxPp = moveMetadata.get(move.name)?.pp ?? move.current_pp;
      const currentPp = Math.max(0, Math.min(move.current_pp, maxPp));
      ppText.push(`PP ${String(currentPp).padStart(2, " ")}/${String(maxPp).padStart(2, " ")}`);
    }
    return { names, pp: ppText };
  }

  private typeLabels(pokemon: Pokemon): [string, string | null] {
    const type1 = this.formatType(pokemon.species.type1);
    const type2 = this.formatType(pokemon.species.type2 ?? PokemonType.NONE);
    if (type1 === type2 || pokemon.species.type2 === PokemonType.NONE) {
      return [type1, null];
    }
    return [type1, type2];
  }

  private formatType(typeValue: PokemonType): string {
    if (typeValue === PokemonType.CURSE_TYPE) {
      return "???";
    }
    return typeValue.replace(/_TYPE$/, "").replace(/_/g, " ");
  }

  private statusLabel(pokemon: Pokemon): string {
    if (this.monType === MonType.BOXMON) {
      return "OK";
    }
    if (pokemon.hp <= 0) {
      return "FNT";
    }
    const statusKey = pokemon.status ?? "NONE";
    return STATUS_LABELS[String(statusKey)] ?? "OK";
  }

  private pokerusStatus(pokemon: Pokemon): number {
    const legacy = pokemon as PokemonWithLegacyFields;
    const raw = legacy.pokerus_status ?? legacy.pokerus ?? 0;
    if (Array.isArray(raw)) {
      return raw.length ? raw[0] & 0xff : 0;
    }
    const value = Number(raw);
    return Number.isNaN(value) ? 0 : value & 0xff;
  }

  private otGenderSymbol(pokemon: Pokemon): string | null {
    const legacy = pokemon as PokemonWithLegacyFields;
    const raw = legacy.original_trainer_gender ?? legacy.caught_gender;
    if (raw === null || raw === undefined) {
      return null;
    }
    if (raw === PlayerGender.MALE) {
      return "♂";
    }
    if (raw === PlayerGender.FEMALE) {
      return "♀";
    }
    if (typeof raw === "number") {
      if (raw === 0 || raw === 0x7f) {
        return null;
      }
      return raw & 0x80 ? "♀" : "♂";
    }
    return null;
  }

  private genderSymbol(pokemon: Pokemon): string | null {
    if (pokemon.gender === PlayerGender.MALE) {
      return "♂";
    }
    if (pokemon.gender === PlayerGender.FEMALE) {
      return "♀";
    }
    const dvs = pokemon.dvs;
    const ratio = pokemon.species.gender_ratio;
    if (ratio === GenderRatio.GENDER_UNKNOWN || !dvs) {
      return null;
    }
    if (ratio === GenderRatio.GENDER_F0) {
      return "♂";
    }
    if (ratio === GenderRatio.GENDER_F100) {
      return "♀";
    }
    const dvValue = ((dvs.attack & 0x0f) << 4) | (dvs.speed & 0x0f);
    return ratio >= dvValue ? "♀" : "♂";
  }

  private isShiny(pokemon: Pokemon): boolean {
    const dvs = pokemon.dvs;
    return (
      dvs.defense === 10 &&
      dvs.speed === 10 &&
      dvs.special === 10 &&
      [2, 3, 6, 7, 10, 11, 14, 15].includes(dvs.attack)
    );
  }

  private latchMenuJoypad(): number {
    updateJoypadStateFromKeys(this.gameState.hram.joypad, this.heldKeys);
    const joypad = this.gameState.hram.joypad;
    const filtered = joypad.hJoyPressed & (this.gameState.wram.wMenuJoypadFilter & 0xff);
    this.gameState.wram.wMenuJoypad = joypad.hJoyDown;
    return filtered;
  }

  private updateHeldKeys(event: KeyEvent): void {
    const keys = new Set<string>();
    const rawKey = event.code ?? event.key;
    if (rawKey !== null && rawKey !== undefined) {
      keys.add(String(rawKey));
      const normalized = normalizeKeycode(rawKey);
      if (normalized !== null) {
        keys.add(String(normalized));
      }
    }
    if (event.direction) {
      const directionKey = {
        up: "ArrowUp",
        down: "ArrowDown",
        left: "ArrowLeft",
        right: "ArrowRight",
      }[event.direction];
      if (directionKey) {
        keys.add(directionKey);
      }
    }
    if (event.button) {
      for (const buttonKey of buttonKeys(event.button)) {
        keys.add(buttonKey);
      }
    }
    if (!keys.size) {
      return;
    }
    if (isKeyDownEvent(event)) {
      for (const value of keys) {
        this.heldKeys.add(value);
      }
    } else if (isKeyUpEvent(event)) {
      for (const value of keys) {
        this.heldKeys.delete(value);
      }
    }
  }

  private writeTilemap(tilemap: TilemapSurface, tileset: TilemapTileset): void {
    if (!this.ui.screen) {
      return;
    }
    tilemap.blit(this.ui.screen, tileset);
    this.bgMapWriter.request(tilemap);
  }

  private tileset(paletteOrder: PaletteOrder, cacheKey: string): TilemapTileset {
    const cached = this.tilesetCache.get(cacheKey);
    if (cached) {
      return cached;
    }
    const paletteVariants = this.ui.font.paletteVariants;
    if (!paletteVariants) {
      throw new Error("PokemonStatsScreen requires font palette variants.");
    }
    const baseTileset = paletteVariants(paletteOrder);
    const tileset: TilemapTileset = { ...baseTileset };
    const statVariants = this.statsTileVariants(paletteOrder);
    for (const [tileId, paletteMap] of Object.entries(statVariants)) {
      tileset[Number(tileId)] = paletteMap;
    }
    this.tilesetCache.set(cacheKey, tileset);
    return tileset;
  }

  private playCry(pokemon: Pokemon): void {
    if (this.ui.playCry) {
      this.ui.playCry(pokemon.species.id);
    }
  }

  private drawFrontSprite(pokemon: Pokemon): void {
    if (!this.ui.getPokemonFrontSurface || !this.ui.screen) {
      return;
    }
    const sprite = this.ui.getPokemonFrontSurface(pokemon.species.id, 0);
    if (!sprite) {
      return;
    }
    this.ui.screen.blit(sprite, [0, 0]);
  }

  private formatLevelText(level: number): string {
    if (level < 100) {
      return `${LV_GLYPH}${String(level).padEnd(2, " ")}`;
    }
    return String(level).padEnd(3, " ").slice(0, 3);
  }

  private monPalette(pokemon: Pokemon): Palette {
    const shiny = this.isShiny(pokemon);
    const cacheKey = `${pokemon.species.id.toLowerCase()}-${shiny}`;
    const cached = this.monPaletteCache.get(cacheKey);
    if (cached) {
      return cached;
    }
    const sourceDir = getAssetPath("gfx", "pokemon", pokemon.species.id.toLowerCase());
    let middleColours: [number, number, number][];
    if (shiny) {
      const palettePath = `${sourceDir}/shiny.pal`;
      middleColours = loadPaletteEntries(palettePath);
      if (middleColours.length !== 2) {
        throw new Error(`Shiny palette ${palettePath} must contain exactly 2 colours.`);
      }
    } else {
      const palettePath = `${sourceDir}/normal.gbcpal`;
      middleColours = decodeGbcpalPalette(palettePath);
      if (middleColours.length !== 2) {
        throw new Error(`Palette ${palettePath} must contain exactly 2 middle colours.`);
      }
    }
    const palette: Palette = [
      [255, 255, 255],
      middleColours[0],
      middleColours[1],
      [0, 0, 0],
    ];
    this.monPaletteCache.set(cacheKey, palette);
    return palette;
  }

  private paletteOrder(monPalette: Palette, hpPaletteIndex: number): PaletteOrder {
    const hpPalette = PALETTES_FOR_HP[hpPaletteIndex];
    const pageTint = STAT_PAGE_TINTS[this.page - 1];
    const tintedHp: Palette = [pageTint, hpPalette[1], hpPalette[2], hpPalette[3]];
    const tintedExp: Palette = [pageTint, EXP_PALETTE[1], EXP_PALETTE[2], EXP_PALETTE[3]];
    const [pink, green, blue] = PAGE_PALETTES;
    return [tintedHp, monPalette, tintedExp, pink, green, blue];
  }

  private statsTileVariants(paletteOrder: PaletteOrder): TilemapTileset {
    const cacheKey = JSON.stringify(paletteOrder);
    const cached = this.statTilesetCache.get(cacheKey);
    if (cached) {
      return cached;
    }
    const levels = this.loadStatTileLevels();
    const variants: TilemapTileset = {};
    for (const [tileIdStr, levelIndices] of Object.entries(levels)) {
      const tileId = Number(tileIdStr);
      const paletteMap: Record<number, Surface> = {};
      paletteOrder.forEach((palette, paletteIndex) => {
        if (palette.length !== 4) {
          throw new Error(`Stats screen palette ${paletteIndex} must have 4 entries, got ${palette.length}.`);
        }
        const surface = new Surface(TILE_SIZE, TILE_SIZE);
        levelIndices.forEach((colourIndex, idx) => {
          if (colourIndex >= palette.length) {
            throw new Error(`Palette index ${colourIndex} out of range for stats tile 0x${tileId.toString(16)}.`);
          }
          const x = idx % TILE_SIZE;
          const y = Math.floor(idx / TILE_SIZE);
          const colour = palette[colourIndex];
          surface.setAt(x, y, [colour[0], colour[1], colour[2], 255]);
        });
        paletteMap[paletteIndex] = surface;
      });
      variants[tileId] = paletteMap;
    }
    this.statTilesetCache.set(cacheKey, variants);
    return variants;
  }

  private loadStatTileLevels(): Record<number, number[]> {
    if (this.statTileLevels) {
      return this.statTileLevels;
    }
    if (!fs.existsSync(STATS_TILES_PATH)) {
      throw new Error(`Missing stats screen tiles at ${STATS_TILES_PATH}`);
    }
    const raw = fs.readFileSync(STATS_TILES_PATH);
    const levelIndices = decode2bppTileIndices(raw);
    if (levelIndices.length < STATS_TILE_COUNT) {
      throw new Error(
        `Expected at least ${STATS_TILE_COUNT} stats tiles, found ${levelIndices.length} in ${STATS_TILES_PATH}.`
      );
    }
    const levels: Record<number, number[]> = {};
    for (let offset = 0; offset < STATS_TILE_COUNT; offset++) {
      const tileId = STATS_TILE_BASE + offset;
      levels[tileId] = levelIndices[offset];
    }
    this.statTileLevels = levels;
    return levels;
  }

  private applyAttrmap(tilemap: TilemapSurface): void {
    tilemap.fillAttrRect(0, 8, tilemap.width, tilemap.height - 8, 0);
    tilemap.fillAttrRect(0, 0, tilemap.width, 8, TOP_HALF_ATTR);
    tilemap.fillAttrRect(10, 16, 10, 1, EXP_BAR_ATTR);
    tilemap.fillAttrRect(13, 5, 2, 2, PINK_ATTR);
    tilemap.fillAttrRect(15, 5, 2, 2, GREEN_ATTR);
    tilemap.fillAttrRect(17, 5, 2, 2, BLUE_ATTR);
  }

  private hpBarState(pokemon: Pokemon, maxHp: number, currentHp: number): [number[], number, number] {
    const [tiles, pixels] = this.hpBarTiles(pokemon, maxHp, currentHp);
    const paletteIndex = hpPaletteIndexFromPixels(pixels);
    return [tiles, pixels, paletteIndex];
  }

  private pokemonDisplayName(pokemon: Pokemon): string {
    const nickname = (pokemon.nickname ?? "").trim();
    return nickname.length ? nickname : pokemon.species.id;
  }

  private hpPaletteLabel(index: number): string {
    if (index === 0) {
      return "GREEN";
    }
    if (index === 1) {
      return "YELLOW";
    }
    return "RED";
  }

  private eggHatchLines(pokemon: Pokemon): string[] {
    const happiness = pokemon.happiness;
    if (happiness < 6) {
      return ["It's making sounds", "inside. It's going", "to hatch soon!", ""];
    }
    if (happiness < 11) {
      return ["It moves around", "inside sometimes.", "It must be close", "to hatching."];
    }
    if (happiness < 41) {
      return ["Wonder what's", "inside? It needs", "more time, though.", ""];
    }
    return ["This EGG needs a", "lot more time to", "hatch.", ""];
  }

  private eggHatchLine(pokemon: Pokemon, index: number): string {
    return this.eggHatchLines(pokemon)[index] ?? "";
  }
}

const gbToRgb = (value: number): number => gbc5To8(value);

const loadPaletteEntries = (path: string): [number, number, number][] => {
  const entries: [number, number, number][] = [];
  const lines = fs.readFileSync(path, "utf-8").split(/\r?\n/);
  for (const rawLine of lines) {
    const stripped = rawLine.split(";")[0].trim();
    if (!stripped || !stripped.toUpperCase().startsWith("RGB")) {
      continue;
    }
    const components = stripped.replace("RGB", "").replace(/,/g, " ").split(/\s+/).filter(Boolean);
    if (components.length !== 3) {
      throw new Error(`Palette entry '${stripped}' in ${path} does not contain 3 components.`);
    }
    const [r, g, b] = components.map((component) => Number(component));
    entries.push([gbToRgb(r), gbToRgb(g), gbToRgb(b)]);
  }
  return entries;
};

const decodeGbcpalPalette = (path: string): [number, number, number][] => {
  const data = fs.readFileSync(path);
  if (data.length !== 8) {
    throw new Error(`Palette ${path} must be exactly 8 bytes (4 colours), got ${data.length}.`);
  }
  const palette = [
    gbcWordToRgb(data.readUInt16LE(0)),
    gbcWordToRgb(data.readUInt16LE(2)),
    gbcWordToRgb(data.readUInt16LE(4)),
    gbcWordToRgb(data.readUInt16LE(6)),
  ];
  return [palette[1], palette[2]];
};

const statPageTints = (): [number, number, number][] => {
  const entries = loadPaletteEntries(STATS_PALETTE_PATH);
  if (entries.length !== NUM_STAT_PAGES) {
    throw new Error(`stats.pal should contain exactly ${NUM_STAT_PAGES} entries, found ${entries.length}.`);
  }
  return entries;
};

const chunkPages = (entries: [number, number, number][]): PaletteOrder => {
  if (entries.length % 4 !== 0) {
    throw new Error(`pages.pal must be divisible into 4-colour palettes; found ${entries.length} rows.`);
  }
  const palettes: Palette[] = [];
  for (let index = 0; index < entries.length; index += 4) {
    palettes.push(entries.slice(index, index + 4));
  }
  if (palettes.length !== 3) {
    throw new Error(`pages.pal should describe exactly 3 palettes, found ${palettes.length}.`);
  }
  return palettes;
};

const expPalette = (): Palette => {
  const entries = loadPaletteEntries(EXP_PALETTE_PATH);
  if (entries.length !== 2) {
    throw new Error(`exp_bar.pal should contain exactly two colours for the exp palette, found ${entries.length}.`);
  }
  return [[255, 255, 255], entries[0], entries[1], [0, 0, 0]];
};

const hpPalettes = (): Record<number, Palette> => {
  const palettes: Record<number, Palette> = {};
  Object.entries(HP_GB_PALETTES).forEach(([index, colours]) => {
    palettes[Number(index)] = colours.map(([r, g, b]) => [gbToRgb(r), gbToRgb(g), gbToRgb(b)]);
  });
  return palettes;
};

const decode2bppTileIndices = (data: Buffer): number[][] => {
  if (data.length % 16 !== 0) {
    throw new Error("2bpp payload must be aligned to 16-byte tiles");
  }
  const tiles: number[][] = [];
  const tileCount = data.length / 16;
  for (let tileIndex = 0; tileIndex < tileCount; tileIndex++) {
    const base = tileIndex * 16;
    const pixels: number[] = [];
    for (let row = 0; row < TILE_SIZE; row++) {
      const plane0 = data[base + row * 2];
      const plane1 = data[base + row * 2 + 1];
      for (let col = 0; col < TILE_SIZE; col++) {
        const bit = 7 - col;
        const idx = ((plane0 >> bit) & 1) | (((plane1 >> bit) & 1) << 1);
        pixels.push(idx);
      }
    }
    tiles.push(pixels);
  }
  return tiles;
};

const hpPaletteIndexFromPixels = (pixels: number): number => {
  if (pixels <= 0) {
    return 2;
  }
  if (pixels >= HP_GREEN_PIXEL_THRESHOLD) {
    return 0;
  }
  if (pixels >= HP_YELLOW_PIXEL_THRESHOLD) {
    return 1;
  }
  return 2;
};

const STAT_PAGE_TINTS = statPageTints();
const PAGE_PALETTES = chunkPages(loadPaletteEntries(PAGES_PALETTE_PATH));
const EXP_PALETTE = expPalette();
const PALETTES_FOR_HP = hpPalettes();
