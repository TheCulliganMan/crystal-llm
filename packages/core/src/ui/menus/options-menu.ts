// ASM mapping: engine/menus/options_menu.asm.
import { gameEngine } from "../game-engine";
import { AudioEngine } from "../../engine/systems/audio";
import { GameState } from "../../core/state";
import { BattleScene, BattleStyle } from "../../core/enums/battle";
import { FrameType, MenuAccount, PrintOption, Sound, TextSpeed, frameTypeRenderId, orderedFrameTypes, orderedPrintOptions } from "../../core/enums/ui-enums";
import { TILE_SIZE } from "../../engine/world/tile";
import { KeyEvent, isCancelEvent, isConfirmEvent, isKeyDownEvent, isKeyUpEvent } from "../../input/buttons";
import type { Options as CoreOptions } from "../../core/models";
import type { MenuUI } from "@pokecrystal/core/ui/menus/types";
import { renderFontText } from "../text/render-font";

const TextSpeed_VALUES = new Set(Object.values(TextSpeed));
const BattleScene_VALUES = new Set(Object.values(BattleScene));
const BattleStyle_VALUES = new Set(Object.values(BattleStyle));
const Sound_VALUES = new Set(Object.values(Sound));
const MenuAccount_VALUES = new Set(Object.values(MenuAccount));
const PrintOption_VALUES = new Set(Object.values(PrintOption));
const FrameType_VALUES = new Set(Object.values(FrameType));

const TEXT_SPEED_DELAY_FRAMES: Record<TextSpeed, number> = {
  [TextSpeed.FAST]: 1,
  [TextSpeed.MID]: 3,
  [TextSpeed.SLOW]: 5,
};

const TEXT_SPEED_BITS: Record<TextSpeed, number> = {
  [TextSpeed.FAST]: 0x01,
  [TextSpeed.MID]: 0x03,
  [TextSpeed.SLOW]: 0x05,
};

const TEXT_SPEED_STRINGS: Record<TextSpeed, string> = {
  [TextSpeed.FAST]: "FAST",
  [TextSpeed.MID]: "MID ",
  [TextSpeed.SLOW]: "SLOW",
};

const BATTLE_SCENE_STRINGS: Record<BattleScene, string> = {
  [BattleScene.ON]: "ON ",
  [BattleScene.OFF]: "OFF",
};

const BATTLE_STYLE_STRINGS: Record<BattleStyle, string> = {
  [BattleStyle.SHIFT]: "SHIFT",
  [BattleStyle.SET]: "SET  ",
};

const SOUND_STRINGS: Record<Sound, string> = {
  [Sound.MONO]: "MONO  ",
  [Sound.STEREO]: "STEREO",
};

const PRINT_STRINGS: Record<PrintOption, string> = {
  [PrintOption.LIGHTEST]: "LIGHTEST",
  [PrintOption.LIGHTER]: "LIGHTER ",
  [PrintOption.NORMAL]: "NORMAL  ",
  [PrintOption.DARKER]: "DARKER  ",
  [PrintOption.DARKEST]: "DARKEST ",
};

const MENU_ACCOUNT_STRINGS: Record<MenuAccount, string> = {
  [MenuAccount.ON]: "ON ",
  [MenuAccount.OFF]: "OFF",
};

const STRING_OPTIONS =
  "TEXT SPEED\n" +
  "        :\n" +
  "BATTLE SCENE\n" +
  "        :\n" +
  "BATTLE STYLE\n" +
  "        :\n" +
  "SOUND\n" +
  "        :\n" +
  "PRINT\n" +
  "        :\n" +
  "MENU ACCOUNT\n" +
  "        :\n" +
  "FRAME\n" +
  "        :TYPE\n" +
  "CANCEL";

const REPEAT_INITIAL_DELAY = 15;
const REPEAT_INTERVAL = 4;

const NO_TEXT_SCROLL_BIT = 1 << 4;
const STEREO_BIT = 1 << 5;
const BATTLE_SHIFT_BIT = 1 << 6;
const BATTLE_SCENE_BIT = 1 << 7;
const MENU_ACCOUNT_BIT = 1 << 0;

