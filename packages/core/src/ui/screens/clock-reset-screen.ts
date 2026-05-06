import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { GameState } from "@pokecrystal/core/core/state";
import { isCancelEvent, isConfirmEvent, isKeyDownEvent } from "@pokecrystal/core/input/controls";
import type { KeyEvent } from "@pokecrystal/core/input/buttons";
import { TimeSystem } from "@pokecrystal/core/engine/systems/time";
import { TILE_SIZE } from "@pokecrystal/core/engine/world/tile";
import { ScreenUI, isTextUI } from "@pokecrystal/core/ui/screens/screen-types";
import { renderFontText } from "@pokecrystal/core/ui/text/render-font";
import { buildPromptScreenSnapshot, buildPromptCursorLines, PROMPT_CONTROL_LINES } from "./prompt-screen-snapshot";

// ASM reference: Clock reset flow in engine/menus/intro_menu.asm.

enum ClockPhase {
  CONFIRM = "confirm",
  SET_DAY = "set_day",
  SET_HOUR = "set_hour",
  SET_MINUTE = "set_minute",
}

const DECREMENT_KEYS = new Set([gameEngine.K_LEFT, gameEngine.K_DOWN]);
const INCREMENT_KEYS = new Set([gameEngine.K_RIGHT, gameEngine.K_UP]);

const DAY_NAMES = ["SUN", "MON", "TUES", "WEDNES", "THURS", "FRI", "SATUR"] as const;

const Z_INDEX_DIALOGUE = 10;
const Z_INDEX_PROMPT = 20;

type PromptBox = {
  x: number;
  y: number;
  width: number;
  height: number;
};

const PROMPT_BOX: PromptBox = { x: 1, y: 6, width: 18, height: 4 };
// ASM: home/menu.asm::YesNoBox (lb bc, SCREEN_WIDTH - 6, 7).
const YES_NO_BOX: PromptBox = { x: 14, y: 7, width: 6, height: 4 };
const VALUE_BOX: PromptBox = { x: 11, y: 9, width: 6, height: 4 };

const isTextOnlyUi = (ui: ScreenUI): boolean =>
  isTextUI(ui) && typeof (ui as { getChildren?: () => unknown[] }).getChildren !== "function";

export class ClockResetScreen {
  private readonly screen: InstanceType<typeof gameEngine.Surface> | null;
  private readonly isTextOnlyUi: boolean;
  private phase = ClockPhase.CONFIRM;
  private confirmSelection = 1;
  private day: number;
  private hour: number;
  private minute: number;

  constructor(private readonly ui: ScreenUI, private readonly gameState: GameState) {
    this.screen = ui.screen;
    this.isTextOnlyUi = isTextOnlyUi(ui);
    this.day = gameState.sram.day_of_week % 7;
    this.hour = Math.max(0, Math.min(gameState.sram.game_time_hours, 23));
    this.minute = Math.max(0, Math.min(gameState.sram.game_time_minutes, 59));
  }

  reset(): void {
    this.phase = ClockPhase.CONFIRM;
    this.confirmSelection = 1;
  }

  handleInput(event: KeyEvent): "confirm" | "cancel" | null {
    if (!isKeyDownEvent(event)) {
      return null;
    }
    if (isCancelEvent(event)) {
      this.reset();
      return "cancel";
    }

    if (this.phase === ClockPhase.CONFIRM) {
      return this.handleConfirmInput(event);
    }
    if (this.phase === ClockPhase.SET_DAY) {
      return this.handleDayInput(event);
    }
    if (this.phase === ClockPhase.SET_HOUR) {
      return this.handleHourInput(event);
    }
    if (this.phase === ClockPhase.SET_MINUTE) {
      return this.handleMinuteInput(event);
    }
    return null;
  }

