import { BATTLETOWER_STREAK_LENGTH } from "@pokecrystal/core/core/constants";
import { BATTLE_TOWER_TRAINER_SLOT_SENTINEL } from "@pokecrystal/core/core/memory/sram";
import { GameState } from "@pokecrystal/core/core/state";
import { ScriptRunner, ensureRunnerVariables } from "./utils";

type Overworld = unknown;
type EventManager = unknown;

type BattleTowerRecentRecord = { day: number; wins: number; result: string };
type BattleTowerVariables = Record<
  string,
  string | number | null | undefined | BattleTowerRecentRecord[]
>;

export const BATTLETOWER_NO_CHALLENGE = 0;
export const BATTLETOWER_SAVED_AND_LEFT = 1;
export const BATTLETOWER_CHALLENGE_IN_PROGRESS = 2;
export const BATTLETOWER_WON_CHALLENGE = 3;
export const BATTLETOWER_RECEIVED_REWARD = 4;

const SAVE_FILE_FLAG_YOURS = 0x1;
const SAVE_FILE_FLAG_EXPLANATION = 0x2;

const ENGLISH_MENU_OPTIONS = ["Challenge", "Explanation", "Cancel"] as const;
const JAPANESE_MENU_OPTIONS = ["NewsDownload", "NewsView", "Explanation", "Cancel"] as const;

const getBattleTowerVariables = (runner?: ScriptRunner | null): BattleTowerVariables =>
  ensureRunnerVariables(runner) as BattleTowerVariables;

const normalizeScriptToken = (token: unknown): string => {
  if (token === null || token === undefined) {
    return "";
  }
  if (Array.isArray(token)) {
    return "";
  }
  return String(token).trim();
};

const parseScriptInt = (token: unknown): number => {
  const text = normalizeScriptToken(token);
  if (!text) {
    throw new Error("Script numeric tokens cannot be empty.");
  }
  if (text.startsWith("$")) {
    return parseInt(text.slice(1), 16);
  }
  if (text.toLowerCase().startsWith("0x")) {
    return parseInt(text, 16);
  }
  return parseInt(text, 10);
};

const scriptBool = (token: unknown): boolean => {
  const normalized = normalizeScriptToken(token);
  if (!normalized) {
    return false;
  }
  const upper = normalized.toUpperCase();
  if (["TRUE", "T", "YES", "Y"].includes(upper)) {
    return true;
  }
  if (["FALSE", "F", "NO", "N"].includes(upper)) {
    return false;
  }
  return parseScriptInt(normalized) !== 0;
};

const resolveMenuSelection = (
  variables: BattleTowerVariables,
  optionCount: number
): { selection: number; cancelledByButton: boolean } => {
  const rawSelection = variables._battle_tower_menu_selection;
  let selection = rawSelection == null ? 1 : parseScriptInt(rawSelection);
  if (selection < 1) {
    throw new Error("Menu selection indices must start at 1.");
  }

  let cancelledByButton = Boolean(
    variables._selection_cancelled && scriptBool(variables._selection_cancelled)
  );

  if (cancelledByButton) {
    selection = optionCount < 4 ? 4 : optionCount;
  }

  if (selection > optionCount) {
    if (selection === 4 && optionCount === 3) {
      cancelledByButton = true;
    } else {
      throw new Error(
        `Selection ${selection} exceeds available menu entries (${optionCount}).`
      );
    }
  }

  return { selection, cancelledByButton };
};

const storeMenuSelection = (
  game_state: GameState,
  runner: ScriptRunner | null | undefined,
  selection: number,
  {
    cancelledByButton,
    optionCount,
  }: { cancelledByButton: boolean; optionCount: number }
): string => {
  const cursor = Math.min(selection, optionCount);
  game_state.wram.wMenuCursorY = cursor;
  game_state.wram.wMenuSelection = cursor;
  const value = String(selection);
  if (!runner) {
    return value;
  }
  const variables = getBattleTowerVariables(runner);
  variables._value = value;
  runner.last_value = value;
  runner.last_condition_result = !cancelledByButton;
  return value;
};

