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
const CHAT_CHANNELS = new Set(['local', 'trade', 'whisper']);
const MAX_CHAT_MESSAGE_LENGTH = 240;
const CHAT_RATE_WINDOW_MS = 10_000;
const CHAT_RATE_LIMIT = 5;
const DEFAULT_REMOTE_STALE_MS = 15_000;
const PRESENCE_HEARTBEAT_MS = 10_000;
const DEFAULT_WORLD_ID = 'main';
const DEFAULT_MODPACK_ID = 'core-modular';

export type MultiplayerWorldOptions = {
  worldId?: string;
  modpackId?: string;
};

const stableTopicHash = (value: string): string => {
  let hash = 0x811c9dc5;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193);
  }
  return (hash >>> 0).toString(36);
};

const topicSegment = (value: string, fallback: string): string => {
  const source = value.trim() || fallback;
  const readable = source
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 32) || fallback;
  return `${readable}-${stableTopicHash(source)}`;
};

/** A bounded, deterministic Realtime topic scoped to one world, modpack, and map. */
export const buildOverworldChannelName = (
  mapName: string,
  options: MultiplayerWorldOptions = {},
): string => [
  'overworld',
  topicSegment(options.worldId ?? DEFAULT_WORLD_ID, DEFAULT_WORLD_ID),
  topicSegment(options.modpackId ?? DEFAULT_MODPACK_ID, DEFAULT_MODPACK_ID),
  topicSegment(mapName, 'unknown'),
].join(':');

export const buildChatChannelName = (
  options: MultiplayerWorldOptions = {},
): string => [
  'chat',
  topicSegment(options.worldId ?? DEFAULT_WORLD_ID, DEFAULT_WORLD_ID),
  topicSegment(options.modpackId ?? DEFAULT_MODPACK_ID, DEFAULT_MODPACK_ID),
].join(':');

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

export type MultiplayerChatChannel = 'local' | 'trade' | 'whisper';