enum OptionIndex {
  TEXT_SPEED = 0,
  BATTLE_SCENE = 1,
  BATTLE_STYLE = 2,
  SOUND = 3,
  PRINT = 4,
  MENU_ACCOUNT = 5,
  FRAME = 6,
  CANCEL = 7,
}

type Options = CoreOptions;

type OptionsUI = MenuUI & {
  screenWidth?: number;
  screenHeight?: number;
  setDefaultFrame?: (frameId: number) => void;
};

const normalizeOptions = (raw: unknown): Options => {
  const data = (raw && typeof raw === "object") ? (raw as Partial<Options>) : {};
  const resolveTextSpeed = (): TextSpeed => {
    const value = data.text_speed;
    if (value !== undefined && TextSpeed_VALUES.has(value as TextSpeed)) {
      return value as TextSpeed;
    }
    if (typeof value === "number") {
      if (value <= 1) {
        return TextSpeed.FAST;
      }
      if (value <= 3) {
        return TextSpeed.MID;
      }
      return TextSpeed.SLOW;
    }
    return TextSpeed.MID;
  };

  const resolveBattleScene = (): BattleScene => {
    const value = data.battle_scene;
    if (value !== undefined && BattleScene_VALUES.has(value as BattleScene)) {
      return value as BattleScene;
    }
    if (typeof value === "boolean") {
      return value ? BattleScene.ON : BattleScene.OFF;
    }
    return BattleScene.ON;
  };

  const resolveBattleStyle = (): BattleStyle => {
    const value = data.battle_style;
    if (value !== undefined && BattleStyle_VALUES.has(value as BattleStyle)) {
      return value as BattleStyle;
    }
    if (typeof value === "boolean") {
      return value ? BattleStyle.SHIFT : BattleStyle.SET;
    }
    return BattleStyle.SHIFT;
  };

  const resolveSound = (): Sound => {
    const value = data.sound;
    if (value !== undefined && Sound_VALUES.has(value as Sound)) {
      return value as Sound;
    }
    if (typeof value === "boolean") {
      return value ? Sound.STEREO : Sound.MONO;
    }
    return Sound.STEREO;
  };

  const resolveMenuAccount = (): MenuAccount => {
    const value = data.menu_account;
    if (value !== undefined && MenuAccount_VALUES.has(value as MenuAccount)) {
      return value as MenuAccount;
    }
    if (typeof value === "boolean") {
      return value ? MenuAccount.ON : MenuAccount.OFF;
    }
    return MenuAccount.ON;
  };

  const resolvePrintOption = (): PrintOption => {
    const value = data.print_option;
    if (value !== undefined && PrintOption_VALUES.has(value as PrintOption)) {
      return value as PrintOption;
    }
    return PrintOption.NORMAL;
  };

  const resolveFrame = (): FrameType => {
    const value = data.frame;
    if (value !== undefined && FrameType_VALUES.has(value as FrameType)) {
      return value as FrameType;
    }
    if (typeof value === "number") {
      const clamped = Math.max(0, Math.min(value, orderedFrameTypes().length));
      if (clamped >= 1 && clamped <= orderedFrameTypes().length) {
        return orderedFrameTypes()[clamped - 1];
      }
    }
    return FrameType.FRAME_1;
  };

  return {
    text_speed: resolveTextSpeed(),
    battle_scene: resolveBattleScene(),
    battle_style: resolveBattleStyle(),
    sound: resolveSound(),
    menu_account: resolveMenuAccount(),
    print_option: resolvePrintOption(),
    frame: resolveFrame(),
    no_text_scroll: Boolean(data.no_text_scroll),
  };
};

export class OptionsMenu {
  private selection: OptionIndex = OptionIndex.TEXT_SPEED;
  private heldDirections = new Set<string>();
  private repeatTimers = new Map<string, number>();

  constructor(
    private readonly ui: OptionsUI,
    private readonly audioEngine: AudioEngine | null,
    private readonly gameState: GameState,
  ) {
    this.applyOptionEffects();
  }

