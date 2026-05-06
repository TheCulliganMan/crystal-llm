/**
 * Faithful overworld timekeeper mirroring engine/overworld/time.asm.
 */

import type { GameState } from "../../../core/state";
import { DailyEventSystem } from "../../systems/daily-events";
import { TimeSystem, canonicaliseTimeOfDay, type TimeOfDay } from "../../systems/time";
import type { Date as DateModel } from "../../../core/models";

function extractDate(saved: DateModel | null | undefined): globalThis.Date {
  if (!saved) {
    const now = new globalThis.Date();
    now.setHours(0, 0, 0, 0);
    return now;
  }
  try {
    // Month is 0-indexed in JS Date, but 1-indexed in our model
    const date = new globalThis.Date(saved.year, saved.month - 1, saved.day);
    date.setHours(0, 0, 0, 0);
    return date;
  } catch (e) {
    const now = new globalThis.Date();
    now.setHours(0, 0, 0, 0);
    return now;
  }
}

// Helper to compare only the date part of two Date objects
function isSameDate(a: globalThis.Date, b: globalThis.Date): boolean {
  return a.getTime() === b.getTime();
}

export class OverworldTimeSystem {
  private readonly _wram;
  private readonly _sram;
  private readonly _timeSystem: TimeSystem;
  private readonly _dailyEvents: DailyEventSystem;
  private readonly _onTimeOfDayChange?: (previous: TimeOfDay, current: TimeOfDay) => void;

  private _lastTimeOfDay: TimeOfDay;
  private _lastDay: number;
  private _lastDate: globalThis.Date;

  constructor(
    public readonly gameState: GameState,
    {
      dailyEventSystem,
      onTimeOfDayChange,
    }: {
      dailyEventSystem?: DailyEventSystem;
      onTimeOfDayChange?: (previous: TimeOfDay, current: TimeOfDay) => void;
    } = {}
  ) {
    this._wram = gameState.wram;
    this._sram = gameState.sram;
    this._timeSystem = new TimeSystem(gameState);
    this._dailyEvents = dailyEventSystem ?? new DailyEventSystem(gameState);
    this._onTimeOfDayChange = onTimeOfDayChange;

    this._lastTimeOfDay = canonicaliseTimeOfDay(this._wram.time_of_day ?? "day") as TimeOfDay;
    this._lastDay = this._wram.wCurDay ?? 0;
    this._lastDate = extractDate(this._sram.current_date);
  }

  /**
   * Advance the overworld clock and dispatch any resulting events.
   */
  public tick(): void {
    this._timeSystem.updateTime();
    this._checkDayChange();
    this._checkTimeOfDayChange();
  }

  private _checkDayChange(): void {
    const currentDay = this._wram.wCurDay;
    const currentDate = extractDate(this._sram.current_date);

    if (currentDay === this._lastDay && isSameDate(currentDate, this._lastDate)) {
      return;
    }

    this._lastDay = currentDay;
    this._lastDate = currentDate;
    this._dailyEvents.process({ currentDate });
  }

  private _checkTimeOfDayChange(): void {
    const rawTime = this._wram.time_of_day;
    const currentTime = canonicaliseTimeOfDay(rawTime) as TimeOfDay;
    if (currentTime === this._lastTimeOfDay) {
      return;
    }

    const previous = this._lastTimeOfDay;
    this._lastTimeOfDay = currentTime;
    if (this._onTimeOfDayChange) {
      this._onTimeOfDayChange(previous, currentTime);
    }
  }
}
