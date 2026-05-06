import { GameState } from "../../../core/state";
import type { DataLoader } from "@pokecrystal/core/core/data-loader";
import { ItemSystem } from "../../systems/items";
import * as tmhmSystem from "../../systems/tmhm";
import { ScriptRunner, ensureRunnerVariables } from "./utils";
import { gameEngine, type GameEngineEvent, GameEngineEventQueue } from "@pokecrystal/core/ui/game-engine";
import { nextFrame } from "@pokecrystal/core/ui/async-loop";
import { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { isCancelEvent, isConfirmEvent, isKeyDownEvent } from "@pokecrystal/core/input/buttons";
import { mapKeyToDirection, type InputEventLike } from "@pokecrystal/core/input/controls";
import { TILE_SIZE } from "@pokecrystal/core/engine/world/tile";
import type { Surface } from "@pokecrystal/core/ui/surface";
import type { RenderTextOptions } from "@pokecrystal/core/ui/font-renderer";

const APRICORN_TYPES: readonly [string, string][] = [
  ["RED_APRICORN", "RED APRICORN"],
  ["BLU_APRICORN", "BLUE APRICORN"],
  ["YLW_APRICORN", "YELLOW APRICORN"],
  ["GRN_APRICORN", "GREEN APRICORN"],
  ["WHT_APRICORN", "WHITE APRICORN"],
  ["BLK_APRICORN", "BLACK APRICORN"],
  ["PNK_APRICORN", "PINK APRICORN"],
];

const SELECTION_VAR = "_kurt_apricorn_type";
const QUANTITY_VAR = "_kurt_apricorn_quantity";
const KURT_VAR = "VAR_KURT_APRICORNS";

type Overworld = {
  ui?: PromptUI | null;
  audio_engine?: AudioEngine;
  data_loader?: DataLoader | null;
  dataLoader?: DataLoader | null;
  draw?: () => void;
  item_system?: ItemSystem | null;
};

type EventManager = unknown;

type ApricornChoice = [string, string, number];

type PromptDrawOptions = {
  frameId?: number;
  frame_id?: number;
  fill?: [number, number, number];
};

type PromptFontRenderer = {
  renderText?: (
    text: string,
    x: number,
    y: number,
    surface: Surface,
    options?: RenderTextOptions | boolean
  ) => void;
  render_text?: (
    text: string,
    x: number,
    y: number,
    surface: Surface,
    options?: RenderTextOptions
  ) => void;
};

type PromptUI = {
  screen: Surface | null;
  screenWidth: number;
  screenHeight: number;
  eventQueue?: GameEngineEventQueue;
  tile_size?: number;
  tileSize?: number;
  drawWindow?: (
    surface: Surface,
    x: number,
    y: number,
    widthTiles: number,
    heightTiles: number,
    options?: PromptDrawOptions
  ) => void;
  draw_window?: (
    surface: Surface,
    x: number,
    y: number,
    widthTiles: number,
    heightTiles: number,
    options?: PromptDrawOptions
  ) => void;
  clearScreen?: (color: [number, number, number]) => void;
  update?: () => void;
  font?: PromptFontRenderer;
};

type OverworldDataLoaderCarrier = {
  data_loader?: DataLoader | null;
  dataLoader?: DataLoader | null;
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
  const options: RenderTextOptions = { uppercase };
  if (font?.render_text) {
    font.render_text(text, x, y, surface, options);
    return;
  }
  if (font?.renderText) {
    font.renderText(text, x, y, surface, options);
    return;
  }
  throw new Error("KurtApricornPrompt requires a font renderer");
};

class KurtApricornPrompt {
  private readonly ui: PromptUI;
  private readonly screen: Surface | null;
  private readonly options: string[];
  private readonly audioEngine?: AudioEngine;
  private readonly title: string;
  private index = 0;
  private finished = false;
  public wasCanceled = false;

  constructor(ui: PromptUI, options: string[], { audioEngine, title = "WHICH APRICORN?" }: { audioEngine?: AudioEngine; title?: string } = {}) {
    if (!options.length) {
      throw new Error("KurtApricornPrompt requires at least one option");
    }
    this.ui = ui;
    this.screen = ui.screen;
    this.options = options;
    this.audioEngine = audioEngine;
    this.title = title;
  }

  handleInput(event: InputEventLike): void {
    if (!isKeyDownEvent(event)) {
      return;
    }
    const direction = mapKeyToDirection(String(event.code ?? event.key ?? ""));
    if (direction === "up" || direction === "left") {
      this.index = (this.index - 1 + this.options.length) % this.options.length;
      this.playCursor();
      return;
    }
    if (direction === "down" || direction === "right") {
      this.index = (this.index + 1) % this.options.length;
      this.playCursor();
      return;
    }
    if (isCancelEvent(event)) {
      this.wasCanceled = true;
      this.finished = true;
      this.playCursor();
      return;
    }
    if (isConfirmEvent(event)) {
      this.finished = true;
      this.playConfirm();
    }
  }

  draw(clearBackground = true): void {
    const screen = this.screen;
    if (!screen) {
      return;
    }
    if (clearBackground) {
      this.ui.clearScreen?.([0, 0, 0]);
    }

    const tileSize = getTileSize(this.ui);
    const textX = tileSize;
    const titleY = tileSize;
    const optionY = tileSize * 2;

    renderPromptText(this.ui, this.title, textX, titleY, screen, true);
    this.options.forEach((option, idx) => {
      const cursor = idx === this.index ? ">" : " ";
      renderPromptText(this.ui, `${cursor}${option}`, textX, optionY + idx * tileSize, screen, true);
    });
  }

  result(): number {
    return this.index;
  }

  run({ drawCallback }: { drawCallback?: () => void } = {}): Promise<number> {
    return this.runAsync({ drawCallback });
  }

  async runAsync({ drawCallback }: { drawCallback?: () => void } = {}): Promise<number> {
    while (!this.finished) {
      for (const event of gameEngine.event.get(this.ui.eventQueue)) {
        if (event.type === gameEngine.QUIT) {
          gameEngine.quit();
          throw new Error("KurtApricornPrompt interrupted by quit event");
        }
        this.handleInput(event);
      }
      if (drawCallback) {
        drawCallback();
        this.draw(false);
      } else {
        this.draw();
      }
      this.ui.update?.();
      await nextFrame();
    }
    return this.result();
  }

  private playCursor(): void {
    this.audioEngine?.playSound("menu_cursor");
  }

  private playConfirm(): void {
    this.audioEngine?.playSound("menu_option");
  }
}

export function select_apricorn_for_kurt(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld | null; event_manager?: EventManager } = {}
): string | Promise<string> {
  // ASM: engine/events/kurt.asm::SelectApricornForKurt
  void event_manager;

  let itemSystem = resolveItemSystem(runner, overworld);
  if (!itemSystem) {
    const overworldLoader =
      overworld && typeof overworld === "object"
        ? ((overworld as OverworldDataLoaderCarrier).data_loader ??
            (overworld as OverworldDataLoaderCarrier).dataLoader)
        : undefined;
    const dataLoader = runner?.data_loader ?? runner?.dataLoader ?? overworldLoader ?? undefined;
    itemSystem = new ItemSystem(game_state, dataLoader);
  }

  const available = gatherApricornChoices(game_state, itemSystem);
  if (!available.length) {
    setRunnerCancel(runner ?? null);
    return "FALSE";
  }

  let selectionResult = consumeRunnerSelection(runner ?? null, available);
  const resolveSelection = (selected: [string, number] | null): string => {
    if (!selected) {
      setRunnerCancel(runner ?? null);
      return "FALSE";
    }

    const [apricornToken, quantity] = selected;
    if (!removeFromInventory(game_state, itemSystem, apricornToken, quantity)) {
      setRunnerCancel(runner ?? null);
      return "FALSE";
    }

    assignRunnerVariables(runner ?? null, quantity);
    if (runner) {
      runner.last_condition_result = true;
    }
    return apricornToken;
  };

  if (!selectionResult) {
    const ui = overworld?.ui ?? null;
    if (!ui || !ui.screen) {
      setRunnerCancel(runner ?? null);
      return "FALSE";
    }
    return promptWithUiAsync(available, ui, overworld?.audio_engine, overworld ?? null).then(
      (selected) => resolveSelection(selected),
    );
  }
  return resolveSelection(selectionResult);
}

