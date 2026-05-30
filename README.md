# Pokemon Crystal LLM Benchmarking Toolkit

![Pokemon battle](img/battle.png)

A terminal based Pokemon Crystal with TUI Renderer, faithful tile
graphics, and built in MCP/Skill support. Aimed at benchmarking local LLMs on a consistent, deterministic, and fully observable environment with a rich state space and long-horizon gameplay.

## Setup

Use Node.js `24.x+`, npm `10.5+`, Git, and `ffmpeg`.

If you use `nvm`:

```bash
nvm install 24
nvm use 24
```

On macOS, install `ffmpeg` before generating audio:

```bash
brew install ffmpeg
```

Then clone both repositories and generate the local runtime assets:

```bash
git clone https://github.com/OWNER/pokecrystal-python.git
cd pokecrystal-python

git clone https://github.com/pret/pokecrystal.git vendor/pokecrystal

npm install
npm run export:core
node apps/web/scripts/prepare-public.js
npm run build:cli
```

## Docker Server Container

The Docker dev server is the `pokecrystal-ts` service in `docker-compose.yml`.
It runs the Next.js web/MCP server inside the container on port `3000` and
publishes it on the host as `http://localhost:3003`.

Start or rebuild the server container:

```bash
docker compose up --build pokecrystal-ts
```

Run it in the background:

```bash
docker compose up -d --build pokecrystal-ts
```

Stop the server container without deleting saves:

```bash
docker compose stop pokecrystal-ts
```

Start a stopped container again:

```bash
docker compose start pokecrystal-ts
```

Stop and remove the container/network while keeping the named save volume:

```bash
docker compose down
```

Follow server logs:

```bash
docker compose logs -f pokecrystal-ts
```

## Reset Docker Save State

The Docker dev stack persists game saves in the named volume `pokecrystal_saves`,
mounted at `/data` inside the container. That means a plain container restart
does not reset the save state.

To restart from a clean save in Docker:

```bash
docker compose down -v
docker compose up --build
```

`down -v` removes the save volume, so the next `up` starts with a fresh
autosave slot. If you want to keep the container but wipe only the save files,
remove the saved slot inside the running container under `/data` and then restart
the service.

The audio bundle is optional for launching the TUI, but required if you want the
browser or CLI audio manifests and MP3 assets:

```bash
npm run audio:bundle --workspace @pokecrystal/web
node apps/web/scripts/prepare-public.js
```

## Disassembly And Generated Assets

The `vendor/pokecrystal/` checkout is a critical local input. It is ignored by Git because it is a separate upstream repo, but fresh development environments need it unless `POKECRYSTAL_DISASSEMBLY_ROOT` points somewhere else.

Expected layout:

```text
pokecrystal-python/
  vendor/
    pokecrystal/
      audio/
      data/
      engine/
      gfx/
      maps/
```

The generated data and audio commands read from that checkout:

```bash
npm run export:core
npm run audio:bundle --workspace @pokecrystal/web
node apps/web/scripts/prepare-public.js
```

`npm run export:core` regenerates ASM-derived runtime data through the TypeScript exporter path. `node apps/web/scripts/prepare-public.js` exports runtime fallback assets, refreshes content-pack indexes, and regenerates the web and core asset manifest files required by the local runtime. `npm run audio:bundle --workspace @pokecrystal/web` compiles the disassembly audio sources into ignored browser MP3 files and manifests under `apps/web/assets/audio/`.

Legacy root-level `pokecrystal_disassembly/` checkouts still work, but new clones should use `vendor/pokecrystal/`. If your `pret/pokecrystal` checkout lives outside this repo, leave it there and set:

```bash
export POKECRYSTAL_DISASSEMBLY_ROOT=/absolute/path/to/pokecrystal
npm run export:core
npm run audio:bundle --workspace @pokecrystal/web
```

## Quick Start: TUI

Build the CLI, then start the terminal UI:

```bash
npm run build:cli
node packages/cli/dist/bin/pokecrystal-cli.js play --session-id my-session
```

What `play` gives you:
- Local Pokemon Crystal runtime
- Terminal UI
- Session-scoped MCP endpoint owned by the TUI

Useful controls:
- Arrow keys, `WASD`, or `HJKL`: move
- `Z`, `J`, or Space: A
- `X`, `K`, or `B`: B
- Enter: Start
- Tab: Select
- `.`: wait
- `R`: refresh
- `:q!`: quit

## Render Modes

The TUI has two Game Boy render modes:
- `text`: ASCII viewport, works in any terminal
- `kitty`: frame/image rendering for Kitty-graphics terminals such as Ghostty and Kitty

Switch renderers while the TUI is running:

```text
:u
```

