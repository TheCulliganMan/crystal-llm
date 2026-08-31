# Rust multiplayer server

Multiplayer is hosted by one Tokio/Axum executable. The production container has
no Node.js, npm, Next.js, Supabase, or JavaScript server process. It serves the
prebuilt Rust/WASM game, content packs, health/status endpoints, and WebSockets.
The final image is a non-root distroless runtime containing the Rust binary and
the Rust-built browser assets; compilers and build tools stay in the build stage.

## Start it

Docker is the shortest path:

```sh
cp .env.multiplayer.example .env
# Replace CRYSTAL_SERVER_TOKEN for a private host, or configure signed user
# tokens as described below, before exposing this port publicly.
docker compose -f docker-compose.production.yml up -d --build
curl http://localhost:3003/healthz
```

Open `http://localhost:3003`. Server status and the configured modpack allowlist
are available at `http://localhost:3003/v1/status`; WebSocket clients connect to
`ws://localhost:3003/v1/ws?token=...`.

To run the same executable without Docker:

```sh
cargo run --release \
  --manifest-path rust/Cargo.toml \
  --package crystal-web-server \
  --bin crystal-web-server -- \
  --host 0.0.0.0 --port 3003 \
  --dir rust/web-dist --pack-dir content-packs
```

## Player experience

Players in the same world and exact modpack build appear on the overworld as
translucent blue ghosts. Position and facing changes are sent only when they
change; leaving a map or disconnecting removes the ghost immediately.

The original Cable Club supports Battle, Trade, and Time Capsule queues. A
player can also face an adjacent ghost and press `C` for Battle, `V` for Trade,
or `T` for Time Capsule. The challenged player presses `Z` to accept or `X` to
decline. Completed and cancelled sessions return both clients to the lobby, so
another session does not require restarting the game.

Battle turns, forced replacements, parties, trades, confirmations, menus,
checkpoints, and deterministic inputs use the binary Rust link protocol. Both
clients report a terminal battle result, and ranked ratings change only when
their reports agree.

## Authentication

`CRYSTAL_SERVER_TOKEN` is the simple private-server option: everyone uses one
shared token. For a public host, leave it empty and set `CRYSTAL_AUTH_SECRET` to
at least 32 random bytes. Issue an identity-bound token with the container:

```sh
docker compose -f docker-compose.production.yml run --rm \
  pokecrystal-multiplayer --issue-token player-123 2592000
```

The final argument is lifetime in seconds. A signed token authenticates only
its embedded player id; expired or modified tokens are rejected before the
WebSocket upgrade. Never set both authentication variables.

## Modpacks

Mount `.crystalpack` files below `/srv/crystal/packs` (the Compose file mounts
`./content-packs` there). A client hello includes all three compatibility keys:

- world id
- runtime modpack id
- runtime content hash

Presence, challenges, and matchmaking require an exact identity match, so two
different pack builds cannot enter the same session. Set `CRYSTAL_MODPACKS` to a
comma-separated exact allowlist for a public host, for example:

```text
CRYSTAL_MODPACKS=core-modular=RUNTIME_HASH,gen3=RUNTIME_HASH
```

An empty allowlist accepts any syntactically valid modpack identity while still
keeping identities isolated. That is convenient for private modpack development;
an explicit allowlist is safer for a public host.

## Capacity

The hub partitions presence by world, exact modpack build, and map. Matchmaking
is also partitioned by exact world/modpack identity. Each socket has bounded
outbound buffering, relay payloads are capped at 64 KiB, and inbound traffic is
rate limited. The container raises its file-descriptor ceiling to 65,536.

The pure-Rust hub benchmark defaults to 10,000 simulated users:

```sh
cargo run --release \
  --manifest-path rust/Cargo.toml \
  --package crystal-web-server \
  --bin crystal-multiplayer-load -- 10000
```

That benchmark verifies hub behavior and gives a CPU baseline; it is not a claim
that every host can sustain 10,000 live Internet sockets. Exercise real sockets
against a running server with:

```sh
cargo run --release \
  --manifest-path rust/Cargo.toml \
  --package crystal-web-server \
  --bin crystal-multiplayer-load -- \
  --network ws://127.0.0.1:3003/v1/ws 10000
```

The network test verifies real socket upgrades, pair-isolated ghost identity and
tile updates, matchmaking, binary session relay, and ping latency. If shared
authentication is enabled, append `?token=YOUR_TOKEN` to the quoted URL.
Before a public launch, run this against the exact VM/container limits and monitor
memory, file descriptors, message latency, reconnects, and the busiest map.

Ratings are atomically persisted in the `crystal_multiplayer_data` Docker volume
and loaded on restart. Active WebSockets and in-progress real-time sessions end
when the process restarts; deterministic battles are not falsely resumed after
a network break. Thousands of sockets on one appropriately sized host are the
initial deployment target. Multiple replicas require an explicit shared Rust
routing layer and are not silently treated as interchangeable.