function gatherApricornChoices(game_state: GameState, itemSystem: ItemSystem): ApricornChoice[] {
  const choices: ApricornChoice[] = [];
  for (const [token, fallbackName] of APRICORN_TYPES) {
    const count = itemSystem.getQuantity(token);
    if (count <= 0) {
      continue;
    }
    const display = resolveDisplayName(itemSystem, token) || fallbackName;
    choices.push([token, display, count]);
  }
  return choices;
}

async function promptWithUiAsync(
  available: ApricornChoice[],
  ui: PromptUI,
  audioEngine?: AudioEngine,
  overworld?: Overworld | null
): Promise<[string, number] | null> {
  while (true) {
    const selection = await promptTypeAsync(ui, available, audioEngine, overworld ?? null);
    if (!selection) {
      return null;
    }
    const [token, display, availableCount] = selection;
    const quantity = await promptQuantityAsync(
      ui,
      audioEngine,
      overworld ?? null,
      display,
      availableCount,
    );
    if (!quantity) {
      continue;
    }
    return [token, quantity];
  }
}

async function promptTypeAsync(
  ui: PromptUI,
  available: ApricornChoice[],
  audioEngine?: AudioEngine,
  overworld?: Overworld | null
): Promise<ApricornChoice | null> {
  const options = available.map(([, display, count]) => `${display.padEnd(14, " ")} x${count}`);
  const prompt = new KurtApricornPrompt(ui, options, { audioEngine });

  const draw = () => {
    drawOverworld(overworld ?? null);
  };

  await prompt.runAsync({ drawCallback: draw });
  if (prompt.wasCanceled) {
    return null;
  }
  const index = prompt.result();
  if (index === null || index >= available.length) {
    return null;
  }
  return available[index];
}