export function menu_challenge_explanation_cancel(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld; event_manager?: EventManager } = {}
): string {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerMenu_ChallengeExplanationCancel
  void overworld;
  void event_manager;
  const variables = getBattleTowerVariables(runner);
  const languageFlag = variables._value ?? "0";
  const englishMenu = scriptBool(languageFlag);
  const options = englishMenu ? ENGLISH_MENU_OPTIONS : JAPANESE_MENU_OPTIONS;
  const { selection, cancelledByButton } = resolveMenuSelection(variables, options.length);
  return storeMenuSelection(game_state, runner, selection, {
    cancelledByButton,
    optionCount: options.length,
  });
}

const battleTowerState = (game_state: GameState) => game_state.sram.battle_tower;

const currentDay = (game_state: GameState): number => {
  const day = Number(game_state.wram.wCurDay);
  if (!Number.isFinite(day)) {
    throw new Error("wCurDay must be numeric.");
  }
  return day;
};

const syncWramBeatenCount = (game_state: GameState): void => {
  const state = battleTowerState(game_state);
  game_state.wram.wNrOfBeatenBattleTowerTrainers = Math.min(
    Math.max(0, Number(state.beaten_trainers ?? 0)),
    99
  );
};

const battleTowerParty = (game_state: GameState) =>
  (game_state.sram.party?.pokemon ?? []).filter((member): member is NonNullable<typeof member> =>
    Boolean(member)
  );

const isEgg = (member: NonNullable<ReturnType<typeof battleTowerParty>[number]>): boolean =>
  String(member.species?.id ?? "").toUpperCase() === "EGG";

const battleTowerRuleFailure = (game_state: GameState): string | null => {
  // ASM: engine/events/battle_tower/rules.asm::_CheckForBattleTowerRules
  const party = battleTowerParty(game_state);
  if (party.length !== 3) {
    return "OnlyThreeMonMayBeEnteredText";
  }

  const species = new Set<string>();
  for (const member of party) {
    if (isEgg(member)) {
      continue;
    }
    const speciesId = String(member.species?.id ?? "").toUpperCase();
    if (!speciesId) {
      continue;
    }
    if (species.has(speciesId)) {
      return "TheMonMustAllBeDifferentKindsText";
    }
    species.add(speciesId);
  }

  const heldItems = new Set<string>();
  for (const member of party) {
    if (isEgg(member)) {
      continue;
    }
    const item = String(member.item ?? "").toUpperCase();
    if (!item || item === "NO_ITEM") {
      continue;
    }
    if (heldItems.has(item)) {
      return "TheMonMustNotHoldTheSameItemsText";
    }
    heldItems.add(item);
  }

  if (party.some(isEgg)) {
    return "YouCantTakeAnEggText";
  }

  return null;
};

const resetTrainerRecords = (game_state: GameState): void => {
  const state = battleTowerState(game_state);
  state.beaten_trainers = 0;
  state.trainer_history = Array(BATTLETOWER_STREAK_LENGTH).fill(
    BATTLE_TOWER_TRAINER_SLOT_SENTINEL
  );
};

const recordRun = (
  game_state: GameState,
  beaten: number,
  { day, success }: { day: number; success: boolean }
): void => {
  const state = battleTowerState(game_state);
  const sanitized = Math.max(0, Math.min(Number(beaten) || 0, BATTLETOWER_STREAK_LENGTH));
  state.record_streaks = [sanitized, ...(state.record_streaks ?? [])].slice(
    0,
    BATTLETOWER_STREAK_LENGTH
  );
  state.record_outcomes = [Boolean(success), ...(state.record_outcomes ?? [])].slice(
    0,
    BATTLETOWER_STREAK_LENGTH
  );
  state.record_days = [Number(day), ...(state.record_days ?? [])].slice(
    0,
    BATTLETOWER_STREAK_LENGTH
  );
  state.record_state = 1;
  state.record_last_day = Number(day);
  state.record_reset_counter = 0;
  state.leaderboard_acknowledged = false;
};

