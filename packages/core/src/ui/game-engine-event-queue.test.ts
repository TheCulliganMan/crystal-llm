import { gameEngine } from "./game-engine";

describe("game-engine event queue", () => {
  afterEach(() => {
    gameEngine.event.get(gameEngine.event.getActiveQueue());
    gameEngine.event.setActiveQueue(null);
  });

  it("drains a provided queue into a caller-supplied target array", () => {
    const queue = gameEngine.event.createQueue();
    const target: ReturnType<typeof gameEngine.event.createQueue> = [];
    const eventItem = new gameEngine.event.Event(7);

    gameEngine.event.post(eventItem, queue);

    const drained = gameEngine.event.get(queue, target);

    expect(drained).toBe(target);
    expect(drained).toEqual([eventItem]);
    expect(queue).toHaveLength(0);
  });

  it("uses the active queue when no queue is provided", () => {
    const queue = gameEngine.event.createQueue();
    const eventItem = new gameEngine.event.Event(11);

    gameEngine.event.setActiveQueue(queue);
    gameEngine.event.post(eventItem);

    expect(gameEngine.event.get()).toEqual([eventItem]);
    expect(queue).toHaveLength(0);
  });
});
