const PLAYER_KEY = 'crystal.multiplayer.player_id';
const TOKEN_KEY = 'crystal.multiplayer.token';
const CREDENTIAL_KEY = 'crystal.multiplayer.credentials';

function validPlayerId(value) {
  return typeof value === 'string' && /^[1-9][0-9]*$/.test(value)
    && BigInt(value) <= 18446744073709551615n;
}

export function prepareSession({ url, localStorage, sessionStorage, crypto, now = () => Date.now() / 1000 }) {
  url = new URL(url);
  if (url.searchParams.get('multiplayer') === 'off') {
    return { playerId: 'local', url };
  }
  const server = new URL(url.searchParams.get('multiplayer_server')
    ?? `${url.protocol === 'https:' ? 'wss:' : 'ws:'}//${url.host}/v1/ws`);
  if (!['ws:', 'wss:'].includes(server.protocol) || server.username || server.password) {
    throw new Error('The multiplayer server must be a WebSocket URL without embedded credentials.');
  }
  const saved = JSON.parse(sessionStorage.getItem(CREDENTIAL_KEY) ?? localStorage.getItem(CREDENTIAL_KEY) ?? 'null');
  const token = url.searchParams.get('token')
    ?? (saved?.server === server.href ? saved.token : null);
  const explicitId = url.searchParams.get('player_id');
  let playerId = explicitId;
  if (token) {
    // This only selects the local save slot. The server verifies the signature
    // and binds the WebSocket hello to this identity before accepting a player.
    let claims;
    try {
      const parts = token.split('.');
      if (parts.length !== 2) throw new Error();
      claims = JSON.parse(atob(parts[0].replace(/-/g, '+').replace(/_/g, '/')));
    } catch {
      throw new Error('This multiplayer invite is invalid. Open a new signed invite.');
    }
    if (!Number.isFinite(claims.expires_at) || claims.expires_at <= now()) {
      throw new Error('This multiplayer invite has expired. Open a new invite.');
    }
    if (typeof claims.user_id !== 'string' || !claims.user_id.startsWith('player-') || !validPlayerId(claims.user_id.slice(7))) {
      throw new Error('Browser invites must identify a player with a nonzero numeric ID.');
    }
    playerId = claims.user_id.slice(7);
    if (explicitId && explicitId !== playerId) {
      throw new Error('This invite belongs to a different player than the player_id in the URL.');
    }
  }
  if (explicitId && !validPlayerId(explicitId)) throw new Error('The player ID is invalid.');
  playerId ??= sessionStorage.getItem(PLAYER_KEY) ?? localStorage.getItem(PLAYER_KEY);
  if (!validPlayerId(playerId)) {
    const bytes = crypto.getRandomValues(new Uint32Array(2));
    playerId = ((BigInt(bytes[0]) << 32n) | BigInt(bytes[1]) | 1n).toString();
  }
  // Commit server and token together only after all invite validation succeeds.
  if (token) {
    sessionStorage.setItem(CREDENTIAL_KEY, JSON.stringify({ server: server.href, token }));
    localStorage.setItem(CREDENTIAL_KEY, JSON.stringify({ server: server.href, token }));
    sessionStorage.setItem(TOKEN_KEY, token);
  } else {
    sessionStorage.removeItem(CREDENTIAL_KEY);
    sessionStorage.removeItem(TOKEN_KEY);
  }
  sessionStorage.setItem(PLAYER_KEY, playerId);
  localStorage.setItem(PLAYER_KEY, playerId);
  url.searchParams.delete('token');
  return { playerId, url, hasToken: Boolean(token) };
}

export async function startOnlineSession(context) {
  // Invoke fetch without the session object as its Web IDL receiver.
  const { fetch } = context;
  let session = prepareSession(context);
  if (session.playerId === 'local' || session.url.searchParams.has('multiplayer_server')) return session;
  const response = await fetch('/v1/status', { cache: 'no-store' });
  if (!response.ok) throw new Error('The multiplayer server is unavailable. Please reload shortly.');
  const status = await response.json();
  if (status.authRequired && !session.hasToken) {
    const registration = await fetch('/v1/session', { method: 'POST', cache: 'no-store' });
    if (!registration.ok) throw new Error('Unable to start your online session. Please reload shortly.');
    const { token } = await registration.json();
    if (typeof token !== 'string' || !token) throw new Error('The server returned an invalid player session.');
    const url = new URL(session.url);
    url.searchParams.delete('player_id');
    url.searchParams.set('token', token);
    session = prepareSession({ ...context, url });
  }
  return session;
}

export async function runWithPlayerLock(locks, playerId, start) {
  if (!locks) throw new Error('Open the game over HTTPS in a browser that supports Web Locks.');
  await locks.request(`crystal.game.player-${playerId}`, { ifAvailable: true }, async lock => {
    if (!lock) throw new Error('This player is already open in another tab. Close that tab, then reload here.');
    await start();
    // Keep ownership for the lifetime of the page, including while WASM's
    // event loop runs after initialization returns. Closing the tab releases it.
    await new Promise(() => {});
  });
}