const formatValue = (value: unknown): string => {
  if (typeof value === "string") {
    return value;
  }
  if (typeof value === "boolean") {
    return value ? "$1" : "$0";
  }
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) {
    return String(value);
  }
  return `$${numeric}`;
};

const storeRunnerValue = (
  runner: ScriptRunner | null | undefined,
  value: unknown,
  {
    truthy,
    register,
  }: { truthy?: boolean | null; register?: string | null } = {}
): string => {
  const token = formatValue(value);
  if (!runner) {
    return token;
  }
  const variables = getBattleTowerVariables(runner);
  variables._value = token;
  runner.last_value = token;
  if (register) {
    variables[register] = token;
  }
  const resolved =
    truthy ?? (Boolean(token) && !["FALSE", "$0", "0"].includes(String(token)));
  runner.last_condition_result = Boolean(resolved);
  return token;
};

const resolveActionName = (runner?: ScriptRunner | null): string => {
  const raw = runner?.variables?._value ?? "";
  const trimmed = String(raw).split(";", 1)[0].trim();
  if (!trimmed) {
    return "0";
  }
  return trimmed.split(/\s+/)[0].toUpperCase();
};

const handleCheckSaveFile = (game_state: GameState, runner?: ScriptRunner | null): string => {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerAction_CheckSaveFileIsYours
  const state = battleTowerState(game_state);
  state.save_file_flags |= SAVE_FILE_FLAG_YOURS;
  return storeRunnerValue(runner, true, { truthy: true });
};

const handleCheckExplanationRead = (
  game_state: GameState,
  runner?: ScriptRunner | null
): string => {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerAction_CheckExplanationRead
  const state = battleTowerState(game_state);
  return storeRunnerValue(runner, state.explanation_read, {
    truthy: state.explanation_read,
  });
};

const handleSetExplanationRead = (
  game_state: GameState,
  runner?: ScriptRunner | null
): string => {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerAction_SetExplanationRead
  const state = battleTowerState(game_state);
  state.explanation_read = true;
  state.save_file_flags |= SAVE_FILE_FLAG_EXPLANATION;
  return storeRunnerValue(runner, true, { truthy: true });
};

const handleGetChallengeState = (
  game_state: GameState,
  runner?: ScriptRunner | null
): string => {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerAction_GetChallengeState
  const state = battleTowerState(game_state);
  syncWramBeatenCount(game_state);
  return storeRunnerValue(runner, state.challenge_state, {
    truthy: state.challenge_state !== BATTLETOWER_NO_CHALLENGE,
  });
};

const handleResetData = (game_state: GameState, runner?: ScriptRunner | null): string => {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerAction_ResetData
  const state = battleTowerState(game_state);
  resetTrainerRecords(game_state);
  state.challenge_state = BATTLETOWER_NO_CHALLENGE;
  state.reward_given = false;
  state.quick_saved = false;
  state.record_state = 0;
  state.record_reset_counter = 0;
  state.record_last_day = -1;
  game_state.wram.wBTChoiceOfLvlGroup = 0;
  syncWramBeatenCount(game_state);
  return storeRunnerValue(runner, "$0", { truthy: false });
};

const handleSaveLevelGroup = (
  game_state: GameState,
  runner?: ScriptRunner | null
): string => {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerAction_SaveLevelGroup
  const state = battleTowerState(game_state);
  state.level_group = game_state.wram.wBTChoiceOfLvlGroup;
  return storeRunnerValue(runner, true, { truthy: true });
};

const handleLoadLevelGroup = (
  game_state: GameState,
  runner?: ScriptRunner | null
): string => {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerAction_LoadLevelGroup
  const state = battleTowerState(game_state);
  game_state.wram.wBTChoiceOfLvlGroup = state.level_group;
  return storeRunnerValue(runner, true, { truthy: true });
};

