import { GameState } from "@pokecrystal/core/core/state";
import { Item } from "@pokecrystal/core/core/enums/item";
import { SCREEN_TILE_WIDTH } from "@pokecrystal/core/core/text-constants";
import { resolveTextboxFrameRenderId } from "@pokecrystal/core/core/textbox-frame";
import { ItemSystem } from "@pokecrystal/core/engine/systems/items";
import { showText, waitForInput } from "@pokecrystal/core/engine/events/events";
import type { EventManager } from "@pokecrystal/core/engine/events/events";
import { TILE_SIZE } from "@pokecrystal/core/engine/world/tile";
import { GameButton, isButtonEvent } from "@pokecrystal/core/input/buttons";
import { gameEngine, type GameEngineEventQueue } from "@pokecrystal/core/ui/game-engine";
import { nextFrame } from "@pokecrystal/core/ui/async-loop";
import type { RGB } from "@pokecrystal/core/ui/screens/screen-types";
import type { FontSurface } from "@pokecrystal/core/ui/base-ui";
import type { Surface } from "@pokecrystal/core/ui/surface";
import type { RenderTextOptions } from "@pokecrystal/core/ui/font-renderer";
import type { ScriptRunner } from "./utils";
import { acquireUnownOverlayLock } from "./unown-overlay-lock";

type UnownFont = {
  render_text?: (text: string, x: number, y: number, surface: FontSurface, options?: RenderTextOptions) => void;
  renderText?: (text: string, x: number, y: number, surface: FontSurface, options?: RenderTextOptions | boolean) => void;
  get_char_tile?: (char: string) => FontSurface | null | undefined;
  getCharTile?: (char: string) => FontSurface | null | undefined;
};

type DrawWindowOptions = {
  frame_id?: number;
  frameId?: number;
  fill?: RGB;
};

type OverworldUI = {
  screen?: Surface | null;
  tile_size?: number;
  tileSize?: number;
  font?: UnownFont | null;
  draw_window?: (surface: Surface, x: number, y: number, width: number, height: number, options?: DrawWindowOptions) => void;
  drawWindow?: (surface: Surface, x: number, y: number, width: number, height: number, options?: DrawWindowOptions) => void;
  get_context_palette?: (key?: string) => RGB[] | null | undefined;
  getContextPalette?: (key?: string) => RGB[] | null | undefined;
  default_frame_id?: number;
  defaultFrameId?: number;
  eventQueue?: GameEngineEventQueue;
  update?: () => void;
};

type UnownOverworldAudio = {
  play_sound?: (name: string) => void;
  playSound?: (name: string) => void;
};

type UnownOverworld = {
  ui?: OverworldUI | null;
  current_map_name?: string | null;
  draw?: () => void;
  input_capture_active?: boolean;
  refresh_event_flag?: (flag: string, options?: { value?: boolean }) => void;
  audio_engine?: UnownOverworldAudio | null;
  audioEngine?: UnownOverworldAudio | null;
};

const UNOWN_WORDS: readonly string[] = ["ESCAPE", "LIGHT", "WATER", "HO-OH"];
const UNOWN_WORD_CONFIRM_SOUND = "SFX_READ_TEXT_2";

const UNOWN_LABEL_TO_INDEX: Record<string, number> = {
  UNOWNWORDS_ESCAPE: 0,
  UNOWNWORDS_LIGHT: 1,
  UNOWNWORDS_WATER: 2,
  UNOWNWORDS_HO_OH: 3,
};

const OMANYTE_FLAG = "EVENT_WALL_OPENED_IN_OMANYTE_CHAMBER";
const AERODACTYL_FLAG = "EVENT_WALL_OPENED_IN_AERODACTYL_CHAMBER";
const KABUTO_FLAG = "EVENT_WALL_OPENED_IN_KABUTO_CHAMBER";
const AERODACTYL_MAP = "RuinsOfAlphAerodactylChamber";
const KABUTO_MAP = "RuinsOfAlphKabutoChamber";

const UNOWN_MENU_TOP = 4;
const UNOWN_MENU_BOTTOM = 9;
const UNOWN_WORD_ROW_OFFSET = 2;
const UNOWN_WORD_COL_OFFSET = 1;

const resolveUnownWordIndex = (runner?: ScriptRunner | null): number => {
  if (!runner?.variables) {
    return 0;
  }
  const value = runner.variables._value;
  if (value === null || value === undefined) {
    return 0;
  }
  if (typeof value === "number") {
    if (value >= 0 && value < UNOWN_WORDS.length) {
      return value;
    }
    throw new Error(`Unown word index ${value} is out of range.`);
  }
  const normalized = String(value).trim().toUpperCase();
  if (normalized in UNOWN_LABEL_TO_INDEX) {
    return UNOWN_LABEL_TO_INDEX[normalized];
  }
  if (/^\d+$/.test(normalized)) {
    const numeric = Number(normalized);
    if (numeric >= 0 && numeric < UNOWN_WORDS.length) {
      return numeric;
    }
  }
  throw new Error(`Unknown Unown word constant '${value}'.`);
};

