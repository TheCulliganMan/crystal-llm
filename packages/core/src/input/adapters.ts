import { BackendAdapter, BackendEvent } from "../backend/api";
import { getBackend } from "../backend/registry";
import { buttonKeyCodes, mapKeycodeToButton } from "./bindings";
import { GameButton } from "./controls";
import { KEYS } from "./keycodes";

export type InputDirection = "up" | "down" | "left" | "right";

export enum InputEventType {
  KEYDOWN = "keydown",
  KEYUP = "keyup",
  QUIT = "quit",
  TEXT = "text",
}

export class InputEvent {
  public type: number;
  public source: string | null;
  public button: GameButton | null;
  public direction: InputDirection | null;
  public is_press: boolean;
  public key: number | null;
  public text: string | null;
  public rawEvent: unknown | null;
  public timestamp: number | null;
  [key: string]: unknown;

  constructor(options: {
    type: number;
    source?: string | null;
    button?: GameButton | null;
    direction?: InputDirection | null;
    is_press?: boolean;
    key?: number | null;
    text?: string | null;
    rawEvent?: unknown | null;
    timestamp?: number | null;
  }) {
    this.type = options.type;
    this.source = options.source ?? null;
    this.button = options.button ?? null;
    this.direction = options.direction ?? null;
    this.is_press = options.is_press ?? false;
    this.key = options.key ?? null;
    this.text = options.text ?? null;
    this.rawEvent = options.rawEvent ?? null;
    this.timestamp = options.timestamp ?? null;
  }

  get unicode(): string {
    if (this.text !== null && this.text !== undefined) {
      return this.text;
    }
    const raw = this.rawEvent as { unicode?: string } | null;
    if (raw?.unicode) {
      return raw.unicode;
    }
    return "";
  }
}

export interface InputAdapter {
  poll(): InputEvent[];
  close(): void;
}

const DIRECTION_KEYS: Record<number, InputDirection> = {
  [KEYS.UP]: "up",
  [KEYS.DOWN]: "down",
  [KEYS.LEFT]: "left",
  [KEYS.RIGHT]: "right",
};

const monotonicSeconds = (): number => {
  if (typeof performance !== "undefined" && typeof performance.now === "function") {
    return performance.now() / 1000;
  }
  return Date.now() / 1000;
};

const mapKeycodeToDirection = (key: number | null | undefined): InputDirection | null => {
  if (key === null || key === undefined) {
    return null;
  }
  return DIRECTION_KEYS[key] ?? null;
};

const buttonKeys = (button: GameButton): number[] => {
  return buttonKeyCodes(button);
};

export class InputFanInAdapter implements InputAdapter {
  private readonly adapters: InputAdapter[];
  private sequenceCounter = 0;

  constructor(adapters: Iterable<InputAdapter>) {
    this.adapters = Array.from(adapters);
  }

  poll(): InputEvent[] {
    const ordered: Array<[number, number, InputEvent]> = [];
    for (const adapter of this.adapters) {
      for (const event of adapter.poll()) {
        const timestamp = event.timestamp ?? monotonicSeconds();
        event.timestamp = timestamp;
        ordered.push([timestamp, this.sequenceCounter, event]);
        this.sequenceCounter += 1;
      }
    }
    ordered.sort((a, b) => (a[0] === b[0] ? a[1] - b[1] : a[0] - b[0]));

    const merged: InputEvent[] = [];
    const seenPresses = new Set<string>();
    for (const [, , event] of ordered) {
      if (event.is_press) {
        const pressKey = `${event.button ?? ""}|${event.direction ?? ""}|${event.key ?? ""}`;
        if (seenPresses.has(pressKey)) {
          continue;
        }
        seenPresses.add(pressKey);
      }
      merged.push(event);
    }
    return merged;
  }

  close(): void {
    for (const adapter of this.adapters) {
      adapter.close();
    }
  }
}

export class PygameInputAdapter implements InputAdapter {
  private readonly backend: BackendAdapter;

