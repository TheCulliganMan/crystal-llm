import { GameState } from "@pokecrystal/core/core/state";
import { BLUE_CARD_POINT_CAP } from "@pokecrystal/core/core/constants";
import { ItemSystem } from "@pokecrystal/core/engine/systems/items";
import { Event, EventManager } from "@pokecrystal/core/engine/world/events";
import { ScriptRunner, ensureRunnerVariables } from "./utils";
import type { Overworld } from "@pokecrystal/core/types/overworld";

type ItemSystemLike = ItemSystem;

type YesNoCommand = {
  runner?: ScriptRunner | null;
  on_result?: (value: boolean) => void;
  execute?: (
    game_state: GameState,
    event_manager: EventManager | null | undefined,
    overworld: Overworld | null | undefined
  ) => void;
};

type YesNoCommandFactory = () => YesNoCommand;

type RunnerCommandMap = {
  yesno?: YesNoCommandFactory;
  [key: string]: YesNoCommandFactory | undefined;
};

type ScriptRunnerWithCommandMap = ScriptRunner & {
  command_map?: RunnerCommandMap;
};

type BuenaPasswordCategory = {
  label: string;
  category_type: string;
  points: number;
  options: readonly string[];
};

type BuenaPrize = {
  item: string;
  cost: number;
};

type BuenaPrizeResult = {
  prize: string;
  quantity: number;
  points_spent: number;
  balance: number;
};

const _BUENAS_PASSWORD_FLAG = 1 << 7;

const BUENA_PASSWORD_CATEGORIES: readonly BuenaPasswordCategory[] = [
  {
    label: "Johto Starters",
    category_type: "MON",
    points: 10,
    options: ["CYNDAQUIL", "TOTODILE", "CHIKORITA"],
  },
  {
    label: "Beverages",
    category_type: "ITEM",
    points: 12,
    options: ["FRESH_WATER", "SODA_POP", "LEMONADE"],
  },
  {
    label: "Healing Items",
    category_type: "ITEM",
    points: 12,
    options: ["POTION", "ANTIDOTE", "PARLYZ_HEAL"],
  },
  {
    label: "Balls",
    category_type: "ITEM",
    points: 12,
    options: ["POKE_BALL", "GREAT_BALL", "ULTRA_BALL"],
  },
  {
    label: "Pokemon 1",
    category_type: "MON",
    points: 10,
    options: ["PIKACHU", "RATTATA", "GEODUDE"],
  },
  {
    label: "Pokemon 2",
    category_type: "MON",
    points: 10,
    options: ["HOOTHOOT", "SPINARAK", "DROWZEE"],
  },
  {
    label: "Johto Towns",
    category_type: "STRING",
    points: 16,
    options: ["NEW BARK TOWN", "CHERRYGROVE CITY", "AZALEA TOWN"],
  },
  {
    label: "Types",
    category_type: "STRING",
    points: 6,
    options: ["FLYING", "BUG", "GRASS"],
  },
  {
    label: "Moves",
    category_type: "MOVE",
    points: 12,
    options: ["TACKLE", "GROWL", "MUD_SLAP"],
  },
  {
    label: "X Items",
    category_type: "ITEM",
    points: 12,
    options: ["X_ATTACK", "X_DEFEND", "X_SPEED"],
  },
  {
    label: "Radio Stations",
    category_type: "STRING",
    points: 13,
    options: ["#MON TALK", "#MON MUSIC", "LUCKY CHANNEL"],
  },
];

const BUENA_PRIZES: readonly BuenaPrize[] = [
  { item: "ULTRA_BALL", cost: 2 },
  { item: "FULL_RESTORE", cost: 2 },
  { item: "NUGGET", cost: 3 },
  { item: "RARE_CANDY", cost: 3 },
  { item: "PROTEIN", cost: 5 },
  { item: "IRON", cost: 5 },
  { item: "CARBOS", cost: 5 },
  { item: "CALCIUM", cost: 5 },
  { item: "HP_UP", cost: 5 },
];

const normalizeChoice = (value: string | null | undefined): string => {
  if (!value) {
    return "";
  }
  const normalized = String(value).replace(/@/g, " ").toUpperCase();
  return normalized
    .split(" ")
    .filter((token) => token.length)
    .join(" ");
};