Useful environment overrides:

```bash
POKECRYSTAL_CLI_KITTY=0   # force text mode
POKECRYSTAL_CLI_KITTY=1   # force Kitty/Ghostty image mode
```

If image rendering is unavailable, the CLI falls back to text automatically.

Bedroom frame image:

![Starting bedroom frame](img/bedroom.png)

Example text-frame capture from the starting bedroom (`PlayersHouse2F`):

```text
   00 01 02 03 04 05 06 07
00 #  #  #  #  #  #  S  D
01 .  @v P  S  !  B  .  .
02 .  .  .  .  .  .  .  .
03 .  .  .  .  .  .  .  .
04 #  .  .  .  #  #  .  .
05 #  .  .  .  .  .  .  .
```

Legend:
- `@`: player
- `D`: downstairs door
- `P`: PC
- `S`: sign
- `B`: bookshelf
- `#`: blocked
- `.`: floor

## Quick Start: MCP

Run the CLI as a stdio MCP server:

```bash
npm run build:cli
node packages/cli/dist/bin/pokecrystal-cli.js mcp --session-id my-session
```

Target a running web app instead of the local runtime:

```bash
node packages/cli/dist/bin/pokecrystal-cli.js mcp \
  --transport http \
  --base-url http://localhost:3000 \
  --session-id my-session
```

## Quick Start: Skill

Print the packaged Codex skill:

```bash
npm run build:cli
node packages/cli/dist/bin/pokecrystal-cli.js skill --print
```

Use that skill with any Codex or MCP client that can consume repo-local skill instructions.

## Benchmark Local LLMs In The TUI

Build the CLI and agent packages:

```bash
npm run build:cli
npm run build:agents
```

### Ollama

Start Ollama, then run a linked agent from inside the TUI:

```bash
env OLLAMA_BASE_URL=http://127.0.0.1:11434 \
  OLLAMA_API_KEY=local \
  node packages/cli/dist/bin/pokecrystal-cli.js play \
  --session-id ollama-benchmark \
  --agent \
  --agent-model ollama/gemma3:12b \
  --agent-goal "Play honest main-story Pokemon Crystal progress." \
  --agent-graph-cycle-steps 800000 \
  --agent-request-delay-ms 100 \
  --agent-identity-name ollama-benchmark \
  --training-dir /tmp/pokecrystal-ollama-benchmark
```

Change only `ollama/<model>` to compare models.

### llama.cpp Or Other OpenAI-Compatible Local Servers

Make sure the server exposes `/v1/models`, then run:

```bash
env LLAMA_CPP_BASE_URL=http://127.0.0.1:8080 \
  OLLAMA_API_KEY=local \
  node packages/cli/dist/bin/pokecrystal-cli.js play \
  --session-id llamacpp-benchmark \
  --agent \
  --agent-model ollama/your-model-id \
  --agent-goal "Continue beating the game." \
  --agent-max-steps 100000 \
  --agent-graph-cycle-steps 800000 \
  --agent-request-delay-ms 100 \
  --agent-identity-name llamacpp-benchmark \
  --training-dir /tmp/pokecrystal-llamacpp-benchmark
```

`your-model-id` must match the model id returned by the local server.

Concrete example:

```bash
env LLAMA_CPP_BASE_URL=http://127.0.0.1:8080 \
  OLLAMA_API_KEY=local \
  node packages/cli/dist/bin/pokecrystal-cli.js play \
  --session-id local2-llamacpp-gemma4-e2b \
  --agent \
  --agent-model ollama/gemma-4-E4B-it-Q4_K_M.gguf \
  --agent-goal "Continue beating the game." \
  --agent-max-steps 100000 \
  --agent-graph-cycle-steps 800000 \
  --agent-request-delay-ms 100 \
  --agent-identity-name llamacpp-gemma4-agent-2 \
  --training-dir /tmp/pokecrystal-local2-llamacpp-gemma4-e2b
```

## Benchmark Workflow

Use the same process for each local model:

1. Keep the goal text fixed.
2. Use a fresh `--session-id`.
3. Use a separate `--training-dir`.
4. Compare the saved traces after each run.

Useful outputs:
- `apps/web/.pokecrystal-agents/runs/<session-id>/training/episode.jsonl`
- `apps/web/.pokecrystal-agents/runs/<session-id>/training/manifest.json`
- `apps/web/mcp-<session-id>-runtime.json`

## Common Commands

```bash
npm run build:cli
npm run build:agents
npm run test:cli
npm run test:agents
npm run build
```

## More Docs

- [packages/cli/README.md](packages/cli/README.md)
- [apps/web/README.md](apps/web/README.md)
- [packages/core/README.md](packages/core/README.md)