const handleSaveOptions = (
  game_state: GameState,
  runner?: ScriptRunner | null
): string => {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerAction_SaveOptions
  const state = battleTowerState(game_state);
  const selectedToken = runner?.variables?._selected_reward ?? state.reward_item ?? "";
  const selected = String(selectedToken ?? "").trim() || "POTION";
  state.reward_item = selected;
  state.reward_given = false;
  return storeRunnerValue(runner, true, { truthy: true });
};

const handleChooseReward = (
  game_state: GameState,
  runner?: ScriptRunner | null
): string => {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerAction_ChooseReward
  const state = battleTowerState(game_state);
  const item = state.reward_item || "POTION";
  return storeRunnerValue(runner, item, { truthy: true });
};

const handleSaveAndQuit = (
  game_state: GameState,
  runner?: ScriptRunner | null
): string => {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerAction_SaveAndQuit
  const state = battleTowerState(game_state);
  state.challenge_state = BATTLETOWER_SAVED_AND_LEFT;
  state.quick_saved = true;
  state.beaten_trainers = Math.max(
    Number(state.beaten_trainers ?? 0),
    Number(game_state.wram.wNrOfBeatenBattleTowerTrainers ?? 0)
  );
  state.record_last_day = currentDay(game_state);
  state.record_state = Math.max(state.record_state ?? 0, 1);
  syncWramBeatenCount(game_state);
  return storeRunnerValue(runner, true, { truthy: true });
};

const handleChallengeCanceled = (
  game_state: GameState,
  runner?: ScriptRunner | null
): string => {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerAction_SetByteToCancelChallenge
  const state = battleTowerState(game_state);
  state.challenge_state = BATTLETOWER_NO_CHALLENGE;
  state.quick_saved = false;
  state.reward_given = false;
  state.beaten_trainers = 0;
  syncWramBeatenCount(game_state);
  return storeRunnerValue(runner, true, { truthy: true });
};

const handleResetTimers = (
  game_state: GameState,
  runner?: ScriptRunner | null
): string => {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerAction_06
  const state = battleTowerState(game_state);
  state.quick_saved = false;
  state.record_state = 0;
  state.record_last_day = -1;
  state.record_reset_counter = 0;
  return storeRunnerValue(runner, true, { truthy: true });
};

const handleAudioReset = (
  _game_state: GameState,
  runner?: ScriptRunner | null
): string => {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerAction_0A
  return storeRunnerValue(runner, true, { truthy: true });
};

const handleGsBall = (game_state: GameState, runner?: ScriptRunner | null): string => {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerAction_GSBall
  const state = battleTowerState(game_state);
  const value = state.gs_ball_flag ? 0x0b : 0;
  return storeRunnerValue(runner, value, { truthy: value !== 0 });
};

const handleWonChallenge = (
  game_state: GameState,
  runner?: ScriptRunner | null
): string => {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerAction_1C
  const state = battleTowerState(game_state);
  state.challenge_state = BATTLETOWER_WON_CHALLENGE;
  state.reward_given = false;
  game_state.wram.wNrOfBeatenBattleTowerTrainers = Math.min(
    Number(game_state.wram.wNrOfBeatenBattleTowerTrainers ?? 0) + 1,
    BATTLETOWER_STREAK_LENGTH
  );
  state.beaten_trainers = Number(game_state.wram.wNrOfBeatenBattleTowerTrainers ?? 0);
  state.record_last_day = currentDay(game_state);
  state.record_state = Math.max(state.record_state ?? 0, 1);
  return storeRunnerValue(runner, true, { truthy: true });
};

const handleGiveReward = (
  game_state: GameState,
  runner?: ScriptRunner | null
): string => {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerAction_GiveReward
  const state = battleTowerState(game_state);
  const reward = state.reward_item || "POTION";
  return storeRunnerValue(runner, reward, { truthy: true });
};

