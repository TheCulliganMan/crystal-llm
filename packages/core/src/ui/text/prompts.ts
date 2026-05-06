// ASM mapping: pokecrystal_disassembly/home/menu.asm (menu cursor handling and prompt confirmation flow).
import { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { TILE_SIZE } from "@pokecrystal/core/engine/world/tile";
import { KeyEvent, isCancelEvent, isConfirmEvent, isKeyDownEvent } from "@pokecrystal/core/input/buttons";
import { mapKeyToDirection } from "@pokecrystal/core/input/controls";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { BaseUI } from "@pokecrystal/core/ui/base-ui";
import type { ScreenUI } from "@pokecrystal/core/ui/screens/screen-types";
import { Z_INDEX_PROMPT } from "@pokecrystal/core/ui/z-index";
import { nextFrame } from "@pokecrystal/core/ui/async-loop";
import type { RenderTextOptions } from "@pokecrystal/core/ui/font-renderer";
import type { Surface } from "@pokecrystal/core/ui/surface";

interface SelectionPromptOptions {
  audioEngine?: AudioEngine;
  title?: string;
  initialIndex?: number;
  cancelResult?: number;
  windowOriginTiles?: [number, number];
  windowMinWidth?: number;
  windowMinHeight?: number;
  windowFill?: [number, number, number];
}

interface PromptRunOptions {
  drawCallback?: () => void;
  eventProvider?: () => KeyEvent[];
}

type PromptUI = BaseUI & {
  screen: Surface | null;
  drawWindow?: BaseUI["drawWindow"];
  draw_window?: BaseUI["draw_window"];
  renderSnapshot?: ScreenUI["renderSnapshot"];
  _record_window_region?: (
    surface: Surface,
    x: number,
    y: number,
    widthTiles: number,
    heightTiles: number,
    zIndex: number
  ) => void;
};

export const isPromptUI = (ui: unknown): ui is PromptUI => {
  if (!ui || typeof ui !== "object") {
    return false;
  }
  const candidate = ui as PromptUI;
  const font = candidate.font as { render_text?: unknown; renderText?: unknown } | undefined;
  const hasFont = !!font && (typeof font.render_text === "function" || typeof font.renderText === "function");
  const hasScreen = "screen" in candidate;
  const hasUpdate = typeof (candidate as { update?: unknown }).update === "function";
  const hasSize =
    typeof (candidate as { screenWidth?: unknown }).screenWidth === "number" &&
    typeof (candidate as { screenHeight?: unknown }).screenHeight === "number";
  const hasDrawWindow =
    typeof (candidate as { drawWindow?: unknown }).drawWindow === "function" ||
    typeof (candidate as { draw_window?: unknown }).draw_window === "function";
  return hasFont && hasScreen && hasUpdate && hasSize && hasDrawWindow;
};

const getTileSize = (ui: PromptUI): number => {
  return ui.tile_size ?? ui.tileSize ?? TILE_SIZE;
};

const renderPromptText = (
  ui: PromptUI,
  text: string,
  x: number,
  y: number,
  surface: Surface,
  uppercase = true
): void => {
  const font = ui.font;
  const textOptions: RenderTextOptions = { uppercase };
  if (font?.render_text) {
    font.render_text(text, x, y, surface, textOptions);
    return;
  }
  if (font?.renderText) {
    font.renderText(text, x, y, surface, textOptions);
    return;
  }
  throw new Error("SelectionPrompt requires a font renderer");
};

const renderSelectionPromptSnapshot = (
  ui: PromptUI,
  options: {
    title?: string;
    menuLines: string[];
  }
): void => {
  if (!ui.renderSnapshot) {
    return;
  }
  const title = options.title?.trim() || "Prompt";
  const infoLines = ["D-Pad=Move A=Select B=Back"];
  ui.renderSnapshot(
    [title],
    infoLines,
    title,
    "Legend",
    options.menuLines,
    null,
    null
  );
};

export class SelectionPrompt {
  private readonly ui: PromptUI;
  private readonly screen: Surface | null;
  private readonly options: string[];
  private readonly settings: SelectionPromptOptions;
  private readonly audioEngine?: AudioEngine;
  private readonly title?: string;
  private readonly windowOriginTiles?: [number, number];
  private readonly windowMinWidth: number;
  private readonly windowMinHeight: number;
  private readonly windowFill: [number, number, number];
  private index = 0;
  private finished = false;

  constructor(ui: PromptUI, options: string[], settings: SelectionPromptOptions = {}) {
    if (!options.length) {
      throw new Error("SelectionPrompt requires at least one option");
    }
    this.ui = ui;
    this.screen = ui.screen;
    this.options = options;
    this.settings = settings;
    this.audioEngine = settings.audioEngine;
    this.title = settings.title;
    this.index = Math.max(0, Math.min(options.length - 1, Math.trunc(settings.initialIndex ?? 0)));
    this.windowOriginTiles = settings.windowOriginTiles;
    this.windowMinWidth = Math.max(0, settings.windowMinWidth ?? 0);
    this.windowMinHeight = Math.max(0, settings.windowMinHeight ?? 0);
    this.windowFill = settings.windowFill ?? [255, 255, 255];
  }

  private windowGeometry(): [number, number, number, number] | null {
    if (!this.screen || !this.windowOriginTiles) {
      return null;
    }
    const [originX, originY] = this.windowOriginTiles;
    let maxChars = Math.max(...this.options.map((option) => option.length + 1));
    if (this.title) {
      maxChars = Math.max(maxChars, this.title.length);
    }
    let widthTiles = Math.max(this.windowMinWidth, maxChars + 2);
    const screenWidthTiles = Math.max(1, Math.floor(this.ui.screenWidth / TILE_SIZE));
    const availableWidth = Math.max(1, screenWidthTiles - originX);
    widthTiles = Math.min(widthTiles, availableWidth);

    const innerLines = this.options.length + (this.title ? 1 : 0);
    let heightTiles = Math.max(this.windowMinHeight, innerLines + 2);
    const screenHeightTiles = Math.max(1, Math.floor(this.ui.screenHeight / TILE_SIZE));
    const availableHeight = Math.max(1, screenHeightTiles - originY);
    heightTiles = Math.min(heightTiles, availableHeight);

    return [originX * TILE_SIZE, originY * TILE_SIZE, widthTiles, heightTiles];
  }

  handleInput(event: KeyEvent): void {
    if (!isKeyDownEvent(event)) {
      return;
    }
    const direction = mapKeyToDirection(event.direction ?? event.code ?? event.key ?? null);
    if (direction === "up" || direction === "left") {
      this.index = (this.index - 1 + this.options.length) % this.options.length;
    } else if (direction === "down" || direction === "right") {
      this.index = (this.index + 1) % this.options.length;
    } else if (isConfirmEvent(event)) {
      this.finished = true;
      this.playConfirm();
    } else if (isCancelEvent(event)) {
      this.index = this.settings.cancelResult ?? this.options.length - 1;
      this.finished = true;
      this.playConfirm();
    }
  }

  draw(clearBackground = true): void {
    const surface = this.screen;
    if (!surface) {
      return;
    }
    const geometry = this.windowGeometry();
    let textX = TILE_SIZE;
    let titleY = TILE_SIZE;
    let optionY = TILE_SIZE * 2;
    if (geometry) {
      const [originX, originY, widthTiles, heightTiles] = geometry;
      const drawWindow = (this.ui.drawWindow ?? this.ui.draw_window)?.bind(this.ui);
      if (!drawWindow) {
        throw new Error("SelectionPrompt requires drawWindow/draw_window");
      }
      drawWindow(surface, originX, originY, widthTiles, heightTiles, { fill: this.windowFill });
      textX = originX + TILE_SIZE;
      titleY = originY + TILE_SIZE;
      optionY = originY + (this.title ? 2 * TILE_SIZE : TILE_SIZE);
      this.ui._record_window_region?.(
        surface,
        originX,
        originY,
        widthTiles,
        heightTiles,
        Z_INDEX_PROMPT
      );
    } else if (clearBackground) {
      this.ui.clearScreen?.([0, 0, 0]);
    }

    if (this.title) {
      renderPromptText(this.ui, this.title, textX, titleY, surface, true);
    }

    this.options.forEach((option, idx) => {
      const cursor = idx === this.index ? "▶" : " ";
      renderPromptText(
        this.ui,
        `${cursor}${option}`,
        textX,
        optionY + idx * TILE_SIZE,
        surface,
        true
      );
    });

    renderSelectionPromptSnapshot(this.ui, {
      title: this.title,
      menuLines: this.options.map((option, idx) =>
        `${idx === this.index ? "▶" : " "} ${option}`
      ),
    });
  }

  result(): number {
    return this.index;
  }

  run(options: PromptRunOptions = {}): number {
    while (!this.finished) {
      const events = options.eventProvider ? options.eventProvider() : gameEngine.event.get(this.ui.eventQueue);
      for (const event of events) {
        if (event.type === gameEngine.QUIT) {
          gameEngine.quit();
          throw new Error("SelectionPrompt interrupted by quit event");
        }
        this.handleInput(event as KeyEvent);
      }
      if (options.drawCallback) {
        options.drawCallback();
        this.draw(false);
      } else {
        this.draw();
      }
      this.ui.update();
    }
    return this.result();
  }

  async runAsync(options: PromptRunOptions = {}): Promise<number> {
    while (!this.finished) {
      const events = options.eventProvider ? options.eventProvider() : gameEngine.event.get(this.ui.eventQueue);
      for (const event of events) {
        if (event.type === gameEngine.QUIT) {
          gameEngine.quit();
          throw new Error("SelectionPrompt interrupted by quit event");
        }
        this.handleInput(event as KeyEvent);
      }
      if (options.drawCallback) {
        options.drawCallback();
        this.draw(false);
      } else {
        this.draw();
      }
      this.ui.update();
      await nextFrame();
    }
    return this.result();
  }

  private playConfirm(): void {
    this.audioEngine?.playSound?.("menu_option");
  }
}

export class NumberPrompt {
  private readonly ui: PromptUI;
  private readonly screen: Surface | null;
  private readonly minimum: number;
  private readonly maximum: number;
  private readonly audioEngine?: AudioEngine;
  public value: number;
  public finished = false;

  constructor(
    ui: PromptUI,
    options: { minimum?: number; maximum?: number; initial?: number; audioEngine?: AudioEngine } = {}
  ) {
    this.ui = ui;
    this.screen = ui.screen;
    this.minimum = options.minimum ?? 1;
    this.maximum = options.maximum ?? 99;
    const initial = options.initial ?? this.minimum;
    this.value = Math.max(this.minimum, Math.min(this.maximum, initial));
    this.audioEngine = options.audioEngine;
  }

  handleInput(event: KeyEvent): void {
    if (!isKeyDownEvent(event)) {
      return;
    }
    const direction = mapKeyToDirection(event.direction ?? event.code ?? event.key ?? null);
    if (direction === "left" || direction === "down") {
      this.value = Math.max(this.minimum, this.value - 1);
    } else if (direction === "right" || direction === "up") {
      this.value = Math.min(this.maximum, this.value + 1);
    } else if (isConfirmEvent(event)) {
      this.finished = true;
      this.playConfirm();
    } else if (isCancelEvent(event)) {
      this.value = this.minimum;
      this.finished = true;
    }
  }

  draw(): void {
    if (!this.screen) {
      return;
    }
    const drawWindow = (this.ui.drawWindow ?? this.ui.draw_window)?.bind(this.ui);
    if (!drawWindow) {
      throw new Error("NumberPrompt requires drawWindow/draw_window");
    }
    const x = 13 * TILE_SIZE;
    const y = 10 * TILE_SIZE;
    drawWindow(this.screen, x, y, 6, 4, { fill: [255, 255, 255] });
    renderPromptText(this.ui, `AMT ${String(this.value).padStart(2, "0")}`, x + TILE_SIZE, y + TILE_SIZE, this.screen, false);
    renderPromptText(this.ui, "A:OK B:EXIT", x, y + 2 * TILE_SIZE, this.screen, false);
  }

  private playConfirm(): void {
    this.audioEngine?.playSound?.("menu_option");
  }
}