const playerHasWaterStone = (game_state: GameState): boolean => {
  const itemSystem = new ItemSystem(game_state);
  if (itemSystem.hasItem(Item.WATER_STONE)) {
    return true;
  }
  for (const mon of game_state.sram.party?.pokemon ?? []) {
    if (!mon) {
      continue;
    }
    const held = mon.item ?? "";
    if (String(held).toUpperCase() === Item.WATER_STONE) {
      return true;
    }
  }
  return false;
};

const resolveOverworld = (
  overworld?: UnownOverworld | null,
  runner?: ScriptRunner | null
): UnownOverworld | null => {
  if (overworld) {
    return overworld;
  }
  return (runner?.overworld as UnownOverworld) ?? null;
};

const isInMap = (target: UnownOverworld | null, mapName: string): boolean => {
  return target?.current_map_name === mapName;
};

const menuBoundsForWord = (word: string): { left: number; top: number; width: number; height: number } => {
  const length = word.length;
  const left = Math.max(0, 9 - length);
  const right = Math.min(SCREEN_TILE_WIDTH - 1, 10 + length);
  const width = Math.max(2, right - left + 1);
  const height = Math.max(2, UNOWN_MENU_BOTTOM - UNOWN_MENU_TOP + 1);
  return { left, top: UNOWN_MENU_TOP, width, height };
};

const renderUnownWord = (
  ui: OverworldUI,
  surface: Surface,
  word: string,
  { tileX, tileY }: { tileX: number; tileY: number }
): void => {
  const tileSize = ui.tile_size ?? ui.tileSize ?? TILE_SIZE;
  const drawX = tileX * tileSize;
  const drawY = tileY * tileSize;
  const renderText = ui.font?.render_text;
  if (typeof renderText === "function") {
    renderText.call(ui.font, word, drawX, drawY, surface, { uppercase: true });
    return;
  }
  const renderTextAlt = ui.font?.renderText;
  if (typeof renderTextAlt === "function") {
    renderTextAlt.call(ui.font, word, drawX, drawY, surface, true);
    return;
  }

  const getCharTile = ui.font?.get_char_tile ?? ui.font?.getCharTile;
  if (typeof getCharTile !== "function") {
    return;
  }
  let x = drawX;
  for (const char of word.toUpperCase()) {
    const tile = getCharTile.call(ui.font, char);
    if (tile) {
      surface.blit(tile, [x, drawY]);
    }
    x += tileSize;
  }
};

const drawUnownWordOverlayFrame = (
  ui: OverworldUI,
  screen: Surface,
  word: string,
  xPx: number,
  yPx: number,
  tileX: number,
  tileY: number,
  width: number,
  height: number,
  frameId: number | undefined
): void => {
  const drawWindow = ui.draw_window ?? ui.drawWindow;
  if (typeof drawWindow === "function") {
    const fill = ui.get_context_palette?.("textbox")?.[0] ?? ui.getContextPalette?.("textbox")?.[0];
    drawWindow(screen, xPx, yPx, width, height, {
      frame_id: frameId,
      frameId,
      fill,
    });
  }
  renderUnownWord(ui, screen, word, {
    tileX,
    tileY,
  });
};

const playUnownWordSound = (overworld: UnownOverworld | null, sound: string): void => {
  const audioEngine = overworld?.audio_engine ?? overworld?.audioEngine;
  if (!audioEngine) {
    return;
  }
  const playSound = audioEngine.play_sound ?? audioEngine.playSound;
  if (typeof playSound === "function") {
    playSound.call(audioEngine, sound);
  }
};

const showUnownWordOverlay = (
  {
    game_state,
    overworld,
    word,
  }: {
    game_state: GameState;
    overworld: UnownOverworld | null;
    word: string;
  }
): Promise<boolean> => {
  if (!overworld) {
    return Promise.resolve(false);
  }
  const ui = overworld.ui ?? null;
  const screen = ui?.screen ?? null;
  if (!ui || !screen || !ui.eventQueue) {
    return Promise.resolve(false);
  }

  const { left, top, width, height } = menuBoundsForWord(word);
  const xTile = left;
  const yTile = top;
  const tileSize = ui.tile_size ?? ui.tileSize ?? TILE_SIZE;
  const xPx = xTile * tileSize;
  const yPx = yTile * tileSize;
  const frameId = resolveTextboxFrameRenderId(
    game_state.sram.options?.frame,
    ui.default_frame_id ?? ui.defaultFrameId ?? 1
  );

  const releaseOverlayLock = acquireUnownOverlayLock(game_state, overworld);
  const run = async (): Promise<boolean> => {

    if (typeof overworld.draw === "function") {
      overworld.draw();
    }
    drawUnownWordOverlayFrame(
      ui,
      screen,
      word,
      xPx,
      yPx,
      xTile + UNOWN_WORD_COL_OFFSET,
      yTile + UNOWN_WORD_ROW_OFFSET,
      width,
      height,
      frameId,
    );
    if (typeof ui.update === "function") {
      ui.update();
    }

    // ASM sequence is: render static overlay once, wait for input with JoyWaitAorB.
    while (true) {
      for (const event of gameEngine.event.get(ui.eventQueue)) {
        if (event.type === gameEngine.QUIT) {
          gameEngine.quit();
          throw new Error("Quit requested during Unown word display.");
        }
      if (isButtonEvent(event, GameButton.A) || isButtonEvent(event, GameButton.B)) {
        playUnownWordSound(overworld, UNOWN_WORD_CONFIRM_SOUND);
        return true;
      }
      }
      // ASM parity: UI overlays advance on frame cadence (DelayFrame/VBlank), not CPU busy-waits.
      await nextFrame();
    }
  };

  return run().finally(() => {
    if (typeof overworld.draw === "function") {
      overworld.draw();
    }
    if (typeof ui.update === "function") {
      ui.update();
    }
    releaseOverlayLock();
  });
};

