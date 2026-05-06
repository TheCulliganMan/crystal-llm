import { BattleSynchronizer } from './battle-synchronizer';
import type { BattleSyncMessage, BattleSyncTransport } from './battle-synchronizer';
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

describe('BattleSynchronizer', () => {
  test('initRng applies host state to both peers', async () => {
    const [tHost, tClient] = makeTransportPair();
    const hostState = { hram: {} } as GameState;
    const clientState = { hram: {} } as GameState;

    const host = new BattleSynchronizer({
      isHost: true,
      transport: tHost,
      gameState: hostState,
    });
    const client = new BattleSynchronizer({
      isHost: false,
      transport: tClient,
      gameState: clientState,
    });

    const init = await Promise.all([host.initRng({ hardware_divider: 2, hRandomAdd: 3, hRandomSub: 4 }), client.initRng()]);

    expect(init[0]).toEqual({ hardware_divider: 2, hRandomAdd: 3, hRandomSub: 4 });
    expect(init[1]).toEqual({ hardware_divider: 2, hRandomAdd: 3, hRandomSub: 4 });

    expect(hostState.hram.hardware_divider).toBe(2);
    expect(clientState.hram.hRandomAdd).toBe(3);
    expect(clientState.hram.hRandomSub).toBe(4);

    host.destroy();
    client.destroy();
  });

  test('exchangeTurnAction swaps actions', async () => {
    const [tA, tB] = makeTransportPair();
    const aState = { hram: {} } as GameState;
    const bState = { hram: {} } as GameState;

    const a = new BattleSynchronizer({ isHost: true, transport: tA, gameState: aState });
    const b = new BattleSynchronizer({ isHost: false, transport: tB, gameState: bState });

    const aAction = { actionType: 'move', moveName: 'TACKLE' as any };
    const bAction = { actionType: 'run' };

    const [aRemote, bRemote] = await Promise.all([
      a.exchangeTurnAction(0, aAction),
      b.exchangeTurnAction(0, bAction),
    ]);

    expect(aRemote).toEqual(bAction);
    expect(bRemote).toEqual(aAction);

    a.destroy();
    b.destroy();
  });

  test('fnv1a32 is stable', () => {
    expect(BattleSynchronizer.fnv1a32('abc')).toBe(BattleSynchronizer.fnv1a32('abc'));
    expect(BattleSynchronizer.fnv1a32('abc')).not.toBe(BattleSynchronizer.fnv1a32('abcd'));
  });
});

