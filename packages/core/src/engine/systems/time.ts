import { z } from 'zod';
import { GameState } from '@pokecrystal/core/core/state';
import { GameDate, DateSchema } from '@pokecrystal/core/core/models/date';
import { Time, TimeSchema } from '@pokecrystal/core/core/models/time';
import { TimeOfDay } from '@pokecrystal/core/core/enums';
export type { TimeOfDay };

// Time of day constants (matching ASM)
export const MORN_HOUR = 4; // 4:00
export const DAY_HOUR = 10; // 10:00
export const NITE_HOUR = 18; // 18:00
export const MAX_HOUR = 24; // 24:00

// Time of day values
export const MORN_F = 1; // Morning
export const DAY_F = 2; // Day
export const NITE_F = 4; // Night

// Canonical time of day labels
const _TIME_OF_DAY_ALIASES: { [key: string]: string } = {
  morning: 'MORN',
  morn: 'MORN',
  day: 'DAY',
  afternoon: 'DAY',
  evening: 'NIGHT',
  night: 'NIGHT',
  nite: 'NIGHT',
  dark: 'NIGHT',
  darkness: 'NIGHT',
};

export function canonicaliseTimeOfDay(label: string | null | undefined): TimeOfDay {
  if (label === 'MORN' || label === 'DAY' || label === 'NIGHT') {
    return label as TimeOfDay; // already canonical
  }
  const token = label === null || label === undefined ? 'DAY' : String(label).trim().toLowerCase();
  return (_TIME_OF_DAY_ALIASES[token] || 'DAY') as TimeOfDay;
}

// RTC status flags
export const RTC_RESET = 0x80;
export const RTC_DAYS_EXCEED_255 = 0x40;
export const RTC_DAYS_EXCEED_139 = 0x20;

// RTC DH register bits
export const B_RAMB_RTC_DH_HIGH = 0;
export const B_RAMB_RTC_DH_HALT = 6;
export const B_RAMB_RTC_DH_CARRY = 7;

function setIfChanged<T, K extends keyof T>(target: T, attr: K, value: T[K]): void {
    if (target[attr] !== value) {
        target[attr] = value;
    }
}

function mod(n: number, m: number): number {
  return ((n % m) + m) % m;
}

export class TimeSystem {
    private gameState: GameState;
    private _hram: GameState['hram'];
    private _wram: GameState['wram'];
    private _sram: GameState['sram'];
    private _last_rtc_day_count: number;
    private _last_clock_snapshot: string | null;

    constructor(gameState: GameState) {
        this.gameState = gameState;
        this._hram = gameState.hram;
        this._wram = gameState.wram;
        this._sram = gameState.sram;
        this._last_rtc_day_count = 0;
        this._last_clock_snapshot = null;

        if (this._sram.rtc_anchor == null) {
            const now = new Date();
            this._sram.rtc_anchor = { year: now.getFullYear(), month: now.getMonth() + 1, day: now.getDate() };
        }
    }

    public updateTime(): void {
        const now = new Date();
        const snapshot = this._clockSnapshot(now);
        if (snapshot === this._last_clock_snapshot) {
            return;
        }
        this._getClock(now);
        this._updateTimeRegisters();
        this._last_clock_snapshot = this._clockSnapshot(now);
    }

    public setManualTime({ day, hour = 0, minute = 0, second = 0 }: { day?: number | null, hour?: number, minute?: number, second?: number }): void {
        const targetDay = mod(day !== null && day !== undefined ? day : this._sram.day_of_week, 7);
        const target: Time = {
            day: targetDay,
            hour: mod(hour, 24),
            minute: mod(minute, 60),
            second: mod(second, 60),
        };

        this._getClock();
        this._setStartOffset(target);
        this._updateTimeRegisters();
    }

    private _clockSnapshot(now: Date): string {
        const sram = this._sram;
        const wram = this._wram;

        return JSON.stringify({
            now: now.getTime(),
            start: sram.start_time,
            anchor: sram.rtc_anchor,
            currentDate: sram.current_date,
            wCurDay: wram.wCurDay,
            timeOfDay: wram.time_of_day,
        });
    }

    private _getClock(now: Date | null = null): void {
        if (now === null) {
            now = new Date();
        }
        this._last_rtc_day_count = this._computeDayCount(now);
        const high_day_bit = (this._last_rtc_day_count >> 8) & 0x01;

        const hram = this._hram;
        setIfChanged(hram, 'hRTCSeconds', now.getSeconds());
        setIfChanged(hram, 'hRTCMinutes', now.getMinutes());
        setIfChanged(hram, 'hRTCHours', now.getHours());

        const day_lo = this._last_rtc_day_count & 0xFF;
        const day_hi = high_day_bit << B_RAMB_RTC_DH_HIGH;
        setIfChanged(hram, 'hRTCDayLo', day_lo);
        setIfChanged(hram, 'hRTCDayHi', day_hi);

        const sram = this._sram;
        const current_date = sram.current_date;
        if (
            current_date?.year !== now.getFullYear() ||
            current_date?.month !== now.getMonth() + 1 ||
            current_date?.day !== now.getDate()
        ) {
            sram.current_date = { year: now.getFullYear(), month: now.getMonth() + 1, day: now.getDate() };
        }
    }

