/**
 * Supabase Realtime Manager
 *
 * Manages Supabase Realtime channels for WebRTC signaling messages
 * (offer, answer, ICE candidates) between two players in a match.
 *
 * Uses Supabase Realtime broadcast feature to send P2P signaling messages
 * without storing them in the database (ephemeral messaging).
 */

import {
  createMultiplayerClient,
  type MultiplayerRealtimeChannel,
} from '@pokecrystal/core/adapters/multiplayer-client';

export type SignalingMessageType =
  | 'webrtc:offer'
  | 'webrtc:answer'
  | 'webrtc:ice'
  | 'ready'
  | 'disconnect';

export interface SignalingMessage {
  type: SignalingMessageType;
  from: string;
  payload: unknown;
}

const SIGNALING_MESSAGE_TYPES = new Set<SignalingMessageType>([
  'webrtc:offer',
  'webrtc:answer',
  'webrtc:ice',
  'ready',
  'disconnect',
]);

const isSignalingMessageType = (value: unknown): value is SignalingMessageType => {
  return typeof value === 'string' && SIGNALING_MESSAGE_TYPES.has(value as SignalingMessageType);
};

export class RealtimeManager {
  private channel: MultiplayerRealtimeChannel | null = null;
  private supabase = createMultiplayerClient();
  private messageCallbacks: ((msg: SignalingMessage) => void)[] = [];
  private userId: string | null = null;

  /**
   * Join a match channel for WebRTC signaling
   * @param matchId - UUID of the match
   */
  async joinMatchChannel(matchId: string): Promise<void> {
    if (!this.supabase) {
      throw new Error('Supabase client not initialized');
    }

    // Get current user ID
    const {
      data: { user },
    } = await this.supabase.auth.getUser();
    if (!user) {
      throw new Error('User not authenticated');
    }
    this.userId = user.id;

    // Create Realtime channel for this match
    this.channel = this.supabase.channel(`match:${matchId}`, {
      config: {
        broadcast: { ack: true }, // Wait for acknowledgment of messages
      },
    });

    // Subscribe to all broadcast events on this channel
    this.channel.on('broadcast', { event: '*' }, ({ event, payload }) => {
      if (!isSignalingMessageType(event)) {
        return;
      }

      if (!payload || typeof payload !== 'object') {
        return;
      }

      const envelope = payload as { from?: unknown; data?: unknown };

      // Ignore messages from ourselves
      if (typeof envelope.from !== 'string' || envelope.from === this.userId) {
        return;
      }

      // Parse and forward to callbacks
      const message: SignalingMessage = {
        type: event,
        from: envelope.from,
        payload: envelope.data,
      };

      this.messageCallbacks.forEach((cb) => cb(message));
    });

    // Subscribe to channel
    await new Promise<void>((resolve, reject) => {
      if (!this.channel) {
        reject(new Error('Realtime channel not initialized'));
        return;
      }
      this.channel.subscribe((status) => {
        if (status === 'SUBSCRIBED') {
          resolve();
          return;
        }
        if (status === 'CHANNEL_ERROR' || status === 'TIMED_OUT') {
          reject(new Error(`Realtime subscribe failed: ${status}`));
        }
      });
    });
    console.log(`[Realtime] Joined match channel: match:${matchId}`);
  }

  /**
   * Send a signaling message to the opponent
   * @param message - Message to send (type and payload)
   */
  async sendMessage(message: Omit<SignalingMessage, 'from'>): Promise<void> {
    if (!this.channel) {
      throw new Error('Not connected to a channel');
    }
    if (!this.userId) {
      throw new Error('User ID not set');
    }

    // Broadcast message to channel
    await this.channel.send({
      type: 'broadcast',
      event: message.type,
      payload: {
        from: this.userId,
        data: message.payload,
      },
    });
  }

  /**
   * Register a callback for received messages
   * @param callback - Function to call when a message is received
   */
  onMessage(callback: (msg: SignalingMessage) => void): void {
    this.messageCallbacks.push(callback);
  }

  /**
   * Remove a message callback
   * @param callback - The callback to remove
   */
  offMessage(callback: (msg: SignalingMessage) => void): void {
    const index = this.messageCallbacks.indexOf(callback);
    if (index !== -1) {
      this.messageCallbacks.splice(index, 1);
    }
  }

  /**
   * Disconnect from the channel and clean up
   */
  async disconnect(): Promise<void> {
    if (this.channel && this.supabase) {
      // Send disconnect message before leaving
      try {
        await this.sendMessage({
          type: 'disconnect',
          payload: { timestamp: Date.now() },
        });
      } catch (error) {
        console.warn('[Realtime] Failed to send disconnect message:', error);
      }

      // Unsubscribe and remove channel
      await this.supabase.removeChannel(this.channel);
      this.channel = null;
      console.log('[Realtime] Disconnected from channel');
    }

    // Clear callbacks
    this.messageCallbacks = [];
    this.userId = null;
  }

  /**
   * Check if currently connected to a channel
   */
  isConnected(): boolean {
    return this.channel !== null;
  }

  /**
   * Get channel status
   */
  getStatus(): 'connected' | 'disconnected' | 'connecting' | 'error' {
    if (!this.channel) return 'disconnected';

    switch (this.channel.state) {
      case 'joined':
        return 'connected';
      case 'joining':
        return 'connecting';
      case 'errored':
      case 'closed':
        return 'error';
      default:
        return 'disconnected';
    }
  }
}