  constructor(options?: { backend?: BackendAdapter | null }) {
    this.backend = options?.backend ?? getBackend();
  }

  poll(): InputEvent[] {
    const events: InputEvent[] = [];
    let rawEvents: BackendEvent[] = [];
    try {
      rawEvents = this.backend.pollEvents();
    } catch (error) {
      const backendName = (this.backend as { name?: string }).name ?? "unknown";
      const message = error instanceof Error ? error.message : String(error);
      throw new Error(`Backend '${backendName}' cannot produce events: ${message}`);
    }
    for (const rawEvent of rawEvents) {
      const eventTimestamp = monotonicSeconds();
      events.push(this.normalizeEvent(rawEvent, eventTimestamp));
    }
    return events;
  }

  private normalizeEvent(rawEvent: BackendEvent, timestamp: number): InputEvent {
    if (rawEvent.type === KEYS.QUIT) {
      return new InputEvent({
        type: KEYS.QUIT,
        source: "manual",
        rawEvent,
        timestamp,
      });
    }

    if (rawEvent.type === KEYS.KEYDOWN || rawEvent.type === KEYS.KEYUP) {
      const button = mapKeycodeToButton(rawEvent.key ?? null);
      const direction = mapKeycodeToDirection(rawEvent.key ?? null);
      return new InputEvent({
        type: rawEvent.type,
        source: "manual",
        button,
        direction,
        is_press: rawEvent.type === KEYS.KEYDOWN,
        key: rawEvent.key ?? null,
        text: rawEvent.unicode ?? null,
        rawEvent,
        timestamp,
      });
    }

    return new InputEvent({
      type: typeof rawEvent.type === "number" ? rawEvent.type : -1,
      source: "manual",
      rawEvent,
      timestamp,
    });
  }

  close(): void {
    this.backend.close();
  }
}

export class TerminalInputAdapter implements InputAdapter {
  private readonly stdin: NodeJS.ReadableStream | null;
  private readonly holdFrames: number;
  private readonly debugInputs: boolean;
  private pendingReleases: Array<{ event: InputEvent; framesLeft: number }> = [];
  private buffer = "";

  private static readonly BUTTON_TOKENS: Record<string, GameButton> = {
    a: GameButton.A,
    b: GameButton.B,
    start: GameButton.Start,
    select: GameButton.Select,
  };

  private static readonly DIRECTION_TOKENS: Record<string, InputDirection> = {
    up: "up",
    u: "up",
    down: "down",
    d: "down",
    left: "left",
    l: "left",
    right: "right",
    r: "right",
  };

  constructor(options?: {
    stdin?: NodeJS.ReadableStream | null;
    holdFrames?: number;
    debugInputs?: boolean;
  }) {
    this.stdin = options?.stdin ?? (typeof process !== "undefined" ? process.stdin : null);
    this.holdFrames = options?.holdFrames ?? 6;
    this.debugInputs = options?.debugInputs ?? false;

    if (this.stdin && typeof (this.stdin as NodeJS.ReadableStream).setEncoding === "function") {
      (this.stdin as NodeJS.ReadableStream & { setEncoding: (encoding: string) => void }).setEncoding("utf-8");
    }

    if (this.stdin && typeof (this.stdin as NodeJS.ReadableStream).on === "function") {
      this.stdin.on("data", (chunk: string | Buffer) => {
        const text = typeof chunk === "string" ? chunk : chunk.toString("utf-8");
        this.buffer += text;
      });
    }
  }

  poll(): InputEvent[] {
    const events: InputEvent[] = [];
    events.push(...this.emitDueReleases());

    const tokens = [...this.readTokens()];
    for (const token of tokens) {
      events.push(...this.eventsForToken(token));
    }

    if (this.debugInputs && tokens.length > 0) {
      console.error(`[TerminalInputAdapter] tokens=${JSON.stringify(tokens)}`);
    }

    return events;
  }

  close(): void {
    return;
  }

