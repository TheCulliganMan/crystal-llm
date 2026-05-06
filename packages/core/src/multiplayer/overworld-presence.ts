import {
  createMultiplayerClient,
  type MultiplayerRealtimeChannel,
} from '@pokecrystal/core/adapters/multiplayer-client';
import type { RemoteOverworldPlayer } from '@pokecrystal/core/types/overworld';

export type LocalPresenceState = {
  playerName: string;
  entityType?: 'player' | 'ai';
  mapName: string;
  tileX: number;
  tileY: number;
  direction: 'up' | 'down' | 'left' | 'right';
};

type PresenceEntry = {
  userId?: unknown;
  playerName?: unknown;
  entityType?: unknown;
  mapName?: unknown;
  tileX?: unknown;
  tileY?: unknown;
  direction?: unknown;
  updatedAtMs?: unknown;
};

type PresenceState = Record<string, PresenceEntry[]>;

const VALID_DIRECTIONS = new Set(['up', 'down', 'left', 'right']);
const INTERACTION_KINDS = new Set(['battle', 'trade']);
const DEFAULT_REMOTE_STALE_MS = 15_000;

export type MultiplayerInteractionKind = 'battle' | 'trade';

export type MultiplayerInteractionRequest = {
  requestId: string;
  fromUserId: string;
  fromPlayerName: string;
  toUserId: string;
  kind: MultiplayerInteractionKind;
  timestampMs: number;
};

export type MultiplayerInteractionResponse = {
  requestId: string;
  fromUserId: string;
  toUserId: string;
  kind: MultiplayerInteractionKind;
  accepted: boolean;
  timestampMs: number;
};

const toFiniteInt = (value: unknown): number | null => {
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    return null;
  }
  return Math.trunc(value);
};

const toDirection = (value: unknown): RemoteOverworldPlayer['direction'] | null => {
  if (typeof value !== 'string') {
    return null;
  }
  const normalized = value.toLowerCase();
  if (!VALID_DIRECTIONS.has(normalized)) {
    return null;
  }
  return normalized as RemoteOverworldPlayer['direction'];
};

const normalizeRemotePlayer = (entry: PresenceEntry): RemoteOverworldPlayer | null => {
  if (typeof entry.userId !== 'string' || entry.userId.length === 0) {
    return null;
  }
  if (typeof entry.playerName !== 'string' || entry.playerName.length === 0) {
    return null;
  }
  if (typeof entry.mapName !== 'string' || entry.mapName.length === 0) {
    return null;
  }
  const tileX = toFiniteInt(entry.tileX);
  const tileY = toFiniteInt(entry.tileY);
  if (tileX === null || tileY === null) {
    return null;
  }
  const direction = toDirection(entry.direction);
  if (!direction) {
    return null;
  }
  const updatedAtMs =
    typeof entry.updatedAtMs === 'number' && Number.isFinite(entry.updatedAtMs)
      ? Math.trunc(entry.updatedAtMs)
      : Date.now();

  const entityType = entry.entityType === 'ai' ? 'ai' : 'player';
  return {
    userId: entry.userId,
    playerName: entry.playerName,
    entityType,
    mapName: entry.mapName,
    tileX,
    tileY,
    direction,
    updatedAtMs,
  };
};

export const extractRemotePlayersFromPresence = (
  presence: PresenceState,
  localUserId: string,
  options: { nowMs?: number; staleMs?: number } = {}
): RemoteOverworldPlayer[] => {
  const nowMs = options.nowMs ?? Date.now();
  const staleMs = options.staleMs ?? DEFAULT_REMOTE_STALE_MS;
  const byUser = new Map<string, RemoteOverworldPlayer>();
  for (const entries of Object.values(presence)) {
    for (const rawEntry of entries) {
      const player = normalizeRemotePlayer(rawEntry);
      if (!player || player.userId === localUserId) {
        continue;
      }
      if (nowMs - player.updatedAtMs > staleMs) {
        continue;
      }
      const previous = byUser.get(player.userId);
      if (!previous || player.updatedAtMs > previous.updatedAtMs) {
        byUser.set(player.userId, player);
      }
    }
  }
  return Array.from(byUser.values());
};

