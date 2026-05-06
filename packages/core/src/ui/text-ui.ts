import type { BaseFontRenderer } from "./base-ui";
import { BaseUI } from "./base-ui";
import { Surface } from "./surface";

export const ACTION_LOG_MAX_LINES = 8;

export type TextSnapshot = {
  viewportLines: string[];
  infoLines: string[];
  marker?: [number, number, string] | null;
  actionLog?: string[] | null;
  viewportTitle?: string;
  infoTitle?: string;
  menuLines?: string[] | null;
  promptLines?: string[] | null;
  dialogueLines?: string[] | null;
};

const areLinesEqual = (
  previous: readonly string[] | null | undefined,
  next: readonly string[] | null | undefined
): boolean => {
  const left = previous ?? null;
  const right = next ?? null;
  if (left === right) {
    return true;
  }
  if (!left || !right || left.length !== right.length) {
    return false;
  }
  for (let index = 0; index < left.length; index += 1) {
    if (left[index] !== right[index]) {
      return false;
    }
  }
  return true;
};

const areMarkersEqual = (
  previous: TextSnapshot["marker"],
  next: TextSnapshot["marker"]
): boolean => {
  if (previous === next) {
    return true;
  }
  if (!previous || !next) {
    return previous === next;
  }
  return previous[0] === next[0] && previous[1] === next[1] && previous[2] === next[2];
};

export class TextUI extends BaseUI {
  private snapshot: TextSnapshot | null = null;
  private marker: [number, number, string] | null = null;
  private actionLog: string[] = [];
  private readonly liveMode: boolean;
  private readonly refreshHz: number | null;
  private lastUpdateAt = 0;

  public readonly font: BaseFontRenderer = {
    renderText: (_text, _x, _y, _surface, _options) => {
      // Text UI has no pixel output; snapshot rendering is handled separately.
    },
  };

  constructor(
    screenWidth?: number | null,
    screenHeight?: number | null,
    scale: number = 1,
    _useSuperGameBoyTiles?: boolean | null,
    liveMode: boolean = true,
    refreshHz: number | null = 60,
    _preferHeadless: boolean = true
  ) {
    if (refreshHz !== null && refreshHz < 0) {
      throw new Error("refreshHz must be zero or greater.");
    }
    super(screenWidth ?? 160, screenHeight ?? 144, scale);
    this.liveMode = Boolean(liveMode) && (refreshHz ?? 0) !== 0;
    this.refreshHz = refreshHz;
  }

  protected createScreenSurface(): Surface {
    return new Surface(this.screenWidth, this.screenHeight);
  }

  clearScreen(_color: [number, number, number] = [0, 0, 0]): void {
    // Text UI snapshots are rendered separately from the offscreen surface, so
    // clearing the hidden backing surface every frame only burns CPU.
  }

  update(): void {
    if (!this.liveMode || !this.snapshot) {
      return;
    }
    if (this.refreshHz !== null && this.refreshHz > 0) {
      const now = typeof performance !== "undefined" ? performance.now() : Date.now();
      const minDelta = 1000 / this.refreshHz;
      if (now - this.lastUpdateAt < minDelta) {
        return;
      }
      this.lastUpdateAt = now;
    }
    this.flush_window_stack();
  }

  setMarker(x: number, y: number, char: string = "@"): void {
    this.marker = [x, y, char ? char.slice(0, 1) : "@"];
  }

  renderOverworldOverlay(
    viewportLines: string[],
    infoLines: string[],
    options?: { menuLines?: string[] | null; promptLines?: string[] | null; dialogueLines?: string[] | null }
  ): void {
    this.renderSnapshot(
      viewportLines,
      infoLines,
      "Overworld",
      "Legend",
      options?.menuLines ?? null,
      options?.promptLines ?? null,
      options?.dialogueLines ?? null
    );
  }

  renderSnapshot(
    viewportLines: string[],
    infoLines: string[],
    viewportTitle: string = "Overworld",
    infoTitle: string = "Info",
    menuLines: string[] | null = null,
    promptLines: string[] | null = null,
    dialogueLines: string[] | null = null
  ): void {
    const nextSnapshot: TextSnapshot = {
      viewportLines: [...viewportLines],
      infoLines: [...infoLines],
      marker: this.marker,
      actionLog: [...this.actionLog],
      viewportTitle,
      infoTitle,
      menuLines: menuLines ? [...menuLines] : null,
      promptLines: promptLines ? [...promptLines] : null,
      dialogueLines: dialogueLines ? [...dialogueLines] : null,
    };
    if (
      this.snapshot &&
      this.snapshot.viewportTitle === nextSnapshot.viewportTitle &&
      this.snapshot.infoTitle === nextSnapshot.infoTitle &&
      areMarkersEqual(this.snapshot.marker, nextSnapshot.marker) &&
      areLinesEqual(this.snapshot.viewportLines, nextSnapshot.viewportLines) &&
      areLinesEqual(this.snapshot.infoLines, nextSnapshot.infoLines) &&
      areLinesEqual(this.snapshot.actionLog, nextSnapshot.actionLog) &&
      areLinesEqual(this.snapshot.menuLines, nextSnapshot.menuLines) &&
      areLinesEqual(this.snapshot.promptLines, nextSnapshot.promptLines) &&
      areLinesEqual(this.snapshot.dialogueLines, nextSnapshot.dialogueLines)
    ) {
      return;
    }
    this.snapshot = nextSnapshot;
  }

  setActionLog(lines: string[], limit: number = ACTION_LOG_MAX_LINES): void {
    if (limit <= 0) {
      this.actionLog = [];
      return;
    }
    const nextLog = [...lines].slice(-limit);
    if (areLinesEqual(this.actionLog, nextLog)) {
      return;
    }
    this.actionLog = nextLog;
  }

  getSnapshot(): TextSnapshot | null {
    return this.snapshot;
  }

  close(): void {
    this.snapshot = null;
    this.actionLog = [];
    this.marker = null;
    this.flush_window_stack();
  }
}
