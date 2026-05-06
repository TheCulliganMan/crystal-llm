import { LinkCableEmulator } from './link-cable';

class MockWebRTCConnection {
  private callbacks: Array<(msg: any) => void> = [];
  public peer: MockWebRTCConnection | null = null;

  onData(cb: (msg: any) => void): void {
    this.callbacks.push(cb);
  }

  send(msg: any): void {
    // Deliver immediately to the other side.
    this.peer?.callbacks.forEach((cb) => cb(msg));
  }

  destroy(): void {}
}

function makePair(): [MockWebRTCConnection, MockWebRTCConnection] {
  const a = new MockWebRTCConnection();
  const b = new MockWebRTCConnection();
  a.peer = b;
  b.peer = a;
  return [a, b];
}

describe('LinkCableEmulator', () => {
  test('preamble handshake succeeds', async () => {
    const [a, b] = makePair();
    const host = new LinkCableEmulator(a as any, true);
    const client = new LinkCableEmulator(b as any, false);

    const [hostOk, clientOk] = await Promise.all([
      host.establishConnection(),
      client.establishConnection(),
    ]);

    expect(hostOk).toBe(true);
    expect(clientOk).toBe(true);
  });

  test('sendByte exchanges bytes when both sides transfer', async () => {
    const [a, b] = makePair();
    const host = new LinkCableEmulator(a as any, true);
    const client = new LinkCableEmulator(b as any, false);

    await Promise.all([host.establishConnection(), client.establishConnection()]);

    const [hostIn, clientIn] = await Promise.all([
      host.sendByte(0x42),
      client.sendByte(0x99),
    ]);

    expect(hostIn).toBe(0x99);
    expect(clientIn).toBe(0x42);
  });

  test('sendByte rejects invalid byte', async () => {
    const [a] = makePair();
    const host = new LinkCableEmulator(a as any, true);
    await expect(host.sendByte(0x1ff)).rejects.toThrow(/Invalid byte value/);
  });
});

