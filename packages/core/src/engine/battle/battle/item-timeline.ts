import { BattleTurn } from '../../../core/enums';
import type { GameState } from '@pokecrystal/core/core/state';
import type { Item, Pokemon } from '../../../core/models';
import { Event } from '../../events/events';
import type { EventManager } from '../../events/events';

const EVENT_APPLY_ITEM = 'apply_battle_item';
export const ITEM_EFFECT_DELAY_FRAMES = 12;

export type QueuedBattleItem = {
  attackerSide: BattleTurn;
  item: Item;
  target: Pokemon;
  moveIndex: number | null;
};

// ASM: engine/battle/core.asm::DoItemEffect
export class BattleItemTimeline {
  private eventManager: EventManager | null;
  private delayFrames: number;
  private pending: QueuedBattleItem | null = null;
  private applyCallback: ((queued: QueuedBattleItem) => boolean) | null = null;
  private onComplete: ((queued: QueuedBattleItem, result: boolean) => void) | null = null;

  constructor(eventManager: EventManager | null, delayFrames: number = ITEM_EFFECT_DELAY_FRAMES) {
    this.eventManager = eventManager;
    this.delayFrames = Math.max(0, delayFrames);

    if (this.eventManager) {
      this.eventManager.on(EVENT_APPLY_ITEM, this.applyFromEvent.bind(this));
    }
  }

  get applying(): boolean {
    return this.pending !== null;
  }

  queue(
    queued: QueuedBattleItem,
    applyCallback: (queued: QueuedBattleItem) => boolean,
    onComplete: (queued: QueuedBattleItem, result: boolean) => void,
  ): void {
    if (this.pending !== null) {
      throw new Error('An item effect is already queued.');
    }

    this.pending = queued;
    this.applyCallback = applyCallback;
    this.onComplete = onComplete;

    const dispatch = this.eventManager?.dispatch;
    if (typeof dispatch === 'function') {
      const event = new Event(EVENT_APPLY_ITEM, { queued });
      // Drive delays through EventManager frames so timing stays on the GB tick cadence.
      dispatch.call(this.eventManager, event, { delay: this.delayFrames });
      return;
    }

    this.resolveImmediately();
  }

  private applyFromEvent(event: Event, _state: GameState): void {
    if (event.name !== EVENT_APPLY_ITEM) {
      return;
    }
    this.resolveImmediately();
  }

  private resolveImmediately(): void {
    const queued = this.pending;
    const applyCallback = this.applyCallback;
    const onComplete = this.onComplete;
    this.pending = null;
    this.applyCallback = null;
    this.onComplete = null;

    if (!queued || !applyCallback || !onComplete) {
      return;
    }

    const result = applyCallback(queued);
    onComplete(queued, result);
  }
}
