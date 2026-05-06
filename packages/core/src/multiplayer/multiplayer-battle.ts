/**
 * Multiplayer Battle Wrapper
 *
 * Orchestrates an existing Battle instance using a P2P transport.
 *
 * Important: this wrapper deliberately "pauses" the local Battle update loop
 * while waiting for the remote action, to prevent Battle's AI from selecting
 * an enemy action.
 */

import type { GameState } from '@pokecrystal/core/core/state';
import { BattleStateEnum, type BattleAction } from '@pokecrystal/core/engine/battle/battle/battle-context';
import type { Battle } from '@pokecrystal/core/engine/battle/battle/battle-logic';
import { BattleSynchronizer, type BattleSyncTransport } from './battle-synchronizer';

export type MultiplayerBattleOptions = {
  battle: Battle;
  transport: BattleSyncTransport;
  isHost: boolean;
  gameState: GameState;
};

export class MultiplayerBattle {
  public readonly battle: Battle;
  public readonly synchronizer: BattleSynchronizer;

  private currentTurn = 0;
  private awaitingRemoteTurn: number | null = null;

  private lastLocalActionJson: string | null = null;
  private remoteActionByTurn: Map<number, BattleAction> = new Map();
  private lastState: BattleStateEnum | null = null;

  constructor(options: MultiplayerBattleOptions) {
    this.battle = options.battle;
    this.synchronizer = new BattleSynchronizer({
      isHost: options.isHost,
      transport: options.transport,
      gameState: options.gameState,
    });

    this.synchronizer.onRemoteAction((turn, action) => {
      this.remoteActionByTurn.set(turn, action);
    });
  }

  async initRng(): Promise<void> {
    await this.synchronizer.initRng();
  }

  destroy(): void {
    this.synchronizer.destroy();
    this.remoteActionByTurn.clear();
  }

  /**
   * Frame update hook; call this instead of `battle.update()` when in multiplayer.
   */
  update(): void {
    const ctx = this.battle.context;
    const prevState = this.lastState ?? ctx.currentState;
    this.lastState = ctx.currentState;

    // If we're waiting for the remote action, do not advance Battle until we have it.
    if (ctx.currentState === BattleStateEnum.ENEMY_ACTION_SELECT) {
      if (!ctx.enemyAction) {
        const resolved = this.remoteActionByTurn.get(this.currentTurn);
        if (resolved) {
          ctx.enemyAction = resolved;
          this.remoteActionByTurn.delete(this.currentTurn);
        } else {
          // Pause the battle loop; remote action will arrive via transport.
          return;
        }
      }
    }

    this.battle.update();

    // After the battle step, if the player action is now chosen for this turn,
    // kick off the network exchange.
    this.maybeSendLocalAction();

    // Advance turn counter once the battle has consumed the enemy action selection.
    const nextState = this.battle.context.currentState;
    if (prevState === BattleStateEnum.ENEMY_ACTION_SELECT && nextState !== BattleStateEnum.ENEMY_ACTION_SELECT) {
      this.currentTurn += 1;
      this.lastLocalActionJson = null;
    }

    this.lastState = nextState;
  }

  private maybeSendLocalAction(): void {
    const ctx = this.battle.context;
    if (!ctx.playerAction) {
      return;
    }

    const localActionJson = JSON.stringify(ctx.playerAction);
    if (localActionJson === this.lastLocalActionJson) {
      return;
    }

    // New action for the current turn. Exchange and cache the remote result.
    this.lastLocalActionJson = localActionJson;
    const turn = this.currentTurn;

    // Guard: only one exchange in flight per turn.
    if (this.awaitingRemoteTurn === turn) {
      return;
    }
    this.awaitingRemoteTurn = turn;

    void this.synchronizer
      .exchangeTurnAction(turn, ctx.playerAction)
      .finally(() => {
        if (this.awaitingRemoteTurn === turn) {
          this.awaitingRemoteTurn = null;
        }
      });
  }
}
