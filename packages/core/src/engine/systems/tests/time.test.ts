import { TimeSystem } from '../time';
import { GameState, WRAMSchema, HRAMSchema, SRAMSchema, GameStateSchema } from '../../../core/state';
import { TimeOfDay } from '../../../core/enums';
import { z } from 'zod';

const createMockGameState = (): GameState => {
    return GameStateSchema.parse({
      sram: SRAMSchema.parse({}),
      wram: WRAMSchema.parse({}),
      hram: HRAMSchema.parse({
        joypad: {
            hJoypadReleased: 0,
            hJoypadPressed: 0,
            hJoypadDown: 0,
            hJoypadSum: 0,
            hJoyReleased: 0,
            hJoyPressed: 0,
            hJoyDown: 0,
            hJoyLast: 0,
        }
      }),
      vram: {
        bank0: { tile_blocks: {}, sprite_pages: {}, bg_maps: {} },
        bank1: { tile_blocks: {}, sprite_pages: {}, bg_maps: {} },
      },
      has_seen_intro: false,
    });
  };

describe('TimeSystem', () => {
    let gameState: GameState;
    let timeSystem: TimeSystem;

    beforeEach(() => {
        gameState = createMockGameState();
        timeSystem = new TimeSystem(gameState);
        jest.useFakeTimers();
      });

      afterEach(() => {
        jest.useRealTimers();
      });

      it('should initialize with the current date if no rtc_anchor is set', () => {
        const now = new Date();
        timeSystem.updateTime();
        expect(gameState.sram.rtc_anchor).toEqual({
            year: now.getFullYear(),
            month: now.getMonth() + 1,
            day: now.getDate(),
        })
      });

      it('should update the time correctly', () => {
          jest.setSystemTime(new Date('2024-01-01T12:00:00'));

          timeSystem.updateTime();

          expect(gameState.hram.hRTCHours).toBe(12);
          expect(gameState.hram.hRTCMinutes).toBe(0);
          expect(gameState.hram.hRTCSeconds).toBe(0);
          expect(gameState.wram.time_of_day).toBe(TimeOfDay.DAY);
      });

      it('should set manual time correctly', () => {
          jest.setSystemTime(new Date('2024-01-01T12:00:00'));

          timeSystem.setManualTime({ day: null, hour: 20, minute: 30, second: 15 });

          expect(gameState.hram.hHours).toBe(20);
          expect(gameState.hram.hMinutes).toBe(30);
          expect(gameState.hram.hSeconds).toBe(15);
          expect(gameState.wram.time_of_day).toBe(TimeOfDay.NIGHT);
      });

      it('should correctly identify morning', () => {
          jest.setSystemTime(new Date('2024-01-01T06:00:00'));
          timeSystem.updateTime();
          expect(gameState.wram.time_of_day).toBe(TimeOfDay.MORN);
      });

      it('should correctly identify night', () => {
          jest.setSystemTime(new Date('2024-01-01T22:00:00'));
          timeSystem.updateTime();
          expect(gameState.wram.time_of_day).toBe(TimeOfDay.NIGHT);
      });

      it('should handle negative time adjustments correctly', () => {
        jest.setSystemTime(new Date('2024-01-01T12:00:00'));
        timeSystem.updateTime(); // Initialize time

        // Set start_time to effectively move time backward
        gameState.sram.start_time = { day: 0, hour: 0, minute: 0, second: -30 };

        timeSystem['_fixTime']();

        // With floor division, -30 / 60 = -1, so carry_minutes should be -1
        // and minutes should roll back.
        expect(gameState.hram.hMinutes).toBe(59);
      });
});
