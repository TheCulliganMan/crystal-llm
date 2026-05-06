// ASM mapping: pokecrystal_disassembly/engine/events/pokecenter_pc.asm::PokemonCenterPC.TopMenu.
import { gameEngine, type GameEngineEventQueue, type GameEngineEvent } from "../game-engine";
import { Surface } from "../surface";
import { AudioEngine } from "../../engine/systems/audio";
import { TILE_SIZE } from "../../engine/world/tile";
import { isCancelEvent, isConfirmEvent, isKeyDownEvent } from "../../input/buttons";
import { Z_INDEX_DIALOGUE } from "../z-index";
import { PC_WINDOW_FILL } from "./pc-views";
import { nextFrame } from "../async-loop";
import type { RenderTextOptions } from "../font-renderer";
import type { ScreenUI } from "../screens/screen-types";

type PCHubUI = {
  screen: Surface | null;
  screenWidth?: number;
  screenHeight?: number;
  font?: {
    render_text?: (
      text: string,
      x: number,
      y: number,
      surface: Surface,
      options?: RenderTextOptions
    ) => void;
    renderText?: (
      text: string,
      x: number,
      y: number,
      surface: Surface,
      options?: RenderTextOptions
    ) => void;
  };
  drawWindow?: (
    surface: Surface,
    x: number,
    y: number,
    widthTiles: number,
    heightTiles: number,
    options?: { fill?: [number, number, number]; zIndex?: number; frameId?: number; record?: boolean }
  ) => void;
  draw_window?: (
    surface: Surface,
    x: number,
    y: number,
    widthTiles: number,
    heightTiles: number,
    options?: { fill?: [number, number, number]; zIndex?: number; frameId?: number; record?: boolean }
  ) => void;
  update?: () => void;
  eventQueue?: GameEngineEventQueue;
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

class HubMenuWindow {
  constructor(
    public readonly x: number = 0,
    public readonly y: number = 0,
    public readonly width: number = 16,
    public readonly height: number = 13,
  ) {}

  origin(): [number, number] {
    return [this.x * TILE_SIZE, this.y * TILE_SIZE];
  }
}

type PCHubMenuOptions = {
  promptText?: string;
  window?: HubMenuWindow;
  windowFill?: [number, number, number];
  eventProvider?: () => GameEngineEvent[];
};

export class PCHubMenu {
  private static readonly WINDOW = new HubMenuWindow();
  public index = 0;
  public finished = false;
  private readonly promptText: string;
  private readonly window: HubMenuWindow;
  private readonly windowFill: [number, number, number];
  private readonly eventProvider?: () => GameEngineEvent[];

  constructor(
    private readonly ui: PCHubUI,
    private readonly options: string[],
    private readonly audioEngine: AudioEngine | null = null,
    settings: PCHubMenuOptions = {},
  ) {
    if (!options.length) {
      throw new Error("PCHubMenu requires at least one option");
    }
    this.promptText = settings.promptText?.trim() ?? "";
    this.window = settings.window ?? PCHubMenu.WINDOW;
    this.windowFill = settings.windowFill ?? PC_WINDOW_FILL;
    this.eventProvider = settings.eventProvider;
  }

  handleInput(event: GameEngineEvent): void {
    if (!isKeyDownEvent(event)) {
      return;
    }
    if (event.key === gameEngine.K_UP || event.key === gameEngine.K_LEFT) {
      this.index = (this.index - 1 + this.options.length) % this.options.length;
      this.playCursor();
    } else if (event.key === gameEngine.K_DOWN || event.key === gameEngine.K_RIGHT) {
      this.index = (this.index + 1) % this.options.length;
      this.playCursor();
    } else if (isConfirmEvent(event)) {
      this.finished = true;
      this.playConfirm();
    } else if (isCancelEvent(event)) {
      this.index = this.options.length - 1;
      this.finished = true;
      this.playConfirm();
    }
  }

  draw(): void {
    const screen = this.ui.screen;
    const drawWindow = this.ui.drawWindow ?? this.ui.draw_window;
    if (!screen || typeof drawWindow !== "function") {
      return;
    }
    const renderText = this.resolveRenderText();
    if (!renderText) {
      return;
    }
    const [originX, originY] = this.window.origin();
    drawWindow.call(
      this.ui,
      screen,
      originX,
      originY,
      this.window.width,
      this.window.height,
      { fill: this.windowFill, zIndex: Z_INDEX_DIALOGUE },
    );
    this.ui._record_window_region?.(
      screen,
      originX,
      originY,
      this.window.width,
      this.window.height,
      Z_INDEX_DIALOGUE,
    );
    const textX = originX + TILE_SIZE;
    let textY = originY + TILE_SIZE;
    if (this.promptText) {
      renderText(
        this.promptText,
        textX,
        textY,
        screen,
        {
          textWidth: Math.max(0, (this.window.width - 2) * TILE_SIZE),
          maxLines: 2,
          uppercase: false,
        },
      );
      textY += 3 * TILE_SIZE;
    }
    this.options.forEach((option, idx) => {
      const cursor = idx === this.index ? "\u25b6" : " ";
      renderText(
        `${cursor}${option}`,
        textX,
        textY + idx * TILE_SIZE,
        screen,
        { uppercase: true },
      );
    });
    this.renderTextSnapshot();
  }

  run(drawCallback?: () => void): number {
    while (!this.finished) {
      const events = this.eventProvider ? this.eventProvider() : gameEngine.event.get(this.ui?.eventQueue ?? undefined);
      for (const event of events) {
        if (event.type === gameEngine.QUIT) {
          gameEngine.quit();
          throw new Error("PCHubMenu aborted by QUIT event.");
        }
        this.handleInput(event);
      }
      if (drawCallback) {
        drawCallback();
      }
      this.draw();
      this.ui.update?.();
    }
    return this.index;
  }

  async runAsync(drawCallback?: () => void): Promise<number> {
    while (!this.finished) {
      const events = this.eventProvider ? this.eventProvider() : gameEngine.event.get(this.ui?.eventQueue);
      for (const event of events) {
        if (event.type === gameEngine.QUIT) {
          gameEngine.quit();
          throw new Error("PCHubMenu aborted by QUIT event.");
        }
        this.handleInput(event);
      }
      if (drawCallback) {
        drawCallback();
      }
      this.draw();
      this.ui.update?.();
      await nextFrame();
    }
    return this.index;
  }

  private playCursor(): void {
    this.audioEngine?.playSound("SFX_CHOOSE_PC_OPTION");
  }

  private playConfirm(): void {
    this.audioEngine?.playSound("SFX_CHOOSE_PC_OPTION");
  }

  private renderTextSnapshot(): void {
    if (!this.ui.renderSnapshot) {
      return;
    }
    const viewportLines = this.promptText ? [this.promptText] : ["PC"];
    const menuLines = this.options.map((option, idx) => `${idx === this.index ? "▶" : " "} ${option}`);
    this.ui.renderSnapshot(
      viewportLines,
      ["D-Pad=Move A=Select B=Back"],
      "PC",
      "Legend",
      menuLines,
      null,
      null,
    );
  }

  private resolveRenderText():
    | ((
        text: string,
        x: number,
        y: number,
        surface: Surface,
        options?: RenderTextOptions
      ) => void)
    | null {
    const font = this.ui.font;
    if (!font) {
      return null;
    }
    if (typeof font.render_text === "function") {
      return font.render_text.bind(font);
    }
    if (typeof font.renderText === "function") {
      return font.renderText.bind(font);
    }
    return null;
  }
}