export type MultiplayerChatMessage = {
  messageId: string;
  fromUserId: string;
  fromPlayerName: string;
  toUserId: string | null;
  channel: MultiplayerChatChannel;
  mapName: string;
  text: string;
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
  private chatChannel: MultiplayerRealtimeChannel | null = null;
  private localUserId: string | null = null;
  private localState: LocalPresenceState | null = null;
  private lastPresencePushAtMs = 0;
  private readonly worldOptions: Required<MultiplayerWorldOptions>;
  private readonly callbacks: Array<(players: RemoteOverworldPlayer[]) => void> = [];
  private readonly interactionRequestCallbacks: Array<
    (request: MultiplayerInteractionRequest) => void
  > = [];
  private readonly interactionResponseCallbacks: Array<
    (response: MultiplayerInteractionResponse) => void
  > = [];
  private readonly chatMessageCallbacks: Array<(message: MultiplayerChatMessage) => void> = [];
  private readonly blockedUserIds = new Set<string>();
  private chatSendTimestamps: number[] = [];

  constructor(options: MultiplayerWorldOptions = {}) {
    this.worldOptions = {
      worldId: options.worldId?.trim() || DEFAULT_WORLD_ID,
      modpackId: options.modpackId?.trim() || DEFAULT_MODPACK_ID,
    };
  }

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
    await this.joinChatChannel();
    await this.joinMapChannel(localState.mapName);
    await this.pushLocalState();
  }

  private async joinMapChannel(mapName: string): Promise<void> {
    if (!this.supabase || !this.localUserId) {
      throw new Error('Multiplayer presence is not initialized');
    }

    this.channel = this.supabase.channel(buildOverworldChannelName(mapName, this.worldOptions), {
      config: {
        broadcast: { ack: true },
        presence: { key: this.localUserId },
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

  }

  private async joinChatChannel(): Promise<void> {
    if (!this.supabase || !this.localUserId) {
      throw new Error('Multiplayer chat is not initialized');
    }
    this.chatChannel = this.supabase.channel(buildChatChannelName(this.worldOptions), {
      config: { broadcast: { ack: true } },
    });
    this.chatChannel.on('broadcast', { event: 'chat:message' }, ({ payload }) => {
      const message = this.normalizeChatMessage(payload);
      if (!message || message.fromUserId === this.localUserId || this.blockedUserIds.has(message.fromUserId)) {
        return;
      }
      if (message.channel === 'whisper' && message.toUserId !== this.localUserId) {
        return;
      }
      if (message.channel === 'local' && message.mapName !== this.localState?.mapName) {
        return;
      }
      for (const callback of this.chatMessageCallbacks) {
        callback(message);
      }
    });
    await new Promise<void>((resolve, reject) => {
      const activeChannel = this.chatChannel;
      if (!activeChannel) {
        reject(new Error('Chat channel not initialized'));
        return;
      }
      activeChannel.subscribe((status) => {
        if (status === 'SUBSCRIBED') {
          resolve();
          return;
        }
        if (status === 'CHANNEL_ERROR' || status === 'TIMED_OUT') {
          reject(new Error(`Chat subscribe failed: ${status}`));
        }
      });
    });
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
    const changedMap = this.localState?.mapName !== next.mapName;
    const changedState = !this.localState || (
      this.localState.playerName !== next.playerName
      || this.localState.entityType !== next.entityType
      || this.localState.mapName !== next.mapName
      || this.localState.tileX !== next.tileX
      || this.localState.tileY !== next.tileY
      || this.localState.direction !== next.direction
    );
    this.localState = next;
    if (!this.channel) {
      return;
    }
    if (changedMap) {
      if (this.supabase) {
        await this.supabase.removeChannel(this.channel);
      }
      this.channel = null;
      this.emit([]);
      await this.joinMapChannel(next.mapName);
    }
    if (!changedState && Date.now() - this.lastPresencePushAtMs < PRESENCE_HEARTBEAT_MS) {
      return;
    }
    await this.pushLocalState();
  }

  async disconnect(): Promise<void> {
    if (this.supabase && this.channel) {
      await this.supabase.removeChannel(this.channel);
    }
    if (this.supabase && this.chatChannel) {
      await this.supabase.removeChannel(this.chatChannel);
    }
    this.channel = null;
    this.chatChannel = null;
    this.localUserId = null;
    this.localState = null;
    this.lastPresencePushAtMs = 0;
    this.chatSendTimestamps = [];
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

  onChatMessage(callback: (message: MultiplayerChatMessage) => void): void {
    this.chatMessageCallbacks.push(callback);
  }

  offChatMessage(callback: (message: MultiplayerChatMessage) => void): void {
    const index = this.chatMessageCallbacks.indexOf(callback);
    if (index >= 0) {
      this.chatMessageCallbacks.splice(index, 1);
    }
  }

  setBlockedUserIds(userIds: Iterable<string>): void {
    this.blockedUserIds.clear();
    for (const userId of userIds) {
      if (userId) {
        this.blockedUserIds.add(userId);
      }
    }
  }

  async sendChatMessage(
    channel: MultiplayerChatChannel,
    text: string,
    toUserId: string | null = null,
  ): Promise<MultiplayerChatMessage> {
    if (!this.chatChannel || !this.localUserId || !this.localState) {
      throw new Error('Not connected to multiplayer chat');
    }
    if (!CHAT_CHANNELS.has(channel)) {
      throw new Error('Unknown chat channel');
    }
    const normalizedText = text.trim();
    if (!normalizedText) {
      throw new Error('Message cannot be empty');
    }
    if (normalizedText.length > MAX_CHAT_MESSAGE_LENGTH) {
      throw new Error(`Message cannot exceed ${MAX_CHAT_MESSAGE_LENGTH} characters`);
    }
    if (channel === 'whisper' && !toUserId) {
      throw new Error('Whisper requires a recipient');
    }
    const nowMs = Date.now();
    this.chatSendTimestamps = this.chatSendTimestamps.filter(
      (timestamp) => nowMs - timestamp < CHAT_RATE_WINDOW_MS,
    );
    if (this.chatSendTimestamps.length >= CHAT_RATE_LIMIT) {
      throw new Error('You are sending messages too quickly');
    }
    this.chatSendTimestamps.push(nowMs);
    const message: MultiplayerChatMessage = {
      messageId: this.createMessageId(),
      fromUserId: this.localUserId,
      fromPlayerName: this.localState.playerName,
      toUserId: channel === 'whisper' ? toUserId : null,
      channel,
      mapName: this.localState.mapName,
      text: normalizedText,
      timestampMs: nowMs,
    };
    await this.chatChannel.send({
      type: 'broadcast',
      event: 'chat:message',
      payload: message,
    });
    return message;
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
      worldId: this.worldOptions.worldId,
      modpackId: this.worldOptions.modpackId,
      updatedAtMs: Date.now(),
    });
    this.lastPresencePushAtMs = Date.now();
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

  private createMessageId(): string {
    if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
      return crypto.randomUUID();
    }
    return `msg-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
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

  private normalizeChatMessage(payload: unknown): MultiplayerChatMessage | null {
    const data = payload as Record<string, unknown> | null;
    if (!data) {
      return null;
    }
    const { messageId, fromUserId, fromPlayerName, toUserId, channel, mapName } = data;
    const text = typeof data.text === 'string' ? data.text.trim() : '';
    if (
      typeof messageId !== 'string' || !messageId ||
      typeof fromUserId !== 'string' || !fromUserId ||
      typeof fromPlayerName !== 'string' || !fromPlayerName ||
      (toUserId !== null && typeof toUserId !== 'string') ||
      typeof channel !== 'string' || !CHAT_CHANNELS.has(channel) ||
      typeof mapName !== 'string' || !mapName ||
      !text || text.length > MAX_CHAT_MESSAGE_LENGTH ||
      (channel === 'whisper' && (typeof toUserId !== 'string' || !toUserId))
    ) {
      return null;
    }
    return {
      messageId,
      fromUserId,
      fromPlayerName,
      toUserId: typeof toUserId === 'string' ? toUserId : null,
      channel: channel as MultiplayerChatChannel,
      mapName,
      text,
      timestampMs:
        typeof data.timestampMs === 'number' && Number.isFinite(data.timestampMs)
          ? Math.trunc(data.timestampMs)
          : Date.now(),
    };
  }
}
