/**
 * Battle Synchronizer
 *
 * Provides deterministic sync primitives for link battles:
 * - Host-to-client RNG state initialization (HardwareRNG via GameState HRAM)
 * - Turn action exchange (local -> remote, remote -> local)
 * - Optional state hash exchange to detect desync
 *
 * This module is intentionally strict: it throws on invalid payloads so that
 * tests/telemetry catch protocol drift early.
 */

import type { GameState } from '@pokecrystal/core/core/state';
import type { BattleAction } from '@pokecrystal/core/engine/battle/battle/battle-context';

export type BattleSyncMessage =
  | {
      type: 'battle:rng_init';
      data: BattleRngState;
    }
  | {
      type: 'battle:action';
      data: {
        turn: number;
        action: BattleAction;
        stateHash?: string;
      };
    }
  | {
      type: 'battle:state_hash';
      data: {
        turn: number;
        hash: string;
      };
    };

export interface BattleSyncTransport {
  send(message: BattleSyncMessage): void;
  onData(callback: (message: BattleSyncMessage) => void): void;
  offData?(callback: (message: BattleSyncMessage) => void): void;
}

export type BattleRngState = {
  hardware_divider: number; // 16-bit
  hRandomAdd: number; // 8-bit
  hRandomSub: number; // 8-bit
};

export type BattleSynchronizerOptions = {
  isHost: boolean;
  transport: BattleSyncTransport;
  gameState: GameState;
};

export class BattleSynchronizer {
  private readonly isHost: boolean;
  private readonly transport: BattleSyncTransport;
  private readonly gameState: GameState;

  private receivedRngInit: BattleRngState | null = null;

  private readonly pendingActions: Map<number, BattleAction> = new Map();
  private readonly pendingResolvers: Map<number, (action: BattleAction) => void> =
    new Map();

  private readonly actionListeners: Array<(turn: number, action: BattleAction) => void> = [];

  private readonly onMessageBound: (message: BattleSyncMessage) => void;

  constructor(options: BattleSynchronizerOptions) {
    this.isHost = options.isHost;
    this.transport = options.transport;
    this.gameState = options.gameState;

    this.onMessageBound = (message) => this.onMessage(message);
    this.transport.onData(this.onMessageBound);
  }

  destroy(): void {
    if (this.transport.offData) {
      this.transport.offData(this.onMessageBound);
    }
    this.pendingActions.clear();
    this.pendingResolvers.clear();
    this.actionListeners.length = 0;
  }

  /**
   * Initialize HardwareRNG state in both peers.
   *
   * Host chooses a state (either provided, or generated) and sends to client.
   * Client waits for and applies the host-provided state.
   */
  async initRng(state?: BattleRngState): Promise<BattleRngState> {
    if (this.isHost) {
      const resolved = state ?? BattleSynchronizer.generateRngState();
      BattleSynchronizer.applyRngState(this.gameState, resolved);
      this.transport.send({ type: 'battle:rng_init', data: resolved });
      return resolved;
    }

    if (this.receivedRngInit) {
      const cached = this.receivedRngInit;
      this.receivedRngInit = null;
      BattleSynchronizer.applyRngState(this.gameState, cached);
      return cached;
    }

    return await new Promise<BattleRngState>((resolve) => {
      const waiter = (message: BattleSyncMessage) => {
        if (message.type !== 'battle:rng_init') return;
        BattleSynchronizer.assertRngState(message.data);
        BattleSynchronizer.applyRngState(this.gameState, message.data);
        if (this.transport.offData) {
          this.transport.offData(waiter);
          this.transport.onData(this.onMessageBound);
        }
        resolve(message.data);
      };

      // Temporarily intercept messages until we get rng_init.
      if (this.transport.offData) {
        this.transport.offData(this.onMessageBound);
      }
      this.transport.onData(waiter);
    });
  }

  onRemoteAction(callback: (turn: number, action: BattleAction) => void): void {
    this.actionListeners.push(callback);
  }

  offRemoteAction(callback: (turn: number, action: BattleAction) => void): void {
    const idx = this.actionListeners.indexOf(callback);
    if (idx !== -1) {
      this.actionListeners.splice(idx, 1);
    }
  }

  /**
   * Send local action for a given turn and wait for the remote action.
   * Both sides call this once per turn with the same turn number.
   */
  async exchangeTurnAction(
    turn: number,
    localAction: BattleAction,
    options?: { localStateHash?: string },
  ): Promise<BattleAction> {
    BattleSynchronizer.assertTurn(turn);
    BattleSynchronizer.assertAction(localAction);

    this.transport.send({
      type: 'battle:action',
      data: {
        turn,
        action: localAction,
        stateHash: options?.localStateHash,
      },
    });

    const existing = this.pendingActions.get(turn);
    if (existing) {
      this.pendingActions.delete(turn);
      return existing;
    }

    return await new Promise<BattleAction>((resolve) => {
      this.pendingResolvers.set(turn, resolve);
    });
  }

  /**
   * Drain already-received remote action for the given turn, if present.
   */
  tryConsumeRemoteAction(turn: number): BattleAction | null {
    const action = this.pendingActions.get(turn);
    if (!action) return null;
    this.pendingActions.delete(turn);
    return action;
  }