  private get options(): Options {
    return normalizeOptions(this.gameState.sram.options);
  }

  reset(): void {
    this.selection = OptionIndex.TEXT_SPEED;
    this.heldDirections.clear();
    this.repeatTimers.clear();
    this.applyOptionEffects();
  }

  resetOptions(options: Options): void {
    this.gameState.sram.options = options;
    this.reset();
  }

  handleInput(event: KeyEvent): string | null {
    if (isKeyDownEvent(event)) {
      if (event.key === undefined) {
        return null;
      }
      if (isCancelEvent(event)) {
        return this.exitWithTransaction();
      }
      if (isConfirmEvent(event) && this.selection === OptionIndex.CANCEL) {
        return this.exitWithTransaction();
      }
      const direction = this.directionForKey(event.key);
      if (direction) {
        if (!this.heldDirections.has(direction)) {
          this.heldDirections.add(direction);
          this.repeatTimers.set(direction, REPEAT_INITIAL_DELAY);
        }
        this.handleDirection(direction);
        return null;
      }
      return null;
    }
    if (isKeyUpEvent(event)) {
      const direction = this.directionForKey(event.key ?? null);
      if (direction) {
        this.heldDirections.delete(direction);
        this.repeatTimers.delete(direction);
      }
    }
    return null;
  }

  update(): void {
    for (const direction of Array.from(this.heldDirections)) {
      const timer = (this.repeatTimers.get(direction) ?? 0) - 1;
      if (timer <= 0) {
        this.handleDirection(direction);
        this.repeatTimers.set(direction, REPEAT_INTERVAL);
      } else {
        this.repeatTimers.set(direction, timer);
      }
    }
  }

  draw(): void {
    if (!this.ui.screen) {
      return;
    }
    const palette = this.textboxPalette();
    this.ui.screen.fill([palette[1][0], palette[1][1], palette[1][2], 255]);
    const screenWidth = this.ui.screenWidth ?? this.ui.screen?.width ?? 160;
    const screenHeight = this.ui.screenHeight ?? this.ui.screen?.height ?? 144;
    const screenTilesX = Math.max(1, Math.floor(screenWidth / TILE_SIZE));
    const screenTilesY = Math.max(1, Math.floor(screenHeight / TILE_SIZE));
    const frameId = frameTypeRenderId(this.options.frame);
    this.ui.drawWindow(this.ui.screen, 0, 0, screenTilesX, screenTilesY, {
      frameId,
      fill: palette[0],
    });
    this.drawLabels();
    this.drawValues();
    this.drawCursor();
  }

  getTextMenuLines(): string[] {
    const options = this.options;
    const frameIndex = orderedFrameTypes().indexOf(options.frame);
    const frameValue = frameIndex >= 0 ? String(frameIndex + 1) : String(frameTypeRenderId(options.frame));
    const entries: Array<{ index: OptionIndex; label: string; value: string | null }> = [
      { index: OptionIndex.TEXT_SPEED, label: "TEXT SPEED", value: TEXT_SPEED_STRINGS[options.text_speed] },
      { index: OptionIndex.BATTLE_SCENE, label: "BATTLE SCENE", value: BATTLE_SCENE_STRINGS[options.battle_scene] },
      { index: OptionIndex.BATTLE_STYLE, label: "BATTLE STYLE", value: BATTLE_STYLE_STRINGS[options.battle_style] },
      { index: OptionIndex.SOUND, label: "SOUND", value: SOUND_STRINGS[options.sound] },
      { index: OptionIndex.PRINT, label: "PRINT", value: PRINT_STRINGS[options.print_option] },
      { index: OptionIndex.MENU_ACCOUNT, label: "MENU ACCOUNT", value: MENU_ACCOUNT_STRINGS[options.menu_account] },
      { index: OptionIndex.FRAME, label: "FRAME", value: frameValue },
      { index: OptionIndex.CANCEL, label: "CANCEL", value: null },
    ];
    return entries.map((entry) => {
      const prefix = entry.index === this.selection ? "\u25b6" : " ";
      const suffix = entry.value ? `: ${entry.value}` : "";
      return `${prefix} ${entry.label}${suffix}`;
    });
  }

