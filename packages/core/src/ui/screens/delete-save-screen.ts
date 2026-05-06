import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { buttonKeys, GameButton, isKeyDownEvent } from "@pokecrystal/core/input/controls";
import type { KeyEvent } from "@pokecrystal/core/input/buttons";
import { TILE_SIZE } from "@pokecrystal/core/engine/world/tile";
import { ScreenUI, isTextUI } from "@pokecrystal/core/ui/screens/screen-types";
import { renderFontText } from "@pokecrystal/core/ui/text/render-font";

type BoxDefinition = {
  x: number;
  y: number;
  width: number;
  height: number;
};

const PROMPT_BOX: BoxDefinition = { x: 1, y: 6, width: 18, height: 4 };
const OPTION_BOX: BoxDefinition = { x: 11, y: 9, width: 6, height: 4 };
const OPTIONS = ["YES", "NO"] as const;

const confirmKeys = new Set(buttonKeys("a"));
const cancelKeys = new Set(buttonKeys("b"));
const toggleKeys = new Set([
  gameEngine.K_UP,
  gameEngine.K_DOWN,
  gameEngine.K_LEFT,
  gameEngine.K_RIGHT,
]);

const isTextOnlyUi = (ui: ScreenUI): boolean =>
  isTextUI(ui) && typeof (ui as { getChildren?: () => unknown[] }).getChildren !== "function";

export class DeleteSaveScreen {
  private readonly screen: InstanceType<typeof gameEngine.Surface> | null;
  private readonly isTextOnlyUi: boolean;
  private selection = 1;

  constructor(private readonly ui: ScreenUI) {
    this.screen = ui.screen;
    this.isTextOnlyUi = isTextOnlyUi(ui);
  }

  reset(): void {
    this.selection = 1;
  }

  handleInput(event: KeyEvent): "confirm" | "cancel" | null {
    if (!isKeyDownEvent(event)) {
      return null;
    }
    const key = event.code || event.key;
    if (typeof key === 'string' && cancelKeys.has(key)) {
      return "cancel";
    }
    if (typeof key === 'string' && toggleKeys.has(key)) {
      this.selection = 1 - this.selection;
      return null;
    }
    if (typeof key === 'string' && confirmKeys.has(key)) {
      return this.selection === 0 ? "confirm" : "cancel";
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
    this.drawPrompt();
    this.drawOptions();
    this.renderTextSnapshot();
  }

  private drawPrompt(): void {
    if (!this.ui.drawTextBox || !this.screen) {
      throw new Error("DeleteSaveScreen requires UI.drawTextBox to render.");
    }
    this.ui.drawTextBox(
      this.screen,
      "Delete all saved data?",
      PROMPT_BOX.x,
      PROMPT_BOX.y,
      PROMPT_BOX.width,
      PROMPT_BOX.height
    );
  }

  private drawOptions(): void {
    if (!this.ui.drawBox || !this.screen) {
      throw new Error("DeleteSaveScreen requires UI.drawBox to render.");
    }
    this.ui.drawBox(
      this.screen,
      OPTION_BOX.x * TILE_SIZE,
      OPTION_BOX.y * TILE_SIZE,
      OPTION_BOX.width,
      OPTION_BOX.height
    );

    OPTIONS.forEach((label, index) => {
      const text = index === this.selection ? `▶${label}` : ` ${label}`;
      renderFontText(
        this.ui.font,
        text,
        (OPTION_BOX.x + 1) * TILE_SIZE,
        (OPTION_BOX.y + 1 + index) * TILE_SIZE,
        this.screen as InstanceType<typeof gameEngine.Surface>
      );
    });
  }

  private renderTextSnapshot(): void {
    if (!this.isTextOnlyUi || !this.ui.renderSnapshot) {
      return;
    }
    const promptLines = [
      this.selection === 0 ? "▶ YES" : "  YES",
      this.selection === 1 ? "▶ NO" : "  NO",
    ];
    const infoLines = [
      "STATE: delete_save",
      `SELECTION: ${OPTIONS[this.selection]}`,
    ];
    this.ui.renderSnapshot(
      ["DELETE SAVE"],
      infoLines,
      "Delete Save",
      "Delete Save",
      null,
      promptLines,
      ["Delete all saved data?"]
    );
  }
}