const handleRewardGiven = (
  game_state: GameState,
  runner?: ScriptRunner | null
): string => {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerAction_1D
  const state = battleTowerState(game_state);
  state.challenge_state = BATTLETOWER_RECEIVED_REWARD;
  state.reward_given = true;
  state.beaten_trainers = Math.max(
    Number(state.beaten_trainers ?? 0),
    Number(game_state.wram.wNrOfBeatenBattleTowerTrainers ?? 0)
  );
  recordRun(game_state, state.beaten_trainers, {
    day: currentDay(game_state),
    success: true,
  });
  syncWramBeatenCount(game_state);
  return storeRunnerValue(runner, true, { truthy: true });
};

const handleRecordStatus = (
  game_state: GameState,
  runner?: ScriptRunner | null
): string => {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerAction_05
  const state = battleTowerState(game_state);
  let status = Number(state.record_state ?? 0);
  let expired = false;
  if (status) {
    if ((state.record_reset_counter ?? 0) >= 2) {
      expired = true;
    }
    if ((state.record_last_day ?? -1) >= 0) {
      let delta = currentDay(game_state) - Number(state.record_last_day ?? 0);
      if (delta < 0) {
        delta += 0x100;
      }
      if (delta >= 8) {
        expired = true;
      }
    }
  }
  if (expired) {
    status = 8;
    state.record_state = 0;
  }
  return storeRunnerValue(runner, status, { truthy: status !== 0 });
};

const handleAckFlag = (
  game_state: GameState,
  runner: ScriptRunner | null | undefined,
  value: boolean
): string => {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerAction_11/12
  const state = battleTowerState(game_state);
  state.leaderboard_acknowledged = value;
  return storeRunnerValue(runner, Number(value), { truthy: value });
};

const handleReadAckFlag = (
  game_state: GameState,
  runner?: ScriptRunner | null
): string => {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerAction_13
  const state = battleTowerState(game_state);
  return storeRunnerValue(runner, Number(state.leaderboard_acknowledged), {
    truthy: state.leaderboard_acknowledged,
  });
};

const handleCheckSavefileFlag = (
  game_state: GameState,
  runner?: ScriptRunner | null
): string => {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerAction_14
  const state = battleTowerState(game_state);
  const value = Number(Boolean(state.save_file_flags & SAVE_FILE_FLAG_YOURS));
  return storeRunnerValue(runner, value, { truthy: Boolean(value) });
};

const handleMarkSavefileYours = (
  game_state: GameState,
  runner?: ScriptRunner | null
): string => {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerAction_15
  const state = battleTowerState(game_state);
  state.save_file_flags |= SAVE_FILE_FLAG_YOURS;
  return storeRunnerValue(runner, 1, { truthy: true });
};

const handleTimerMark = (
  game_state: GameState,
  runner?: ScriptRunner | null
): string => {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerAction_16
  const state = battleTowerState(game_state);
  state.record_last_day = currentDay(game_state);
  state.record_reset_counter = 0;
  return storeRunnerValue(runner, true, { truthy: true });
};

const handleTimerCheck = (
  game_state: GameState,
  runner?: ScriptRunner | null
): string => {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerAction_17
  const state = battleTowerState(game_state);
  let expired = (state.record_reset_counter ?? 0) >= 2;
  if ((state.record_last_day ?? -1) >= 0) {
    let delta = currentDay(game_state) - Number(state.record_last_day ?? 0);
    if (delta < 0) {
      delta += 0x100;
    }
    if (delta >= 11) {
      expired = true;
    }
  }
  if (expired) {
    state.record_last_day = -1;
    state.record_reset_counter = 0;
    return storeRunnerValue(runner, 1, { truthy: true });
  }
  return storeRunnerValue(runner, 0, { truthy: false });
};

