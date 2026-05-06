import { GameState } from "@pokecrystal/core/core/state";
import { HardwareRNG } from "@pokecrystal/core/engine/games/rng";
import { SlotMachine as SlotMachineGame, SlotMachineMode, SlotSymbol } from "@pokecrystal/core/engine/games/slots";
import { CardFlipGame } from "@pokecrystal/core/engine/games/card-flip";
import { MemoryGame } from "@pokecrystal/core/engine/games/memory-game";
import { EngineUnownPuzzle, UnownPuzzleLayout, UnownPuzzleLayoutSchema } from "@pokecrystal/core/engine/games/unown-puzzle";
import { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import type { EventManager } from "@pokecrystal/core/engine/events/events";
import { UnownPuzzleOverlay, type UnownPuzzleUI } from "@pokecrystal/core/ui/overlays/unown-puzzle";
import { SlotMachineOverlay } from "@pokecrystal/core/ui/overlays/slot-machine";
import * as unownPuzzleAssets from "@pokecrystal/assets/content/data/unown-puzzles/unown-puzzle-assets";
import logger from "@pokecrystal/core/core/logger";
import { ScriptRunner, ensureRunnerVariables } from "./utils";
import { showLabelledText } from "@pokecrystal/core/engine/world/story-events/specials/helpers";
import { MAX_COINS } from "@pokecrystal/core/core/constants";
import { acquireUnownOverlayLock } from "./unown-overlay-lock";

type SpecialGamesOverworld = {
  ui?: UnownPuzzleUI | null;
  audio_engine?: AudioEngine | null;
  audioEngine?: AudioEngine | null;
  script_runner?: ScriptRunner | null;
  input_capture_active?: boolean;
} & Record<string, unknown>;

type AudioEngineLike = {
  play_sound?: (name: string) => void;
  playSound?: (name: string) => void;
};

type GameCornerBlockedReason = "no_coins" | "no_coin_case";

type SlotMachineOutcome =
  | { played: false; reason: GameCornerBlockedReason }
  | {
      played: true;
      bet: number;
      payout: number;
      matched_symbol: keyof typeof SlotSymbol | null;
      winning_lines: string[];
      coins: number;
    };

type SlotMachineOverlayOutcome = {
  played: boolean;
  bet: number;
  payout: number;
  matched_symbol: keyof typeof SlotSymbol | null;
  winning_lines: string[];
  coins: number;
};

type CardFlipOutcome =
  | { played: false; reason: GameCornerBlockedReason }
  | {
      played: true;
      card_index: number;
      card_name: string;
      payout: number;
      coins: number;
    };

type MemoryGameOutcome = {
  matched: boolean;
  symbol: string | null;
  first_index: number;
  second_index: number;
};

type MemoryGameOutcomeResult =
  | { played: false; reason: GameCornerBlockedReason }
  | MemoryGameOutcome;

type UnownPuzzleOutcome = {
  solved: boolean;
  moves: number;
  layout: UnownPuzzleLayout;
  holding_piece: number | null;
  puzzle_id: string;
};

const SLOT_SYMBOL_NAME_MAP = {
  SEVEN: SlotSymbol.SEVEN,
  POKEBALL: SlotSymbol.POKEBALL,
  CHERRY: SlotSymbol.CHERRY,
  PIKACHU: SlotSymbol.PIKACHU,
  SQUIRTLE: SlotSymbol.SQUIRTLE,
  STARYU: SlotSymbol.STARYU,
} as const;

const getSlotSymbolByName = (name: string): SlotSymbol | null => {
  if (name in SLOT_SYMBOL_NAME_MAP) {
    return SLOT_SYMBOL_NAME_MAP[name as keyof typeof SLOT_SYMBOL_NAME_MAP];
  }
  return null;
};

const resolveRunnerVariable = <T>(runner: ScriptRunner | null | undefined, name: string, defaultValue: T): T => {
  if (!runner) {
    return defaultValue;
  }
  const variables = runner.variables;
  if (!variables) {
    return defaultValue;
  }
  const value = variables[name];
  return (value ?? defaultValue) as T;
};

const storeRunnerVariable = (runner: ScriptRunner | null | undefined, name: string, value: unknown): void => {
  if (!runner) {
    return;
  }
  const variables = ensureRunnerVariables(runner);
  variables[name] = value;
};

const playSound = (audio: AudioEngineLike | null | undefined, name: string): void => {
  if (!audio) {
    return;
  }
  if (audio.play_sound) {
    audio.play_sound(name);
    return;
  }
  if (audio.playSound) {
    audio.playSound(name);
  }
};

const normalizeScriptBoolean = (value: unknown): boolean | null => {
  if (value === null || value === undefined) {
    return null;
  }
  if (typeof value === "boolean") {
    return value;
  }
  if (typeof value === "number") {
    return value !== 0;
  }
  if (typeof value === "string") {
    const token = value.trim().toUpperCase();
    if (token === "TRUE" || token === "1") {
      return true;
    }
    if (token === "FALSE" || token === "0") {
      return false;
    }
  }
  return null;
};

const hasCoinCase = (game_state: GameState): boolean =>
  Boolean(game_state.sram.key_items?.COIN_CASE ?? 0);

const ensureCoinsAndCoinCase = (
  game_state: GameState,
  runner?: ScriptRunner | null,
  event_manager?: EventManager | null,
): { ok: true } | { ok: false; reason: GameCornerBlockedReason } => {
  if (runner && event_manager && !runner.event_manager) {
    runner.event_manager = event_manager;
  }
  const coins = Number(game_state.sram.coins ?? 0);
  if (coins <= 0) {
    if (runner) {
      showLabelledText(runner, "_NoCoinsText", { wait: true, autoCloseAfterWait: true });
    }
    return { ok: false, reason: "no_coins" };
  }
  if (!hasCoinCase(game_state)) {
    if (runner) {
      showLabelledText(runner, "_NoCoinCaseText", { wait: true, autoCloseAfterWait: true });
    }
    return { ok: false, reason: "no_coin_case" };
  }
  return { ok: true };
};

export function slot_machine_special(
  {
    game_state,
    runner,
    audio_engine,
    event_manager,
  }: {
    game_state: GameState;
    runner?: ScriptRunner | null;
    audio_engine?: AudioEngineLike | null;
    event_manager?: EventManager | null;
  }
): SlotMachineOutcome {
  // ASM: data/events/special_pointers.asm::SlotMachine
  const access = ensureCoinsAndCoinCase(game_state, runner ?? null, event_manager ?? null);
  if (!access.ok) {
    if (runner) {
      runner.last_condition_result = false;
      runner.last_value = access.reason;
    }
    return { played: false, reason: access.reason };
  }
  const rng = new HardwareRNG(game_state);
  const machine = new SlotMachineGame(rng);

  let bet = Number(resolveRunnerVariable(runner, "slot_bet", 3));
  if (!Number.isFinite(bet)) {
    bet = 3;
  }
  bet = Math.max(1, Math.min(3, Math.trunc(bet)));

  let coins = Number(game_state.sram.coins ?? 0);
  if (coins < bet) {
    bet = Math.max(1, coins);
  }

  const biasName = resolveRunnerVariable<string | null>(runner, "slot_bias", null);
  const biasSymbol =
    typeof biasName === "string" ? getSlotSymbolByName(biasName.toUpperCase()) : null;

  const modeOverride = normalizeScriptBoolean(runner?.variables?._value ?? null);
  let modeName = resolveRunnerVariable<string | null>(runner, "slot_mode", null);
  if (modeName === null || modeName === undefined || modeName === "") {
    modeName = modeOverride === null ? "normal" : modeOverride ? "lucky" : "normal";
  }
  const modeToken = String(modeName).toLowerCase();
  const mode =
    modeToken === SlotMachineMode.LUCKY ? SlotMachineMode.LUCKY : SlotMachineMode.NORMAL;

  playSound(audio_engine ?? null, "SFX_SLOT_MACHINE_START");

  const result = machine.spin({ bet, mode, bias: biasSymbol ?? undefined });
  coins = Math.max(0, Math.min(MAX_COINS, coins - bet + result.payout));
  game_state.sram.coins = coins;

  if (result.matchedSymbol !== null) {
    if (result.matchedSymbol === SlotSymbol.SEVEN) {
      playSound(audio_engine ?? null, "SFX_2ND_PLACE");
    } else if (result.matchedSymbol === SlotSymbol.POKEBALL) {
      playSound(audio_engine ?? null, "SFX_3RD_PLACE");
    } else {
      playSound(audio_engine ?? null, "SFX_PRESENT");
    }
  }

  const outcome: SlotMachineOutcome = {
    played: true,
    bet,
    payout: result.payout,
    matched_symbol: result.matchedSymbol !== null
      ? (SlotSymbol[result.matchedSymbol] as keyof typeof SlotSymbol)
      : null,
    winning_lines: result.winningLines,
    coins,
  };

  if (runner) {
    runner.last_value = outcome;
    runner.last_condition_result = result.payout > 0;
  }
  return outcome;
}

export async function slot_machine_ui_special(
  {
    game_state,
    runner,
    overworld,
    audio_engine,
    event_manager,
  }: {
    game_state: GameState;
    runner?: ScriptRunner | null;
    overworld?: SpecialGamesOverworld | null;
    audio_engine?: AudioEngineLike | null;
    event_manager?: EventManager | null;
  }
): Promise<SlotMachineOverlayOutcome | SlotMachineOutcome> {
  const access = ensureCoinsAndCoinCase(game_state, runner ?? null, event_manager ?? null);
  if (!access.ok) {
    if (runner) {
      runner.last_condition_result = false;
      runner.last_value = access.reason;
    }
    return { played: false, reason: access.reason };
  }
  const ui = overworld?.ui ?? null;
  if (!overworld || !ui?.eventQueue) {
    return slot_machine_special({ game_state, runner, audio_engine, event_manager });
  }

  let bet = Number(resolveRunnerVariable(runner, "slot_bet", 3));
  if (!Number.isFinite(bet)) {
    bet = 3;
  }

  const modeOverride = normalizeScriptBoolean(runner?.variables?._value ?? null);
  let modeName = resolveRunnerVariable<string | null>(runner, "slot_mode", null);
  if (modeName === null || modeName === undefined || modeName === "") {
    modeName = modeOverride === null ? "normal" : modeOverride ? "lucky" : "normal";
  }
  const mode =
    String(modeName).toLowerCase() === SlotMachineMode.LUCKY
      ? SlotMachineMode.LUCKY
      : SlotMachineMode.NORMAL;
  const previousCapture = Boolean(overworld.input_capture_active);
  overworld.input_capture_active = true;
  try {
    const overlay = new SlotMachineOverlay(ui, game_state, audio_engine ?? null, { bet, mode });
    const outcome = await overlay.runAsync();
    if (runner) {
      runner.last_value = outcome;
      runner.last_condition_result = outcome.played && outcome.payout > 0;
    }
    return outcome;
  } finally {
    overworld.input_capture_active = previousCapture;
  }
}

export function card_flip_special(
  {
    game_state,
    runner,
    audio_engine,
    event_manager,
  }: {
    game_state: GameState;
    runner?: ScriptRunner | null;
    audio_engine?: AudioEngineLike | null;
    event_manager?: EventManager | null;
  }
): CardFlipOutcome {
  // ASM: data/events/special_pointers.asm::CardFlip
  const access = ensureCoinsAndCoinCase(game_state, runner ?? null, event_manager ?? null);
  if (!access.ok) {
    if (runner) {
      runner.last_condition_result = false;
      runner.last_value = access.reason;
    }
    return { played: false, reason: access.reason };
  }
  const coinsBeforePlay = Number(game_state.sram.coins ?? 0);
  if (coinsBeforePlay < 3) {
    if (runner) {
      showLabelledText(runner, "_CardFlipNotEnoughCoinsText", { wait: true, autoCloseAfterWait: true });
      runner.last_condition_result = false;
      runner.last_value = "no_coins";
    }
    return { played: false, reason: "no_coins" };
  }
  game_state.sram.coins = Math.max(0, coinsBeforePlay - 3);

  const rng = new HardwareRNG(game_state);

  const deckState = Array.from(resolveRunnerVariable<string[]>(runner, "card_flip_deck", []));
  const revealedState = Array.from(
    resolveRunnerVariable<boolean[]>(runner, "card_flip_revealed", Array(24).fill(false))
  );

  const game = new CardFlipGame(rng, deckState.length ? deckState : undefined);
  if (!deckState.length) {
    game.shuffle();
  }
  if (revealedState.length) {
    game.revealed = revealedState.slice(0, game.deck.length);
    while (game.revealed.length < game.deck.length) {
      game.revealed.push(false);
    }
  }

  let index = Number(resolveRunnerVariable(runner, "card_flip_index", 0));
  if (!Number.isFinite(index) || index < 0 || index >= game.deck.length) {
    index = 0;
  }
  if (game.revealed[index]) {
    const fallback = game.revealed.findIndex((flag) => !flag);
    if (fallback !== -1) {
      index = fallback;
    }
  }

  const result = game.flip(index);
  const coins = Math.max(0, Math.min(MAX_COINS, Number(game_state.sram.coins ?? 0) + result.payout));
  game_state.sram.coins = coins;

  if (result.payout >= 36) {
    playSound(audio_engine ?? null, "SFX_2ND_PLACE");
  } else {
    playSound(audio_engine ?? null, "SFX_PAY_DAY");
  }

  storeRunnerVariable(runner, "card_flip_deck", [...game.deck]);
  storeRunnerVariable(runner, "card_flip_revealed", [...game.revealed]);

  const outcome: CardFlipOutcome = {
    played: true,
    card_index: result.cardIndex,
    card_name: result.cardName,
    payout: result.payout,
    coins,
  };
  if (runner) {
    runner.last_value = outcome;
    runner.last_condition_result = result.payout > 0;
  }
  return outcome;
}

export function memory_game_special(
  {
    game_state,
    runner,
    event_manager,
  }: { game_state: GameState; runner?: ScriptRunner | null; event_manager?: EventManager | null }
): MemoryGameOutcomeResult {
  // ASM: data/events/special_pointers.asm::DummyGame
  const access = ensureCoinsAndCoinCase(game_state, runner ?? null, event_manager ?? null);
  if (!access.ok) {
    if (runner) {
      runner.last_condition_result = false;
      runner.last_value = access.reason;
    }
    return { played: false, reason: access.reason };
  }
  const rng = new HardwareRNG(game_state);
  const boardState = Array.from(resolveRunnerVariable<string[]>(runner, "memory_board", []));
  const revealedState = Array.from(resolveRunnerVariable<boolean[]>(runner, "memory_revealed", []));
  const game = new MemoryGame(rng);

  if (boardState.length) {
    game.board = boardState.slice();
    if (revealedState.length) {
      game.revealed = revealedState.slice(0, boardState.length).map((flag) => Boolean(flag));
      while (game.revealed.length < boardState.length) {
        game.revealed.push(false);
      }
    } else {
      game.revealed = Array(boardState.length).fill(false);
    }
  } else {
    game.shuffle();
  }

  const first = Number(resolveRunnerVariable(runner, "memory_first", 0));
  const second = Number(resolveRunnerVariable(runner, "memory_second", 1));

  let result: ReturnType<MemoryGame["reveal"]>;
  try {
    result = game.reveal(first, second);
  } catch {
    result = game.reveal(0, 1);
  }

  storeRunnerVariable(runner, "memory_board", [...game.board]);
  storeRunnerVariable(runner, "memory_revealed", [...game.revealed]);

  const outcome: MemoryGameOutcome = {
    matched: result.matched,
    symbol: result.symbol,
    first_index: result.firstIndex,
    second_index: result.secondIndex,
  };
  if (runner) {
    runner.last_value = outcome;
    runner.last_condition_result = result.matched;
  }
  return outcome;
}

const puzzleVariableKey = (baseName: string, puzzleId: string): string => `${baseName}_${puzzleId}`;

const resolvePuzzleRunnerVariable = <
  T = unknown,
>(
  runner: ScriptRunner | null | undefined,
  baseName: string,
  puzzleId: string,
  defaultValue?: T,
): T | undefined => {
  if (!runner || !runner.variables) {
    return defaultValue;
  }
  const variables = runner.variables;
  const fullKey = puzzleVariableKey(baseName, puzzleId);
  if (fullKey in variables) {
    return variables[fullKey] as T;
  }
  const value = variables[baseName];
  if (value === null || value === undefined) {
    return defaultValue;
  }
  variables[fullKey] = value;
  delete variables[baseName];
  return value as T;
};

const clearRunnerVariable = (runner: ScriptRunner | null | undefined, name: string): void => {
  if (!runner?.variables) {
    return;
  }
  delete runner.variables[name];
};

const PUZZLE_TOKEN_MAP: Record<string, string> = {
  UNOWNPUZZLE_KABUTO: "KABUTO",
  UNOWNPUZZLE_OMANYTE: "OMANYTE",
  UNOWNPUZZLE_AERODACTYL: "AERODACTYL",
  UNOWNPUZZLE_HO_OH: "HOOH",
};

const normalizeRawPuzzleValue = (value: unknown): number | null => {
  if (typeof value === "number") {
    if (Number.isInteger(value)) {
      return value;
    }
    return null;
  }
  if (typeof value !== "string") {
    return null;
  }
  const normalized = value.trim();
  if (!normalized) {
    return null;
  }
  if (!Number.isFinite(Number(normalized))) {
    return null;
  }
  const numeric = Number(normalized);
  return Number.isInteger(numeric) ? numeric : null;
};

const isPuzzleId = (value: string): value is (typeof unownPuzzleAssets.PUZZLE_IDS)[number] =>
  (unownPuzzleAssets.PUZZLE_IDS as readonly string[]).includes(value);

const normalizePuzzleId = (runner: ScriptRunner | null | undefined): string => {
  const value = resolveRunnerVariable(runner, "_value", null);
  const numericValue = normalizeRawPuzzleValue(value);
  if (
    numericValue !== null &&
    numericValue >= 0 &&
    numericValue < unownPuzzleAssets.PUZZLE_IDS.length
  ) {
    return unownPuzzleAssets.PUZZLE_IDS[numericValue];
  }
  const token = value !== null && value !== undefined ? String(value).trim().toUpperCase() : "";
  const resolved = PUZZLE_TOKEN_MAP[token] ?? token;
  if (isPuzzleId(resolved)) {
    return resolved;
  }

  throw new Error(`Unknown Unown puzzle id '${String(value)}'.`);
};

export function unown_puzzle_special(
  {
    game_state,
    runner,
    overworld,
  }: { game_state: GameState; runner?: ScriptRunner | null; overworld?: SpecialGamesOverworld | null }
): UnownPuzzleOutcome | Promise<UnownPuzzleOutcome> {
  // ASM: data/events/special_pointers.asm::UnownPuzzle
  const rng = new HardwareRNG(game_state);
  const puzzleId = normalizePuzzleId(runner);
  game_state.wram.wSolvedUnownPuzzle = false;
  const puzzle = new EngineUnownPuzzle(rng);

  const finalize = (solved: boolean): UnownPuzzleOutcome => {
    const solvedFlag = Boolean(game_state.wram.wSolvedUnownPuzzle || solved);
    game_state.wram.wSolvedUnownPuzzle = solvedFlag;
    const status = puzzle.status();
    storeRunnerVariable(runner, puzzleVariableKey("unown_layout", puzzleId), status.layout);
    storeRunnerVariable(runner, puzzleVariableKey("unown_holding_piece", puzzleId), status.holding_piece);
    storeRunnerVariable(runner, puzzleVariableKey("unown_moves", puzzleId), status.moves);
    [
      "unown_layout",
      "unown_holding_piece",
      "unown_moves",
      "unown_action",
      "unown_x",
      "unown_y",
    ].forEach((name) => clearRunnerVariable(runner, name));

    const resultPayload: UnownPuzzleOutcome = {
      solved: solvedFlag,
      moves: status.moves,
      layout: status.layout,
      holding_piece: status.holding_piece,
      puzzle_id: puzzleId,
    };

    if (runner) {
      runner.last_value = resultPayload;
      runner.last_condition_result = solvedFlag;
    }

    return resultPayload;
  };

  const actionRaw = resolveRunnerVariable(runner, "unown_action", null);

  if (actionRaw !== null && actionRaw !== undefined) {
    const layoutState = resolvePuzzleRunnerVariable<UnownPuzzleLayout>(
      runner,
      "unown_layout",
      puzzleId,
    );
    const holdingState = resolvePuzzleRunnerVariable<number | null>(
      runner,
      "unown_holding_piece",
      puzzleId,
      null,
    );
    const movesState = Number(
      resolvePuzzleRunnerVariable<number>(runner, "unown_moves", puzzleId, 0),
    );
    if (layoutState) {
      const parsedLayout = UnownPuzzleLayoutSchema.safeParse(layoutState);
      if (parsedLayout.success) {
        puzzle.loadState(parsedLayout.data, { holding_piece: holdingState, moves: movesState });
      } else {
        throw new Error(`Stored Unown puzzle layout for ${puzzleId} is invalid.`);
      }
    } else {
      puzzle.shuffle();
    }
    const action = String(actionRaw).toLowerCase();
    const x = Number(resolveRunnerVariable(runner, "unown_x", -1));
    const y = Number(resolveRunnerVariable(runner, "unown_y", -1));
    if (action === "shuffle") {
      puzzle.shuffle();
    } else if (action === "pickup") {
      puzzle.pickup(x, y);
    } else if (action === "place") {
      puzzle.place(x, y);
    } else if (action !== "noop") {
      throw new Error(`Unknown Unown puzzle action '${actionRaw}'.`);
    }
    return finalize(puzzle.isSolved());
  }

  // ASM: _UnownPuzzle always calls InitUnownPuzzlePiecePositions on entry.
  puzzle.shuffle();
  const ui = overworld?.ui;
  const audio_engine = overworld?.audio_engine ?? overworld?.audioEngine ?? null;
  if (!ui) {
    throw new Error("Unown puzzle requires an active UI surface.");
  }
  const overlay = new UnownPuzzleOverlay(ui, game_state, audio_engine);
  const releaseOverlayLock = acquireUnownOverlayLock(game_state, overworld);
  logger.debug(`[unown-puzzle-special] begin overlay puzzleId=${puzzleId}`);
  return overlay
    .runAsync(puzzleId, rng, puzzle)
    .then((solved) => finalize(Boolean(game_state.wram.wSolvedUnownPuzzle || solved)))
    .finally(() => {
      logger.debug(`[unown-puzzle-special] end overlay puzzleId=${puzzleId}`);
      releaseOverlayLock();
    });
}

export function CardFlip(game_state: GameState, overworld?: SpecialGamesOverworld | null): CardFlipOutcome {
  // ASM: data/events/special_pointers.asm::CardFlip
  const runner = overworld?.script_runner ?? null;
  const audio_engine = overworld?.audio_engine ?? overworld?.audioEngine ?? null;
  return card_flip_special({ game_state, runner, audio_engine });
}

export function SlotMachine(
  game_state: GameState,
  overworld?: SpecialGamesOverworld | null
): SlotMachineOutcome | Promise<SlotMachineOutcome | SlotMachineOverlayOutcome> {
  // ASM: data/events/special_pointers.asm::SlotMachine
  const runner = overworld?.script_runner ?? null;
  const audio_engine = overworld?.audio_engine ?? overworld?.audioEngine ?? null;
  if (overworld?.ui?.eventQueue) {
    return slot_machine_ui_special({ game_state, runner, overworld, audio_engine });
  }
  return slot_machine_special({ game_state, runner, audio_engine });
}

export function UnownPuzzle(
  game_state: GameState,
  overworld?: SpecialGamesOverworld | null,
): UnownPuzzleOutcome | Promise<UnownPuzzleOutcome> {
  // ASM: data/events/special_pointers.asm::UnownPuzzle
  const runner = overworld?.script_runner ?? null;
  return unown_puzzle_special({ game_state, runner, overworld });
}

export function DummyGame(game_state: GameState, overworld?: SpecialGamesOverworld | null): MemoryGameOutcomeResult {
  // ASM: data/events/special_pointers.asm::DummyGame
  const runner = overworld?.script_runner ?? null;
  return memory_game_special({ game_state, runner });
}