  private directionForKey(key: string | number | null | undefined): string | null {
    const value = key ? String(key) : null;
    if (!value) {
      return null;
    }
    const mapping: Record<string, string> = {
      ArrowUp: "up",
      ArrowDown: "down",
      ArrowLeft: "left",
      ArrowRight: "right",
    };
    return mapping[value] ?? null;
  }

  private handleDirection(direction: string): void {
    if (direction === "up" || direction === "down") {
      this.moveSelection(direction === "up" ? -1 : 1);
    } else {
      this.adjustValue(direction === "left" ? -1 : 1);
    }
  }

  private moveSelection(delta: number): void {
    const current = this.selection;
    if (delta > 0) {
      this.selection = current === OptionIndex.CANCEL ? OptionIndex.TEXT_SPEED : (current + 1) as OptionIndex;
    } else {
      if (current === OptionIndex.FRAME) {
        this.selection = OptionIndex.MENU_ACCOUNT;
      } else if (current === OptionIndex.TEXT_SPEED) {
        this.selection = OptionIndex.CANCEL;
      } else {
        this.selection = (current - 1) as OptionIndex;
      }
    }
  }

  private adjustValue(delta: number): void {
    if (this.selection === OptionIndex.CANCEL) {
      return;
    }
    let changed = false;
    if (this.selection === OptionIndex.TEXT_SPEED) {
      changed = this.cycleTextSpeed(delta);
    } else if (this.selection === OptionIndex.BATTLE_SCENE) {
      changed = this.toggleBattleScene();
    } else if (this.selection === OptionIndex.BATTLE_STYLE) {
      changed = this.toggleBattleStyle();
    } else if (this.selection === OptionIndex.SOUND) {
      changed = this.toggleSound();
    } else if (this.selection === OptionIndex.PRINT) {
      changed = this.cyclePrintOption(delta);
    } else if (this.selection === OptionIndex.MENU_ACCOUNT) {
      changed = this.toggleMenuAccount();
    } else if (this.selection === OptionIndex.FRAME) {
      changed = this.cycleFrame(delta);
    }
    if (changed) {
      this.applyOptionEffects();
    }
  }

  private cycleTextSpeed(delta: number): boolean {
    const order = [TextSpeed.FAST, TextSpeed.MID, TextSpeed.SLOW];
    const current = this.options.text_speed;
    const index = Math.max(0, order.indexOf(current));
    const newValue = order[(index + delta + order.length) % order.length];
    if (newValue === current) {
      return false;
    }
    this.gameState.sram.options.text_speed = newValue;
    return true;
  }

  private toggleBattleScene(): boolean {
    const current = this.options.battle_scene;
    const next = current === BattleScene.ON ? BattleScene.OFF : BattleScene.ON;
    this.gameState.sram.options.battle_scene = next;
    return next !== current;
  }

  private toggleBattleStyle(): boolean {
    const current = this.options.battle_style;
    const next = current === BattleStyle.SHIFT ? BattleStyle.SET : BattleStyle.SHIFT;
    this.gameState.sram.options.battle_style = next;
    return next !== current;
  }

  private toggleSound(): boolean {
    const current = this.options.sound;
    const next = current === Sound.STEREO ? Sound.MONO : Sound.STEREO;
    this.gameState.sram.options.sound = next;
    return next !== current;
  }

  private cyclePrintOption(delta: number): boolean {
    const order = orderedPrintOptions();
    const current = this.options.print_option;
    const index = Math.max(0, order.indexOf(current));
    const newValue = order[(index + delta + order.length) % order.length];
    if (newValue === current) {
      return false;
    }
    this.gameState.sram.options.print_option = newValue;
    return true;
  }

  private toggleMenuAccount(): boolean {
    const current = this.options.menu_account;
    const next = current === MenuAccount.ON ? MenuAccount.OFF : MenuAccount.ON;
    this.gameState.sram.options.menu_account = next;
    return next !== current;
  }

