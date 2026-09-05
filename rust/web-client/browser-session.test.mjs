import test from 'node:test';
import assert from 'node:assert/strict';
import { prepareSession, startOnlineSession, runWithPlayerLock } from './browser-session.js';

function storage(entries = {}) {
  const values = new Map(Object.entries(entries));
  return { getItem: key => values.get(key) ?? null, setItem: (key, value) => values.set(key, value), removeItem: key => values.delete(key) };
}
function context(url = 'https://game.test/') {
  return { url: new URL(url), localStorage: storage(), sessionStorage: storage(), crypto: globalThis.crypto, now: () => 1000 };
}
const signedToken = (id, expiry = 2000) => `${Buffer.from(JSON.stringify({ user_id: id, expires_at: expiry })).toString('base64url')}.signature`;

test('signed invite selects its own save identity and removes token from URL', () => {
  const token = signedToken('player-42');
  const ctx = context(`https://game.test/?token=${token}`);
  ctx.localStorage.setItem('crystal.multiplayer.player_id', '99');
  const result = prepareSession(ctx);
  assert.equal(result.playerId, '42');
  assert.equal(ctx.sessionStorage.getItem('crystal.multiplayer.token'), token);
  assert.equal(ctx.sessionStorage.getItem('crystal.multiplayer.player_id'), '42');
  assert.equal(result.url.searchParams.has('token'), false);
});
test('reload retains token and player identity within the tab', () => {
  const ctx = context(`https://game.test/?token=${signedToken('player-42')}`);
  const first = prepareSession(ctx);
  ctx.url = first.url;
  assert.equal(prepareSession(ctx).playerId, '42');
});
test('rejects expired tokens and conflicting explicit identities', () => {
  assert.throws(() => prepareSession(context(`https://game.test/?token=${signedToken('player-42', 999)}`)), /expired/);
  assert.throws(() => prepareSession(context(`https://game.test/?token=${signedToken('player-42')}&player_id=99`)), /different player/);
});
test('new anonymous player is stable and uses a nonzero integer', () => {
  const ctx = context();
  const first = prepareSession(ctx);
  assert.ok(BigInt(first.playerId) > 0n);
  assert.equal(prepareSession(ctx).playerId, first.playerId);
});
test('duplicate tab never starts the game or changes its save identity', async () => {
  let started = false;
  const locks = { request: async (name, options, callback) => { assert.equal(name, 'crystal.game.player-42'); assert.equal(options.ifAvailable, true); return callback(null); } };
  await assert.rejects(runWithPlayerLock(locks, '42', () => { started = true; }), /already open/);
  assert.equal(started, false);
});
test('saved credentials are never forwarded to a different multiplayer server', () => {
  const ctx = context(`https://game.test/?token=${signedToken('player-42')}`);
  prepareSession(ctx);
  ctx.url = new URL('https://game.test/?multiplayer_server=wss://other.test/v1/ws');
  const result = prepareSession(ctx);
  assert.equal(result.hasToken, false);
  assert.equal(ctx.sessionStorage.getItem('crystal.multiplayer.token'), null);
});
test('a rejected invite cannot rebind a saved token to another server', () => {
  const ctx = context(`https://game.test/?token=${signedToken('player-42')}`);
  prepareSession(ctx);
  ctx.url = new URL('https://game.test/?multiplayer_server=wss://other.test/v1/ws&token=bad');
  assert.throws(() => prepareSession(ctx), /invalid/);
  ctx.url.searchParams.delete('token');
  assert.equal(prepareSession(ctx).hasToken, false);
});

test('online play obtains credentials without an invite and reuses them in a new tab', async () => {
  const ctx = context();
  let registrations = 0;
  ctx.fetch = async (url, options) => {
    if (url === '/v1/status') return { ok: true, json: async () => ({ authRequired: true }) };
    assert.equal(url, '/v1/session');
    assert.equal(options.method, 'POST');
    registrations++;
    return { ok: true, json: async () => ({ token: signedToken('player-42') }) };
  };
  assert.equal((await startOnlineSession(ctx)).playerId, '42');
  ctx.sessionStorage = storage();
  assert.equal((await startOnlineSession(ctx)).playerId, '42');
  assert.equal(registrations, 1);
});

test('online startup calls an unbound browser fetch without the session as its receiver', async () => {
  const ctx = context();
  const calls = [];
  ctx.fetch = async function (url) {
    // Window.fetch rejects a plain session object as its Web IDL receiver.
    assert.equal(this, undefined, "fetch must be called as a function, not context.fetch()");
    calls.push(url);
    return { ok: true, json: async () => url === '/v1/status'
      ? { authRequired: true } : { token: signedToken('player-42') } };
  };
  assert.equal((await startOnlineSession(ctx)).playerId, '42');
  assert.deepEqual(calls, ['/v1/status', '/v1/session']);
  assert.equal((await startOnlineSession(ctx)).playerId, '42');
  assert.equal(calls.length, 3);
});
