import { WebRTCBattleTransport, type WebRTCConnectionLike } from "./webrtc-battle-transport";
import type { BattleSyncMessage } from "./battle-synchronizer";

function makeConn() {
  const cbs: Array<(msg: { type: string; data: any }) => void> = [];
  const conn: WebRTCConnectionLike & { emit: (msg: { type: string; data: any }) => void } = {
    send: jest.fn(),
    onData: (cb) => {
      cbs.push(cb);
    },
    offData: (cb) => {
      const idx = cbs.indexOf(cb);
      if (idx !== -1) cbs.splice(idx, 1);
    },
    emit: (msg) => {
      cbs.forEach((cb) => cb(msg));
    },
  };
  return conn;
}

describe("WebRTCBattleTransport", () => {
  test("wraps send() payloads in battle_sync envelopes", () => {
    const conn = makeConn();
    const transport = new WebRTCBattleTransport(conn);

    const message: BattleSyncMessage = {
      type: "battle:state_hash",
      data: { turn: 3, hash: "abc" },
    };
    transport.send(message);

    expect(conn.send).toHaveBeenCalledWith({ type: "battle_sync", data: message });
  });

  test("delivers battle_sync payloads to listeners", () => {
    const conn = makeConn();
    const transport = new WebRTCBattleTransport(conn);
    const received: BattleSyncMessage[] = [];

    transport.onData((msg) => received.push(msg));

    const message: BattleSyncMessage = { type: "battle:rng_init", data: { hardware_divider: 1, hRandomAdd: 2, hRandomSub: 3 } };
    conn.emit({ type: "battle_sync", data: message });

    expect(received).toEqual([message]);
  });

  test("ignores non battle_sync envelopes", () => {
    const conn = makeConn();
    const transport = new WebRTCBattleTransport(conn);
    const received: BattleSyncMessage[] = [];

    transport.onData((msg) => received.push(msg));
    conn.emit({ type: "other", data: { hello: "world" } });

    expect(received).toEqual([]);
  });

  test("throws on invalid battle_sync payloads", () => {
    const conn = makeConn();
    const transport = new WebRTCBattleTransport(conn);
    transport.onData(() => undefined);

    expect(() => conn.emit({ type: "battle_sync", data: { type: "nope" } })).toThrow(
      /Invalid battle_sync payload/
    );
  });
});

