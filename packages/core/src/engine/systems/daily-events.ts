/**
 * Daily event helpers for time-sensitive world state.
 *
 * @see {@link https://github.com/pret/pokecrystal/blob/master/engine/events/daily_events.asm}
 */

import type { GameState } from "../../core/state";
import { DateSchema, type Date as DateModel } from "../../core/models";
import { HardwareRNG } from "../games/rng";

function sampleKenjiBreakCountdown(gameState: GameState): void {
  // Mirrors `SampleKenjiBreakCountdown` in `engine/overworld/time.asm`.
  const rng = new HardwareRNG(gameState);
  const value = (rng.nextByte() & 0x03) + 3;
  gameState.wram.wKenjiBreakTimer = value;
}

function toDate(date: DateModel | null | undefined): globalThis.Date {
  if (date === null || date === undefined) {
    return new globalThis.Date();
  }
  try {
    return new globalThis.Date(date.year, date.month - 1, date.day);
  } catch {
    return new globalThis.Date();
  }
}

function isSameDay(a: globalThis.Date, b: globalThis.Date): boolean {
  return (
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate()
  );
}

export class DailyEventSystem {
  private _lastResetDate: globalThis.Date;

  constructor(public readonly gameState: GameState) {
    this._lastResetDate = toDate(this.gameState.sram.current_date);
  }

  /**
   * Check for a calendar day change and run the reset logic if needed.
   */
  public process({ currentDate }: { currentDate?: globalThis.Date }): void {
    const target = currentDate ?? toDate(this.gameState.sram.current_date);
    if (isSameDay(target, this._lastResetDate)) {
      return;
    }
    this._resetForDay(target);
  }

  /**
   * Trigger faithful daily resets for PC Crystal events.
   */
  private _resetForDay(target: globalThis.Date): void {
    this._lastResetDate = target;
    this._updateSavedDate(target);
    this._clearDailyFlags();
    this._clearFruitTreeFlags();
    this._resetMysteryGiftLimit();
    this._refreshKenjiTimer();
  }

  private _updateSavedDate(target: globalThis.Date): void {
    this.gameState.sram.current_date = DateSchema.parse({
      year: target.getFullYear(),
      month: target.getMonth() + 1,
      day: target.getDate(),
    });
  }

  private _clearDailyFlags(): void {
    const wram = this.gameState.wram;
    wram.daily_reset_timer = 0;
    wram.daily_flags1 = 0;
    wram.daily_flags2 = 0;
    wram.swarm_flags = 0;
    this._zeroList(wram.daily_rematch_flags);
    this._zeroList(wram.daily_phone_item_flags);
    this._zeroList(wram.daily_phone_time_of_day_flags);
    wram.engine_flags.ENGINE_DAILY_BUG_CONTEST = false;
  }

  private _clearFruitTreeFlags(): void {
    const toClear: string[] = [];
    for (const key of Object.keys(this.gameState.wram.event_flags)) {
      if (key.startsWith("FRUITTREE_") && key.endsWith("_COLLECTED")) {
        toClear.push(key);
      }
    }
    for (const key of toClear) {
      delete this.gameState.wram.event_flags[key];
      delete this.gameState.sram.event_flags[key];
    }
  }

  private _resetMysteryGiftLimit(): void {
    // Mirrors `ResetDailyMysteryGiftLimitIfUnlocked` in `engine/link/mystery_gift.asm`.
    if (!this.gameState.sram.mystery_gift_unlocked) {
      return;
    }
    this.gameState.sram.mystery_gift.daily_partner_ids = [];
  }

  private _refreshKenjiTimer(): void {
    const wram = this.gameState.wram;
    let timer = wram.wKenjiBreakTimer ?? 0;
    if (timer > 0) {
      timer -= 1;
    }
    if (timer <= 0) {
      sampleKenjiBreakCountdown(this.gameState);
    } else {
      wram.wKenjiBreakTimer = timer;
    }
  }

  private _zeroList(target: number[]): void {
    for (let i = 0; i < target.length; i++) {
      target[i] = 0;
    }
  }
}