const handleLevelCheck = (
  game_state: GameState,
  runner?: ScriptRunner | null
): string => {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerAction_LevelCheck
  const state = battleTowerState(game_state);
  const levelGroup =
    state.level_group ?? game_state.wram.wBTChoiceOfLvlGroup ?? 0;
  const levelCap = Math.max(10, Math.min(Number(levelGroup) * 10, 100));
  let highest = 0;
  for (const member of game_state.sram.party?.pokemon ?? []) {
    if (!member) {
      continue;
    }
    const level = Number(member.level ?? 0);
    highest = Math.max(highest, level);
    if (highest > levelCap) {
      return storeRunnerValue(runner, highest, { truthy: true });
    }
  }
  return storeRunnerValue(runner, 0, { truthy: false });
};

const handleUbersCheck = (
  game_state: GameState,
  runner?: ScriptRunner | null
): string => {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerAction_UbersCheck
  const banned = new Set(["MEWTWO", "MEW", "LUGIA", "HO_OH", "CELEBI"]);
  for (const member of game_state.sram.party?.pokemon ?? []) {
    if (!member?.species?.id) {
      continue;
    }
    const speciesId = String(member.species.id).toUpperCase();
    if (banned.has(speciesId)) {
      return storeRunnerValue(runner, 1, { truthy: true });
    }
  }
  return storeRunnerValue(runner, 0, { truthy: false });
};

const ACTION_HANDLERS: Record<string, (game_state: GameState, runner?: ScriptRunner | null) => string> = {
  BATTLETOWERACTION_CHECKSAVEFILEISYOURS: handleCheckSaveFile,
  BATTLETOWERACTION_CHECK_EXPLANATION_READ: handleCheckExplanationRead,
  BATTLETOWERACTION_SET_EXPLANATION_READ: handleSetExplanationRead,
  BATTLETOWERACTION_GET_CHALLENGE_STATE: handleGetChallengeState,
  BATTLETOWERACTION_05: handleRecordStatus,
  BATTLETOWERACTION_RESETDATA: handleResetData,
  BATTLETOWERACTION_SAVELEVELGROUP: handleSaveLevelGroup,
  BATTLETOWERACTION_LOADLEVELGROUP: handleLoadLevelGroup,
  BATTLETOWERACTION_SAVEOPTIONS: handleSaveOptions,
  BATTLETOWERACTION_CHOOSEREWARD: handleChooseReward,
  BATTLETOWERACTION_SAVE_AND_QUIT: handleSaveAndQuit,
  BATTLETOWERACTION_CHALLENGECANCELED: handleChallengeCanceled,
  BATTLETOWERACTION_06: handleResetTimers,
  BATTLETOWERACTION_0A: handleAudioReset,
  BATTLETOWERACTION_GSBALL: handleGsBall,
  BATTLETOWERACTION_1C: handleWonChallenge,
  BATTLETOWERACTION_GIVEREWARD: handleGiveReward,
  BATTLETOWERACTION_1D: handleRewardGiven,
  BATTLETOWERACTION_11: (game_state, runner) => handleAckFlag(game_state, runner, false),
  BATTLETOWERACTION_12: (game_state, runner) => handleAckFlag(game_state, runner, true),
  BATTLETOWERACTION_13: handleReadAckFlag,
  BATTLETOWERACTION_14: handleCheckSavefileFlag,
  BATTLETOWERACTION_15: handleMarkSavefileYours,
  BATTLETOWERACTION_16: handleTimerMark,
  BATTLETOWERACTION_17: handleTimerCheck,
  BATTLETOWERACTION_LEVEL_CHECK: handleLevelCheck,
  BATTLETOWERACTION_UBERS_CHECK: handleUbersCheck,
};

export function battle_tower_action(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld; event_manager?: EventManager } = {}
): string {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerAction
  void overworld;
  void event_manager;
  const state = battleTowerState(game_state);
  const actionName = resolveActionName(runner);
  state.save_file_flags |= SAVE_FILE_FLAG_YOURS;
  const handler = ACTION_HANDLERS[actionName];
  if (!handler) {
    throw new Error(`Unhandled Battle Tower action '${actionName}'`);
  }
  return handler(game_state, runner);
}

