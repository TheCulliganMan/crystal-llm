import { GameState } from "../../../core/state";
import { ItemSystem } from "../../systems/items";
import { showText, waitForInput, type EventManager } from "../../events/events";
import type { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import type { DataLoader } from "@pokecrystal/core/core/data-loader";
import { ScriptRunner } from "./utils";

type MysteryGiftOverworld = {
  audio_engine?: AudioEngine | null;
  data_loader?: DataLoader | null;
  item_system?: ItemSystem | null;
} | null | undefined;

const resolveItemSystem = (
  game_state: GameState,
  {
    runner,
    overworld,
  }: { runner?: ScriptRunner | null; overworld?: MysteryGiftOverworld } = {}
): ItemSystem => {
  const candidate = overworld?.item_system;
  if (candidate instanceof ItemSystem) {
    return candidate;
  }
  const runnerCandidate = runner?.item_system;
  if (runnerCandidate instanceof ItemSystem) {
    return runnerCandidate;
  }
  const dataLoader = runner?.data_loader ?? overworld?.data_loader;
  return new ItemSystem(game_state, dataLoader ?? undefined);
};

export function check_mystery_gift({ game_state }: { game_state: GameState }): number {
  // ASM: engine/events/specials.asm::CheckMysteryGift
  return Number(Boolean(game_state.sram.mystery_gift?.stored_item));
}

export function get_mystery_gift_item({
  game_state,
  runner,
  overworld,
  event_manager,
}: {
  game_state: GameState;
  runner?: ScriptRunner | null;
  overworld?: MysteryGiftOverworld;
  event_manager?: EventManager | null;
}): boolean {
  // ASM: engine/events/specials.asm::GetMysteryGiftItem
  const saved = game_state.sram.mystery_gift;
  const itemName = saved?.stored_item ?? null;
  if (!itemName) {
    return false;
  }

  const itemSystem = resolveItemSystem(game_state, { runner, overworld });
  let added = false;
  try {
    added = itemSystem.addItem(itemName);
  } catch (error) {
    added = false;
  }
  if (!added) {
    return false;
  }

  saved.stored_item = null;
  saved.backup_item = null;

  let displayName = itemSystem.getDisplayName(itemName).trim();
  displayName = displayName.replace(/#/g, "POKE").replace(/\n/g, " ").trim();
  if (!displayName) {
    displayName = "ITEM";
  }

  const player = String(game_state.sram.player_name ?? "").trim() || "PLAYER";
  const message = `${player} received\n${displayName}!`;

  const audioEngine = overworld?.audio_engine ?? runner?.audio_engine ?? null;
  if (audioEngine?.playSound) {
    audioEngine.playSound("SFX_ITEM");
  }
  if (runner) {
    runner.last_sound_effect = "SFX_ITEM";
  }

  if (event_manager) {
    showText(event_manager, message);
    const pause = runner?.pause;
    if (typeof pause === "function") {
      pause();
    }
    waitForInput(event_manager);
  }

  return true;
}

export function unlock_mystery_gift({
  game_state,
  runner,
  overworld,
  event_manager,
}: {
  game_state: GameState;
  runner?: ScriptRunner | null;
  overworld?: MysteryGiftOverworld;
  event_manager?: EventManager | null;
}): boolean {
  // ASM: engine/link/mystery_gift.asm::UnlockMysteryGift
  void overworld;
  void event_manager;

  const locked = !Boolean(game_state.sram.mystery_gift_unlocked);
  if (locked) {
    game_state.sram.mystery_gift_unlocked = true;
    game_state.sram.mystery_gift.stored_item = null;
    game_state.sram.mystery_gift.backup_item = null;
  }

  if (runner) {
    runner.last_condition_result = true;
    runner.last_value = locked ? 1 : 0;
  }
  return Boolean(locked);
}
