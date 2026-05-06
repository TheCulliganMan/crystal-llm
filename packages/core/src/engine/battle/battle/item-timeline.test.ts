import { createInitialGameState } from "@pokecrystal/core/core/state";
import { BattleTurn } from "@pokecrystal/core/core/enums";
import type { Item, Pokemon } from "@pokecrystal/core/core/models";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { BattleItemTimeline, type QueuedBattleItem } from "./item-timeline";

const queuedItem = (): QueuedBattleItem => ({
  attackerSide: BattleTurn.PLAYER,
  item: { name: "POTION" } as unknown as Item,
  target: { nickname: "CHIKORITA" } as unknown as Pokemon,
  moveIndex: null,
});

describe("BattleItemTimeline", () => {
  it("applies queued items on EventManager frame delay boundaries", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const timeline = new BattleItemTimeline(eventManager, 2);
    const queued = queuedItem();
    const applyCallback = jest.fn(() => true);
    const onComplete = jest.fn();

    timeline.queue(queued, applyCallback, onComplete);

    expect(timeline.applying).toBe(true);
    expect(applyCallback).not.toHaveBeenCalled();
    expect(onComplete).not.toHaveBeenCalled();

    eventManager.advanceFrame();
    expect(applyCallback).not.toHaveBeenCalled();

    eventManager.advanceFrame();
    expect(applyCallback).toHaveBeenCalledTimes(1);
    expect(applyCallback).toHaveBeenCalledWith(queued);
    expect(onComplete).toHaveBeenCalledTimes(1);
    expect(onComplete).toHaveBeenCalledWith(queued, true);
    expect(timeline.applying).toBe(false);
  });

  it("falls back to immediate apply when no EventManager is present", () => {
    const timeline = new BattleItemTimeline(null, 12);
    const queued = queuedItem();
    const applyCallback = jest.fn(() => false);
    const onComplete = jest.fn();

    timeline.queue(queued, applyCallback, onComplete);

    expect(applyCallback).toHaveBeenCalledTimes(1);
    expect(onComplete).toHaveBeenCalledTimes(1);
    expect(onComplete).toHaveBeenCalledWith(queued, false);
    expect(timeline.applying).toBe(false);
  });
});
