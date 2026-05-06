// ASM mapping: pokecrystal_disassembly/engine/menus/party_menu.asm (PartyMenuQuality jump table).
import { LV_GLYPH } from "@pokecrystal/assets/content/text-constants";
import { PlayerGender } from "@pokecrystal/core/core/enums";
import { PartyMenuQuality } from "@pokecrystal/core/core/enums/party-menu";
import { MoveName } from "@pokecrystal/core/core/enums/move";
import { Pokemon } from "@pokecrystal/core/core/models";
import { Evolution } from "@pokecrystal/core/engine/systems/evolution";
import { pokemonCanLearnTmhm } from "@pokecrystal/core/engine/systems/tmhm";
import { TILE_SIZE } from "@pokecrystal/core/engine/world/tile";
import {
  CANCEL_COLUMN,
  COMPAT_COLUMN,
  HP_BAR_COLUMN,
  HP_BAR_LENGTH_PX,
  HP_DIGITS_COLUMN,
  LEVEL_COLUMN,
  NAME_COLUMN,
  PartyEntry,
  PartyMenuTilemap,
  STATUS_COLUMN,
  STATUS_LABELS,
} from "./party-menu-layout";
import { buildDefaultCharMap } from "@pokecrystal/core/ui/text/glyph-map";

const HP_TILE_START = 0x60;
const HP_END_CAP_TILE = HP_TILE_START + 0x0b;
const HP_GREEN_PIXEL_THRESHOLD = Math.floor((HP_BAR_LENGTH_PX * 50) / 100);
const HP_YELLOW_PIXEL_THRESHOLD = Math.floor((HP_BAR_LENGTH_PX * 21) / 100);
export const CHAR_MAP = buildDefaultCharMap();
export const _CHAR_MAP = CHAR_MAP;

type PokemonLike = Pokemon & {
  max_hp?: number;
  gender?: PlayerGender | null;
};

type MenuLike = {
  gameState?: { wram: { wHPPals: number[]; wCurHPPal: number; wSGBPals: number } };
  game_state?: { wram: { wHPPals: number[]; wCurHPPal: number; wSGBPals: number } };
  _tmhm_move?: MoveName | null;
  _current_item?: string | null;
  _is_egg: (pokemon: PokemonLike) => boolean;
  _name_row_y: (rowIndex: number) => number;
  _status_row_y: (rowIndex: number) => number;
  _cancel_row_y: (entryCount: number) => number;
};

type PartyEntryLike = PartyEntry | null;

export class PartyMenuQualityRenderer {
  private readonly handlers: Map<
    PartyMenuQuality,
    (menu: MenuLike, tilemap: PartyMenuTilemap, entries: PartyEntryLike[]) => void
  > = new Map([
    [PartyMenuQuality.NICKNAMES, this.placePartyNicknames.bind(this)],
    [PartyMenuQuality.HP_BAR, this.placePartyHpBars.bind(this)],
    [PartyMenuQuality.HP_DIGITS, this.placePartyHpDigits.bind(this)],
    [PartyMenuQuality.LEVEL, this.placePartyLevels.bind(this)],
    [PartyMenuQuality.STATUS, this.placePartyStatus.bind(this)],
    [PartyMenuQuality.TMHM_COMPAT, this.placeTmhmCompatibility.bind(this)],
    [PartyMenuQuality.EVO_STONE_COMPAT, this.placeEvoStoneCompatibility.bind(this)],
    [PartyMenuQuality.GENDER, this.placeGenderLabels.bind(this)],
    [PartyMenuQuality.MOBILE_SELECTION, this.placeMobileSelection.bind(this)],
  ]);

  apply(
    menu: MenuLike,
    tilemap: PartyMenuTilemap,
    entries: PartyEntryLike[],
    qualities: Iterable<PartyMenuQuality>
  ): void {
    for (const quality of qualities) {
      const handler = this.handlers.get(quality);
      if (!handler) {
        continue;
      }
      handler(menu, tilemap, entries);
    }
  }

  placePartyNicknames(
    menu: MenuLike,
    tilemap: PartyMenuTilemap,
    entries: PartyEntryLike[]
  ): void {
    entries.forEach((entry, rowIndex) => {
      if (!entry || menu._is_egg(entry.pokemon)) {
        return;
      }
      const nickname = String(entry.pokemon.nickname ?? "").slice(0, 10);
      tilemap.writeText(NAME_COLUMN, menu._name_row_y(rowIndex), nickname.padEnd(10, " "), {
        maxLength: 10,
        pad: true,
        uppercase: false,
      });
    });
    if (entries.length) {
      const cancelRow = menu._cancel_row_y(entries.length);
      tilemap.writeText(CANCEL_COLUMN, cancelRow, "CANCEL", { maxLength: 10, pad: false });
    }
  }