export function check_for_battle_tower_rules(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld; event_manager?: EventManager } = {}
): string {
  // ASM: engine/events/battle_tower/rules.asm::_CheckForBattleTowerRules
  void overworld;
  void event_manager;
  const failure = battleTowerRuleFailure(game_state);
  if (runner) {
    const variables = getBattleTowerVariables(runner);
    variables.battle_tower_rule_failure = failure;
  }
  return storeRunnerValue(runner, failure ? "TRUE" : "FALSE", {
    truthy: Boolean(failure),
  });
}

export function battle_tower_room_menu(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld; event_manager?: EventManager } = {}
): string {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerRoomMenu
  void overworld;
  void event_manager;
  const state = battleTowerState(game_state);
  if (runner) {
    const variables = getBattleTowerVariables(runner);
    variables.$a = "FALSE";
    const recent: Array<{ day: number; wins: number; result: string }> = [];
    const days = state.record_days ?? [];
    const streaks = state.record_streaks ?? [];
    const outcomes = state.record_outcomes ?? [];
    const count = Math.min(days.length, streaks.length, outcomes.length);
    for (let i = 0; i < count; i++) {
      recent.push({
        day: Number(days[i] ?? 0),
        wins: Number(streaks[i] ?? 0),
        result: outcomes[i] ? "win" : "loss",
      });
    }
    variables.battle_tower_recent_records = recent;
  }
  return storeRunnerValue(runner, "$0", { truthy: false });
}

export function battle_tower_battle(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld; event_manager?: EventManager } = {}
): string {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerBattle
  void overworld;
  void event_manager;
  const state = battleTowerState(game_state);
  state.quick_saved = false;
  const battleResult = runner?.variables?._battle_result ?? game_state.wram.battle_result;
  const resultCode = Number(battleResult);
  if (!Number.isFinite(resultCode)) {
    throw new Error(`Invalid battle result token: ${battleResult}`);
  }

  if (resultCode !== 0) {
    state.challenge_state = BATTLETOWER_NO_CHALLENGE;
    state.reward_given = false;
    state.beaten_trainers = 0;
    syncWramBeatenCount(game_state);
    return storeRunnerValue(runner, resultCode, { truthy: false });
  }

  state.challenge_state = BATTLETOWER_CHALLENGE_IN_PROGRESS;
  const startCount = Math.max(
    Number(state.beaten_trainers ?? 0),
    Number(game_state.wram.wNrOfBeatenBattleTowerTrainers ?? 0)
  );
  const newCount = Math.min(startCount + 1, BATTLETOWER_STREAK_LENGTH);
  game_state.wram.wNrOfBeatenBattleTowerTrainers = newCount;
  state.beaten_trainers = newCount;
  state.record_state = Math.max(state.record_state ?? 0, 1);
  if (newCount >= BATTLETOWER_STREAK_LENGTH) {
    state.challenge_state = BATTLETOWER_WON_CHALLENGE;
    state.record_last_day = currentDay(game_state);
  }
  return storeRunnerValue(runner, true, { truthy: true });
}

export function battle_tower_fade(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld; event_manager?: EventManager } = {}
): string {
  // ASM: engine/tilesets/timeofday_pals.asm::BattleTowerFade
  void overworld;
  void event_manager;
  battleTowerState(game_state).quick_saved = false;
  return storeRunnerValue(runner, 1, { truthy: true });
}

export function battle_tower_mobile_error(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld; event_manager?: EventManager } = {}
): string {
  // ASM: engine/events/battle_tower/battle_tower.asm::BattleTowerMobileError
  void game_state;
  void overworld;
  void event_manager;
  return storeRunnerValue(runner, "$0", { truthy: false });
}
