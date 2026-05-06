import { WebRTCConnection } from './webrtc-connection';

jest.mock('simple-peer', () => {
  const handlersByPeer = new WeakMap<any, Map<string, Function>>();

  class FakePeer {
    public destroyed = false;
    public _pc = { getStats: jest.fn(async () => ({})) };
    public readonly opts: any;
    constructor(opts: any) {
      this.opts = opts;
      handlersByPeer.set(this, new Map());
    }
    on(event: string, cb: any) {
      handlersByPeer.get(this)!.set(event, cb);
      return this;
    }
    emit(event: string, ...args: any[]) {
      const cb = handlersByPeer.get(this)!.get(event);
      cb?.(...args);
    }
    signal = jest.fn();
    send = jest.fn();
    destroy = jest.fn(() => {
      this.destroyed = true;
    });
  }

  return {
    __esModule: true,
    default: FakePeer,
  };
});

jest.mock('./realtime-manager', () => ({
  RealtimeManager: jest.fn().mockImplementation(() => ({
    joinMatchChannel: jest.fn(async () => {}),
    onMessage: jest.fn(),
    sendMessage: jest.fn(async () => {}),
    disconnect: jest.fn(async () => {}),
  })),
}));

describe('WebRTCConnection', () => {
  test('emits offer/answer/ice signaling types', async () => {
    const { RealtimeManager } = jest.requireMock('./realtime-manager') as any;
    const rtc = new WebRTCConnection({ matchId: 'm1', isHost: true });

    // Let async init run.
    await new Promise((r) => setTimeout(r, 0));

    const peer: any = (rtc as any).peer;
    const rm = (rtc as any).realtimeManager;
    expect(rm.joinMatchChannel).toHaveBeenCalledWith('m1');

    peer.emit('signal', { type: 'offer', sdp: 'o' });
    peer.emit('signal', { type: 'answer', sdp: 'a' });
    peer.emit('signal', { candidate: 'c' });

    expect(rm.sendMessage).toHaveBeenCalledWith({
      type: 'webrtc:offer',
      payload: { type: 'offer', sdp: 'o' },
    });
    expect(rm.sendMessage).toHaveBeenCalledWith({
      type: 'webrtc:answer',
      payload: { type: 'answer', sdp: 'a' },
    });
    expect(rm.sendMessage).toHaveBeenCalledWith({
      type: 'webrtc:ice',
      payload: { candidate: 'c' },
    });

    // Ensure our mock was used.
    expect(RealtimeManager).toHaveBeenCalledTimes(1);
  });

  test('reports connection lifecycle through callbacks and isConnected()', async () => {
    const rtc = new WebRTCConnection({ matchId: 'm1', isHost: true });
    await new Promise((r) => setTimeout(r, 0));

    const peer: any = (rtc as any).peer;
    const onConnect = jest.fn();
    const onDisconnect = jest.fn();
    const onError = jest.fn();

    rtc.onStatus({ onConnect, onDisconnect, onError });

    peer.emit('connect');
    expect(onConnect).toHaveBeenCalledTimes(1);
    expect(rtc.isConnected()).toBe(true);

    const error = new Error('boom');
    peer.emit('error', error);
    expect(onError).toHaveBeenCalledWith(error);
    expect(rtc.isConnected()).toBe(false);

    peer.emit('close');
    expect(onDisconnect).toHaveBeenCalledTimes(1);
    expect(rtc.isConnected()).toBe(false);
  });
});
