import {
  resetMultiplayerClientFactory,
  setMultiplayerClientFactory,
} from '@pokecrystal/core/adapters/multiplayer-client';
import { extractRemotePlayersFromPresence, OverworldPresenceManager } from './overworld-presence';

describe('extractRemotePlayersFromPresence', () => {
  test('filters invalid entries, excludes local user, and keeps newest update per user', () => {
    const players = extractRemotePlayersFromPresence(
      {
        alpha: [
          {
            userId: 'me',
            playerName: 'Local',
            entityType: 'player',
            mapName: 'NewBarkTown',
            tileX: 8,
            tileY: 10,
            direction: 'down',
            updatedAtMs: 50,
          },
          {
            userId: 'u1',
            playerName: 'Alice',
            entityType: 'player',
            mapName: 'NewBarkTown',
            tileX: 7,
            tileY: 10,
            direction: 'left',
            updatedAtMs: 10,
          },
          {
            userId: 'u1',
            playerName: 'Alice',
            entityType: 'player',
            mapName: 'NewBarkTown',
            tileX: 9,
            tileY: 12,
            direction: 'right',
            updatedAtMs: 20,
          },
        ],
        beta: [
          {
            userId: 'u2',
            playerName: 'Bob',
            entityType: 'ai',
            mapName: 'Route29',
            tileX: 30,
            tileY: 9,
            direction: 'up',
            updatedAtMs: 15,
          },
          {
            userId: '',
            playerName: 'bad',
            mapName: 'Route29',
            tileX: 1,
            tileY: 1,
            direction: 'up',
            updatedAtMs: 1,
          },
        ],
      },
      'me',
      { nowMs: 100, staleMs: 1_000 }
    );

    expect(players).toHaveLength(2);
    expect(players.find((p) => p.userId === 'u1')).toEqual({
      userId: 'u1',
      playerName: 'Alice',
      entityType: 'player',
      mapName: 'NewBarkTown',
      tileX: 9,
      tileY: 12,
      direction: 'right',
      updatedAtMs: 20,
    });
    expect(players.find((p) => p.userId === 'u2')?.entityType).toBe('ai');
  });

  test('filters stale remote presence entries', () => {
    const players = extractRemotePlayersFromPresence(
      {
        remote: [
          {
            userId: 'u1',
            playerName: 'Alice',
            entityType: 'player',
            mapName: 'NewBarkTown',
            tileX: 7,
            tileY: 10,
            direction: 'left',
            updatedAtMs: 10,
          },
        ],
      },
      'me',
      { nowMs: 30_000, staleMs: 1_000 }
    );

    expect(players).toEqual([]);
  });
});

describe('OverworldPresenceManager', () => {
  afterEach(() => {
    resetMultiplayerClientFactory();
  });

  test('connects, tracks local state, emits remote players, and disconnects', async () => {
    let syncHandler: (() => void) | null = null;
    const presenceState = jest.fn(() => ({
      remote: [
        {
          userId: 'u2',
          playerName: 'Bob',
          entityType: 'player',
          mapName: 'NewBarkTown',
          tileX: 10,
          tileY: 9,
          direction: 'left',
          updatedAtMs: Date.now(),
        },
      ],
    }));

    const channel = {
      send: jest.fn(async () => ({ status: 'ok' })),
      on: jest.fn((_kind: string, filter: { event: string }, cb: () => void) => {
        if (filter.event === 'sync') {
          syncHandler = cb;
        }
        return channel;
      }),
      subscribe: jest.fn((cb?: (status: string) => void) => {
        cb?.('SUBSCRIBED');
        return channel;
      }),
      track: jest.fn(async () => ({ status: 'ok' })),
      presenceState,
    };

    const supabase = {
      auth: { getUser: jest.fn(async () => ({ data: { user: { id: 'me' } } })) },
      channel: jest.fn(() => channel),
      removeChannel: jest.fn(async () => ({ data: null })),
    };

    setMultiplayerClientFactory(() => supabase as any);

    const manager = new OverworldPresenceManager();
    const events: unknown[] = [];
    manager.onRemotePlayersChange((players) => events.push(players));

    await manager.connect({
      playerName: 'Local',
      mapName: 'NewBarkTown',
      tileX: 8,
      tileY: 10,
      direction: 'down',
    });

    expect(supabase.channel).toHaveBeenCalledWith('overworld:presence', {
      config: { broadcast: { ack: true }, presence: { key: 'me' } },
    });
    expect(channel.track).toHaveBeenCalledTimes(1);

    syncHandler?.();
    expect(events.length).toBeGreaterThanOrEqual(1);
    expect((events.at(-1) as any[])[0]).toMatchObject({ userId: 'u2', playerName: 'Bob' });

    await manager.updateLocalState({
      playerName: 'Local',
      mapName: 'Route29',
      tileX: 30,
      tileY: 8,
      direction: 'right',
    });

    expect(channel.track).toHaveBeenCalledTimes(2);
    await manager.disconnect();
    expect(supabase.removeChannel).toHaveBeenCalledTimes(1);
    expect(events.at(-1)).toEqual([]);
  });

  test('sends and receives interaction requests/responses', async () => {
    let requestHandler: ((args: { payload: unknown }) => void) | null = null;
    let responseHandler: ((args: { payload: unknown }) => void) | null = null;
    const channel = {
      on: jest.fn((_kind: string, filter: { event: string }, cb: (args: { payload: unknown }) => void) => {
        if (filter.event === 'interaction:request') {
          requestHandler = cb;
        }
        if (filter.event === 'interaction:response') {
          responseHandler = cb;
        }
        return channel;
      }),
      subscribe: jest.fn((cb?: (status: string) => void) => {
        cb?.('SUBSCRIBED');
        return channel;
      }),
      track: jest.fn(async () => ({ status: 'ok' })),
      presenceState: jest.fn(() => ({})),
      send: jest.fn(async () => ({ status: 'ok' })),
    };

    const supabase = {
      auth: { getUser: jest.fn(async () => ({ data: { user: { id: 'me' } } })) },
      channel: jest.fn(() => channel),
      removeChannel: jest.fn(async () => ({ data: null })),
    };
    setMultiplayerClientFactory(() => supabase as any);

    const manager = new OverworldPresenceManager();
    const requests: unknown[] = [];
    const responses: unknown[] = [];
    manager.onInteractionRequest((request) => requests.push(request));
    manager.onInteractionResponse((response) => responses.push(response));

    await manager.connect({
      playerName: 'Local',
      mapName: 'NewBarkTown',
      tileX: 8,
      tileY: 10,
      direction: 'down',
    });

    const requestId = await manager.sendInteractionRequest('u2', 'battle');
    expect(requestId.length).toBeGreaterThan(4);
    expect(channel.send).toHaveBeenCalledWith(
      expect.objectContaining({
        type: 'broadcast',
        event: 'interaction:request',
      })
    );

    requestHandler?.({
      payload: {
        requestId: 'r1',
        fromUserId: 'u2',
        fromPlayerName: 'Opponent',
        toUserId: 'me',
        kind: 'trade',
        timestampMs: 10,
      },
    });
    expect(requests).toHaveLength(1);

    await manager.sendInteractionResponse(requests[0] as any, true);
    expect(channel.send).toHaveBeenCalledWith(
      expect.objectContaining({
        type: 'broadcast',
        event: 'interaction:response',
      })
    );

    responseHandler?.({
      payload: {
        requestId: 'r1',
        fromUserId: 'u2',
        toUserId: 'me',
        kind: 'trade',
        accepted: false,
        timestampMs: 12,
      },
    });
    expect(responses).toHaveLength(1);
  });
});