  draw(): void {
    if (this.isTextOnlyUi) {
      this.renderTextSnapshot();
      return;
    }
    if (!this.screen) {
      return;
    }
    this.screen.fill([0, 0, 0, 255]);

    if (this.phase === ClockPhase.CONFIRM) {
      this.drawConfirmPrompt();
    } else if (this.phase === ClockPhase.SET_DAY) {
      this.drawDayPrompt();
    } else if (this.phase === ClockPhase.SET_HOUR) {
      this.drawHourPrompt();
    } else if (this.phase === ClockPhase.SET_MINUTE) {
      this.drawMinutePrompt();
    }
    this.renderTextSnapshot();
  }

  private handleConfirmInput(event: KeyEvent): "confirm" | "cancel" | null {
    const key = event.code || event.key;
    if (typeof key === 'string' && (INCREMENT_KEYS.has(key) || DECREMENT_KEYS.has(key))) {
      this.confirmSelection = 1 - this.confirmSelection;
      return null;
    }
    if (isConfirmEvent(event)) {
      if (this.confirmSelection === 0) {
        this.phase = ClockPhase.SET_DAY;
        return null;
      }
      this.reset();
      return "cancel";
    }
    return null;
  }

  private handleDayInput(event: KeyEvent): null {
    const key = event.code || event.key;
    if (typeof key === 'string' && DECREMENT_KEYS.has(key)) {
      this.day = (this.day - 1 + DAY_NAMES.length) % DAY_NAMES.length;
      return null;
    }
    if (typeof key === 'string' && INCREMENT_KEYS.has(key)) {
      this.day = (this.day + 1) % DAY_NAMES.length;
      return null;
    }
    if (isConfirmEvent(event)) {
      this.phase = ClockPhase.SET_HOUR;
    }
    return null;
  }

  private handleHourInput(event: KeyEvent): null {
    const key = event.code || event.key;
    if (typeof key === 'string' && DECREMENT_KEYS.has(key)) {
      this.hour = (this.hour - 1 + 24) % 24;
      return null;
    }
    if (typeof key === 'string' && INCREMENT_KEYS.has(key)) {
      this.hour = (this.hour + 1) % 24;
      return null;
    }
    if (isConfirmEvent(event)) {
      this.phase = ClockPhase.SET_MINUTE;
    }
    return null;
  }

  private handleMinuteInput(event: KeyEvent): "confirm" | null {
    const key = event.code || event.key;
    if (typeof key === 'string' && DECREMENT_KEYS.has(key)) {
      this.minute = (this.minute - 1 + 60) % 60;
      return null;
    }
    if (typeof key === 'string' && INCREMENT_KEYS.has(key)) {
      this.minute = (this.minute + 1) % 60;
      return null;
    }
    if (isConfirmEvent(event)) {
      this.applyTime();
      this.reset();
      return "confirm";
    }
    return null;
  }

  private drawConfirmPrompt(): void {
    this.drawTextBox("Reset clock?");
    if (!this.ui.drawBox || !this.screen) {
      throw new Error("ClockResetScreen requires UI.drawBox to render.");
    }
    this.ui.drawBox(
      this.screen,
      YES_NO_BOX.x * TILE_SIZE,
      YES_NO_BOX.y * TILE_SIZE,
      YES_NO_BOX.width,
      YES_NO_BOX.height
    );
    const options = ["YES", "NO"];
    options.forEach((label, index) => {
      const prefix = index === this.confirmSelection ? "▶" : " ";
      renderFontText(
        this.ui.font,
        `${prefix}${label}`,
        (YES_NO_BOX.x + 1) * TILE_SIZE,
        (YES_NO_BOX.y + 1 + index) * TILE_SIZE,
        this.screen as InstanceType<typeof gameEngine.Surface>
      );
    });
    if (this.ui._recordWindowRegion) {
      this.ui._recordWindowRegion(
        this.screen,
        YES_NO_BOX.x * TILE_SIZE,
        YES_NO_BOX.y * TILE_SIZE,
        YES_NO_BOX.width,
        YES_NO_BOX.height,
        Z_INDEX_PROMPT
      );
    }
  }