  private cycleFrame(delta: number): boolean {
    const order = orderedFrameTypes();
    const current = this.options.frame;
    const index = Math.max(0, order.indexOf(current));
    const newValue = order[(index + delta + order.length) % order.length];
    if (newValue === current) {
      return false;
    }
    this.gameState.sram.options.frame = newValue;
    return true;
  }

  private exitWithTransaction(): string {
    this.audioEngine?.playSound("SFX_TRANSACTION");
    return "exit";
  }

  private drawLabels(): void {
    const labelX = 2 * TILE_SIZE;
    const labelY = 2 * TILE_SIZE;
    renderFontText(this.ui.font, STRING_OPTIONS, labelX, labelY, this.ui.screen!);
  }

  private drawValues(): void {
    this.renderValue(OptionIndex.TEXT_SPEED, TEXT_SPEED_STRINGS[this.options.text_speed]);
    this.renderValue(OptionIndex.BATTLE_SCENE, BATTLE_SCENE_STRINGS[this.options.battle_scene]);
    this.renderValue(OptionIndex.BATTLE_STYLE, BATTLE_STYLE_STRINGS[this.options.battle_style]);
    this.renderValue(OptionIndex.SOUND, SOUND_STRINGS[this.options.sound]);
    this.renderValue(OptionIndex.PRINT, PRINT_STRINGS[this.options.print_option]);
    this.renderValue(OptionIndex.MENU_ACCOUNT, MENU_ACCOUNT_STRINGS[this.options.menu_account]);
    this.renderValue(OptionIndex.FRAME, String(frameTypeRenderId(this.options.frame)), 16);
  }

  private renderValue(option: OptionIndex, text: string, xTiles?: number): void {
    const x = (xTiles ?? 11) * TILE_SIZE;
    const y = this.valueRowYTiles(option) * TILE_SIZE;
    renderFontText(this.ui.font, text, x, y, this.ui.screen!);
  }

  private drawCursor(): void {
    const x = 1 * TILE_SIZE;
    const y = this.rowYTiles(this.selection) * TILE_SIZE;
    renderFontText(this.ui.font, "\u25b6", x, y, this.ui.screen!);
  }

  private rowYTiles(option: OptionIndex): number {
    return 2 + 2 * Number(option);
  }

  private valueRowYTiles(option: OptionIndex): number {
    return this.rowYTiles(option) + 1;
  }

  private applyOptionEffects(): void {
    const options = this.options;
    const delayFrames = TEXT_SPEED_DELAY_FRAMES[options.text_speed] ?? 3;
    this.gameState.wram.wTextDelayFrames = delayFrames;
    this.gameState.wram.wOptions = this.computeOptionsMask(options);
    this.gameState.wram.wOptions2 = options.menu_account === MenuAccount.ON ? MENU_ACCOUNT_BIT : 0;
    this.gameState.wram.wGBPrinterBrightness = options.print_option;
    this.gameState.wram.wTextboxFrame = options.frame;
    const frameId = frameTypeRenderId(options.frame);
    this.ui.setDefaultFrame?.(frameId);
    this.gameState.sram.textbox_frame = options.frame;
  }

  private computeOptionsMask(options: Options): number {
    let mask = TEXT_SPEED_BITS[options.text_speed] ?? 0;
    if (options.no_text_scroll) {
      mask |= NO_TEXT_SCROLL_BIT;
    }
    if (options.sound === Sound.STEREO) {
      mask |= STEREO_BIT;
    }
    if (options.battle_style === BattleStyle.SET) {
      mask |= BATTLE_SHIFT_BIT;
    }
    if (options.battle_scene === BattleScene.OFF) {
      mask |= BATTLE_SCENE_BIT;
    }
    return mask;
  }

  private textboxPalette(): [number, number, number][] {
    try {
      return this.ui.getContextPalette?.("textbox") ?? [
        [255, 255, 255],
        [0, 0, 0],
        [0, 0, 0],
        [0, 0, 0],
      ];
    } catch {
      return [
        [255, 255, 255],
        [0, 0, 0],
        [0, 0, 0],
        [0, 0, 0],
      ];
    }
  }
}
