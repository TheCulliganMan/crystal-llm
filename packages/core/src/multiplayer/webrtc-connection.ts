/**
 * WebRTC Connection Manager
 *
 * Manages WebRTC peer-to-peer connection using simple-peer library.
 * Uses Supabase Realtime (via RealtimeManager) for signaling.
 *
 * Establishes a direct P2P data channel between two players for
 * low-latency battle/trade communication.
 */

import SimplePeer from 'simple-peer';
import { RealtimeManager } from './realtime-manager';

export interface WebRTCMessage {
  type: string;
  data: unknown;
}

export interface WebRTCConfig {
  matchId: string;
  isHost: boolean;
  /** STUN and TURN servers. Production should include short-lived TURN credentials. */
  iceServers?: RTCIceServer[];
}

function isSimplePeerSignalData(value: unknown): value is SimplePeer.SignalData {
  if (!value || typeof value !== 'object') {
    return false;
  }
  const candidateRecord = value as { type?: unknown; candidate?: unknown; sdp?: unknown };
  if (typeof candidateRecord.candidate === 'string') {
    return true;
  }
  return (
    typeof candidateRecord.type === 'string' &&
    (candidateRecord.type === 'offer' ||
      candidateRecord.type === 'answer' ||
      candidateRecord.type === 'candidate' ||
      candidateRecord.type === 'ice') &&
    (typeof candidateRecord.sdp === 'string' || typeof candidateRecord.candidate === 'string')
  );
}

export class WebRTCConnection {
  private peer: SimplePeer.Instance | null = null;
  private realtimeManager: RealtimeManager;
  private isHost: boolean;
  private connected = false;
  private initializationError: Error | null = null;
  private dataCallbacks: ((msg: WebRTCMessage) => void)[] = [];
  private statusCallbacks: {
    onConnect?: () => void;
    onDisconnect?: () => void;
    onError?: (error: Error) => void;
  } = {};

  constructor(config: WebRTCConfig) {
    this.isHost = config.isHost;
    this.realtimeManager = new RealtimeManager();

    // Setup realtime signaling message handler
    this.realtimeManager.onMessage((msg) => {
      if (
        msg.type === 'webrtc:offer' ||
        msg.type === 'webrtc:answer' ||
        msg.type === 'webrtc:ice'
      ) {
        if (isSimplePeerSignalData(msg.payload)) {
          this.handleSignal(msg.payload);
        }
      }
    });

    // Initialize connection
    void this.init(config).catch((error: unknown) => {
      const normalized = error instanceof Error ? error : new Error(String(error));
      this.initializationError = normalized;
      this.statusCallbacks.onError?.(normalized);
    });
  }

  /**
   * Initialize WebRTC peer connection
   */
  private async init(config: WebRTCConfig): Promise<void> {
    // Join Supabase Realtime channel for signaling
    await this.realtimeManager.joinMatchChannel(config.matchId);

    const iceServers = config.iceServers?.length
      ? config.iceServers
      : [
          { urls: 'stun:stun.l.google.com:19302' },
          { urls: 'stun:stun1.l.google.com:19302' },
          { urls: 'stun:stun2.l.google.com:19302' },
        ];

    // Create SimplePeer instance
    this.peer = new SimplePeer({
      initiator: this.isHost, // Host initiates WebRTC offer
      trickle: true, // Send ICE candidates as they're discovered
      config: {
        iceServers,
      },
    });

    // Setup event handlers
    this.setupPeerHandlers();

    console.log(
      `[WebRTC] Initialized as ${this.isHost ? 'host' : 'client'}`
    );
  }

