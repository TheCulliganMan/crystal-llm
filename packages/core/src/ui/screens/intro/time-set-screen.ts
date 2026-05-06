import { GameState } from "@pokecrystal/core/core/state";
import { SCREEN_TILE_WIDTH, TEXTBOX_HEIGHT_TILES, TEXTBOX_Y_TILES } from "@pokecrystal/core/core/text-constants";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { TimeSystem, DAY_HOUR, MORN_HOUR, NITE_HOUR } from "@pokecrystal/core/engine/systems/time";
import {
  isConfirmEvent,
  isCancelEvent,
  mapKeyToDirection,
  isButtonEvent,
  isKeyDownEvent,
} from "@pokecrystal/core/input/controls";
import type { InputEventLike } from "@pokecrystal/core/input/controls";
import { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import type { FontRenderer as TextboxFontRenderer } from "@pokecrystal/core/ui/textbox";
import {
  TILE_SIZE,
  fillScreen,
} from "./rendering";
import { BootTextboxRenderer } from "./boot-textbox-renderer";
import type { TextSnapshotPayload } from "../../text-overlays";
import {
  buildPromptCursorLines,
  buildPromptScreenSnapshot,
} from "../prompt-screen-snapshot";

// ASM reference: engine/menus/intro_menu.asm::InitClock

type Surface = InstanceType<typeof gameEngine.Surface>;

enum TimeSetPhase {
  WAKE_DIALOGUE = "wake_dialogue",
  SET_HOUR = "set_hour",
  HOUR_CONFIRM = "hour_confirm",
  SET_MINUTE = "set_minute",
  MINUTE_CONFIRM = "minute_confirm",
  FINAL_REACTION = "final_reaction",
  COMPLETE = "complete",
}

class CanvasDialogue {
  constructor(private readonly textboxRenderer: BootTextboxRenderer) {}

  private text = "";
  private visibleChars = 0;
  private timer = 0;
  private readonly speedFrames = 2;

  open(text: string): void {
    this.text = text;
    this.visibleChars = 0;
    this.timer = 0;
  }

  clear(): void {
    this.text = "";
    this.visibleChars = 0;
    this.timer = 0;
  }

  complete(): void {
    this.visibleChars = this.text.length;
  }

  isComplete(): boolean {
    return this.visibleChars >= this.text.length;
  }

  getVisibleText(): string {
    return this.text.slice(0, this.visibleChars);
  }

  update(): void {
    if (this.visibleChars >= this.text.length) {
      return;
    }
    this.timer += 1;
    if (this.timer >= this.speedFrames) {
      this.timer = 0;
      this.visibleChars = Math.min(this.text.length, this.visibleChars + 1);
    }
  }

  draw(ctx: CanvasRenderingContext2D): void {
    if (!this.text) {
      return;
    }
    this.textboxRenderer.drawTextBox(
      ctx,
      this.text.slice(0, this.visibleChars),
      0,
      TEXTBOX_Y_TILES,
      SCREEN_TILE_WIDTH,
      TEXTBOX_HEIGHT_TILES
    );
  }
}

class CanvasYesNoPrompt {
  private selection = 0;
  private finished = false;

  constructor(
    private readonly textboxRenderer: BootTextboxRenderer,
    private readonly audioEngine?: AudioEngine
  ) {}

  get isFinished(): boolean {
    return this.finished;
  }

  handleInput(event: InputEventLike): void {
    if (!isKeyDownEvent(event)) {
      return;
    }
    const direction = mapKeyToDirection(event.direction ?? event.key ?? event.code ?? null);
    if (direction) {
      this.selection = 1 - this.selection;
      return;
    }
    if (isConfirmEvent(event)) {
      this.finished = true;
      this.playConfirm();
    } else if (isCancelEvent(event)) {
      this.selection = 1;
      this.finished = true;
      this.playConfirm();
    }
  }

  draw(ctx: CanvasRenderingContext2D): void {
    // ASM: home/menu.asm::YesNoBox (lb bc, SCREEN_WIDTH - 6, 7).
    const boxX = 14;
    const boxY = 7;
    const boxWidth = 6;
    const boxHeight = 4;
    this.textboxRenderer.drawWindow(ctx, boxX, boxY, boxWidth, boxHeight);
    const baseX = (boxX + 1) * TILE_SIZE;
    const baseY = (boxY + 1) * TILE_SIZE;
    const labels = ["YES", "NO"];
    labels.forEach((label, index) => {
      const cursor = index === this.selection ? "\u25b6" : " ";
      this.textboxRenderer.drawText(ctx, `${cursor}${label}`, baseX, baseY + index * TILE_SIZE);
    });
  }

  result(): boolean {
    return this.selection === 0;
  }

  getMenuLines(): string[] {
    return ["YES", "NO"].map((label, index) => `${index === this.selection ? "▶" : " "} ${label}`);
  }
  private playConfirm(): void {
    this.audioEngine?.playSound?.("menu_option");
  }
}

export class TimeSetScreen {
  // ASM: engine/rtc/timeset.asm::InitClock. `Textbox` uses inner dimensions,
  // so the rendered windows are two tiles larger in each axis.
  private static readonly HOUR_BOX = { x: 3, y: 7, width: 17, height: 4 };
  private static readonly MINUTE_BOX = { x: 11, y: 7, width: 9, height: 4 };
  private static readonly HOUR_TEXT_X = 4;
  private static readonly HOUR_TEXT_Y = 9;
  private static readonly MINUTE_TEXT_X = 12;
  private static readonly MINUTE_TEXT_Y = 9;
  private static readonly HOUR_ARROW_X = 11;
  private static readonly MINUTE_ARROW_X = 15;
  private static readonly ARROW_TOP_Y = 7;
  private static readonly ARROW_BOTTOM_Y = 10;

  private static readonly WAKE_TEXT = [
    "...... ...... ...... ...... ...... ......",
    "...... ...... ...... ...... ...... ......",
    "Zzz... Hm? Wha... ?\nYou woke me up!",
    "Will you check the\nclock for me?",
  ];

  private static readonly QUESTION_HOUR = "What time is it?";
  private static readonly QUESTION_MINUTE = "How many minutes?";

  private phase: TimeSetPhase = TimeSetPhase.WAKE_DIALOGUE;
  private wakeIndex = 0;
  private hour = 10;
  private minute = 0;
  private inputCooldown = 0;
  private finished = false;
  private reactionText = "";
  private readonly textboxRenderer: BootTextboxRenderer;
  private readonly upArrow: Surface | null;
  private readonly downArrow: Surface | null;

  private readonly dialogue: CanvasDialogue;
  private yesNo: CanvasYesNoPrompt | null = null;

  constructor(
    private readonly gameState: GameState,
    private readonly audioEngine?: AudioEngine,
    font?: TextboxFontRenderer
  ) {
    if (!font) {
      throw new Error("TimeSetScreen requires the shared textbox font renderer.");
    }
    this.textboxRenderer = new BootTextboxRenderer(font, TILE_SIZE);
    this.upArrow = this.loadArrowSurface("up_arrow.png");
    this.downArrow = this.loadArrowSurface("down_arrow.png");
    this.dialogue = new CanvasDialogue(this.textboxRenderer);
    this.reset();
  }

  reset(): void {
    this.phase = TimeSetPhase.WAKE_DIALOGUE;
    this.wakeIndex = 0;
    this.hour = 10;
    this.minute = 0;
    this.inputCooldown = 0;
    this.finished = false;
    this.reactionText = "";
    this.dialogue.open(TimeSetScreen.WAKE_TEXT[this.wakeIndex]);
    this.yesNo = null;
  }

  isFinished(): boolean {
    return this.finished;
  }

  getPhase(): TimeSetPhase {
    return this.phase;
  }

  update(): void {
    if (
      this.phase === TimeSetPhase.WAKE_DIALOGUE ||
      this.phase === TimeSetPhase.HOUR_CONFIRM ||
      this.phase === TimeSetPhase.MINUTE_CONFIRM ||
      this.phase === TimeSetPhase.FINAL_REACTION
    ) {
      this.dialogue.update();
    }
    if (this.inputCooldown > 0) {
      this.inputCooldown -= 1;
    }
  }

  handleInput(event: InputEventLike): void {
    if (this.finished || !isKeyDownEvent(event)) {
      return;
    }

    switch (this.phase) {
      case TimeSetPhase.WAKE_DIALOGUE:
        this.handleWakeDialogue(event);
        break;
      case TimeSetPhase.SET_HOUR:
        this.handleHourInput(event);
        break;
      case TimeSetPhase.HOUR_CONFIRM:
        this.handleHourConfirm(event);
        break;
      case TimeSetPhase.SET_MINUTE:
        this.handleMinuteInput(event);
        break;
      case TimeSetPhase.MINUTE_CONFIRM:
        this.handleMinuteConfirm(event);
        break;
      case TimeSetPhase.FINAL_REACTION:
        this.handleFinalReaction(event);
        break;
      default:
        break;
    }
  }

  draw(ctx: CanvasRenderingContext2D): void {
    fillScreen(ctx, [248, 248, 248]);

    switch (this.phase) {
      case TimeSetPhase.SET_HOUR:
        this.textboxRenderer.drawTextBox(
          ctx,
          TimeSetScreen.QUESTION_HOUR,
          0,
          TEXTBOX_Y_TILES,
          SCREEN_TILE_WIDTH,
          TEXTBOX_HEIGHT_TILES
        );
        this.drawHourBox(ctx);
        break;
      case TimeSetPhase.SET_MINUTE:
        this.textboxRenderer.drawTextBox(
          ctx,
          TimeSetScreen.QUESTION_MINUTE,
          0,
          TEXTBOX_Y_TILES,
          SCREEN_TILE_WIDTH,
          TEXTBOX_HEIGHT_TILES
        );
        this.drawMinuteBox(ctx);
        break;
      case TimeSetPhase.HOUR_CONFIRM:
      case TimeSetPhase.MINUTE_CONFIRM:
      case TimeSetPhase.FINAL_REACTION:
      case TimeSetPhase.WAKE_DIALOGUE:
        this.dialogue.draw(ctx);
        if (this.yesNo) {
          this.yesNo.draw(ctx);
        }
        break;
      default:
        break;
    }
  }

  private handleWakeDialogue(event: InputEventLike): void {
    if (!this.dialogue.isComplete()) {
      if (isConfirmEvent(event) || isCancelEvent(event)) {
        this.dialogue.complete();
      }
      return;
    }

    if (isConfirmEvent(event) || isCancelEvent(event)) {
      if (this.wakeIndex < TimeSetScreen.WAKE_TEXT.length - 1) {
        this.wakeIndex += 1;
        this.dialogue.open(TimeSetScreen.WAKE_TEXT[this.wakeIndex]);
        return;
      }
      this.phase = TimeSetPhase.SET_HOUR;
      this.dialogue.clear();
      this.inputCooldown = 0;
    }
  }

  private handleHourInput(event: InputEventLike): void {
    if (this.inputCooldown > 0) {
      return;
    }

    const direction = mapKeyToDirection(event.direction ?? event.key ?? event.code ?? null);
    if (direction === "up") {
      this.hour = (this.hour + 1) % 24;
    } else if (direction === "down") {
      this.hour = (this.hour + 23) % 24;
    } else if (isConfirmEvent(event)) {
      this.phase = TimeSetPhase.HOUR_CONFIRM;
      this.dialogue.open(this.buildHourConfirmText());
      this.yesNo = new CanvasYesNoPrompt(this.textboxRenderer, this.audioEngine);
    } else if (isButtonEvent(event, "b")) {
      this.phase = TimeSetPhase.WAKE_DIALOGUE;
      this.wakeIndex = TimeSetScreen.WAKE_TEXT.length - 1;
      this.dialogue.open(TimeSetScreen.WAKE_TEXT[this.wakeIndex]);
    }
  }

  private handleHourConfirm(event: InputEventLike): void {
    if (!this.yesNo) {
      return;
    }
    this.yesNo.handleInput(event);
    if (!this.yesNo.isFinished) {
      return;
    }
    if (this.yesNo.result()) {
      this.phase = TimeSetPhase.SET_MINUTE;
      this.dialogue.clear();
      this.yesNo = null;
      this.inputCooldown = 0;
    } else {
      this.phase = TimeSetPhase.SET_HOUR;
      this.dialogue.clear();
      this.yesNo = null;
      this.inputCooldown = 0;
    }
  }

  private handleMinuteInput(event: InputEventLike): void {
    if (this.inputCooldown > 0) {
      return;
    }

    const direction = mapKeyToDirection(event.direction ?? event.key ?? event.code ?? null);
    if (direction === "up" || direction === "right") {
      this.minute = (this.minute + 1) % 60;
    } else if (direction === "down" || direction === "left") {
      this.minute = (this.minute + 59) % 60;
    } else if (isConfirmEvent(event)) {
      this.phase = TimeSetPhase.MINUTE_CONFIRM;
      this.dialogue.open(this.buildMinuteConfirmText());
      this.yesNo = new CanvasYesNoPrompt(this.textboxRenderer, this.audioEngine);
    } else if (isButtonEvent(event, "b")) {
      this.phase = TimeSetPhase.SET_HOUR;
      this.yesNo = null;
      this.dialogue.clear();
      this.inputCooldown = 0;
    }
  }

  private handleMinuteConfirm(event: InputEventLike): void {
    if (!this.yesNo) {
      return;
    }
    this.yesNo.handleInput(event);
    if (!this.yesNo.isFinished) {
      return;
    }
    if (this.yesNo.result()) {
      this.applySelectedTime();
      this.phase = TimeSetPhase.FINAL_REACTION;
      this.dialogue.open(this.reactionText);
      this.yesNo = null;
    } else {
      this.phase = TimeSetPhase.SET_MINUTE;
      this.dialogue.clear();
      this.yesNo = null;
      this.inputCooldown = 0;
    }
  }

  private handleFinalReaction(event: InputEventLike): void {
    if (!this.dialogue.isComplete()) {
      if (isConfirmEvent(event) || isCancelEvent(event)) {
        this.dialogue.complete();
      }
      return;
    }
    if (isConfirmEvent(event) || isCancelEvent(event)) {
      this.phase = TimeSetPhase.COMPLETE;
      this.dialogue.clear();
      this.finished = true;
    }
  }

  private drawHourBox(ctx: CanvasRenderingContext2D): void {
    this.textboxRenderer.drawWindow(
      ctx,
      TimeSetScreen.HOUR_BOX.x,
      TimeSetScreen.HOUR_BOX.y,
      TimeSetScreen.HOUR_BOX.width,
      TimeSetScreen.HOUR_BOX.height
    );
    this.drawArrows(ctx, TimeSetScreen.HOUR_ARROW_X);
    const text = this.buildHourDisplay();
    this.textboxRenderer.drawText(
      ctx,
      text,
      TimeSetScreen.HOUR_TEXT_X * TILE_SIZE,
      TimeSetScreen.HOUR_TEXT_Y * TILE_SIZE
    );
  }

  private drawMinuteBox(ctx: CanvasRenderingContext2D): void {
    this.textboxRenderer.drawWindow(
      ctx,
      TimeSetScreen.MINUTE_BOX.x,
      TimeSetScreen.MINUTE_BOX.y,
      TimeSetScreen.MINUTE_BOX.width,
      TimeSetScreen.MINUTE_BOX.height
    );
    this.drawArrows(ctx, TimeSetScreen.MINUTE_ARROW_X);
    const text = this.buildMinuteDisplay();
    this.textboxRenderer.drawText(
      ctx,
      text,
      TimeSetScreen.MINUTE_TEXT_X * TILE_SIZE,
      TimeSetScreen.MINUTE_TEXT_Y * TILE_SIZE
    );
  }

  private drawArrows(ctx: CanvasRenderingContext2D, arrowTileX: number): void {
    const xPx = arrowTileX * TILE_SIZE;
    const topYPx = TimeSetScreen.ARROW_TOP_Y * TILE_SIZE;
    const bottomYPx = TimeSetScreen.ARROW_BOTTOM_Y * TILE_SIZE;
    if (this.upArrow) {
      this.textboxRenderer.drawPromptArrow(ctx, this.upArrow, xPx, topYPx);
    } else {
      this.textboxRenderer.drawText(ctx, "\u25b2", xPx, topYPx);
    }
    if (this.downArrow) {
      this.textboxRenderer.drawPromptArrow(ctx, this.downArrow, xPx, bottomYPx);
    } else {
      this.textboxRenderer.drawText(ctx, "\u25bc", xPx, bottomYPx);
    }
  }

  private loadArrowSurface(filename: "up_arrow.png" | "down_arrow.png"): Surface | null {
    const loader = gameEngine.image.loadSync;
    if (typeof loader !== "function") {
      return null;
    }
    return loader(getAssetPath("gfx", "new_game", filename));
  }

  private buildHourDisplay(): string {
    const period = this.timeOfDayString(this.hour);
    const hour = this.twelveHour(this.hour);
    return `${period} ${hour.toString().padStart(2, " ")} o'clock`;
  }

  private buildHourConfirmText(): string {
    return `What?\n${this.buildHourDisplay()}?`;
  }

  private buildMinuteDisplay(): string {
    return `${this.minute.toString().padStart(2, " ")} min.`;
  }

  private buildMinuteConfirmText(): string {
    return `Whoa!\n${this.buildMinuteDisplay()}?`;
  }

  private buildReactionText(): string {
    const period = this.timeOfDayString(this.hour);
    const hour = this.twelveHour(this.hour);
    const timeLine = `${period} ${hour.toString().padStart(2, " ")}:${this.minute
      .toString()
      .padStart(2, "0")}`;
    const reaction = this.reactionSuffix(this.hour);
    return `${timeLine}\n${reaction}`;
  }

  private timeOfDayString(hour: number): string {
    if (hour < MORN_HOUR) {
      return "NITE";
    }
    if (hour < DAY_HOUR) {
      return "MORN";
    }
    if (hour < NITE_HOUR) {
      return "DAY";
    }
    return "NITE";
  }

  private twelveHour(hour: number): number {
    const h = hour % 24;
    if (h === 0) {
      return 12;
    }
    if (h > 12) {
      return h - 12;
    }
    return h;
  }

  private reactionSuffix(hour: number): string {
    if (hour < MORN_HOUR) {
      return "!\nNo wonder it's so\ndark!";
    }
    if (hour <= DAY_HOUR) {
      return "!\nI overslept!";
    }
    if (hour < NITE_HOUR) {
      return "!\nYikes! I over-\nslept!";
    }
    return "!\nNo wonder it's so\ndark!";
  }

  private applySelectedTime(): void {
    new TimeSystem(this.gameState).setManualTime({
      hour: this.hour,
      minute: this.minute,
      second: 0,
    });
    this.reactionText = this.buildReactionText();
  }

  getTextSnapshot(): TextSnapshotPayload {
    const dialogueText = this.dialogue.getVisibleText();
    const dialogueLines = dialogueText ? dialogueText.split("\n") : null;
    let snapshot = buildPromptScreenSnapshot();

    switch (this.phase) {
      case TimeSetPhase.SET_HOUR:
        snapshot = buildPromptScreenSnapshot({
          infoLines: ["Use move up/down to adjust hour; press a to select; press b to back."],
          dialogueLines: [TimeSetScreen.QUESTION_HOUR],
          menuLines: [this.buildHourDisplay()],
        });
        break;
      case TimeSetPhase.SET_MINUTE:
        snapshot = buildPromptScreenSnapshot({
          infoLines: ["Use move up/down/left/right to adjust minute; press a to select; press b to back."],
          dialogueLines: [TimeSetScreen.QUESTION_MINUTE],
          menuLines: [this.buildMinuteDisplay()],
        });
        break;
      case TimeSetPhase.HOUR_CONFIRM:
      case TimeSetPhase.MINUTE_CONFIRM:
        snapshot = buildPromptScreenSnapshot({
          infoLines: ["Use move up/down to choose YES/NO; press a to select; press b to cancel."],
          dialogueLines,
          promptLines: buildPromptCursorLines(["YES", "NO"], this.yesNo?.result() ? 0 : 1),
        });
        break;
      case TimeSetPhase.FINAL_REACTION:
      case TimeSetPhase.WAKE_DIALOGUE:
        snapshot = buildPromptScreenSnapshot({
          infoLines: [this.dialogue.isComplete() ? "A/B=Continue" : "A/B=Show full text"],
          dialogueLines,
        });
        break;
      case TimeSetPhase.COMPLETE:
        snapshot = buildPromptScreenSnapshot({
          infoLines: ["WAIT: returning to Oak intro"],
        });
        break;
      default:
        break;
    }

    return snapshot;
  }
}
