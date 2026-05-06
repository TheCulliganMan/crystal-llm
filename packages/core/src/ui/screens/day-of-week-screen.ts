import { GameState } from "@pokecrystal/core/core/state";
import { SCREEN_TILE_WIDTH, TEXTBOX_HEIGHT_TILES, TEXTBOX_Y_TILES } from "@pokecrystal/core/core/text-constants";
import { resolveTextboxFrameRenderId } from "@pokecrystal/core/core/textbox-frame";
import type { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { TILE_SIZE } from "@pokecrystal/core/engine/world/tile";
import {
  mapKeyToDirection,
  mapKeyToButton,
  isConfirmEvent,
  isKeyDownEvent,
  isKeyUpEvent,
  type InputEventLike,
} from "@pokecrystal/core/input/controls";
import { GameButton } from "@pokecrystal/core/input/config";
import { nextFrame } from "@pokecrystal/core/ui/async-loop";
import { gameEngine, type GameEngineEventQueue } from "@pokecrystal/core/ui/game-engine";
import { YesNoPrompt, type DialogueUI } from "@pokecrystal/core/ui/text/dialogue";
import { Z_INDEX_DIALOGUE } from "@pokecrystal/core/ui/z-index";
import { Surface } from "@pokecrystal/core/ui/surface";
import type { BaseFontRenderer } from "@pokecrystal/core/ui/base-ui";
import type { RGB } from "@pokecrystal/core/ui/screens/screen-types";
import { buildPromptScreenSnapshot } from "./prompt-screen-snapshot";

// ASM reference: engine/menus/intro_menu.asm::SetDayOfWeek

export type DayOfWeekUI = {
  screen: Surface | null;
  tile_size?: number;
  tileSize?: number;
  default_frame_id?: number;
  defaultFrameId?: number;
  eventQueue?: GameEngineEventQueue | null;
  update?: () => void;
  drawTextBox?: (
    surface: Surface,
    text: string,
    x: number,
    y: number,
    width: number,
    height: number,
    frameId?: number,
    fill?: RGB,
    textColor?: RGB,
    zIndex?: number
  ) => void;
  draw_text_box?: (
    surface: Surface,
    text: string,
    x: number,
    y: number,
    width: number,
    height: number,
    frame_id?: number,
    fill?: RGB,
    text_color?: RGB,
    z_index?: number
  ) => void;
  drawWindow?: (
    surface: Surface,
    x: number,
    y: number,
    width: number,
    height: number,
    options?: { frameId?: number; fill?: RGB; zIndex?: number; record?: boolean }
  ) => void;
  draw_window?: (
    surface: Surface,
    x: number,
    y: number,
    width: number,
    height: number,
    options?: { frame_id?: number; fill?: RGB; z_index?: number; record?: boolean }
  ) => void;
  font?: BaseFontRenderer;
  _record_window_region?: (
    surface: Surface,
    x: number,
    y: number,
    width: number,
    height: number,
    zIndex: number,
    overlay?: Surface | null
  ) => void;
  renderSnapshot?: (
    viewportLines: string[],
    infoLines: string[],
    viewportTitle: string,
    infoTitle: string,
    menuLines?: string[] | null,
    promptLines?: string[] | null,
    dialogueLines?: string[] | null
  ) => void;
};

enum DayOfWeekPhase {
  SELECT_DAY,
  CONFIRM,
}

const DAY_STRINGS = [
  " SUNDAY",
  " MONDAY",
  " TUESDAY",
  "WEDNESDAY",
  "THURSDAY",
  " FRIDAY",
  "SATURDAY",
];

const getTileSize = (ui: DayOfWeekUI): number => {
  return ui.tile_size ?? ui.tileSize ?? TILE_SIZE;
};

const renderFontText = (
  ui: DayOfWeekUI,
  text: string,
  x: number,
  y: number,
  surface: Surface
): void => {
  if (ui.font?.render_text) {
    ui.font.render_text(text, x, y, surface, { uppercase: false });
    return;
  }
  if (ui.font?.renderText) {
    ui.font.renderText(text, x, y, surface, { uppercase: false });
    return;
  }
  throw new Error("DayOfWeekScreen requires a font renderer");
};

class TileRect {
  constructor(
    public readonly left: number,
    public readonly top: number,
    public readonly width: number,
    public readonly height: number
  ) {}

  topLeftPx(tileSize: number): [number, number] {
    return [this.left * tileSize, this.top * tileSize];
  }

  offsetPx(dx: number, dy: number, tileSize: number): [number, number] {
    return [(this.left + dx) * tileSize, (this.top + dy) * tileSize];
  }
}

export class DayOfWeekScreen {
  // ASM: engine/rtc/timeset.asm::SetDayOfWeek (hlcoord 0,12 + lb bc,4,18 textbox region).
  private static readonly QUESTION_BOX = new TileRect(0, TEXTBOX_Y_TILES, SCREEN_TILE_WIDTH, TEXTBOX_HEIGHT_TILES);
  private static readonly DAY_BOX = new TileRect(9, 3, 11, 4);
  private static readonly DAY_TEXT_OFFSET_Y = 2; // hlcoord 10, 5
  private static readonly ARROW_COLUMN_OFFSET = 5; // hlcoord 14, *

  private phase = DayOfWeekPhase.SELECT_DAY;
  private selectedDay: number;
  private ignoreConfirmUntilRelease = true;

  constructor(
    private readonly ui: DayOfWeekUI,
    private readonly gameState: GameState,
    private readonly audioEngine?: AudioEngine | null
  ) {
    // ASM initializes SetDayOfWeek from Sunday in wTempDayOfWeek.
    this.selectedDay = 0;
  }

  reset(): void {
    this.phase = DayOfWeekPhase.SELECT_DAY;
    this.ignoreConfirmUntilRelease = true;
  }

  isConfirming(): boolean {
    return this.phase === DayOfWeekPhase.CONFIRM;
  }

  getSelectedDay(): number {
    return this.selectedDay;
  }

  handleInput(event: InputEventLike): void {
    if (isKeyUpEvent(event) && mapKeyToButton(event.button ?? event.code ?? event.key ?? null) === GameButton.A) {
      this.ignoreConfirmUntilRelease = false;
      return;
    }
    if (!isKeyDownEvent(event)) {
      return;
    }

    if (this.phase === DayOfWeekPhase.SELECT_DAY) {
      this.handleDaySelection(event);
    }
  }

  draw(): void {
    if (!this.ui.screen) {
      return;
    }
    if (this.phase === DayOfWeekPhase.SELECT_DAY) {
      this.drawSelection();
    } else if (this.phase === DayOfWeekPhase.CONFIRM) {
      this.drawConfirmation();
    }
  }

  async runConfirmation({ drawCallback }: { drawCallback?: () => void } = {}): Promise<boolean> {
    const yesNo = new YesNoPrompt(this.ui as DialogueUI, this.audioEngine ?? undefined);
    while (!yesNo.finished) {
      for (const event of gameEngine.event.get(this.ui.eventQueue ?? undefined)) {
        if (event.type === gameEngine.QUIT) {
          gameEngine.quit();
          throw new Error("DayOfWeekScreen confirmation interrupted by quit event");
        }
        yesNo.handle_input(event);
      }
      if (drawCallback) {
        drawCallback();
      }
      this.drawConfirmation();
      yesNo.draw();
      this.ui.update?.();
      await nextFrame();
    }
    return yesNo.result();
  }

  private handleDaySelection(event: InputEventLike): void {
    const direction = mapKeyToDirection(event.direction ?? event.code ?? event.key ?? null);
    if (direction === "up") {
      this.selectedDay = (this.selectedDay + 1) % 7;
    } else if (direction === "down") {
      this.selectedDay = (this.selectedDay + 6) % 7;
    } else if (isConfirmEvent(event)) {
      if (this.ignoreConfirmUntilRelease) {
        this.ignoreConfirmUntilRelease = false;
        return;
      }
      this.phase = DayOfWeekPhase.CONFIRM;
    }
  }

  private drawSelection(): void {
    if (!this.ui.screen) {
      return;
    }
    const tileSize = getTileSize(this.ui);
    const frameId = this.resolveFrameId();
    this.drawPromptTextBox("What day is it?", frameId);

    const [dayX, dayY] = DayOfWeekScreen.DAY_BOX.topLeftPx(tileSize);
    this.drawWindow(dayX, dayY, DayOfWeekScreen.DAY_BOX.width, DayOfWeekScreen.DAY_BOX.height, {
      frameId,
      zIndex: Z_INDEX_DIALOGUE,
    });

    const [textX, textY] = DayOfWeekScreen.DAY_BOX.offsetPx(1, DayOfWeekScreen.DAY_TEXT_OFFSET_Y, tileSize);
    renderFontText(this.ui, DAY_STRINGS[this.selectedDay], textX, textY, this.ui.screen!);

    const [arrowX, arrowTop] = DayOfWeekScreen.DAY_BOX.offsetPx(
      DayOfWeekScreen.ARROW_COLUMN_OFFSET,
      0,
      tileSize
    );
    const [, arrowBottom] = DayOfWeekScreen.DAY_BOX.offsetPx(
      DayOfWeekScreen.ARROW_COLUMN_OFFSET,
      DayOfWeekScreen.DAY_BOX.height - 1,
      tileSize
    );
    renderFontText(this.ui, "▲", arrowX, arrowTop, this.ui.screen!);
    renderFontText(this.ui, "▼", arrowX, arrowBottom, this.ui.screen!);

    this.renderSelectionSnapshot();
  }

  private drawConfirmation(): void {
    const frameId = this.resolveFrameId();
    const confirmText = `${DAY_STRINGS[this.selectedDay]}, is it?`;
    this.drawPromptTextBox(confirmText, frameId);
    this.renderConfirmationSnapshot(confirmText);
  }

  private resolveFrameId(): number {
    return resolveTextboxFrameRenderId(
      this.gameState?.sram?.options?.frame,
      this.ui.default_frame_id ?? this.ui.defaultFrameId ?? 1
    );
  }

  private drawWindow(
    xPx: number,
    yPx: number,
    widthTiles: number,
    heightTiles: number,
    {
      frameId,
      zIndex,
    }: {
      frameId: number;
      zIndex: number;
    }
  ): void {
    if (this.ui.drawWindow) {
      this.ui.drawWindow(this.ui.screen!, xPx, yPx, widthTiles, heightTiles, {
        frameId,
        fill: [255, 255, 255],
        zIndex,
        record: true,
      });
      return;
    }
    if (this.ui.draw_window) {
      this.ui.draw_window(this.ui.screen!, xPx, yPx, widthTiles, heightTiles, {
        frame_id: frameId,
        fill: [255, 255, 255],
        z_index: zIndex,
        record: true,
      });
      return;
    }
    throw new Error("DayOfWeekScreen requires drawWindow/draw_window");
  }

  private drawPromptTextBox(text: string, frameId: number): void {
    const drawTextBox = this.ui.drawTextBox ?? this.ui.draw_text_box;
    if (!drawTextBox || !this.ui.screen) {
      throw new Error("DayOfWeekScreen requires drawTextBox/draw_text_box");
    }
    drawTextBox.call(
      this.ui,
      this.ui.screen,
      text,
      DayOfWeekScreen.QUESTION_BOX.left,
      DayOfWeekScreen.QUESTION_BOX.top,
      DayOfWeekScreen.QUESTION_BOX.width,
      DayOfWeekScreen.QUESTION_BOX.height,
      frameId,
      undefined,
      undefined,
      Z_INDEX_DIALOGUE
    );
  }

  private renderSelectionSnapshot(): void {
    if (!this.ui.renderSnapshot) {
      return;
    }
    const menuLines = DAY_STRINGS.map((label, index) => `${index === this.selectedDay ? "▶" : " "} ${label.trimStart()}`);
    const snapshot = buildPromptScreenSnapshot({
      infoLines: ["Up/Down=Choose A=OK"],
      menuLines,
      dialogueLines: ["What day is it?"],
    });
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

  private renderConfirmationSnapshot(confirmText: string): void {
    if (!this.ui.renderSnapshot) {
      return;
    }
    const snapshot = buildPromptScreenSnapshot({
      infoLines: ["A=OK B=Cancel"],
      dialogueLines: [confirmText],
    });
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