  /**
   * Setup SimplePeer event handlers
   */
  private setupPeerHandlers(): void {
    if (!this.peer) return;

    // Signal event - send signaling data via Supabase Realtime
    this.peer.on('signal', (signal: SimplePeer.SignalData) => {
      // simple-peer emits:
      // - { type: 'offer', sdp: '...' }
      // - { type: 'answer', sdp: '...' }
      // - ICE candidates (no `type` field)
      const type =
        signal.type === 'offer'
          ? 'webrtc:offer'
          : signal.type === 'answer'
            ? 'webrtc:answer'
            : 'webrtc:ice';

      // Send via Supabase Realtime
      this.realtimeManager
        .sendMessage({
          type,
          payload: signal,
        })
        .catch((error) => {
          console.error('[WebRTC] Failed to send signal:', error);
        });
    });

    // Connect event - P2P connection established
    this.peer.on('connect', () => {
      console.log('[WebRTC] P2P connection established!');
      this.connected = true;
      this.statusCallbacks.onConnect?.();
    });

    // Data event - received message from peer
    this.peer.on('data', (data: Uint8Array) => {
      try {
        const message: WebRTCMessage = JSON.parse(data.toString());
        this.dataCallbacks.forEach((cb) => cb(message));
      } catch (error) {
        console.error('[WebRTC] Failed to parse message:', error);
      }
    });

    // Error event
    this.peer.on('error', (err: Error) => {
      console.error('[WebRTC] Error:', err);
      this.connected = false;
      this.statusCallbacks.onError?.(err);
    });

    // Close event
    this.peer.on('close', () => {
      console.log('[WebRTC] Connection closed');
      this.connected = false;
      this.statusCallbacks.onDisconnect?.();
    });
  }

  /**
   * Handle incoming WebRTC signal from opponent
   * @param signal - WebRTC signal data (offer, answer, or ICE candidate)
   */
  private handleSignal(signal: SimplePeer.SignalData): void {
    if (this.peer && !this.peer.destroyed) {
      try {
        this.peer.signal(signal);
      } catch (error) {
        console.error('[WebRTC] Failed to handle signal:', error);
      }
    }
  }

  /**
   * Send a message to the peer
   * @param message - Message to send
   */
  send(message: WebRTCMessage): void {
    if (!this.peer || this.peer.destroyed) {
      console.warn('[WebRTC] Cannot send: peer not connected');
      return;
    }

    try {
      const data = JSON.stringify(message);
      this.peer.send(data);
    } catch (error) {
      console.error('[WebRTC] Failed to send message:', error);
    }
  }

  /**
   * Register a callback for received data
   * @param callback - Function to call when data is received
   */
  onData(callback: (msg: WebRTCMessage) => void): void {
    this.dataCallbacks.push(callback);
  }

  /**
   * Remove a data callback
   * @param callback - The callback to remove
   */
  offData(callback: (msg: WebRTCMessage) => void): void {
    const index = this.dataCallbacks.indexOf(callback);
    if (index !== -1) {
      this.dataCallbacks.splice(index, 1);
    }
  }

  /**
   * Register connection status callbacks
   * @param callbacks - Object with onConnect, onDisconnect, onError functions
   */
  onStatus(callbacks: {
    onConnect?: () => void;
    onDisconnect?: () => void;
    onError?: (error: Error) => void;
  }): void {
    this.statusCallbacks = callbacks;
    if (this.initializationError) {
      callbacks.onError?.(this.initializationError);
    }
  }

  /**
   * Get connection statistics
   * @returns Promise with RTCStatsReport
   */
  async getStats(): Promise<RTCStatsReport | null> {
    if (!this.peer || this.peer.destroyed) {
      return null;
    }

    // Access underlying RTCPeerConnection
    const internalPeer = this.peer as { _pc?: unknown } | null;
    const pc = internalPeer?._pc;
    if (!(pc instanceof RTCPeerConnection)) return null;

    return await pc.getStats();
  }

  /**
   * Check if connection is active
   */
  isConnected(): boolean {
    return this.connected && this.peer !== null && !this.peer.destroyed;
  }

  /**
   * Destroy the WebRTC connection and clean up
   */
  destroy(): void {
    console.log('[WebRTC] Destroying connection...');
    this.connected = false;

    // Destroy peer connection
    if (this.peer) {
      this.peer.destroy();
      this.peer = null;
    }

    // Disconnect from Realtime
    this.realtimeManager.disconnect();

    // Clear callbacks
    this.dataCallbacks = [];
    this.statusCallbacks = {};

    console.log('[WebRTC] Connection destroyed');
  }
}