  placePartyHpBars(
    menu: MenuLike,
    tilemap: PartyMenuTilemap,
    entries: PartyEntryLike[]
  ): void {
    const gameState = menu.gameState ?? menu.game_state;
    if (!gameState) {
      throw new Error("Party menu HP bar rendering requires game state.");
    }
    const wram = gameState.wram;
    wram.wSGBPals = 0;
    entries.forEach((entry, rowIndex) => {
      if (!entry || menu._is_egg(entry.pokemon)) {
        wram.wSGBPals += 1;
        return;
      }
      const row = menu._status_row_y(rowIndex);
      const [tiles, pixels] = this.hpBarTiles(entry.pokemon);
      tilemap.writeTiles(HP_BAR_COLUMN, row, tiles);
      const paletteIndex = PartyMenuQualityRenderer.hpPaletteIndexFromPixels(pixels);
      PartyMenuQualityRenderer.applyHpBarPalette(tilemap, row, paletteIndex);
      if (rowIndex < wram.wHPPals.length) {
        wram.wHPPals[rowIndex] = paletteIndex;
      }
      wram.wCurHPPal = paletteIndex;
      wram.wSGBPals += 1;
    });
  }

  placePartyHpDigits(
    menu: MenuLike,
    tilemap: PartyMenuTilemap,
    entries: PartyEntryLike[]
  ): void {
    entries.forEach((entry, rowIndex) => {
      if (!entry || menu._is_egg(entry.pokemon)) {
        return;
      }
      const pokemon = entry.pokemon as PokemonLike;
      const maxHp = Math.max(0, pokemon.max_hp ?? pokemon.hp ?? 0);
      const current = Math.max(0, Math.min(pokemon.hp, maxHp));
      const text = `${current.toString().padStart(3, " ")}/${maxHp
        .toString()
        .padStart(3, " ")}`;
      tilemap.writeText(HP_DIGITS_COLUMN, menu._name_row_y(rowIndex), text, {
        maxLength: 7,
        pad: false,
        uppercase: false,
      });
    });
  }

  placePartyLevels(
    menu: MenuLike,
    tilemap: PartyMenuTilemap,
    entries: PartyEntryLike[]
  ): void {
    entries.forEach((entry, rowIndex) => {
      if (!entry || menu._is_egg(entry.pokemon)) {
        return;
      }
      const level = Math.max(1, entry.pokemon.level);
      const text = level < 100 ? `${LV_GLYPH}${level.toString().padStart(2, " ")}` : String(level).padEnd(3, " ").slice(0, 3);
      tilemap.writeText(LEVEL_COLUMN, menu._status_row_y(rowIndex), text, {
        maxLength: 3,
        pad: true,
        uppercase: false,
      });
    });
  }

  placePartyStatus(
    menu: MenuLike,
    tilemap: PartyMenuTilemap,
    entries: PartyEntryLike[]
  ): void {
    entries.forEach((entry, rowIndex) => {
      if (!entry || menu._is_egg(entry.pokemon)) {
        return;
      }
      const statusLabel = PartyMenuQualityRenderer.statusLabel(entry.pokemon);
      tilemap.writeText(STATUS_COLUMN, menu._status_row_y(rowIndex), statusLabel, {
        maxLength: 3,
        pad: true,
      });
    });
  }

  placeTmhmCompatibility(
    menu: MenuLike,
    tilemap: PartyMenuTilemap,
    entries: PartyEntryLike[]
  ): void {
    if (!menu._tmhm_move) {
      throw new Error("TM/HM move was not set before rendering compatibility.");
    }
    entries.forEach((entry, rowIndex) => {
      if (!entry || menu._is_egg(entry.pokemon)) {
        return;
      }
      const label = PartyMenuQualityRenderer.tmhmLabel(menu, entry.pokemon);
      tilemap.writeText(COMPAT_COLUMN, menu._status_row_y(rowIndex), label, {
        maxLength: 8,
        pad: true,
      });
    });
  }