export class OverworldPresenceManager {
  private readonly supabase = createMultiplayerClient();
  private channel: MultiplayerRealtimeChannel | null = null;
  private localUserId: string | null = null;
  private localState: LocalPresenceState | null = null;
  private readonly callbacks: Array<(players: RemoteOverworldPlayer[]) => void> = [];
  private readonly interactionRequestCallbacks: Array<
    (request: MultiplayerInteractionRequest) => void
  > = [];
  private readonly interactionResponseCallbacks: Array<
    (response: MultiplayerInteractionResponse) => void
  > = [];

  async connect(localState: LocalPresenceState): Promise<void> {
    if (!this.supabase) {
      throw new Error('Supabase client not initialized');
    }
    if (this.channel) {
      await this.disconnect();
    }

    const {
      data: { user },
    } = await this.supabase.auth.getUser();
    if (!user) {
      throw new Error('User not authenticated');
    }

    this.localUserId = user.id;
    this.localState = localState;
    this.channel = this.supabase.channel('overworld:presence', {
      config: {
        broadcast: { ack: true },
        presence: { key: user.id },
      },
    });

    this.channel.on('presence', { event: 'sync' }, () => {
      this.emitRemotePlayers();
    });
    this.channel.on('broadcast', { event: 'interaction:request' }, ({ payload }) => {
      const request = this.normalizeInteractionRequest(payload);
      if (!request || request.toUserId !== this.localUserId || request.fromUserId === this.localUserId) {
        return;
      }
      for (const callback of this.interactionRequestCallbacks) {
        callback(request);
      }
    });
    this.channel.on('broadcast', { event: 'interaction:response' }, ({ payload }) => {
      const response = this.normalizeInteractionResponse(payload);
      if (!response || response.toUserId !== this.localUserId || response.fromUserId === this.localUserId) {
        return;
      }
      for (const callback of this.interactionResponseCallbacks) {
        callback(response);
      }
    });

    await new Promise<void>((resolve, reject) => {
      const activeChannel = this.channel;
      if (!activeChannel) {
        reject(new Error('Realtime channel not initialized'));
        return;
      }
      activeChannel.subscribe((status) => {
        if (status === 'SUBSCRIBED') {
          resolve();
          return;
        }
        if (status === 'CHANNEL_ERROR' || status === 'TIMED_OUT') {
          reject(new Error(`Presence subscribe failed: ${status}`));
        }
      });
    });

    await this.pushLocalState();
  }

  onRemotePlayersChange(callback: (players: RemoteOverworldPlayer[]) => void): void {
    this.callbacks.push(callback);
  }

  offRemotePlayersChange(callback: (players: RemoteOverworldPlayer[]) => void): void {
    const index = this.callbacks.indexOf(callback);
    if (index >= 0) {
      this.callbacks.splice(index, 1);
    }
  }

  async updateLocalState(next: LocalPresenceState): Promise<void> {
    this.localState = next;
    if (!this.channel) {
      return;
    }
    await this.pushLocalState();
  }

  async disconnect(): Promise<void> {
    if (this.supabase && this.channel) {
      await this.supabase.removeChannel(this.channel);
    }
    this.channel = null;
    this.localUserId = null;
    this.localState = null;
    this.emit([]);
  }

  onInteractionRequest(callback: (request: MultiplayerInteractionRequest) => void): void {
    this.interactionRequestCallbacks.push(callback);
  }

  offInteractionRequest(callback: (request: MultiplayerInteractionRequest) => void): void {
    const index = this.interactionRequestCallbacks.indexOf(callback);
    if (index >= 0) {
      this.interactionRequestCallbacks.splice(index, 1);
    }
  }

  onInteractionResponse(callback: (response: MultiplayerInteractionResponse) => void): void {
    this.interactionResponseCallbacks.push(callback);
  }

  offInteractionResponse(callback: (response: MultiplayerInteractionResponse) => void): void {
    const index = this.interactionResponseCallbacks.indexOf(callback);
    if (index >= 0) {
      this.interactionResponseCallbacks.splice(index, 1);
    }
  }

