// ASM mapping: engine/menus/menu.asm basic list menu behavior.
import { gameEngine } from "../game-engine";
import { MenuUI } from "./types";
import { KeyEvent, isCancelEvent, isConfirmEvent, isKeyDownEvent, isKeyUpEvent } from "../../input/buttons";
import { mapKeyToDirection } from "../../input/controls";
import { Z_INDEX_MENU } from "../z-index";
import { renderFontText } from "../text/render-font";

const INITIAL_REPEAT_DELAY_FRAMES = 12;
const REPEAT_INTERVAL_FRAMES = 4;
const CURSOR_BLINK_PERIOD_FRAMES = 8;
const CONFIRM_DELAY_FRAMES = 10;

const directionForKey = (key: string | number | null | undefined): "up" | "down" | null => {
  const direction = mapKeyToDirection(key ? String(key) : null);
  return direction === "up" || direction === "down" ? direction : null;
};

class MenuRepeatState {
  private held = new Set<string>();
  private repeatTimers = new Map<string, number>();

  isHeld(direction: string): boolean {
    return this.held.has(direction);
  }

  start(direction: string): void {
    this.held.add(direction);
    this.repeatTimers.set(direction, INITIAL_REPEAT_DELAY_FRAMES);
  }

  stop(direction: string): void {
    this.held.delete(direction);
    this.repeatTimers.delete(direction);
  }

  stopAll(): void {
    this.held.clear();
    this.repeatTimers.clear();
  }

  tick(): string[] {
    const repeated: string[] = [];
    for (const direction of Array.from(this.held)) {
      const timer = (this.repeatTimers.get(direction) ?? 0) - 1;
      if (timer <= 0) {
        repeated.push(direction);
        this.repeatTimers.set(direction, REPEAT_INTERVAL_FRAMES);
      } else {
        this.repeatTimers.set(direction, timer);
      }
    }
    return repeated;
  }
}

export class Menu {
  public selectedOption = 0;
  private repeatState = new MenuRepeatState();
  private cursorFrame = 0;
  private cursorVisibleValue = true;
  private confirmCooldown = 0;

  constructor(
    private readonly ui: MenuUI,
    private readonly options: string[],
    private readonly x: number,
    private readonly y: number,
    private readonly width: number,
    private readonly height?: number | null,
    private readonly rowHeightTiles: number = 1,
    private readonly zIndex: number = Z_INDEX_MENU,
    private readonly frameId: number | null = null,
    private readonly audioEngine?: { playSound?: (name: string) => void } | null,
  ) {}

  handleInput(event: KeyEvent): string | null {
    if (isKeyUpEvent(event)) {
      const direction = directionForKey(event.key ?? null);
      if (direction) {
        this.repeatState.stop(direction);
      }
      return null;
    }
    if (this.confirmCooldown > 0) {
      return null;
    }
    if (isKeyDownEvent(event)) {
      const direction = directionForKey(event.key ?? null);
      if (direction) {
        if (!this.repeatState.isHeld(direction)) {
          this.repeatState.start(direction);
          this.moveCursor(direction);
        }
        return null;
      }
      if (isConfirmEvent(event)) {
        this.repeatState.stopAll();
        this.playConfirmSound();
        this.confirmCooldown = CONFIRM_DELAY_FRAMES;
        return this.confirmSelection();
      }
      if (isCancelEvent(event)) {
        this.repeatState.stopAll();
        this.playCancelSound();
        this.confirmCooldown = CONFIRM_DELAY_FRAMES;
        return "CANCEL";
      }
    }
    return null;
  }

  update(): void {
    if (this.confirmCooldown > 0) {
      this.confirmCooldown = Math.max(0, this.confirmCooldown - 1);
    }
    this.cursorFrame = (this.cursorFrame + 1) % (CURSOR_BLINK_PERIOD_FRAMES * 2);
    this.cursorVisibleValue = this.cursorFrame < CURSOR_BLINK_PERIOD_FRAMES;
    if (this.confirmCooldown > 0) {
      return;
    }
    for (const direction of this.repeatState.tick()) {
      this.moveCursor(direction as "up" | "down");
    }
  }

  draw(): void {
    if (!this.ui.screen) {
      return;
    }
    const textboxPalette = this.textboxPalette();
    const fillColor = textboxPalette?.[0] ?? ([255, 255, 255] as [number, number, number]);
    const width = Math.max(this.width, Menu.minimumWidth(this.options));
    const height = this.height ?? this.options.length + 2;
    this.ui.drawWindow(this.ui.screen, this.x, this.y, width, height, { fill: fillColor });
    const textX = this.x + this.ui.tileSize;
    const textY = this.y + this.ui.tileSize * 2;
    const textWidth = Math.max(0, (width - 2) * this.ui.tileSize);
    const lineHeight = Math.max(1, this.rowHeightTiles) * this.ui.tileSize;
    for (let index = 0; index < this.options.length; index += 1) {
      const cursorActive = index === this.selectedOption && this.cursorVisibleValue;
      const cursor = cursorActive ? "\u25b6" : " ";
      renderFontText(this.ui.font, `${cursor}${this.options[index]}`, textX, textY + index * lineHeight, this.ui.screen, {
        palette: textboxPalette ?? undefined,
        textWidth,
        maxLines: 1,
      });
    }
  }

  private confirmSelection(): string | null {
    if (!this.options.length) {
      throw new Error("Menu confirmation requested with no options");
    }
    const selection = this.options[this.selectedOption];
    if (selection === undefined) {
      throw new Error(`Menu selected invalid option index ${this.selectedOption}`);
    }
    return selection;
  }

  private moveCursor(direction: "up" | "down"): void {
    if (!this.options.length) {
      return;
    }
    const delta = direction === "up" ? -1 : 1;
    this.selectedOption = (this.selectedOption + delta + this.options.length) % this.options.length;
    this.cursorFrame = 0;
    this.cursorVisibleValue = true;
  }

  private playConfirmSound(): void {
    this.audioEngine?.playSound?.("menu_option");
  }

  private playCancelSound(): void {
    this.audioEngine?.playSound?.("menu_cancel");
  }

  private textboxPalette(): [number, number, number][] | null {
    const getter = this.ui.get_context_palette ?? this.ui.getContextPalette;
    if (!getter) {
      return null;
    }
    return getter.call(this.ui, "textbox");
  }

  static minimumWidth(options: string[]): number {
    if (!options.length) {
      return 2;
    }
    const longest = Math.max(...options.map((option) => option.length + 1));
    return longest + 2;
  }

  getOptions(): string[] {
    return [...this.options];
  }

  getSelectedOption(): number {
    return this.selectedOption;
  }

  get cursorVisible(): boolean {
    return this.cursorVisibleValue;
  }
}
