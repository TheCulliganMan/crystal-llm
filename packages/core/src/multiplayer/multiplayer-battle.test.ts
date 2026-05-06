import { MultiplayerBattle } from './multiplayer-battle';
import type { BattleSyncMessage, BattleSyncTransport } from './battle-synchronizer';
import { BattleStateEnum } from '@pokecrystal/core/engine/battle/battle/battle-context';
import type { GameState } from '@pokecrystal/core/core/state';

function makeTransportPair(): [BattleSyncTransport, BattleSyncTransport] {
  const aCbs: Array<(m: BattleSyncMessage) => void> = [];
  const bCbs: Array<(m: BattleSyncMessage) => void> = [];

  const a: BattleSyncTransport = {
    send(message) {
      bCbs.forEach((cb) => cb(message));
    },
    onData(cb) {
      aCbs.push(cb);
    },
    offData(cb) {
      const idx = aCbs.indexOf(cb);
      if (idx !== -1) aCbs.splice(idx, 1);
    },
  };

  const b: BattleSyncTransport = {
    send(message) {
      aCbs.forEach((cb) => cb(message));
    },
    onData(cb) {
      bCbs.push(cb);
    },
    offData(cb) {
      const idx = bCbs.indexOf(cb);
      if (idx !== -1) bCbs.splice(idx, 1);
    },
  };

  return [a, b];
}

function makeFakeBattle() {
  const ctx: any = {
    currentState: BattleStateEnum.PLAYER_ACTION_SELECT,
    playerAction: undefined,
    enemyAction: undefined,
  };

  const battle: any = {
    context: ctx,
    update: () => {
      if (ctx.currentState === BattleStateEnum.PLAYER_ACTION_SELECT) {
        if (ctx.playerAction) {
          ctx.currentState = BattleStateEnum.ENEMY_ACTION_SELECT;
          return;
        }
        return;
      }

      if (ctx.currentState === BattleStateEnum.ENEMY_ACTION_SELECT) {
        if (ctx.enemyAction) {
          ctx.currentState = BattleStateEnum.PRE_TURN_EFFECTS;
          return;
        }
        // In real Battle this would invoke AI. Our wrapper must prevent calling update here.
        throw new Error('AI was invoked');
      }

      if (ctx.currentState === BattleStateEnum.PRE_TURN_EFFECTS) {
        ctx.playerAction = undefined;
        ctx.enemyAction = undefined;
        ctx.currentState = BattleStateEnum.PLAYER_ACTION_SELECT;
      }
    },
  };

  return battle;
}

describe('MultiplayerBattle', () => {
  test('pauses in ENEMY_ACTION_SELECT until remote action arrives', async () => {
    const [tA, tB] = makeTransportPair();
    const aState = { hram: {} } as GameState;
    const bState = { hram: {} } as GameState;

    const battleA = makeFakeBattle();
    const battleB = makeFakeBattle();

    const a = new MultiplayerBattle({
      battle: battleA as any,
      transport: tA,
      isHost: true,
      gameState: aState,
    });
    const b = new MultiplayerBattle({
      battle: battleB as any,
      transport: tB,
      isHost: false,
      gameState: bState,
    });

    battleA.context.playerAction = { actionType: 'move', moveName: 'TACKLE' as any };
    battleB.context.playerAction = { actionType: 'run' };

    // First update: each battle moves into ENEMY_ACTION_SELECT and sends action.
    a.update();
    b.update();
    expect(battleA.context.currentState).toBe(BattleStateEnum.ENEMY_ACTION_SELECT);
    expect(battleB.context.currentState).toBe(BattleStateEnum.ENEMY_ACTION_SELECT);

    // Next update: remote actions should be available; both can advance.
    a.update();
    b.update();

    expect(battleA.context.currentState).toBe(BattleStateEnum.PRE_TURN_EFFECTS);
    expect(battleB.context.currentState).toBe(BattleStateEnum.PRE_TURN_EFFECTS);

    a.destroy();
    b.destroy();
  });
});

