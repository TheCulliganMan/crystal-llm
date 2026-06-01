// ASM mapping: pokecrystal_disassembly/home/text.asm (PrintTextboxTextAt, TextboxBorder) and home/joypad.asm (JoyTextDelay).
import { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { Event, EventManager as WorldEventManager, close_text } from "@pokecrystal/core/engine/world/events";
import { GameState } from "@pokecrystal/core/core/state";
import { TextSpeed } from "@pokecrystal/core/core/enums/ui-enums";
import { resolveTextboxFrameRenderId } from "@pokecrystal/core/core/textbox-frame";
import {
  SCREEN_TILE_WIDTH,
  TEXTBOX_DELAY_FAST_FRAMES,
  TEXTBOX_DELAY_FLAG,
  TEXTBOX_FAST_DELAY_FLAG,
  TEXTBOX_HEIGHT_TILES,
  TEXTBOX_Y_TILES,
} from "@pokecrystal/core/core/text-constants";
import { TILE_SIZE } from "@pokecrystal/core/engine/world/tile";
import {
  GameButton,
  KeyEvent,
  buttonKeys,
  isButtonEvent,
  isConfirmEvent,
  isKeyDownEvent,
  isKeyUpEvent,
  normalizeButtonKey,
} from "@pokecrystal/core/input/buttons";
import { mapKeyToDirection } from "@pokecrystal/core/input/controls";
import { B_PAD_A, B_PAD_B, B_PAD_DOWN, B_PAD_LEFT, B_PAD_RIGHT, B_PAD_UP } from "@pokecrystal/core/input/controls";
import { Z_INDEX_DIALOGUE, Z_INDEX_PROMPT } from "@pokecrystal/core/ui/z-index";
import { type CompositeUI, type ScreenUI, type RGB } from "@pokecrystal/core/ui/screens/screen-types";
import type { RenderTextOptions } from "@pokecrystal/core/ui/font-renderer";
import { Surface } from "@pokecrystal/core/ui/game-engine";
import { filterPromptContextLines } from "@pokecrystal/core/ui/text/prompt-context";
import type { EventManager } from "@pokecrystal/core/engine/events/events";

const TEXT_SPEED_FRAMES: Record<string, number> = {
  fast: 1,
  mid: 3,
  slow: 5,
};

const WOPTIONS_TEXT_SPEED_MASK = 0b111;
const WOPTIONS_NO_TEXT_SCROLL_BIT = 1 << 4;

type DialogueFont = {
  render_text?: (text: string, x: number, y: number, surface: Surface, options?: RenderTextOptions) => void;
  renderText?: (text: string, x: number, y: number, surface: Surface, options?: RenderTextOptions) => void;
  charWidth?: number;
  char_width?: number;
  normalizeText?: (value: string) => string;
  _normalizeText?: (value: string) => string;
  _normalize_text?: (value: string) => string;
  wrapText?: (value: string, width: number) => string[];
  _wrapText?: (value: string, width: number) => string[];
  _wrap_text?: (value: string, width: number) => string[];
};

export type DialogueUI = {
  screen: ScreenUI["screen"];
  tile_size?: number;
  tileSize?: number;
  default_frame_id?: number;
  defaultFrameId?: number;
  // Alias of Surface.fill RGB tuples.
  // ASM: home/text.asm uses fixed palette indices; UI wrappers may override with RGB fills.
  // Keeping the parameter surface here lets callers pass `undefined` to preserve defaults.
  font?: DialogueFont;
  // Keep both snake_case and camelCase; some callers pass fill/text_color/z_index for Next UI.
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
  drawTextBox?: (
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
  draw_window?: (
    surface: Surface,
    x: number,
    y: number,
    widthTiles: number,
    heightTiles: number,
    options?: { fill?: RGB }
  ) => void;
  drawWindow?: (
    surface: Surface,
    x: number,
    y: number,
    widthTiles: number,
    heightTiles: number,
    options?: { fill?: RGB }
  ) => void;
  _record_window_region?: (
    surface: Surface,
    x: number,
    y: number,
    widthTiles: number,
    heightTiles: number,
    zIndex: number,
    sourceSurface?: Surface
  ) => void;
  renderSnapshot?: ScreenUI["renderSnapshot"];
  get_context_palette?: (name: string) => RGB[];
  getContextPalette?: (name: string) => RGB[];
};

const isDialogueScreenUI = (candidate: unknown): candidate is ScreenUI => {
  if (!candidate || typeof candidate !== "object") {
    return false;
  }
  const record = candidate as Record<string, unknown>;
  return (
    typeof record.renderSnapshot === "function" &&
    typeof record.getSnapshot === "function"
  );
};

const resolveDialogueSnapshotSource = (candidate: unknown): ScreenUI | null => {
  if (isDialogueScreenUI(candidate)) {
    return candidate;
  }
  if (!candidate || typeof candidate !== "object") {
    return null;
  }
  const record = candidate as { getChildren?: () => unknown[] };
  if (typeof record.getChildren !== "function") {
    return null;
  }
  for (const child of record.getChildren()) {
    const resolved = resolveDialogueSnapshotSource(child);
    if (resolved) {
      return resolved;
    }
  }
  return null;
};

type ScriptRunnerLike = {
  pause?: () => void;
  resume?: () => void;
  formatText?: (text: string) => string;
  format_text?: (text: string) => string;
  stop_execution?: boolean;
  stopExecution?: boolean;
  last_yes_no_result?: boolean;
  last_condition_result?: boolean;
  event_manager?: { dispatch: (event: Event) => void };
  eventManager?: { dispatch: (event: Event) => void };
  _script_stack?: Array<{ name?: string }>;
  _awaiting_resume?: number;
};

type DialogueWindowOptions = {
  mask_text?: () => boolean;
};

const maskTextWithSpaces = (text: string): string => {
  if (!text) {
    return text;
  }
  let output = "";
  for (const char of text) {
    if (char === "\n" || char === "\r") {
      output += char;
      continue;
    }
    output += " ";
  }
  return output;
};

const resolveTextSpeedFrames = (value: TextSpeed | number | undefined | null): number => {
  if (typeof value === "string") {
    return TEXT_SPEED_FRAMES[value] ?? 3;
  }
  if (typeof value === "number" && Number.isFinite(value)) {
    const rounded = Math.round(value);
    if (rounded <= 0) {
      return 3;
    }
    return Math.min(5, rounded);
  }
  return 3;
};

const autoCloseAfterWaitRequested = (data: unknown): boolean => {
  if (!data || typeof data !== "object") {
    return false;
  }
  const payload = data as {
    auto_close_after_wait?: unknown;
    autoCloseAfterWait?: unknown;
  };
  const snake = payload.auto_close_after_wait;
  const camel = payload.autoCloseAfterWait;
  if (
    snake !== undefined &&
    camel !== undefined &&
    Boolean(snake) !== Boolean(camel)
  ) {
    throw new Error(
      "show_text payload has conflicting auto_close_after_wait/autoCloseAfterWait values."
    );
  }
  return Boolean(snake ?? camel);
};

const runnerStack = (runner?: ScriptRunnerLike | null): Array<{ name?: string }> => {
  if (!runner) {
    return [];
  }
  const stack = runner._script_stack;
  return Array.isArray(stack) ? stack : [];
};

const runnerAwaitingResume = (runner?: ScriptRunnerLike | null): number => {
  if (!runner) {
    return 0;
  }
  const awaiting = runner._awaiting_resume;
  return typeof awaiting === "number" ? awaiting : 0;
};

const getTileSize = (ui: DialogueUI): number => {
  return ui.tile_size ?? ui.tileSize ?? TILE_SIZE;
};

const getTextboxPalette = (ui: DialogueUI): RGB[] | null => {
  if (ui.get_context_palette) {
    return ui.get_context_palette("textbox");
  }
  if (ui.getContextPalette) {
    return ui.getContextPalette("textbox");
  }
  return null;
};

const renderFontText = (
  ui: DialogueUI,
  text: string,
  x: number,
  y: number,
  surface: Surface,
  options?: RenderTextOptions
): void => {
  if (ui.font?.render_text) {
    ui.font.render_text(text, x, y, surface, options);
    return;
  }
  if (ui.font?.renderText) {
    ui.font.renderText(text, x, y, surface, options);
    return;
  }
  throw new Error("DialogueWindow requires a font renderer");
};

export class DialogueWindow {
  public game_state: GameState;

  private readonly ui: DialogueUI;
  private readonly screen: ScreenUI["screen"];
  private readonly lines: number;
  private readonly _audio_engine?: AudioEngine | null;
  private readonly mask_text?: () => boolean;
  private text = "";
  private visible_chars = 0;
  private timer = 0;
  private saved_textbox_flags: number | null = null;
  private pages: string[] = [""];
  private pageIndex = 0;
  private readonly render_all_pages: boolean;

  constructor(
    ui: DialogueUI,
    game_state: GameState,
    lines = 2,
    audio_engine?: AudioEngine | null,
    options: DialogueWindowOptions = {}
  ) {
    this.ui = ui;
    this.screen = ui.screen;
    this.game_state = game_state;
    this.lines = lines;
    this._audio_engine = audio_engine;
    this.mask_text = options.mask_text;
    this.render_all_pages = isDialogueScreenUI(ui);
  }

  get audio_engine(): AudioEngine | null | undefined {
    return this._audio_engine;
  }

  get visible_text(): string {
    const payload = this.text.slice(0, this.visible_chars);
    return this.mask_text?.() ? maskTextWithSpaces(payload) : payload;
  }

  get current_page_text(): string {
    return this.mask_text?.() ? maskTextWithSpaces(this.text) : this.text;
  }

  open(text: string): void {
    this.store_textbox_flags();
    this.pages = this.paginate(text);
    this.pageIndex = 0;
    this.text = this.pages[this.pageIndex] ?? "";
    this.visible_chars = 0;
    this.timer = 0;
    this.render_page_immediately_if_needed();
  }

  update(): void {
    if (this.visible_chars >= this.text.length) {
      return;
    }
    const speed = this.get_text_delay_frames();
    if (speed === 0) {
      this.visible_chars = this.text.length;
      return;
    }
    this.timer += 1;
    if (this.timer >= speed) {
      this.timer = 0;
      const previous = this.visible_chars;
      this.visible_chars = Math.min(this.text.length, this.visible_chars + 1);
      this.trigger_character_audio(previous, this.visible_chars);
    }
  }

  draw(): void {
    if (!this.screen) {
      return;
    }
    const width = SCREEN_TILE_WIDTH;
    const height = TEXTBOX_HEIGHT_TILES;
    const x = 0;
    const y = TEXTBOX_Y_TILES;
    const payload = this.text.slice(0, this.visible_chars);
    const masked = this.mask_text?.() ? maskTextWithSpaces(payload) : payload;
    if (this.ui.draw_text_box) {
      this.ui.draw_text_box(
        this.screen,
        masked,
        x,
        y,
        width,
        height,
        this.frame_id(),
        undefined,
        undefined,
        Z_INDEX_DIALOGUE,
      );
      return;
    }
    if (this.ui.drawTextBox) {
      this.ui.drawTextBox(
        this.screen,
        masked,
        x,
        y,
        width,
        height,
        this.frame_id(),
        undefined,
        undefined,
        Z_INDEX_DIALOGUE,
      );
      return;
    }
    throw new Error("DialogueWindow requires ui.drawTextBox or ui.draw_text_box");
  }

  is_complete(): boolean {
    return this.visible_chars >= this.text.length;
  }

  has_more_pages(): boolean {
    return this.pageIndex < this.pages.length - 1;
  }

  get page_index(): number {
    return this.pageIndex;
  }

  get total_pages(): number {
    return this.pages.length;
  }

  advance_page(): void {
    if (!this.has_more_pages()) {
      return;
    }
    this.pageIndex += 1;
    this.text = this.pages[this.pageIndex] ?? "";
    this.visible_chars = 0;
    this.timer = 0;
    this.render_page_immediately_if_needed();
  }

  complete(): void {
    this.visible_chars = this.text.length;
    this.timer = 0;
  }

  clear(): void {
    this.restore_textbox_flags();
    this.text = "";
    this.visible_chars = 0;
    this.timer = 0;
  }

  handle_input(event: KeyEvent): void {
    if (!isKeyDownEvent(event)) {
      return;
    }
    if (isConfirmEvent(event)) {
      if (!this.is_complete()) {
        this.complete();
      } else if (this.has_more_pages()) {
        this.advance_page();
      }
    }
  }

  private paginate(text: string): string[] {
    if (!text) {
      return [""];
    }
    const font = this.ui.font;
    if (!font) {
      return [text];
    }
    const charWidth = font.charWidth ?? font.char_width ?? TILE_SIZE;
    const usableWidthPx = (20 - 2) * TILE_SIZE;
    const f = font!;
    const normalize = (value: string) => {
      if (typeof f.normalizeText === "function") return f.normalizeText(value);
      if (typeof f._normalizeText === "function") return f._normalizeText(value);
      if (typeof f._normalize_text === "function") return f._normalize_text(value);
      return value;
    };
    const wrap = (value: string, width: number) => {
      if (typeof f.wrapText === "function") return f.wrapText(value, width);
      if (typeof f._wrapText === "function") return f._wrapText(value, width);
      if (typeof f._wrap_text === "function") return f._wrap_text(value, width);
      return null;
    };

    let normalized = normalize(text);
    if (typeof normalized !== "string") {
      normalized = String(normalized);
    }

    const wrapped = wrap ? wrap(normalized, usableWidthPx) : null;
    const fallback = this.fallback_wrap_text(
      normalized,
      Math.max(1, Math.floor(usableWidthPx / Math.max(1, charWidth)))
    );
    const lines = Array.isArray(wrapped)
      ? wrapped.length
        ? wrapped.map(String)
        : normalized
        ? fallback
        : [""]
      : typeof wrapped === "string"
      ? [wrapped]
      : fallback;

    const pages: string[] = [];
    for (let index = 0; index < lines.length; index += this.lines) {
      const pageLines = lines.slice(index, index + this.lines);
      pages.push(pageLines.join("\n"));
    }
    return pages.length ? pages : [""];
  }

  private fallback_wrap_text(text: string, maxCharsPerLine: number): string[] {
    if (maxCharsPerLine <= 0) {
      throw new Error("maxCharsPerLine must be positive");
    }
    const lines: string[] = [];
    const rawLines = text.split("\n");
    for (const rawLine of rawLines) {
      if (!rawLine) {
        lines.push("");
        continue;
      }
      const words = rawLine.split(/\s+/).filter(Boolean);
      let currentLine = "";
      for (let word of words) {
        if (word.includes("@")) {
          if (currentLine) {
            lines.push(currentLine.trimEnd());
            currentLine = "";
          }
          word = word.replace(/@/g, "");
          if (!word) {
            continue;
          }
        }
        const testLine = currentLine ? `${currentLine} ${word}` : word;
        if (testLine.length <= maxCharsPerLine) {
          currentLine = testLine;
        } else {
          if (currentLine) {
            lines.push(currentLine);
          }
          currentLine = word;
        }
      }
      if (currentLine) {
        lines.push(currentLine);
      }
    }
    return lines.length ? lines : [""];
  }

  private get_text_delay_frames(): number {
    const wram = this.game_state?.wram;
    const textboxFlags = wram?.wTextboxFlags ?? 0;
    const wOptions = this.compute_w_options_mask();
    if (this.should_skip_text_delay(textboxFlags, wOptions)) {
      return 0;
    }
    const options = this.game_state?.sram?.options;
    if (!wram) {
      return resolveTextSpeedFrames(options?.text_speed);
    }
    const flags = this.apply_text_speed_overrides(textboxFlags, wram);
    if (!(flags & TEXTBOX_FAST_DELAY_FLAG)) {
      return TEXTBOX_DELAY_FAST_FRAMES;
    }
    return resolveTextSpeedFrames(options?.text_speed);
  }

  private should_skip_text_delay(textboxFlags: number, wOptions: number): boolean {
    const options = this.game_state?.sram?.options;
    if (options?.no_text_scroll) {
      return true;
    }
    if (wOptions & WOPTIONS_NO_TEXT_SCROLL_BIT) {
      return true;
    }
    return !(textboxFlags & TEXTBOX_DELAY_FLAG);
  }

  private compute_w_options_mask(): number {
    const options = this.game_state?.sram?.options;
    const speedValue = resolveTextSpeedFrames(options?.text_speed);
    let mask = speedValue & WOPTIONS_TEXT_SPEED_MASK;
    if (options?.no_text_scroll) {
      mask |= WOPTIONS_NO_TEXT_SCROLL_BIT;
    }
    return mask;
  }

  private apply_text_speed_overrides(flags: number, wram: GameState["wram"]): number {
    if ((wram as { wDisableTextAcceleration?: boolean }).wDisableTextAcceleration) {
      return flags;
    }
    if (this.is_text_acceleration_requested()) {
      return flags & ~TEXTBOX_FAST_DELAY_FLAG;
    }
    return flags;
  }

  private render_page_immediately_if_needed(): void {
    if (!this.render_all_pages) {
      return;
    }
    this.visible_chars = this.text.length;
    this.timer = 0;
  }

  private is_text_acceleration_requested(): boolean {
    const joypad = this.game_state?.hram?.joypad;
    if (!joypad) {
      return false;
    }
    const mask = joypad.hJoyDown ?? joypad.hJoypadDown ?? 0;
    return (mask & (B_PAD_A | B_PAD_B)) !== 0;
  }

  private store_textbox_flags(): void {
    const wram = this.game_state.wram;
    this.restore_textbox_flags();
    const currentFlags = wram.wTextboxFlags ?? 0;
    this.saved_textbox_flags = currentFlags;
    wram.wTextboxFlags = currentFlags | TEXTBOX_DELAY_FLAG;
  }

  private restore_textbox_flags(): void {
    if (this.saved_textbox_flags === null) {
      return;
    }
    const wram = this.game_state.wram;
    wram.wTextboxFlags = this.saved_textbox_flags;
    this.saved_textbox_flags = null;
  }

  private trigger_character_audio(_start: number, _end: number): void {
    // ASM text SFX are emitted by explicit text commands (see TextCommand_SOUND),
    // not by the ordinary per-glyph textbox flow.
    return;
  }

  private frame_id(): number {
    const frameType = this.game_state?.sram?.options?.frame;
    const fallback = this.ui.default_frame_id ?? this.ui.defaultFrameId ?? 1;
    return resolveTextboxFrameRenderId(frameType, fallback);
  }
}

export class YesNoPrompt {
  private readonly ui: DialogueUI;
  private readonly screen: Surface | null;
  private readonly audio_engine?: AudioEngine | null;
  public selection = 0;
  public finished = false;

  // ASM: pokecrystal_disassembly/home/menu.asm::YesNoBox (lb bc, SCREEN_WIDTH - 6, 7).
  private static readonly MENU_TOP_TILE = 7;
  private static readonly MENU_WIDTH_TILES = 6;
  private static readonly MENU_HEIGHT_TILES = 4;

  constructor(ui: DialogueUI, audio_engine?: AudioEngine | null) {
    this.ui = ui;
    this.screen = ui.screen;
    this.audio_engine = audio_engine;
  }

  handle_input(event: KeyEvent): void {
    if (!isKeyDownEvent(event)) {
      return;
    }
    const direction = mapKeyToDirection(event.direction ?? event.code ?? event.key ?? null);
    if (direction) {
      this.selection = 1 - this.selection;
    } else if (isConfirmEvent(event)) {
      this.finished = true;
      this.play_confirm();
    } else if (isButtonEvent(event, GameButton.B)) {
      this.selection = 1;
      this.finished = true;
      this.play_confirm();
    }
  }

  handle_joypad(pressed: number): void {
    if (!pressed) {
      return;
    }
    const directionMask = B_PAD_UP | B_PAD_DOWN | B_PAD_LEFT | B_PAD_RIGHT;
    if (pressed & directionMask) {
      this.selection = 1 - this.selection;
    } else if (pressed & B_PAD_A) {
      this.finished = true;
      this.play_confirm();
    } else if (pressed & B_PAD_B) {
      this.selection = 1;
      this.finished = true;
      this.play_confirm();
    }
  }

  draw(): void {
    if (!this.screen) {
      return;
    }
    const tileSize = getTileSize(this.ui);
    const x = (SCREEN_TILE_WIDTH - YesNoPrompt.MENU_WIDTH_TILES) * tileSize;
    const y = YesNoPrompt.MENU_TOP_TILE * tileSize;
    const widthTiles = YesNoPrompt.MENU_WIDTH_TILES;
    const heightTiles = YesNoPrompt.MENU_HEIGHT_TILES;

    const SurfaceCtor = this.screen.constructor as new (width: number, height: number) => typeof this.screen;
    const promptSurface = new SurfaceCtor(widthTiles * tileSize, heightTiles * tileSize);
    if (!promptSurface) {
      throw new Error("Failed to create prompt surface");
    }
    if (promptSurface.fill) {
      promptSurface.fill([0, 0, 0, 0]);
    }

    const drawWindow =
      this.ui.draw_window?.bind(this.ui) ?? this.ui.drawWindow?.bind(this.ui);
    if (!drawWindow) {
      throw new Error("YesNoPrompt requires ui.drawWindow or ui.draw_window");
    }
    drawWindow(promptSurface, 0, 0, widthTiles, heightTiles, { fill: [255, 255, 255] });

    const options = ["YES", "NO"];
    options.forEach((label, index) => {
      const cursor = index === this.selection ? "▶" : " ";
      renderFontText(
        this.ui,
        `${cursor}${label}`,
        tileSize,
        (index + 1) * tileSize,
        promptSurface
      );
    });

    this.screen.blit(promptSurface, [x, y]);
    if (this.ui._record_window_region) {
      this.ui._record_window_region(this.screen, x, y, widthTiles, heightTiles, Z_INDEX_PROMPT, promptSurface);
    }

    if (this.ui.renderSnapshot) {
      const baseSnapshot =
        resolveDialogueSnapshotSource(this.ui)?.getSnapshot?.() ?? null;
      const contextLines = filterPromptContextLines(baseSnapshot?.dialogueLines).slice(-2);
      const promptLines = [
        ...contextLines,
        ...options.map((label, index) =>
          `${index === this.selection ? "▶" : " "} ${label}`
        ),
      ];
      const shouldSuppressDialogue = contextLines.length > 0;
      this.ui.renderSnapshot(
        ["Prompt"],
        ["Up/Down=Choose A=OK B=Cancel"],
        "Prompt",
        "Legend",
        baseSnapshot?.menuLines ?? null,
        promptLines,
        shouldSuppressDialogue ? null : baseSnapshot?.dialogueLines ?? null
      );
    }
  }

  result(): boolean {
    return this.selection === 0;
  }
  private play_confirm(): void {
    this.audio_engine?.playSound?.("menu_option");
  }
}

export class SelectionListPrompt {
  private readonly ui: DialogueUI;
  private readonly screen: Surface | null;
  private readonly options: string[];
  private readonly audio_engine?: AudioEngine | null;
  public selection: number;
  public finished = false;

  constructor(
    ui: DialogueUI,
    options: string[],
    settings: { initialIndex?: number; cancelIndex?: number; audioEngine?: AudioEngine | null } = {}
  ) {
    if (!options.length) {
      throw new Error("SelectionListPrompt requires at least one option");
    }
    this.ui = ui;
    this.screen = ui.screen;
    this.options = [...options];
    this.audio_engine = settings.audioEngine ?? null;
    this.selection = Math.max(
      0,
      Math.min(this.options.length - 1, Math.trunc(settings.initialIndex ?? 0)),
    );
    this.cancel_index = Math.max(
      0,
      Math.min(this.options.length - 1, Math.trunc(settings.cancelIndex ?? this.options.length - 1)),
    );
  }

  private readonly cancel_index: number;

  handle_input(event: KeyEvent): void {
    if (!isKeyDownEvent(event)) {
      return;
    }
    const direction = mapKeyToDirection(event.direction ?? event.code ?? event.key ?? null);
    if (direction === "up" || direction === "left") {
      this.selection = (this.selection - 1 + this.options.length) % this.options.length;
    } else if (direction === "down" || direction === "right") {
      this.selection = (this.selection + 1) % this.options.length;
    } else if (isConfirmEvent(event)) {
      this.finished = true;
      this.play_confirm();
    } else if (isButtonEvent(event, GameButton.B)) {
      this.selection = this.cancel_index;
      this.finished = true;
      this.play_confirm();
    }
  }

  draw(): void {
    if (!this.screen) {
      return;
    }
    const tileSize = getTileSize(this.ui);
    const maxChars = Math.max(...this.options.map((option) => option.length + 2));
    const widthTiles = Math.min(SCREEN_TILE_WIDTH, Math.max(6, maxChars + 2));
    const heightTiles = Math.min(12, this.options.length + 2);
    const x = (SCREEN_TILE_WIDTH - widthTiles) * tileSize;
    const y = Math.max(0, (TEXTBOX_Y_TILES - heightTiles) * tileSize);

    const SurfaceCtor = this.screen.constructor as new (width: number, height: number) => typeof this.screen;
    const promptSurface = new SurfaceCtor(widthTiles * tileSize, heightTiles * tileSize);
    if (!promptSurface) {
      throw new Error("Failed to create selection prompt surface");
    }
    promptSurface.fill?.([0, 0, 0, 0]);

    const drawWindow =
      this.ui.draw_window?.bind(this.ui) ?? this.ui.drawWindow?.bind(this.ui);
    if (!drawWindow) {
      throw new Error("SelectionListPrompt requires ui.drawWindow or ui.draw_window");
    }
    drawWindow(promptSurface, 0, 0, widthTiles, heightTiles, { fill: [255, 255, 255] });

    this.options.slice(0, heightTiles - 2).forEach((label, index) => {
      renderFontText(
        this.ui,
        `${index === this.selection ? "▶" : " "} ${label}`,
        tileSize,
        (index + 1) * tileSize,
        promptSurface,
      );
    });

    this.screen.blit(promptSurface, [x, y]);
    this.ui._record_window_region?.(this.screen, x, y, widthTiles, heightTiles, Z_INDEX_PROMPT, promptSurface);
    this.render_snapshot();
  }

  result(): number {
    return this.selection;
  }

  snapshot_lines(): string[] {
    return this.options.map((label, index) => `${index === this.selection ? "▶" : " "} ${label}`);
  }

  private render_snapshot(): void {
    if (!this.ui.renderSnapshot) {
      return;
    }
    const baseSnapshot = resolveDialogueSnapshotSource(this.ui)?.getSnapshot?.() ?? null;
    this.ui.renderSnapshot(
      ["Prompt"],
      ["Up/Down=Choose A=OK B=Cancel"],
      "Prompt",
      "Legend",
      baseSnapshot?.menuLines ?? null,
      this.snapshot_lines(),
      baseSnapshot?.dialogueLines ?? null,
    );
  }

  private play_confirm(): void {
    this.audio_engine?.playSound?.("menu_option");
  }
}

export class FieldDialogueManager {
  private readonly ui: DialogueUI;
  private readonly window: DialogueWindow;
  private readonly script_runner: ScriptRunnerLike | null;
  private readonly pending_text: string[] = [];
  private readonly event_callbacks: Array<(event: Event, game_state: GameState) => void> = [];
  private readonly confirm_keys = new Set(buttonKeys[GameButton.A]);

  public visible = false;
  public waiting_for_input = false;

  private script_paused = false;
  private current_text = "";
  private pendingWaits = 0;
  private audio_engine?: AudioEngine | null;
  private yes_no_prompt: YesNoPrompt | null = null;
  private yes_no_callback: ((result: boolean) => void) | null = null;
  private selection_prompt: SelectionListPrompt | null = null;
  private selection_callback: ((result: number) => void) | null = null;
  private suspended = false;
  private ignore_confirm_until_release = false;
  private auto_close_requested = false;
  private pending_script_waits = 0;
  public _suppress_orphan_close = false;
  private pending_yes_no_request = false;
  private pending_yes_no_callback: ((result: boolean) => void) | null = null;
  private pending_selection_request: {
    options: string[];
    initialIndex: number;
    cancelIndex: number;
    callback: ((result: number) => void) | null;
  } | null = null;

  constructor(
    ui: DialogueUI,
    game_state: GameState,
    script_runner: ScriptRunnerLike,
    audio_engine?: AudioEngine | null,
    options: DialogueWindowOptions = {}
  ) {
    this.ui = ui;
    this.window = new DialogueWindow(ui, game_state, 2, audio_engine ?? undefined, options);
    this.script_runner = script_runner;
    this.audio_engine = audio_engine;
  }

  get active(): boolean {
    return this.visible;
  }

  get pending_waits_count(): number {
    return this.pendingWaits;
  }

  get pending_waits(): number {
    return this.pendingWaits;
  }

  get pending_text_count(): number {
    return this.pending_text.length;
  }

  get pending_script_waits_count(): number {
    return this.pending_script_waits;
  }

  get is_script_paused(): boolean {
    return this.script_paused;
  }

  get _yes_no_prompt(): { selection: number } | null {
    if (!this.yes_no_prompt) {
      return null;
    }
    return { selection: this.yes_no_prompt.selection };
  }

  get _selection_prompt(): { selection: number; lines: string[] } | null {
    if (!this.selection_prompt) {
      return null;
    }
    return {
      selection: this.selection_prompt.selection,
      lines: this.selection_prompt.snapshot_lines(),
    };
  }

  get script_runner_instance(): ScriptRunnerLike | null {
    return this.script_runner;
  }

  handle_event(event: Event, game_state: GameState): void {
    if (this.suspended) {
      return;
    }
    switch (event.name) {
      case "open_text":
        this.handle_open_text();
        break;
      case "close_text":
        this.handle_close_text(false);
        break;
      case "show_text": {
        const text = event.data?.text ?? "";
        if (autoCloseAfterWaitRequested(event.data)) {
          this.auto_close_requested = true;
        }
        this.queue_text(this.format_event_text(String(text)));
        break;
      }
      case "wait_for_input": {
        const data = event.data as
          | { pause_runner?: boolean; pauseRunner?: boolean }
          | boolean
          | null
          | undefined;
        const pause_runner = Boolean(
          typeof data === "boolean" ? data : data?.pause_runner ?? data?.pauseRunner
        );
        if (pause_runner) {
          this.script_runner?.pause?.();
        }
        this.pendingWaits += 1;
        this.waiting_for_input = true;
        this.script_paused = true;
        this.ignore_confirm_until_release = true;
        if (runnerStack(this.script_runner).length || runnerAwaitingResume(this.script_runner) > 0) {
          this.pending_script_waits += 1;
        }
        break;
      }
      case "prompt_yes_no":
        this.handle_prompt_yes_no(event);
        break;
      case "prompt_selection":
        this.handle_prompt_selection(event);
        break;
      default:
        break;
    }

    for (const callback of this.event_callbacks) {
      callback(event, game_state);
    }
  }

  register_event_callback(callback: (event: Event, game_state: GameState) => void): void {
    this.event_callbacks.push(callback);
  }

  suspend(): void {
    if (this.suspended) {
      return;
    }
    this.suspended = true;
    this.handle_close_text(true);
  }

  resume(): void {
    if (!this.suspended) {
      return;
    }
    this.suspended = false;
  }

  clear_script_waits(): void {
    this.pendingWaits = 0;
    this.pending_script_waits = 0;
    this.script_paused = false;
    this.waiting_for_input = false;
  }

  acknowledge_wait(): boolean {
    if (this.pendingWaits === 0 && !this.waiting_for_input) {
      return false;
    }
    if (this.pendingWaits > 0) {
      this.pendingWaits = Math.max(0, this.pendingWaits - 1);
    }
    this.waiting_for_input = this.pendingWaits > 0;
    this.script_paused = false;
    if (this.pending_script_waits > 0) {
      this.pending_script_waits = Math.max(0, this.pending_script_waits - 1);
    }
    if (this.auto_close_requested && !this.waiting_for_input) {
      this.auto_close_requested = false;
      const eventManager = this.script_runner?.event_manager ?? this.script_runner?.eventManager;
      if (eventManager) {
        close_text(eventManager as WorldEventManager);
      }
    }
    return true;
  }

  update(): void {
    if (this.suspended || !this.visible) {
      return;
    }
    this.window.update();
    this.maybe_activate_yes_no_prompt();
    if (this.window.is_complete() && !this.waiting_for_input && this.pending_text.length) {
      this.show_next_text();
    } else if (
      this.auto_close_requested &&
      this.window.is_complete() &&
      !this.waiting_for_input &&
      !this.pending_text.length &&
      !this.pending_yes_no_request &&
      !this.yes_no_prompt &&
      !this.pending_selection_request &&
      !this.selection_prompt
    ) {
      const eventManager = this.script_runner?.event_manager ?? this.script_runner?.eventManager;
      this.auto_close_requested = false;
      if (eventManager) {
        close_text(eventManager as WorldEventManager);
      } else {
        this.handle_close_text(true);
      }
    } else if (
      this.script_paused &&
      this.window.is_complete() &&
      !this.waiting_for_input &&
      !this.pending_text.length
    ) {
      this.script_paused = false;
      const runner = this.script_runner;
      if (runner && (runner.stop_execution || runner.stopExecution)) {
        runner.resume?.();
      }
    }
  }

  draw(): void {
    if (this.suspended || !this.visible) {
      return;
    }
    this.window.draw();
    this.draw_prompt_cursor();
    this.yes_no_prompt?.draw();
    this.selection_prompt?.draw();
  }

  handle_input(event: KeyEvent): boolean {
    if (this.suspended) {
      return false;
    }
    if (this.clear_stale_blank_wait_if_needed()) {
      return isKeyDownEvent(event) || isKeyUpEvent(event);
    }
    if (isKeyUpEvent(event)) {
      const keyCode = normalizeButtonKey(event.code ?? event.key ?? null);
      if (keyCode !== null && this.confirm_keys.has(keyCode)) {
        this.ignore_confirm_until_release = false;
      }
      return false;
    }
    if (this.yes_no_prompt && isKeyDownEvent(event)) {
      if (this.ignore_confirm_until_release && isConfirmEvent(event)) {
        return true;
      }
      const prompt = this.yes_no_prompt;
      prompt.handle_input(event);
      if (prompt.finished) {
        const result = prompt.result();
        const runner = this.script_runner;
        if (runner) {
          runner.last_yes_no_result = result;
          runner.last_condition_result = result;
        }
        if (this.pendingWaits > 0) {
          this.pendingWaits = Math.max(0, this.pendingWaits - 1);
        }
        this.waiting_for_input = this.pendingWaits > 0;
        this.script_paused = false;
        if (this.pending_script_waits > 0) {
          this.pending_script_waits = Math.max(0, this.pending_script_waits - 1);
        }
        const callback = this.yes_no_callback;
        this.yes_no_prompt = null;
        this.yes_no_callback = null;
        if (callback) {
          callback(result);
        }
        const stackDepth = runnerStack(runner).length;
        const awaitingResume = runnerAwaitingResume(runner);
        if (stackDepth > 0 || awaitingResume > 0) {
          runner?.resume?.();
        }
      }
      return true;
    }
    if (this.selection_prompt && isKeyDownEvent(event)) {
      if (this.ignore_confirm_until_release && isConfirmEvent(event)) {
        return true;
      }
      const prompt = this.selection_prompt;
      prompt.handle_input(event);
      if (prompt.finished) {
        const result = prompt.result();
        if (this.pendingWaits > 0) {
          this.pendingWaits = Math.max(0, this.pendingWaits - 1);
        }
        this.waiting_for_input = this.pendingWaits > 0;
        this.script_paused = false;
        if (this.pending_script_waits > 0) {
          this.pending_script_waits = Math.max(0, this.pending_script_waits - 1);
        }
        const callback = this.selection_callback;
        this.selection_prompt = null;
        this.selection_callback = null;
        callback?.(result);
        const runner = this.script_runner;
        const stackDepth = runnerStack(runner).length;
        const awaitingResume = runnerAwaitingResume(runner);
        if (stackDepth > 0 || awaitingResume > 0) {
          runner?.resume?.();
        }
      }
      return true;
    }

    if (!this.visible && !this.waiting_for_input) {
      return false;
    }
    if (!isKeyDownEvent(event)) {
      return false;
    }
    if (this.ignore_confirm_until_release && isConfirmEvent(event)) {
      return true;
    }
    if (!isConfirmEvent(event)) {
      return true;
    }

    let advancedPage = false;
    if (!this.window.is_complete()) {
      this.window.complete();
      if ((this.waiting_for_input || this.pendingWaits > 0) && this.window.has_more_pages()) {
        this.window.advance_page();
        advancedPage = true;
      }
    }
    if (advancedPage) {
      return true;
    }
    if (this.waiting_for_input || this.pendingWaits > 0) {
      if (this.window.has_more_pages()) {
        this.window.advance_page();
        return true;
      }
      if (this.pending_text.length) {
        this.show_next_text();
        if (this.pending_text.length) {
          return true;
        }
      }
      let consumedWait = false;
      if (this.pendingWaits > 0) {
        this.pendingWaits = Math.max(0, this.pendingWaits - 1);
        consumedWait = true;
      }
      this.waiting_for_input = this.pendingWaits > 0;
      if (consumedWait) {
        this.script_paused = false;
        if (this.pending_script_waits > 0) {
          this.pending_script_waits = Math.max(0, this.pending_script_waits - 1);
          const stackDepth = runnerStack(this.script_runner).length;
          const awaitingResume = runnerAwaitingResume(this.script_runner);
          if (stackDepth > 0 || awaitingResume > 0) {
            this.script_runner?.resume?.();
          }
        }
        this.maybe_activate_yes_no_prompt();
        this.maybe_activate_selection_prompt();
        if (this.pending_yes_no_request || this.yes_no_prompt || this.pending_selection_request || this.selection_prompt) {
          return true;
        }
        if (this.auto_close_requested && !this.waiting_for_input && !this.pending_text.length) {
          this.auto_close_requested = false;
          const eventManager = this.script_runner?.event_manager ?? this.script_runner?.eventManager;
          if (eventManager) {
            close_text(eventManager as WorldEventManager);
          }
        }
        return true;
      }
    }

    const stackDepth = runnerStack(this.script_runner).length;
    const awaitingResume = runnerAwaitingResume(this.script_runner);
    if (
      stackDepth === 0 &&
      awaitingResume === 0 &&
      this.window.is_complete() &&
      !this.pending_text.length &&
      !this._suppress_orphan_close
    ) {
      const eventManager = this.script_runner?.event_manager ?? this.script_runner?.eventManager;
      if (eventManager) {
        close_text(eventManager as WorldEventManager);
      } else {
        this.handle_close_text(true);
      }
      return true;
    }
    if (this.script_runner?.stop_execution || this.script_runner?.stopExecution) {
      this.script_paused = false;
      this.script_runner?.resume?.();
      return true;
    }
    return true;
  }

  private handle_open_text(): void {
    this.visible = true;
    this.waiting_for_input = false;
    this.script_paused = false;
    this.pending_text.length = 0;
    this.window.clear();
    this.current_text = "";
    this.pendingWaits = 0;
    this.pending_script_waits = 0;
    this.ignore_confirm_until_release = false;
  }

  public forceCloseText(): void {
    this.handle_close_text(true);
  }

  public _handle_close_text(options?: { force?: boolean }): void {
    this.handle_close_text(Boolean(options?.force));
  }

  private handle_close_text(force: boolean): void {
    if (!force && (this.waiting_for_input || this.pendingWaits > 0 || this.pending_text.length)) {
      this.auto_close_requested = true;
      return;
    }
    this.visible = false;
    this.waiting_for_input = false;
    this.script_paused = false;
    this.pending_text.length = 0;
    this.window.clear();
    this.current_text = "";
    this.pendingWaits = 0;
    this.pending_script_waits = 0;
    this.ignore_confirm_until_release = false;
    this.yes_no_prompt = null;
    this.yes_no_callback = null;
    this.selection_prompt = null;
    this.selection_callback = null;
    this.auto_close_requested = false;
    this.pending_yes_no_request = false;
    this.pending_yes_no_callback = null;
    this.pending_selection_request = null;
  }

  private handle_prompt_yes_no(event: Event): void {
    if (this.yes_no_prompt || this.pending_yes_no_request) {
      return;
    }
    const callback =
      typeof event.data?.callback === "function"
        ? (event.data.callback as (result: boolean) => void)
        : null;
    this.visible = true;
    this.waiting_for_input = true;
    this.script_paused = true;
    if (!this.is_yes_no_ready()) {
      this.pending_yes_no_request = true;
      this.pending_yes_no_callback = callback;
      return;
    }
    this.activate_yes_no_prompt(callback);
  }

  private handle_prompt_selection(event: Event): void {
    if (this.selection_prompt || this.pending_selection_request) {
      return;
    }
    const rawOptions = event.data?.options;
    const options = Array.isArray(rawOptions)
      ? rawOptions.map((option) => String(option)).filter((option) => option.length > 0)
      : [];
    if (!options.length) {
      return;
    }
    const callback =
      typeof event.data?.callback === "function"
        ? (event.data.callback as (result: number) => void)
        : null;
    const initialIndex = Number(event.data?.initial_index ?? event.data?.initialIndex ?? 0);
    const cancelIndex = Number(event.data?.cancel_index ?? event.data?.cancelIndex ?? options.length - 1);
    this.visible = true;
    this.waiting_for_input = true;
    this.script_paused = true;
    this.pending_selection_request = {
      options,
      initialIndex: Number.isFinite(initialIndex) ? initialIndex : 0,
      cancelIndex: Number.isFinite(cancelIndex) ? cancelIndex : options.length - 1,
      callback,
    };
    this.maybe_activate_selection_prompt();
  }

  private is_yes_no_ready(): boolean {
    if (this.pending_text.length) {
      return false;
    }
    if (this.window.has_more_pages()) {
      return false;
    }
    return this.window.is_complete();
  }

  private activate_yes_no_prompt(callback: ((result: boolean) => void) | null): void {
    const audio = this.audio_engine ?? this.window.audio_engine ?? null;
    this.yes_no_prompt = new YesNoPrompt(this.ui, audio);
    this.yes_no_callback = callback;
    this.pending_yes_no_request = false;
    this.pending_yes_no_callback = null;
    this.visible = true;
    this.waiting_for_input = true;
    this.script_paused = true;
    this.ignore_confirm_until_release = false;
  }

  private maybe_activate_yes_no_prompt(): void {
    if (!this.pending_yes_no_request) {
      return;
    }
    if (!this.is_yes_no_ready()) {
      return;
    }
    this.activate_yes_no_prompt(this.pending_yes_no_callback);
  }

  private maybe_activate_selection_prompt(): void {
    if (!this.pending_selection_request || !this.is_yes_no_ready()) {
      return;
    }
    const request = this.pending_selection_request;
    const audio = this.audio_engine ?? this.window.audio_engine ?? null;
    this.selection_prompt = new SelectionListPrompt(this.ui, request.options, {
      initialIndex: request.initialIndex,
      cancelIndex: request.cancelIndex,
      audioEngine: audio,
    });
    this.selection_callback = request.callback;
    this.pending_selection_request = null;
    this.visible = true;
    this.waiting_for_input = true;
    this.script_paused = true;
    this.ignore_confirm_until_release = false;
  }

  private is_choice_input_held(): boolean {
    const joypad = this.window?.game_state?.hram?.joypad ?? null;
    if (!joypad) {
      return false;
    }
    const mask = joypad.hJoyDown ?? joypad.hJoypadDown ?? 0;
    return (mask & (B_PAD_A | B_PAD_B)) !== 0;
  }

  private queue_text(text: string): void {
    if (!text) {
      return;
    }
    this.visible = true;
    this.pending_text.push(text);
    if (this.current_text && !this.window.is_complete()) {
      return;
    }
    this.show_next_text();
  }

  private show_next_text(): void {
    if (!this.pending_text.length) {
      this.window.clear();
      this.current_text = "";
      return;
    }
    const nextText = this.pending_text.shift() ?? "";
    this.window.open(nextText);
    this.current_text = nextText;
    this.waiting_for_input = this.pendingWaits > 0;
  }

  private clear_stale_blank_wait_if_needed(): boolean {
    if (!this.waiting_for_input || this.pendingWaits > 0) {
      return false;
    }
    if (
      this.current_text.trim() ||
      this.pending_text.length ||
      this.window.current_page_text.trim() ||
      this.window.has_more_pages() ||
      this.yes_no_prompt ||
      this.selection_prompt ||
      this.pending_yes_no_request ||
      this.pending_selection_request
    ) {
      return false;
    }
    this.waiting_for_input = false;
    this.script_paused = false;
    this.pending_script_waits = 0;
    this.ignore_confirm_until_release = false;
    this.auto_close_requested = false;
    this.visible = false;
    this.window.clear();
    const runner = this.script_runner;
    if (runner && (runner.stop_execution || runner.stopExecution || runnerAwaitingResume(runner) > 0)) {
      runner.resume?.();
    }
    return true;
  }

  private format_event_text(text: string): string {
    return this.script_runner?.formatText?.(text) ?? this.script_runner?.format_text?.(text) ?? text;
  }

  private draw_prompt_cursor(): void {
    if (this.yes_no_prompt) {
      return;
    }
    const window = this.window;
    if (!window.is_complete()) {
      return;
    }
    const shouldShow =
      this.waiting_for_input ||
      this.pendingWaits > 0 ||
      window.has_more_pages() ||
      this.pending_text.length > 0;
    if (!shouldShow) {
      return;
    }
    // ASM mapping: pokecrystal_disassembly/home/text.asm LoadBlinkingCursor
    // and pokecrystal_disassembly/home/joypad.asm PromptButton blink cadence.
    const frame = this.window.game_state?.frame_counter ?? 0;
    if ((frame & (1 << 4)) === 0) {
      return;
    }
    const tileSize = getTileSize(this.ui);
    const x = (SCREEN_TILE_WIDTH - 2) * tileSize;
    const y = (TEXTBOX_Y_TILES + TEXTBOX_HEIGHT_TILES - 1) * tileSize;
    const palette = getTextboxPalette(this.ui);
    const color = palette?.length ? palette[palette.length - 1] : undefined;
    if (!this.ui.screen) {
      return;
    }
    renderFontText(this.ui, "▼", x, y, this.ui.screen, {
      color,
      palette: palette ?? undefined,
      max_lines: 1,
    });
  }
}