async function promptQuantityAsync(
  ui: PromptUI,
  audioEngine: AudioEngine | undefined,
  overworld: Overworld | null,
  displayName: string,
  maxQuantity: number
): Promise<number | null> {
  let quantity = Math.max(1, Math.min(maxQuantity, 1));
  while (true) {
    for (const event of gameEngine.event.get(ui.eventQueue)) {
      if (event.type === gameEngine.QUIT) {
        gameEngine.quit();
        throw new Error("KurtApricornPrompt interrupted by quit event");
      }
      if (!isKeyDownEvent(event)) {
        continue;
      }
      if (isCancelEvent(event)) {
        playSound(audioEngine, "menu_cancel");
        return null;
      }
      if (isConfirmEvent(event)) {
        playSound(audioEngine, "menu_option");
        return quantity;
      }
      const delta = quantityDelta(event);
      if (!delta) {
        continue;
      }
      quantity = Math.max(1, Math.min(maxQuantity, quantity + delta));
      playSound(audioEngine, "menu_cursor");
    }
    drawQuantityWindow(ui, overworld ?? null, displayName, quantity, maxQuantity);
    ui.update?.();
    await nextFrame();
  }
}

function quantityDelta(event: GameEngineEvent): number {
  const direction = mapKeyToDirection(String(event.code ?? event.key ?? ""));
  if (direction === "up") {
    return 1;
  }
  if (direction === "down") {
    return -1;
  }
  if (direction === "right") {
    return 10;
  }
  if (direction === "left") {
    return -10;
  }
  return 0;
}

function drawQuantityWindow(
  ui: PromptUI,
  overworld: Overworld | null,
  displayName: string,
  quantity: number,
  maxQuantity: number
): void {
  drawOverworld(overworld);
  const screen = ui.screen;
  if (!screen) {
    return;
  }
  const tileSize = getTileSize(ui);
  const width = 12;
  const height = 6;
  const x = Math.floor((ui.screenWidth / tileSize - width) / 2) * tileSize;
  const y = (Math.floor(ui.screenHeight / tileSize) - height - 2) * tileSize;

  const drawWindow = ui.drawWindow ?? ui.draw_window;
  if (!drawWindow) {
    throw new Error("KurtApricornPrompt requires drawWindow/draw_window");
  }
  drawWindow(screen, x, y, width, height, { fill: [255, 255, 255] });

  const lines = [displayName, `Qty: ${quantity}/${maxQuantity}`, "A=OK  B=CANCEL"];
  lines.forEach((line, offset) => {
    renderPromptText(ui, line, x + tileSize, y + tileSize + offset * tileSize, screen, true);
  });
}

function drawOverworld(overworld: Overworld | null): void {
  const draw = overworld?.draw;
  if (typeof draw === "function") {
    draw();
  }
}

