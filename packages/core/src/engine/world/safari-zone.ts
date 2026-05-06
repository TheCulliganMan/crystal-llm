import { GameState } from "@pokecrystal/core/core/state";
import { EventManager, Event } from "@pokecrystal/core/engine/events/events";

export const SAFARI_BATTLE_TYPE = "BATTLETYPE_SAFARI";
export const DEFAULT_SAFARI_BALLS = 30;
export const DEFAULT_SAFARI_TIME = 500;
export const SAFARI_MAPS: ReadonlySet<string> = new Set([
  "SafariZoneBeta",
  "SafariZoneFuchsiaGateBeta",
]);

/**
 * Return True if the requested map hosts Safari encounters.
 * @param mapName The name of the map to check.
 * @returns True if the map is a Safari map.
 */
export function isSafariMap(mapName?: string | null): boolean {
  if (!mapName) {
    return false;
  }
  return SAFARI_MAPS.has(mapName);
}

/**
 * Indicate whether Safari mode is currently running.
 * @param gameState The current game state.
 * @returns True if Safari mode is active.
 */
export function safariIsActive(gameState: GameState): boolean {
  return gameState.wram.safari_active ?? false;
}

/**
 * Begin a Safari session, resetting balls and time.
 * @param gameState The current game state.
 * @param balls The number of balls to start with.
 * @param timeLimit The time limit for the session.
 */
export function startSafariZone(
  gameState: GameState,
  {
    balls = DEFAULT_SAFARI_BALLS,
    timeLimit = DEFAULT_SAFARI_TIME,
  }: { balls?: number; timeLimit?: number } = {}
): void {
  const { wram } = gameState;
  wram.safari_active = true;
  wram.safari_balls_remaining = Math.max(0, balls);
  wram.safari_time_remaining = Math.max(0, timeLimit);
  wram.safari_bait_remaining = 0;
  wram.safari_rocks_remaining = 0;
  wram.battle_type = SAFARI_BATTLE_TYPE;
}

/**
 * Conclude the current Safari session and clear its state.
 * @param gameState The current game state.
 * @param eventManager The event manager to dispatch events.
 * @param announce Whether to announce the end of the session.
 */
export function endSafariZone(
  gameState: GameState,
  {
    eventManager,
    announce = false,
  }: { eventManager?: EventManager; announce?: boolean } = {}
): void {
  if (!safariIsActive(gameState)) {
    return;
  }
  const { wram } = gameState;
  wram.safari_active = false;
  wram.safari_balls_remaining = 0;
  wram.safari_time_remaining = 0;
  wram.safari_bait_remaining = 0;
  wram.safari_rocks_remaining = 0;
  if (wram.battle_type === SAFARI_BATTLE_TYPE) {
    wram.battle_type = "BATTLETYPE_NORMAL";
  }
  if (eventManager && announce) {
    eventManager.dispatch(new Event("show_text", { text: "The Safari game has ended." }));
    eventManager.dispatch(new Event("wait_for_input", {}));
  }
}

/**
 * Consume one Safari step; return True when the timer expires.
 * @param gameState The current game state.
 * @param eventManager The event manager to dispatch events.
 * @returns True when the timer expires.
 */
export function advanceSafariTimer(
  gameState: GameState,
  { eventManager }: { eventManager?: EventManager } = {}
): boolean {
  if (!safariIsActive(gameState)) {
    return false;
  }
  const { wram } = gameState;
  if (wram.safari_time_remaining <= 0) {
    return false;
  }
  wram.safari_time_remaining -= 1;
  if (wram.safari_time_remaining > 0) {
    return false;
  }
  endSafariZone(gameState, { eventManager, announce: true });
  return true;
}

/**
 * Ensure the current battle type matches the Safari state.
 * @param gameState The current game state.
 */
export function applySafariBattleType(gameState: GameState): void {
  if (safariIsActive(gameState)) {
    gameState.wram.battle_type = SAFARI_BATTLE_TYPE;
  }
}

/**
 * Decrement the Safari ball count; return False if none remain.
 * @param gameState The current game state.
 * @returns False if no balls remain.
 */
export function spendSafariBall(gameState: GameState): boolean {
  const { wram } = gameState;
  const count = wram.safari_balls_remaining ?? 0;
  if (count <= 0) {
    return false;
  }
  wram.safari_balls_remaining = count - 1;
  return true;
}
