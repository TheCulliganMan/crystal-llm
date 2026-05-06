import type { Surface } from "@pokecrystal/core/ui/surface";
import type { RGB, ScreenUI } from "@pokecrystal/core/ui/screens/screen-types";

type ActionLogHandler = (lines: string[], limit?: number) => void;
type MarkerHandler = (x: number, y: number, char?: string) => void;

export type CompositeChild = {
  screen?: ScreenUI["screen"];
  font?: ScreenUI["font"];
  clearScreen?: ScreenUI["clearScreen"];
  update?: ScreenUI["update"];
  close?: () => void;
  screen_width?: number;
  screen_height?: number;
  scale?: number;
  display_width?: number;
  display_height?: number;
  draw_text_box?: (surface: Surface, text: string, x: number, y: number, width: number, height: number, frameId?: number, fill?: RGB, textColor?: RGB, zIndex?: number) => void;
  drawTextBox?: (surface: Surface, text: string, x: number, y: number, width: number, height: number, frameId?: number, fill?: RGB, textColor?: RGB, zIndex?: number) => void;
  draw_window?: (surface: Surface, x: number, y: number, width: number, height: number, options?: { frame_id?: number; fill?: RGB; z_index?: number; record?: boolean }) => void;
  drawWindow?: (surface: Surface, x: number, y: number, width: number, height: number, options?: { frameId?: number; fill?: RGB; zIndex?: number; record?: boolean }) => void;
  draw_sprite?: (spriteId: string, x: number, y: number, spriteType?: string, frame?: number) => void;
  drawSprite?: (spriteId: string, x: number, y: number, spriteType?: string, frame?: number) => void;
  _record_window_region?: (
    surface: Surface,
    x: number,
    y: number,
    widthTiles: number,
    heightTiles: number,
    zIndex: number,
    sourceSurface?: Surface
  ) => void;
  _recordWindowRegion?: ScreenUI["_recordWindowRegion"];
  renderSnapshot?: ScreenUI["renderSnapshot"];
  renderOverworldOverlay?: (
    viewportLines: string[],
    infoLines: string[],
    options?: { menuLines?: string[] | null; promptLines?: string[] | null; dialogueLines?: string[] | null }
  ) => void;
  setActionLog?: ActionLogHandler;
  setMarker?: MarkerHandler;
  [key: string]: unknown;
};

export class CompositeUI {
  private static readonly BROADCAST_METHODS = new Set([
    "clearScreen",
    "update",
    "close",
    "setActionLog",
    "setMarker",
    "renderSnapshot",
    "renderOverworldOverlay",
  ]);
  private readonly children: CompositeChild[];
  private readonly primary: CompositeChild;

  constructor(...children: CompositeChild[]) {
    if (!children.length) {
      throw new Error("CompositeUI requires at least one child UI.");
    }
    this.children = [...children];
    this.primary = this.children[0];
    const mirrored = [
      "screen",
      "screen_width",
      "screen_height",
      "screenWidth",
      "screenHeight",
      "scale",
      "display_width",
      "display_height",
      "displayWidth",
      "displayHeight",
    ] as const;
    for (const attr of mirrored) {
      if (attr in this.primary) {
        Object.defineProperty(this, attr, {
          get: () => this.primary[attr],
          configurable: true,
          enumerable: true,
        });
      }
    }
    const mirroredAssignments = new Set(["font", "eventQueue"]);
    return new Proxy(this, {
      get: (target, prop, receiver) => {
        if (Reflect.has(target, prop)) {
          return Reflect.get(target, prop, receiver);
        }
        if (typeof prop !== "string") {
          return undefined;
        }
        const value = target.primary[prop as keyof CompositeChild];
        if (typeof value === "function") {
          return (...args: unknown[]) => {
            if (CompositeUI.BROADCAST_METHODS.has(prop)) {
              return target.broadcast(prop, ...args);
            }
            return value.apply(target.primary, args);
          };
        }
        return value;
      },
      set: (target, prop, value, receiver) => {
        const result = Reflect.set(target, prop, value, receiver);
        if (typeof prop === "string" && mirroredAssignments.has(prop)) {
          for (const child of target.children) {
            (child as Record<string, unknown>)[prop] = value;
          }
        }
        return result;
      },
    });
  }

  getChildren(): CompositeChild[] {
    return [...this.children];
  }

  getPrimary(): CompositeChild {
    return this.primary;
  }

  clearScreen(color: [number, number, number] = [0, 0, 0]): void {
    this.broadcast("clearScreen", color);
  }

  update(): void {
    this.broadcast("update");
  }

  close(): void {
    this.broadcast("close");
  }

  setActionLog(...args: unknown[]): void {
    this.broadcast("setActionLog", ...args);
  }

  setMarker(...args: unknown[]): void {
    this.broadcast("setMarker", ...args);
  }

  renderSnapshot(...args: unknown[]): void {
    this.broadcast("renderSnapshot", ...args);
  }

  renderOverworldOverlay(...args: unknown[]): void {
    this.broadcast("renderOverworldOverlay", ...args);
  }

  getSnapshot(...args: unknown[]): unknown {
    return this.broadcast("getSnapshot", ...args);
  }

  private broadcast(name: string, ...args: unknown[]): unknown {
    let result: unknown = undefined;
    for (const child of this.children) {
      const target = child[name as keyof CompositeChild];
      if (typeof target === "function") {
        result = (target as (...inner: unknown[]) => unknown).apply(child, args);
      }
    }
    return result;
  }
}