  protected readTokens(): Iterable<string> {
    if (!this.stdin) {
      return [];
    }
    if (!this.buffer) {
      return [];
    }
    const tokens = this.buffer.split(/\s+/).filter((token) => token.length > 0);
    this.buffer = "";
    return tokens;
  }

  protected eventsForToken(token: string, source = "manual"): InputEvent[] {
    let normalized = token.trim().toLowerCase();
    if (normalized.startsWith(":")) {
      normalized = normalized.slice(1);
    }
    if (!normalized) {
      return [];
    }
    if (["quit", "exit", "q!"].includes(normalized)) {
      return [new InputEvent({ type: KEYS.QUIT, source, text: normalized })];
    }
    if (["wq", "wq!", "x", "x!", "xit"].includes(normalized)) {
      return [new InputEvent({ type: KEYS.QUIT, source, text: normalized })];
    }
    const direction = TerminalInputAdapter.DIRECTION_TOKENS[normalized];
    if (direction) {
      const key = TerminalInputAdapter.keyForDirection(direction);
      return this.tap({ direction, key, source });
    }
    const button = TerminalInputAdapter.BUTTON_TOKENS[normalized];
    if (button) {
      const key = TerminalInputAdapter.keyForButton(button);
      return this.tap({ button, key, source });
    }
    throw new Error(
      `Unknown input token '${token}' (text/script adapters accept a/b/start/select/up/down/left/right plus vim-style quit commands like :q!/:wq/:x/:exit).`
    );
  }

  private static keyForDirection(direction: InputDirection): number | null {
    switch (direction) {
      case "up":
        return KEYS.UP;
      case "down":
        return KEYS.DOWN;
      case "left":
        return KEYS.LEFT;
      case "right":
        return KEYS.RIGHT;
      default:
        return null;
    }
  }

  private static keyForButton(button: GameButton): number | null {
    const bindings = buttonKeys(button);
    return bindings.length > 0 ? bindings[0] : null;
  }

  private tap(options: {
    button?: GameButton | null;
    direction?: InputDirection | null;
    key?: number | null;
    source?: string;
  }): InputEvent[] {
    const timestamp = monotonicSeconds();
    const press = new InputEvent({
      type: KEYS.KEYDOWN,
      source: options.source ?? "manual",
      button: options.button ?? null,
      direction: options.direction ?? null,
      is_press: true,
      key: options.key ?? null,
      timestamp,
    });
    const release = new InputEvent({
      type: KEYS.KEYUP,
      source: options.source ?? "manual",
      button: options.button ?? null,
      direction: options.direction ?? null,
      is_press: false,
      key: options.key ?? null,
      timestamp,
    });
    this.pendingReleases.push({ event: release, framesLeft: this.holdFrames });
    return [press];
  }

  private emitDueReleases(): InputEvent[] {
    if (this.pendingReleases.length === 0) {
      return [];
    }
    const releases: InputEvent[] = [];
    const remaining: Array<{ event: InputEvent; framesLeft: number }> = [];
    for (const pending of this.pendingReleases) {
      const nextFrames = pending.framesLeft - 1;
      if (nextFrames <= 0) {
        releases.push(pending.event);
      } else {
        remaining.push({ event: pending.event, framesLeft: nextFrames });
      }
    }
    this.pendingReleases = remaining;
    return releases;
  }
}

export class PromptInputAdapter extends TerminalInputAdapter {
  private readonly promptProvider?: () => string | null;

  constructor(options?: {
    stdin?: NodeJS.ReadableStream | null;
    promptProvider?: () => string | null;
  }) {
    super({ stdin: options?.stdin ?? null });
    this.promptProvider = options?.promptProvider;
  }

  poll(): InputEvent[] {
    const raw = this.promptProvider ? this.promptProvider() : null;
    if (raw === null || raw === undefined) {
      return [];
    }
    const trimmed = raw.trim();
    const tokens = trimmed ? [trimmed] : [];
    const events: InputEvent[] = [];
    for (const token of tokens) {
      events.push(...this.eventsForToken(token));
    }
    return events;
  }
}
