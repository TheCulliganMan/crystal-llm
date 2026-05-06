import { createInitialGameState } from "@pokecrystal/core/core/state";
import { Event, EventManager, closeText, openText, showText, waitForInput } from "./events";

describe("Event", () => {
  it("stores the event name and data", () => {
    const event = new Event("show_text", { text: "Hello" });

    expect(event.name).toBe("show_text");
    expect(event.type).toBe("show_text");
    expect(event.data).toEqual({ text: "Hello" });
  });
});

describe("EventManager", () => {
  it("registers and dispatches events to listeners", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const listener = jest.fn();

    eventManager.on("test_event", listener);
    const event = new Event("test_event", { payload: "test_data" });
    eventManager.dispatch(event);
    eventManager.process_pending_events();

    expect(listener).toHaveBeenCalledTimes(1);
    expect(listener).toHaveBeenCalledWith(event, gameState);
  });

  it("tracks pending queue entries and drains them on advance_frame", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const listener = jest.fn();

    eventManager.on("test_event", listener);
    eventManager.dispatch(new Event("test_event", {}), { delay: 1 });

    expect(eventManager.has_pending_events).toBe(true);
    eventManager.advance_frame();
    expect(listener).toHaveBeenCalledTimes(1);
    expect(eventManager.has_pending_events).toBe(false);
  });

  it("respects delayed delivery timing", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const listener = jest.fn();

    eventManager.on("delayed", listener);
    eventManager.dispatch(new Event("delayed", {}), { delay: 2 });

    eventManager.advance_frame();
    expect(listener).not.toHaveBeenCalled();
    eventManager.advance_frame();
    expect(listener).toHaveBeenCalledTimes(1);
  });
});

describe("Event helpers", () => {
  it("dispatches open_text/close_text/show_text/wait_for_input events", () => {
    const dispatch = jest.fn();
    const eventManager = { dispatch } as unknown as EventManager;

    openText(eventManager);
    closeText(eventManager);
    showText(eventManager, "Test text");
    waitForInput(eventManager, { pauseRunner: true });

    expect(dispatch).toHaveBeenCalledTimes(4);
    const [openEvent, closeEvent, showEvent, waitEvent] = dispatch.mock.calls.map((call) => call[0]);
    expect(openEvent.name).toBe("open_text");
    expect(closeEvent.name).toBe("close_text");
    expect(showEvent.name).toBe("show_text");
    expect(showEvent.data).toEqual({ text: "Test text" });
    expect(waitEvent.name).toBe("wait_for_input");
    expect(waitEvent.data).toEqual({ pauseRunner: true });
  });
});
