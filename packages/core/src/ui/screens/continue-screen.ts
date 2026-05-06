import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { GameState } from "@pokecrystal/core/core/state";
import { countPokedexEntries } from "@pokecrystal/core/core/pokedex";
import { syncGameClock } from "@pokecrystal/core/engine/systems/time";
import { isCancelEvent, isConfirmEvent, isKeyDownEvent } from "@pokecrystal/core/input/controls";
import type { KeyEvent } from "@pokecrystal/core/input/buttons";
import { TILE_SIZE } from "@pokecrystal/core/engine/world/tile";
import { ScreenUI, isTextUI } from "@pokecrystal/core/ui/screens/screen-types";
import { renderFontText } from "@pokecrystal/core/ui/text/render-font";
import { buildContinueScreenControlLines } from "../control-lines";

// ASM reference: DisplayNormalContinueData in engine/menus/intro_menu.asm.

const DAY_STRINGS = ["SUN", "MON", "TUES", "WEDNES", "THURS", "FRI", "SATUR"] as const;

type TileBox = {
  left: number;
  top: number;
  width: number;
  height: number;
};

// ASM mapping: continue screen uses menu_coords 0, 0, 15, 9.
const BOX: TileBox = { left: 0, top: 0, width: 16, height: 10 };

const LABEL_POSITIONS: Record<string, [number, number]> = {
  PLAYER: [1, 2],
  BADGES: [1, 4],
  "#DEX": [1, 6],
  TIME: [1, 8],
};

const VALUE_POSITIONS: Record<string, [number, number]> = {
  PLAYER: [8, 2],
  BADGES: [13, 4],
  "#DEX": [12, 6],
  TIME: [9, 8],
};

const isTextOnlyUi = (ui: ScreenUI): boolean =>
  isTextUI(ui) && typeof (ui as { getChildren?: () => unknown[] }).getChildren !== "function";

export class ContinueScreen {
  private readonly screen: InstanceType<typeof gameEngine.Surface> | null;
  private readonly isTextOnlyUi: boolean;

  constructor(
    private readonly ui: ScreenUI,
    private readonly gameState: GameState,
    private readonly audioEngine: AudioEngine | null = null
  ) {
    this.screen = ui.screen;
    this.isTextOnlyUi = isTextOnlyUi(ui);
  }

  draw(): void {
    syncGameClock(this.gameState);
    if (this.isTextOnlyUi) {
      this.renderTextSnapshot();
      return;
    }
    if (!this.screen) {
      return;
    }
    if (!this.ui.clearScreen) {
      throw new Error("ContinueScreen requires UI.clearScreen to render.");
    }
    this.ui.clearScreen([255, 255, 255]);
    this.drawFrame();
    this.drawLabels();
    this.drawValues();
    this.renderTextSnapshot();
  }

  handleInput(event: KeyEvent): "confirm" | "cancel" | null {
    if (!isKeyDownEvent(event)) {
      return null;
    }
    if (isConfirmEvent(event)) {
      return "confirm";
    }
    if (isCancelEvent(event)) {
      return "cancel";
    }
    return null;
  }

  dayString(): string {
    const index = this.gameState.sram.day_of_week % DAY_STRINGS.length;
    return `${DAY_STRINGS[index]}DAY`;
  }

  private drawFrame(): void {
    if (!this.ui.drawBox || !this.screen) {
      throw new Error("ContinueScreen requires UI.drawBox to render.");
    }
    const boxX = BOX.left * TILE_SIZE;
    const boxY = BOX.top * TILE_SIZE;
    gameEngine.draw.rect(
      this.screen,
      [255, 255, 255, 255],
      new gameEngine.Rect(
        boxX,
        boxY,
        BOX.width * TILE_SIZE,
        BOX.height * TILE_SIZE
      )
    );
    this.ui.drawBox(this.screen, boxX, boxY, BOX.width, BOX.height);
  }

  private drawLabels(): void {
    if (!this.screen) {
      return;
    }
    for (const [label, [dx, dy]] of Object.entries(LABEL_POSITIONS)) {
      if (label === "#DEX" && !this.hasPokedex()) {
        continue;
      }
      const [x, y] = this.tileToPixel(dx, dy);
      renderFontText(this.ui.font, label, x, y, this.screen, true);
    }
  }

  private drawValues(): void {
    if (!this.screen) {
      return;
    }
    const playerName = this.playerName();
    const [nameX, nameY] = this.tileToPixel(...VALUE_POSITIONS.PLAYER);
    renderFontText(this.ui.font, playerName, nameX, nameY, this.screen, true);

    const badgeCount = `${this.badgeCount()}`.padStart(2, " ");
    const [badgeX, badgeY] = this.tileToPixel(...VALUE_POSITIONS.BADGES);
    renderFontText(this.ui.font, badgeCount, badgeX, badgeY, this.screen, true);

    if (this.hasPokedex()) {
      const dexCount = `${this.pokedexCount()}`.padStart(3, " ");
      const [dexX, dexY] = this.tileToPixel(...VALUE_POSITIONS["#DEX"]);
      renderFontText(this.ui.font, dexCount, dexX, dexY, this.screen, true);
    }

    const timeString = this.formatTime();
    const [timeX, timeY] = this.tileToPixel(...VALUE_POSITIONS.TIME);
    renderFontText(this.ui.font, timeString, timeX, timeY, this.screen, true);
  }

  private tileToPixel(dx: number, dy: number): [number, number] {
    return [(BOX.left + dx) * TILE_SIZE, (BOX.top + dy) * TILE_SIZE];
  }

  private playerName(): string {
    const name = this.gameState.sram.player_name.trim();
    return name ? name : "PLAYER";
  }

  private badgeCount(): number {
    const johto = this.gameState.sram.badges.johto.filter(Boolean).length;
    const kanto = this.gameState.sram.badges.kanto.filter(Boolean).length;
    return johto + kanto;
  }

  private hasPokedex(): boolean {
    return Boolean(
      this.gameState.sram.johto_pokedex ||
        countPokedexEntries(this.gameState.sram.pokedex_seen) ||
        countPokedexEntries(this.gameState.sram.pokedex_owned)
    );
  }

  private pokedexCount(): number {
    return countPokedexEntries(this.gameState.sram.pokedex_owned);
  }

  private formatTime(): string {
    const hours = Math.max(0, Math.min(this.gameState.sram.game_time_hours, 999));
    const minutes = Math.max(0, Math.min(this.gameState.sram.game_time_minutes, 59));
    return `${String(hours).padStart(3, " ")}:${String(minutes).padStart(2, "0")}`;
  }

  private renderTextSnapshot(): void {
    if (!this.isTextOnlyUi || !this.ui.renderSnapshot) {
      return;
    }
    const playerName = this.playerName();
    const badgeCount = this.badgeCount();
    const hasDex = this.hasPokedex();
    const infoLines = [
      "STATE: continue_screen",
      `PLAYER: ${playerName}`,
      `BADGES: ${badgeCount}`,
      `POKEDEX: ${hasDex ? this.pokedexCount() : "---"}`,
      `TIME: ${this.formatTime()}`,
      `DAY: ${this.dayString()}`,
      ...buildContinueScreenControlLines(),
    ];
    this.ui.renderSnapshot(
      ["CONTINUE"],
      infoLines,
      "Continue",
      "Continue",
      null,
      null,
      null
    );
  }
}