const generatePassword = (game_state: GameState, day: number): void => {
  const wram = game_state.wram;
  const categoryIndex = Math.floor(Math.random() * BUENA_PASSWORD_CATEGORIES.length);
  const category = BUENA_PASSWORD_CATEGORIES[categoryIndex];
  const optionIndex = Math.floor(Math.random() * category.options.length);
  wram.buenas_password_category = categoryIndex;
  wram.buenas_password_index = optionIndex;
  wram.buenas_password_generation_day = day;
  wram.daily_flags2 = (wram.daily_flags2 ?? 0) | _BUENAS_PASSWORD_FLAG;
};

const ensureDailyPassword = (game_state: GameState): [BuenaPasswordCategory, string] => {
  const wram = game_state.wram;
  const currentDay = Math.max(0, Number(wram.wCurDay ?? 0));
  if (wram.buenas_password_generation_day !== currentDay) {
    generatePassword(game_state, currentDay);
  }
  const categoryIndex = wram.buenas_password_category % BUENA_PASSWORD_CATEGORIES.length;
  const category = BUENA_PASSWORD_CATEGORIES[categoryIndex];
  const optionIndex = wram.buenas_password_index % category.options.length;
  return [category, category.options[optionIndex]];
};

const resolveRunnerGuess = (runner?: ScriptRunner | null): string | null => {
  if (!runner) {
    return null;
  }
  const variables = runner.variables ?? {};
  let value: unknown = variables.BUENA_PASSWORD ?? variables._selected_password;
  if (Array.isArray(value)) {
    value = value.length ? value[0] : null;
  }
  if (value === null || value === undefined) {
    return null;
  }
  return String(value).trim();
};

const resolveSelectedPrize = (runner?: ScriptRunner | null): BuenaPrize => {
  let selection = "HP_UP";
  if (runner) {
    let raw: unknown = runner.variables?._selected_prize;
    if (Array.isArray(raw)) {
      raw = raw.length ? raw[0] : null;
    }
    if (raw) {
      selection = String(raw);
    }
  }
  const normalized = selection.trim().toUpperCase();
  for (const prize of BUENA_PRIZES) {
    if (prize.item === normalized) {
      return prize;
    }
  }
  if (/^\d+$/.test(normalized)) {
    const index = Number(normalized) - 1;
    if (index >= 0 && index < BUENA_PRIZES.length) {
      return BUENA_PRIZES[index];
    }
  }
  return BUENA_PRIZES[0];
};

const resolveQuantity = (runner?: ScriptRunner | null): number => {
  if (!runner) {
    return 1;
  }
  let raw: unknown = runner.variables?._selected_prize_quantity;
  if (Array.isArray(raw)) {
    raw = raw.length ? raw[0] : null;
  }
  if (raw === null || raw === undefined) {
    return 1;
  }
  const value = Number(raw);
  if (!Number.isFinite(value)) {
    return 1;
  }
  return Math.max(1, Math.trunc(value));
};

const looksLikeItemSystem = (value: unknown): value is ItemSystemLike => {
  if (!value || typeof value !== "object") {
    return false;
  }
  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.addItem === "function" ||
    typeof candidate.add_item === "function"
  );
};

const resolveItemSystem = (runner?: ScriptRunner | null, overworld?: Overworld | null): ItemSystemLike | null => {
  const candidates: (ItemSystemLike | undefined | null)[] = [
    runner?.item_system,
    runner?.itemSystem,
    (overworld as { item_system?: unknown })?.item_system as ItemSystemLike | undefined | null,
    (overworld as { itemSystem?: unknown })?.itemSystem as ItemSystemLike | undefined | null,
  ];
  for (const candidate of candidates) {
    if (looksLikeItemSystem(candidate)) {
      return candidate;
    }
  }
  return null;
};

const addItemToSystem = (itemSystem: ItemSystemLike, item: string, quantity: number): boolean => {
  if (typeof itemSystem.addItem !== "function") {
    throw new Error("ItemSystem implementation missing addItem().");
  }
  return itemSystem.addItem(item, quantity);
};

const getBlueCardBalance = (game_state: GameState): number => {
  const balance = Number(game_state.wram.blue_card_balance ?? 0);
  return Math.max(0, Math.min(BLUE_CARD_POINT_CAP, balance));
};

