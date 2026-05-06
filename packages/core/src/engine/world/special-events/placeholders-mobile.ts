import { GameState } from "@pokecrystal/core/core/state";
import { MOBILE_LOGIN_PASSWORD_LENGTH } from "@pokecrystal/core/core/constants";
import { BattleTowerSaveData, MobileAdapterSaveData } from "@pokecrystal/core/core/memory/sram";
import { SerialConnectionStatus } from "@pokecrystal/core/core/memory/registers";
import {
  ScriptRunner,
  ensureRunnerVariables,
  setRunnerValue,
  setSerialConnectionStatus,
} from "./utils";
import { LINK_MOBILE, LINK_NULL, _touch_state } from "./placeholders";
import type { EventManager } from "@pokecrystal/core/engine/events/events";
import type { Overworld } from "@pokecrystal/core/types/overworld";

type BattleTowerRecord = {
  streak: number;
  outcome: "win" | "loss";
  day: number;
};

interface MobileState {
  mode?: string;
  adapter_status?: unknown;
  adapter_secondary_status?: unknown;
  battle_timer?: [number, number, number];
  login_password?: string;
  last_activity?: Date;
  handshakes?: number;
  leaderboard?: BattleTowerRecord[];
  terminated?: boolean;
  [key: string]: unknown;
}

const mobileState = (game_state: GameState): MobileState => {
  return _touch_state(game_state, "mobile_link");
};

const validateMobileAdapter = (adapter: MobileAdapterSaveData): [number, number, number] => {
  if (!adapter) {
    throw new Error("Mobile adapter SRAM must be initialised.");
  }
  const password = adapter.login_password ?? "";
  if (password.length > MOBILE_LOGIN_PASSWORD_LENGTH) {
    throw new Error("Mobile adapter password exceeds SRAM capacity.");
  }
  const timer = Array.isArray(adapter.battle_timer)
    ? adapter.battle_timer
    : [0, 0, 0];
  if (timer.length !== 3) {
    throw new Error("Mobile battle timer must contain three components.");
  }
  const [first, second, third] = timer;
  return [Number(first), Number(second), Number(third)];
};

const battleTowerRecords = (state: BattleTowerSaveData): BattleTowerRecord[] => {
  const streaks = Array.isArray(state.record_streaks) ? [...state.record_streaks] : [];
  const outcomes = Array.isArray(state.record_outcomes) ? [...state.record_outcomes] : [];
  const days = Array.isArray(state.record_days) ? [...state.record_days] : [];
  if (new Set([streaks.length, outcomes.length, days.length]).size !== 1) {
    throw new Error("Battle Tower record arrays must remain the same length.");
  }
  return streaks.map((streak, idx) => {
    const outcome = outcomes[idx];
    const day = days[idx];
    return {
      streak: Number(streak),
      outcome: outcome ? "win" : "loss",
      day: Number(day) & 0xff,
    };
  });
};

const recordMobileHandshake = (
  game_state: GameState,
  { mode }: { mode: string }
): MobileState => {
  const adapter = game_state.sram.mobile_adapter;
  const timer = validateMobileAdapter(adapter);
  const state = mobileState(game_state);
  const now = new Date();
  adapter.last_activity = now;
  state.mode = mode;
  state.adapter_status = adapter.adapter_status;
  state.adapter_secondary_status = adapter.adapter_secondary_status;
  state.battle_timer = timer;
  state.login_password = adapter.login_password ?? "";
  state.last_activity = now;
  state.handshakes = Number(state.handshakes ?? 0) + 1;
  state.leaderboard = battleTowerRecords(game_state.sram.battle_tower);
  return state;
};

export function function1700ba(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld | null; event_manager?: EventManager } = {}
): string {
  // ASM: engine/events/battle_tower/battle_tower.asm::Function1700ba
  void overworld;
  void event_manager;

  const records = battleTowerRecords(game_state.sram.battle_tower);
  if (runner) {
    const variables = ensureRunnerVariables(runner);
    variables.battle_tower_leaderboard = records;
  }
  if (!records.length) {
    return setRunnerValue(runner, "$a", { truthy: false });
  }
  game_state.sram.battle_tower.leaderboard_acknowledged = false;
  return setRunnerValue(runner, "$0", { truthy: true });
}

export function battle_tower_mobile_function(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
    name,
  }: {
    runner?: ScriptRunner | null;
    overworld?: Overworld | null;
    event_manager?: EventManager;
    name: string;
  }
): boolean {
  // ASM: mobile/mobile_40.asm::BattleTowerMobileFunction
  void overworld;
  void event_manager;

  _touch_state(game_state, "battle_tower")[name] = true;
  return Boolean(setRunnerValue(runner, 1, { truthy: true }));
}