  private drawDayPrompt(): void {
    this.drawTextBox("What day is it?");
    const value = `${DAY_NAMES[this.day]}DAY`;
    this.drawValueBox(value);
  }

  private drawHourPrompt(): void {
    this.drawTextBox("What hour is it?");
    this.drawValueBox(String(this.hour).padStart(2, "0"));
  }

  private drawMinutePrompt(): void {
    this.drawTextBox("What minute?");
    this.drawValueBox(String(this.minute).padStart(2, "0"));
  }

  private drawTextBox(text: string): void {
    if (!this.ui.drawTextBox || !this.screen) {
      throw new Error("ClockResetScreen requires UI.drawTextBox to render.");
    }
    this.ui.drawTextBox(
      this.screen,
      text,
      PROMPT_BOX.x,
      PROMPT_BOX.y,
      PROMPT_BOX.width,
      PROMPT_BOX.height,
      Z_INDEX_DIALOGUE
    );
  }

  private drawValueBox(value: string): void {
    if (!this.ui.drawBox || !this.screen) {
      throw new Error("ClockResetScreen requires UI.drawBox to render.");
    }
    this.ui.drawBox(
      this.screen,
      VALUE_BOX.x * TILE_SIZE,
      VALUE_BOX.y * TILE_SIZE,
      VALUE_BOX.width,
      VALUE_BOX.height
    );
    renderFontText(
      this.ui.font,
      value,
      (VALUE_BOX.x + 1) * TILE_SIZE,
      (VALUE_BOX.y + 1) * TILE_SIZE,
      this.screen
    );
    if (this.ui._recordWindowRegion) {
      this.ui._recordWindowRegion(
        this.screen,
        VALUE_BOX.x * TILE_SIZE,
        VALUE_BOX.y * TILE_SIZE,
        VALUE_BOX.width,
        VALUE_BOX.height,
        Z_INDEX_DIALOGUE
      );
    }
  }

  private applyTime(): void {
    new TimeSystem(this.gameState).setManualTime({
      day: this.day,
      hour: this.hour,
      minute: this.minute,
      second: 0,
    });
  }

  private renderTextSnapshot(): void {
    if (!this.isTextOnlyUi || !this.ui.renderSnapshot) {
      return;
    }
    let snapshot = buildPromptScreenSnapshot();

    if (this.phase === ClockPhase.CONFIRM) {
      snapshot = buildPromptScreenSnapshot({
        infoLines: PROMPT_CONTROL_LINES,
        dialogueLines: ["Reset clock?"],
        promptLines: buildPromptCursorLines(["YES", "NO"], this.confirmSelection),
      });
    } else if (this.phase === ClockPhase.SET_DAY) {
      snapshot = buildPromptScreenSnapshot({
        infoLines: ["Up/Down=Choose A=OK B=Cancel"],
        dialogueLines: ["What day is it?"],
        menuLines: DAY_NAMES.map(
          (name, idx) => `${idx === this.day ? "▶" : " "} ${name}DAY`
        ),
      });
    } else if (this.phase === ClockPhase.SET_HOUR) {
      snapshot = buildPromptScreenSnapshot({
        infoLines: ["Up/Down=Adjust A=OK B=Cancel"],
        dialogueLines: ["What hour is it?"],
        menuLines: [`▶ ${String(this.hour).padStart(2, "0")}`],
      });
    } else if (this.phase === ClockPhase.SET_MINUTE) {
      snapshot = buildPromptScreenSnapshot({
        infoLines: ["Up/Down=Adjust A=OK B=Cancel"],
        dialogueLines: ["What minute?"],
        menuLines: [`▶ ${String(this.minute).padStart(2, "0")}`],
      });
    }

    this.ui.renderSnapshot(
      snapshot.viewportLines,
      snapshot.infoLines,
      snapshot.viewportTitle,
      snapshot.infoTitle,
      snapshot.menuLines ?? null,
      snapshot.promptLines ?? null,
      snapshot.dialogueLines ?? null
    );
  }
}