const applyEventFlag = (
  game_state: GameState,
  flagName: string,
  { overworld, value }: { overworld?: UnownOverworld | null; value: boolean }
): void => {
  game_state.wram.event_flags[flagName] = value;
  if (game_state.sram.event_flags !== game_state.wram.event_flags) {
    game_state.sram.event_flags[flagName] = value;
  }
  if (overworld?.refresh_event_flag) {
    overworld.refresh_event_flag(flagName, { value });
  }
};

export async function display_unown_words(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: UnownOverworld | null; event_manager?: EventManager | null } = {}
): Promise<string> {
  // ASM: engine/events/unown_walls.asm::DisplayUnownWords
  const index = resolveUnownWordIndex(runner ?? null);
  const word = UNOWN_WORDS[index];
  if (runner) {
    runner.last_value = word;
    runner.last_condition_result = true;
  }
  const targetOverworld = resolveOverworld(overworld, runner ?? null);
  if (
    await showUnownWordOverlay({
      game_state,
      overworld: targetOverworld,
      word,
    })
  ) {
    return word;
  }
  if (event_manager) {
    showText(event_manager, word);
    if (runner?.pause) {
      runner.pause();
    }
    waitForInput(event_manager, { pauseRunner: true });
  }
  return word;
}

export function omanyte_chamber(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: UnownOverworld | null; event_manager?: EventManager | null } = {}
): boolean {
  // ASM: engine/events/unown_walls.asm::OmanyteChamber
  void event_manager;

  if (game_state.wram.event_flags[OMANYTE_FLAG]) {
    if (runner) {
      runner.last_condition_result = true;
    }
    return true;
  }
  if (!playerHasWaterStone(game_state)) {
    if (runner) {
      runner.last_condition_result = false;
    }
    return false;
  }

  const targetOverworld = resolveOverworld(overworld, runner ?? null);
  applyEventFlag(game_state, OMANYTE_FLAG, { overworld: targetOverworld, value: true });
  if (runner) {
    runner.last_condition_result = true;
  }
  return true;
}

export function aerodactyl_chamber(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: UnownOverworld | null; event_manager?: EventManager | null } = {}
): boolean {
  // ASM: engine/events/unown_walls.asm::SpecialAerodactylChamber
  void event_manager;

  const targetOverworld = resolveOverworld(overworld, runner ?? null);
  if (!isInMap(targetOverworld, AERODACTYL_MAP)) {
    if (runner) {
      runner.last_condition_result = false;
    }
    return false;
  }

  applyEventFlag(game_state, AERODACTYL_FLAG, { overworld: targetOverworld, value: true });
  if (runner) {
    runner.last_condition_result = true;
  }
  return true;
}

export function kabuto_chamber(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: UnownOverworld | null; event_manager?: EventManager | null } = {}
): boolean {
  // ASM: engine/events/unown_walls.asm::SpecialKabutoChamber
  void event_manager;

  const targetOverworld = resolveOverworld(overworld, runner ?? null);
  if (!isInMap(targetOverworld, KABUTO_MAP)) {
    if (runner) {
      runner.last_condition_result = false;
    }
    return false;
  }

  applyEventFlag(game_state, KABUTO_FLAG, { overworld: targetOverworld, value: true });
  if (runner) {
    runner.last_condition_result = true;
  }
  return true;
}

export function unown_printer(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: UnownOverworld | null; event_manager?: EventManager | null } = {}
): Record<string, boolean> {
  // ASM: engine/events/print_unown.asm::_UnownPrinter
  void overworld;
  void event_manager;

  game_state.sram.unown_dex = true;
  game_state.wram.wUnlockedUnownMode = true;
  if (runner) {
    runner.last_condition_result = true;
    runner.last_value = { unown_dex: true };
  }
  return { unown_dex: true };
}
