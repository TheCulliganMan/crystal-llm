import {
  resetMultiplayerClientFactory,
  setMultiplayerClientFactory,
} from '@pokecrystal/core/adapters/multiplayer-client';
import { useMultiplayerStore } from './multiplayer-store';
import { MatchmakingService } from './matchmaking-service';

describe('MatchmakingService', () => {
  beforeEach(() => {
    useMultiplayerStore.getState().reset();
  });

  afterEach(() => {
    resetMultiplayerClientFactory();
  });

  test('joinQueue inserts queue entry and subscribes for matches', async () => {
    const matchInsertHandlers: Array<(payload: any) => void> = [];

    const channel = {
      on: jest.fn((_type: string, _filter: any, cb: any) => {
        matchInsertHandlers.push(cb);
        return channel;
      }),
      subscribe: jest.fn((cb?: (status: string) => void) => {
        cb?.('SUBSCRIBED');
        return channel;
      }),
    };

    const queueUpsert = jest.fn(async () => ({ error: null }));
    const supabase = {
      auth: { getUser: jest.fn(async () => ({ data: { user: { id: 'me' } } })) },
      from: jest.fn((table: string) => {
        if (table === 'matchmaking_queue') {
          return {
            upsert: queueUpsert,
            delete: jest.fn(() => ({
              eq: jest.fn(async () => ({ error: null })),
            })),
            select: jest.fn(() => ({
              eq: jest.fn(() => ({
                eq: jest.fn(() => ({
                  single: jest.fn(async () => ({
                    data: { created_at: '2026-02-08T00:00:00Z', rating: 1000 },
                    error: null,
                  })),
                })),
              })),
            })),
          };
        }
        if (table === 'arena_profiles') {
          return {
            select: jest.fn(() => ({
              eq: jest.fn(() => ({
                single: jest.fn(async () => ({
                  data: { display_name: 'Opponent', handle: 'opp' },
                  error: null,
                })),
              })),
            })),
          };
        }
        throw new Error(`unexpected table: ${table}`);
      }),
      channel: jest.fn(() => channel),
      removeChannel: jest.fn(async () => ({})),
    };

    setMultiplayerClientFactory(() => supabase as any);

    const svc = new MatchmakingService();
    await svc.joinQueue({ mode: 'battle', rating: 1200, modpackId: 'gen3@1' });

    expect(useMultiplayerStore.getState().inQueue).toBe(true);
    expect(useMultiplayerStore.getState().queueMode).toBe('battle');
    expect(queueUpsert).toHaveBeenCalledWith(
      expect.objectContaining({ modpack_id: 'gen3@1' }),
      { onConflict: 'user_id,mode' },
    );
    expect(supabase.channel).toHaveBeenCalledWith('matchmaking:me');

    // Simulate match found (as player1 / host)
    const match = {
      id: 'match1',
      player1_id: 'me',
      player2_id: 'them',
      mode: 'battle',
      modpack_id: 'gen3@1',
      channel_name: 'match:match1',
      created_at: '2026-02-08T00:00:00Z',
    };

    // Handler 0 is likely player1_id filter.
    await matchInsertHandlers[0]({ new: match });

    const state = useMultiplayerStore.getState();
    expect(state.currentMatchId).toBe('match1');
    expect(state.currentMatchChannelName).toBe('match:match1');
    expect(state.opponentId).toBe('them');
    expect(state.opponentName).toBe('Opponent');
    expect(state.isHost).toBe(true);
    expect(state.currentModpackId).toBe('gen3@1');
  });

  test('leaveQueue deletes queue entry and clears queue state', async () => {
    const channel = {
      on: jest.fn(() => channel),
      subscribe: jest.fn((cb?: (status: string) => void) => {
        cb?.('SUBSCRIBED');
        return channel;
      }),
    };

    const queueDelete = jest.fn(() => ({
      eq: jest.fn(async () => ({ error: null })),
    }));

    const supabase = {
      auth: { getUser: jest.fn(async () => ({ data: { user: { id: 'me' } } })) },
      from: jest.fn((table: string) => {
        if (table === 'matchmaking_queue') {
          return {
            upsert: jest.fn(async () => ({ error: null })),
            delete: queueDelete,
          };
        }
        throw new Error(`unexpected table: ${table}`);
      }),
      channel: jest.fn(() => channel),
      removeChannel: jest.fn(async () => ({})),
    };

    setMultiplayerClientFactory(() => supabase as any);

    const svc = new MatchmakingService();
    await svc.joinQueue({ mode: 'battle' });
    await svc.leaveQueue();

    expect(queueDelete).toHaveBeenCalledTimes(1);
    expect(useMultiplayerStore.getState().inQueue).toBe(false);
  });
});
