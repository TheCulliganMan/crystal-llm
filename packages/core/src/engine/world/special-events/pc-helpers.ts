import { GameState } from "../../../core/state";

export enum PCHubState {
  BEFORE_POKEDEX = "before_pokedex",
  BEFORE_HOF = "before_hof",
  POSTGAME = "postgame",
}

export enum PCHubAction {
  BILLS_PC = "bills_pc",
  PLAYER_PC = "player_pc",
  OAK_PC = "oak_pc",
  HALL_OF_FAME = "hall_of_fame",
  TURN_OFF = "turn_off",
}

export type PCHubEntry = {
  label: string;
  action: PCHubAction;
};

export function has_pokedex(game_state: GameState): boolean {
  const flags = game_state.wram.engine_flags ?? {};
  const sram = game_state.sram;
  return Boolean((sram && sram.johto_pokedex) || flags["ENGINE_POKEDEX"]);
}

function coerceHallOfFameCount(value: unknown): number | null {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return null;
  }
  return value & 0xff;
}

export function has_hall_of_fame_record(game_state: GameState): boolean {
  // Mirrors pokecrystal_disassembly/engine/events/pokecenter_pc.asm::ChooseWhichPCListToUse.
  const count = coerceHallOfFameCount(game_state.wram?.wHallOfFameCount);
  if (count !== null && count > 0) {
    return true;
  }

  // Compatibility fallback for legacy saves whose missing count parsed as the schema default.
  const entries = game_state.sram?.hall_of_fame ?? [];
  return Boolean(entries.length);
}

export function pc_hub_state(game_state: GameState): PCHubState {
  if (has_hall_of_fame_record(game_state)) {
    return PCHubState.POSTGAME;
  }
  if (has_pokedex(game_state)) {
    return PCHubState.BEFORE_HOF;
  }
  return PCHubState.BEFORE_POKEDEX;
}

const playerPcLabel = (game_state: GameState): string => {
  let playerName = String(game_state.sram?.player_name ?? "").trim();
  if (!playerName) {
    playerName = "PLAYER";
  }
  return `${playerName}'s PC`;
};

export function pc_hub_entries(
  game_state: GameState,
  { include_hall_of_fame = false }: { include_hall_of_fame?: boolean } = {}
): PCHubEntry[] {
  const baseEntries: PCHubEntry[] = [
    { label: "BILL's PC", action: PCHubAction.BILLS_PC },
    { label: playerPcLabel(game_state), action: PCHubAction.PLAYER_PC },
  ];
  const state = pc_hub_state(game_state);
  if (state === PCHubState.BEFORE_POKEDEX) {
    return [...baseEntries, { label: "TURN OFF", action: PCHubAction.TURN_OFF }];
  }
  if (state === PCHubState.BEFORE_HOF) {
    return [
      ...baseEntries,
      { label: "PROF.OAK's PC", action: PCHubAction.OAK_PC },
      { label: "TURN OFF", action: PCHubAction.TURN_OFF },
    ];
  }

  const entries: PCHubEntry[] = [
    ...baseEntries,
    { label: "PROF.OAK's PC", action: PCHubAction.OAK_PC },
  ];
  if (include_hall_of_fame || state === PCHubState.POSTGAME) {
    entries.push({ label: "HALL OF FAME", action: PCHubAction.HALL_OF_FAME });
  }
  entries.push({ label: "TURN OFF", action: PCHubAction.TURN_OFF });
  return entries;
}

export function pc_hub_options(
  game_state: GameState,
  { include_hall_of_fame = false }: { include_hall_of_fame?: boolean } = {}
): string[] {
  return pc_hub_entries(game_state, { include_hall_of_fame }).map((entry) => entry.label);
}

export const pcHubEntries = pc_hub_entries;
