import {
  resetMultiplayerClientFactory,
  setMultiplayerClientFactory,
} from '@pokecrystal/core/adapters/multiplayer-client';
import { RealtimeManager } from './realtime-manager';

type BroadcastHandler = (payload: { event: string; payload: any }) => void;

describe('RealtimeManager', () => {
  afterEach(() => {
    resetMultiplayerClientFactory();
  });

  test('joins channel and forwards broadcasts (ignoring self)', async () => {
    let broadcastHandler: ((args: { event: string; payload: any }) => void) | null = null;

    const channel = {
      state: 'joined',
      on: jest.fn((_type: string, _filter: any, cb: BroadcastHandler) => {
        broadcastHandler = cb as any;
        return channel;
      }),
      subscribe: jest.fn((cb?: (status: string) => void) => {
        cb?.('SUBSCRIBED');
        return channel;
      }),
      send: jest.fn(async () => ({})),
    };

    const supabase = {
      auth: { getUser: jest.fn(async () => ({ data: { user: { id: 'me' } } })) },
      channel: jest.fn(() => channel),
      removeChannel: jest.fn(async () => ({})),
    };

    setMultiplayerClientFactory(() => supabase as any);

    const mgr = new RealtimeManager();
    const received: any[] = [];
    mgr.onMessage((m) => received.push(m));

    await mgr.joinMatchChannel('match-123');

    // self message ignored
    broadcastHandler?.({
      event: 'webrtc:offer',
      payload: { from: 'me', data: { hello: 1 } },
    });
    expect(received).toHaveLength(0);

    // remote message forwarded
    broadcastHandler?.({
      event: 'webrtc:offer',
      payload: { from: 'them', data: { sdp: '...' } },
    });

    expect(received).toHaveLength(1);
    expect(received[0]).toEqual({
      type: 'webrtc:offer',
      from: 'them',
      payload: { sdp: '...' },
    });

    await mgr.disconnect();
    expect(supabase.removeChannel).toHaveBeenCalledTimes(1);
  });

  test('sendMessage broadcasts with from field', async () => {
    const channel = {
      state: 'joined',
      on: jest.fn(() => channel),
      subscribe: jest.fn((cb?: (status: string) => void) => {
        cb?.('SUBSCRIBED');
        return channel;
      }),
      send: jest.fn(async () => ({})),
    };

    const supabase = {
      auth: { getUser: jest.fn(async () => ({ data: { user: { id: 'me' } } })) },
      channel: jest.fn(() => channel),
      removeChannel: jest.fn(async () => ({})),
    };

    setMultiplayerClientFactory(() => supabase as any);

    const mgr = new RealtimeManager();
    await mgr.joinMatchChannel('match-123');
    await mgr.sendMessage({ type: 'ready', payload: { ok: true } });

    expect(channel.send).toHaveBeenCalledWith({
      type: 'broadcast',
      event: 'ready',
      payload: { from: 'me', data: { ok: true } },
    });
  });
});