  sendStateHash(turn: number, hash: string): void {
    BattleSynchronizer.assertTurn(turn);
    if (!hash) {
      throw new Error('state hash must be non-empty');
    }
    this.transport.send({ type: 'battle:state_hash', data: { turn, hash } });
  }

  // ---------------------------------------------------------------------------
  // Deterministic hashing helpers
  // ---------------------------------------------------------------------------

  static fnv1a32(input: string): string {
    let hash = 0x811c9dc5;
    for (let i = 0; i < input.length; i++) {
      hash ^= input.charCodeAt(i);
      hash = Math.imul(hash, 0x01000193);
      hash >>>= 0;
    }
    return hash.toString(16).padStart(8, '0');
  }

  // ---------------------------------------------------------------------------
  // RNG helpers
  // ---------------------------------------------------------------------------

  static generateRngState(seed: number = Date.now() >>> 0): BattleRngState {
    // Keep it simple and fully explicit; the HardwareRNG will take over from this state.
    // Ensure divider is non-zero to avoid falling back to the hardcoded 0xACE1.
    const divider = (seed ^ 0xa5a5) & 0xffff;
    return {
      hardware_divider: divider === 0 ? 1 : divider,
      hRandomAdd: (seed >>> 8) & 0xff,
      hRandomSub: seed & 0xff,
    };
  }

  static applyRngState(gameState: GameState, state: BattleRngState): void {
    BattleSynchronizer.assertRngState(state);
    if (!gameState.hram) {
      throw new Error('GameState.hram is required for HardwareRNG');
    }
    gameState.hram.hardware_divider = state.hardware_divider & 0xffff;
    gameState.hram.hRandomAdd = state.hRandomAdd & 0xff;
    gameState.hram.hRandomSub = state.hRandomSub & 0xff;
  }

  static assertRngState(state: BattleRngState): void {
    if (
      !state ||
      typeof state.hardware_divider !== 'number' ||
      typeof state.hRandomAdd !== 'number' ||
      typeof state.hRandomSub !== 'number'
    ) {
      throw new Error('Invalid rng state payload');
    }
    if (state.hardware_divider < 0 || state.hardware_divider > 0xffff) {
      throw new Error('hardware_divider must be a 16-bit value');
    }
    if (state.hRandomAdd < 0 || state.hRandomAdd > 0xff) {
      throw new Error('hRandomAdd must be an 8-bit value');
    }
    if (state.hRandomSub < 0 || state.hRandomSub > 0xff) {
      throw new Error('hRandomSub must be an 8-bit value');
    }
  }

  private onMessage(message: BattleSyncMessage): void {
    if (!message || typeof message !== 'object' || typeof message.type !== 'string') {
      throw new Error('Invalid battle sync message');
    }
    switch (message.type) {
      case 'battle:rng_init': {
        // Client should apply via initRng(); but we also cache in case it arrives early.
        BattleSynchronizer.assertRngState(message.data);
        if (!this.isHost) {
          this.receivedRngInit = message.data;
        }
        return;
      }
      case 'battle:action': {
        const { turn, action } = message.data ?? {};
        BattleSynchronizer.assertTurn(turn);
        BattleSynchronizer.assertAction(action);

        for (const listener of this.actionListeners) {
          listener(turn, action);
        }

        const resolver = this.pendingResolvers.get(turn);
        if (resolver) {
          this.pendingResolvers.delete(turn);
          resolver(action);
          return;
        }
        this.pendingActions.set(turn, action);
        return;
      }
      case 'battle:state_hash': {
        const { turn, hash } = message.data ?? {};
        BattleSynchronizer.assertTurn(turn);
        if (!hash || typeof hash !== 'string') {
          throw new Error('Invalid state hash payload');
        }
        return;
      }
      default: {
        const unreachable: never = message;
        throw new Error(`Unhandled battle sync message: ${(unreachable as { type: string }).type}`);
      }
    }
  }

  static assertTurn(turn: number): void {
    if (!Number.isInteger(turn) || turn < 0) {
      throw new Error(`Invalid turn: ${turn}`);
    }
  }

  static assertAction(action: unknown): void {
    if (!action || typeof action !== 'object') {
      throw new Error('Invalid action payload');
    }
    const actionRecord = action as { actionType?: unknown };

    if (
      actionRecord.actionType !== 'move' &&
      actionRecord.actionType !== 'switch' &&
      actionRecord.actionType !== 'item' &&
      actionRecord.actionType !== 'run'
    ) {
      throw new Error(`Invalid actionType: ${String(actionRecord.actionType)}`);
    }
    if (actionRecord.actionType === 'move' && !(action as { moveName?: unknown }).moveName) {
      throw new Error('Move action requires moveName');
    }
    if (
      actionRecord.actionType === 'switch' &&
      (action as { switchToPokemonIndex?: unknown }).switchToPokemonIndex === undefined
    ) {
      throw new Error('Switch action requires switchToPokemonIndex');
    }
    if (actionRecord.actionType === 'item' && !(action as { item?: unknown }).item) {
      throw new Error('Item action requires item');
    }
  }
}
