import {
  resetMultiplayerClientFactory,
  setMultiplayerClientFactory,
} from '@pokecrystal/core/adapters/multiplayer-client';
import {
  OverworldPresenceManager,
  type MultiplayerInteractionRequest,
  type MultiplayerInteractionResponse,
  type MultiplayerChatMessage,
} from './overworld-presence';

type PresencePayload = Record<string, unknown>;
type PresenceSyncHandler = () => void;
type BroadcastHandler = (payload: { event: string; payload: unknown }) => void;

class PresenceHub {
  private channelsByName = new Map<string, Set<FakeRealtimeChannel>>();
  private presenceByName = new Map<string, Map<string, PresencePayload>>();

  register(channel: FakeRealtimeChannel): void {
    if (!this.channelsByName.has(channel.name)) {
      this.channelsByName.set(channel.name, new Set());
    }
    this.channelsByName.get(channel.name)?.add(channel);
  }

  remove(channel: FakeRealtimeChannel): void {
    this.channelsByName.get(channel.name)?.delete(channel);
    this.presenceByName.get(channel.name)?.delete(channel.presenceKey);
    this.emitPresenceSync(channel.name);
  }

  track(channel: FakeRealtimeChannel, payload: PresencePayload): void {
    if (!this.presenceByName.has(channel.name)) {
      this.presenceByName.set(channel.name, new Map());
    }
    this.presenceByName.get(channel.name)?.set(channel.presenceKey, payload);
    this.emitPresenceSync(channel.name);
  }

  broadcast(source: FakeRealtimeChannel, event: string, payload: unknown): void {
    const channels = this.channelsByName.get(source.name);
    if (!channels) {
      return;
    }
    for (const channel of channels) {
      channel.emitBroadcast(event, payload);
    }
  }

  presenceState(name: string): Record<string, PresencePayload[]> {
    const map = this.presenceByName.get(name);
    if (!map) {
      return {};
    }
    const result: Record<string, PresencePayload[]> = {};
    for (const [key, payload] of map.entries()) {
      result[key] = [payload];
    }
    return result;
  }

  private emitPresenceSync(name: string): void {
    const channels = this.channelsByName.get(name);
    if (!channels) {
      return;
    }
    for (const channel of channels) {
      channel.emitPresenceSync();
    }
  }
}

class FakeRealtimeChannel {
  private presenceHandlers: PresenceSyncHandler[] = [];
  private broadcastHandlers: Array<{ event: string; handler: BroadcastHandler }> = [];

  constructor(
    private hub: PresenceHub,
    public readonly name: string,
    public readonly presenceKey: string
  ) {}

  on(type: string, filter: { event: string }, cb: PresenceSyncHandler | BroadcastHandler): FakeRealtimeChannel {
    if (type === 'presence' && filter.event === 'sync') {
      this.presenceHandlers.push(cb as PresenceSyncHandler);
    }
    if (type === 'broadcast') {
      this.broadcastHandlers.push({ event: filter.event, handler: cb as BroadcastHandler });
    }
    return this;
  }

  subscribe(cb?: (status: string) => void): FakeRealtimeChannel {
    this.hub.register(this);
    cb?.('SUBSCRIBED');
    return this;
  }

  async track(payload: PresencePayload): Promise<void> {
    this.hub.track(this, payload);
  }

  presenceState(): Record<string, PresencePayload[]> {
    return this.hub.presenceState(this.name);
  }

  async send(message: { type: string; event: string; payload: unknown }): Promise<void> {
    if (message.type !== 'broadcast') {
      return;
    }
    this.hub.broadcast(this, message.event, message.payload);
  }

  emitPresenceSync(): void {
    for (const handler of this.presenceHandlers) {
      handler();
    }
  }

  emitBroadcast(event: string, payload: unknown): void {
    for (const entry of this.broadcastHandlers) {
      if (entry.event === '*' || entry.event === event) {
        entry.handler({ event, payload });
      }
    }
  }
}

describe('OverworldPresenceManager multi-session integration', () => {
  afterEach(() => {
    resetMultiplayerClientFactory();
  });

  test('synchronizes two sessions and supports talk/battle/trade interactions', async () => {
    const hub = new PresenceHub();

    const makeSupabaseClient = (userId: string) => ({
      auth: { getUser: jest.fn(async () => ({ data: { user: { id: userId } } })) },
      channel: jest.fn((name: string, options: { config?: { presence?: { key?: string } } }) => {
        const key = options?.config?.presence?.key ?? userId;
        return new FakeRealtimeChannel(hub, name, key);
      }),
      removeChannel: jest.fn(async (channel: FakeRealtimeChannel) => {
        hub.remove(channel);
        return { data: null };
      }),
    });

    const queue = [makeSupabaseClient('u1'), makeSupabaseClient('u2')];
    setMultiplayerClientFactory(() => queue.shift() as any);

    const managerA = new OverworldPresenceManager();
    const managerB = new OverworldPresenceManager();

    let remoteSeenByA: unknown[] = [];
    let remoteSeenByB: unknown[] = [];
    const incomingB: MultiplayerInteractionRequest[] = [];
    const responsesA: MultiplayerInteractionResponse[] = [];
    const chatSeenByB: MultiplayerChatMessage[] = [];

    managerA.onRemotePlayersChange((players) => {
      remoteSeenByA = players;
    });
    managerB.onRemotePlayersChange((players) => {
      remoteSeenByB = players;
    });
    managerB.onInteractionRequest((request) => incomingB.push(request));
    managerA.onInteractionResponse((response) => responsesA.push(response));
    managerB.onChatMessage((message) => chatSeenByB.push(message));

    await managerA.connect({
      playerName: 'Alice',
      mapName: 'NewBarkTown',
      tileX: 10,
      tileY: 9,
      direction: 'down',
      entityType: 'player',
    });

    await managerB.connect({
      playerName: 'Bob',
      mapName: 'NewBarkTown',
      tileX: 11,
      tileY: 9,
      direction: 'left',
      entityType: 'player',
    });

    expect((remoteSeenByA as any[]).some((p) => p.userId === 'u2')).toBe(true);
    expect((remoteSeenByB as any[]).some((p) => p.userId === 'u1')).toBe(true);

    const localMessage = await managerA.sendChatMessage('local', '  Want to battle?  ');
    expect(localMessage.text).toBe('Want to battle?');
    expect(chatSeenByB).toEqual([
      expect.objectContaining({
        messageId: localMessage.messageId,
        fromUserId: 'u1',
        fromPlayerName: 'Alice',
        toUserId: null,
        channel: 'local',
        text: 'Want to battle?',
      }),
    ]);

    await managerA.sendChatMessage('trade', 'Trading Cyndaquil');
    await managerA.sendChatMessage('whisper', 'Secret plan', 'u2');
    expect(chatSeenByB.map((message) => message.channel)).toEqual(['local', 'trade', 'whisper']);

    managerB.setBlockedUserIds(['u1']);
    await managerA.sendChatMessage('local', 'This should be ignored');
    expect(chatSeenByB).toHaveLength(3);

    const requestId = await managerA.sendInteractionRequest('u2', 'battle');
    expect(requestId.length).toBeGreaterThan(4);
    expect(incomingB).toHaveLength(1);
    expect(incomingB[0]?.kind).toBe('battle');

    await managerB.sendInteractionResponse(incomingB[0], true);
    expect(responsesA).toHaveLength(1);
    expect(responsesA[0]).toMatchObject({
      requestId: incomingB[0].requestId,
      kind: 'battle',
      accepted: true,
    });

    await managerA.disconnect();
    await managerB.disconnect();
  });
});