export function function1011f1(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld | null; event_manager?: EventManager } = {}
): string {
  // ASM: mobile/mobile_40.asm::Function1011f1
  void overworld;
  void event_manager;

  recordMobileHandshake(game_state, { mode: "init" });
  game_state.wram.wLinkMode = LINK_MOBILE;

  setSerialConnectionStatus(game_state, SerialConnectionStatus.CONNECTION_NOT_ESTABLISHED);

  return setRunnerValue(runner, "$0", { truthy: true });
}

export function function101220(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld | null; event_manager?: EventManager } = {}
): boolean {
  // ASM: mobile/mobile_40.asm::Function101220
  void overworld;
  void event_manager;

  mobileState(game_state).terminated = true;
  game_state.wram.wLinkMode = LINK_NULL;

  setSerialConnectionStatus(game_state, SerialConnectionStatus.CONNECTION_NOT_ESTABLISHED);

  return Boolean(setRunnerValue(runner, "$0", { truthy: true }));
}

export function function101225(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld | null; event_manager?: EventManager } = {}
): string {
  // ASM: mobile/mobile_40.asm::Function101225
  void overworld;
  void event_manager;

  const state = recordMobileHandshake(game_state, { mode: "battle" });
  game_state.wram.wLinkMode = LINK_MOBILE;

  setSerialConnectionStatus(game_state, SerialConnectionStatus.USING_EXTERNAL_CLOCK);

  if (runner) {
    const variables = ensureRunnerVariables(runner);
    variables.mobile_session = state;
    variables.battle_tower_leaderboard = state.leaderboard ?? [];
  }

  return setRunnerValue(runner, "$0", { truthy: true });
}

export function function101231(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld | null; event_manager?: EventManager } = {}
): boolean {
  // ASM: mobile/mobile_40.asm::Function101231
  void overworld;
  void event_manager;

  const state = recordMobileHandshake(game_state, { mode: "trade" });
  game_state.wram.wLinkMode = LINK_MOBILE;

  setSerialConnectionStatus(game_state, SerialConnectionStatus.USING_EXTERNAL_CLOCK);

  if (runner) {
    const variables = ensureRunnerVariables(runner);
    variables.mobile_session = state;
  }

  return Boolean(setRunnerValue(runner, "$0", { truthy: true }));
}

export function function103780(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld | null; event_manager?: EventManager } = {}
): boolean {
  // ASM: mobile/mobile_40.asm::Function103780
  return battle_tower_mobile_function(game_state, {
    runner,
    overworld,
    event_manager,
    name: "function103780",
  });
}

export function function1037c2(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld | null; event_manager?: EventManager } = {}
): boolean {
  // ASM: mobile/mobile_40.asm::Function1037c2
  return battle_tower_mobile_function(game_state, {
    runner,
    overworld,
    event_manager,
    name: "function1037c2",
  });
}

export function function1037eb(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld | null; event_manager?: EventManager } = {}
): boolean {
  // ASM: mobile/mobile_40.asm::Function1037eb
  return battle_tower_mobile_function(game_state, {
    runner,
    overworld,
    event_manager,
    name: "function1037eb",
  });
}

export function function10383c(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld | null; event_manager?: EventManager } = {}
): boolean {
  // ASM: mobile/mobile_40.asm::Function10383c
  return battle_tower_mobile_function(game_state, {
    runner,
    overworld,
    event_manager,
    name: "function10383c",
  });
}

export function function10387b(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld | null; event_manager?: EventManager } = {}
): boolean {
  // ASM: mobile/mobile_40.asm::Function10387b
  return battle_tower_mobile_function(game_state, {
    runner,
    overworld,
    event_manager,
    name: "function10387b",
  });
}

export function mobile_select_three_mons(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld | null; event_manager?: EventManager } = {}
): boolean {
  // ASM: mobile/mobile_40.asm::Mobile_SelectThreeMons
  void overworld;
  void event_manager;

  const variables = ensureRunnerVariables(runner ?? undefined);
  const rawIndexes = variables._selected_party_indexes;
  const indexes = Array.isArray(rawIndexes) && rawIndexes.length
    ? rawIndexes
        .map((value, idx) => {
          const numeric = Number(value);
          return Number.isFinite(numeric) ? numeric : idx;
        })
        .filter((value) => Number.isFinite(value))
    : [0, 1, 2];
  _touch_state(game_state, "battle_tower").selected_party_indexes = [...indexes];
  return Boolean(setRunnerValue(runner, 1, { truthy: true }));
}