function playSound(audioEngine: AudioEngine | undefined, soundId: string): void {
  if (!audioEngine) {
    return;
  }
  try {
    audioEngine.playSound(soundId);
  } catch (error) {
    void error;
  }
}

function removeFromInventory(
  game_state: GameState,
  itemSystem: ItemSystem | null,
  itemName: string,
  quantity: number
): boolean {
  if (itemSystem) {
    return itemSystem.removeItem(itemName, quantity);
  }
  let remaining = quantity;
  const inventories: Array<Record<string, number> | number[]> = [
    game_state.sram.items,
    game_state.sram.key_items,
    game_state.sram.balls,
    game_state.sram.tm_hm,
  ];

  for (const inventory of inventories) {
    if (Array.isArray(inventory)) {
      let index: number;
      try {
        index = tmhmSystem.tmhmIndex(itemName);
      } catch (error) {
        continue;
      }
      if (index >= inventory.length || tmhmSystem.isHmIndex(index)) {
        continue;
      }
      if (inventory[index] <= 0) {
        continue;
      }
      inventory[index] = 0;
      remaining -= 1;
      if (remaining <= 0) {
        return true;
      }
    } else {
      const current = inventory[itemName] ?? 0;
      if (current <= 0) {
        continue;
      }
      const take = Math.min(current, remaining);
      const newValue = current - take;
      if (newValue) {
        inventory[itemName] = newValue;
      } else {
        delete inventory[itemName];
      }
      remaining -= take;
      if (remaining <= 0) {
        return true;
      }
    }
  }
  return false;
}

function clampQuantity(requested: unknown, available: number): number {
  let quantity = 1;
  try {
    quantity = Number.parseInt(String(requested ?? ""), 10);
  } catch {
    quantity = 1;
  }
  if (!Number.isFinite(quantity)) {
    quantity = 1;
  }
  return Math.max(1, Math.min(available, quantity));
}

function popRunnerVariable(runner: ScriptRunner | null, key: string): unknown {
  if (!runner) {
    return null;
  }
  const variables = ensureRunnerVariables(runner);
  const value = variables[key];
  delete variables[key];
  return value ?? null;
}

function assignRunnerVariables(runner: ScriptRunner | null, quantity: number): void {
  if (!runner) {
    return;
  }
  const variables = ensureRunnerVariables(runner);
  variables[KURT_VAR] = quantity;
}

function consumeRunnerSelection(
  runner: ScriptRunner | null,
  available: ApricornChoice[]
): [string, number] | null {
  if (!runner) {
    return null;
  }
  const selection = popRunnerVariable(runner, SELECTION_VAR);
  const quantityHint = popRunnerVariable(runner, QUANTITY_VAR) ?? 1;
  if (selection === null || selection === undefined) {
    return null;
  }
  if (typeof selection === "number") {
    if (selection >= 0 && selection < available.length) {
      return [available[selection][0], clampQuantity(quantityHint, available[selection][2])];
    }
    return null;
  }
  const normalized = String(selection).toUpperCase();
  for (const [token, , count] of available) {
    if (token === normalized) {
      return [token, clampQuantity(quantityHint, count)];
    }
  }
  return null;
}

function setRunnerCancel(runner: ScriptRunner | null): void {
  if (!runner) {
    return;
  }
  runner.last_condition_result = false;
}

type ItemSystemHost = { item_system?: ItemSystem | null };

const hasItemSystem = (value: unknown): value is ItemSystemHost =>
  typeof value === "object" && value !== null && "item_system" in value;

function resolveItemSystem(runner?: ScriptRunner | null, overworld?: Overworld | null): ItemSystem | null {
  if (hasItemSystem(overworld) && overworld.item_system instanceof ItemSystem) {
    return overworld.item_system;
  }
  if (hasItemSystem(runner) && runner.item_system instanceof ItemSystem) {
    return runner.item_system;
  }
  return null;
}

function resolveDisplayName(itemSystem: ItemSystem | null, itemName: string): string {
  if (itemSystem) {
    try {
      return itemSystem.getDisplayName(itemName);
    } catch (error) {
      void error;
    }
  }
  return itemName
    .replace(/_/g, " ")
    .toLowerCase()
    .replace(/\b\w/g, (match) => match.toUpperCase());
}
