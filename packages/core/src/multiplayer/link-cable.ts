/**
 * Link Cable Emulator
 *
 * Emulates the Game Boy Color link cable byte-level serial transfer protocol
 * over WebRTC. Maintains protocol fidelity with Gen 2 Pokemon games for
 * battles and trading.
 *
 * Original Link Cable Specs:
 * - Transfer rate: 8192 Hz (1 byte per ~0.122ms)
 * - Serial transfer with clock synchronization
 * - Half-duplex bidirectional communication
 */

import { WebRTCConnection, WebRTCMessage } from './webrtc-connection';

type LinkBytePayload = {
  byte: number;
  clock: number;
  timestamp: number;
};

type LinkSyncPayload = {
  t0: number;
  t1: number;
  t2: number;
};

export class LinkCableEmulator {
  private connection: WebRTCConnection;
  private isHost: boolean;

  // Buffer for bytes received when nobody is currently waiting.
  private receiveBuffer: number[] = [];
  private receiveWaiters: Array<(byte: number) => void> = [];

  // Synchronization state
  private remoteClock = 0;
  private localClock = 0;
  private latencyMs = 0;

  constructor(connection: WebRTCConnection, isHost: boolean) {
    this.connection = connection;
    this.isHost = isHost;

    // Listen for link cable messages
    this.connection.onData((msg: WebRTCMessage) => {
      this.handleRemoteData(msg);
    });
  }

  /**
   * Establish link cable connection with preamble handshake
   * Matches original Gen 2 link cable initialization sequence
   *
   * @returns True if connection established successfully
   */
  async establishConnection(): Promise<boolean> {
    const PREAMBLE_BYTE = 0x00;
    const PREAMBLE_RESPONSE = 0x61;
    const MAX_ATTEMPTS = 16;

    console.log(
      `[LinkCable] Establishing connection as ${this.isHost ? 'host' : 'client'}...`
    );

    if (this.isHost) {
      // Host initiates handshake
      for (let i = 0; i < MAX_ATTEMPTS; i++) {
        const response = await this.sendByte(PREAMBLE_BYTE);
        if (response === PREAMBLE_RESPONSE) {
          console.log('[LinkCable] Connection established!');
          return true;
        }
      }
      console.error('[LinkCable] Failed to establish connection (host)');
      return false;
    } else {
      // Client responds to handshake
      for (let i = 0; i < MAX_ATTEMPTS; i++) {
        try {
          const byte = await this.receiveByte(1000); // 1s timeout
          if (byte === PREAMBLE_BYTE) {
            // Respond without waiting for a reply; the host's in-flight transfer is waiting
            // specifically for this response byte.
            const clock = ++this.localClock;
            this.sendRawByte(PREAMBLE_RESPONSE, clock);
            console.log('[LinkCable] Connection established!');
            return true;
          }
        } catch (error) {
          // Timeout, continue waiting
        }
      }
      console.error('[LinkCable] Failed to establish connection (client)');
      return false;
    }
  }

  private sendRawByte(byte: number, clock: number): void {
    this.connection.send({
      type: 'link:byte',
      data: {
        byte,
        clock,
        timestamp: performance.now(),
      },
    });
  }

  /**
   * Send a single byte and wait for response byte
   * Simulates bidirectional serial transfer
   *
   * @param byte - Byte to send (0x00-0xFF)
   * @returns Response byte from opponent
   */
  async sendByte(byte: number): Promise<number> {
    // Validate byte
    if (byte < 0 || byte > 0xff) {
      throw new Error(`Invalid byte value: ${byte} (must be 0x00-0xFF)`);
    }

    // Increment local clock
    const clock = ++this.localClock;

    // Send byte with clock tick
    this.sendRawByte(byte, clock);

    // Wait for response byte
    return this.receiveByte();
  }

  /**
   * Send multiple bytes in sequence
   * @param bytes - Array of bytes to send
   * @returns Array of response bytes
   */
  async sendBytes(bytes: number[]): Promise<number[]> {
    const received: number[] = [];
    for (const byte of bytes) {
      const response = await this.sendByte(byte);
      received.push(response);
    }
    return received;
  }

  /**
   * Receive a single byte (waits for remote to send)
   * @param timeout - Max time to wait in milliseconds (default: 5000ms)
   * @returns Received byte
   */
  private async receiveByte(timeout = 5000): Promise<number> {
    return new Promise<number>((resolve, reject) => {
      // Check if byte already in buffer
      if (this.receiveBuffer.length > 0) {
        resolve(this.receiveBuffer.shift()!);
        return;
      }

      // Wait for next byte to arrive.
      const waiter = (nextByte: number) => {
        clearTimeout(timeoutId);
        resolve(nextByte);
      };

      const timeoutId = setTimeout(() => {
        const index = this.receiveWaiters.indexOf(waiter);
        if (index !== -1) {
          this.receiveWaiters.splice(index, 1);
        }
        reject(new Error('Link cable receive timeout'));
      }, timeout);

      this.receiveWaiters.push(waiter);
    });
  }

  /**
   * Handle incoming link cable data from remote peer
   * @param msg - WebRTC message containing byte data
   */
  private handleRemoteData(msg: WebRTCMessage): void {
    switch (msg.type) {
      case 'link:byte': {
        const { byte, clock, timestamp } = msg.data as LinkBytePayload;

        // Calculate latency
        const latency = performance.now() - timestamp;
        this.latencyMs = this.latencyMs * 0.9 + latency * 0.1; // Moving average

        // Update remote clock
        this.remoteClock = clock;

        const waiter = this.receiveWaiters.shift();
        if (waiter) {
          waiter(byte);
        } else {
          this.receiveBuffer.push(byte);
        }

        break;
      }

      case 'link:sync': {
        this.handleSync(msg.data as LinkSyncPayload);
        break;
      }

      default:
        console.warn(`[LinkCable] Unknown message type: ${msg.type}`);
    }
  }

  /**
   * Handle clock synchronization message
   * Uses NTP-like algorithm to adjust local clock
   */
  private handleSync(data: LinkSyncPayload): void {
    const t0 = data.t0; // Client send time
    const t1 = data.t1; // Server receive time
    const t2 = data.t2; // Server send time
    const t3 = performance.now(); // Client receive time

    const roundTripDelay = t3 - t0 - (t2 - t1);
    const offset = ((t1 - t0) + (t2 - t3)) / 2;

    // Adjust local clock
    this.localClock += Math.round(offset);
  }

  /**
   * Start periodic clock synchronization loop
   * Keeps clocks aligned over time
   */
  startSyncLoop(): void {
    if (!this.isHost) return; // Only host sends sync messages

    setInterval(() => {
      const t0 = performance.now();
      this.connection.send({
        type: 'link:sync',
        data: {
          t0,
          t1: t0,
          t2: t0,
        },
      });
    }, 1000); // Sync every second
  }

  /**
   * Get current network latency
   * @returns Latency in milliseconds
   */
  getLatency(): number {
    return this.latencyMs;
  }

  /**
   * Get clock synchronization status
   * @returns Clock drift in ticks
   */
  getClockDrift(): number {
    return Math.abs(this.localClock - this.remoteClock);
  }

  /**
   * Reset the link cable state
   */
  reset(): void {
    this.receiveBuffer = [];
    this.receiveWaiters = [];
    this.localClock = 0;
    this.remoteClock = 0;
    this.latencyMs = 0;
  }

  /**
   * Close the link cable connection
   */
  close(): void {
    this.reset();
    this.connection.destroy();
  }
}