  placeEvoStoneCompatibility(
    menu: MenuLike,
    tilemap: PartyMenuTilemap,
    entries: PartyEntryLike[]
  ): void {
    if (!menu._current_item) {
      throw new Error("Evolution item was not provided to the party menu.");
    }
    entries.forEach((entry, rowIndex) => {
      if (!entry || menu._is_egg(entry.pokemon)) {
        return;
      }
      const label = this.evoStoneLabel(menu, entry.pokemon);
      tilemap.writeText(COMPAT_COLUMN, menu._status_row_y(rowIndex), label, {
        maxLength: 8,
        pad: true,
      });
    });
  }

  placeGenderLabels(
    menu: MenuLike,
    tilemap: PartyMenuTilemap,
    entries: PartyEntryLike[]
  ): void {
    entries.forEach((entry, rowIndex) => {
      if (!entry || menu._is_egg(entry.pokemon)) {
        return;
      }
      const label = PartyMenuQualityRenderer.genderLabel(
        (entry.pokemon as PokemonLike).gender ?? null
      );
      tilemap.writeText(COMPAT_COLUMN, menu._status_row_y(rowIndex), label, {
        maxLength: 8,
        pad: true,
      });
    });
  }

  placeMobileSelection(
    menu: MenuLike,
    tilemap: PartyMenuTilemap,
    entries: PartyEntryLike[]
  ): void {
    entries.forEach((_entry, rowIndex) => {
      const row = menu._status_row_y(rowIndex);
      tilemap.writeText(COMPAT_COLUMN, row, "", { maxLength: 8, pad: true });
    });
  }

  hpBarTiles(pokemon: PokemonLike): [number[], number] {
    const maxHp = Math.max(0, pokemon.max_hp ?? pokemon.hp ?? 0);
    const currentHp = Math.max(0, Math.min(pokemon.hp, maxHp));
    let pixels = 0;
    if (maxHp > 0 && currentHp > 0) {
      pixels = Math.floor((currentHp * HP_BAR_LENGTH_PX) / maxHp);
      if (pixels === 0) {
        pixels = 1;
      }
    }
    pixels = Math.min(HP_BAR_LENGTH_PX, pixels);
    const tiles = [HP_TILE_START, HP_TILE_START + 1];
    let remaining = pixels;
    for (let i = 0; i < 6; i += 1) {
      if (remaining >= TILE_SIZE) {
        tiles.push(HP_TILE_START + 0x0a);
        remaining -= TILE_SIZE;
      } else if (remaining > 0) {
        tiles.push(HP_TILE_START + 0x02 + remaining);
        remaining = 0;
      } else {
        tiles.push(HP_TILE_START + 0x02);
      }
    }
    tiles.push(HP_END_CAP_TILE);
    return [tiles, pixels];
  }

  static hpPaletteIndexFromPixels(pixels: number): number {
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
  }

  private static applyHpBarPalette(
    tilemap: PartyMenuTilemap,
    row: number,
    paletteIndex: number
  ): void {
    tilemap.fillAttrRect(HP_BAR_COLUMN, row, 8, 2, paletteIndex + 1);
  }

  private static statusLabel(pokemon: PokemonLike): string {
    if (pokemon.hp <= 0) {
      return "FNT";
    }
    const status = pokemon.status ?? "";
    return STATUS_LABELS[String(status)] ?? "OK";
  }

  private static tmhmLabel(menu: MenuLike, pokemon: PokemonLike): string {
    if (!menu._tmhm_move) {
      return "";
    }
    if (menu._is_egg(pokemon)) {
      return "NOT ABLE";
    }
    if (pokemonCanLearnTmhm(pokemon, menu._tmhm_move)) {
      return "ABLE";
    }
    return "NOT ABLE";
  }

  private evoStoneLabel(menu: MenuLike, pokemon: PokemonLike): string {
    if (!menu._current_item) {
      return "";
    }
    if (menu._is_egg(pokemon)) {
      return "NOT ABLE";
    }
    const evolution = new Evolution(pokemon, {
      current_item: menu._current_item,
      force_evolution: true,
    });
    const candidate = evolution.check_for_evolution();
    return candidate ? "ABLE" : "NOT ABLE";
  }

  private static genderLabel(gender: PlayerGender | null): string {
    if (gender === PlayerGender.MALE) {
      return "\u2642\u2026MALE";
    }
    if (gender === PlayerGender.FEMALE) {
      return "\u2640\u2026FEMALE";
    }
    return "\u2026UNKNOWN";
  }

  static canonicalizeItem(item: string | null | undefined): string | null {
    if (!item) {
      return null;
    }
    const token = String(item).trim().toUpperCase().replace(/[^0-9A-Z]+/g, "_");
    const normalized = token.replace(/^_+|_+$/g, "");
    return normalized.length ? normalized : null;
  }
}
