import type { GameState } from "@pokecrystal/core/core/state";
import type { Event } from "@pokecrystal/core/engine/world/events";
import type { EventManager } from "@pokecrystal/core/engine/world/events";
import type { FieldDialogueManager } from "@pokecrystal/core/ui/text/dialogue";

type Listener = (event: Event, gameState: GameState) => void;

type LegacyDialogueEventManager = EventManager & {
  _listeners?: Record<string, Listener[]>;
};

const _is_legacy_dialogue_event_manager = (
  manager: EventManager,
): manager is LegacyDialogueEventManager => "_listeners" in manager;

const getLegacyEventManagerListeners = (
  manager: EventManager | null
): Record<string, Listener[]> | undefined => {
  if (!manager || !_is_legacy_dialogue_event_manager(manager)) {
    return undefined;
  }
  return manager._listeners;
};

export class DialogueEventController {
  private readonly eventManager: EventManager | null;
  private readonly dialogue: FieldDialogueManager | null;
  private active = false;
  private bindings: Array<[string, Listener]> = [];

  constructor(
    eventManager: EventManager | null,
    dialogue: FieldDialogueManager | null
  ) {
    this.eventManager = eventManager;
    this.dialogue = dialogue;
    if (dialogue) {
      const bind = (eventName: string): [string, Listener] => {
        const handler = dialogue.handle_event.bind(dialogue) as Listener & {
          __dialogueOwner?: FieldDialogueManager;
        };
        handler.__dialogueOwner = dialogue;
        return [eventName, handler];
      };
      this.bindings = [
        bind("show_text"),
        bind("open_text"),
        bind("close_text"),
        bind("wait_for_input"),
        bind("prompt_yes_no"),
        bind("prompt_selection"),
      ];
    }
  }

  register(): void {
    if (!this.eventManager || !this.dialogue || this.bindings.length === 0) {
      return;
    }
    this.removeExistingDialogueHandlers();
    for (const [eventName, handler] of this.bindings) {
      this.eventManager.on(eventName, handler);
    }
    this.active = true;
  }

  suspend(): void {
    if (this.active && this.eventManager) {
      for (const [eventName, handler] of this.bindings) {
        this.eventManager.off(eventName, handler);
      }
    }
    this.active = false;
    this.closeDialogue();
  }

  resume(): void {
    if (!this.eventManager || !this.dialogue || this.bindings.length === 0) {
      return;
    }
    if (!this.active) {
      for (const [eventName, handler] of this.bindings) {
        this.eventManager.on(eventName, handler);
      }
      this.active = true;
    }
    this.openDialogueChannel();
  }

  private closeDialogue(): void {
    if (this.dialogue) {
      this.dialogue.suspend();
    }
  }

  private openDialogueChannel(): void {
    if (this.dialogue) {
      this.dialogue.resume();
    }
  }

  private removeExistingDialogueHandlers(): void {
    if (!this.eventManager || this.bindings.length === 0) {
      return;
    }
    const relevant = new Set(this.bindings.map(([eventName]) => eventName));
    const listeners = getLegacyEventManagerListeners(this.eventManager);
    if (!listeners) {
      return;
    }
    for (const eventName of relevant) {
      const handlers = listeners[eventName] ?? [];
      for (const handler of handlers) {
        const owner = (handler as { __dialogueOwner?: FieldDialogueManager })
          .__dialogueOwner;
        if (owner) {
          this.eventManager.off(eventName, handler);
        }
      }
    }
  }
}
