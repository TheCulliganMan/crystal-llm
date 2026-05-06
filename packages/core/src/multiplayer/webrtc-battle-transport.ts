/**
 * WebRTC Battle Transport
 *
 * Adapts an arbitrary WebRTC data-channel connection into a BattleSyncTransport.
 *
 * This module intentionally avoids importing `simple-peer` so it can be tested
 * in Node/Jest without browser/WebRTC globals.
 */

import type { BattleSyncMessage, BattleSyncTransport } from "./battle-synchronizer";

export type WebRTCEnvelope = {
  type: string;
  data: unknown;
};

export interface WebRTCConnectionLike {
  send(message: WebRTCEnvelope): void;
  onData(callback: (message: WebRTCEnvelope) => void): void;
  offData?(callback: (message: WebRTCEnvelope) => void): void;
}

const BATTLE_ENVELOPE_TYPE = "battle_sync";

const isObject = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null;

function isBattleSyncMessage(value: unknown): value is BattleSyncMessage {
  if (!isObject(value)) return false;
  if (typeof value.type !== "string") return false;
  if (!("data" in value)) return false;
  return (
    value.type === "battle:rng_init" ||
    value.type === "battle:action" ||
    value.type === "battle:state_hash"
  );
}

export class WebRTCBattleTransport implements BattleSyncTransport {
  private readonly conn: WebRTCConnectionLike;
  private readonly listeners = new Set<(message: BattleSyncMessage) => void>();
  private readonly onDataBound: (message: WebRTCEnvelope) => void;

  constructor(conn: WebRTCConnectionLike) {
    this.conn = conn;
    this.onDataBound = (message) => this.onEnvelope(message);
  }

  send(message: BattleSyncMessage): void {
    this.conn.send({ type: BATTLE_ENVELOPE_TYPE, data: message });
  }

  onData(callback: (message: BattleSyncMessage) => void): void {
    if (this.listeners.size === 0) {
      this.conn.onData(this.onDataBound);
    }
    this.listeners.add(callback);
  }

  offData(callback: (message: BattleSyncMessage) => void): void {
    this.listeners.delete(callback);
    if (this.listeners.size === 0 && this.conn.offData) {
      this.conn.offData(this.onDataBound);
    }
  }

  private onEnvelope(envelope: WebRTCEnvelope): void {
    if (!envelope || envelope.type !== BATTLE_ENVELOPE_TYPE) {
      return;
    }
    const payload = envelope.data;
    if (!isBattleSyncMessage(payload)) {
      throw new Error("[WebRTCBattleTransport] Invalid battle_sync payload");
    }
    this.listeners.forEach((cb) => cb(payload));
  }
}