    private _updateTimeRegisters(): void {
        this._fixDays();
        this._fixTime();
        this._syncGameTimeFields();
        this._getTimeOfDay();
    }

    private _computeDayCount(now: Date): number {
        let anchor = this._sram.rtc_anchor;
        if (anchor === null) {
            anchor = { year: now.getFullYear(), month: now.getMonth() + 1, day: now.getDate() };
            this._sram.rtc_anchor = anchor;
        }

        if (!anchor) {
          return 0;
        }
        const anchorDate = new Date(anchor.year, anchor.month - 1, anchor.day);
        const nowDate = new Date(now.getFullYear(), now.getMonth(), now.getDate());

        const delta = (nowDate.getTime() - anchorDate.getTime()) / (1000 * 60 * 60 * 24);

        if (delta < 0) {
            this._sram.rtc_anchor = { year: now.getFullYear(), month: now.getMonth() + 1, day: now.getDate() };
            return 0;
        }
        return Math.floor(delta);
    }

    private _setStartOffset(target: Time): void {
        const rtc = this._hram;
        const seconds_raw = target.second - rtc.hRTCSeconds;
        const start_seconds = mod(seconds_raw, 60);
        const carry_minutes = seconds_raw < 0 ? 1 : 0;

        const minutes_raw = target.minute - rtc.hRTCMinutes - carry_minutes;
        const start_minutes = mod(minutes_raw, 60);
        const carry_hours = minutes_raw < 0 ? 1 : 0;

        const hours_raw = target.hour - rtc.hRTCHours - carry_hours;
        const start_hours = mod(hours_raw, 24);
        const carry_days = hours_raw < 0 ? 1 : 0;

        const base_day = mod(this._last_rtc_day_count, 7);
        const day_raw = target.day - (base_day + carry_days);
        const start_day = mod(day_raw, 256);

        this._sram.start_time = {
            day: start_day,
            hour: start_hours,
            minute: start_minutes,
            second: start_seconds,
        };
    }

    private _fixDays(): void {
        const hram = this._hram;
        let status = 0;
        let day_lo = hram.hRTCDayLo;
        let day_hi = hram.hRTCDayHi;

        if (day_hi & (1 << B_RAMB_RTC_DH_HIGH)) {
            day_hi &= ~(1 << B_RAMB_RTC_DH_HIGH);
            day_lo = mod(day_lo, 140);
            status |= RTC_DAYS_EXCEED_255;
        } else if (day_lo >= 140) {
            day_lo = mod(day_lo, 140);
            status |= RTC_DAYS_EXCEED_139;
        }

        setIfChanged(hram, 'hRTCDayLo', day_lo);
        setIfChanged(hram, 'hRTCDayHi', day_hi);
        setIfChanged(this._sram, 'rtc_status_flags', status);
    }

    private _fixTime(): void {
        const start = this._sram.start_time;
        const hram = this._hram;
        const wram = this._wram;

        const total_seconds = hram.hRTCSeconds + start.second;
        const total_minutes = hram.hRTCMinutes + start.minute;
        const total_hours = hram.hRTCHours + start.hour;
        const total_days = hram.hRTCDayLo + start.day;

        const seconds = mod(total_seconds, 60);
        setIfChanged(hram, 'hSeconds', seconds);
        const carry_minutes = Math.floor(total_seconds / 60);

        const minutes_total = total_minutes + carry_minutes;
        const minutes = mod(minutes_total, 60);
        setIfChanged(hram, 'hMinutes', minutes);
        const carry_hours = Math.floor(minutes_total / 60);

        const hours_total = total_hours + carry_hours;
        const hours = mod(hours_total, 24);
        setIfChanged(hram, 'hHours', hours);
        const carry_days = Math.floor(hours_total / 24);

        const current_day = mod(total_days + carry_days, 256);
        setIfChanged(wram, 'wCurDay', current_day);
    }

    private _syncGameTimeFields(): void {
        const sram = this._sram;
        const hram = this._hram;
        const wram = this._wram;

        setIfChanged(sram, 'game_time_seconds', hram.hSeconds);
        setIfChanged(sram, 'game_time_minutes', hram.hMinutes);
        setIfChanged(sram, 'game_time_hours', hram.hHours);
        setIfChanged(sram, 'day_of_week', mod(wram.wCurDay, 7));
    }

    private _getTimeOfDay(): void {
        const hour = this._hram.hHours;
        let period: TimeOfDay;

        if (hour < MORN_HOUR) {
            period = TimeOfDay.NIGHT;
        } else if (hour < DAY_HOUR) {
            period = TimeOfDay.MORN;
        } else if (hour < NITE_HOUR) {
            period = TimeOfDay.DAY;
        } else {
            period = TimeOfDay.NIGHT;
        }
        setIfChanged(this._wram, 'time_of_day', canonicaliseTimeOfDay(period));
    }
}

export function syncGameClock(gameState: GameState): void {
    new TimeSystem(gameState).updateTime();
}
