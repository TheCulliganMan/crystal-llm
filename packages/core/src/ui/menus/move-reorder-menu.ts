// ASM mapping: pokecrystal_disassembly/engine/pokemon/mon_menu.asm
// (ManagePokemonMoves, MoveScreenLoop, SetUpMoveScreenBG, SetUpMoveList, PlaceMoveData).
import { MenuUI } from "./types";
import { Menu } from "./menu";
import { Move, Pokemon } from "@pokecrystal/core/core/models";
import { toPokemon } from "@pokecrystal/core/core/models/pokemon";
import { GameState } from "@pokecrystal/core/core/state";
import { loadAllMoves } from "@pokecrystal/core/core/data-loader";
import { asmMoveDescriptionsLoader } from "@pokecrystal/core/core/asm-move-descriptions-loader";
import { moveDisplayName } from "@pokecrystal/assets/content/move-names";
import type { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { KeyEvent, isKeyDownEvent } from "@pokecrystal/core/input/buttons";
import { mapKeyToDirection } from "@pokecrystal/core/input/controls";
import { LV_GLYPH } from "@pokecrystal/core/ui/text/constants";
import { renderFontText } from "@pokecrystal/core/ui/text/render-font";

const TOP_BOX_X_TILES = 0;
const TOP_BOX_Y_TILES = 1;
const TOP_BOX_WIDTH_TILES = 20;
const TOP_BOX_HEIGHT_TILES = 11;
const BOTTOM_BOX_X_TILES = 0;
const BOTTOM_BOX_Y_TILES = 11;
const BOTTOM_BOX_WIDTH_TILES = 20;
const BOTTOM_BOX_HEIGHT_TILES = 7;

const CURSOR_X_TILES = 1;
const CURSOR_Y_TILES = 3;
const MOVE_NAME_X_TILES = 2;
const MOVE_NAME_Y_TILES = 3;
const MOVE_PP_X_TILES = 10;
const MOVE_PP_Y_TILES = 4;
const MOVE_ROW_SPACING_TILES = 2;

const MOVE_TYPE_TOP_X_TILES = 0;
const MOVE_TYPE_TOP_Y_TILES = 10;
const MOVE_TYPE_BOTTOM_X_TILES = 0;
const MOVE_TYPE_BOTTOM_Y_TILES = 11;
const MOVE_TYPE_LABEL_X_TILES = 2;
const MOVE_TYPE_LABEL_Y_TILES = 12;
const MOVE_ATK_LABEL_X_TILES = 12;
const MOVE_ATK_LABEL_Y_TILES = 12;
const MOVE_POWER_X_TILES = 16;
const MOVE_POWER_Y_TILES = 12;
const MOVE_DESCRIPTION_X_TILES = 1;
const MOVE_DESCRIPTION_Y_TILES = 14;
const WHERE_PROMPT_X_TILES = 1;
const WHERE_PROMPT_Y_TILES = 12;

const CURSOR_GLYPH = "\u25b6";
const HOLLOW_CURSOR_GLYPH = "\u25b7";
const LEFT_ARROW_GLYPH = "\u25c0";
const RIGHT_ARROW_GLYPH = "\u25b6";
const MOVE_TYPE_TOP_LABEL = "┌─────┐";
const MOVE_TYPE_BOTTOM_LABEL = "│TYPE/└";
const MOVE_ATK_LABEL = "ATK/";
const MOVE_NO_POWER = "---";
const WHERE_PROMPT = "Where?";

const TYPE_SUFFIX = "_TYPE";
const typeDisplayName = (type: string): string => {
  const raw = String(type ?? "");
  const trimmed = raw.endsWith(TYPE_SUFFIX) ? raw.slice(0, -TYPE_SUFFIX.length) : raw;
  return trimmed.replace(/_/g, " ");
};

export class MoveReorderMenu {
  private pokemon: Pokemon | null = null;
  private menu: Menu | null = null;
  private moveIndices: number[] = [];
  private swapOrigin: number | null = null;
  private readonly moveData: Map<string, Move>;

  constructor(
    private readonly ui: MenuUI,
    private readonly gameState: GameState,
    private readonly audioEngine?: AudioEngine | null,
  ) {
    this.moveData = loadAllMoves();
  }

  showPokemon(pokemon: Pokemon): void {
    this.swapOrigin = null;
    const index = this.gameState.wram.wCurPartyMon;
    this.setCurrentPokemon(index, pokemon);
  }

  reset(): void {
    this.pokemon = null;
    this.menu = null;
    this.moveIndices = [];
    this.swapOrigin = null;
    this.gameState.wram.wMenuCursorY = 0;
  }

  update(): void {
    if (!this.menu) {
      return;
    }
    this.menu.update();
    this.gameState.wram.wMenuCursorY = this.menu.selectedOption + 1;
  }

  handleInput(event: KeyEvent): "exit" | null {
    if (!this.menu || !this.pokemon) {
      return "exit";
    }
    if (isKeyDownEvent(event)) {
      const direction = mapKeyToDirection(String(event.key ?? event.code ?? ""));
      if (direction === "left" || direction === "right") {
        if (this.swapOrigin === null) {
          this.cycleParty(direction);
        }
        return null;
      }
    }
    const selection = this.menu.handleInput(event);
    if (!selection) {
      return null;
    }
    if (selection === "CANCEL") {
      if (this.swapOrigin !== null) {
        this.menu.selectedOption = this.swapOrigin;
        this.swapOrigin = null;
        return null;
      }
      this.reset();
      return "exit";
    }
    const selectedIndex = this.menu.selectedOption;
    if (this.swapOrigin === null) {
      this.swapOrigin = selectedIndex;
      return null;
    }
    this.swapMoves(this.swapOrigin, selectedIndex);
    this.swapOrigin = null;
    this.playSwapSound();
    this.buildMenu(this.menu.selectedOption);
    return null;
  }

  draw(): void {
    if (!this.menu || !this.pokemon || !this.ui.screen) {
      return;
    }
    const screen = this.ui.screen;
    const tile = this.ui.tileSize;
    const textboxPalette = this.textboxPalette();
    const fillColor = textboxPalette?.[0] ?? ([255, 255, 255] as [number, number, number]);
    const renderText = (xTiles: number, yTiles: number, text: string, textWidthTiles?: number): void => {
      renderFontText(this.ui.font, text, xTiles * tile, yTiles * tile, screen, {
        palette: textboxPalette ?? undefined,
        textWidth: textWidthTiles ? textWidthTiles * tile : undefined,
        maxLines: 1,
      });
    };

    this.ui.drawWindow(
      screen,
      TOP_BOX_X_TILES * tile,
      TOP_BOX_Y_TILES * tile,
      TOP_BOX_WIDTH_TILES,
      TOP_BOX_HEIGHT_TILES,
      { fill: fillColor },
    );
    this.ui.drawWindow(
      screen,
      BOTTOM_BOX_X_TILES * tile,
      BOTTOM_BOX_Y_TILES * tile,
      BOTTOM_BOX_WIDTH_TILES,
      BOTTOM_BOX_HEIGHT_TILES,
      { fill: fillColor },
    );

    const moveNameWidthTiles = TOP_BOX_WIDTH_TILES - MOVE_NAME_X_TILES - 1;
    this.drawMoveList(renderText, moveNameWidthTiles);
    this.drawPartyArrows(renderText);
    this.drawHeader(renderText);

    if (this.swapOrigin !== null) {
      const descriptionWidthTiles = BOTTOM_BOX_WIDTH_TILES - 2;
      renderText(WHERE_PROMPT_X_TILES, WHERE_PROMPT_Y_TILES, WHERE_PROMPT, descriptionWidthTiles);
      return;
    }
    this.drawMoveDetails(renderText);
  }

  getMoveNames(): string[] {
    if (!this.pokemon) {
      return [];
    }
    return this.pokemon.moves
      .filter((move): move is NonNullable<typeof move> => Boolean(move))
      .map((move) => this.formatMoveName(String(move.name)));
  }

  getSelectionIndex(): number {
    return this.menu?.selectedOption ?? 0;
  }

  getSwapOrigin(): number | null {
    return this.swapOrigin;
  }

  getActivePokemon(): Pokemon | null {
    return this.pokemon;
  }

  private buildMenu(selectedOption: number | null = null): void {
    if (!this.pokemon) {
      return;
    }
    const options: string[] = [];
    this.moveIndices = [];
    this.pokemon.moves.forEach((move, index) => {
      if (!move) {
        return;
      }
      options.push(this.formatMoveName(String(move.name)));
      this.moveIndices.push(index);
    });
    const tile = this.ui.tileSize;
    this.menu = new Menu(
      this.ui,
      options,
      tile * MOVE_NAME_X_TILES,
      tile * MOVE_NAME_Y_TILES,
      TOP_BOX_WIDTH_TILES,
      TOP_BOX_HEIGHT_TILES,
      MOVE_ROW_SPACING_TILES,
      undefined,
      null,
      this.audioEngine ?? null,
    );
    if (selectedOption !== null) {
      const maxIndex = Math.max(0, options.length - 1);
      this.menu.selectedOption = Math.max(0, Math.min(selectedOption, maxIndex));
    }
  }

  private cycleParty(direction: "left" | "right"): void {
    const party = this.gameState.sram.party.pokemon;
    if (!party.length) {
      return;
    }
    const step = direction === "right" ? 1 : -1;
    let index = this.gameState.wram.wCurPartyMon;
    for (let attempts = 0; attempts < party.length; attempts += 1) {
      index = (index + step + party.length) % party.length;
      const entry = party[index];
      if (!entry) {
        continue;
      }
      const pokemon = toPokemon(entry);
      if (pokemon.species.id === "EGG") {
        continue;
      }
      this.setCurrentPokemon(index, pokemon);
      return;
    }
  }

  private setCurrentPokemon(index: number, pokemon: Pokemon): void {
    this.pokemon = pokemon;
    this.gameState.wram.wCurPartyMon = index;
    this.gameState.wram.wCurPartySpecies = String(pokemon.species?.id ?? "").toUpperCase();
    this.gameState.wram.wPartyMenuCursor = index + 1;
    this.buildMenu(0);
  }

  private swapMoves(originMenuIndex: number, targetMenuIndex: number): void {
    if (!this.pokemon) {
      return;
    }
    const originIndex = this.moveIndices[originMenuIndex];
    const targetIndex = this.moveIndices[targetMenuIndex];
    if (originIndex === undefined || targetIndex === undefined) {
      throw new Error("Move menu selection is out of range.");
    }
    const moves = this.pokemon.moves;
    const temp = moves[originIndex];
    moves[originIndex] = moves[targetIndex];
    moves[targetIndex] = temp;
  }

  private formatMoveName(name: string): string {
    return moveDisplayName(name);
  }

  private playSwapSound(): void {
    this.audioEngine?.playSound?.("SFX_SWITCH_POKEMON");
    this.audioEngine?.playSound?.("SFX_SWITCH_POKEMON");
  }

  private textboxPalette(): [number, number, number][] | null {
    const getter = this.ui.get_context_palette ?? this.ui.getContextPalette;
    if (!getter) {
      return null;
    }
    return getter.call(this.ui, "textbox");
  }

  private drawHeader(
    renderText: (xTiles: number, yTiles: number, text: string, textWidthTiles?: number) => void,
  ): void {
    if (!this.pokemon) {
      return;
    }
    const name = String(this.pokemon.nickname ?? "").trim();
    if (name) {
      renderText(5, 1, name, TOP_BOX_WIDTH_TILES - 6);
      const levelText = `${LV_GLYPH}${Math.max(1, Math.min(255, Math.trunc(this.pokemon.level)))}`;
      const levelX = 6 + name.length;
      renderText(levelX, 1, levelText);
    }
  }

  private drawMoveList(
    renderText: (xTiles: number, yTiles: number, text: string, textWidthTiles?: number) => void,
    moveNameWidthTiles: number,
  ): void {
    if (!this.menu || !this.pokemon) {
      return;
    }
    const cursorVisible = this.menu.cursorVisible;
    const selectedIndex = this.menu.selectedOption;
    const swapOrigin = this.swapOrigin;
    const moves = this.pokemon.moves
      .filter((move): move is NonNullable<typeof move> => Boolean(move))
      .map((move) => move);

    for (let index = 0; index < moves.length; index += 1) {
      const move = moves[index];
      const name = this.formatMoveName(String(move.name));
      const nameY = MOVE_NAME_Y_TILES + index * MOVE_ROW_SPACING_TILES;
      const ppY = MOVE_PP_Y_TILES + index * MOVE_ROW_SPACING_TILES;
      const shouldDrawCursor =
        cursorVisible && index === selectedIndex && (swapOrigin === null || swapOrigin !== index);
      if (shouldDrawCursor) {
        renderText(CURSOR_X_TILES, nameY, CURSOR_GLYPH, 1);
      } else if (index === selectedIndex && swapOrigin !== index) {
        renderText(CURSOR_X_TILES, nameY, " ", 1);
      }
      if (swapOrigin !== null && swapOrigin === index) {
        renderText(CURSOR_X_TILES, nameY, HOLLOW_CURSOR_GLYPH, 1);
      }
      renderText(MOVE_NAME_X_TILES, nameY, name, moveNameWidthTiles);
      const moveMeta = this.moveData.get(move.name);
      if (!moveMeta) {
        throw new Error(`Missing move metadata for ${move.name}.`);
      }
      const maxPp = Math.max(0, Math.min(moveMeta.pp ?? 0, 99));
      const currentPp = Math.max(0, Math.min(move.current_pp ?? 0, 99));
      const ppText = `PP ${String(currentPp).padStart(2, " ")}/${String(maxPp).padStart(2, " ")}`;
      renderText(MOVE_PP_X_TILES, ppY, ppText);
    }
  }

  private drawMoveDetails(
    renderText: (xTiles: number, yTiles: number, text: string, textWidthTiles?: number) => void,
  ): void {
    if (!this.menu || !this.pokemon) {
      return;
    }
    renderText(MOVE_TYPE_TOP_X_TILES, MOVE_TYPE_TOP_Y_TILES, MOVE_TYPE_TOP_LABEL);
    renderText(MOVE_TYPE_BOTTOM_X_TILES, MOVE_TYPE_BOTTOM_Y_TILES, MOVE_TYPE_BOTTOM_LABEL);
    renderText(MOVE_ATK_LABEL_X_TILES, MOVE_ATK_LABEL_Y_TILES, MOVE_ATK_LABEL);
    const selectedMove = this.resolveSelectedMove();
    if (!selectedMove) {
      renderText(MOVE_TYPE_LABEL_X_TILES, MOVE_TYPE_LABEL_Y_TILES, "----");
      renderText(MOVE_POWER_X_TILES, MOVE_POWER_Y_TILES, MOVE_NO_POWER);
      return;
    }
    const moveMeta = this.moveData.get(selectedMove.name);
    if (!moveMeta) {
      throw new Error(`Missing move metadata for ${selectedMove.name}.`);
    }
    const typeLabel = typeDisplayName(String(moveMeta.type ?? ""));
    renderText(MOVE_TYPE_LABEL_X_TILES, MOVE_TYPE_LABEL_Y_TILES, typeLabel);
    const power = Math.max(0, Math.min(moveMeta.power ?? 0, 255));
    const powerText = power < 2 ? MOVE_NO_POWER : String(power).padStart(3, " ");
    renderText(MOVE_POWER_X_TILES, MOVE_POWER_Y_TILES, powerText);
    const description = asmMoveDescriptionsLoader.get(selectedMove.name);
    const lines = description.split("\n").slice(0, 2);
    const descriptionWidthTiles = BOTTOM_BOX_WIDTH_TILES - 2;
    lines.forEach((line, index) => {
      renderText(
        MOVE_DESCRIPTION_X_TILES,
        MOVE_DESCRIPTION_Y_TILES + index,
        line,
        descriptionWidthTiles,
      );
    });
  }

  private resolveSelectedMove(): Pokemon["moves"][number] | null {
    if (!this.menu || !this.pokemon) {
      return null;
    }
    const selectedIndex = this.menu.selectedOption;
    const moveIndex = this.moveIndices[selectedIndex];
    if (moveIndex === undefined) {
      return null;
    }
    return this.pokemon.moves[moveIndex] ?? null;
  }

  private drawPartyArrows(
    renderText: (xTiles: number, yTiles: number, text: string, textWidthTiles?: number) => void,
  ): void {
    if (!this.pokemon) {
      return;
    }
    if (this.hasPreviousPokemon()) {
      renderText(16, 0, LEFT_ARROW_GLYPH, 1);
    }
    if (this.hasNextPokemon()) {
      renderText(18, 0, RIGHT_ARROW_GLYPH, 1);
    }
  }

  private hasPreviousPokemon(): boolean {
    const party = this.gameState.sram.party.pokemon;
    if (!party.length) {
      return false;
    }
    const current = this.gameState.wram.wCurPartyMon;
    for (let index = current - 1; index >= 0; index -= 1) {
      const entry = party[index];
      if (!entry) {
        continue;
      }
      const pokemon = toPokemon(entry);
      if (pokemon.species.id !== "EGG") {
        return true;
      }
    }
    return false;
  }

  private hasNextPokemon(): boolean {
    const party = this.gameState.sram.party.pokemon;
    if (!party.length) {
      return false;
    }
    const current = this.gameState.wram.wCurPartyMon;
    for (let index = current + 1; index < party.length; index += 1) {
      const entry = party[index];
      if (!entry) {
        continue;
      }
      const pokemon = toPokemon(entry);
      if (pokemon.species.id !== "EGG") {
        return true;
      }
    }
    return false;
  }
}