const setBlueCardBalance = (game_state: GameState, value: number): void => {
  const clamped = Math.max(0, Math.min(BLUE_CARD_POINT_CAP, value));
  game_state.wram.blue_card_balance = clamped;
};

export function buenas_password(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld | null; event_manager?: EventManager | null } = {}
): boolean {
  // ASM: engine/events/buena.asm::BuenasPassword
  void overworld;
  void event_manager;

  const [category, correctValue] = ensureDailyPassword(game_state);
  const normalizedCorrect = normalizeChoice(correctValue);
  const guess = resolveRunnerGuess(runner);
  const normalizedGuess = normalizeChoice(guess);
  const isCorrect = Boolean(normalizedGuess && normalizedGuess === normalizedCorrect);

  if (runner) {
    runner.last_value = {
      category: category.label,
      type: category.category_type,
      guess,
      correct: correctValue,
      result: isCorrect,
    };
    const variables = ensureRunnerVariables(runner);
    variables._value = isCorrect ? 1 : 0;
  }

  return isCorrect;
}

export function buena_prize(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld | null; event_manager?: EventManager | null } = {}
): BuenaPrizeResult {
  // ASM: engine/events/buena.asm::BuenaPrize
  void event_manager;

  const prize = resolveSelectedPrize(runner);
  const quantity = resolveQuantity(runner);
  const cost = prize.cost * quantity;
  const balance = getBlueCardBalance(game_state);
  if (cost > balance) {
    throw new Error("Not enough Buena points for prize redemption.");
  }

  const itemSystem = resolveItemSystem(runner, overworld);
  if (!itemSystem) {
    throw new Error("Item system unavailable for Buena prize redemption.");
  }

  const added = addItemToSystem(itemSystem, prize.item, quantity);
  if (!added) {
    throw new Error("Unable to add Buena prize to the player's inventory.");
  }

  setBlueCardBalance(game_state, balance - cost);
  const result: BuenaPrizeResult = {
    prize: prize.item,
    quantity,
    points_spent: cost,
    balance: getBlueCardBalance(game_state),
  };

  if (runner) {
    runner.last_value = result;
  }

  return result;
}

const storeRunnerValue = (runner: ScriptRunner | null | undefined, result: boolean): void => {
  if (!runner) {
    return;
  }
  const variables = ensureRunnerVariables(runner);
  runner.last_condition_result = result;
  runner.last_value = result ? 1 : 0;
  variables._value = runner.last_value;
};

const runYesNoViaRunner = (
  game_state: GameState,
  event_manager: EventManager | null | undefined,
  overworld: Overworld | null | undefined,
  runner?: ScriptRunner | null
): boolean | null => {
  if (!runner) {
    return null;
  }
  const runnerWithCommands = runner as ScriptRunnerWithCommandMap;
  const factory = runnerWithCommands.command_map?.yesno;
  if (!factory) {
    return null;
  }
  const command = factory();
  command.runner = runner;
  const results: boolean[] = [];
  command.on_result = (value: boolean) => {
    results.splice(0, results.length, Boolean(value));
  };
  if (event_manager) {
    command.execute?.(game_state, event_manager, overworld);
  }
  if (results.length) {
    return results[0];
  }
  return Boolean(runner.last_yes_no_result ?? false);
};

const runYesNoPrompt = (
  game_state: GameState,
  event_manager: EventManager | null | undefined,
  overworld: Overworld | null | undefined,
  runner?: ScriptRunner | null
): boolean => {
  const runnerResult = runYesNoViaRunner(game_state, event_manager, overworld, runner);
  if (runnerResult !== null) {
    return runnerResult;
  }
  if (!event_manager || !overworld) {
    return Boolean(runner?.last_yes_no_result ?? false);
  }
  const result = { value: Boolean(runner?.last_yes_no_result ?? false) };
  event_manager.dispatch(new Event("prompt_yes_no", { callback: (value: boolean) => (result.value = value) }));
  return Boolean(result.value);
};

export function ask_remember_password(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld | null; event_manager?: EventManager | null } = {}
): boolean {
  // ASM: engine/events/buena_menu.asm::AskRememberPassword
  const result = runYesNoPrompt(game_state, event_manager, overworld, runner);
  storeRunnerValue(runner, result);
  return result;
}
