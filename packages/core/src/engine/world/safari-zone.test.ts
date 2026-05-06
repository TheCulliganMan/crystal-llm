import { GameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import {
  isSafariMap,
  safariIsActive,
  startSafariZone,
  endSafariZone,
  advanceSafariTimer,
  applySafariBattleType,
  spendSafariBall,
  SAFARI_BATTLE_TYPE,
  DEFAULT_SAFARI_BALLS,
  DEFAULT_SAFARI_TIME,
} from "./safari-zone";

describe("safari_zone", () => {
  let gameState: GameState;
  let eventManager: EventManager;

  beforeEach(() => {
    gameState = {
      wram: {},
    } as GameState;
    eventManager = new EventManager(gameState);
  });

  describe("isSafariMap", () => {
    it("should return true for safari maps", () => {
      expect(isSafariMap("SafariZoneBeta")).toBe(true);
      expect(isSafariMap("SafariZoneFuchsiaGateBeta")).toBe(true);
    });

    it("should return false for non-safari maps", () => {
      expect(isSafariMap("SomeOtherMap")).toBe(false);
    });

    it("should return false for null or undefined map names", () => {
      expect(isSafariMap(null)).toBe(false);
      expect(isSafariMap(undefined)).toBe(false);
    });
  });

  describe("safariIsActive", () => {
    it("should return true when safari mode is active", () => {
      gameState.wram.safari_active = true;
      expect(safariIsActive(gameState)).toBe(true);
    });

    it("should return false when safari mode is not active", () => {
      gameState.wram.safari_active = false;
      expect(safariIsActive(gameState)).toBe(false);
    });

    it("should return false when safari_active is not set", () => {
      expect(safariIsActive(gameState)).toBe(false);
    });
  });

  describe("startSafariZone", () => {
    it("should start safari mode with default values", () => {
      startSafariZone(gameState);
      expect(gameState.wram.safari_active).toBe(true);
      expect(gameState.wram.safari_balls_remaining).toBe(DEFAULT_SAFARI_BALLS);
      expect(gameState.wram.safari_time_remaining).toBe(DEFAULT_SAFARI_TIME);
      expect(gameState.wram.safari_bait_remaining).toBe(0);
      expect(gameState.wram.safari_rocks_remaining).toBe(0);
      expect(gameState.wram.battle_type).toBe(SAFARI_BATTLE_TYPE);
    });

    it("should start safari mode with custom values", () => {
      startSafariZone(gameState, { balls: 10, timeLimit: 100 });
      expect(gameState.wram.safari_active).toBe(true);
      expect(gameState.wram.safari_balls_remaining).toBe(10);
      expect(gameState.wram.safari_time_remaining).toBe(100);
    });
  });

  describe("endSafariZone", () => {
    it("should end safari mode", () => {
      startSafariZone(gameState);
      endSafariZone(gameState);
      expect(gameState.wram.safari_active).toBe(false);
      expect(gameState.wram.safari_balls_remaining).toBe(0);
      expect(gameState.wram.safari_time_remaining).toBe(0);
      expect(gameState.wram.safari_bait_remaining).toBe(0);
      expect(gameState.wram.safari_rocks_remaining).toBe(0);
      expect(gameState.wram.battle_type).toBe("BATTLETYPE_NORMAL");
    });

    it("should not do anything if safari mode is not active", () => {
      const wram = { ...gameState.wram };
      endSafariZone(gameState);
      expect(gameState.wram).toEqual(wram);
    });

    it("should announce the end of the session", () => {
      startSafariZone(gameState);
      const dispatchSpy = jest.spyOn(eventManager, "dispatch");
      endSafariZone(gameState, { eventManager, announce: true });
      expect(dispatchSpy).toHaveBeenCalledTimes(2);
      expect(dispatchSpy).toHaveBeenCalledWith(
        expect.objectContaining({
          name: "show_text",
          data: { text: "The Safari game has ended." },
        })
      );
      expect(dispatchSpy).toHaveBeenCalledWith(
        expect.objectContaining({ name: "wait_for_input" })
      );
    });
  });

  describe("advanceSafariTimer", () => {
    it("should decrement the timer", () => {
      startSafariZone(gameState);
      advanceSafariTimer(gameState);
      expect(gameState.wram.safari_time_remaining).toBe(DEFAULT_SAFARI_TIME - 1);
    });

    it("should end the session when the timer expires", () => {
      startSafariZone(gameState, { timeLimit: 1 });
      const result = advanceSafariTimer(gameState);
      expect(result).toBe(true);
      expect(safariIsActive(gameState)).toBe(false);
    });

    it("should not do anything if safari mode is not active", () => {
      const wram = { ...gameState.wram };
      const result = advanceSafariTimer(gameState);
      expect(result).toBe(false);
      expect(gameState.wram).toEqual(wram);
    });
  });

  describe("applySafariBattleType", () => {
    it("should set the battle type to safari", () => {
      startSafariZone(gameState);
      applySafariBattleType(gameState);
      expect(gameState.wram.battle_type).toBe(SAFARI_BATTLE_TYPE);
    });

    it("should not do anything if safari mode is not active", () => {
      gameState.wram.battle_type = "BATTLETYPE_NORMAL";
      applySafariBattleType(gameState);
      expect(gameState.wram.battle_type).toBe("BATTLETYPE_NORMAL");
    });
  });

  describe("spendSafariBall", () => {
    it("should decrement the ball count", () => {
      startSafariZone(gameState);
      const result = spendSafariBall(gameState);
      expect(result).toBe(true);
      expect(gameState.wram.safari_balls_remaining).toBe(DEFAULT_SAFARI_BALLS - 1);
    });

    it("should return false if no balls remain", () => {
      startSafariZone(gameState, { balls: 0 });
      const result = spendSafariBall(gameState);
      expect(result).toBe(false);
      expect(gameState.wram.safari_balls_remaining).toBe(0);
    });
  });
});
