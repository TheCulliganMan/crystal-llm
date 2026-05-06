import { GameState } from "@pokecrystal/core/core/state";
import type { Pokemon, Trainer } from "@pokecrystal/core/core/models";
import type { DudeAutoInputController } from "@pokecrystal/core/engine/battle/auto-input";

export type EventPayload = Record<string, unknown>;

export interface StartBattleEventPayload extends EventPayload {
  player_pokemon: Pokemon;
  enemy_pokemon: Pokemon;
  player_party: Pokemon[];
  enemy_party: Pokemon[];
  trainer?: Trainer | null;
  trainer_id?: string | null;
  trainer_reward?: number;
  auto_input?: DudeAutoInputController | null;
  playerPokemon?: Pokemon;
  enemyPokemon?: Pokemon;
  playerParty?: Pokemon[];
  enemyParty?: Pokemon[];
  trainerId?: string | null;
  trainerReward?: number;
  autoInput?: DudeAutoInputController | null;
}

export class Event<T extends EventPayload = EventPayload> {
  constructor(public readonly name: string, public readonly data: T) {}

  get type(): string {
    return this.name;
  }
}

type EventListener<T extends EventPayload = EventPayload> = (
  event: Event<T>,
  gameState: GameState
) => void;

type QueuedEvent = {
  deliverFrame: number;
  priority: number;
  sequence: number;
  event: Event;
};

export class StartBattleEvent extends Event<StartBattleEventPayload> {
  constructor(data: StartBattleEventPayload) {
    super("start_battle", data);
  }
}

export class StartCreditsEvent extends Event<{
  allow_skip: boolean;
  on_complete: (() => void) | null;
  return_state: string | null;
}> {
  constructor(options: { allow_skip?: boolean; on_complete?: (() => void) | null; return_state?: string | null } = {}) {
    super("start_credits", {
      allow_skip: Boolean(options.allow_skip),
      on_complete: options.on_complete ?? null,
      return_state: options.return_state ?? null,
    });
  }
}

export class EventManager {
  public listeners: Map<string, Set<EventListener>> = new Map();
  public _listeners: Record<string, EventListener[]> = {};
  public gameState: GameState;
  public currentFrame = 0;
  private _pendingEvents: QueuedEvent[] = [];
  private _sequence = 0;

  constructor(gameState: GameState) {
    this.gameState = gameState;
  }

  get game_state(): GameState {
    return this.gameState;
  }

  get _current_frame(): number {
    return this.currentFrame;
  }

  get _pending_events(): QueuedEvent[] {
    return this._pendingEvents;
  }

  on<T extends EventPayload = EventPayload>(eventName: string, listener: EventListener<T>): void {
    if (!this.listeners.has(eventName)) {
      this.listeners.set(eventName, new Set());
    }
    if (!this._listeners[eventName]) {
      this._listeners[eventName] = [];
    }
    const set = this.listeners.get(eventName)!;
    set.add(listener as EventListener);
    if (!this._listeners[eventName].includes(listener as EventListener)) {
      this._listeners[eventName].push(listener as EventListener);
    }
  }

  off<T extends EventPayload = EventPayload>(eventName: string, listener: EventListener<T>): void {
    const set = this.listeners.get(eventName);
    if (set) {
      set.delete(listener as EventListener);
      if (set.size === 0) {
        this.listeners.delete(eventName);
      }
    }
    const arr = this._listeners[eventName];
    if (arr) {
      const idx = arr.indexOf(listener as EventListener);
      if (idx >= 0) {
        arr.splice(idx, 1);
      }
      if (arr.length === 0) {
        delete this._listeners[eventName];
      }
    }
  }

  dispatch<T extends EventPayload = EventPayload>(
    event: Event<T>,
    { priority = 0, delay = 0 }: { priority?: number; delay?: number } = {}
  ): void {
    const queued: QueuedEvent = {
      deliverFrame: this.currentFrame + Math.max(0, delay),
      priority,
      sequence: this._sequence++,
      event,
    };
    if (queued.deliverFrame <= this.currentFrame && priority === 0 && this._pendingEvents.length === 0) {
      this.emit(event);
      return;
    }
    this._pendingEvents.push(queued);
    this._pendingEvents.sort((a, b) => {
      if (a.deliverFrame !== b.deliverFrame) return a.deliverFrame - b.deliverFrame;
      if (a.priority !== b.priority) return a.priority - b.priority;
      return a.sequence - b.sequence;
    });
    this.processPendingEvents();
  }

  private emit<T extends EventPayload = EventPayload>(event: Event<T>): void {
    const listeners = [...(this._listeners[event.name] ?? [])];
    for (const listener of listeners) {
      try {
        listener(event, this.gameState);
      } catch (error) {
        console.error(`Error in event listener for ${event.name}:`, error);
      }
    }
  }

  processPendingEvents(): void {
    while (this._pendingEvents.length > 0 && this._pendingEvents[0].deliverFrame <= this.currentFrame) {
      const queued = this._pendingEvents.shift()!;
      this.emit(queued.event);
    }
  }

  process_pending_events(): void {
    this.processPendingEvents();
  }

  advanceFrame(): void {
    this.currentFrame += 1;
    this.processPendingEvents();
  }

  advance_frame(): void {
    this.advanceFrame();
  }

  get hasPendingEvents(): boolean {
    return this._pendingEvents.length > 0;
  }

  get has_pending_events(): boolean {
    return this.hasPendingEvents;
  }

  hasListener(eventName: string): boolean {
    const listeners = this.listeners.get(eventName);
    return Boolean(listeners && listeners.size);
  }
}

export const openText = (eventManager: EventManager): void => {
  eventManager.dispatch(new Event("open_text", {}));
};

export const closeText = (eventManager: EventManager): void => {
  eventManager.dispatch(new Event("close_text", {}));
};

export const showText = (
  eventManager: EventManager,
  text: string,
  data: Record<string, unknown> = {}
): void => {
  eventManager.dispatch(new Event("show_text", { ...data, text }));
};

export const waitForInput = (eventManager: EventManager, data: Record<string, unknown> = {}): void => {
  eventManager.dispatch(new Event("wait_for_input", data));
};