  async sendInteractionRequest(toUserId: string, kind: MultiplayerInteractionKind): Promise<string> {
    if (!this.channel || !this.localUserId || !this.localState) {
      throw new Error('Not connected to multiplayer presence');
    }
    const requestId = this.createRequestId();
    await this.channel.send({
      type: 'broadcast',
      event: 'interaction:request',
      payload: {
        requestId,
        fromUserId: this.localUserId,
        fromPlayerName: this.localState.playerName,
        toUserId,
        kind,
        timestampMs: Date.now(),
      },
    });
    return requestId;
  }

  async sendInteractionResponse(
    request: MultiplayerInteractionRequest,
    accepted: boolean
  ): Promise<void> {
    if (!this.channel || !this.localUserId) {
      throw new Error('Not connected to multiplayer presence');
    }
    await this.channel.send({
      type: 'broadcast',
      event: 'interaction:response',
      payload: {
        requestId: request.requestId,
        fromUserId: this.localUserId,
        toUserId: request.fromUserId,
        kind: request.kind,
        accepted: Boolean(accepted),
        timestampMs: Date.now(),
      },
    });
  }

  private async pushLocalState(): Promise<void> {
    const activeChannel = this.channel;
    if (!activeChannel || !activeChannel.track || !this.localUserId || !this.localState) {
      return;
    }
    await activeChannel.track({
      userId: this.localUserId,
      playerName: this.localState.playerName,
      entityType: this.localState.entityType ?? 'player',
      mapName: this.localState.mapName,
      tileX: this.localState.tileX,
      tileY: this.localState.tileY,
      direction: this.localState.direction,
      updatedAtMs: Date.now(),
    });
    this.emitRemotePlayers();
  }

  private emitRemotePlayers(): void {
    const activeChannel = this.channel;
    if (!activeChannel || !activeChannel.presenceState || !this.localUserId) {
      this.emit([]);
      return;
    }

    const state = activeChannel.presenceState() as PresenceState;
    const players = extractRemotePlayersFromPresence(state, this.localUserId);
    this.emit(players);
  }

  private emit(players: RemoteOverworldPlayer[]): void {
    for (const callback of this.callbacks) {
      callback(players);
    }
  }

  private createRequestId(): string {
    if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
      return crypto.randomUUID();
    }
    return `req-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
  }

  private normalizeInteractionRequest(payload: unknown): MultiplayerInteractionRequest | null {
    const data = payload as Record<string, unknown> | null;
    if (!data) {
      return null;
    }
    const requestId = data.requestId;
    const fromUserId = data.fromUserId;
    const fromPlayerName = data.fromPlayerName;
    const toUserId = data.toUserId;
    const kind = data.kind;
    if (
      typeof requestId !== 'string' ||
      typeof fromUserId !== 'string' ||
      typeof fromPlayerName !== 'string' ||
      typeof toUserId !== 'string' ||
      typeof kind !== 'string' ||
      !INTERACTION_KINDS.has(kind)
    ) {
      return null;
    }
    return {
      requestId,
      fromUserId,
      fromPlayerName,
      toUserId,
      kind: kind as MultiplayerInteractionKind,
      timestampMs:
        typeof data.timestampMs === 'number' && Number.isFinite(data.timestampMs)
          ? Math.trunc(data.timestampMs)
          : Date.now(),
    };
  }

  private normalizeInteractionResponse(payload: unknown): MultiplayerInteractionResponse | null {
    const data = payload as Record<string, unknown> | null;
    if (!data) {
      return null;
    }
    const requestId = data.requestId;
    const fromUserId = data.fromUserId;
    const toUserId = data.toUserId;
    const kind = data.kind;
    const accepted = data.accepted;
    if (
      typeof requestId !== 'string' ||
      typeof fromUserId !== 'string' ||
      typeof toUserId !== 'string' ||
      typeof kind !== 'string' ||
      !INTERACTION_KINDS.has(kind) ||
      typeof accepted !== 'boolean'
    ) {
      return null;
    }
    return {
      requestId,
      fromUserId,
      toUserId,
      kind: kind as MultiplayerInteractionKind,
      accepted,
      timestampMs:
        typeof data.timestampMs === 'number' && Number.isFinite(data.timestampMs)
          ? Math.trunc(data.timestampMs)
          : Date.now(),
    };
  }
}
