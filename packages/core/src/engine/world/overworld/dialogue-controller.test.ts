import { createInitialGameState } from "@pokecrystal/core/core/state";
import { Event, EventManager } from "@pokecrystal/core/engine/events/events";
import type { FieldDialogueManager } from "@pokecrystal/core/ui/text/dialogue";
import { DialogueEventController } from "./dialogue-controller";

describe("DialogueEventController", () => {
  it("routes generic selection prompts to the field dialogue manager", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const handleEvent = jest.fn();
    const dialogue = {
      handle_event: handleEvent,
      suspend: jest.fn(),
      resume: jest.fn(),
    } as unknown as FieldDialogueManager;

    const controller = new DialogueEventController(eventManager, dialogue);
    controller.register();

    const event = new Event("prompt_selection", {
      options: ["1F", "2F"],
      callback: jest.fn(),
    });
    eventManager.dispatch(event);

    expect(handleEvent).toHaveBeenCalledWith(event, gameState);
    expect(eventManager.hasListener("prompt_selection")).toBe(true);
  });
});
